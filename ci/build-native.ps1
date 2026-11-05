#requires -Version 7.0
[CmdletBinding(DefaultParameterSetName='Build', PositionalBinding=$false)]
param(
    [Parameter(Mandatory, ParameterSetName='Build')][ValidateSet('cpu','gpu')][string]$Variant,
    [Parameter(ParameterSetName='Build')][ValidatePattern('^[a-zA-Z0-9][a-zA-Z0-9_-]{0,31}$')][string]$BuildId = ('build-' + [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss')),
    [Parameter(ParameterSetName='Build')][switch]$CheckOnly,
    [Parameter(ParameterSetName='Build')][switch]$PrepareOnly,
    [Parameter(Mandatory, ParameterSetName='SelfTest')][switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$ProgressPreference = 'SilentlyContinue'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\')
$configPath = Join-Path $repo 'native/build-config.json'
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json -AsHashtable
$cache = Join-Path $repo 'native/cache'
$utf8 = [Text.UTF8Encoding]::new($false)
. (Join-Path $PSScriptRoot 'native-workspace.ps1')

function Assert-Within([string]$Path, [string]$Root) {
    $full = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    $base = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    if (-not $full.StartsWith($base + '\', [StringComparison]::OrdinalIgnoreCase)) { throw "Path outside allowed root: $full (root $base)" }
    $cursor = $full
    while ($cursor) {
        if (Test-Path -LiteralPath $cursor) {
            if ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Reparse point refused: $cursor" }
        }
        $parent = Split-Path -Parent $cursor
        if ($parent -eq $cursor) { break }
        $cursor = $parent
    }
    return $full
}

function Assert-Version([string]$Name, [string]$Actual, [string]$Expected) {
    if ($Actual -cne $Expected) { throw "$Name version differs: expected $Expected, found $Actual. No automatic update or fallback." }
}

function Save-BuildManifest([Collections.IDictionary]$Value, [string]$Path) {
    [void](Assert-Within $Path $run)
    $pending = $Path + '.pending'
    [IO.File]::WriteAllText($pending, (($Value | ConvertTo-Json -Depth 12) + "`n"), $utf8)
    [IO.File]::Move($pending, $Path, $true)
}

function Clear-VcpkgEnvironment {
    foreach ($entry in Get-ChildItem Env: | Where-Object { $_.Name -match '^(X_)?VCPKG_' }) {
        Remove-Item -LiteralPath ('Env:' + $entry.Name)
    }
}

function Assert-ExecutableDigest([string]$Actual, [string]$Expected) {
    if ($Actual -ine $Expected -or $Actual -notmatch '^[0-9a-fA-F]{64}$') { throw 'vcpkg.exe SHA-256 differs from the pinned release digest; execution refused.' }
}

function Assert-VcpkgExecutable {
    $metadata = Get-Content -LiteralPath (Join-Path $vcpkg 'scripts/vcpkg-tool-metadata.txt') -Raw | ConvertFrom-StringData
    Assert-Version 'vcpkg bootstrap release metadata' $metadata.VCPKG_TOOL_RELEASE_TAG $config.vcpkgExecutable.release
    $path = Assert-Within (Join-Path $vcpkg 'vcpkg.exe') $workspace
    $digest = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    Assert-ExecutableDigest $digest $config.vcpkgExecutable.sha256
    return $digest
}

function New-NativeBuildOptions([hashtable]$Base, [string]$BuildVariant) {
    $options = @{} + $Base
    foreach ($key in $options.Keys) {
        if ($key -match '^CMAKE_(C|CXX|CUDA|SHARED_LINKER|EXE_LINKER)_FLAGS(?:_(DEBUG|RELEASE|RELWITHDEBINFO|MINSIZEREL))?$') {
            throw "Full flag override refused: $key. Add determinism through *_FLAGS_INIT to preserve platform defaults."
        }
    }
    foreach ($key in @('CMAKE_C_FLAGS_INIT','CMAKE_CXX_FLAGS_INIT','CMAKE_SHARED_LINKER_FLAGS_INIT','CMAKE_EXE_LINKER_FLAGS_INIT')) {
        if ($options[$key] -cne '/Brepro') { throw "Expected additive /Brepro initializer: $key" }
    }
    if ($BuildVariant -eq 'gpu') { $options.CMAKE_CUDA_FLAGS_INIT='-Xcompiler=/Brepro' }
    elseif ($BuildVariant -ne 'cpu') { throw 'Native build options require cpu or gpu.' }
    return $options
}

function Assert-NativeEffectiveFlags([Collections.IDictionary]$Effective, [string]$BuildVariant) {
    $requirements = [ordered]@{
        CMAKE_C_FLAGS=@('/Brepro','/DWIN32','/D_WINDOWS')
        CMAKE_CXX_FLAGS=@('/Brepro','/DWIN32','/D_WINDOWS','/EHsc')
        CMAKE_SHARED_LINKER_FLAGS=@('/Brepro','/machine:x64')
        CMAKE_EXE_LINKER_FLAGS=@('/Brepro','/machine:x64')
    }
    if ($BuildVariant -eq 'gpu') { $requirements.CMAKE_CUDA_FLAGS=@('/Brepro','/DWIN32','/D_WINDOWS','/EHsc') }
    elseif ($BuildVariant -ne 'cpu') { throw 'Effective native flag checks require cpu or gpu.' }
    $checked = [ordered]@{}
    foreach ($key in $requirements.Keys) {
        if (-not $Effective.Contains($key) -or -not $Effective.Contains($key+'_RELEASE')) { throw "Effective flags missing: $key or ${key}_RELEASE" }
        $flags=([string]$Effective[$key]+' '+[string]$Effective[$key+'_RELEASE']).Trim()
        # CUDA host options are quoted/comma-separated inside -Xcompiler. Never match partial flags.
        $flat=if ($key -eq 'CMAKE_CUDA_FLAGS') { $flags.Replace('-Xcompiler=','') } else { $flags }
        $tokens=@($flat.Replace('"','').Split([char[]]" `t`r`n,",[StringSplitOptions]::RemoveEmptyEntries) | ForEach-Object { $_ -replace '^-','/' })
        foreach ($token in $tokens) {
            if (($key -in @('CMAKE_CXX_FLAGS','CMAKE_CUDA_FLAGS') -and $token.StartsWith('/EH',[StringComparison]::OrdinalIgnoreCase) -and $token -cne '/EHsc') -or
                $token -ieq '/Brepro-' -or $token -imatch '^/U(?:WIN32|_WINDOWS)$' -or
                ($token -imatch '^/machine:' -and $token -ine '/machine:x64')) {
                throw "Conflicting effective native flag in ${key}: $token"
            }
        }
        foreach ($required in $requirements[$key]) {
            if ($required -notin $tokens) { throw "Required effective native flag missing in ${key}: $required" }
        }
        $checked[$key]=$flags
    }
    return [ordered]@{status='PASS';variant=$BuildVariant;checkedFlags=$checked;diagnosticPolicy='C4530 fails during streaming; no automatic retry'}
}

function Assert-NativeDiagnostic([string]$Line) {
    if ($Line -match '\bC4530\b') { throw 'C4530 detected: C++ exception-unwinding semantics are missing. Stop; no automatic retry.' }
}

function Initialize-NativeProcessHelpers {
    if ($script:NativeProcessHelpersLoaded) { return }
    $source=Join-Path $PSScriptRoot 'guard-native.ps1'
    [void](Assert-Within $source $repo)
    $tokens=$null; $parseErrors=$null
    $ast=[Management.Automation.Language.Parser]::ParseFile($source,[ref]$tokens,[ref]$parseErrors)
    if ($parseErrors.Count) { throw 'Cannot load the existing process-family controls: guard parse failed.' }
    # Import only these versioned function definitions; never execute the guard entry point.
    foreach ($name in @('New-VoxPopuliProcessFamily','Assert-VoxPopuliCleanupDeadline','Update-VoxPopuliProcessFamily',
        'Stop-VoxPopuliKnownProcesses','Stop-VoxPopuliProcessFamily','Close-VoxPopuliProcessFamily')) {
        $definitions=@($ast.EndBlock.Statements | Where-Object { $_ -is [Management.Automation.Language.FunctionDefinitionAst] -and $_.Name -ceq $name })
        if ($definitions.Count -ne 1) { throw "Existing process-family helper missing or duplicated: $name" }
        $definition=$definitions[0].Extent.Text.Replace("function $name(","function script:${name}(")
        . ([scriptblock]::Create($definition))
    }
    $script:NativeProcessHelpersSha256=(Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
    $script:NativeProcessHelpersLoaded=$true
}

function Invoke-Logged([string]$Program, [string[]]$Arguments, [string]$Label) {
    Initialize-NativeProcessHelpers
    [IO.File]::AppendAllText((Join-Path $run 'commands.log'), ($Program + ' ' + ($Arguments | ConvertTo-Json -Compress) + "`n"), $utf8)
    $log = Join-Path $run ($Label + '.log')
    $psi=[Diagnostics.ProcessStartInfo]::new($Program)
    $psi.UseShellExecute=$false; $psi.CreateNoWindow=$true
    $psi.WorkingDirectory=if ($workspace) { $workspace } else { $run }
    $psi.RedirectStandardOutput=$true; $psi.RedirectStandardError=$true
    foreach ($argument in $Arguments) { $psi.ArgumentList.Add($argument) }
    $process=[Diagnostics.Process]::new(); $process.StartInfo=$psi
    $writer=[IO.StreamWriter]::new($log,$false,$utf8); $writer.AutoFlush=$true
    $started=$false; $streamDeadline=$null; $family=$null
    try {
        if (-not $process.Start()) { throw "Could not start $Label." }
        $started=$true
        $family=New-VoxPopuliProcessFamily $process
        $streams=@(
            @{reader=$process.StandardOutput;task=$process.StandardOutput.ReadLineAsync();done=$false},
            @{reader=$process.StandardError;task=$process.StandardError.ReadLineAsync();done=$false}
        )
        while (-not ($streams[0].done -and $streams[1].done -and $process.HasExited)) {
            Update-VoxPopuliProcessFamily $family
            $received=$false
            foreach ($stream in $streams) {
                # Bounded batches keep stderr diagnostics responsive under a busy stdout.
                for ($batch=0; $batch -lt 64 -and -not $stream.done -and $stream.task.IsCompleted; $batch++) {
                    $line=$stream.task.GetAwaiter().GetResult()
                    if ($null -eq $line) { $stream.done=$true; break }
                    $received=$true; $writer.WriteLine($line)
                    Assert-NativeDiagnostic $line
                    if ($line -match '^-- |^\[\d+/\d+\]|error|Error|FAILED|Installing|Downloading|Building package') { Write-Host $line }
                    $stream.task=$stream.reader.ReadLineAsync()
                }
            }
            if ($process.HasExited) {
                if ($null -eq $streamDeadline) {
                    Update-VoxPopuliProcessFamily $family -Force
                    if (@($family.entries.Values | Where-Object { -not $_.process.HasExited }).Count) { throw "$Label exited while an observed descendant remained alive." }
                    $streamDeadline=[DateTimeOffset]::UtcNow.AddSeconds(10)
                }
                if ([DateTimeOffset]::UtcNow -ge $streamDeadline) { throw "$Label output streams did not close within ten seconds." }
            }
            if (-not $received) { Start-Sleep -Milliseconds 10 }
        }
        $process.WaitForExit(); $code=$process.ExitCode
        Update-VoxPopuliProcessFamily $family -Force
        if (@($family.entries.Values | Where-Object { -not $_.process.HasExited }).Count) { throw "$Label exited while an observed descendant remained alive." }
        [IO.File]::AppendAllText((Join-Path $run 'commands.log'), "exit=$code label=$Label`n", $utf8)
        if ($code -ne 0) { throw "$Label failed (exit $code); see $log. Stop; no automatic retry." }
    } catch {
        [IO.File]::AppendAllText((Join-Path $run 'commands.log'), "pipeline_failed label=$Label message=$($_.Exception.Message)`n", $utf8)
        throw
    } finally {
        try {
            if ($null -ne $family) {
                try { Stop-VoxPopuliProcessFamily $family }
                finally {
                    try {
                        $receipt=[ordered]@{label=$Label;helperSourceSha256=$script:NativeProcessHelpersSha256
                            cleanupClosed=$family.cleanupClosed;cleanupPasses=$family.cleanupPasses
                            entries=@(foreach($entry in $family.entries.GetEnumerator()) {
                                [pscustomobject]@{pid=$entry.Key;createdUtc=$entry.Value.createdUtc.ToString('o');exited=$entry.Value.process.HasExited}
                            })}
                        [IO.File]::AppendAllText((Join-Path $run 'process-control.jsonl'),(($receipt|ConvertTo-Json -Depth 6 -Compress)+"`n"),$utf8)
                    } finally { Close-VoxPopuliProcessFamily $family }
                }
            } elseif ($started -and -not $process.HasExited) {
                $process.Kill($true)
                if (-not $process.WaitForExit(5000)) { throw "$Label owned process did not stop after interruption." }
            }
        } finally { $writer.Dispose(); $process.Dispose() }
    }
}

function Assert-Source([string]$Path, [hashtable]$Pin) {
    [void](Assert-Within $Path $cache)
    [void](Assert-Within (Join-Path $Path '.git') $cache)
    if (-not (Test-Path -LiteralPath (Join-Path $Path '.git') -PathType Container)) { throw "Expected independent Git checkout missing: $Path" }
    $head = & $git --no-optional-locks -C $Path rev-parse HEAD
    if ($LASTEXITCODE -ne 0 -or $head -cne $Pin.commit) { throw "Source revision differs at $Path (expected $($Pin.commit), found $head)" }
    $origin = & $git --no-optional-locks -C $Path remote get-url origin
    if ($LASTEXITCODE -ne 0 -or $origin -cne $Pin.url) { throw "Source origin differs at $Path" }
    $dirty = @(& $git --no-optional-locks -C $Path status --porcelain --untracked-files=no)
    if ($LASTEXITCODE -ne 0 -or $dirty.Count) { throw "Tracked source modifications refused at $Path" }
    # A source symlink can escape the cache when downstream CMake walks the tree.
    $links = @(& $git --no-optional-locks -C $Path ls-files --stage | Where-Object { $_ -match '^120000 ' })
    if ($LASTEXITCODE -ne 0 -or $links.Count) { throw "Source symlinks refused at $Path" }
}

function Get-Source([string]$Name, [string]$Path, [hashtable]$Pin) {
    [void](Assert-Within $Path $cache)
    if (Test-Path -LiteralPath $Path) {
        if (Test-Path -LiteralPath (Join-Path $Path '.git')) {
            Assert-Source $Path $Pin
            return
        }
        Assert-EmptySourceDirectory $Path
    }
    Invoke-Logged $git @('init', $Path) "$Name-init"
    Invoke-Logged $git @('-C', $Path, 'remote', 'add', 'origin', $Pin.url) "$Name-origin"
    Invoke-Logged $git @('-c','gc.auto=0','-C',$Path,'fetch','--depth=1','origin',$Pin.commit) "$Name-fetch"
    Invoke-Logged $git @('-c','core.autocrlf=false','-c',"core.hooksPath=$cache/empty-hooks",'-C',$Path,'checkout','--detach',$Pin.commit) "$Name-checkout"
    Assert-Source $Path $Pin
}

function Assert-EmptySourceDirectory([string]$Path) {
    [void](Assert-Within $Path $cache)
    if (-not (Test-Path -LiteralPath $Path -PathType Container) -or @(Get-ChildItem -LiteralPath $Path -Force).Count) {
        throw "Source directory exists without Git metadata and is not empty: $Path. Nothing removed."
    }
}

function Assert-GuardContext {
    $guardPid = $env:WHISPER_NATIVE_BUILD_GUARD_PID
    if ($guardPid -notmatch '^[1-9][0-9]*$' -or $env:WHISPER_NATIVE_BUILD_VARIANT -cne $Variant -or $env:WHISPER_NATIVE_BUILD_ID -cne $BuildId) {
        throw 'Invalid native build guard context; use the public build command.'
    }
    $self = Get-CimInstance Win32_Process -Filter "ProcessId=$PID" -Property ParentProcessId -ErrorAction Stop
    if ([int]$self.ParentProcessId -ne [int]$guardPid) { throw 'Native build guard is not the actual parent process.' }
    $parent = Get-CimInstance Win32_Process -Filter "ProcessId=$guardPid" -Property CommandLine -ErrorAction Stop
    $guardScript = Join-Path $PSScriptRoot 'guard-native.ps1'
    if (-not ([string]$parent.CommandLine).Contains($guardScript,[StringComparison]::OrdinalIgnoreCase)) { throw 'Native build parent is not the pinned guard script.' }
    if ($env:WHISPER_NATIVE_BUILD_SCRIPT_SHA256 -ine (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash) { throw 'Native build script changed after guard launch.' }
}

function Get-Tools {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    $vs = @(& $vswhere -version '[17.0,18.0)' -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
    if ($LASTEXITCODE -ne 0 -or $vs.Count -ne 1) { throw 'VS2022 C++ Build Tools not found. Installation is not automatic.' }
    $vc = Join-Path $vs[0] "VC/Tools/MSVC/$($config.tools.msvcToolset)/bin/Hostx64/x64"
    $tools = [ordered]@{
        vs=$vs[0]; vc=$vc
        cmake=(Join-Path $env:ProgramFiles 'CMake/bin/cmake.exe')
        ninja=(Join-Path $vs[0] 'Common7/IDE/CommonExtensions/Microsoft/CMake/Ninja/ninja.exe')
        cl=(Join-Path $vc 'cl.exe'); dumpbin=(Join-Path $vc 'dumpbin.exe')
        vcvars=(Join-Path $vs[0] 'VC/Auxiliary/Build/vcvarsall.bat')
        bootstrapPowerShell=(Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe')
        cuda=(Join-Path $env:ProgramFiles "NVIDIA GPU Computing Toolkit/CUDA/$($config.tools.cudaDirectory)")
    }
    foreach ($name in @('cmake','ninja','cl','dumpbin','vcvars','bootstrapPowerShell')) {
        if (-not (Test-Path -LiteralPath $tools[$name] -PathType Leaf)) { throw "Missing tool: $($tools[$name])" }
    }
    $tools.bootstrapPowerShellVersion=(Get-Item -LiteralPath $tools.bootstrapPowerShell).VersionInfo.FileVersion
    $cmakeVersion = (& $tools.cmake --version | Select-Object -First 1) -replace '^cmake version ', ''
    Assert-Version 'CMake' $cmakeVersion $config.tools.cmake
    $ninjaVersion = & $tools.ninja --version
    Assert-Version 'Ninja (authorized assumption)' $ninjaVersion $config.tools.ninja
    $clBanner = (& $tools.cl 2>&1 | Out-String)
    if ($clBanner -notmatch ('\b' + [regex]::Escape($config.tools.cl) + '\b')) { throw "MSVC compiler version differs: $clBanner" }
    $sdk = Join-Path ${env:ProgramFiles(x86)} "Windows Kits/10/Include/$($config.tools.sdk)/ucrt"
    if (-not (Test-Path -LiteralPath $sdk -PathType Container)) { throw "Historical SDK not installed: $sdk" }
    if ($Variant -eq 'gpu') {
        $nvcc = Join-Path $tools.cuda 'bin/nvcc.exe'
        if (-not (Test-Path -LiteralPath $nvcc -PathType Leaf)) { throw "CUDA compiler missing: $nvcc" }
        $nvccBanner = (& $nvcc --version | Out-String)
        if ($nvccBanner -notmatch ('\bV' + [regex]::Escape($config.tools.nvcc) + '\b')) { throw "CUDA compiler version differs: $nvccBanner" }
        $tools.nvcc = $nvcc
    }
    return $tools
}

function Import-BuildEnvironment([hashtable]$Tools) {
    # VsDevCmd's documented-in-script switch suppresses its detached telemetry
    # sender, not compiler diagnostics or any NativeBuild notification rule.
    $env:VSCMD_SKIP_SENDTELEMETRY='1'
    $environmentCommand = '"' + $Tools.vcvars + '" x64 ' + $config.tools.sdk + ' -vcvars_ver=14.44 >nul && set'
    $lines = & $env:ComSpec /d /s /c $environmentCommand
    if ($LASTEXITCODE -ne 0) { throw 'MSVC environment initialization failed.' }
    foreach ($line in $lines) {
        if ($line -match '^([^=]+)=(.*)$') { [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process') }
    }
    Assert-Version 'SDK selected by vcvars' $env:WindowsSDKVersion.TrimEnd('\') $config.tools.sdk
    Assert-Version 'MSVC toolset selected by vcvars' $env:VCToolsVersion.TrimEnd('\') $config.tools.msvcToolset
    $env:PATH = ($Tools.vc, (Split-Path $Tools.ninja), (Split-Path $Tools.cmake), $env:PATH) -join ';'
    foreach ($name in @('CC','CXX','CFLAGS','CXXFLAGS','CL','_CL_','LINK','_LINK_','CMAKE_ARGS','CMAKE_PREFIX_PATH','CMAKE_TOOLCHAIN_FILE','SOURCE_DATE_EPOCH')) {
        Remove-Item -LiteralPath ('Env:' + $name) -ErrorAction SilentlyContinue
    }
    foreach ($entry in Get-ChildItem Env: | Where-Object { $_.Name -match '^(GGML_|NEMO_|WHISPER_|PARAKEET_|CUDA_PATH)' }) {
        Remove-Item -LiteralPath ('Env:' + $entry.Name)
    }
    Clear-VcpkgEnvironment
    foreach ($entry in @{TEMP='temp';TMP='temp';LOCALAPPDATA='localappdata';APPDATA='appdata';HOME='home';USERPROFILE='userprofile';XDG_CACHE_HOME='xdg';GNUPGHOME='gnupg'}.GetEnumerator()) {
        $path = Assert-Within (Join-Path $workspace $entry.Value) $workspace
        New-Item -ItemType Directory -Path $path -Force | Out-Null
        [Environment]::SetEnvironmentVariable($entry.Key, $path, 'Process')
    }
    foreach ($entry in @{VCPKG_DOWNLOADS='downloads';VCPKG_DEFAULT_BINARY_CACHE='binary-cache';X_VCPKG_REGISTRIES_CACHE='registries'}.GetEnumerator()) {
        $path = Assert-Within (Join-Path $cache $entry.Value) $cache
        New-Item -ItemType Directory -Path $path -Force | Out-Null
        [Environment]::SetEnvironmentVariable($entry.Key, $path, 'Process')
    }
    $env:VCPKG_ROOT = $vcpkg
    # Rebuild dependencies at the fixed path; old binary caches embed other paths.
    $env:VCPKG_BINARY_SOURCES = 'clear'
    $env:VCPKG_VISUAL_STUDIO_PATH = $Tools.vs
    $env:VCPKG_DISABLE_METRICS = '1'
    $env:VCPKG_MAX_CONCURRENCY = '4'
    $env:CMAKE_BUILD_PARALLEL_LEVEL = '4'
    if ($Variant -eq 'gpu') {
        $env:CUDA_PATH = $Tools.cuda
        $env:CUDA_PATH_V12_9 = $Tools.cuda
        $env:PATH = (Join-Path $Tools.cuda 'bin') + ';' + $env:PATH
    }
}

if ($SelfTest) {
    $rejects = 0
    foreach ($case in @(@{p=(Join-Path $repo '../outside');r=$repo}, @{p=$repo;r=$repo}, @{p=($repo+'-other/child');r=$repo})) {
        $rejected = $false
        try { [void](Assert-Within $case.p $case.r) } catch { $rejected = $true }
        if (-not $rejected) { throw 'Path negative control failed.' }; $rejects++
    }
    [void](Assert-Within (Join-Path $repo 'native/cache/nemo') $repo)
    $rejected = $false
    try { Assert-Version 'negative control' '1.12.0' '1.12.1' } catch { $rejected = $true }
    if (-not $rejected) { throw 'Version negative control failed.' }; $rejects++
    $rejected = $false
    try { Assert-ExecutableDigest ('0' * 64) $config.vcpkgExecutable.sha256 } catch { $rejected = $true }
    if (-not $rejected) { throw 'Executable digest negative control failed.' }; $rejects++
    $vcpkgEnvironment = @{}
    foreach ($entry in Get-ChildItem Env: | Where-Object { $_.Name -match '^(X_)?VCPKG_' }) { $vcpkgEnvironment[$entry.Name]=$entry.Value }
    try {
        foreach ($name in @('VCPKG_BINARY_SOURCES','X_VCPKG_REGISTRIES_CACHE','VCPKG_OVERLAY_PORTS','VCPKG_CHAINLOAD_TOOLCHAIN_FILE')) {
            [Environment]::SetEnvironmentVariable($name,'C:\outside-negative-control','Process')
        }
        Clear-VcpkgEnvironment
        if (@(Get-ChildItem Env: | Where-Object { $_.Name -match '^(X_)?VCPKG_' }).Count) { throw 'Inherited vcpkg environment negative control failed.' }
        $rejects++
    } finally {
        Clear-VcpkgEnvironment
        foreach ($name in $vcpkgEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name,$vcpkgEnvironment[$name],'Process') }
    }
    foreach ($pin in $config.sources.Values) { if ($pin.commit -notmatch '^[0-9a-f]{40}$') { throw 'A source pin is not a full commit.' } }
    $sourceFixture=Join-Path $cache ('source-selftests/'+[guid]::NewGuid().ToString('N'))
    foreach ($child in @('empty','nonempty')) { New-Item -ItemType Directory -Path (Assert-Within (Join-Path $sourceFixture $child) $cache) -Force | Out-Null }
    $keptFile=Join-Path $sourceFixture 'nonempty/keep.txt'
    'must remain' | Set-Content -LiteralPath $keptFile
    Assert-EmptySourceDirectory (Join-Path $sourceFixture 'empty')
    $rejected=$false
    try { Assert-EmptySourceDirectory (Join-Path $sourceFixture 'nonempty') } catch { $rejected=$true }
    if (-not $rejected -or (Get-Content -LiteralPath $keptFile -Raw).Trim() -cne 'must remain') { throw 'Source acquisition overwrite negative control failed.' }
    $rejects++
    $run = Assert-Within (Join-Path $repo ('target/native-build/manifest-selftests/'+[guid]::NewGuid().ToString('N'))) $repo
    New-Item -ItemType Directory -Path $run -Force | Out-Null
    $testManifestPath=Join-Path $run 'manifest.json'
    Save-BuildManifest ([ordered]@{status='STARTED';stage='ACQUIRE_SOURCES'}) $testManifestPath
    $snapshot=Get-Content -LiteralPath $testManifestPath -Raw | ConvertFrom-Json
    if ($snapshot.status -cne 'STARTED' -or $snapshot.stage -cne 'ACQUIRE_SOURCES') { throw 'Initial durable manifest was not readable.' }
    Save-BuildManifest ([ordered]@{status='STARTED';stage='CONFIGURE';requestedOptions=@{GGML_CUDA='OFF'}}) $testManifestPath
    $snapshot=Get-Content -LiteralPath $testManifestPath -Raw | ConvertFrom-Json
    if ($snapshot.stage -cne 'CONFIGURE' -or $snapshot.requestedOptions.GGML_CUDA -cne 'OFF' -or (Test-Path -LiteralPath ($testManifestPath+'.pending'))) { throw 'Durable manifest replacement failed.' }
    $rejected=$false
    try { Save-BuildManifest @{status='INVALID'} ($run+'-outside.json') } catch { $rejected=$true }
    if (-not $rejected -or (Test-Path -LiteralPath ($run+'-outside.json'))) { throw 'Manifest outside-root negative control failed.' }
    $rejects++
    . (Join-Path $PSScriptRoot 'test-native-build-flags.ps1')
    Invoke-NativeBuildFlagTests $run
    . (Join-Path $PSScriptRoot 'test-native-workspace.ps1')
    Invoke-NativeWorkspaceTests $run
    "PASS native script self-tests: negative controls=$rejects; no downloads or compilation."
    return
}

if ($CheckOnly -and $PrepareOnly) { throw '-CheckOnly and -PrepareOnly are mutually exclusive.' }
$requestedBuildOptions = New-NativeBuildOptions $config.options $Variant
if (-not $CheckOnly) {
    if ($env:WHISPER_NATIVE_BUILD_GUARD_PID) { Assert-GuardContext }
    else {
        $guardProfile = if ($Variant -eq 'cpu') { 'NativeDllCpu' } else { 'NativeDllGpu' }
        $guardArgs = @('-NoLogo','-NoProfile','-NonInteractive','-File',(Join-Path $PSScriptRoot 'guard-native.ps1'),'-Mode','Run','-Profile',$guardProfile,'-NativeBuildId',$BuildId,'-LotRoot','target/native-build')
        if ($PrepareOnly) { $guardArgs += '-NativePrepareOnly' }
        & (Join-Path $PSHOME 'pwsh.exe') @guardArgs
        if ($LASTEXITCODE -ne 0) { throw "Native build guard failed (exit $LASTEXITCODE). No automatic retry." }
        return
    }
}
[void](Assert-Within $cache $repo)
$run = Assert-Within (Join-Path $repo "target/native-build/$Variant/$BuildId") $repo
$nemo = Assert-Within (Join-Path $cache 'nemo') $cache
$llama = Assert-Within (Join-Path $nemo 'llama.cpp') $cache
$vcpkg = Assert-Within (Join-Path $cache 'vcpkg') $cache
$git = (Get-Command git.exe -ErrorAction Stop).Source
$tools = Get-Tools
if ($CheckOnly) {
    [ordered]@{status='CHECK_ONLY';variant=$Variant;tools=$tools;versions=$config.tools;assumptions=$config.assumptions;output=$run} | ConvertTo-Json -Depth 6
    return
}
if ((Get-PSDrive -Name ([IO.Path]::GetPathRoot($repo).Substring(0,1))).Free -lt 30GB) { throw 'Less than 30 GiB free: no native build or preparation.' }
if (Test-Path -LiteralPath $run) { throw "Output already exists: $run. Choose a new -BuildId; no cleanup performed." }
New-Item -ItemType Directory -Path $run -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $cache 'empty-hooks') -Force | Out-Null
$savedEnvironment = @{}
foreach ($entry in Get-ChildItem Env:) { $savedEnvironment[$entry.Name] = $entry.Value }
$manifest = [ordered]@{schemaVersion=1;status='STARTED';variant=$Variant;buildId=$BuildId;startedUtc=[DateTime]::UtcNow.ToString('o');configSha256=(Get-FileHash $configPath -Algorithm SHA256).Hash;sources=$config.sources;versions=$config.tools;tools=$tools;assumptions=$config.assumptions;determinism='MSVC /Brepro (C/C++, CUDA host compiler, linker); not a proof of binary reproducibility';output=$run}
Initialize-NativeProcessHelpers
$manifest.processFamilyHelperSha256=$script:NativeProcessHelpersSha256
$manifestPath = Join-Path $run 'manifest.json'
$manifest.stage = 'ACQUIRE_SOURCES'
Save-BuildManifest $manifest $manifestPath
try {
    foreach ($name in @('TEMP','TMP')) {
        $tempPath = Assert-Within (Join-Path $run 'temp') $run
        New-Item -ItemType Directory -Path $tempPath -Force | Out-Null
        [Environment]::SetEnvironmentVariable($name, $tempPath, 'Process')
    }
    $env:GIT_LFS_SKIP_SMUDGE = '1'
    Get-Source 'nemo' $nemo $config.sources.nemo
    $llamaLink = & $git --no-optional-locks -C $nemo ls-tree HEAD llama.cpp
    if ($LASTEXITCODE -ne 0 -or $llamaLink -notmatch ('^160000 commit ' + $config.sources.llama.commit + '\s+llama\.cpp$')) { throw 'NeMo llama gitlink differs from the pinned commit.' }
    Get-Source 'llama' $llama $config.sources.llama
    Get-Source 'vcpkg' $vcpkg $config.sources.vcpkg
    $manifest.sourceStatus = 'PINNED_AND_TRACKED_CLEAN'
    $series = @(Get-Content -LiteralPath (Join-Path $nemo 'patches/series') | Where-Object { $_ -and -not $_.StartsWith('#') })
    if ($series.Count -ne 19) { throw "Expected 19 historical patches, found $($series.Count)." }
    $manifest.patches = @($series | ForEach-Object { $path=Assert-Within (Join-Path $nemo "patches/$_") $nemo; @{name=$_;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash} })
    $manifest.stage = 'SOURCES_PREPARED'
    Save-BuildManifest $manifest $manifestPath
    if ($PrepareOnly) { $manifest.status='SOURCES_PREPARED'; return }
    $cachedNemo=$nemo; $cachedLlama=$llama; $cachedVcpkg=$vcpkg
    $workspace=Assert-NativeWorkspacePath (Get-NativeWorkspacePath $Variant) $Variant
    $owner=Get-Content -LiteralPath (Join-Path $workspace '.whisper-native-workspace.json') -Raw | ConvertFrom-Json
    if ($owner.buildId -cne $BuildId -or $owner.variant -cne $Variant -or [string]$owner.guardPid -cne $env:WHISPER_NATIVE_BUILD_GUARD_PID) { throw 'Fixed workspace is not owned by this guard invocation.' }
    $manifest.workspace=$workspace
    New-Item -ItemType Directory -Path (Join-Path $workspace 'temp') -Force | Out-Null
    $env:TEMP=Join-Path $workspace 'temp'; $env:TMP=$env:TEMP
    $nemo=Join-Path $workspace 'src/nemo'; $llama=Join-Path $nemo 'llama.cpp'; $vcpkg=Join-Path $workspace 'src/vcpkg'
    Export-NativeSource $cachedNemo $nemo 'nemo'
    # Git archives represent submodules by an empty directory.
    if (Test-Path -LiteralPath $llama) {
        if (@(Get-ChildItem -LiteralPath $llama -Force).Count) { throw 'Exported llama gitlink is not empty.' }
        [IO.Directory]::Delete($llama,$false)
    }
    Export-NativeSource $cachedLlama $llama 'llama'
    Export-NativeSource $cachedVcpkg $vcpkg 'vcpkg'
    if (Test-Path -LiteralPath (Join-Path $cachedVcpkg 'vcpkg.exe')) { Copy-Item -LiteralPath (Join-Path $cachedVcpkg 'vcpkg.exe') -Destination $vcpkg }
    $localPatch=Join-Path $repo 'native/patches/ggml-pinned-build-commit.patch'
    Copy-Item -LiteralPath $localPatch -Destination (Join-Path $nemo 'patches/ggml-pinned-build-commit.patch')
    [IO.File]::AppendAllText((Join-Path $nemo 'patches/series'),"`nggml-pinned-build-commit.patch`n",$utf8)
    $manifest.localPatches=@(@{name='ggml-pinned-build-commit.patch';sha256=(Get-FileHash -LiteralPath $localPatch).Hash})
    Import-BuildEnvironment $tools
    $manifest.environmentControls=@{VSCMD_SKIP_SENDTELEMETRY='1';reason='Disable detached VsDevCmd telemetry; NativeBuild stop policy unchanged'}
    $manifest.stage = 'BOOTSTRAP'
    Save-BuildManifest $manifest $manifestPath
    if (Test-Path -LiteralPath (Join-Path $vcpkg 'vcpkg.exe')) {
        [void](Assert-VcpkgExecutable)
    } else {
        # Same vendor script/interpreter as bootstrap-vcpkg.bat, with no PATH lookup.
        Invoke-Logged $tools.bootstrapPowerShell @('-NoLogo','-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',(Join-Path $vcpkg 'scripts/bootstrap.ps1'),'-disableMetrics') 'vcpkg-bootstrap'
    }
    $manifest.vcpkgExecutableSha256 = Assert-VcpkgExecutable
    $build = Join-Path $workspace 'build'
    $options = @{} + $requestedBuildOptions
    $options.GGML_BUILD_COMMIT=$config.sources.llama.commit
    $options.GGML_CUDA = if ($Variant -eq 'gpu') { 'ON' } else { 'OFF' }
    $options.CMAKE_C_COMPILER = $tools.cl.Replace('\','/')
    $options.CMAKE_CXX_COMPILER = $options.CMAKE_C_COMPILER
    $options.CMAKE_MAKE_PROGRAM = $tools.ninja.Replace('\','/')
    $options.CMAKE_TOOLCHAIN_FILE = (Join-Path $vcpkg 'scripts/buildsystems/vcpkg.cmake').Replace('\','/')
    $options.VCPKG_INSTALLED_DIR = (Join-Path $build 'vcpkg_installed').Replace('\','/')
    $options.CMAKE_FIND_USE_PACKAGE_REGISTRY = 'OFF'
    $options.CMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY = 'OFF'
    if ($Variant -eq 'gpu') {
        $options.CMAKE_CUDA_ARCHITECTURES='61'
        $options.CMAKE_CUDA_COMPILER=$tools.nvcc.Replace('\','/')
        $options.CMAKE_CUDA_HOST_COMPILER=$options.CMAKE_C_COMPILER
    }
    $manifest.requestedOptions = $options
    $configure = @('-S',$nemo,'-B',$build,'-G','Ninja')
    foreach ($key in $options.Keys | Sort-Object) { $configure += "-D${key}=$($options[$key])" }
    $manifest.stage = 'CONFIGURE'
    Save-BuildManifest $manifest $manifestPath
    Invoke-Logged $tools.cmake $configure 'configure'
    $effective = @{}
    foreach ($line in Get-Content -LiteralPath (Join-Path $build 'CMakeCache.txt')) {
        if ($line -match '^([^/#][^:]*):[^=]+=(.*)$') { $effective[$matches[1]]=$matches[2] }
    }
    $manifest.effectiveOptions = $effective
    foreach ($key in @('GGML_NATIVE','GGML_CUDA','CMAKE_BUILD_TYPE','CMAKE_MSVC_RUNTIME_LIBRARY','VCPKG_TARGET_TRIPLET')) {
        if ($effective[$key] -cne $options[$key]) { throw "Effective CMake option differs: $key" }
    }
    $manifest.flagValidation = Assert-NativeEffectiveFlags $effective $Variant
    $manifest.stage = 'BUILD'
    Save-BuildManifest $manifest $manifestPath
    Invoke-Logged $tools.cmake @('--build',$build,'--parallel','4') 'build'
    $manifest.ggmlCommit=Get-NativePeCommit (Join-Path $build 'bin/ggml-base.dll')
    if ($manifest.ggmlCommit -cne $config.sources.llama.commit) { throw 'Decoded ggml_commit differs from pinned llama source.' }
    $dllOutput=Join-Path $run 'dll'
    New-Item -ItemType Directory -Path $dllOutput | Out-Null
    $expected = @('ggml-base.dll','ggml-cpu.dll','ggml.dll','nemo_speech_asr.dll','nemo_speech_asr_c.dll')
    if ($Variant -eq 'gpu') { $expected += 'ggml-cuda.dll' }
    $manifest.dlls = @($expected | ForEach-Object {
        $path = Assert-Within (Join-Path $build "bin/$_") $workspace
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Expected DLL missing: $path" }
        Copy-Item -LiteralPath $path -Destination $dllOutput
        $path=Join-Path $dllOutput $_
        @{name=$_;path=$path;bytes=(Get-Item -LiteralPath $path).Length;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash}
    })
    if ($Variant -eq 'gpu') {
        $installedInventory=Get-Content -LiteralPath (Join-Path $repo 'docs/architecture/raw/native-build/installed-inventory.json') -Raw | ConvertFrom-Json
        $manifest.cudaRedistributed=@(foreach ($name in @('cublas64_12.dll','cublasLt64_12.dll','cudart64_12.dll')) {
            $path=Join-Path $tools.cuda "bin/$name"
            $hash=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
            $reference=@($installedInventory | Where-Object RelativePath -IEQ "gpu\$name")
            if ($reference.Count -ne 1 -or $reference[0].Sha256 -ine $hash -or $reference[0].Size -ne (Get-Item -LiteralPath $path).Length) { throw "Toolkit CUDA runtime differs from installed reference: $name" }
            Copy-Item -LiteralPath $path -Destination $dllOutput
            [pscustomobject]@{name=$name;source=$path;path=(Join-Path $dllOutput $name);sha256=$hash;bytes=(Get-Item -LiteralPath $path).Length;compiled=$false}
        })
    }
    Assert-Source $cachedNemo $config.sources.nemo
    Assert-Source $cachedLlama $config.sources.llama
    Assert-Source $cachedVcpkg $config.sources.vcpkg
    $manifest.status = 'BUILT_NOT_QUALIFIED'
    Write-Host "Built $Variant. Equivalence NOT established. Manifest: $manifestPath"
} catch {
    $manifest.status = 'FAILED'
    $manifest.failure = $_.Exception.Message
    throw
} finally {
    $manifest.finishedUtc=[DateTime]::UtcNow.ToString('o')
    Save-BuildManifest $manifest $manifestPath
    foreach ($entry in Get-ChildItem Env:) {
        if (-not $savedEnvironment.ContainsKey($entry.Name)) { Remove-Item -LiteralPath ('Env:' + $entry.Name) }
    }
    foreach ($name in $savedEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name,$savedEnvironment[$name],'Process') }
}
