Avancée : les 20 entrées `dead_code` du bilan CLIPPY-LOCAL ont été rapprochées des sources et harness actuels.
Blocage : aucune vérification Cargo n’a été exécutée ; les variantes compilées hors des traces archivées restent à confirmer par validation.
Décision : conserver le code produit et les supports nécessaires ; retirer du seul harness CPU l’inclusion de cache inutilisée et son stub de constructeur.

# Whisper — décision sur les entrées `dead_code` de CLIPPY-LOCAL

## Besoin et décision

Décider du traitement des 20 entrées `dead_code` du [bilan CLIPPY-LOCAL](../architecture/CLIPPY-LOCAL-BILAN.md), sans masquer une perte de fonctionnalité. Le tableau du bilan contient 20 lignes, certaines regroupant plusieurs symboles.

**Décision proposée :** conserver les huit éléments de traçage conditionnel, le helper de fixture PE, les deux décodeurs de bloc CPU/GPU, les caches et la qualification GPU, ainsi que les stubs nécessaires aux harness GPU. Dans le harness CPU seulement, ne plus inclure `engine_cache.rs` par `#[path]` et supprimer le stub `CpuEngine::load` qui n’est référencé que par ce cache inclus. La fonction de cache et le vrai `CpuEngine::load` de production restent inchangés. Cette correction locale des deux entrées CPU réduirait les diagnostics de harness sans annoter le code de production avec `allow(dead_code)`.

## Constat et limites

Les preuves statiques ci-dessous rapprochent le bilan, les sources actuelles et les cibles de tests mentionnées dans l’archive CLIPPY-LOCAL. Le bilan et les journaux archivés sont rattachés à `27da87b8`; le HEAD lu pour ce cadrage est `ac37bd11`. Les sources ciblées ont été comparées entre ces révisions et aucun changement les affectant n’a été relevé. Cela ne prouve pas qu’une campagne a été exécutée sur `ac37bd11`. Aucun Cargo, test, build ou Clippy n’a été lancé pour ce cadrage.

Les avertissements d’une compilation de cible de test ne démontrent pas qu’un symbole est inutile en production. Les harness `#[path]` compilent des modules avec des dépendances factices et n’exercent qu’une partie de ces modules. L’état exact de toutes les combinaisons de features ne peut être inféré d’une seule sortie Clippy.

## Décisions par entrée

