Avancée : cliquet Clippy structurel intégré et vérifié dans `cliquet-clippy`.
Blocage : aucun ; N1 complet n’était pas requis et n’a pas été exécuté.
Décision : 248 alertes préexistantes acceptées ; nouvelles alertes structurelles bloquées.

# CLIQUET-CLIPPY — bilan

**Date :** 11 novembre 2026. **Worktree/branche :** `C:/wt/cliquet-clippy` / `cliquet-clippy`.
**Commit de référence :** `1e043f08cf2bb9eb7a9b35a0fea9798654e9d5d3`.
**Référence :** `ci/clippy-structure-baseline.tsv`, recréée manuellement depuis la sortie Clippy fraîche du même commit source ; la garde ne la régénère jamais automatiquement.

## Référence et rapprochement

| Lint | Clés distinctes | Compte multi-ensemble |
|---|---:|---:|
| `too_many_lines` | 192 | 192 |
| `cognitive_complexity` | 40 | 40 |
| `too_many_arguments` | 8 | 8 |
| `type_complexity` | 7 | 8 |
| **Total** | **247** | **248** |

La sortie avait 322 émissions. La déduplication multi-cibles par lint, package, chemin, offsets du span primaire et message laisse **248 sites**, identiques au bilan `CLIPPY-LOCAL`. La nouvelle signature identifie d’abord `fn <nom>` ou un item nommé ; seuls **6 clés (7 occurrences)** restent sans nom de fonction/item et utilisent la règle de type 3. **0 alerte** sans ligne ou texte de span source ; aucune signature vide ou `pub(`. La référence contient 247 clés et 248 occurrences ; le multi-ensemble préserve l’occurrence doublée.

Clés de type 3 restantes (compte par clé) :

| Compte | Clé |
|---:|---|
| 1 | `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/service/mod.rs|) -> (` |
| 1 | `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs|gates: Arc<` |
| 1 | `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs|samples_rx: Receiver<` |
| 2 | `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs|) -> (` |
| 1 | `clippy::type_complexity|whisper-adapters|crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs|controlled_capture_blocks: Option<` |
| 1 | `clippy::type_complexity|whisper-worker-parakeet-cpu|crates/whisper-worker-parakeet-cpu/src/transcription.rs|) -> Result<` |

## Implémentation et vérification

- Comparateur PowerShell ajouté à `ci/guard-level1.ps1`, seulement pour Clippy JSON incluant les quatre `-W`. Référence absente/invalide, span incomplet, alerte nouvelle et échec Clippy échouent ; aucune mise à jour de référence automatique.
- SelfTest N1 : **72 positifs / 106 négatifs — PASS**. Inclut les cas `pub(crate) fn a` nouveau face à un existant, et `pub(crate) fn a` / `pub fn a` produisant la même clé `fn a`, ainsi que les fixtures du cycle précédent.
- Clippy workspace `--all-targets --message-format=json` via la garde : **code 0**, `keys=247 sites=248 baseline_keys=247 baseline_sites=248`.
- Probe négatif temporaire de test : la garde a échoué sur `fn clippy_ratchet_probe` (`current=1, baseline=0`) ; fichier retiré et absent du diff.
- Revue générique ciblée : premier cycle a trouvé une omission des offsets dans la déduplication ; correction faite et couverte par SelfTest. Relecture de cette correction (cycle 2/2) : **CLEAN**, aucun défaut sur la question « nouvelle alerte ou échec Clippy masqué ? ».
- `AGENTS.md` corrigé : distinction du contrôle de tests existant et règle N1 + Clippy pour clore un lot Rust. `ci/check-structure.rs` est inchangé.

## Limites et livraison

Le remplacement d’une alerte par une autre portant exactement la même clé peut rester sous le plafond multi-ensemble ; les renommages, eux, bloquent. Cette limite suit l’identité et la comparaison choisies par l’utilisateur. N1 complet, tests Rust complets, tests GPU, validation Windows, merge et push : **NON EXÉCUTÉS**.

Les changements sont conservés localement sur `cliquet-clippy`. Aucun merge/push autorisé. Le worktree et ses caches sont conservés. Commandes de nettoyage proposées, non exécutées : après vérification des processus, des chemins exacts et de la rétention, `git worktree remove C:/wt/cliquet-clippy` puis `git branch -d cliquet-clippy`. Ne pas supprimer les caches persistants avant leur disposition explicite.
