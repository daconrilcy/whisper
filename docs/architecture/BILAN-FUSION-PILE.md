Avancée : quatre lots fusionnés ; niveau 1 PASS ; Turbo et Parakeet validés manuellement en Auto et CPU sur un second PC sans CUDA déclaré par l'utilisateur.
Blocage : aucun pour les essais CPU rapportés ; message de détection GPU au démarrage toujours présent.
Décision : qualification CPU acceptée pour cette recette ; limites GPU et corpus conservées ; aucun push.

Le 2 novembre 2026 : tag `avant-fusion-pile` = `248f8679` ; fusion unique `c830881c`, parents `248f8679`/`85dcd7fc`, arbre identique à la tête revue.

Livré : [TESTS-AUTONOMES](TESTS-AUTONOMIE-INVENTAIRE.md), tests autonomes, feature `native-tests` et contrôle de structure ; [GARDE-N1](GARDE-N1.md), catégories Cargo ; [R8-FIX](R8-FIX.md), compilation séparée, contrôle PE et livraison ; [GARDE-NVML](../cadrage/GARDE-NVML.md), `ProgramW6432` système validé pour les profils NVIDIA.

Préconditions : 93 commits linéaires (18/5/60/10), main initial exact, deux relevés sans activité avant fusion, autres agents terminés, suivis propres ; non suivis main/R8 conservés. Revue indépendante : zéro constat. **121 fichiers : 64 tests, 8 manifestes, 2 extraction GPU, 7 CI, 1 livraison, 1 AGENTS, 38 documents/preuves**. Dérogations : extraction équivalente, harnais CPU/GPU, sept ZIP avec dix WAV/une fixture MP3. Lock/dépendances inchangés.

Décision documentaire du 2 novembre 2026 : la mention « deux relevés sans activité avant fusion » ci-dessus est un constat historique, pas une exigence d'absence globale de Git. Pour FUSION-PILE, les restrictions globales « Aucune activité Git » et « Toute activité inconnue est bloquante » sont remplacées par la vérification des activités concurrentes pertinentes pour l'opération, selon la politique de décision des agents d'[AGENTS.md](../../AGENTS.md). Les prochains lots appliquent cette politique ; les protections contre les conflits établis et les contrôles d'intégrité restent applicables.

Sur main, via `ci/guard-level1.ps1`, offline/locked lorsque applicable, environnement isolé, CUDA/GGML retirées pour les tests :

| Commande | Résultat main | Comparaison tête de pile |
|---|---|---|
| `cargo fmt --check` | PASS | identique |
| `cargo clippy --workspace --all-targets` | 0 erreur ; 92 avertissements cumulés sur 23 résumés | 92, identique |
| `cargo test` | 616 PASS / 121 ignorés / 0 échec | identique |
| `cargo test -p whisper-worker-cpu` | 12 PASS / 9 ignorés / 0 échec | identique |
| `cargo test -p whisper-worker-parakeet-cpu` | 26 PASS / 11 ignorés / 0 échec | identique |
| `ci/check-structure.rs` via mode Structure | PASS ; 855 entrées, 159 gates natives, 0 retour silencieux | identique |

Auto-tests : 30 positifs/58 négatifs PASS. Prérequis `--no-run`/PE : neuf harnais CPU PASS, zéro import CUDA, SHA consignés. Total : **654 PASS, 141 ignorés, zéro échec** ; aucun natif/GPU exécuté. Journaux : `target/parakeet-tests/fusion-pile/scratch/` ; référence Clippy déplacée sous `target/pile-preserved/nv/target/parakeet-tests/fusion-pile-head-v2/scratch/`, SHA inchangé.

Qualification existante : **43 corps ciblés PASS** (31 intermédiaire ; 23 contrats GPU, trois Parakeet hors inventaire initial), repli CPU démontré, worker CPU release sans imports CUDA (PE/SHA), `PersonalUnifiedGpuOnly` PASS. Non qualifiés : campagne native complète, FR87/FR27 perdus, `ParakeetGpu DISCOVERY_PENDING`, généralisation de l'unique inférence GPU. Windows Sandbox non exécuté ; la recette réelle sur le second PC ci-dessous apporte la validation CPU sans CUDA rapportée par l'utilisateur. `Whisper/settings` est préparé par la garde et le lanceur portable : création par le produit à traiter séparément.

