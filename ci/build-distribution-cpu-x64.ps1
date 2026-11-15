$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
Set-StrictMode -Version Latest

function Get-ProcessEnvironment {
    $snapshot = @{}
    foreach ($entry in Get-ChildItem Env:) {
        $snapshot[$entry.Name] = $entry.Value
    }
    return $snapshot
}

function Restore-ProcessEnvironment([hashtable]$Snapshot) {
    foreach ($entry in @(Get-ChildItem Env:)) {
        if (-not $Snapshot.ContainsKey($entry.Name)) {
            Remove-Item -LiteralPath "Env:$($entry.Name)" -ErrorAction SilentlyContinue
        }
    }
    foreach ($name in $Snapshot.Keys) {
        [Environment]::SetEnvironmentVariable($name, [string]$Snapshot[$name], 'Process')
    }
}

function Remove-ProcessEnvironmentMatching([string]$Pattern) {
    foreach ($entry in @(Get-ChildItem Env: | Where-Object Name -Match $Pattern)) {
        Remove-Item -LiteralPath "Env:$($entry.Name)" -ErrorAction SilentlyContinue
    }
}

function Resolve-Ninja {
    $command = Get-Command ninja -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        throw 'T11 preflight: Ninja is not on PATH and vswhere.exe is unavailable.'
    }
    $installations = @(& $vswhere -products '*' -property installationPath)
    if ($LASTEXITCODE -ne 0) {
        throw 'T11 preflight: vswhere.exe could not enumerate Visual Studio installations.'
    }
    foreach ($installation in $installations) {
        $candidate = Join-Path $installation 'Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja/ninja.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return $candidate
        }
    }
    throw 'T11 preflight: CMAKE_GENERATOR=Ninja requires ninja.exe; none was found in Visual Studio installations.'
}

