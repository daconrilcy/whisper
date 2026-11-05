Avancée : inventaire initial en lecture ; worktree tests-hygiene créé depuis 9f5ab7aa, aucun code ni Cargo exécuté.
Blocage : arbitrage du chemin de validation sans CARGO_TARGET_DIR et avec cible extérieure, incompatible avec la garde actuelle.
Décision : faire valider ce plan par rust_review_coordinator ; aucune implémentation avant fermeture des prérequis.

# TESTS-HYGIENE — plan

## Mandat et périmètre

Rendre les tests autonomes du produit fiables : trois `cargo test --no-fail-fast` consécutifs sans échec ni fenêtre système, fixtures indépendantes de CARGO_TARGET_DIR/target et reprise GPU manuelle testée sans course. Branche `tests-hygiene`, worktree `C:/wt/tests-hygiene`, base `9f5ab7aa9ef291155f8018c05a0bf3fd685884ff`. Aucun merge/push, aucun changement de dépendance/Cargo.lock ni de code produit. Seuls Rust, tests/support, contrôles N1 nécessaires et consigne AGENTS sont concernés. C/C++/CMake/compilation de DLL suspendus ; les faux workers sont des exécutables Rust, compilés par rustc et liés normalement.

Livrables : ce plan versionné, bilan figé d'une page `docs/architecture/TESTS-HYGIENE-BILAN.md`, preuves détaillées archivées sans modèles ni anciens bilans réécrits. Six tâches, deux cycles revue/correction maximum par livrable. La session principale écrit les documents et les commite ; rust_implementer possède les modifications Rust, rust_review_coordinator organise la revue indépendante.

## 1 — Préflight et accord d'exécution

État initial : main propre au SHA demandé ; worktree neuf ; mémoire validée 35 829 911 552 / 257 457 549 312 octets (13,92 %), environ 435 Gio libres. Avant chaque campagne, contrôler espace ≥30 Gio, committed ≤85 %, absence de conflit pertinent, chemins canoniques sans jonction et environnement isolé. Relever mémoire finale. Ne créer aucun service, tâche, ACL ni surveillance nouvelle.

Conflit constaté dans ci/guard-level1.ps1 : cible par défaut partagée, cibles alternatives internes au lot, variable CARGO_TARGET_DIR obligatoire ; --no-fail-fast/--lib/--test et filtres généraux non admis. Proposer une extension bornée de cette garde : autoriser les sélections de test utiles (avec maintien des refus native-tests/ignored/options inconnues), un mode d'exécution sans CARGO_TARGET_DIR utilisant une configuration Cargo vérifiée, et une cible extérieure explicitement déclarée `C:/wb/tests-hygiene/cargo-target`. Le cache Cargo isolé persistant reste `C:/wt/tests-hygiene/target/cargo-home-shared/` ; TEMP/TMP/LOCALAPPDATA sous `target/tests-hygiene/`. CARGO_INCREMENTAL=0 et debug=line-tables-only. En mode sans variable, la configuration de cible ne doit pas introduire une dépendance de fixture ; contrôler le target_directory effectif avant exécution. Pas de modification du profil Cargo utilisateur. Cet élargissement de garde et le chemin extérieur demandent un accord précis avant code/écriture.

Le test depuis C:/dev/whisper utilisera le manifeste du worktree corrigé (`--manifest-path C:/wt/tests-hygiene/Cargo.toml`) dans un shell à ce répertoire courant, sans modifier main. Les trois campagnes de référence exécutent la commande simple depuis le worktree. Les autres variables d'isolation ne sont pas des prérequis fonctionnels des tests. Cache manquant : copie contrôlée du cache existant ou fetch --locked via garde, jamais installation implicite. Les default-members ne comprennent pas les workers whisper-rs-sys ; ne pas élargir aux builds workers/CUDA ni lancer une dépendance C/C++.

Point d'arrêt : revue du plan et arbitrage de l'extension/cible ; pas de Cargo avant leur réussite.

## 2 — Fixtures autonomes et absence de hard error Windows (A, B, E)

A est établi par l'humain : aucun nouvel essai de diagnostic. Dans `crates/whisper-adapters/tests/support/parakeet_runtime_isolation.rs`, compiler le faux worker sous base temporaire existante, retirer toute lecture obligatoire de CARGO_TARGET_DIR ; nettoyage par propriétaire même sur échec. Ce support est utilisé par lib et integration ; vérifier les deux chemins. Ne pas déplacer une dépendance vers une autre variable ou un dossier de lot.

Inventorier tous les créateurs de faux exécutables et leurs consommateurs, notamment `src/parakeet_import/{legacy_runtime_tests,worker_log_tests}.rs`, `src/worker_ipc/tests/reprocessing_and_transport.rs`, tests/support et supports partagés. Pour chaque cas, capturer assertions/branche avant-après : inexistant pour erreur d'absence ; vrai PE Rust quittant en erreur pour processus démarré puis échoué. Préserver code d'erreur public, ordre et nombre des tentatives CPU, logs et politique Auto/GPU. Si la distinction OS193/absence/processus échoué exige une modification produit, arrêter avant celle-ci. Ne pas exécuter un faux EXE invalide pour réobserver une fenêtre déjà signalée. SetErrorMode/SetThreadErrorMode ne sont pas retenus sans expérience autorisée, ni ajout unsafe implicite.

