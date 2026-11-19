Avancée : protocole de faisabilité des ressources Parakeet existantes préparé.
Blocage : pilote, admission de l’essai local et compatibilité des binaires historiques avec le protocole à vérifier.
Décision : mandat proposé à l’utilisateur avant toute préparation exécutable ou exécution.

## Objectif et portée

Déterminer si les workers et DLL identifiés dans [l’inventaire](inventaire-parakeet-distribution.md) chargent le modèle et transcrivent un WAV sur les trois témoins retenus. Quatre lancements maximum, dans cet ordre : F1 CPU sur Z790, F2 CUDA sur Z790 après F1 réussi, F3 CPU sur Dell, F4 CPU sur Snapdragon. Un lancement et une inférence par cas ; arrêt de la campagne au premier écart, sans relance automatique. Dell et Snapdragon sont opérés par l’utilisateur depuis leur bureau. Les délais sont des limites de diagnostic, pas des critères de vitesse produit.

Cette faisabilité worker ne valide ni desktop, ni Auto, MP3, micro ou exports TXT/SRT. Les droits de redistribution, la compatibilité générale et la qualification du futur paquet restent à établir séparément.

## Préparation proposée

Dans `C:/wt/distcpu-l2-reprise/deliverables/parakeet-faisabilite/`, préparer un pilote PowerShell 5.1 autonome, sa notice et deux ZIP pour l’opérateur : CPU Dell/Snapdragon et CPU+GPU Z790. Les sources sont le modèle `C:/dev/whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf`, les deux workers et les 14 DLL de `C:/dev/whisper/target/n2-native/qualification-gpu/inputs/personal-runtime/`, dont les SHA-256 sont dans l’inventaire, et `C:/dev/whisper/docs/cadrage/feas-pm-01-corpus/fr_passage_28-32.wav` SHA-256 `1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad`. Le modèle attendu est `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`. Vérifier avant copie, après copie et dans les ZIP ; divergence ou fichier absent arrête la préparation. Les ressources et preuves existantes restent conservées.

Noms prévus : `Essai-Parakeet.ps1`, `Parakeet-faisabilite-CPU.zip` et `Parakeet-faisabilite-Z790.zip` dans ce répertoire. Le répertoire de travail des workers est la racine de session propre au cas, et tous les journaux et sorties sont sous cette racine.

Les ZIP sont extraits dans une racine de l’opérateur. Chaque lancement crée une session neuve sous `C:/Tests/ParakeetFaisabilite/<identifiant>/` sur la machine concernée, contenant `profil/Whisper/models`, `native/cpu`, `native/gpu` si GPU, `worker`, `logs`, `temp` et résultats. Fixer `LOCALAPPDATA` à `profil`, `TEMP` et `TMP` à `temp`, `PARAKEET_L2_RUNTIME_ROOT` à `profil/Whisper`. N’utiliser aucune installation Whisper réelle ni répertoire de toolkit comme source de DLL. Vérifier chemins canoniques, absence de collision et intégrité à l’arrivée ; laisser les copies et preuves en place.

Faire relire indépendamment le pilote, ses préflights et sa politique d’arrêt avant toute exécution. Le pilote devra être testé sur fixtures synthétiques pour les protocoles positif et négatifs, délais et archives, sans lancer le worker ni charger les DLL. Les commandes exactes, chemins des ZIP et empreintes finales seront communiqués après préparation et revue.

Sur Z790, vérifier si la garde native existante admet explicitement ce pilote sans Cargo. En son absence, l’autorisation de ce mandat couvre l’exécution bornée du seul pilote et des workers identifiés hors garde, avec racines ci-dessus, après revue. Aucun contournement d’un refus de garde ni aucun Cargo n’est autorisé. Sur Dell et Snapdragon, les mêmes écritures et exécutions bornées sont autorisées dans leur racine `C:/Tests/ParakeetFaisabilite/` ; aucune modification du profil réel ou élévation.

