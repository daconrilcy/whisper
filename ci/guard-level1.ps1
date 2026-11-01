param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('SelfTest', 'TestList', 'IgnoredList', 'Tests', 'Clippy', 'Fmt', 'Structure')]
    [string]$Mode
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$guardRoot = Join-Path $repo 'target/parakeet-tests/tests-autonomes'
$cargoRoot = 'C:\dev\whisper\target\parakeet-tests\cargo-shared'
$cacheSource = 'C:\dev\whisper\target\parakeet-tests\refacto-parakeet-live\cargo-home'
$cargoHome = Join-Path $guardRoot 'cargo-home'
$tempRoot = Join-Path $guardRoot 'temp'
$scratchRoot = Join-Path $guardRoot 'scratch'
$runRoot = $null
$allowedCargoModes = @('TestList', 'IgnoredList', 'Tests', 'Clippy', 'Fmt')

function Get-FullPath([string]$Path) {
    [IO.Path]::GetFullPath($Path).TrimEnd('\')
}

function Test-InRoot([string]$Path, [string]$Root) {
    $pathFull = Get-FullPath $Path
    $rootFull = Get-FullPath $Root
    return $pathFull.Equals($rootFull, [StringComparison]::OrdinalIgnoreCase) -or
        $pathFull.StartsWith($rootFull + '\', [StringComparison]::OrdinalIgnoreCase)
}

function Assert-NoReparseChain([string]$Path, [string]$Name) {
    $full = Get-FullPath $Path
    $cursor = $full
    while ($true) {
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -LiteralPath $cursor -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "$Name crosses a reparse point: $($item.FullName)"
            }
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

function Get-CargoArguments([string]$SelectedMode) {
    switch ($SelectedMode) {
        'TestList'    { return @('test', '--offline', '--locked', '--workspace', '--', '--list') }
        'IgnoredList' { return @('test', '--offline', '--locked', '--workspace', '--', '--ignored', '--list') }
        'Tests'       { return @('test', '--offline', '--locked', '--workspace') }
        'Clippy'      { return @('clippy', '--offline', '--locked', '--workspace', '--all-targets') }
        'Fmt'         { return @('fmt', '--check') }
        default       { throw "Mode does not run Cargo: $SelectedMode" }
    }
}

function Assert-ModeArguments([string]$SelectedMode, [string[]]$Arguments) {
    $expected = @(Get-CargoArguments $SelectedMode)
    if ($Arguments.Count -ne $expected.Count) { throw "Unexpected Cargo argument count for $SelectedMode" }
    for ($index = 0; $index -lt $expected.Count; $index++) {
        if ($Arguments[$index] -cne $expected[$index]) { throw "Unexpected Cargo arguments for $SelectedMode" }
    }
}

function Invoke-CapturedProcess([string]$Executable, [string[]]$Arguments, [string]$StdoutPath, [string]$StderrPath) {
    foreach ($outputPath in @($StdoutPath, $StderrPath)) {
        if (-not (Test-InRoot $outputPath $runRoot)) { throw "Process output path is outside this run root: $outputPath" }
        Assert-NoReparseChain (Split-Path -Parent $outputPath) 'process output directory'
    }
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Executable
    $startInfo.WorkingDirectory = $repo
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $Arguments) { [void]$startInfo.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    try {
        if (-not $process.Start()) { throw "Could not start process: $Executable" }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        $utf8NoBom = [Text.UTF8Encoding]::new($false)
        [IO.File]::WriteAllText($StdoutPath, $stdout, $utf8NoBom)
        [IO.File]::WriteAllText($StderrPath, $stderr, $utf8NoBom)
        Assert-NoReparseChain $StdoutPath 'process stdout log'
        Assert-NoReparseChain $StderrPath 'process stderr log'
        return [pscustomobject]@{ ExitCode = $process.ExitCode; Stdout = $stdout; Stderr = $stderr }
    } finally {
        $process.Dispose()
    }
}

function Assert-WriteRoots {
    foreach ($path in @($guardRoot, $cargoRoot, $cargoHome, $tempRoot, $scratchRoot)) {
        if (-not (Test-InRoot $path $guardRoot) -and (Get-FullPath $path) -ine (Get-FullPath $cargoRoot)) {
            throw "Write root escaped allowed roots: $path"
        }
        Assert-NoReparseChain $path 'campaign write root'
    }
    if ((Get-FullPath $env:CARGO_TARGET_DIR) -ine (Get-FullPath $cargoRoot)) {
        throw 'CARGO_TARGET_DIR must equal target/parakeet-tests/cargo-shared'
    }
    foreach ($name in @('CARGO_HOME', 'TEMP', 'TMP', 'LOCALAPPDATA')) {
        $value = [Environment]::GetEnvironmentVariable($name)
        if ([string]::IsNullOrWhiteSpace($value)) { throw "$name is required" }
        if (-not (Test-InRoot $value $guardRoot)) { throw "$name is outside campaign root" }
        Assert-NoReparseChain $value $name
    }
    if ((Get-FullPath $env:CARGO_HOME) -ine (Get-FullPath $cargoHome)) { throw 'CARGO_HOME is not isolated' }
    if (-not (Test-InRoot $env:TEMP $tempRoot) -or -not (Test-InRoot $env:TMP $tempRoot)) { throw 'TEMP and TMP must be isolated below temp' }
    if (-not (Test-InRoot $env:LOCALAPPDATA $runRoot)) { throw 'LOCALAPPDATA is outside this run root' }
    if ($env:CARGO_INCREMENTAL -cne '0' -or $env:CARGO_PROFILE_DEV_DEBUG -cne 'line-tables-only') {
        throw 'Cargo profile inputs do not match the level-1 campaign settings'
    }
    if ((Get-PSDrive -Name C).Free -lt 30GB) { throw 'C: has less than 30 GiB free; Cargo is forbidden' }
    $entries = Get-ChildItem -LiteralPath $env:LOCALAPPDATA -Force -ErrorAction Stop
    if ($entries.Count -ne 0) { throw 'LOCALAPPDATA must be an empty isolated directory before every run' }
    foreach ($entry in Get-ChildItem Env:) {
        if ($entry.Name -match '^(WHISPER_|PARAKEET_)') { throw "Campaign variable was not cleared: $($entry.Name)" }
    }
}

function Initialize-Roots {
    if ((Get-PSDrive -Name C).Free -lt 30GB) { throw 'C: has less than 30 GiB free; campaign setup is forbidden' }
    $roots = @($guardRoot, (Split-Path -Parent $cargoRoot), $cargoRoot, $cargoHome, $tempRoot, $scratchRoot)
    foreach ($path in $roots) {
        Assert-NoReparseChain $path 'campaign root preflight'
    }
    if (-not (Test-Path -LiteralPath (Join-Path $cargoHome 'registry/src') -PathType Container)) {
        Assert-NoReparseChain $cacheSource 'offline Cargo cache source'
    }
    foreach ($path in $roots) { Initialize-Directory $path 'campaign root' }
    if (-not (Test-Path -LiteralPath (Join-Path $cargoHome 'registry/src') -PathType Container)) {
        if (-not (Test-Path -LiteralPath (Join-Path $cacheSource 'registry/src') -PathType Container)) {
            throw "Offline cache is unavailable at the approved source: $cacheSource"
        }
        Assert-NoReparseChain $cargoHome 'CARGO_HOME before cache copy'
        Assert-NoReparseChain $cacheSource 'offline Cargo cache source'
        Get-ChildItem -LiteralPath $cacheSource -Force | Copy-Item -Destination $cargoHome -Recurse -Force
    }
    Assert-NoReparseChain $cargoHome 'CARGO_HOME'
}

function Set-CleanEnvironment([string]$SelectedMode) {
    foreach ($entry in @(Get-ChildItem Env: | Where-Object Name -Match '^(WHISPER_|PARAKEET_)')) {
        [Environment]::SetEnvironmentVariable($entry.Name, $null, 'Process')
    }
    $script:runRoot = Join-Path $scratchRoot ((Get-Date -Format 'yyyyMMddTHHmmssfff') + '-' + [guid]::NewGuid().ToString('N'))
    $localAppData = Join-Path $script:runRoot 'localappdata-empty'
    $temp = Join-Path $tempRoot 'temp'
    $tmp = Join-Path $tempRoot 'tmp'
    foreach ($path in @($script:runRoot, $localAppData, $temp, $tmp)) {
        Assert-NoReparseChain $path 'run root preflight'
    }
    Initialize-Directory $script:runRoot 'run root'
    Initialize-Directory $localAppData 'isolated LOCALAPPDATA'
    Initialize-Directory $temp 'isolated TEMP'
    Initialize-Directory $tmp 'isolated TMP'
    foreach ($name in @('CARGO_HOME', 'CARGO_TARGET_DIR', 'CARGO_INCREMENTAL', 'CARGO_PROFILE_DEV_DEBUG', 'TEMP', 'TMP', 'LOCALAPPDATA')) {
        [Environment]::SetEnvironmentVariable($name, $null, 'Process')
    }
    $env:CARGO_HOME = $cargoHome
    $env:CARGO_TARGET_DIR = $cargoRoot
    $env:CARGO_INCREMENTAL = '0'
    $env:CARGO_PROFILE_DEV_DEBUG = 'line-tables-only'
    $env:TEMP = $temp
    $env:TMP = $tmp
    $env:LOCALAPPDATA = $localAppData
    if ($SelectedMode -eq 'Clippy') { $env:RUSTFLAGS = $null }
}

function Invoke-SelfTests {
    foreach ($selected in @('TestList', 'IgnoredList', 'Tests', 'Clippy', 'Fmt')) {
        $arguments = @(Get-CargoArguments $selected)
        Assert-ModeArguments $selected $arguments
    }
    foreach ($invalid in @(
        @('--offline', '--workspace'),
        @('test', '--workspace', '--', '--list', '--ignored'),
        @('test', '--offline', '--locked', '--workspace', '--features', 'native-tests')
    )) {
        $rejected = $false
        try { Assert-ModeArguments 'TestList' $invalid } catch { $rejected = $true }
        if (-not $rejected) { throw 'Self-test accepted an invalid Cargo command' }
    }
    if ($allowedCargoModes.Count -ne 5) { throw 'Self-test failed to enumerate all level-1 Cargo modes' }
    $guardText = Get-Content -LiteralPath $PSCommandPath -Raw
    $watcherPattern = 'FileSystem' + 'Watcher|Register-' + 'ObjectEvent'
    if ($guardText -match $watcherPattern) { throw 'Level-1 guard must not arm file-system monitoring' }
    Assert-WriteRoots
    $pwsh = Join-Path $PSHOME 'pwsh.exe'
    $zeroOut = Join-Path $runRoot 'selftest-zero.stdout.log'
    $zeroErr = Join-Path $runRoot 'selftest-zero.stderr.log'
    $zero = Invoke-CapturedProcess $pwsh @('-NoProfile', '-Command', '[Console]::Out.Write("stdout-marker"); [Console]::Error.Write("stderr-marker"); exit 0') $zeroOut $zeroErr
    if ($zero.ExitCode -ne 0 -or $zero.Stdout -cne 'stdout-marker' -or $zero.Stderr -cne 'stderr-marker') {
        throw 'Self-test failed to capture stdout, stderr and exit code zero under the campaign root'
    }
    $nonzeroOut = Join-Path $runRoot 'selftest-nonzero.stdout.log'
    $nonzeroErr = Join-Path $runRoot 'selftest-nonzero.stderr.log'
    $nonzero = Invoke-CapturedProcess $pwsh @('-NoProfile', '-Command', '[Console]::Out.Write("failure-out"); [Console]::Error.Write("failure-error"); exit 23') $nonzeroOut $nonzeroErr
    if ($nonzero.ExitCode -ne 23 -or $nonzero.Stdout -cne 'failure-out' -or $nonzero.Stderr -cne 'failure-error') {
        throw 'Self-test failed to capture stdout, stderr and nonzero exit code under the campaign root'
    }
    $junctionTarget = Join-Path $runRoot 'reparse-target'
    $junctionPath = Join-Path $runRoot 'reparse-link'
    Initialize-Directory $junctionTarget 'junction self-test target'
    New-Item -ItemType Junction -Path $junctionPath -Target $junctionTarget | Out-Null
    $blockedRoot = Join-Path $junctionPath 'must-not-be-created'
    $junctionRejected = $false
    try { Initialize-Directory $blockedRoot 'junction self-test destination' } catch { $junctionRejected = $true }
    if (-not $junctionRejected) { throw 'Self-test accepted a destination below a junction' }
    if (Test-Path -LiteralPath (Join-Path $junctionTarget 'must-not-be-created')) {
        throw 'Self-test created a directory through a rejected junction'
    }
    $saved = @{}
    try {
        foreach ($name in @('CARGO_TARGET_DIR', 'CARGO_HOME', 'TEMP', 'TMP', 'LOCALAPPDATA')) {
            $saved[$name] = [Environment]::GetEnvironmentVariable($name)
        }
        $env:CARGO_TARGET_DIR = Join-Path $repo 'models'
        $rejected = $false
        try { Assert-WriteRoots } catch { $rejected = $true }
        if (-not $rejected) { throw 'Self-test accepted a Cargo target outside cargo-shared' }
        $env:CARGO_TARGET_DIR = $cargoRoot
        foreach ($name in @('CARGO_HOME', 'TEMP', 'TMP', 'LOCALAPPDATA')) {
            [Environment]::SetEnvironmentVariable($name, (Join-Path $repo 'models'), 'Process')
            $rejected = $false
            try { Assert-WriteRoots } catch { $rejected = $true }
            if (-not $rejected) { throw "Self-test accepted an external $name" }
            [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process')
        }
        $env:WHISPER_N1_SELFTEST = 'forbidden'
        $rejected = $false
        try { Assert-WriteRoots } catch { $rejected = $true }
        if (-not $rejected) { throw 'Self-test accepted a WHISPER_* variable' }
        Remove-Item Env:WHISPER_N1_SELFTEST -ErrorAction SilentlyContinue
    } finally {
        foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process') }
        Remove-Item Env:WHISPER_N1_SELFTEST -ErrorAction SilentlyContinue
    }
    'PASS level-1 arguments are pinned; native-tests and test execution flags are excluded from inventory modes.'
    'PASS level-1 roots, environment inputs and no-watcher rule reject invalid state before Cargo.'
    'PASS explicit process capture preserves stdout, stderr and exit codes 0/23 in run-root logs.'
    'PASS a junction destination is rejected before its target directory is created.'
}

Initialize-Roots
if ($Mode -eq 'SelfTest') {
    Set-CleanEnvironment $Mode
    Invoke-SelfTests
    exit 0
}

Set-CleanEnvironment $Mode
if ($Mode -eq 'Structure') {
    Assert-WriteRoots
    $source = Join-Path $PSScriptRoot 'check-structure.rs'
    $baseline = Join-Path $PSScriptRoot 'tests-baseline.txt'
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw 'ci/check-structure.rs is missing' }
    if (-not (Test-Path -LiteralPath $baseline -PathType Leaf)) { throw 'ci/tests-baseline.txt is missing' }
    $rustc = (Get-Command rustc -ErrorAction Stop).Source
    $checker = Join-Path $runRoot 'check-structure.exe'
    $compileOut = Join-Path $runRoot 'structure-compile.stdout.log'
    $compileErr = Join-Path $runRoot 'structure-compile.stderr.log'
    $compile = Invoke-CapturedProcess $rustc @('--edition=2024', $source, '-o', $checker) $compileOut $compileErr
    if ($compile.ExitCode -ne 0) {
        Get-Content -LiteralPath $compileOut -Tail 40
        Get-Content -LiteralPath $compileErr -Tail 40
        throw "Rust structure checker compilation failed with exit code $($compile.ExitCode); inspect logs under $runRoot"
    }
    $checkerSelfTestRoot = Join-Path $runRoot 'checker-selftest-fixtures'
    Assert-NoReparseChain $checkerSelfTestRoot 'checker self-test fixture root'
    $selfTestOut = Join-Path $runRoot 'structure-selftest.stdout.log'
    $selfTestErr = Join-Path $runRoot 'structure-selftest.stderr.log'
    $selfTest = Invoke-CapturedProcess $checker @('--self-test', $checkerSelfTestRoot) $selfTestOut $selfTestErr
    Get-Content -LiteralPath $selfTestOut -Tail 20
    Get-Content -LiteralPath $selfTestErr -Tail 20
    if ($selfTest.ExitCode -ne 0) {
        throw "Rust structure checker self-tests failed with exit code $($selfTest.ExitCode); inspect logs under $runRoot"
    }
    $checkOut = Join-Path $runRoot 'structure-check.stdout.log'
    $checkErr = Join-Path $runRoot 'structure-check.stderr.log'
    $result = Invoke-CapturedProcess $checker @($baseline) $checkOut $checkErr
    Get-Content -LiteralPath $checkOut -Tail 40
    Get-Content -LiteralPath $checkErr -Tail 40
    "N1_GUARD mode=Structure exit_code=$($result.ExitCode) stdout=$checkOut stderr=$checkErr"
    if ($result.ExitCode -ne 0) { throw "Structure checks failed with exit code $($result.ExitCode); inspect logs under $runRoot" }
    exit 0
}

if ($Mode -notin $allowedCargoModes) { throw "Unknown level-1 mode: $Mode" }
Invoke-SelfTests
$arguments = @(Get-CargoArguments $Mode)
Assert-ModeArguments $Mode $arguments
Assert-WriteRoots
$cargo = (Get-Command cargo -ErrorAction Stop).Source
$stdoutPath = Join-Path $runRoot ($Mode + '.stdout.log')
$stderrPath = Join-Path $runRoot ($Mode + '.stderr.log')
$clock = [Diagnostics.Stopwatch]::StartNew()
$result = Invoke-CapturedProcess $cargo $arguments $stdoutPath $stderrPath
$clock.Stop()
Get-Content -LiteralPath $stdoutPath -Tail 24
Get-Content -LiteralPath $stderrPath -Tail 24
"N1_GUARD mode=$Mode exit_code=$($result.ExitCode) seconds=$([math]::Round($clock.Elapsed.TotalSeconds, 3)) stdout=$stdoutPath stderr=$stderrPath"
if ($result.ExitCode -ne 0) { throw "$Mode failed with exit code $($result.ExitCode); campaign stopped. Inspect captured output before any correction or retry." }
