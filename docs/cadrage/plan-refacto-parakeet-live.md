Avancée : plan de refactoring proposé à partir du cadrage DESIGN-PL v1.
Blocage : acceptation du seuil, des chemins et du préflight de campagne avant implémentation.
Décision : dix tâches au plus, sans autorisation de coder ni de lancer Cargo dans cette session.

# Plan de refactoring de Parakeet Live — PLAN-PL v1

## 1. Parent et portée

Parent : [DESIGN-PL v1](refacto-parakeet-live.md), établi sur `b09a8b669834803cfd46e15c3c10eefdf744b518`. Mandat documentaire du 31 octobre 2026 : cadrage puis plan, après clarification « fichier entier, tests compris ». Le plan est **proposé pour validation** ; sa rédaction ne vaut ni acceptation du lot, ni autorisation de code, de campagne Cargo ou de push.

Objectif du lot futur : refactoring structurel à comportement constant. Chemins source proposés en écriture : `crates/whisper-adapters/src/parakeet_live.rs` et les neuf fichiers sous `crates/whisper-adapters/src/parakeet_live/` énumérés au §4 du design (six modules de production et trois modules de tests). Le répertoire n'existe pas encore. Aucun autre source, manifeste, `Cargo.lock`, modèle, runtime, script ou dossier non suivi n'est ouvert en écriture par ce plan. Le compte rendu du lot utilisera les livrables de cadrage existants ; tout nouveau chemin, garde ou artefact exigera l'accord précis applicable.

## 2. Préflight et conditions d'exécution

Avant toute édition, relever HEAD, branche, état Git et contenus non suivis ; comparer au parent. Lire de nouveau `AGENTS.md` racine/adapters et les sources utiles. Si la base a dérivé, réévaluer les plages, contrats et tâches avant de coder. L'utilisateur doit accepter dans la session d'implémentation le seuil **≤ 650 lignes physiques**, le lot, ses chemins et les vérifications ; la session principale route alors le code à `rust_implementer` en mode IMPLEMENT, puis la revue à `rust_review_coordinator` selon `AGENTS.md`.

Avant tout Cargo, relever l'espace libre (arrêt sous 30 Gio), fixer une racine de lot distincte sous `target/parakeet-tests/` par accord précis, trouver une garde applicable, faire ses auto-tests et vérifier canoniquement `CARGO_HOME`, `TEMP`, `TMP`, `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. `CARGO_TARGET_DIR` est uniquement `target/parakeet-tests/cargo-shared/`. Utiliser `--offline --locked` lorsque accepté ; aucune commande Cargo directe ni téléchargement sans campagne autorisée. Si la garde existante ne couvre pas ce lot, arrêter et faire arbitrer l'adaptation et ses chemins avant exécution. Toute violation ou échec suit la règle d'arrêt et de conservation des preuves d'`AGENTS.md`.

## 3. Tâches bornées

| Tâche | Changement autorisable et dépendance | Acceptation / contrôle |
| --- | --- | --- |
| T1 | Caractériser la base avant extraction : inventaire `pub(crate)`, neuf tests inline, filtres exacts, consommateurs, scénarios et branches d'erreur ; aucun code produit. Dépend du préflight. | CA-03/04/05 : relevé initial, contrôle des filtres, limites des tests natifs distinguées |
| T2 | Extraire `UnifiedCapturePump`, gain et méthodes de capture/télémétrie dans `capture.rs`, façade inchangée. Après T1. | CA-02/03 : ACK, conversion, gain, WAV/backlog et erreurs inchangés |
| T3 | Extraire `TraceSink`, sérialisation bornée et diagnostic D2 dans `trace.rs` avec leurs `cfg(test)`. Après T1. | CA-02/03 : même activation, destination, limites et aucun diagnostic D2 produit |
| T4 | Extraire types/loop d'inférence et transcription dans `inference.rs`, conserver les canaux bornés et la politique CPU/GPU/Auto. Après T3. | CA-02/03 : mêmes identités, backend, timeouts, erreurs et marqueurs |
| T5 | Extraire ordonnancement, validation, reprise et ledger dans `windows.rs`. Après T4. | CA-02/03 : mêmes plages, séquences, tentative et nettoyage |
| T6 | Extraire `poll`, confirmation, événements et erreurs de fenêtre dans `events.rs`. Après T5. | CA-02/03 : mêmes événements, ordre, blocs confirmés et refus |
| T7 | Extraire démarrage, arrêt normal/erreur, drainage, `Drop` dans `lifecycle.rs`. Après T2/T6. | CA-02/03 : acquisition, finalisation WAV/TXT/SRT et échecs inchangés |
| T8 | Déplacer les trois modules de tests inline dans leurs fichiers homonymes, garder leur déclaration et leurs noms qualifiés ; limiter le parent à la façade. Après T7. | CA-01/02/04 : neuf tests conservés, parent ≤ 650 lignes physiques sans compression artificielle |
| T9 | Vérifier le candidat cumulatif sous garde autorisée : format, compilation, tests ciblés et contrôles utiles du lot ; confronter résultats à T1. Après T8. | CA-01 à CA-05 : diff, comptage, noms, tests et limites consignés |
| T10 | Revue indépendante et corrections bornées selon `AGENTS.md`, puis bilan du lot et remise de la recette manuelle si prévue. Après T9. | CA-01 à CA-05 : CLEAN ou LIVRÉ AVEC RÉSERVES après deux cycles maximum |

