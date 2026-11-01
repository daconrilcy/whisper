Avancée : plan GARDE-N1-CATEGORIES accepté dans la session du 1er novembre 2026 ; implémentation déléguée.
Blocage : aucun identifié avant implémentation ; preuves à produire.
Décision : réaliser le périmètre ci-dessous sur garde/n1-categories depuis refacto/tests-autonomes (939fbbbe).

# GARDE-N1 — catégories de commandes Cargo

Ce lot rend possibles les diagnostics des workers locaux, notamment les résolutions et compilations froides bloquées de R8, sans modifier le produit ni affaiblir les contrôles de niveau 1.

## Plan accepté et critères

1. Étendre ci/guard-level1.ps1 et ses auto-tests/fixtures par une liste d’autorisation d’arguments ; conserver les contrôles de chemins, liens, variables isolées, espace libre, capture stdout/stderr et codes de sortie, hors ligne/verrouillé. Aucune surveillance de fichiers.
2. Autoriser les catégories et paramètres ci-dessous ; refuser par défaut tout argument inconnu et tous les refus explicites de la demande.
3. Vérifier les auto-tests positifs/négatifs, les commandes R8 en DryRun, un tree réel et les tests workspace ; consigner les résultats réels et leurs limites.
4. Remplacer uniquement la description N1 d’AGENTS.md ; terminer ce rapport (deux pages maximum), journaux sous raw/garde-n1/ ; revue indépendante, deux cycles de correction maximum, commits locaux sans push.

| Catégorie | Commandes | Conditions |
| --- | --- | --- |
| Lecture | tree, metadata, fmt --check | aucune écriture hors racines autorisées |
| Compilation | check, build, test --no-run, clippy | --workspace ou un/plusieurs -p membres, --release permis |
| Tests | test | sans --ignored, --include-ignored, --features native-tests |

Cible partagée existante ou répertoire neuf sous la racine du lot via -FreshTarget <nom>. Les membres -p proviennent de cargo metadata. -UnsetEnv permet seulement CUDA_PATH, CUDA_PATH_V*, GGML_*, WHISPER_*, PARAKEET_*.

Refus : run, install, uninstall, update, publish, clean, add, remove, fix, generate-lockfile, vendor, commandes externes non listées ; --fix, --allow-dirty, --all-features, native-tests, recette-hooks avec --release, --ignored, --include-ignored, --config, -Z, --target-dir, --manifest-path hors worktree, tout argument inconnu.

## Preuves à produire

1. Auto-tests : au moins un positif par catégorie, tree CPU features inverse sys, build CPU cible neuve, test CPU no-run, tests workspace ; négatifs de tous les refus, package inexistant, cible hors racine/jonction, UnsetEnv hors liste et argument inconnu, sans Cargo pour les rejets.
2. DryRun des six commandes bloquées de raw/r8/execution-compatibility.json et cibles A/B/D du plan R8 (sources en lecture sur diag/r8-cuda-cpu, commit 982592ba).
3. cargo tree --offline --locked -p whisper-worker-cpu -e features -i whisper-rs-sys ; sortie conservée.
4. cargo test --workspace via la garde ; référence attendue 687 réussis / 0 échec / 147 ignorés.

État initial : worktree dédié propre ; environ 171 Gio libres. Les fichiers non suivis models/ et sound example/ du checkout principal sont préservés. Non vérifié à ce stade : toutes les preuves ci-dessus ; aucune recette native ou fenêtre Windows prévue.