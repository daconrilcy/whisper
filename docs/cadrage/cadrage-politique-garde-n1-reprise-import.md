Avancée : proposition bornée pour permettre les vérifications prévues du lot `resume_interrupted`.
Blocage : la branche documentaire ne contient pas le durcissement de garde de `6193fbdf`; le worker CPU peut aussi déclencher CMake.
Décision : matrice POL-N1-REPRISE-01 acceptée par l’utilisateur le 12 novembre 2026; l’accord ne couvre ni le code de garde ni une compilation CPU/CMake.

# Cadrage — politique de garde N1 pour la reprise d’import

**Nature :** adaptation ciblée de `ci/guard-level1.ps1`, préalable au lot d’implémentation de `resume_interrupted`.
**État :** ACCEPTÉ le 12 novembre 2026 — la matrice POL-N1-REPRISE-01 a été acceptée telle quelle dans la session. Cela n’autorise pas la modification de la garde ni l’exécution de commandes Cargo.
**But produit :** permettre les contrôles N1 et Clippy nécessaires pour livrer la refactorisation sans changer le comportement de reprise.

## Sources et état observé

- Plan PLANS CLEAN : `docs/cadrage/plan-refacto-reprise-import.md`, commit documentaire `3812836ae0b4d8581827b1b09929866c576caeb7`; son accord d’implémentation est limité à `recovery.rs` et conditionné au T0.
- La branche documentaire courante est `codex/clippy-priorites-impl@7993ad9d`; elle diverge de `main@6193fbdf`, qui contient le durcissement de sélection des paquets. Le fichier de garde de la branche documentaire ne démontre donc pas la politique réellement à adapter. Le futur candidat doit partir d’une base qui contient `6193fbdf` ou un descendant sans régression de ce durcissement.
- Les décisions D-02 et D-04 de `docs/cadrage/evo-garde-n1-paquets.md` sont déjà acceptées : politique versionnée liée à une racine canonique; refus global de `--workspace`; sélections `-p` uniquement aux racines enregistrées.
- La garde actuelle enregistre une liste exacte de six paquets pour une racine et ne peut pas, avec cette seule liste, admettre à la même racine le Clippy de six paquets, les sélections worker unitaires, `fmt` et les tests sans `-p`.
- Le `whisper-worker-cpu` peut reconstruire `whisper-rs-sys` et appeler CMake. La cible partagée ou le cache présent ne prouvent pas que cette compilation sera évitée. Une ancienne autorisation de compilation CPU était limitée à un autre lot.

## Besoin et limites

À la racine exacte `target/parakeet-tests/refacto-reprise-import`, la garde doit admettre seulement les commandes requises par T0–T4 du plan, avec les sélections et options convenues. Elle doit refuser les commandes et combinaisons hors matrice avant tout lancement de Cargo, notamment `metadata`. La politique historique des autres racines reste en vigueur; aucun accès n’est ajouté à `--workspace`, aux sélections GPU ou aux racines non enregistrées.

Cette adaptation ne modifie ni le produit, ni `recovery.rs`, ni les manifestes/verrou, ni la baseline Clippy, ni les contrôles PE. Elle n’autorise pas d’exécuter Cargo, de compiler `whisper-rs-sys`, de lancer un build natif/DLL, de modifier la garde, de pousser ou de fusionner. La validation produit Windows reste réservée à l’utilisateur.

## Contrat proposé — POL-N1-REPRISE-01

1. Enregistrer la racine canonique exacte `target/parakeet-tests/refacto-reprise-import` et une matrice de profils versionnés. Résoudre la politique supplémentaire de ce lot après canonicalisation de la racine et avant tout sous-processus Cargo. Une racine voisine ou enfant ne reçoit pas ces nouveaux profils; les règles déjà en vigueur pour cette racine restent inchangées.
2. Réutiliser le parseur N1 existant et comparer les arguments validés, après les normalisations internes existantes. Une entrée ne devient admissible que si verbe, options, mode, filtres et sélection correspondent à un profil complet ci-dessous; aucune option additionnelle n’est admise.
3. Profils proposés pour cette seule racine :
   - `fmt --check`;
   - `test --no-fail-fast` sans sélection `-p`, y compris le mode gardé sans variable `CARGO_TARGET_DIR` prévu par le plan;
   - `test <filtre>` pour les huit filtres de témoins du plan : `resume_validation_rejects_later_fifo_row_without_advancing_any_generation`, `interrupted_`, `injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume`, `cancelling_duplicate_import_releases_only_the_new_source_snapshot`, `stale_duplicate_dialog_response_cannot_submit_a_replacement`, `r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision`, `resumed_prefix_copy_retries_after_pending_creation_without_duplication`, `unified_poll_resumed_prefix_emits_only_new_segments`;
   - pour chacun de `whisper-worker-cpu`, `whisper-worker-parakeet-cpu`, `whisper-worker-common`, `test -p <membre>`, `test --no-run -p <membre>` et `test -p <membre> -- --list`;
   - le vecteur Clippy exact du plan : six membres `whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common`, `whisper-worker-parakeet-cpu`, `--all-targets`, JSON et les quatre lints structurels;
   - `test` et les témoins sans `-p` ne peuvent pas inclure `--manifest-path`, `--workspace`, `--features`, `--release`, ni une autre option absente des profils. `fetch` est refusé à cette racine; le cache doit déjà satisfaire le préflight hors ligne.
