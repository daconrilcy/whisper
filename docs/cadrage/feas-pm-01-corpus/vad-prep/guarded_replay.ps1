$ErrorActionPreference = 'Stop'
$lot = 'C:\dev\whisper\target\parakeet-tests\feas-pm-01-vad-prep'
$shared = 'C:\dev\whisper\target\parakeet-tests\cargo-shared'
$repo = 'C:\dev\whisper'

function In-Root([string]$path, [string]$root) {
    $p = [IO.Path]::GetFullPath($path).TrimEnd('\')
    $r = [IO.Path]::GetFullPath($root).TrimEnd('\')
    return $p.Equals($r, [StringComparison]::OrdinalIgnoreCase) -or
        $p.StartsWith($r + '\', [StringComparison]::OrdinalIgnoreCase)
}

if ((In-Root 'C:\dev\whisper\docs' $lot) -or (In-Root 'C:\Users\cyril\AppData\Local\Temp' $lot)) {
    throw 'guard self-test failed: an outside path was admitted'
}
$tempRoot = Join-Path $lot 'tmp'
$cargoHome = Join-Path $lot 'cargo-home'
$source = Join-Path $lot 'vad_probe.rs'
$output = Join-Path $shared 'debug\feas_pm_01_vad_probe_frames.exe'
$rlib = Join-Path $shared 'debug\deps\libwebrtc_vad-b7f6f18617313723.rlib'
foreach ($path in @($tempRoot, $cargoHome, $source)) {
    if (-not (In-Root $path $lot)) { throw "out-of-lot path: $path" }
}
foreach ($path in @($output, $rlib)) {
    if (-not (In-Root $path $shared)) { throw "out-of-shared path: $path" }
}
foreach ($path in @($lot, $shared, $tempRoot, $cargoHome)) {
    $item = Get-Item -LiteralPath $path
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "reparse root: $path"
    }
}

$env:TEMP = $tempRoot
$env:TMP = $tempRoot
$env:TMPDIR = $tempRoot
$env:RUSTC_TMPDIR = $tempRoot
$env:CARGO_HOME = $cargoHome
$env:CARGO_TARGET_DIR = $shared
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_DEV_DEBUG = 'line-tables-only'
$env:PYTHONDONTWRITEBYTECODE = '1'

$watchers = @()
$sourceIds = @()
$rootsToWatch = @($repo, 'C:\Users\cyril\.cargo', 'C:\Users\cyril\.rustup', 'C:\Users\cyril\AppData\Local\Temp')
try {
    for ($i = 0; $i -lt $rootsToWatch.Count; $i++) {
        $path = $rootsToWatch[$i]
        if (-not (Test-Path -LiteralPath $path)) { continue }
        $watcher = [IO.FileSystemWatcher]::new($path)
        $watcher.IncludeSubdirectories = $true
        $watcher.NotifyFilter = [IO.NotifyFilters]'FileName, DirectoryName, LastWrite, CreationTime, Size'
        $watcher.InternalBufferSize = 65536
        foreach ($eventName in @('Created', 'Changed', 'Renamed', 'Error')) {
            $id = "feas-vad-$i-$eventName"
            Register-ObjectEvent -InputObject $watcher -EventName $eventName -SourceIdentifier $id | Out-Null
            $sourceIds += $id
        }
        $watcher.EnableRaisingEvents = $true
        $watchers += $watcher
    }
    rustc --edition=2021 -C debuginfo=0 -C opt-level=1 --extern "webrtc_vad=$rlib" -L "dependency=$(Join-Path $shared 'debug\deps')" -o $output $source
    if ($LASTEXITCODE -ne 0) { throw "rustc failed: $LASTEXITCODE" }
    python (Join-Path $lot 'run_probe.py') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "probe failed: $LASTEXITCODE" }
    python (Join-Path $lot 'analyze_probe.py') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "analysis failed: $LASTEXITCODE" }
    Start-Sleep -Milliseconds 300
    $events = @(Get-Event | Where-Object { $_.SourceIdentifier -like 'feas-vad-*' })
    $outside = @()
    foreach ($event in $events) {
        if ($event.SourceIdentifier -like '*-Error') {
            $outside += "watcher error: $($event.SourceIdentifier)"
            continue
        }
        $path = $event.SourceEventArgs.FullPath
        if (-not (In-Root $path $lot) -and -not (In-Root $path $shared)) {
            $outside += $path
        }
    }
    "observed_events=$($events.Count) outside_events=$($outside.Count)"
    if ($outside.Count -gt 0) {
        $outside | Sort-Object -Unique
        throw 'out-of-root write observed; campaign stopped'
    }
    "raw_sha256=$((Get-FileHash -LiteralPath (Join-Path $lot 'vad_raw.json') -Algorithm SHA256).Hash)"
    "analysis_sha256=$((Get-FileHash -LiteralPath (Join-Path $lot 'vad_analysis.json') -Algorithm SHA256).Hash)"
} finally {
    foreach ($id in $sourceIds) {
        Unregister-Event -SourceIdentifier $id -ErrorAction SilentlyContinue
        Get-Event -SourceIdentifier $id -ErrorAction SilentlyContinue | Remove-Event -ErrorAction SilentlyContinue
    }
    foreach ($watcher in $watchers) { $watcher.Dispose() }
}
