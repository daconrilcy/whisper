[CmdletBinding(PositionalBinding = $false)]
param(
    [ValidateSet('SelfTest','Preflight','List','Run')]
    [string]$Mode = 'Preflight',
    [string]$Profile,
    [string]$ValuesJson,
    [string]$LotRoot = 'target/n2-native',
    [switch]$DryRun,
    [ValidatePattern('^[a-zA-Z0-9][a-zA-Z0-9_-]{0,31}$')][string]$NativeBuildId,
    [switch]$NativePrepareOnly
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$PSNativeCommandUseErrorActionPreference = $false

$script:Repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path.TrimEnd('\')
$script:ProfilesPath = Join-Path $PSScriptRoot 'native-profiles.json'
$script:CargoTarget = 'C:\dev\whisper\target\parakeet-tests\cargo-shared'
$script:Lot = $null
$script:RunRoot = $null
$script:LogRoot = $null
$script:Utf8NoBom = [Text.UTF8Encoding]::new($false)
. (Join-Path $PSScriptRoot 'native-workspace.ps1')
$script:AllowedReadRoots = @(
    $script:Repo,
    'C:\dev\whisper\target\parakeet-tests',
    'C:\dev\whisper\sound example',
    'C:\dev\whisper\models'
)
$script:AllowedToolRoots = @(
    'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools',
    'C:\Program Files\Microsoft Visual Studio\2022\BuildTools',
    'C:\Program Files (x86)\Windows Kits',
    'C:\Program Files\CMake',
    'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA'
)
$script:AllowedProfileEnvironmentNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($name in @('CUDA_PATH','CUDA_PATH_V12_9','CUDA_HOME','GGML_CUDA','CMAKE_GENERATOR','CMAKE_PREFIX_PATH','LIBCLANG_PATH','CLANG_PATH','CC','CXX','CFLAGS','CXXFLAGS','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS')) { [void]$script:AllowedProfileEnvironmentNames.Add($name) }
$script:AllowedCompileEnvironment = @{
    WHISPER_L04_CAMPAIGN_BUILD = '1'
    PARAKEET_L2_ACCEPTANCE_TEST_BUILD = '1'
}

function Get-FullPath([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) { throw 'A path is required.' }
    return [IO.Path]::GetFullPath($Path).TrimEnd('\')
}

function Test-InRoot([string]$Path, [string]$Root) {
    $fullPath = Get-FullPath $Path
    $fullRoot = Get-FullPath $Root
    return $fullPath.Equals($fullRoot, [StringComparison]::OrdinalIgnoreCase) -or
        $fullPath.StartsWith($fullRoot + '\', [StringComparison]::OrdinalIgnoreCase)
}

function Assert-NoReparseChain([string]$Path, [string]$Context) {
    $cursor = Get-FullPath $Path
    while ($true) {
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -LiteralPath $cursor -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "$Context crosses a reparse point: $cursor" }
        }
        $parent = Split-Path -Parent $cursor
        if ([string]::IsNullOrWhiteSpace($parent) -or $parent -eq $cursor) { break }
        $cursor = $parent
    }
}

function Resolve-ProgramW6432Directory([string]$ProcessValue, [string]$RegistryValue) {
    if ([string]::IsNullOrWhiteSpace($ProcessValue) -or [string]::IsNullOrWhiteSpace($RegistryValue)) { throw 'ProgramW6432 process and registry values are both required.' }
    foreach ($entry in @(@{name='process';value=$ProcessValue},@{name='registry';value=$RegistryValue})) {
        if (-not [IO.Path]::IsPathRooted([string]$entry.value)) { throw "ProgramW6432 $($entry.name) value is not an absolute path." }
        $canonical = Get-FullPath ([string]$entry.value)
        if ($canonical -ine ([string]$entry.value).TrimEnd('\')) { throw "ProgramW6432 $($entry.name) value is not canonical." }
        if (-not (Test-Path -LiteralPath $canonical -PathType Container)) { throw "ProgramW6432 $($entry.name) directory is missing: $canonical" }
        Assert-NoReparseChain $canonical "ProgramW6432 $($entry.name) directory"
        if ($entry.name -ceq 'process') { $processCanonical = $canonical } else { $registryCanonical = $canonical }
    }
    if ($processCanonical -ine $registryCanonical) { throw 'ProgramW6432 process and 64-bit registry paths differ.' }
    return $processCanonical
}

function Get-ValidatedProgramW6432Directory {
    $processValue = [Environment]::GetEnvironmentVariable('ProgramW6432','Process')
    $baseKey = $null; $key = $null
    try {
        $baseKey = [Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::LocalMachine,[Microsoft.Win32.RegistryView]::Registry64)
        $key = $baseKey.OpenSubKey('SOFTWARE\Microsoft\Windows\CurrentVersion',$false)
        if ($null -eq $key) { throw '64-bit Windows CurrentVersion registry key is missing.' }
        $registryValue = [string]$key.GetValue('ProgramW6432Dir',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        return Resolve-ProgramW6432Directory ([string]$processValue) $registryValue
    } catch {
        throw "ProgramW6432 host validation failed closed: $($_.Exception.Message)"
    } finally {
        if ($null -ne $key) { $key.Dispose() }
        if ($null -ne $baseKey) { $baseKey.Dispose() }
    }
}

function Test-ProgramW6432ProfileId([string]$ProfileId) {
    return $ProfileId -cin @(
        'GpuCudaProbe','TurboGpu','TurboGpuLive','TurboGpuReusableImports',
        'PersonalUnified','PersonalUnifiedGpuOnly',
        'NativeClosureGpuReference','NativeClosureGpuRebuilt',
        'ParakeetGpu','ParakeetGpuFrenchShort','ParakeetGpuFrenchLong',
        'ParakeetGpuEnglishLong','ParakeetGpuFrenchAutoLong','ParakeetGpuEnglishAutoLong',
        'NvidiaSmiInventoryDiagnostic','NvidiaSmiEnvironmentCompareDiagnostic','NvidiaProgramW6432Witness'
    )
}

function Invoke-ProgramW6432ResolverSelfTests([string]$Lot) {
    $root = Join-Path $Lot ('selftest-programw6432-' + [guid]::NewGuid().ToString('N'))
    $junction = Join-Path $root 'reparse-directory'
    New-Item -ItemType Directory -Path (Join-Path $root 'valid'),(Join-Path $root 'other') -Force -ErrorAction Stop | Out-Null
    try {
        $validPath = Get-FullPath (Join-Path $root 'valid')
        $otherPath = Get-FullPath (Join-Path $root 'other')
        $resolved = Resolve-ProgramW6432Directory $validPath $validPath
        if ($resolved -ine $validPath) { throw 'ProgramW6432 resolver changed a canonical matching process/registry path.' }
        Test-Rejects 'missing ProgramW6432 process value' { Resolve-ProgramW6432Directory '' $validPath }
        Test-Rejects 'missing ProgramW6432 registry value' { Resolve-ProgramW6432Directory $validPath '' }
        Test-Rejects 'ProgramW6432 process/registry mismatch' { Resolve-ProgramW6432Directory $validPath $otherPath }
        $missingPath = Get-FullPath (Join-Path $root 'missing')
        Test-Rejects 'nonexistent ProgramW6432 directory' { Resolve-ProgramW6432Directory $missingPath $missingPath }
        New-Item -ItemType Junction -Path $junction -Target $validPath -ErrorAction Stop | Out-Null
        Test-Rejects 'ProgramW6432 reparse directory' { Resolve-ProgramW6432Directory $junction $junction }
        return [pscustomobject]@{matchingCanonicalPath='PASS';missingProcessValue='REJECTED';missingRegistryValue='REJECTED';registryMismatch='REJECTED';missingDirectory='REJECTED';junction='REJECTED';machineRegistryModified=$false}
    } finally {
        if (Test-Path -LiteralPath $junction) { Remove-Item -LiteralPath $junction -Force -ErrorAction SilentlyContinue }
        if (Test-InRoot $root $Lot) { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue }
    }
}

function Assert-PathUnder([string]$Path, [string]$Root, [string]$Context, [switch]$MustExist, [switch]$Directory) {
    $full = Get-FullPath $Path
    $rootFull = Get-FullPath $Root
    if (-not (Test-InRoot $full $rootFull)) { throw "$Context escaped its authorized root: $full" }
    Assert-NoReparseChain $full $Context
    if ($MustExist) {
        $expectedType = if ($Directory) { 'Container' } else { 'Leaf' }
        if (-not (Test-Path -LiteralPath $full -PathType $expectedType)) { throw "$Context is missing: $full" }
    }
    return $full
}

function Get-SafeRelativePath([string]$Path, [string]$Context) {
    if ([string]::IsNullOrWhiteSpace($Path) -or $Path.Length -gt 240 -or [IO.Path]::IsPathRooted($Path) -or $Path -match '^[A-Za-z]:' -or $Path.StartsWith('\\') -or $Path.StartsWith('/')) {
        throw "$Context must be a bounded relative path."
    }
    $segments = @($Path -split '[\\/]')
    if ($segments.Count -eq 0) { throw "$Context is empty." }
    foreach ($segment in $segments) {
        if ([string]::IsNullOrWhiteSpace($segment) -or $segment -in @('.','..') -or $segment -match '[<>:"|?*\x00-\x1F]' -or $segment.EndsWith('.') -or $segment.EndsWith(' ')) {
            throw "$Context contains an unsafe path segment: $segment"
        }
        if ($segment -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\..*)?$') { throw "$Context contains a reserved Windows path segment: $segment" }
    }
    return [string]::Join('\',$segments)
}

function Convert-JsonStringArray([string]$Json, [string]$Context) {
    try { $parsed = ConvertFrom-Json -InputObject $Json -ErrorAction Stop }
    catch { throw "$Context is not valid JSON: $($_.Exception.Message)" }
    if ($parsed -is [string]) { return ,([string[]]@($parsed)) }
    if ($parsed -isnot [array] -or @($parsed | Where-Object { $_ -isnot [string] }).Count -gt 0) {
        throw "$Context must be a JSON array of strings."
    }
    return ,([string[]]$parsed)
}

function Convert-StringArrayToJson([string[]]$Values) {
    return ConvertTo-Json -InputObject ([string[]]$Values) -Compress
}

function Test-ExactStringSet([string[]]$Left, [string[]]$Right) {
    $leftItems = @($Left)
    $rightItems = @($Right)
    if ($leftItems.Count -ne $rightItems.Count) { return $false }
    foreach ($item in $leftItems) { if ($rightItems -cnotcontains [string]$item) { return $false } }
    return $true
}

function Get-Sha256([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $hasher = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($hasher.ComputeHash($stream)) -replace '-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $hasher.Dispose() }
}

function New-InvocationRecord([object]$Profile, [string]$CaseId, [object]$Toolchain, [string]$ArgvJson) {
    return [ordered]@{
        schemaVersion = 1; profile = [string]$Profile.id; operation = [string]$Profile.operation
        case = $CaseId; executable = [string]$Toolchain.cargo; argvJson = $ArgvJson
        cwd = $script:Repo; target = $script:CargoTarget; result = 'PENDING'
        exitCode = $null; elapsedSeconds = $null; testEvidence = $null; failure = $null
    }
}

function Update-InvocationRecord([string]$Path, [string]$Status, [object]$ExitCode, [object]$ElapsedSeconds, [object]$TestEvidence, [object]$Failure) {
    $record = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json -ErrorAction Stop
    if ($record.schemaVersion -ne 1) { throw "Unsupported invocation receipt schema: $Path" }
    foreach ($name in @('result','exitCode','elapsedSeconds','testEvidence','failure')) {
        if ($null -eq $record.PSObject.Properties[$name]) { throw "Invocation receipt lacks required field '$name': $Path" }
    }
    $record.result = $Status
    $record.exitCode = $ExitCode
    $record.elapsedSeconds = $ElapsedSeconds
    $record.testEvidence = $TestEvidence
    $record.failure = $Failure
    $record | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $Path -Encoding UTF8
    return $record
}

function Invoke-InvocationReceiptSelfTest([string]$Lot) {
    $path = Join-Path (Join-Path $Lot 'logs') 'guard-selftest-correction2-receipt.json'
    $toolchain = [pscustomobject]@{ cargo='C:\synthetic\cargo.exe' }
    $cases = @(
        [pscustomobject]@{ id='ReceiptNoRun'; operation='NoRun'; case='PreparationNoRun'; success='NO_RUN_PASS' },
        [pscustomobject]@{ id='ReceiptList'; operation='List'; case='InventoryList'; success='LIST_PASS' },
        [pscustomobject]@{ id='ReceiptRun'; operation='Run'; case='capture::tests::synthetic'; success='PASS' }
    )
    $summary = [Collections.Generic.List[object]]::new()
    foreach ($scenario in $cases) {
        $profile = [pscustomobject]@{ id=$scenario.id; operation=$scenario.operation }
        $evidence = if ($scenario.operation -eq 'Run') { [pscustomobject]@{ exactTest=$scenario.case; executionCount=1; passed=1; failed=0; ignored=0; verdict='PASS' } } else { $null }
        $initial = New-InvocationRecord $profile $scenario.case $toolchain '["test","--offline","--locked"]'
        $initial | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $path -Encoding UTF8
        $readInitial = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json -ErrorAction Stop
        foreach ($field in @('result','exitCode','elapsedSeconds','testEvidence','failure')) {
            if ($null -eq $readInitial.PSObject.Properties[$field]) { throw "Initial invocation receipt omitted $field for $($scenario.operation)." }
        }
        if ($readInitial.result -cne 'PENDING' -or $null -ne $readInitial.exitCode -or $null -ne $readInitial.elapsedSeconds) { throw "Initial invocation receipt state was invalid for $($scenario.operation)." }
        Update-InvocationRecord $path $scenario.success 0 1.25 $evidence $null | Out-Null
        $readSuccess = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json -ErrorAction Stop
        if ($readSuccess.result -cne $scenario.success -or [int]$readSuccess.exitCode -ne 0 -or [double]$readSuccess.elapsedSeconds -ne 1.25 -or $null -ne $readSuccess.failure) { throw "Success invocation receipt update failed for $($scenario.operation)." }
        if ($scenario.operation -eq 'Run' -and ([string]$readSuccess.testEvidence.exactTest -cne $scenario.case -or [int]$readSuccess.testEvidence.passed -ne 1)) { throw 'Run invocation receipt did not preserve testEvidence.' }
        if ($scenario.operation -ne 'Run' -and $null -ne $readSuccess.testEvidence) { throw "NoRun/List invocation receipt unexpectedly included testEvidence: $($scenario.operation)." }
        Update-InvocationRecord $path 'FAIL' 23 2.5 $null 'synthetic receipt failure' | Out-Null
        $readFailure = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json -ErrorAction Stop
        if ($readFailure.result -cne 'FAIL' -or [int]$readFailure.exitCode -ne 23 -or [double]$readFailure.elapsedSeconds -ne 2.5 -or $readFailure.failure -cne 'synthetic receipt failure') { throw "Failure invocation receipt update failed for $($scenario.operation)." }
        if ($null -ne $readFailure.testEvidence) { throw "Failure invocation receipt retained stale testEvidence for $($scenario.operation)." }
        $summary.Add([pscustomobject]@{ operation=$scenario.operation; initial='PENDING schema complete'; success=$readSuccess.result; failure=$readFailure.result; fields=@('exitCode','elapsedSeconds','testEvidence','failure') })
    }
    return @($summary.ToArray())
}

function Assert-ExactTestSelector([string]$Name) {
    if ([string]::IsNullOrWhiteSpace($Name) -or $Name -cnotmatch '^[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*$') {
        throw "Test selector must be one exact Rust test name: $Name"
    }
}

function Get-VoxPopuliProfileContract([string]$Id) {
    switch -CaseSensitive ($Id) {
        'ParakeetGpu' { return [pscustomobject]@{ operation='Run'; selector='voxpopuli::english_short_mp3_gpu'; timeoutSeconds=1200 } }
        'ParakeetGpuFrenchShort' { return [pscustomobject]@{ operation='Run'; selector='voxpopuli::french_short_wav_and_mp3_gpu'; timeoutSeconds=2100 } }
        'ParakeetGpuFrenchLong' { return [pscustomobject]@{ operation='Run'; selector='voxpopuli::french_long_gpu'; timeoutSeconds=5700 } }
        'ParakeetGpuEnglishLong' { return [pscustomobject]@{ operation='Run'; selector='voxpopuli::english_long_gpu'; timeoutSeconds=5700 } }
        'ParakeetGpuFrenchAutoLong' { return [pscustomobject]@{ operation='Run'; selector='voxpopuli::french_long_auto_gpu'; timeoutSeconds=5700 } }
        'ParakeetGpuEnglishAutoLong' { return [pscustomobject]@{ operation='Run'; selector='voxpopuli::english_long_auto_gpu'; timeoutSeconds=5700 } }
        'ParakeetGpuPreparation' { return [pscustomobject]@{ operation='NoRun'; selector=''; timeoutSeconds=1800 } }
        'ParakeetGpuInventory' { return [pscustomobject]@{ operation='List'; selector=''; timeoutSeconds=300 } }
        default { return $null }
    }
}

function Get-NativeCommandTimeoutSeconds([object]$Profile) {
    $contract = Get-VoxPopuliProfileContract ([string]$Profile.id)
    $hasTimeout = $Profile.PSObject.Properties.Name -contains 'commandTimeoutSeconds'
    if ($null -eq $contract) {
        if ($hasTimeout) { throw "Command timeout is only declared for the bounded VoxPopuli profiles: $($Profile.id)" }
        return $null
    }
    if (-not $hasTimeout -or [string]$Profile.commandTimeoutSeconds -cne [string]$contract.timeoutSeconds) {
        throw "VoxPopuli command timeout differs from its exact approved bound: $($Profile.id)"
    }
    return [int]$contract.timeoutSeconds
}

function Assert-VoxPopuliProfileContract([object]$Profile) {
    $contract = Get-VoxPopuliProfileContract ([string]$Profile.id)
    [void](Get-NativeCommandTimeoutSeconds $Profile)
    if ($null -eq $contract) { return }
    if ($Profile.package -cne 'whisper-adapters' -or $Profile.operation -cne $contract.operation -or @($Profile.features).Count -ne 1 -or $Profile.features[0] -cne 'whisper-adapters/native-tests' -or @($Profile.compileEnvironment).Count -ne 0) {
        throw "VoxPopuli package, operation, features or compile environment differs from its bounded contract: $($Profile.id)"
    }
    if ($Profile.status -ceq 'DISCOVERY_PENDING') {
        if ($contract.operation -cne 'Run') { throw 'VoxPopuli preparation and inventory must remain READY.' }
        foreach ($field in @('inputs','values','outputs','cases','staging')) {
            if (@($Profile.$field | Where-Object { $null -ne $_ }).Count -ne 0) { throw "Pending VoxPopuli profile must not contain provisional $field." }
        }
        return
    }
    if ($Profile.status -cne 'READY') { throw 'VoxPopuli profile has an unsupported readiness state.' }
    if ($contract.operation -ne 'Run') {
        foreach ($field in @('inputs','outputs','cases','staging')) {
            if (@($Profile.$field | Where-Object { $null -ne $_ }).Count -ne 0) { throw "VoxPopuli discovery profile must not declare $field." }
        }
        $actualValues = @($Profile.values | ForEach-Object { "{0}|{1}|{2}|{3}|{4}" -f $_.key,$_.environmentVariable,$_.kind,$_.allowedRoot,[bool]$_.required })
        $expectedValues = @('cudaToolkitPath|CUDA_PATH|directory|cuda-12.9|True','cudaToolkitVersionedPath|CUDA_PATH_V12_9|directory|cuda-12.9|True','libclangPath|LIBCLANG_PATH|literal||True')
        if (-not (Test-ExactStringSet $actualValues $expectedValues)) { throw 'VoxPopuli discovery environment differs from the bounded CUDA/libclang values.' }
        return
    }
    $actualInputs = @($Profile.inputs | ForEach-Object { "{0}|{1}|{2}" -f $_.key,$_.environmentVariable,$_.kind })
    $expectedInputs = @('personalRuntime|WHISPER_N2_PERSONAL_RUNTIME_ROOT|directory','voxpopuliCorpus|WHISPER_VOXPOPULI_CORPUS_ROOT|directory','voxpopuliSources|WHISPER_VOXPOPULI_PROVENANCE_ROOT|directory')
    if (-not (Test-ExactStringSet $actualInputs $expectedInputs)) { throw 'VoxPopuli inputs differ from its three explicit read-only sources.' }
    if (@($Profile.values).Count -ne 1 -or $Profile.values[0].key -cne 'cudaToolkitPath' -or $Profile.values[0].environmentVariable -cne 'CUDA_PATH' -or $Profile.values[0].kind -cne 'directory' -or $Profile.values[0].allowedRoot -cne 'cuda-12.9' -or -not [bool]$Profile.values[0].required) { throw 'VoxPopuli Run environment must declare only the exact CUDA toolkit value.' }
    if (@($Profile.outputs).Count -ne 1 -or $Profile.outputs[0].key -cne 'campaignRun' -or $Profile.outputs[0].environmentVariable -cne 'WHISPER_L04_CAMPAIGN_RUN' -or $Profile.outputs[0].kind -cne 'directory' -or $Profile.outputs[0].path -cne 'writes/voxpopuli-campaign') { throw 'VoxPopuli output must remain under its explicit RunRoot campaign directory.' }
    $actualStages = @($Profile.staging | ForEach-Object { "{0}|{1}|{2}|{3}|{4}" -f $_.inputKey,$_.destinationBase,$_.destination,$_.requiredFileCount,$_.requiredBytes })
    if (-not (Test-ExactStringSet $actualStages @('personalRuntime|localappdata|Whisper|21|3196008875','voxpopuliCorpus|run|writes/voxpopuli-corpus|14|151356410'))) { throw 'VoxPopuli staging differs from the qualified runtime and fourteen corpus file pins.' }
    if (@($Profile.cases).Count -ne 1 -or $Profile.cases[0].targetKind -cne 'integration' -or $Profile.cases[0].targetName -cne 'parakeet_unified_native' -or $Profile.cases[0].exactTest -cne $contract.selector -or [bool]$Profile.cases[0].ignored -or $Profile.cases[0].verdict -cne 'QUALIFICATION') { throw 'VoxPopuli profile must contain exactly its approved integration selector.' }
}

function Assert-VoxPopuliValues([object]$Profile, [hashtable]$Inputs) {
    $contract = Get-VoxPopuliProfileContract ([string]$Profile.id)
    if ($null -eq $contract -or $contract.operation -cne 'Run') { return }
    $runtime = $Inputs['personalRuntime']
    if (@($runtime.files).Count -ne 21) { throw 'VoxPopuli personal runtime must contain its exact twenty-one pinned files.' }
    foreach ($entry in @($runtime.files)) {
        $normalized = ([string]$entry.path).Replace('\','/')
        if ($normalized -ieq 'settings' -or $normalized.StartsWith('settings/',[StringComparison]::OrdinalIgnoreCase)) { throw 'VoxPopuli runtime staging must not precreate settings.' }
    }
    $expectedCorpusNames = @(
        'fr-short-60s.wav','fr-short-60s.mp3','fr-short-60s.reference.txt','fr-short-60s.annotations.json',
        'en-short-60s.wav','en-short-60s.mp3','en-short-60s.reference.txt','en-short-60s.annotations.json',
        'fr-long-35m.wav','fr-long-35m.reference.txt','fr-long-35m.annotations.json',
        'en-long-40m.wav','en-long-40m.reference.txt','en-long-40m.annotations.json'
    )
    if (-not (Test-ExactStringSet @($Inputs['voxpopuliCorpus'].files | ForEach-Object { [string]$_.path }) $expectedCorpusNames)) { throw 'VoxPopuli corpus files differ from the fourteen approved names.' }
    $sources = $Inputs['voxpopuliSources']
    $expectedSourceRoot = Get-FullPath (Join-Path $script:Lot 'qualification-gpu\corpus-preparation\raw')
    if ([string]$sources.path -ine $expectedSourceRoot) { throw 'VoxPopuli original tracks must remain at their exact read-only provenance root.' }
    if (@($sources.files).Count -ne 5 -or [long](@($sources.files | Measure-Object -Property bytes -Sum)[0].Sum) -ne 769120357) { throw 'VoxPopuli original tracks differ from the five pinned local archive members.' }
    foreach ($entry in @($sources.files)) {
        if ([string]$entry.path -cnotmatch '^202001(?:13|14|15|16|29)-0900-PLENARY_original\.ogg$') { throw 'VoxPopuli source track is outside the approved archive member set.' }
    }
}

function Assert-ProfileFeatureScope([object]$Profile) {
    $personalFeature = 'whisper-worker-parakeet-cpu/parakeet-personal'
    $features = @($Profile.features | ForEach-Object { [string]$_ })
    if ($features -ccontains $personalFeature -and ($Profile.id -cne 'ParakeetCpuContracts' -or $Profile.package -cne 'whisper-worker-parakeet-cpu')) {
        throw "Feature $personalFeature is restricted to ParakeetCpuContracts on whisper-worker-parakeet-cpu."
    }
    if ($Profile.id -ceq 'ParakeetCpuContracts') {
        $expected = @('whisper-worker-parakeet-cpu/native-tests',$personalFeature)
        if ($Profile.package -cne 'whisper-worker-parakeet-cpu' -or $Profile.status -cne 'READY' -or $Profile.operation -cne 'Run' -or $features.Count -ne $expected.Count -or [string]::Join('|',$features) -cne [string]::Join('|',$expected)) {
            throw 'ParakeetCpuContracts must be READY/Run on its package and use only native-tests and parakeet-personal.'
        }
    }
}

function Get-NativeClosurePinName([string]$Id) {
    switch -CaseSensitive ($Id) {
        'NativeClosureCpuReference' { 'reference' }
        'NativeClosureGpuReference' { 'reference' }
        'NativeClosureCpuRebuilt' { 'cpu' }
        'NativeClosureGpuRebuilt' { 'gpu' }
        default { throw 'Unknown native closure profile.' }
    }
}

function Assert-NativeClosureProfile([object]$Profile,[object]$Template) {
    if ($Profile.id -cnotmatch '^NativeClosure') { return }
    $pin=Get-NativeClosurePinName $Profile.id
    if ($Profile.runtimePin -cne "native/runtime-pins/$pin.json") { throw 'Native closure runtime pin path differs.' }
    $gpu=$Profile.id -cmatch '^NativeClosureGpu'
    $expected=@($Template.cases | Where-Object { ($_.exactTest -cmatch '_gpu_') -eq $gpu })
    foreach ($name in @('status','operation','package','features','compileEnvironment','inputs','values','outputs')) {
        if (($Profile.$name | ConvertTo-Json -Depth 8 -Compress) -cne ($Template.$name | ConvertTo-Json -Depth 8 -Compress)) { throw "Native closure descriptor differs: $name" }
    }
    if (($Profile.cases | ConvertTo-Json -Depth 8 -Compress) -cne ($expected | ConvertTo-Json -Depth 8 -Compress)) { throw 'Native closure exact cases differ.' }
    $staging=$Template.staging | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $staging[0].requiredBytes=1
    if (($Profile.staging | ConvertTo-Json -Depth 8 -Compress) -cne ($staging | ConvertTo-Json -Depth 8 -Compress)) { throw 'Native closure staging descriptor differs.' }
}

function Read-NativeClosurePin([string]$Name) {
    if ($Name -cnotin @('reference','cpu','gpu')) { throw 'Unknown native runtime pin.' }
    $relative="native/runtime-pins/$Name.json"
    $path=Join-Path $script:Repo $relative
    Assert-NoReparseChain $path 'persistent native runtime pin'
    $tracked=@(& git.exe --no-optional-locks -C $script:Repo ls-files --error-unmatch -- $relative 2>$null)
    if ($LASTEXITCODE -ne 0 -or $tracked.Count -ne 1) { throw "Persistent runtime pin is missing or unversioned: $relative" }
    & git.exe --no-optional-locks -C $script:Repo diff --quiet HEAD -- $relative
    if ($LASTEXITCODE -ne 0) { throw "Persistent runtime pin differs from HEAD: $relative" }
    $pin=Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    if ($pin.schemaVersion -ne 1 -or $pin.id -cne $Name -or @($pin.files).Count -ne 21) { throw 'Native runtime pin schema/count/id differs.' }
    $map=@{}; $total=[long]0
    foreach ($file in $pin.files) {
        $key=(Get-SafeRelativePath ([string]$file.path) 'native runtime pin').Replace('\','/').ToLowerInvariant()
        if ($map.ContainsKey($key) -or [long]$file.bytes -le 0 -or $file.sha256 -cnotmatch '^[0-9a-f]{64}$') { throw 'Invalid/duplicate native runtime pin file.' }
        $map[$key]=$file; $total+=[long]$file.bytes
    }
    if ([long]$pin.totalBytes -ne $total) { throw 'Native runtime pin total differs.' }
    return [pscustomobject]@{document=$pin;files=$map;totalBytes=$total}
}

function Assert-NativeClosurePayload([object]$pin,[object]$reference,[string]$name,[object[]]$files) {
    if ($name -ne 'reference' -and $pin.document.llamaCommit -cne 'bd4f514db14d87fded667787a7a963bfbaa98e89') { throw 'Rebuilt runtime source commit differs.' }
    $allowed=@('ggml-base.dll','ggml-cpu.dll','ggml.dll','nemo_speech_asr.dll','nemo_speech_asr_c.dll')
    if ($name -eq 'gpu') { $allowed+='ggml-cuda.dll' }
    foreach ($key in $reference.files.Keys) {
        if (-not $pin.files.ContainsKey($key)) { throw 'Native runtime pin file names differ.' }
        $old=$reference.files[$key]; $new=$pin.files[$key]
        $replacement=$name -ne 'reference' -and $key -cin @($allowed | ForEach-Object { "native/$name/$_" })
        if (-not $replacement -and ($old.sha256 -cne $new.sha256 -or $old.bytes -ne $new.bytes)) { throw "Native runtime changed a protected payload: $key" }
    }
    if ($files.Count -ne 21) { throw 'Native runtime input count differs from persistent pin.' }
    foreach ($file in $files) {
        $key=([string]$file.path).Replace('\','/').ToLowerInvariant()
        if (-not $pin.files.ContainsKey($key) -or $pin.files[$key].sha256 -cne $file.sha256 -or $pin.files[$key].bytes -ne $file.bytes) { throw "Native runtime input differs from persistent pin: $key" }
    }
}

function Assert-NativeClosureValues([object]$Profile,[hashtable]$Inputs) {
    if ($Profile.id -cnotmatch '^NativeClosure') { return }
    $name=Get-NativeClosurePinName $Profile.id
    $pin=Read-NativeClosurePin $name; $reference=Read-NativeClosurePin 'reference'
    Assert-NativeClosurePayload $pin $reference $name @($Inputs.personalRuntime.files)
    $Profile.staging[0].requiredBytes=$pin.totalBytes
}

function Get-ProfileRegistry {
    if (-not (Test-Path -LiteralPath $script:ProfilesPath -PathType Leaf)) { throw "Profile registry is missing: $script:ProfilesPath" }
    Assert-NoReparseChain $script:ProfilesPath 'profile registry'
    try { $registry = Get-Content -LiteralPath $script:ProfilesPath -Raw | ConvertFrom-Json -ErrorAction Stop }
    catch { throw "Profile registry JSON is invalid: $($_.Exception.Message)" }
    if ($registry.schemaVersion -ne 1) { throw 'Unsupported native profile registry schema.' }
    if ([IO.Path]::GetFullPath($registry.sharedCargoTarget).TrimEnd('\') -ine $script:CargoTarget) {
        throw 'Profile registry shared Cargo target differs from the accepted canonical path.'
    }
    $packageSet = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($package in @($registry.allowedPackages)) {
        if ($package -notmatch '^whisper-[a-z0-9-]+$' -or -not $packageSet.Add([string]$package)) { throw "Invalid or duplicate package in profile registry: $package" }
    }
    $featureSet = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($feature in @($registry.allowedFeatures)) {
        if ($feature -notmatch '^(?:native-tests|whisper-[a-z0-9-]+/(?:native-tests|recette-hooks)|whisper-worker-parakeet-cpu/parakeet-personal)$' -or -not $featureSet.Add([string]$feature)) {
            throw "Invalid or duplicate Cargo feature in profile registry: $feature"
        }
    }
    $profileSet = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($profile in @($registry.profiles)) {
        if ($profile.id -notmatch '^[A-Za-z][A-Za-z0-9]{0,47}$' -or -not $profileSet.Add([string]$profile.id)) { throw "Invalid or duplicate profile id: $($profile.id)" }
        if ($profile.operation -notin @('NoRun','List','Run','Diagnostic')) { throw "Unsupported operation for profile $($profile.id)." }
        if ($profile.status -notin @('READY','DISCOVERY_PENDING')) { throw "Unsupported status for profile $($profile.id)." }
        if ($profile.package -cne 'workspace' -and -not $packageSet.Contains([string]$profile.package)) { throw "Unapproved package for profile $($profile.id): $($profile.package)" }
        foreach ($feature in @($profile.features)) { if (-not $featureSet.Contains([string]$feature)) { throw "Unapproved feature '$feature' in profile $($profile.id)." } }
        Assert-ProfileFeatureScope $profile
        Assert-VoxPopuliProfileContract $profile
        Assert-NativeClosureProfile $profile (@($registry.profiles | Where-Object id -CEQ 'PersonalUnified')[0])
        foreach ($entry in @($profile.compileEnvironment)) {
            $name = [string]$entry.name
            if (-not $script:AllowedCompileEnvironment.ContainsKey($name) -or [string]$entry.value -cne $script:AllowedCompileEnvironment[$name]) {
                throw "Compile environment flag is not authorized for profile $($profile.id): $name"
            }
        }
        foreach ($asset in @($profile.inputs)) {
            if ($asset.key -notmatch '^[A-Za-z][A-Za-z0-9_-]{0,63}$' -or $asset.kind -notin @('file','directory')) { throw "Invalid input descriptor in profile $($profile.id)." }
            if ([string]$asset.environmentVariable -notmatch '^(?:WHISPER_|PARAKEET_)[A-Z0-9_]+$') { throw "Invalid input environment variable in profile $($profile.id)." }
        }
        foreach ($value in @($profile.values)) {
            if ($value.key -notmatch '^[A-Za-z][A-Za-z0-9_-]{0,63}$' -or ([string]$value.environmentVariable -notmatch '^(?:WHISPER_|PARAKEET_)[A-Z0-9_]+$' -and -not $script:AllowedProfileEnvironmentNames.Contains([string]$value.environmentVariable)) -or $value.kind -notin @('literal','directory')) {
                throw "Invalid literal-value descriptor in profile $($profile.id)."
            }
            if ($value.kind -eq 'directory' -and $value.allowedRoot -cne 'cuda-12.9') { throw "Directory profile value has no approved tool root in $($profile.id)." }
        }
        foreach ($output in @($profile.outputs)) {
            $outputKind = if ($output.PSObject.Properties.Name -contains 'kind') { [string]$output.kind } else { 'directory' }
            $hasOutputPath = $output.PSObject.Properties.Name -contains 'path'
            if ($output.key -notmatch '^[A-Za-z][A-Za-z0-9_-]{0,63}$' -or [string]$output.environmentVariable -notmatch '^(?:WHISPER_|PARAKEET_)[A-Z0-9_]+$' -or $outputKind -notin @('file','directory') -or ($outputKind -eq 'file' -and -not $hasOutputPath)) {
                throw "Invalid output descriptor in profile $($profile.id)."
            }
            if ($hasOutputPath) { [void](Get-SafeRelativePath ([string]$output.path) "Output path in profile $($profile.id)") }
        }
        $stagedInputKeys = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        $stagingDestinations = [Collections.Generic.List[string]]::new()
        foreach ($staging in @($profile.staging | Where-Object { $null -ne $_ })) {
            $inputKey = [string]$staging.inputKey
            $inputDescriptor = @($profile.inputs | Where-Object { [string]$_.key -ceq $inputKey })
            if ($profile.status -cne 'READY' -or $profile.operation -cne 'Run' -or $inputKey -notmatch '^[A-Za-z][A-Za-z0-9_-]{0,63}$' -or $inputDescriptor.Count -ne 1 -or $inputDescriptor[0].kind -notin @('file','directory') -or -not $stagedInputKeys.Add($inputKey)) {
                throw "Invalid or duplicate staging input in profile $($profile.id)."
            }
            if ($staging.PSObject.Properties.Name -notcontains 'requiredFileCount' -or $staging.PSObject.Properties.Name -notcontains 'requiredBytes' -or [int]$staging.requiredFileCount -lt 1 -or [int]$staging.requiredFileCount -gt 256 -or [long]$staging.requiredBytes -lt 1) {
                throw "Staging file count/byte pins are missing or invalid in profile $($profile.id)."
            }
            $destination = Get-SafeRelativePath ([string]$staging.destination) "Staging destination in profile $($profile.id)"
            $destinationBase = if ($staging.PSObject.Properties.Name -contains 'destinationBase') { [string]$staging.destinationBase } else { 'localappdata' }
            if ($destinationBase -notin @('localappdata','run')) { throw "Unsupported staging destination base in profile $($profile.id): $destinationBase" }
            $qualifiedDestination = $destinationBase + '\' + $destination
            foreach ($priorDestination in $stagingDestinations) {
                if ($qualifiedDestination.Equals($priorDestination,[StringComparison]::OrdinalIgnoreCase) -or $qualifiedDestination.StartsWith($priorDestination + '\',[StringComparison]::OrdinalIgnoreCase) -or $priorDestination.StartsWith($qualifiedDestination + '\',[StringComparison]::OrdinalIgnoreCase)) {
                    throw "Overlapping staging destinations in profile $($profile.id): $qualifiedDestination and $priorDestination"
                }
            }
            $stagingDestinations.Add($qualifiedDestination)
        }
        $caseNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        foreach ($case in @($profile.cases)) {
            if ($case.targetKind -notin @('lib','integration','bin')) {
                throw "Invalid exact test selector in profile $($profile.id)."
            }
            Assert-ExactTestSelector ([string]$case.exactTest)
            if (-not $caseNames.Add([string]$case.exactTest)) { throw "Duplicate exact test selector in profile $($profile.id): $($case.exactTest)" }
            if ($case.ignored -and $profile.id -cne 'ManualDiagnostic') { throw "Only ManualDiagnostic may select an ignored case: $($case.exactTest)" }
            if ($case.verdict -notin @('QUALIFICATION','DIAGNOSTIC_ONLY')) { throw "Invalid verdict class in profile $($profile.id)." }
            if ($profile.id -ceq 'ManualDiagnostic' -and $case.verdict -cne 'DIAGNOSTIC_ONLY') { throw 'ManualDiagnostic cannot be reported as a qualification.' }
        }
        if ($profile.status -ceq 'READY' -and $profile.operation -eq 'Run' -and @($profile.cases).Count -eq 0) { throw "Runnable profile $($profile.id) has no exact selectors." }
        if ($profile.operation -ceq 'Diagnostic') {
            Assert-NvidiaSmiDiagnosticProfile $profile
            if (@($profile.features).Count -ne 0 -or @($profile.compileEnvironment).Count -ne 0 -or @($profile.cases).Count -ne 0 -or @($profile.inputs).Count -ne 0 -or @($profile.outputs).Count -ne 0) { throw 'NVIDIA diagnostic profile cannot declare Cargo features, compile variables, selectors, inputs, or outputs.' }
        }
        if ($profile.status -ceq 'DISCOVERY_PENDING' -and @($profile.cases).Count -gt 0) { throw "Pending profile $($profile.id) must not contain provisional selectors." }
    }
    return $registry
}

function Get-Profile([object]$Registry, [string]$Id) {
    if ([string]::IsNullOrWhiteSpace($Id)) { throw '-Profile is required for this mode.' }
    $matches = @($Registry.profiles | Where-Object { $_.id -ceq $Id })
    if ($matches.Count -ne 1) { throw "Unknown native profile: $Id" }
    if ($matches[0].status -cne 'READY') { throw "Native profile $Id is pending exact test discovery; it cannot run yet." }
    return $matches[0]
}

function Assert-NvidiaSmiDiagnosticProfile([object]$Profile, [switch]$VerifyInstalledExecutable) {
    $expectedExe = Get-FullPath (Join-Path $env:SystemRoot 'System32\nvidia-smi.exe')
    $configuredExe = Get-FullPath ([string]$Profile.command.executable)
    $expectedArgs = @('--query-gpu=count,pci.bus_id,uuid','--format=csv,noheader,nounits')
    if ($Profile.id -notin @('NvidiaSmiInventoryDiagnostic','NvidiaSmiEnvironmentCompareDiagnostic','NvidiaProgramW6432Witness') -or $configuredExe -ine $expectedExe) { throw 'NVIDIA diagnostic executable must be the pinned System32 nvidia-smi.exe.' }
    $expectedCommandProperties = if ($Profile.id -ceq 'NvidiaSmiEnvironmentCompareDiagnostic') { @('executable','arguments','contexts','captureModules','expectedSha256') } else { @('executable','arguments') }
    $actualCommandProperties = @($Profile.command.PSObject.Properties.Name | Sort-Object)
    $sortedExpectedCommandProperties = @($expectedCommandProperties | Sort-Object)
    if ([string]::Join('|',$actualCommandProperties) -cne [string]::Join('|',$sortedExpectedCommandProperties)) { throw "NVIDIA diagnostic command has fields outside its pinned contract: $($Profile.id)" }
    if ($VerifyInstalledExecutable) {
        if (-not (Test-Path -LiteralPath $expectedExe -PathType Leaf)) { throw "Pinned System32 nvidia-smi.exe is missing: $expectedExe" }
        $version = [Diagnostics.FileVersionInfo]::GetVersionInfo($expectedExe).FileVersion
        if ($version -cne '8.17.15.8129') { throw "Pinned System32 nvidia-smi.exe version differs from 8.17.15.8129: $version" }
    }
    $actualArgs = @($Profile.command.arguments | ForEach-Object { [string]$_ })
    if ($actualArgs.Count -ne $expectedArgs.Count -or [string]::Join("`n",$actualArgs) -cne [string]::Join("`n",$expectedArgs)) { throw 'NVIDIA diagnostic argv differs from the exact source command.' }
    if ($Profile.id -ceq 'NvidiaSmiEnvironmentCompareDiagnostic') {
        $expectedContexts = @('clean','host-nvidia-filtered','path-only','vars-only')
        $actualContexts = @($Profile.command.contexts | ForEach-Object { [string]$_ })
        if (-not [bool]$Profile.command.captureModules -or $actualContexts.Count -ne $expectedContexts.Count -or [string]::Join('|',$actualContexts) -cne [string]::Join('|',$expectedContexts) -or [string]$Profile.command.expectedSha256 -cne 'f874229e4df6313f39b4af658add6d2b74682f2a44b5957aa6104d19a8593ec6') { throw 'NVIDIA environment comparison contexts, module capture flag, or executable SHA pin differ from the bounded profile.' }
        if (@($Profile.features).Count -ne 0 -or @($Profile.compileEnvironment).Count -ne 0 -or @($Profile.inputs).Count -ne 0 -or @($Profile.outputs).Count -ne 0 -or @($Profile.cases).Count -ne 0) { throw 'NVIDIA environment comparison must not declare Cargo features, inputs, outputs, compile flags, or test selectors.' }
        if ($VerifyInstalledExecutable -and (Get-Sha256 $expectedExe) -cne [string]$Profile.command.expectedSha256) { throw 'Pinned System32 nvidia-smi.exe SHA-256 differs from the approved comparison executable.' }
    }
    $expectedValues = @(
        [pscustomobject]@{key='cudaToolkitPath';environmentVariable='CUDA_PATH';kind='directory';allowedRoot='cuda-12.9'},
        [pscustomobject]@{key='cudaToolkitVersionedPath';environmentVariable='CUDA_PATH_V12_9';kind='directory';allowedRoot='cuda-12.9'}
    )
    $actualValues = @($Profile.values)
    if ($actualValues.Count -ne $expectedValues.Count) { throw 'NVIDIA diagnostic must declare only the two bounded CUDA 12.9 paths.' }
    for ($index = 0; $index -lt $expectedValues.Count; $index++) {
        foreach ($name in @('key','environmentVariable','kind','allowedRoot')) {
            if ([string]$actualValues[$index].$name -cne [string]$expectedValues[$index].$name) { throw 'NVIDIA diagnostic CUDA environment descriptors differ from the bounded CUDA 12.9 contract.' }
        }
        if (-not [bool]$actualValues[$index].required) { throw 'NVIDIA diagnostic CUDA path descriptors must be required.' }
    }
}

function Get-NvidiaDiagnosticHostEnvironment {
    $environment = @{}
    foreach ($name in @('SystemRoot','WINDIR','SystemDrive','ComSpec','PATHEXT','OS','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS','PATH','CUDA_PATH','CUDA_PATH_V12_9')) {
        $value = [Environment]::GetEnvironmentVariable($name,'Process')
        if (-not [string]::IsNullOrWhiteSpace($value)) { $environment[$name] = $value }
    }
    foreach ($name in @('ProgramFiles','ProgramFiles(x86)','ProgramW6432','CommonProgramFiles','CommonProgramFiles(x86)','CommonProgramW6432')) {
        $value = [Environment]::GetEnvironmentVariable($name,'Machine')
        if (-not [string]::IsNullOrWhiteSpace($value)) { $environment[$name] = $value }
    }
    return $environment
}

function Get-NvidiaDiagnosticFilteredPath([hashtable]$HostEnvironment, [string]$CudaBin, [string]$NvSmiDirectory, [bool]$IncludeHostDirectories) {
    $systemRoot = Get-FullPath ([string]$HostEnvironment.SystemRoot)
    $system32 = Get-FullPath (Join-Path $systemRoot 'System32')
    Assert-NoReparseChain $systemRoot 'diagnostic SystemRoot PATH entry'
    Assert-NoReparseChain $system32 'diagnostic System32 PATH entry'
    $entries = [Collections.Generic.List[string]]::new()
    $included = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($path in @($system32,$systemRoot)) { if ($included.Add($path)) { $entries.Add($path) } }
    $includedHost = [Collections.Generic.List[string]]::new()
    $omittedHost = 0
    $invalidHost = 0
    $reparseRejected = 0
    if ($IncludeHostDirectories) {
        $allowedVendorPaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        foreach ($path in @($CudaBin,$NvSmiDirectory)) {
            if (-not [string]::IsNullOrWhiteSpace($path) -and (Test-Path -LiteralPath $path -PathType Container)) {
                try { Assert-NoReparseChain $path 'approved NVIDIA diagnostic PATH directory'; [void]$allowedVendorPaths.Add((Get-FullPath $path)) }
                catch { $reparseRejected++ }
            }
        }
        $hostPath = [string]$HostEnvironment.PATH
        foreach ($part in $hostPath.Split(';')) {
            if ([string]::IsNullOrWhiteSpace($part)) { continue }
            if ($part.Contains('"') -or $part.Contains('%') -or -not [IO.Path]::IsPathRooted($part)) { $invalidHost++; continue }
            try { $full = Get-FullPath $part }
            catch { $invalidHost++; continue }
            if ($full -cne $part -or -not (Test-Path -LiteralPath $full -PathType Container)) { $invalidHost++; continue }
            if ($full -ieq $systemRoot -or $full -ieq $system32) { continue }
            if (-not $allowedVendorPaths.Contains($full)) { $omittedHost++; continue }
            try { Assert-NoReparseChain $full 'filtered NVIDIA diagnostic PATH entry' }
            catch { $reparseRejected++; continue }
            if ($included.Add($full)) { $entries.Add($full); $includedHost.Add($full) }
        }
    }
    return [pscustomobject]@{
        value=[string]::Join(';',$entries.ToArray())
        includedDirectories=@($entries.ToArray())
        includedHostVendorDirectories=@($includedHost.ToArray())
        omittedHostEntryCount=$omittedHost
        invalidHostEntryCount=$invalidHost
        reparseRejectedCount=$reparseRejected
    }
}

function New-NvidiaDiagnosticContextEnvironment([string]$ContextId, [hashtable]$BaseEnvironment, [hashtable]$HostEnvironment, [string]$RunRoot) {
    $contextPolicies = @{
        clean = [pscustomobject]@{ includeHostPaths=$false; includeCudaVariables=$false }
        'host-nvidia-filtered' = [pscustomobject]@{ includeHostPaths=$true; includeCudaVariables=$true }
        'path-only' = [pscustomobject]@{ includeHostPaths=$true; includeCudaVariables=$false }
        'vars-only' = [pscustomobject]@{ includeHostPaths=$false; includeCudaVariables=$true }
    }
    if (-not $contextPolicies.ContainsKey($ContextId)) { throw "Unapproved NVIDIA environment comparison context: $ContextId" }
    $policy = $contextPolicies[$ContextId]
    $runRoot = Get-FullPath $RunRoot
    if (-not (Test-InRoot $runRoot $script:Lot)) { throw "Diagnostic context root escaped the candidate lot: $runRoot" }
    Assert-NoReparseChain $runRoot 'diagnostic context RunRoot'
    $cudaPath = Get-FullPath ([string]$BaseEnvironment.CUDA_PATH)
    $versionedCudaPath = Get-FullPath ([string]$BaseEnvironment.CUDA_PATH_V12_9)
    if ($cudaPath -ine $versionedCudaPath -or -not (Test-Path -LiteralPath $cudaPath -PathType Container)) { throw 'NVIDIA comparison requires the same existing CUDA 12.9 root for CUDA_PATH and CUDA_PATH_V12_9.' }
    Assert-NoReparseChain $cudaPath 'validated CUDA 12.9 comparison root'
    $validatedProgramW = Get-ValidatedProgramW6432Directory
    if (-not $BaseEnvironment.ContainsKey('ProgramW6432') -or [string]$BaseEnvironment['ProgramW6432'] -ine $validatedProgramW) { throw 'NVIDIA comparison base environment omitted the validated ProgramW6432 directory.' }
    $cudaBin = Get-FullPath (Join-Path $cudaPath 'bin')
    $programFiles = ''
    $programFilesValue = [string]$HostEnvironment.ProgramFiles
    if (-not [string]::IsNullOrWhiteSpace($programFilesValue) -and [IO.Path]::IsPathRooted($programFilesValue)) {
        try {
            $candidateProgramFiles = Get-FullPath $programFilesValue
            if ($candidateProgramFiles -ieq $programFilesValue.TrimEnd('\') -and (Test-Path -LiteralPath $candidateProgramFiles -PathType Container)) { Assert-NoReparseChain $candidateProgramFiles 'machine ProgramFiles directory'; $programFiles=$candidateProgramFiles }
        } catch { $programFiles='' }
    }
    $nvSmiDirectory = if (-not [string]::IsNullOrWhiteSpace($programFiles)) { Get-FullPath (Join-Path $programFiles 'NVIDIA Corporation\NVSMI') } else { '' }
    $nvSmiDirectoryAvailable = -not [string]::IsNullOrWhiteSpace($nvSmiDirectory) -and (Test-Path -LiteralPath $nvSmiDirectory -PathType Container)
    if ($nvSmiDirectoryAvailable) { Assert-NoReparseChain $nvSmiDirectory 'NVIDIA System Management Interface PATH directory' } else { $nvSmiDirectory = '' }
    $pathEvidence = Get-NvidiaDiagnosticFilteredPath $HostEnvironment $cudaBin $nvSmiDirectory ([bool]$policy.includeHostPaths)

    $environment = @{}
    $systemRoot = Get-FullPath ([string]$HostEnvironment.SystemRoot)
    $systemDrive = [IO.Path]::GetPathRoot($systemRoot).TrimEnd('\')
    $comSpec = Get-FullPath (Join-Path $systemRoot 'System32\cmd.exe')
    if (-not (Test-Path -LiteralPath $comSpec -PathType Leaf)) { throw 'Windows ComSpec executable is missing from the pinned System32 root.' }
    Assert-NoReparseChain $comSpec 'diagnostic ComSpec'
    $environment.SystemRoot = $systemRoot
    $environment.WINDIR = $systemRoot
    $environment.SystemDrive = $systemDrive
    $environment.ComSpec = $comSpec
    $readOnlyEnvironmentNames = @('PATHEXT','OS','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS')
    foreach ($name in $readOnlyEnvironmentNames) {
        $value = [string]$HostEnvironment[$name]
        if (-not [string]::IsNullOrWhiteSpace($value)) { $environment[$name] = $value }
    }
    if (-not [string]::IsNullOrWhiteSpace($programFiles)) { $environment.ProgramFiles=$programFiles }
    $systemNames = @('SystemRoot','WINDIR','SystemDrive','ComSpec','ProgramFiles','ProgramW6432') + $readOnlyEnvironmentNames
    $environment['PATH'] = $pathEvidence.value
    $runDrive = [IO.Path]::GetPathRoot($runRoot).TrimEnd('\')
    $homePath = $runRoot.Substring($runDrive.Length)
    $programData = Join-Path $runRoot 'programdata'
    $publicData = Join-Path $runRoot 'public'
    $environment['TEMP'] = Join-Path $runRoot 'temp'
    $environment['TMP'] = Join-Path $runRoot 'tmp'
    $environment['LOCALAPPDATA'] = Join-Path $runRoot 'localappdata'
    $environment['APPDATA'] = Join-Path $runRoot 'appdata'
    $environment['USERPROFILE'] = $runRoot
    $environment['HOME'] = $runRoot
    $environment['HOMEDRIVE'] = $runDrive
    $environment['HOMEPATH'] = $homePath
    $environment['PROGRAMDATA'] = $programData
    $environment['ALLUSERSPROFILE'] = $programData
    $environment['PUBLIC'] = $publicData
    foreach ($name in @('CARGO_HOME','CARGO_TARGET_DIR','CARGO_INCREMENTAL','CARGO_PROFILE_DEV_DEBUG')) {
        if ($BaseEnvironment.ContainsKey($name)) { $environment[$name] = [string]$BaseEnvironment[$name] }
    }
    $environment['ProgramW6432'] = $validatedProgramW
    $cudaVariableEvidence = [Collections.Generic.List[object]]::new()
    foreach ($name in @('CUDA_PATH','CUDA_PATH_V12_9')) {
        $hostValue = [string]$HostEnvironment[$name]
        if ([string]::IsNullOrWhiteSpace($hostValue)) {
            $cudaVariableEvidence.Add([pscustomobject]@{name=$name;status='NOT_SET'})
            continue
        }
        $valid = $false
        $normalized = $null
        try {
            $normalized = Get-FullPath $hostValue
            if ($normalized -ieq $cudaPath -and (Test-Path -LiteralPath $normalized -PathType Container)) { Assert-NoReparseChain $normalized 'filtered host CUDA variable'; $valid = $true }
        } catch { $valid = $false }
        if ($valid -and [bool]$policy.includeCudaVariables) {
            $environment[$name] = $normalized
            $cudaVariableEvidence.Add([pscustomobject]@{name=$name;status='INCLUDED_VALIDATED';path=$normalized})
        } elseif ($valid) {
            $cudaVariableEvidence.Add([pscustomobject]@{name=$name;status='VALIDATED_NOT_PASSED'})
        } else {
            $cudaVariableEvidence.Add([pscustomobject]@{name=$name;status='FILTERED_NONCANONICAL_OR_OUTSIDE_CUDA_12_9'})
        }
    }
    foreach ($name in @('CARGO_HOME','TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME','HOMEDRIVE','HOMEPATH','PROGRAMDATA','ALLUSERSPROFILE','PUBLIC')) {
        if ([string]::IsNullOrWhiteSpace([string]$environment[$name])) { throw "Diagnostic context omitted required isolated environment root $name." }
    }
    if (-not (Test-InRoot $environment.CARGO_HOME $script:Lot) -or (Get-FullPath $environment.CARGO_TARGET_DIR) -ine $script:CargoTarget) { throw 'NVIDIA diagnostic CARGO roots differ from the approved lot/shared target.' }
    Assert-NoReparseChain $environment.CARGO_HOME 'diagnostic CARGO_HOME'
    Assert-NoReparseChain $environment.CARGO_TARGET_DIR 'diagnostic CARGO_TARGET_DIR'
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME','PROGRAMDATA','ALLUSERSPROFILE','PUBLIC')) {
        if (-not (Test-InRoot $environment[$name] $runRoot)) { throw "$name escaped the isolated diagnostic context root: $($environment[$name])" }
        Assert-NoReparseChain $environment[$name] "diagnostic context $name"
    }
    if (($environment.HOMEDRIVE + $environment.HOMEPATH).TrimEnd('\') -ine $runRoot) { throw 'HOMEDRIVE/HOMEPATH do not resolve to the isolated diagnostic context root.' }
    return [pscustomobject]@{
        environment=$environment
        evidence=[pscustomobject]@{
            context=$ContextId
            inheritedEnvironmentNames=@($systemNames | Where-Object { $environment.ContainsKey($_) })
            cudaVariables=@($cudaVariableEvidence.ToArray())
            filteredPathDirectories=@($pathEvidence.includedDirectories)
            includedHostVendorDirectories=@($pathEvidence.includedHostVendorDirectories)
            omittedHostPathEntryCount=[int]$pathEvidence.omittedHostEntryCount
            invalidHostPathEntryCount=[int]$pathEvidence.invalidHostEntryCount
            reparsePathEntryRejectedCount=[int]$pathEvidence.reparseRejectedCount
            writableEnvironmentRoots=@('CARGO_HOME','CARGO_TARGET_DIR','TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME','HOMEDRIVE','HOMEPATH','PROGRAMDATA','ALLUSERSPROFILE','PUBLIC')
            environmentDumpWritten=$false
        }
    }
}

function New-NvidiaDiagnosticProcessInfo([string]$ProfileId, [string]$Executable, [string[]]$Arguments, [hashtable]$Environment, [string]$RunRoot) {
    if ($ProfileId -cne 'NvidiaSmiEnvironmentCompareDiagnostic') { throw "Profile $ProfileId must use its Cargo-backed ProcessStartInfo builder." }
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = $Executable
    $psi.WorkingDirectory = $script:Repo
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Arguments = [string]::Join(' ',[string[]]@($Arguments | ForEach-Object { Get-WindowsCommandLineArgument ([string]$_) }))
    $vars = $psi.EnvironmentVariables
    $vars.Clear()
    foreach ($entry in $Environment.GetEnumerator()) { $vars[[string]$entry.Key] = [string]$entry.Value }
    $programWAllowed = Test-ProgramW6432ProfileId $ProfileId
    if ($programWAllowed) {
        $validatedProgramW = Get-ValidatedProgramW6432Directory
        if (-not $Environment.ContainsKey('ProgramW6432') -or [string]$Environment['ProgramW6432'] -ine $validatedProgramW -or [string]$vars['ProgramW6432'] -ine $validatedProgramW) {
            throw 'NVIDIA/GPU profile ProgramW6432 is missing or differs from the validated host directory.'
        }
    } elseif ($Environment.ContainsKey('ProgramW6432') -or $vars.ContainsKey('ProgramW6432')) {
        throw "Profile $ProfileId is not authorized to receive ProgramW6432."
    }
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME','PROGRAMDATA','ALLUSERSPROFILE','PUBLIC')) {
        if (-not (Test-InRoot ([string]$Environment[$name]) $RunRoot)) { throw "NVIDIA diagnostic process root $name escaped its context RunRoot." }
    }
    $declaredCargoHome = [string]$Environment['CARGO_HOME']
    $declaredCargoTarget = [string]$Environment['CARGO_TARGET_DIR']
    $canonicalCargoHome = $null
    $canonicalCargoTarget = $null
    $canonicalLot = Get-FullPath $script:Lot
    $canonicalSharedTarget = Get-FullPath $script:CargoTarget
    try { $canonicalCargoHome = Get-FullPath $declaredCargoHome } catch { }
    try { $canonicalCargoTarget = Get-FullPath $declaredCargoTarget } catch { }
    $declaredCargoHomeInLot = $false
    $declaredCargoTargetMatches = $false
    if ($null -ne $canonicalCargoHome) { $declaredCargoHomeInLot = Test-InRoot $canonicalCargoHome $canonicalLot }
    if ($null -ne $canonicalCargoTarget) { $declaredCargoTargetMatches = $canonicalCargoTarget -ieq $canonicalSharedTarget }
    $copiedCargoHome = [string]$vars['CARGO_HOME']
    $copiedCargoTarget = [string]$vars['CARGO_TARGET_DIR']
    $canonicalCopiedCargoHome = $null
    $canonicalCopiedCargoTarget = $null
    try { $canonicalCopiedCargoHome = Get-FullPath $copiedCargoHome } catch { }
    try { $canonicalCopiedCargoTarget = Get-FullPath $copiedCargoTarget } catch { }
    $copiedCargoHomeInLot = $false
    $copiedCargoTargetMatches = $false
    if ($null -ne $canonicalCopiedCargoHome) { $copiedCargoHomeInLot = Test-InRoot $canonicalCopiedCargoHome $canonicalLot }
    if ($null -ne $canonicalCopiedCargoTarget) { $copiedCargoTargetMatches = $canonicalCopiedCargoTarget -ieq $canonicalSharedTarget }
    if (-not $declaredCargoHomeInLot -or -not $declaredCargoTargetMatches -or -not $copiedCargoHomeInLot -or -not $copiedCargoTargetMatches) {
        throw "NVIDIA diagnostic process Cargo root validation failed before child start: declaredCARGO_HOME='$declaredCargoHome' canonical='$canonicalCargoHome' lot='$canonicalLot' inLot=$declaredCargoHomeInLot; declaredCARGO_TARGET_DIR='$declaredCargoTarget' canonical='$canonicalCargoTarget' sharedTarget='$canonicalSharedTarget' matches=$declaredCargoTargetMatches; copiedCARGO_HOME='$copiedCargoHome' canonical='$canonicalCopiedCargoHome' inLot=$copiedCargoHomeInLot; copiedCARGO_TARGET_DIR='$copiedCargoTarget' canonical='$canonicalCopiedCargoTarget' matches=$copiedCargoTargetMatches."
    }
    return $psi
}

function New-NvidiaSmiInventoryProcessInfo([object]$Profile, [hashtable]$Environment, [string]$RunRoot) {
    if ($Profile.id -cne 'NvidiaSmiInventoryDiagnostic') { throw 'Cargo-backed NVIDIA inventory builder requires the exact inventory profile.' }
    Assert-NvidiaSmiDiagnosticProfile $Profile -VerifyInstalledExecutable
    if (Test-Path -LiteralPath $RunRoot) { throw 'NVIDIA inventory preflight RunRoot unexpectedly exists.' }
    $previousRunRoot = $script:RunRoot
    try {
        $script:RunRoot = $RunRoot
        New-Item -ItemType Directory -Path $RunRoot,(Join-Path $RunRoot 'temp'),(Join-Path $RunRoot 'tmp'),(Join-Path $RunRoot 'localappdata'),(Join-Path $RunRoot 'appdata') -Force -ErrorAction Stop | Out-Null
        $toolchain = Get-CargoExecutable
        $vsEnvironment = Get-VsEnvironment (Get-VsDevCommand) $Environment.TEMP $Environment.TMP $RunRoot
        $psi = New-CargoProcessInfo $toolchain.cargo $toolchain.rustc $toolchain.toolchainBin @() $Environment $vsEnvironment $RunRoot $Profile.id
        $psi.FileName = Get-FullPath (Join-Path $env:SystemRoot 'System32\nvidia-smi.exe')
        $psi.Arguments = [string]::Join(' ',[string[]]@($Profile.command.arguments | ForEach-Object { Get-WindowsCommandLineArgument ([string]$_) }))
        return $psi
    } finally {
        $script:RunRoot = $previousRunRoot
        if (Test-InRoot $RunRoot $script:Lot -and (Test-Path -LiteralPath $RunRoot)) { Remove-Item -LiteralPath $RunRoot -Recurse -Force -ErrorAction SilentlyContinue }
    }
}

function New-NvidiaModuleCapture([int]$ProcessId) {
    return [pscustomobject]@{
        processId=$ProcessId
        method='System.Diagnostics.Process.Modules polling'
        targetPollMilliseconds=20
        maxSamples=1600
        maxModulePaths=4096
        maxCandidateMetadata=128
        samples=[Collections.Generic.List[object]]::new()
        modules=[Collections.Generic.Dictionary[string,object]]::new([StringComparer]::OrdinalIgnoreCase)
        errors=[Collections.Generic.List[string]]::new()
        sampleCount=0
        errorCount=0
        modulePathTruncated=$false
        sampleTruncated=$false
        processExitedBeforeFirstSample=$false
        firstSampleUtc=$null
        lastSampleUtc=$null
    }
}

function Add-NvidiaModuleCaptureError([object]$Capture, [string]$Message) {
    $Capture.errorCount = [int]$Capture.errorCount + 1
    if ($Capture.errors.Count -lt 64) { $Capture.errors.Add($Message) }
}

function Capture-NvidiaProcessModules([Diagnostics.Process]$Process, [object]$Capture) {
    if ($Capture.sampleCount -ge $Capture.maxSamples) { $Capture.sampleTruncated=$true; return }
    $sampleAt = [DateTimeOffset]::UtcNow
    try {
        $Process.Refresh()
        if ($Process.HasExited) {
            if ($Capture.sampleCount -eq 0) { $Capture.processExitedBeforeFirstSample=$true }
            return
        }
        $moduleCollection = $Process.Modules
        $moduleCount = [int]$moduleCollection.Count
        $nvidiaNames = [Collections.Generic.List[string]]::new()
        $sampleTruncated = $moduleCount -gt $Capture.maxModulePaths
        $processed = 0
        foreach ($module in $moduleCollection) {
            if ($processed -ge $Capture.maxModulePaths) { break }
            $processed++
            try {
                $name = [string]$module.ModuleName
                $path = Get-FullPath ([string]$module.FileName)
                if ([string]::IsNullOrWhiteSpace($name) -or [string]::IsNullOrWhiteSpace($path)) { throw 'Process.Modules returned an empty module name or path.' }
                $key = $path.ToLowerInvariant()
                if ($Capture.modules.ContainsKey($key)) {
                    $prior = $Capture.modules[$key]
                    $prior.lastObservedUtc = $sampleAt.ToString('o')
                    $prior.observedSamples = [int]$prior.observedSamples + 1
                } elseif ($Capture.modules.Count -lt $Capture.maxModulePaths) {
                    $Capture.modules.Add($key,[pscustomobject]@{name=$name;path=$path;firstObservedUtc=$sampleAt.ToString('o');lastObservedUtc=$sampleAt.ToString('o');observedSamples=1})
                } else { $Capture.modulePathTruncated=$true }
                if ($name -match '(?i)^(?:nv|nvidia|cuda|cublas|cudnn|cufft|curand|cusparse|ggml-cuda).*\.dll$' -and $nvidiaNames.Count -lt $Capture.maxCandidateMetadata) { $nvidiaNames.Add($name) }
            } catch { Add-NvidiaModuleCaptureError $Capture "module read: $($_.Exception.Message)" }
        }
        if ($moduleCount -gt $Capture.maxModulePaths) { $sampleTruncated=$true; $Capture.modulePathTruncated=$true }
        $Capture.sampleCount = [int]$Capture.sampleCount + 1
        if ($null -eq $Capture.firstSampleUtc) { $Capture.firstSampleUtc=$sampleAt.ToString('o') }
        $Capture.lastSampleUtc=$sampleAt.ToString('o')
        $Capture.samples.Add([pscustomobject]@{atUtc=$sampleAt.ToString('o');moduleCount=$moduleCount;modulePathsEnumerated=$processed;nvidiaModuleNames=@($nvidiaNames.ToArray());truncated=$sampleTruncated})
    } catch { Add-NvidiaModuleCaptureError $Capture "module snapshot: $($_.Exception.Message)" }
}

function Complete-NvidiaModuleCapture([object]$Capture) {
    $metadata = [Collections.Generic.List[object]]::new()
    $candidates = @($Capture.modules.Values | Where-Object { $_.name -match '(?i)^(?:nv|nvidia|cuda|cublas|cudnn|cufft|curand|cusparse|ggml-cuda).*\.dll$' } | Sort-Object -Property path)
    $candidateTruncated = $candidates.Count -gt $Capture.maxCandidateMetadata
    foreach ($module in @($candidates | Select-Object -First $Capture.maxCandidateMetadata)) {
        $fileVersion = $null; $sha256 = $null; $bytes = $null; $metadataError = $null
        try {
            if (-not (Test-Path -LiteralPath $module.path -PathType Leaf)) { throw 'Observed module file no longer exists after process exit.' }
            $fileVersion = [Diagnostics.FileVersionInfo]::GetVersionInfo([string]$module.path).FileVersion
            $bytes = [long](Get-Item -LiteralPath $module.path -Force).Length
            $sha256 = Get-Sha256 ([string]$module.path)
        } catch { $metadataError = $_.Exception.Message; Add-NvidiaModuleCaptureError $Capture "module metadata $($module.name): $metadataError" }
        $metadata.Add([pscustomobject]@{name=$module.name;path=$module.path;fileVersion=$fileVersion;bytes=$bytes;sha256=$sha256;metadataError=$metadataError;firstObservedUtc=$module.firstObservedUtc;lastObservedUtc=$module.lastObservedUtc;observedSamples=$module.observedSamples})
    }
    $nvml = @($metadata | Where-Object { $_.name -ieq 'nvml.dll' })
    $nvmlObservation = if ($Capture.sampleCount -eq 0) { 'NO_SAMPLE' } elseif ($nvml.Count -gt 0) { 'OBSERVED_IN_SAMPLE' } else { 'NOT_OBSERVED_IN_POLLED_SNAPSHOTS' }
    return [pscustomobject]@{
        processId=$Capture.processId
        method=$Capture.method
        coverage='POLLING_ONLY_NONEXHAUSTIVE'
        absenceConclusion='NOT_SUPPORTED'
        targetPollMilliseconds=$Capture.targetPollMilliseconds
        sampleCount=$Capture.sampleCount
        samplesTruncated=[bool]$Capture.sampleTruncated
        modulePathUnionTruncated=[bool]$Capture.modulePathTruncated
        candidateMetadataTruncated=$candidateTruncated
        processExitedBeforeFirstSample=[bool]$Capture.processExitedBeforeFirstSample
        firstSampleUtc=$Capture.firstSampleUtc
        lastSampleUtc=$Capture.lastSampleUtc
        nvmlObservation=$nvmlObservation
        samples=@($Capture.samples.ToArray())
        observedModuleUnion=@($Capture.modules.Values | Sort-Object -Property path)
        nvidiaModuleMetadata=@($metadata.ToArray())
        errorCount=$Capture.errorCount
        errors=@($Capture.errors.ToArray())
    }
}

function Invoke-NvidiaDiagnosticContextSelfTests([string]$Lot) {
    $root = Join-Path $Lot ('selftest-nvidia-context-' + [guid]::NewGuid().ToString('N'))
    $junctionPath = Join-Path $root 'reparse-nv-smi'
    $rootJunctionPath = Join-Path $root 'reparse-context-root'
    New-Item -ItemType Directory -Path $root -Force -ErrorAction Stop | Out-Null
    try {
        $cudaRoot = Join-Path $root 'cuda\v12.9'
        $cudaBin = Join-Path $cudaRoot 'bin'
        $programFiles = Join-Path $root 'program-files'
        $nvSmi = Join-Path $programFiles 'NVIDIA Corporation\NVSMI'
        $secretPath = Join-Path $root 'ambient-secret-path'
        New-Item -ItemType Directory -Path $cudaBin,$nvSmi,$secretPath -Force -ErrorAction Stop | Out-Null
        $hostEnvironment = Get-NvidiaDiagnosticHostEnvironment
        $hostEnvironment.ProgramFiles = $programFiles
        $hostEnvironment.CUDA_PATH = $cudaRoot
        $hostEnvironment.CUDA_PATH_V12_9 = $cudaRoot
        $hostEnvironment.USERPROFILE = 'C:\host-profile-secret-sentinel'
        $hostEnvironment.NVIDIA_SECRET_SENTINEL = 'never-copy'
        $hostEnvironment.PATH = [string]::Join(';',@((Join-Path $env:SystemRoot 'System32'),$env:SystemRoot,$cudaBin,$nvSmi,$secretPath))
        $baseEnvironment = @{
            CARGO_HOME=(Join-Path $Lot 'cargo-home'); CARGO_TARGET_DIR=$script:CargoTarget; CARGO_INCREMENTAL='0'; CARGO_PROFILE_DEV_DEBUG='line-tables-only'
            CUDA_PATH=$cudaRoot; CUDA_PATH_V12_9=$cudaRoot; ProgramW6432=(Get-ValidatedProgramW6432Directory)
        }
        $contextResults = @{}
        foreach ($contextId in @('clean','host-nvidia-filtered','path-only','vars-only')) {
            $contextRoot = Join-Path $root (Join-Path 'runs' $contextId)
            New-Item -ItemType Directory -Path $contextRoot -Force -ErrorAction Stop | Out-Null
            $contextResults[$contextId] = New-NvidiaDiagnosticContextEnvironment $contextId $baseEnvironment $hostEnvironment $contextRoot
        }
        $clean = $contextResults.clean.environment
        $hostFiltered = $contextResults['host-nvidia-filtered'].environment
        $pathOnly = $contextResults['path-only'].environment
        $varsOnly = $contextResults['vars-only'].environment
        $expectedCleanPath = [string]::Join(';',@((Join-Path $env:SystemRoot 'System32'),(Get-FullPath $env:SystemRoot)))
        if ($clean.PATH -ine $expectedCleanPath -or $clean.ContainsKey('CUDA_PATH') -or $clean.ContainsKey('CUDA_PATH_V12_9')) { throw 'Clean NVIDIA diagnostic context retained a CUDA variable or non-system PATH entry.' }
        foreach ($context in @($hostFiltered,$pathOnly)) { if ($context.PATH -notlike "*$cudaBin*" -or $context.PATH -notlike "*$nvSmi*" -or $context.PATH -like "*$secretPath*") { throw 'Filtered NVIDIA PATH admitted a non-allowlisted directory or omitted an approved directory.' } }
        if ($hostFiltered.CUDA_PATH -ine $cudaRoot -or $pathOnly.ContainsKey('CUDA_PATH')) { throw 'Host-filtered/path-only CUDA variable separation failed.' }
        if ($varsOnly.CUDA_PATH -ine $cudaRoot -or $varsOnly.PATH -like "*$cudaBin*" -or $varsOnly.PATH -like "*$nvSmi*") { throw 'Variables-only context retained host PATH entries or lost the approved CUDA variable.' }
        foreach ($contextId in @('clean','host-nvidia-filtered','path-only','vars-only')) {
            $context = $contextResults[$contextId]
            $psi = New-NvidiaDiagnosticProcessInfo 'NvidiaSmiEnvironmentCompareDiagnostic' (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe') @('-NoProfile') $context.environment (Join-Path $root (Join-Path 'runs' $contextId))
            $expectedContextRoot = Get-FullPath (Join-Path $root (Join-Path 'runs' $contextId))
            if (-not $psi.EnvironmentVariables.ContainsKey('ProgramW6432') -or [string]$psi.EnvironmentVariables['ProgramW6432'] -ine $baseEnvironment.ProgramW6432) { throw "NVIDIA context did not transmit validated ProgramW6432: $contextId" }
            if ($psi.EnvironmentVariables.ContainsKey('NVIDIA_SECRET_SENTINEL')) { throw "NVIDIA context leaked an ambient secret: $contextId" }
            foreach ($name in @('USERPROFILE','PROGRAMDATA','ALLUSERSPROFILE','PUBLIC')) {
                if (-not $psi.EnvironmentVariables.ContainsKey($name)) { throw "NVIDIA context omitted copied environment root ${name}: $contextId" }
            }
            $expectedProgramData = Get-FullPath (Join-Path $expectedContextRoot 'programdata')
            $expectedPublic = Get-FullPath (Join-Path $expectedContextRoot 'public')
            if ((Get-FullPath ([string]$psi.EnvironmentVariables['USERPROFILE'])) -cne $expectedContextRoot -or
                (Get-FullPath ([string]$psi.EnvironmentVariables['PROGRAMDATA'])) -cne $expectedProgramData -or
                (Get-FullPath ([string]$psi.EnvironmentVariables['ALLUSERSPROFILE'])) -cne $expectedProgramData -or
                (Get-FullPath ([string]$psi.EnvironmentVariables['PUBLIC'])) -cne $expectedPublic) { throw "NVIDIA context did not copy the exact isolated root values: $contextId" }
        }
        $reparseTarget = Join-Path $root 'vendor-target'
        New-Item -ItemType Directory -Path $reparseTarget -Force -ErrorAction Stop | Out-Null
        New-Item -ItemType Junction -Path $junctionPath -Target $reparseTarget -ErrorAction Stop | Out-Null
        $reparseHost = @{} + $hostEnvironment
        $reparseHost.PATH = [string]::Join(';',@((Join-Path $env:SystemRoot 'System32'),$env:SystemRoot,$cudaBin,$junctionPath))
        $filteredReparse = Get-NvidiaDiagnosticFilteredPath $reparseHost $cudaBin $junctionPath $true
        if ($filteredReparse.reparseRejectedCount -lt 1 -or $filteredReparse.includedHostVendorDirectories -contains $junctionPath) { throw 'NVIDIA diagnostic PATH did not reject a reparse vendor directory.' }
        $rootTarget = Join-Path $root 'run-root-target'
        New-Item -ItemType Directory -Path $rootTarget -Force -ErrorAction Stop | Out-Null
        New-Item -ItemType Junction -Path $rootJunctionPath -Target $rootTarget -ErrorAction Stop | Out-Null
        $badRootRejected = $false
        try { New-NvidiaDiagnosticContextEnvironment 'clean' $baseEnvironment $hostEnvironment $rootJunctionPath | Out-Null } catch { $badRootRejected=$true }
        if (-not $badRootRejected) { throw 'NVIDIA diagnostic context accepted a reparse RunRoot.' }
        $outsideRootRejected = $false
        try { New-NvidiaDiagnosticContextEnvironment 'clean' $baseEnvironment $hostEnvironment $env:TEMP | Out-Null } catch { $outsideRootRejected=$true }
        if (-not $outsideRootRejected) { throw 'NVIDIA diagnostic context accepted an outside LotRoot.' }
        $equalLeft = [pscustomobject]@{exitCode=255;stdoutSha256='a';stderrSha256='b';stdout='first';elapsedSeconds=1}
        $equalRight = [pscustomobject]@{exitCode=255;stdoutSha256='a';stderrSha256='b';stdout='second';elapsedSeconds=2}
        if (Test-NvidiaDiagnosticSignaturesDiffer $equalLeft $equalRight) { throw 'NVIDIA conditional-context comparison considered paths/timing instead of only exit/stdout/stderr.' }
        foreach ($changed in @(
            [pscustomobject]@{exitCode=0;stdoutSha256='a';stderrSha256='b'},
            [pscustomobject]@{exitCode=255;stdoutSha256='changed';stderrSha256='b'},
            [pscustomobject]@{exitCode=255;stdoutSha256='a';stderrSha256='changed'}
        )) { if (-not (Test-NvidiaDiagnosticSignaturesDiffer $equalLeft $changed)) { throw 'NVIDIA conditional context was not triggered by an exit/stdout/stderr difference.' } }
        return [pscustomobject]@{contexts=@('clean','host-nvidia-filtered','path-only','vars-only');hostPathExactAllowlist='PASS';cudaVariableSplit='PASS';writableRootsRerouted='PASS';ambientSecretExcluded='PASS';reparsePathRejected='PASS';outsideRootRejected='PASS';conditionalSignature='exitCode/stdout/stderr only';processStartInfoConstructed=$true;processStarted=$false}
    } finally {
        if (Test-Path -LiteralPath $junctionPath) { Remove-Item -LiteralPath $junctionPath -Force -ErrorAction SilentlyContinue }
        if (Test-Path -LiteralPath $rootJunctionPath) { Remove-Item -LiteralPath $rootJunctionPath -Force -ErrorAction SilentlyContinue }
        if (Test-InRoot $root $Lot) { Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue }
    }
}

function Resolve-LotRoot([string]$Path) {
    $full = if ([IO.Path]::IsPathRooted($Path)) { Get-FullPath $Path } else { Get-FullPath (Join-Path $script:Repo $Path) }
    if (-not (Test-InRoot $full $script:Repo)) { throw "Lot root must stay inside this candidate worktree: $full" }
    $n2Root = Get-FullPath (Join-Path $script:Repo 'target\n2-native')
    $mainFacadesRoot = Get-FullPath (Join-Path $script:Repo 'target\parakeet-tests\main-facades')
    if ($full -ine $n2Root -and $full -ine $mainFacadesRoot) { throw "Lot root must be the exact candidate-local N2 or main-facades root: $full" }
    Assert-NoReparseChain $full 'candidate lot root'
    return $full
}

function Assert-LotRootForProfile([string]$ProfileId, [string]$Lot) {
    $n2Root = Get-FullPath (Join-Path $script:Repo 'target\n2-native')
    $mainFacadesRoot = Get-FullPath (Join-Path $script:Repo 'target\parakeet-tests\main-facades')
    if ($ProfileId -in @('TurboCpuReusableImports','TurboGpuReusableImports','ParakeetCpuContracts')) {
        if ($Lot -ine $mainFacadesRoot) { throw "Profile $ProfileId requires the exact candidate-local main-facades root: $mainFacadesRoot" }
        return
    }
    if ($Lot -ine $n2Root) { throw "Profile $ProfileId requires the exact candidate-local N2 root: $n2Root" }
}

function Assert-FreeSpace {
    $drive = [IO.Path]::GetPathRoot($script:Repo).TrimEnd('\').TrimEnd(':')
    $psDrive = Get-PSDrive -Name $drive -ErrorAction Stop
    if ($psDrive.Free -lt 30GB) { throw "$drive`: has less than 30 GiB free; native campaign is forbidden." }
}

function Initialize-LotDirectories([string]$Lot) {
    foreach ($path in @($Lot,(Join-Path $Lot 'runs'),(Join-Path $Lot 'logs'),(Join-Path $Lot 'cargo-home'),(Join-Path $Lot 'temp'),(Join-Path $Lot 'tmp'))) {
        Assert-NoReparseChain $path 'candidate lot write root'
        New-Item -ItemType Directory -Path $path -Force | Out-Null
        Assert-NoReparseChain $path 'candidate lot write root'
    }
}

function Get-ValuesDocument([string]$Path, [object]$Profile) {
    if (@($Profile.inputs).Count -eq 0 -and @($Profile.values).Count -eq 0) {
        if (-not [string]::IsNullOrWhiteSpace($Path)) { throw "Profile $($Profile.id) accepts no -ValuesJson." }
        return [pscustomobject]@{ inputs = @(); values = [pscustomobject]@{} }
    }
    if ([string]::IsNullOrWhiteSpace($Path)) { throw "Profile $($Profile.id) requires a bounded -ValuesJson manifest." }
    $manifest = Assert-PathUnder $Path $script:Lot 'profile values manifest' -MustExist
    try { $valuesDoc = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json -ErrorAction Stop }
    catch { throw "Values manifest JSON is invalid: $($_.Exception.Message)" }
    if ($valuesDoc.schemaVersion -ne 1 -or $valuesDoc.profile -cne $Profile.id) { throw 'Values manifest schema/profile does not match the selected profile.' }
    $expectedInputs = @($Profile.inputs | ForEach-Object { [string]$_.key })
    $givenInputs = @($valuesDoc.inputs | ForEach-Object { [string]$_.key })
    if (@($givenInputs | Sort-Object -Unique).Count -ne $givenInputs.Count -or -not (Test-ExactStringSet $expectedInputs $givenInputs)) {
        throw 'Values manifest input keys do not exactly match the profile.'
    }
    $expectedValues = @($Profile.values | ForEach-Object { [string]$_.key })
    $valueProps = @($valuesDoc.values.PSObject.Properties | ForEach-Object { $_.Name })
    if (@($valueProps | Sort-Object -Unique).Count -ne $valueProps.Count -or -not (Test-ExactStringSet $expectedValues $valueProps)) {
        throw 'Values manifest literal keys do not exactly match the profile.'
    }
    $assetMap = @{}
    foreach ($asset in @($valuesDoc.inputs)) {
        $descriptor = @($Profile.inputs | Where-Object { $_.key -ceq $asset.key })
        if ($descriptor.Count -ne 1 -or $asset.kind -cne $descriptor[0].kind) { throw "Input type mismatch for '$($asset.key)'." }
        $fullPath = Get-FullPath ([string]$asset.path)
        $allowed = $false
        foreach ($root in $script:AllowedReadRoots) { if (Test-InRoot $fullPath $root) { $allowed = $true; break } }
        if (-not $allowed) { throw "Input '$($asset.key)' is outside the explicit read-only source roots: $fullPath" }
        Assert-NoReparseChain $fullPath "input $($asset.key)"
        $validatedFiles = [Collections.Generic.List[object]]::new()
        $assetBytes = [long]0
        if ($asset.kind -ceq 'file') {
            if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf) -or [string]$asset.sha256 -notmatch '^[0-9a-fA-F]{64}$') { throw "Input file/hash is invalid: $($asset.key)" }
            $actual = Get-Sha256 $fullPath
            if ($actual -cne ([string]$asset.sha256).ToLowerInvariant()) { throw "Input SHA-256 mismatch for $($asset.key): $actual" }
            $fileInfo = Get-Item -LiteralPath $fullPath -Force
            if ($fileInfo.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Input file is a reparse point: $($asset.key)" }
            if ($asset.PSObject.Properties.Name -contains 'bytes' -and ([long]$asset.bytes -lt 0 -or [long]$asset.bytes -ne $fileInfo.Length)) { throw "Input byte length mismatch for '$($asset.key)'." }
            $assetBytes = [long]$fileInfo.Length
        } else {
            if (-not (Test-Path -LiteralPath $fullPath -PathType Container) -or @($asset.files).Count -eq 0) { throw "Input directory manifest is empty or missing: $($asset.key)" }
            $relativeNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
            foreach ($entry in @($asset.files)) {
                $relative = Get-SafeRelativePath ([string]$entry.path) "File path in directory input manifest '$($asset.key)'"
                if (-not $relativeNames.Add($relative)) { throw "Duplicate file in directory input manifest '$($asset.key)': $relative" }
                $filePath = Get-FullPath (Join-Path $fullPath $relative)
                if (-not (Test-InRoot $filePath $fullPath)) { throw "Directory input escaped its root: $relative" }
                Assert-NoReparseChain $filePath "directory input $($asset.key)"
                if (-not (Test-Path -LiteralPath $filePath -PathType Leaf) -or [string]$entry.sha256 -notmatch '^[0-9a-fA-F]{64}$') { throw "Directory input file/hash invalid: $relative" }
                $actual = Get-Sha256 $filePath
                if ($actual -cne ([string]$entry.sha256).ToLowerInvariant()) { throw "Directory input SHA-256 mismatch for '$relative': $actual" }
                $fileInfo = Get-Item -LiteralPath $filePath -Force
                if ($entry.PSObject.Properties.Name -contains 'bytes' -and ([long]$entry.bytes -lt 0 -or [long]$entry.bytes -ne $fileInfo.Length)) { throw "Directory input byte length mismatch for '$relative'." }
                $validatedFiles.Add([pscustomobject]@{ path=$relative; sha256=([string]$entry.sha256).ToLowerInvariant(); bytes=[long]$fileInfo.Length })
            }
        }
        $assetMap[$asset.key] = [pscustomobject]@{ path = $fullPath; kind = $asset.kind; sha256 = ([string]$asset.sha256).ToLowerInvariant(); bytes=$assetBytes; files = @($validatedFiles.ToArray()) }
    }
    foreach ($descriptor in @($Profile.values)) {
        $value = [string]$valuesDoc.values.($descriptor.key)
        if ([string]::IsNullOrWhiteSpace($value) -or $value.Length -gt 1024 -or $value.Contains("`r") -or $value.Contains("`n")) { throw "Literal profile value is missing or invalid: $($descriptor.key)" }
        if ($descriptor.kind -eq 'directory') {
            $value = Get-FullPath $value
            $cudaRoot = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'
            if ($descriptor.allowedRoot -cne 'cuda-12.9' -or $value -ine $cudaRoot -or -not (Test-Path -LiteralPath $value -PathType Container)) { throw "Profile tool directory differs from the approved CUDA 12.9 root or is missing: $value" }
            Assert-NoReparseChain $value "tool value $($descriptor.key)"
        }
    }
    Assert-VoxPopuliValues $Profile $assetMap
    Assert-NativeClosureValues $Profile $assetMap
    return [pscustomobject]@{ inputs = $assetMap; values = $valuesDoc.values }
}

function Get-StagingDestinationPath([object]$Profile, [object]$Rule, [string]$LocalAppData, [string]$RunRoot) {
    $base = if ($Rule.PSObject.Properties.Name -contains 'destinationBase') { [string]$Rule.destinationBase } else { 'localappdata' }
    $baseRoot = if ($base -ceq 'run') { $RunRoot } elseif ($base -ceq 'localappdata') { $LocalAppData } else { throw "Unsupported staging destination base in profile $($Profile.id): $base" }
    $relative = Get-SafeRelativePath ([string]$Rule.destination) "Staging destination in profile $($Profile.id)"
    $destination = Get-FullPath (Join-Path $baseRoot $relative)
    if (-not (Test-InRoot $destination $baseRoot) -or $destination -ieq (Get-FullPath $baseRoot)) { throw "Staging destination escaped its base root: $destination" }
    return $destination
}

function Get-ProfileEnvironment([object]$Profile, [object]$ValuesDoc, [string]$RunRoot, [string]$Lot, [bool]$CreateOutputs) {
    $environment = @{}
    foreach ($item in @($Profile.compileEnvironment)) { $environment[[string]$item.name] = [string]$item.value }
    $localAppDataPath = Join-Path $RunRoot 'localappdata'
    foreach ($item in @($Profile.inputs)) {
        $stageRules = @($Profile.staging | Where-Object { $null -ne $_ -and [string]$_.inputKey -ceq [string]$item.key })
        if ($stageRules.Count -gt 1) { throw "Input has more than one staging destination: $($item.key)" }
        $inputPath = if ($stageRules.Count -eq 1) { Get-StagingDestinationPath $Profile $stageRules[0] $localAppDataPath $RunRoot } else { [string]$ValuesDoc.inputs[$item.key].path }
        $environment[[string]$item.environmentVariable] = $inputPath
    }
    foreach ($item in @($Profile.values)) { $environment[[string]$item.environmentVariable] = [string]$ValuesDoc.values.($item.key) }
    foreach ($item in @($Profile.outputs)) {
        $slug = ([string]$item.key -replace '[^A-Za-z0-9._-]','-')
        $outputKind = if ($item.PSObject.Properties.Name -contains 'kind') { [string]$item.kind } else { 'directory' }
        $relativeOutput = if ($item.PSObject.Properties.Name -contains 'path') { Get-SafeRelativePath ([string]$item.path) "Output path in profile $($Profile.id)" } else { Join-Path 'writes' $slug }
        $output = Get-FullPath (Join-Path $RunRoot $relativeOutput)
        if (-not (Test-InRoot $output $RunRoot)) { throw "Profile output escaped its run root: $output" }
        Assert-NoReparseChain $output 'profile output'
        if ($CreateOutputs) {
            if ($outputKind -eq 'file') {
                $parent = Split-Path -Parent $output
                if (-not (Test-Path -LiteralPath $parent -PathType Container)) { New-Item -ItemType Directory -Path $parent -Force -ErrorAction Stop | Out-Null }
                if (Test-Path -LiteralPath $output) { throw "Profile file output already exists; refusing overwrite: $output" }
            } else {
                New-Item -ItemType Directory -Path $output -Force -ErrorAction Stop | Out-Null
            }
            Assert-NoReparseChain $output 'created profile output'
        }
        $environment[[string]$item.environmentVariable] = $output
    }
    if (Test-ProgramW6432ProfileId ([string]$Profile.id)) { $environment['ProgramW6432'] = Get-ValidatedProgramW6432Directory }
    $environment['CARGO_HOME'] = Join-Path $Lot 'cargo-home'
    $environment['CARGO_TARGET_DIR'] = $script:CargoTarget
    $environment['CARGO_INCREMENTAL'] = '0'
    $environment['CARGO_PROFILE_DEV_DEBUG'] = 'line-tables-only'
    $environment['TEMP'] = Join-Path $RunRoot 'temp'
    $environment['TMP'] = Join-Path $RunRoot 'tmp'
    $environment['LOCALAPPDATA'] = Join-Path $RunRoot 'localappdata'
    $environment['APPDATA'] = Join-Path $RunRoot 'appdata'
    $environment['USERPROFILE'] = $RunRoot
    $environment['HOME'] = $RunRoot
    return $environment
}

function Assert-StagingCapacity([long]$AvailableBytes, [long]$CopyBytes) {
    if ($CopyBytes -lt 0 -or $CopyBytes -gt ([long]::MaxValue - [long]30GB)) { throw 'Staging byte count is outside the supported range.' }
    $requiredBytes = [long]30GB + $CopyBytes
    if ($AvailableBytes -lt $requiredBytes) { throw "Staging needs at least 30 GiB plus $CopyBytes copy bytes; only $AvailableBytes bytes are free." }
}

function Copy-ProfileStaging([object]$Profile, [object]$ValuesDoc, [string]$LocalAppData, [string]$RunRoot) {
    $receipts = [Collections.Generic.List[object]]::new()
    $rules = @($Profile.staging | Where-Object { $null -ne $_ })
    if ($rules.Count -eq 0) { return }
    $localRoot = Assert-PathUnder $LocalAppData $RunRoot 'isolated LOCALAPPDATA' -MustExist -Directory
    $copyPlan = [Collections.Generic.List[object]]::new()
    $destinationRoots = [Collections.Generic.List[string]]::new()
    $totalBytes = [long]0
    foreach ($rule in $rules) {
        $inputKey = [string]$rule.inputKey
        $asset = $ValuesDoc.inputs[$inputKey]
        if ($null -eq $asset -or $asset.kind -notin @('file','directory')) { throw "Staging input is missing or has an unsupported kind: $inputKey" }
        $destinationBase = if ($rule.PSObject.Properties.Name -contains 'destinationBase') { [string]$rule.destinationBase } else { 'localappdata' }
        $baseRoot = if ($destinationBase -ceq 'localappdata') { $localRoot } elseif ($destinationBase -ceq 'run') { Get-FullPath $RunRoot } else { throw "Unsupported staging destination base in profile $($Profile.id): $destinationBase" }
        $destinationRoot = Get-StagingDestinationPath $Profile $rule $localRoot $RunRoot
        Assert-NoReparseChain $destinationRoot 'staging destination path'
        if (Test-Path -LiteralPath $destinationRoot) { throw "Staging destination already exists; refusing overwrite: $destinationRoot" }
        $qualifiedRoot = $destinationBase + '\' + (Get-SafeRelativePath ([string]$rule.destination) "Staging destination in profile $($Profile.id)")
        foreach ($priorRoot in $destinationRoots) {
            if ($qualifiedRoot.Equals($priorRoot,[StringComparison]::OrdinalIgnoreCase) -or $qualifiedRoot.StartsWith($priorRoot + '\',[StringComparison]::OrdinalIgnoreCase) -or $priorRoot.StartsWith($qualifiedRoot + '\',[StringComparison]::OrdinalIgnoreCase)) {
                throw "Overlapping staging destinations: $qualifiedRoot and $priorRoot"
            }
        }
        $destinationRoots.Add($qualifiedRoot)
        $files = if ($asset.kind -ceq 'directory') { @($asset.files) } else { [pscustomobject]@{path=[IO.Path]::GetFileName([string]$asset.path);sha256=[string]$asset.sha256;bytes=[long]$asset.bytes} }
        $files = @($files)
        if ($files.Count -ne [int]$rule.requiredFileCount -or $files.Count -gt 256) { throw "Staging input file count differs from its profile pin: $inputKey ($($files.Count))" }
        $copyPlan.Add([pscustomobject]@{ destinationRoot=$destinationRoot; rootMarker=$true; rootKind=[string]$asset.kind; inputKey=$inputKey; baseRoot=$baseRoot })
        $seenFiles = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        $inputBytes = [long]0
        foreach ($file in $files) {
            $relative = if ($asset.kind -ceq 'directory') { Get-SafeRelativePath ([string]$file.path) "Staging source file path for $inputKey" } else { [IO.Path]::GetFileName([string]$asset.path) }
            if (-not $seenFiles.Add($relative)) { throw "Duplicate staging source file path: $relative" }
            $source = if ($asset.kind -ceq 'directory') { Get-FullPath (Join-Path ([string]$asset.path) $relative) } else { Get-FullPath ([string]$asset.path) }
            if ($asset.kind -ceq 'directory' -and -not (Test-InRoot $source ([string]$asset.path))) { throw "Staging source escaped its declared directory: $source" }
            Assert-NoReparseChain $source 'staging source path'
            if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Staging source file is missing: $source" }
            $sourceInfo = Get-Item -LiteralPath $source -Force
            if ($sourceInfo.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Staging source file is a reparse point: $source" }
            $expectedHash = ([string]$file.sha256).ToLowerInvariant()
            if ($expectedHash -notmatch '^[0-9a-f]{64}$') { throw "Staging source SHA-256 is invalid: $relative" }
            $beforeHash = Get-Sha256 $source
            if ($beforeHash -cne $expectedHash) { throw "Staging source SHA-256 mismatch before copy for '$relative': $beforeHash" }
            if ($file.PSObject.Properties.Name -contains 'bytes' -and [long]$file.bytes -ne $sourceInfo.Length) { throw "Staging source byte length changed before copy: $relative" }
            if ($totalBytes -gt ([long]::MaxValue - $sourceInfo.Length)) { throw 'Staging byte count overflow.' }
            $totalBytes += [long]$sourceInfo.Length
            if ($inputBytes -gt ([long]::MaxValue - $sourceInfo.Length)) { throw 'Staging input byte count overflow.' }
            $inputBytes += [long]$sourceInfo.Length
            $destination = if ($asset.kind -ceq 'directory') { Get-FullPath (Join-Path $destinationRoot $relative) } else { $destinationRoot }
            if (-not (Test-InRoot $destination $destinationRoot) -and $asset.kind -ceq 'directory') { throw "Staging file destination escaped its root: $destination" }
            if (-not (Test-InRoot $destination $baseRoot)) { throw "Staging file destination escaped its authorized base: $destination" }
            Assert-NoReparseChain $destination 'staging destination file path'
            if (Test-Path -LiteralPath $destination) { throw "Staging destination file already exists; refusing overwrite: $destination" }
            $copyPlan.Add([pscustomobject]@{ source=$source; destination=$destination; relative=$relative; inputKey=$inputKey; expectedHash=$expectedHash; bytes=[long]$sourceInfo.Length; sourceHashBefore=$beforeHash })
        }
        if ($inputBytes -ne [long]$rule.requiredBytes) { throw "Staging input byte count differs from its profile pin: $inputKey ($inputBytes)" }
    }
    $drive = [IO.Path]::GetPathRoot($script:Repo).TrimEnd('\').TrimEnd(':')
    $psDrive = Get-PSDrive -Name $drive -ErrorAction Stop
    Assert-StagingCapacity ([long]$psDrive.Free) $totalBytes
    foreach ($item in $copyPlan) {
        if ($item.rootMarker) {
            if (Test-Path -LiteralPath $item.destinationRoot) { throw "Staging destination appeared before creation; refusing overwrite: $($item.destinationRoot)" }
            if ($item.rootKind -ceq 'directory') {
                $parent = Split-Path -Parent $item.destinationRoot
                if (-not (Test-Path -LiteralPath $parent -PathType Container)) { New-Item -ItemType Directory -Path $parent -Force -ErrorAction Stop | Out-Null }
                New-Item -ItemType Directory -Path $item.destinationRoot -ErrorAction Stop | Out-Null
                Assert-NoReparseChain $item.destinationRoot 'created staging destination'
            } else {
                $parent = Split-Path -Parent $item.destinationRoot
                if (-not (Test-Path -LiteralPath $parent -PathType Container)) { New-Item -ItemType Directory -Path $parent -Force -ErrorAction Stop | Out-Null }
                Assert-NoReparseChain $parent 'created staging destination parent'
            }
            $receipts.Add([pscustomobject]@{ inputKey=$item.inputKey; destination=$item.destinationRoot; fileCount=0; bytes=0 })
            continue
        }
        $receipt = $null
        foreach ($candidate in $receipts) { if ([string]$candidate.inputKey -ceq [string]$item.inputKey) { $receipt = $candidate; break } }
        if ($null -eq $receipt) { throw "Staging receipt root is missing for $($item.destination)." }
        $parent = Split-Path -Parent $item.destination
        if (-not (Test-Path -LiteralPath $parent -PathType Container)) { New-Item -ItemType Directory -Path $parent -Force -ErrorAction Stop | Out-Null }
        Assert-NoReparseChain $parent 'staging destination directory'
        if (Test-Path -LiteralPath $item.destination) { throw "Staging destination file appeared before copy; refusing overwrite: $($item.destination)" }
        [IO.File]::Copy([string]$item.source,[string]$item.destination,$false)
        Assert-NoReparseChain $item.destination 'copied staging file'
        $copiedInfo = Get-Item -LiteralPath $item.destination -Force
        $destinationHash = Get-Sha256 ([string]$item.destination)
        $sourceHashAfter = Get-Sha256 ([string]$item.source)
        if ($destinationHash -cne $item.expectedHash -or $sourceHashAfter -cne $item.sourceHashBefore -or $copiedInfo.Length -ne $item.bytes) {
            throw "Staging copy verification failed for '$($item.relative)' (destination=$destinationHash sourceAfter=$sourceHashAfter)."
        }
        $receipt.fileCount = [int]$receipt.fileCount + 1
        $receipt.bytes = [long]$receipt.bytes + [long]$item.bytes
    }
    return $receipts.ToArray()
}

function Initialize-PersonalGpuSettingsParent([string]$ProfileId, [string]$RunRoot) {
    if ($ProfileId -cne 'PersonalUnifiedGpuOnly') { return $null }
    $runRootFull = Assert-PathUnder $RunRoot $script:Lot 'PersonalUnifiedGpuOnly RunRoot' -MustExist -Directory
    $localAppData = Assert-PathUnder (Join-Path $runRootFull 'localappdata') $runRootFull 'PersonalUnifiedGpuOnly LOCALAPPDATA' -MustExist -Directory
    $settingsPath = Get-FullPath (Join-Path $localAppData 'Whisper\settings')
    if (-not (Test-InRoot $settingsPath $runRootFull)) { throw "PersonalUnifiedGpuOnly settings parent escaped RunRoot: $settingsPath" }
    Assert-NoReparseChain $settingsPath 'PersonalUnifiedGpuOnly settings parent before creation'
    if (Test-Path -LiteralPath $settingsPath -PathType Leaf) { throw "PersonalUnifiedGpuOnly settings parent is not a directory: $settingsPath" }
    if (-not (Test-Path -LiteralPath $settingsPath -PathType Container)) {
        New-Item -ItemType Directory -Path $settingsPath -Force -ErrorAction Stop | Out-Null
    }
    $createdPath = Assert-PathUnder $settingsPath $runRootFull 'PersonalUnifiedGpuOnly settings parent' -MustExist -Directory
    Assert-NoReparseChain $createdPath 'PersonalUnifiedGpuOnly settings parent after creation'
    return $createdPath
}

function Invoke-PersonalGpuSettingsParentSelfTest([string]$Lot) {
    $fixtureRoot = Join-Path $Lot ('selftest-personal-gpu-settings-' + [guid]::NewGuid().ToString('N'))
    $runRoot = Join-Path $fixtureRoot 'run'
    $outsideRoot = Join-Path $fixtureRoot 'outside'
    $reparseRunRoot = Join-Path $fixtureRoot 'reparse-run'
    $junction = Join-Path (Join-Path $reparseRunRoot 'localappdata') 'Whisper'
    New-Item -ItemType Directory -Path (Join-Path $runRoot 'localappdata'),$outsideRoot -Force -ErrorAction Stop | Out-Null
    try {
        $localAppDataBefore = [Environment]::GetEnvironmentVariable('LOCALAPPDATA','Process')
        $created = Initialize-PersonalGpuSettingsParent 'PersonalUnifiedGpuOnly' $runRoot
        $settingsPath = Join-Path (Join-Path $runRoot 'localappdata') 'Whisper\settings'
        if ($created -ine (Get-FullPath $settingsPath) -or -not (Test-Path -LiteralPath $settingsPath -PathType Container)) { throw 'GPU-only settings parent was not created under its RunRoot.' }
        if (Test-Path -LiteralPath (Join-Path $settingsPath 'logs')) { throw 'GPU-only settings prerequisite created the product-owned logs directory.' }
        $beforeEntries = @(Get-ChildItem -LiteralPath $settingsPath -Force | Select-Object -ExpandProperty FullName)
        $again = Initialize-PersonalGpuSettingsParent 'PersonalUnifiedGpuOnly' $runRoot
        $afterEntries = @(Get-ChildItem -LiteralPath $settingsPath -Force | Select-Object -ExpandProperty FullName)
        if ($again -ine $created -or [string]::Join("`n",$beforeEntries) -cne [string]::Join("`n",$afterEntries)) { throw 'GPU-only settings parent initialization was not idempotent.' }
        $regularRunRoot = Join-Path $fixtureRoot 'regular-run'
        New-Item -ItemType Directory -Path (Join-Path $regularRunRoot 'localappdata') -Force -ErrorAction Stop | Out-Null
        if ($null -ne (Initialize-PersonalGpuSettingsParent 'PersonalUnified' $regularRunRoot) -or (Test-Path -LiteralPath (Join-Path $regularRunRoot 'localappdata\Whisper'))) { throw 'Settings parent was created for the non-GPU-only PersonalUnified profile.' }
        Test-Rejects 'GPU-only settings parent outside RunRoot' { Initialize-PersonalGpuSettingsParent 'PersonalUnifiedGpuOnly' $outsideRoot }
        $junctionTarget = Join-Path $outsideRoot 'Whisper'
        New-Item -ItemType Directory -Path (Join-Path $reparseRunRoot 'localappdata'),$junctionTarget -Force -ErrorAction Stop | Out-Null
        New-Item -ItemType Junction -Path $junction -Target $junctionTarget -ErrorAction Stop | Out-Null
        Test-Rejects 'GPU-only settings parent reparse chain' { Initialize-PersonalGpuSettingsParent 'PersonalUnifiedGpuOnly' $reparseRunRoot }
        if ([Environment]::GetEnvironmentVariable('LOCALAPPDATA','Process') -cne $localAppDataBefore) { throw 'GPU-only settings prerequisite changed the process environment.' }
        return [pscustomobject]@{ aliasOnly='PASS'; underRunRoot='PASS'; reparseRejected='PASS'; idempotent='PASS'; logsNotCreated='PASS'; environmentUnchanged=$true }
    } finally {
        if (Test-Path -LiteralPath $junction) { Remove-Item -LiteralPath $junction -Force -ErrorAction SilentlyContinue }
        if ((Test-InRoot $fixtureRoot $Lot) -and (Test-Path -LiteralPath $fixtureRoot)) { Remove-Item -LiteralPath $fixtureRoot -Recurse -Force -ErrorAction SilentlyContinue }
    }
}

function Assert-CampaignRoots([string]$Lot, [string]$RunRoot, [hashtable]$Environment) {
    if (-not (Test-InRoot $Lot $script:Repo)) { throw 'Lot root escaped the candidate worktree.' }
    if (-not (Test-InRoot $RunRoot $Lot)) { throw 'Run root escaped the candidate lot.' }
    Assert-NoReparseChain $script:Repo 'candidate worktree'
    Assert-NoReparseChain $Lot 'candidate lot root'
    Assert-NoReparseChain $script:CargoTarget 'shared Cargo target'
    if (-not (Test-Path -LiteralPath $script:CargoTarget -PathType Container)) { throw "Canonical shared Cargo target is missing: $script:CargoTarget" }
    foreach ($name in @('CARGO_HOME','TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) {
        $value = [string]$Environment[$name]
        if ([string]::IsNullOrWhiteSpace($value)) { throw "$name is unset in the isolated campaign environment." }
        $root = if ($name -eq 'CARGO_HOME') { $Lot } else { $RunRoot }
        if (-not (Test-InRoot $value $root)) { throw "$name escaped its canonical campaign root: $value" }
        Assert-NoReparseChain $value $name
    }
    if ((Get-FullPath $Environment.CARGO_TARGET_DIR) -ine $script:CargoTarget) { throw 'CARGO_TARGET_DIR is not the approved shared target.' }
    if ($Environment.CARGO_INCREMENTAL -cne '0' -or $Environment.CARGO_PROFILE_DEV_DEBUG -cne 'line-tables-only') { throw 'Cargo profile environment is not the accepted N2 configuration.' }
}

function Get-VsDevCommand {
    $candidate = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat'
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) { throw "Visual Studio environment initializer is missing: $candidate" }
    Assert-NoReparseChain $candidate 'Visual Studio environment initializer'
    return $candidate
}

function Get-VsEnvironment([string]$VsDevCommand, [string]$Temp, [string]$Tmp, [string]$RunRoot) {
    $bootstrap = Join-Path $script:RunRoot 'temp\n2-vsdev-environment.cmd'
    $bootstrapBody = "@echo off`r`ncall `"$VsDevCommand`" -no_logo -arch=x64 -host_arch=x64`r`nif errorlevel 1 exit /b %errorlevel%`r`nset`r`n"
    [IO.File]::WriteAllText($bootstrap,$bootstrapBody,[Text.Encoding]::ASCII)
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = $env:ComSpec
    $psi.Arguments = '/d /c "' + $bootstrap + '"'
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $vars = $psi.EnvironmentVariables
    $vars.Clear()
    foreach ($name in @('SystemRoot','WINDIR','SystemDrive','ComSpec','PATHEXT','OS','PROCESSOR_ARCHITECTURE')) {
        $value = [Environment]::GetEnvironmentVariable($name)
        if (-not [string]::IsNullOrWhiteSpace($value)) { $vars[$name] = $value }
    }
    $vars['PATH'] = [string]::Join(';',@((Join-Path $env:SystemRoot 'System32'),$env:SystemRoot))
    $vars['TEMP'] = $Temp; $vars['TMP'] = $Tmp
    $vars['USERPROFILE'] = $RunRoot
    $vars['APPDATA'] = Join-Path $RunRoot 'appdata'
    $vars['LOCALAPPDATA'] = Join-Path $RunRoot 'localappdata'
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $psi
    try {
        if (-not $process.Start()) { throw 'VsDevCmd environment bootstrap failed to start.' }
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "VsDevCmd failed with $($process.ExitCode): $stderr" }
        $environment = @{}
        foreach ($line in ($stdout -split "`r?`n")) {
            $equals = $line.IndexOf('=')
            if ($equals -le 0) { continue }
            $environment[$line.Substring(0,$equals)] = $line.Substring($equals + 1)
        }
        $allowedNames = @('PATH','INCLUDE','LIB','VCToolsInstallDir','VCToolsVersion','VCToolsRedistDir','WindowsSdkDir','WindowsSDKVersion','WindowsSDKLibVersion','UniversalCRTSdkDir','UCRTVersion','VCINSTALLDIR','VSINSTALLDIR','VisualStudioVersion','DevEnvDir','VSCMD_VER','VSCMD_ARG_TGT_ARCH','VSCMD_ARG_HOST_ARCH')
        $filtered = @{}
        foreach ($name in $allowedNames) { if ($environment.ContainsKey($name)) { $filtered[$name] = [string]$environment[$name] } }
        foreach ($name in @('PATH','INCLUDE','LIB','VCToolsInstallDir','WindowsSdkDir','WindowsSDKVersion')) {
            if ([string]::IsNullOrWhiteSpace([string]$filtered[$name])) { throw "VsDevCmd did not provide required native environment variable $name." }
        }
        return $filtered
    } finally { $process.Dispose() }
}

function Get-CargoExecutable {
    $rustToolchain = Join-Path $script:Repo 'rust-toolchain.toml'
    if (-not (Test-Path -LiteralPath $rustToolchain -PathType Leaf)) { throw 'Candidate rust-toolchain.toml is missing.' }
    $channel = (Get-Content -LiteralPath $rustToolchain | Where-Object { $_ -match '^\s*channel\s*=\s*"([^"]+)"' } | ForEach-Object { $Matches[1] } | Select-Object -First 1)
    if ([string]::IsNullOrWhiteSpace($channel)) { throw 'Could not resolve the candidate Rust toolchain channel without invoking Cargo.' }
    $toolchain = Join-Path $env:USERPROFILE (Join-Path '.rustup\toolchains' (Join-Path ($channel + '-x86_64-pc-windows-msvc') 'bin'))
    $cargo = Join-Path $toolchain 'cargo.exe'
    $rustc = Join-Path $toolchain 'rustc.exe'
    foreach ($path in @($cargo,$rustc)) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Candidate pinned Rust toolchain executable is missing: $path" }
        Assert-NoReparseChain $path 'Rust toolchain executable'
    }
    return [pscustomobject]@{ cargo = $cargo; rustc = $rustc; toolchainBin = $toolchain }
}

function New-CargoArguments([object]$Profile, [object]$Case) {
    $argsList = [Collections.Generic.List[string]]::new()
    $argsList.Add('test'); $argsList.Add('--offline'); $argsList.Add('--locked')
    if ($Profile.package -ceq 'workspace') { $argsList.Add('--workspace') }
    else { $argsList.Add('-p'); $argsList.Add([string]$Profile.package) }
    if (@($Profile.features).Count -gt 0) { $argsList.Add('--features'); $argsList.Add([string]::Join(',', [string[]]$Profile.features)) }
    if ($Profile.id -cin @('ParakeetGpuPreparation','ParakeetGpuInventory')) { $argsList.Add('--test'); $argsList.Add('parakeet_unified_native') }
    if ($Profile.operation -eq 'NoRun') { $argsList.Add('--no-run'); return ,([string[]]$argsList.ToArray()) }
    if ($Profile.operation -eq 'List') { $argsList.Add('--'); $argsList.Add('--list'); return ,([string[]]$argsList.ToArray()) }
    switch ([string]$Case.targetKind) {
        'lib' { $argsList.Add('--lib') }
        'integration' { if ([string]::IsNullOrWhiteSpace([string]$Case.targetName) -or $Case.targetName -notmatch '^[A-Za-z0-9_-]+$') { throw 'Integration test target name is invalid.' }; $argsList.Add('--test'); $argsList.Add([string]$Case.targetName) }
        'bin' { if ([string]::IsNullOrWhiteSpace([string]$Case.targetName) -or $Case.targetName -notmatch '^[A-Za-z0-9_-]+$') { throw 'Binary test target name is invalid.' }; $argsList.Add('--bin'); $argsList.Add([string]$Case.targetName) }
        default { throw "Unsupported Cargo target kind: $($Case.targetKind)" }
    }
    $argsList.Add([string]$Case.exactTest)
    $argsList.Add('--')
    if ($Case.ignored) { $argsList.Add('--ignored') }
    $argsList.Add('--exact'); $argsList.Add([string]$Case.exactTest)
    $argsList.Add('--test-threads=1'); $argsList.Add('--nocapture')
    return ,([string[]]$argsList.ToArray())
}

function Get-WindowsCommandLineArgument([string]$Argument) {
    if ($Argument.Length -gt 0 -and $Argument -notmatch '[\s"]') { return $Argument }
    $builder = [Text.StringBuilder]::new()
    [void]$builder.Append('"')
    $backslashes = 0
    foreach ($character in $Argument.ToCharArray()) {
        if ($character -eq '\') { $backslashes++; continue }
        if ($character -eq '"') {
            [void]$builder.Append(('\' * (($backslashes * 2) + 1))).Append('"')
            $backslashes = 0
            continue
        }
        if ($backslashes -gt 0) { [void]$builder.Append(('\' * $backslashes)); $backslashes = 0 }
        [void]$builder.Append($character)
    }
    if ($backslashes -gt 0) { [void]$builder.Append(('\' * ($backslashes * 2))) }
    [void]$builder.Append('"')
    return $builder.ToString()
}

function Get-EventDisposition([object]$Event, [string[]]$AllowedRoots, [string[]]$ProtectedRoots, [string]$UserTemp) {
    if ($Event.type -eq 'WatcherError') { return 'VIOLATION' }
    $path = Get-FullPath ([string]$Event.path)
    $oldPath = if ($Event.type -eq 'Renamed' -and -not [string]::IsNullOrWhiteSpace([string]$Event.old_path)) { Get-FullPath ([string]$Event.old_path) } else { '' }
    $newInTemp = Test-InRoot $path $UserTemp
    $oldInTemp = if ($oldPath) { Test-InRoot $oldPath $UserTemp } else { $false }
    if ($Event.type -eq 'Renamed') {
        if ($newInTemp -and $oldInTemp) { return 'UNATTRIBUTED_TEMP' }
        if ($newInTemp -or $oldInTemp) { return 'VIOLATION' }
    } elseif ($newInTemp -and $Event.type -in @('Created','Changed','Deleted')) {
        return 'UNATTRIBUTED_TEMP'
    }
    foreach ($root in $ProtectedRoots) { if (Test-InRoot $path $root) { return 'VIOLATION' } }
    $newAllowed = $false
    foreach ($root in $AllowedRoots) { if (Test-InRoot $path $root) { $newAllowed = $true; break } }
    if (-not $newAllowed) { return 'VIOLATION' }
    if ($Event.type -eq 'Renamed') {
        $oldAllowed = $false
        foreach ($root in $AllowedRoots) { if (Test-InRoot $oldPath $root) { $oldAllowed = $true; break } }
        foreach ($root in $ProtectedRoots) { if (Test-InRoot $oldPath $root) { $oldAllowed = $false; break } }
        if (-not $oldAllowed) { return 'VIOLATION' }
    }
    return 'ALLOWED'
}

function Register-Watch([System.Collections.Generic.List[object]]$Specs, [string]$Path, [bool]$Recursive) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) { return }
    Assert-NoReparseChain $Path 'watcher root'
    $Specs.Add([pscustomobject]@{ path = (Get-FullPath $Path); recursive = $Recursive })
}

function Add-SplitTreeWatches([System.Collections.Generic.List[object]]$Specs, [string]$Root, [string[]]$ExcludedChildren) {
    Register-Watch $Specs $Root $false
    foreach ($child in (Get-ChildItem -LiteralPath $Root -Directory -Force)) {
        if ($child.Name -in $ExcludedChildren) { continue }
        Register-Watch $Specs $child.FullName $true
    }
}

function Add-NativeProtectedRootWatches([System.Collections.Generic.List[object]]$Specs, [string[]]$Roots) {
    foreach ($root in $Roots) { Register-Watch $Specs $root $true }
}

function Get-RunWatchSpecs([string]$Lot, [string[]]$AdditionalProtectedRoots = [string[]](Get-NativeProtectedRoots)) {
    $specs = [Collections.Generic.List[object]]::new()
    Add-SplitTreeWatches $specs $script:Repo @('.git','target')
    $repoTarget = Join-Path $script:Repo 'target'
    Register-Watch $specs $repoTarget $false
    foreach ($child in (Get-ChildItem -LiteralPath $repoTarget -Directory -Force -ErrorAction SilentlyContinue)) {
        if ($child.FullName -ieq $Lot) { continue }
        Register-Watch $specs $child.FullName $true
    }

    $mainRoot = 'C:\dev\whisper'
    if ((Get-FullPath $mainRoot) -ine (Get-FullPath $script:Repo)) {
        Register-Watch $specs $mainRoot $false
        foreach ($child in (Get-ChildItem -LiteralPath $mainRoot -Directory -Force)) {
            if ($child.Name -in @('.git','target','.wt')) { continue }
            Register-Watch $specs $child.FullName $true
        }
        $worktreesRoot = Join-Path $mainRoot '.wt'
        if (Test-Path -LiteralPath $worktreesRoot -PathType Container) {
            Register-Watch $specs $worktreesRoot $false
            foreach ($child in (Get-ChildItem -LiteralPath $worktreesRoot -Directory -Force)) {
                if ((Get-FullPath $child.FullName) -ieq (Get-FullPath $script:Repo)) { continue }
                Register-Watch $specs $child.FullName $true
            }
        }
        $mainTarget = Join-Path $mainRoot 'target'
        if (Test-Path -LiteralPath $mainTarget -PathType Container) {
            Register-Watch $specs $mainTarget $false
            foreach ($child in (Get-ChildItem -LiteralPath $mainTarget -Directory -Force)) {
                if ($child.Name -ieq 'parakeet-tests') { continue }
                Register-Watch $specs $child.FullName $true
            }
            $testsRoot = Join-Path $mainTarget 'parakeet-tests'
            if (Test-Path -LiteralPath $testsRoot -PathType Container) {
                Register-Watch $specs $testsRoot $false
                foreach ($child in (Get-ChildItem -LiteralPath $testsRoot -Directory -Force)) {
                    if ($child.Name -ieq 'cargo-shared') { continue }
                    Register-Watch $specs $child.FullName $true
                }
            }
        }
    }
    $userTempRoot = Join-Path $env:LOCALAPPDATA 'Temp'
    if (-not (Test-Path -LiteralPath $userTempRoot -PathType Container)) { throw "Real user Temp is missing; native campaign watcher cannot classify it: $userTempRoot" }
    foreach ($path in @((Join-Path $env:USERPROFILE '.cargo'),$userTempRoot,(Join-Path $env:LOCALAPPDATA 'Whisper'))) {
        if (-not (Test-Path -LiteralPath $path -PathType Container)) { continue }
        Register-Watch $specs $path $true
    }
    Add-NativeProtectedRootWatches $specs $AdditionalProtectedRoots
    $unique = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    return @($specs | Where-Object { $unique.Add("$($_.path)|$($_.recursive)") })
}

function Initialize-NativeDirectoryWatch([string]$TempRoot) {
    if ('Whisper.NativeWatch.Collector' -as [type]) { return }
    if (-not (Test-InRoot $TempRoot $script:Lot)) { throw 'Native collector compiler temporaries must remain under the lot.' }
    Assert-NoReparseChain $TempRoot 'native collector compiler temporary root'
    New-Item -ItemType Directory -Path $TempRoot -Force | Out-Null
    $source=Join-Path $PSScriptRoot 'native-directory-watch.cs'
    Assert-NoReparseChain $source 'native notification source'
    $controlSource=Join-Path $PSScriptRoot 'native-build-control.cs'
    Assert-NoReparseChain $controlSource 'native build control source'
    $previousTemp=$env:TEMP; $previousTmp=$env:TMP
    try {
        $env:TEMP=$TempRoot; $env:TMP=$TempRoot
        Add-Type -Path @($source,$controlSource) -ErrorAction Stop
    } finally { $env:TEMP=$previousTemp; $env:TMP=$previousTmp }
}

function Start-NativeBuildWatchState([object[]]$Specs, [string]$LogRoot) {
    Initialize-NativeDirectoryWatch (Join-Path $LogRoot 'collector-temp')
    $roots=[string[]]@($Specs | ForEach-Object { Assert-NoReparseChain $_.path 'native watcher root'; [string]$_.path })
    $recursive=[bool[]]@($Specs | ForEach-Object { [bool]$_.recursive })
    $collector=[Whisper.NativeWatch.Collector]::new($roots,$recursive)
    return [pscustomobject]@{
        queue=[Collections.Concurrent.ConcurrentQueue[object]]::new()
        nativeCollector=$collector; nativeLogRoot=$LogRoot; nativeStopped=$false
        watchers=@(); subscriptions=@()
    }
}

function Receive-NativeBuildWatchEvents([object]$State) {
    $snapshot=$State.nativeCollector.Drain()
    # One immutable snapshot is the boundary. Never wait for a later half or infer its path.
    if ($snapshot.Records.Count -gt 0) {
        Write-JsonLine (Join-Path $State.nativeLogRoot 'watcher-native-records.jsonl') $snapshot
    }
    foreach ($observation in $snapshot.Events) {
        $records=@($observation.Records)
        $newRecord=if ($observation.Type -eq 'Renamed') { $records[1] } else { $records[0] }
        $oldRecord=if ($observation.Type -eq 'Renamed') { $records[0] } else { $null }
        $raw=[ordered]@{
            watcher_root=$newRecord.Root; args_type='Win32.FILE_NOTIFY_EXTENDED_INFORMATION'
            raw_name=$newRecord.Name; name_is_null=($null -eq $newRecord.Name); name_is_empty=($newRecord.Name -ceq '')
            raw_old_name=if ($null -ne $oldRecord) { $oldRecord.Name } else { $null }
            file_id=$newRecord.FileId.ToString('x16'); volume=$newRecord.Volume; session=$newRecord.Session
            drain=$snapshot.Drain; records=$records
        }
        $State.queue.Enqueue([pscustomobject]@{
            path=$observation.Path; old_path=$observation.OldPath; type=$observation.Type
            at=$observation.At; detail=$observation.Detail; raw_event=$raw
        })
    }
}

function Start-WatchState([object[]]$Specs) {
    $queue = [Collections.Concurrent.ConcurrentQueue[object]]::new()
    $watchers = [Collections.Generic.List[IO.FileSystemWatcher]]::new()
    $subscriptions = [Collections.Generic.List[string]]::new()
    foreach ($spec in $Specs) {
        $watcher = [IO.FileSystemWatcher]::new([string]$spec.path)
        $watcher.IncludeSubdirectories = [bool]$spec.recursive
        $watcher.NotifyFilter = [IO.NotifyFilters]::FileName -bor [IO.NotifyFilters]::DirectoryName -bor [IO.NotifyFilters]::LastWrite -bor [IO.NotifyFilters]::Size
        $watcher.InternalBufferSize = 65536
        foreach ($eventName in @('Created','Changed','Deleted','Renamed','Error')) {
            $sourceId = 'n2-' + $eventName + '-' + [guid]::NewGuid().ToString('N')
            Register-ObjectEvent -InputObject $watcher -EventName $eventName -SourceIdentifier $sourceId | Out-Null
            $subscriptions.Add($sourceId)
        }
        $watcher.EnableRaisingEvents = $true
        $watchers.Add($watcher)
    }
    return [pscustomobject]@{ queue=$queue; watchers=$watchers; subscriptions=$subscriptions }
}

function Get-WatcherObservation([object]$EventArgs, [string]$WatcherRoot) {
    $isFileEvent = $EventArgs -is [IO.FileSystemEventArgs]
    $isRename = $EventArgs -is [IO.RenamedEventArgs]
    $name = if ($isFileEvent) { $EventArgs.Name } else { $null }
    $oldName = if ($isRename) { $EventArgs.OldName } else { $null }
    return [ordered]@{
        watcher_root=$WatcherRoot; args_type=$EventArgs.GetType().FullName
        name_applicable=$isFileEvent; raw_name=$name
        name_is_null=($isFileEvent -and $null -eq $name)
        name_is_empty=($isFileEvent -and $null -ne $name -and $name.Length -eq 0)
        old_name_applicable=$isRename; raw_old_name=$oldName
        old_name_is_null=($isRename -and $null -eq $oldName)
        old_name_is_empty=($isRename -and $null -ne $oldName -and $oldName.Length -eq 0)
    }
}

function Receive-WatcherEvents([object]$State) {
    if ($null -eq $State) { return }
    if ($State.PSObject.Properties['nativeCollector']) { Receive-NativeBuildWatchEvents $State; return }
    $subscribers = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($id in $State.subscriptions) { [void]$subscribers.Add([string]$id) }
    foreach ($eventRecord in @(Get-Event -ErrorAction SilentlyContinue | Where-Object { $subscribers.Contains([string]$_.SourceIdentifier) })) {
        $args = $eventRecord.SourceEventArgs
        $observation = Get-WatcherObservation $args ([string]$eventRecord.Sender.Path)
        $observation.source_identifier = [string]$eventRecord.SourceIdentifier
        if ($args -is [IO.ErrorEventArgs]) {
            $State.queue.Enqueue([pscustomobject]@{ path=''; old_path=''; type='WatcherError'; at=[DateTimeOffset]::UtcNow.ToString('o'); detail=$args.GetException().Message; raw_event=$observation })
        } else {
            $type = ($eventRecord.SourceIdentifier -split '-',3)[1]
            $oldPath = if ($args -is [IO.RenamedEventArgs]) { $args.OldFullPath } else { '' }
            $State.queue.Enqueue([pscustomobject]@{ path=$args.FullPath; old_path=$oldPath; type=$type; at=[DateTimeOffset]::UtcNow.ToString('o'); detail=''; raw_event=$observation })
        }
        Remove-Event -EventIdentifier $eventRecord.EventIdentifier -ErrorAction SilentlyContinue
    }
}

function Stop-WatchState([object]$State) {
    if ($null -eq $State) { return }
    if ($State.PSObject.Properties['nativeCollector']) {
        if (-not $State.nativeStopped) {
            $State.nativeCollector.Dispose()
            $State.nativeStopped=$true
        }
        if (-not $State.PSObject.Properties['nativeControl']) { Receive-NativeBuildWatchEvents $State }
        return
    }
    foreach ($watcher in $State.watchers) { try { $watcher.EnableRaisingEvents = $false } catch { } }
    Start-Sleep -Milliseconds 120
    Receive-WatcherEvents $State
    foreach ($watcher in $State.watchers) { try { $watcher.Dispose() } catch { } }
    foreach ($id in $State.subscriptions) {
        Unregister-Event -SourceIdentifier $id -ErrorAction SilentlyContinue
        Get-Job -Name $id -ErrorAction SilentlyContinue | Remove-Job -Force -ErrorAction SilentlyContinue
    }
}

function Write-JsonLine([string]$Path, [object]$Value) {
    [IO.File]::AppendAllText($Path, (ConvertTo-Json -InputObject $Value -Compress -Depth 8) + "`n", $script:Utf8NoBom)
}

function Stop-CargoProcessTree([Diagnostics.Process]$Process) {
    if ($null -eq $Process) { return }
    try { if (-not $Process.HasExited) { & "$env:WINDIR\System32\taskkill.exe" /PID $Process.Id /T /F 2>$null | Out-Null; $Process.WaitForExit() } } catch { }
}

function Assert-NativeCommandDeadline([object]$Deadline, [string]$CaseId) {
    if ($null -ne $Deadline -and [DateTimeOffset]::UtcNow -ge [DateTimeOffset]$Deadline) {
        throw "VoxPopuli command exceeded its bounded parent deadline: $CaseId"
    }
}

function New-VoxPopuliProcessFamily([Diagnostics.Process]$Process) {
    $entries = @{}
    $entries[[int]$Process.Id] = [pscustomobject]@{ process=$Process; createdUtc=$Process.StartTime.ToUniversalTime(); root=$true; scannedAfterExit=$false }
    return [pscustomobject]@{ root=$Process; entries=$entries; lastScanUtc=[DateTimeOffset]::MinValue; streamDeadline=$null; lastQueryFilter=$null; cleanupPasses=0; cleanupClosed=$false; cleanupDeadlineUtc=$null }
}

function Assert-VoxPopuliCleanupDeadline([object]$Deadline) {
    if ($null -ne $Deadline -and [DateTimeOffset]::UtcNow -ge [DateTimeOffset]$Deadline) { throw 'VoxPopuli descendant cleanup exhausted its global 10 second deadline.' }
}

function Update-VoxPopuliProcessFamily([object]$Family, [switch]$Force, [scriptblock]$QueryForSelfTest = $null, [object]$CleanupDeadline = $null) {
    if ($null -eq $Family) { return }
    $now = [DateTimeOffset]::UtcNow
    if (-not $Force -and ($now - $Family.lastScanUtc).TotalSeconds -lt 1) { return }
    $knownParentIds = @($Family.entries.GetEnumerator() | Where-Object { -not $_.Value.process.HasExited -or -not $_.Value.scannedAfterExit } | ForEach-Object { [int]$_.Key } | Sort-Object)
    for ($offset = 0; $offset -lt $knownParentIds.Count; $offset += 32) {
        Assert-VoxPopuliCleanupDeadline $CleanupDeadline
        $parentIds = @($knownParentIds | Select-Object -Skip $offset -First 32)
        $exitedBeforeQuery = [Collections.Generic.HashSet[int]]::new()
        foreach ($parentId in $parentIds) {
            $entry = $Family.entries[$parentId]
            if ($entry.process.HasExited) { [void]$exitedBeforeQuery.Add($parentId) }
            elseif ([Math]::Abs(($entry.process.StartTime.ToUniversalTime() - $entry.createdUtc).TotalMilliseconds) -gt 1) { throw 'Observed parent identity changed before the descendant query.' }
        }
        $filter = [string]::Join(' OR ',[string[]]@($parentIds | ForEach-Object { 'ParentProcessId = ' + $_ }))
        $Family.lastQueryFilter = $filter
        try {
            $snapshot = if ($null -ne $QueryForSelfTest) { @(& $QueryForSelfTest $filter) } else { @(Get-CimInstance -ClassName Win32_Process -Filter $filter -Property ProcessId,ParentProcessId,CreationDate -OperationTimeoutSec 2 -ErrorAction Stop) }
        } catch { throw "VoxPopuli descendant query failed for known parent identities [$filter]: $($_.Exception.Message)" }
        foreach ($row in $snapshot) {
            $processId = [int]$row.ProcessId
            $parentId = [int]$row.ParentProcessId
            if ($Family.entries.ContainsKey($processId) -or $parentIds -notcontains $parentId -or $null -eq $row.CreationDate) { continue }
            $createdUtc = ([DateTime]$row.CreationDate).ToUniversalTime()
            if ($createdUtc -lt $Family.entries[$parentId].createdUtc) { continue }
            $observed = $null
            try {
                $observed = [Diagnostics.Process]::GetProcessById($processId)
                [void]$observed.Handle
                if ($observed.HasExited -or [Math]::Abs(($observed.StartTime.ToUniversalTime() - $createdUtc).TotalMilliseconds) -gt 1) { $observed.Dispose(); continue }
                $Family.entries[$processId] = [pscustomobject]@{ process=$observed; createdUtc=$createdUtc; root=$false; scannedAfterExit=$false }
            } catch [ArgumentException] {
                if ($null -ne $observed) { $observed.Dispose() }
            } catch [InvalidOperationException] {
                if ($null -ne $observed) { $observed.Dispose() }
            }
        }
        foreach ($parentId in $parentIds) { if ($exitedBeforeQuery.Contains($parentId)) { $Family.entries[$parentId].scannedAfterExit = $true } }
        Assert-VoxPopuliCleanupDeadline $CleanupDeadline
    }
    $Family.lastScanUtc = [DateTimeOffset]::UtcNow
}

function Test-VoxPopuliCompletionPending([object]$Family, [object]$Deadline, [string]$CaseId, [bool]$StdoutEof, [bool]$StderrEof) {
    Assert-NativeCommandDeadline $Deadline $CaseId
    Update-VoxPopuliProcessFamily $Family
    Assert-NativeCommandDeadline $Deadline $CaseId
    if ($Family.root.HasExited) {
        if ($null -eq $Family.streamDeadline) {
            Update-VoxPopuliProcessFamily $Family -Force
            $Family.streamDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        }
        Assert-NativeCommandDeadline $Deadline $CaseId
        if ($StdoutEof -and $StderrEof) { return $false }
        if ([DateTimeOffset]::UtcNow -ge $Family.streamDeadline) { throw 'VoxPopuli stdout/stderr capture did not reach EOF before its 10 second deadline.' }
    }
    return $true
}

function Stop-VoxPopuliKnownProcesses([object]$Family, [Collections.Generic.List[string]]$Errors, [DateTimeOffset]$Deadline) {
    $entries = @($Family.entries.Values | Sort-Object -Property createdUtc -Descending)
    foreach ($entry in $entries) {
        try { if (-not $entry.process.HasExited) { $entry.process.Kill() } }
        catch { $Errors.Add('Termination failed for observed process ' + $entry.process.Id + ': ' + $_.Exception.Message) }
    }
    foreach ($entry in $entries) {
        try {
            $remaining = [Math]::Max(0,[Math]::Min(5000,($Deadline - [DateTimeOffset]::UtcNow).TotalMilliseconds))
            if (-not $entry.process.HasExited -and -not $entry.process.WaitForExit([int]$remaining)) { $Errors.Add('Observed process survived the bounded cleanup: ' + $entry.process.Id) }
        } catch { $Errors.Add('Termination verification failed: ' + $_.Exception.Message) }
    }
}

function Stop-VoxPopuliProcessFamily([object]$Family, [scriptblock]$QueryForSelfTest = $null) {
    if ($null -eq $Family) { return }
    $errors = [Collections.Generic.List[string]]::new()
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
    $Family.cleanupDeadlineUtc = $deadline
    $Family.cleanupPasses = 0
    $Family.cleanupClosed = $false
    while ($true) {
        $Family.cleanupPasses++
        Stop-VoxPopuliKnownProcesses $Family $errors $deadline
        try { Assert-VoxPopuliCleanupDeadline $deadline }
        catch { $errors.Add($_.Exception.Message); break }
        $pendingParents = @($Family.entries.Values | Where-Object { -not $_.process.HasExited -or -not $_.scannedAfterExit })
        if ($pendingParents.Count -eq 0) { $Family.cleanupClosed = $true; break }
        try { Update-VoxPopuliProcessFamily $Family -Force -QueryForSelfTest $QueryForSelfTest -CleanupDeadline $deadline }
        catch {
            $errors.Add('Final descendant observation failed after stopping known handles: ' + $_.Exception.Message)
            Stop-VoxPopuliKnownProcesses $Family $errors $deadline
            break
        }
    }
    if (-not $Family.cleanupClosed) { $errors.Add('Descendant cleanup could not close every observed parent identity within its global bound.') }
    if ($errors.Count -gt 0) { throw [string]::Join('; ',$errors.ToArray()) }
}

function Close-VoxPopuliProcessFamily([object]$Family) {
    if ($null -eq $Family) { return }
    foreach ($entry in @($Family.entries.Values)) { if (-not $entry.root) { $entry.process.Dispose() } }
}

function Invoke-VoxPopuliLateDescendantSelfTest([string]$Lot) {
    $root = Join-Path $Lot ('selftest-voxpopuli-late-descendants-' + [guid]::NewGuid().ToString('N'))
    foreach ($name in @('temp','tmp','localappdata','appdata')) { New-Item -ItemType Directory -Path (Join-Path $root $name) -Force | Out-Null }
    $powershellExe = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $cmdExe = Join-Path $env:SystemRoot 'System32\cmd.exe'
    $descendantReady = Join-Path $root 'descendants-ready.json'
    $grandchildReady = Join-Path $root 'grandchild-ready.pid'
    $escapedExe = $powershellExe.Replace("'","''")
    $escapedRoot = $root.Replace("'","''")
    $escapedReady = $descendantReady.Replace("'","''")
    $escapedGrandchildReady = $grandchildReady.Replace("'","''")
    $grandchildScript = "[IO.File]::WriteAllText('${escapedGrandchildReady}.part',[string]`$PID); [IO.File]::Move('${escapedGrandchildReady}.part','$escapedGrandchildReady'); Start-Sleep -Seconds 60"
    $grandchildEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($grandchildScript))
    $childScript = "`$info=New-Object Diagnostics.ProcessStartInfo; `$info.FileName='$escapedExe'; `$info.Arguments='-NoLogo -NoProfile -NonInteractive -EncodedCommand $grandchildEncoded'; `$info.WorkingDirectory='$escapedRoot'; `$info.UseShellExecute=`$false; `$info.CreateNoWindow=`$true; `$grandchild=[Diagnostics.Process]::Start(`$info); [IO.File]::WriteAllText('${escapedReady}.part',([ordered]@{childId=`$PID;grandchildId=`$grandchild.Id} | ConvertTo-Json)); [IO.File]::Move('${escapedReady}.part','$escapedReady'); Start-Sleep -Seconds 60"
    $childEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($childScript))
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = $cmdExe
    $psi.Arguments = '/d /q /s /c ""{0}" -NoLogo -NoProfile -NonInteractive -EncodedCommand {1}"' -f $powershellExe,$childEncoded
    $psi.WorkingDirectory = $root
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.EnvironmentVariables.Clear()
    foreach ($name in @('SystemRoot','windir','COMSPEC')) { $value = [Environment]::GetEnvironmentVariable($name,'Process'); if ($value) { $psi.EnvironmentVariables[$name] = $value } }
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA')) { $psi.EnvironmentVariables[$name] = Join-Path $root $name.ToLowerInvariant() }
    foreach ($name in @('USERPROFILE','HOME')) { $psi.EnvironmentVariables[$name] = $root }
    $psi.EnvironmentVariables['PATH'] = (Join-Path $env:SystemRoot 'System32') + ';' + (Split-Path -Parent $powershellExe)
    $parent = New-Object Diagnostics.Process
    $parent.StartInfo = $psi
    $family = $null
    $fixtureHandles = [Collections.Generic.List[Diagnostics.Process]]::new()
    $result = $null
    $failure = $null
    try {
        if (-not $parent.Start()) { throw 'Late-descendant fixture parent did not start.' }
        $family = New-VoxPopuliProcessFamily $parent
        $startupDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        while (-not (Test-Path -LiteralPath $descendantReady -PathType Leaf) -or -not (Test-Path -LiteralPath $grandchildReady -PathType Leaf)) {
            if ($parent.HasExited -or [DateTimeOffset]::UtcNow -ge $startupDeadline) { throw 'Three-generation fixture did not become ready before its startup deadline.' }
            Start-Sleep -Milliseconds 20
        }
        $ready = Get-Content -LiteralPath $descendantReady -Raw | ConvertFrom-Json -ErrorAction Stop
        if ([int][IO.File]::ReadAllText($grandchildReady) -ne [int]$ready.grandchildId) { throw 'Three-generation fixture grandchild identity handshake differs.' }
        foreach ($processId in @([int]$ready.childId,[int]$ready.grandchildId)) {
            $observed = [Diagnostics.Process]::GetProcessById($processId)
            [void]$observed.Handle
            $fixtureHandles.Add($observed)
            if ($observed.HasExited -or $observed.StartTime.ToUniversalTime() -lt $family.entries[[int]$parent.Id].createdUtc) { throw 'Three-generation fixture descendant identity is not current.' }
        }
        if ($family.entries.Count -ne 1) { throw 'Late-descendant fixture must begin cleanup knowing only its root.' }
        Stop-VoxPopuliProcessFamily $family
        if (-not $family.cleanupClosed -or $family.cleanupPasses -lt 3 -or -not $family.entries.ContainsKey([int]$ready.childId) -or -not $family.entries.ContainsKey([int]$ready.grandchildId)) { throw 'Cleanup did not discover and close two previously unknown descendant generations.' }
        if (@($family.entries.Values | Where-Object { -not $_.process.HasExited }).Count -ne 0 -or @($fixtureHandles | Where-Object { -not $_.HasExited }).Count -ne 0) { throw 'A three-generation fixture descendant survived cleanup.' }
        $result = [pscustomobject]@{fixture='three-generations-two-late-levels';knownBeforeCleanup=1;lateLevelsDiscovered=2;cleanupPasses=$family.cleanupPasses;cleanupClosed=$family.cleanupClosed;allObservedProcessesExited=$true;globalCleanupBoundSeconds=10;nativeOrCargoStarted=$false}
        return $result
    } catch {
        $failure = $_.Exception.Message
        throw
    } finally {
        try {
            foreach ($observed in @($fixtureHandles | Sort-Object -Property StartTime -Descending)) { if (-not $observed.HasExited) { $observed.Kill() } }
            foreach ($observed in @($fixtureHandles)) { if (-not $observed.HasExited -and -not $observed.WaitForExit(1000)) { throw 'Controlled fixture descendant survived fallback cleanup.' } }
            if ($null -ne $family -and -not $parent.HasExited) { Stop-VoxPopuliProcessFamily $family }
            [ordered]@{fixture='three-generations-two-late-levels';failure=$failure;result=$result;nativeOrCargoStarted=$false} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $root 'fixture-trace.json') -Encoding UTF8
        } finally {
            foreach ($observed in @($fixtureHandles)) { $observed.Dispose() }
            Close-VoxPopuliProcessFamily $family
            $parent.Dispose()
        }
    }
}

function Invoke-VoxPopuliDeadlineProcessSelfTest([string]$Lot, [bool]$ParentExits, [switch]$InjectCimFailure) {
    $kind = if ($InjectCimFailure) { 'cim-failure-known-handles' } elseif ($ParentExits) { 'exited-parent-open-streams' } else { 'live-parent' }
    $root = Join-Path $Lot ('selftest-voxpopuli-' + $kind + '-' + [guid]::NewGuid().ToString('N'))
    foreach ($name in @('temp','tmp','localappdata','appdata')) { New-Item -ItemType Directory -Path (Join-Path $root $name) -Force | Out-Null }
    $powershellExe = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $cmdExe = Join-Path $env:SystemRoot 'System32\cmd.exe'
    $childReadyPath = Join-Path $root 'child-ready.json'
    $childPidPath = Join-Path $root 'child.pid'
    $escapedReadyPath = $childReadyPath.Replace("'","''")
    $escapedPidPath = $childPidPath.Replace("'","''")
    $childScript = "[IO.File]::WriteAllText('${escapedPidPath}.part',[string]`$PID); [IO.File]::Move('${escapedPidPath}.part','$escapedPidPath'); `$outOk=`$false; `$errOk=`$false; `$errors=@(); try { [Console]::Out.WriteLine('N2_DEADLINE_CHILD_STDOUT'); [Console]::Out.Flush(); `$outOk=`$true } catch { `$errors += `$_.Exception.ToString() }; try { [Console]::Error.WriteLine('N2_DEADLINE_CHILD_STDERR'); [Console]::Error.Flush(); `$errOk=`$true } catch { `$errors += `$_.Exception.ToString() }; `$ready=[ordered]@{processId=`$PID;stdoutEmissionCompleted=`$outOk;stderrEmissionCompleted=`$errOk;stdoutRedirected=[Console]::IsOutputRedirected;stderrRedirected=[Console]::IsErrorRedirected;errors=`$errors}; [IO.File]::WriteAllText('${escapedReadyPath}.part',(`$ready | ConvertTo-Json -Depth 4)); [IO.File]::Move('${escapedReadyPath}.part','$escapedReadyPath'); Start-Sleep -Seconds 60"
    $childEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($childScript))
    $releasePath = Join-Path $root 'release-parent'
    $escapedReleasePath = $releasePath.Replace("'","''")
    $parentEnd = if ($ParentExits) { "while (-not [IO.File]::Exists('$escapedReleasePath')) { Start-Sleep -Milliseconds 20 }; exit 0" } else { 'Start-Sleep -Seconds 60' }
    $gateEncoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($parentEnd))
    # CMD's /b launch preserves the redirected standard handles; the diagnostic
    # proved that an unredirected ProcessStartInfo child received console handles.
    $parentArguments = '/d /q /s /c "start "" /b "{0}" -NoLogo -NoProfile -NonInteractive -EncodedCommand {1} & "{0}" -NoLogo -NoProfile -NonInteractive -EncodedCommand {2}"' -f $powershellExe,$childEncoded,$gateEncoded
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = $cmdExe
    $psi.Arguments = $parentArguments
    $psi.WorkingDirectory = $root
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.EnvironmentVariables.Clear()
    foreach ($name in @('SystemRoot','windir','COMSPEC')) { $value = [Environment]::GetEnvironmentVariable($name,'Process'); if ($value) { $psi.EnvironmentVariables[$name] = $value } }
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA')) { $psi.EnvironmentVariables[$name] = Join-Path $root $name.ToLowerInvariant() }
    foreach ($name in @('USERPROFILE','HOME')) { $psi.EnvironmentVariables[$name] = $root }
    $psi.EnvironmentVariables['PATH'] = (Join-Path $env:SystemRoot 'System32') + ';' + (Split-Path -Parent $powershellExe)
    $parent = New-Object Diagnostics.Process
    $parent.StartInfo = $psi
    $family = $null
    $capture = $null
    $queue = [Collections.Concurrent.ConcurrentQueue[object]]::new()
    $stdoutLines = [Collections.Generic.List[string]]::new()
    $stderrLines = [Collections.Generic.List[string]]::new()
    $stdoutEof = $false
    $stderrEof = $false
    $stdoutWriter = [IO.StreamWriter]::new((Join-Path $root 'stdout.log'),$false,$script:Utf8NoBom)
    $stderrWriter = [IO.StreamWriter]::new((Join-Path $root 'stderr.log'),$false,$script:Utf8NoBom)
    $stage = 'initializing'
    $failure = $null
    $cleanupFailure = $null
    $childReady = $null
    $deadlineArmed = $false
    $timedOut = $false
    $observedExitedParentWithOpenStreams = $false
    $familyCleanupComplete = $false
    $simulatedObservationFailure = $null
    [Console]::Out.WriteLine("N2_VOXPOPULI_DEADLINE_FIXTURE kind=$kind trace=$(Join-Path $root 'fixture-trace.json')")
    try {
        $capture = Register-ProcessStreamHandlers $parent $queue
        if (-not $parent.Start()) { throw 'Synthetic deadline parent did not start.' }
        $family = New-VoxPopuliProcessFamily $parent
        $parent.BeginOutputReadLine(); $parent.BeginErrorReadLine()
        $startupDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        while (-not (Test-Path -LiteralPath $childPidPath -PathType Leaf)) {
            if ($parent.HasExited -or [DateTimeOffset]::UtcNow -ge $startupDeadline) { throw 'Synthetic deadline child did not become observable.' }
            Start-Sleep -Milliseconds 20
        }
        $childPid = [int][IO.File]::ReadAllText($childPidPath)
        $stage = 'observing-child'
        while (-not $family.entries.ContainsKey($childPid)) {
            if ([DateTimeOffset]::UtcNow -ge $startupDeadline) { throw 'Synthetic deadline child was not tracked by process identity.' }
            Update-VoxPopuliProcessFamily $family -Force
        }
        $child = $family.entries[$childPid].process
        $stage = 'child-stream-handshake'
        $handshakeDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        while ($true) {
            Receive-ProcessOutput $queue $stdoutWriter $stderrWriter $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
            if ($null -eq $childReady -and (Test-Path -LiteralPath $childReadyPath -PathType Leaf)) { $childReady = Get-Content -LiteralPath $childReadyPath -Raw | ConvertFrom-Json -ErrorAction Stop }
            if ($null -ne $childReady -and (-not $childReady.stdoutRedirected -or -not $childReady.stderrRedirected)) { throw 'Deadline fixture child did not inherit both redirected standard streams.' }
            if ($null -ne $childReady -and $childReady.stdoutEmissionCompleted -and $childReady.stderrEmissionCompleted -and $stdoutLines -ccontains 'N2_DEADLINE_CHILD_STDOUT' -and $stderrLines -ccontains 'N2_DEADLINE_CHILD_STDERR') { break }
            if ($parent.HasExited -or $child.HasExited -or [DateTimeOffset]::UtcNow -ge $handshakeDeadline) { throw 'Deadline fixture handshake did not observe both completed child emissions in the captured streams before its 10 second deadline.' }
            Start-Sleep -Milliseconds 20
        }
        if ($stdoutEof -or $stderrEof) { throw 'Deadline fixture child did not retain both redirected streams after its handshake.' }
        if ($InjectCimFailure) {
            $stage = 'synthetic-cim-failure'
            $observationFailureReported = $false
            $failingObservation = { param($Filter) throw 'SYNTHETIC_CIM_OBSERVATION_FAILURE' }
            try { Update-VoxPopuliProcessFamily $family -Force -QueryForSelfTest $failingObservation }
            catch {
                if ($_.Exception.Message -cnotlike '*SYNTHETIC_CIM_OBSERVATION_FAILURE*') { throw }
                $observationFailureReported = $true
            }
            $probe = [pscustomobject]@{queryEntered=$false;knownHandlesStoppedBeforeQuery=$false;failureReported=$false}
            $failingFinalObservation = {
                param($Filter)
                $probe.queryEntered = $true
                $probe.knownHandlesStoppedBeforeQuery = @($family.entries.Values | Where-Object { -not $_.process.HasExited }).Count -eq 0
                if (-not $probe.knownHandlesStoppedBeforeQuery) { throw 'Synthetic final observation ran before known handles were stopped.' }
                throw 'SYNTHETIC_CIM_FINAL_OBSERVATION_FAILURE'
            }.GetNewClosure()
            try { Stop-VoxPopuliProcessFamily $family -QueryForSelfTest $failingFinalObservation }
            catch {
                if ($_.Exception.Message -cnotlike '*SYNTHETIC_CIM_FINAL_OBSERVATION_FAILURE*') { throw }
                $probe.failureReported = $true
            }
            if (-not $observationFailureReported -or -not $probe.queryEntered -or -not $probe.knownHandlesStoppedBeforeQuery -or -not $probe.failureReported -or -not $parent.HasExited -or -not $child.HasExited) { throw 'Synthetic CIM failure did not remain visible while known process handles were terminated.' }
            $familyCleanupComplete = $true
            $simulatedObservationFailure = [pscustomobject]@{observationFailureReported=$observationFailureReported;cleanup=$probe;queryFilter=$family.lastQueryFilter;realCimFailureInjected=$false}
            return [pscustomobject]@{fixture=$kind;simulatedCimFailure='PASS';observationErrorPropagated=$true;knownHandlesStoppedBeforeFinalQuery=$true;cleanupErrorPropagated=$true;nativeOrCargoStarted=$false}
        }
        if ($ParentExits) { [IO.File]::WriteAllText($releasePath,'release after child identity was retained') }
        $deadline = [DateTimeOffset]::UtcNow.AddSeconds(2)
        $deadlineArmed = $true
        $stage = 'deadline-armed'
        try {
            while (Test-VoxPopuliCompletionPending $family $deadline 'synthetic parent and child' $stdoutEof $stderrEof) {
                Receive-ProcessOutput $queue $stdoutWriter $stderrWriter $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
                if ($parent.HasExited -and -not ($stdoutEof -and $stderrEof)) { $observedExitedParentWithOpenStreams = $true }
                Start-Sleep -Milliseconds 20
            }
        } catch {
            if ($_.Exception.Message -cnotlike 'VoxPopuli command exceeded its bounded parent deadline:*') { throw }
            $timedOut = $true
            Stop-VoxPopuliProcessFamily $family
            $familyCleanupComplete = $true
        }
        if (-not $timedOut -or -not $parent.HasExited -or -not $child.HasExited) { throw 'Deadline cleanup did not terminate both synthetic parent and child.' }
        if ($ParentExits -and (-not $observedExitedParentWithOpenStreams -or $parent.ExitCode -ne 0)) { throw 'Redirected-stream fixture did not observe the parent exit normally while its child held the streams.' }
        Receive-ProcessOutput $queue $stdoutWriter $stderrWriter $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
        if ($stdoutLines -cnotcontains 'N2_DEADLINE_CHILD_STDOUT' -or $stderrLines -cnotcontains 'N2_DEADLINE_CHILD_STDERR') { throw 'Deadline fixture did not capture both inherited child streams.' }
        return [pscustomobject]@{fixture=$kind;parentDeadline='PASS';childTerminated=$child.HasExited;parentTerminated=$parent.HasExited;observedExitedParentWithOpenStreams=$observedExitedParentWithOpenStreams;childIdentityTracked=$true;nativeOrCargoStarted=$false}
    } catch {
        $failure = $_.Exception.Message
        throw
    } finally {
        try { if ($null -ne $family -and -not $familyCleanupComplete) { Stop-VoxPopuliProcessFamily $family } }
        catch { $cleanupFailure = $_.Exception.Message; throw }
        finally {
            try {
                $eofDeadline = [DateTimeOffset]::UtcNow.AddSeconds(1)
                do {
                    Receive-ProcessOutput $queue $stdoutWriter $stderrWriter $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
                    if ($stdoutEof -and $stderrEof) { break }
                    Start-Sleep -Milliseconds 20
                } while ([DateTimeOffset]::UtcNow -lt $eofDeadline)
                $jobErrors = @()
                if ($null -ne $capture) {
                    foreach ($job in @($capture.jobs)) {
                        $jobErrors += [pscustomobject]@{id=$job.Id;state=[string]$job.State;reason=[string]$job.JobStateInfo.Reason;errors=@($job.Error | ForEach-Object { [string]$_ });childJobErrors=@($job.ChildJobs | ForEach-Object { $_.Error } | ForEach-Object { [string]$_ })}
                    }
                }
                [ordered]@{
                    fixture=$kind;stage=$stage;failure=$failure;cleanupFailure=$cleanupFailure;deadlineArmed=$deadlineArmed;timedOut=$timedOut
                    childHandshake=$childReady;stdout=@($stdoutLines.ToArray());stderr=@($stderrLines.ToArray());stdoutEof=$stdoutEof;stderrEof=$stderrEof
                    parentRedirectsStdout=$psi.RedirectStandardOutput;parentRedirectsStderr=$psi.RedirectStandardError
                    parentExecutable=$psi.FileName;parentArguments=$psi.Arguments
                    simulatedObservationFailure=$simulatedObservationFailure
                    observedExitedParentWithOpenStreams=$observedExitedParentWithOpenStreams;eventJobs=$jobErrors;nativeOrCargoStarted=$false
                } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $root 'fixture-trace.json') -Encoding UTF8
            } finally {
                $stdoutWriter.Dispose(); $stderrWriter.Dispose()
                Remove-ProcessStreamHandlers $capture
                if ($null -ne $capture) { Assert-NoProcessStreamHandlers $capture }
                Close-VoxPopuliProcessFamily $family
                $parent.Dispose()
            }
        }
    }
}

function Invoke-VoxPopuliContractSelfTests([object]$Registry, [string]$Lot) {
    $ids = @('ParakeetGpu','ParakeetGpuFrenchShort','ParakeetGpuFrenchLong','ParakeetGpuEnglishLong','ParakeetGpuFrenchAutoLong','ParakeetGpuEnglishAutoLong','ParakeetGpuPreparation','ParakeetGpuInventory')
    foreach ($id in $ids) {
        $profile = @($Registry.profiles | Where-Object { $_.id -ceq $id })[0]
        Assert-VoxPopuliProfileContract $profile
        $wrongTimeout = ConvertFrom-Json (ConvertTo-Json -InputObject $profile -Depth 12)
        $wrongTimeout.commandTimeoutSeconds = 0
        Test-Rejects "VoxPopuli zero timeout $id" { Assert-VoxPopuliProfileContract $wrongTimeout }
        $wrongTimeout.commandTimeoutSeconds = (Get-VoxPopuliProfileContract $id).timeoutSeconds + 1
        Test-Rejects "VoxPopuli enlarged timeout $id" { Assert-VoxPopuliProfileContract $wrongTimeout }
    }
    Test-Rejects 'timeout added to an unrelated profile' { Get-NativeCommandTimeoutSeconds ([pscustomobject]@{id='TurboCpu';commandTimeoutSeconds=1200}) }
    Assert-NativeCommandDeadline $null 'unbounded historical profile'
    Assert-NativeCommandDeadline ([DateTimeOffset]::UtcNow.AddSeconds(10)) 'future deadline'
    Test-Rejects 'expired parent deadline' { Assert-NativeCommandDeadline ([DateTimeOffset]::UtcNow.AddSeconds(-1)) 'expired fixture' }
    Test-Rejects 'expired global descendant cleanup deadline' { Assert-VoxPopuliCleanupDeadline ([DateTimeOffset]::UtcNow.AddSeconds(-1)) }
    foreach ($id in $ids | Where-Object { (Get-VoxPopuliProfileContract $_).operation -ceq 'Run' }) {
        $runProfile = @($Registry.profiles | Where-Object { $_.id -ceq $id })[0]
        if ((Get-NativeCommandTimeoutSeconds $runProfile) -lt 1) { throw 'VoxPopuli Run deadline was not finite.' }
    }
    foreach ($discovery in @(
        [pscustomobject]@{id='ParakeetGpuPreparation';suffix=@('--no-run')},
        [pscustomobject]@{id='ParakeetGpuInventory';suffix=@('--','--list')}
    )) {
        $profile = @($Registry.profiles | Where-Object { $_.id -ceq $discovery.id })[0]
        $expected = @('test','--offline','--locked','-p','whisper-adapters','--features','whisper-adapters/native-tests','--test','parakeet_unified_native') + @($discovery.suffix)
        $actual = New-CargoArguments $profile $null
        if ([string]::Join("`n",[string[]]$actual) -cne [string]::Join("`n",[string[]]$expected)) { throw 'VoxPopuli discovery arguments must compile/list only the dedicated integration target.' }
    }
    $queryProbe = [pscustomobject]@{filters=[Collections.Generic.List[string]]::new()}
    $created = [DateTime]::UtcNow
    $syntheticFamily = [pscustomobject]@{
        entries=@{12345=[pscustomobject]@{process=[pscustomobject]@{HasExited=$false;StartTime=$created};createdUtc=$created;root=$true;scannedAfterExit=$false}}
        lastScanUtc=[DateTimeOffset]::MinValue;lastQueryFilter=$null
    }
    $emptyTargetedQuery = { param($Filter) $queryProbe.filters.Add([string]$Filter) }.GetNewClosure()
    Update-VoxPopuliProcessFamily $syntheticFamily -Force -QueryForSelfTest $emptyTargetedQuery
    if ($queryProbe.filters.Count -ne 1 -or $queryProbe.filters[0] -cne 'ParentProcessId = 12345') { throw 'VoxPopuli descendant query did not stay limited to known parent identities.' }
    $readyFixture = [pscustomobject]@{
        id='ParakeetGpu';status='READY';operation='Run';package='whisper-adapters';features=@('whisper-adapters/native-tests');compileEnvironment=@();commandTimeoutSeconds=1200
        inputs=@(
            [pscustomobject]@{key='personalRuntime';environmentVariable='WHISPER_N2_PERSONAL_RUNTIME_ROOT';kind='directory'},
            [pscustomobject]@{key='voxpopuliCorpus';environmentVariable='WHISPER_VOXPOPULI_CORPUS_ROOT';kind='directory'},
            [pscustomobject]@{key='voxpopuliSources';environmentVariable='WHISPER_VOXPOPULI_PROVENANCE_ROOT';kind='directory'}
        )
        values=@([pscustomobject]@{key='cudaToolkitPath';environmentVariable='CUDA_PATH';required=$true;kind='directory';allowedRoot='cuda-12.9'})
        outputs=@([pscustomobject]@{key='campaignRun';environmentVariable='WHISPER_L04_CAMPAIGN_RUN';kind='directory';path='writes/voxpopuli-campaign'})
        staging=@(
            [pscustomobject]@{inputKey='personalRuntime';destinationBase='localappdata';destination='Whisper';requiredFileCount=21;requiredBytes=3196008875},
            [pscustomobject]@{inputKey='voxpopuliCorpus';destinationBase='run';destination='writes/voxpopuli-corpus';requiredFileCount=14;requiredBytes=151356410}
        )
        cases=@([pscustomobject]@{targetKind='integration';targetName='parakeet_unified_native';exactTest='voxpopuli::english_short_mp3_gpu';ignored=$false;verdict='QUALIFICATION'})
    }
    Assert-VoxPopuliProfileContract $readyFixture
    foreach ($mutation in @('selector','staging','environment','feature')) {
        $invalid = ConvertFrom-Json (ConvertTo-Json -InputObject $readyFixture -Depth 12)
        switch ($mutation) {
            'selector' { $invalid.cases[0].exactTest = 'voxpopuli::unapproved' }
            'staging' { $invalid.staging[0].destination = 'OtherRuntime' }
            'environment' { $invalid.inputs[0].environmentVariable = 'WHISPER_UNDECLARED' }
            'feature' { $invalid.features = @('whisper-adapters/recette-hooks') }
        }
        Test-Rejects "VoxPopuli modified $mutation contract" { Assert-VoxPopuliProfileContract $invalid }
    }
    $deadlineFixtures = @(
        (Invoke-VoxPopuliDeadlineProcessSelfTest $Lot $false),
        (Invoke-VoxPopuliDeadlineProcessSelfTest $Lot $true),
        (Invoke-VoxPopuliDeadlineProcessSelfTest $Lot $false -InjectCimFailure),
        (Invoke-VoxPopuliLateDescendantSelfTest $Lot)
    )
    return [pscustomobject]@{profiles=$ids;exactDeadlines='PASS';unknownTimeoutRejected='PASS';targetedDescendantQueries='PASS';discoveryIntegrationTargetOnly='PASS';deadlineFixtures=$deadlineFixtures;nativeOrCargoStarted=$false}
}

function Drain-WatcherEvents([object]$State, [string[]]$AllowedRoots, [string[]]$ProtectedRoots, [string]$UserTemp, [Diagnostics.Process]$Process, [string]$Mode, [string]$LogRoot, [object]$ProcessFamily = $null) {
    if ($Mode -ceq 'NativeBuild' -and $State.PSObject.Properties['nativeControl']) {
        $violation=$State.nativeControl.Process($State.nativeCollector.Drain())
        if ($null -ne $violation) {
            if ($null -ne $ProcessFamily) { Stop-VoxPopuliProcessFamily $ProcessFamily } else { Stop-CargoProcessTree $Process }
            throw "Native campaign guard stopped on $($violation.Type) at '$($violation.Path)' (event $($violation.At)); see $LogRoot\guard-events.jsonl"
        }
        return
    }
    Receive-WatcherEvents $State
    $event = $null
    while ($State.queue.TryDequeue([ref]$event)) {
        $disposition = Get-EventDisposition $event $AllowedRoots $ProtectedRoots $UserTemp
        $rawEvent = if ($event.PSObject.Properties.Name -contains 'raw_event') { $event.raw_event } else { $null }
        if ($Mode -ceq 'NativeBuild') {
            Write-JsonLine (Join-Path $LogRoot 'watcher-events.jsonl') ([ordered]@{mode=$Mode;path=$event.path;old_path=$event.old_path;event=$event.type;event_at=$event.at;disposition=$disposition;raw_event=$rawEvent})
        }
        if ($disposition -eq 'UNATTRIBUTED_TEMP') {
            Write-JsonLine (Join-Path $LogRoot 'unattributed-temp.jsonl') ([ordered]@{ mode=$Mode; path=$event.path; old_path=$event.old_path; event=$event.type; event_at=$event.at; detected_at=[DateTimeOffset]::UtcNow.ToString('o'); attribution='UNATTRIBUTED'; detail=$event.detail; raw_event=$rawEvent })
            $event = $null
            continue
        }
        if ($disposition -eq 'VIOLATION') {
            $record = [ordered]@{ mode=$Mode; status='FAIL'; path=$event.path; old_path=$event.old_path; event=$event.type; event_at=$event.at; detected_at=[DateTimeOffset]::UtcNow.ToString('o'); action='campaign-process-tree-terminated'; detail=$event.detail; raw_event=$rawEvent }
            Write-JsonLine (Join-Path $LogRoot 'guard-events.jsonl') $record
            if ($null -ne $ProcessFamily) { Stop-VoxPopuliProcessFamily $ProcessFamily } else { Stop-CargoProcessTree $Process }
            throw "Native campaign guard stopped on $($event.type) at '$($event.path)' (event $($event.at)); see $LogRoot\guard-events.jsonl"
        }
        $event = $null
    }
}

function Get-GitState([string]$Repository) {
    $head = @(& git.exe -C $Repository --no-optional-locks rev-parse HEAD)
    if ($LASTEXITCODE -ne 0) { throw 'Could not read candidate Git HEAD with --no-optional-locks.' }
    $status = @(& git.exe -C $Repository --no-optional-locks status --porcelain=v1 -uall)
    if ($LASTEXITCODE -ne 0) { throw 'Could not read candidate Git status with --no-optional-locks.' }
    return [pscustomobject]@{ head=($head -join "`n"); status=($status -join "`n") }
}

function Initialize-CargoCache([string]$Lot) {
    $cargoHome = Join-Path $Lot 'cargo-home'
    $source = Join-Path $env:USERPROFILE '.cargo'
    Assert-NoReparseChain $source 'source Cargo cache'
    if (-not (Test-Path -LiteralPath $source -PathType Container)) { throw "Offline Cargo cache source is missing: $source" }
    foreach ($name in @('registry','git')) {
        $destination = Join-Path $cargoHome $name
        $origin = Join-Path $source $name
        if (Test-Path -LiteralPath $destination) {
            if (-not (Test-Path -LiteralPath $destination -PathType Container)) { throw "Cargo cache destination is not a directory: $destination" }
        } else {
            if (-not (Test-Path -LiteralPath $origin -PathType Container)) { if ($name -eq 'git') { continue }; throw "Offline Cargo cache component is missing: $origin" }
            Assert-NoReparseChain $origin 'source Cargo cache component'
            Copy-Item -LiteralPath $origin -Destination $destination -Recurse -Force
        }
    }
    foreach ($name in @('.global-cache','.package-cache','.package-cache-mutate','.crates.toml','.crates2.json')) {
        $origin = Join-Path $source $name
        $destination = Join-Path $cargoHome $name
        if (-not (Test-Path -LiteralPath $destination) -and (Test-Path -LiteralPath $origin -PathType Leaf)) { Copy-Item -LiteralPath $origin -Destination $destination -Force }
    }
    if (-not (Test-Path -LiteralPath (Join-Path $cargoHome 'registry\src') -PathType Container)) { throw 'Isolated CARGO_HOME has no cached registry sources for offline builds.' }
    Assert-NoReparseChain $cargoHome 'isolated CARGO_HOME'
}

function New-CargoProcessInfo([string]$CargoExe, [string]$RustcExe, [string]$ToolchainBin, [string[]]$CargoArgs, [hashtable]$Environment, [hashtable]$VsEnvironment, [string]$RunRoot, [string]$ProfileId) {
    $psi = New-Object Diagnostics.ProcessStartInfo
    $psi.FileName = $CargoExe
    $psi.WorkingDirectory = $script:Repo
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Arguments = [string]::Join(' ', [string[]]@($CargoArgs | ForEach-Object { Get-WindowsCommandLineArgument ([string]$_) }))
    $vars = $psi.EnvironmentVariables
    $vars.Clear()
    foreach ($name in @('SystemRoot','WINDIR','SystemDrive','ComSpec','PATHEXT','OS','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS')) {
        $value = [Environment]::GetEnvironmentVariable($name)
        if (-not [string]::IsNullOrWhiteSpace($value)) { $vars[$name] = $value }
    }
    foreach ($entry in $VsEnvironment.GetEnumerator()) { if ($entry.Key -cne 'PATH' -and $entry.Key -cne 'ProgramW6432') { $vars[[string]$entry.Key] = [string]$entry.Value } }
    $path = [Collections.Generic.List[string]]::new()
    $path.Add($ToolchainBin)
    $path.Add((Join-Path $env:SystemRoot 'System32'))
    $path.Add($env:SystemRoot)
    foreach ($part in ([string]$VsEnvironment.PATH).Split(';')) {
        if ([string]::IsNullOrWhiteSpace($part)) { continue }
        foreach ($root in $script:AllowedToolRoots) {
            if (Test-InRoot $part $root) { if (-not $path.Contains($part)) { $path.Add($part) }; break }
        }
    }
    if ($Environment.ContainsKey('CUDA_PATH')) {
        $cudaBin = Join-Path ([string]$Environment.CUDA_PATH) 'bin'
        if (Test-Path -LiteralPath $cudaBin -PathType Container) { $path.Add($cudaBin) }
    }
    $vars['PATH'] = [string]::Join(';',$path.ToArray())
    $protectedNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($name in @('CARGO_HOME','CARGO_TARGET_DIR','CARGO_INCREMENTAL','CARGO_PROFILE_DEV_DEBUG','TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME','RUSTC','PATH')) { [void]$protectedNames.Add($name) }
    foreach ($entry in $Environment.GetEnumerator()) {
        $name = [string]$entry.Key
        if ($protectedNames.Contains($name)) { continue }
        if ($name -ceq 'ProgramW6432') {
            if (-not (Test-ProgramW6432ProfileId $ProfileId)) { throw "Profile $ProfileId is not authorized to receive ProgramW6432." }
            $validatedProgramW = Get-ValidatedProgramW6432Directory
            if ([string]$entry.Value -ine $validatedProgramW) { throw 'Profile ProgramW6432 differs from the validated host directory.' }
            $vars['ProgramW6432'] = $validatedProgramW
            continue
        }
        if ($name -notmatch '^(?:WHISPER_|PARAKEET_)[A-Z0-9_]+$' -and -not $script:AllowedProfileEnvironmentNames.Contains($name)) { throw "Environment variable is not declared by an approved profile: $name" }
        $vars[$name] = [string]$entry.Value
    }
    $vars['RUSTC'] = $RustcExe
    $vars['CARGO_HOME'] = [string]$Environment.CARGO_HOME
    $vars['CARGO_TARGET_DIR'] = $script:CargoTarget
    $vars['CARGO_INCREMENTAL'] = '0'
    $vars['CARGO_PROFILE_DEV_DEBUG'] = 'line-tables-only'
    $vars['TEMP'] = [string]$Environment.TEMP
    $vars['TMP'] = [string]$Environment.TMP
    $vars['LOCALAPPDATA'] = [string]$Environment.LOCALAPPDATA
    $vars['APPDATA'] = [string]$Environment.APPDATA
    $vars['USERPROFILE'] = [string]$Environment.USERPROFILE
    $vars['HOME'] = [string]$Environment.HOME
    if (Test-ProgramW6432ProfileId $ProfileId) {
        if (-not $vars.ContainsKey('ProgramW6432') -or [string]$vars['ProgramW6432'] -ine (Get-ValidatedProgramW6432Directory)) { throw "Profile $ProfileId did not pass the validated ProgramW6432 value to Cargo." }
    } elseif ($vars.ContainsKey('ProgramW6432')) { throw "Profile $ProfileId unexpectedly received ProgramW6432." }
    return $psi
}

function Register-ProcessStreamHandlers([Diagnostics.Process]$Process, [Collections.Concurrent.ConcurrentQueue[object]]$Queue, [int]$FailAfterRegistrationForSelfTest = 0) {
    if ($FailAfterRegistrationForSelfTest -lt 0 -or $FailAfterRegistrationForSelfTest -gt 1) { throw 'Invalid stream-registration self-test fault selector.' }
    $sourceIds = [Collections.Generic.List[string]]::new()
    $jobs = [Collections.Generic.List[object]]::new()
    $capture = [pscustomobject]@{ sourceIds=$sourceIds; jobs=$jobs }
    try {
        foreach ($stream in @('stdout','stderr')) {
            $sourceId = 'n2-stream-' + [guid]::NewGuid().ToString('N')
            $sourceIds.Add($sourceId)
            $messageData = @{ queue=$Queue; stream=$stream }
            $action = {
                $streamName = [string]$Event.MessageData.stream
                $data = $Event.SourceEventArgs.Data
                if ($null -ne $data) { $Event.MessageData.queue.Enqueue([pscustomobject]@{ stream=$streamName; line=$data }) }
                else { $Event.MessageData.queue.Enqueue([pscustomobject]@{ stream=($streamName + '-eof'); line='' }) }
            }.GetNewClosure()
            $job = Register-ObjectEvent -InputObject $Process -EventName $(if ($stream -eq 'stdout') { 'OutputDataReceived' } else { 'ErrorDataReceived' }) -SourceIdentifier $sourceId -MessageData $messageData -Action $action
            if ($null -ne $job) { $jobs.Add($job) }
            if ($FailAfterRegistrationForSelfTest -gt 0 -and $sourceIds.Count -eq $FailAfterRegistrationForSelfTest) { throw 'Synthetic stream registration failure for cleanup self-test.' }
        }
        return $capture
    } catch {
        Remove-ProcessStreamHandlers $capture
        throw
    }
}

function Remove-ProcessStreamHandlers([object]$Capture) {
    if ($null -eq $Capture) { return }
    foreach ($sourceId in @($Capture.sourceIds)) {
        if (-not [string]::IsNullOrWhiteSpace([string]$sourceId)) { Unregister-Event -SourceIdentifier ([string]$sourceId) -ErrorAction SilentlyContinue }
    }
    foreach ($job in @($Capture.jobs)) {
        if ($null -ne $job -and $job -is [System.Management.Automation.Job]) { Remove-Job -Job $job -Force -ErrorAction SilentlyContinue }
    }
    foreach ($sourceId in @($Capture.sourceIds)) {
        if ([string]::IsNullOrWhiteSpace([string]$sourceId)) { continue }
        Get-Job -Name ([string]$sourceId) -ErrorAction SilentlyContinue | Remove-Job -Force -ErrorAction SilentlyContinue
    }
}

function Assert-NoProcessStreamHandlers([object]$Capture) {
    foreach ($sourceId in @($Capture.sourceIds)) {
        if (@(Get-EventSubscriber -SourceIdentifier ([string]$sourceId) -ErrorAction SilentlyContinue).Count -gt 0) { throw "Process stream event subscription leaked: $sourceId" }
        if (@(Get-Job -Name ([string]$sourceId) -ErrorAction SilentlyContinue).Count -gt 0) { throw "Process stream event job leaked: $sourceId" }
    }
}

function Receive-ProcessOutput([Collections.Concurrent.ConcurrentQueue[object]]$Queue, [IO.StreamWriter]$StdoutWriter, [IO.StreamWriter]$StderrWriter, [Collections.Generic.List[string]]$StdoutLines, [Collections.Generic.List[string]]$StderrLines, [ref]$StdoutEof, [ref]$StderrEof) {
    $record = $null
    while ($Queue.TryDequeue([ref]$record)) {
        switch ([string]$record.stream) {
            'stdout' { if ($null -ne $StdoutWriter) { $StdoutWriter.WriteLine([string]$record.line); $StdoutWriter.Flush() }; if ($null -ne $StdoutLines) { $StdoutLines.Add([string]$record.line) } }
            'stderr' { if ($null -ne $StderrWriter) { $StderrWriter.WriteLine([string]$record.line); $StderrWriter.Flush() }; if ($null -ne $StderrLines) { $StderrLines.Add([string]$record.line) } }
            'stdout-eof' { $StdoutEof.Value = $true }
            'stderr-eof' { $StderrEof.Value = $true }
            default { throw "Unexpected captured process stream record: $($record.stream)" }
        }
        $record = $null
    }
}

function Get-TestBodyEvidence([object]$Case, [int]$ExitCode, [string[]]$StdoutLines) {
    if ($ExitCode -ne 0) { throw "Selected test process exited with code $ExitCode." }
    if ($null -eq $Case) { throw 'Test-body evidence requires an exact case selector.' }
    $testMatches = @($StdoutLines | ForEach-Object {
        $normalized = [regex]::Replace([string]$_, '\x1b\[[0-?]*[ -/]*[@-~]', '').TrimEnd("`r")
        if ($normalized -match '^\s*test\s+(?<name>\S+)\s+\.\.\.(?<tail>.*)$') { [pscustomobject]@{ name=$Matches.name; tail=[string]$Matches.tail } }
    })
    $expectedMatches = @($testMatches | Where-Object { $_.name -ceq [string]$Case.exactTest })
    if ($expectedMatches.Count -ne 1) { throw "Output did not identify exactly one execution of $($Case.exactTest)." }
    $inlineTail = ([string]$expectedMatches[0].tail).Trim()
    if ($inlineTail -match '^(?<status>ok|ignored|FAILED)$' -and $Matches.status -cne 'ok') { throw "Selected test output status was not ok: $($Case.exactTest)." }
    $summaryMatches = @($StdoutLines | ForEach-Object {
        if ([string]$_ -match '^\s*test result:\s+(?<result>ok|FAILED)\.\s+(?<passed>\d+)\s+passed;\s+(?<failed>\d+)\s+failed;\s+(?<ignored>\d+)\s+ignored;') {
            [pscustomobject]@{ result=$Matches.result; passed=[int]$Matches.passed; failed=[int]$Matches.failed; ignored=[int]$Matches.ignored }
        }
    })
    if ($summaryMatches.Count -ne 1 -or $summaryMatches[0].result -cne 'ok' -or $summaryMatches[0].passed -ne 1 -or $summaryMatches[0].failed -ne 0 -or $summaryMatches[0].ignored -ne 0) { throw "libtest summary does not prove exactly one passed body for $($Case.exactTest)." }
    $verdict = if ($Case.verdict -ceq 'DIAGNOSTIC_ONLY') { 'DIAGNOSTIC_ONLY' } else { 'PASS' }
    return [pscustomobject]@{ exactTest=[string]$Case.exactTest; executionCount=1; bodyStatus='passed'; passed=1; failed=0; ignored=0; summary='verified'; verdict=$verdict }
}

function Get-CompletedCommandVerdict([object]$Profile, [object]$TestEvidence) {
    switch ([string]$Profile.operation) {
        'NoRun' { return 'NO_RUN_PASS' }
        'List' { return 'LIST_PASS' }
        'Run' {
            if ($null -eq $TestEvidence) { throw 'Runnable test command has no verified body evidence.' }
            return [string]$TestEvidence.verdict
        }
        default { throw "Unsupported completed command operation: $($Profile.operation)" }
    }
}

function Invoke-ProcessStreamProbe([string]$ExpectedExitCode) {
    $powershellExe = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    if (-not (Test-Path -LiteralPath $powershellExe -PathType Leaf)) { throw "Windows PowerShell 5.1 executable is missing for stream self-test: $powershellExe" }
    $root = Join-Path $script:Lot ('selftest-process-' + [guid]::NewGuid().ToString('N'))
    foreach ($name in @('temp','tmp','appdata','localappdata')) { New-Item -ItemType Directory -Path (Join-Path $root $name) -Force | Out-Null }
    $probeExit = if ($ExpectedExitCode -eq '0') { 0 } else { 23 }
    $probeScript = "[Console]::Out.WriteLine('N2_STREAM_STDOUT'); [Console]::Error.WriteLine('N2_STREAM_STDERR'); exit $probeExit"
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($probeScript))
    $startInfo = New-Object Diagnostics.ProcessStartInfo
    $startInfo.FileName = $powershellExe
    $startInfo.Arguments = '-NoLogo -NoProfile -NonInteractive -EncodedCommand ' + $encoded
    $startInfo.WorkingDirectory = $script:Repo
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $vars = $startInfo.EnvironmentVariables
    $vars.Clear()
    foreach ($name in @('SystemRoot','WINDIR','SystemDrive','ComSpec','PATHEXT','OS','PROCESSOR_ARCHITECTURE')) {
        $value = [Environment]::GetEnvironmentVariable($name)
        if (-not [string]::IsNullOrWhiteSpace($value)) { $vars[$name] = $value }
    }
    $vars['PATH'] = [string]::Join(';',@((Join-Path $env:SystemRoot 'System32'),$env:SystemRoot))
    foreach ($name in @('TEMP','TMP','APPDATA','LOCALAPPDATA','USERPROFILE','HOME')) { $vars[$name] = Join-Path $root $name.ToLowerInvariant() }
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $startInfo
    $queue = [Collections.Concurrent.ConcurrentQueue[object]]::new()
    $capture = $null
    $stdoutLines = [Collections.Generic.List[string]]::new()
    $stderrLines = [Collections.Generic.List[string]]::new()
    $stdoutEof = $false
    $stderrEof = $false
    $exitCode = $null
    $started = $false
    try {
        $capture = Register-ProcessStreamHandlers $process $queue
        if (-not $process.Start()) { throw 'Synthetic PowerShell stream process failed to start.' }
        $started = $true
        $process.BeginOutputReadLine(); $process.BeginErrorReadLine()
        $deadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        while (-not $process.HasExited) {
            Receive-ProcessOutput $queue $null $null $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
            if ([DateTimeOffset]::UtcNow -ge $deadline) { throw 'Synthetic PowerShell stream process exceeded its 10 second deadline.' }
            Start-Sleep -Milliseconds 20
        }
        $process.WaitForExit()
        $exitCode = $process.ExitCode
        $deadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        while (-not ($stdoutEof -and $stderrEof)) {
            Receive-ProcessOutput $queue $null $null $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
            if ([DateTimeOffset]::UtcNow -ge $deadline) { throw 'Synthetic PowerShell streams did not reach EOF before the 10 second deadline.' }
            Start-Sleep -Milliseconds 20
        }
        Receive-ProcessOutput $queue $null $null $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
        if ($exitCode -ne $probeExit) { throw "Synthetic PowerShell process exit mismatch: expected $probeExit, received $exitCode." }
        if ($stdoutLines -cnotcontains 'N2_STREAM_STDOUT' -or $stderrLines -cnotcontains 'N2_STREAM_STDERR') { throw 'Synthetic PowerShell stdout/stderr probe output was not captured.' }
    } finally {
        if ($started -and $null -ne $process -and -not $process.HasExited) { try { $process.Kill(); $process.WaitForExit() } catch { } }
        Remove-ProcessStreamHandlers $capture
        if ($null -ne $capture) { Assert-NoProcessStreamHandlers $capture }
        $process.Dispose()
    }
    return [pscustomobject]@{ expectedExitCode=$probeExit; actualExitCode=$exitCode; stdout=@($stdoutLines.ToArray()); stderr=@($stderrLines.ToArray()); subscriptionsAndJobsRemoved=$true }
}

function Invoke-OneCargoCase([object]$Profile, [object]$Case, [object]$Environment, [hashtable]$VsEnvironment, [object]$Toolchain, [string]$RunRoot, [string]$LogRoot, [bool]$DryRun) {
    $commandTimeoutSeconds = Get-NativeCommandTimeoutSeconds $Profile
    $cargoArgs = New-CargoArguments $Profile $Case
    $argvJson = Convert-StringArrayToJson $cargoArgs
    $stableArgs = Convert-JsonStringArray $argvJson 'serialized Cargo argv'
    if ($stableArgs.Count -ne $cargoArgs.Count) { throw 'Cargo argv JSON round-trip changed the argument count.' }
    $caseId = if ($Case) { [string]$Case.exactTest } else { [string]$Profile.id }
    $safeId = ($caseId -replace '[^A-Za-z0-9._-]','_')
    $stdoutPath = Join-Path $LogRoot ($safeId + '.stdout.log')
    $stderrPath = Join-Path $LogRoot ($safeId + '.stderr.log')
    $invocationPath = Join-Path $LogRoot ($safeId + '.invocation.json')
    if ($DryRun) {
        "DRY_RUN profile=$($Profile.id) case=$caseId exe=$($Toolchain.cargo) argv=$argvJson cwd=$script:Repo target=$script:CargoTarget"
        return [pscustomobject]@{ case=$caseId; exitCode=$null; status='DRY_RUN'; stdout=$stdoutPath; stderr=$stderrPath }
    }
    $initialInvocation = New-InvocationRecord $Profile $caseId $Toolchain $argvJson
    if ($null -ne $commandTimeoutSeconds) { $initialInvocation['commandTimeoutSeconds'] = $commandTimeoutSeconds }
    $initialInvocation | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $invocationPath -Encoding UTF8

    $cargoHome = [string]$Environment.CARGO_HOME
    Initialize-CargoCache $script:Lot
    $vsEnvironment = Get-VsEnvironment (Get-VsDevCommand) $Environment.TEMP $Environment.TMP $RunRoot
    $psi = New-CargoProcessInfo $Toolchain.cargo $Toolchain.rustc $Toolchain.toolchainBin $stableArgs $Environment $vsEnvironment $RunRoot $Profile.id
    $stdoutQueue = [Collections.Concurrent.ConcurrentQueue[object]]::new()
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $psi
    $allowedRoots = @($script:Lot,$script:CargoTarget)
    $userTemp = Get-FullPath (Join-Path $env:LOCALAPPDATA 'Temp')
    $protectedRoots = @((Join-Path $env:USERPROFILE '.cargo'),(Join-Path $env:LOCALAPPDATA 'Whisper'))
    foreach ($input in @($ValuesDoc.inputs.Values)) { $protectedRoots += [string]$input.path }
    $watchState = $null
    $streamCapture = $null
    $stdoutWriter = $null
    $stderrWriter = $null
    $stdoutLines = [Collections.Generic.List[string]]::new()
    $stdoutEof = $false
    $stderrEof = $false
    $processStarted = $false
    $processFamily = $null
    $beforeGit = Get-GitState $script:Repo
    $started = [DateTimeOffset]::UtcNow
    $exitCode = $null
    $testEvidence = $null
    try {
        $watchSpecs = Get-RunWatchSpecs $script:Lot
        $voxContract = Get-VoxPopuliProfileContract ([string]$Profile.id)
        if ($null -ne $voxContract -and $voxContract.operation -ceq 'Run') {
            $sourceWatchSpecs = [Collections.Generic.List[object]]::new()
            foreach ($spec in @($watchSpecs)) { $sourceWatchSpecs.Add($spec) }
            foreach ($input in @($ValuesDoc.inputs.Values)) {
                if (Test-InRoot ([string]$input.path) $script:Lot) { Register-Watch $sourceWatchSpecs ([string]$input.path) $true }
            }
            $watchSpecs = @($sourceWatchSpecs.ToArray())
        }
        $watchState = Start-WatchState $watchSpecs
        $streamCapture = Register-ProcessStreamHandlers $process $stdoutQueue
        $stdoutWriter = [IO.StreamWriter]::new($stdoutPath,$false,$script:Utf8NoBom)
        $stderrWriter = [IO.StreamWriter]::new($stderrPath,$false,$script:Utf8NoBom)
        if (-not $process.Start()) { throw 'Cargo process failed to start.' }
        $processStarted = $true
        if ($null -ne $commandTimeoutSeconds) { $processFamily = New-VoxPopuliProcessFamily $process }
        $process.BeginOutputReadLine(); $process.BeginErrorReadLine()
        $commandDeadline = if ($null -ne $commandTimeoutSeconds) { [DateTimeOffset]::UtcNow.AddSeconds($commandTimeoutSeconds) } else { $null }
        [ordered]@{ processId=$process.Id; startedUtc=$started.ToString('o'); case=$caseId } | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path $LogRoot ($safeId + '.process.json')) -Encoding UTF8
        if ($null -ne $processFamily) {
            while (Test-VoxPopuliCompletionPending $processFamily $commandDeadline $caseId $stdoutEof $stderrEof) {
                Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot $processFamily
                Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $null ([ref]$stdoutEof) ([ref]$stderrEof)
                Start-Sleep -Milliseconds 20
            }
            foreach ($entry in @($processFamily.entries.Values)) {
                if (-not $entry.process.HasExited) { throw 'VoxPopuli parent completed while an observed descendant remained alive.' }
            }
        } else {
            while (-not $process.HasExited) {
                Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot
                Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $null ([ref]$stdoutEof) ([ref]$stderrEof)
                Start-Sleep -Milliseconds 50
            }
            $process.WaitForExit()
            $streamDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
            while (-not ($stdoutEof -and $stderrEof)) {
                Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot
                Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $null ([ref]$stdoutEof) ([ref]$stderrEof)
                if ([DateTimeOffset]::UtcNow -ge $streamDeadline) { throw 'Cargo stdout/stderr capture did not reach EOF before its 10 second deadline.' }
                Start-Sleep -Milliseconds 20
            }
        }
        $exitCode = $process.ExitCode
        Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot $processFamily
        Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $null ([ref]$stdoutEof) ([ref]$stderrEof)
        $stdoutWriter.Flush(); $stderrWriter.Flush()
        if ($exitCode -ne 0) { throw "Cargo returned exit code $exitCode for $caseId; logs: $stdoutPath, $stderrPath" }
        if ($Profile.operation -eq 'Run') { $testEvidence = Get-TestBodyEvidence $Case $exitCode $stdoutLines.ToArray() }
        $afterGit = Get-GitState $script:Repo
        if ($afterGit.head -cne $beforeGit.head -or $afterGit.status -cne $beforeGit.status) { throw 'Candidate Git HEAD/status changed during native Cargo command; campaign stopped.' }
        Stop-WatchState $watchState
        Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot $processFamily
        $watchState = $null
        $verdict = Get-CompletedCommandVerdict $Profile $testEvidence
        $elapsedSeconds = [Math]::Round(([DateTimeOffset]::UtcNow - $started).TotalSeconds,3)
        Update-InvocationRecord $invocationPath $verdict $exitCode $elapsedSeconds $testEvidence $null | Out-Null
        return [pscustomobject]@{ case=$caseId; exitCode=$exitCode; status=$verdict; elapsedSeconds=$elapsedSeconds; stdout=$stdoutPath; stderr=$stderrPath }
    } catch {
        $failure = $_.Exception.Message
        if ($processStarted -and $null -ne $processFamily) {
            try { Stop-VoxPopuliProcessFamily $processFamily }
            catch { $failure += '; bounded process cleanup failed: ' + $_.Exception.Message }
        } elseif ($processStarted -and $null -ne $process -and -not $process.HasExited) { Stop-CargoProcessTree $process }
        try {
            $elapsedSeconds = [Math]::Round(([DateTimeOffset]::UtcNow - $started).TotalSeconds,3)
            Update-InvocationRecord $invocationPath 'FAIL' $exitCode $elapsedSeconds $testEvidence $failure | Out-Null
        } catch { throw "Campaign failed ($failure); invocation receipt update also failed: $($_.Exception.Message)" }
        throw
    } finally {
        if ($null -ne $watchState) {
            Stop-WatchState $watchState
            Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot $processFamily
        }
        if ($null -ne $stdoutWriter) { $stdoutWriter.Dispose() }
        if ($null -ne $stderrWriter) { $stderrWriter.Dispose() }
        Remove-ProcessStreamHandlers $streamCapture
        Close-VoxPopuliProcessFamily $processFamily
        if ($null -ne $process) { $process.Dispose() }
    }
}

function Invoke-NvidiaSmiDiagnostic([object]$Profile, [object]$Environment, [hashtable]$VsEnvironment, [object]$Toolchain, [string]$RunRoot, [string]$LogRoot, [bool]$DryRun, [switch]$SyntheticSelfTest, [string]$TestExecutable, [string[]]$TestArguments, [switch]$InjectWatcherFailure, [string]$ContextId, [object]$ContextEvidence, [Diagnostics.ProcessStartInfo]$ProcessStartInfo) {
    if ($SyntheticSelfTest) {
        $executable = Get-FullPath $TestExecutable
        $arguments = @($TestArguments)
        $hostPowerShell = Get-FullPath (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe')
        $missingRunExe = (Test-InRoot $executable $RunRoot) -and -not (Test-Path -LiteralPath $executable)
        if ($arguments.Count -lt 1 -or ($executable -ine $hostPowerShell -and -not $missingRunExe)) { throw 'Synthetic diagnostic process fixture is invalid.' }
    } else {
        Assert-NvidiaSmiDiagnosticProfile $Profile -VerifyInstalledExecutable
        $executable = Get-FullPath (Join-Path $env:SystemRoot 'System32\nvidia-smi.exe')
        $arguments = @('--query-gpu=count,pci.bus_id,uuid','--format=csv,noheader,nounits')
    }
    $argvJson = Convert-StringArrayToJson $arguments
    $stableArgs = Convert-JsonStringArray $argvJson 'pinned NVIDIA diagnostic argv'
    if ($stableArgs.Count -ne $arguments.Count -or [string]::Join("`n",$stableArgs) -cne [string]::Join("`n",$arguments)) { throw 'NVIDIA diagnostic argv JSON round-trip changed the pinned arguments.' }
    $caseId = if ([string]::IsNullOrWhiteSpace($ContextId)) { 'nvidia-smi-query-gpu' } else { 'nvidia-smi-query-gpu-' + $ContextId }
    $stdoutPath = Join-Path $LogRoot ($caseId + '.stdout.log')
    $stderrPath = Join-Path $LogRoot ($caseId + '.stderr.log')
    $invocationPath = Join-Path $LogRoot ($caseId + '.invocation.json')
    if ($DryRun) {
        Write-Host "DRY_RUN profile=$($Profile.id) exe=$executable argv=$argvJson cwd=$script:Repo"
        return [pscustomobject]@{ case=$caseId; executable=$executable; argvJson=$argvJson; exitCode=$null; status='DRY_RUN'; stdout=$stdoutPath; stderr=$stderrPath }
    }

    $started = [DateTimeOffset]::UtcNow
    $executableVersion = if (Test-Path -LiteralPath $executable -PathType Leaf) { [Diagnostics.FileVersionInfo]::GetVersionInfo($executable).FileVersion } else { 'UNAVAILABLE_SYNTHETIC' }
    $record = [ordered]@{
        schemaVersion = 1
        profile = [string]$Profile.id
        operation = 'Diagnostic'
        case = $caseId
        executable = $executable
        executableVersion = $executableVersion
        argvJson = $argvJson
        cwd = $script:Repo
        runRoot = $RunRoot
        result = 'PENDING'
        exitCode = $null
        elapsedSeconds = $null
        classification = 'DIAGNOSTIC_ONLY'
        syntheticSelfTest = [bool]$SyntheticSelfTest
        syntheticWatcherFailure = [bool]$InjectWatcherFailure
        syntheticWatcherEvents = if ($InjectWatcherFailure) { @('Created','Created','WatcherError') } else { @() }
        stdout = $stdoutPath
        stderr = $stderrPath
        failure = $null
    }
    if ($Profile.id -in @('NvidiaSmiEnvironmentCompareDiagnostic','NvidiaProgramW6432Witness')) {
        $record.executableSha256 = Get-Sha256 $executable
        $record.environmentContext = $ContextEvidence
    }
    $record | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $invocationPath -Encoding UTF8
    $invocation = [pscustomobject]@{
        profileId=[string]$Profile.id; contextId=$ContextId; contextEvidence=$ContextEvidence; captureModules=($Profile.id -ceq 'NvidiaSmiEnvironmentCompareDiagnostic' -and [bool]$Profile.command.captureModules)
        executable=$executable; arguments=$stableArgs; environment=$Environment; vsEnvironment=$VsEnvironment
        toolchain=$Toolchain; runRoot=$RunRoot; logRoot=$LogRoot; caseId=$caseId
        stdoutPath=$stdoutPath; stderrPath=$stderrPath; invocationPath=$invocationPath
        record=$record; started=$started; injectWatcherFailure=[bool]$InjectWatcherFailure; processStartInfo=$ProcessStartInfo
    }
    return Invoke-WatchedDiagnosticProcess $invocation
}

function Assert-NvidiaProgramW6432ProcessPair([Diagnostics.ProcessStartInfo]$Positive, [Diagnostics.ProcessStartInfo]$Negative, [string]$ExpectedProgramW) {
    if ($Positive.FileName -ine $Negative.FileName -or $Positive.Arguments -cne $Negative.Arguments -or $Positive.WorkingDirectory -ine $Negative.WorkingDirectory -or $Positive.UseShellExecute -ne $Negative.UseShellExecute -or $Positive.CreateNoWindow -ne $Negative.CreateNoWindow -or $Positive.RedirectStandardOutput -ne $Negative.RedirectStandardOutput -or $Positive.RedirectStandardError -ne $Negative.RedirectStandardError) { throw 'ProgramW6432 witness ProcessStartInfo values differ beyond the intended environment comparison.' }
    $positiveVars = $Positive.EnvironmentVariables
    $negativeVars = $Negative.EnvironmentVariables
    if (-not $positiveVars.ContainsKey('ProgramW6432') -or $negativeVars.ContainsKey('ProgramW6432') -or [string]$positiveVars['ProgramW6432'] -ine $ExpectedProgramW) { throw 'ProgramW6432 witness ProcessStartInfo must include the validated key only in the positive call.' }
    $keys = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($key in $positiveVars.Keys) { [void]$keys.Add([string]$key) }
    foreach ($key in $negativeVars.Keys) { [void]$keys.Add([string]$key) }
    $differentKeys = [Collections.Generic.List[string]]::new()
    foreach ($key in $keys) {
        $positiveHas = $positiveVars.ContainsKey($key)
        $negativeHas = $negativeVars.ContainsKey($key)
        if ($positiveHas -ne $negativeHas -or ($positiveHas -and [string]$positiveVars[$key] -cne [string]$negativeVars[$key])) { $differentKeys.Add($key) }
    }
    if ($differentKeys.Count -ne 1 -or $differentKeys[0] -ine 'ProgramW6432') { throw "ProgramW6432 witness child environments differ in unexpected keys: $([string]::Join('|',$differentKeys.ToArray()))." }
    return [pscustomobject]@{workingDirectory=$Positive.WorkingDirectory;executable=$Positive.FileName;arguments=$Positive.Arguments;runRootIsShared=$true;differingEnvironmentKeys=@($differentKeys.ToArray());positiveProgramW6432=$positiveVars['ProgramW6432'];negativeProgramW6432Present=$negativeVars.ContainsKey('ProgramW6432')}
}

function New-NvidiaProgramW6432ProcessPair([object]$Profile, [hashtable]$Environment, [hashtable]$VsEnvironment, [object]$Toolchain, [string]$RunRoot) {
    if ($Profile.id -cne 'NvidiaProgramW6432Witness' -or $null -eq $Toolchain) { throw 'ProgramW6432 witness requires its exact diagnostic profile and resolved Cargo toolchain.' }
    Assert-NvidiaSmiDiagnosticProfile $Profile -VerifyInstalledExecutable
    $validatedProgramW = Get-ValidatedProgramW6432Directory
    if (-not $Environment.ContainsKey('ProgramW6432') -or [string]$Environment['ProgramW6432'] -ine $validatedProgramW) { throw 'ProgramW6432 witness environment does not contain the validated host directory.' }
    $executable = Get-FullPath (Join-Path $env:SystemRoot 'System32\nvidia-smi.exe')
    $arguments = @($Profile.command.arguments | ForEach-Object { [string]$_ })
    $positiveStartInfo = New-CargoProcessInfo $Toolchain.cargo $Toolchain.rustc $Toolchain.toolchainBin @() $Environment $VsEnvironment $RunRoot $Profile.id
    $negativeStartInfo = New-CargoProcessInfo $Toolchain.cargo $Toolchain.rustc $Toolchain.toolchainBin @() $Environment $VsEnvironment $RunRoot $Profile.id
    foreach ($startInfo in @($positiveStartInfo,$negativeStartInfo)) {
        $startInfo.FileName = $executable
        $startInfo.Arguments = [string]::Join(' ',[string[]]@($arguments | ForEach-Object { Get-WindowsCommandLineArgument ([string]$_) }))
    }
    $negativeVariables = $negativeStartInfo.EnvironmentVariables
    if (-not $negativeVariables.ContainsKey('ProgramW6432')) { throw 'Negative witness ProcessStartInfo did not contain the key that must be omitted.' }
    $negativeVariables.Remove('ProgramW6432')
    if ($negativeVariables.ContainsKey('ProgramW6432')) { throw 'Could not omit only ProgramW6432 from the negative witness ProcessStartInfo.' }
    $pairEvidence = Assert-NvidiaProgramW6432ProcessPair $positiveStartInfo $negativeStartInfo $validatedProgramW
    return [pscustomobject]@{positive=$positiveStartInfo;negative=$negativeStartInfo;executable=$executable;arguments=$arguments;validatedProgramW6432=$validatedProgramW;evidence=$pairEvidence}
}

function Assert-NvidiaProgramW6432PositiveResult([object]$Result, [string[]]$OutputLines) {
    if ($Result.status -cne 'DIAGNOSTIC_ONLY' -or [int]$Result.exitCode -ne 0) { throw "ProgramW6432 positive NVIDIA witness did not return a diagnostic-only code 0 result: status=$($Result.status) exit=$($Result.exitCode)." }
    $nonemptyLines = @($OutputLines | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    $uuidPattern = 'GPU-[0-9A-Fa-f]{8}(?:-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}'
    $uuidLines = @($nonemptyLines | Where-Object { [regex]::IsMatch([string]$_,$uuidPattern) })
    $uuidCount = if ($uuidLines.Count -eq 1) { [regex]::Matches([string]$uuidLines[0],$uuidPattern).Count } else { 0 }
    if ($nonemptyLines.Count -ne 1 -or $uuidLines.Count -ne 1 -or $uuidCount -ne 1) { throw "ProgramW6432 positive NVIDIA witness must have exactly one nonempty output line containing one UUID: lines=$($nonemptyLines.Count) uuidLines=$($uuidLines.Count) uuids=$uuidCount." }
    return [pscustomobject]@{nonemptyLineCount=$nonemptyLines.Count;uuidLineCount=$uuidLines.Count;uuidCount=$uuidCount;uuid=([regex]::Match([string]$uuidLines[0],$uuidPattern).Value)}
}

function Invoke-NvidiaProgramW6432CallSequence([scriptblock]$InvokeCase) {
    $positiveEnvelope = & $InvokeCase 'positive'
    if ($null -eq $positiveEnvelope -or $null -eq $positiveEnvelope.result) { throw 'ProgramW6432 positive NVIDIA witness produced no result envelope.' }
    $positiveEvidence = Assert-NvidiaProgramW6432PositiveResult $positiveEnvelope.result ([string[]]@($positiveEnvelope.outputLines))
    $negativeEnvelope = & $InvokeCase 'negative'
    if ($null -eq $negativeEnvelope -or $null -eq $negativeEnvelope.result -or $negativeEnvelope.result.status -cne 'DIAGNOSTIC_ONLY' -or [int]$negativeEnvelope.result.exitCode -ne 255) { throw "ProgramW6432 negative NVIDIA witness did not return the expected diagnostic-only provider code 255: $($negativeEnvelope.result.exitCode)." }
    return [pscustomobject]@{calls=2;positive=$positiveEnvelope.result;negative=$negativeEnvelope.result;positiveEvidence=$positiveEvidence}
}

function Invoke-NvidiaProgramW6432Witness([object]$Profile, [hashtable]$Environment, [hashtable]$VsEnvironment, [object]$Toolchain, [string]$RunRoot, [string]$LogRoot, [bool]$DryRun) {
    Assert-NvidiaSmiDiagnosticProfile $Profile -VerifyInstalledExecutable
    $validatedProgramW = Get-ValidatedProgramW6432Directory
    if (-not $Environment.ContainsKey('ProgramW6432') -or [string]$Environment['ProgramW6432'] -ine $validatedProgramW) { throw 'ProgramW6432 witness did not receive the validated host directory.' }
    if ($DryRun) {
        $executable = Get-FullPath (Join-Path $env:SystemRoot 'System32\nvidia-smi.exe')
        $arguments = @($Profile.command.arguments | ForEach-Object { [string]$_ })
        return [pscustomobject]@{status='DRY_RUN';cases=@([pscustomobject]@{case='positive';executable=$executable;argvJson=(Convert-StringArrayToJson $arguments)},[pscustomobject]@{case='negative';executable=$executable;argvJson=(Convert-StringArrayToJson $arguments)});witness=$null;callsStarted=0}
    }
    $pair = New-NvidiaProgramW6432ProcessPair $Profile $Environment $VsEnvironment $Toolchain $RunRoot
    $invokeCase = {
        param([string]$Role)
        $startInfo = if ($Role -ceq 'positive') { $pair.positive } elseif ($Role -ceq 'negative') { $pair.negative } else { throw "Unexpected ProgramW6432 witness role: $Role" }
        $contextEvidence = if ($Role -ceq 'positive') { [pscustomobject]@{role='positive';programW6432=$validatedProgramW;expectedExitCode=0;expectedOutput='one UUID line'} } else { [pscustomobject]@{role='negative';programW6432='OMITTED';expectedExitCode=255;expectedOutput='provider failure'} }
        $result = Invoke-NvidiaSmiDiagnostic $Profile $Environment @{} $Toolchain $RunRoot $LogRoot $false -ContextId ('programw-' + $Role) -ContextEvidence $contextEvidence -ProcessStartInfo $startInfo
        $outputLines = @(Get-Content -LiteralPath $result.stdout -ErrorAction Stop)
        return [pscustomobject]@{result=$result;outputLines=$outputLines}
    }.GetNewClosure()
    $sequence = Invoke-NvidiaProgramW6432CallSequence $invokeCase
    return [pscustomobject]@{status='DIAGNOSTIC_ONLY';calls=$sequence.calls;cases=@($sequence.positive,$sequence.negative);witness=$pair.evidence;positive=$sequence.positive;negative=$sequence.negative;uuidLineCount=$sequence.positiveEvidence.uuidLineCount;qualification='NONE'}
}

function Get-NvidiaDiagnosticOutputSignature([object]$Result) {
    if (-not (Test-Path -LiteralPath $Result.stdout -PathType Leaf) -or -not (Test-Path -LiteralPath $Result.stderr -PathType Leaf)) { throw 'NVIDIA diagnostic output is missing; context comparison cannot continue.' }
    return [pscustomobject]@{
        exitCode=$Result.exitCode
        stdoutSha256=Get-Sha256 ([string]$Result.stdout)
        stderrSha256=Get-Sha256 ([string]$Result.stderr)
    }
}

function Test-NvidiaDiagnosticSignaturesDiffer([object]$Left, [object]$Right) {
    return [int]$Left.exitCode -ne [int]$Right.exitCode -or [string]$Left.stdoutSha256 -cne [string]$Right.stdoutSha256 -or [string]$Left.stderrSha256 -cne [string]$Right.stderrSha256
}

function New-NvidiaDiagnosticContextRoots([string]$ContextId, [string]$RunRoot, [string]$LogRoot, [switch]$Create) {
    if ($ContextId -notin @('clean','host-nvidia-filtered','path-only','vars-only')) { throw "Unapproved NVIDIA context root id: $ContextId" }
    $contextRunRoot = Get-FullPath (Join-Path $RunRoot (Join-Path 'contexts' $ContextId))
    $contextLogRoot = Get-FullPath (Join-Path $LogRoot (Join-Path 'contexts' $ContextId))
    foreach ($path in @($contextRunRoot,$contextLogRoot)) {
        if (-not (Test-InRoot $path $script:Lot)) { throw "NVIDIA context output escaped candidate LotRoot: $path" }
        Assert-NoReparseChain $path 'NVIDIA diagnostic context output'
        if (Test-Path -LiteralPath $path) { throw "NVIDIA context output already exists: $path" }
    }
    if ($Create) {
        New-Item -ItemType Directory -Path $contextRunRoot,$contextLogRoot -Force -ErrorAction Stop | Out-Null
        foreach ($name in @('temp','tmp','localappdata','appdata','programdata','public')) { New-Item -ItemType Directory -Path (Join-Path $contextRunRoot $name) -Force -ErrorAction Stop | Out-Null }
        Assert-NoReparseChain $contextRunRoot 'created NVIDIA context RunRoot'
        Assert-NoReparseChain $contextLogRoot 'created NVIDIA context LogRoot'
    }
    return [pscustomobject]@{runRoot=$contextRunRoot;logRoot=$contextLogRoot}
}

function Invoke-NvidiaSmiEnvironmentComparison([object]$Profile, [hashtable]$BaseEnvironment, [object]$Toolchain, [string]$RunRoot, [string]$LogRoot, [bool]$DryRun) {
    Assert-NvidiaSmiDiagnosticProfile $Profile -VerifyInstalledExecutable
    $hostEnvironment = Get-NvidiaDiagnosticHostEnvironment
    $caseResults = [Collections.Generic.List[object]]::new()
    $signatures = [Collections.Generic.List[object]]::new()
    $primaryContexts = @('clean','host-nvidia-filtered')
    foreach ($contextId in $primaryContexts) {
        $roots = New-NvidiaDiagnosticContextRoots $contextId $RunRoot $LogRoot -Create:(-not $DryRun)
        $context = New-NvidiaDiagnosticContextEnvironment $contextId $BaseEnvironment $hostEnvironment $roots.runRoot
        $result = Invoke-NvidiaSmiDiagnostic $Profile $context.environment @{} $Toolchain $roots.runRoot $roots.logRoot $DryRun -ContextId $contextId -ContextEvidence $context.evidence
        $caseResults.Add($result)
        if (-not $DryRun) { $signatures.Add((Get-NvidiaDiagnosticOutputSignature $result)) }
    }
    if ($DryRun) {
        return [pscustomobject]@{status='DRY_RUN';cases=@($caseResults.ToArray());comparison=[pscustomobject]@{conditionalContexts=@('path-only','vars-only');condition='only if primary exitCode/stdout/stderr differ';performed=$false}}
    }
    $same = -not (Test-NvidiaDiagnosticSignaturesDiffer $signatures[0] $signatures[1])
    $primaryResults = @($caseResults.ToArray())
    if ($primaryResults[0].executableSha256 -cne $primaryResults[1].executableSha256) { throw 'Pinned nvidia-smi executable SHA-256 changed between primary contexts.' }
    $conditionalStatus = if ($same) { 'NOT_RUN_PRIMARY_OUTPUTS_EQUAL' } else { 'RUN' }
    if (-not $same) {
        foreach ($contextId in @('path-only','vars-only')) {
            $roots = New-NvidiaDiagnosticContextRoots $contextId $RunRoot $LogRoot -Create
            $context = New-NvidiaDiagnosticContextEnvironment $contextId $BaseEnvironment $hostEnvironment $roots.runRoot
            $conditionalResult = Invoke-NvidiaSmiDiagnostic $Profile $context.environment @{} $Toolchain $roots.runRoot $roots.logRoot $false -ContextId $contextId -ContextEvidence $context.evidence
            if ($conditionalResult.executableSha256 -cne $primaryResults[0].executableSha256) { throw 'Pinned nvidia-smi executable SHA-256 changed during conditional contexts.' }
            $caseResults.Add($conditionalResult)
        }
    }
    return [pscustomobject]@{
        status='DIAGNOSTIC_ONLY'
        cases=@($caseResults.ToArray())
        comparison=[pscustomobject]@{
            condition='conditional contexts run only when primary exitCode/stdout/stderr differ'
            primaryOutputsEqual=[bool]$same
            cleanSignature=$signatures[0]
            hostNvidiaFilteredSignature=$signatures[1]
            conditionalContexts=@('path-only','vars-only')
            conditionalStatus=$conditionalStatus
            differenceCriteria=@('exitCode','stdoutSha256','stderrSha256')
        }
    }
}

function Invoke-WatchedDiagnosticProcess([object]$Invocation) {
    if ($null -ne $Invocation.processStartInfo) {
        $psi = $Invocation.processStartInfo
    } elseif ($Invocation.profileId -ceq 'NvidiaSmiEnvironmentCompareDiagnostic') {
        $psi = New-NvidiaDiagnosticProcessInfo $Invocation.profileId $Invocation.executable $Invocation.arguments $Invocation.environment $Invocation.runRoot
    } else {
        $psi = New-CargoProcessInfo $Invocation.toolchain.cargo $Invocation.toolchain.rustc $Invocation.toolchain.toolchainBin $Invocation.arguments $Invocation.environment $Invocation.vsEnvironment $Invocation.runRoot $Invocation.profileId
        $psi.FileName = $Invocation.executable
        $psi.Arguments = [string]::Join(' ',[string[]]@($Invocation.arguments | ForEach-Object { Get-WindowsCommandLineArgument ([string]$_) }))
    }
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $psi
    $stdoutQueue = [Collections.Concurrent.ConcurrentQueue[object]]::new()
    $allowedRoots = @($script:Lot,$script:CargoTarget)
    $userTemp = Get-FullPath (Join-Path $env:LOCALAPPDATA 'Temp')
    $protectedRoots = @((Join-Path $env:USERPROFILE '.cargo'),(Join-Path $env:LOCALAPPDATA 'Whisper'))
    $watchState = $null; $streamCapture = $null; $stdoutWriter = $null; $stderrWriter = $null
    $stdoutLines = [Collections.Generic.List[string]]::new(); $stderrLines = [Collections.Generic.List[string]]::new()
    $stdoutEof = $false; $stderrEof = $false; $processStarted = $false; $exitCode = $null
    $moduleCapture = $null
    $beforeGit = Get-GitState $script:Repo
    try {
        $watchState = Start-WatchState (Get-RunWatchSpecs $script:Lot)
        if ($Invocation.injectWatcherFailure) {
            foreach ($eventPath in @((Join-Path $script:Repo 'outside-synthetic-a'),(Join-Path $script:Repo 'outside-synthetic-b'))) {
                $watchState.queue.Enqueue([pscustomobject]@{path=$eventPath;old_path='';type='Created';at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic cleanup self-test'})
            }
            $watchState.queue.Enqueue([pscustomobject]@{path='';old_path='';type='WatcherError';at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic cleanup self-test'})
        }
        $streamCapture = Register-ProcessStreamHandlers $process $stdoutQueue
        $stdoutWriter = [IO.StreamWriter]::new($Invocation.stdoutPath,$false,$script:Utf8NoBom)
        $stderrWriter = [IO.StreamWriter]::new($Invocation.stderrPath,$false,$script:Utf8NoBom)
        if (-not $process.Start()) { throw 'Pinned nvidia-smi process failed to start.' }
        $processStarted = $true
        $process.BeginOutputReadLine(); $process.BeginErrorReadLine()
        if ($Invocation.captureModules) {
            $moduleCapture = New-NvidiaModuleCapture $process.Id
            Capture-NvidiaProcessModules $process $moduleCapture
        }
        $processRecord = [ordered]@{ processId=$process.Id; startedUtc=$Invocation.started.ToString('o'); executable=$Invocation.executable; case=$Invocation.caseId }
        if ($Invocation.profileId -ceq 'NvidiaSmiEnvironmentCompareDiagnostic') { $processRecord.context=$Invocation.contextId }
        $processRecord | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path $Invocation.logRoot ($Invocation.caseId + '.process.json')) -Encoding UTF8
        $deadline = [DateTimeOffset]::UtcNow.AddSeconds(30)
        while (-not $process.HasExited) {
            if ($Invocation.captureModules) { Capture-NvidiaProcessModules $process $moduleCapture }
            Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $Invocation.logRoot
            Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
            if ([DateTimeOffset]::UtcNow -ge $deadline) { throw 'Pinned nvidia-smi command exceeded its 30 second deadline.' }
            $pollMilliseconds = if ($Invocation.captureModules) { 20 } else { 50 }
            Start-Sleep -Milliseconds $pollMilliseconds
        }
        $process.WaitForExit()
        $exitCode = $process.ExitCode
        $streamDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        while (-not ($stdoutEof -and $stderrEof)) {
            Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $Invocation.logRoot
            Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
            if ([DateTimeOffset]::UtcNow -ge $streamDeadline) { throw 'NVIDIA diagnostic stdout/stderr capture did not reach EOF before its 10 second deadline.' }
            Start-Sleep -Milliseconds 20
        }
        Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $Invocation.logRoot
        Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $stderrLines ([ref]$stdoutEof) ([ref]$stderrEof)
        $stdoutWriter.Flush(); $stderrWriter.Flush()
        if ($Invocation.captureModules) {
            if (-not $stdoutEof -or -not $stderrEof -or -not $stdoutQueue.IsEmpty) { throw 'NVIDIA diagnostic streams were not fully drained before finalization.' }
            $stdoutWriter.Dispose(); $stderrWriter.Dispose()
            if ($moduleCapture.sampleCount -eq 0 -and $process.HasExited) { $moduleCapture.processExitedBeforeFirstSample=$true }
            $Invocation.record.moduleCapture = Complete-NvidiaModuleCapture $moduleCapture
            $Invocation.record.stdoutBytes = [long](Get-Item -LiteralPath $Invocation.stdoutPath -Force).Length
            $Invocation.record.stderrBytes = [long](Get-Item -LiteralPath $Invocation.stderrPath -Force).Length
            $Invocation.record.stdoutSha256 = Get-Sha256 $Invocation.stdoutPath
            $Invocation.record.stderrSha256 = Get-Sha256 $Invocation.stderrPath
        }
        $afterGit = Get-GitState $script:Repo
        if ($afterGit.head -cne $beforeGit.head -or $afterGit.status -cne $beforeGit.status) { throw 'Candidate Git HEAD/status changed during NVIDIA diagnostic; campaign stopped.' }
        Stop-WatchState $watchState
        Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $Invocation.logRoot
        $watchState = $null
        $elapsed = [Math]::Round(([DateTimeOffset]::UtcNow - $Invocation.started).TotalSeconds,3)
        $Invocation.record.result = 'DIAGNOSTIC_ONLY'; $Invocation.record.exitCode = $exitCode; $Invocation.record.elapsedSeconds = $elapsed
        $Invocation.record.stdoutLines = $stdoutLines.Count; $Invocation.record.stderrLines = $stderrLines.Count
        $Invocation.record | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Invocation.invocationPath -Encoding UTF8
        if ($Invocation.profileId -ceq 'NvidiaSmiEnvironmentCompareDiagnostic') {
            return [pscustomobject]@{ case=$Invocation.caseId; context=$Invocation.contextId; executable=$Invocation.executable; executableVersion=$Invocation.record.executableVersion; executableSha256=$Invocation.record.executableSha256; exitCode=$exitCode; status='DIAGNOSTIC_ONLY'; elapsedSeconds=$elapsed; stdout=$Invocation.stdoutPath; stderr=$Invocation.stderrPath; invocation=$Invocation.invocationPath }
        }
        return [pscustomobject]@{ case=$Invocation.caseId; executable=$Invocation.executable; exitCode=$exitCode; status='DIAGNOSTIC_ONLY'; elapsedSeconds=$elapsed; stdout=$Invocation.stdoutPath; stderr=$Invocation.stderrPath; invocation=$Invocation.invocationPath }
    } catch {
        if ($processStarted -and $null -ne $process -and -not $process.HasExited) { Stop-CargoProcessTree $process }
        $Invocation.record.result = 'FAIL'; $Invocation.record.exitCode = $exitCode
        $Invocation.record.elapsedSeconds = [Math]::Round(([DateTimeOffset]::UtcNow - $Invocation.started).TotalSeconds,3)
        $Invocation.record.failure = $_.Exception.Message
        $Invocation.record | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Invocation.invocationPath -Encoding UTF8
        throw
    } finally {
        $cleanupState = [pscustomobject]@{
            watchState=$watchState; allowedRoots=$allowedRoots; protectedRoots=$protectedRoots; userTemp=$userTemp
            process=$process; logRoot=$Invocation.logRoot; stdoutWriter=$stdoutWriter; stderrWriter=$stderrWriter
            streamCapture=$streamCapture; record=$Invocation.record; invocationPath=$Invocation.invocationPath
        }
        $cleanupResult = Complete-DiagnosticProcessCleanup $cleanupState
        if ($cleanupResult.errors.Count -gt 0) { throw ([string]::Join('; ',$cleanupResult.errors)) }
    }
}

function Complete-DiagnosticProcessCleanup([object]$State) {
    $errors = [Collections.Generic.List[string]]::new()
    $evidence = [ordered]@{watchStopped=$false;watchersDisposed=$false;watcherHandlersRemoved=$false;stdoutDisposed=$false;stderrDisposed=$false;streamHandlersRemoved=$false;processStopped=$false;processDisposed=$false}
    if ($null -ne $State.watchState) {
        try { Stop-WatchState $State.watchState; $evidence.watchStopped=$true } catch { $errors.Add("watch stop: $($_.Exception.Message)") }
        try { Drain-WatcherEvents $State.watchState $State.allowedRoots $State.protectedRoots $State.userTemp $State.process $Mode $State.logRoot } catch { $errors.Add("watch drain: $($_.Exception.Message)") }
        $watchersDisposed=$true
        foreach ($watcher in @($State.watchState.watchers)) { try { $watcher.EnableRaisingEvents=$false; $watcher.Dispose() } catch { $watchersDisposed=$false; $errors.Add("watcher dispose: $($_.Exception.Message)") } }
        $evidence.watchersDisposed=$watchersDisposed
        $handlersRemoved=$true
        foreach ($sourceId in @($State.watchState.subscriptions)) {
            try { Get-Event -SourceIdentifier ([string]$sourceId) -ErrorAction SilentlyContinue | Remove-Event -ErrorAction SilentlyContinue } catch { $handlersRemoved=$false; $errors.Add("watch event removal: $($_.Exception.Message)") }
            try { Unregister-Event -SourceIdentifier ([string]$sourceId) -ErrorAction SilentlyContinue; Get-Job -Name ([string]$sourceId) -ErrorAction SilentlyContinue | Remove-Job -Force -ErrorAction SilentlyContinue } catch { $handlersRemoved=$false; $errors.Add("watch handler removal: $($_.Exception.Message)") }
            if (@(Get-EventSubscriber -SourceIdentifier ([string]$sourceId) -ErrorAction SilentlyContinue).Count -gt 0 -or @(Get-Job -Name ([string]$sourceId) -ErrorAction SilentlyContinue).Count -gt 0) { $handlersRemoved=$false; $errors.Add("watch handler remains: $sourceId") }
        }
        $evidence.watcherHandlersRemoved=$handlersRemoved
    }
    if ($null -ne $State.stdoutWriter) { try { $State.stdoutWriter.Dispose(); $evidence.stdoutDisposed=$true } catch { $errors.Add("stdout dispose: $($_.Exception.Message)") } }
    if ($null -ne $State.stderrWriter) { try { $State.stderrWriter.Dispose(); $evidence.stderrDisposed=$true } catch { $errors.Add("stderr dispose: $($_.Exception.Message)") } }
    try {
        Remove-ProcessStreamHandlers $State.streamCapture
        if ($null -ne $State.streamCapture) { Assert-NoProcessStreamHandlers $State.streamCapture }
        $evidence.streamHandlersRemoved=$true
    } catch {
        $errors.Add("stream cleanup: $($_.Exception.Message)")
        if ($null -ne $State.streamCapture) {
            foreach ($sourceId in @($State.streamCapture.sourceIds)) { try { Unregister-Event -SourceIdentifier ([string]$sourceId) -ErrorAction SilentlyContinue } catch { } }
            foreach ($job in @($State.streamCapture.jobs)) { try { if ($null -ne $job) { Remove-Job -Job $job -Force -ErrorAction SilentlyContinue } } catch { } }
        }
    }
    try {
        if (-not $State.process.HasExited) { Stop-CargoProcessTree $State.process }
        if (-not $State.process.HasExited) { $State.process.Kill(); $State.process.WaitForExit() }
        $evidence.processStopped=[bool]$State.process.HasExited
        if (-not $evidence.processStopped) { $errors.Add('process remained active during cleanup') }
    } catch { $errors.Add("process stop: $($_.Exception.Message)") }
    try { $State.process.Dispose(); $evidence.processDisposed=$true } catch { $errors.Add("process dispose: $($_.Exception.Message)") }
    $State.record.cleanup=$evidence
    if ($errors.Count -gt 0) {
        $State.record.result='FAIL'
        $cleanupMessage='Diagnostic cleanup failed: ' + [string]::Join('; ',$errors.ToArray())
        $State.record.failure=if ([string]::IsNullOrWhiteSpace([string]$State.record.failure)) { $cleanupMessage } else { [string]$State.record.failure + '; ' + $cleanupMessage }
    }
    try { $State.record | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $State.invocationPath -Encoding UTF8 } catch { $errors.Add("failure receipt write: $($_.Exception.Message)") }
    return [pscustomobject]@{errors=@($errors.ToArray());cleanup=$evidence}
}

function Test-Rejects([string]$Name, [scriptblock]$Action) {
    $rejected = $false
    try { & $Action } catch { $rejected = $true }
    if (-not $rejected) { throw "Native guard self-test accepted $Name." }
}

function Invoke-StagingSelfTests([string]$Lot) {
    $fixtureRoot = Join-Path $Lot ('selftest-staging-' + [guid]::NewGuid().ToString('N'))
    $sourceRoot = Join-Path $fixtureRoot 'source'
    $localAppData = Join-Path $fixtureRoot 'run\localappdata'
    $manifestRoot = Join-Path $fixtureRoot 'manifests'
    $junctionTarget = Join-Path $fixtureRoot 'junction-target'
    foreach ($directory in @((Join-Path $sourceRoot 'app'),(Join-Path $sourceRoot 'models'),$localAppData,$manifestRoot,$junctionTarget)) {
        New-Item -ItemType Directory -Path $directory -Force -ErrorAction Stop | Out-Null
    }
    $runtimePaths = @(
        'app/whisper-desktop.exe','app/whisper-worker-cpu.exe','app/whisper-worker-gpu.exe',
        'worker/whisper-worker-parakeet-cpu.exe','worker/whisper-worker-parakeet-gpu.exe',
        'native/cpu/nemo_speech_asr.dll','native/cpu/ggml-base.dll','native/cpu/ggml-cpu.dll','native/cpu/nemo_speech_asr_c.dll','native/cpu/ggml.dll',
        'native/gpu/ggml-base.dll','native/gpu/nemo_speech_asr.dll','native/gpu/cublasLt64_12.dll','native/gpu/cudart64_12.dll','native/gpu/ggml-cuda.dll','native/gpu/cublas64_12.dll','native/gpu/nemo_speech_asr_c.dll','native/gpu/ggml.dll','native/gpu/ggml-cpu.dll',
        'models/ggml-large-v3-turbo.bin','models/parakeet-tdt-0.6b-v3.q8_0.gguf'
    )
    $sourceFiles = [Collections.Generic.List[object]]::new()
    for ($index = 0; $index -lt $runtimePaths.Count; $index++) {
        $relative = $runtimePaths[$index]
        $sourceFile = Join-Path $sourceRoot $relative
        $sourceDirectory = Split-Path -Parent $sourceFile
        if (-not (Test-Path -LiteralPath $sourceDirectory -PathType Container)) { New-Item -ItemType Directory -Path $sourceDirectory -Force | Out-Null }
        $bytes = [byte[]]::new($index + 1)
        for ($byteIndex = 0; $byteIndex -lt $bytes.Length; $byteIndex++) { $bytes[$byteIndex] = [byte](($index + $byteIndex) % 256) }
        [IO.File]::WriteAllBytes($sourceFile,$bytes)
        $sourceFiles.Add([pscustomobject]@{path=$relative;sha256=(Get-Sha256 $sourceFile);bytes=[long]$bytes.Length})
    }
    $expectedBytes = [long](($sourceFiles | Measure-Object -Property bytes -Sum).Sum)
    $shortSourceRoot = Join-Path $fixtureRoot 'short-sources'
    New-Item -ItemType Directory -Path $shortSourceRoot -Force | Out-Null
    $shortSourcePath = Join-Path $shortSourceRoot 'fr-short.mp3'
    [IO.File]::WriteAllBytes($shortSourcePath,[byte[]]@(0x49,0x44,0x33,0x04,0x05))
    $shortSourceBytes = (Get-Item -LiteralPath $shortSourcePath -Force).Length
    $shortSourceHash = Get-Sha256 $shortSourcePath
    $profile = [pscustomobject]@{
        id='SyntheticStaging'
        operation='Run'
        status='READY'
        inputs=@(
            [pscustomobject]@{key='personalRuntime';kind='directory';environmentVariable='WHISPER_N2_PERSONAL_RUNTIME_ROOT'},
            [pscustomobject]@{key='shortSource';kind='file';environmentVariable='WHISPER_TEST_SHORT_SOURCE'}
        )
        values=@()
        outputs=@([pscustomobject]@{key='campaignRun';environmentVariable='WHISPER_L04_CAMPAIGN_RUN';kind='directory';path='writes/l04-campaign'})
        staging=@(
            [pscustomobject]@{inputKey='personalRuntime';destination='Whisper';destinationBase='localappdata';requiredFileCount=21;requiredBytes=$expectedBytes},
            [pscustomobject]@{inputKey='shortSource';destination='writes/l04-campaign/inputs/fr-short.mp3';destinationBase='run';requiredFileCount=1;requiredBytes=[long]$shortSourceBytes}
        )
    }
    $writeManifest = {
        param([string]$Name,[object[]]$Inputs)
        $manifestPath = Join-Path $manifestRoot $Name
        $document = [ordered]@{schemaVersion=1;profile='SyntheticStaging';inputs=$Inputs;values=@{}}
        $document | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
        return $manifestPath
    }
    $runtimeInput = [ordered]@{key='personalRuntime';kind='directory';path=$sourceRoot;files=$sourceFiles.ToArray()}
    $shortInput = [ordered]@{key='shortSource';kind='file';path=$shortSourcePath;sha256=$shortSourceHash;bytes=[long]$shortSourceBytes}
    $validManifest = & $writeManifest 'valid.json' @($runtimeInput,$shortInput)
    $validValues = Get-ValuesDocument $validManifest $profile
    $runRoot = Join-Path $fixtureRoot 'run'
    New-Item -ItemType Directory -Path $runRoot -Force | Out-Null
    $environment = Get-ProfileEnvironment $profile $validValues $runRoot $Lot $true
    if ($environment.WHISPER_N2_PERSONAL_RUNTIME_ROOT -ine (Join-Path $localAppData 'Whisper') -or $environment.WHISPER_TEST_SHORT_SOURCE -ine (Join-Path $runRoot 'writes\l04-campaign\inputs\fr-short.mp3') -or $environment.WHISPER_L04_CAMPAIGN_RUN -ine (Join-Path $runRoot 'writes\l04-campaign')) { throw 'Staged input/output environment paths did not resolve under their canonical roots.' }
    $receipt = @(Copy-ProfileStaging $profile $validValues $localAppData $runRoot)
    $runtimeReceipt = @($receipt | Where-Object inputKey -ceq 'personalRuntime')
    $shortReceipt = @($receipt | Where-Object inputKey -ceq 'shortSource')
    if ($receipt.Count -ne 2 -or $runtimeReceipt[0].fileCount -ne 21 -or $runtimeReceipt[0].bytes -ne $expectedBytes -or $shortReceipt[0].fileCount -ne 1 -or $shortReceipt[0].bytes -ne $shortSourceBytes) { throw "Synthetic valid staging mismatch: receipts=$($receipt.Count) runtimeFiles=$($runtimeReceipt[0].fileCount) runtimeBytes=$($runtimeReceipt[0].bytes) expectedRuntimeBytes=$expectedBytes shortFiles=$($shortReceipt[0].fileCount) shortBytes=$($shortReceipt[0].bytes) expectedShortBytes=$shortSourceBytes." }
    foreach ($file in $sourceFiles) {
        $copied = Join-Path (Join-Path $localAppData 'Whisper') $file.path
        if ((Get-Sha256 $copied) -cne $file.sha256) { throw "Synthetic staged file hash differs: $($file.path)" }
    }
    foreach ($file in $sourceFiles) {
        if ((Get-Sha256 (Join-Path $sourceRoot $file.path)) -cne $file.sha256) { throw "Synthetic staging changed a source file: $($file.path)" }
    }
    if ((Get-Sha256 (Join-Path $runRoot 'writes\l04-campaign\inputs\fr-short.mp3')) -cne $shortSourceHash -or (Get-Sha256 $shortSourcePath) -cne $shortSourceHash) { throw 'Synthetic fixture staging hash/source immutability check failed.' }
    Test-Rejects 'staging overwrite of existing destinations' { Copy-ProfileStaging $profile $validValues $localAppData $runRoot }
    Test-Rejects 'staging traversal destination' {
        $badProfile = [pscustomobject]@{id='SyntheticStaging';operation='Run';status='READY';inputs=$profile.inputs;values=@();outputs=$profile.outputs;staging=@([pscustomobject]@{inputKey='personalRuntime';destination='..\escape';destinationBase='localappdata';requiredFileCount=21;requiredBytes=$expectedBytes})}
        Copy-ProfileStaging $badProfile $validValues $localAppData $runRoot
    }
    Test-Rejects 'low free space for staging bytes' { Assert-StagingCapacity ([long]30GB) 1 }
    $badHashFiles = @([pscustomobject]@{path=$sourceFiles[0].path;sha256=('0' * 64);bytes=$sourceFiles[0].bytes})
    $badHashInput = [ordered]@{key='personalRuntime';kind='directory';path=$sourceRoot;files=$badHashFiles}
    $badHashManifest = & $writeManifest 'bad-hash.json' @($badHashInput,$shortInput)
    Test-Rejects 'source hash mismatch before staging' { Get-ValuesDocument $badHashManifest $profile }
    $escapeFiles = @([pscustomobject]@{path='../escape.bin';sha256=('0' * 64);bytes=0})
    $escapeInput = [ordered]@{key='personalRuntime';kind='directory';path=$sourceRoot;files=$escapeFiles}
    $escapeManifest = & $writeManifest 'path-escape.json' @($escapeInput,$shortInput)
    Test-Rejects 'staging input path traversal' { Get-ValuesDocument $escapeManifest $profile }
    $duplicateFiles = @($sourceFiles[0],$sourceFiles[0])
    $duplicateInput = [ordered]@{key='personalRuntime';kind='directory';path=$sourceRoot;files=$duplicateFiles}
    $duplicateManifest = & $writeManifest 'duplicate.json' @($duplicateInput,$shortInput)
    Test-Rejects 'duplicate staging source paths' { Get-ValuesDocument $duplicateManifest $profile }
    $missingFiles = @([pscustomobject]@{path='models/missing.bin';sha256=('0' * 64);bytes=0})
    $missingInput = [ordered]@{key='personalRuntime';kind='directory';path=$sourceRoot;files=$missingFiles}
    $missingManifest = & $writeManifest 'missing.json' @($missingInput,$shortInput)
    Test-Rejects 'missing staging source file' { Get-ValuesDocument $missingManifest $profile }
    $linkedFile = Join-Path $junctionTarget 'linked.bin'
    [IO.File]::WriteAllBytes($linkedFile,[byte[]]@(0x51,0x52,0x53))
    $junctionPath = Join-Path $sourceRoot 'linked'
    New-Item -ItemType Junction -Path $junctionPath -Target $junctionTarget -ErrorAction Stop | Out-Null
    $linkedFiles = @([pscustomobject]@{path='linked/linked.bin';sha256=(Get-Sha256 $linkedFile);bytes=3})
    $reparseInput = [ordered]@{key='personalRuntime';kind='directory';path=$sourceRoot;files=$linkedFiles}
    $reparseManifest = & $writeManifest 'reparse.json' @($reparseInput,$shortInput)
    Test-Rejects 'reparse point in staging source chain' { Get-ValuesDocument $reparseManifest $profile }
    return [pscustomobject]@{ fixtureRoot=$fixtureRoot; stagedFiles=($receipt | Measure-Object -Property fileCount -Sum).Sum; stagedBytes=($receipt | Measure-Object -Property bytes -Sum).Sum; rejected=@('overwrite','destination traversal','insufficient free space','source hash mismatch','path traversal','duplicate path','missing source','reparse source chain') }
}

function Invoke-SelfTests([object]$Registry, [string]$Lot) {
    $closureTemplate=Get-Profile $Registry 'PersonalUnified'
    foreach ($id in @('NativeClosureCpuReference','NativeClosureCpuRebuilt','NativeClosureGpuReference','NativeClosureGpuRebuilt')) {
        $closure=Get-Profile $Registry $id
        Assert-NativeClosureProfile $closure $closureTemplate
        foreach ($field in @('runtimePin','package')) {
            $bad=$closure | ConvertTo-Json -Depth 12 | ConvertFrom-Json
            $bad.$field='unapproved'
            Test-Rejects "closure profile $id $field" { Assert-NativeClosureProfile $bad $closureTemplate }
        }
        $bad=$closure | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $bad.cases[0].exactTest='arbitrary_test'
        Test-Rejects "closure profile $id selector" { Assert-NativeClosureProfile $bad $closureTemplate }
        $bad=$closure | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        $bad.staging[0].requiredBytes=2
        Test-Rejects "closure profile $id byte override" { Assert-NativeClosureProfile $bad $closureTemplate }
    }
    Test-Rejects 'unknown closure pin' { Get-NativeClosurePinName 'NativeClosureUnknown' }
    Test-Rejects 'unknown persistent pin' { Read-NativeClosurePin '../outside' }
    $referencePin=Read-NativeClosurePin 'reference'
    Assert-NativeClosurePayload $referencePin $referencePin 'reference' @($referencePin.files.Values)
    foreach ($variant in @('cpu','gpu')) {
        $replacement=$referencePin | ConvertTo-Json -Depth 12 | ConvertFrom-Json -AsHashtable
        $replacement.document.llamaCommit='bd4f514db14d87fded667787a7a963bfbaa98e89'
        $replacement.files["native/$variant/ggml-base.dll"].sha256='0'*64
        Assert-NativeClosurePayload $replacement $referencePin $variant @($replacement.files.Values)
        $replacement.files['worker/whisper-worker-parakeet-cpu.exe'].sha256='1'*64
        Test-Rejects "closure $variant changed worker" { Assert-NativeClosurePayload $replacement $referencePin $variant @($replacement.files.Values) }
    }
    Test-Rejects 'closure input missing file' { Assert-NativeClosurePayload $referencePin $referencePin 'reference' @($referencePin.files.Values | Select-Object -First 20) }
    $processInfoFunction = Get-Command New-NvidiaDiagnosticProcessInfo -CommandType Function -ErrorAction Stop
    $functionTokens = $null; $functionParseErrors = $null
    $processInfoAst = [System.Management.Automation.Language.Parser]::ParseInput($processInfoFunction.Definition,[ref]$functionTokens,[ref]$functionParseErrors)
    if ($functionParseErrors.Count -gt 0) { throw 'NVIDIA diagnostic process builder did not parse for the Cargo-root operator guard.' }
    $ambiguousCargoRootCalls = @($processInfoAst.FindAll({
        param($node)
        if ($node -isnot [System.Management.Automation.Language.CommandAst] -or $node.GetCommandName() -ine 'Get-FullPath') { return $false }
        foreach ($element in $node.CommandElements) { if ($element.Extent.Text -in @('-ine','-ieq','-ne','-eq')) { return $true } }
        return $false
    },$true))
    if ($ambiguousCargoRootCalls.Count -gt 0) { throw 'NVIDIA diagnostic Cargo-root comparison must apply comparison operators to the resolved path value, not pass them as Get-FullPath command arguments.' }
    $rootFixture = Join-Path $Lot ('selftest-lot-roots-' + [guid]::NewGuid().ToString('N'))
    $savedRepo = $script:Repo
    $lotRootEvidence = $null
    try {
        $script:Repo = $rootFixture
        $syntheticN2Root = Get-FullPath (Join-Path $script:Repo 'target\n2-native')
        $syntheticMainFacadesRoot = Get-FullPath (Join-Path $script:Repo 'target\parakeet-tests\main-facades')
        New-Item -ItemType Directory -Path (Split-Path -Parent $syntheticN2Root),(Split-Path -Parent $syntheticMainFacadesRoot) -Force | Out-Null
        if ((Resolve-LotRoot 'target\n2-native') -ine $syntheticN2Root -or (Resolve-LotRoot 'target\parakeet-tests\main-facades') -ine $syntheticMainFacadesRoot) { throw 'Exact candidate lot roots did not resolve canonically.' }
        Assert-LotRootForProfile 'TurboCpuReusableImports' $syntheticMainFacadesRoot
        Assert-LotRootForProfile 'TurboGpuReusableImports' $syntheticMainFacadesRoot
        Assert-LotRootForProfile 'ParakeetCpuContracts' $syntheticMainFacadesRoot
        Assert-LotRootForProfile 'PreparationNoRun' $syntheticN2Root
        foreach ($profileId in @('TurboCpuReusableImports','TurboGpuReusableImports','ParakeetCpuContracts')) {
            Test-Rejects "$profileId with N2 root" { Assert-LotRootForProfile $profileId $syntheticN2Root }
        }
        Test-Rejects 'non-reusable profile with main-facades root' { Assert-LotRootForProfile 'TurboCpuLive' $syntheticMainFacadesRoot }
        Test-Rejects 'unapproved candidate lot root' { Resolve-LotRoot 'target\parakeet-tests\other' }
        $junctionTarget = Join-Path $rootFixture 'reparse-target'
        $junctionParent = Join-Path $script:Repo 'target\parakeet-tests'
        New-Item -ItemType Directory -Path $junctionTarget -Force | Out-Null
        $junctionPath = Join-Path $junctionParent 'main-facades'
        New-Item -ItemType Junction -Path $junctionPath -Target $junctionTarget -ErrorAction Stop | Out-Null
        Test-Rejects 'reparse main-facades lot root' { Resolve-LotRoot 'target\parakeet-tests\main-facades' }
        $lotRootEvidence = [pscustomobject]@{ mainFacadesProfiles=@('TurboCpuReusableImports','TurboGpuReusableImports','ParakeetCpuContracts'); n2Profiles='all other registered profiles'; rejected=@('main-facades profile with N2 root','non-main-facades profile with main-facades root','unapproved candidate root','reparse main-facades root') }
    } finally {
        $script:Repo = $savedRepo
        $junctionPath = Join-Path (Join-Path $rootFixture 'target\parakeet-tests') 'main-facades'
        if (Test-Path -LiteralPath $junctionPath) { Remove-Item -LiteralPath $junctionPath -Force -ErrorAction SilentlyContinue }
        if (Test-Path -LiteralPath $rootFixture) { Remove-Item -LiteralPath $rootFixture -Recurse -Force -ErrorAction SilentlyContinue }
    }
    $expectedReusableCases = @{
        TurboCpuReusableImports = @(
            'worker_contract|turbo_reusable_import_contract',
            'worker_contract|reusable_imports::cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache',
            'reusable_imports|cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache'
        )
        TurboGpuReusableImports = @('worker_contract|turbo_reusable_import_contract')
    }
    foreach ($profileId in @('TurboCpuReusableImports','TurboGpuReusableImports')) {
        $profile = Get-Profile $Registry $profileId
        $expectedPackage = if ($profileId -ceq 'TurboCpuReusableImports') { 'whisper-worker-cpu' } else { 'whisper-worker-gpu' }
        if ($profile.package -cne $expectedPackage -or $profile.features.Count -ne 1 -or $profile.features[0] -cne "$expectedPackage/native-tests" -or @($profile.compileEnvironment).Count -ne 0) { throw "Reusable profile package/feature contract is unexpected: $profileId" }
        if (@($profile.cases).Count -ne $expectedReusableCases[$profileId].Count) { throw "Reusable profile case count differs: $profileId" }
        for ($caseIndex = 0; $caseIndex -lt @($profile.cases).Count; $caseIndex++) {
            $case = @($profile.cases)[$caseIndex]
            $expected = [string]$expectedReusableCases[$profileId][$caseIndex]
            $actual = [string]$case.targetName + '|' + [string]$case.exactTest
            if ($case.targetKind -cne 'integration' -or $actual -cne $expected -or $case.ignored -or $case.verdict -cne 'QUALIFICATION') { throw "Reusable profile selector differs from the authorized case set: $profileId / $actual" }
            $args = New-CargoArguments $profile $case
            if ($args -notcontains [string]$case.exactTest -or $args -contains '--ignored') { throw "Reusable profile selector command is malformed: $profileId / $($case.exactTest)" }
        }
    }
    $parakeetProfile = Get-Profile $Registry 'ParakeetCpuContracts'
    Assert-ProfileFeatureScope $parakeetProfile
    $expectedParakeetCases = @('parakeet_refusal_order_contract','parakeet_errors_and_shutdown_contract','parakeet_native_transcription_contract')
    if ($parakeetProfile.package -cne 'whisper-worker-parakeet-cpu' -or @($parakeetProfile.inputs).Count -ne 4 -or @($parakeetProfile.values).Count -ne 0 -or @($parakeetProfile.outputs).Count -ne 1 -or @($parakeetProfile.staging).Count -ne 2 -or @($parakeetProfile.cases).Count -ne $expectedParakeetCases.Count -or @($parakeetProfile.compileEnvironment).Count -ne 0) { throw 'ParakeetCpuContracts profile fields differ from the bounded three-case contract.' }
    $expectedInputs = @('personalRuntime|WHISPER_N2_PERSONAL_RUNTIME_ROOT|directory','referenceExe|WHISPER_TEST_REFERENCE_EXE|file','candidateExe|WHISPER_TEST_WORKER_EXE|file','referenceShort|WHISPER_TEST_REFERENCE_SHORT_AUDIO|file')
    for ($inputIndex = 0; $inputIndex -lt $expectedInputs.Count; $inputIndex++) {
        $input = @($parakeetProfile.inputs)[$inputIndex]
        if (($input.key + '|' + $input.environmentVariable + '|' + $input.kind) -cne $expectedInputs[$inputIndex]) { throw "ParakeetCpuContracts input differs from its approved manifest contract: $($input.key)" }
    }
    $output = @($parakeetProfile.outputs)[0]
    if (($output.key + '|' + $output.environmentVariable + '|' + $output.path) -cne 'workRoot|WHISPER_TEST_WORK_ROOT|writes/main-facades-work') { throw 'ParakeetCpuContracts work root differs from its approved candidate-local path.' }
    $runtimeStage = @($parakeetProfile.staging | Where-Object inputKey -CEQ 'personalRuntime')[0]
    $shortStage = @($parakeetProfile.staging | Where-Object inputKey -CEQ 'referenceShort')[0]
    if ($runtimeStage.destinationBase -cne 'localappdata' -or $runtimeStage.destination -cne 'Whisper' -or [int]$runtimeStage.requiredFileCount -ne 21 -or [long]$runtimeStage.requiredBytes -ne 3196008875 -or $shortStage.destinationBase -cne 'run' -or $shortStage.destination -cne 'writes/main-facades-work/reference-short.wav' -or [int]$shortStage.requiredFileCount -ne 1 -or [long]$shortStage.requiredBytes -ne 484910) { throw 'ParakeetCpuContracts staging destinations or byte/file pins differ from the approved contract.' }
    for ($caseIndex = 0; $caseIndex -lt $expectedParakeetCases.Count; $caseIndex++) {
        $case = @($parakeetProfile.cases)[$caseIndex]
        if ($case.targetKind -cne 'integration' -or $case.targetName -cne 'worker_contract' -or $case.exactTest -cne $expectedParakeetCases[$caseIndex] -or $case.ignored -or $case.verdict -cne 'QUALIFICATION') { throw "ParakeetCpuContracts selector differs from the authorized case set: $($case.exactTest)" }
        $args = New-CargoArguments $parakeetProfile $case
        if ($args -notcontains [string]$case.exactTest -or $args -contains '--ignored') { throw "ParakeetCpuContracts command is malformed: $($case.exactTest)" }
    }
    Test-Rejects 'parakeet-personal feature on another profile' { Assert-ProfileFeatureScope ([pscustomobject]@{id='TurboCpuLive';package='whisper-worker-parakeet-cpu';features=@('whisper-worker-parakeet-cpu/native-tests','whisper-worker-parakeet-cpu/parakeet-personal')}) }
    Test-Rejects 'parakeet-personal feature with another package' { Assert-ProfileFeatureScope ([pscustomobject]@{id='ParakeetCpuContracts';package='whisper-worker-cpu';features=@('whisper-worker-parakeet-cpu/native-tests','whisper-worker-parakeet-cpu/parakeet-personal')}) }
    $positive = Convert-StringArrayToJson @('cargo','test')
    $roundTrip = Convert-JsonStringArray $positive 'self-test argv'
    if ($roundTrip.Count -ne 2 -or $roundTrip[0] -cne 'cargo' -or $roundTrip[1] -cne 'test') { throw 'ARGV JSON multi-value roundtrip failed.' }
    $singletonJson = Convert-StringArrayToJson @('--list')
    $singleton = Convert-JsonStringArray $singletonJson 'self-test singleton argv'
    if ($singleton.Count -ne 1 -or $singleton[0] -cne '--list') { throw 'ARGV JSON singleton roundtrip failed.' }
    Test-Rejects 'unknown profile' { Get-Profile $Registry 'UnknownProfile' }
    $prepareProfile = Get-Profile $Registry 'PreparationNoRun'
    $prepareArgs = New-CargoArguments $prepareProfile $null
    $expectedPrepare = @('test','--offline','--locked','--workspace','--features','native-tests','--no-run')
    if ([string]::Join("`n",$prepareArgs) -cne [string]::Join("`n",$expectedPrepare)) { throw 'PreparationNoRun command did not match its bounded argv.' }
    $listProfile = Get-Profile $Registry 'InventoryList'
    $listArgs = New-CargoArguments $listProfile $null
    $expectedList = @('test','--offline','--locked','--workspace','--features','native-tests','--','--list')
    if ([string]::Join("`n",$listArgs) -cne [string]::Join("`n",$expectedList)) { throw 'InventoryList command did not match its bounded argv.' }
    foreach ($profileId in @('WasapiInventory','WasapiCapture')) {
        $profile = Get-Profile $Registry $profileId
        $case = @($profile.cases)[0]
        $args = New-CargoArguments $profile $case
        if ($args -notcontains '--exact' -or $args -contains '--ignored' -or $args[$args.Count - 1] -cne '--nocapture') { throw "Exact discovery case command is malformed: $profileId" }
        if ($args -notcontains [string]$case.exactTest) { throw "Exact selector is missing from command: $profileId" }
    }
    $fixtureCase = [pscustomobject]@{ exactTest='capture::tests::synthetic_body'; verdict='QUALIFICATION' }
    $validOutput = @('test capture::tests::synthetic_body ... ok','test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out')
    $validEvidence = Get-TestBodyEvidence $fixtureCase 0 $validOutput
    if ($validEvidence.verdict -cne 'PASS' -or $validEvidence.passed -ne 1) { throw 'Positive exact-test evidence fixture was not accepted as PASS.' }
    $inventoryCase = [pscustomobject]@{ exactTest='capture::tests::native_wasapi_input_catalog_has_stable_parseable_ids'; verdict='QUALIFICATION' }
    $inventoryOutput = @('running 1 test','test capture::tests::native_wasapi_input_catalog_has_stable_parseable_ids ... enumerating active capture endpoints','device catalog contains 3 entries','ok','test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 83 filtered out')
    $inventoryEvidence = Get-TestBodyEvidence $inventoryCase 0 $inventoryOutput
    if ($inventoryEvidence.executionCount -ne 1 -or $inventoryEvidence.passed -ne 1) { throw 'Multiline WASAPI inventory fixture was not accepted.' }
    $captureCase = [pscustomobject]@{ exactTest='capture::tests::native_selected_microphone_survives_initial_discontinuity'; verdict='QUALIFICATION' }
    $captureOutput = @('running 1 test','test capture::tests::native_selected_microphone_survives_initial_discontinuity ... selected endpoint: Headset Microphone','captured buffer discontinuity recovered','ok','test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 91 filtered out')
    $captureEvidence = Get-TestBodyEvidence $captureCase 0 $captureOutput
    if ($captureEvidence.executionCount -ne 1 -or $captureEvidence.passed -ne 1) { throw 'Multiline WASAPI capture fixture was not accepted.' }
    if ((Get-CompletedCommandVerdict ([pscustomobject]@{operation='NoRun'}) $null) -cne 'NO_RUN_PASS' -or (Get-CompletedCommandVerdict ([pscustomobject]@{operation='List'}) $null) -cne 'LIST_PASS') { throw 'NoRun/List command completion verdicts are not distinct.' }
    Test-Rejects 'Run command without test-body evidence' { Get-CompletedCommandVerdict ([pscustomobject]@{operation='Run'}) $null }
    $diagnosticCase = [pscustomobject]@{ exactTest='capture::tests::synthetic_diagnostic'; verdict='DIAGNOSTIC_ONLY' }
    $diagnosticEvidence = Get-TestBodyEvidence $diagnosticCase 0 @('test capture::tests::synthetic_diagnostic ... ok','test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out')
    if ($diagnosticEvidence.verdict -cne 'DIAGNOSTIC_ONLY' -or (Get-CompletedCommandVerdict ([pscustomobject]@{operation='Run'}) $diagnosticEvidence) -cne 'DIAGNOSTIC_ONLY') { throw 'Manual diagnostic evidence lost its distinct verdict.' }
    Test-Rejects 'zero-pass / ignored-only libtest output' { Get-TestBodyEvidence $fixtureCase 0 @('test capture::tests::synthetic_body ... ignored','test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 12 filtered out') }
    Test-Rejects 'wrong exact test name in output' { Get-TestBodyEvidence $fixtureCase 0 @('test capture::tests::other_body ... ok','test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out') }
    Test-Rejects 'incomplete libtest summary' { Get-TestBodyEvidence $fixtureCase 0 @('test capture::tests::synthetic_body ... multiline body started','detail output without terminal summary') }
    Test-Rejects 'multiple executed tests in exact output' { Get-TestBodyEvidence $fixtureCase 0 @('test capture::tests::synthetic_body ... ok','test capture::tests::extra ... ok','test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out') }
    Test-Rejects 'nonzero test process exit' { Get-TestBodyEvidence $fixtureCase 23 $validOutput }
    $receiptEvidence = Invoke-InvocationReceiptSelfTest $Lot
    Test-Rejects 'pending profile without selectors' { Get-Profile $Registry 'ParakeetCpu' }
    Assert-ExactTestSelector 'capture::tests::native_wasapi_input_catalog_has_stable_parseable_ids'
    Test-Rejects 'profile with broad wildcard selector' { Assert-ExactTestSelector '*' }
    Test-Rejects 'path traversal outside lot' { Assert-PathUnder (Join-Path $Lot '..\outside') $Lot 'self-test path' }
    $syntheticRunRoot = Join-Path $Lot 'runs\selftest-canonical-roots'
    $syntheticEnvironment = @{
        CARGO_HOME = Join-Path $Lot 'cargo-home'; CARGO_TARGET_DIR = $script:CargoTarget
        TEMP = Join-Path $syntheticRunRoot 'temp'; TMP = Join-Path $syntheticRunRoot 'tmp'
        LOCALAPPDATA = Join-Path $syntheticRunRoot 'localappdata'; APPDATA = Join-Path $syntheticRunRoot 'appdata'
        USERPROFILE = $syntheticRunRoot; HOME = $syntheticRunRoot
        CARGO_INCREMENTAL = '0'; CARGO_PROFILE_DEV_DEBUG = 'line-tables-only'
        WHISPER_TEST_MIC_ID = 'synthetic-cpal-device-id'
        WHISPER_L04_CAMPAIGN_BUILD = '1'
        PARAKEET_L2_ACCEPTANCE_TEST_BUILD = '1'
        CUDA_PATH_V12_9 = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'
    }
    Assert-CampaignRoots $Lot $syntheticRunRoot $syntheticEnvironment
    $badRootEnvironment = @{} + $syntheticEnvironment
    $badRootEnvironment.TMP = $env:TEMP
    Test-Rejects 'TEMP/TMP root outside lot' { Assert-CampaignRoots $Lot $syntheticRunRoot $badRootEnvironment }
    $savedGgml = [Environment]::GetEnvironmentVariable('GGML_CUDA','Process')
    $savedWhisper = [Environment]::GetEnvironmentVariable('WHISPER_UNLISTED_SENTINEL','Process')
    $savedCudaHome = [Environment]::GetEnvironmentVariable('CUDA_HOME','Process')
    $savedCudaPathV1210 = [Environment]::GetEnvironmentVariable('CUDA_PATH_V12_10','Process')
    try {
        [Environment]::SetEnvironmentVariable('GGML_CUDA','ambient-sentinel','Process')
        [Environment]::SetEnvironmentVariable('WHISPER_UNLISTED_SENTINEL','ambient-sentinel','Process')
        [Environment]::SetEnvironmentVariable('CUDA_HOME','ambient-cuda-home-sentinel','Process')
        [Environment]::SetEnvironmentVariable('CUDA_PATH_V12_10','ambient-versioned-cuda-sentinel','Process')
        $syntheticEnvironment.CUDA_PATH = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'
        $syntheticVs = @{ PATH = 'C:\ambient-tool;C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\bin' ; INCLUDE='C:\vs\include'; LIB='C:\vs\lib'; ProgramW6432='C:\ambient-programw-spoof' }
        $syntheticPsi = New-CargoProcessInfo 'C:\toolchain\cargo.exe' 'C:\toolchain\rustc.exe' 'C:\toolchain' @('test','--offline','--locked') $syntheticEnvironment $syntheticVs $syntheticRunRoot 'PreparationNoRun'
        $syntheticVars = $syntheticPsi.EnvironmentVariables
        if ($syntheticVars.ContainsKey('GGML_CUDA') -or $syntheticVars.ContainsKey('WHISPER_UNLISTED_SENTINEL') -or $syntheticVars.ContainsKey('RUSTFLAGS')) { throw 'Ambient build variables survived the sanitized Cargo environment.' }
        if ($syntheticVars.ContainsKey('ProgramW6432')) { throw 'ProgramW6432 entered a non-GPU Cargo child through the Visual Studio environment.' }
        foreach ($name in @('WHISPER_TEST_MIC_ID','WHISPER_L04_CAMPAIGN_BUILD','PARAKEET_L2_ACCEPTANCE_TEST_BUILD','CUDA_PATH','CUDA_PATH_V12_9')) {
            if ($syntheticVars[$name] -cne [string]$syntheticEnvironment[$name]) { throw "Declared profile environment value was not passed exactly: $name" }
        }
        foreach ($name in @('CUDA_HOME','CUDA_PATH_V12_10')) {
            if ($syntheticVars.ContainsKey($name)) { throw "Undeclared CUDA environment alias survived the sanitized Cargo environment: $name" }
        }
        foreach ($name in @('CARGO_HOME','CARGO_TARGET_DIR','TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) {
            if ($syntheticVars[$name] -cne [string]$syntheticEnvironment[$name]) { throw "Isolated root was not preserved in Cargo environment: $name" }
        }
        if ($syntheticVars['PATH'] -match 'C:\\ambient-tool' -or $syntheticVars['PATH'] -notmatch 'CUDA\\v12\.9\\bin') { throw 'Cargo PATH was not constrained to approved tools.' }
        $validatedProgramW = Get-ValidatedProgramW6432Directory
        $programWEnvironment = @{} + $syntheticEnvironment
        $programWEnvironment['ProgramW6432'] = $validatedProgramW
        foreach ($profileId in @('GpuCudaProbe','TurboGpu','TurboGpuLive','TurboGpuReusableImports','PersonalUnified','PersonalUnifiedGpuOnly','NativeClosureGpuReference','NativeClosureGpuRebuilt','ParakeetGpu','ParakeetGpuFrenchShort','ParakeetGpuFrenchLong','ParakeetGpuEnglishLong','ParakeetGpuFrenchAutoLong','ParakeetGpuEnglishAutoLong')) {
            $profilePsi = New-CargoProcessInfo 'C:\toolchain\cargo.exe' 'C:\toolchain\rustc.exe' 'C:\toolchain' @('test','--offline','--locked') $programWEnvironment $syntheticVs $syntheticRunRoot $profileId
            if (-not $profilePsi.EnvironmentVariables.ContainsKey('ProgramW6432') -or [string]$profilePsi.EnvironmentVariables['ProgramW6432'] -ine $validatedProgramW) { throw "Authorized GPU profile did not pass validated ProgramW6432: $profileId" }
        }
        foreach ($profileId in @('TurboCpu','PersonalAutoFallback','NativeClosureCpuReference','NativeClosureCpuRebuilt','PersonalLongCpu','PreparationNoRun')) {
            $profilePsi = New-CargoProcessInfo 'C:\toolchain\cargo.exe' 'C:\toolchain\rustc.exe' 'C:\toolchain' @('test','--offline','--locked') $syntheticEnvironment $syntheticVs $syntheticRunRoot $profileId
            if ($profilePsi.EnvironmentVariables.ContainsKey('ProgramW6432')) { throw "Non-GPU profile inherited ProgramW6432 through the Visual Studio environment: $profileId" }
        }
        Test-Rejects 'ProgramW6432 declared for a non-GPU profile' {
            New-CargoProcessInfo 'C:\toolchain\cargo.exe' 'C:\toolchain\rustc.exe' 'C:\toolchain' @('test','--offline','--locked') $programWEnvironment $syntheticVs $syntheticRunRoot 'TurboCpu'
        }
    } finally {
        [Environment]::SetEnvironmentVariable('GGML_CUDA',$savedGgml,'Process')
        [Environment]::SetEnvironmentVariable('WHISPER_UNLISTED_SENTINEL',$savedWhisper,'Process')
        [Environment]::SetEnvironmentVariable('CUDA_HOME',$savedCudaHome,'Process')
        [Environment]::SetEnvironmentVariable('CUDA_PATH_V12_10',$savedCudaPathV1210,'Process')
    }
    $streamNormal = Invoke-ProcessStreamProbe '0'
    $streamFailure = Invoke-ProcessStreamProbe '23'
    if (-not $streamNormal.subscriptionsAndJobsRemoved -or -not $streamFailure.subscriptionsAndJobsRemoved) { throw 'Synthetic process stream cleanup did not verify empty subscription/job state.' }
    $programWResolverEvidence = Invoke-ProgramW6432ResolverSelfTests $Lot
    $personalUnifiedProfile = Get-Profile $Registry 'PersonalUnified'
    $personalGpuOnlyProfile = Get-Profile $Registry 'PersonalUnifiedGpuOnly'
    if (@($personalUnifiedProfile.cases).Count -ne 3 -or @($personalGpuOnlyProfile.cases).Count -ne 1 -or $personalGpuOnlyProfile.cases[0].exactTest -cne 'native_personal_gpu_imports_english_mp3_with_attested_candidate') { throw 'PersonalUnifiedGpuOnly must contain only the already accepted GPU selector while PersonalUnified retains its three cases.' }
    foreach ($name in @('package','features','inputs','values','outputs','staging')) {
        if ((ConvertTo-Json -InputObject $personalUnifiedProfile.$name -Depth 12 -Compress) -cne (ConvertTo-Json -InputObject $personalGpuOnlyProfile.$name -Depth 12 -Compress)) { throw "PersonalUnifiedGpuOnly differs from PersonalUnified $name." }
    }
    foreach ($profileId in @('GpuCudaProbe','TurboGpu','TurboGpuLive','TurboGpuReusableImports','PersonalUnified','PersonalUnifiedGpuOnly','NativeClosureGpuReference','NativeClosureGpuRebuilt','ParakeetGpu','ParakeetGpuFrenchShort','ParakeetGpuFrenchLong','ParakeetGpuEnglishLong','ParakeetGpuFrenchAutoLong','ParakeetGpuEnglishAutoLong','NvidiaSmiInventoryDiagnostic','NvidiaSmiEnvironmentCompareDiagnostic','NvidiaProgramW6432Witness')) {
        if (-not (Test-ProgramW6432ProfileId $profileId)) { throw "Approved ProgramW6432 profile is not on the exact allowlist: $profileId" }
    }
    foreach ($profileId in @('PreparationNoRun','InventoryList','WasapiInventory','WasapiCapture','TurboCpu','PersonalAutoFallback','NativeClosureCpuReference','NativeClosureCpuRebuilt','PersonalLongCpu','ParakeetGpuPreparation','ParakeetGpuInventory')) {
        if (Test-ProgramW6432ProfileId $profileId) { throw "Non-GPU or pending profile entered the ProgramW6432 allowlist: $profileId" }
    }
    $nvidiaDiagnosticProfile = Get-Profile $Registry 'NvidiaSmiInventoryDiagnostic'
    Assert-NvidiaSmiDiagnosticProfile $nvidiaDiagnosticProfile -VerifyInstalledExecutable
    $badDiagnosticProfile = ConvertFrom-Json (ConvertTo-Json -InputObject $nvidiaDiagnosticProfile -Depth 10)
    $badDiagnosticProfile.command.arguments[0] = '--query-gpu=everything'
    Test-Rejects 'arbitrary NVIDIA diagnostic argv' { Assert-NvidiaSmiDiagnosticProfile $badDiagnosticProfile }
    $badDiagnosticProfile = ConvertFrom-Json (ConvertTo-Json -InputObject $nvidiaDiagnosticProfile -Depth 10)
    $badDiagnosticProfile.command.executable = Join-Path $env:SystemRoot 'System32\where.exe'
    Test-Rejects 'alternate NVIDIA diagnostic executable' { Assert-NvidiaSmiDiagnosticProfile $badDiagnosticProfile }
    $nvidiaCompareProfile = Get-Profile $Registry 'NvidiaSmiEnvironmentCompareDiagnostic'
    Assert-NvidiaSmiDiagnosticProfile $nvidiaCompareProfile -VerifyInstalledExecutable
    $badCompareProfile = ConvertFrom-Json (ConvertTo-Json -InputObject $nvidiaCompareProfile -Depth 10)
    $badCompareProfile.command.contexts[1] = 'arbitrary-user-context'
    Test-Rejects 'arbitrary NVIDIA comparison context' { Assert-NvidiaSmiDiagnosticProfile $badCompareProfile }
    $badCompareProfile = ConvertFrom-Json (ConvertTo-Json -InputObject $nvidiaCompareProfile -Depth 10)
    Add-Member -InputObject $badCompareProfile.command -MemberType NoteProperty -Name unlistedEnvironmentVariable -Value 'PATH' -Force
    Test-Rejects 'unlisted NVIDIA comparison field' { Assert-NvidiaSmiDiagnosticProfile $badCompareProfile }
    $programWWitnessProfile = Get-Profile $Registry 'NvidiaProgramW6432Witness'
    Assert-NvidiaSmiDiagnosticProfile $programWWitnessProfile -VerifyInstalledExecutable
    $badWitnessProfile = ConvertFrom-Json (ConvertTo-Json -InputObject $programWWitnessProfile -Depth 10)
    $badWitnessProfile.command.arguments[1] = '--format=csv'
    Test-Rejects 'arbitrary ProgramW6432 witness argv' { Assert-NvidiaSmiDiagnosticProfile $badWitnessProfile }
    $nvidiaContextEvidence = Invoke-NvidiaDiagnosticContextSelfTests $Lot
    $dryRunManifestPath = Join-Path (Join-Path $Lot 'logs') 'selftest-nvidia-diagnostic-dryrun-values.json'
    $dryRunManifest = [ordered]@{schemaVersion=1;profile='NvidiaSmiInventoryDiagnostic';inputs=@();values=@{cudaToolkitPath='C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9';cudaToolkitVersionedPath='C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'}}
    $dryRunManifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $dryRunManifestPath -Encoding UTF8
    try {
        $dryRunValues = Get-ValuesDocument $dryRunManifestPath $nvidiaDiagnosticProfile
        $dryRunResult = Get-ProfileResult $nvidiaDiagnosticProfile $dryRunValues $Lot -DryRunOnly
        $dryRunCase = @($dryRunResult.cases)[0]
        $expectedDiagnosticArgv = Convert-StringArrayToJson @('--query-gpu=count,pci.bus_id,uuid','--format=csv,noheader,nounits')
        if ($dryRunResult.status -cne 'DRY_RUN' -or $dryRunCase.argvJson -cne $expectedDiagnosticArgv -or $dryRunCase.executable -ine (Join-Path $env:SystemRoot 'System32\nvidia-smi.exe')) { throw 'Diagnostic DryRun did not preserve the exact pinned argv and executable.' }
        if ((Test-Path -LiteralPath $dryRunResult.runRoot) -or (Test-Path -LiteralPath $dryRunResult.logs)) { throw 'Diagnostic DryRun created a run or log directory.' }
    } finally { Remove-Item -LiteralPath $dryRunManifestPath -Force -ErrorAction SilentlyContinue }
    $diagnosticToolchain = Get-CargoExecutable
    $diagnosticRunRoot = Join-Path $Lot 'runs\selftest-nvidia-diagnostic'
    $diagnosticLogRoot = Join-Path $Lot 'logs\selftest-nvidia-diagnostic'
    foreach ($path in @($diagnosticRunRoot,$diagnosticLogRoot)) { New-Item -ItemType Directory -Path $path -Force | Out-Null }
    foreach ($name in @('temp','tmp','localappdata','appdata')) { New-Item -ItemType Directory -Path (Join-Path $diagnosticRunRoot $name) -Force | Out-Null }
    $diagnosticEnvironment = @{
        CARGO_HOME = Join-Path $Lot 'cargo-home'; CARGO_TARGET_DIR = $script:CargoTarget
        CARGO_INCREMENTAL = '0'; CARGO_PROFILE_DEV_DEBUG = 'line-tables-only'
        TEMP = Join-Path $diagnosticRunRoot 'temp'; TMP = Join-Path $diagnosticRunRoot 'tmp'
        LOCALAPPDATA = Join-Path $diagnosticRunRoot 'localappdata'; APPDATA = Join-Path $diagnosticRunRoot 'appdata'
        USERPROFILE = $diagnosticRunRoot; HOME = $diagnosticRunRoot
        CUDA_PATH = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'
        CUDA_PATH_V12_9 = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'
        ProgramW6432 = (Get-ValidatedProgramW6432Directory)
    }
    $diagnosticVsEnvironment = @{ PATH='C:\ambient-tool;C:\Program Files\CMake\bin'; INCLUDE='C:\vs\include'; LIB='C:\vs\lib'; ProgramW6432='C:\ambient-programw-spoof' }
    $witnessProfile = Get-Profile $Registry 'NvidiaProgramW6432Witness'
    $witnessValues = [pscustomobject]@{inputs=@{};values=@{cudaToolkitPath=$diagnosticEnvironment.CUDA_PATH;cudaToolkitVersionedPath=$diagnosticEnvironment.CUDA_PATH_V12_9}}
    $witnessBaseEnvironment = Get-ProfileEnvironment $witnessProfile $witnessValues $diagnosticRunRoot $Lot $false
    foreach ($name in @('PROGRAMDATA','ALLUSERSPROFILE','PUBLIC')) { if ($witnessBaseEnvironment.ContainsKey($name)) { throw "ProgramW6432 witness fixture unexpectedly enriched Get-ProfileEnvironment with $name." } }
    if ([string]$witnessBaseEnvironment.ProgramW6432 -ine (Get-ValidatedProgramW6432Directory)) { throw 'ProgramW6432 witness base environment did not come from Get-ProfileEnvironment.' }
    $syntheticToolchain = [pscustomobject]@{cargo='C:\toolchain\cargo.exe';rustc='C:\toolchain\rustc.exe';toolchainBin='C:\toolchain'}
    $witnessProcessPair = New-NvidiaProgramW6432ProcessPair $witnessProfile $witnessBaseEnvironment $diagnosticVsEnvironment $syntheticToolchain $diagnosticRunRoot
    if ($witnessProcessPair.positive.EnvironmentVariables.ContainsKey('ProgramW6432') -eq $false -or $witnessProcessPair.negative.EnvironmentVariables.ContainsKey('ProgramW6432') -or [string]$witnessProcessPair.positive.EnvironmentVariables['ProgramW6432'] -ieq $diagnosticVsEnvironment.ProgramW6432) { throw 'ProgramW6432 witness ProcessStartInfo pair did not use the un-enriched profile environment and validated process value.' }
    $negativePath = [string]$witnessProcessPair.negative.EnvironmentVariables['PATH']
    $witnessProcessPair.negative.EnvironmentVariables['PATH'] = $negativePath + ';C:\synthetic-unlisted-path'
    Test-Rejects 'ProgramW6432 witness rejects a second environment difference' { Assert-NvidiaProgramW6432ProcessPair $witnessProcessPair.positive $witnessProcessPair.negative $witnessProcessPair.validatedProgramW6432 }
    $witnessProcessPair.negative.EnvironmentVariables['PATH'] = $negativePath
    $positiveProgramW = [string]$witnessProcessPair.positive.EnvironmentVariables['ProgramW6432']
    $witnessProcessPair.positive.EnvironmentVariables['ProgramW6432'] = 'C:\synthetic-invalid-programw'
    Test-Rejects 'ProgramW6432 witness rejects a wrong positive value' { Assert-NvidiaProgramW6432ProcessPair $witnessProcessPair.positive $witnessProcessPair.negative $witnessProcessPair.validatedProgramW6432 }
    $witnessProcessPair.positive.EnvironmentVariables['ProgramW6432'] = $positiveProgramW
    $inventoryProfile = Get-Profile $Registry 'NvidiaSmiInventoryDiagnostic'
    $inventoryBaseEnvironment = Get-ProfileEnvironment $inventoryProfile $witnessValues $diagnosticRunRoot $Lot $false
    foreach ($name in @('PROGRAMDATA','ALLUSERSPROFILE','PUBLIC')) { if ($inventoryBaseEnvironment.ContainsKey($name)) { throw "NVIDIA inventory fixture unexpectedly enriched Get-ProfileEnvironment with $name." } }
    $inventoryPsi = New-CargoProcessInfo $syntheticToolchain.cargo $syntheticToolchain.rustc $syntheticToolchain.toolchainBin @() $inventoryBaseEnvironment $diagnosticVsEnvironment $diagnosticRunRoot $inventoryProfile.id
    $inventoryPsi.FileName = Get-FullPath (Join-Path $env:SystemRoot 'System32\nvidia-smi.exe')
    $inventoryPsi.Arguments = [string]::Join(' ',[string[]]@($inventoryProfile.command.arguments | ForEach-Object { Get-WindowsCommandLineArgument ([string]$_) }))
    if (-not $inventoryPsi.EnvironmentVariables.ContainsKey('ProgramW6432') -or [string]$inventoryPsi.EnvironmentVariables['ProgramW6432'] -ine (Get-ValidatedProgramW6432Directory) -or [string]$inventoryPsi.EnvironmentVariables['ProgramW6432'] -ieq $diagnosticVsEnvironment.ProgramW6432) { throw 'NVIDIA inventory fixture did not construct a compatible Cargo-backed PSI with validated ProgramW6432.' }
    $validUuid = 'GPU-12345678-abcd-1234-ABCD-1234567890ab'
    $validPositiveEnvelope = [pscustomobject]@{result=[pscustomobject]@{status='DIAGNOSTIC_ONLY';exitCode=0};outputLines=@("1, 0000:01:00.0, $validUuid")}
    $validNegativeEnvelope = [pscustomobject]@{result=[pscustomobject]@{status='DIAGNOSTIC_ONLY';exitCode=255};outputLines=@('provider unavailable')}
    $validSequenceCalls = [Collections.Generic.List[string]]::new()
    $validSequenceInvoker = { param([string]$Role); $validSequenceCalls.Add($Role); if ($Role -ceq 'positive') { $validPositiveEnvelope } else { $validNegativeEnvelope } }.GetNewClosure()
    $validSequence = Invoke-NvidiaProgramW6432CallSequence $validSequenceInvoker
    if ($validSequence.calls -ne 2 -or [string]::Join('|',$validSequenceCalls.ToArray()) -cne 'positive|negative' -or $validSequence.positiveEvidence.uuid -cne $validUuid) { throw 'ProgramW6432 witness valid synthetic pair did not perform exactly positive then negative.' }
    $invalidPositiveFixtures = @(
        [pscustomobject]@{name='unexpected provider failure';envelope=[pscustomobject]@{result=[pscustomobject]@{status='DIAGNOSTIC_ONLY';exitCode=255};outputLines=@('provider unavailable')}},
        [pscustomobject]@{name='empty output';envelope=[pscustomobject]@{result=[pscustomobject]@{status='DIAGNOSTIC_ONLY';exitCode=0};outputLines=@('','  ')}},
        [pscustomobject]@{name='multiple UUID lines';envelope=[pscustomobject]@{result=[pscustomobject]@{status='DIAGNOSTIC_ONLY';exitCode=0};outputLines=@("1, 0000:01:00.0, $validUuid",'1, 0000:02:00.0, GPU-87654321-abcd-1234-ABCD-1234567890ab')}}
    )
    foreach ($fixture in $invalidPositiveFixtures) {
        $fixtureCalls = [Collections.Generic.List[string]]::new()
        $fixtureInvoker = { param([string]$Role); $fixtureCalls.Add($Role); if ($Role -ceq 'positive') { $fixture.envelope } else { $validNegativeEnvelope } }.GetNewClosure()
        Test-Rejects "ProgramW6432 witness $($fixture.name) stops before negative call" { Invoke-NvidiaProgramW6432CallSequence $fixtureInvoker }
        if ($fixtureCalls.Count -ne 1 -or $fixtureCalls[0] -cne 'positive') { throw "ProgramW6432 witness $($fixture.name) fixture invoked a second call after invalid positive output." }
    }
    $syntheticPowerShell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $moduleCaptureRun = Join-Path $diagnosticRunRoot 'module-capture'
    $moduleCaptureLogs = Join-Path $diagnosticLogRoot 'module-capture'
    New-Item -ItemType Directory -Path $moduleCaptureRun,$moduleCaptureLogs -Force | Out-Null
    foreach ($name in @('temp','tmp','localappdata','appdata','programdata','public')) { New-Item -ItemType Directory -Path (Join-Path $moduleCaptureRun $name) -Force | Out-Null }
    $moduleCaptureHost = @{
        SystemRoot=$env:SystemRoot; WINDIR=$env:WINDIR; SystemDrive=$env:SystemDrive; ComSpec=$env:ComSpec; PATHEXT=$env:PATHEXT; OS=$env:OS
        PROCESSOR_ARCHITECTURE=$env:PROCESSOR_ARCHITECTURE; NUMBER_OF_PROCESSORS=$env:NUMBER_OF_PROCESSORS; PATH=[string]::Join(';',@((Join-Path $env:SystemRoot 'System32'),$env:SystemRoot))
    }
    $moduleCaptureContext = New-NvidiaDiagnosticContextEnvironment 'clean' $diagnosticEnvironment $moduleCaptureHost $moduleCaptureRun
    $moduleCaptureScript = "[Console]::Out.WriteLine('N2_MODULE_CAPTURE_STDOUT'); [Console]::Error.WriteLine('N2_MODULE_CAPTURE_STDERR'); Start-Sleep -Milliseconds 150; exit 0"
    $moduleCaptureArgs = @('-NoLogo','-NoProfile','-NonInteractive','-EncodedCommand',[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($moduleCaptureScript)))
    $moduleCaptureResult = Invoke-NvidiaSmiDiagnostic $nvidiaCompareProfile $moduleCaptureContext.environment @{} $null $moduleCaptureRun $moduleCaptureLogs $false -SyntheticSelfTest -TestExecutable $syntheticPowerShell -TestArguments $moduleCaptureArgs -ContextId 'clean' -ContextEvidence $moduleCaptureContext.evidence
    $moduleCaptureReceipt = Get-Content -LiteralPath $moduleCaptureResult.invocation -Raw | ConvertFrom-Json -ErrorAction Stop
    if ($moduleCaptureResult.status -cne 'DIAGNOSTIC_ONLY' -or [int]$moduleCaptureResult.exitCode -ne 0 -or $moduleCaptureReceipt.moduleCapture.coverage -cne 'POLLING_ONLY_NONEXHAUSTIVE' -or $moduleCaptureReceipt.moduleCapture.absenceConclusion -cne 'NOT_SUPPORTED' -or [int]$moduleCaptureReceipt.moduleCapture.sampleCount -lt 1 -or @($moduleCaptureReceipt.moduleCapture.observedModuleUnion).Count -lt 1) { throw 'Synthetic child module polling did not record bounded, explicitly non-exhaustive observations.' }
    if ($moduleCaptureReceipt.executableSha256 -cne (Get-Sha256 $syntheticPowerShell) -or $moduleCaptureReceipt.stdoutSha256 -cne (Get-Sha256 $moduleCaptureResult.stdout) -or $moduleCaptureReceipt.stderrSha256 -cne (Get-Sha256 $moduleCaptureResult.stderr)) { throw 'Synthetic child executable or stream SHA-256 evidence did not match its files.' }
    $emptyModuleCapture = New-NvidiaModuleCapture 999
    $emptyModuleCapture.processExitedBeforeFirstSample = $true
    $emptyModuleEvidence = Complete-NvidiaModuleCapture $emptyModuleCapture
    if ($emptyModuleEvidence.nvmlObservation -cne 'NO_SAMPLE' -or $emptyModuleEvidence.absenceConclusion -cne 'NOT_SUPPORTED') { throw 'No-sample module capture implied a negative DLL conclusion.' }
    $collectorErrorCapture = New-NvidiaModuleCapture 998
    Capture-NvidiaProcessModules $null $collectorErrorCapture
    $collectorErrorEvidence = Complete-NvidiaModuleCapture $collectorErrorCapture
    if ([int]$collectorErrorEvidence.errorCount -lt 1 -or @($collectorErrorEvidence.errors).Count -lt 1 -or $collectorErrorEvidence.coverage -cne 'POLLING_ONLY_NONEXHAUSTIVE' -or $collectorErrorEvidence.absenceConclusion -cne 'NOT_SUPPORTED') {
        throw "Synthetic null-process module collection did not preserve collector-error evidence: errorCount=$($collectorErrorEvidence.errorCount); errors=$([string]::Join('|',@($collectorErrorEvidence.errors))); coverage=$($collectorErrorEvidence.coverage); absenceConclusion=$($collectorErrorEvidence.absenceConclusion)."
    }
    $moduleCaptureSelfTestEvidence = [pscustomobject]@{status='PASS';sampleCount=[int]$moduleCaptureReceipt.moduleCapture.sampleCount;coverage=$moduleCaptureReceipt.moduleCapture.coverage;nvmlObservation=$moduleCaptureReceipt.moduleCapture.nvmlObservation;absenceConclusion=$moduleCaptureReceipt.moduleCapture.absenceConclusion;unionCount=@($moduleCaptureReceipt.moduleCapture.observedModuleUnion).Count;metadataCount=@($moduleCaptureReceipt.moduleCapture.nvidiaModuleMetadata).Count;stdoutSha256=$moduleCaptureReceipt.stdoutSha256;stderrSha256=$moduleCaptureReceipt.stderrSha256;zeroSampleCase=$emptyModuleEvidence.nvmlObservation;collectorError=[pscustomobject]@{status='PASS';errorCount=$collectorErrorEvidence.errorCount;errors=@($collectorErrorEvidence.errors);coverage=$collectorErrorEvidence.coverage;absenceConclusion=$collectorErrorEvidence.absenceConclusion}}
    foreach ($scenario in @(
        [pscustomobject]@{name='success';exit=0},
        [pscustomobject]@{name='provider-error';exit=23}
    )) {
        $scriptText = "[Console]::Out.WriteLine('N2_DIAGNOSTIC_STDOUT'); [Console]::Error.WriteLine('N2_DIAGNOSTIC_STDERR'); exit $($scenario.exit)"
        $encodedScript = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($scriptText))
        $syntheticArgs = @('-NoLogo','-NoProfile','-NonInteractive','-EncodedCommand',$encodedScript)
        $scenarioRun = Join-Path $diagnosticRunRoot $scenario.name
        $scenarioLogs = Join-Path $diagnosticLogRoot $scenario.name
        foreach ($path in @($scenarioRun,$scenarioLogs)) { New-Item -ItemType Directory -Path $path -Force | Out-Null }
        foreach ($name in @('temp','tmp','localappdata','appdata')) { New-Item -ItemType Directory -Path (Join-Path $scenarioRun $name) -Force | Out-Null }
        $scenarioEnvironment = @{} + $diagnosticEnvironment
        foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) {
            $scenarioEnvironment[$name] = Join-Path $scenarioRun $name.ToLowerInvariant()
            if ($name -in @('TEMP','TMP','LOCALAPPDATA','APPDATA')) { New-Item -ItemType Directory -Path $scenarioEnvironment[$name] -Force | Out-Null }
        }
        $beforeDiagnosticSubscribers = @((Get-EventSubscriber -ErrorAction SilentlyContinue | Where-Object { $_.SourceIdentifier -like 'n2-*' } | ForEach-Object { [string]$_.SourceIdentifier }) | Sort-Object)
        $beforeDiagnosticJobs = @((Get-Job -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'n2-*' } | ForEach-Object { [string]$_.InstanceId }) | Sort-Object)
        $diagnosticEvidence = Invoke-NvidiaSmiDiagnostic $nvidiaDiagnosticProfile $scenarioEnvironment $diagnosticVsEnvironment $diagnosticToolchain $scenarioRun $scenarioLogs $false -SyntheticSelfTest -TestExecutable $syntheticPowerShell -TestArguments $syntheticArgs
        if ($diagnosticEvidence.status -cne 'DIAGNOSTIC_ONLY' -or [int]$diagnosticEvidence.exitCode -ne [int]$scenario.exit) { throw "Synthetic diagnostic $($scenario.name) lost its provider exit status or data-only classification." }
        $receipt = Get-Content -LiteralPath $diagnosticEvidence.invocation -Raw | ConvertFrom-Json -ErrorAction Stop
        if ($receipt.result -cne 'DIAGNOSTIC_ONLY' -or $receipt.executable -ine $syntheticPowerShell -or [int]$receipt.exitCode -ne [int]$scenario.exit -or $receipt.classification -cne 'DIAGNOSTIC_ONLY') { throw "Synthetic diagnostic $($scenario.name) invocation receipt was incomplete." }
        if ((Get-Content -LiteralPath $diagnosticEvidence.stdout -Raw) -notmatch 'N2_DIAGNOSTIC_STDOUT' -or (Get-Content -LiteralPath $diagnosticEvidence.stderr -Raw) -notmatch 'N2_DIAGNOSTIC_STDERR') { throw "Synthetic diagnostic $($scenario.name) did not retain both process streams." }
        $afterDiagnosticSubscribers = @((Get-EventSubscriber -ErrorAction SilentlyContinue | Where-Object { $_.SourceIdentifier -like 'n2-*' } | ForEach-Object { [string]$_.SourceIdentifier }) | Sort-Object)
        $afterDiagnosticJobs = @((Get-Job -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'n2-*' } | ForEach-Object { [string]$_.InstanceId }) | Sort-Object)
        if ([string]::Join('|',$beforeDiagnosticSubscribers) -cne [string]::Join('|',$afterDiagnosticSubscribers) -or [string]::Join('|',$beforeDiagnosticJobs) -cne [string]::Join('|',$afterDiagnosticJobs)) { throw "Synthetic diagnostic $($scenario.name) leaked a watcher subscription or process-stream job." }
    }
    $launchFailureRun = Join-Path $diagnosticRunRoot 'launch-failure'
    $launchFailureLogs = Join-Path $diagnosticLogRoot 'launch-failure'
    foreach ($path in @($launchFailureRun,$launchFailureLogs)) { New-Item -ItemType Directory -Path $path -Force | Out-Null }
    foreach ($name in @('temp','tmp','localappdata','appdata')) { New-Item -ItemType Directory -Path (Join-Path $launchFailureRun $name) -Force | Out-Null }
    $launchEnvironment = @{} + $diagnosticEnvironment
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) {
        $launchEnvironment[$name] = Join-Path $launchFailureRun $name.ToLowerInvariant()
        if ($name -in @('TEMP','TMP','LOCALAPPDATA','APPDATA')) { New-Item -ItemType Directory -Path $launchEnvironment[$name] -Force | Out-Null }
    }
    $launchFailureMessage = $null
    try { Invoke-NvidiaSmiDiagnostic $nvidiaDiagnosticProfile $launchEnvironment $diagnosticVsEnvironment $diagnosticToolchain $launchFailureRun $launchFailureLogs $false -SyntheticSelfTest -TestExecutable (Join-Path $launchFailureRun 'missing.exe') -TestArguments @('arg') | Out-Null } catch { $launchFailureMessage = $_.Exception.Message }
    $launchReceiptPath = Join-Path $launchFailureLogs 'nvidia-smi-query-gpu.invocation.json'
    if (-not (Test-Path -LiteralPath $launchReceiptPath -PathType Leaf)) { throw "Synthetic diagnostic launch did not leave its failure receipt: $launchFailureMessage" }
    $launchReceipt = Get-Content -LiteralPath $launchReceiptPath -Raw | ConvertFrom-Json -ErrorAction Stop
    if ($launchReceipt.result -cne 'FAIL' -or [string]::IsNullOrWhiteSpace([string]$launchReceipt.failure)) { throw 'Synthetic diagnostic launch failure did not retain a failed invocation receipt.' }
    $afterFailureSubscribers = @((Get-EventSubscriber -ErrorAction SilentlyContinue | Where-Object { $_.SourceIdentifier -like 'n2-*' } | ForEach-Object { [string]$_.SourceIdentifier }) | Sort-Object)
    $afterFailureJobs = @((Get-Job -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'n2-*' } | ForEach-Object { [string]$_.InstanceId }) | Sort-Object)
    if ($afterFailureSubscribers.Count -gt 0 -or $afterFailureJobs.Count -gt 0) { throw 'Synthetic diagnostic launch failure leaked a watcher subscription or process-stream job.' }
    $watcherFailureRun = Join-Path $diagnosticRunRoot 'watcher-failure'
    $watcherFailureLogs = Join-Path $diagnosticLogRoot 'watcher-failure'
    foreach ($path in @($watcherFailureRun,$watcherFailureLogs)) { New-Item -ItemType Directory -Path $path -Force | Out-Null }
    foreach ($name in @('temp','tmp','localappdata','appdata')) { New-Item -ItemType Directory -Path (Join-Path $watcherFailureRun $name) -Force | Out-Null }
    $watcherEnvironment = @{} + $diagnosticEnvironment
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) {
        $watcherEnvironment[$name] = Join-Path $watcherFailureRun $name.ToLowerInvariant()
        if ($name -in @('TEMP','TMP','LOCALAPPDATA','APPDATA')) { New-Item -ItemType Directory -Path $watcherEnvironment[$name] -Force | Out-Null }
    }
    $watcherFailureMessage = $null
    try { Invoke-NvidiaSmiDiagnostic $nvidiaDiagnosticProfile $watcherEnvironment $diagnosticVsEnvironment $diagnosticToolchain $watcherFailureRun $watcherFailureLogs $false -SyntheticSelfTest -TestExecutable $syntheticPowerShell -TestArguments $syntheticArgs -InjectWatcherFailure | Out-Null } catch { $watcherFailureMessage = $_.Exception.Message }
    $watcherFailureReceiptPath = Join-Path $watcherFailureLogs 'nvidia-smi-query-gpu.invocation.json'
    if (-not (Test-Path -LiteralPath $watcherFailureReceiptPath -PathType Leaf)) { throw "Synthetic watcher violation did not retain a failure receipt: $watcherFailureMessage" }
    $watcherFailureReceipt = Get-Content -LiteralPath $watcherFailureReceiptPath -Raw | ConvertFrom-Json -ErrorAction Stop
    if ($watcherFailureReceipt.result -cne 'FAIL' -or -not $watcherFailureReceipt.syntheticWatcherFailure -or [string]::IsNullOrWhiteSpace([string]$watcherFailureReceipt.failure) -or [string]::Join('|',$watcherFailureReceipt.syntheticWatcherEvents) -cne 'Created|Created|WatcherError') { throw 'Synthetic watcher violation sequence was not retained as FAIL.' }
    foreach ($resource in @('watchersDisposed','watcherHandlersRemoved','stdoutDisposed','stderrDisposed','streamHandlersRemoved','processStopped','processDisposed')) {
        if (-not [bool]$watcherFailureReceipt.cleanup.$resource) { throw "Synthetic watcher violation left cleanup evidence false for $resource." }
    }
    $afterWatcherSubscribers = @((Get-EventSubscriber -ErrorAction SilentlyContinue | Where-Object { $_.SourceIdentifier -like 'n2-*' } | ForEach-Object { [string]$_.SourceIdentifier }) | Sort-Object)
    $afterWatcherJobs = @((Get-Job -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'n2-*' } | ForEach-Object { [string]$_.InstanceId }) | Sort-Object)
    if ($afterWatcherSubscribers.Count -gt 0 -or $afterWatcherJobs.Count -gt 0) { throw 'Synthetic watcher violation leaked an event subscription or job.' }
    $beforeStreamSubscribers = @((Get-EventSubscriber -ErrorAction SilentlyContinue | Where-Object { $_.SourceIdentifier -like 'n2-stream-*' } | ForEach-Object { [string]$_.SourceIdentifier }) | Sort-Object)
    $beforeStreamJobs = @((Get-Job -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'n2-stream-*' } | ForEach-Object { [string]$_.InstanceId }) | Sort-Object)
    $partialProcess = New-Object Diagnostics.Process
    Test-Rejects 'partial output-handler registration' { Register-ProcessStreamHandlers $partialProcess ([Collections.Concurrent.ConcurrentQueue[object]]::new()) 1 }
    $partialProcess.Dispose()
    $afterStreamSubscribers = @((Get-EventSubscriber -ErrorAction SilentlyContinue | Where-Object { $_.SourceIdentifier -like 'n2-stream-*' } | ForEach-Object { [string]$_.SourceIdentifier }) | Sort-Object)
    $afterStreamJobs = @((Get-Job -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'n2-stream-*' } | ForEach-Object { [string]$_.InstanceId }) | Sort-Object)
    if ([string]::Join('|',$beforeStreamSubscribers) -cne [string]::Join('|',$afterStreamSubscribers) -or [string]::Join('|',$beforeStreamJobs) -cne [string]::Join('|',$afterStreamJobs)) { throw 'Partial stream-handler initialization left a subscription or job behind.' }
    $manifestRoot = Join-Path $Lot ('selftest-manifest-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $manifestRoot -Force | Out-Null
    try {
        $manifestPath = Join-Path $manifestRoot 'valid.json'
        $validManifest = [ordered]@{ schemaVersion=1; profile='PreparationNoRun'; inputs=@(); values=@{ cudaToolkitPath='C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'; cudaToolkitVersionedPath='C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'; libclangPath='C:/dev/whisper/.wt/r8/target/n2-native/toolchain/libclang.dll' } }
        $validManifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $manifestPath -Encoding UTF8
        $validated = Get-ValuesDocument $manifestPath $prepareProfile
        if ($validated.values.cudaToolkitPath -ine 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9' -or $validated.values.cudaToolkitVersionedPath -ine 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9') { throw 'Approved CUDA tool paths were changed while validating their manifest.' }
        $invalidPath = Join-Path $manifestRoot 'invalid.json'
        $invalidManifest = [ordered]@{ schemaVersion=1; profile='PreparationNoRun'; inputs=@(); values=@{ cudaToolkitPath='C:\Windows'; cudaToolkitVersionedPath='C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'; libclangPath='C:/dev/whisper/.wt/r8/target/n2-native/toolchain/libclang.dll' } }
        $invalidManifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $invalidPath -Encoding UTF8
        Test-Rejects 'unapproved tool directory in manifest' { Get-ValuesDocument $invalidPath $prepareProfile }
    } finally {
        if (Test-InRoot $manifestRoot $Lot) { Remove-Item -LiteralPath $manifestRoot -Recurse -Force }
    }
    Test-Rejects 'unknown Cargo target' { New-CargoArguments ([pscustomobject]@{ operation='Run';package='workspace';features=@() }) ([pscustomobject]@{ targetKind='weird';exactTest='safe' }) }
    $temp = Join-Path $env:LOCALAPPDATA 'Temp'
    $allowed = @($Lot,$script:CargoTarget)
    $protected = @((Join-Path $env:USERPROFILE '.cargo'),(Join-Path $env:LOCALAPPDATA 'Whisper'))
    $unknownTemp = [pscustomobject]@{type='Created';path=(Join-Path $temp 'synthetic-unattributed.tmp');old_path='';at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic-only'}
    if ((Get-EventDisposition $unknownTemp $allowed $protected $temp) -cne 'UNATTRIBUTED_TEMP') { throw 'Unattributed Temp event was not classified UNATTRIBUTED.' }
    $insideRename = [pscustomobject]@{type='Renamed';path=(Join-Path $temp 'new.tmp');old_path=(Join-Path $temp 'old.tmp');at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic-only'}
    if ((Get-EventDisposition $insideRename $allowed $protected $temp) -cne 'UNATTRIBUTED_TEMP') { throw 'In-Temp rename was not classified UNATTRIBUTED.' }
    $crossRename = [pscustomobject]@{type='Renamed';path=(Join-Path $temp 'new.tmp');old_path=(Join-Path $Lot 'old.tmp');at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic-only'}
    if ((Get-EventDisposition $crossRename $allowed $protected $temp) -cne 'VIOLATION') { throw 'Cross-boundary rename was not rejected.' }
    $insideWrite = [pscustomobject]@{type='Created';path=(Join-Path $Lot 'marker');old_path='';at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic-only'}
    if ((Get-EventDisposition $insideWrite $allowed $protected $temp) -cne 'ALLOWED') { throw 'Write under allowed lot root was rejected.' }
    $outsideWrite = [pscustomobject]@{type='Created';path=(Join-Path $env:USERPROFILE 'Downloads\not-created');old_path='';at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic-only'}
    if ((Get-EventDisposition $outsideWrite $allowed $protected $temp) -cne 'VIOLATION') { throw 'Write outside campaign roots was not rejected.' }
    $watcherError = [pscustomobject]@{type='WatcherError';path='';old_path='';at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic-only'}
    if ((Get-EventDisposition $watcherError $allowed $protected $temp) -cne 'VIOLATION') { throw 'Watcher error was not rejected.' }
    $stagingEvidence = Invoke-StagingSelfTests $Lot
    $personalGpuSettingsEvidence = Invoke-PersonalGpuSettingsParentSelfTest $Lot
    $voxPopuliEvidence = Invoke-VoxPopuliContractSelfTests $Registry $Lot
    $watchDir = Join-Path $Lot ('selftest-watch-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $watchDir -Force | Out-Null
    $state = Start-WatchState @([pscustomobject]@{path=$watchDir;recursive=$true})
    try {
        $marker = Join-Path $watchDir 'allowed-marker.txt'
        [IO.File]::WriteAllText($marker,'watcher self-test')
        Start-Sleep -Milliseconds 150
        Receive-WatcherEvents $state
        $seen = $false
        $observed = $null
        while ($state.queue.TryDequeue([ref]$observed)) {
            if ($observed.path -ieq $marker -and $observed.type -eq 'Created') { $seen = $true }
            $observed = $null
        }
        if (-not $seen) { throw 'FileSystemWatcher did not observe a synthetic in-lot marker.' }
    } finally {
        Stop-WatchState $state
        if (Test-InRoot $watchDir $Lot) { Remove-Item -LiteralPath $watchDir -Recurse -Force }
    }
    $selfTestEvidence = [ordered]@{
        schemaVersion = 1; mode = 'PowerShell synthetic only'; cargoOrNativeTestsStarted = $false
        r1ProfileEnvironment = 'PASS: declared mic/L04/L2/CUDA variables passed; ambient sentinels absent; isolated roots retained'
        r2ProcessStreams = @($streamNormal,$streamFailure,[pscustomobject]@{ partialInitializationCleanup='PASS'; subscriberCount=$afterStreamSubscribers.Count; jobCount=$afterStreamJobs.Count })
        r3TestBodyParser = @('one-line test output PASS','multiline WASAPI inventory output PASS','multiline WASAPI capture output PASS','distinct NO_RUN_PASS/LIST_PASS','diagnostic body remains DIAGNOSTIC_ONLY','rejected ignored-only','rejected wrong test','rejected incomplete summary','rejected multiple bodies','rejected nonzero exit')
        r4InvocationReceipts = $receiptEvidence
        nvidiaEnvironmentContexts = $nvidiaContextEvidence
        programW6432WitnessProcessPair = $witnessProcessPair.evidence
        programW6432WitnessCallOrder = [pscustomobject]@{validCalls=@($validSequenceCalls.ToArray());invalidPositiveFixtures=@($invalidPositiveFixtures | ForEach-Object { $_.name });invalidPositiveStoppedAtOneCall=$true}
        programW6432InventoryProcessInfo = [pscustomobject]@{builder='New-CargoProcessInfo';profileEnvironmentRoots=@('CARGO_HOME','CARGO_TARGET_DIR','TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME');extraHostRootsAdded=$false;validatedProgramW6432Present=$inventoryPsi.EnvironmentVariables.ContainsKey('ProgramW6432');childStarted=$false}
        programW6432Resolver = $programWResolverEvidence
        programW6432ProfileAllowlist = 'PASS: only GPU/NVIDIA profiles listed; CPU, Auto fallback, and discovery profiles excluded; pending VoxPopuli Run profiles remain non-executable'
        programW6432CargoEnvironment = 'PASS: validated host value passed only to approved GPU Cargo profiles; Visual Studio spoof ignored'
        programW6432WitnessPair = $nvidiaContextEvidence.programW6432WitnessPair
        nvidiaChildModuleCapture = $moduleCaptureSelfTestEvidence
        discoveryArgv = @('PreparationNoRun','InventoryList','WasapiInventory','WasapiCapture')
        lotRootPolicy = $lotRootEvidence
        staging = $stagingEvidence
        personalGpuSettingsParent = $personalGpuSettingsEvidence
        voxPopuliContractAndDeadline = $voxPopuliEvidence
        watcherAndRoots = @('allowed in-lot event','disallowed external event','Temp UNATTRIBUTED','cross-boundary rename rejected','watcher error rejected','live FileSystemWatcher observed in-lot marker')
    }
    $selfTestPath = Join-Path (Join-Path $Lot 'logs') 'guard-selftest-reusable-roots.json'
    $selfTestEvidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $selfTestPath -Encoding UTF8
    "PASS native guard self-tests: exact NVIDIA DryRun argv without VsDevCmd, pinned clean/host-filtered comparison contexts, synthetic child module polling and provider-error/launch-error receipts, multiple watcher violations plus WatcherError cleanup, stdout/stderr capture, stream cleanup, profile-bound N2/main-facades roots and existing guard fixtures; evidence=$selfTestPath; no Cargo/native diagnostic command started."
}

function Get-GitDiffIdentity {
    $head = @(& git.exe -C $script:Repo --no-optional-locks rev-parse HEAD)
    if ($LASTEXITCODE -ne 0) { throw 'Could not read candidate HEAD.' }
    return ($head -join "`n")
}

function Get-ProfileResult([object]$Profile, [object]$ValuesDoc, [string]$Lot, [switch]$DryRunOnly) {
    $runId = '{0}-{1}' -f $Profile.id,([guid]::NewGuid().ToString('N'))
    $script:RunRoot = Join-Path (Join-Path $Lot 'runs') $runId
    $script:LogRoot = Join-Path (Join-Path $Lot 'logs') $runId
    foreach ($root in @($script:RunRoot,$script:LogRoot)) {
        if (-not (Test-InRoot $root $Lot)) { throw "Generated output escaped candidate lot: $root" }
        Assert-NoReparseChain $root 'generated campaign output root'
        if (Test-Path -LiteralPath $root) { throw "Generated N2 run path already exists: $root" }
    }
    if (-not $DryRunOnly) {
        New-Item -ItemType Directory -Path $script:RunRoot,$script:LogRoot,(Join-Path $script:RunRoot 'temp'),(Join-Path $script:RunRoot 'tmp'),(Join-Path $script:RunRoot 'localappdata'),(Join-Path $script:RunRoot 'appdata') -Force | Out-Null
    }
    $environment = Get-ProfileEnvironment $Profile $ValuesDoc $script:RunRoot $Lot (-not $DryRunOnly)
    Assert-CampaignRoots $Lot $script:RunRoot $environment
    $stagingEvidence = if (-not $DryRunOnly) { Copy-ProfileStaging $Profile $ValuesDoc $environment.LOCALAPPDATA $script:RunRoot } else { @() }
    $voxContract = Get-VoxPopuliProfileContract ([string]$Profile.id)
    if (-not $DryRunOnly -and $null -ne $voxContract -and $voxContract.operation -ceq 'Run') {
        $settingsPath = Join-Path $environment.LOCALAPPDATA 'Whisper\settings'
        if (Test-Path -LiteralPath $settingsPath) { throw 'VoxPopuli staging unexpectedly precreated settings; the product must create it.' }
    }
    if (-not $DryRunOnly) { $null = Initialize-PersonalGpuSettingsParent $Profile.id $script:RunRoot }
    $toolchain = if ($Profile.operation -ceq 'Diagnostic') { if ($Profile.id -in @('NvidiaProgramW6432Witness','NvidiaSmiInventoryDiagnostic') -and -not $DryRunOnly) { Get-CargoExecutable } else { $null } } else { Get-CargoExecutable }
    if ($Profile.operation -ceq 'Diagnostic') {
        if ($Profile.id -ceq 'NvidiaSmiEnvironmentCompareDiagnostic') {
            $outcome = Invoke-NvidiaSmiEnvironmentComparison $Profile $environment $toolchain $script:RunRoot $script:LogRoot ([bool]$DryRunOnly)
            return [pscustomobject]@{ profile=$Profile.id; status=$outcome.status; cases=@($outcome.cases); comparison=$outcome.comparison; staging=@($stagingEvidence); runRoot=$script:RunRoot; logs=$script:LogRoot }
        }
        if ($Profile.id -ceq 'NvidiaProgramW6432Witness') {
            $vsEnvironment = @{}
            if (-not $DryRunOnly) { $vsEnvironment = Get-VsEnvironment (Get-VsDevCommand) $Environment.TEMP $Environment.TMP $script:RunRoot }
            $outcome = Invoke-NvidiaProgramW6432Witness $Profile $environment $vsEnvironment $toolchain $script:RunRoot $script:LogRoot ([bool]$DryRunOnly)
            return [pscustomobject]@{ profile=$Profile.id; status=$outcome.status; cases=@($outcome.cases); witness=$outcome.witness; staging=@($stagingEvidence); runRoot=$script:RunRoot; logs=$script:LogRoot }
        }
        $vsEnvironment = @{}
        if ($Profile.id -ceq 'NvidiaSmiInventoryDiagnostic' -and -not $DryRunOnly) { $vsEnvironment = Get-VsEnvironment (Get-VsDevCommand) $Environment.TEMP $Environment.TMP $script:RunRoot }
        $outcome = Invoke-NvidiaSmiDiagnostic $Profile $environment $vsEnvironment $toolchain $script:RunRoot $script:LogRoot ([bool]$DryRunOnly)
        return [pscustomobject]@{ profile=$Profile.id; status=$outcome.status; cases=@($outcome); staging=@($stagingEvidence); runRoot=$script:RunRoot; logs=$script:LogRoot }
    }
    if ($Profile.operation -ne 'Run') {
        $outcome = Invoke-OneCargoCase $Profile $null $environment @{} $toolchain $script:RunRoot $script:LogRoot ([bool]$DryRunOnly)
        return [pscustomobject]@{ profile=$Profile.id; status=$outcome.status; cases=@($outcome); staging=@($stagingEvidence); runRoot=$script:RunRoot; logs=$script:LogRoot }
    }
    $caseResults = [Collections.Generic.List[object]]::new()
    foreach ($case in @($Profile.cases)) {
        $caseResult = Invoke-OneCargoCase $Profile $case $environment @{} $toolchain $script:RunRoot $script:LogRoot ([bool]$DryRunOnly)
        $caseResults.Add($caseResult)
        if ($caseResult.status -eq 'FAIL') { throw "Profile $($Profile.id) stopped at first failed case $($case.exactTest)." }
    }
    $status = if ([bool]$DryRunOnly) { 'DRY_RUN' } elseif (@($caseResults | Where-Object status -eq 'DIAGNOSTIC_ONLY').Count -eq $caseResults.Count) { 'DIAGNOSTIC_ONLY' } else { 'PASS' }
    return [pscustomobject]@{ profile=$Profile.id; status=$status; cases=@($caseResults.ToArray()); staging=@($stagingEvidence); runRoot=$script:RunRoot; logs=$script:LogRoot }
}

function Assert-NativeBuildProfile([object]$Entry) {
    $variant = switch -CaseSensitive ([string]$Entry.id) { 'NativeDllCpu' { 'cpu' }; 'NativeDllGpu' { 'gpu' }; default { throw 'Unknown native DLL build profile.' } }
    $keys = @($Entry.PSObject.Properties.Name | Sort-Object) -join '|'
    if ($keys -cne 'cache|id|outputs|script|timeoutSeconds|variant|workspace' -or $Entry.workspace -cne "C:/wb/native/$variant" -or $Entry.variant -cne $variant -or $Entry.script -cne 'ci/build-native.ps1' -or $Entry.cache -cne 'native/cache' -or $Entry.outputs -cne 'target/native-build' -or $Entry.timeoutSeconds -ne 3600) {
        throw 'Native DLL build profile differs from its bounded contract.'
    }
}

function Get-NativeBuildProfile([string]$Id) {
    $registry = Get-Content -LiteralPath $script:ProfilesPath -Raw | ConvertFrom-Json
    $entries = @($registry.nativeBuildProfiles)
    if ($registry.schemaVersion -ne 1 -or $entries.Count -ne 2) { throw 'Expected exactly two native DLL build profiles.' }
    foreach ($entry in $entries) { Assert-NativeBuildProfile $entry }
    $selected = @($entries | Where-Object id -CEQ $Id)
    if ($selected.Count -ne 1) { throw 'Native DLL build profile is missing or duplicated.' }
    return $selected[0]
}

function New-NativeBuildProcessInfo([object]$Entry, [string]$BuildId, [string]$Root, [string]$Probe = '') {
    Assert-NativeBuildProfile $Entry
    if ($BuildId -cnotmatch '^[a-zA-Z0-9][a-zA-Z0-9_-]{0,31}$') { throw 'Native DLL build identifier is invalid.' }
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = Join-Path $PSHOME 'pwsh.exe'
    $fullRoot=Get-FullPath $Root
    if ($fullRoot -ieq $script:Lot -or -not (Test-InRoot $fullRoot $script:Lot)) { throw 'Native DLL working directory must be a child of the declared execution root.' }
    Assert-NoReparseChain $fullRoot 'native DLL working directory'
    $psi.WorkingDirectory = $fullRoot
    $psi.UseShellExecute=$false; $psi.CreateNoWindow=$true
    $psi.RedirectStandardOutput=$true; $psi.RedirectStandardError=$true
    $childScript = Get-FullPath (Join-Path $script:Repo $Entry.script)
    Assert-NoReparseChain $childScript 'native DLL build script'
    foreach ($arg in @('-NoLogo','-NoProfile','-NonInteractive','-File',$childScript)) { $psi.ArgumentList.Add($arg) }
    if ($Probe -eq 'SelfTest') { $psi.ArgumentList.Add('-SelfTest') }
    else {
        foreach ($arg in @('-Variant',[string]$Entry.variant,'-BuildId',$BuildId)) { $psi.ArgumentList.Add($arg) }
        if ($NativePrepareOnly) { $psi.ArgumentList.Add('-PrepareOnly') }
    }
    if ($Probe -notin @('','SelfTest','InvalidParent')) { throw 'Unknown native DLL build probe.' }
    $psi.Environment.Clear()
    foreach ($name in @('SystemRoot','WINDIR','SystemDrive','ComSpec','PATHEXT','OS','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS','ProgramFiles','ProgramFiles(x86)','ProgramW6432')) {
        $value=[Environment]::GetEnvironmentVariable($name,'Process')
        if ($value) { $psi.Environment[$name]=$value }
    }
    $gitDirectory = Split-Path (Get-Command git.exe -ErrorAction Stop).Source
    $psi.Environment['PATH'] = (@($gitDirectory,$PSHOME,(Join-Path $env:SystemRoot 'System32'),$env:SystemRoot) -join ';')
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) { $psi.Environment[$name]=Join-Path $Root $name.ToLowerInvariant() }
    $psi.Environment['CARGO_INCREMENTAL']='0'
    $psi.Environment['CARGO_PROFILE_DEV_DEBUG']='line-tables-only'
    $psi.Environment['WHISPER_NATIVE_BUILD_GUARD_PID'] = if ($Probe -eq 'InvalidParent') { '1' } else { [string]$PID }
    $psi.Environment['WHISPER_NATIVE_BUILD_VARIANT']=[string]$Entry.variant
    $psi.Environment['WHISPER_NATIVE_BUILD_ID']=$BuildId
    $psi.Environment['WHISPER_NATIVE_BUILD_SCRIPT_SHA256']=Get-Sha256 $childScript
    return $psi
}

function Complete-NativeBuildCleanup([scriptblock[]]$Actions, [object]$Record, [string]$Root) {
    $errors=[Collections.Generic.List[string]]::new()
    foreach ($action in $Actions) {
        try { & $action } catch { $errors.Add($_.Exception.Message) }
    }
    if ($errors.Count -gt 0) {
        $Record.status='FAIL'
        $Record.cleanupFailures=$errors.ToArray()
    }
    $Record.finishedUtc=[DateTimeOffset]::UtcNow.ToString('o')
    $Record | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $Root 'guard-result.json') -Encoding utf8
    # Do not replace an exception already propagating through the caller's finally.
    if ($errors.Count -gt 0 -and -not $Record.Contains('failure')) { throw "NativeBuild cleanup failed: $($errors -join '; ')" }
}

function Get-NativeProtectedRoots {
    @(
        (Join-Path $script:Repo '.git'),
        (Join-Path $env:LOCALAPPDATA 'Whisper'),
        'C:\wb\preserved\poll-preserved',
        'C:\wb\preserved\pile-preserved'
    )
}

function Assert-NativeProtectedRootsExist {
    $roots = [string[]](Get-NativeProtectedRoots)
    foreach ($path in $roots) {
        if (-not (Test-Path -LiteralPath $path)) { throw "Protected native root is missing: $path" }
        Assert-NoReparseChain $path 'protected native root'
    }
    return $roots
}

function Invoke-NativeBuildProcess([object]$Entry, [string]$BuildId, [string]$Root, [string]$CacheRoot, [string]$Probe = '') {
    $protected = Assert-NativeProtectedRootsExist
    $psi=New-NativeBuildProcessInfo $Entry $BuildId $Root $Probe
    $process=[Diagnostics.Process]::new(); $process.StartInfo=$psi
    $queue=[Collections.Concurrent.ConcurrentQueue[object]]::new()
    $outLines=[Collections.Generic.List[string]]::new(); $errLines=[Collections.Generic.List[string]]::new()
    $stdoutEof=$false; $stderrEof=$false; $watch=$null; $capture=$null; $started=$false
    $outWriter=$null; $errWriter=$null
    $before=Get-GitState $script:Repo
    $userTemp=Join-Path $env:LOCALAPPDATA 'Temp'
    $allowed=@($script:Lot,$CacheRoot)
    $workspace=$null
    if (-not $Probe -and -not $NativePrepareOnly) {
        $workspace=Initialize-NativeWorkspace $Entry.variant $BuildId $Root
        $allowed+=@($workspace)
    }
    $record=[ordered]@{profile=$Entry.id;variant=$Entry.variant;buildId=$BuildId;probe=$Probe;status='STARTED';startedUtc=[DateTimeOffset]::UtcNow.ToString('o');stdout=(Join-Path $Root 'stdout.log');stderr=(Join-Path $Root 'stderr.log');allowedRoots=$allowed;sourceScriptSha256=(Get-Sha256 (Join-Path $script:Repo $Entry.script))}
    try {
        $specs=[Collections.Generic.List[object]]::new()
        foreach ($spec in @(Get-RunWatchSpecs $script:Lot)) { $specs.Add($spec) }
        if ($workspace) {
            Register-Watch $specs $workspace $true
            Register-Watch $specs (Split-Path -Parent $workspace) $false
        }
        $watch=Start-NativeBuildWatchState $specs $Root
        $watch | Add-Member -NotePropertyName nativeControl -NotePropertyValue ([Whisper.NativeBuild.Control]::new([string[]]$allowed,[string[]]$protected,$userTemp,$Root))
        $capture=Register-ProcessStreamHandlers $process $queue
        $outWriter=[IO.StreamWriter]::new($record.stdout,$false,$script:Utf8NoBom)
        $errWriter=[IO.StreamWriter]::new($record.stderr,$false,$script:Utf8NoBom)
        if (-not $process.Start()) { throw 'Could not start pinned native build script.' }
        $started=$true; $record.processId=$process.Id
        $process.BeginOutputReadLine(); $process.BeginErrorReadLine()
        $deadline=[DateTimeOffset]::UtcNow.AddSeconds([int]$Entry.timeoutSeconds)
        while (-not $process.HasExited) {
            Drain-WatcherEvents $watch $allowed $protected $userTemp $process 'NativeBuild' $Root
            Receive-ProcessOutput $queue $outWriter $errWriter $outLines $errLines ([ref]$stdoutEof) ([ref]$stderrEof)
            if ([DateTimeOffset]::UtcNow -ge $deadline) { throw 'Native DLL build exceeded its one-hour command budget.' }
            Start-Sleep -Milliseconds 100
        }
        $process.WaitForExit(); $record.exitCode=$process.ExitCode
        $streamDeadline=[DateTimeOffset]::UtcNow.AddSeconds(10)
        while (-not ($stdoutEof -and $stderrEof)) {
            Drain-WatcherEvents $watch $allowed $protected $userTemp $process 'NativeBuild' $Root
            Receive-ProcessOutput $queue $outWriter $errWriter $outLines $errLines ([ref]$stdoutEof) ([ref]$stderrEof)
            if ([DateTimeOffset]::UtcNow -ge $streamDeadline) { throw 'Native DLL build output did not close within ten seconds.' }
            Start-Sleep -Milliseconds 25
        }
        Stop-WatchState $watch
        Drain-WatcherEvents $watch $allowed $protected $userTemp $process 'NativeBuild' $Root
        $watch.nativeControl.Dispose()
        $watch=$null
        $after=Get-GitState $script:Repo
        if ($before.head -cne $after.head -or $before.status -cne $after.status) { throw 'Candidate HEAD or tracked/untracked status changed during native DLL build.' }
        $expectedExit=if ($Probe -eq 'InvalidParent') { 1 } else { 0 }
        if ($record.exitCode -ne $expectedExit) { throw "Native DLL build failed (exit $($record.exitCode)); see $Root." }
        if ($Probe -eq 'InvalidParent' -and ($errLines -join "`n") -notmatch 'actual parent process') { throw 'Invalid-parent negative control failed for an unrelated reason.' }
        $record.status='PASS'
        return [pscustomobject]$record
    } catch {
        $record.status='FAIL'; $record.failure=$_.Exception.Message
        if ($started -and -not $process.HasExited) { Stop-CargoProcessTree $process }
        throw
    } finally {
        Complete-NativeBuildCleanup @(
            { if ($null -ne $watch) { Stop-WatchState $watch } },
            { if ($null -ne $watch) { Drain-WatcherEvents $watch $allowed $protected $userTemp $process 'NativeBuild' $Root } },
            { if ($null -ne $watch -and $watch.PSObject.Properties['nativeControl']) { $watch.nativeControl.Dispose() } },
            { Remove-ProcessStreamHandlers $capture },
            { if ($null -ne $capture) { Assert-NoProcessStreamHandlers $capture } },
            { if ($null -ne $outWriter) { $outWriter.Dispose() } },
            { if ($null -ne $errWriter) { $errWriter.Dispose() } },
            { $process.Dispose() }
        ) $record $Root
    }
}

function Invoke-NativeBuildEntry {
    $entry=Get-NativeBuildProfile $Profile
    $null=Assert-NativeProtectedRootsExist
    $expectedLot=Get-FullPath (Join-Path $script:Repo $entry.outputs)
    $script:Lot=Get-FullPath (Join-Path $script:Repo $LotRoot)
    if ($script:Lot -ine $expectedLot) { throw 'Native DLL build requires exact target/native-build root.' }
    if ($ValuesJson -or $Mode -notin @('SelfTest','Preflight','Run')) { throw 'Native DLL build accepts only bounded modes, without ValuesJson.' }
    $cache=Get-FullPath (Join-Path $script:Repo $entry.cache)
    foreach ($path in @($cache,$script:Lot)) { Assert-NoReparseChain $path 'native DLL build root' }
    Assert-FreeSpace
    if ($Mode -eq 'Preflight' -or $DryRun) {
        if (-not $NativeBuildId) { throw 'Native DLL build identifier is required.' }
        $null=New-NativeBuildProcessInfo $entry $NativeBuildId (Join-Path $script:Lot 'guard/preflight')
        "NATIVE_BUILD_PREFLIGHT profile=$Profile cache=$cache outputs=$script:Lot childStarted=false"
        return
    }
    New-Item -ItemType Directory -Path $script:Lot,$cache -Force | Out-Null
    if ($Mode -eq 'SelfTest') {
        $root=Join-Path $script:Lot ('guard/selftest-'+[guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        foreach ($change in @('variant','script','cache','outputs','timeoutSeconds','workspace')) {
            $bad=$entry | ConvertTo-Json | ConvertFrom-Json
            if ($change -eq 'timeoutSeconds') { $bad.$change=3601 } else { $bad.$change='unapproved' }
            Test-Rejects "native DLL profile $change" { Assert-NativeBuildProfile $bad }
        }
        Test-Rejects 'native DLL id traversal' { New-NativeBuildProcessInfo $entry '../outside' $root }
        Test-Rejects 'native DLL cwd outside lot' { New-NativeBuildProcessInfo $entry 'valid' $script:Repo }
        $cwdWitness=New-NativeBuildProcessInfo $entry 'valid' $root
        if ($cwdWitness.WorkingDirectory -ine (Get-FullPath $root)) { throw 'Native DLL working directory witness differs from its bounded execution root.' }
        $event=[pscustomobject]@{type='Changed';path=(Join-Path $root 'allowed');old_path='';at=[DateTimeOffset]::UtcNow.ToString('o');detail=''}
        if ((Get-EventDisposition $event @($root) @() (Join-Path $root 'user-temp')) -cne 'ALLOWED') { throw 'Allowed native build event refused.' }
        $event.type='WatcherError'
        if ((Get-EventDisposition $event @($root) @() (Join-Path $root 'user-temp')) -cne 'VIOLATION') { throw 'Native build watcher error accepted.' }
        foreach ($kind in @('outside','protected','boundary','watcher-error')) {
            $state=[pscustomobject]@{queue=[Collections.Concurrent.ConcurrentQueue[object]]::new();subscriptions=@()}
            $path=if ($kind -eq 'outside') { Join-Path $script:Repo 'native/outside-negative' } else { Join-Path $root 'violation-negative' }
            $type=if ($kind -eq 'watcher-error') { 'WatcherError' } elseif ($kind -eq 'boundary') { 'Renamed' } else { 'Changed' }
            $old=if ($kind -eq 'boundary') { Join-Path (Join-Path $root 'user-temp') 'old.txt' } else { '' }
            $protected=if ($kind -eq 'protected') { @($path) } else { @() }
            $state.queue.Enqueue([pscustomobject]@{type=$type;path=$path;old_path=$old;at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic negative control; no external write'})
            Test-Rejects "native DLL watcher $kind" { Drain-WatcherEvents $state @($root) $protected (Join-Path $root 'user-temp') $null 'SelfTest' $root }
        }
        $tempEvent=[pscustomobject]@{type='Changed';path=(Join-Path $root 'user-temp/unknown');old_path=''}
        if ((Get-EventDisposition $tempEvent @($root) @() (Join-Path $root 'user-temp')) -cne 'UNATTRIBUTED_TEMP') { throw 'Unknown real Temp policy changed.' }
        $renameConstructor=[IO.RenamedEventArgs].GetConstructor([type[]]@([IO.WatcherChangeTypes],[string],[string],[string]))
        foreach ($case in @(
            @{label='complete';name='allowed\new.txt';old='allowed\old.tmp';disposition='ALLOWED';nameNull=$false;nameEmpty=$false;oldNull=$false},
            @{label='missing-new';name=$null;old='allowed\old.tmp';disposition='VIOLATION';nameNull=$true;nameEmpty=$false;oldNull=$false},
            @{label='empty-new';name='';old='allowed\old.tmp';disposition='VIOLATION';nameNull=$false;nameEmpty=$true;oldNull=$false},
            @{label='missing-old';name='allowed\new.txt';old=$null;disposition='VIOLATION';nameNull=$false;nameEmpty=$false;oldNull=$true}
        )) {
            $constructorArgs=[object[]]@([IO.WatcherChangeTypes]::Renamed,[string]$root,$null,$null)
            if ($null -ne $case.name) { $constructorArgs[2]=[string]$case.name }
            if ($null -ne $case.old) { $constructorArgs[3]=[string]$case.old }
            $renameArgs=$renameConstructor.Invoke($constructorArgs)
            $raw=Get-WatcherObservation $renameArgs $root
            if ($raw.name_is_null -ne $case.nameNull -or $raw.name_is_empty -ne $case.nameEmpty -or $raw.old_name_is_null -ne $case.oldNull) { throw "Raw rename observation failed: $($case.label)" }
            $state=[pscustomobject]@{queue=[Collections.Concurrent.ConcurrentQueue[object]]::new();subscriptions=@()}
            $state.queue.Enqueue([pscustomobject]@{type='Renamed';path=$renameArgs.FullPath;old_path=$renameArgs.OldFullPath;at=[DateTimeOffset]::UtcNow.ToString('o');detail='synthetic raw-name control';raw_event=$raw})
            $allowed=@((Join-Path $root 'allowed'))
            if ($case.disposition -eq 'VIOLATION') {
                Test-Rejects "raw rename remains blocking: $($case.label)" { Drain-WatcherEvents $state $allowed @() (Join-Path $root 'user-temp') $null 'NativeBuild' $root }
            } else { Drain-WatcherEvents $state $allowed @() (Join-Path $root 'user-temp') $null 'NativeBuild' $root }
            $receipt=Get-Content -LiteralPath (Join-Path $root 'watcher-events.jsonl') -Tail 1 | ConvertFrom-Json
            if ($receipt.disposition -cne $case.disposition -or $receipt.raw_event.watcher_root -ine $root -or $receipt.raw_event.name_is_null -ne $case.nameNull -or $receipt.raw_event.old_name_is_null -ne $case.oldNull) { throw "Serialized raw rename receipt failed: $($case.label)" }
        }
        . (Join-Path $PSScriptRoot 'test-native-directory-watch.ps1')
        Invoke-NativeDirectoryWatchTests $root (Get-RunWatchSpecs $script:Lot)
        $specs=[Collections.Generic.List[object]]::new(); Register-Watch $specs $root $true
        $live=Start-WatchState $specs
        try {
            $marker=Join-Path $root 'live-watcher-marker.txt'; 'native watcher witness' | Set-Content -LiteralPath $marker
            $observed=$false; $limit=[DateTimeOffset]::UtcNow.AddSeconds(3)
            while (-not $observed -and [DateTimeOffset]::UtcNow -lt $limit) {
                Receive-WatcherEvents $live
                $observed=$live.queue.ToArray().Where({$_.path -ieq $marker}).Count -gt 0
                Start-Sleep -Milliseconds 20
            }
            if (-not $observed) { throw 'Real native build watcher missed its in-root witness.' }
            $realEvent=@($live.queue.ToArray() | Where-Object path -IEQ $marker)[0]
            if ($realEvent.raw_event.watcher_root -ine $root -or $realEvent.raw_event.raw_name -cne 'live-watcher-marker.txt' -or $realEvent.raw_event.name_is_null -or $realEvent.raw_event.name_is_empty) { throw 'Real watcher raw event metadata differs from its witnessed file.' }
        } finally { Stop-WatchState $live }
        foreach ($probe in @('SelfTest','InvalidParent')) {
            $probeRoot=Join-Path $root $probe
            foreach ($child in @('temp','tmp','localappdata','appdata','userprofile','home')) { New-Item -ItemType Directory -Path (Join-Path $probeRoot $child) -Force | Out-Null }
            $null=Invoke-NativeBuildProcess $entry 'guard-negative' $probeRoot $cache $probe
        }
        "PASS native DLL guard self-tests: profile/path/parent rejection, complete/null/empty raw renames with unchanged stops, live existing watcher metadata, script/manifest controls; no bootstrap/CMake. evidence=$root"
        return
    }
    if (-not $NativeBuildId) { throw 'Native DLL build identifier is required.' }
    $root=Join-Path $script:Lot ("guard/$($entry.id)-$NativeBuildId")
    Assert-NoReparseChain $root 'native DLL invocation'
    if (Test-Path -LiteralPath $root) { throw "Native DLL invocation already exists: $root. Nothing removed." }
    foreach ($child in @('temp','tmp','localappdata','appdata','userprofile','home')) { New-Item -ItemType Directory -Path (Join-Path $root $child) -Force | Out-Null }
    $result=Invoke-NativeBuildProcess $entry $NativeBuildId $root $cache
    "NATIVE_BUILD status=$($result.status) profile=$Profile logs=$root"
}

try {
    if ($Profile -cin @('NativeDllCpu','NativeDllGpu')) {
        Invoke-NativeBuildEntry
        exit 0
    }
    if ($NativeBuildId -or $NativePrepareOnly) { throw 'Native DLL arguments require an exact native DLL build profile.' }
    $registry = Get-ProfileRegistry
    $script:Lot = Resolve-LotRoot $LotRoot
    Assert-FreeSpace

    if ($Mode -eq 'SelfTest') {
        if ($script:Lot -ine (Get-FullPath (Join-Path $script:Repo 'target\n2-native'))) { throw 'Guard self-tests must use the exact candidate-local N2 root.' }
        Initialize-LotDirectories $script:Lot
        Invoke-SelfTests $registry $script:Lot
        exit 0
    }
    $profile = Get-Profile $registry $Profile
    Assert-LotRootForProfile $profile.id $script:Lot
    if ($Mode -eq 'List' -and $profile.operation -cne 'List') { throw "Profile $Profile does not define a List operation." }
    if ($Mode -eq 'Run' -and $profile.operation -eq 'List') { throw 'Use -Mode List for the InventoryList profile.' }
    if ($Mode -eq 'Preflight' -and $profile.operation -eq 'Run' -and @($profile.cases).Count -eq 0) { throw "Profile $Profile has no exact case selectors." }
    if ($Mode -eq 'Preflight' -and $profile.operation -ne 'Run' -and @($profile.cases).Count -ne 0) { throw 'Preflight profile operation mismatch.' }

    if ($Mode -eq 'Preflight' -or $Mode -eq 'List' -or $Mode -eq 'Run') {
        if ($Mode -ne 'Preflight') { Initialize-LotDirectories $script:Lot }
        $valuesDoc = Get-ValuesDocument $ValuesJson $profile
        if ($Mode -eq 'Preflight') {
            $runRoot = Join-Path (Join-Path $script:Lot 'runs') ('preflight-' + [guid]::NewGuid().ToString('N'))
            $environment = Get-ProfileEnvironment $profile $valuesDoc $runRoot $script:Lot $false
            Assert-CampaignRoots $script:Lot $runRoot $environment
            if ($profile.operation -ceq 'Diagnostic') {
                Assert-NvidiaSmiDiagnosticProfile $profile -VerifyInstalledExecutable
                if ($profile.id -ceq 'NvidiaSmiEnvironmentCompareDiagnostic') {
                    $hostEnvironment = Get-NvidiaDiagnosticHostEnvironment
                    foreach ($contextId in @('clean','host-nvidia-filtered','path-only','vars-only')) {
                        $contextRoot = Join-Path $runRoot (Join-Path 'contexts' $contextId)
                        $null = New-NvidiaDiagnosticContextEnvironment $contextId $environment $hostEnvironment $contextRoot
                    }
                }
                $nvidiaExe = Get-FullPath (Join-Path $env:SystemRoot 'System32\nvidia-smi.exe')
                $nvidiaVersion = [Diagnostics.FileVersionInfo]::GetVersionInfo($nvidiaExe).FileVersion
                $nvidiaSha = Get-Sha256 $nvidiaExe
                if ($profile.id -ceq 'NvidiaProgramW6432Witness') {
                    if (Test-Path -LiteralPath $runRoot) { throw 'ProgramW6432 witness preflight RunRoot unexpectedly exists.' }
                    $previousRunRoot = $script:RunRoot
                    try {
                        $script:RunRoot = $runRoot
                        New-Item -ItemType Directory -Path $runRoot,(Join-Path $runRoot 'temp'),(Join-Path $runRoot 'tmp'),(Join-Path $runRoot 'localappdata'),(Join-Path $runRoot 'appdata') -Force -ErrorAction Stop | Out-Null
                        $witnessToolchain = Get-CargoExecutable
                        $witnessVsEnvironment = Get-VsEnvironment (Get-VsDevCommand) $environment.TEMP $environment.TMP $runRoot
                        $witnessPair = New-NvidiaProgramW6432ProcessPair $profile $environment $witnessVsEnvironment $witnessToolchain $runRoot
                    } finally {
                        $script:RunRoot = $previousRunRoot
                        if (Test-InRoot $runRoot $script:Lot -and (Test-Path -LiteralPath $runRoot)) { Remove-Item -LiteralPath $runRoot -Recurse -Force -ErrorAction SilentlyContinue }
                    }
                } elseif ($profile.id -ceq 'NvidiaSmiInventoryDiagnostic') {
                    $inventoryPsi = New-NvidiaSmiInventoryProcessInfo $profile $environment $runRoot
                    if (-not $inventoryPsi.EnvironmentVariables.ContainsKey('ProgramW6432') -or [string]$inventoryPsi.EnvironmentVariables['ProgramW6432'] -ine (Get-ValidatedProgramW6432Directory)) { throw 'NVIDIA inventory preflight PSI did not contain the validated ProgramW6432 environment value.' }
                } else {
                    $null = New-NvidiaDiagnosticProcessInfo $profile.id $nvidiaExe ([string[]]$profile.command.arguments) $environment $runRoot
                }
                "N2_PREFLIGHT profile=$($profile.id) operation=$($profile.operation) candidate=$script:Repo executable=$nvidiaExe version=$nvidiaVersion sha256=$nvidiaSha contexts=$($profile.command.contexts -join ',') programW6432=$(Get-ValidatedProgramW6432Directory) sharedTarget=$script:CargoTarget lot=$script:Lot freeBytes=$((Get-PSDrive -Name C).Free) cargoStarted=false"
                exit 0
            }
            $toolchain = Get-CargoExecutable
            $vsDev = Get-VsDevCommand
            "N2_PREFLIGHT profile=$($profile.id) operation=$($profile.operation) candidate=$script:Repo cargo=$($toolchain.cargo) rustc=$($toolchain.rustc) vsDevCmd=$vsDev sharedTarget=$script:CargoTarget lot=$script:Lot freeBytes=$((Get-PSDrive -Name C).Free)"
            exit 0
        }
        if ($DryRun) { Initialize-LotDirectories $script:Lot }
        $result = Get-ProfileResult $profile $valuesDoc $script:Lot -DryRunOnly:$DryRun
        if (-not $DryRun) {
            $resultPath = Join-Path $script:LogRoot 'result.json'
            $result | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $resultPath -Encoding UTF8
            "N2_CAMPAIGN profile=$($profile.id) status=$($result.status) cases=$($result.cases.Count) logs=$script:LogRoot result=$resultPath"
        } else { "N2_CAMPAIGN profile=$($profile.id) status=DRY_RUN" }
        if ($result.status -in @('PASS','NO_RUN_PASS','LIST_PASS','DRY_RUN','DIAGNOSTIC_ONLY')) { exit 0 }
        exit 1
    }
    throw "Unsupported N2 mode: $Mode"
} catch {
    if (-not [string]::IsNullOrWhiteSpace([string]$script:LogRoot) -and (Test-Path -LiteralPath $script:LogRoot -PathType Container)) {
        $failureResult = [ordered]@{ profile=$Profile; status='FAIL'; message=$_.Exception.Message; runRoot=$script:RunRoot; logs=$script:LogRoot; failedAtUtc=[DateTimeOffset]::UtcNow.ToString('o') }
        $failureResult | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $script:LogRoot 'result.json') -Encoding UTF8
        [Console]::Error.WriteLine("N2_RESULT logs=$script:LogRoot result=$(Join-Path $script:LogRoot 'result.json')")
    }
    [Console]::Error.WriteLine("N2_GUARD_FAIL mode=$Mode profile=$Profile message=$($_.Exception.Message)")
    exit 1
}