4. Refuser les doublons, omissions, surplus, mauvais ordres/verbes, options inconnues, sélections croisées, GPU, autre forme de filtre, toute racine non inscrite pour une sélection explicite et toute tentative de contourner la sélection via `--manifest-path`. Le refus global de `--workspace` reste inchangé.
5. Garder intacte la liste historique de six paquets et ses tests. La politique nouvelle ne doit ni rendre `-p` permissif hors racine enregistrée ni modifier les autres protections environnementales et de chemins.

## Critères d’acceptation proposés

- **AC-POL-01 — admission ciblée :** chaque profil ci-dessus est accepté à la racine exacte du nouveau lot, sous les deux interfaces d’entrée `-CargoArgs` et `-CargoArgsJson`, et avec les formes de sélection déjà reconnues par la garde (`-p nom`, `-pnom`).
- **AC-POL-02 — refus avant Cargo :** pour les commandes ciblées par la nouvelle politique, chaque classe de refus (racine sœur/enfant/non inscrite avec sélection explicite `-p`, sélection absente/dupliquée/supplémentaire/croisée à la racine du nouveau lot, workspace, manifest-path, option ou commande hors profil, GPU) n’appelle ni Cargo ni `metadata`; les diagnostics identifient le motif. Le SelfTest instrumente la frontière d’invocation et prouve zéro lancement réel pour les refus.
- **AC-POL-03 — non-régression :** les positifs et négatifs de la politique historique restent couverts; les refus globaux de `--workspace`, des sélections non enregistrées et des options existantes sont préservés. Le comportement de l’ancienne racine et des commandes hors de cette nouvelle matrice reste inchangé.
- **AC-POL-04 — gate distincte :** les SelfTests de politique ne sont pas présentés comme preuve d’exécution N1. Les appels positifs à `-DryRun` et le SelfTest complet peuvent atteindre `metadata`; leurs effets et prérequis sont rapportés séparément. Les références T0–T4 restent conditionnées aux autres contrôles du plan.

## Risque et question qui reste ouverte

**Q-POL-01 — compilation worker CPU avant l’exécution N1.** Les sélections `--no-run`, inventaire ou test peuvent déclencher la compilation de `whisper-rs-sys`/CMake si une dépendance doit être reconstruite. Options pour le futur lot : (A) arrêter avant la première compilation si elle est nécessaire et demander une autorisation spécifique, recommandation; (B) solliciter dans la session d’exécution une exception précisément limitée à cette compilation CPU avec lecture WHEA avant/après; (C) réviser formellement la matrice N1 si une autre preuve est proposée. Un cache ou une cible existante ne suffit pas à établir que le build sera évité. Les builds DLL natifs restent suspendus.

## Traçabilité et readiness

| Besoin / source | Critère | Décision ou preuve |
|---|---|---|
| Le plan de reprise exige Clippy, tests ciblés et matrice N1; AGENTS fixe la suite et les protections | AC-POL-01, 04 | Politique ciblée proposée; commandes encore NON EXÉCUTÉES |
| D-02/D-04 acceptés dans `evo-garde-n1-paquets.md` | AC-POL-02, 03 | Racine canonique versionnée et refus `--workspace` global maintenus |
| `main@6193fbdf` contient le durcissement absent de `7993ad9d` | AC-POL-03 | Base du futur candidat doit contenir `6193fbdf` |
| Le worker CPU peut dépendre d’une reconstruction CMake | Q-POL-01 | Autorisation de compilation non résolue et non transmise d’un ancien lot |

