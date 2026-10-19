param(
    [switch]$StageOnly,
    [switch]$RefreshDesktop,
    [switch]$Publish,
    [string]$CandidateId,
    [string]$RollbackId,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$lot = Join-Path $repo 'target\parakeet-tests\l6-perso'
$cargo = Join-Path $repo 'target\parakeet-tests\cargo-shared\release'
$artifactRoot = Join-Path $lot 'artifacts'
$modelSource = Join-Path $repo 'models'
$logs = Join-Path $lot 'logs'
$personalRoot = Join-Path $env:LOCALAPPDATA 'Whisper'
$components = @('app', 'worker', 'native', 'models', 'settings')
$script:FailurePoint = $null

function Get-Sha([string]$Path) {
    $stream = [System.IO.File]::OpenRead($Path)
    $hasher = [System.Security.Cryptography.SHA256]::Create()
    try {
        ([BitConverter]::ToString($hasher.ComputeHash($stream)) -replace '-', '').ToLowerInvariant()
    } finally {
        $stream.Dispose()
        $hasher.Dispose()
    }
}

function Assert-PathInside([string]$Path, [string]$Root) {
    $fullPath = [System.IO.Path]::GetFullPath($Path).TrimEnd('\')
    $fullRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd('\')
    if (-not $fullPath.StartsWith($fullRoot + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Path escapes authorized root: $Path"
    }
}

function Copy-Verified([string]$Source, [string]$Destination, [string]$ExpectedSha) {
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Missing local source: $Source" }
    $sourceSha = Get-Sha $Source
    if ($sourceSha -ne $ExpectedSha) { throw "Source SHA mismatch: $Source ($sourceSha)" }
    $parent = Split-Path -Parent $Destination
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
    $destinationSha = Get-Sha $Destination
    if ($destinationSha -ne $ExpectedSha) { throw "Copied SHA mismatch: $Destination ($destinationSha)" }
}

function Invoke-FailurePoint([string]$Point) {
    if ($script:FailurePoint -eq $Point) { throw "Injected installer failure: $Point" }
}

function Get-ComponentSnapshot([string]$Root, [string[]]$Names = $script:components) {
    $present = @($Names | Where-Object { Test-Path -LiteralPath (Join-Path $Root $_) -PathType Container })
    $files = @(
        foreach ($name in $present) {
            Get-ChildItem -LiteralPath (Join-Path $Root $name) -File -Recurse -Force | ForEach-Object {
                [pscustomobject]@{
                    path = $_.FullName.Substring($Root.Length + 1)
                    length = $_.Length
                    sha256 = Get-Sha $_.FullName
                }
            }
        }
    ) | Sort-Object path
    [pscustomobject]@{ components = @($present | Sort-Object); files = @($files) }
}

function Assert-Snapshot([string]$Root, $Expected, [string[]]$Names = $script:components, [string[]]$AllowedChangedFiles = @()) {
    $actual = Get-ComponentSnapshot $Root $Names
    if ((@($actual.components) -join "`n") -cne (@($Expected.components) -join "`n")) {
        throw "Installation component set differs at $Root"
    }
    if (@($actual.files).Count -ne @($Expected.files).Count) { throw "Installation file count differs at $Root" }
    for ($index = 0; $index -lt @($Expected.files).Count; $index++) {
        $left = $actual.files[$index]
        $right = $Expected.files[$index]
        $allowedChange = @($AllowedChangedFiles | Where-Object { $_.Replace('\', '/') -ceq $right.path.Replace('\', '/') }).Count -gt 0
        if ($left.path -cne $right.path -or (-not $allowedChange -and ($left.length -ne $right.length -or $left.sha256 -cne $right.sha256))) {
            throw "Installation file differs at $Root : $($right.path)"
        }
    }
}

function Copy-ComponentSet([string]$Source, [string]$Destination, [string[]]$Names = $script:components) {
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    foreach ($name in $Names) {
        $from = Join-Path $Source $name
        if (Test-Path -LiteralPath $from -PathType Container) {
            Copy-Item -LiteralPath $from -Destination (Join-Path $Destination $name) -Recurse
        }
        Invoke-FailurePoint "copy-after:$name"
    }
}

function Copy-PersonalSettings([string]$PersonalRoot, [string]$StagingSettings) {
    $activePersonalSettings = Join-Path $PersonalRoot 'settings\settings.json'
    $activeBackup = "$activePersonalSettings.bak"
    $legacySettings = Join-Path $PersonalRoot 'settings.json'
    if (Test-Path -LiteralPath $activePersonalSettings -PathType Leaf) {
        Copy-Item -LiteralPath $activePersonalSettings -Destination (Join-Path $StagingSettings 'settings.json')
        if (Test-Path -LiteralPath $activeBackup -PathType Leaf) {
            Copy-Item -LiteralPath $activeBackup -Destination (Join-Path $StagingSettings 'settings.json.bak')
        } else {
            Copy-Item -LiteralPath $activePersonalSettings -Destination (Join-Path $StagingSettings 'settings.json.bak')
        }
    } elseif (Test-Path -LiteralPath $legacySettings -PathType Leaf) {
        $newBackup = Join-Path $StagingSettings 'settings.json.bak'
        Copy-Item -LiteralPath $legacySettings -Destination (Join-Path $StagingSettings 'settings.json')
        if (-not (Test-Path -LiteralPath $newBackup)) {
            Copy-Item -LiteralPath $legacySettings -Destination $newBackup
        }
    }
}

function Write-AtomicJson([string]$Path, $Value) {
    $temporary = "$Path.$([guid]::NewGuid().ToString('N')).tmp"
    $Value | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $temporary -Encoding UTF8
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function New-VerifiedBackup([string]$SourceRoot, [string]$BackupRoot, [string]$Id) {
    Assert-PathInside $BackupRoot (Join-Path $personalRoot 'backups')
    if (Test-Path -LiteralPath $BackupRoot) { throw "Sauvegarde déjà présente: $BackupRoot" }
    $temporary = Join-Path (Join-Path $personalRoot 'backups') ".backup-$Id-$([guid]::NewGuid().ToString('N')).pending"
    Assert-PathInside $temporary (Join-Path $personalRoot 'backups')
    try {
        Copy-ComponentSet $SourceRoot $temporary
        $snapshot = Get-ComponentSnapshot $SourceRoot
        Assert-Snapshot $temporary $snapshot
        Write-AtomicJson (Join-Path $temporary 'snapshot.json') $snapshot
        $snapshotSha = Get-Sha (Join-Path $temporary 'snapshot.json')
        Write-AtomicJson (Join-Path $temporary 'complete.json') ([pscustomobject]@{ schema = 1; snapshotSha256 = $snapshotSha })
        Move-Item -LiteralPath $temporary -Destination $BackupRoot
        Assert-VerifiedBackup $BackupRoot
        return $BackupRoot
    } catch {
        if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue }
        throw
    }
}

function Read-VerifiedBackup([string]$BackupRoot) {
    Assert-PathInside $BackupRoot (Join-Path $personalRoot 'backups')
    $snapshotPath = Join-Path $BackupRoot 'snapshot.json'
    $completePath = Join-Path $BackupRoot 'complete.json'
    if (-not (Test-Path -LiteralPath $snapshotPath -PathType Leaf) -or -not (Test-Path -LiteralPath $completePath -PathType Leaf)) {
        throw "Sauvegarde incomplète: $BackupRoot"
    }
    $complete = Get-Content -LiteralPath $completePath -Raw | ConvertFrom-Json
    if ($complete.schema -ne 1 -or (Get-Sha $snapshotPath) -ne $complete.snapshotSha256) { throw "Marqueur de sauvegarde invalide: $BackupRoot" }
    $snapshot = Get-Content -LiteralPath $snapshotPath -Raw | ConvertFrom-Json
    Assert-Snapshot $BackupRoot $snapshot
    return $snapshot
}

function Assert-VerifiedBackup([string]$BackupRoot) { $null = Read-VerifiedBackup $BackupRoot }

function Restore-FromBackup([string]$BackupRoot, [string]$RestoreId) {
    $snapshot = Read-VerifiedBackup $BackupRoot
    $restoreRoot = Join-Path (Join-Path $personalRoot '.staging') ".restore-$RestoreId-$([guid]::NewGuid().ToString('N'))"
    Assert-PathInside $restoreRoot (Join-Path $personalRoot '.staging')
    Copy-ComponentSet $BackupRoot $restoreRoot
    Assert-Snapshot $restoreRoot $snapshot
    $heldRoot = Join-Path (Join-Path $personalRoot '.staging') ".held-$RestoreId-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $heldRoot | Out-Null
    try {
        foreach ($name in $script:components) {
            $active = Join-Path $personalRoot $name
            $held = Join-Path $heldRoot $name
            if (Test-Path -LiteralPath $active) { Move-Item -LiteralPath $active -Destination $held }
            if ($name -in $snapshot.components) { Move-Item -LiteralPath (Join-Path $restoreRoot $name) -Destination $active }
        }
        Assert-Snapshot $personalRoot $snapshot
    } catch {
        throw "Restauration incomplète; les copies complètes restent dans $BackupRoot et $heldRoot : $($_.Exception.Message)"
    }
    Remove-Item -LiteralPath $heldRoot -Recurse -Force
    Remove-Item -LiteralPath $restoreRoot -Recurse -Force
}

function Write-PackageManifest([string]$Root, [string]$ManifestPath) {
    $snapshot = Get-ComponentSnapshot $Root
    Write-AtomicJson $ManifestPath $snapshot
}

function Read-PackageManifest([string]$Root, [string]$ManifestPath, [string[]]$AllowedChangedFiles = @()) {
    if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) { throw "Manifeste de staging absent: $ManifestPath" }
    $manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
    if ($manifest -is [array]) {
        foreach ($entry in $manifest) {
            if (@($AllowedChangedFiles | Where-Object { $_.Replace('\', '/') -ceq ([string]$entry.path).Replace('\', '/') }).Count -gt 0) { continue }
            $path = Join-Path $Root $entry.path
            if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or (Get-Item -LiteralPath $path).Length -ne $entry.length -or (Get-Sha $path) -ne $entry.sha256) {
                throw "Ressource modifiée après staging: $path"
            }
        }
        return Get-ComponentSnapshot $Root
    }
    Assert-Snapshot $Root $manifest $script:components $AllowedChangedFiles
    return $manifest
}

function Assert-FinalDesktopBuild {
    $receiptPath = Join-Path $logs 'PersonalDesktop-artifact.json'
    $invocationPath = Join-Path $logs 'PersonalDesktop-invocation.json'
    $exitPath = Join-Path $logs 'PersonalDesktop-exit-code.txt'
    $desktop = Join-Path $cargo 'whisper-desktop.exe'
    if (-not (Test-Path -LiteralPath $receiptPath -PathType Leaf) -or -not (Test-Path -LiteralPath $invocationPath -PathType Leaf) -or -not (Test-Path -LiteralPath $exitPath -PathType Leaf)) {
        throw 'Build desktop Perso final non attesté par la campagne gardée.'
    }
    $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    $invocation = Get-Content -LiteralPath $invocationPath -Raw | ConvertFrom-Json
    $args = @($invocation.arguments)
    $receiptArgs = @($receipt.arguments)
    if ((Get-Content -LiteralPath $exitPath -Raw).Trim() -ne '0' -or
        $receipt.exitCode -ne 0 -or $receipt.sha256 -ne (Get-Sha $desktop) -or
        $receipt.modelRegistrySha256 -ne (Get-Sha (Join-Path $repo 'crates\whisper-core\src\model_registry.rs')) -or
        $args -notcontains 'personal-install' -or $args -contains 'personal-qualification' -or
        $receiptArgs -notcontains 'personal-install' -or $receiptArgs -contains 'personal-qualification') {
        throw 'Le desktop release ne correspond pas au dernier build Perso sans injection et au registre actuel.'
    }
    return $receipt.sha256
}

function Recover-InterruptedTransaction {
    $transactionPath = Join-Path $personalRoot '.staging\active-transaction.json'
    if (-not (Test-Path -LiteralPath $transactionPath -PathType Leaf)) { return }
    Assert-DesktopClosed
    $transaction = Get-Content -LiteralPath $transactionPath -Raw | ConvertFrom-Json
    $backup = [string]$transaction.recoveryBackup
    Restore-FromBackup $backup ([string]$transaction.id)
    Remove-Item -LiteralPath $transactionPath -Force
    Write-Output "Récupération d'une transaction interrompue terminée depuis $backup"
}

function Apply-Generation([string]$SourceRoot, $DesiredSnapshot, [string]$RecoveryBackup, [string]$Operation, [string]$Id) {
    $transactionPath = Join-Path $personalRoot '.staging\active-transaction.json'
    if (Test-Path -LiteralPath $transactionPath) { throw 'Une transaction active doit être récupérée avant toute nouvelle opération.' }
    $replacementRoot = Join-Path (Join-Path $personalRoot '.staging') ".replacement-$Id-$([guid]::NewGuid().ToString('N'))"
    $heldRoot = Join-Path (Join-Path $personalRoot '.staging') ".old-$Id-$([guid]::NewGuid().ToString('N'))"
    $transaction = [pscustomobject]@{ schema = 1; id = $Id; operation = $Operation; recoveryBackup = $RecoveryBackup }
    Write-AtomicJson $transactionPath $transaction
    try {
        Copy-ComponentSet $SourceRoot $replacementRoot
        Assert-Snapshot $replacementRoot $DesiredSnapshot
        Invoke-FailurePoint "$Operation-after-prepare"
        New-Item -ItemType Directory -Path $heldRoot | Out-Null
        foreach ($name in $script:components) {
            $active = Join-Path $personalRoot $name
            $held = Join-Path $heldRoot $name
            if (Test-Path -LiteralPath $active) { Move-Item -LiteralPath $active -Destination $held }
            if ($name -in $DesiredSnapshot.components) { Move-Item -LiteralPath (Join-Path $replacementRoot $name) -Destination $active }
            Invoke-FailurePoint "$Operation-after:$name"
        }
        Assert-Snapshot $personalRoot $DesiredSnapshot
        Remove-Item -LiteralPath $transactionPath -Force
        Remove-Item -LiteralPath $heldRoot -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $replacementRoot -Recurse -Force -ErrorAction SilentlyContinue
    } catch {
        $failure = $_.Exception.Message
        # The injected fault models the failed operation, not the recovery path.
        $script:FailurePoint = $null
        try {
            Restore-FromBackup $RecoveryBackup $Id
            Remove-Item -LiteralPath $transactionPath -Force -ErrorAction SilentlyContinue
            Remove-Item -LiteralPath $heldRoot -Recurse -Force -ErrorAction SilentlyContinue
            Remove-Item -LiteralPath $replacementRoot -Recurse -Force -ErrorAction SilentlyContinue
        } catch {
            throw "$Operation failed ($failure); recovery remains journaled for retry: $($_.Exception.Message)"
        }
        throw "$Operation failed; previous generation restored from $RecoveryBackup : $failure"
    }
}

function New-TestGeneration([string]$Root, [string]$Label) {
    foreach ($path in @('app/whisper-desktop.exe', 'worker/cpu.exe', 'worker/gpu.exe', 'native/cpu.dll', 'native/gpu.dll', 'models/parakeet.gguf', 'settings/settings.json', 'settings/settings.json.bak')) {
        $file = Join-Path $Root $path
        New-Item -ItemType Directory -Path (Split-Path -Parent $file) -Force | Out-Null
        Set-Content -LiteralPath $file -Value "${Label}:$path" -Encoding ascii
    }
}

function Invoke-InstallerSelfTests {
    $selfRoot = Join-Path $lot ('installer-selftest-' + [guid]::NewGuid().ToString('N'))
    Assert-PathInside $selfRoot $lot
    $script:personalRoot = Join-Path $selfRoot 'active'
    New-Item -ItemType Directory -Path (Join-Path $personalRoot 'backups') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $personalRoot '.staging') -Force | Out-Null
    $results = [System.Collections.Generic.List[string]]::new()

    $old = Join-Path $selfRoot 'old-generation'
    New-TestGeneration $old 'old'
    $oldSnapshot = Get-ComponentSnapshot $old
    $script:FailurePoint = 'copy-after:worker'
    $backupFailed = $false
    try { New-VerifiedBackup $old (Join-Path $personalRoot 'backups\backup-fail') 'backup-fail' | Out-Null } catch { $backupFailed = $true }
    if (-not $backupFailed) { throw 'Installer self-test expected an injected backup failure.' }
    $script:FailurePoint = $null
    Assert-Snapshot $old $oldSnapshot
    $results.Add('backup-copy-failure preserves the active generation')

    $settingsSource = Join-Path $selfRoot 'settings-source'
    $settingsStage = Join-Path $selfRoot 'settings-stage'
    New-Item -ItemType Directory -Path (Join-Path $settingsSource 'settings'), $settingsStage -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $settingsSource 'settings\settings.json') -Value 'user-edited-settings' -Encoding ascii
    Set-Content -LiteralPath (Join-Path $settingsSource 'settings\settings.json.bak') -Value 'original-v1-backup' -Encoding ascii
    Copy-PersonalSettings $settingsSource $settingsStage
    if ((Get-Content (Join-Path $settingsStage 'settings.json') -Raw).Trim() -ne 'user-edited-settings' -or
        (Get-Content (Join-Path $settingsStage 'settings.json.bak') -Raw).Trim() -ne 'original-v1-backup') {
        throw 'Installer self-test failed to preserve the original settings backup after preference changes.'
    }
    $results.Add('settings update preserves an older .bak after active preferences change')

    $new = Join-Path $selfRoot 'new-generation'
    New-TestGeneration $new 'new'
    $newSnapshot = Get-ComponentSnapshot $new
    $interruptedRoot = Join-Path $selfRoot 'interrupted-install'
    New-Item -ItemType Directory -Path (Join-Path $interruptedRoot 'backups'), (Join-Path $interruptedRoot '.staging') -Force | Out-Null
    Copy-ComponentSet $new $interruptedRoot
    Remove-Item -LiteralPath (Join-Path $interruptedRoot 'app') -Recurse -Force
    Copy-Item -LiteralPath (Join-Path $old 'app') -Destination (Join-Path $interruptedRoot 'app') -Recurse
    $originalPersonalRoot = $script:personalRoot
    $script:personalRoot = $interruptedRoot
    try {
        $recoveryBackup = Join-Path $interruptedRoot 'backups\before-interruption'
        $null = New-VerifiedBackup $old $recoveryBackup 'before-interruption'
        Write-AtomicJson (Join-Path $interruptedRoot '.staging\active-transaction.json') ([pscustomobject]@{
            schema = 1; id = 'interrupted'; operation = 'publish'; recoveryBackup = $recoveryBackup
        })
        Recover-InterruptedTransaction
        Assert-Snapshot $interruptedRoot $oldSnapshot
    } finally {
        $script:personalRoot = $originalPersonalRoot
    }
    $results.Add('startup recovery restores a verified prior generation after a mixed-component interruption')

    $originalPersonalRoot = $script:personalRoot
    foreach ($name in $script:components) {
        $active = Join-Path $selfRoot "publish-$name"
        $script:personalRoot = $active
        New-Item -ItemType Directory -Path (Join-Path $active 'backups'), (Join-Path $active '.staging') -Force | Out-Null
        Copy-ComponentSet $old $active
        $backupRoot = Join-Path $active "backups\publish-$name"
        $null = New-VerifiedBackup $active $backupRoot "publish-$name"
        $script:FailurePoint = "publish-after:$name"
        $failed = $false
        try { Apply-Generation $new $newSnapshot $backupRoot 'publish' "publish-$name" } catch { $failed = $true }
        if (-not $failed) { throw "Installer self-test expected a publish interruption after $name." }
        Assert-Snapshot $active $oldSnapshot
        $results.Add("publish interruption after $name restores every prior component")
    }

    foreach ($name in $script:components) {
        $active = Join-Path $selfRoot "rollback-$name"
        $script:personalRoot = $active
        New-Item -ItemType Directory -Path (Join-Path $active 'backups'), (Join-Path $active '.staging') -Force | Out-Null
        Copy-ComponentSet $new $active
        $targetBackup = Join-Path $active "backups\rollback-target-$name"
        $currentBackup = Join-Path $active "backups\rollback-current-$name"
        $null = New-VerifiedBackup $old $targetBackup "rollback-target-$name"
        $null = New-VerifiedBackup $active $currentBackup "rollback-current-$name"
        $targetSnapshot = Read-VerifiedBackup $targetBackup
        $script:FailurePoint = "rollback-after:$name"
        $failed = $false
        try { Apply-Generation $targetBackup $targetSnapshot $currentBackup 'rollback' "rollback-$name" } catch { $failed = $true }
        if (-not $failed) { throw "Installer self-test expected a rollback interruption after $name." }
        Assert-Snapshot $active $newSnapshot
        $results.Add("rollback interruption after $name restores every pre-rollback component")
    }
    $script:FailurePoint = $null
    $script:personalRoot = $originalPersonalRoot
    $results | ForEach-Object { "PASS $_" }
}

function Get-ComponentPaths([string]$Root) {
    @('app', 'worker', 'native', 'models', 'settings') | ForEach-Object { Join-Path $Root $_ }
}

function Assert-DesktopClosed {
    $running = @(Get-Process -Name 'whisper-desktop' -ErrorAction SilentlyContinue)
    if ($running.Count -gt 0) { throw 'Fermez Whisper avant le staging, la publication ou le rollback.' }
}

if ($SelfTest) { Invoke-InstallerSelfTests; exit 0 }
if (-not (Test-Path -LiteralPath $env:LOCALAPPDATA -PathType Container)) { throw 'LOCALAPPDATA indisponible.' }
if (-not (Test-Path -LiteralPath $personalRoot -PathType Container)) { New-Item -ItemType Directory -Path $personalRoot | Out-Null }
if ((@($StageOnly, $RefreshDesktop, $Publish).Where({ $_ }).Count -ne 1) -and -not $RollbackId) {
    throw 'Choisissez exactement -StageOnly, -RefreshDesktop ou -Publish, ou fournissez -RollbackId.'
}
Assert-DesktopClosed
Recover-InterruptedTransaction
if ($RollbackId) {
    if ($RollbackId -notmatch '^[A-Za-z0-9._-]+$') { throw 'RollbackId invalide.' }
    $backupRoot = Join-Path $personalRoot "backups\$RollbackId"
    $targetSnapshot = Read-VerifiedBackup $backupRoot
    $currentBackup = Join-Path $personalRoot "backups\after-rollback-$RollbackId"
    $null = New-VerifiedBackup $personalRoot $currentBackup "after-rollback-$RollbackId"
    Apply-Generation $backupRoot $targetSnapshot $currentBackup 'rollback' $RollbackId
    Write-Output "Rollback terminé; l'état remplacé est conservé sous $currentBackup"
    exit 0
}

if ($CandidateId -notmatch '^[A-Za-z0-9._-]+$') { throw 'CandidateId invalide.' }
Assert-DesktopClosed
$stage = Join-Path $personalRoot ".staging\$CandidateId"
Assert-PathInside $stage (Join-Path $personalRoot '.staging')

$turboSha = '1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69'
$parakeetSha = 'e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e'
$cpuDlls = @{
    'ggml-base.dll' = '398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a'
    'ggml-cpu.dll' = 'f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0'
    'ggml.dll' = '2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794'
    'nemo_speech_asr.dll' = '923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019'
    'nemo_speech_asr_c.dll' = '9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764'
}
$gpuDlls = @{
    'cublas64_12.dll' = '52ce5ba0ec5327d39be6021f0f9362d2ed4d64d7206f9b5afc06398d827b8e82'
    'cublasLt64_12.dll' = '71705a7cf0923f4ab034d0d2620bbfc989ff669ac3d79f868b7a7c597ca559c5'
    'cudart64_12.dll' = '760c38928bbe5759f7b31ed6692599eb7ec83cedd5702e84c2b72028a89837e1'
    'ggml-base.dll' = '24b7342f10bc14e17dcb55ebafdbaa1a0fd0187cf5ae1a8ecf03bb9f8a17ef93'
    'ggml-cpu.dll' = '23166143accf20055ccaafb8b720ad3dac33bf16ac027cc77225e0bb2a99e699'
    'ggml-cuda.dll' = '7e244413387852825ed420fceef5e3b318f3a264f97f59fe0233789dfbdb6822'
    'ggml.dll' = '9148ba2ed0eaeee385d09eba37e1a297d5e2f0649a02ffd4e325fea562c29ea6'
    'nemo_speech_asr.dll' = '4fac537a8d99a8cee4cd1dba2391379cb895fafb7d21beb9d6f4fe670be05449'
    'nemo_speech_asr_c.dll' = '6643c1c657f05bd545655044ec59fc052a789042ce9c17496b9c77a240c7bbb9'
}

if ($StageOnly) {
    if (Test-Path -LiteralPath $stage) { throw "Staging already exists; refusing overwrite: $stage" }
    $cpuWorker = Join-Path $artifactRoot 'whisper-worker-parakeet-cpu.exe'
    $gpuWorker = Join-Path $artifactRoot 'whisper-worker-parakeet-gpu.exe'
    $cpuFrozen = Get-Content (Join-Path $lot 'logs\PersonalCPU-worker-frozen.json') -Raw | ConvertFrom-Json
    $gpuFrozen = Get-Content (Join-Path $lot 'logs\PersonalGPU-worker-frozen.json') -Raw | ConvertFrom-Json
    $stageNew = Join-Path $personalRoot ".staging\$CandidateId"
    New-Item -ItemType Directory -Path $stageNew -Force | Out-Null
    try {
        foreach ($directory in @('app', 'worker', 'native\cpu', 'native\gpu', 'models', 'settings')) {
            New-Item -ItemType Directory -Path (Join-Path $stageNew $directory) -Force | Out-Null
        }
        Copy-Verified $cpuWorker (Join-Path $stageNew 'worker\whisper-worker-parakeet-cpu.exe') $cpuFrozen.sha256
        Copy-Verified $gpuWorker (Join-Path $stageNew 'worker\whisper-worker-parakeet-gpu.exe') $gpuFrozen.sha256
        $desktopSha = Assert-FinalDesktopBuild
        Copy-Verified (Join-Path $cargo 'whisper-desktop.exe') (Join-Path $stageNew 'app\whisper-desktop.exe') $desktopSha
        Copy-Verified (Join-Path $cargo 'whisper-worker-cpu.exe') (Join-Path $stageNew 'app\whisper-worker-cpu.exe') (Get-Sha (Join-Path $cargo 'whisper-worker-cpu.exe'))
        Copy-Verified (Join-Path $cargo 'whisper-worker-gpu.exe') (Join-Path $stageNew 'app\whisper-worker-gpu.exe') (Get-Sha (Join-Path $cargo 'whisper-worker-gpu.exe'))
        Copy-Verified (Join-Path $modelSource 'ggml-large-v3-turbo.bin') (Join-Path $stageNew 'models\ggml-large-v3-turbo.bin') $turboSha
        Copy-Verified (Join-Path $modelSource 'parakeet-tdt-0.6b-v3.q8_0.gguf') (Join-Path $stageNew 'models\parakeet-tdt-0.6b-v3.q8_0.gguf') $parakeetSha
        foreach ($dll in $cpuDlls.Keys) {
            Copy-Verified (Join-Path $repo "target\parakeet-tests\l2\l2b\native\$dll") (Join-Path $stageNew "native\cpu\$dll") $cpuDlls[$dll]
        }
        foreach ($dll in $gpuDlls.Keys) {
            Copy-Verified (Join-Path $repo "target\parakeet-tests\l3\l3b\native\$dll") (Join-Path $stageNew "native\gpu\$dll") $gpuDlls[$dll]
        }
        Copy-PersonalSettings $personalRoot (Join-Path $stageNew 'settings')
        Write-PackageManifest $stageNew (Join-Path $lot "stage-$CandidateId-manifest.json")
    } catch {
        Remove-Item -LiteralPath $stageNew -Recurse -Force -ErrorAction SilentlyContinue
        throw
    }
    Write-Output "Staging prêt et vérifié : $stageNew"
    exit 0
}

if ($RefreshDesktop) {
    if (-not (Test-Path -LiteralPath $stage -PathType Container)) { throw "Staging absent: $stage" }
    $manifestPath = Join-Path $lot "stage-$CandidateId-manifest.json"
    $null = Read-PackageManifest $stage $manifestPath @('app/whisper-desktop.exe')
    $desktopSha = Assert-FinalDesktopBuild
    Copy-Verified (Join-Path $cargo 'whisper-desktop.exe') (Join-Path $stage 'app\whisper-desktop.exe') $desktopSha
    Write-PackageManifest $stage $manifestPath
    $finalSnapshot = Read-PackageManifest $stage $manifestPath
    Assert-Snapshot $stage $finalSnapshot
    Write-Output "Desktop Perso final rafraîchi et manifeste régénéré : $stage"
    exit 0
}

if ($Publish) {
    if (-not (Test-Path -LiteralPath $stage -PathType Container)) { throw "Staging absent: $stage" }
    $manifestPath = Join-Path $lot "stage-$CandidateId-manifest.json"
    $null = Read-PackageManifest $stage $manifestPath
    $desktopSha = Assert-FinalDesktopBuild
    Copy-Verified (Join-Path $cargo 'whisper-desktop.exe') (Join-Path $stage 'app\whisper-desktop.exe') $desktopSha
    Write-PackageManifest $stage $manifestPath
    $stageSnapshot = Read-PackageManifest $stage $manifestPath
    $backupId = "before-$CandidateId"
    $backupRoot = Join-Path $personalRoot "backups\$backupId"
    $null = New-VerifiedBackup $personalRoot $backupRoot $backupId
    Apply-Generation $stage $stageSnapshot $backupRoot 'publish' $CandidateId
    Write-Output "Publication terminée; sauvegarde conservée : $backupRoot"
}
