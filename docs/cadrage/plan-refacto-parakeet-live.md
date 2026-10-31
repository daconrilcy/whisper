Avancée : découpage réalisé, contrôles ciblés réussis et revue indépendante CLEAN.
Blocage : recette D2 native, GPU réel et fenêtre Windows non exécutés ; suite globale interrompue non déclarée PASS.
Décision : commit local et push du périmètre exact autorisés dans cette session ; conserver les limites de preuve.

# Plan de refactoring de Parakeet Live — PLAN-PL v1

## 1. Parent et portée

Parent : [DESIGN-PL v1](refacto-parakeet-live.md), établi sur `b09a8b669834803cfd46e15c3c10eefdf744b518`. Mandat documentaire du 31 octobre 2026 : cadrage puis plan, après clarification « fichier entier, tests compris ». Après versionnement de ces documents au commit `23fa1c8318d86e997ef00583186b170325e57201`, l'utilisateur a écrit dans cette session : « Ok tu peux implémenter à partir de ces documents ». Cet accord accepte le lot du plan, sa borne de 650 lignes et ses dix chemins source ; il ne vaut ni autorisation d'un nouveau chemin de campagne ou d'une adaptation de garde, ni autorisation de push.

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

**Statut : PLANS ACCEPTÉ POUR IMPLÉMENTATION.** L'accord sur le lot source est reçu dans cette session. La racine de campagne, la garde et l'exécution de Cargo restent soumises à un accord précis distinct. Aucun code, test, build, lint ou revue indépendante du candidat n'a été exécuté lors de cette mise à jour documentaire.

## 6. Réalisation, preuves et clôture du lot

Sur la base `bf2d3a2e23d5235664537e5d2f00305d2a216aba`, l'implémenteur a modifié uniquement les dix chemins Rust proposés au §1. Le fichier principal est passé de 3 138 à **202 lignes physiques** ; les neuf extraits comptent de 57 à 551 lignes. Les 313 noms de tests de bibliothèque listés avant et après sont identiques, dont les neuf tests inline déplacés. Aucun manifeste, `Cargo.lock`, modèle, worker ou fichier des autres parcours n'a été modifié par ce lot.

Sous la garde `target/parakeet-tests/refacto-parakeet-live/guard.ps1`, avec cache et temporaires isolés, `CARGO_TARGET_DIR` canonique, `--offline --locked` et les réglages de campagne prescrits : les 36 tests `parakeet_live::` ont réussi avant et après extraction ; le test consommateur de gain a réussi ; `cargo fmt --all -- --check`, `cargo build -p whisper-adapters` et Clippy `--all-targets` avec les quatre lints structurels ont terminé code 0. Clippy émet encore des avertissements, dont sur des fonctions déplacées ; ce succès ne prétend pas que chaque seuil structurel est atteint. `git diff --check` a réussi. Le relecteur Rust indépendant a rendu **CLEAN** sur le candidat, puis maintenu ce verdict après les nouvelles preuves : deux passages de revue, zéro correction demandée.

Une tentative supplémentaire de suite globale `whisper-adapters --lib` a été interrompue après huit `FAILED` observés et avant son bilan final : elle n'est **pas PASS**. L'inventaire statique a relevé des variables de racine ou de fixture manquantes, sans établir les messages de panic de cette tentative. Après arbitrage explicite, sept de ces tests ont été repris séparément sous la garde adaptée avec racines isolées et une copie MP3 vérifiée ; chacun a réussi (1 test, 0 échec). Les sept journaux sont dans `target/parakeet-tests/refacto-parakeet-live/logs/`, runs `20261031T161807022-756cc816`, `20261031T161823652-97373bbd`, `20261031T161836051-35f0b6de`, `20261031T161855333-c4227322`, `20261031T161857122-a132f08e`, `20261031T161858822-4e105e6e` et `20261031T161900486-7f7d963e`. La copie MP3 conserve le SHA-256 attendu `eebce5a2af18d4f929af5c243c50a6bc31b0910a583577cb940e0e4895e7cd80`.

La recette D2 native reste **NOT RUN** : les ressources qu'elle attend sous `%LOCALAPPDATA%\Whisper` sont absentes. GPU réel et fenêtre Windows restent également **NOT RUN** ; aucune validation produit native n'est déduite des tests contrôlés. Neuf événements Temp observés pendant la reprise ont été consignés `UNATTRIBUTED` selon la règle permanente ; leur origine reste inconnue et ils ne deviennent pas des écritures de campagne autorisées. Aucune autre violation de garde n'a été signalée.

Les 35 petits fichiers de `logs/` (370 725 octets) ont été copiés dans `target/parakeet-tests/refacto-parakeet-live/archive/closure-20261031/logs/` en conservant l'arborescence ; nombre, chemins relatifs, tailles et SHA-256 correspondent aux originaux. Les originaux, le cache partagé et les fixtures sont conservés : aucune suppression sans manifeste approuvé. Espace libre C: relevé à **162,71 Gio** avant commit.

Après la proposition de clôture limitée aux dix fichiers Rust et au présent bilan, l'utilisateur a répondu dans cette session : « ok commit et push ». Cet accord autorise le commit local de ces onze chemins et le push de la branche courante `evo-startup-perf` vers son amont `origin/evo-startup-perf`, après contrôle de l'état distant. Les dossiers non suivis `docs/architecture/`, `models/` et `sound example/` restent hors livraison.

**Statut de réalisation : CLEAN pour le périmètre du plan**, avec les limites de validation native et de suite globale détaillées ci-dessus.
