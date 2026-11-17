# Remise L4 — candidat WhisperTurbo

Avancée : gate L3→L4 ouverte après la recette CPU sur les moyens disponibles et décision utilisateur du 17 novembre 2026.
Blocage : qualification et distribution interdites tant que les essais obligatoires, dont T33 complet, ne sont pas acquis.
Décision : l'utilisateur reporte à L4 l'essai sans dépôt, SDK CUDA ni outils de build ; il reste obligatoire avant distribution.

**Décision utilisateur du 18 novembre 2026 :** les essais de panne GPU avant audio et après progrès sont hors périmètre de qualification L4 ; la piste de débogage et son prototype sont abandonnés. Les sections historiques ci-dessous décrivent les études et observations déjà réalisées : elles ne prescrivent plus ces essais. Les comportements de panne existants ne sont ni modifiés ni considérés comme qualifiés. Les scénarios Auto GPU absent/incompatible et GPU disponible, ainsi que les autres critères matériels applicables, restent au périmètre.

Le paquet candidat déjà assemblé contient une copie antérieure du plan parmi ses notices ; il n'est pas modifié par cette décision. Le script d'assemblage et le bootstrap vérifient l'empreinte de cette notice : un assemblage ultérieur utilisant le plan actualisé devra donc réconcilier cette empreinte avant de réussir.

## Candidat et configuration provisoire

