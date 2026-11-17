# Recette du candidat corrigé sur le second PC

Avancée : candidat corrigé assemblé et vérifié automatiquement ; la recette Windows reste à effectuer par l'utilisateur.
Blocage : la disparition de la fenêtre `nvcuda.dll` et le fonctionnement du nouveau candidat sans NVIDIA ne sont pas encore observés sur le second PC.
Décision : utiliser une installation isolée neuve et conserver l'ancienne recette.

## 1. Copier les fichiers

Sur le premier PC, copiez dans un même dossier sur le second PC :

1. Le **contenu entier** du dossier `C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\turbo-package-gpu-fix\`, placé dans un sous-dossier nommé `paquet`.
2. Le modèle `C:\dev\whisper\models\ggml-large-v3-turbo.bin`, à côté de `paquet`.
3. Le fichier `C:\wt\distcpu-l2-reprise\docs\cadrage\feas-pm-01-corpus\fr_passage_28-32.wav`, à côté de `paquet`.

Le dossier choisi doit donc contenir `paquet`, `ggml-large-v3-turbo.bin` et `fr_passage_28-32.wav`. Le modèle est fourni séparément ; ne modifiez aucun fichier du paquet. Prévoyez au moins 5 Gio libres pour les copies et l'installation.

## 2. Vérifier et installer

Sur le second PC, ouvrez une nouvelle fenêtre **PowerShell**, copiez le bloc ci-dessous en entier, puis appuyez sur Entrée. Quand il le demande, collez le chemin du dossier qui contient `paquet`, le modèle et le WAV. Le bloc vérifie les empreintes, crée un dossier `profil-gpu-fix` neuf et lance l'installateur. Si ce dossier existe déjà, il s'arrête sans l'écraser.

```powershell
$ErrorActionPreference = 'Stop'
$base = (Read-Host 'Chemin du dossier contenant paquet, modèle et WAV').Trim().Trim('"')
$files = @{
    'paquet\whisper-bootstrap.exe' = '25205a05500fecd2d39a237d6d8c9caab00b7ed1e442f0bed6636f84f6476ea3'
    'paquet\app\whisper-desktop.exe' = 'd3a288b1a455e069b5ad0e6a93cd348279fbee200b2fbafd809d4a0c83b91613'
    'paquet\app\whisper-worker-cpu.exe' = 'a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b'
    'paquet\app\whisper-worker-gpu.exe' = 'd874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9'
    'ggml-large-v3-turbo.bin' = '1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69'
    'fr_passage_28-32.wav' = '1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad'
}
foreach ($name in $files.Keys) {
    $path = Join-Path $base $name
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $files[$name]) {
        throw "Fichier différent du candidat : $name"
    }
}
$package = Join-Path $base 'paquet'
if (@(Get-ChildItem -LiteralPath $package -Recurse -File).Count -ne 19) {
    throw 'Le paquet ne contient pas les 19 fichiers attendus.'
}
$localTestRoot = Join-Path $base 'profil-gpu-fix'
if (Test-Path -LiteralPath $localTestRoot) {
    throw "Le dossier de recette existe déjà : $localTestRoot"
}
New-Item -ItemType Directory -Path $localTestRoot | Out-Null
$env:LOCALAPPDATA = $localTestRoot
Write-Host 'À la question du modèle, copiez ce chemin :'
Write-Host (Join-Path $base 'ggml-large-v3-turbo.bin')
& (Join-Path $package 'whisper-bootstrap.exe')
if ($LASTEXITCODE -ne 0) { throw 'Installation échouée : conservez le message et arrêtez.' }
$desktop = Join-Path $localTestRoot 'WhisperTurbo\app\whisper-desktop.exe'
if (-not (Test-Path -LiteralPath $desktop)) { throw 'Desktop installé absent : arrêtez et signalez-le.' }
Start-Process -FilePath $desktop -WorkingDirectory $base
```

Lorsque l'installateur demande le chemin du modèle, collez le chemin affiché par PowerShell puis appuyez sur Entrée. À la confirmation, tapez `o` et appuyez sur Entrée. L'installation vise `profil-gpu-fix\WhisperTurbo` dans le dossier choisi ; elle ne réutilise pas le profil de la recette précédente. Ne créez pas vous-même les dossiers de réglages ou de journaux.

## 3. Observer le démarrage et transcrire

1. Au démarrage, notez si une fenêtre Windows indique encore que `nvcuda.dll` est introuvable, ainsi que tout autre message. Ne téléchargez pas cette DLL.
2. Dans Whisper, choisissez **CPU**, sélectionnez le WAV copié, importez-le et lancez **Traiter**. Vérifiez que la transcription est lisible et que les fichiers TXT et SRT sont produits.
3. Recommencez avec **Auto**. Relevez le moteur affiché, les messages éventuels et vérifiez la transcription ainsi que les fichiers TXT et SRT. Sans GPU NVIDIA utilisable, le résultat attendu est le moteur CPU ; une sonde GPU est permise.
4. Indiquez les résultats de chaque étape, en particulier la présence ou l'absence de la fenêtre d'erreur. En cas d'échec, conservez le dossier de recette et transmettez le message affiché.

L'essai vérifie le cas observé sur ce PC. Il ne démontre pas l'absence de toute erreur possible du chargeur Windows ni le comportement sur d'autres machines. Le paquet reste un candidat non qualifié pour distribution.

## Résultat communiqué par l'utilisateur

Sur le second PC, l'utilisateur indique qu'aucune alerte n'apparaît plus au démarrage et qu'Auto choisit bien CPU. En CPU manuel, l'import et le microphone fonctionnent, avec des transcriptions présentes et de qualité. Il confirme ensuite qu'une transcription en Auto fonctionne et que les fichiers TXT et SRT sont produits en CPU et en Auto.

Le mode GPU reste sélectionnable ; la transcription est refusée avec `BackendMismatch: selected GPU has no current runtime identity proof`. Le refus sans bascule en mode GPU manuel est conforme au choix fixe ; ce message technique en anglais n'explique pas clairement à l'utilisateur que le GPU n'est pas disponible. Les sources attribuent ce message à l'absence d'identité GPU utilisable pour la requête. L'utilisateur a demandé sa correction ; le résultat de cette correction reste à vérifier.

## Recette du candidat avec message corrigé

Cette section concerne le nouveau dossier `C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\turbo-package-message-fix\`. Les instructions et empreintes des sections précédentes restent celles du candidat déjà testé ; ne les utilisez pas pour ce nouveau paquet.

Copiez sur le second PC le contenu entier de ce nouveau dossier dans un dossier nommé `paquet-message`, à côté du modèle `ggml-large-v3-turbo.bin` et du WAV `fr_passage_28-32.wav`. Le modèle et le WAV de la recette précédente peuvent être réutilisés ; prévoyez un espace suffisant pour une installation isolée supplémentaire. Dans une nouvelle fenêtre PowerShell, copiez ce bloc, puis fournissez le chemin du dossier contenant les trois éléments :

```powershell
$ErrorActionPreference = 'Stop'
$base = (Read-Host 'Chemin du dossier contenant paquet-message, modèle et WAV').Trim().Trim('"')
$files = @{
    'paquet-message\whisper-bootstrap.exe' = '59c94bac7bf36260048e885bd24ee0f8ee31b3598e91d0ef0acb3fcb257efc4b'
    'paquet-message\app\whisper-desktop.exe' = '01e2ea8ba221ed3245b9227c2fdb2bd5a36da073bff8305b255ad4dfe1be1f48'
    'paquet-message\app\whisper-worker-cpu.exe' = 'a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b'
    'paquet-message\app\whisper-worker-gpu.exe' = 'd874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9'
    'ggml-large-v3-turbo.bin' = '1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69'
    'fr_passage_28-32.wav' = '1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad'
}
foreach ($name in $files.Keys) {
    if ((Get-FileHash -LiteralPath (Join-Path $base $name) -Algorithm SHA256).Hash -ine $files[$name]) { throw "Fichier différent du candidat : $name" }
}
$package = Join-Path $base 'paquet-message'
if (@(Get-ChildItem -LiteralPath $package -Recurse -File).Count -ne 19) { throw 'Le paquet ne contient pas les 19 fichiers attendus.' }
$localTestRoot = Join-Path $base 'profil-message-fix'
if (Test-Path -LiteralPath $localTestRoot) { throw "Le dossier de recette existe déjà : $localTestRoot" }
New-Item -ItemType Directory -Path $localTestRoot | Out-Null
$env:LOCALAPPDATA = $localTestRoot
Write-Host 'À la question du modèle, copiez ce chemin :'
Write-Host (Join-Path $base 'ggml-large-v3-turbo.bin')
& (Join-Path $package 'whisper-bootstrap.exe')
if ($LASTEXITCODE -ne 0) { throw 'Installation échouée : conservez le message et arrêtez.' }
$desktop = Join-Path $localTestRoot 'WhisperTurbo\app\whisper-desktop.exe'
if (-not (Test-Path -LiteralPath $desktop)) { throw 'Desktop installé absent : arrêtez et signalez-le.' }
Start-Process -FilePath $desktop -WorkingDirectory $base
```

À la demande, collez le chemin du modèle affiché et confirmez l'installation avec `o`. Vérifiez ensuite l'absence de la fenêtre `nvcuda.dll`, une transcription CPU et une transcription Auto. Enfin, sélectionnez GPU et tentez une transcription : elle doit être refusée sans bascule automatique et un message en français doit indiquer que l'identité GPU ne peut pas être vérifiée, en proposant CPU ou Auto. Signalez le texte affiché et tout autre message. Ne modifiez ni le paquet ni l'ancienne installation de recette.

### Résultat communiqué pour ce candidat

L'utilisateur confirme que les transcriptions en CPU et en Auto fonctionnent sur le second PC, et qu'aucune fenêtre d'erreur `nvcuda.dll` n'apparaît au démarrage. En GPU manuel, la transcription est refusée ; il rapporte le message français « GPU indisponible pour la transcription. Vous pouvez utiliser le CPU. »

Cette formulation correspond au message de disponibilité du GPU déjà présent dans l'interface desktop. Le retour confirme qu'un message compréhensible a été observé et que la transcription GPU a été refusée, mais ne démontre pas que le nouveau libellé détaillé de refus d'identité a été affiché. Les résultats GPU de performance et qualité observés sur le candidat précédent ne valent pas automatiquement validation du présent desktop reconstruit.

L'essai T32 de ressource manquante a ensuite été rejoué sur une copie des 19 fichiers du candidat `turbo-package-message-fix`, identifiée par l'empreinte de bootstrap `59c94bac7bf36260048e885bd24ee0f8ee31b3598e91d0ef0acb3fcb257efc4b`. Après comparaison des fichiers copiés, le desktop a été retiré de la copie seule. Le bootstrap a refusé l'installation avec le message de ressource absente et le code 1 ; aucune racine `WhisperTurbo` n'a été créée. Les autres copies sont restées identiques aux sources ; le candidat original est préservé. Les deux [sorties de l'essai](preuves-t32-message-fix.zip) ont été archivées et leurs empreintes comparées aux journaux sources.
