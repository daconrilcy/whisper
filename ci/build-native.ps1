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
    $path = Assert-Within (Join-Path $vcpkg 'vcpkg.exe') $cache
    $digest = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    Assert-ExecutableDigest $digest $config.vcpkgExecutable.sha256
    return $digest
}

function Invoke-Logged([string]$Program, [string[]]$Arguments, [string]$Label) {
    [IO.File]::AppendAllText((Join-Path $run 'commands.log'), ($Program + ' ' + ($Arguments | ConvertTo-Json -Compress) + "`n"), $utf8)
    $log = Join-Path $run ($Label + '.log')
    & $Program @Arguments 2>&1 | ForEach-Object { $_.ToString() } | Tee-Object -FilePath $log | ForEach-Object {
        if ($_ -match '^-- |^\[\d+/\d+\]|error|Error|FAILED|Installing|Downloading|Building package') { Write-Host $_ }
    }
    $code = $LASTEXITCODE
    [IO.File]::AppendAllText((Join-Path $run 'commands.log'), "exit=$code label=$Label`n", $utf8)
    if ($code -ne 0) { throw "$Label failed (exit $code); see $log. Stop; no automatic retry." }
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
        Assert-Source $Path $Pin
        return
    }
    Invoke-Logged $git @('init', $Path) "$Name-init"
    Invoke-Logged $git @('-C', $Path, 'remote', 'add', 'origin', $Pin.url) "$Name-origin"
    Invoke-Logged $git @('-c','gc.auto=0','-C',$Path,'fetch','--depth=1','origin',$Pin.commit) "$Name-fetch"
    Invoke-Logged $git @('-c','core.autocrlf=false','-c',"core.hooksPath=$cache/empty-hooks",'-C',$Path,'checkout','--detach',$Pin.commit) "$Name-checkout"
    Assert-Source $Path $Pin
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
        cuda=(Join-Path $env:ProgramFiles "NVIDIA GPU Computing Toolkit/CUDA/$($config.tools.cudaDirectory)")
    }
    foreach ($name in @('cmake','ninja','cl','dumpbin','vcvars')) {
        if (-not (Test-Path -LiteralPath $tools[$name] -PathType Leaf)) { throw "Missing tool: $($tools[$name])" }
    }
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
    foreach ($entry in @{TEMP='temp';TMP='temp';LOCALAPPDATA='localappdata';APPDATA='appdata';XDG_CACHE_HOME='xdg';GNUPGHOME='gnupg'}.GetEnumerator()) {
        $path = Assert-Within (Join-Path $run $entry.Value) $run
        New-Item -ItemType Directory -Path $path -Force | Out-Null
        [Environment]::SetEnvironmentVariable($entry.Key, $path, 'Process')
    }
    foreach ($entry in @{VCPKG_DOWNLOADS='downloads';VCPKG_DEFAULT_BINARY_CACHE='binary-cache';X_VCPKG_REGISTRIES_CACHE='registries'}.GetEnumerator()) {
        $path = Assert-Within (Join-Path $cache $entry.Value) $cache
        New-Item -ItemType Directory -Path $path -Force | Out-Null
        [Environment]::SetEnvironmentVariable($entry.Key, $path, 'Process')
    }
    $env:VCPKG_ROOT = $vcpkg
    $env:VCPKG_BINARY_SOURCES = 'clear;files,' + $env:VCPKG_DEFAULT_BINARY_CACHE + ',readwrite'
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
    "PASS native script self-tests: negative controls=$rejects; no downloads or compilation."
    return
}