Le candidat local est le dossier `C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\turbo-package-cpu-fix\` : 19 fichiers, dont huit notices. Il ne s'agit pas d'une distribution qualifiée. Le modèle est fourni séparément, sans téléchargement par l'application. Ne pas modifier ce dossier pour les essais. Son fichier `README-INSTALL-CANDIDATE.md` décrit le contenu ; ses indications historiques « inférence non exécutée » et « validation utilisateur NOT RUN » sont remplacées par les résultats utilisateur ci-dessous.

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

Sur cette machine, la [recette CPU isolée](recette-manuelle-cpu-candidat.md) a installé ce candidat et le modèle vérifié dans `C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\recette-cpu-fix\LocalAppData\WhisperTurbo\`. Elle protège le profil Windows réel ; le script de première installation refuse de réutiliser un dossier déjà existant. Les résultats fonctionnels CPU, GPU et Auto communiqués ensuite sont ci-dessous ; ils ne valident pas les autres machines.

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

### Essai à préparer sur le second PC Intel

L'utilisateur indique disposer d'un PC Intel de 11e génération, sans GPU dédié ni outils de développement ; AMD et ARM64 ne sont pas disponibles pour l'instant. Cet essai peut couvrir T33 et le cas Auto sans GPU NVIDIA utilisable, si les conditions et observations sont confirmées. Il ne remplace ni le témoin v2 exact ni les autres bancs.

Copier sur ce PC, sans modifier les originaux, le dossier candidat entier en le nommant `paquet`, le modèle `ggml-large-v3-turbo.bin` et le WAV de référence `fr_passage_28-32.wav`, tous dans un même dossier choisi par l'utilisateur. Prévoir au moins 5 Gio libres pour les copies et l'installation. Vérifier que ce PC n'a accès ni au dépôt, ni au SDK CUDA, ni aux outils de build du PC de développement ; relever sa version et son édition de Windows. Si une condition est incertaine, la signaler plutôt que conclure à une absence.

Sur le second PC, ouvrir PowerShell, puis copier le bloc suivant. Il demande le chemin du dossier choisi, vérifie les empreintes et crée uniquement une racine `profil-local` neuve. Les données applicatives restent isolées dans cette racine. Le bootstrap et l'application créent eux-mêmes leur état.

```powershell
$ErrorActionPreference = 'Stop'
$base = (Read-Host 'Chemin du dossier contenant paquet, modèle et WAV').Trim().Trim('"')
$files = @{
    'paquet\whisper-bootstrap.exe' = 'd7852efb06b5c010623ab8ee9833c6b23dbc7515f4208e5bff3d934febef6f5a'
    'paquet\app\whisper-desktop.exe' = '7f352689d12787d529462438c5e1ad24c8a741f5c7cf134e0cb5ddc09090ecfb'
    'paquet\app\whisper-worker-cpu.exe' = 'a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b'
    'paquet\app\whisper-worker-gpu.exe' = 'd874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9'
    'ggml-large-v3-turbo.bin' = '1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69'
    'fr_passage_28-32.wav' = '1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad'
}
foreach ($name in $files.Keys) {
    if ((Get-FileHash -LiteralPath (Join-Path $base $name) -Algorithm SHA256).Hash -ine $files[$name]) { throw "Fichier différent du candidat : $name" }
}
$localTestRoot = Join-Path $base 'profil-local'
if (Test-Path -LiteralPath $localTestRoot) { throw "Le profil existe déjà : $localTestRoot ; ne pas l'écraser." }
New-Item -ItemType Directory -Path $localTestRoot | Out-Null
$env:LOCALAPPDATA = $localTestRoot
Write-Host 'À la question du modèle, copiez ce chemin :'
Write-Host (Join-Path $base 'ggml-large-v3-turbo.bin')
& (Join-Path $base 'paquet\whisper-bootstrap.exe')
if ($LASTEXITCODE -ne 0) { throw 'Installation échouée : conserver le message et arrêter.' }
$desktop = Join-Path $localTestRoot 'WhisperTurbo\app\whisper-desktop.exe'
if (-not (Test-Path -LiteralPath $desktop)) { throw 'Desktop installé absent : conserver le message et arrêter.' }
Start-Process -FilePath $desktop -WorkingDirectory $base
```

Après installation, ouvrir le WAV copié dans Whisper, sélectionner CPU et vérifier une transcription ainsi que TXT et SRT. Puis sélectionner Auto, refaire l'import et relever la préférence affichée, le moteur effectivement utilisé, les éventuels messages et les sorties. Le résultat attendu sans GPU NVIDIA utilisable est CPU sans transcription GPU ; la sonde GPU reste permise. Signaler toute erreur ou ambiguïté sans modifier les DLL, le pilote, le paquet ou l'installation. Une preuve complète du nombre et du rôle des processus peut demander une observation complémentaire ; le simple résultat affiché ne la démontre pas.

### Retour utilisateur — 17 novembre 2026

L'utilisateur rapporte, avec large-v3-turbo : import WAV CPU et microphone CPU « ok » ; import WAV GPU et microphone GPU « ok », perçus comme quasi instantanés ; en mode Auto, sélection du GPU NVIDIA GTX 1080 Ti. Ce retour s'inscrit dans la recette de l'installation isolée remise ci-dessus. Après ce retour, les empreintes des trois exécutables installés correspondaient encore au candidat et le desktop ainsi que le worker GPU de cette installation étaient ouverts ; cela n'établit pas indépendamment toutes les observations pendant les essais.

L'utilisateur confirme ensuite avoir vérifié, pour les essais micro CPU, GPU et Auto, la présence des fichiers WAV, TXT et SRT et la conformité des transcriptions, phrase finale comprise. Il indique que le MP3 correspondant au WAV testé a été importé et transcrit conformément en CPU et en GPU. Il confirme également qu'un bloc de texte est apparu avant l'arrêt d'un enregistrement micro, sans préciser pour quels modes ; la nature confirmée de ce bloc n'est pas précisée.

L'utilisateur estime la qualité de transcription identique en CPU et GPU. Il indique que le CPU est très long et le GPU ne prend que quelques secondes au plus ; ces appréciations ne sont pas des mesures de durée. La sélection GPU en Auto ne démontre pas les scénarios GPU absent, échec avant audio ou panne après début de traitement.

Mesures rapportées ensuite par l'utilisateur sur le GPU : WAV de 5 minutes transcrit en 1 minute 11 secondes, soit environ 4,2 fois plus vite que la durée de l'audio ; premier texte micro après 5 secondes. L'utilisateur confirme que ce premier texte était un bloc confirmé. Ces observations satisfont les deux seuils de temps sur cet essai ; elles ne constituent pas une qualification de toutes les configurations GPU.

Relevé local après ces retours : Windows 11 Professionnel 64 bits, build `26200` ; processeur Intel Core i9-14900K ; NVIDIA GeForce GTX 1080 Ti, pilote Windows `32.0.15.8129`. Les empreintes des trois exécutables installés correspondent toujours aux valeurs ci-dessus. Ce relevé ne démontre pas le prédicat CPUID complet, ni que cette machine est le témoin v2 exact de T41 ; aucun témoin AMD ou Windows ARM64 n'est établi par ce relevé.

### Retour utilisateur — second PC Intel

L'utilisateur rapporte que l'application démarre malgré des fenêtres d'erreur NVIDIA, que les imports et le microphone fonctionnent en CPU et Auto et que les transcriptions sont de la qualité attendue. Il indique environ quatre minutes pour décoder le WAV testé ; sa durée et le mode de ce chronométrage ne sont pas précisés. La capture montre une erreur système du processus `whisper-worker-gpu.exe` indiquant que `nvcuda.dll` est introuvable. Ce défaut visible sur un PC sans NVIDIA reste à diagnostiquer et à corriger ; il n'invalide pas les transcriptions CPU observées. Le worker GPU importe `nvcuda.dll` et le desktop lance une sonde GPU au démarrage : ce sont des éléments compatibles avec la fenêtre, mais la chaîne exacte reste à établir.

L'utilisateur confirme ensuite la sélection effective du CPU en Auto et l'absence de GPU NVIDIA et de SDK CUDA ; il avait indiqué que ce PC ne possède pas d'outils de développement. Sa capture identifie Windows 11 Famille 26H2, build `26300.9711`, Intel Core i7-1185G7, 16 Go de RAM et un circuit graphique intégré Intel Iris Xe. L'accès éventuel au dépôt et l'identité des fichiers copiés ne sont pas confirmés ; le résultat affiché ne démontre pas à lui seul l'absence de transcription GPU avant le choix CPU. T33 complet et la preuve exhaustive du parcours Auto restent partiels. Sur le worktree de développement, le WAV de référence suivi `docs/cadrage/feas-pm-01-corpus/fr_passage_28-32.wav` est constaté absent après la préparation de l'essai ; son origine est inconnue et il n'a pas été restauré.

## Suivi des preuves restantes

- **T33 complet : partiel.** Premier lancement et inférence rapportés sur le second PC ; conditions d'environnement et identité à confirmer, fenêtre d'erreur GPU à traiter. La seule réduction du `PATH` sur le premier PC ne démontrait pas l'absence du dépôt, du SDK CUDA et des outils de build.
- **T41 et T42 : NOT RUN.** Témoin x86-64-v2 exact et Windows ARM64 sous émulation : accès, protocole UIA et mandat d'exécution/activation CI requis ; aucun job n'est activé par cette remise.
- **T43 : partiel.** WAV et MP3 correspondants en CPU/GPU, micro CPU/GPU/Auto avec exports et phrase finale, sélection GPU en Auto et apparition d'un bloc micro avant arrêt signalés fonctionnels par l'utilisateur ; qualité CPU/GPU estimée identique et temps GPU relevés dans la recette du candidat actuel. Les preuves Auto applicables et les témoins matériels restent à consolider selon le [plan](plan-implementation-distribution-cpu-x64.md) ; les pannes provoquées sont hors périmètre.
- **T45 : en attente.** Consolider les résultats seulement après réception des preuves ; aucune publication, qualification ni distribution ne découle de cette fiche.

## Préparation des pannes Auto — historique, hors périmètre

Ces essais portent sur le candidat `turbo-package-message-fix`, bootstrap SHA-256 `59c94bac7bf36260048e885bd24ee0f8ee31b3598e91d0ef0acb3fcb257efc4b`, desktop `01e2ea8ba221ed3245b9227c2fdb2bd5a36da073bff8305b255ad4dfe1be1f48`, worker CPU `a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b` et worker GPU `d874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9`. Le modèle et le WAV de référence sont ceux de la [recette du candidat](recette-second-pc-gpu-fix.md). Ne pas altérer les fichiers du candidat. Les tentatives avec entrée de test se font dans une installation et un dossier de sortie isolés, sans données utilisateur à préserver.

| Situation | Préparation et observation attendue | État et limite |
|---|---|---|
| GPU absent ou incompatible | Sur le second PC Intel sans NVIDIA, utiliser Auto et vérifier la préférence conservée, le moteur CPU, la cause affichée et les sorties. Une sonde GPU préalable est permise ; il ne doit pas y avoir de transcription GPU. | L'utilisateur a confirmé Auto sur CPU et les exports pour le candidat précédent, ainsi que la transcription Auto pour le candidat actuel. L'absence de transcription GPU et l'unicité de la tentative n'ont pas été établies par un relevé corrélé : preuve partielle. |
| GPU compatible | Sur la GTX 1080 Ti, utiliser Auto et constater GPU, sortie conforme et préférence Auto. | L'utilisateur confirme GPU en Auto, import et micro conformes sur le candidat actuel. |
| Échec GPU avant audio | Préparation historique d'un essai avec repli CPU et maintien d'Auto. | **Hors périmètre** depuis la décision utilisateur du 18 novembre 2026 ; non exécuté et non qualifié. |
| Panne GPU après progrès | Préparation historique d'un essai vérifiant l'absence de repli après traitement commencé. | **Hors périmètre** depuis la décision utilisateur du 18 novembre 2026 ; non exécuté et non qualifié. |

Pour les deux pannes, le protocole doit d'abord identifier l'EXE exact, le PID, l'heure de création, l'ascendance et le job/instance concernés, puis vérifier à nouveau cette identité juste avant une éventuelle interruption ; ne jamais arrêter un processus non identifié. Observer les événements `Ready`/`Segment`/`End` et les changements de moteur pour le même job, et compter les lancements CPU. Si les événements ou le lien PID/job ne sont pas observables de façon fiable sur le paquet réel, marquer le cas `NOT RUN`, sans inventer de résultat ni ajouter de hook au produit. La méthode précise de stimulus et de collecte reste à établir avant tout essai ; l'utilisateur effectue la recette dans la fenêtre Windows.

La N1 gardée du candidat a exécuté des tests avec faux workers pour un échec GPU avant `Begin`, un échec d'initialisation, l'absence de repli après démarrage et le refus en GPU manuel. Ils apportent une preuve logicielle, pas la preuve d'une panne du worker réel ni de son exécution matérielle. La recette GPU N2 `TurboGpuReusableImports` reste aussi un gate distinct avant distribution et demande son mandat propre.

## Témoins matériels restant à obtenir

- **Intel et NVIDIA :** deux PC Intel et une GTX 1080 Ti ont été exercés ; conserver les résultats et les limites dans la [fiche de recette](recette-second-pc-gpu-fix.md). Les scénarios de panne ci-dessus restent ouverts.
- **AMD :** obtenir un PC Windows compatible, installer et exercer le même candidat sur les parcours applicables. Aucun banc AMD n'est actuellement disponible.
- **x86-64-v2 exact et sous v2 :** obtenir le témoin T01 avec toutes les capacités v2 et aucune v3/v4, ainsi qu'un témoin sous v2 capable de lancer le bootstrap pour le refus réel. L'étiquette d'un processeur Intel ne prouve pas ce prédicat.
- **Windows ARM64 :** obtenir un environnement Windows ARM64 permettant d'exécuter le même paquet x64 sous émulation, avec preuve OS/PE et pilotage GUI reproductible. Aucun tel banc n'est actuellement disponible ; ce n'est pas une qualification ARM native.
- **CI T41/T42 :** accès à des runners et mandat explicite d'activation/exécution, ainsi que méthode UI Automation et captation des capacités prouvées. Sans eux, les deux jobs restent `NOT RUN` ; aucun workflow n'est activé par cette préparation.
- **T33 complet :** confirmer l'absence d'accès au dépôt et aux outils de build sur le second PC, ainsi que l'identité des fichiers de la recette avant de conclure. L'absence de SDK CUDA et le fonctionnement CPU ont été rapportés ; ne pas présumer les autres éléments.

T45 et la distribution restent bloqués tant que les critères obligatoires applicables sont `NOT RUN` ou incomplets. L'obtention d'un témoin et l'activation CI font l'objet d'un mandat adapté avant exécution.

### Étude des moyens d'observation

Le journal `worker-0.jsonl` contient notamment date, job, génération, moteur, `worker_started`, `backend_ready` et progression ; le type `DiagnosticRecord` ne contient ni PID ni identifiant d'instance. Le transport consomme les événements IPC du worker ; le marqueur explicite `worker_pid` trouvé dans le transport est conditionné à la feature distincte `l01-memory-qualification`. Il n'est donc pas établi qu'il soit présent dans le candidat remis. En particulier, `worker_started` ou la seule présence d'un processus GPU ne prouve pas que ce PID traite ce job, ni que la frontière avant audio n'a pas été franchie.

Une méthode fiable pour interrompre au bon moment et attribuer les deux pannes au job réel n'est pas encore établie. Aucune terminaison de processus, instrumentation ni recette de panne n'a été exécutée dans cette étude. Les tests simulés et les résultats utilisateurs ne lèvent pas cette limite.

Mandat d'observation proposé avant une éventuelle recette de panne : sur le PC NVIDIA, utiliser une seule transcription GPU Auto normale du candidat identifié, avec un fichier de test et un dossier de sortie isolé ; l'utilisateur pilote la fenêtre. Pendant cette transcription, relever uniquement les processus desktop et workers Whisper par leur chemin, PID, parent et heure de création, sans les arrêter ; après la fin, lire les enregistrements du diagnostic de cette sortie et relever job, génération, moteur et progrès, ainsi que l'éventuelle ligne de diagnostic d'identité du worker. Une seule exécution, aucune élévation, aucun téléchargement, aucun changement du paquet ou des réglages Windows. Préserver les sorties et demander l'arbitrage si leur emplacement ou l'identité du candidat n'est pas certain. Cette observation peut révéler des indices utiles, mais ne démontrera pas à elle seule la frontière avant audio ni l'unicité de toutes les tentatives ; si la corrélation PID/job reste absente, ne pas interrompre de worker et proposer une décision distincte sur les moyens de preuve.

La lecture ponctuelle proposée pendant la transcription est la commande ci-dessous ; elle ne modifie ni n'interrompt de processus. Une valeur absente ou ambiguë ne permet pas d'identifier le worker. Le dossier de sortie prévu est `C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\obs-auto-l4`, à condition qu'il soit neuf et sans jonction ; l'application crée elle-même son état. Le chemin exact de l'installation utilisée doit être confirmé avant l'essai.

```powershell
Get-CimInstance Win32_Process -Filter "Name='whisper-desktop.exe' OR Name='whisper-worker-gpu.exe' OR Name='whisper-worker-cpu.exe'" |
    Select-Object Name, ProcessId, ParentProcessId, CreationDate, ExecutablePath |
    Format-Table -AutoSize
