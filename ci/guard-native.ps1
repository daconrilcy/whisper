[CmdletBinding(PositionalBinding = $false)]
param(
    [ValidateSet('SelfTest','Preflight','List','Run')]
    [string]$Mode = 'Preflight',
    [string]$Profile,
    [string]$ValuesJson,
    [string]$LotRoot = 'target/n2-native',
    [switch]$DryRun
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
        if ($feature -notmatch '^(?:native-tests|whisper-[a-z0-9-]+/(?:native-tests|recette-hooks))$' -or -not $featureSet.Add([string]$feature)) {
            throw "Invalid or duplicate Cargo feature in profile registry: $feature"
        }
    }
    $profileSet = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($profile in @($registry.profiles)) {
        if ($profile.id -notmatch '^[A-Za-z][A-Za-z0-9]{0,47}$' -or -not $profileSet.Add([string]$profile.id)) { throw "Invalid or duplicate profile id: $($profile.id)" }
        if ($profile.operation -notin @('NoRun','List','Run')) { throw "Unsupported operation for profile $($profile.id)." }
        if ($profile.status -notin @('READY','DISCOVERY_PENDING')) { throw "Unsupported status for profile $($profile.id)." }
        if ($profile.package -cne 'workspace' -and -not $packageSet.Contains([string]$profile.package)) { throw "Unapproved package for profile $($profile.id): $($profile.package)" }
        foreach ($feature in @($profile.features)) { if (-not $featureSet.Contains([string]$feature)) { throw "Unapproved feature '$feature' in profile $($profile.id)." } }
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

function Resolve-LotRoot([string]$Path) {
    $full = if ([IO.Path]::IsPathRooted($Path)) { Get-FullPath $Path } else { Get-FullPath (Join-Path $script:Repo $Path) }
    if (-not (Test-InRoot $full $script:Repo)) { throw "Lot root must stay inside this candidate worktree: $full" }
    $expected = Get-FullPath (Join-Path $script:Repo 'target\n2-native')
    if ($full -ine $expected) { throw "N2 lot root must be the exact candidate-local root: $expected" }
    Assert-NoReparseChain $full 'N2 lot root'
    return $full
}

function Assert-FreeSpace {
    $drive = [IO.Path]::GetPathRoot($script:Repo).TrimEnd('\').TrimEnd(':')
    $psDrive = Get-PSDrive -Name $drive -ErrorAction Stop
    if ($psDrive.Free -lt 30GB) { throw "$drive`: has less than 30 GiB free; native campaign is forbidden." }
}

function Initialize-LotDirectories([string]$Lot) {
    foreach ($path in @($Lot,(Join-Path $Lot 'runs'),(Join-Path $Lot 'logs'),(Join-Path $Lot 'cargo-home'),(Join-Path $Lot 'temp'),(Join-Path $Lot 'tmp'))) {
        Assert-NoReparseChain $path 'N2 write root'
        New-Item -ItemType Directory -Path $path -Force | Out-Null
        Assert-NoReparseChain $path 'N2 write root'
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

function Assert-CampaignRoots([string]$Lot, [string]$RunRoot, [hashtable]$Environment) {
    if (-not (Test-InRoot $Lot $script:Repo)) { throw 'Lot root escaped the candidate worktree.' }
    if (-not (Test-InRoot $RunRoot $Lot)) { throw 'Run root escaped the N2 lot.' }
    Assert-NoReparseChain $script:Repo 'candidate worktree'
    Assert-NoReparseChain $Lot 'N2 lot root'
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

function Get-RunWatchSpecs([string]$Lot) {
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
    $unique = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    return @($specs | Where-Object { $unique.Add("$($_.path)|$($_.recursive)") })
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

function Receive-WatcherEvents([object]$State) {
    if ($null -eq $State) { return }
    $subscribers = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($id in $State.subscriptions) { [void]$subscribers.Add([string]$id) }
    foreach ($eventRecord in @(Get-Event -ErrorAction SilentlyContinue | Where-Object { $subscribers.Contains([string]$_.SourceIdentifier) })) {
        $args = $eventRecord.SourceEventArgs
        if ($args -is [IO.ErrorEventArgs]) {
            $State.queue.Enqueue([pscustomobject]@{ path=''; old_path=''; type='WatcherError'; at=[DateTimeOffset]::UtcNow.ToString('o'); detail=$args.GetException().Message })
        } else {
            $type = ($eventRecord.SourceIdentifier -split '-',3)[1]
            $oldPath = if ($args -is [IO.RenamedEventArgs]) { $args.OldFullPath } else { '' }
            $State.queue.Enqueue([pscustomobject]@{ path=$args.FullPath; old_path=$oldPath; type=$type; at=[DateTimeOffset]::UtcNow.ToString('o'); detail='' })
        }
        Remove-Event -EventIdentifier $eventRecord.EventIdentifier -ErrorAction SilentlyContinue
    }
}

function Stop-WatchState([object]$State) {
    if ($null -eq $State) { return }
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

function Drain-WatcherEvents([object]$State, [string[]]$AllowedRoots, [string[]]$ProtectedRoots, [string]$UserTemp, [Diagnostics.Process]$Process, [string]$Mode, [string]$LogRoot) {
    Receive-WatcherEvents $State
    $event = $null
    while ($State.queue.TryDequeue([ref]$event)) {
        $disposition = Get-EventDisposition $event $AllowedRoots $ProtectedRoots $UserTemp
        if ($disposition -eq 'UNATTRIBUTED_TEMP') {
            Write-JsonLine (Join-Path $LogRoot 'unattributed-temp.jsonl') ([ordered]@{ mode=$Mode; path=$event.path; old_path=$event.old_path; event=$event.type; event_at=$event.at; detected_at=[DateTimeOffset]::UtcNow.ToString('o'); attribution='UNATTRIBUTED'; detail=$event.detail })
            $event = $null
            continue
        }
        if ($disposition -eq 'VIOLATION') {
            $record = [ordered]@{ mode=$Mode; status='FAIL'; path=$event.path; old_path=$event.old_path; event=$event.type; event_at=$event.at; detected_at=[DateTimeOffset]::UtcNow.ToString('o'); action='campaign-process-tree-terminated'; detail=$event.detail }
            Write-JsonLine (Join-Path $LogRoot 'guard-events.jsonl') $record
            Stop-CargoProcessTree $Process
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

function New-CargoProcessInfo([string]$CargoExe, [string]$RustcExe, [string]$ToolchainBin, [string[]]$CargoArgs, [hashtable]$Environment, [hashtable]$VsEnvironment, [string]$RunRoot) {
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
    foreach ($entry in $VsEnvironment.GetEnumerator()) { if ($entry.Key -cne 'PATH') { $vars[[string]$entry.Key] = [string]$entry.Value } }
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
    $initialInvocation | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $invocationPath -Encoding UTF8

    $cargoHome = [string]$Environment.CARGO_HOME
    Initialize-CargoCache $script:Lot
    $vsEnvironment = Get-VsEnvironment (Get-VsDevCommand) $Environment.TEMP $Environment.TMP $RunRoot
    $psi = New-CargoProcessInfo $Toolchain.cargo $Toolchain.rustc $Toolchain.toolchainBin $stableArgs $Environment $vsEnvironment $RunRoot
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
    $beforeGit = Get-GitState $script:Repo
    $started = [DateTimeOffset]::UtcNow
    $exitCode = $null
    $testEvidence = $null
    try {
        $watchSpecs = Get-RunWatchSpecs $script:Lot
        $watchState = Start-WatchState $watchSpecs
        $streamCapture = Register-ProcessStreamHandlers $process $stdoutQueue
        $stdoutWriter = [IO.StreamWriter]::new($stdoutPath,$false,$script:Utf8NoBom)
        $stderrWriter = [IO.StreamWriter]::new($stderrPath,$false,$script:Utf8NoBom)
        if (-not $process.Start()) { throw 'Cargo process failed to start.' }
        $processStarted = $true
        $process.BeginOutputReadLine(); $process.BeginErrorReadLine()
        [ordered]@{ processId=$process.Id; startedUtc=$started.ToString('o'); case=$caseId } | ConvertTo-Json -Compress | Set-Content -LiteralPath (Join-Path $LogRoot ($safeId + '.process.json')) -Encoding UTF8
        while (-not $process.HasExited) {
            Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot
            Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $null ([ref]$stdoutEof) ([ref]$stderrEof)
            Start-Sleep -Milliseconds 50
        }
        $process.WaitForExit()
        $exitCode = $process.ExitCode
        $streamDeadline = [DateTimeOffset]::UtcNow.AddSeconds(10)
        while (-not ($stdoutEof -and $stderrEof)) {
            Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot
            Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $null ([ref]$stdoutEof) ([ref]$stderrEof)
            if ([DateTimeOffset]::UtcNow -ge $streamDeadline) { throw 'Cargo stdout/stderr capture did not reach EOF before its 10 second deadline.' }
            Start-Sleep -Milliseconds 20
        }
        Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot
        Receive-ProcessOutput $stdoutQueue $stdoutWriter $stderrWriter $stdoutLines $null ([ref]$stdoutEof) ([ref]$stderrEof)
        $stdoutWriter.Flush(); $stderrWriter.Flush()
        if ($exitCode -ne 0) { throw "Cargo returned exit code $exitCode for $caseId; logs: $stdoutPath, $stderrPath" }
        if ($Profile.operation -eq 'Run') { $testEvidence = Get-TestBodyEvidence $Case $exitCode $stdoutLines.ToArray() }
        $afterGit = Get-GitState $script:Repo
        if ($afterGit.head -cne $beforeGit.head -or $afterGit.status -cne $beforeGit.status) { throw 'Candidate Git HEAD/status changed during native Cargo command; campaign stopped.' }
        Stop-WatchState $watchState
        Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot
        $watchState = $null
        $verdict = Get-CompletedCommandVerdict $Profile $testEvidence
        $elapsedSeconds = [Math]::Round(([DateTimeOffset]::UtcNow - $started).TotalSeconds,3)
        Update-InvocationRecord $invocationPath $verdict $exitCode $elapsedSeconds $testEvidence $null | Out-Null
        return [pscustomobject]@{ case=$caseId; exitCode=$exitCode; status=$verdict; elapsedSeconds=$elapsedSeconds; stdout=$stdoutPath; stderr=$stderrPath }
    } catch {
        $failure = $_.Exception.Message
        if ($processStarted -and $null -ne $process -and -not $process.HasExited) { Stop-CargoProcessTree $process }
        try {
            $elapsedSeconds = [Math]::Round(([DateTimeOffset]::UtcNow - $started).TotalSeconds,3)
            Update-InvocationRecord $invocationPath 'FAIL' $exitCode $elapsedSeconds $testEvidence $failure | Out-Null
        } catch { throw "Campaign failed ($failure); invocation receipt update also failed: $($_.Exception.Message)" }
        throw
    } finally {
        if ($null -ne $watchState) {
            Stop-WatchState $watchState
            Drain-WatcherEvents $watchState $allowedRoots $protectedRoots $userTemp $process $Mode $LogRoot
        }
        if ($null -ne $stdoutWriter) { $stdoutWriter.Dispose() }
        if ($null -ne $stderrWriter) { $stderrWriter.Dispose() }
        Remove-ProcessStreamHandlers $streamCapture
        if ($null -ne $process) { $process.Dispose() }
    }
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
    Test-Rejects 'pending profile without selectors' { Get-Profile $Registry 'TurboCpu' }
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
        $syntheticVs = @{ PATH = 'C:\ambient-tool;C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\bin' ; INCLUDE='C:\vs\include'; LIB='C:\vs\lib' }
        $syntheticPsi = New-CargoProcessInfo 'C:\toolchain\cargo.exe' 'C:\toolchain\rustc.exe' 'C:\toolchain' @('test','--offline','--locked') $syntheticEnvironment $syntheticVs $syntheticRunRoot
        $syntheticVars = $syntheticPsi.EnvironmentVariables
        if ($syntheticVars.ContainsKey('GGML_CUDA') -or $syntheticVars.ContainsKey('WHISPER_UNLISTED_SENTINEL') -or $syntheticVars.ContainsKey('RUSTFLAGS')) { throw 'Ambient build variables survived the sanitized Cargo environment.' }
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
    } finally {
        [Environment]::SetEnvironmentVariable('GGML_CUDA',$savedGgml,'Process')
        [Environment]::SetEnvironmentVariable('WHISPER_UNLISTED_SENTINEL',$savedWhisper,'Process')
        [Environment]::SetEnvironmentVariable('CUDA_HOME',$savedCudaHome,'Process')
        [Environment]::SetEnvironmentVariable('CUDA_PATH_V12_10',$savedCudaPathV1210,'Process')
    }
    $streamNormal = Invoke-ProcessStreamProbe '0'
    $streamFailure = Invoke-ProcessStreamProbe '23'
    if (-not $streamNormal.subscriptionsAndJobsRemoved -or -not $streamFailure.subscriptionsAndJobsRemoved) { throw 'Synthetic process stream cleanup did not verify empty subscription/job state.' }
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
        discoveryArgv = @('PreparationNoRun','InventoryList','WasapiInventory','WasapiCapture')
        staging = $stagingEvidence
        watcherAndRoots = @('allowed in-lot event','disallowed external event','Temp UNATTRIBUTED','cross-boundary rename rejected','watcher error rejected','live FileSystemWatcher observed in-lot marker')
    }
    $selfTestPath = Join-Path (Join-Path $Lot 'logs') 'guard-selftest-correction2.json'
    $selfTestEvidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $selfTestPath -Encoding UTF8
    "PASS native guard self-tests: R1 profile environment, R2 normal/error PowerShell child stdout/stderr + subscription/job cleanup, R3 one-line and multiline WASAPI test-body fixtures, R4 invocation receipt lifecycle, exact discovery argv, canonical roots, staging SHA/path/capacity fixtures, ambient environment removal, and watcher dispositions; evidence=$selfTestPath; no Cargo/native process started."
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
        if (-not (Test-InRoot $root $Lot)) { throw "Generated output escaped N2 lot: $root" }
        Assert-NoReparseChain $root 'generated N2 output root'
        if (Test-Path -LiteralPath $root) { throw "Generated N2 run path already exists: $root" }
    }
    if (-not $DryRunOnly) {
        New-Item -ItemType Directory -Path $script:RunRoot,$script:LogRoot,(Join-Path $script:RunRoot 'temp'),(Join-Path $script:RunRoot 'tmp'),(Join-Path $script:RunRoot 'localappdata'),(Join-Path $script:RunRoot 'appdata') -Force | Out-Null
    }
    $environment = Get-ProfileEnvironment $Profile $ValuesDoc $script:RunRoot $Lot (-not $DryRunOnly)
    Assert-CampaignRoots $Lot $script:RunRoot $environment
    $stagingEvidence = if (-not $DryRunOnly) { Copy-ProfileStaging $Profile $ValuesDoc $environment.LOCALAPPDATA $script:RunRoot } else { @() }
    $toolchain = Get-CargoExecutable
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

try {
    $registry = Get-ProfileRegistry
    $script:Lot = Resolve-LotRoot $LotRoot
    Assert-FreeSpace

    if ($Mode -eq 'SelfTest') {
        Initialize-LotDirectories $script:Lot
        Invoke-SelfTests $registry $script:Lot
        exit 0
    }
    $profile = Get-Profile $registry $Profile
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
