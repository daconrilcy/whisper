Avancée : plan borné préparé pour implémenter la matrice de garde acceptée.
Blocage : l’accord actuel accepte la politique, pas l’édition PowerShell ni les quatre probes `cargo metadata` du SelfTest.
Décision : soumettre ce plan à la revue et demander ensuite l’accord d’exécution précis avant toute modification ou commande.

# Plan — politique de garde N1 pour la reprise d’import

**Référence design :** [cadrage-politique-garde-n1-reprise-import.md](cadrage-politique-garde-n1-reprise-import.md), accepté le 12 novembre 2026; DESIGN CLEAN au commit `d9be2e3a`.
**Base d’exécution exigée :** `main@6193fbdf` ou descendant conservant le durcissement de sélection Cargo.
**Périmètre proposé :** `ci/guard-level1.ps1` seulement, incluant ses SelfTests intégrés. Aucun fichier produit, manifeste, verrou ou autre garde.
**État :** DRAFT à revue indépendante. L’acceptation de la matrice n’autorise pas l’édition du script ni les appels de validation décrits ici.

## Lots et tâches

Un seul lot, cinq tâches, séquentielles.

| Tâche | Travail | Critère de sortie / arrêt |
|---|---|---|
| **G0 — Préflight et accord** | Dans un worktree dédié, relever l’état Git, HEAD, l’ascendance de `6193fbdf`, les chemins existants, l’espace libre et les racines de lot/cache/cible. Confirmer le responsable de l’édition PowerShell, l’accord précis sur `ci/guard-level1.ps1`, le SelfTest complet et ses probes metadata, ainsi que l’emplacement d’archive. | Aucun changement avant l’accord et le préflight; arrêter sur base antérieure à `6193fbdf`, worktree sale, racine ambiguë, lien non prévu ou espace libre sous 30 Gio. Worktree candidat observé : `C:/wt/guard-n1-paquets`, branche `codex/impl-garde-n1-paquets`, HEAD `6193fbdf`; à revalider au G0. |
| **G1 — Admission commune** | Ajouter la politique exacte de `target/parakeet-tests/refacto-reprise-import` et faire passer les chemins `Run` concernés, notamment `fmt` et `fetch`, par un point d’admission commun avant tout appel Cargo. Garder la politique historique et la validation des membres après admission. | Témoins positifs pour `fmt --check` et le Clippy exact; `fetch --locked` rejeté avant Cargo. Les commandes sans sélection et sélections des autres racines gardent le comportement historique. |
| **G2 — Matrice du lot** | Admettre exactement les 20 profils du cadrage : format (1), N1 par défaut (1), filtres ciblés (8), commandes workers (9), Clippy (1). Réutiliser le parseur et les règles de normalisation existants; traiter `-p nom` et `-pnom` selon les formes déjà prises en charge. | Toute commande hors matrice ou toute sélection absente, dupliquée, supplémentaire ou croisée est refusée à la racine du lot avant Cargo. `--workspace` demeure refusé globalement. Pas de filtre GPU, même pour `--list`. |
| **G3 — Auto-tests et vérification** | Étendre le SelfTest intégré avec vecteurs positifs/négatifs indépendants de la table de politique, deux interfaces d’arguments, racines de fixtures, branche historique, options de contournement et compteurs d’invocation. Vérifier statiquement chaque branche `Run`; exécuter l’analyse syntaxique PowerShell et `git diff --check`. | Chaque refus cible prouve zéro rappel de `metadata` et zéro nouveau processus Cargo; chaque admission atteint le rappel simulé attendu. Les appels `metadata` du SelfTest complet sont rapportés séparément et ne sont lancés qu’avec l’accord G0. Aucun Clippy/test/build Cargo du produit dans ce lot. |
| **G4 — Revue et clôture** | Faire relire le diff par un reviewer indépendant, deux cycles maximum. Attribuer le résultat au commit exact; préserver et archiver la preuve du SelfTest selon les règles du dépôt. | Verdict CLEAN ou LIVRÉ AVEC RÉSERVES; preuve, nombre de fichiers, empreintes et limites consignés. La garde livrée rend possible un nouveau préflight T0 de `recovery.rs`, sans déclarer celui-ci satisfait. |

## Contrat des profils G2

Les profils s’appliquent seulement à la racine canonique exacte du nouveau lot. Les options de la garde (`-Mode`, cible, isolation, etc.) restent soumises à leurs contrôles habituels.

