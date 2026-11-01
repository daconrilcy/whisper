param(
    [ValidateSet('All','Cpu','Gpu','ParakeetCpu')]
    [string]$Worker = 'All',
    [string]$LotRoot = 'target/r8-fix'
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$guard = Join-Path $PSScriptRoot 'guard-level1.ps1'
$checker = Join-Path $PSScriptRoot 'check-worker-imports.ps1'
$powerShell = (Get-Process -Id $PID).Path
$workers = @(
    [pscustomobject]@{ Name = 'whisper-worker-cpu'; Kind = 'Cpu'; Target = 'worker-cpu-release' },
    [pscustomobject]@{ Name = 'whisper-worker-gpu'; Kind = 'Gpu'; Target = 'worker-gpu-release' },
    [pscustomobject]@{ Name = 'whisper-worker-parakeet-cpu'; Kind = 'ParakeetCpu'; Target = 'worker-parakeet-cpu-personal'; Features = @('parakeet-personal'); OutputName = 'whisper-worker-parakeet-cpu.exe' },
    [pscustomobject]@{ Name = 'whisper-worker-parakeet-cpu'; Kind = 'ParakeetCpu'; Target = 'worker-parakeet-gpu-personal'; Features = @('parakeet-personal','parakeet-cuda'); OutputName = 'whisper-worker-parakeet-gpu.exe' }
)
if ($Worker -ne 'All') { $workers = @($workers | Where-Object { $_.Kind -eq $Worker }) }
if (-not $workers.Count) { throw "No worker selected: $Worker" }

foreach ($selectedWorker in $workers) {
    $cargoArgs = @('build','--offline','--locked','--release','-p',$selectedWorker.Name)
    if ($selectedWorker.Features) { $cargoArgs += @('--features',($selectedWorker.Features -join ',')) }
    $cargoArgsJson = ConvertTo-Json -InputObject ([string[]]$cargoArgs) -Compress
    $guardArgs = @('-NoProfile','-File',$guard,'-Mode','Run','-LotRoot',$LotRoot,'-FreshTarget',$selectedWorker.Target,'-CargoArgsJson',$cargoArgsJson)
    Write-Output "BUILD $($selectedWorker.Name): cargo $($cargoArgs -join ' ')"
    & $powerShell @guardArgs
    $guardExit = $LASTEXITCODE
    if ($guardExit -ne 0) { throw "Guarded build failed for $($selectedWorker.Name), exit $guardExit; stopping before the next worker." }
    $lotPath = if ([IO.Path]::IsPathRooted($LotRoot)) { [IO.Path]::GetFullPath($LotRoot) } else { [IO.Path]::GetFullPath((Join-Path $repo $LotRoot)) }
    $targetRoot = Join-Path $lotPath "targets/$($selectedWorker.Target)/release"
    $binary = Join-Path $targetRoot "$($selectedWorker.Name).exe"
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Expected worker output is missing: $binary" }
    Write-Output "PE CHECK $($selectedWorker.Name): $binary"
    & $powerShell -NoProfile -File $checker -Kind $selectedWorker.Kind -Path $binary
    $checkExit = $LASTEXITCODE
    if ($checkExit -ne 0) { throw "PE import gate failed for $($selectedWorker.Name), exit $checkExit." }
    if ($selectedWorker.OutputName -and $selectedWorker.OutputName -cne [IO.Path]::GetFileName($binary)) {
        $namedOutput = Join-Path $targetRoot $selectedWorker.OutputName
        Copy-Item -LiteralPath $binary -Destination $namedOutput
        & $powerShell -NoProfile -File $checker -Kind $selectedWorker.Kind -Path $namedOutput
        $checkExit = $LASTEXITCODE
        if ($checkExit -ne 0) { throw "PE import gate failed for named output $namedOutput, exit $checkExit." }
    }
}