Recherche exacte demandée : `rg -n 'env::var(_os)?\(".*"\)\.(expect|unwrap)' crates/`, avec classement des occurrences en parent autonome, enfant explicitement configuré, tests natifs ignorés/feature-gated, fallback et code produit. Compléter les expressions multilignes si pertinent ; ne corriger que les préconditions cassant cargo test simple. Relever les sites fichier:ligne, grouper les résultats dans le bilan et conserver l'inventaire complet dans les preuves.

## 3 — Cause de la course (C)

Périmètre initial `src/worker_ipc/runtime/poll/tests/stop_recovery.rs`, supports de ce module et chemins poll/transport en lecture. Distinguer délai (Instant), files d'événements concurrentes, racines partagées, handshake du faux worker et état de capture. Budget : trois essais diagnostiques ou trente minutes pour ce point, journal compact des hypothèses/observations ; pas de reproduction du point A. Corriger la synchronisation/fixture/assertion seulement après preuve, sans allonger un timeout ni ignorer un test. Les assertions de préservation de la requête et d'attente du choix doivent rester effectives.

Point d'arrêt impératif : si le défaut concerne le code produit, rendre cause et preuve, ne pas le corriger. Aucun développement produit n'est autorisé par ce lot.

## 4 — Anti-régression proportionnée (D)

Recommander un constructeur partagé de faux worker PE, utilisé par les fixtures exécutées, et un test de contrat refusant un contenu non-PE sans jamais le lancer. Éliminer les écritures ad hoc d'exécutables invalides identifiées ; un contrôle ciblé des sites de création/launch complète le contrat si nécessaire, sans moteur d'analyse général. Les fixtures PE malformées destinées uniquement à l'analyse binaire ne doivent pas être exécutées ni confondues avec des workers. Pas de dépendance à des binaires sous target ni à une DLL installée. Les nouveaux tests restent séparés et leurs enregistrements ci/tests-baseline.txt sont actualisés si nécessaire, sans changement de classement pour masquer un test natif.

## 5 — Validation et revue

Sous le chemin validé, sans CARGO_TARGET_DIR et avec mêmes default-members : inventaire --list avant/après ; tests A lib et integration depuis worktree et depuis C:/dev/whisper (manifeste du worktree), puis trois campagnes `cargo test --no-fail-fast`, totaux passed/failed/ignored et durées par cible, noms identiques sauf ajouts justifiés. Rust offline/locked, sans features natives ; ne pas relancer une compilation native.

Après correction C, cinquante exécutions de **la suite whisper-adapters lib complète**, afin de conserver sa concurrence, avec ce test présent et passé à chaque itération. Ce sont des validations, distinctes du quota de diagnostic. Pas cinquante exécutions isolées qui masqueraient la contention. Les échecs arrêtent la campagne selon AGENTS ; aucun retry caché.

Guard-level1 SelfTest et contrôle Structure/registre ; guard-native SelfTest borné sans bootstrap/CMake/DLL. Vérifications Rust utiles (format et lint concernés) via voie autorisée, sans commande ajoutée hors plan. L'implémenteur rapporte les preuves ; le coordinateur confie la revue au rust_reviewer indépendant avant commit final. L'attestation de zéro fenêtre système reste humaine, demandée pour l'intervalle réel des campagnes et consignée séparément des résultats automatisés.

Ajouter une ligne AGENTS en validation : une validation N1 comprend une exécution sans CARGO_TARGET_DIR dans un shell dédié, `cargo test --no-fail-fast`, via le mode autorisé contrôlant la cible effective. Cette règle doit correspondre à la voie réellement approuvée, sans donner une exemption implicite aux contrôles de racines.

## 6 — Rétention et remise

Relever les dossiers temporaires effectivement créés et les préexistants correspondant à whisper-adapter-tests, parakeet-personal-no-l3b-* et parakeet-p1-* : chemin, propriétaire du lot si démontrable, taille, liens et éventuels processus actifs. Ne rien supprimer ; fournir à l'humain des commandes limitées aux chemins vérifiés, après archivage des preuves utiles. Préserver caches partagés/persistants, sources, audios et tous les espaces protégés des lots natifs.

Bilan ≤1 page : causes A/B/C, anti-régression, recherche d'environnements regroupée, trois campagnes, cinquante itérations, totaux/durées/écarts, attestation humaine, mémoire et dettes. Données détaillées dans l'archive. Commit(s) locaux sur tests-hygiene uniquement, aucun merge/push. Si une attestation ou un critère manque, le marquer NON EXÉCUTÉ/NON ATTESTÉ, sans annoncer l'objectif atteint.
