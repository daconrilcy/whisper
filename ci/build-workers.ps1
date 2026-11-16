param(
    [ValidateSet('All','Cpu','Gpu','ParakeetCpu')]
    [string]$Worker = 'All',
    [string]$LotRoot = 'target/r8-fix',
    [switch]$PortableCpuDistribution
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

if ($PortableCpuDistribution) {
    if ($Worker -cne 'Gpu') { throw '-PortableCpuDistribution requires -Worker Gpu.' }
    if ($LotRoot -cne 'target/parakeet-tests/main-facades') {
        throw '-PortableCpuDistribution requires -LotRoot target/parakeet-tests/main-facades.'
    }
}

function Get-ProcessEnvironmentSnapshot {
    $snapshot = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in [Environment]::GetEnvironmentVariables('Process').GetEnumerator()) {
        $snapshot[[string]$entry.Key] = [string]$entry.Value
    }
    return $snapshot
}

function Restore-ProcessEnvironment($Snapshot) {
    $current = [Environment]::GetEnvironmentVariables('Process')
    foreach ($name in @($current.Keys)) {
        if (-not $Snapshot.ContainsKey([string]$name)) {
            [Environment]::SetEnvironmentVariable([string]$name, $null, 'Process')
        }
    }
    foreach ($entry in $Snapshot.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, 'Process')
    }
}

function Resolve-Ninja {
    $command = Get-Command ninja -ErrorAction SilentlyContinue
    if ($null -ne $command) { return $command.Source }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        throw 'Portable GPU build requires Ninja; it is not on PATH and vswhere.exe is unavailable.'
    }
    $installations = @(& $vswhere -products '*' -property installationPath)
    if ($LASTEXITCODE -ne 0) { throw 'Portable GPU build could not enumerate Visual Studio installations.' }
    foreach ($installation in $installations) {
        $candidate = Join-Path $installation 'Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja/ninja.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    throw 'Portable GPU build requires ninja.exe; no Visual Studio installation contains it.'
}