| # | Entrée du bilan | Preuve d’usage ou raison | Décision |
|---:|---|---|---|
| 1 | `native_timing::TRACE` | État du writer singleton, initialisé par `trace_lock`. Les trois tests unitaires couvrent le chemin, l’écriture JSONL et l’activation, mais pas directement `TRACE`. | Garder ; outil de diagnostic conditionnel. |
| 2 | `native_timing::TraceWriter` | Porte le writer et l’origine temporelle dans `open_trace` et `record`. | Garder ; composant du traçage activé par `native-tests`. |
| 3 | `native_timing::Phase` | `phase` retourne le garde et son `Drop` écrit la durée de fin. | Garder ; mesure de phase conditionnelle. |
| 4 | `native_timing::phase` | Appelé dans `worker_ipc/runtime/import.rs`; `model_catalog.rs` appelle `record`. | Garder ; instrumentation conditionnelle du travail produit. |
| 5 | `native_timing::record` | Appelé par catalogue/import ; écrit uniquement lorsque la trace est activée. | Garder ; instrumentation conditionnelle. |
| 6 | `native_timing::enabled` | Garde `phase` et `record`, basée sur `WHISPER_DIAG_BANC_TRACE`. | Garder ; protège l’activation opt-in. |
| 7 | `native_timing::trace_lock` | Fournit le verrou autour du writer singleton. | Garder ; synchronisation du traçage. |
| 8 | `native_timing::open_trace` | Ouvre un fichier dans la racine de campagne et vérifie le chemin canonique. | Garder ; sortie du traçage conditionnel. |
| 9 | `compile_failing_worker` (fixture PE adapters) | Appelé par les tests unitaires `legacy_runtime_tests`, `worker_log_tests` et `reprocessing_and_transport`. Deux cibles d’intégration incluent le fichier partagé sans appeler ce helper. | Garder le helper. Ne pas le supprimer à cause des deux inclusions partielles ; une extraction propre au test reste optionnelle et n’est pas justifiée par le produit. |
| 10 | `decode_requested_block` CPU | Le décodeur produit appelle `whisper_worker_common`; la session de décodage CPU l’appelle. Le harness inclut le module pour tester d’autres fonctions de décodage. | Garder ; diagnostic de compilation du harness partiel. |
| 11 | `cached_cpu_engine` | `crates/whisper-worker-cpu/src/engine_cache.rs`; appelé par les sessions CPU d’import et de session. Le fichier n’a pas de tests propres ; le harness ne consomme pas ce module, son faux `run_session` ne gardant que le type dans sa signature. Diagnostic dans le test d’intégration utilisant le harness autonome. | Garder la fonction produit ; retirer uniquement l’inclusion `#[path]` de `engine_cache.rs` dans le harness. |
| 12 | `CpuEngine::load` (stub du harness CPU) | Le constructeur réel n’est pas inclus : le harness définit un type factice. Après retrait de l’inclusion du cache, ce stub n’a plus de consommateur ; `worker.rs` exige toujours le type factice pour compiler. | Retirer le seul stub de méthode `load`, conserver le type factice et le chargement natif en production. |
| 13 | `decode_requested_block` GPU | Le décodeur produit est appelé depuis la session GPU ; le harness inclut le module pour tester les fonctions de décodage pures. | Garder ; diagnostic de compilation du harness partiel. |
| 14 | Champs GPU `model_path`, `model_sha256`, `candidate`, `engine` | Le cache GPU compare l’identité du modèle et du périphérique et conserve le moteur réutilisable. | Garder ; invariants d’identité et de réutilisation. |
| 15 | `cached_gpu_engine` | Appelé par les sessions GPU. Le harness a besoin du type `CachedGpuEngine` pour le dispatch et inclut les tests de helpers du module, mais ces tests n’appellent pas directement `cached_gpu_engine`. | Garder ; usage produit avéré. |
| 16 | `qualify` | Appelé par `gpu_commands`; le harness inclut la qualification pour tester `qualify_one`. | Garder ; qualification GPU produit/testée. |
| 17 | `GpuEngine::{load, attest, matches_device}` (stubs harness) | Les signatures sont requises par le cache GPU compilé dans le harness ; les stubs empêchent l’accès au moteur natif. | Garder tant que le cache reste compilé dans ce harness ; appels interdits par la frontière native. |
| 18 | `cuda_devices` (stub harness) | Dépendance de `qualify` incluse dans le harness. | Garder ; stub sans accès CUDA réel. |
| 19 | `ggml_devices` (stub harness) | Dépendance de `qualify` incluse dans le harness. | Garder ; stub de frontière native. |
| 20 | `whisper_gpu_device_index` (stub harness) | Le cache/attestation GPU compilé dans le harness exige cette fonction. | Garder ; stub de frontière native. |

Les 20 lignes de ce tableau ne correspondent pas à 20 fonctions distinctes : la ligne 14 regroupe quatre champs, la ligne 17 trois méthodes. Les huit alertes `native_timing` proviennent de la bibliothèque compilée pour les tests sans `native-tests`. Les appels applicatifs à `phase` et `record` sont conditionnés par cette feature. Les entrées des workers CPU/GPU proviennent des harness d’intégration `tests/support/autonomous_harness.rs`, qui compilent des sources via `#[path]` et des frontières natives factices. Les diagnostics reflètent ces cibles de test, pas une démonstration que les chemins produit correspondants sont morts.

