param(
    [switch]$SelfTest,
    [string]$LotRoot = 'target/parakeet-tests/cpu-distribution-x64-20261115'
)

$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$expectedLot = 'target/parakeet-tests/cpu-distribution-x64-20261115'
if ($LotRoot -cne $expectedLot) { throw "-LotRoot must be '$expectedLot'." }
$lotPath = [IO.Path]::GetFullPath((Join-Path $repo $LotRoot))
$stagePath = Join-Path $lotPath 'turbo-package-l3-candidate'
$claimPath = Join-Path $lotPath 'turbo-package-l3-candidate.stage-claim'
if (-not (Test-Path -LiteralPath $lotPath -PathType Container)) { throw "The authorized lot root must already exist: $lotPath" }
$resolvedLotPath = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $lotPath).Path).TrimEnd('\')
if (-not $resolvedLotPath.Equals($lotPath.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)) {
    throw "The authorized lot root did not resolve to its declared canonical path: $resolvedLotPath"
}

function Assert-NoReparsePath([string]$Path, [string]$AllowedRoot) {
    $fullPath = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    $fullRoot = [IO.Path]::GetFullPath($AllowedRoot).TrimEnd('\')
    $rootPrefix = $fullRoot + '\'
    if (-not $fullPath.Equals($fullRoot, [StringComparison]::OrdinalIgnoreCase) -and
        -not $fullPath.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escaped its declared root: $fullPath"
    }
    $volumeRoot = [IO.Path]::GetPathRoot($fullPath)
    $cursor = $volumeRoot
    foreach ($component in $fullPath.Substring($volumeRoot.Length).Split('\')) {
        if ([string]::IsNullOrWhiteSpace($component)) { continue }
        $cursor = Join-Path $cursor $component
        if (-not (Test-Path -LiteralPath $cursor)) { continue }
        $item = Get-Item -LiteralPath $cursor -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Reparse point is forbidden in a staging path: $cursor"
        }
    }
}

function Invoke-PathSelfTest {
    Assert-NoReparsePath $lotPath $repo
    $fixtureRoot = Join-Path $lotPath ('path-selftest-' + [guid]::NewGuid().ToString('N'))
    $plainRoot = Join-Path $fixtureRoot 'plain'
    $junctionPath = Join-Path $fixtureRoot 'linked'
    [void][IO.Directory]::CreateDirectory($plainRoot)
    try {
        Assert-NoReparsePath $plainRoot $repo
        $claimFixture = Join-Path $fixtureRoot 'one-shot.claim'
        $candidateFixture = Join-Path $fixtureRoot 'candidate'
        if (Test-Path -LiteralPath $candidateFixture) { throw 'SelfTest candidate should start absent.' }
        $firstClaim = [IO.File]::Open($claimFixture, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
        try { [void][IO.Directory]::CreateDirectory($candidateFixture) } finally { $firstClaim.Dispose() }
        $secondClaimRejected = $false
        try {
            $secondClaim = [IO.File]::Open($claimFixture, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
            $secondClaim.Dispose()
        } catch [IO.IOException] { $secondClaimRejected = $true }
        if (-not $secondClaimRejected) { throw 'One-shot claim SelfTest accepted a second invocation.' }
        New-Item -ItemType Junction -Path $junctionPath -Target $repo | Out-Null
        $rejected = $false
        try { Assert-NoReparsePath (Join-Path $junctionPath 'child') $repo } catch { $rejected = $true }
        if (-not $rejected) { throw 'Reparse-path SelfTest accepted an ancestor junction.' }
        $manifestFixture = @'
(
    "notices-candidate/license.txt",
    "notices/license.txt",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
)
'@
        Assert-NoticeHashMatchesInstaller $manifestFixture 'license.txt' ('a' * 64)
        $hashMismatchRejected = $false
        try { Assert-NoticeHashMatchesInstaller $manifestFixture 'license.txt' ('b' * 64) } catch { $hashMismatchRejected = $true }
        if (-not $hashMismatchRejected) { throw 'Notice-hash SelfTest accepted a divergent installer hash.' }
        Write-Output 'TURBO_STAGE_PATH_SELFTEST PASS positive=2 negative=2'
    } finally {
        if (Test-Path -LiteralPath $junctionPath) {
            $link = Get-Item -LiteralPath $junctionPath -Force
            if ([IO.Path]::GetFullPath($link.FullName) -ine [IO.Path]::GetFullPath($junctionPath) -or
                ($link.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0 -or $link.LinkType -cne 'Junction' -or
                [IO.Path]::GetFullPath([string]$link.Target) -ine [IO.Path]::GetFullPath($repo)) {
                throw 'SelfTest cleanup refused a path that is not its created junction.'
            }
            Remove-Item -LiteralPath $junctionPath -Force
        }
        Assert-NoReparsePath $fixtureRoot $repo
        if (Test-Path -LiteralPath $plainRoot) { Remove-Item -LiteralPath $plainRoot -Force }
        if (Test-Path -LiteralPath $candidateFixture) { Remove-Item -LiteralPath $candidateFixture -Force }
        if (Test-Path -LiteralPath $claimFixture) { Remove-Item -LiteralPath $claimFixture -Force }
        if (Test-Path -LiteralPath $fixtureRoot) { Remove-Item -LiteralPath $fixtureRoot -Force }
    }
}

Assert-NoReparsePath $lotPath $repo
Assert-NoReparsePath $stagePath $repo
Assert-NoReparsePath $claimPath $repo
if (Test-Path -LiteralPath $stagePath) { throw "Stage already exists; refusing to overwrite: $stagePath" }
if (Test-Path -LiteralPath $claimPath) { throw "Stage claim already exists; refusing to retry: $claimPath" }

$sources = @(
    [pscustomobject]@{ Name='whisper-desktop.exe'; Path=(Join-Path $lotPath 'targets/turbo-edition-l2-final/release/whisper-desktop.exe'); Sha256='af89e1b4d5ce3c40c8c7e1647fd9a7f4dfe5a7aaa9cd0cce2e47f2b190640d15' },
    [pscustomobject]@{ Name='whisper-worker-cpu.exe'; Path='C:/wt/distcpu/target/parakeet-tests/cpu-distribution-x64-20261115/targets/cpu-distribution-release/release/whisper-worker-cpu.exe'; Sha256='03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a' },
    [pscustomobject]@{ Name='whisper-worker-gpu.exe'; Path=(Join-Path $repo 'target/parakeet-tests/main-facades/targets/gpu-l2-final/release/whisper-worker-gpu.exe'); Sha256='d874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9' },
    [pscustomobject]@{ Name='msvcp140.dll'; Path='C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Redist/MSVC/14.44.35112/x64/Microsoft.VC143.CRT/msvcp140.dll'; Sha256='0f885b509a685d2bbfa652fed26b5fb31d88fbdab0a978c641d1c7b8aa460aa9' },
    [pscustomobject]@{ Name='vcruntime140.dll'; Path='C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Redist/MSVC/14.44.35112/x64/Microsoft.VC143.CRT/vcruntime140.dll'; Sha256='d5e4d9a3e835fa679450145d6a7d94e36573a509317111904d9b3712c30d9066' },
    [pscustomobject]@{ Name='vcruntime140_1.dll'; Path='C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Redist/MSVC/14.44.35112/x64/Microsoft.VC143.CRT/vcruntime140_1.dll'; Sha256='1f2d41c4aa5db0bc33ebf7b66d72943a817d7ce6cbe880502a9403823633093f' },
    [pscustomobject]@{ Name='cudart64_12.dll'; Path='C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.9/bin/cudart64_12.dll'; Sha256='760c38928bbe5759f7b31ed6692599eb7ec83cedd5702e84c2b72028a89837e1' },
    [pscustomobject]@{ Name='cublas64_12.dll'; Path='C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.9/bin/cublas64_12.dll'; Sha256='52ce5ba0ec5327d39be6021f0f9362d2ed4d64d7206f9b5afc06398d827b8e82' },
    [pscustomobject]@{ Name='cublasLt64_12.dll'; Path='C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.9/bin/cublasLt64_12.dll'; Sha256='71705a7cf0923f4ab034d0d2620bbfc989ff669ac3d79f868b7a7c597ca559c5' }
)
$noticeSources = @(
    'docs/cadrage/plan-implementation-distribution-cpu-x64.md',
    'docs/cadrage/bilan-l2-distribution-turbo.md',
    'docs/cadrage/inventaire-contrat-installation-turbo.md',
    'docs/cadrage/index-notices-turbo.md',
    'docs/cadrage/recueil-licences-candidates-turbo.md',
    'docs/cadrage/notices-complementaires-turbo.md',
    'docs/cadrage/notices-cuda-entetes.md',
    'crates/whisper-bootstrap/LICENSE-renamest-MIT'
)
$modelPath = 'C:/dev/whisper/models/ggml-large-v3-turbo.bin'
$modelSha256 = '1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69'
$bootstrapSource = Join-Path $lotPath 'targets/bootstrap-v1-l3-final/release/whisper-bootstrap.exe'

foreach ($source in $sources) { Assert-NoReparsePath $source.Path ([IO.Path]::GetPathRoot($source.Path)) }
Assert-NoReparsePath $modelPath ([IO.Path]::GetPathRoot($modelPath))
Assert-NoReparsePath $bootstrapSource $repo
foreach ($relative in $noticeSources) { Assert-NoReparsePath (Join-Path $repo $relative) $repo }

function Assert-FileHash([string]$Path, [string]$Expected) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Required source is missing: $Path" }
    $observed = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($observed -cne $Expected) { throw "SHA-256 mismatch for '$Path': $observed" }
}

function Get-InstallerNoticeHash([string]$InstallerSource, [string]$Name) {
    $escapedName = [regex]::Escape($Name)
    $pattern = '\(\s*"notices-candidate/' + $escapedName + '",\s*"notices/' + $escapedName + '",\s*"([0-9a-f]{64})"\s*,?\s*\)'
    $matches = [regex]::Matches($InstallerSource, $pattern)
    if ($matches.Count -ne 1) { throw "Installer notice hash is missing or ambiguous: $Name" }
    return $matches[0].Groups[1].Value
}

function Assert-NoticeHashMatchesInstaller([string]$InstallerSource, [string]$Name, [string]$ObservedHash) {
    $expected = Get-InstallerNoticeHash $InstallerSource $Name
    if ($ObservedHash -cne $expected) { throw "Installer notice hash mismatch for '$Name': $ObservedHash" }
}

$installerSourceText = Get-Content -LiteralPath (Join-Path $repo 'crates/whisper-bootstrap/src/installer.rs') -Raw
if ($SelfTest) { Invoke-PathSelfTest; exit 0 }

foreach ($source in $sources) { Assert-FileHash $source.Path $source.Sha256 }
Assert-FileHash $modelPath $modelSha256
if (-not (Test-Path -LiteralPath $bootstrapSource -PathType Leaf)) { throw "Required bootstrap is missing: $bootstrapSource" }
$bootstrapSha256 = (Get-FileHash -LiteralPath $bootstrapSource -Algorithm SHA256).Hash.ToLowerInvariant()
foreach ($relative in $noticeSources) {
    $name = Split-Path -Leaf $relative
    $noticeHash = (Get-FileHash -LiteralPath (Join-Path $repo $relative) -Algorithm SHA256).Hash.ToLowerInvariant()
    Assert-NoticeHashMatchesInstaller $installerSourceText $name $noticeHash
}
foreach ($source in @($sources | Where-Object { $_.Name -match '\.dll$' -and $_.Name -match '^(msvcp|vcruntime)' })) {
    $signature = Get-AuthenticodeSignature -LiteralPath $source.Path
    if ($signature.Status -ne 'Valid') { throw "Visual C++ redistributable signature is not valid: $($source.Path)" }
}

$checker = Join-Path $PSScriptRoot 'check-worker-imports.ps1'
$powerShell = (Get-Process -Id $PID).Path
& $powerShell -NoProfile -File $checker -Kind Desktop -Path $sources[0].Path | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Desktop PE/import check failed.' }
& $powerShell -NoProfile -File $checker -Kind Cpu -Path $sources[1].Path | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'CPU worker PE/import check failed.' }
& $powerShell -NoProfile -File $checker -Kind Gpu -Path $sources[2].Path | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'GPU worker PE/import check failed.' }

$bootstrapReport = & $powerShell -NoProfile -File $checker -Kind Cpu -Path $bootstrapSource | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $bootstrapReport.Verdict -cne 'PASS' -or $bootstrapReport.Machine -cne 'AMD64') {
    throw 'Bootstrap PE/import check failed or is not AMD64.'
}

$claim = $null
try {
    $claim = [IO.File]::Open($claimPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    if (Test-Path -LiteralPath $stagePath) { throw "Stage appeared during reservation; refusing to write: $stagePath" }
    Assert-NoReparsePath $stagePath $repo
    [void][IO.Directory]::CreateDirectory($stagePath)
    Assert-NoReparsePath $stagePath $repo
    $appPath = Join-Path $stagePath 'app'
    $noticePath = Join-Path $stagePath 'notices-candidate'
    [void][IO.Directory]::CreateDirectory($appPath)
    [void][IO.Directory]::CreateDirectory($noticePath)

    foreach ($source in $sources) {
        $destination = Join-Path $appPath $source.Name
        $input = [IO.File]::OpenRead($source.Path)
        $output = [IO.File]::Open($destination, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
        try { $input.CopyTo($output) } finally { $output.Dispose(); $input.Dispose() }
        Assert-FileHash $destination $source.Sha256
    }
    $bootstrapDestination = Join-Path $stagePath 'whisper-bootstrap.exe'
    $bootstrapInput = [IO.File]::OpenRead($bootstrapSource)
    $bootstrapOutput = [IO.File]::Open($bootstrapDestination, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $bootstrapInput.CopyTo($bootstrapOutput) } finally { $bootstrapOutput.Dispose(); $bootstrapInput.Dispose() }
    $stagedBootstrapSha256 = (Get-FileHash -LiteralPath $bootstrapDestination -Algorithm SHA256).Hash.ToLowerInvariant()
    $currentBootstrapSha256 = (Get-FileHash -LiteralPath $bootstrapSource -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($stagedBootstrapSha256 -cne $bootstrapSha256 -or $currentBootstrapSha256 -cne $bootstrapSha256) {
        throw 'Bootstrap source or staged copy changed during assembly.'
    }
    foreach ($relative in $noticeSources) {
        $name = Split-Path -Leaf $relative
        $destination = Join-Path $noticePath $name
        $expectedNoticeHash = Get-InstallerNoticeHash $installerSourceText $name
        $input = [IO.File]::OpenRead((Join-Path $repo $relative))
        $output = [IO.File]::Open($destination, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
        try { $input.CopyTo($output) } finally { $output.Dispose(); $input.Dispose() }
        Assert-FileHash $destination $expectedNoticeHash
    }
    $readme = @'
# Candidat local WhisperTurbo — L3

Ce dossier est assemblé une seule fois par `ci/stage-turbo-package.ps1` sous `target/parakeet-tests/cpu-distribution-x64-20261115/turbo-package-l3-candidate/`. Le staging antérieur `turbo-package-candidate/` est préservé ; une réexécution refuse d'écraser cette nouvelle cible.

Lancez `whisper-bootstrap.exe` depuis ce dossier. Après vérification CPU, il demande le chemin du modèle fourni séparément, puis une confirmation. L'installation neuve est préparée dans un répertoire frère sous `%LOCALAPPDATA%` et publiée vers `%LOCALAPPDATA%\WhisperTurbo` sans remplacement d'une destination existante. Elle contient `app/`, `models/` et `notices/`. Aucun modèle n'est inclus. Le modèle attendu est `ggml-large-v3-turbo.bin`, SHA-256 `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`.

Configuration provisoire : Windows 11 x64 Home/Pro supporté à la date de publication à revalider ; plancher CPU x86-64-v2 ; version CUDA 12.9 et codegen GPU sm_61 ; pilote NVIDIA compatible requis. Les DLL Visual C++ proviennent des Build Tools 2022 et sont non modifiées ; redistribution dépendante de la licence Visual Studio Community déclarée. Les DLL CUDA 12.9 sont soumises aux conditions NVIDIA. Les notices candidates sont jointes sous `notices-candidate/`.

La publication utilise `renamest 0.1.3` `rename_exclusive` sans fallback. Sur Windows, la crate appelle `MoveFileExW` sans option de remplacement ; la documentation Microsoft consultée ne garantit pas explicitement l'atomicité. Cette limite est acceptée pour ce candidat. L'alternative MIT de `renamest` est sélectionnée parmi `MIT OR Apache-2.0`; son texte exact et son copyright sont joints dans `notices-candidate/LICENSE-renamest-MIT`. L'essai d'inférence GPU, la première utilisation en profil Windows propre et la validation utilisateur restent NOT RUN ; ce dossier n'est pas une distribution qualifiée.
'@
    $readmePath = Join-Path $stagePath 'README-INSTALL-CANDIDATE.md'
    $readmeStream = [IO.File]::Open($readmePath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
        $readmeBytes = [Text.UTF8Encoding]::new($false).GetBytes($readme)
        $readmeStream.Write($readmeBytes, 0, $readmeBytes.Length)
    } finally { $readmeStream.Dispose() }
    Write-Output "TURBO_RESOURCE_STAGE PASS path=$stagePath payload_files=$($sources.Count) bootstrap_sha256=$bootstrapSha256 notices=$($noticeSources.Count) model=verified-but-not-copied"
} finally {
    if ($claim) { $claim.Dispose() }
}
