Avancée : les cinq scénarios R8 passent sur la clé Test R6 actuelle et la suite workspace est verte après le second cycle.
Blocage : aucun constat de revue ne reste ouvert ; le code R8 et le candidat R7 antérieur restent non commités.
Décision : le reviewer indépendant rend CLEAN pour la reprise R8 sur la clé Test actuelle ; cela ne requalifie pas la clé historique `545a…`.

# Bilan — reprise du R8 historique

Base : branche `wip/l2b-candidate`, HEAD avant candidat `4fbef019`. Le candidat Rust modifie seulement `crates/whisper-adapters/tests/parakeet_import.rs` ; les dix fichiers R7 déjà modifiés restent non commités. Le worker produit lu est `1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0`, le modèle `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e` et les cinq DLL correspondent aux SHA du préflight R8. Les commandes sont hors réseau, avec Cargo partagé et sorties sous `target/parakeet-tests/l2/l2b/r8/`.

| Scénario | Résultat | Preuve principale |
| --- | --- | --- |
| DLL incompatible | PASS | `QualificationMismatch` à la vérification directe ; `ModelNotQualified` dans le parcours produit avant pending ; chargeur natif échoue avant Ready ; aucun TXT/SRT. |
| Worker tué après demande | PASS | `WorkerExited`, aucun résultat. |
| Trame Result incomplète | PASS | `IpcDecode` avec champ manquant. |
| Mort via le parcours produit | PASS | pending et source conservés, aucun Complete, aucun repli Turbo. |
| Trame incomplète via le parcours produit | PASS | pending et source conservés, aucun Complete, aucun repli Turbo. |

La garde a passé le cas valide (exit 0) et refusé `CARGO_TARGET_DIR=C:\dev\whisper\target` (exit 1) avant Cargo. Les sorties sont conservées dans `target/parakeet-tests/l2/l2b/r8/logs/guard-ok.log` et `guard-ko.log`. Les journaux de chaque cas et des deux suites workspace sont dans ce même répertoire.

`cargo test --offline --locked --workspace --no-fail-fast` a été lancé deux fois, avec et sans le flag de compilation des injections : **343 PASS, 22 FAIL, 29 ignorés** à chaque fois. Les échecs sont : deux tests unitaires `worker_ipc`, trois `legacy_model_requests`, un `model_catalog`, six `model_preferences` (racine `WHISPER_L1B_RUNTIME_ROOT` absente) ; huit tests `parakeet_archive` (helper n'admet pas R8) et un cas enfant R4 (audio long non fourni). Aucun de ces échecs ne vient d'une assertion des cinq cas R8. La suite workspace avant ce candidat était verte sous la garde R7, mais cette preuve ne remplace pas le gate R8.

Le premier cycle de revue indépendante ne relève pas de défaut R8 attribuable et rend `BLOCKED` pour le gate. L'utilisateur a alors autorisé le second cycle limité au helper `archive_root()` dans `crates/whisper-adapters/tests/parakeet_archive.rs` et aux variables de campagne manquantes. Les refus hors racine et les chemins R6/R7 y sont préservés. Le test ciblé d'archive passe (9 réussis, zéro échec, 4 ignorés).

Après cette correction, `cargo test --offline --locked --workspace --no-fail-fast` passe sous la garde R8 et le Cargo partagé : **364 réussis, zéro échec, 29 ignorés**. Le second et dernier cycle du même reviewer indépendant rend **CLEAN**, sans constat ni écart de vérification restant. Les 29 tests ignorés par la suite ne comptent pas comme preuve ; les cinq cas R8 ont été lancés individuellement. La garde OK/KO et ses journaux, les cinq journaux de cas, le gate et le script ont été copiés sous `C:\dev\whisper-evidence\l2b-r8\` : 20 fichiers, nombre et SHA-256 vérifiés. L'espace libre après la campagne est de 148 290 940 928 octets (environ 138,1 Gio). Aucun code produit, aucune fiche Test, aucun modèle, aucune DLL de référence ou fichier de `sound example/` n'a été modifié. Aucun commit de code ni push n'a été fait.