function Assert-InRoot([string]$Path, [string]$Root, [string]$Label) {
    $resolvedPath = [IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
    $resolvedRoot = [IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
    if (-not $resolvedPath.StartsWith($resolvedRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "$Label escaped the authorized lot root: $resolvedPath"
    }
}

function Get-RustcBinaryCommand(
    [string[]]$Commands,
    [string]$Crate,
    [string]$TargetName,
    [string]$SourcePath
) {
    $targetPattern = [Regex]::Escape($TargetName)
    $sourcePattern = [Regex]::Escape($SourcePath)
    $matches = @($Commands | Where-Object {
        $_ -match "rustc\.exe .*--crate-name $([Regex]::Escape($Crate)) " -and
        $_ -match '(?i)-C target-cpu=x86-64' -and
        $_ -match "(?i)--out-dir\s+\S*targets\\$targetPattern\\release\\deps" -and
        $_ -match '(?i)--crate-type\s+bin(?:\s|$)' -and
        $_ -match "(?i)(?:^|\s)$sourcePattern(?:\s|$)"
    })
    if ($matches.Count -ne 1) {
        throw "T11 expected exactly one current-target binary rustc invocation for $Crate from $SourcePath; found $($matches.Count)."
    }
    return [string]$matches[0]
}

function Assert-RustcBinarySelectionSelfTest {
    $target = 'r2-selftest'
    $lib = 'rustc.exe --crate-name whisper_desktop --crate-type lib crates\whisper-desktop\src\lib.rs --out-dir C:\repo\targets\r2-selftest\release\deps -C target-cpu=x86-64'
    $bin = 'rustc.exe --crate-name whisper_desktop --crate-type bin crates\whisper-desktop\src\main.rs --out-dir C:\repo\targets\r2-selftest\release\deps -C target-cpu=x86-64'
    $selected = Get-RustcBinaryCommand @($lib, $bin) 'whisper_desktop' $target 'crates\whisper-desktop\src\main.rs'
    if ($selected -cne $bin) {
        throw 'T11 SelfTest selected the desktop library command instead of the binary command.'
    }
    $rejected = $false
    try {
        [void](Get-RustcBinaryCommand @($lib) 'whisper_desktop' $target 'crates\whisper-desktop\src\main.rs')
    } catch {
        $rejected = $_.Exception.Message -match 'found 0'
    }
    if (-not $rejected) {
        throw 'T11 SelfTest accepted desktop library-only rustc evidence for the desktop PE.'
    }
    $needle = [Text.Encoding]::ASCII.GetBytes(('ab' * 32))
    $positiveBytes = [byte[]]@(0,0) + $needle + [byte[]]@(0,0)
    if (-not (Test-ByteSequence $positiveBytes $needle)) {
        throw 'T11 SelfTest failed to find the exact worker digest bytes in a PE fixture.'
    }
    $negativeBytes = [byte[]]$positiveBytes.Clone()
    $negativeBytes[2] = $negativeBytes[2] -bxor 1
    if (Test-ByteSequence $negativeBytes $needle) {
        throw 'T11 SelfTest accepted a PE fixture without the exact worker digest bytes.'
    }
}

function Test-ByteSequence([byte[]]$Bytes, [byte[]]$Sequence) {
    if ($Sequence.Length -eq 0 -or $Sequence.Length -gt $Bytes.Length) { return $false }
    for ($start = 0; $start -le $Bytes.Length - $Sequence.Length; $start++) {
        $matched = $true
        for ($offset = 0; $offset -lt $Sequence.Length; $offset++) {
            if ($Bytes[$start + $offset] -ne $Sequence[$offset]) {
                $matched = $false
                break
            }
        }
        if ($matched) { return $true }
    }
    return $false
}

function Assert-DesktopCpuWorkerDigest([string]$TargetRoot, [string]$DesktopPath) {
    $digest = [string]$script:expectedCpuWorkerSha256
    if ($digest -notmatch '^[0-9a-f]{64}$') { throw 'T11 desktop digest evidence has no valid T11 worker SHA-256.' }
    $depInfo = @(Get-ChildItem -LiteralPath (Join-Path $TargetRoot 'release/deps') -Filter 'whisper_adapters-*.d' -File |
        Where-Object {
            $text = [IO.File]::ReadAllText($_.FullName)
            $text.Contains('# env-dep:WHISPER_EXPECTED_CPU_WORKER_SHA256') -and $text.Contains($digest)
        })
    if ($depInfo.Count -eq 0) {
        throw 'T11 desktop build has no current adapter rustc dep-info containing the exact worker digest and env dependency key.'
    }
    $peBytes = [IO.File]::ReadAllBytes($DesktopPath)
    $digestBytes = [Text.Encoding]::ASCII.GetBytes($digest)
    if (-not (Test-ByteSequence $peBytes $digestBytes)) {
        throw "T11 desktop PE does not embed the exact CPU worker digest ${digest}: $DesktopPath"
    }
    Write-Output "T11_DESKTOP_DIGEST_PROOF sha256=$digest dep_info=$($depInfo[0].FullName) dep_info_env_key=PASS dep_info_digest=PASS pe_literal=PASS pe=$DesktopPath"
}

function Invoke-GuardedBuild([string[]]$CargoArgs, [switch]$Fresh, [string]$DesktopCpuWorkerSha256) {
    $profileEnvironment = Get-ProcessEnvironment
    try {
        $guardArguments = @{
            Mode = 'Run'
            LotRoot = 'target/parakeet-tests/cpu-distribution-x64-20261115'
            CargoArgs = $CargoArgs
        }
        if ($Fresh) {
            $guardArguments.FreshTarget = $script:targetName
        } else {
            $guardArguments.ExistingTarget = $script:targetName
        }
        if ($DesktopCpuWorkerSha256) {
            $guardArguments.DesktopCpuWorkerSha256 = $DesktopCpuWorkerSha256
        }

        if (Test-Path Env:_CL_) { throw 'T11 build invocation must not inherit _CL_.' }

        $guardOutput = @(& $script:guardScript @guardArguments 2>&1)
        $guardText = [string]::Join("`n", [string[]]@($guardOutput | ForEach-Object ToString))
        if ($DesktopCpuWorkerSha256) {
            $digestProof = "CPU_WORKER_DIGEST_INVOCATION sha256=$DesktopCpuWorkerSha256"
            if (-not $guardText.Contains($digestProof)) {
                throw "T11 guard did not confirm the exact desktop digest invocation: $digestProof`n$guardText"
            }
            Write-Output "T11_GUARD_DIGEST_EVIDENCE $digestProof"
        }
        $summary = @($guardText -split "`r?`n" | Where-Object { $_ -match '^N1_GUARD category=.+ exit_code=\d+ ' }) | Select-Object -Last 1
        if ([string]::IsNullOrWhiteSpace($summary)) {
            throw "T11 guarded build did not return an N1 result for: $([string]::Join(' ', $CargoArgs))`n$guardText"
        }
        if ($summary -notmatch 'exit_code=(?<exit>\d+)') {
            throw "T11 could not parse the N1 exit code: $summary"
        }
        $exitCode = [int]$Matches.exit
        if ($exitCode -ne 0) {
            throw "T11 guarded build failed with exit $($exitCode): $summary`n$guardText"
        }
        if ($summary -notmatch 'stdout=(?<stdout>.+) stderr=(?<stderr>.+)$') {
            throw "T11 could not resolve the guarded command logs: $summary"
        }
        $stdoutPath = [IO.Path]::GetFullPath($Matches.stdout)
        $stderrPath = [IO.Path]::GetFullPath($Matches.stderr)
        Assert-InRoot $stdoutPath $script:lotRoot 'Cargo stdout log'
        Assert-InRoot $stderrPath $script:lotRoot 'Cargo stderr log'
        foreach ($path in @($stdoutPath, $stderrPath)) {
            if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
                throw "T11 guarded command log is missing: $path"
            }
        }
        $combined = [IO.File]::ReadAllText($stdoutPath) + "`n" + [IO.File]::ReadAllText($stderrPath)
        $packageIndex = [Array]::IndexOf($CargoArgs, '-p')
        if ($packageIndex -lt 0 -or ($packageIndex + 1) -ge $CargoArgs.Count) {
            throw "T11 could not identify the package for invocation: $($CargoArgs -join ' ')"
        }
        $package = $CargoArgs[$packageIndex + 1]
        $crate = switch ($package) {
            'whisper-worker-cpu' { 'whisper_worker_cpu' }
            'whisper-desktop' { 'whisper_desktop' }
            'whisper-bootstrap' { 'whisper_bootstrap' }
            default { throw "T11 has no crate evidence mapping for $package" }
        }
        $rustcCommands = @($combined -split "`r?`n" | Where-Object {
            $_ -match "rustc\.exe .*--crate-name $crate "
        })
        $sourcePath = switch ($package) {
            'whisper-worker-cpu' { 'crates\whisper-worker-cpu\src\main.rs' }
            'whisper-desktop' { 'crates\whisper-desktop\src\main.rs' }
            'whisper-bootstrap' { 'crates\whisper-bootstrap\src\main.rs' }
        }
        $relevantCommand = Get-RustcBinaryCommand $rustcCommands $crate $script:targetName $sourcePath
        $script:rustBuildEvidence[$package] = [pscustomobject]@{
            Crate = $crate
            Stdout = $stdoutPath
            Stderr = $stderrPath
            Command = $relevantCommand
        }
        Write-Output $summary
        Write-Output "T11_CHILD_ENV package=$package _CL_present=$(Test-Path Env:_CL_)"
        if ($DesktopCpuWorkerSha256) {
            Write-Output "T11_DESKTOP_DIGEST_INPUT package=$package sha256=$DesktopCpuWorkerSha256"
        }
        Write-Output "T11_RUST_PROFILE package=$package crate=$crate target=$script:targetName log=$stderrPath command=$relevantCommand"
        Write-Output "T11_RUST_COMMAND_LOGS stdout=$stdoutPath stderr=$stderrPath"
    } finally {
        Restore-ProcessEnvironment $profileEnvironment
    }
}

function Get-CmakeCacheValues([string]$Path) {
    $values = @{}
    foreach ($line in Get-Content -LiteralPath $Path) {
        if ($line -match '^(?<name>[^:#]+):[^=]*=(?<value>.*)$') {
            $values[$Matches.name] = $Matches.value
        }
    }
    return $values
}

function Assert-PortableCmakeOutputs([string]$TargetRoot) {
    $sysBuildRoots = @(Get-ChildItem -LiteralPath (Join-Path $TargetRoot 'release/build') -Directory -Filter 'whisper-rs-sys-*' |
        ForEach-Object { Join-Path $_.FullName 'out/build' } |
        Where-Object { Test-Path -LiteralPath (Join-Path $_ 'CMakeCache.txt') -PathType Leaf })
    if ($sysBuildRoots.Count -ne 1) {
        throw "T11 expected one whisper-rs-sys CMake output under $TargetRoot/release/build; found $($sysBuildRoots.Count)"
    }
    $sysBuildRoot = [IO.Path]::GetFullPath($sysBuildRoots[0]).TrimEnd('\')
    $compileFile = Join-Path $sysBuildRoot 'compile_commands.json'
    $cacheFile = Join-Path $sysBuildRoot 'CMakeCache.txt'
    if (-not (Test-Path -LiteralPath $compileFile -PathType Leaf)) {
        throw "T11 whisper-rs-sys output has no compile_commands.json: $sysBuildRoot"
    }
    $expectedCache = [ordered]@{
        CMAKE_GENERATOR = 'Ninja'
        CMAKE_EXPORT_COMPILE_COMMANDS = 'ON'
        CMAKE_BUILD_TYPE = 'Release'
        GGML_NATIVE = 'OFF'
        GGML_SSE42 = 'ON'
        GGML_AVX = 'OFF'
        GGML_AVX_VNNI = 'OFF'
        GGML_AVX2 = 'OFF'
        GGML_BMI2 = 'OFF'
        GGML_AVX512 = 'OFF'
        GGML_AVX512_VBMI = 'OFF'
        GGML_AVX512_VNNI = 'OFF'
        GGML_AVX512_BF16 = 'OFF'
    }
    $values = Get-CmakeCacheValues $cacheFile
    foreach ($name in $expectedCache.Keys) {
        if ($values[$name] -cne $expectedCache[$name]) {
            throw "T11 CMake cache profile mismatch: $name=$($values[$name]), expected $($expectedCache[$name]) in $cacheFile"
        }
    }
    foreach ($compilerName in @('CMAKE_C_COMPILER', 'CMAKE_CXX_COMPILER')) {
        if ([IO.Path]::GetFileName([string]$values[$compilerName]) -ine 'cl.exe' -or
            [string]$values[$compilerName] -notmatch '(?i)HostX64[/\\]x64[/\\]cl\.exe$') {
            throw "T11 $compilerName is not the x64 MSVC compiler in ${cacheFile}: $($values[$compilerName])"
        }
    }

    $commandCount = 0
    $commands = Get-Content -LiteralPath $compileFile -Raw | ConvertFrom-Json
    $whisperCommands = 0
    $cCommands = 0
    $cppCommands = 0
    $objectCount = 0
    $sourceRoot = [IO.Path]::GetFullPath((Join-Path $sysBuildRoot '..')).TrimEnd('\')
    foreach ($entry in $commands) {
        $command = if ($entry.command) { [string]$entry.command } else { [string]::Join(' ', [string[]]$entry.arguments) }
        $directory = [IO.Path]::GetFullPath([string]$entry.directory).TrimEnd('\')
        if (-not $directory.Equals($sysBuildRoot, [StringComparison]::OrdinalIgnoreCase)) {
            throw "T11 compiler command directory escaped its whisper-rs-sys output: $directory"
        }
        if ($command -match '(?i)(/arch\s*:\s*AVX\w*|-mavx\w*|-march=native)') {
            throw "T11 found a host-specific or AVX compiler option in $compileFile`: $command"
        }
        if ($command -match '(?i)cl\.exe' -and $entry.file -match '(?i)(whisper|ggml)[^\\/]*\.(c|cc|cpp)$') {
            $sourceFile = [IO.Path]::GetFullPath([string]$entry.file)
            if (-not $sourceFile.StartsWith($sourceRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
                throw "T11 C/C++ source is outside its whisper-rs-sys output: $sourceFile"
            }
            $whisperCommands++
            if ($sourceFile -match '(?i)\.c$') { $cCommands++ }
            if ($sourceFile -match '(?i)\.(cc|cpp)$') { $cppCommands++ }
        }
        if ($entry.output) {
            $objectPath = [IO.Path]::GetFullPath([string]$entry.output)
            if (-not $objectPath.StartsWith($sysBuildRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
                throw "T11 compiled object is outside its whisper-rs-sys output: $objectPath"
            }
            if (-not (Test-Path -LiteralPath $objectPath -PathType Leaf)) {
                throw "T11 fresh CMake output is missing compiled object: $objectPath"
            }
            $objectCount++
        }
        $commandCount++
    }
    if ($commandCount -eq 0 -or $whisperCommands -eq 0 -or $cCommands -eq 0 -or $cppCommands -eq 0) {
        throw "T11 compile_commands.json did not tie C and C++ whisper-rs-sys sources to cl.exe: commands=$commandCount whisper=$whisperCommands C=$cCommands CXX=$cppCommands"
    }
    Write-Output "T11_CMAKE_PROFILE cache=$cacheFile compile_commands=$compileFile compiler_commands=$commandCount whisper_commands=$whisperCommands C=$cCommands CXX=$cppCommands objects=$objectCount output=$sysBuildRoot"
}

function Assert-X64Pe([string]$Package, [string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "T13 release PE is missing: $Path"
    }
    $headers = @(& $script:dumpbin /headers $Path 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "dumpbin /headers failed for $Path with exit $LASTEXITCODE"
    }
    $headerText = [string]::Join("`n", [string[]]$headers)
    if ($headerText -notmatch '(?i)8664 machine \(x64\)') {
        throw "T13 PE is not AMD64 x64: $Path`n$headerText"
    }
    $imports = @(& $script:dumpbin /imports $Path 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "dumpbin /imports failed for $Path with exit $LASTEXITCODE"
    }
    if ($Package -ceq 'whisper-worker-cpu' -and
        ([string]::Join("`n", [string[]]$imports) -match '(?i)(cuda|cublas|cudnn|nvrtc)')) {
        throw "T13 CPU worker imports a CUDA library: $Path"
    }
    $hash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    $evidence = $script:rustBuildEvidence[$Package]
    if ($null -eq $evidence -or -not $evidence.Command.Contains($script:targetName)) {
        throw "T13 PE has no current-script rustc evidence for package ${Package}: $Path"
    }
    Write-Output "T13_PE package=$Package path=$Path sha256=$hash machine=AMD64 rustc_log=$($evidence.Stderr) rustc_crate=$($evidence.Crate) target=$script:targetName"
    Write-Output ([string]::Join("`n", [string[]]$imports))
}

$script:repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$script:lotRoot = Join-Path $script:repo 'target/parakeet-tests/cpu-distribution-x64-20261115'
$script:guardScript = Join-Path $PSScriptRoot 'guard-level1.ps1'
$script:baselineEnvironment = Get-ProcessEnvironment
$script:targetName = 'r2-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
$script:rustBuildEvidence = @{}

try {
    Assert-RustcBinarySelectionSelfTest
    if ((Get-PSDrive -Name C).Free -lt 30GB) {
        throw 'T11 preflight: C: has less than 30 GiB free; no Cargo command was launched.'
    }
    $ninjaPath = Resolve-Ninja
    $cmake = Get-Command cmake -ErrorAction Stop
    $script:dumpbin = (Get-Command dumpbin -ErrorAction Stop).Source
    $cargoGuard = (Resolve-Path -LiteralPath $script:guardScript).Path
    if (-not (Test-Path -LiteralPath $script:lotRoot -PathType Container)) {
        throw "T11 LotRoot has not been prepared: $script:lotRoot"
    }

    $ninjaDirectory = Split-Path -Parent $ninjaPath
    $env:PATH = "$ninjaDirectory$([IO.Path]::PathSeparator)$env:PATH"
    Remove-ProcessEnvironmentMatching '^(CMAKE_|GGML_)'
    foreach ($name in @('CC','CXX','CFLAGS','CXXFLAGS','CL','_CL_','CARGO_BUILD_TARGET','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER')) {
        Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
    }
    foreach ($entry in @(Get-ChildItem Env: | Where-Object Name -Match '^CARGO_TARGET_.*_RUSTFLAGS$')) {
        Remove-Item -LiteralPath "Env:$($entry.Name)" -ErrorAction SilentlyContinue
    }

    $env:RUSTFLAGS = '-C target-cpu=x86-64'
    $env:CMAKE_GENERATOR = 'Ninja'
    $env:CMAKE_EXPORT_COMPILE_COMMANDS = 'ON'
    $ggmlProfile = [ordered]@{
        GGML_NATIVE = 'OFF'
        GGML_SSE42 = 'ON'
        GGML_AVX = 'OFF'
        GGML_AVX_VNNI = 'OFF'
        GGML_AVX2 = 'OFF'
        GGML_BMI2 = 'OFF'
        GGML_AVX512 = 'OFF'
        GGML_AVX512_VBMI = 'OFF'
        GGML_AVX512_VNNI = 'OFF'
        GGML_AVX512_BF16 = 'OFF'
    }
    foreach ($name in $ggmlProfile.Keys) {
        [Environment]::SetEnvironmentVariable($name, $ggmlProfile[$name], 'Process')
    }
    $script:profileEnvironment = Get-ProcessEnvironment

    if (Test-Path Env:_CL_) { throw 'T11 profile must not inherit _CL_.' }
    Write-Output "T11_PROFILE RUSTFLAGS=$env:RUSTFLAGS CMAKE_GENERATOR=$env:CMAKE_GENERATOR CMAKE_EXPORT_COMPILE_COMMANDS=$env:CMAKE_EXPORT_COMPILE_COMMANDS CL=$($env:CL) _CL_=$($env:_CL_)"
    Write-Output "T11_TOOLS ninja=$ninjaPath version=$(& $ninjaPath --version) cmake=$($cmake.Source) dumpbin=$script:dumpbin"
    $turboModel = 'C:\dev\whisper\models\ggml-large-v3-turbo.bin'
    $expectedTurboSha256 = '1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69'
    if (Test-Path -LiteralPath $turboModel -PathType Leaf) {
        $turboSha256 = (Get-FileHash -LiteralPath $turboModel -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($turboSha256 -cne $expectedTurboSha256) { throw "T03 Turbo model SHA-256 mismatch: $turboModel $turboSha256" }
        Write-Output "T03_MODEL tuple=large-v3-turbo path=$turboModel sha256=$turboSha256 inference=NOT_RUN"
    } else {
        Write-Output "T03_MODEL tuple=large-v3-turbo path=$turboModel sha256=$expectedTurboSha256 inference=NOT_RUN model_file=NOT_PRESENT"
    }
    $targetRoot = Join-Path $script:lotRoot "targets/$script:targetName"
    Write-Output "T11_FRESH_TARGET path=$targetRoot"
    Invoke-GuardedBuild @('build','--release','-p','whisper-worker-cpu','-vv') -Fresh
    $cpuWorkerPath = Join-Path $targetRoot 'release/whisper-worker-cpu.exe'
    if (-not (Test-Path -LiteralPath $cpuWorkerPath -PathType Leaf)) {
        throw "T11 CPU worker PE is missing before desktop build: $cpuWorkerPath"
    }
    $script:expectedCpuWorkerSha256 = (Get-FileHash -LiteralPath $cpuWorkerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-Output "T11_EMBEDDED_CPU_WORKER path=$cpuWorkerPath sha256=$script:expectedCpuWorkerSha256"
    Invoke-GuardedBuild @('build','--release','-p','whisper-desktop','-vv') -DesktopCpuWorkerSha256 $script:expectedCpuWorkerSha256
    Invoke-GuardedBuild @('build','--release','-p','whisper-bootstrap','-vv')

    $targetRoot = Join-Path $script:lotRoot "targets/$script:targetName"
    Assert-InRoot $targetRoot $script:lotRoot 'Cargo target'
    Assert-PortableCmakeOutputs $targetRoot
    foreach ($package in @('whisper-worker-cpu','whisper-desktop','whisper-bootstrap')) {
        $binary = switch ($package) {
            'whisper-worker-cpu' { 'whisper-worker-cpu.exe' }
            'whisper-desktop' { 'whisper-desktop.exe' }
            'whisper-bootstrap' { 'whisper-bootstrap.exe' }
        }
        Assert-X64Pe $package (Join-Path $targetRoot "release/$binary")
    }
    Assert-DesktopCpuWorkerDigest $targetRoot (Join-Path $targetRoot 'release/whisper-desktop.exe')
} finally {
    Restore-ProcessEnvironment $script:baselineEnvironment
    $baselineClPresent = $script:baselineEnvironment.ContainsKey('_CL_')
    $restoredClPresent = Test-Path Env:_CL_
    $restoredCl = [Environment]::GetEnvironmentVariable('_CL_', 'Process')
    if ($restoredClPresent -ne $baselineClPresent -or
        ($baselineClPresent -and $restoredCl -cne [string]$script:baselineEnvironment['_CL_'])) {
        throw 'T11 did not restore the original _CL_ process environment value.'
    }
    $digestName = 'WHISPER_EXPECTED_CPU_WORKER_SHA256'
    $baselineDigestPresent = $script:baselineEnvironment.ContainsKey($digestName)
    $restoredDigestPresent = Test-Path -LiteralPath "Env:$digestName"
    $restoredDigest = [Environment]::GetEnvironmentVariable($digestName, 'Process')
    if ($baselineDigestPresent -ne $restoredDigestPresent -or
        ($baselineDigestPresent -and $restoredDigest -cne [string]$script:baselineEnvironment[$digestName])) {
        throw 'T11 did not restore the original expected CPU worker SHA-256 environment value.'
    }
    Write-Output "T11_ENV_RESTORE _CL_present=$restoredClPresent worker_digest_present=$restoredDigestPresent"
}