Les tâches sont séquentielles là où l'état de session est partagé. Chaque extraction garde les méthodes sur le même type `ParakeetLive` et les points d'entrée `pub(crate)` ; elle n'introduit pas de nouvelle politique. Une nécessité de modifier un chemin hors §1, d'ajouter un test/support ou de changer le comportement déclenche un PLAN_CHANGE avant édition.

## 4. Vérifications futures, sous campagne autorisée

Contrôles statiques : `git diff --check`, inventaire des signatures et `#[test]`, noms qualifiés avant/après, `(Get-Content -LiteralPath ...).Count` pour le parent et chaque extrait, `cargo fmt --check` seulement dans le cadre autorisé si son exécution lance une campagne selon la règle locale. Le parent doit compter au plus 650 lignes physiques ; les nouveaux fichiers de production restent sous la cible locale de 1 500 lignes, sans agrandir artificiellement le fichier parent.

Contrôles Cargo **à sélectionner et figer sur le candidat avant campagne** : `cargo test --offline --locked -p whisper-adapters --lib parakeet_live:: -- --test-threads=1`, le test exact `parakeet_live::microphone_spectrum_tests::parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav`, les tests consommateurs `worker_ipc` et le corpus D2 selon leurs prérequis, puis `cargo build --offline --locked -p whisper-adapters` et `cargo clippy --offline --locked -p whisper-adapters --all-targets -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity`. Vérifier d'abord par `cargo test --list` que chaque filtre cible effectivement des cas ; zéro cas n'est pas PASS. Les tests natifs, le GPU réel et la fenêtre Windows sont `NOT RUN` tant qu'ils ne sont pas réellement exécutés dans un périmètre autorisé. La validation finale dans la fenêtre appartient à l'utilisateur lorsqu'elle est prévue.

L'ordre de comparaison est base T1, extraction par lots T2–T8, candidat cumulatif T9, revue T10. Un échec de compilation/test ou une écriture de campagne hors racine attribuée impose l'arrêt, la conservation des preuves et l'arbitrage prévu par `AGENTS.md`. Pas de suppression de fichiers temporaires sans état antérieur et manifeste approuvé.

## 5. Décisions avant implémentation

| ID | Options et conséquence | Recommandation |
| --- | --- | --- |
| D-PL-01 | Accepter ≤ 650 lignes physiques pour le fichier entier, tests compris, ou définir une autre borne chiffrée. | Accepter 650 comme tolérance de « environ 600 », sans compactage artificiel. |
| D-PL-02 | Accepter les dix chemins source proposés, ou demander un découpage différent. | Accepter la famille privée `parakeet_live/` après revue des visibilités. |
| D-PL-03 | Pour la future campagne, identifier une garde existante applicable et autoriser sa racine de lot ; si adaptation nécessaire, demander son périmètre précis. | Traiter ce prérequis lors de l'autorisation d'implémentation, avant tout Cargo. |

**Statut : PLANS PROPOSÉ.** Aucun code, test, build, lint ou revue indépendante du candidat n'a été exécuté pour ce plan. L'accord sur ce plan et les autorisations distinctes d'implémentation/campagne restent nécessaires.
