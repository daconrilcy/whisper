Avancée : garde revue CLEAN ; fetch, format, Clippy et 660 tests réussis, 147 ignorés, aucun test en échec.
Blocage : Structure échoue sur trois lectures LOCALAPPDATA et un registre de tests incomplet.
Décision : livré avec réserves, sans push ; correction du registre à autoriser dans un nouveau lot.

# GARDE-DEPENDANCES — 2 novembre 2026

**Livraison.** `codex/garde-dependances`, worktree `C:/wt/garde-deps`, base `e8f9094f`, plan `fadcfb9b`, SHA vérifié `f7169f96178c706d58f269ec32a2ee17ad442a1b`. Scripts PE/garde, fixtures, AGENTS et documents uniquement ; aucun Rust/manifeste/lock/workflow modifié. Revue indépendante **CLEAN**, sans correction.

**Références durables.** Neuf fixtures versionnées sous `ci/fixtures/pe/` : sans CUDA, cublas, différé CUDA, GPU cudart/nvcuda, cudart seul, nvcuda seul, deux tables tronquées, non-PE. Témoin facultatif : chemin/SHA concordants et rejet CPU exigés ; **témoin réel : NON EXÉCUTÉ**. Cache `target/cargo-home-shared/` persistant protégé. Seul `fetch --locked` accède au réseau, sans compilation/exécution ; cache absent : diagnostic fetch. Contrôles de racines/jonctions conservés.

| Contrôle | Résultat | Référence / écart |
|---|---|---|
| Auto-tests N1 | PASS, 33 positifs / 64 négatifs | sans cache ni Cargo |
| Auto-tests PE | PASS, 4 positifs / 15 négatifs | également réexécutés par le reviewer |
| Paramètres witness/fetch/cache absent | PASS | paire/SHA/rejet CPU, fetch verrouillé, refus hors contrat |
| Garde `fetch --locked` | PASS | cache initialement absent, 440 paquets téléchargés |
| Garde `fmt --check` | PASS | aucun changement |
| Garde Clippy workspace all-targets | PASS, 96 avertissements | référence 92 ; +4 expliqués ci-dessous |
| Garde `cargo test` | 622 PASS / 127 ignorés / 0 échec | référence 616 / 121 / 0 ; +6 PASS et +6 recettes natives |
| Worker CPU | 12 PASS / 9 ignorés / 0 échec | identités et statuts identiques à la fusion |
| Worker Parakeet CPU | 26 PASS / 11 ignorés / 0 échec | identités et statuts identiques à la fusion |
| Structure | ÉCHEC ; auto-tests internes PASS | 3 erreurs environnement, 1 registre, 0 retour silencieux ; référence PASS |

**Écarts.** Trois PASS `parakeet_import/worker_log_tests.rs` (`92f9a2f7`), trois PASS UI `ui/tests/compute_and_catalog.rs` (`1b76a7ea`). Six ajouts natifs/GPU `voxpopuli::*` ignorés (`aa90a4ee`), poursuite acceptée par l’utilisateur ; aucun ancien test perdu/dégradé. Noms/fichiers dans les journaux. Quatre avertissements ajoutés dans `crates/whisper-adapters/tests/parakeet_unified_native.rs` : `214:1` (8 arguments), `226:34`, `254:27`, `372:21` (emprunts inutiles), causés par l’extraction et `source: &Path` de `aa90a4ee`.

**Environnement.** Premier Clippy : code 101, toolkit CUDA vide. Reprise autorisée avec `CUDA_PATH`/`CUDA_PATH_V12_9` pour compilation seule : PASS. Tests : WHISPER/PARAKEET/CUDA/GGML nettoyés, LOCALAPPDATA initialement vide ; neuf harnais workers PE PASS. Aucun test GPU/natif/ignoré exécuté. `personal_gpu_log_creates_missing_settings_and_preserves_existing_files` : **exécuté, PASS**, sans qualification Windows complète.

**Réserve Structure.** Registre : 855 entrées pour 867 tests, douze ajouts absents. Trois tests `worker_log_tests.rs` (`92f9a2f7`) lisent LOCALAPPDATA sans défaut/exception enfant reconnue. Proposition hors périmètre : nouveau commit sur `ci/tests-baseline.txt`, après vérification de l’isolation des enfants, puis Structure. Aucun correctif appliqué.

**Clôture.** Journaux sous `target/parakeet-tests/garde-dependances/`, `journaux.zip` : 77 fichiers, nombre et SHA-256 vérifiés aussi par la revue indépendante. Binaire propre au lot `scratch/20261102T225027137-e5bef048cd9142ea9cf01b95dce0a7c2/check-structure.exe` supprimé après vérification des chemins et de son inactivité. Aucune cible Cargo ni cache isolé propre au lot créé ; caches partagés de compilation et `target/cargo-home-shared/` préservés. 618,87 Gio libres. Rapports antérieurs et main inchangés, aucun push. Rapport figé, revue finale **CLEAN sur le rapport** ; qualification N1 complète non acquise.
