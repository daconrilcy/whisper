# Recette interactive Whisper sur Snapdragon

Exécutez les commandes dans Windows PowerShell **sur le bureau de la réservation Qualcomm**, avec le même compte et la même session pendant toute la recette. Conservez les résultats avant la fin de la réservation.

## 1. Ouvrir le candidat avant de modifier la réservation

Fermez normalement Whisper s’il est ouvert. Ouvrez PowerShell depuis le menu Démarrer du Snapdragon, puis exécutez :

```powershell
Set-Location -LiteralPath 'C:\temp\file' -ErrorAction Stop
Test-Path "$env:LOCALAPPDATA\WhisperTurbo\app\whisper-desktop.exe"
```

Si la réponse est **True**, lancez directement le diagnostic ci-dessous. Si elle est **False**, installez le candidat depuis le paquet extrait :

```powershell
Push-Location .\paquet -ErrorAction Stop
try {
    @((Resolve-Path '..\modele\ggml-large-v3-turbo.bin' -ErrorAction Stop).Path, 'o') | .\whisper-bootstrap.exe
} finally { Pop-Location }
```

Attendez **Installation terminée**. Si l’installateur refuse, conservez le dossier existant et communiquez le message. Une fois le candidat installé :

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\DIAGNOSTIC-OPENGL.ps1"
```

Notez le chemin du premier `rapport-opengl.txt` et le mode de connexion. Le rapport contient le SHA du desktop, le pilote, la session et l’inventaire du pack.

- Si la fenêtre apparaît et que le sélecteur CPU/Auto répond, passez à l’étape 3.
- Si le rapport confirme OpenGL inférieur à 2 **et** affiche « Aucune entree retournee » dans la section du paquet de compatibilité, passez à l’étape 2.
- Si le pack est déjà présent et l’erreur reproduite, si son inventaire échoue, si le candidat diffère ou si une autre erreur apparaît, arrêtez et transmettez le rapport. Un processus encore actif sans fenêtre interactive ne valide pas le démarrage.

## 2. Installer le composant Microsoft si nécessaire

Si la réservation autorise le Store, ouvrez sa fiche depuis le Snapdragon :

```powershell
Start-Process 'ms-windows-store://pdp/?ProductId=9NQPSL29BFFF'
```

Vérifiez le titre **OpenCL™, OpenGL®, and Vulkan® Compatibility Pack** et l’éditeur Microsoft. Cliquez sur **Obtenir** ou **Installer**. Si le Store est bloqué, exige un compte personnel, une élévation ou un achat, arrêtez et notez le message. [Fiche officielle Microsoft](https://apps.microsoft.com/detail/9nqpsl29bfff)

Une fois le téléchargement et l’installation terminés :

```powershell
Get-AppxPackage -Name Microsoft.D3DMappingLayers -ErrorAction Stop |
    Select-Object Name,Version,Architecture
```

Notez la version. Si aucune ligne ne revient, arrêtez et indiquez-le. Sinon, relancez **une fois** la commande de diagnostic de l’étape 1. Conservez le deuxième rapport. Vérifiez que le SHA du desktop, le pilote et la session correspondent au premier rapport. Si le sélecteur CPU/Auto répond, poursuivez. Si la fenêtre reste inutilisable, relevez le message et terminez l’essai ici.

## 3. Tester les deux imports et les deux captures

Préparez quatre dossiers distincts sur le Snapdragon. PowerShell affiche leurs chemins pour le sélecteur de destination de Whisper :

```powershell
$racine = Join-Path 'C:\temp\file' ('L4-' + (Get-Date -Format yyyyMMdd-HHmmss))
foreach ($nom in 'wav-cpu','wav-auto','micro-cpu','micro-auto') {
    New-Item -ItemType Directory -Path (Join-Path $racine $nom) -ErrorAction Stop | Select-Object -ExpandProperty FullName
}
```

1. **WAV CPU.** Dans Whisper, sélectionnez CPU, le fichier `C:\temp\file\audio\fr_passage_28-32.wav` et la destination **wav-cpu**. Lancez le traitement et attendez la fin. Notez le moteur affiché. Ouvrez TXT et SRT, vérifiez leur contenu et les horodatages ordonnés dans les quatre secondes du WAV. Relevez comment le lieu « Hong Kong » est transcrit. Le worker seul avait produit « Haute-Gon ».
2. **WAV Auto.** Reprenez le même WAV en Auto avec **wav-auto**. Refaites les vérifications et notez séparément la préférence après l’import (attendue **Auto**) et le moteur effectif (attendu **CPU**).
3. **Micro CPU.** Sélectionnez CPU et **micro-cpu**. Démarrez la capture. Dites « Le microphone enregistre ma première phrase », faites une pause, puis observez si un bloc confirmé apparaît avant l’arrêt. Dites « La dernière phrase parle du Snapdragon » et arrêtez. Notez le moteur. Vérifiez le WAV audible, les TXT et SRT de même nom et la présence de la phrase finale.
4. **Micro Auto.** Répétez avec les mêmes phrases en Auto dans **micro-auto**. Vérifiez la préférence Auto après arrêt, le moteur CPU, le bloc confirmé, la phrase finale et les trois fichiers. Si aucune entrée micro n’est exposée, inscrivez **micro non exécuté — aucune entrée disponible**. Si une entrée est visible mais que la capture échoue, relevez l’erreur.

## 4. Résultats à communiquer

Rapport avant, rapport après si le pack a été installé, mode de connexion, version du pack, fenêtre interactive avant/après. Pour les essais réalisés :

| Essai | Préférence après / moteur effectif | Fichiers vérifiés | Observation |
|---|---|---|---|
| WAV CPU | | TXT, SRT | Lieu transcrit et horodatages |
| WAV Auto | | TXT, SRT | Lieu transcrit et horodatages |
| Micro CPU | | WAV, TXT, SRT | Bloc avant arrêt et phrase finale |
| Micro Auto | | WAV, TXT, SRT | Bloc avant arrêt et phrase finale |

Si une étape bloque, transmettez son numéro, le rapport disponible et le message exact. Une réussite avec le pack sera rattachée à sa version. La qualification CI T42 reste distincte de cette recette manuelle. [Mandat validé](mandat-l4-snapdragon-opengl.md)
