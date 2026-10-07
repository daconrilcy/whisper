# Plans Whisper — P-WHISPER-15

Statut : DRAFT, revue indépendante exacte de P15 requise. Base PLANS immuable : P-WHISPER-14, digest `2724e325c5377d725b4e4f73541b6079a7bf08a9ff8e4328b10768e6823ca55b`. Parent DESIGN immuable : D-WHISPER-19, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

Le registre actif est `docs/predev/state.json`. À la préparation de P15, il indique D19 READY, P14 READY avec R-WHISPER-PLANS-14-v1 CLEAN, L00–L03 completed et L04–L07 planned. Les mentions DRAFT ou P10 actif dans les documents hérités décrivent leurs versions historiques et ne remplacent pas le registre courant.

## Mandat et autorisations

La demande utilisateur autorise la préparation et la revue indépendante de P15 pour compléter le dossier de préflight L04. L’autorisation d’implémenter L04 demeure NOT_REQUESTED. Elle devra être consignée séparément avec sa source et sa portée exacte avant le contrôle d’exécution L04.

P15 ne change ni D19, ni les réponses acceptées Q-04/Q-06/Q-09 et DETAIL-P03, ni la couverture produit, ni l’allowlist L04. Les campagnes produit, runtime, native UI et qualification L04 restent PROPOSED / NOT RUN.

## Corpus et changements P15

P15 conserve intégralement le corpus P14 et apporte quatre changements documentaires :

- CHANGE-P15-01 : gel du contrôleur central effectif et analyse de son correctif de promotion entre racines documentaire et code.
- CHANGE-P15-02 : représentation vérifiable du fichier CPU hérité de L01 et des quatre créations futures attendues dans le périmètre L04.
- CHANGE-P15-03 : préparation du relien de la dépendance L04 vers la clôture canonique L03 actuellement liée à P14.
- CHANGE-P15-04 : séparation de la promotion documentaire PLANS et du préflight d’implémentation L04, lequel exige encore un mandat de code applicable.

Chaque changement conserve sa source, ses preuves et ses limites. Aucun changement de contrat produit ou de décision structurante n’est introduit. KEEP_D19 est une conclusion d’impact proposée à vérifier par la revue indépendante P15.

## Lots et progression contrôlés

Le graphe et les couvertures des lots sont conservés. L04 dépend de L03 EXECUTION; L05 dépend de L04 EXECUTION. Aucune parallélisation de ces lots n’est proposée.

Le fichier `crates/whisper-worker-cpu/src/native_engine.rs` existe et correspond à un output vérifié de L01 completed. Les quatre autres chemins L04 sont absents au HEAD observé `1310bbcddb8c32d7988211ff0cdd76224dae62b0`; leur création est future. Leur absence ne reçoit aucun hash fictif et ne provoque aucune création de fichier vide.

La preuve canonique actuelle L03 reste liée au candidat réellement exécuté P14. P15 ne la réattribue pas à lui-même. La mise à jour du lien de dépendance L04 est une proposition de checkpoint distincte du corpus immuable P15.

## Carte documentaire

`01_LOTS.md` décrit couverture, ownership et allowlists. `02_VERIFICATION_AND_PREFLIGHT.md` définit les procédures courantes P15 et distingue les annexes historiques. `03_CENTRAL_PACK_IMPACT.md` consigne l’impact procédural du gel effectif. `04_OPEN_DETAILS.md` conserve les détails acceptés. `05_SOURCE_AND_RULE_INDEX.md` décrit sources et provenance. Les documents `06` à `12` conservent les contrats et preuves historiques L01–L03. `13_L04_DETAIL_CONTRACTS.md` conserve les réponses techniques Q-04/Q-06/Q-09 et leurs limites. `14_L04_SOURCE_BASELINE_AND_PREFLIGHT.md` définit l’inventaire L04, la représentation code_state et les gates restant à franchir.

## Allowlist L04 et arrêt sur changement

L’allowlist L04 reste exactement celle de l’état actif :

- `crates/whisper-adapters/src/supervisor.rs`
- `crates/whisper-adapters/tests/worker_control.rs`
- `crates/whisper-core/tests/compute_policy.rs`
- `crates/whisper-worker-cpu/src/native_engine.rs`
- `crates/whisper-worker-gpu/src/native_engine.rs`

Toute nécessité de modifier un chemin supplémentaire, notamment lib.rs, main.rs, UI/root, IPC, ports, manifeste ou configuration, constitue un CHANGE et suspend le démarrage jusqu’à sa réconciliation documentaire et son autorisation applicable. La revue PLANS ne prouve pas à elle seule que l’intégration est possible dans ces cinq chemins.

## Lignées antérieures

P10 a révisé L03 ; P11 a été une étape documentaire partielle ; P12 a restauré le corpus complet ; P13 a reçu FINDINGS sur son registre des détails. Ces mentions expliquent la provenance seulement et ne déterminent pas le statut courant.