1. `fmt --check`.
2. `test --no-fail-fast` sans `-p`; le run N1 dédié utilise le mode gardé sans variable `CARGO_TARGET_DIR` et vérifie `target_directory`.
3. `test <filtre>` pour chacun des huit filtres du cadrage :
   - `resume_validation_rejects_later_fifo_row_without_advancing_any_generation`;
   - `interrupted_`;
   - `injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume`;
   - `cancelling_duplicate_import_releases_only_the_new_source_snapshot`;
   - `stale_duplicate_dialog_response_cannot_submit_a_replacement`;
   - `r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision`;
   - `resumed_prefix_copy_retries_after_pending_creation_without_duplication`;
   - `unified_poll_resumed_prefix_emits_only_new_segments`.
4. Pour chacun de `whisper-worker-cpu`, `whisper-worker-parakeet-cpu`, `whisper-worker-common` : `test -p <membre>`, `test --no-run -p <membre>`, `test -p <membre> -- --list`.
5. Clippy du plan, avec exactement `whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common`, `whisper-worker-parakeet-cpu`, `--all-targets`, JSON et les quatre lints structurels.

Pour toute autre racine, l’admission et les commandes sans `-p` conservent les règles historiques. Les nouvelles sélections `-p` ne sont pas transférées aux racines voisines/enfants/non inscrites. `fetch`, `--manifest-path`, `--workspace`, les options absentes des profils, les combinaisons étrangères et les sélections GPU sont refusés selon le cadrage.

## Preuve et environnement

- Le SelfTest complet actuel effectue quatre probes `cargo metadata --offline --locked --no-deps --format-version 1` (deux modes de cible, deux modes de variable). Il n’exécute pas la compilation du produit; toutefois, il lance Cargo. Ces quatre appels et les écritures de SelfTest doivent figurer dans l’accord précis du G0.
- Invocation unique proposée après accord, depuis la racine du worktree approuvé : `pwsh -NoProfile -File ci/guard-level1.ps1 -SelfTest -LotRoot target/parakeet-tests/refacto-reprise-import`. Ne pas omettre `-LotRoot`, car le défaut vise une autre racine. Attendu : code retour 0, quatre probes `metadata` observées (2 modes de cible × présence/absence de `CARGO_TARGET_DIR`), aucune compilation; arrêt au premier échec et conservation des sorties.
- Ne pas utiliser `-DryRun` comme preuve sans Cargo : il peut appeler `metadata` et initialiser une `FreshTarget`.
- Le candidat de worktree relevé était propre sur `6193fbdf`, mais aucune cible partagée ni cache n’a été détecté lors du plan. Au G0, vérifier leur présence et les autres prérequis. Si le SelfTest réclame une récupération réseau ou une cible de compilation, arrêter; `fetch` n’est pas autorisé par ce plan.
- Les sous-tests de décision de politique utilisent un rappel simulé; ils ne doivent pas lancer de processus Cargo. Les commandes de validation syntaxique et `git diff --check` sont distinctes des quatre probes metadata.
- Archive proposée si les probes sont exécutés : `docs/architecture/raw/GARDE-N1-REPRISE-SELFTEST.zip`, avec stdout/stderr, résultats, compteurs et inventaire préservés. Cet artefact doit être confirmé dans l’accord G0; aucun fichier de preuve n’est créé pendant le cadrage.

## Dépendance à l’implémentation de reprise et risques

- **Q-POL-01 reste ouverte pour le lot produit :** une sélection worker CPU, même `--no-run`, peut reconstruire `whisper-rs-sys` via CMake. La garde ne préjuge pas de cette nécessité. Avant le premier Cargo produit concerné, le responsable doit établir si la compilation est inévitable et obtenir, le cas échéant, une autorisation précise de cette compilation CPU avec les contrôles applicables. L’exception historique d’un autre lot n’est pas transférée. Les builds DLL natives restent suspendus.
- La commande CPU, PE, inventaire et corps de tests doivent rester séquentiels sur une même cible : `--no-run`, contrôle PE de chaque chemin réellement produit (`Cpu` ou `ParakeetCpu`), puis `-- --list` et corps. Un échec bloque les étapes suivantes.
- N1 complet, Clippy de validation, contrôles PE et tests `recovery.rs` ne sont pas exécutés par le lot d’adaptation. Après livraison/revue de la garde, le T0 du plan de reprise doit être rejoué en entier sur un candidat contenant ce commit.

## Autorisations et limites

Ce document propose un plan; il n’autorise pas l’édition du script PowerShell, l’exécution des quatre probes metadata, le test Cargo du produit, une compilation native, l’écriture de l’archive, un commit de code, un merge ou un push. L’utilisateur doit donner un accord précis en session pour le chemin PowerShell, les appels SelfTest/Cargo metadata, le worktree et les preuves. Tout nouveau chemin, outil, gate ou comportement sortant de ce plan impose un arbitrage avant action.

Après accord, l’auteur de code et le chemin seront confirmés avant G1. Le reviewer indépendant rend seul le verdict; la session principale rapporte séparément la garde, les tests, la revue et la validation du lot produit.