**Traçabilité des cibles et fichiers pour CA-01 :** entrées 1–8 : `crates/whisper-adapters/src/native_timing.rs`, compilé dans la cible de test de bibliothèque (`cfg(test)`), avec les appels applicatifs à `phase`/`record` sous `native-tests`; tests unitaires dans `src/native_timing/tests.rs` testant activation, chemin et sérialisation, sans appel direct au verrou. Entrée 9 : `crates/whisper-adapters/tests/support/pe_fixture.rs`, chargée par les tests unitaires cités au tableau et par deux cibles d’intégration (`worker_control`, `parakeet_runtime_isolation`) qui n’appellent pas ce helper. Entrées 10 et 13 : décodeurs `crates/whisper-worker-cpu/src/decoder.rs` et `crates/whisper-worker-gpu/src/decoder.rs`, appelés en production par `session/decode.rs`; compilés aussi depuis les cibles d’intégration qui incluent le harness autonome pour tester d’autres fonctions. Entrées 11–12 : `crates/whisper-worker-cpu/src/engine_cache.rs` et son stub dans `crates/whisper-worker-cpu/tests/support/autonomous_harness.rs`, diagnostic du test d’intégration du harness CPU. Entrées 14–20 : `crates/whisper-worker-gpu/src/engine_cache.rs`, `gpu_qualification.rs` et stubs de `crates/whisper-worker-gpu/tests/support/autonomous_harness.rs`, compilés par la cible d’intégration du harness GPU. La sortie Clippy archivée contient **21 émissions pour 20 sites** ; l’entrée 9 donne deux émissions, une dans chacune des deux cibles d’intégration nommées. Ces attributions sont celles des journaux archivés rattachés à `27da87b8`, pas une nouvelle exécution sur le HEAD lu `ac37bd11`.

Le bilan signale par ailleurs le retrait déjà accepté de `verify_windows_identity` dans le harness GPU ; ce cadrage ne propose pas de nouvelle suppression de fonction produit ni d’annotation `allow(dead_code)`. Ce changement antérieur n’est pas réévalué ici.

## Critères et plan futur

- **CA-01 — Inventaire :** les 20 entrées sont individualisées et les symboles regroupés sont explicités dans le tableau ci-dessus.
- **CA-02 — Usage :** chaque recommandation cite un usage produit, un besoin du harness, ou la limite de preuve connue.
- **CA-03 — Aucun masquage :** aucune annotation générale ou changement de configuration de lint n’est recommandé ; l’ajustement CPU porte uniquement sur des inclusions de harness.
- **CA-04 — Vérifiabilité :** le plan ci-dessous relie chaque modification au résultat de diagnostic attendu.

Plan d’implémentation proposé, non autorisé par ce cadrage :

1. **[CA-01, CA-03]** Dans `crates/whisper-worker-cpu/tests/support/autonomous_harness.rs`, retirer l’inclusion du cache produit et le stub `CpuEngine::load` devenu sans appel ; garder le type factice et les modules nécessaires aux autres tests.
2. **[CA-02, CA-04]** Après autorisation explicite d’implémenter, relever avant toute édition l’inventaire d’identités et les diagnostics Clippy du harness CPU, puis comparer après édition (zéro différence d’identités attendue). Comparer les diagnostics à matrice/options et clé de déduplication identiques ; ensuite exécuter la validation N1 et Clippy prévues par les règles du dépôt. Confirmer la disparition des deux sites attendus et l’absence de nouveaux diagnostics ou de changement des autres entrées.

## Autorisations et suites

Ce cadrage n’autorise pas l’implémentation ni l’exécution de Cargo. Le plan ne devient exécutable qu’après accord précis dans une nouvelle mission et validation de son périmètre contre le plan applicable. N1 et Clippy seront alors soumis à la garde permanente et au préflight des campagnes. La validation manuelle Windows n’est pas pertinente pour ce changement de harness et n’a pas été exécutée.