```

**Observation unique exécutée le 18 novembre 2026.** Après vérification des neuf fichiers applicatifs et du modèle, une installation isolée complète a été créée par le bootstrap sous `target/parakeet-tests/cpu-distribution-x64-20261115/obs-auto-l4/profil-complet/WhisperTurbo`. Une première fenêtre ouverte avec un profil vide ne pouvait pas accéder au modèle : l'utilisateur l'a fermée normalement ; aucune transcription n'avait été lancée. L'utilisateur a ensuite lancé une transcription Auto et confirmé sa fin.

À 00:20:33,966 (+01:00), un seul relevé des processus a observé le desktop PID `121232` et son enfant direct `whisper-worker-gpu.exe` PID `64128`, créés respectivement à 00:18:56 et 00:20:25 depuis l'installation isolée. Aucun worker CPU n'était présent dans cet instantané ; aucun processus n'a été interrompu. Le diagnostic de sortie contient 63 lignes pour le même job et la génération 1 : un `worker_started` CUDA, un `backend_ready` CUDA et 61 progrès, le dernier à `4 800 000/4 800 000` échantillons. Les fichiers TXT et SRT sont présents. Le [journal archivé](preuves-observation-auto-l4.zip) a été comparé à la source par SHA-256.

Ce journal ne contient ni PID ni identifiant d'instance ; l'instantané montre une concordance temporelle et une parenté, sans preuve directe liant le PID au job. Il ne prouve pas non plus l'absence de processus entre les instants observés, ni la frontière avant audio. Les flux IPC `Ready`/`Segment`/`End` n'ont pas été capturés par cette observation. Les deux scénarios de panne restent `NOT RUN` ; aucune méthode fiable d'interruption à la frontière avant audio n'est établie par ce seul essai. Les sorties de recette sont conservées dans le dossier isolé, sans suppression.

### Étude historique : piste d'observation externe abandonnée

Le transport Turbo échange des trames JSON terminées par un saut de ligne. Les commandes de session et les événements du worker portent `job_id`, `generation` et `instance_id`, mais ces valeurs ne sont pas reliées à un PID dans le journal applicatif. Pour l'import Auto, le worker signale `Ready` puis attend une commande `BeginImport` corrélée avant de traiter l'audio ; le journal existant ne prouve pas si cette commande a été envoyée. La simple surveillance de `worker_started` ou l'absence d'une ligne de progrès ne suffit donc pas à situer la panne avant audio.

Une piste à évaluer serait de démarrer le desktop sous un débogueur Windows externe, avec suivi des créations de ses enfants, et de conserver le handle du worker identifié par son image. L'option Windows `DEBUG_PROCESS` prévoit le suivi des enfants ; s'attacher après coup à un processus avec `DebugActiveProcess` ne suit pas automatiquement ses enfants ([création de processus](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags), [attachement](https://learn.microsoft.com/en-us/windows/win32/debug/debugging-a-running-process)). Un dispositif pourrait ensuite contrôler le démarrage d'un enfant et observer ses trames IPC pour lier l'image et le PID à la session, puis interrompre uniquement ce handle avant audio ou après progrès. Mais la lecture complète et sans perturbation des trames, la distinction sonde/session, le maintien de la fenêtre et la gestion d'une panne du débogueur ne sont pas établis. Les outils `cdb.exe` et `windbg.exe` ne sont pas présents aux emplacements habituels examinés. Ceci est une hypothèse de méthode, pas un protocole fiable ni une autorisation d'exécution.

Le prototype externe envisagé lors de cette étude est abandonné avec la piste de débogage ; il ne doit pas être préparé ni exécuté au titre de L4.