function Normalize-FullPath([string]$Path) {
    return [IO.Path]::GetFullPath($Path).TrimEnd('\','/').Replace('/', '\')
}

function Test-SelectedCudaCompilerPath([string]$Observed, [string]$Expected) {
    if ([string]::IsNullOrWhiteSpace($Observed) -or [string]::IsNullOrWhiteSpace($Expected)) { return $false }
    try {
        return (Normalize-FullPath $Observed).Equals((Normalize-FullPath $Expected), [StringComparison]::OrdinalIgnoreCase)
    } catch {
        return $false
    }
}

function Find-PortableGpuRustcCommand([string]$BuildLog, [string]$TargetRoot) {
    $expectedRelativeSource = 'crates/whisper-worker-gpu/src/main.rs'
    $expectedSource = Normalize-FullPath (Join-Path $repo $expectedRelativeSource)
    $expectedOutDir = Normalize-FullPath (Join-Path $TargetRoot 'release/deps')
    $running = [regex]::Matches($BuildLog, '(?m)^\s*Running `(?<command>set [^`\r\n]*rustc\.exe[^`\r\n]*)`\s*$')
    foreach ($match in $running) {
        $command = [string]$match.Groups['command'].Value
        if ($command -notmatch '(?i)--crate-name\s+whisper_worker_gpu(?:\s|$)') { continue }
        if ($command -notmatch '(?i)\brustc\.exe\b') { continue }
        if ($command -notmatch '(?i)--crate-type\s+bin(?:\s|$)') { continue }
        $sourceMatch = [regex]::Match($command, '(?i)(?:^|\s)(?<source>(?:"[^"]*main\.rs"|[^\s&]*main\.rs))(?=\s|$)')
        if (-not $sourceMatch.Success) { continue }
        $source = $sourceMatch.Groups['source'].Value.Trim('"').Replace('/', '\')
        if (-not ($source.Equals($expectedRelativeSource, [StringComparison]::OrdinalIgnoreCase) -or
                (Normalize-FullPath (Join-Path $repo $source)).Equals($expectedSource, [StringComparison]::OrdinalIgnoreCase))) { continue }
        $outDir = [regex]::Match($command, '--out-dir\s+(?:"(?<quoted>[^"]+)"|(?<plain>[^\s&]+))')
        if (-not $outDir.Success) { continue }
        $observedOutDir = if ($outDir.Groups['quoted'].Success) { $outDir.Groups['quoted'].Value } else { $outDir.Groups['plain'].Value }
        if (-not (Test-SelectedCudaCompilerPath $observedOutDir $expectedOutDir)) { continue }
        if ($command -notmatch '(?i)(?:^|\s)-C\s+target-cpu=x86-64(?:\s|$)') { continue }
        return $command
    }
    return $null
}

function Get-PortableGpuCacheEntries([string]$CacheText) {
    $entries = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::Ordinal)
    foreach ($line in ($CacheText -split "`r?`n")) {
        if ($line -match '^\s*$' -or $line.StartsWith('#') -or $line.StartsWith('//')) { continue }
        if ($line -cnotmatch '^(?<name>[^:#=]+):(?<type>[^=]+)=(?<value>.*)$') {
            throw 'Portable GPU CMake cache contains a malformed entry.'
        }
        $name = $Matches.name
        if ($entries.ContainsKey($name)) { throw "Portable GPU CMake cache has duplicate entry: $name" }
        $entries.Add($name, [pscustomobject]@{ Type = $Matches.type; Value = $Matches.value })
    }
    return ,$entries
}

function Assert-PortableGpuCmakeEvidence([string]$CacheText, [string]$NinjaText) {
    $entries = Get-PortableGpuCacheEntries $CacheText
    $requirements = [ordered]@{
        CMAKE_GENERATOR = @{ Types = @('INTERNAL','UNINITIALIZED'); Value = 'Ninja' }
        CMAKE_BUILD_TYPE = @{ Types = @('STRING'); Value = 'Release' }
        CMAKE_EXPORT_COMPILE_COMMANDS = @{ Types = @('BOOL','UNINITIALIZED'); Value = 'ON' }
        GGML_CUDA = @{ Types = @('BOOL'); Value = 'ON' }
        GGML_NATIVE = @{ Types = @('BOOL'); Value = 'OFF' }
        GGML_SSE42 = @{ Types = @('BOOL'); Value = 'ON' }
        GGML_AVX = @{ Types = @('BOOL'); Value = 'OFF' }
        GGML_AVX_VNNI = @{ Types = @('BOOL'); Value = 'OFF' }
        GGML_AVX2 = @{ Types = @('BOOL'); Value = 'OFF' }
        GGML_BMI2 = @{ Types = @('BOOL'); Value = 'OFF' }
        GGML_AVX512 = @{ Types = @('BOOL'); Value = 'OFF' }
        GGML_AVX512_VBMI = @{ Types = @('BOOL'); Value = 'OFF' }
        GGML_AVX512_VNNI = @{ Types = @('BOOL'); Value = 'OFF' }
        GGML_AVX512_BF16 = @{ Types = @('BOOL'); Value = 'OFF' }
    }
    foreach ($name in $requirements.Keys) {
        $expected = $requirements[$name]
        if (-not $entries.ContainsKey($name) -or $entries[$name].Type -cnotin $expected.Types -or $entries[$name].Value -cne $expected.Value) {
            throw "Portable GPU CMake cache has missing or invalid entry: $name"
        }
    }
    if ($NinjaText -cnotmatch '(?m)^# Generated by "Ninja" Generator, CMake Version [^\r\n]+\r?$') {
        throw 'Portable GPU build.ninja does not identify the Ninja generator.'
    }
    return ,$entries
}

function Assert-PortableGpuEvidenceSelfTests {
    $fixtureRoot = 'C:\fixture\repo\target\parakeet-tests\main-facades\targets\gpu2'
    $source = 'crates/whisper-worker-gpu/src/main.rs'
    $valid = 'Running `set CARGO=C:\fixture\cargo.exe&& set CARGO_CRATE_NAME=whisper_worker_gpu&& set CARGO_MANIFEST_DIR=C:\fixture\repo\crates\whisper-worker-gpu&& "C:\fixture\rust\rustc.exe" --crate-name whisper_worker_gpu --edition=2024 ' + $source + ' --crate-type bin --out-dir ' + $fixtureRoot + '\release\deps -C target-cpu=x86-64`'
    if (-not (Find-PortableGpuRustcCommand $valid $fixtureRoot)) { throw 'Portable GPU rustc parser rejected a valid Windows Cargo command.' }
    $missingCargoAssignment = $valid.Replace('set CARGO=', 'echo CARGO=')
    if ($missingCargoAssignment -eq $valid) { throw 'Portable GPU missing-CARGO fixture was not modified.' }
    foreach ($invalid in @(
        $valid.Replace('--crate-name whisper_worker_gpu','--crate-name whisper_worker_cpu'),
        $valid.Replace($fixtureRoot,'C:\fixture\repo\target\other'),
        $valid.Replace('-C target-cpu=x86-64','-C target-cpu=native'),
        $valid.Replace('--crate-type bin','--crate-type lib'),
        $missingCargoAssignment
    )) {
        if (Find-PortableGpuRustcCommand $invalid $fixtureRoot) { throw 'Portable GPU rustc parser accepted an invalid Windows Cargo command.' }
    }
    $expectedCuda = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9\bin\nvcc.exe'
    if (-not (Test-SelectedCudaCompilerPath 'C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.9/bin/nvcc.exe' $expectedCuda)) {
        throw 'CUDA path normalization self-test rejected slash-form v12.9.'
    }
    if (Test-SelectedCudaCompilerPath 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.90\bin\nvcc.exe' $expectedCuda) {
        throw 'CUDA path self-test accepted the v12.90 sibling.'
    }
    $cache = @('CMAKE_GENERATOR:INTERNAL=Ninja','CMAKE_BUILD_TYPE:STRING=Release','CMAKE_EXPORT_COMPILE_COMMANDS:BOOL=ON','GGML_CUDA:BOOL=ON','GGML_NATIVE:BOOL=OFF','GGML_SSE42:BOOL=ON','GGML_AVX:BOOL=OFF','GGML_AVX_VNNI:BOOL=OFF','GGML_AVX2:BOOL=OFF','GGML_BMI2:BOOL=OFF','GGML_AVX512:BOOL=OFF','GGML_AVX512_VBMI:BOOL=OFF','GGML_AVX512_VNNI:BOOL=OFF','GGML_AVX512_BF16:BOOL=OFF') -join "`n"
    $ninjaHeader = '# Generated by "Ninja" Generator, CMake Version 4.4'
    [void](Assert-PortableGpuCmakeEvidence $cache $ninjaHeader)
    $uninitialized = $cache.Replace('CMAKE_GENERATOR:INTERNAL=Ninja','CMAKE_GENERATOR:UNINITIALIZED=Ninja').Replace('CMAKE_EXPORT_COMPILE_COMMANDS:BOOL=ON','CMAKE_EXPORT_COMPILE_COMMANDS:UNINITIALIZED=ON')
    [void](Assert-PortableGpuCmakeEvidence $uninitialized $ninjaHeader)
    foreach ($invalidCache in @(
        $cache.Replace('CMAKE_GENERATOR:INTERNAL=Ninja','CMAKE_GENERATOR:INTERNAL=Other'),
        $cache.Replace('CMAKE_EXPORT_COMPILE_COMMANDS:BOOL=ON','CMAKE_EXPORT_COMPILE_COMMANDS:BOOL=OFF'),
        $cache.Replace('CMAKE_EXPORT_COMPILE_COMMANDS:BOOL=ON','CMAKE_EXPORT_COMPILE_COMMANDS:STRING=ON'),
        $cache.Replace('GGML_NATIVE:BOOL=OFF',''),
        ($cache + "`nCMAKE_GENERATOR:INTERNAL=Ninja")
    )) {
        $rejected = $false
        try { [void](Assert-PortableGpuCmakeEvidence $invalidCache $ninjaHeader) } catch { $rejected = $true }
        if (-not $rejected) { throw 'Portable GPU CMake cache self-test accepted invalid evidence.' }
    }
    $rejected = $false
    try { [void](Assert-PortableGpuCmakeEvidence $cache '# Generated by "Other" Generator, CMake Version 4.4') } catch { $rejected = $true }
    if (-not $rejected) { throw 'Portable GPU CMake self-test accepted a non-Ninja header.' }
    Write-Output 'PORTABLE_GPU_EVIDENCE_SELFTEST PASS windows_rustc=1 negatives=5 cuda_path=2'
}

function Invoke-PortableGpuBuild($SelectedWorker) {
    Assert-PortableGpuEvidenceSelfTests
    $baseline = Get-ProcessEnvironmentSnapshot
    try {
        foreach ($entry in @(Get-ChildItem Env: | Where-Object Name -Match '^(CMAKE_|GGML_)')) {
            Remove-Item -LiteralPath "Env:$($entry.Name)" -ErrorAction SilentlyContinue
        }
        foreach ($name in @('CC','CXX','CFLAGS','CXXFLAGS','CL','_CL_','CARGO_BUILD_TARGET','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER')) {
            Remove-Item -LiteralPath "Env:$name" -ErrorAction SilentlyContinue
        }
        foreach ($entry in @(Get-ChildItem Env: | Where-Object Name -Match '^CARGO_TARGET_.*_RUSTFLAGS$')) {
            Remove-Item -LiteralPath "Env:$($entry.Name)" -ErrorAction SilentlyContinue
        }

        $cudaRoot = 'C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9'
        $cudaNvcc = Join-Path $cudaRoot 'bin/nvcc.exe'
        if (-not (Test-Path -LiteralPath $cudaNvcc -PathType Leaf)) {
            throw "Portable GPU build requires CUDA Toolkit 12.9 nvcc at $cudaNvcc."
        }
        $ninja = Resolve-Ninja
        $env:RUSTFLAGS = '-C target-cpu=x86-64'
        $env:CMAKE_GENERATOR = 'Ninja'
        $env:CMAKE_EXPORT_COMPILE_COMMANDS = 'ON'
        $env:CUDA_PATH = $cudaRoot
        $env:PATH = "$(Split-Path -Parent $ninja);$(Join-Path $cudaRoot 'bin');$($baseline['PATH'])"
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
        foreach ($name in $ggmlProfile.Keys) { [Environment]::SetEnvironmentVariable($name, $ggmlProfile[$name], 'Process') }
        if (Test-Path Env:_CL_) { throw 'Portable GPU profile must not inherit _CL_.' }

        $target = 'gpu-l2-final'
        $selectedWorker.Target = $target
        $cargoArgs = @('build','--offline','--locked','--release','-p','whisper-worker-gpu','-vv')
        $cargoArgsJson = ConvertTo-Json -InputObject ([string[]]$cargoArgs) -Compress
        $guardArgs = @('-NoProfile','-File',$guard,'-Mode','Run','-LotRoot',$LotRoot,'-FreshTarget',$target,'-CargoArgsJson',$cargoArgsJson)
        Write-Output "PORTABLE_GPU_PROFILE RUSTFLAGS=$env:RUSTFLAGS CMAKE_GENERATOR=$env:CMAKE_GENERATOR CMAKE_EXPORT_COMPILE_COMMANDS=$env:CMAKE_EXPORT_COMPILE_COMMANDS CUDA_PATH=$env:CUDA_PATH GGML_CUDA=ON (provided by whisper-rs-sys feature)"
        Write-Output "BUILD $($SelectedWorker.Name): cargo $($cargoArgs -join ' ')"
        $guardOutput = @(& $powerShell @guardArgs 2>&1)
        $guardExit = $LASTEXITCODE
        $guardOutput | ForEach-Object { Write-Output $_ }
        if ($guardExit -ne 0) { throw "Portable GPU guarded build failed, exit $guardExit; stopping." }

        $lotPath = [IO.Path]::GetFullPath((Join-Path $repo $LotRoot))
        $targetRoot = Join-Path $lotPath "targets/$target"
        $binary = Join-Path $targetRoot 'release/whisper-worker-gpu.exe'
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Expected portable GPU worker output is missing: $binary" }
        $guardSummary = @($guardOutput | ForEach-Object { [string]$_ } | Where-Object { $_ -match '^N1_GUARD category=Compilation exit_code=0 .* stdout=\S+ stderr=\S+$' } | Select-Object -Last 1)
        if ($guardSummary.Count -ne 1) { throw 'Portable GPU build did not report a successful guarded build log identity.' }
        $stdoutLog = [regex]::Match($guardSummary[0], 'stdout=(\S+)').Groups[1].Value
        $stderrLog = [regex]::Match($guardSummary[0], 'stderr=(\S+)$').Groups[1].Value
        $rustcLog = (Get-Content -LiteralPath $stdoutLog -Raw) + "`n" + (Get-Content -LiteralPath $stderrLog -Raw)
        $rustcCommand = Find-PortableGpuRustcCommand $rustcLog $targetRoot
        if (-not $rustcCommand) {
            throw "Portable GPU Cargo log has no Windows rustc command for whisper-worker-gpu in the fresh target with -C target-cpu=x86-64; stdout=$stdoutLog stderr=$stderrLog"
        }

        $buildRoot = Join-Path $targetRoot 'release/build'
        $sysBuildRoots = @(Get-ChildItem -LiteralPath $buildRoot -Directory -Filter 'whisper-rs-sys-*' |
            ForEach-Object { Join-Path $_.FullName 'out/build' } |
            Where-Object { Test-Path -LiteralPath (Join-Path $_ 'CMakeCache.txt') -PathType Leaf })
        if ($sysBuildRoots.Count -ne 1) { throw "Expected one whisper-rs-sys CMake output under the fresh GPU target; found $($sysBuildRoots.Count)." }
        $sysBuildRoot = [IO.Path]::GetFullPath($sysBuildRoots[0]).TrimEnd('\')
        $compileFile = Join-Path $sysBuildRoot 'compile_commands.json'
        $cacheFile = Join-Path $sysBuildRoot 'CMakeCache.txt'
        if (-not (Test-Path -LiteralPath $compileFile -PathType Leaf)) { throw "Portable GPU build lacks compile_commands.json: $compileFile" }
        $commands = @(Get-Content -LiteralPath $compileFile -Raw | ConvertFrom-Json)
        $hostCommands = 0
        foreach ($entry in $commands) {
            $command = if ($entry.command) { [string]$entry.command } else { [string]::Join(' ', [string[]]$entry.arguments) }
            if ($command -match '(?i)(/arch\s*:\s*AVX\w*|-mavx\w*|-march=native)') {
                throw "Portable GPU C/C++ commands contain a forbidden CPU architecture flag: $command"
            }
            if ($command -match '(?i)cl\.exe' -and $entry.file -match '(?i)(whisper|ggml)[^\\/]*\.(c|cc|cpp)$') {
                $hostCommands++
            }
        }
        if ($hostCommands -eq 0) {
            throw "Portable GPU compile_commands.json has no MSVC commands for whisper.cpp/GGML C/C++ sources: $compileFile"
        }
        $cacheText = Get-Content -LiteralPath $cacheFile -Raw
        $ninjaFile = Join-Path $sysBuildRoot 'build.ninja'
        if (-not (Test-Path -LiteralPath $ninjaFile -PathType Leaf)) { throw "Portable GPU build lacks build.ninja: $ninjaFile" }
        $entries = Assert-PortableGpuCmakeEvidence $cacheText (Get-Content -LiteralPath $ninjaFile -Raw)
        $cacheValues = @{}
        foreach ($name in $entries.Keys) { $cacheValues[$name] = $entries[$name].Value }
        foreach ($compilerName in @('CMAKE_C_COMPILER','CMAKE_CXX_COMPILER')) {
            if ([IO.Path]::GetFileName([string]$cacheValues[$compilerName]) -ine 'cl.exe' -or
                [string]$cacheValues[$compilerName] -notmatch '(?i)HostX64[/\\]x64[/\\]cl\.exe$') {
                throw "Portable GPU $compilerName is not x64 MSVC: $($cacheValues[$compilerName])"
            }
        }
        $cudaCompiler = [string]$cacheValues.CMAKE_CUDA_COMPILER
        $expectedCudaCompiler = Join-Path $cudaRoot 'bin/nvcc.exe'
        if (-not (Test-SelectedCudaCompilerPath $cudaCompiler $expectedCudaCompiler)) {
            throw "Portable GPU CMake selected CUDA compiler $cudaCompiler; expected exact 12.9 compiler $expectedCudaCompiler"
        }
        $cudaArchitectures = @($commands | ForEach-Object {
            $command = if ($_.command) { [string]$_.command } else { [string]::Join(' ', [string[]]$_.arguments) }
            [regex]::Matches($command, '(?i)(?:sm|compute)_(?<arch>\d+)') | ForEach-Object { $_.Groups['arch'].Value }
        } | Sort-Object -Unique)
        if (-not $cudaArchitectures.Count) { throw "Portable GPU compiler commands did not expose a CUDA architecture: $compileFile" }
        Write-Output "PORTABLE_GPU_RUSTC_COMMAND $rustcCommand"
        Write-Output "PORTABLE_GPU_BUILD_EVIDENCE pe=$binary sha256=$((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()) build_stdout=$stdoutLog build_stderr=$stderrLog compile_commands=$compileFile host_commands=$hostCommands cmake_cache=$cacheFile cuda_compiler=$cudaCompiler cuda_cache_architectures=$($cacheValues.CMAKE_CUDA_ARCHITECTURES) nvcc_architectures=$($cudaArchitectures -join ',')"

        Write-Output "PE CHECK $($SelectedWorker.Name): $binary"
        & $powerShell -NoProfile -File $checker -Kind Gpu -Path $binary
        if ($LASTEXITCODE -ne 0) { throw "GPU PE import gate failed, exit $LASTEXITCODE." }
        Write-Output "PORTABLE_GPU_IMPORTS sha256=$((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant())"
    } finally {
        Restore-ProcessEnvironment $baseline
    }
}

foreach ($selectedWorker in $workers) {
    if ($PortableCpuDistribution) {
        Invoke-PortableGpuBuild $selectedWorker
        continue
    }
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