## Préflight et protocole

Sur chaque machine, relever Windows, architecture et capacités x64 réellement visibles, avec CPUID/XGETBV lorsque disponibles, sans déduire la compatibilité d’un nom de CPU. Sur Snapdragon, vérifier l’architecture ARM64 de l’OS et l’émulation x64 effective du worker ; si cette preuve manque, conserver l’essai comme incomplet. Le pilote vérifie tous les SHA-256 des composants effectivement utilisés et du WAV, avant et après exécution. Signaler toute capacité non établie ; erreur ou divergence empêche le lancement.

F2 identifie le `nvidia-smi.exe` utilisé par le worker, son chemin et empreinte, le pilote et la seule GTX 1080 Ti, son UUID et PCI. L’inventaire NVIDIA doit correspondre au worker et le PATH doit résoudre cet outil et les DLL système, sans toolkit. Plusieurs GPU NVIDIA, outil absent ou identité ambiguë arrêtent F2 avant inférence. Le journal natif GPU est dans la session et reçoit stderr dès le lancement. Les données stdout et stderr sont drainées en parallèle et les délais couvrent tous les appels et lectures ; aucune attente synchrone non bornée.

Le worker CPU est lancé avec `--model <profil/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf> --model-sha256 e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e --native-dir <profil/Whisper/native/cpu>`. GPU utilise le worker GPU, `native/gpu` et `PARAKEET_L3_ASR_LOG` pointant sur le stderr du worker. Le pilote applique le protocole JSONL Parakeet v1 décrit dans `crates/whisper-core/src/parakeet_ipc.rs` : après `ready`, vérifier indépendamment l’identité modèle, build, profil et runtime calculée à partir de la version native et des hashes de DLL, ainsi que le backend CPU ou CUDA0. Les sources actuelles peuvent différer des exécutables historiques ; une incompatibilité de protocole arrête l’essai.

Envoyer une unique commande `transcribe` avec request_id 1, job_id propre au cas, generation et attempt 1, identité vérifiée, chemin absolu du WAV, hash source et `expected_gpu` null en CPU ou UUID/PCI contrôlés en GPU. Lire `result` corrélé puis envoyer `shutdown`, exiger `stopped` et sortie 0. Pour CUDA, exiger aussi `native_call_started`, preuve CUDA0 et identité GPU cohérente. CPU ne requiert pas cet événement.

PASS exige un résultat corrélé, 64000 échantillons, texte non vide et reconnaissable du passage de référence, mots et horodatages positifs, ordonnés, sans chevauchement et bornés à 4000 ms, identité et hash conformes, puis arrêt établi. Conserver le texte pour vérification humaine. FAIL signifie résultat explicite incorrect ; INCOMPLET signifie preuve insuffisante, timeout, arrêt ou environnement non établi. Aucune réussite déduite d’un simple code 0.

## Délais et arrêt

Par cas : 5 minutes pour ready, 30 minutes pour result, puis 30 secondes pour stopped et sortie. À expiration, tenter shutdown et fermer stdin, puis attendre 30 secondes. Le worker traite l’appel natif de façon synchrone : shutdown peut rester en attente. Si nécessaire, autorisation de terminer uniquement le worker identifié par son handle, PID, image et heure de création, ainsi que l’enfant nvidia-smi dont PID, image et ascendance ont été effectivement établis. Aucune terminaison par nom ou de processus non attribué. Si l’identité ou la fin d’un processus ne peut pas être établie, arrêter la campagne, préserver l’état et demander arbitrage.

Chaque cas conserve commandes effectives, identités d’entrées, version Windows et architecture, données CPU/GPU, stdout JSONL, stderr, temps, codes et verdict dans un ZIP vérifié. L’opérateur récupère les ZIP Dell et Snapdragon ; le compte rendu distingue faisabilité des ressources et qualification du produit. Une incapacité détectée sur un témoin ne déclenche aucune reconstruction : une solution native et son mandat seront soumis séparément.