**Readiness :** POL-N1-REPRISE-01 acceptée par l’utilisateur le 12 novembre 2026. La mise en œuvre de la garde exige un accord précis dans la session pour toucher `ci/guard-level1.ps1` et ses SelfTests. La reprise de `recovery.rs` attend en plus le T0 complet, dont la résolution de Q-POL-01 si la compilation CPU est nécessaire.

## Apports bruts des spécialistes

**`rust_requirements_analyst`, DRAFT, base `7993ad9d` comparée à `main@6193fbdf` :**

> REQ-G01 / UC-G01 — Admission de la matrice du lot. AC-G01 : admettre Clippy avec les six membres exacts et les quatre lints, les tests par défaut et les filtres du plan, ainsi que `test -p <worker>` individuellement pour CPU, Parakeet CPU et common, avec préparation, inventaire et exécution. `fmt --check` reste disponible. Tester les deux interfaces d’arguments et les formes `-p name` et `-pname`. Refuser avant tout processus Cargo, y compris `metadata`, les racines inconnues pour une sélection explicite, `--workspace`, les sélections dupliquées, incomplètes ou en surplus, les croisements de commande et de sélection, ainsi que `--manifest-path` et les autres options étrangères à la matrice. Les cas historiques restent couverts.
>
> REQ-G02 / UC-G02 — Exécution contrôlée. AC-G02 : préflight avec ≥30 Gio, racines et cache contrôlés, modèle Parakeet épinglé, cible unique. Le N1 par défaut exécute `test --no-fail-fast` dans un shell dédié sans `CARGO_TARGET_DIR`, avec `target_directory` effectif vérifié. Pour chacun des trois workers, préparer les harnais par `--no-run`, vérifier chaque exécutable effectivement produit avec le contrôle PE, puis inventorier et exécuter sur la même cible. Sur échec, arrêter les opérations suivantes et identifier la première cause. Un refus PE empêche l’inventaire et l’exécution concernés.
>
> REQ-G03 / UC-G03 — Reprise sur preuve. AC-G03 : attribuer au même candidat Git le diff de garde, les résultats d’auto-tests et la revue. `-DryRun` positif peut lancer `metadata` et créer une cible fraîche. L’admission ne vaut pas résultat N1. L’exécution de `recovery.rs` reprend après satisfaction des autres prérequis T0.
>
> Q-G01 : enregistrer une racine exacte et des profils limités par commande, sans réutiliser la garde ancienne de `7993ad9d`. Q-G02 : avant Cargo CPU, établir la cible autorisée et demander une décision si la compilation `whisper-rs-sys` ne peut être écartée; l’autorisation d’un autre lot ne se transmet pas.

**`rust_architect`, proposition technique en lecture seule, base de référence `main@6193fbdf` :**

> Enregistrer `target/parakeet-tests/refacto-reprise-import` comme clé canonique exacte. Pour cette clé, admettre `fmt --check`, `test --no-fail-fast` sans `-p`, les huit filtres du plan, les trois commandes worker (`test`, `--no-run`, `-- --list`) pour CPU/Parakeet CPU/common, et le Clippy JSON exact des six membres avec `--all-targets` et quatre lints. Comparer les arguments parsés par le parseur existant, avec la normalisation existante de `--offline --locked`; ne pas réimplémenter le parseur.
>
> Couvrir les branches `fmt`/`fetch` aussi; interdire `fetch`, `--manifest-path`, `--workspace`, les combinaisons hors matrice, les racines voisines/non inscrites et tout GPU. Les refus sont prouvés avant tout processus Cargo; distinguer cela du SelfTest complet et de `-DryRun`, qui peuvent invoquer `metadata`. Protéger les tests de la politique historique. Le contrôle PE s’applique à chaque harnais effectivement produit avant inventaire et corps.
>
> Base : la branche documentaire `7993ad9d` ne contient pas le durcissement `6193fbdf`; ne pas l’utiliser comme voie T0. La compilation CPU via `whisper-rs-sys`/CMake reste une décision distincte. Aucun changement de code ni test exécuté dans cette analyse.

**Sources vérifiées :** `AGENTS.md`; `ci/guard-level1.ps1` à `main@6193fbdf`; `docs/cadrage/evo-garde-n1-paquets.md`; plan PLANS CLEAN du lot de reprise au commit `3812836a`; dépendance `crates/whisper-worker-cpu/Cargo.toml` et graphe de `Cargo.lock`; antécédent de compilation bornée dans `docs/cadrage/plan-tests-stables-2.md`. Aucun SelfTest, Cargo, PE ou code produit exécuté.