Validation utilisateur du 2 novembre 2026 : le paquet portable `target/remise-cpu-sans-cuda-20261102/Whisper-CPU-Windows-x64.zip` (SHA-256 `5E65044E9FB1B1437B111162DC125AC1C5E5F63C49E7E7B46033A19BD3D18DE0`, 28 fichiers vérifiés avant remise, aucune DLL CUDA embarquée) démarre sur l'autre PC Windows déclaré sans CUDA. **Turbo CPU, Turbo Auto, Parakeet CPU et Parakeet Auto : PASS manuel**, moteur CPU sans erreur et transcriptions jugées bonnes. Le choix GPU est grisé. Le démarrage affiche néanmoins `WorkerExited without End`, un inventaire partiel et les adaptateurs Parsec Virtual Display Adapter / Intel Iris Xe indéterminés ; ce diagnostic reste une réserve d'affichage/détection GPU, sans échec CPU rapporté. Aucun inventaire matériel ou DLL supplémentaire demandé ni exécuté ; il ne s'agit pas d'une qualification GPU. Exports TXT/SRT/WAV, capture micro et mesures de performance non explicitement attestés par ce retour. Les résultats automatisés et leurs comptes restent inchangés.

Nettoyage autorisé et **exécuté** le 2 novembre 2026 depuis `C:/dev/whisper`, sans force ; tailles initiales hors jonctions :

| Commande exécutée | Gio initiaux | État final |
|---|---:|---|
| `git worktree remove target/agent-worktrees/tests-autonomes` | 0,981 | retiré |
| `git worktree remove target/agent-worktrees/garde-n1-categories` | 4,591 | retiré |
| `git worktree remove .wt/r8` | 40,246 | retiré après préservation |
| `git worktree remove .wt/nv` | 10,908 | retiré après préservation |

Branches fusionnées supprimées par `git branch -d` : `refacto/tests-autonomes`, `garde/n1-categories`, `fix/r8-worker-cpu-sans-cuda`, `codex/garde-nvml`. Code de main, tag de sécurité et trois autres worktrees inchangés.

Les dossiers `target` ont été déplacés sous `target/pile-preserved/<lot>/target/` (`r8`, `nv`, `tests-autonomes`, `garde-n1-categories`). Modèles/audio R8 : `target/pile-preserved/r8/models/` et `sound example/` ; **deux modèles et 303 fichiers, empreintes intégrales identiques à la référence avant nettoyage**, revérifiées après suppression manuelle. Les 305 fichiers correspondent aussi, par SHA-256 individuel, aux originaux non suivis de main.

Cibles conservées sous l'archive R8 : `target/r8-fix/targets` 5,485 Gio, `target/r8-install/targets/desktop-personal` 0,928. Cache canonique `target/parakeet-tests/cargo-shared` : inventaire inchangé (**34 114 fichiers, 34 331 489 142 octets, soit 31,974 Gio**), neuf harnais SHA-256 identiques. Les deux `cargo-shared` archivés conservent également leurs inventaires/tailles (4 174 fichiers/3 926 136 114 octets et un fichier/15 178 752 octets). Résultats N2 et racine Fusion main conservés.

[raw/](raw/) et bilans conservés : **12 ZIP recontrôlés, SHA inchangés**, dont sept couples ZIP/JSON de la pile ; journal Clippy de référence inchangé par SHA-256. La purge automatique avait été refusée avant exécution (« blocked by policy »). L'utilisateur a ensuite supprimé manuellement les **dix seuls `cargo-home` listés**, sous `target/pile-preserved/` : **10/10 absents**, vérifié le 2 novembre 2026 à 10:21 UTC. Leur volume antérieur était **6,862 Gio** ; les intermédiaires compilés restent archivés. Espace libre observé **84,24 Gio**. Aucune exécution Cargo/native ni opération distante pour ce contrôle.
