Avancée : dépendance CUDA confirmée sur six artefacts CPU, dont le release et des copies locales de recette.
Blocage : résolutions Cargo et compilations froides A/B/D suspendues, garde N1 inchangée incompatible.
Décision : diagnostic partiel, aucun correctif ni option produit choisi ; arbitrage de garde nécessaire pour achever les mesures.

# R8-DIAG — cause et impact CPU

## Verdict

Le CPU historique utilisait effectivement `whisper-rs-sys[cuda]` ; les binaires CPU normaux et les copies locales de livraison inspectés importent les trois DLL CUDA. L’unification des features CPU/GPU explique ce lien dans une invocation commune, également documentée pour la livraison avec `-p CPU -p GPU`. L’effet d’une construction CPU seule reste non mesuré : les quatre `cargo tree` et A/B froids n’ont pas été exécutés.

## Hypothèses et preuves

Lecture au commit **939fbbbe**, sans Cargo ; références/conditions exactes dans [lecture.json](raw/r8/lecture.json), imports dans [pe-binaries.json](raw/r8/pe-binaries.json).

| H | Statut | Preuve et limite |
| --- | --- | --- |
| H1 — union | **CONFIRMÉE, mécanisme source/cache** | Même whisper-rs 0.16 CPU/GPU ; GPU demande cuda/raw-api. Fingerprint CPU C → RS[_gpu,cuda,raw-api] → SYS[cuda] → link cublas/cudart/cuda. A/B comparatif non mesuré. |
| H2 — défaut CPU | **ÉCARTÉE, manifestes lus** | CPU/common sans activation CUDA ; défauts RS/SYS sans CUDA. Pas de patch/replace ni dépendance héritée masquant l’activation. |
| H3 — environnement | **NON DÉTERMINÉE historiquement** | CUDA_PATH seul n’active pas la feature ; GGML_CUDA est OFF par défaut. SYS transmet néanmoins GGML_*/CMAKE_* à CMake : forçage possible, historique inconnu. |
| H4 — lien global | **NON DÉTERMINÉE historiquement** | Aucun forçage actuel observé ; rustflags=[] dans les fingerprints C. Les liens CUDA relevés viennent de SYS[cuda] ; ancien environnement LINK/CL non reconstitué. |
| H5 — autre cause | **NON DÉTERMINÉE** | Pas d’autre cause nécessaire identifiée ; commande/features explicites ou état historique différent non éliminés sans expériences froides. |

Cargo réunit les features des dépendances communes sélectionnées ensemble, y compris plusieurs `-p`. Les default-members excluent **déjà tous les workers** ; ils ne changent pas --workspace ou -p CPU -p GPU. [Cargo](https://doc.rust-lang.org/cargo/reference/resolver.html#features).

## Binaires

« Oui » signifie imports **directs** de cublas64_12.dll, cudart64_12.dll et nvcuda.dll ; aucun import différé pour les six fichiers. Chemins complets, tailles, SHA-256 et dates UTC : pe-binaries.json.

| Sujet | DLL CUDA | Commande / provenance |
| --- | --- | --- |
| A, CPU debug froid | Non mesuré | `cargo build --offline --locked -p whisper-worker-cpu`, NON EXÉCUTÉ |
| B, CPU test froid | Non mesuré | `cargo test --offline --locked -p whisper-worker-cpu --no-run`, NON EXÉCUTÉ ; harness actuel différent du C ancien |
| C, CPU legacy 36e5609d | **Oui** | Cache existant et fingerprints corrélés ; commande exacte non attribuée |
| D, A sans variables CUDA | Non mesuré | Conditionnel H3, NON EXÉCUTÉ |
| CPU normal debug / release partagé | **Oui / oui** | Fichiers existants, commandes exactes non attribuées à leurs SHA |
| Deux copies de recette récentes / installation isolée du 19 octobre | **Oui / oui** | Copies récentes identiques au release ; ancienne copie distincte |

## Impact produit : oui, pour ces artefacts

Le CPU force use_gpu(false), mais ce réglage intervient après le chargement des DLL. Si une DLL obligatoire manque, Windows ne démarre pas le processus ; mode CPU et repli Auto peuvent donc échouer avec ces binaires. Le code de repli aboutit à WorkerExited/Fail ; aucun scénario matériel n’a été joué. [Windows](https://learn.microsoft.com/en-us/windows/win32/dlls/load-time-dynamic-linking).

Le lanceur récent utilise le Turbo voisin du desktop ; LOCALAPPDATA sert à Parakeet. L’installation personnelle attendue est absente. install-personal.ps1 copie sans compiler ; deux plans documentent une invocation release CPU+GPU commune, un autre deux invocations séparées (lecture.json). Aucun SHA release n’est rattaché avec certitude à une commande.

## Options, sans sélection ni implémentation

| Option / fichiers futurs | Livraison / cargo test --workspace | Risque et preuve suivante |
| --- | --- | --- |
| Compiler CPU/GPU séparément ; procédures/scripts de livraison | Vise un CPU sans CUDA ; l’union workspace reste présente | Refaire A/B froids, puis PE de chaque paquet réellement livré |
| CUDA GPU optionnel ; manifestes et composition native | Une sélection commune de CUDA peut encore réunifier ; workspace sans CUDA à qualifier | GPU omis par oubli ; prouver construction et disponibilité GPU explicites |
| Isoler réellement bindings/graphes CPU-GPU ; manifestes, wrappers et procédures | Vise aussi les builds communs ; effet tests à mesurer | Aliases d’un même package insuffisants ; prouver ABI, packaging et isolation |
| DEC-T-R8-01, harness purs existants ; tests/manifeste/pure FFI | Tests autonomes ; **ne corrige pas les DLL du CPU livré** | Réinclusions #[path] : risque duplicate_mod ; maintenir identités et contrôles PE |

## Non vérifié et arrêt

Aucun Cargo, compilateur, worker, test ou modèle lancé ; aucune release reconstruite. La garde fixe ancienne racine/cible partagée et argv workspace, incompatibles avec tree/build CPU et targets neuves. [Constat et commandes prévues](raw/r8/execution-compatibility.json), [plan](raw/r8/plan.md). Pour achever : autoriser une extension N1 bornée de la garde, ou conserver ce diagnostic partiel ; aucune de ces décisions n’est présumée. Revue indépendante à consigner dans raw/r8 ; aucun fichier de code/configuration/garde modifié.