if ($CheckOnly -and $PrepareOnly) { throw '-CheckOnly and -PrepareOnly are mutually exclusive.' }
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
if (-not $PrepareOnly) {
    throw 'Native compilation is blocked: guard-native.ps1 has no authorized CMake route for native/cache and target/native-build. Approve a bounded extension of the existing guard before any bootstrap or compilation. CheckOnly and PrepareOnly remain available; there is no bypass switch.'
}
if ((Get-PSDrive -Name ([IO.Path]::GetPathRoot($repo).Substring(0,1))).Free -lt 30GB) { throw 'Less than 30 GiB free: no native build or preparation.' }
if (Test-Path -LiteralPath $run) { throw "Output already exists: $run. Choose a new -BuildId; no cleanup performed." }
New-Item -ItemType Directory -Path $run -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $cache 'empty-hooks') -Force | Out-Null
$savedEnvironment = @{}
foreach ($entry in Get-ChildItem Env:) { $savedEnvironment[$entry.Name] = $entry.Value }
$manifest = [ordered]@{schemaVersion=1;status='STARTED';variant=$Variant;buildId=$BuildId;startedUtc=[DateTime]::UtcNow.ToString('o');configSha256=(Get-FileHash $configPath -Algorithm SHA256).Hash;sources=$config.sources;versions=$config.tools;tools=$tools;assumptions=$config.assumptions;determinism='MSVC /Brepro (C/C++, CUDA host compiler, linker); not a proof of binary reproducibility';output=$run}
$manifestPath = Join-Path $run 'manifest.json'
try {
    foreach ($name in @('TEMP','TMP')) {
        $tempPath = Assert-Within (Join-Path $run 'temp') $run
        New-Item -ItemType Directory -Path $tempPath -Force | Out-Null
        [Environment]::SetEnvironmentVariable($name, $tempPath, 'Process')
    }
    $env:GIT_LFS_SKIP_SMUDGE = '1'
    Get-Source 'nemo' $nemo $config.sources.nemo
    Get-Source 'llama' $llama $config.sources.llama
    Get-Source 'vcpkg' $vcpkg $config.sources.vcpkg
    $manifest.sourceStatus = 'PINNED_AND_TRACKED_CLEAN'
    $series = @(Get-Content -LiteralPath (Join-Path $nemo 'patches/series') | Where-Object { $_ -and -not $_.StartsWith('#') })
    if ($series.Count -ne 19) { throw "Expected 19 historical patches, found $($series.Count)." }
    $manifest.patches = @($series | ForEach-Object { $path=Assert-Within (Join-Path $nemo "patches/$_") $nemo; @{name=$_;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash} })
    if ($PrepareOnly) { $manifest.status='SOURCES_PREPARED'; return }
    Import-BuildEnvironment $tools
    if (Test-Path -LiteralPath (Join-Path $vcpkg 'vcpkg.exe')) {
        [void](Assert-VcpkgExecutable)
    } else {
        Invoke-Logged (Join-Path $vcpkg 'bootstrap-vcpkg.bat') @('-disableMetrics') 'vcpkg-bootstrap'
    }
    $manifest.vcpkgExecutableSha256 = Assert-VcpkgExecutable
    $build = Join-Path $run 'build'
    $options = @{} + $config.options
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
        $options.CMAKE_CUDA_FLAGS='-Xcompiler=/Brepro'
    }
    $manifest.requestedOptions = $options
    $configure = @('-S',$nemo,'-B',$build,'-G','Ninja')
    foreach ($key in $options.Keys | Sort-Object) { $configure += "-D${key}=$($options[$key])" }
    Invoke-Logged $tools.cmake $configure 'configure'
    $effective = @{}
    foreach ($line in Get-Content -LiteralPath (Join-Path $build 'CMakeCache.txt')) {
        if ($line -match '^([^/#][^:]*):[^=]+=(.*)$') { $effective[$matches[1]]=$matches[2] }
    }
    $manifest.effectiveOptions = $effective
    foreach ($key in @('GGML_NATIVE','GGML_CUDA','CMAKE_BUILD_TYPE','VCPKG_TARGET_TRIPLET')) {
        if ($effective[$key] -cne $options[$key]) { throw "Effective CMake option differs: $key" }
    }
    Invoke-Logged $tools.cmake @('--build',$build,'--parallel','4') 'build'
    $expected = @('ggml-base.dll','ggml-cpu.dll','ggml.dll','nemo_speech_asr.dll','nemo_speech_asr_c.dll')
    if ($Variant -eq 'gpu') { $expected += 'ggml-cuda.dll' }
    $manifest.dlls = @($expected | ForEach-Object {
        $path = Assert-Within (Join-Path $build "bin/$_") $run
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Expected DLL missing: $path" }
        @{name=$_;path=$path;bytes=(Get-Item -LiteralPath $path).Length;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash}
    })
    Assert-Source $nemo $config.sources.nemo
    Assert-Source $llama $config.sources.llama
    Assert-Source $vcpkg $config.sources.vcpkg
    $manifest.status = 'BUILT_NOT_QUALIFIED'
    Write-Host "Built $Variant. Equivalence NOT established. Manifest: $manifestPath"
} catch {
    $manifest.status = 'FAILED'
    $manifest.failure = $_.Exception.Message
    throw
} finally {
    $manifest.finishedUtc=[DateTime]::UtcNow.ToString('o')
    $manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $manifestPath -Encoding utf8
    foreach ($entry in Get-ChildItem Env:) {
        if (-not $savedEnvironment.ContainsKey($entry.Name)) { Remove-Item -LiteralPath ('Env:' + $entry.Name) }
    }
    foreach ($name in $savedEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name,$savedEnvironment[$name],'Process') }
}
