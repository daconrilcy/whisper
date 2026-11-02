Avancée : quatre lots fusionnés localement dans main ; revue indépendante CLEAN et niveau 1 PASS.
Blocage : aucun pour la fusion ; qualifications résiduelles précisées ci-dessous.
Décision : livraison locale, sans push ; nettoyage proposé uniquement, soumis à un accord distinct.

Le 2 novembre 2026 : tag `avant-fusion-pile` = `248f8679` ; fusion unique `c830881c`, parents `248f8679`/`85dcd7fc`, arbre identique à la tête revue.

Livré : [TESTS-AUTONOMES](TESTS-AUTONOMIE-INVENTAIRE.md), tests autonomes, feature `native-tests` et contrôle de structure ; [GARDE-N1](GARDE-N1.md), catégories Cargo ; [R8-FIX](R8-FIX.md), compilation séparée, contrôle PE et livraison ; [GARDE-NVML](../cadrage/GARDE-NVML.md), `ProgramW6432` système validé pour les profils NVIDIA.

Préconditions : 93 commits linéaires (18/5/60/10), main initial exact, deux relevés sans activité avant fusion, autres agents terminés, suivis propres ; non suivis main/R8 conservés. Revue indépendante : zéro constat. **121 fichiers : 64 tests, 8 manifestes, 2 extraction GPU, 7 CI, 1 livraison, 1 AGENTS, 38 documents/preuves**. Dérogations : extraction équivalente, harnais CPU/GPU, sept ZIP avec dix WAV/une fixture MP3. Lock/dépendances inchangés.

Sur main, via `ci/guard-level1.ps1`, offline/locked lorsque applicable, environnement isolé, CUDA/GGML retirées pour les tests :

| Commande | Résultat main | Comparaison tête de pile |
|---|---|---|
| `cargo fmt --check` | PASS | identique |
| `cargo clippy --workspace --all-targets` | 0 erreur ; 92 avertissements cumulés sur 23 résumés | 92, identique |
| `cargo test` | 616 PASS / 121 ignorés / 0 échec | identique |
| `cargo test -p whisper-worker-cpu` | 12 PASS / 9 ignorés / 0 échec | identique |
| `cargo test -p whisper-worker-parakeet-cpu` | 26 PASS / 11 ignorés / 0 échec | identique |
| `ci/check-structure.rs` via mode Structure | PASS ; 855 entrées, 159 gates natives, 0 retour silencieux | identique |

Auto-tests : 30 positifs/58 négatifs PASS. Prérequis `--no-run`/PE : neuf harnais CPU PASS, zéro import CUDA, SHA consignés. Total : **654 PASS, 141 ignorés, zéro échec** ; aucun natif/GPU exécuté. Journaux : `target/parakeet-tests/fusion-pile/scratch/` ; référence Clippy : `.wt/nv/target/parakeet-tests/fusion-pile-head-v2/scratch/`.

Qualification existante : **43 corps ciblés PASS** (31 intermédiaire ; 23 contrats GPU, trois Parakeet hors inventaire initial), repli CPU démontré, worker CPU release sans imports CUDA (PE/SHA), `PersonalUnifiedGpuOnly` PASS. Non qualifiés : campagne native complète, FR87/FR27 perdus, `ParakeetGpu DISCOVERY_PENDING`, généralisation de l'unique inférence GPU, Windows isolé sans CUDA. `Whisper/settings` est préparé par la garde : création par le produit à traiter séparément.

Nettoyage **non exécuté**, depuis `C:/dev/whisper` ; Gio logiques hors jonctions :

| Commande proposée | Gio | État |
|---|---:|---|
| `git worktree remove target/agent-worktrees/tests-autonomes` | 0,981 | propre |
| `git worktree remove target/agent-worktrees/garde-n1-categories` | 4,591 | propre |
| `git worktree remove .wt/r8` | 40,246 | modèles/audio non suivis à préserver ; retrait refusé en l'état |
| `git worktree remove .wt/nv` | 10,908 | propre ; préserver les journaux de référence |

Potentiel brut **56,726 Gio**, avant conservation des entrées/preuves. Après retrait autorisé : `git branch -d refacto/tests-autonomes garde/n1-categories fix/r8-worker-cpu-sans-cuda codex/garde-nvml` ; branches toutes fusionnées.

Cibles incluses, sous `.wt/r8/target/` : `r8-fix/targets` 5,485 ; `r8-install/targets/desktop-personal` 0,928 ; ancien `parakeet-tests/cargo-shared` 0,014. Caches/sorties N2 : `n2-native` 22,090 ; `.wt/nv/target/n2-native` 9,642. Conserver `target/parakeet-tests/cargo-shared` canonique (31,974 Gio) et `target/parakeet-tests/fusion-pile` (0,737 Gio).

Conserver [raw/](raw/), bilans et sept couples ZIP/JSON (`garde-nvml`, `n2-native`, `r8-fix`, `r8-install`) : leurs `archive.sha256` correspondent aux empreintes recalculées. Espace libre final **76,4 Gio** (>30). Aucune suppression/opération distante.
