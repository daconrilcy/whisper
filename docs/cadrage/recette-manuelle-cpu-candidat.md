# Recette manuelle CPU — candidat Turbo

Avancée : candidat reconstruit et contrôles automatiques gardés réussis.
Blocage : la transcription et les exports de ce nouveau candidat ne sont pas encore validés.
Décision : essai manuel dans la fenêtre par l'utilisateur, sur un profil applicatif isolé.

Ouvrir une **nouvelle fenêtre PowerShell**, y copier le bloc suivant en entier et appuyer sur Entrée. Il vérifie les empreintes, prépare un dossier de recette neuf, installe le modèle fourni séparément et ouvre le desktop. Si le dossier existe déjà, il s'arrête sans l'écraser. Les réglages d'environnement ne concernent que cette fenêtre et l'application qu'elle lance.

```powershell
$ErrorActionPreference = 'Stop'
$repo = 'C:\wt\distcpu-l2-reprise'
$lot = Join-Path $repo 'target\parakeet-tests\cpu-distribution-x64-20261115'
$package = Join-Path $lot 'turbo-package-cpu-fix'
$root = Join-Path $lot 'recette-cpu-fix'
$model = 'C:\dev\whisper\models\ggml-large-v3-turbo.bin'
$source = Join-Path $repo 'docs\cadrage\feas-pm-01-corpus\fr_passage_28-32.wav'
$bootstrap = Join-Path $package 'whisper-bootstrap.exe'

foreach ($path in @($repo, (Join-Path $repo 'target'), (Join-Path $repo 'target\parakeet-tests'), $lot, $package)) {
    $item = Get-Item -LiteralPath $path -Force
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Lien ou jonction refusé : $path" }
}
if (Test-Path -LiteralPath $root) { throw "Le dossier de recette existe déjà : $root" }
if ((Get-PSDrive C).Free -lt 30GB) { throw 'Moins de 30 Gio libres sur C:.' }
if ((Get-FileHash -LiteralPath $bootstrap -Algorithm SHA256).Hash -ine 'd7852efb06b5c010623ab8ee9833c6b23dbc7515f4208e5bff3d934febef6f5a') { throw 'Installateur différent du candidat vérifié.' }
if ((Get-FileHash -LiteralPath $model -Algorithm SHA256).Hash -ine '1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69') { throw 'Modèle absent ou empreinte incorrecte.' }
if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ine '1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad') { throw 'WAV de référence différent.' }

$local = Join-Path $root 'LocalAppData'
$roaming = Join-Path $root 'Roaming'
$temp = Join-Path $root 'Temp'
$work = Join-Path $root 'Travail'
$out = Join-Path $work 'Sorties'
New-Item -ItemType Directory -Path $local,$roaming,$temp,$work,$out -Force | Out-Null
$audio = Join-Path $work 'entree.wav'
Copy-Item -LiteralPath $source -Destination $audio
if ((Get-FileHash -LiteralPath $audio -Algorithm SHA256).Hash -ine '1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad') { throw 'Copie WAV incorrecte.' }

Get-ChildItem Env: | Where-Object { $_.Name -match '^(CUDA_PATH|CUDA_PATH_V.*|GGML_.*|WHISPER_.*|PARAKEET_.*)$' } | ForEach-Object { Remove-Item -LiteralPath "Env:$($_.Name)" }
$env:LOCALAPPDATA = $local
$env:APPDATA = $roaming
$env:USERPROFILE = $root
$env:HOME = $root
$env:TEMP = $temp
$env:TMP = $temp
$env:PATH = "$env:SystemRoot\System32;$env:SystemRoot"

@($model, 'o') | & $bootstrap
if ($LASTEXITCODE -ne 0) { throw "Installation refusée ou échouée (code $LASTEXITCODE)." }
Write-Host "Fichier audio à sélectionner : $audio"
Write-Host "Dossier de sortie à sélectionner : $out"
Start-Process -FilePath (Join-Path $local 'WhisperTurbo\app\whisper-desktop.exe') -WorkingDirectory $work -WindowStyle Normal
```

Dans Whisper :

1. Vérifier que le modèle affiché est **Whisper large-v3-turbo**, puis choisir **CPU** dans « Calcul ».
2. Sélectionner le fichier audio et le dossier de sortie indiqués dans PowerShell.
3. Cliquer sur **Importer et transcrire**, puis sur **Traiter** dans la file.
4. Attendre la fin ; vérifier qu'un texte lisible et des fichiers TXT et SRT sont produits. En cas de blocage, relever le message et la phase affichés, sans supprimer le dossier de recette.
5. Communiquer le résultat, y compris tout message d'erreur ou absence de TXT/SRT. La validation finale de la fenêtre appartient à l'utilisateur.

Ce parcours utilise la machine actuelle ; le `PATH` limité ne prouve pas l'absence d'un SDK ou d'outils sur cette machine. Les données créées sous `recette-cpu-fix` sont conservées jusqu'à décision de clôture.

## Résultat de la recette CPU du 17 novembre 2026

L'utilisateur confirme une transcription CPU et la production des fichiers TXT et SRT. Les empreintes du desktop et du worker CPU installés correspondent au candidat : respectivement `7f352689d12787d529462438c5e1ad24c8a741f5c7cf134e0cb5ddc09090ecfb` et `a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b`. Le modèle installé correspond à l'empreinte attendue. Les sorties TXT et SRT sont présentes ; le journal du worker relève le moteur CPU et le traitement des 64 000 échantillons.

La branche T32 de ressource manquante a été exécutée sur une copie des 19 fichiers du candidat, après comparaison de leurs empreintes. Le retrait du desktop dans cette seule copie a provoqué le refus attendu, sans création de racine d'installation ; le candidat original reste identique. Les sorties de cet essai sont archivées dans `preuves-recette-cpu-l3.zip`.

Sur les processus encore ouverts après la recette, les modules observés du worker CPU provenaient de l'installation de recette ou de Windows ; ceux du desktop comprenaient également un module Windows Defender. Cette observation ponctuelle et le `PATH` limité ne prouvent pas un premier lancement sur une machine dépourvue du SDK CUDA, des outils de build et du dépôt. La recette a été exécutée dans une racine isolée sous le worktree ; cette limite de T33 reste ouverte. Les essais matériels et la qualification de distribution restent prévus en L4.
