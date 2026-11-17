# Remise L4 — candidat WhisperTurbo

Avancée : gate L3→L4 ouverte après la recette CPU sur les moyens disponibles et décision utilisateur du 17 novembre 2026.
Blocage : qualification et distribution interdites tant que les essais obligatoires, dont T33 complet, ne sont pas acquis.
Décision : l'utilisateur reporte à L4 l'essai sans dépôt, SDK CUDA ni outils de build ; il reste obligatoire avant distribution.

## Candidat et configuration provisoire

Le candidat local est le dossier `C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\turbo-package-cpu-fix\` : 19 fichiers, dont huit notices. Il ne s'agit pas d'une distribution qualifiée. Le modèle est fourni séparément, sans téléchargement par l'application. Ne pas modifier ce dossier pour les essais. Son fichier `README-INSTALL-CANDIDAT.md` décrit le contenu ; ses indications historiques « inférence non exécutée » et « validation utilisateur NOT RUN » sont remplacées par le résultat CPU limité ci-dessous.

| Élément | SHA-256 |
|---|---|
| `whisper-bootstrap.exe` | `d7852efb06b5c010623ab8ee9833c6b23dbc7515f4208e5bff3d934febef6f5a` |
| `app/whisper-desktop.exe` | `7f352689d12787d529462438c5e1ad24c8a741f5c7cf134e0cb5ddc09090ecfb` |
| `app/whisper-worker-cpu.exe` | `a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b` |
| `app/whisper-worker-gpu.exe` | `d874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9` |
| Modèle séparé `C:\dev\whisper\models\ggml-large-v3-turbo.bin` | `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` |
| WAV `docs/cadrage/feas-pm-01-corpus/fr_passage_28-32.wav` | `1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad` |

Plancher CPU provisoire : x86-64-v2 complet ; il ne se confond pas avec les exigences de Windows. Cible OS provisoire : Windows 11 x64 Home/Pro, à qualifier sur les versions réellement testées. Le binaire NVIDIA a été compilé avec CUDA 12.9 et `61-real` ; un pilote compatible est nécessaire et aucune autre architecture n'est revendiquée sans preuve. Les DLL Visual C++ et CUDA ainsi que leurs notices accompagnent le candidat ; les conditions et limites de droits sont documentées dans [l'inventaire](inventaire-contrat-installation-turbo.md) et les notices livrées. La preuve de bon fonctionnement en l'absence de l'environnement de développement reste ouverte.

## Installation et premier parcours

Sur cette machine, la [recette CPU isolée](recette-manuelle-cpu-candidat.md) a installé ce candidat et le modèle vérifié dans `C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\recette-cpu-fix\LocalAppData\WhisperTurbo\`. Elle protège le profil Windows réel ; le script de première installation refuse de réutiliser un dossier déjà existant. L'utilisateur a confirmé la transcription WAV CPU et la production de TXT et SRT. Le journal relève le moteur CPU et le traitement complet des 64 000 échantillons. Cela ne valide pas encore l'import MP3, le microphone ou les autres machines.

Pour poursuivre dans la fenêtre déjà ouverte, utiliser cette installation et le WAV de la recette ; conserver les sorties de chaque essai dans un dossier distinct. Pour un nouveau lancement de l'installation isolée, ouvrir une nouvelle fenêtre PowerShell et exécuter :

```powershell
$root = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\recette-cpu-fix'
$env:LOCALAPPDATA = Join-Path $root 'LocalAppData'
$env:APPDATA = Join-Path $root 'Roaming'
$env:USERPROFILE = $root
$env:HOME = $root
$env:TEMP = Join-Path $root 'Temp'
$env:TMP = $env:TEMP
$env:PATH = "$env:SystemRoot\System32;$env:SystemRoot"
Get-ChildItem Env: | Where-Object { $_.Name -match '^(CUDA_PATH|CUDA_PATH_V.*|GGML_.*|WHISPER_.*|PARAKEET_.*)$' } | ForEach-Object { Remove-Item -LiteralPath "Env:$($_.Name)" }
$app = Join-Path $env:LOCALAPPDATA 'WhisperTurbo\app'
$expected = @{
    'whisper-desktop.exe' = '7f352689d12787d529462438c5e1ad24c8a741f5c7cf134e0cb5ddc09090ecfb'
    'whisper-worker-cpu.exe' = 'a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b'
    'whisper-worker-gpu.exe' = 'd874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9'
}
foreach ($name in $expected.Keys) {
    if ((Get-FileHash -LiteralPath (Join-Path $app $name) -Algorithm SHA256).Hash -ine $expected[$name]) { throw "Binaire différent du candidat : $name" }
}
Start-Process -FilePath (Join-Path $env:LOCALAPPDATA 'WhisperTurbo\app\whisper-desktop.exe') -WorkingDirectory (Join-Path $root 'Travail')
```

Sur un nouveau profil, suivre les instructions du paquet : lancer `whisper-bootstrap.exe`, fournir le modèle vérifié puis confirmer. L'installation vise `%LOCALAPPDATA%\WhisperTurbo` et refuse d'écraser une racine existante. N'utiliser un autre profil ou une autre machine que lorsqu'ils sont disponibles et autorisés ; ne pas fabriquer les dossiers `settings` ou `logs` à la place de l'application.

## Fiche de recette utilisateur T43

Pour chaque essai, relever date, Windows et son édition, processeur et, si concerné, GPU et pilote ; conserver l'identité du candidat, l'entrée, le moteur demandé et celui effectivement affiché, le résultat et les messages. Ne jamais altérer le paquet exact pour provoquer une erreur.

1. **Fichier CPU.** Refaire le WAV si nécessaire, puis tester le MP3 correspondant : choisir CPU, importer, traiter et vérifier texte lisible, TXT et SRT chronologique, et source intacte. Le WAV CPU est déjà confirmé sur la machine de recette ; le MP3 correspondant reste à fournir et tester.
2. **Micro CPU.** Dire une phrase, marquer un silence puis dire une phrase finale connue. Vérifier un bloc confirmé avant l'arrêt ; arrêter normalement et vérifier le WAV lisible et le TXT/SRT complets, y compris la phrase finale.
3. **GPU manuel.** Sur un banc NVIDIA compatible, refaire fichier et micro en sélectionnant GPU ; relever le moteur effectif et comparer qualité et temps au CPU sur les mêmes entrées. Mesurer si le traitement est plus rapide que la durée audio et si le premier texte micro apparaît en moins de 15 secondes. En cas d'échec d'initialisation, vérifier l'absence de bascule automatique en CPU.
4. **Auto.** Avec GPU compatible, vérifier le choix GPU ; sur un témoin sans GPU utilisable, vérifier le choix CPU. Les scénarios d'échec d'initialisation et de panne après début audio nécessitent des stimuli maîtrisés : ne pas provoquer ces pannes en modifiant le candidat ; les conserver `NOT RUN` tant que leur méthode n'est pas établie. Relever cause, moteur effectif et préférence Auto conservée.
5. **Témoins CPU.** Sur Intel et AMD, relever les résultats ; sur le témoin v2 exact, vérifier les capacités exigées, l'absence des capacités v3/v4 et l'admission du même paquet. Le refus sur matériel sous v2 reste à observer sur un témoin capable d'exécuter le bootstrap.

Le test T32 d'indétermination utilise une injection dans le parcours de décision ; la ressource manquante a été testée séparément sur une copie du candidat, sans mutation de l'original ([traces](preuves-recette-cpu-l3.zip)). Ces résultats ne remplacent pas les témoins matériels.

## Suivi des preuves restantes

- **T33 complet : NOT RUN.** Premier lancement et inférence sans dépôt, SDK CUDA ni outils de build, reportés à L4 par décision utilisateur ; la seule réduction du `PATH` ne démontre pas cette condition.
- **T41 et T42 : NOT RUN.** Témoin x86-64-v2 exact et Windows ARM64 sous émulation : accès, protocole UIA et mandat d'exécution/activation CI requis ; aucun job n'est activé par cette remise.
- **T43 : partiel.** WAV CPU confirmé sur cette machine ; MP3, micro, GPU, Auto et témoins matériels restent à exercer selon les critères ci-dessus et le [plan](plan-implementation-distribution-cpu-x64.md).
- **T45 : en attente.** Consolider les résultats seulement après réception des preuves ; aucune publication, qualification ni distribution ne découle de cette fiche.
