Avancée : quatre lots fusionnés ; revue CLEAN, niveau 1 PASS ; quatre worktrees et branches retirés.
Blocage : purge complémentaire des caches/intermédiaires refusée automatiquement ; fichiers conservés.
Décision : preuves, modèles, audios et cache partagé préservés ; aucun push.

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

Qualification existante : **43 corps ciblés PASS** (31 intermédiaire ; 23 contrats GPU, trois Parakeet hors inventaire initial), repli CPU démontré, worker CPU release sans imports CUDA (PE/SHA), `PersonalUnifiedGpuOnly` PASS. Non qualifiés : campagne native complète, FR87/FR27 perdus, `ParakeetGpu DISCOVERY_PENDING`, généralisation de l'unique inférence GPU, Windows isolé sans CUDA. `Whisper/settings` est préparé par la garde : création par le produit à traiter séparément.

Nettoyage autorisé et **exécuté** le 2 novembre 2026 depuis `C:/dev/whisper`, sans force ; tailles initiales hors jonctions :

| Commande exécutée | Gio initiaux | État final |
|---|---:|---|
| `git worktree remove target/agent-worktrees/tests-autonomes` | 0,981 | retiré |
| `git worktree remove target/agent-worktrees/garde-n1-categories` | 4,591 | retiré |
| `git worktree remove .wt/r8` | 40,246 | retiré après préservation |
| `git worktree remove .wt/nv` | 10,908 | retiré après préservation |

Branches fusionnées supprimées par `git branch -d` : `refacto/tests-autonomes`, `garde/n1-categories`, `fix/r8-worker-cpu-sans-cuda`, `codex/garde-nvml`. Code de main, tag de sécurité et trois autres worktrees inchangés.

Les dossiers `target` complets ont été déplacés sous `target/pile-preserved/<lot>/target/` (`r8`, `nv`, `tests-autonomes`, `garde-n1-categories`). Modèles/audio R8 : `target/pile-preserved/r8/models/` et `sound example/` ; **deux modèles et 303 fichiers, empreintes intégrales identiques**. Les originaux non suivis de main restent en place.

Cibles conservées sous l'archive R8 : `target/r8-fix/targets` 5,485 Gio, `target/r8-install/targets/desktop-personal` 0,928. Résultats N2 complets conservés. Cache canonique `target/parakeet-tests/cargo-shared` inchangé : **34 114 fichiers, 31,974 Gio** ; racine Fusion main (0,737 Gio) conservée.

[raw/](raw/) et bilans conservés : **12 ZIP recontrôlés, SHA inchangés**, dont sept couples ZIP/JSON de la pile. La purge supplémentaire des caches/intermédiaires archivés a été refusée avant exécution (« blocked by policy », sans motif détaillé) ; aucun de ces fichiers supprimé. Gain au premier relevé **0,57 Gio**, inférieur aux 56,726 Gio bruts puisque les données ont été conservées ; dernier espace libre **77,1 Gio**. Aucune opération distante.
