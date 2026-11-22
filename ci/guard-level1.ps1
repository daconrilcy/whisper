param(
    [ValidateSet('Run','SelfTest','Structure')]
    [string]$Mode = 'Run',
    [switch]$SelfTest,
    [switch]$DryRun,
    [switch]$WithoutCargoTargetDir,
    [switch]$TurboEdition,
    [switch]$DualCpuRebaseline,
    [string]$FreshTarget,
    [string]$ExistingTarget,
    [string]$PersonalParakeetModelPath,
    [string]$DesktopCpuWorkerSha256,
    [string]$DualParakeetCpuWorkerSha256,
    [string]$DualCpuRuntimeSha256,
    [string[]]$UnsetEnv = @(),
    [string[]]$CargoArgs = @(),
    [string]$CargoArgsJson,
    [string]$LotRoot = 'target/parakeet-tests/garde-n1-categories'
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$script:turboEditionEnabled = [bool]$TurboEdition
$script:dualCpuRebaselineEnabled = [bool]$DualCpuRebaseline
$script:turboEditionMarker = 'WHISPER_DISTRIBUTION_EDITION_TURBO'
$script:dualEditionMarker = 'WHISPER_DISTRIBUTION_EDITION_DUAL'
$script:dualEditionBuildActive = $false
$script:authorizedDualParakeetCpuWorkerSha256 = $null
$script:authorizedDualCpuRuntimeSha256 = $null
$script:authorizedDualBootstrapAssetSha256 = @{}

function Assert-DedicatedWorktree([string]$WorkingDirectory, [string]$RepositoryRoot) {
    $mainRoot = [IO.Path]::GetFullPath('C:/dev/whisper').TrimEnd('\', '/')
    foreach ($path in @($WorkingDirectory, $RepositoryRoot)) {
        $full = [IO.Path]::GetFullPath($path).TrimEnd('\', '/')
        if ($full.Equals($mainRoot, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Campaign refused in the main repository root C:/dev/whisper; use a dedicated worktree.'
        }
    }
}

# Reject both a main-root cwd and a main-checkout script before campaign side effects.
Assert-DedicatedWorktree (Get-Location).ProviderPath (Join-Path $PSScriptRoot '..')

function Convert-CargoArgsJson([string]$Json) {
    $parsed = ConvertFrom-Json -InputObject $Json
    if ($parsed -is [string]) { return ,([string[]]@($parsed)) }
    if ($parsed -isnot [array] -or @($parsed | Where-Object { $_ -isnot [string] }).Count -gt 0) {
        throw '-CargoArgsJson must be a JSON array of strings.'
    }
    return ,([string[]]$parsed)
}

$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$script:requestedLotRoot = $LotRoot
$lotRoot = Join-Path $repo $LotRoot
$cargoRoot = Join-Path $repo 'target/parakeet-tests/cargo-shared'
$cargoHome = Join-Path $repo 'target/cargo-home-shared'
$tempRoot = Join-Path $lotRoot 'temp'
$scratchRoot = Join-Path $lotRoot 'scratch'
$runRoot = $null
$utf8NoBom = [Text.UTF8Encoding]::new($false)
$allowedUnsetPatterns = @('^CUDA_PATH$', '^CUDA_PATH_V.*$', '^GGML_.*$', '^WHISPER_.*$', '^PARAKEET_.*$')
$script:cargoProcessLaunchCount = 0
# A scoped package invocation is allowed only through its registered canonical lot root.
$script:packageSetPolicies = @{
    'target/parakeet-tests/garde-n1-paquets-20261112' = @(
        'whisper-core',
        'whisper-adapters',
        'whisper-desktop',
        'whisper-bootstrap',
        'whisper-worker-common',
        'whisper-worker-parakeet-cpu'
    )
}
$resumeImportProfiles = [Collections.Generic.List[string]]::new()
$resumeImportProfiles.Add((ConvertTo-Json -InputObject ([string[]]@('fmt','--check')) -Compress))
$resumeImportProfiles.Add((ConvertTo-Json -InputObject ([string[]]@('test','--no-fail-fast')) -Compress))
foreach ($filter in @(
    'resume_validation_rejects_later_fifo_row_without_advancing_any_generation',
    'interrupted_',
    'injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume',
    'cancelling_duplicate_import_releases_only_the_new_source_snapshot',
    'stale_duplicate_dialog_response_cannot_submit_a_replacement',
    'r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision',
    'resumed_prefix_copy_retries_after_pending_creation_without_duplication',
    'unified_poll_resumed_prefix_emits_only_new_segments'
)) {
    $resumeImportProfiles.Add((ConvertTo-Json -InputObject ([string[]]@('test',$filter)) -Compress))
}
foreach ($worker in @('whisper-worker-cpu','whisper-worker-parakeet-cpu','whisper-worker-common')) {
    $workerProfiles = [Collections.Generic.List[object]]::new()
    $workerProfiles.Add([string[]]@('test','-p',$worker))
    $workerProfiles.Add([string[]]@('test','--no-run','-p',$worker))
    $workerProfiles.Add([string[]]@('test','-p',$worker,'--','--list'))
    foreach ($profile in $workerProfiles) {
        $resumeImportProfiles.Add((ConvertTo-Json -InputObject ([string[]]$profile) -Compress))
    }
}
$clippyProfile = [Collections.Generic.List[string]]::new()
$clippyProfile.Add('clippy')
foreach ($package in @('whisper-core','whisper-adapters','whisper-desktop','whisper-bootstrap','whisper-worker-common','whisper-worker-parakeet-cpu')) {
    $clippyProfile.Add('-p')
    $clippyProfile.Add($package)
}
foreach ($argument in @('--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')) {
    $clippyProfile.Add($argument)
}
$resumeImportProfiles.Add((ConvertTo-Json -InputObject ([string[]]$clippyProfile.ToArray()) -Compress))
$clippyCorrectionsProfiles = [Collections.Generic.List[object]]::new()
$clippyCorrectionsProfiles.Add([string[]]@('fmt','--check'))
$clippyCorrectionsProfiles.Add([string[]]@('test','--no-fail-fast'))
foreach ($worker in @('whisper-worker-cpu','whisper-worker-parakeet-cpu','whisper-worker-common')) {
    $clippyCorrectionsProfiles.Add([string[]]@('test','--no-run','-p',$worker))
    $clippyCorrectionsProfiles.Add([string[]]@('test','-p',$worker,'--','--list'))
    $clippyCorrectionsProfiles.Add([string[]]@('test','-p',$worker))
}
$clippyCorrectionsProfile = [Collections.Generic.List[string]]::new()
$clippyCorrectionsProfile.Add('clippy')
foreach ($package in @('whisper-core','whisper-adapters','whisper-desktop','whisper-bootstrap','whisper-worker-common','whisper-worker-parakeet-cpu')) {
    $clippyCorrectionsProfile.Add('-p')
    $clippyCorrectionsProfile.Add($package)
}
foreach ($argument in @('--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')) {
    $clippyCorrectionsProfile.Add($argument)
}
$clippyCorrectionsProfiles.Add([string[]]$clippyCorrectionsProfile.ToArray())
$script:cargoInvocationPolicies = @{
    'target/parakeet-tests/refacto-reprise-import' = [string[]]$resumeImportProfiles.ToArray()
    'target/parakeet-tests/clippy-corrections-suivantes-20261113' = [string[]]@($clippyCorrectionsProfiles | ForEach-Object { ConvertTo-Json -InputObject ([string[]]$_) -Compress })
}

$cpuDistributionRoot = 'target/parakeet-tests/cpu-distribution-x64-20261115'
$cpuDistributionProfiles = [Collections.Generic.List[string]]::new()
foreach ($profile in @(
    @('fetch'),
    @('fmt','--check'),
    @('test','--no-fail-fast'),
    @('test','--no-run','-p','whisper-worker-cpu'),
    @('test','-p','whisper-worker-cpu','--','--list'),
    @('test','-p','whisper-worker-cpu'),
    @('test','--no-run','-p','whisper-worker-parakeet-cpu'),
    @('test','-p','whisper-worker-parakeet-cpu','--','--list'),
    @('test','-p','whisper-worker-parakeet-cpu'),
    @('test','--no-run','-p','whisper-worker-common'),
    @('test','-p','whisper-worker-common','--','--list'),
    @('test','-p','whisper-worker-common'),
    @('build','--release','-p','whisper-worker-cpu','-vv'),
    @('build','--release','-p','whisper-desktop','-vv'),
    @('build','--release','-p','whisper-bootstrap','-vv')
)) {
    $cpuDistributionProfiles.Add((ConvertTo-Json -InputObject ([string[]]$profile) -Compress))
}
$cpuDistributionClippy = [Collections.Generic.List[string]]::new()
$cpuDistributionClippy.Add('clippy')
foreach ($package in @('whisper-core','whisper-adapters','whisper-desktop','whisper-bootstrap','whisper-worker-cpu','whisper-worker-common','whisper-worker-parakeet-cpu')) {
    $cpuDistributionClippy.Add('-p')
    $cpuDistributionClippy.Add($package)
}
foreach ($argument in @('--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')) {
    $cpuDistributionClippy.Add($argument)
}
$cpuDistributionProfiles.Add((ConvertTo-Json -InputObject ([string[]]$cpuDistributionClippy.ToArray()) -Compress))
$gpuDistributionRoot = 'target/parakeet-tests/main-facades'
$gpuDistributionProfiles = [string[]]@(
    (ConvertTo-Json -InputObject ([string[]]@('build','--release','-p','whisper-worker-gpu','-vv')) -Compress)
)
$script:cargoInvocationPolicies[$cpuDistributionRoot] = [string[]]$cpuDistributionProfiles.ToArray()
$script:cargoInvocationPolicies[$gpuDistributionRoot] = $gpuDistributionProfiles

$dualCpuRebaselineRoot = 'target/parakeet-tests/dual-cpu-r1'
$dualCpuRebaselineProfiles = [Collections.Generic.List[string]]::new()
foreach ($profile in @(
    @('test','--no-fail-fast'),
    @('fmt','--check'),
    @('test','--no-run','-p','whisper-worker-cpu'),
    @('test','-p','whisper-worker-cpu','--','--list'),
    @('test','-p','whisper-worker-cpu'),
    @('test','--no-run','-p','whisper-worker-parakeet-cpu'),
    @('test','-p','whisper-worker-parakeet-cpu','--','--list'),
    @('test','-p','whisper-worker-parakeet-cpu'),
    @('test','--no-run','-p','whisper-worker-common'),
    @('test','-p','whisper-worker-common','--','--list'),
    @('test','-p','whisper-worker-common'),
    @('build','--release','-p','whisper-worker-cpu','-vv'),
    @('build','--release','-p','whisper-worker-parakeet-cpu','--features','parakeet-personal','-vv'),
    @('build','--release','-p','whisper-desktop','--features','personal-install','-vv'),
    @('build','--release','-p','whisper-bootstrap','-vv')
)) {
    $dualCpuRebaselineProfiles.Add((ConvertTo-Json -InputObject ([string[]]$profile) -Compress))
}
$dualCpuRebaselineClippy = [Collections.Generic.List[string]]::new()
$dualCpuRebaselineClippy.Add('clippy')
foreach ($package in @('whisper-core','whisper-adapters','whisper-desktop','whisper-bootstrap','whisper-worker-common','whisper-worker-parakeet-cpu')) {
    $dualCpuRebaselineClippy.Add('-p')
    $dualCpuRebaselineClippy.Add($package)
}
foreach ($argument in @('--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')) {
    $dualCpuRebaselineClippy.Add($argument)
}
$dualCpuRebaselineProfiles.Add((ConvertTo-Json -InputObject ([string[]]$dualCpuRebaselineClippy.ToArray()) -Compress))
$script:cargoInvocationPolicies[$dualCpuRebaselineRoot] = [string[]]$dualCpuRebaselineProfiles.ToArray()

if ($CargoArgsJson) {
    if ($CargoArgs.Count -gt 0) { throw 'Use either -CargoArgs or -CargoArgsJson, not both.' }
    $CargoArgs = Convert-CargoArgsJson $CargoArgsJson
}
if (($DualParakeetCpuWorkerSha256 -or $DualCpuRuntimeSha256) -and -not $DualCpuRebaseline) {
    throw 'Dual worker/runtime SHA inputs require -DualCpuRebaseline.'
}

function Test-DesktopCpuWorkerSha256Policy([string]$Mode, [string]$Digest, [string[]]$Arguments, [string]$Root) {
    if ($Mode -cne 'Run' -or $Digest -notmatch '^[0-9a-fA-F]{64}$' -or
        $Root -cne 'target/parakeet-tests/cpu-distribution-x64-20261115') {
        return $false
    }
    $expected = @('build','--release','-p','whisper-desktop','-vv')
    if ($Arguments.Count -ne $expected.Count) { return $false }
    for ($index = 0; $index -lt $expected.Count; $index++) {
        if ($Arguments[$index] -cne $expected[$index]) { return $false }
    }
    return $true
}

function Test-DesktopCpuWorkerInvocation([string[]]$Arguments) {
    $profile = @($Arguments | Where-Object { $_ -cne '--offline' -and $_ -cne '--locked' })
    $allowedProfiles = [Collections.Generic.List[string[]]]::new()
    $allowedProfiles.Add([string[]]@('build','--release','-p','whisper-desktop','-vv'))
    if ($script:dualCpuRebaselineEnabled) {
        $allowedProfiles.Add([string[]]@('build','--release','-p','whisper-desktop','--features','personal-install','-vv'))
    }
    foreach ($expected in $allowedProfiles) {
        if ($profile.Count -ne $expected.Count) { continue }
        $matches = $true
        for ($index = 0; $index -lt $expected.Count; $index++) {
            if ($profile[$index] -cne $expected[$index]) { $matches = $false; break }
        }
        if ($matches) { return $true }
    }
    return $false
}

$script:authorizedDesktopCpuWorkerSha256 = $null
if ($DesktopCpuWorkerSha256) {
    if (-not (Test-DesktopCpuWorkerSha256Policy $Mode $DesktopCpuWorkerSha256 $CargoArgs $script:requestedLotRoot)) {
        throw 'Desktop CPU worker digest is allowed only for the exact T11 desktop release build and lot root.'
    }
    $script:authorizedDesktopCpuWorkerSha256 = $DesktopCpuWorkerSha256.ToLowerInvariant()
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

function Assert-PersonalParakeetModel([string]$Path, [string]$ExpectedSha) {
    if (-not [IO.Path]::IsPathFullyQualified($Path)) { throw 'Personal Parakeet model path must be absolute.' }
    $full = Get-FullPath $Path
    if ([IO.Path]::GetFileName($full) -cne 'parakeet-tdt-0.6b-v3.q8_0.gguf') { throw 'Unexpected personal Parakeet model name.' }
    Assert-NoReparseChain $full 'personal Parakeet model'
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { throw "Personal Parakeet model absent: $full" }
    if ((Get-FileHash -LiteralPath $full -Algorithm SHA256).Hash -ine $ExpectedSha) { throw "Personal Parakeet model SHA-256 mismatch: $full" }
    return $full
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
        if ($name -ceq $script:turboEditionMarker) { throw "Guard-managed edition marker cannot be listed in -UnsetEnv: $name" }
        if ($script:authorizedDesktopCpuWorkerSha256 -and $name -ieq 'WHISPER_EXPECTED_CPU_WORKER_SHA256') {
            throw 'The authorized desktop worker digest cannot be listed in -UnsetEnv.'
        }
        if (-not $valid) { throw "Environment variable is not allowed in -UnsetEnv: $name" }
    }
}

function Set-CargoEditionEnvironment([System.Collections.IDictionary]$Environment, [bool]$Turbo) {
    if ($Turbo) { $Environment[$script:turboEditionMarker] = '1' }
    else { [void]$Environment.Remove($script:turboEditionMarker) }
}

function Assert-CargoEditionEnvironment([System.Collections.IDictionary]$Environment, [bool]$Turbo) {
    if ($Turbo -and $Environment[$script:turboEditionMarker] -cne '1') {
        throw 'Turbo edition compile marker is missing or invalid in the Cargo child environment.'
    }
    if (-not $Turbo -and $Environment.Contains($script:turboEditionMarker)) {
        throw 'Generic Cargo child environment inherited a Turbo edition marker.'
    }
}

function Set-CargoDualEditionEnvironment([System.Collections.IDictionary]$Environment, [bool]$Dual) {
    if ($Dual) { $Environment[$script:dualEditionMarker] = '1' }
    else { [void]$Environment.Remove($script:dualEditionMarker) }
}

function Assert-CargoDualEditionEnvironment([System.Collections.IDictionary]$Environment, [bool]$Dual) {
    if ($Dual -and $Environment[$script:dualEditionMarker] -cne '1') {
        throw 'Dual edition compile marker is missing or invalid in the Cargo child environment.'
    }
    if (-not $Dual -and $Environment.Contains($script:dualEditionMarker)) {
        throw 'Generic Cargo child environment inherited a dual edition marker.'
    }
}

function Assert-CargoDualBuildIdentityEnvironment([System.Collections.IDictionary]$Environment, [bool]$CargoInvocation) {
    $expected = @{}
    if ($CargoInvocation -and $script:authorizedDesktopCpuWorkerSha256) {
        $expected['WHISPER_EXPECTED_CPU_WORKER_SHA256'] = $script:authorizedDesktopCpuWorkerSha256
    }
    if ($CargoInvocation -and $script:dualEditionBuildActive) {
        if ($script:authorizedDualParakeetCpuWorkerSha256) {
            $expected['WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256'] = $script:authorizedDualParakeetCpuWorkerSha256
            $expected['WHISPER_DUAL_PARAKEET_CPU_RUNTIME_SHA256'] = $script:authorizedDualCpuRuntimeSha256
        }
        foreach ($name in $script:authorizedDualBootstrapAssetSha256.Keys) {
            $expected[$name] = $script:authorizedDualBootstrapAssetSha256[$name]
        }
    }
    foreach ($name in @(
        'WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256',
        'WHISPER_DUAL_PARAKEET_CPU_RUNTIME_SHA256',
        'WHISPER_EXPECTED_CPU_WORKER_SHA256',
        'WHISPER_DUAL_BOOTSTRAP_DESKTOP_SHA256',
        'WHISPER_DUAL_BOOTSTRAP_TURBO_CPU_SHA256',
        'WHISPER_DUAL_BOOTSTRAP_PARAKEET_CPU_SHA256'
    )) {
        $actual = [string]$Environment[$name]
        if ($expected.ContainsKey($name)) {
            if ($actual -cne $expected[$name]) { throw "Dual build identity variable differs in Cargo child environment: $name" }
        } elseif ($Environment.Contains($name)) {
            throw "Unauthorized dual build identity variable reached Cargo: $name"
        }
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

function Test-CargoArguments([string[]]$CommandTokens, [string[]]$Members, [switch]$SkipMemberValidation) {
    if ($CommandTokens.Count -lt 1) { throw 'A Cargo command is required.' }
    if ($CommandTokens[0] -ceq 'fetch') {
        if ($CommandTokens.Count -ne 2 -or $CommandTokens[1] -cne '--locked') {
            throw 'Only cargo fetch --locked is allowed for network access.'
        }
        return [pscustomobject]@{ Args = @('fetch','--locked'); Category = 'Fetch' }
    }
    if ($CommandTokens[0] -ceq 'clippy') {
        $delimiterIndex = [Array]::IndexOf($CommandTokens, '--')
        if ($delimiterIndex -ge 0) {
            if ($delimiterIndex -ge $CommandTokens.Count - 1) { throw 'Clippy lint arguments are required after --.' }
            $rawLintArgs = @($CommandTokens[($delimiterIndex + 1)..($CommandTokens.Count - 1)])
            $allowedLints = @('clippy::too_many_lines','clippy::cognitive_complexity','clippy::too_many_arguments','clippy::type_complexity')
            if (($rawLintArgs.Count % 2) -ne 0) { throw 'Clippy requires -W and an allowed lint in pairs.' }
            for ($j = 0; $j -lt $rawLintArgs.Count; $j += 2) {
                if ($rawLintArgs[$j] -cne '-W' -or $rawLintArgs[$j + 1] -cnotin $allowedLints) { throw 'Only explicitly allowed Clippy warnings may follow --.' }
            }
        }
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
    $testTarget = $null
    $libSelection = $false
    $noFailFast = $false
    for ($i = 1; $i -lt $normalizedArgs.Count; $i++) {
        $arg = $normalizedArgs[$i]
        if ($arg -eq '--') {
            if ($sub -eq 'clippy') {
                $lintArgs = @($normalizedArgs[($i + 1)..($normalizedArgs.Count - 1)])
                $allowedLints = @('clippy::too_many_lines','clippy::cognitive_complexity','clippy::too_many_arguments','clippy::type_complexity')
                if ($lintArgs.Count -eq 0 -or ($lintArgs.Count % 2) -ne 0) { throw 'Clippy requires -W and an allowed lint in pairs.' }
                for ($j = 0; $j -lt $lintArgs.Count; $j += 2) {
                    if ($lintArgs[$j] -cne '-W' -or $lintArgs[$j + 1] -cnotin $allowedLints) { throw 'Only explicitly allowed Clippy warnings may follow --.' }
                }
                $i = $normalizedArgs.Count
                break
            }
            if ($sub -ne 'test' -or $i -ge ($normalizedArgs.Count - 1)) { throw 'Only the allowlisted cargo test options are allowed after --.' }
            $harnessArgs = @($normalizedArgs[($i + 1)..($normalizedArgs.Count - 1)])
            if ($harnessArgs.Count -eq 1 -and $harnessArgs[0] -ceq '--list') {
                $list = $true
            } elseif ($harnessArgs.Count -eq 1 -and $harnessArgs[0] -ceq '--exact') {
                $exact = $true
            } else {
                throw 'Only cargo test -- --list or --exact is allowed after --.'
            }
            $i = $normalizedArgs.Count
            break
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
        if ($arg -ceq '-vv') {
            if ($sub -cne 'build') { throw '-vv is only allowed for an explicitly profiled cargo build.' }
            continue
        }
        if ($arg -eq '--no-run') {
            if ($sub -ne 'test') { throw '--no-run is only allowed with cargo test.' }
            $noRun = $true
            continue
        }
        if ($arg -eq '--no-fail-fast') {
            if ($sub -ne 'test') { throw '--no-fail-fast is only allowed with cargo test.' }
            $noFailFast = $true
            continue
        }
        if ($arg -eq '--lib') {
            if ($sub -ne 'test' -or $testTarget) { throw '--lib is only allowed alone as a cargo test target selector.' }
            $libSelection = $true
            continue
        }
        if ($arg -eq '--test') {
            if ($sub -ne 'test' -or $libSelection -or $testTarget -or ($i + 1) -ge $normalizedArgs.Count) { throw '--test requires one test target and cannot be combined with --lib.' }
            $i++
            $testTarget = $normalizedArgs[$i]
            if ($testTarget -notmatch '^[A-Za-z0-9_-]+$') { throw '--test accepts a simple test target name only.' }
            continue
        }
        if ($arg -eq '--all-targets') {
            if ($sub -ne 'clippy') { throw '--all-targets is only allowed with cargo clippy.' }
            continue
        }
        if ($arg -ceq '--message-format=json') {
            if ($sub -ne 'clippy') { throw '--message-format=json is only allowed with cargo clippy.' }
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
        if ($isTest -and -not $arg.StartsWith('-')) {
            if ($null -ne $testFilter) { throw 'Only one test filter is allowed.' }
            $testFilter = $arg
            continue
        }
        if ($arg -match '^-[A-Za-z]') { throw "Unrecognized Cargo option: $arg" }
        if ($arg.StartsWith('-')) { throw "Unrecognized Cargo option: $arg" }
        throw "Unrecognized Cargo argument: $arg"
    }
    if ($sub -eq 'test') {
        if ($null -ne $testFilter -and $hasWorkspace -and $packages.Count -gt 0) { throw 'A cargo test filter cannot be combined with both --workspace and -p.' }
        if ($list -and $noRun) { throw 'cargo test --no-run cannot be combined with --list.' }
        if ($exact -and ($null -eq $testFilter -or $list)) { throw '--exact requires one test filter and cannot be combined with --list.' }
        if (-not $list -and $packages -contains 'whisper-worker-gpu') { throw 'Direct whisper-worker-gpu tests are level 2; only -- --list is allowed through level 1.' }
        if ($noRun -and ($hasWorkspace -or $packages.Count -gt 0)) { $category = 'Compilation' }
        elseif ($noRun) { throw 'cargo test --no-run requires --workspace or -p.' }
    } elseif ($noRun) { throw '--no-run is only allowed with cargo test.' }
    if ($sub -eq 'tree' -and ($release -or $noRun)) { throw 'cargo tree accepts only its allowlisted options.' }
    if ($sub -eq 'metadata' -and ($release -or $noRun)) { throw 'cargo metadata accepts only its allowlisted options.' }
    Assert-AllowedFeatures $features.ToArray() $release
    if ($sub -in @('check','build','clippy') -and -not ($hasWorkspace -or $packages.Count -gt 0)) { throw "$sub requires --workspace or one or more -p members." }
    if ($sub -in $readCommands -and ($hasWorkspace -or $packages.Count -gt 0)) { }
    if (-not $SkipMemberValidation) {
        foreach ($package in $packages) { if ($package -cnotin $Members) { throw "Unknown workspace package: $package" } }
    }
    $result = [Collections.Generic.List[string]]::new()
    $result.Add($sub)
    $result.Add('--offline')
    $result.Add('--locked')
    for ($resultIndex = 1; $resultIndex -lt $normalizedArgs.Count; $resultIndex++) { $result.Add($normalizedArgs[$resultIndex]) }
    return [pscustomobject]@{ Args = @($result.ToArray()); Category = $category }
}

function Get-CargoPackageScope([string[]]$Arguments) {
    $packages = [Collections.Generic.List[string]]::new()
    $workspace = $false
    for ($i = 1; $i -lt $Arguments.Count; $i++) {
        $arg = $Arguments[$i]
        if ($arg -eq '--') { break }
        if ($arg -ceq '--workspace') { $workspace = $true; continue }
        if ($arg -ceq '-p') {
            if (($i + 1) -lt $Arguments.Count) { $i++; $packages.Add($Arguments[$i]) }
            continue
        }
        if ($arg -cmatch '^-p(.+)$') { $packages.Add($Matches[1]) }
    }
    return [pscustomobject]@{ Packages=@($packages.ToArray()); Workspace=$workspace }
}

function Get-PackagePolicyKey([string]$Root) {
    $relative = [IO.Path]::GetRelativePath($repo, (Get-FullPath $Root)).Replace('\','/')
    return $relative.ToLowerInvariant()
}

function Get-CargoInvocationPolicySignature([string[]]$Arguments) {
    $normalized = [Collections.Generic.List[string]]::new()
    for ($i = 0; $i -lt $Arguments.Count; $i++) {
        $argument = $Arguments[$i]
        if ($argument -in @('--offline','--locked')) { continue }
        if ($argument -cmatch '^-p(.+)$') {
            $normalized.Add('-p')
            $normalized.Add($Matches[1])
        } else {
            $normalized.Add($argument)
        }
    }
    return ConvertTo-Json -InputObject ([string[]]$normalized.ToArray()) -Compress
}

function Assert-CargoInvocationPolicy([string[]]$Arguments, [string]$Root) {
    $policyKey = Get-PackagePolicyKey $Root
    if ($policyKey -ceq $dualCpuRebaselineRoot -and -not $script:dualCpuRebaselineEnabled) {
        throw "Cargo lot '$policyKey' requires -DualCpuRebaseline; Cargo not launched."
    }
    if (-not $script:cargoInvocationPolicies.ContainsKey($policyKey)) {
        if ($script:packageSetPolicies.ContainsKey($policyKey) -or $policyKey -ceq 'target/parakeet-tests/garde-n1-categories') { return $false }
        throw "No versioned package policy is registered for lot root '$policyKey'; Cargo not launched."
    }
    $signature = Get-CargoInvocationPolicySignature $Arguments
    foreach ($profile in $script:cargoInvocationPolicies[$policyKey]) {
        if ($signature -ceq $profile) { return $true }
    }
    throw "Cargo invocation does not match a versioned profile for lot root '$policyKey'; Cargo not launched. Arguments: $signature"
}

function Assert-DualCpuRebaselineInvocation(
    [string[]]$Arguments,
    [string]$Root,
    [string]$SelectedFreshTarget,
    [string]$SelectedExistingTarget,
    [bool]$NoCargoTargetDir,
    [string]$DualParakeetCpuWorkerSha256 = '',
    [string]$DualCpuRuntimeSha256 = ''
) {
    if (-not $script:dualCpuRebaselineEnabled) { return }
    $desktopBuild = ConvertTo-Json -InputObject ([string[]]@('build','--release','-p','whisper-desktop','--features','personal-install','-vv')) -Compress
    if ($script:turboEditionEnabled -or
        (Get-FullPath $Root) -ine (Resolve-LotRoot $dualCpuRebaselineRoot) -or
        $SelectedFreshTarget -notin @('dual-cpu-r1-retry-03',$null,'') -or
        $SelectedExistingTarget -notin @('dual-cpu-r1-retry-03',$null,'') -or
        ($SelectedFreshTarget -and $SelectedExistingTarget)) {
        throw '-DualCpuRebaseline requires its exact lot root and retry target name, without another edition profile.'
    }
    $signature = Get-CargoInvocationPolicySignature $Arguments
    if (($DualParakeetCpuWorkerSha256 -or $DualCpuRuntimeSha256) -and $signature -cne $desktopBuild) {
        throw 'Dual worker/runtime SHA inputs are accepted only for the exact desktop build.'
    }
    $n1 = ConvertTo-Json -InputObject ([string[]]@('test','--no-fail-fast')) -Compress
    $fmt = ConvertTo-Json -InputObject ([string[]]@('fmt','--check')) -Compress
    $isN1 = $signature -ceq $n1
    $isFmt = $signature -ceq $fmt
    if ($isN1) {
        if ($SelectedFreshTarget -cne 'dual-cpu-r1-retry-03' -or $SelectedExistingTarget -or -not $NoCargoTargetDir) {
            throw 'Dual CPU rebaseline N1 requires a fresh target and no CARGO_TARGET_DIR.'
        }
        if ($script:personalModel -cne (Get-FullPath 'C:/dev/whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf')) {
            throw 'Dual CPU rebaseline N1 requires the verified personal Parakeet model path.'
        }
        return
    }
    if ($SelectedFreshTarget -or $SelectedExistingTarget -cne 'dual-cpu-r1-retry-03') {
        throw 'Dual CPU rebaseline commands after N1 require the existing dual-cpu-r1-retry-03 target.'
    }
    if ($isFmt) {
        if ($NoCargoTargetDir) { throw 'Dual CPU rebaseline fmt uses the existing target without no-variable mode.' }
        if ($DualParakeetCpuWorkerSha256 -or $DualCpuRuntimeSha256) { throw 'Dual build identities are valid only for the desktop build.' }
        return
    }
    if ($NoCargoTargetDir -ne (-not ($signature -match '"build"'))) {
        throw 'Dual CPU rebaseline target-variable mode does not match the registered command.'
    }
    if ($signature -match 'whisper-worker-parakeet-cpu' -and $signature -match '"test"' -and
        $script:personalModel -cne (Get-FullPath 'C:/dev/whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf')) {
        throw 'Parakeet CPU test profiles require the verified personal model path.'
    }
    if ($signature -ceq $desktopBuild) {
        if ($DualParakeetCpuWorkerSha256 -notmatch '^[0-9a-fA-F]{64}$' -or
            $DualCpuRuntimeSha256.ToLowerInvariant() -cne 'df69460841635b9d3ff080015ee678b8bb74ec08148e04142a234403247dbc31') {
            throw 'The dual desktop build requires the verified Parakeet worker SHA and P7 CPU runtime identity.'
        }
        $workerPath = Join-Path $script:targetRoot 'release/whisper-worker-parakeet-cpu.exe'
        Assert-NoReparseChain $workerPath 'dual Parakeet worker build identity'
        if (-not (Test-Path -LiteralPath $workerPath -PathType Leaf)) { throw 'The guarded Parakeet CPU worker output is absent before the dual desktop build.' }
        $observedWorkerSha = (Get-FileHash -LiteralPath $workerPath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($observedWorkerSha -cne $DualParakeetCpuWorkerSha256.ToLowerInvariant()) {
            throw 'The guarded Parakeet CPU worker SHA differs from the desktop build identity input.'
        }
        $script:authorizedDualParakeetCpuWorkerSha256 = $observedWorkerSha
        $script:authorizedDualCpuRuntimeSha256 = $DualCpuRuntimeSha256.ToLowerInvariant()
        $turboWorkerPath = Join-Path $script:targetRoot 'release/whisper-worker-cpu.exe'
        Assert-NoReparseChain $turboWorkerPath 'dual Turbo CPU worker build identity'
        if (-not (Test-Path -LiteralPath $turboWorkerPath -PathType Leaf)) { throw 'The guarded Turbo CPU worker output is absent before the dual desktop build.' }
        $script:authorizedDesktopCpuWorkerSha256 = (Get-FileHash -LiteralPath $turboWorkerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    } elseif ($DualParakeetCpuWorkerSha256 -or $DualCpuRuntimeSha256) {
        throw 'Dual Parakeet worker/runtime SHA inputs are allowed only for the exact desktop build.'
    }
    $bootstrapBuild = ConvertTo-Json -InputObject ([string[]]@('build','--release','-p','whisper-bootstrap','-vv')) -Compress
    if ($signature -ceq $bootstrapBuild) {
        foreach ($asset in @(
            @{ Name='whisper-desktop.exe'; Variable='WHISPER_DUAL_BOOTSTRAP_DESKTOP_SHA256' },
            @{ Name='whisper-worker-cpu.exe'; Variable='WHISPER_DUAL_BOOTSTRAP_TURBO_CPU_SHA256' },
            @{ Name='whisper-worker-parakeet-cpu.exe'; Variable='WHISPER_DUAL_BOOTSTRAP_PARAKEET_CPU_SHA256' }
        )) {
            $assetPath = Join-Path $script:targetRoot (Join-Path 'release' $asset.Name)
            Assert-NoReparseChain $assetPath 'dual bootstrap executable identity'
            if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) { throw "Dual bootstrap input is absent: $($asset.Name)" }
            $script:authorizedDualBootstrapAssetSha256[$asset.Variable] = (Get-FileHash -LiteralPath $assetPath -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    }
}

function Invoke-DualCpuRebaselinePolicySelfTests {
    $root = Join-Path $repo $dualCpuRebaselineRoot
    $members = @('whisper-core','whisper-adapters','whisper-worker-common','whisper-worker-cpu','whisper-worker-parakeet-cpu','whisper-desktop','whisper-bootstrap')
    $positive = 0
    $negative = 0
    $fixtureTarget = Join-Path $runRoot 'dual-cpu-policy-fixture'
    $fixtureRelease = Join-Path $fixtureTarget 'release'
    Initialize-Directory $fixtureRelease 'dual CPU policy fixture'
    foreach ($name in @('whisper-worker-parakeet-cpu.exe','whisper-worker-cpu.exe','whisper-desktop.exe')) {
        Set-Content -LiteralPath (Join-Path $fixtureRelease $name) -Value "self-test fixture: $name" -Encoding Ascii
    }
    $savedTargetRoot = $script:targetRoot
    $savedDesktopWorkerSha = $script:authorizedDesktopCpuWorkerSha256
    $savedAuthorizedBootstrap = @{}
    foreach ($name in $script:authorizedDualBootstrapAssetSha256.Keys) {
        $savedAuthorizedBootstrap[$name] = $script:authorizedDualBootstrapAssetSha256[$name]
    }
    $script:targetRoot = $fixtureTarget
    $registered = [string[]]$script:cargoInvocationPolicies[$dualCpuRebaselineRoot]
    if ($registered.Count -ne 16) { throw 'Dual CPU rebaseline profile count changed unexpectedly.' }
    try {
        $workerFixture = Join-Path $fixtureRelease 'whisper-worker-parakeet-cpu.exe'
        $workerSha = (Get-FileHash -LiteralPath $workerFixture -Algorithm SHA256).Hash.ToLowerInvariant()
        $runtimeSha = 'df69460841635b9d3ff080015ee678b8bb74ec08148e04142a234403247dbc31'
        $desktopBuild = ConvertTo-Json -InputObject ([string[]]@('build','--release','-p','whisper-desktop','--features','personal-install','-vv')) -Compress
        $n1Args = [string[]]@('test','--no-fail-fast')
        $savedPersonalModel = $script:personalModel
        try {
            $script:personalModel = $null
            $rejected = $false
            try { Assert-DualCpuRebaselineInvocation $n1Args $root 'dual-cpu-r1-retry-03' $null $true } catch { $rejected = $true }
            if (-not $rejected) { throw 'Dual CPU rebaseline SelfTest accepted N1 without the verified personal Parakeet model.' }
            $negative++
        } finally { $script:personalModel = $savedPersonalModel }
        foreach ($profileJson in $registered) {
            $profile = [string[]](Convert-CargoArgsJson $profileJson)
            $checked = Test-CargoArguments $profile $members -SkipMemberValidation
            $candidateRoot = $root
            $fresh = $null
            $existing = 'dual-cpu-r1-retry-03'
            $noVariable = $checked.Args[0] -ne 'build' -and $checked.Args[0] -ne 'fmt'
            if ($profileJson -ceq (ConvertTo-Json -InputObject ([string[]]@('test','--no-fail-fast')) -Compress)) {
                $fresh = 'dual-cpu-r1-retry-03'
                $existing = $null
                $noVariable = $true
            } elseif ($checked.Args[0] -ceq 'fmt') {
                $existing = 'dual-cpu-r1-retry-03'
                $noVariable = $false
            }
            $identity = @{}
            if ((Get-CargoInvocationPolicySignature $checked.Args) -ceq $desktopBuild) {
                $identity = @{ DualParakeetCpuWorkerSha256=$workerSha; DualCpuRuntimeSha256=$runtimeSha }
            }
            Assert-DualCpuRebaselineInvocation $checked.Args $candidateRoot $fresh $existing $noVariable @identity
            if (-not (Assert-CargoInvocationPolicy $checked.Args $candidateRoot)) { throw "Dual CPU rebaseline SelfTest rejected: $profileJson" }
            $positive++
        }
        $turboWorkerFixture = Join-Path $fixtureRelease 'whisper-worker-cpu.exe'
        $turboWorkerFixtureSha = (Get-FileHash -LiteralPath $turboWorkerFixture -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($script:authorizedDesktopCpuWorkerSha256 -cne $turboWorkerFixtureSha) {
            throw 'Dual CPU rebaseline SelfTest did not pin the Turbo CPU worker SHA for the desktop build.'
        }
        $desktopArgs = [string[]]@('build','--release','-p','whisper-desktop','--features','personal-install','-vv')
        $rejected = $false
        try { Assert-DualCpuRebaselineInvocation $desktopArgs $root $null 'dual-cpu-r1-retry-03' $false '' $runtimeSha } catch { $rejected = $true }
        if (-not $rejected) { throw 'Dual CPU rebaseline SelfTest accepted a desktop build without the verified Parakeet worker SHA.' }
        $negative++
        $rejected = $false
        try { Assert-DualCpuRebaselineInvocation $desktopArgs $root $null 'dual-cpu-r1-retry-03' $false $workerSha ('0' * 64) } catch { $rejected = $true }
        if (-not $rejected) { throw 'Dual CPU rebaseline SelfTest accepted an unpinned Parakeet runtime identity.' }
        $negative++
        $rejected = $false
        try { Assert-DualCpuRebaselineInvocation $desktopArgs $root $null 'dual-cpu-r1-retry-03' $false ('b' * 64) $runtimeSha } catch { $rejected = $true }
        if (-not $rejected) { throw 'Dual CPU rebaseline SelfTest accepted a Parakeet worker digest that differs from the fixture.' }
        $negative++
    } finally {
        $script:targetRoot = $savedTargetRoot
        $script:authorizedDesktopCpuWorkerSha256 = $savedDesktopWorkerSha
        foreach ($name in @($script:authorizedDualBootstrapAssetSha256.Keys)) { $script:authorizedDualBootstrapAssetSha256.Remove($name) }
        foreach ($name in $savedAuthorizedBootstrap.Keys) { $script:authorizedDualBootstrapAssetSha256[$name] = $savedAuthorizedBootstrap[$name] }
    }
    $badCases = @(
        @{ Args=@('build','--release','-p','whisper-worker-gpu','-vv'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh=$null; Existing='dual-cpu-r1-retry-03'; NoVariable=$false },
        @{ Args=@('build','--release','-p','whisper-worker-cpu','-vv','--features','native-tests'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh=$null; Existing='dual-cpu-r1-retry-03'; NoVariable=$false },
        @{ Args=@('test','--no-fail-fast'); Root=(Join-Path $repo 'target/parakeet-tests/cpu-distribution-x64-20261115'); Fresh='dual-cpu-r1-retry-03'; Existing=$null; NoVariable=$true },
        @{ Args=@('test','--no-fail-fast'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh=$null; Existing='dual-cpu-r1-retry-03'; NoVariable=$true },
        @{ Args=@('test','--no-fail-fast'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh='dual-cpu-r1'; Existing=$null; NoVariable=$true },
        @{ Args=@('test','--no-fail-fast'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh='dual-cpu-r1-retry-01'; Existing=$null; NoVariable=$true },
        @{ Args=@('test','--no-fail-fast'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh='dual-cpu-r1-retry-02'; Existing=$null; NoVariable=$true },
        @{ Args=@('fmt','--check'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh=$null; Existing='dual-cpu-r1'; NoVariable=$false },
        @{ Args=@('fmt','--check'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh=$null; Existing='dual-cpu-r1-retry-01'; NoVariable=$false },
        @{ Args=@('fmt','--check'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh=$null; Existing='dual-cpu-r1-retry-02'; NoVariable=$false },
        @{ Args=@('fmt','--check','--all'); Root=(Join-Path $repo $dualCpuRebaselineRoot); Fresh=$null; Existing='dual-cpu-r1-retry-03'; NoVariable=$false }
    )
    foreach ($bad in $badCases) {
        $rejected = $false
        try {
            $checked = Test-CargoArguments $bad.Args $members -SkipMemberValidation
            Assert-DualCpuRebaselineInvocation $checked.Args $bad.Root $bad.Fresh $bad.Existing $bad.NoVariable
            [void](Assert-CargoInvocationPolicy $checked.Args $bad.Root)
        } catch { $rejected = $true }
        if (-not $rejected) { throw "Dual CPU rebaseline SelfTest accepted: $($bad.Args -join ' ')" }
        $negative++
    }
    $saved = $script:dualCpuRebaselineEnabled
    try {
        $script:dualCpuRebaselineEnabled = $false
        $rejected = $false
        try { [void](Assert-CargoInvocationPolicy @('test','--offline','--locked','--no-fail-fast') $root) } catch { $rejected = $true }
        if (-not $rejected) { throw 'Dual CPU rebaseline SelfTest accepted the profile without its switch.' }
        $negative++
    } finally { $script:dualCpuRebaselineEnabled = $saved }
    $probe = [Diagnostics.ProcessStartInfo]::new()
    foreach ($name in @($script:dualEditionMarker,'WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256','WHISPER_DUAL_PARAKEET_CPU_RUNTIME_SHA256')) {
        [void]$probe.Environment.Remove($name)
    }
    Set-CargoDualEditionEnvironment $probe.Environment $true
    Assert-CargoDualEditionEnvironment $probe.Environment $true
    Set-CargoDualEditionEnvironment $probe.Environment $false
    Assert-CargoDualEditionEnvironment $probe.Environment $false
    $positive += 2
    $savedDualActive = $script:dualEditionBuildActive
    $savedDesktopWorkerSha = $script:authorizedDesktopCpuWorkerSha256
    $savedDualEnabled = $script:dualCpuRebaselineEnabled
    try {
        $script:dualEditionBuildActive = $false
        $script:authorizedDesktopCpuWorkerSha256 = 'e' * 64
        $probe.Environment['WHISPER_EXPECTED_CPU_WORKER_SHA256'] = $script:authorizedDesktopCpuWorkerSha256
        Assert-CargoDualBuildIdentityEnvironment $probe.Environment $true
        $positive++
        $probe.Environment['WHISPER_EXPECTED_CPU_WORKER_SHA256'] = 'f' * 64
        $rejected = $false
        try { Assert-CargoDualBuildIdentityEnvironment $probe.Environment $true } catch { $rejected = $true }
        if (-not $rejected) { throw 'Dual CPU rebaseline SelfTest accepted a changed desktop worker identity during guarded Cargo metadata.' }
        $negative++
        $desktopArgs = [string[]]@('build','--release','-p','whisper-desktop','--features','personal-install','-vv')
        if (-not (Test-DesktopCpuWorkerInvocation $desktopArgs)) { throw 'Dual CPU rebaseline SelfTest rejected the exact personal-install desktop profile.' }
        $positive++
        $script:dualCpuRebaselineEnabled = $false
        if (Test-DesktopCpuWorkerInvocation $desktopArgs) { throw 'Dual CPU rebaseline SelfTest accepted its desktop profile without the dual switch.' }
        $negative++
    } finally {
        $script:dualEditionBuildActive = $savedDualActive
        $script:authorizedDesktopCpuWorkerSha256 = $savedDesktopWorkerSha
        $script:dualCpuRebaselineEnabled = $savedDualEnabled
    }
    $savedDualActive = $script:dualEditionBuildActive
    $savedWorkerSha = $script:authorizedDualParakeetCpuWorkerSha256
    $savedRuntimeSha = $script:authorizedDualCpuRuntimeSha256
    $savedDesktopWorkerSha = $script:authorizedDesktopCpuWorkerSha256
    try {
        $script:dualEditionBuildActive = $true
        $script:authorizedDesktopCpuWorkerSha256 = 'c' * 64
        $script:authorizedDualParakeetCpuWorkerSha256 = 'a' * 64
        $script:authorizedDualCpuRuntimeSha256 = 'df69460841635b9d3ff080015ee678b8bb74ec08148e04142a234403247dbc31'
        $probe.Environment['WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256'] = $script:authorizedDualParakeetCpuWorkerSha256
        $probe.Environment['WHISPER_DUAL_PARAKEET_CPU_RUNTIME_SHA256'] = $script:authorizedDualCpuRuntimeSha256
        $probe.Environment['WHISPER_EXPECTED_CPU_WORKER_SHA256'] = $script:authorizedDesktopCpuWorkerSha256
        Assert-CargoDualBuildIdentityEnvironment $probe.Environment $true
        $positive++
        $probe.Environment['WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256'] = 'b' * 64
        $rejected = $false
        try { Assert-CargoDualBuildIdentityEnvironment $probe.Environment $true } catch { $rejected = $true }
        if (-not $rejected) { throw 'Dual CPU rebaseline SelfTest accepted a changed worker identity in the Cargo child environment.' }
        $negative++
        $probe.Environment['WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256'] = $script:authorizedDualParakeetCpuWorkerSha256
        $probe.Environment['WHISPER_EXPECTED_CPU_WORKER_SHA256'] = 'd' * 64
        $rejected = $false
        try { Assert-CargoDualBuildIdentityEnvironment $probe.Environment $true } catch { $rejected = $true }
        if (-not $rejected) { throw 'Dual CPU rebaseline SelfTest accepted a changed Turbo worker identity in the Cargo child environment.' }
        $negative++
    } finally {
        $script:dualEditionBuildActive = $savedDualActive
        $script:authorizedDesktopCpuWorkerSha256 = $savedDesktopWorkerSha
        $script:authorizedDesktopCpuWorkerSha256 = $savedDesktopWorkerSha
        $script:authorizedDualParakeetCpuWorkerSha256 = $savedWorkerSha
        $script:authorizedDualCpuRuntimeSha256 = $savedRuntimeSha
    }
    return [pscustomobject]@{ Positive=$positive; Negative=$negative }
}

function Assert-TurboEditionInvocation(
    [string[]]$Arguments,
    [string]$Root,
    [string]$SelectedFreshTarget,
    [string]$SelectedExistingTarget,
    [bool]$NoCargoTargetDir
) {
    if (-not $script:turboEditionEnabled) { return }
    if ((Get-FullPath $Root) -ine (Resolve-LotRoot $script:cpuDistributionRoot) -or
        $SelectedFreshTarget -cnotin @('turbo-edition-release','turbo-edition-final','turbo-edition-l2-final','turbo-edition-cpu-fix') -or
        -not [string]::IsNullOrWhiteSpace($SelectedExistingTarget) -or $NoCargoTargetDir) {
        throw '-TurboEdition requires the CPU distribution lot and a fresh authorized Turbo target.'
    }
    if ($SelectedFreshTarget -ceq 'turbo-edition-cpu-fix' -and
        $script:authorizedDesktopCpuWorkerSha256 -cne 'a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b') {
        throw 'The CPU-fix Turbo target requires the authorized portable CPU worker SHA-256.'
    }
    if ($SelectedFreshTarget -cin @('turbo-edition-final','turbo-edition-l2-final') -and
        $script:authorizedDesktopCpuWorkerSha256 -cne '03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a') {
        throw 'The final Turbo target requires the authorized L1 CPU worker SHA-256.'
    }
    $signature = Get-CargoInvocationPolicySignature $Arguments
    $expected = ConvertTo-Json -InputObject ([string[]]@('build','--release','-p','whisper-desktop','-vv')) -Compress
    if ($signature -cne $expected) {
        throw '-TurboEdition admits only the profiled release build of whisper-desktop.'
    }
}

function Invoke-TurboEditionPolicySelfTests {
    $saved = $script:turboEditionEnabled
    $savedDigest = $script:authorizedDesktopCpuWorkerSha256
    $script:turboEditionEnabled = $true
    $positive = 0
    $negative = 0
    try {
        $valid = @('build','--release','-p','whisper-desktop','-vv','--offline','--locked')
        $resolvedCpuRoot = Resolve-LotRoot $script:cpuDistributionRoot
        Assert-TurboEditionInvocation $valid $resolvedCpuRoot 'turbo-edition-release' $null $false
        $positive++
        $script:authorizedDesktopCpuWorkerSha256 = '03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a'
        Assert-TurboEditionInvocation $valid $resolvedCpuRoot 'turbo-edition-final' $null $false
        $positive++
        Assert-TurboEditionInvocation $valid $resolvedCpuRoot 'turbo-edition-l2-final' $null $false
        $positive++
        $script:authorizedDesktopCpuWorkerSha256 = 'a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b'
        Assert-TurboEditionInvocation $valid $resolvedCpuRoot 'turbo-edition-cpu-fix' $null $false
        $positive++
        $rejected = $false
        try { Assert-AllowedUnset @('WHISPER_EXPECTED_CPU_WORKER_SHA256') } catch { $rejected = $true }
        if (-not $rejected) { throw 'Turbo edition SelfTest accepted unsetting the authorized worker digest.' }
        $negative++
        foreach ($invalidDigest in @($null, ('a' * 64))) {
            $script:authorizedDesktopCpuWorkerSha256 = $invalidDigest
            $rejected = $false
            try { Assert-TurboEditionInvocation $valid $resolvedCpuRoot 'turbo-edition-final' $null $false } catch { $rejected = $true }
            if (-not $rejected) { throw 'Turbo edition SelfTest accepted an unauthorized final worker digest.' }
            $negative++
        }
        foreach ($invalidDigest in @($null, '03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a', ('a' * 64))) {
            $script:authorizedDesktopCpuWorkerSha256 = $invalidDigest
            $rejected = $false
            try { Assert-TurboEditionInvocation $valid $resolvedCpuRoot 'turbo-edition-cpu-fix' $null $false } catch { $rejected = $true }
            if (-not $rejected) { throw 'Turbo edition SelfTest accepted an unauthorized CPU-fix worker digest.' }
            $negative++
        }
        $script:authorizedDesktopCpuWorkerSha256 = 'a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b'
        foreach ($target in @('turbo-edition-final','turbo-edition-l2-final')) {
            $rejected = $false
            try { Assert-TurboEditionInvocation $valid $resolvedCpuRoot $target $null $false } catch { $rejected = $true }
            if (-not $rejected) { throw 'Turbo edition SelfTest let the CPU-fix digest authorize a prior final target.' }
            $negative++
        }
        $script:authorizedDesktopCpuWorkerSha256 = 'a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b'
        $workerCommandRejectedAtSignature = $false
        try {
            Assert-TurboEditionInvocation @('build','--release','-p','whisper-worker-cpu','-vv') $resolvedCpuRoot 'turbo-edition-cpu-fix' $null $false
        } catch {
            $workerCommandRejectedAtSignature = $_.Exception.Message -ceq '-TurboEdition admits only the profiled release build of whisper-desktop.'
        }
        if (-not $workerCommandRejectedAtSignature) {
            throw 'Turbo edition SelfTest did not reject the CPU-fix worker command at its signature check.'
        }
        $negative++
        $script:authorizedDesktopCpuWorkerSha256 = '03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a'
        foreach ($case in @(
            @{ Args=@('build','--release','-p','whisper-worker-cpu','-vv'); Root=$resolvedCpuRoot; Fresh='turbo-edition-release'; Existing=$null; NoTarget=$false },
            @{ Args=$valid; Root=(Resolve-LotRoot 'target/parakeet-tests/main-facades'); Fresh='turbo-edition-release'; Existing=$null; NoTarget=$false },
            @{ Args=$valid; Root=$resolvedCpuRoot; Fresh='cpu-distribution-release'; Existing=$null; NoTarget=$false },
            @{ Args=$valid; Root=$resolvedCpuRoot; Fresh='turbo-edition-l2'; Existing=$null; NoTarget=$false },
            @{ Args=$valid; Root=$resolvedCpuRoot; Fresh='turbo-edition-release'; Existing='turbo-edition-release'; NoTarget=$false },
            @{ Args=$valid; Root=$resolvedCpuRoot; Fresh='turbo-edition-release'; Existing=$null; NoTarget=$true }
        )) {
            $rejected = $false
            try { Assert-TurboEditionInvocation $case.Args $case.Root $case.Fresh $case.Existing $case.NoTarget } catch { $rejected = $true }
            if (-not $rejected) { throw 'Turbo edition SelfTest accepted an unprofiled command or target.' }
            $negative++
        }
    } finally {
        $script:turboEditionEnabled = $saved
        $script:authorizedDesktopCpuWorkerSha256 = $savedDigest
    }
    return [pscustomobject]@{ Positive=$positive; Negative=$negative }
}

function Invoke-TurboEditionEnvironmentSelfTests {
    $saved = [Environment]::GetEnvironmentVariable($script:turboEditionMarker,'Process')
    $positive = 0
    $negative = 0
    try {
        foreach ($invalid in @($null,'','0','generic')) {
            $probe = [Diagnostics.ProcessStartInfo]::new()
            [void]$probe.Environment.Remove($script:turboEditionMarker)
            if ($null -ne $invalid) { $probe.Environment[$script:turboEditionMarker] = $invalid }
            $rejected = $false
            try { Assert-CargoEditionEnvironment $probe.Environment $true } catch { $rejected = $true }
            if (-not $rejected) { throw 'Turbo marker SelfTest accepted an absent or altered value.' }
            $negative++
        }
        $turboProbe = [Diagnostics.ProcessStartInfo]::new()
        [void]$turboProbe.Environment.Remove($script:turboEditionMarker)
        Set-CargoEditionEnvironment $turboProbe.Environment $true
        Assert-CargoEditionEnvironment $turboProbe.Environment $true
        $positive++
        [Environment]::SetEnvironmentVariable($script:turboEditionMarker,'inherited','Process')
        $genericProbe = [Diagnostics.ProcessStartInfo]::new()
        Set-CargoEditionEnvironment $genericProbe.Environment $false
        Assert-CargoEditionEnvironment $genericProbe.Environment $false
        if ([Environment]::GetEnvironmentVariable($script:turboEditionMarker,'Process') -cne 'inherited') {
            throw 'Generic marker SelfTest changed the caller environment.'
        }
        $positive++
        $rejectedUnset = $false
        try { Assert-AllowedUnset @($script:turboEditionMarker) } catch { $rejectedUnset = $true }
        if (-not $rejectedUnset) { throw 'Turbo marker SelfTest accepted an explicit -UnsetEnv conflict.' }
        $negative++
    } finally {
        Set-ProcessEnvironmentValue $script:turboEditionMarker $saved
    }
    return [pscustomobject]@{ Positive=$positive; Negative=$negative }
}

function Assert-CargoPackagePolicy([string[]]$Arguments, [string]$Root) {
    $scope = Get-CargoPackageScope $Arguments
    $policyKey = Get-PackagePolicyKey $Root
    if ($script:cargoInvocationPolicies.ContainsKey($policyKey)) { return $null }
    $isProtectedLot = $script:packageSetPolicies.ContainsKey($policyKey)
    if (-not $isProtectedLot -and $scope.Packages.Count -eq 0 -and -not $scope.Workspace) { return $null }
    if (-not $isProtectedLot) {
        throw "No versioned package policy is registered for lot root '$policyKey'; Cargo not launched."
    }
    if ($scope.Workspace) {
        throw "--workspace is not allowed for package policy '$policyKey'; Cargo not launched."
    }
    if ($scope.Packages.Count -eq 0) {
        throw "Package policy '$policyKey' requires an explicit -p selection; Cargo not launched."
    }
    $expected = @($script:packageSetPolicies[$policyKey])
    $unexpected = @($scope.Packages | Where-Object { $_ -cnotin $expected })
    $missing = @($expected | Where-Object { $_ -cnotin $scope.Packages })
    $duplicates = @($scope.Packages | Group-Object | Where-Object Count -gt 1 | ForEach-Object Name)
    if ($unexpected.Count -gt 0 -or $missing.Count -gt 0 -or $duplicates.Count -gt 0) {
        $details = [Collections.Generic.List[string]]::new()
        if ($unexpected.Count -gt 0) { $details.Add('unexpected=' + [string]::Join(',', [string[]]$unexpected)) }
        if ($missing.Count -gt 0) { $details.Add('missing=' + [string]::Join(',', [string[]]$missing)) }
        if ($duplicates.Count -gt 0) { $details.Add('duplicate=' + [string]::Join(',', [string[]]$duplicates)) }
        throw "Cargo package selection differs from versioned lot policy '$policyKey' ($([string]::Join('; ', [string[]]$details))); Cargo not launched."
    }
    return [pscustomobject]@{ Key=$policyKey; Packages=$expected }
}

function Invoke-GuardedCargoPreflight([string[]]$CommandTokens, [string]$Root, [scriptblock]$CargoMetadataAction) {
    $prechecked = Test-CargoArguments $CommandTokens @() -SkipMemberValidation
    [void](Assert-CargoInvocationPolicy $prechecked.Args $Root)
    [void](Assert-CargoPackagePolicy $prechecked.Args $Root)
    $metadata = & $CargoMetadataAction $prechecked
    return [pscustomobject]@{ Prechecked=$prechecked; Metadata=$metadata }
}

function Assert-WriteRoots {
    if (-not (Test-InRoot $lotRoot $repo)) { throw "Lot root must be inside the worktree: $lotRoot" }
    $lotTargets = Join-Path $lotRoot 'targets'
    if ((Get-FullPath $script:targetRoot) -ine (Get-FullPath $cargoRoot) -and
        (-not (Test-InRoot $script:targetRoot $lotTargets) -or (Get-FullPath $script:targetRoot) -ieq (Get-FullPath $lotTargets))) {
        throw 'Cargo target must be the shared worktree target or a selected target below the lot targets directory.'
    }
    foreach ($path in @($lotRoot, $cargoRoot, $cargoHome, $tempRoot, $scratchRoot, $runRoot, $script:targetRoot)) { Assert-NoReparseChain $path 'write root' }
    if ((Get-FullPath $cargoHome) -ine (Get-FullPath (Join-Path $repo 'target/cargo-home-shared'))) { throw 'CARGO_HOME shared root differs from its declared persistent location.' }
    if (-not (Test-InRoot $tempRoot $lotRoot) -or -not (Test-InRoot $scratchRoot $lotRoot)) { throw 'Campaign roots escaped the lot root.' }
    if ($script:useConfigTarget) {
        if ($null -ne [Environment]::GetEnvironmentVariable('CARGO_TARGET_DIR','Process')) { throw 'CARGO_TARGET_DIR must be absent in no-variable mode.' }
    } elseif ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR) -or (Get-FullPath $env:CARGO_TARGET_DIR) -ine (Get-FullPath $script:targetRoot)) { throw 'CARGO_TARGET_DIR differs from the selected target.' }
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA')) {
        $value = [Environment]::GetEnvironmentVariable($name)
        if ([string]::IsNullOrWhiteSpace($value) -or -not (Test-InRoot $value $lotRoot)) { throw "$name is missing or outside the lot root." }
        Assert-NoReparseChain $value $name
    }
    if ([string]::IsNullOrWhiteSpace($env:CARGO_HOME) -or (Get-FullPath $env:CARGO_HOME) -ine (Get-FullPath $cargoHome)) { throw 'CARGO_HOME differs from its declared persistent location.' }
    Assert-NoReparseChain $env:CARGO_HOME 'CARGO_HOME'
    if ($env:CLIPPY_CONF_DIR) {
        if (-not [IO.Path]::IsPathFullyQualified($env:CLIPPY_CONF_DIR)) { throw 'CLIPPY_CONF_DIR must be an absolute path.' }
        if (-not (Test-InRoot $env:CLIPPY_CONF_DIR $lotRoot)) { throw 'CLIPPY_CONF_DIR must be inside the lot root.' }
        Assert-NoReparseChain $env:CLIPPY_CONF_DIR 'CLIPPY_CONF_DIR'
        if (-not (Test-Path -LiteralPath $env:CLIPPY_CONF_DIR -PathType Container)) { throw 'CLIPPY_CONF_DIR must exist.' }
    }
    if (-not (Test-InRoot $env:TEMP $tempRoot) -or -not (Test-InRoot $env:TMP $tempRoot)) { throw 'TEMP/TMP are not isolated below the lot root.' }
    if (-not (Test-InRoot $env:LOCALAPPDATA $runRoot)) { throw 'LOCALAPPDATA is not isolated under this run root.' }
    if ($env:CARGO_INCREMENTAL -cne '0' -or $env:CARGO_PROFILE_DEV_DEBUG -cne 'line-tables-only') { throw 'Cargo profile environment differs from level-1 settings.' }
    if ((Get-PSDrive -Name C).Free -lt 30GB) { throw 'C: has less than 30 GiB free; Cargo is forbidden.' }
    if (@(Get-ChildItem -LiteralPath $env:LOCALAPPDATA -Force).Count -ne 0) { throw 'LOCALAPPDATA must be empty before every run.' }
    foreach ($entry in Get-ChildItem Env:) {
        if ($entry.Name -ceq $script:turboEditionMarker) { continue }
        if ($entry.Name -ceq $script:dualEditionMarker -and $script:dualEditionBuildActive -and $entry.Value -ceq '1') { continue }
        if ($entry.Name -ceq 'WHISPER_EXPECTED_CPU_WORKER_SHA256' -and
            $script:authorizedDesktopCpuWorkerSha256 -and
            $entry.Value -ceq $script:authorizedDesktopCpuWorkerSha256) { continue }
        $dualExpected = switch -CaseSensitive ($entry.Name) {
            'WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256' { $script:authorizedDualParakeetCpuWorkerSha256 }
            'WHISPER_DUAL_PARAKEET_CPU_RUNTIME_SHA256' { $script:authorizedDualCpuRuntimeSha256 }
            default { $script:authorizedDualBootstrapAssetSha256[$entry.Name] }
        }
        if ($dualExpected -and $entry.Value -ceq $dualExpected) { continue }
        if ($entry.Name -match '^(WHISPER_|PARAKEET_)') { throw "Campaign variable was not cleared: $($entry.Name)" }
    }
    if ([Environment]::GetEnvironmentVariable('N1_TEST_PERSONAL_PARAKEET_MODEL','Process') -cne $script:personalModel) { throw 'Personal model test path differs from the validated selection.' }
}

function Invoke-Captured([string]$Executable, [string[]]$Arguments, [string]$Label, [switch]$CargoInvocation) {
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
    Set-CargoEditionEnvironment $start.Environment ([bool]($script:turboEditionEnabled -and $CargoInvocation))
    Set-CargoDualEditionEnvironment $start.Environment ([bool]($script:dualEditionBuildActive -and $CargoInvocation))
    if ($CargoInvocation) {
        Assert-CargoEditionEnvironment $start.Environment $script:turboEditionEnabled
        Assert-CargoDualEditionEnvironment $start.Environment ([bool]$script:dualEditionBuildActive)
    }
    Assert-CargoDualBuildIdentityEnvironment $start.Environment $CargoInvocation
    if ($script:useConfigTarget) {
        [void]$start.Environment.Remove('CARGO_TARGET_DIR')
        if ($start.Environment.ContainsKey('CARGO_TARGET_DIR')) { throw 'CARGO_TARGET_DIR remains in the Cargo child environment; process launch refused.' }
    }
    foreach ($argument in $Arguments) { [void]$start.ArgumentList.Add($argument) }
    [IO.File]::WriteAllText($outPath,'',$utf8NoBom)
    [IO.File]::WriteAllText($errPath,'',$utf8NoBom)
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
        return [pscustomobject]@{ ExitCode=$process.ExitCode; Stdout=$stdout; Stderr=$stderr; OutPath=$outPath; ErrPath=$errPath; CargoTargetEnvPresent=$start.Environment.ContainsKey('CARGO_TARGET_DIR') }
    } finally { $process.Dispose() }
}

function Invoke-CargoCaptured([string[]]$Arguments, [string]$Label) {
    $script:cargoProcessLaunchCount++
    return Invoke-Captured (Get-Command cargo -ErrorAction Stop).Source $Arguments $Label -CargoInvocation
}

function Get-CargoTestExecutablePaths([string[]]$OutputStreams, [string]$TargetRoot) {
    $paths = [Collections.Generic.List[string]]::new()
    foreach ($stream in $OutputStreams) {
      foreach ($line in ($stream -split "`r?`n")) {
        # Cargo's human output (which may be emitted on stderr) is accepted.
        # It is `Executable <target> (<path>)`; capture through the final .exe so
        # spaces or parentheses in earlier display text do not alter the path.
        $rawPath = $null
        if ($line -match '^\s*Executable\((.+\.exe)\)\s*$') { $rawPath = $Matches[1].Trim().Trim('"') }
        elseif ($line -match '^\s*Executable\s+.+\s+\((.+\.exe)\)\s*$') { $rawPath = $Matches[1].Trim().Trim('"') }
        if ([string]::IsNullOrWhiteSpace($rawPath)) { continue }
        $path = if ([IO.Path]::IsPathRooted($rawPath)) { Get-FullPath $rawPath } else { Get-FullPath (Join-Path $repo $rawPath) }
        if ([IO.Path]::GetExtension($path) -ine '.exe') { throw "Cargo test executable is not a Windows PE path: $path" }
        if (-not (Test-InRoot $path $TargetRoot)) { throw "Cargo test executable escaped the selected target root: $path" }
        Assert-NoReparseChain $path 'Cargo test executable'
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Cargo test executable is missing: $path" }
        if (-not @($paths | Where-Object { $_ -ieq $path }).Count) { $paths.Add($path) }
      }
    }
    if ($paths.Count -eq 0) { throw 'cargo test --no-run reported no executable harness; refusing worker test execution.' }
    return ,([string[]]$paths.ToArray())
}

function Invoke-WorkerHarnessImportGate([string]$Package, [string[]]$ExecutablePaths, [string]$LabelPrefix, [scriptblock]$CheckAction = $null) {
    if ($Package -notin @('whisper-worker-cpu','whisper-worker-parakeet-cpu','whisper-worker-common')) { throw "No N1 worker import-gate mapping for package '$Package'." }
    if ($ExecutablePaths.Count -eq 0) { throw "No test harness was supplied for '$Package'." }
    $kind = switch ($Package) {
        'whisper-worker-cpu' { 'Cpu' }
        'whisper-worker-parakeet-cpu' { 'ParakeetCpu' }
        'whisper-worker-common' { 'Cpu' }
    }
    $checker = Join-Path $repo 'ci/check-worker-imports.ps1'
    $powerShell = (Get-Process -Id $PID).Path
    $index = 0
    foreach ($path in $ExecutablePaths) {
        $fullPath = Get-FullPath $path
        if (-not (Test-InRoot $fullPath $script:targetRoot)) { throw "Worker test harness escaped the selected Cargo target: $fullPath" }
        Assert-NoReparseChain $fullPath 'worker test harness'
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) { throw "Worker test harness is missing: $fullPath" }
        $result = if ($null -ne $CheckAction) {
            & $CheckAction $kind $fullPath
        } else {
            Invoke-Captured $powerShell @('-NoProfile','-File',$checker,'-Kind',$kind,'-Path',$fullPath) ("$LabelPrefix-$Package-$index")
        }
        if ($null -eq $result -or $result.ExitCode -ne 0) { throw "PE import gate failed for '$Package' harness '$fullPath'; refusing inventory or test execution." }
        $sha256 = (Get-FileHash -LiteralPath $fullPath -Algorithm SHA256).Hash.ToLowerInvariant()
        Write-Host "WORKER_HARNESS_PE package=$Package kind=$kind path=$fullPath sha256=$sha256 verdict=PASS"
        $index++
    }
    return $index
}

function Invoke-WorkerHarnessPreflight([string]$Package, [string]$Phase) {
    $preflightArgs = [string[]]@('test','--offline','--locked','--no-run','-p',$Package)
    [void](Assert-CargoInvocationPolicy $preflightArgs $lotRoot)
    [void](Assert-CargoPackagePolicy $preflightArgs $lotRoot)
    if ($script:useConfigTarget) { $preflightArgs += @('--config',('build.target-dir=' + (ConvertTo-Json -Compress $script:targetRoot))) }
    Assert-WriteRoots
    $preflight = Invoke-CargoCaptured $preflightArgs ("worker-harness-$Phase-$Package")
    if ($preflight.ExitCode -ne 0) { throw "Worker harness preflight build failed for '$Package' ($($preflight.ExitCode)); inventory/test not launched. Logs: $($preflight.OutPath), $($preflight.ErrPath)" }
    $executables = Get-CargoTestExecutablePaths @($preflight.Stdout,$preflight.Stderr) $script:targetRoot
    $checked = Invoke-WorkerHarnessImportGate $Package $executables "worker-harness-$Phase"
    "WORKER_HARNESS_IMPORT_GATE package=$Package checked=$checked phase=$Phase target=$script:targetRoot"
}

$script:structuralClippyLints = @(
    'clippy::too_many_lines',
    'clippy::cognitive_complexity',
    'clippy::too_many_arguments',
    'clippy::type_complexity'
)

function Test-StructuralClippyInvocation([string[]]$Arguments) {
    if ($Arguments.Count -eq 0 -or $Arguments[0] -cne 'clippy' -or '--message-format=json' -cnotin $Arguments) { return $false }
    $delimiter = [Array]::IndexOf($Arguments,'--')
    if ($delimiter -lt 0) { return $false }
    $lintArgs = @($Arguments[($delimiter + 1)..($Arguments.Count - 1)])
    foreach ($lint in $script:structuralClippyLints) {
        $found = $false
        for ($i = 0; $i + 1 -lt $lintArgs.Count; $i += 2) {
            if ($lintArgs[$i] -ceq '-W' -and $lintArgs[$i + 1] -ceq $lint) { $found = $true; break }
        }
        if (-not $found) { return $false }
    }
    return $true
}

function Get-StructuralClippyKey($Diagnostic, [string]$RepositoryRoot) {
    $lint = [string]$Diagnostic.message.code.code
    if ($lint -cnotin $script:structuralClippyLints) { return $null }
    $span = @($Diagnostic.message.spans | Where-Object { $_.is_primary }) | Select-Object -First 1
    if ($null -eq $span -or $null -eq $span.text -or $span.text.Count -lt 1 -or [string]::IsNullOrWhiteSpace([string]$span.text[0].text)) {
        throw "Structural Clippy diagnostic $lint has no primary source text."
    }
    if ($null -eq $span.line_start -or [int]$span.line_start -lt 1) { throw "Structural Clippy diagnostic $lint has no source line." }
    if ($null -eq $span.byte_start -or $null -eq $span.byte_end -or [long]$span.byte_end -lt [long]$span.byte_start) { throw "Structural Clippy diagnostic $lint has no valid primary span byte range." }
    $packageId = [string]$Diagnostic.package_id
    if ([string]::IsNullOrWhiteSpace($packageId)) { throw "Structural Clippy diagnostic $lint has no package id." }
    $package = $packageId
    if ($packageId -match '^path\+.+/([^/#]+)(?:#[^/]*)?$') { $package = [Uri]::UnescapeDataString($Matches[1]) }
    $rawPath = [string]$span.file_name
    if ([string]::IsNullOrWhiteSpace($rawPath)) { throw "Structural Clippy diagnostic $lint has no source path." }
    $root = Get-FullPath $RepositoryRoot
    $resolvedPath = if ([IO.Path]::IsPathRooted($rawPath)) {
        Get-FullPath $rawPath
    } else {
        Get-FullPath (Join-Path $root $rawPath)
    }
    if (-not (Test-InRoot $resolvedPath $root) -or $resolvedPath.Equals($root,[StringComparison]::OrdinalIgnoreCase)) {
        throw "Structural Clippy path is outside the worktree: $rawPath"
    }
    $path = $resolvedPath.Substring($root.Length).TrimStart('\').Replace('\','/')
    if ([string]::IsNullOrWhiteSpace($path) -or ($path -split '/') -contains '..' -or ($path -split '/') -contains '.' -or $path.Contains("`t")) {
        throw "Structural Clippy path is not a normalized relative path: $path"
    }
    $signature = ([regex]::Replace([string]$span.text[0].text,'\s+',' ')).Trim()
    $functionMatch = [regex]::Match($signature,'\bfn\s+([A-Za-z_][A-Za-z0-9_]*)')
    if ($functionMatch.Success) { $signature = "fn $($functionMatch.Groups[1].Value)" }
    else {
        $itemMatch = [regex]::Match($signature,'\b(struct|enum|type|trait|impl)\s+([A-Za-z_][A-Za-z0-9_]*)')
        if ($itemMatch.Success) { $signature = "$($itemMatch.Groups[1].Value) $($itemMatch.Groups[2].Value)" }
        else {
            $signature = [regex]::Replace($signature,'^pub\([^)]*\)\s*','')
            $cut = $signature.Length
            foreach ($marker in @('(','<','{')) {
                $index = $signature.IndexOf($marker,[StringComparison]::Ordinal)
                if ($index -ge 0 -and ($index + 1) -lt $cut) { $cut = $index + 1 }
            }
            $signature = $signature.Substring(0,$cut).Trim()
        }
    }
    if ([string]::IsNullOrWhiteSpace($signature) -or $signature -ceq 'pub(') { throw "Structural Clippy diagnostic $lint has an empty or invalid normalized signature." }
    return "$lint|$package|$path|$signature"
}

function ConvertFrom-StructuralClippyBaseline([string]$Text) {
    $metadata = @{}
    $rows = [Collections.Generic.List[string]]::new()
    foreach ($line in ($Text -split "`r?`n")) {
        if ($line.StartsWith('#')) {
            if ($line -match '^# (source_commit|date|command)=(.*)$') {
                if ($metadata.ContainsKey($Matches[1])) { throw "Clippy structure baseline repeats # $($Matches[1]) metadata." }
                $metadata[$Matches[1]] = $Matches[2]
            }
            continue
        }
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $rows.Add($line)
    }
    foreach ($name in @('source_commit','date','command')) {
        if (-not $metadata.ContainsKey($name) -or [string]::IsNullOrWhiteSpace([string]$metadata[$name])) { throw "Clippy structure baseline is missing # $name metadata." }
    }
    if ($metadata.source_commit -notmatch '^[0-9a-f]{40}$') { throw 'Clippy structure baseline source_commit is not a full Git hash.' }
    if ($metadata.date -notmatch '^\d{4}-\d{2}-\d{2}$') { throw 'Clippy structure baseline date must use YYYY-MM-DD.' }
    $parsedDate = [datetime]::MinValue
    if (-not [datetime]::TryParseExact($metadata.date,'yyyy-MM-dd',[Globalization.CultureInfo]::InvariantCulture,[Globalization.DateTimeStyles]::None,[ref]$parsedDate)) { throw 'Clippy structure baseline date is invalid.' }
    if ($rows.Count -lt 1 -or $rows[0] -cne "key`tcount") { throw 'Clippy structure baseline header must be key<TAB>count.' }
    $counts = @{}
    $previous = $null
    for ($i = 1; $i -lt $rows.Count; $i++) {
        $parts = $rows[$i].Split("`t")
        if ($parts.Count -ne 2 -or [string]::IsNullOrWhiteSpace($parts[0]) -or $parts[0] -notmatch '^(clippy::too_many_lines|clippy::cognitive_complexity|clippy::too_many_arguments|clippy::type_complexity)\|') { throw "Invalid Clippy structure baseline row $($i + 1)." }
        $keyParts = $parts[0].Split([char]'|',4,[StringSplitOptions]::None)
        if ($keyParts.Count -ne 4 -or [string]::IsNullOrWhiteSpace($keyParts[1]) -or [string]::IsNullOrWhiteSpace($keyParts[2]) -or [string]::IsNullOrWhiteSpace($keyParts[3])) { throw "Invalid Clippy structure baseline key on row $($i + 1)." }
        if ($keyParts[2].Contains('\') -or [IO.Path]::IsPathRooted($keyParts[2]) -or ($keyParts[2] -split '/') -contains '..' -or ($keyParts[2] -split '/') -contains '.') { throw "Clippy structure baseline path is not normalized and relative on row $($i + 1)." }
        if ($keyParts[3] -match '\s{2,}' -or $keyParts[3] -ne $keyParts[3].Trim()) { throw "Clippy structure baseline signature is not whitespace-normalized on row $($i + 1)." }
        $count = 0
        if (-not [int]::TryParse($parts[1],[ref]$count) -or $count -lt 1) { throw "Invalid Clippy structure baseline count on row $($i + 1)." }
        if ($null -ne $previous -and [StringComparer]::Ordinal.Compare($previous,$parts[0]) -ge 0) { throw 'Clippy structure baseline keys must be unique and ordinally sorted.' }
        if ($counts.ContainsKey($parts[0])) { throw "Duplicate Clippy structure baseline key: $($parts[0])" }
        $counts[$parts[0]] = $count
        $previous = $parts[0]
    }
    return [pscustomobject]@{ Counts=$counts; Metadata=$metadata }
}

function Read-StructuralClippyBaseline([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw 'Clippy structure baseline is missing; guarded Clippy fails closed.' }
    return [IO.File]::ReadAllText($Path)
}

function Get-StructuralClippyCounts([string]$JsonLines, [string]$RepositoryRoot) {
    $counts = @{}
    $emissions = @{}
    foreach ($line in ($JsonLines -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $record = ConvertFrom-Json -InputObject $line -ErrorAction Stop } catch { throw "Clippy JSON output is invalid: $($_.Exception.Message)" }
        if ($record.reason -cne 'compiler-message' -or $null -eq $record.message.code -or $record.message.code.code -cnotin $script:structuralClippyLints) { continue }
        $key = Get-StructuralClippyKey $record $RepositoryRoot
        $span = @($record.message.spans | Where-Object { $_.is_primary }) | Select-Object -First 1
        $packageId = [string]$record.package_id
        $package = $packageId
        if ($packageId -match '^path\+.+/([^/#]+)(?:#[^/]*)?$') { $package = [Uri]::UnescapeDataString($Matches[1]) }
        $path = ([string]$span.file_name).Replace('\','/')
        $emission = "$($record.message.code.code)|$package|$path|$($span.byte_start)|$($span.byte_end)|$($record.message.message)"
        if ($emissions.ContainsKey($emission)) { continue }
        $emissions[$emission] = $true
        if (-not $counts.ContainsKey($key)) { $counts[$key] = 0 }
        $counts[$key]++
    }
    return $counts
}

function Compare-StructuralClippy([string]$JsonLines, [string]$BaselineText, [string]$RepositoryRoot) {
    $baseline = ConvertFrom-StructuralClippyBaseline $BaselineText
    $current = Get-StructuralClippyCounts $JsonLines $RepositoryRoot
    $newSites = [Collections.Generic.List[string]]::new()
    foreach ($key in $current.Keys) {
        $accepted = if ($baseline.Counts.ContainsKey($key)) { [int]$baseline.Counts[$key] } else { 0 }
        if ([int]$current[$key] -gt $accepted) { $newSites.Add("$key (current=$($current[$key]), baseline=$accepted)") }
    }
    return [pscustomobject]@{ NewSites=@($newSites.ToArray()); Current=$current; Baseline=$baseline }
}

function Assert-StructuralClippyPassed([int]$ExitCode) {
    if ($ExitCode -ne 0) { throw "Guarded Cargo command failed with exit code $ExitCode." }
}

function New-StructuralClippyFixture([string]$Lint, [string]$Path, [string]$Signature, [int]$Line, [string]$TargetKind='lib', [int]$ByteStart=0) {
    $name = switch ($TargetKind) { 'lib' { 'fixture_lib' } 'test' { 'fixture_test' } default { 'fixture_other' } }
    return [pscustomobject]@{
        reason='compiler-message'
        package_id='path+file:///C:/fixture/crates/fixture-package#0.1.0'
        target=[pscustomobject]@{ name=$name; kind=@($TargetKind) }
        message=[pscustomobject]@{
            code=[pscustomobject]@{ code=$Lint }
            spans=@([pscustomobject]@{ is_primary=$true; file_name=$Path; line_start=$Line; line_end=$Line; byte_start=$ByteStart; byte_end=($ByteStart + $Signature.Length); text=@([pscustomobject]@{ text=$Signature }) })
        }
    } | ConvertTo-Json -Depth 8 -Compress
}

function Invoke-StructuralClippySelfTests {
    $baseSignature = 'pub(crate) fn convert(value: usize) {'
    $baseRecord = New-StructuralClippyFixture 'clippy::too_many_lines' 'crates/fixture/src/lib.rs' $baseSignature 10
    $movedRecord = New-StructuralClippyFixture 'clippy::too_many_lines' 'crates/fixture/src/lib.rs' $baseSignature 99
    $key = 'clippy::too_many_lines|fixture-package|crates/fixture/src/lib.rs|fn convert'
    $baseline = "# source_commit=$('a' * 40)`n# date=2026-11-11`n# command=cargo clippy fixture`nkey`tcount`n$key`t1"
    $positive = 0
    $negative = 0
    foreach ($case in @(
        @{ Name='unchanged'; Lines=@($baseRecord); Expected=0 },
        @{ Name='retired'; Lines=@(); Expected=0 },
        @{ Name='moved'; Lines=@($movedRecord); Expected=0 },
        @{ Name='multi-target duplicate'; Lines=@($baseRecord,(New-StructuralClippyFixture 'clippy::too_many_lines' 'crates/fixture/src/lib.rs' $baseSignature 10 'test')); Expected=0 },
        @{ Name='out-of-scope lint'; Lines=@((New-StructuralClippyFixture 'clippy::needless_return' 'crates/fixture/src/lib.rs' $baseSignature 10)); Expected=0 }
    )) {
        $result = Compare-StructuralClippy ($case.Lines -join "`n") $baseline $repo
        if ($result.NewSites.Count -ne $case.Expected) { throw "Structural Clippy SelfTest failed for $($case.Name)." }
        $positive++
    }
    $existingFunction = New-StructuralClippyFixture 'clippy::too_many_lines' 'crates/fixture/src/lib.rs' 'pub(crate) fn a(value: usize) {' 30 'lib' 0
    $newFunction = New-StructuralClippyFixture 'clippy::too_many_lines' 'crates/fixture/src/lib.rs' 'pub(crate) fn b(value: usize) {' 60 'lib' 100
    $functionBaselineKey = 'clippy::too_many_lines|fixture-package|crates/fixture/src/lib.rs|fn a'
    $functionBaseline = "# source_commit=$('a' * 40)`n# date=2026-11-11`n# command=cargo clippy fixture`nkey`tcount`n$functionBaselineKey`t1"
    $functionComparison = Compare-StructuralClippy (@($existingFunction,$newFunction) -join "`n") $functionBaseline $repo
    if ($functionComparison.NewSites.Count -ne 1 -or $functionComparison.NewSites[0] -notmatch '\|fn b \(current=1, baseline=0\)') { throw "Structural Clippy SelfTest missed a new pub(crate) function in a file with an existing function: $([string]::Join('; ',[string[]]$functionComparison.NewSites))" }
    $positive++
    $restrictedFunction = Get-StructuralClippyKey ((New-StructuralClippyFixture 'clippy::too_many_lines' 'crates/fixture/src/lib.rs' 'pub(crate) fn a(value: usize) {' 30 | ConvertFrom-Json)) $repo
    $publicFunction = Get-StructuralClippyKey ((New-StructuralClippyFixture 'clippy::too_many_lines' 'crates/fixture/src/lib.rs' 'pub fn a(value: usize) {' 60 | ConvertFrom-Json)) $repo
    if ($restrictedFunction -cne $publicFunction -or -not $restrictedFunction.EndsWith('|fn a',[StringComparison]::Ordinal)) { throw 'Structural Clippy SelfTest did not normalize visibility for the same function name.' }
    $positive++
    $traversingInRoot = New-StructuralClippyFixture 'clippy::cognitive_complexity' 'crates/whisper-adapters/../../scripts/recette-build.rs' 'fn main() {' 1 | ConvertFrom-Json
    $traversingInRootKey = Get-StructuralClippyKey $traversingInRoot $repo
    if ($traversingInRootKey -cne 'clippy::cognitive_complexity|fixture-package|scripts/recette-build.rs|fn main') {
        throw "Structural Clippy SelfTest did not normalize an in-root parent path: $traversingInRootKey"
    }
    $positive++
    $escapingRoot = New-StructuralClippyFixture 'clippy::cognitive_complexity' '../../outside.rs' 'fn outside() {' 1 | ConvertFrom-Json
    $rejected = $false
    try { [void](Get-StructuralClippyKey $escapingRoot $repo) } catch { $rejected = $_.Exception.Message.Contains('outside the worktree') }
    if (-not $rejected) { throw 'Structural Clippy SelfTest accepted a source path escaping the repository root.' }
    $negative++
    foreach ($case in @(
        @{ Name='new alert'; Lines=@((New-StructuralClippyFixture 'clippy::cognitive_complexity' 'crates/fixture/src/lib.rs' 'fn fresh() {' 20)); Baseline=$baseline; Match='clippy::cognitive_complexity' },
        @{ Name='same-key duplicate occurrence'; Lines=@($baseRecord,(New-StructuralClippyFixture 'clippy::too_many_lines' 'crates/fixture/src/lib.rs' $baseSignature 10 'lib' 100)); Baseline=$baseline; Match='current=2, baseline=1' },
        @{ Name='invalid baseline'; Lines=@($baseRecord); Baseline="# source_commit=bad`n# date=2026-11-11`n# command=cargo clippy`nkey`tcount`n$key`t1"; Match='source_commit' },
        @{ Name='malformed JSON'; Lines=@('{bad json'); Baseline=$baseline; Match='JSON output is invalid' }
    )) {
        $rejected = $false
        try { $result = Compare-StructuralClippy ($case.Lines -join "`n") $case.Baseline $repo; $rejected = $result.NewSites.Count -gt 0 } catch { $rejected = $_.Exception.Message.Contains($case.Match) }
        if (-not $rejected) { throw "Structural Clippy SelfTest accepted $($case.Name)." }
        $negative++
    }
    $missingSpan = New-StructuralClippyFixture 'clippy::too_many_lines' 'crates/fixture/src/lib.rs' $baseSignature 10 | ConvertFrom-Json
    $missingSpan.message.spans[0].text = @()
    $rejected = $false
    try { [void](Compare-StructuralClippy ($missingSpan | ConvertTo-Json -Depth 8 -Compress) $baseline $repo) } catch { $rejected = $_.Exception.Message.Contains('no primary source text') }
    if (-not $rejected) { throw 'Structural Clippy SelfTest accepted a primary span without text.' }
    $negative++
    $rejected = $false
    try { [void](Read-StructuralClippyBaseline (Join-Path $runRoot ('missing-baseline-' + [guid]::NewGuid().ToString('N') + '.tsv'))) } catch { $rejected = $_.Exception.Message.Contains('baseline is missing') }
    if (-not $rejected) { throw 'Structural Clippy SelfTest accepted a missing baseline.' }
    $negative++
    $rejected = $false
    try { Assert-StructuralClippyPassed 101 } catch { $rejected = $_.Exception.Message -match 'exit code 101' }
    if (-not $rejected) { throw 'Structural Clippy SelfTest accepted a failed Cargo command.' }
    $negative++
    $goodArgs = @('clippy','--workspace','--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')
    if (-not (Test-StructuralClippyInvocation $goodArgs) -or (Test-StructuralClippyInvocation @('clippy','--workspace','--message-format=json','--','-W','clippy::too_many_lines'))) { throw 'Structural Clippy SelfTest found an invalid activation boundary.' }
    $positive++
    return [pscustomobject]@{ Positive=$positive; Negative=$negative }
}

function Set-ProcessEnvironmentValue([string]$Name, [AllowNull()][object]$Value) {
    if ($null -eq $Value) {
        # A null passed to the .NET string parameter can leave an empty entry.
        Remove-Item -LiteralPath "Env:$Name" -ErrorAction SilentlyContinue
    } else {
        [Environment]::SetEnvironmentVariable($Name,[string]$Value,'Process')
    }
}

function Set-AuthorizedDesktopCpuWorkerDigest {
    Set-ProcessEnvironmentValue 'WHISPER_EXPECTED_CPU_WORKER_SHA256' $script:authorizedDesktopCpuWorkerSha256
}

function Set-CleanEnvironment {
    foreach ($entry in @(Get-ChildItem Env: | Where-Object { $_.Name -Match '^(WHISPER_.*|PARAKEET_.*)$' -and $_.Name -cne $script:turboEditionMarker })) { Remove-Item "Env:$($entry.Name)" -ErrorAction SilentlyContinue }
    Set-AuthorizedDesktopCpuWorkerDigest
    foreach ($name in $UnsetEnv) { Set-ProcessEnvironmentValue $name $null }
    $script:runRoot = Join-Path $scratchRoot ((Get-Date -Format 'yyyyMMddTHHmmssfff') + '-' + [guid]::NewGuid().ToString('N'))
    foreach ($path in @($script:runRoot,(Join-Path $runRoot 'localappdata'),(Join-Path $tempRoot 'temp'),(Join-Path $tempRoot 'tmp'))) { Initialize-Directory $path 'isolated run directory' }
    $script:cargoTargetDirPresentBeforeClean = $null -ne [Environment]::GetEnvironmentVariable('CARGO_TARGET_DIR','Process')
    foreach ($name in @('CARGO_HOME','CARGO_TARGET_DIR','CARGO_INCREMENTAL','CARGO_PROFILE_DEV_DEBUG','TEMP','TMP','LOCALAPPDATA')) { Set-ProcessEnvironmentValue $name $null }
    $script:cargoTargetDirPresentAfterClean = $null -ne [Environment]::GetEnvironmentVariable('CARGO_TARGET_DIR','Process')
    $env:CARGO_HOME = $cargoHome
    if (-not $script:useConfigTarget) { $env:CARGO_TARGET_DIR = $script:targetRoot }
    $env:CARGO_INCREMENTAL = '0'
    $env:CARGO_PROFILE_DEV_DEBUG = 'line-tables-only'
    $env:TEMP = Join-Path $tempRoot 'temp'
    $env:TMP = Join-Path $tempRoot 'tmp'
    $env:LOCALAPPDATA = Join-Path $runRoot 'localappdata'
    Set-ProcessEnvironmentValue 'N1_TEST_PERSONAL_PARAKEET_MODEL' $script:personalModel
}

function Set-CargoCommandEnvironment([string]$Subcommand, [string[]]$Arguments = @()) {
    if ($Subcommand -eq 'clippy') { Set-ProcessEnvironmentValue 'RUSTFLAGS' $null }
    Set-ProcessEnvironmentValue $script:dualEditionMarker $null
    foreach ($name in @(
        'WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256',
        'WHISPER_DUAL_PARAKEET_CPU_RUNTIME_SHA256',
        'WHISPER_DUAL_BOOTSTRAP_DESKTOP_SHA256',
        'WHISPER_DUAL_BOOTSTRAP_TURBO_CPU_SHA256',
        'WHISPER_DUAL_BOOTSTRAP_PARAKEET_CPU_SHA256'
    )) { Set-ProcessEnvironmentValue $name $null }
    $script:dualEditionBuildActive = $false
    if (-not $script:dualCpuRebaselineEnabled -or $Subcommand -cne 'build') { return }
    $signature = Get-CargoInvocationPolicySignature $Arguments
    $dualBuildProfiles = @(
        (ConvertTo-Json -InputObject ([string[]]@('build','--release','-p','whisper-worker-parakeet-cpu','--features','parakeet-personal','-vv')) -Compress),
        (ConvertTo-Json -InputObject ([string[]]@('build','--release','-p','whisper-desktop','--features','personal-install','-vv')) -Compress),
        (ConvertTo-Json -InputObject ([string[]]@('build','--release','-p','whisper-bootstrap','-vv')) -Compress)
    )
    if ($signature -cnotin $dualBuildProfiles) { return }
    $script:dualEditionBuildActive = $true
    Set-ProcessEnvironmentValue $script:dualEditionMarker '1'
    if ($signature -ceq $dualBuildProfiles[1]) {
        Set-ProcessEnvironmentValue 'WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256' $script:authorizedDualParakeetCpuWorkerSha256
        Set-ProcessEnvironmentValue 'WHISPER_DUAL_PARAKEET_CPU_RUNTIME_SHA256' $script:authorizedDualCpuRuntimeSha256
    }
    if ($signature -ceq $dualBuildProfiles[2]) {
        foreach ($name in $script:authorizedDualBootstrapAssetSha256.Keys) {
            Set-ProcessEnvironmentValue $name $script:authorizedDualBootstrapAssetSha256[$name]
        }
    }
}

function Initialize-Roots {
    if ((Get-PSDrive -Name C).Free -lt 30GB) { throw 'C: has less than 30 GiB free; campaign setup is forbidden.' }
    $roots = @($lotRoot,$cargoRoot,$cargoHome,$tempRoot,$scratchRoot)
    $roots += $script:targetRoot
    foreach ($path in $roots) { Assert-NoReparseChain $path 'lot root preflight' }
    foreach ($path in @($lotRoot,$tempRoot,$scratchRoot)) { Initialize-Directory $path 'lot root' }
}

function Assert-OfflineCache([string]$CacheRoot) {
    Assert-NoReparseChain $CacheRoot 'persistent Cargo cache'
    if (-not (Test-Path -LiteralPath (Join-Path $CacheRoot 'registry/src') -PathType Container)) {
        throw "Offline Cargo cache is absent or empty at $CacheRoot. Initialize it through ci/guard-level1.ps1 -CargoArgsJson '[`"fetch`",`"--locked`"]' before an offline command."
    }
}

function Invoke-PackagePolicySelfTests {
    $policyRoot = Join-Path $repo 'target/parakeet-tests/garde-n1-paquets-20261112'
    $expected = @('whisper-core','whisper-adapters','whisper-desktop','whisper-bootstrap','whisper-worker-common','whisper-worker-parakeet-cpu')
    $before = $script:cargoProcessLaunchCount
    $script:packagePolicyCallbackCalls = 0
    $metadataProbe = { param($checked) $script:packagePolicyCallbackCalls++; return [pscustomobject]@{ Args=$checked.Args } }
    $argv = @('clippy')
    foreach ($package in $expected) { $argv += @('-p',$package) }
    $checked = Invoke-GuardedCargoPreflight $argv $policyRoot $metadataProbe
    $positive = 1
    $jsonArgs = Convert-CargoArgsJson ('["clippy",' + (($expected | ForEach-Object { '"-p","' + $_ + '"' }) -join ',') + ']')
    $checkedJson = Invoke-GuardedCargoPreflight $jsonArgs $policyRoot $metadataProbe
    $positive++
    if ($script:packagePolicyCallbackCalls -ne 2) { throw 'Package policy SelfTest did not pass exact selections to the post-authorization metadata boundary.' }
    $beforeRejections = $script:packagePolicyCallbackCalls
    $negative = 0
    foreach ($extraPackage in @('whisper-worker-cpu','whisper-worker-gpu')) {
        $bad = @('clippy')
        foreach ($package in $expected) { $bad += @('-p',$package) }
        $bad += @('-p',$extraPackage)
        $rejected = $false
        try {
            [void](Invoke-GuardedCargoPreflight $bad $policyRoot $metadataProbe)
        } catch { $rejected = $_.Exception.Message.Contains($extraPackage) -and $_.Exception.Message.Contains('Cargo not launched') }
        if (-not $rejected) { throw "Package policy SelfTest accepted extra $extraPackage or omitted the preflight diagnostic." }
        $negative++
        $badJson = Convert-CargoArgsJson ('["clippy",' + (($expected | ForEach-Object { '"-p","' + $_ + '"' }) -join ',') + ',"-p","' + $extraPackage + '"]')
        $rejected = $false
        try {
            [void](Invoke-GuardedCargoPreflight $badJson $policyRoot $metadataProbe)
        } catch { $rejected = $_.Exception.Message.Contains($extraPackage) -and $_.Exception.Message.Contains('Cargo not launched') }
        if (-not $rejected) { throw "JSON package policy SelfTest accepted extra $extraPackage or omitted the preflight diagnostic." }
        $negative++
    }
    foreach ($bad in @(
        @{ Args=@('clippy','-p','whisper-core'); Match='missing=' },
        @{ Args=@('clippy','-p','whisper-core','-p','whisper-core'); Match='duplicate=' },
        @{ Args=@('clippy','--workspace'); Match='--workspace' },
        @{ Args=@('test'); Match='requires an explicit -p selection' }
    )) {
        $rejected = $false
        try {
            [void](Invoke-GuardedCargoPreflight $bad.Args $policyRoot $metadataProbe)
        } catch { $rejected = $_.Exception.Message.Contains($bad.Match) -and $_.Exception.Message.Contains('Cargo not launched') }
        if (-not $rejected) { throw "Package policy SelfTest failed to reject $($bad.Args -join ' ')." }
        $negative++
    }
    $unregisteredRoot = Join-Path $repo 'target/parakeet-tests/unregistered-package-policy'
    foreach ($unregisteredScope in @(@('clippy','-p','whisper-core'),@('clippy','--workspace'))) {
        $rejected = $false
        try { [void](Invoke-GuardedCargoPreflight $unregisteredScope $unregisteredRoot $metadataProbe) }
        catch { $rejected = $_.Exception.Message.Contains('No versioned package policy') -and $_.Exception.Message.Contains('Cargo not launched') }
        if (-not $rejected) { throw "Package policy SelfTest accepted scope under an unregistered lot root: $($unregisteredScope -join ' ')" }
        $negative++
    }
    if ($script:packagePolicyCallbackCalls -ne $beforeRejections) { throw 'Package policy rejection SelfTest reached the Cargo metadata boundary.' }
    if ($script:cargoProcessLaunchCount -ne $before) { throw 'Package policy rejection SelfTest launched a Cargo process.' }
    "PACKAGE_POLICY_SELFTEST cargo_processes=$($script:cargoProcessLaunchCount - $before)"
    return [pscustomobject]@{ Positive=$positive; Negative=$negative }
}

function Invoke-CargoInvocationPolicySelfTests {
    $policyRoot = Join-Path $repo 'target/parakeet-tests/refacto-reprise-import'
    $members = @('whisper-core','whisper-adapters','whisper-worker-common','whisper-worker-cpu','whisper-worker-gpu','whisper-worker-parakeet-cpu','whisper-desktop','whisper-bootstrap')
    $expectedProfiles = [Collections.Generic.List[object]]::new()
    $expectedProfiles.Add([string[]]@('fmt','--check'))
    $expectedProfiles.Add([string[]]@('test','--no-fail-fast'))
    foreach ($filter in @(
        'resume_validation_rejects_later_fifo_row_without_advancing_any_generation',
        'interrupted_',
        'injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume',
        'cancelling_duplicate_import_releases_only_the_new_source_snapshot',
        'stale_duplicate_dialog_response_cannot_submit_a_replacement',
        'r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision',
        'resumed_prefix_copy_retries_after_pending_creation_without_duplication',
        'unified_poll_resumed_prefix_emits_only_new_segments'
    )) { $expectedProfiles.Add([string[]]@('test',$filter)) }
    foreach ($worker in @('whisper-worker-cpu','whisper-worker-parakeet-cpu','whisper-worker-common')) {
        $expectedProfiles.Add([string[]]@('test','-p',$worker))
        $expectedProfiles.Add([string[]]@('test','--no-run','-p',$worker))
        $expectedProfiles.Add([string[]]@('test','-p',$worker,'--','--list'))
    }
    $clippyProfile = [Collections.Generic.List[string]]::new()
    $clippyProfile.Add('clippy')
    foreach ($package in @('whisper-core','whisper-adapters','whisper-desktop','whisper-bootstrap','whisper-worker-common','whisper-worker-parakeet-cpu')) {
        $clippyProfile.Add('-p')
        $clippyProfile.Add($package)
    }
    foreach ($argument in @('--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')) {
        $clippyProfile.Add($argument)
    }
    $expectedProfiles.Add([string[]]$clippyProfile.ToArray())
    if ($expectedProfiles.Count -ne 20) { throw "Invocation policy SelfTest expected 20 profiles, found $($expectedProfiles.Count)." }

    $beforeProcesses = $script:cargoProcessLaunchCount
    $script:invocationPolicyCallbackCalls = 0
    $metadataProbe = { param($checked) $script:invocationPolicyCallbackCalls++; return [pscustomobject]@{ Args=$checked.Args } }
    $positive = 0
    foreach ($profile in $expectedProfiles) {
        $inputVariants = [Collections.Generic.List[object]]::new()
        $inputVariants.Add([string[]]$profile)
        $compact = [Collections.Generic.List[string]]::new()
        for ($i = 0; $i -lt $profile.Count; $i++) {
            if ($profile[$i] -ceq '-p' -and ($i + 1) -lt $profile.Count) {
                $i++
                $compact.Add('-p' + $profile[$i])
            } else {
                $compact.Add($profile[$i])
            }
        }
        if ((Get-CargoPackageScope $profile).Packages.Count -gt 0) { $inputVariants.Add([string[]]$compact.ToArray()) }
        foreach ($tokens in $inputVariants) {
            $checked = Test-CargoArguments ([string[]]$tokens) $members -SkipMemberValidation
            if ($tokens[0] -ceq 'fmt') {
                if (-not (Assert-CargoInvocationPolicy $checked.Args $policyRoot)) { throw 'Invocation policy SelfTest rejected fmt --check.' }
            } else {
                [void](Invoke-GuardedCargoPreflight ([string[]]$tokens) $policyRoot $metadataProbe)
            }
            $json = ConvertTo-Json -InputObject ([string[]]$tokens) -Compress
            $jsonTokens = Convert-CargoArgsJson $json
            $jsonChecked = Test-CargoArguments $jsonTokens $members -SkipMemberValidation
            if ($tokens[0] -ceq 'fmt') {
                if (-not (Assert-CargoInvocationPolicy $jsonChecked.Args $policyRoot)) { throw 'Invocation policy JSON SelfTest rejected fmt --check.' }
            } else {
                [void](Invoke-GuardedCargoPreflight $jsonTokens $policyRoot $metadataProbe)
            }
            $positive += 2
        }
    }
    $expectedCallbacks = $positive - 2
    if ($script:invocationPolicyCallbackCalls -ne $expectedCallbacks) { throw "Invocation policy SelfTest reached the metadata boundary $($script:invocationPolicyCallbackCalls) times; expected $expectedCallbacks." }

    $negative = 0
    $badProfiles = @(
        @{ Args=@('fetch','--locked'); Match='versioned profile' },
        @{ Args=@('test','--no-fail-fast','--release'); Match='versioned profile' },
        @{ Args=@('test','--workspace'); Match='versioned profile' },
        @{ Args=@('test','--manifest-path','crates/whisper-worker-cpu/Cargo.toml'); Match='versioned profile' },
        @{ Args=@('test','-p','whisper-worker-cpu','-p','whisper-worker-common'); Match='versioned profile' },
        @{ Args=@('test','-p','whisper-worker-gpu','--','--list'); Match='versioned profile' },
        @{ Args=@('test','-p','whisper-worker-cpu','--no-run'); Match='versioned profile' },
        @{ Args=@('test','interrupted_model_not_in_the_plan'); Match='versioned profile' },
        @{ Args=@('clippy','-p','whisper-core','-p','whisper-adapters','-p','whisper-worker-cpu','-p','whisper-bootstrap','-p','whisper-worker-common','-p','whisper-worker-parakeet-cpu','--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity'); Match='versioned profile' }
    )
    foreach ($case in $badProfiles) {
        foreach ($useJson in @($false,$true)) {
            $tokens = [string[]]$case.Args
            if ($useJson) { $tokens = Convert-CargoArgsJson (ConvertTo-Json -InputObject $tokens -Compress) }
            $rejected = $false
            try { [void](Invoke-GuardedCargoPreflight $tokens $policyRoot $metadataProbe) }
            catch { $rejected = $_.Exception.Message.Contains($case.Match) -and $_.Exception.Message.Contains('Cargo not launched') }
            if (-not $rejected) { throw "Invocation policy SelfTest did not reject: $($case.Args -join ' ')" }
            $negative++
        }
    }
    foreach ($otherRoot in @(
        (Join-Path $repo 'target/parakeet-tests/refacto-reprise-import-sibling'),
        (Join-Path $policyRoot 'child'),
        (Join-Path $repo 'target/parakeet-tests/unregistered-package-policy')
    )) {
        $rejected = $false
        try { [void](Invoke-GuardedCargoPreflight @('test','-p','whisper-worker-cpu') $otherRoot $metadataProbe) }
        catch { $rejected = $_.Exception.Message.Contains('No versioned package policy') -and $_.Exception.Message.Contains('Cargo not launched') }
        if (-not $rejected) { throw "Invocation policy SelfTest accepted worker selection at '$otherRoot'." }
        $negative++
    }
    if ($script:invocationPolicyCallbackCalls -ne $expectedCallbacks) { throw 'Invocation policy rejection SelfTest reached the metadata boundary.' }
    if ($script:cargoProcessLaunchCount -ne $beforeProcesses) { throw 'Invocation policy SelfTest launched a Cargo process.' }
    $summary = "CARGO_INVOCATION_POLICY_SELFTEST profiles=$($expectedProfiles.Count) positive=$positive negative=$negative cargo_processes=$($script:cargoProcessLaunchCount - $beforeProcesses) metadata_callbacks=$($script:invocationPolicyCallbackCalls)"
    return [pscustomobject]@{ Positive=$positive; Negative=$negative; Summary=$summary }
}

function Invoke-ClippyCorrectionsInvocationPolicySelfTests {
    $policyRoot = Join-Path $repo 'target/parakeet-tests/clippy-corrections-suivantes-20261113'
    $members = @('whisper-core','whisper-adapters','whisper-worker-common','whisper-worker-cpu','whisper-worker-gpu','whisper-worker-parakeet-cpu','whisper-desktop','whisper-bootstrap')
    $profiles = [Collections.Generic.List[object]]::new()
    $profiles.Add([string[]]@('fmt','--check'))
    $profiles.Add([string[]]@('test','--no-fail-fast'))
    foreach ($worker in @('whisper-worker-cpu','whisper-worker-parakeet-cpu','whisper-worker-common')) {
        $profiles.Add([string[]]@('test','--no-run','-p',$worker))
        $profiles.Add([string[]]@('test','-p',$worker,'--','--list'))
        $profiles.Add([string[]]@('test','-p',$worker))
    }
    $clippy = [Collections.Generic.List[string]]::new()
    $clippy.Add('clippy')
    foreach ($package in @('whisper-core','whisper-adapters','whisper-desktop','whisper-bootstrap','whisper-worker-common','whisper-worker-parakeet-cpu')) {
        $clippy.Add('-p'); $clippy.Add($package)
    }
    foreach ($argument in @('--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')) { $clippy.Add($argument) }
    $profiles.Add([string[]]$clippy.ToArray())
    $before = $script:cargoProcessLaunchCount
    $script:clippyCorrectionsPolicyCallbacks = 0
    $probe = { param($checked) $script:clippyCorrectionsPolicyCallbacks++; return [pscustomobject]@{ Args=$checked.Args } }
    $positive = 0
    foreach ($profile in $profiles) {
        $checked = Test-CargoArguments ([string[]]$profile) $members -SkipMemberValidation
        if ($profile[0] -ceq 'fmt') { [void](Assert-CargoInvocationPolicy $checked.Args $policyRoot) }
        else { [void](Invoke-GuardedCargoPreflight ([string[]]$profile) $policyRoot $probe) }
        $positive++
    }
    $badProfiles = @(
        @('test','--workspace'),
        @('test','-p','whisper-worker-gpu','--','--list'),
        @('test','--no-run','-p','whisper-worker-gpu'),
        @('clippy','-p','whisper-core','-p','whisper-adapters','-p','whisper-desktop','-p','whisper-bootstrap','-p','whisper-worker-common','-p','whisper-worker-parakeet-cpu','-p','whisper-worker-cpu','--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')
    )
    $negative = 0
    foreach ($profile in $badProfiles) {
        $rejected = $false
        $failure = $null
        try { [void](Invoke-GuardedCargoPreflight ([string[]]$profile) $policyRoot $probe) }
        catch {
            $failure = $_.Exception.Message
            $rejected = ($failure.Contains('versioned profile') -and $failure.Contains('Cargo not launched')) -or
                ($failure.Contains('level 2') -and $profile -contains 'whisper-worker-gpu')
        }
        if (-not $rejected) { throw "Clippy corrections policy SelfTest failed for '$($profile -join ' ')': $failure" }
        $negative++
    }
    foreach ($outsideRoot in @((Join-Path $repo 'target/parakeet-tests/clippy-corrections-suivantes-sibling'),(Join-Path $policyRoot 'child'))) {
        $rejected = $false
        try { [void](Invoke-GuardedCargoPreflight @('test','-p','whisper-worker-cpu') $outsideRoot $probe) }
        catch { $rejected = $_.Exception.Message.Contains('No versioned package policy') -and $_.Exception.Message.Contains('Cargo not launched') }
        if (-not $rejected) { throw "Clippy corrections policy SelfTest accepted unregistered root: $outsideRoot" }
        $negative++
    }
    if ($script:clippyCorrectionsPolicyCallbacks -ne $positive - 1) { throw 'Clippy corrections policy SelfTest reached metadata unexpectedly.' }
    if ($script:cargoProcessLaunchCount -ne $before) { throw 'Clippy corrections policy SelfTest launched Cargo.' }
    return [pscustomobject]@{ Positive=$positive; Negative=$negative }
}

function Invoke-CpuDistributionInvocationPolicySelfTests {
    $cpuRoot = Join-Path $repo $cpuDistributionRoot
    $gpuRoot = Join-Path $repo $gpuDistributionRoot
    $members = @('whisper-core','whisper-adapters','whisper-worker-common','whisper-worker-cpu','whisper-worker-gpu','whisper-worker-parakeet-cpu','whisper-desktop','whisper-bootstrap')
    $expectedCpuProfiles = [Collections.Generic.List[string]]::new()
    foreach ($profile in @(
        @('fetch'),@('fmt','--check'),@('test','--no-fail-fast'),
        @('test','--no-run','-p','whisper-worker-cpu'),@('test','-p','whisper-worker-cpu','--','--list'),@('test','-p','whisper-worker-cpu'),
        @('test','--no-run','-p','whisper-worker-parakeet-cpu'),@('test','-p','whisper-worker-parakeet-cpu','--','--list'),@('test','-p','whisper-worker-parakeet-cpu'),
        @('test','--no-run','-p','whisper-worker-common'),@('test','-p','whisper-worker-common','--','--list'),@('test','-p','whisper-worker-common'),
        @('build','--release','-p','whisper-worker-cpu','-vv'),@('build','--release','-p','whisper-desktop','-vv'),@('build','--release','-p','whisper-bootstrap','-vv')
    )) { $expectedCpuProfiles.Add((ConvertTo-Json -InputObject ([string[]]$profile) -Compress)) }
    $expectedClippy = [Collections.Generic.List[string]]::new()
    $expectedClippy.Add('clippy')
    foreach ($package in @('whisper-core','whisper-adapters','whisper-desktop','whisper-bootstrap','whisper-worker-cpu','whisper-worker-common','whisper-worker-parakeet-cpu')) { $expectedClippy.Add('-p'); $expectedClippy.Add($package) }
    foreach ($argument in @('--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')) { $expectedClippy.Add($argument) }
    $expectedCpuProfiles.Add((ConvertTo-Json -InputObject ([string[]]$expectedClippy.ToArray()) -Compress))
    $expectedGpuProfiles = [string[]]@((ConvertTo-Json -InputObject ([string[]]@('build','--release','-p','whisper-worker-gpu','-vv')) -Compress))
    if ($expectedCpuProfiles.Count -ne 16 -or $expectedGpuProfiles.Count -ne 1) { throw 'CPU distribution SelfTest expected profile count changed unexpectedly.' }
    if ([string]::Join("`n",[string[]]$expectedCpuProfiles.ToArray()) -cne [string]::Join("`n",[string[]]$script:cargoInvocationPolicies[$cpuDistributionRoot])) { throw 'CPU distribution registered profiles differ from the independently declared SelfTest allowlist.' }
    if ([string]::Join("`n",$expectedGpuProfiles) -cne [string]::Join("`n",[string[]]$script:cargoInvocationPolicies[$gpuDistributionRoot])) { throw 'GPU producer registered profiles differ from the independently declared SelfTest allowlist.' }
    $beforeProcesses = $script:cargoProcessLaunchCount
    $script:cpuDistributionMetadataCallbacks = 0
    $metadataProbe = { param($checked) $script:cpuDistributionMetadataCallbacks++; return [pscustomobject]@{ Args=$checked.Args } }
    $positive = 0
    foreach ($profileJson in $expectedCpuProfiles) {
        $profile = [string[]](Convert-CargoArgsJson $profileJson)
        foreach ($useJson in @($false,$true)) {
            $tokens = $profile
            if ($profile.Count -eq 1 -and $profile[0] -ceq 'fetch') { $tokens = @('fetch','--locked') }
            if ($useJson) { $tokens = Convert-CargoArgsJson (ConvertTo-Json -InputObject ([string[]]$tokens) -Compress) }
            $checked = Test-CargoArguments ([string[]]$tokens) $members -SkipMemberValidation
            if ($checked.Category -in @('Fetch','Lecture')) {
                if (-not (Assert-CargoInvocationPolicy $checked.Args $cpuRoot)) { throw "CPU distribution SelfTest rejected allowlisted $($profile -join ' ')." }
            } else {
                [void](Invoke-GuardedCargoPreflight ([string[]]$tokens) $cpuRoot $metadataProbe)
            }
            $positive++
        }
    }
    foreach ($profileJson in $expectedGpuProfiles) {
        $profile = [string[]](Convert-CargoArgsJson $profileJson)
        foreach ($useJson in @($false,$true)) {
            $tokens = $profile
            if ($useJson) { $tokens = Convert-CargoArgsJson (ConvertTo-Json -InputObject ([string[]]$profile) -Compress) }
            [void](Invoke-GuardedCargoPreflight ([string[]]$tokens) $gpuRoot $metadataProbe)
            $positive++
        }
    }
    $expectedCallbacks = ($positive - 4)
    if ($script:cpuDistributionMetadataCallbacks -ne $expectedCallbacks) { throw "CPU distribution SelfTest reached the metadata boundary $($script:cpuDistributionMetadataCallbacks) times; expected $expectedCallbacks." }

    $negative = 0
    $savedTargetRoot = $script:targetRoot
    $gateRoot = Join-Path $runRoot 'worker-harness-import-gate'
    $gateDeps = Join-Path $gateRoot 'debug/deps'
    Initialize-Directory $gateDeps 'worker harness SelfTest fixture'
    $fixtureHarness = Join-Path $gateDeps 'worker_contract-selftest.exe'
    Copy-Item -LiteralPath (Join-Path $repo 'ci/fixtures/pe/no-cuda.bin') -Destination $fixtureHarness
    $script:targetRoot = $gateRoot
    try {
        $humanStderr = "Executable unittests src/lib.rs ($fixtureHarness)`r`nExecutable tests\worker.rs ($fixtureHarness)"
        $paths = Get-CargoTestExecutablePaths @('', $humanStderr) $gateRoot
        if ($paths.Count -ne 1 -or $paths[0] -ine (Get-FullPath $fixtureHarness)) { throw 'Worker harness SelfTest did not resolve the exact reported PE path.' }
        $successfulChecker = { param($kind,$path) $script:cpuDistributionCheckerCalls++; $script:cpuDistributionCheckerKind=$kind; $script:cpuDistributionCheckerPath=$path; return [pscustomobject]@{ ExitCode=0; Kind=$kind; Path=$path } }
        $script:cpuDistributionCheckerCalls = 0
        foreach ($mapping in @(
            @{ Package='whisper-worker-cpu'; Kind='Cpu' },
            @{ Package='whisper-worker-parakeet-cpu'; Kind='ParakeetCpu' },
            @{ Package='whisper-worker-common'; Kind='Cpu' }
        )) {
            $checkedCount = Invoke-WorkerHarnessImportGate $mapping.Package $paths 'selftest' $successfulChecker
            if ($checkedCount -ne 1 -or $script:cpuDistributionCheckerKind -cne $mapping.Kind -or $script:cpuDistributionCheckerPath -ine (Get-FullPath $fixtureHarness)) { throw "Worker harness SelfTest did not enforce the $($mapping.Kind) mapping for $($mapping.Package): count=$checkedCount kind=$script:cpuDistributionCheckerKind path=$script:cpuDistributionCheckerPath expected=$(Get-FullPath $fixtureHarness)." }
        }
        if ($script:cpuDistributionCheckerCalls -ne 3) { throw 'Worker harness SelfTest did not call the import-check boundary once for each worker package.' }
        $positive += 3

        foreach ($badOutput in @(@('',''), @('',"Executable unittests src/lib.rs ($(Join-Path $gateDeps 'missing.exe'))"))) {
            $rejected = $false
            try { [void](Get-CargoTestExecutablePaths $badOutput $gateRoot) } catch { $rejected = $true }
            if (-not $rejected) { throw 'Worker harness SelfTest accepted absent or missing Cargo test output.' }
            $negative++
        }
        $failedChecker = { param($kind,$path) return [pscustomobject]@{ ExitCode=1; Kind=$kind; Path=$path } }
        $rejected = $false
        try { [void](Invoke-WorkerHarnessImportGate 'whisper-worker-cpu' $paths 'selftest-failure' $failedChecker) } catch { $rejected = $_.Exception.Message.Contains('PE import gate failed') }
        if (-not $rejected) { throw 'Worker harness SelfTest did not stop on an import-check failure.' }
        $negative++
    } finally { $script:targetRoot = $savedTargetRoot }

    $bad = @(
        @{ Root=$cpuRoot; Args=@('test','--workspace'); Match='versioned profile' },
        @{ Root=$cpuRoot; Args=@('test','-p','whisper-worker-cpu','-p','whisper-worker-common'); Match='versioned profile' },
        @{ Root=$cpuRoot; Args=@('build','--release','-p','whisper-worker-cpu'); Match='versioned profile' },
        @{ Root=$cpuRoot; Args=@('build','--release','-p','whisper-worker-gpu','-vv'); Match='versioned profile' },
        @{ Root=$gpuRoot; Args=@('build','--release','-p','whisper-worker-gpu'); Match='versioned profile' },
        @{ Root=$gpuRoot; Args=@('build','--release','-p','whisper-worker-cpu','-vv'); Match='versioned profile' },
        @{ Root=$gpuRoot; Args=@('test','-p','whisper-worker-gpu','--','--list'); Match='versioned profile' },
        @{ Root=$gpuRoot; Args=@('fetch','--locked'); Match='versioned profile' },
        @{ Root=(Join-Path $repo 'target/parakeet-tests/unregistered-cpu-distribution'); Args=@('test'); Match='No versioned package policy' },
        @{ Root=(Join-Path $repo 'target/parakeet-tests/unregistered-cpu-distribution'); Args=@('test','--no-fail-fast'); Match='No versioned package policy' },
        @{ Root=(Join-Path $repo 'target/parakeet-tests/unregistered-cpu-distribution'); Args=@('test','-p','whisper-worker-cpu'); Match='No versioned package policy' },
        @{ Root=(Join-Path $repo 'target/parakeet-tests/unregistered-cpu-distribution'); Args=@('fmt','--check'); Match='No versioned package policy' }
    )
    foreach ($case in $bad) {
        foreach ($useJson in @($false,$true)) {
            $tokens = [string[]]$case.Args
            if ($useJson) { $tokens = Convert-CargoArgsJson (ConvertTo-Json -InputObject ([string[]]$tokens) -Compress) }
            $rejected = $false
            try { [void](Invoke-GuardedCargoPreflight $tokens $case.Root $metadataProbe) }
            catch { $rejected = $_.Exception.Message.Contains($case.Match) -and $_.Exception.Message.Contains('Cargo not launched') }
            if (-not $rejected) { throw "CPU distribution SelfTest did not reject '$($case.Args -join ' ') at '$($case.Root)'." }
            $negative++
        }
    }
    foreach ($badArguments in @(@('build','-p','whisper-worker-cpu','-v'),@('build','-p','whisper-worker-cpu','-vvv'),@('test','-p','whisper-worker-cpu','-vv'))) {
        $rejected = $false
        try { [void](Test-CargoArguments $badArguments $members -SkipMemberValidation) } catch { $rejected = $true }
        if (-not $rejected) { throw "CPU distribution SelfTest accepted invalid verbosity: $($badArguments -join ' ')." }
        $negative++
    }
    if ($script:cpuDistributionMetadataCallbacks -ne $expectedCallbacks) { throw 'CPU distribution negative profile checks reached Cargo metadata.' }
    if ($script:cargoProcessLaunchCount -ne $beforeProcesses) { throw 'CPU distribution policy SelfTest launched a Cargo process.' }
    $summary = "CPU_DISTRIBUTION_POLICY_SELFTEST cpu_profiles=$($expectedCpuProfiles.Count) gpu_profiles=$($expectedGpuProfiles.Count) positive=$positive negative=$negative cargo_processes=$($script:cargoProcessLaunchCount - $beforeProcesses) metadata_callbacks=$($script:cpuDistributionMetadataCallbacks)"
    return [pscustomobject]@{ Positive=$positive; Negative=$negative; Summary=$summary }
}

function Invoke-SelfTests {
    $members = @('whisper-core','whisper-adapters','whisper-worker-common','whisper-worker-cpu','whisper-worker-gpu','whisper-worker-parakeet-cpu')
    $digest = 'a' * 64
    $desktopBuild = @('build','--release','-p','whisper-desktop','-vv')
    if (-not (Test-DesktopCpuWorkerSha256Policy 'Run' $digest $desktopBuild 'target/parakeet-tests/cpu-distribution-x64-20261115')) {
        throw 'Self-test rejected the exact T11 desktop digest build.'
    }
    if (-not (Test-DesktopCpuWorkerInvocation (@('--offline','--locked') + $desktopBuild))) {
        throw 'Self-test rejected the normalized guarded T11 desktop invocation.'
    }
    foreach ($case in @(
        @{ Mode='Run'; Digest='bad'; Args=$desktopBuild; Root='target/parakeet-tests/cpu-distribution-x64-20261115' },
        @{ Mode='Run'; Digest=$digest; Args=@('build','--release','-p','whisper-worker-cpu','-vv'); Root='target/parakeet-tests/cpu-distribution-x64-20261115' },
        @{ Mode='Run'; Digest=$digest; Args=$desktopBuild; Root='target/parakeets/other' },
        @{ Mode='SelfTest'; Digest=$digest; Args=$desktopBuild; Root='target/parakeets/cpu-distribution-x64-20261115' }
    )) {
        if (Test-DesktopCpuWorkerSha256Policy $case.Mode $case.Digest $case.Args $case.Root) {
            throw 'Self-test accepted an invalid T11 desktop digest scope.'
        }
    }
    $savedDigest = $script:authorizedDesktopCpuWorkerSha256
    $savedEnvironmentDigest = [Environment]::GetEnvironmentVariable('WHISPER_EXPECTED_CPU_WORKER_SHA256','Process')
    try {
        $script:authorizedDesktopCpuWorkerSha256 = $digest
        Set-AuthorizedDesktopCpuWorkerDigest
        if ([Environment]::GetEnvironmentVariable('WHISPER_EXPECTED_CPU_WORKER_SHA256','Process') -cne $digest) {
            throw 'Self-test accepted the T11 digest policy but did not pass the digest to the guarded desktop invocation environment.'
        }
        $script:cpuDigestHandoffSelfTestPositive = 1
        $script:authorizedDesktopCpuWorkerSha256 = $null
        Set-AuthorizedDesktopCpuWorkerDigest
        if (Test-Path -LiteralPath 'Env:WHISPER_EXPECTED_CPU_WORKER_SHA256') {
            throw 'Self-test retained an unauthorized desktop worker digest in the guarded invocation environment.'
        }
        $script:cpuDigestHandoffSelfTestNegative = 1
    } finally {
        $script:authorizedDesktopCpuWorkerSha256 = $savedDigest
        Set-ProcessEnvironmentValue 'WHISPER_EXPECTED_CPU_WORKER_SHA256' $savedEnvironmentDigest
    }
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
        @('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end'),
        @('test','-p','whisper-worker-cpu','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact'),
        @('test','--no-fail-fast'),
        @('test','-p','whisper-adapters','--lib'),
        @('test','-p','whisper-adapters','--test','parakeet_runtime_isolation'),
        @('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--release','--','--exact'),
        @('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--no-run','--','--exact'),
        @('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--list'),
        @('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end_typo','--','--exact'),
        @('test','-p','whisper-adapters','worker_ipc::tests::Microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact'),
        @('test','-p','whisper-adapters','microphone_legacy_live_pump','--','--exact'),
        @('test','-p','whisper-adapters','--lib','poll_stopped_manual_gpu_failure_waits_for_choice_and_preserves_pending_request'),
        @('test','-p','whisper-adapters','poll_stopped_manual_gpu_failure_waits_for_choice_and_preserves_pending_request','--','--exact'),
        @('test'),
        @('test','--','--list'),
        @('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact'),
        @('test','--offline','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact','--locked'),
        @('test','-p','whisper-worker-common','--','--list'),
        @('clippy','--workspace','--all-targets'),
        @('clippy','-p','whisper-adapters','--all-targets','--message-format=json','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity'),
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
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact','--nocapture')
    $negative += ,@('test','--','--ignored')
    $negative += ,@('test','--','--include-ignored')
    $negative += ,@('test','-p','whisper-adapters','--lib','--test','parakeet_runtime_isolation')
    $negative += ,@('test','--no-fail-fast','--unknown')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','whisper-worker-common','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','--workspace','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--features','native-tests','--','--exact')
    $negative += ,@('check','-p','whisper-adapters','worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end','--','--exact')
    $negative += ,@('test','-p','whisper-worker-gpu')
    $negative += ,@('test','-p','whisper-worker-gpu','--no-run')
    $negative += ,@('clippy','-p','whisper-adapters','--','-W','clippy::all')
    $negative += ,@('clippy','-p','whisper-adapters','--','-D','clippy::too_many_lines')
    $negative += ,@('clippy','-p','whisper-adapters','--','-W')
    $negative += ,@('clippy','-p','whisper-adapters','--','-W','clippy::too_many_lines','--config','bad')
    $negative += ,@('clippy','-p','whisper-adapters','--','-W','clippy::too_many_lines','--locked')
    $negative += ,@('test','--message-format=json')
    $policyTests = Invoke-PackagePolicySelfTests
    $pCount = 2 + $policyTests.Positive
    $nCount = 3 + $policyTests.Negative
    $invocationPolicyTests = Invoke-CargoInvocationPolicySelfTests
    $invocationPolicyTests.Summary
    $pCount += $invocationPolicyTests.Positive
    $nCount += $invocationPolicyTests.Negative
    $cpuDistributionPolicyTests = Invoke-CpuDistributionInvocationPolicySelfTests
    $cpuDistributionPolicyTests.Summary
    $pCount += $cpuDistributionPolicyTests.Positive
    $nCount += $cpuDistributionPolicyTests.Negative
    if ($script:dualCpuRebaselineEnabled) {
        $dualCpuPolicyTests = Invoke-DualCpuRebaselinePolicySelfTests
        "DUAL_CPU_REBASELINE_POLICY_SELFTEST positive=$($dualCpuPolicyTests.Positive) negative=$($dualCpuPolicyTests.Negative)"
        $pCount += $dualCpuPolicyTests.Positive
        $nCount += $dualCpuPolicyTests.Negative
    }
    $turboEditionPolicyTests = Invoke-TurboEditionPolicySelfTests
    "TURBO_EDITION_POLICY_SELFTEST positive=$($turboEditionPolicyTests.Positive) negative=$($turboEditionPolicyTests.Negative)"
    $pCount += $turboEditionPolicyTests.Positive
    $nCount += $turboEditionPolicyTests.Negative
    $turboEditionEnvironmentTests = Invoke-TurboEditionEnvironmentSelfTests
    "TURBO_EDITION_ENVIRONMENT_SELFTEST positive=$($turboEditionEnvironmentTests.Positive) negative=$($turboEditionEnvironmentTests.Negative)"
    $pCount += $turboEditionEnvironmentTests.Positive
    $nCount += $turboEditionEnvironmentTests.Negative
    if ($script:cpuDigestHandoffSelfTestPositive -ne 1 -or $script:cpuDigestHandoffSelfTestNegative -ne 1) {
        throw 'CPU worker digest handoff SelfTest did not complete both cases.'
    }
    'CPU_WORKER_DIGEST_HANDOFF_SELFTEST positive=1 negative=1'
    $pCount++
    $nCount++
    $fetch = Test-CargoArguments @('fetch','--locked') $members
    if ($fetch.Category -cne 'Fetch' -or ($fetch.Args -join ' ') -cne 'fetch --locked') { throw 'Self-test did not preserve locked fetch without offline.' }
    $pCount++
    $missingCache = Join-Path $runRoot 'absent-cache'
    [void](Test-CargoArguments @('build','--offline','--workspace') $members)
    $cacheRejected = $false
    try { Assert-OfflineCache $missingCache } catch { $cacheRejected = $_.Exception.Message -match 'cache is absent.*fetch' }
    if (-not $cacheRejected -or (Test-Path -LiteralPath $missingCache)) { throw 'Missing offline cache must be rejected with fetch guidance before Cargo.' }
    $nCount++
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
    $modelFixture = Join-Path $runRoot 'parakeet-tdt-0.6b-v3.q8_0.gguf'
    [IO.File]::WriteAllText($modelFixture, 'synthetic personal model')
    $modelSha = (Get-FileHash -LiteralPath $modelFixture -Algorithm SHA256).Hash
    if ((Assert-PersonalParakeetModel $modelFixture $modelSha) -ine (Get-FullPath $modelFixture)) { throw 'Model fixture path changed.' }
    $pCount++
    foreach ($badModel in @('relative/parakeet-tdt-0.6b-v3.q8_0.gguf', (Join-Path $runRoot 'absent/parakeet-tdt-0.6b-v3.q8_0.gguf'), (Join-Path $runRoot 'wrong.gguf'))) {
        $rejected=$false; try { [void](Assert-PersonalParakeetModel $badModel $modelSha) } catch { $rejected=$true }
        if (-not $rejected) { throw "Self-test accepted invalid personal model: $badModel" }; $nCount++
    }
    $rejected=$false; try { [void](Assert-PersonalParakeetModel $modelFixture ('0' * 64)) } catch { $rejected=$true }
    if (-not $rejected) { throw 'Self-test accepted an incorrect personal model SHA.' }; $nCount++
    $rejected=$false
    try { Assert-AllowedUnset @('CUDA_PATH_NOT_V') } catch { $rejected=$true }
    if (-not $rejected) { throw 'Self-test accepted CUDA_PATH_NOT_V.' }
    $nCount++
    $junctionBase = Join-Path $runRoot 'junction-fixture'
    $outsideBase = Join-Path $runRoot 'junction-target'
    Initialize-Directory $outsideBase 'junction self-test target'
    New-Item -ItemType Junction -Path $junctionBase -Target $outsideBase | Out-Null
    $rejected=$false; try { [void](Assert-PersonalParakeetModel (Join-Path $junctionBase 'parakeet-tdt-0.6b-v3.q8_0.gguf') $modelSha) } catch { $rejected=$true }
    if (-not $rejected) { throw 'Self-test accepted model path through a junction.' }; $nCount++
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
    if ((Get-FullPath $cargoRoot) -ine (Get-FullPath (Join-Path $repo 'target/parakeet-tests/cargo-shared'))) { throw 'Self-test found a noncanonical worktree Cargo target.' }
    $freshFixturePath = Get-FreshTargetPath $lotRoot ('selftest-' + [guid]::NewGuid().ToString('N'))
    if (-not (Test-InRoot $freshFixturePath $lotRoot) -or (Get-FullPath $freshFixturePath) -ieq (Get-FullPath $cargoRoot)) { throw 'Fresh target is not distinct and under the lot root.' }
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
        foreach ($name in @('WHISPER_N1_REVIEW','PARAKEET_N1_REVIEW','RUSTFLAGS','CLIPPY_CONF_DIR','N1_TEST_PERSONAL_PARAKEET_MODEL')) { $savedVariables[$name] = [Environment]::GetEnvironmentVariable($name) }
        Set-ProcessEnvironmentValue 'N1_TEST_PERSONAL_PARAKEET_MODEL' 'unvalidated'
        $rejected=$false; try { Assert-WriteRoots } catch { $rejected=$true }
        if (-not $rejected) { throw 'Self-test accepted an unvalidated model environment.' }; $nCount++
        Set-ProcessEnvironmentValue 'N1_TEST_PERSONAL_PARAKEET_MODEL' $script:personalModel
        Assert-WriteRoots; $pCount++
        $confFixture = Join-Path $runRoot 'clippy-conf'
        Initialize-Directory $confFixture 'Clippy configuration fixture'
        $env:CLIPPY_CONF_DIR = $confFixture
        Assert-WriteRoots
        $pCount++
        $previousDirectory = Get-Location
        $previousProcessDirectory = [Environment]::CurrentDirectory
        try {
            Set-Location -LiteralPath $confFixture
            [Environment]::CurrentDirectory = $confFixture
            $env:CLIPPY_CONF_DIR = '.'
            if (-not (Test-InRoot $env:CLIPPY_CONF_DIR $lotRoot)) { throw 'Self-test relative fixture did not resolve inside the lot.' }
            Assert-NoReparseChain $env:CLIPPY_CONF_DIR 'relative Clippy fixture'
            $currentRootCheck = (Get-Command Assert-WriteRoots -CommandType Function).ScriptBlock
            $absoluteRule = "        if (-not [IO.Path]::IsPathFullyQualified(`$env:CLIPPY_CONF_DIR)) { throw 'CLIPPY_CONF_DIR must be an absolute path.' }"
            $previousRootCheck = $currentRootCheck.ToString().Replace($absoluteRule, '')
            if ($previousRootCheck -ceq $currentRootCheck.ToString()) { throw 'Self-test could not construct the previous root check in memory.' }
            try {
                Set-Item -LiteralPath Function:script:Assert-WriteRoots -Value ([scriptblock]::Create($previousRootCheck))
                Assert-WriteRoots
            } finally {
                Set-Item -LiteralPath Function:script:Assert-WriteRoots -Value $currentRootCheck
            }
            $pCount++
            $rejected = $false
            try { Assert-WriteRoots } catch { $rejected = $_.Exception.Message -ceq 'CLIPPY_CONF_DIR must be an absolute path.' }
            if (-not $rejected) { throw 'Self-test did not reject an internal relative CLIPPY_CONF_DIR for the absolute-path rule.' }
            $nCount++
        } finally {
            [Environment]::CurrentDirectory = $previousProcessDirectory
            Set-Location -LiteralPath $previousDirectory
            $env:CLIPPY_CONF_DIR = $confFixture
        }
        $env:CLIPPY_CONF_DIR = Join-Path $repo 'unapproved-clippy-conf'
        $rejected = $false
        try { Assert-WriteRoots } catch { $rejected = $true }
        if (-not $rejected) { throw 'Self-test accepted CLIPPY_CONF_DIR outside the lot.' }
        $nCount++
        Set-ProcessEnvironmentValue 'CLIPPY_CONF_DIR' $null
        Set-ProcessEnvironmentValue 'WHISPER_N1_REVIEW' ''
        if ($null -eq [Environment]::GetEnvironmentVariable('WHISPER_N1_REVIEW','Process') -or
            [Environment]::GetEnvironmentVariable('WHISPER_N1_REVIEW','Process').Length -ne 0) { throw 'Self-test did not preserve a present empty variable.' }
        $pCount++
        Set-ProcessEnvironmentValue 'WHISPER_N1_REVIEW' $null
        if ($null -ne [Environment]::GetEnvironmentVariable('WHISPER_N1_REVIEW','Process')) { throw 'Self-test did not restore an absent variable.' }
        $pCount++
        $savedCargoHome = $env:CARGO_HOME
        try {
            $env:CARGO_HOME = Join-Path $runRoot 'unapproved-cargo-home'
            $rejected=$false
            try { Assert-WriteRoots } catch { $rejected=$true }
            if (-not $rejected) { throw 'Self-test accepted a noncanonical CARGO_HOME.' }
            $nCount++
        } finally { $env:CARGO_HOME = $savedCargoHome }
        foreach ($name in @('WHISPER_N1_REVIEW','PARAKEET_N1_REVIEW')) {
            Set-ProcessEnvironmentValue $name 'not-cleared'
            $rejected=$false
            try { Assert-WriteRoots } catch { $rejected=$true }
            if (-not $rejected) { throw "Self-test accepted surviving $name." }
            Set-ProcessEnvironmentValue $name $null
            if ($null -ne [Environment]::GetEnvironmentVariable($name,'Process')) { throw "Self-test did not remove $name." }
            $nCount++
        }
        [Environment]::SetEnvironmentVariable('RUSTFLAGS','-Dwarnings','Process')
        Set-CargoCommandEnvironment 'clippy'
        if ($null -ne [Environment]::GetEnvironmentVariable('RUSTFLAGS','Process')) { throw 'Self-test did not remove RUSTFLAGS for clippy.' }
        $pCount++
        [Environment]::SetEnvironmentVariable('RUSTFLAGS','-Dwarnings','Process')
        Set-CargoCommandEnvironment 'build'
        if ([Environment]::GetEnvironmentVariable('RUSTFLAGS') -cne '-Dwarnings') { throw 'Self-test changed RUSTFLAGS for a non-clippy command.' }
        $pCount++
    } finally {
        foreach ($name in $savedVariables.Keys) { Set-ProcessEnvironmentValue $name $savedVariables[$name] }
    }
    $savedTargetRoot = $script:targetRoot
    $savedConfigTarget = $script:useConfigTarget
    $oldTargetEnv = [Environment]::GetEnvironmentVariable('CARGO_TARGET_DIR','Process')
    try {
        foreach ($withoutVariable in @($false,$true)) {
            $script:useConfigTarget = $withoutVariable
            foreach ($target in @($cargoRoot,$freshFixturePath)) {
                $script:targetRoot = $target
                Set-ProcessEnvironmentValue 'CARGO_TARGET_DIR' $(if ($withoutVariable) { $null } else { $target })
                Assert-WriteRoots
                $pCount++
                $metadataArgs = @('metadata','--offline','--locked','--no-deps','--format-version','1')
                if ($withoutVariable) {
                    $metadataArgs += @('--config',('build.target-dir=' + (ConvertTo-Json -Compress $target)))
                    # Prove that the process boundary removes even a subsequently injected value.
                    [Environment]::SetEnvironmentVariable('CARGO_TARGET_DIR','C:\invalid-selftest-target','Process')
                }
                $probe = Invoke-CargoCaptured $metadataArgs ('selftest-cargo-target-' + $pCount)
                if ($probe.ExitCode -ne 0) { throw "Self-test cargo metadata failed ($($probe.ExitCode)): $($probe.ErrPath)" }
                if ($probe.CargoTargetEnvPresent -eq $withoutVariable) { throw 'Self-test found an unexpected Cargo child target environment.' }
                $observedTarget = (ConvertFrom-Json $probe.Stdout).target_directory
                if ((Get-FullPath $observedTarget) -ine (Get-FullPath $target)) { throw "Self-test Cargo target_directory mismatch: $observedTarget" }
                "CARGO_CHILD_ENV CARGO_TARGET_DIR_present=$($probe.CargoTargetEnvPresent) target_directory=$observedTarget"
                $pCount++
            }
            foreach ($invalidTarget in @('C:\wb\inspect-ciblee\cargo-target',(Join-Path $repo 'target/unapproved'),(Join-Path $lotRoot 'targets'))) {
                $script:targetRoot = $invalidTarget
                Set-ProcessEnvironmentValue 'CARGO_TARGET_DIR' $(if ($withoutVariable) { $null } else { $invalidTarget })
                $rejected = $false
                try { Assert-WriteRoots } catch { $rejected = $true }
                if (-not $rejected) { throw "Self-test accepted an unapproved target: $invalidTarget" }
                $nCount++
            }
            $script:targetRoot = $cargoRoot
            foreach ($invalidEnv in @('C:\invalid-selftest-target',$(if ($withoutVariable) { $cargoRoot } else { $null }))) {
                Set-ProcessEnvironmentValue 'CARGO_TARGET_DIR' $invalidEnv
                $rejected = $false
                try { Assert-WriteRoots } catch { $rejected = $true }
                if (-not $rejected) { throw 'Self-test accepted an invalid CARGO_TARGET_DIR environment.' }
                $nCount++
            }
        }
    } finally {
        $script:targetRoot = $savedTargetRoot
        $script:useConfigTarget = $savedConfigTarget
        Set-ProcessEnvironmentValue 'CARGO_TARGET_DIR' $oldTargetEnv
    }
    $savedModel = $script:personalModel
    try {
        $script:personalModel = Assert-PersonalParakeetModel $modelFixture $modelSha
        $env:WHISPER_N1_REVIEW = 'must-be-cleared'
        $env:N1_TEST_PERSONAL_PARAKEET_MODEL = 'untrusted-input'
        Set-CleanEnvironment
        Assert-WriteRoots
        $probe = Invoke-Captured (Get-Process -Id $PID).Path @('-NoProfile','-Command', '[pscustomobject]@{ model=$env:N1_TEST_PERSONAL_PARAKEET_MODEL; forbidden=$env:WHISPER_N1_REVIEW; local=$env:LOCALAPPDATA; temp=$env:TEMP; tmp=$env:TMP } | ConvertTo-Json -Compress') 'model-environment-selftest'
        if ($probe.ExitCode -ne 0) { throw 'Model environment child probe failed.' }
        $child = $probe.Stdout | ConvertFrom-Json
        if ($child.model -cne $script:personalModel -or $null -ne $child.forbidden -or $child.local -cne $env:LOCALAPPDATA -or $child.temp -cne $env:TEMP -or $child.tmp -cne $env:TMP) { throw 'Model environment child probe observed incorrect isolation or variables.' }
        $pCount++
    } finally {
        $script:personalModel = $savedModel
        Set-CleanEnvironment
    }
    $clippySelfTests = Invoke-StructuralClippySelfTests
    $pCount += $clippySelfTests.Positive
    $nCount += $clippySelfTests.Negative
    $clippyCorrectionsPolicyTests = Invoke-ClippyCorrectionsInvocationPolicySelfTests
    $pCount += $clippyCorrectionsPolicyTests.Positive
    $nCount += $clippyCorrectionsPolicyTests.Negative
    "CLIPPY_CORRECTIONS_POLICY_SELFTEST positive=$($clippyCorrectionsPolicyTests.Positive) negative=$($clippyCorrectionsPolicyTests.Negative)"
    "CARGO_TARGET_DIR_CLEANUP present_before=$script:cargoTargetDirPresentBeforeClean present_after=$script:cargoTargetDirPresentAfterClean"
    'PASS self-test parser and Clippy ratchet cases: positives={0}, negatives={1}.' -f $pCount,$nCount
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
$script:personalModel = $null
if ($PersonalParakeetModelPath) {
    $script:personalModel = Assert-PersonalParakeetModel $PersonalParakeetModelPath 'e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e'
    if (Test-InRoot $script:personalModel (Join-Path $repo 'target')) { throw 'Personal model must be persistent outside target.' }
}
if ((Get-PSDrive -Name C).Free -lt 30GB) { throw 'C: has less than 30 GiB free; campaign setup is forbidden.' }
$script:useConfigTarget = [bool]$WithoutCargoTargetDir
if ($FreshTarget) {
    if ($ExistingTarget) { throw 'Choose either -FreshTarget or -ExistingTarget, not both.' }
    $script:targetRoot = Get-FreshTargetPath $lotRoot $FreshTarget
    if (Test-Path -LiteralPath $script:targetRoot) { throw "Fresh target already exists: $script:targetRoot" }
    if (-not (Test-InRoot $script:targetRoot $lotRoot)) { throw 'Fresh target escaped the lot root.' }
    Assert-NoReparseChain (Split-Path -Parent $script:targetRoot) 'fresh target parent'
} elseif ($ExistingTarget) { $script:targetRoot = Get-ExistingTargetPath $lotRoot $ExistingTarget }
else { $script:targetRoot = $cargoRoot }
if ($Mode -eq 'Structure' -and $CargoArgs.Count -gt 0) { throw 'Structure mode does not accept Cargo arguments.' }
Assert-TurboEditionInvocation $CargoArgs $lotRoot $FreshTarget $ExistingTarget $WithoutCargoTargetDir
if ($DualCpuRebaseline) {
    if ((Get-FullPath $lotRoot) -ine (Resolve-LotRoot $dualCpuRebaselineRoot)) {
        throw '-DualCpuRebaseline requires -LotRoot target/parakeet-tests/dual-cpu-r1.'
    }
    if (-not $SelfTest -and $CargoArgs.Count -gt 0) {
        Assert-DualCpuRebaselineInvocation $CargoArgs $lotRoot $FreshTarget $ExistingTarget $WithoutCargoTargetDir $DualParakeetCpuWorkerSha256 $DualCpuRuntimeSha256
    }
}
Initialize-Roots
Set-CleanEnvironment
Assert-WriteRoots
if ($SelfTest) { Invoke-SelfTests; exit 0 }
if ($Mode -eq 'Structure') { Invoke-Structure; exit 0 }

$members = @()
if ($CargoArgs.Count -gt 0 -and $CargoArgs[0] -ceq 'fetch') {
    $checked = Test-CargoArguments $CargoArgs $members
    [void](Assert-CargoInvocationPolicy $checked.Args $lotRoot)
    if (-not $DryRun) { Initialize-Directory $cargoHome 'persistent Cargo cache' }
} elseif ($CargoArgs.Count -gt 0 -and $CargoArgs[0] -ceq 'fmt') {
    $checked = Test-CargoArguments $CargoArgs $members
    [void](Assert-CargoInvocationPolicy $checked.Args $lotRoot)
} else {
    $preflight = Invoke-GuardedCargoPreflight $CargoArgs $lotRoot {
        param($prechecked)
        Assert-OfflineCache $cargoHome
        $metadataArgs = @('metadata','--offline','--locked','--no-deps','--format-version','1')
        if ($script:useConfigTarget) { $metadataArgs += @('--config',('build.target-dir=' + (ConvertTo-Json -Compress $script:targetRoot))) }
        $metadataRun = Invoke-CargoCaptured $metadataArgs 'workspace-metadata'
        if ($metadataRun.ExitCode -ne 0) { throw "Guarded cargo metadata failed: $($metadataRun.ExitCode). If dependencies are missing, run guarded cargo fetch --locked." }
        $metadata = $metadataRun.Stdout | ConvertFrom-Json
        if ((Get-FullPath $metadata.target_directory) -ine (Get-FullPath $script:targetRoot)) { throw "Cargo metadata target_directory mismatch: $($metadata.target_directory)" }
        return [pscustomobject]@{ Run=$metadataRun; Members=@($metadata.packages | ForEach-Object name) }
    }
    $metadataRun = $preflight.Metadata.Run
    $members = $preflight.Metadata.Members
    $checked = Test-CargoArguments $CargoArgs $members
}
Set-CargoCommandEnvironment $checked.Args[0] $checked.Args
$script:structuralClippyActive = Test-StructuralClippyInvocation $checked.Args
$cargoArgs = @($checked.Args)
if ($script:useConfigTarget -and $checked.Args[0] -cne 'fmt') {
    if ($checked.Category -eq 'Fetch') { throw 'The no-variable mode is not available for cargo fetch.' }
    $configArgs = @('--config',('build.target-dir=' + (ConvertTo-Json -Compress $script:targetRoot)))
    $delimiter = [Array]::IndexOf([string[]]$cargoArgs,'--')
    if ($delimiter -ge 0) { $cargoArgs = @($cargoArgs[0..($delimiter - 1)]) + $configArgs + @($cargoArgs[$delimiter..($cargoArgs.Count - 1)]) }
    else { $cargoArgs += $configArgs }
}
Assert-WriteRoots
if ($FreshTarget) {
    if (Test-Path -LiteralPath $script:targetRoot) { throw "Metadata created the target that was required to be fresh: $script:targetRoot" }
    Initialize-Directory (Split-Path -Parent $script:targetRoot) 'fresh target parent'
    Initialize-Directory $script:targetRoot 'fresh cargo target'
} elseif (-not (Test-Path -LiteralPath $script:targetRoot -PathType Container)) {
    throw "Shared Cargo target must already exist: $script:targetRoot"
}
if ($DryRun) { "DRY_RUN category=$($checked.Category) cargo $([string]::Join(' ', [string[]]$cargoArgs)) target=$script:targetRoot"; exit 0 }
Assert-WriteRoots
if ($script:authorizedDesktopCpuWorkerSha256) {
    if (-not (Test-DesktopCpuWorkerInvocation $checked.Args)) {
        throw 'Authorized desktop worker digest reached a non-desktop Cargo invocation.'
    }
    "CPU_WORKER_DIGEST_INVOCATION sha256=$script:authorizedDesktopCpuWorkerSha256"
}
if ($checked.Args[0] -ceq 'test' -and '--no-run' -cnotin $checked.Args) {
    $scope = Get-CargoPackageScope $checked.Args
    if ($scope.Packages.Count -eq 1 -and $scope.Packages[0] -in @('whisper-worker-cpu','whisper-worker-parakeet-cpu','whisper-worker-common')) {
        Invoke-WorkerHarnessPreflight $scope.Packages[0] 'before-test'
    }
}
$result = Invoke-CargoCaptured $cargoArgs 'cargo-command'
Get-Content -LiteralPath $result.OutPath -Tail 30
Get-Content -LiteralPath $result.ErrPath -Tail 30
"N1_GUARD category=$($checked.Category) exit_code=$($result.ExitCode) CARGO_TARGET_DIR_child=$($result.CargoTargetEnvPresent) target_directory=$script:targetRoot stdout=$($result.OutPath) stderr=$($result.ErrPath)"
try { Assert-StructuralClippyPassed $result.ExitCode } catch { throw $_ }
if ($result.ExitCode -eq 0 -and $checked.Args[0] -ceq 'test' -and '--no-run' -cin $checked.Args) {
    $scope = Get-CargoPackageScope $checked.Args
    if ($scope.Packages.Count -eq 1 -and $scope.Packages[0] -in @('whisper-worker-cpu','whisper-worker-parakeet-cpu','whisper-worker-common')) {
        $executables = Get-CargoTestExecutablePaths @($result.Stdout,$result.Stderr) $script:targetRoot
        $checkedHarnesses = Invoke-WorkerHarnessImportGate $scope.Packages[0] $executables 'worker-harness-no-run'
        "WORKER_HARNESS_IMPORT_GATE package=$($scope.Packages[0]) checked=$checkedHarnesses phase=no-run target=$script:targetRoot"
    }
}
if ($script:structuralClippyActive) {
    $baselinePath = Join-Path $repo 'ci/clippy-structure-baseline.tsv'
    $baselineText = Read-StructuralClippyBaseline $baselinePath
    $comparison = Compare-StructuralClippy $result.Stdout $baselineText $repo
    if ($comparison.NewSites.Count -gt 0) {
        $details = [string]::Join("`n",[string[]]$comparison.NewSites)
        throw "New structural Clippy alert(s) detected:`n$details"
    }
    $currentSites = [int](($comparison.Current.Values | Measure-Object -Sum).Sum)
    $baselineSites = [int](($comparison.Baseline.Counts.Values | Measure-Object -Sum).Sum)
    "CLIPPY_STRUCTURE_RATCHET PASS keys=$($comparison.Current.Count) sites=$currentSites baseline_keys=$($comparison.Baseline.Counts.Count) baseline_sites=$baselineSites"
}
