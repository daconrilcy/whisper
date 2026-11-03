param(
    [ValidateSet('Run','SelfTest','Structure')]
    [string]$Mode = 'Run',
    [switch]$SelfTest,
    [switch]$DryRun,
    [string]$FreshTarget,
    [string]$ExistingTarget,
    [string[]]$UnsetEnv = @(),
    [string[]]$CargoArgs = @(),
    [string]$CargoArgsJson,
    [string]$LotRoot = 'target/parakeet-tests/garde-n1-categories'
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
function Convert-CargoArgsJson([string]$Json) {
    $parsed = ConvertFrom-Json -InputObject $Json
    if ($parsed -is [string]) { return ,([string[]]@($parsed)) }
    if ($parsed -isnot [array] -or @($parsed | Where-Object { $_ -isnot [string] }).Count -gt 0) {
        throw '-CargoArgsJson must be a JSON array of strings.'
    }
    return ,([string[]]$parsed)
}

$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$lotRoot = Join-Path $repo $LotRoot
$cargoRoot = 'C:\dev\whisper\target\parakeet-tests\cargo-shared'
$cargoHome = Join-Path $repo 'target/cargo-home-shared'
$tempRoot = Join-Path $lotRoot 'temp'
$scratchRoot = Join-Path $lotRoot 'scratch'
$runRoot = $null
$utf8NoBom = [Text.UTF8Encoding]::new($false)
$allowedUnsetPatterns = @('^CUDA_PATH$', '^CUDA_PATH_V.*$', '^GGML_.*$', '^WHISPER_.*$', '^PARAKEET_.*$')

if ($CargoArgsJson) {
    if ($CargoArgs.Count -gt 0) { throw 'Use either -CargoArgs or -CargoArgsJson, not both.' }
    $CargoArgs = Convert-CargoArgsJson $CargoArgsJson
}

function Get-FullPath([string]$Path) { [IO.Path]::GetFullPath($Path).TrimEnd('\') }

function Resolve-LotRoot([string]$Path) {
    $full = if ([IO.Path]::IsPathRooted($Path)) { Get-FullPath $Path } else { Get-FullPath (Join-Path $repo $Path) }
    if (-not (Test-InRoot $full $repo)) { throw "Lot root must be inside the worktree: $full" }
    Assert-NoReparseChain $full 'lot root preflight'
    return $full
}

function Test-InRoot([string]$Path, [string]$Root) {
    $pathFull = Get-FullPath $Path
    $rootFull = Get-FullPath $Root
    return $pathFull.Equals($rootFull, [StringComparison]::OrdinalIgnoreCase) -or
        $pathFull.StartsWith($rootFull + '\', [StringComparison]::OrdinalIgnoreCase)
}

function Assert-NoReparseChain([string]$Path, [string]$Name) {
    $cursor = Get-FullPath $Path
    while ($true) {
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -LiteralPath $cursor -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "$Name crosses a reparse point: $cursor" }
        }
        $parent = Split-Path -Parent $cursor
        if (-not $parent -or $parent -eq $cursor) { return }
        $cursor = $parent
    }
}

function Initialize-Directory([string]$Path, [string]$Name) {
    Assert-NoReparseChain $Path $Name
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
    Assert-NoReparseChain $Path $Name
}

function Get-FreshTargetPath([string]$Root, [string]$Name) {
    if ($Name -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$') { throw '-FreshTarget accepts a simple name only.' }
    $path = Join-Path $Root (Join-Path 'targets' $Name)
    if (-not (Test-InRoot $path $Root)) { throw '-FreshTarget escaped the lot root.' }
    return $path
}

function Get-ExistingTargetPath([string]$Root, [string]$Name) {
    $path = Get-FreshTargetPath $Root $Name
    if (-not (Test-Path -LiteralPath $path -PathType Container)) { throw "Existing target does not exist: $path" }
    Assert-NoReparseChain $path 'existing target'
    if (-not (Test-InRoot $path $Root)) { throw 'Existing target escaped its lot root.' }
    return $path
}

function Assert-AllowedUnset([string[]]$Names) {
    foreach ($name in $Names) {
        $valid = $false
        foreach ($pattern in $allowedUnsetPatterns) { if ($name -cmatch $pattern) { $valid = $true; break } }
        if (-not $valid) { throw "Environment variable is not allowed in -UnsetEnv: $name" }
    }
}

function Get-FeatureNames([string]$FeatureList) {
    foreach ($token in [regex]::Split($FeatureList, '[,\s]+')) {
        if ([string]::IsNullOrWhiteSpace($token)) { continue }
        $lastSegment = ($token -split '/')[-1].TrimEnd('?')
        $lastSegment
    }
}

function Assert-AllowedFeatures([string[]]$Values, [bool]$Release) {
    foreach ($value in $Values) {
        foreach ($featureName in Get-FeatureNames $value) {
            if ($featureName -ceq 'native-tests' -or ($featureName -ceq 'recette-hooks' -and $Release)) {
                throw "Feature is not allowed in this context: $value"
            }
        }
    }
}

function Get-WorkspaceMembers([string[]]$Members) { return @($Members) }

function Test-CargoArguments([string[]]$CommandTokens, [string[]]$Members) {
    if ($CommandTokens.Count -lt 1) { throw 'A Cargo command is required.' }
    if ($CommandTokens[0] -ceq 'fetch') {
        if ($CommandTokens.Count -ne 2 -or $CommandTokens[1] -cne '--locked') {
            throw 'Only cargo fetch --locked is allowed for network access.'
        }
        return [pscustomobject]@{ Args = @('fetch','--locked'); Category = 'Fetch' }
    }
    $normalizedArgs = [Collections.Generic.List[string]]::new()
    foreach ($arg in $CommandTokens) {
        if ($arg -in @('--offline', '--locked')) { continue }
        $normalizedArgs.Add($arg)
    }
    $sub = $normalizedArgs[0]
    $readCommands = @('tree', 'metadata')
    $compileCommands = @('check', 'build', 'clippy')
    if ($sub -notin ($readCommands + $compileCommands + @('test', 'fmt'))) { throw "Cargo subcommand is not allowed: $sub" }
    if ($sub -eq 'fmt') {
        if ($normalizedArgs.Count -ne 2 -or $normalizedArgs[1] -cne '--check') { throw 'Only cargo fmt --check is allowed.' }
        return [pscustomobject]@{ Args = @('fmt', '--check'); Category = 'Lecture' }
    }
    $isTest = $sub -eq 'test'
    $category = if ($sub -in $readCommands) { 'Lecture' } elseif ($isTest) { 'Tests' } else { 'Compilation' }
    $hasWorkspace = $false
    $packages = [Collections.Generic.List[string]]::new()
    $features = [Collections.Generic.List[string]]::new()
    $release = $false
    $noRun = $false
    $list = $false
    $exact = $false
    $testFilter = $null
    for ($i = 1; $i -lt $normalizedArgs.Count; $i++) {
        $arg = $normalizedArgs[$i]
        if ($arg -eq '--') {
            if ($sub -ne 'test' -or $i -ne ($normalizedArgs.Count - 2)) { throw 'Only the allowlisted cargo test options are allowed after --.' }
            if ($normalizedArgs[$i + 1] -ceq '--list') {
                $list = $true
            } elseif ($normalizedArgs[$i + 1] -ceq '--exact') {
                $exact = $true
            } else {
                throw 'Only cargo test -- --list or the approved exact microphone test is allowed after --.'
            }
            $i++
            continue
        }
        if ($arg -in @('--fix','--allow-dirty','--all-features','--ignored','--include-ignored','--config','-Z','--target-dir','--workspace-dir','--all','--examples','--example','--benches','--bench','--bins','--bin','--tests','--doc','--no-default-features')) { throw "Cargo option is not allowed: $arg" }
        if ($arg -match '^--(fix|allow-dirty|all-features|ignored|include-ignored|config|target-dir)(=|$)' -or $arg -match '^-Z') { throw "Cargo option is not allowed: $arg" }
        if ($arg -eq '--manifest-path') {
            if (($i + 1) -ge $normalizedArgs.Count) { throw '--manifest-path requires a path.' }
            $i++
            $manifestPath = $normalizedArgs[$i]
            $candidateManifest = if ([IO.Path]::IsPathRooted($manifestPath)) { Get-FullPath $manifestPath } else { Get-FullPath (Join-Path $repo $manifestPath) }
            if (-not (Test-InRoot $candidateManifest $repo) -or [IO.Path]::GetFileName($candidateManifest) -cne 'Cargo.toml') { throw '--manifest-path must select Cargo.toml inside the worktree.' }
            Assert-NoReparseChain $candidateManifest 'manifest path'
            if (-not (Test-Path -LiteralPath $candidateManifest -PathType Leaf)) { throw '--manifest-path does not exist inside the worktree.' }
            continue
        }
        if ($arg -eq '--workspace') {
            if ($sub -notin @('tree','check','build','clippy','test')) { throw "--workspace is not valid for cargo $sub." }
            $hasWorkspace = $true
            continue
        }
        if ($arg -eq '-p') {
            if ($sub -notin @('tree','check','build','clippy','test')) { throw "-p is not valid for cargo $sub." }
            if (($i + 1) -ge $normalizedArgs.Count -or $normalizedArgs[$i + 1].StartsWith('-')) { throw '-p requires a workspace package name.' }
            $i++
            $packages.Add($normalizedArgs[$i])
            continue
        }
        if ($arg -match '^-p(.+)$') {
            if ($sub -notin @('tree','check','build','clippy','test')) { throw "-p is not valid for cargo $sub." }
            $packages.Add($Matches[1])
            continue
        }
        if ($arg -eq '--release') {
            if ($sub -notin @('check','build','clippy','test')) { throw "--release is not valid for cargo $sub." }
            $release = $true
            continue
        }
        if ($arg -eq '--no-run') {
            if ($sub -ne 'test') { throw '--no-run is only allowed with cargo test.' }
            $noRun = $true
            continue
        }
        if ($arg -eq '--all-targets') {
            if ($sub -ne 'clippy') { throw '--all-targets is only allowed with cargo clippy.' }
            continue
        }
        if ($arg -in @('-e','--edges','-i','--invert','--format-version','--filter-platform')) {
            $allowedCommands = if ($arg -in @('-e','--edges','-i','--invert')) { @('tree') } else { @('metadata') }
            if ($sub -notin $allowedCommands) { throw "$arg is not valid for cargo $sub." }
            if (($i + 1) -ge $normalizedArgs.Count -or $normalizedArgs[$i + 1].StartsWith('-')) { throw "$arg requires a value." }
            $i++
            continue
        }
        if ($arg -eq '--features' -or $arg -eq '--exclude') {
            if ($sub -notin @('tree','check','build','clippy','test')) { throw "$arg is not valid for cargo $sub." }
            if (($i + 1) -ge $normalizedArgs.Count) { throw "$arg requires a value." }
            $i++
            $value = $normalizedArgs[$i]
            $features.Add($value)
            if ($arg -eq '--exclude') { throw '--exclude is not allowed.' }
            continue
        }
        if ($arg -match '^--features=(.*)$') {
            if ($sub -notin @('tree','check','build','clippy','test')) { throw '--features is not valid for cargo $sub.' }
            $value = $Matches[1]
            $features.Add($value)
            continue
        }
        if ($arg -eq '--no-deps') {
            if ($sub -ne 'metadata') { throw '--no-deps is only allowed with cargo metadata.' }
            continue
        }
        if ($isTest -and $arg -ceq 'worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end') {
            if ($null -ne $testFilter) { throw 'Only one test filter is allowed.' }
            $testFilter = $arg
            continue
        }
        if ($arg -match '^-[A-Za-z]') { throw "Unrecognized Cargo option: $arg" }
        if ($arg.StartsWith('-')) { throw "Unrecognized Cargo option: $arg" }
        throw "Unrecognized Cargo argument: $arg"
    }
    if ($exact -or $null -ne $testFilter) {
        # The only exact-filter exception authorized for level 1 is this one micro regression test.
        $expectedExact = @('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
        if (-not $exact -or $null -eq $testFilter -or $list -or $sub -cne 'test' -or $hasWorkspace -or $packages.Count -ne 1 -or $packages[0] -cne 'whisper-adapters' -or $features.Count -gt 0 -or $release -or $noRun -or $normalizedArgs.Count -ne $expectedExact.Count) {
            throw 'Only the approved exact microphone test selection is allowed.'
        }
        for ($exactIndex = 0; $exactIndex -lt $expectedExact.Count; $exactIndex++) {
            if ($normalizedArgs[$exactIndex] -cne $expectedExact[$exactIndex]) { throw 'Only the approved exact microphone test selection is allowed.' }
        }
    }
    if ($sub -eq 'test') {
        if ($list -and $noRun) { throw 'cargo test --no-run cannot be combined with --list.' }
        if (-not $list -and $packages -contains 'whisper-worker-gpu') { throw 'Direct whisper-worker-gpu tests are level 2; only -- --list is allowed through level 1.' }
        if ($noRun -and ($hasWorkspace -or $packages.Count -gt 0)) { $category = 'Compilation' }
        elseif ($noRun) { throw 'cargo test --no-run requires --workspace or -p.' }
    } elseif ($noRun) { throw '--no-run is only allowed with cargo test.' }
    if ($sub -eq 'tree' -and ($release -or $noRun)) { throw 'cargo tree accepts only its allowlisted options.' }
    if ($sub -eq 'metadata' -and ($release -or $noRun)) { throw 'cargo metadata accepts only its allowlisted options.' }
    Assert-AllowedFeatures $features.ToArray() $release
    if ($sub -in @('check','build','clippy') -and -not ($hasWorkspace -or $packages.Count -gt 0)) { throw "$sub requires --workspace or one or more -p members." }
    if ($sub -in $readCommands -and ($hasWorkspace -or $packages.Count -gt 0)) { }
    foreach ($package in $packages) { if ($package -cnotin $Members) { throw "Unknown workspace package: $package" } }
    $result = [Collections.Generic.List[string]]::new()
    $result.Add($sub)
    $result.Add('--offline')
    $result.Add('--locked')
    for ($resultIndex = 1; $resultIndex -lt $normalizedArgs.Count; $resultIndex++) { $result.Add($normalizedArgs[$resultIndex]) }
    return [pscustomobject]@{ Args = @($result.ToArray()); Category = $category }
}

function Assert-WriteRoots {
    if (-not (Test-InRoot $lotRoot $repo)) { throw "Lot root must be inside the worktree: $lotRoot" }
    foreach ($path in @($lotRoot, $cargoRoot, $cargoHome, $tempRoot, $scratchRoot, $runRoot, $script:targetRoot)) { Assert-NoReparseChain $path 'write root' }
    if ((Get-FullPath $cargoHome) -ine (Get-FullPath (Join-Path $repo 'target/cargo-home-shared'))) { throw 'CARGO_HOME shared root differs from its declared persistent location.' }
    if (-not (Test-InRoot $tempRoot $lotRoot) -or -not (Test-InRoot $scratchRoot $lotRoot)) { throw 'Campaign roots escaped the lot root.' }
    if ((Get-FullPath $env:CARGO_TARGET_DIR) -ine (Get-FullPath $script:targetRoot)) { throw 'CARGO_TARGET_DIR differs from the selected target.' }
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA')) {
        $value = [Environment]::GetEnvironmentVariable($name)
        if ([string]::IsNullOrWhiteSpace($value) -or -not (Test-InRoot $value $lotRoot)) { throw "$name is missing or outside the lot root." }
        Assert-NoReparseChain $value $name
    }
    if ([string]::IsNullOrWhiteSpace($env:CARGO_HOME) -or (Get-FullPath $env:CARGO_HOME) -ine (Get-FullPath $cargoHome)) { throw 'CARGO_HOME differs from its declared persistent location.' }
    Assert-NoReparseChain $env:CARGO_HOME 'CARGO_HOME'
    if (-not (Test-InRoot $env:TEMP $tempRoot) -or -not (Test-InRoot $env:TMP $tempRoot)) { throw 'TEMP/TMP are not isolated below the lot root.' }
    if (-not (Test-InRoot $env:LOCALAPPDATA $runRoot)) { throw 'LOCALAPPDATA is not isolated under this run root.' }
    if ($env:CARGO_INCREMENTAL -cne '0' -or $env:CARGO_PROFILE_DEV_DEBUG -cne 'line-tables-only') { throw 'Cargo profile environment differs from level-1 settings.' }
    if ((Get-PSDrive -Name C).Free -lt 30GB) { throw 'C: has less than 30 GiB free; Cargo is forbidden.' }
    if (@(Get-ChildItem -LiteralPath $env:LOCALAPPDATA -Force).Count -ne 0) { throw 'LOCALAPPDATA must be empty before every run.' }
    foreach ($entry in Get-ChildItem Env:) {
        if ($entry.Name -match '^(WHISPER_|PARAKEET_)') { throw "Campaign variable was not cleared: $($entry.Name)" }
    }
}

function Invoke-Captured([string]$Executable, [string[]]$Arguments, [string]$Label) {
    $outPath = Join-Path $runRoot "$Label.stdout.log"
    $errPath = Join-Path $runRoot "$Label.stderr.log"
    foreach ($path in @($outPath,$errPath)) {
        if (-not (Test-InRoot $path $runRoot)) { throw "Output path outside run root: $path" }
        Assert-NoReparseChain (Split-Path -Parent $path) 'process output directory'
    }
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $Executable
    $start.WorkingDirectory = $repo
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    foreach ($argument in $Arguments) { [void]$start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    try {
        if (-not $process.Start()) { throw "Could not start $Executable" }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        [IO.File]::WriteAllText($outPath,$stdout,$utf8NoBom)
        [IO.File]::WriteAllText($errPath,$stderr,$utf8NoBom)
        Assert-NoReparseChain $outPath 'stdout log'
        Assert-NoReparseChain $errPath 'stderr log'
        return [pscustomobject]@{ ExitCode=$process.ExitCode; Stdout=$stdout; Stderr=$stderr; OutPath=$outPath; ErrPath=$errPath }
    } finally { $process.Dispose() }
}

function Set-CleanEnvironment {
    foreach ($entry in @(Get-ChildItem Env: | Where-Object Name -Match '^(WHISPER_.*|PARAKEET_.*)$')) { Remove-Item "Env:$($entry.Name)" -ErrorAction SilentlyContinue }
    foreach ($name in $UnsetEnv) { [Environment]::SetEnvironmentVariable($name,$null,'Process') }
    $script:runRoot = Join-Path $scratchRoot ((Get-Date -Format 'yyyyMMddTHHmmssfff') + '-' + [guid]::NewGuid().ToString('N'))
    foreach ($path in @($script:runRoot,(Join-Path $runRoot 'localappdata'),(Join-Path $tempRoot 'temp'),(Join-Path $tempRoot 'tmp'))) { Initialize-Directory $path 'isolated run directory' }
    foreach ($name in @('CARGO_HOME','CARGO_TARGET_DIR','CARGO_INCREMENTAL','CARGO_PROFILE_DEV_DEBUG','TEMP','TMP','LOCALAPPDATA')) { [Environment]::SetEnvironmentVariable($name,$null,'Process') }
    $env:CARGO_HOME = $cargoHome
    $env:CARGO_TARGET_DIR = $script:targetRoot
    $env:CARGO_INCREMENTAL = '0'
    $env:CARGO_PROFILE_DEV_DEBUG = 'line-tables-only'
    $env:TEMP = Join-Path $tempRoot 'temp'
    $env:TMP = Join-Path $tempRoot 'tmp'
    $env:LOCALAPPDATA = Join-Path $runRoot 'localappdata'
}

function Set-CargoCommandEnvironment([string]$Subcommand) {
    if ($Subcommand -eq 'clippy') { [Environment]::SetEnvironmentVariable('RUSTFLAGS',$null,'Process') }
}

function Initialize-Roots {
    if ((Get-PSDrive -Name C).Free -lt 30GB) { throw 'C: has less than 30 GiB free; campaign setup is forbidden.' }
    foreach ($path in @($lotRoot,$cargoRoot,$cargoHome,$tempRoot,$scratchRoot)) { Assert-NoReparseChain $path 'lot root preflight' }
    foreach ($path in @($lotRoot,$tempRoot,$scratchRoot)) { Initialize-Directory $path 'lot root' }
}

function Assert-OfflineCache([string]$CacheRoot) {
    Assert-NoReparseChain $CacheRoot 'persistent Cargo cache'
    if (-not (Test-Path -LiteralPath (Join-Path $CacheRoot 'registry/src') -PathType Container)) {
        throw "Offline Cargo cache is absent or empty at $CacheRoot. Initialize it through ci/guard-level1.ps1 -CargoArgsJson '[`"fetch`",`"--locked`"]' before an offline command."
    }
}

function Invoke-SelfTests {
    $members = @('whisper-core','whisper-adapters','whisper-worker-common','whisper-worker-cpu','whisper-worker-gpu','whisper-worker-parakeet-cpu')
    $singletonJsonArgs = @(Convert-CargoArgsJson '["test"]')
    if ($singletonJsonArgs.Count -ne 1 -or $singletonJsonArgs[0] -cne 'test') { throw 'Self-test did not preserve a singleton JSON Cargo argv.' }
    $positive = @(
        @('fetch','--locked'),
        @('tree','-p','whisper-worker-cpu','-e','features','-i','whisper-rs-sys'),
        @('metadata','--format-version','1'),
        @('fmt','--check'),
        @('build','-p','whisper-worker-cpu','--features','cpu-fast,another-feature'),
        @('build','-p','whisper-worker-cpu','--features','cpu-fast another-feature'),
        @('build','-p','whisper-worker-cpu','--features','recette-hooks'),
        @('build','-p','whisper-worker-cpu'),
        @('test','--no-run','-p','whisper-worker-cpu'),
        @('test','--workspace'),
        @('test'),
        @('test','--','--list'),
        @('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact'),
        @('test','--offline','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact','--locked'),
        @('test','-p','whisper-worker-common','--','--list'),
        @('clippy','--workspace','--all-targets'),
        @('check','--workspace','--release')
    )
    $negative = @(
        @('fetch'),@('fetch','--offline'),@('fetch','--locked','--offline'),@('fetch','--locked','--config','x'),
        @('run','--workspace'),@('install','whisper-core'),@('uninstall','whisper-core'),@('update'),@('publish'),@('clean'),@('add','serde'),@('remove','serde'),@('fix'),@('generate-lockfile'),@('vendor'),@('cargo-unknown'),
        @('build','--fix','--workspace'),@('build','--allow-dirty','--workspace'),@('build','--all-features','--workspace'),@('test','--ignored','--workspace'),@('test','--include-ignored','--workspace'),@('test','--features','native-tests','--workspace'),@('test','--features','recette-hooks','--release','--workspace'),@('build','--config','x','--workspace'),@('build','-Z','foo','--workspace'),@('build','--target-dir','x','--workspace'),@('build','--manifest-path','..\Cargo.toml','--workspace'),@('build','--unknown','--workspace'),@('build','-p','not-a-member')
    )
    $negative += ,@('build','-p','whisper-worker-cpu','--features','native-tests')
    $negative += ,@('build','-p','whisper-worker-cpu','--features=whisper-worker-cpu/native-tests')
    $negative += ,@('build','-p','whisper-worker-cpu','--features','other native-tests')
    $negative += ,@('build','-p','whisper-worker-cpu','--features','another,whisper-worker-cpu/native-tests')
    $negative += ,@('build','-p','whisper-worker-cpu','--features','safe','--features','native-tests')
    $negative += ,@('build','-p','whisper-worker-cpu','--features','whisper-worker-cpu/recette-hooks','--release')
    $negative += ,@('build','-p','whisper-worker-cpu','--features=whisper-worker-cpu/recette-hooks','--release')
    $negative += ,@('build','-p','whisper-worker-cpu','--release','--features','recette-hooks')
    $negative += ,@('build','-p','whisper-worker-cpu','-e','features')
    $negative += ,@('metadata','-p','whisper-worker-cpu')
    $negative += ,@('tree','--workspace','--release')
    $negative += ,@('check','--workspace','--all-targets')
    $negative += ,@('test','--workspace','-i','features')
    $negative += ,@('test','--workspace','--no-run','--','--list')
    $negative += ,@('test','-p','whisper-worker-cpu','--','--ignored')
    $negative += ,@('test','-p','whisper-worker-cpu','--','--list','--exact')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact','--nocapture')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','-p','whisper-worker-cpu','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','whisper-worker-common','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','--workspace','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--release','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--no-run','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--features','native-tests','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--list')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end_typo','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::Microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','microphone_legacy_live_pump','--','--exact')
    $negative += ,@('check','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','-p','whisper-worker-gpu')
    $negative += ,@('test','-p','whisper-worker-gpu','--no-run')
    $pCount = 1
    $fetch = Test-CargoArguments @('fetch','--locked') $members
    if ($fetch.Category -cne 'Fetch' -or ($fetch.Args -join ' ') -cne 'fetch --locked') { throw 'Self-test did not preserve locked fetch without offline.' }
    $pCount++
    $missingCache = Join-Path $runRoot 'absent-cache'
    [void](Test-CargoArguments @('build','--offline','--workspace') $members)
    $cacheRejected = $false
    try { Assert-OfflineCache $missingCache } catch { $cacheRejected = $_.Exception.Message -match 'cache is absent.*fetch' }
    if (-not $cacheRejected -or (Test-Path -LiteralPath $missingCache)) { throw 'Missing offline cache must be rejected with fetch guidance before Cargo.' }
    $nCount = 1
    $presentCache = Join-Path $runRoot 'populated-cache-fixture'
    Initialize-Directory (Join-Path $presentCache 'registry/src') 'offline cache fixture'
    Assert-OfflineCache $presentCache
    $pCount++
    $listCases = @(
        @{ Args=@('test'); Expected='test --offline --locked'; Category='Tests' },
        @{ Args=@('test','-p','whisper-worker-cpu','--','--list'); Expected='test --offline --locked -p whisper-worker-cpu -- --list'; Category='Tests' },
        @{ Args=@('test','-p','whisper-worker-parakeet-cpu','--','--list'); Expected='test --offline --locked -p whisper-worker-parakeet-cpu -- --list'; Category='Tests' },
        @{ Args=@('test','-p','whisper-worker-common','--','--list'); Expected='test --offline --locked -p whisper-worker-common -- --list'; Category='Tests' },
        @{ Args=@('test','-p','whisper-worker-gpu','--','--list'); Expected='test --offline --locked -p whisper-worker-gpu -- --list'; Category='Tests' }
    )
    foreach ($listCase in $listCases) {
        $parsedList = Test-CargoArguments $listCase.Args $members
        if (([string]::Join(' ',$parsedList.Args)) -cne $listCase.Expected -or $parsedList.Category -cne $listCase.Category) {
            throw "Self-test failed for test argv: $($listCase.Args -join ' ')"
        }
        $pCount++
    }
    $multiJsonArgs = Convert-CargoArgsJson '["test","-p","whisper-worker-cpu","--","--list"]'
    if ($multiJsonArgs.Count -ne 5 -or ([string]::Join(' ',$multiJsonArgs)) -cne 'test -p whisper-worker-cpu -- --list') {
        throw 'Self-test did not preserve a multi-argument JSON Cargo argv.'
    }
    $pCount++
    foreach ($case in $positive) { [void](Test-CargoArguments $case $members); $pCount++ }
    foreach ($case in $negative) { $rejected=$false; try { [void](Test-CargoArguments $case $members) } catch { $rejected=$true }; if (-not $rejected) { throw "Self-test accepted: $($case -join ' ')" }; $nCount++ }
    foreach ($bad in @('OTHER_VAR','CMAKE_PREFIX_PATH','CUDA_VISIBLE_DEVICES')) { $rejected=$false; try { Assert-AllowedUnset @($bad) } catch { $rejected=$true }; if (-not $rejected) { throw "Self-test accepted -UnsetEnv $bad" }; $nCount++ }
    foreach ($good in @('CUDA_PATH','CUDA_PATH_V12_8','GGML_CUDA','WHISPER_ENGINE','PARAKEET_MODEL')) { Assert-AllowedUnset @($good); $pCount++ }
    $rejected=$false
    try { Assert-AllowedUnset @('CUDA_PATH_NOT_V') } catch { $rejected=$true }
    if (-not $rejected) { throw 'Self-test accepted CUDA_PATH_NOT_V.' }
    $nCount++
    $junctionBase = Join-Path $runRoot 'junction-fixture'
    $outsideBase = Join-Path $runRoot 'junction-target'
    Initialize-Directory $outsideBase 'junction self-test target'
    New-Item -ItemType Junction -Path $junctionBase -Target $outsideBase | Out-Null
    $rejected=$false
    try { Assert-NoReparseChain (Join-Path $junctionBase 'fresh') 'fresh target fixture' } catch { $rejected=$true }
    if (-not $rejected) { throw 'Self-test accepted a fresh target below a junction.' }
    $nCount++
    $rejected=$false
    try { [void](Get-FreshTargetPath $runRoot '..\outside') } catch { $rejected=$true }
    if (-not $rejected) { throw 'Self-test accepted a fresh target outside its root.' }
    $nCount++
    $fixtureLot = Join-Path $runRoot 'fresh-fixture'
    Initialize-Directory $fixtureLot 'fresh target fixture lot root'
    New-Item -ItemType Junction -Path (Join-Path $fixtureLot 'targets') -Target $outsideBase | Out-Null
    $fixtureTarget = Get-FreshTargetPath $fixtureLot 'junction-target'
    $rejected=$false
    try { Assert-NoReparseChain (Split-Path -Parent $fixtureTarget) 'fresh target fixture parent' } catch { $rejected=$true }
    if (-not $rejected) { throw 'Self-test accepted a fresh target parent junction.' }
    $nCount++
    Initialize-Directory (Join-Path $outsideBase 'junction-target') 'existing target junction fixture'
    $rejected = $false
    try { [void](Get-ExistingTargetPath $fixtureLot 'junction-target') } catch { $rejected = $true }
    if (-not $rejected) { throw 'Self-test accepted an existing target through a junction.' }
    $nCount++
    foreach ($withFresh in @($false,$true)) {
        $outsideLot = Get-FullPath (Join-Path $repo ('..\garde-n1-selftest-outside-' + [string]$withFresh))
        $rejected=$false
        try {
            $resolved = Resolve-LotRoot $outsideLot
            if ($withFresh) { [void](Get-FreshTargetPath $resolved 'A') }
        } catch { $rejected=$true }
        if (-not $rejected -or (Test-Path $outsideLot)) { throw 'Self-test did not reject an outside lot before creating it.' }
        $nCount++
    }
    if ((Get-FullPath $cargoRoot) -cne 'C:\dev\whisper\target\parakeet-tests\cargo-shared') { throw 'Self-test found a noncanonical shared Cargo target.' }
    $freshFixturePath = Get-FreshTargetPath $runRoot 'fresh-selftest-target'
    if (-not (Test-InRoot $freshFixturePath $runRoot) -or (Get-FullPath $freshFixturePath) -ieq (Get-FullPath $cargoRoot)) { throw 'Fresh target is not distinct and under the lot root.' }
    $pCount += 2
    $existingFixtureLot = Join-Path $runRoot 'existing-fixture'
    Initialize-Directory $existingFixtureLot 'existing target fixture lot root'
    $existingFixturePath = Join-Path $existingFixtureLot 'targets\existing-target'
    Initialize-Directory $existingFixturePath 'existing target self-test fixture'
    if ((Get-FullPath (Get-ExistingTargetPath $existingFixtureLot 'existing-target')) -ine (Get-FullPath $existingFixturePath)) { throw 'Existing target selection returned a different path.' }
    $nCount++
    foreach ($badExisting in @('..\outside','missing-target')) {
        $rejected = $false
        try { [void](Get-ExistingTargetPath $existingFixtureLot $badExisting) } catch { $rejected = $true }
        if (-not $rejected) { throw "Self-test accepted invalid existing target: $badExisting" }
        $nCount++
    }
    $savedVariables = @{}
    try {
        foreach ($name in @('WHISPER_N1_REVIEW','PARAKEET_N1_REVIEW','RUSTFLAGS')) { $savedVariables[$name] = [Environment]::GetEnvironmentVariable($name) }
        $savedCargoHome = $env:CARGO_HOME
        try {
            $env:CARGO_HOME = Join-Path $runRoot 'unapproved-cargo-home'
            $rejected=$false
            try { Assert-WriteRoots } catch { $rejected=$true }
            if (-not $rejected) { throw 'Self-test accepted a noncanonical CARGO_HOME.' }
            $nCount++
        } finally { $env:CARGO_HOME = $savedCargoHome }
        foreach ($name in @('WHISPER_N1_REVIEW','PARAKEET_N1_REVIEW')) {
            [Environment]::SetEnvironmentVariable($name,'not-cleared','Process')
            $rejected=$false
            try { Assert-WriteRoots } catch { $rejected=$true }
            if (-not $rejected) { throw "Self-test accepted surviving $name." }
            [Environment]::SetEnvironmentVariable($name,$null,'Process')
            $nCount++
        }
        [Environment]::SetEnvironmentVariable('RUSTFLAGS','-Dwarnings','Process')
        Set-CargoCommandEnvironment 'clippy'
        if (-not [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable('RUSTFLAGS'))) { throw 'Self-test did not clear RUSTFLAGS for clippy.' }
        $pCount++
        [Environment]::SetEnvironmentVariable('RUSTFLAGS','-Dwarnings','Process')
        Set-CargoCommandEnvironment 'build'
        if ([Environment]::GetEnvironmentVariable('RUSTFLAGS') -cne '-Dwarnings') { throw 'Self-test changed RUSTFLAGS for a non-clippy command.' }
        $pCount++
    } finally {
        foreach ($name in $savedVariables.Keys) { [Environment]::SetEnvironmentVariable($name,$savedVariables[$name],'Process') }
    }
    'PASS self-test parser cases (no Cargo): positives={0}, negatives={1}.' -f $pCount,$nCount
}

function Invoke-Structure {
    Assert-WriteRoots
    $source = Join-Path $PSScriptRoot 'check-structure.rs'
    $baseline = Join-Path $PSScriptRoot 'tests-baseline.txt'
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw 'ci/check-structure.rs is missing.' }
    if (-not (Test-Path -LiteralPath $baseline -PathType Leaf)) { throw 'ci/tests-baseline.txt is missing.' }
    $rustc = (Get-Command rustc -ErrorAction Stop).Source
    $checker = Join-Path $runRoot 'check-structure.exe'
    if ($DryRun) { "DRY_RUN mode=Structure rustc --edition=2024 $source -o $checker; then checker --self-test and $baseline"; return }
    $compile = Invoke-Captured $rustc @('--edition=2024',$source,'-o',$checker) 'structure-compile'
    if ($compile.ExitCode -ne 0) { throw "Structure checker compile failed; logs: $($compile.OutPath), $($compile.ErrPath)" }
    $selfTest = Invoke-Captured $checker @('--self-test',(Join-Path $runRoot 'checker-selftest-fixtures')) 'structure-selftest'
    if ($selfTest.ExitCode -ne 0) { throw "Structure checker self-test failed; logs: $($selfTest.OutPath), $($selfTest.ErrPath)" }
    $check = Invoke-Captured $checker @($baseline) 'structure-check'
    Get-Content -LiteralPath $check.OutPath -Tail 40
    Get-Content -LiteralPath $check.ErrPath -Tail 40
    "N1_GUARD mode=Structure exit_code=$($check.ExitCode) stdout=$($check.OutPath) stderr=$($check.ErrPath)"
    if ($check.ExitCode -ne 0) { throw "Structure checks failed; logs: $($check.OutPath), $($check.ErrPath)" }
}

$SelfTest = $SelfTest -or $Mode -eq 'SelfTest'
Assert-AllowedUnset $UnsetEnv
$lotRoot = Resolve-LotRoot $LotRoot
if ((Get-PSDrive -Name C).Free -lt 30GB) { throw 'C: has less than 30 GiB free; campaign setup is forbidden.' }
if ($FreshTarget) {
    if ($ExistingTarget) { throw 'Choose either -FreshTarget or -ExistingTarget, not both.' }
    $script:targetRoot = Get-FreshTargetPath $lotRoot $FreshTarget
    if (Test-Path -LiteralPath $script:targetRoot) { throw "Fresh target already exists: $script:targetRoot" }
    if (-not (Test-InRoot $script:targetRoot $lotRoot)) { throw 'Fresh target escaped the lot root.' }
    Assert-NoReparseChain (Split-Path -Parent $script:targetRoot) 'fresh target parent'
} elseif ($ExistingTarget) { $script:targetRoot = Get-ExistingTargetPath $lotRoot $ExistingTarget }
else { $script:targetRoot = $cargoRoot }
if ($Mode -eq 'Structure' -and $CargoArgs.Count -gt 0) { throw 'Structure mode does not accept Cargo arguments.' }
Initialize-Roots
Set-CleanEnvironment
Assert-WriteRoots
if ($SelfTest) { Invoke-SelfTests; exit 0 }
if ($Mode -eq 'Structure') { Invoke-Structure; exit 0 }

$members = @()
if ($CargoArgs.Count -gt 0 -and $CargoArgs[0] -ceq 'fetch') {
    $checked = Test-CargoArguments $CargoArgs $members
    if ($FreshTarget -or $ExistingTarget) { throw 'Fetch does not accept a compilation target selection.' }
    if (-not $DryRun) { Initialize-Directory $cargoHome 'persistent Cargo cache' }
} elseif ($CargoArgs.Count -gt 0 -and $CargoArgs[0] -ceq 'fmt') {
    $checked = Test-CargoArguments $CargoArgs $members
} else {
    Assert-OfflineCache $cargoHome
    $metadataRun = Invoke-Captured (Get-Command cargo -ErrorAction Stop).Source @('metadata','--offline','--locked','--no-deps','--format-version','1') 'workspace-metadata'
    if ($metadataRun.ExitCode -ne 0) { throw "Guarded cargo metadata failed: $($metadataRun.ExitCode). If dependencies are missing, run guarded cargo fetch --locked." }
    $metadata = $metadataRun.Stdout | ConvertFrom-Json
    $members = @($metadata.packages | ForEach-Object name)
    $checked = Test-CargoArguments $CargoArgs $members
}
Set-CargoCommandEnvironment $checked.Args[0]
Assert-WriteRoots
if ($FreshTarget) {
    if (Test-Path -LiteralPath $script:targetRoot) { throw "Metadata created the target that was required to be fresh: $script:targetRoot" }
    Initialize-Directory (Split-Path -Parent $script:targetRoot) 'fresh target parent'
    Initialize-Directory $script:targetRoot 'fresh cargo target'
}
if ($DryRun) { "DRY_RUN category=$($checked.Category) cargo $([string]::Join(' ', [string[]]$checked.Args)) target=$script:targetRoot"; exit 0 }
Assert-WriteRoots
$result = Invoke-Captured (Get-Command cargo -ErrorAction Stop).Source $checked.Args 'cargo-command'
Get-Content -LiteralPath $result.OutPath -Tail 30
Get-Content -LiteralPath $result.ErrPath -Tail 30
"N1_GUARD category=$($checked.Category) exit_code=$($result.ExitCode) stdout=$($result.OutPath) stderr=$($result.ErrPath)"
if ($result.ExitCode -ne 0) { throw "Guarded Cargo command failed with exit code $($result.ExitCode)." }
