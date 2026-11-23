Avancée : les findings du second cycle sont intégrés au périmètre, à l’écran actif et à la matrice de validation.
Blocage : l’effet système Windows et l’ajout du package-set dans la garde exigent un accord précis avant ces tâches.
Décision : plan livré avec réserves après deux cycles de revue/correction ; il ne vaut pas GO code.

# Plan — refonte de l’interface du transcripteur

## 1. Autorité, base et limite

- Auteur : session Codex principale. Version 3, 23 novembre 2026 (corrections des cycles 1 et 2 ; réserves de la seconde revue conservées).
- Parent : [design-ui-transcripteur-maquette.md](design-ui-transcripteur-maquette.md), DESIGN v2 proposé ; exigences REQ-01–12, critères AC-01–11, décision technique Q-01.
- Brief source : [brief-ui-transcripteur-maquette.md](brief-ui-transcripteur-maquette.md).
- Base inspectée : commit `540aebf77b1c0c0f423c08285c09816dea7176a2`, branche `codex/ui-transcriber-mockup`. Vérifier à nouveau avant exécution. Le changement préexistant à `AGENTS.md` n’est pas dans le périmètre.
- Aucune implémentation, campagne Cargo, revue indépendante, build ou validation fenêtre n’est autorisée/réalisée par ce plan.

## 2. Périmètre futur et chemins

Chemins relatifs à `crates/` sauf mention. Ces chemins sont les propriétaires proposés après inspection des contrats actuels ; toute extension non listée ou tout format alternatif exige une révision du plan avant modification.

| Groupe | Chemins envisagés | Responsabilité |
|---|---|---|
| D-UI | `whisper-desktop/src/ui/model/view.rs`, `view/active_job.rs`, `view/unified.rs`, `view/actions.rs`, `view/settings.rs`, `view/microphone.rs`, `view/capture.rs`, `view/queue.rs`, `view/history.rs`, `view/dialogs.rs`, `view/notices.rs` | Composition repos/actif/finalisation, parcours et présentation |
| D-controller | `whisper-desktop/src/ui/model/controller/actions.rs`, `controller/observability.rs`, `ui/model.rs`, `ui/model/preferences.rs`, `root.rs` | Intention, admission et injection d’effets |
| D-domain | `whisper-core/src/application/contracts.rs`, `domain.rs`, `ports.rs`, `path_settings.rs`, `application/runtime/queue.rs`, `runtime/import.rs`, `runtime/events.rs`, `runtime/poll.rs`, `runtime/closing.rs`, tests core correspondants | Intention durable, sélection/séquencement, événements terminaux, fermeture, préférences de chemins |
| D-adapter | `whisper-adapters/src/queue_store.rs`, `recovery.rs`, `worker_ipc/runtime/queue.rs`, `worker_ipc/runtime/import.rs`, `path_settings.rs`, nouveau `open_folder.rs`, tests adapters correspondants | Journal compatible, exécution, récupération, réglages et effet Windows |
| D-CI | `ci/guard-level1.ps1` | Enregistrer le lot dédié dans la politique `-p` sans modifier les profils existants, après accord précis sur ce changement de gate |
| D-tests | `whisper-desktop/src/ui/tests.rs`, `ui/tests/`, tests `whisper-core` et `whisper-adapters` des propriétaires modifiés | Tests d’interaction, migration du journal et transitions persistées |

Le journal `queue.jsonl` et le snapshot de préférences sont étendus de façon additive. Les nouveaux binaires relisent les anciens enregistrements ; le downgrade d’un journal contenant de nouveaux états terminaux n’est pas garanti et sera signalé. Pas de migration destructive, dépendance nouvelle, crate ou stockage alternatif. Q-01 propose un port/adaptateur Windows précis ; son effet système n’est pas autorisé. DU-07 reste dans le périmètre et bloque la réalisation jusqu’à accord. L’API d’application et les événements sont ajustés pour porter intention, slot et états terminaux ; aucun modèle, moteur ou protocole worker n’est modifié.

## 3. Lots et dépendances

Neuf tâches, sous la limite commune de dix. Une seule responsabilité d’écriture à la fois ; les chemins UI partagés ne sont pas parallélisés.

| Lot | Tâches | Dépend de | Sortie |
|---|---|---|---|
| L0 — Préflight | T1 | design/plan acceptés, état Git et accords précis d’implémentation et de gate | Q-01 et l’enregistrement de la policy autorisés avant leur périmètre de code |
| L1 — Contrats durables | T2–T4 | L0 | file/intention/migration et préférences disponibles avec tests propriétaires |
| L2 — Parcours UI | T5–T7 | L1 | composition, paramètres, import/file, historique et résultat raccordés |
| L3 — Vérification/remise | T8–T9 | L1–L2 | N1/Clippy, revue indépendante et fenêtre observée par l’utilisateur |

Graphe : **T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9**. Aucun partage d’écriture concurrente. Chaque lot reste sous dix tâches. Les bilans futurs ne dépassent pas dix fichiers. L’ensemble des documents de cadrage est limité au design et au plan.

## 4. Tâches

| ID | Travail | Critères de sortie |
|---|---|---|
| **T1** | Préflight : vérifier branche/état Git, seuil disque, règles root/desktop, contrats `queue.jsonl`, fermeture, archive et settings ; fixer la matrice de migration des lignes historiques. Arrêter si des modifications hors des chemins ci-dessous deviennent nécessaires. | Accords précis code, Q-01 et gate avant travaux concernés ; aucune Cargo. Aucun ancien import ne doit être automatiquement promu à « calcul demandé ». |
| **T2** | Core : `runtime/queue.rs` porte intent et sélection FIFO ; `runtime/import.rs` transforme ProcessQueued en demande/intention et n’appelle le port de calcul que lorsque le slot est libre ; `runtime/events.rs`/`runtime/poll.rs` projettent les événements terminales et demandent leur persistance ; `runtime/closing.rs` suspend les intents au lieu de les supprimer. Admission et doublon acceptés pendant un travail actif. | REQ-02–06 ; AC-01–05. `Published` établit succès, `Stopped` arrêt confirmé, `Failed` échec/récupération. Cœur seul décide du slot, avec au plus un calcul et aucun conflit micro/import. |
| **T3** | Adapter queue journal : `queue_store.rs`, `recovery.rs`, `worker_ipc/runtime/queue.rs` et `runtime/import.rs` portent mutations durables, lancement du prochain intent et reprise. Upsert étendu avec intent/ordre et états terminaux ; conservation des anciennes lignes/tombstones. | REQ-04–06 ; AC-03–05. Ancien `Queued/AwaitingChoice` sans intent reste manuel ; ancien Running devient Interrompu avec confirmation ; aucune suppression métier ni auto-reprise. Status final persistant suit l’événement terminal. |
| **T4** | Réglages core/adaptateur : ajouter le dossier initial audio comme préférence distincte de `audio_directory` (dernier dossier utilisé), charger les anciens réglages sans changement et appliquer la priorité dernière valeur valide → initiale valide → secours. | REQ-08 ; AC-07. Tester absence/chemin disparu, persistance et migration additive sans écraser modèle/micro/GPU/gains. |
| **T5** | Après accord écrit précis Q-01, ajouter `FolderOpenerPort`, `whisper-adapters/src/open_folder.rs`, injection `whisper-desktop/src/root.rs` et commande UI. Vérifier le parent puis lancer `explorer.exe` avec `std::process::Command`, chemin en argument sans shell ; remonter erreur si parent absent ou processus non lancé. | REQ-07 ; AC-06. Fake port prouve parent exact ; vérification Windows succès/échec ; vue ne lance aucun processus. Si Q-01 manque, STOP de la réalisation : liens confirmés non retirés du périmètre. |
| **T6** | Composer l’interface repos/active/finalisation/résultat dans `view.rs`, `active_job.rs`, `capture.rs` et `unified.rs` ; inclure texte confirmé/provisoire, progression et arrêt toujours accessible. | REQ-01/10/12 ; AC-09/10. Distinguer capture historique, traitement actif et session unifiée ; états et action d’arrêt visibles au mode compact. |
| **T7** | Adapter façade, paramètres, import, doublons, file et historique ; projeter les états durables, liens et destination ; spectre à 64 bandes et détail avant/après. | REQ-01–09/11/12 ; AC-01–08/10. Commandes renvoient des identités confirmées ; import seul non schedulé ; choix d’admission/doublon préservé ; accessibilité visuelle. |
| **T8** | Après accord précis sur ce changement de gate, inscrire le lot et son exact package-set dans `ci/guard-level1.ps1` ; exécuter SelfTest puis la matrice §6 depuis worktree dédié. Aucun profil déjà autorisé n’est modifié. | Tous AC automatisables ; commandes §6 ; SelfTest avant Cargo, contrôles PE avant suites worker, N1 obligatoire et cliquet Clippy actif. Aucun Cargo direct, niveau 2 ou recette native. |
| **T9** | Revue du candidat par `rust_review_coordinator`, corrections via `rust_implementer`, deux cycles maximum ; répéter tests affectés. L’utilisateur observe ensuite la fenêtre Windows. | Tous AC ; CLEAN ou LIVRÉ AVEC RÉSERVES. Bilan futur ; commits locaux sur branche de lot si autorisés, aucun push. Fenêtre testée 1536×864, 1024×720, minimum, 100/150/200 %. |

La fermeture actuelle retire toutes les entrées non Running ; elle doit être remplacée par une mutation durable Interrompu/suspendu pour AC-04. L’admission actuelle échoue pendant direct-live et le doublon échoue pendant une activité ; T2/T3 les corrigent sans démarrer un import concurrent. `runtime/import.rs` lance actuellement ProcessQueued immédiatement et devient le raccord au scheduler. `ImportEvent::Published` dans ce module établit le succès publié ; `Stopped` confirme l’arrêt, `Failed` l’échec/récupération, puis le statut terminal est persisté par le contrat queue. `QueueStore` journalise append-only mais Remove ajoute un tombstone ; T3 remplace l’effacement métier par upsert Annulé et garde les tombstones historiques lisibles. Les anciennes entrées Queued/AwaitingChoice sans intent restent manuelles ; les Running historiques deviennent Interrompu avec confirmation.

Les états de l’écran actif doivent réutiliser et recomposer les vues actuelles `active_job.rs`, `capture.rs` et `unified.rs` ; ce n’est pas un simple état illustratif. La configuration des dossiers conserve le `audio_directory` comme dernier audio choisi et ajoute un champ initial distinct.

## 5. Matrice tâches / critères / chemins

| Critères | Tâches | Chemins principaux |
|---|---|---|
| AC-01/02, REQ-02/03 | T2/T3/T7/T8 | contrats, queue journal, `view/actions.rs`, `view/dialogs.rs` |
| AC-03/04/05, REQ-04/05/06 | T2/T3/T7/T8 | runtime core, adapter queue journal, `view/queue.rs` |
| AC-06, REQ-07 | T5/T7/T8 | port/adaptateur Windows, `view/history.rs` |
| AC-07, REQ-08 | T4/T7/T8 | core/adapters `PathSettingsApp`, `settings.json`, `view/settings.rs` |
| AC-08, REQ-09 | T7/T8 | `view/microphone.rs`, projection de spectre et tests synthétiques |
| AC-09/11, REQ-10/11 | T6/T7/T8 | `active_job.rs`, `capture.rs`, `unified.rs`, publication/runtime existants |
| AC-10, REQ-01/12 | T6/T7/T9 | `view.rs`, vues concernées, observation Windows |

## 6. Vérifications futures et limites

- Worktree : créer hors du dépôt principal, depuis le commit de livraison du plan, sous `C:\wt\ui-transcriber-impl`; confirmer branche, racines canoniques, jonctions et ≥30 Gio avant exécution. Les écritures restent dans les racines déterminées par la garde.
- Shell/cwd : PowerShell, cwd `C:\wt\ui-transcriber-impl`. SelfTest obligatoire avant tout Cargo. Les variables, `CARGO_HOME` persistant `target/cargo-home-shared/`, TEMP/TMP et cible de compilation passent par la garde ; aucun `CARGO_TARGET_DIR` environnemental.
- T8 ajoute uniquement un package-set autorisé pour `target/parakeet-tests/ui-transcriber-impl` dans la table existante de `ci/guard-level1.ps1`, après accord précis. Membres whitelistés : `whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common`, `whisper-worker-cpu`, `whisper-worker-parakeet-cpu`, `whisper-worker-gpu`. Le profil GPU reste `--list` seulement en N1. SelfTest vérifie le refus de package/member non autorisé avant Cargo.

| ID | Arguments Cargo passés à la garde | Résultat attendu |
|---|---|---|
| V1 | `fmt --check` | Format des crates modifiées conforme |
| V2 | `test --no-fail-fast`, invocation garde avec `-WithoutCargoTargetDir` | N1 obligatoire ; absence de variable `CARGO_TARGET_DIR`, cible fixée par `--config` de la garde |
| V3 | `test --offline --locked -p whisper-worker-cpu`; répéter pour `whisper-worker-parakeet-cpu` et `whisper-worker-common` | Suites N1 de référence ; contrôle PE des harnais avant chaque sélection |
| V4 | `test --offline --locked -p whisper-worker-gpu -- --list` | Inventaire seulement ; aucun corps GPU exécuté en N1 |
| V5 | `test --offline --locked -p whisper-core`; répéter pour `whisper-adapters` et `whisper-desktop --lib --tests` | Transitions/persistance/UI concernées ; members whitelistés par T8 |
| V6 | `test --offline --locked -p whisper-desktop --lib --tests --features personal-install` | Configuration desktop personnelle si la feature existe toujours au préflight |
| V7 | `clippy --offline --locked --workspace --all-targets --message-format=json -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity` | Active `Test-StructuralClippyInvocation` et le cliquet baseline ; aucune nouvelle alerte |
| V8 | `build --offline --locked --workspace` | Composition workspace compile si autorisée et capacité disque suffisante |
| V9 | `git diff --check` (hors Cargo) | Aucun défaut d’espacement |

Exemples (tous **NOT RUN** ; PowerShell, cwd dédié) :

| Usage | Commande proposée |
|---|---|
| SelfTest | `& .\ci\guard-level1.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/ui-transcriber-impl` |
| Format | `& .\ci\guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/ui-transcriber-impl -CargoArgsJson '["fmt","--check"]'` |
| N1 obligatoire | `& .\ci\guard-level1.ps1 -Mode Run -WithoutCargoTargetDir -LotRoot target/parakeet-tests/ui-transcriber-impl -CargoArgsJson '["test","--no-fail-fast"]'` |
| Worker sélectionné (répété pour les trois workers CPU) | `& .\ci\guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/ui-transcriber-impl -CargoArgsJson '["test","--offline","--locked","-p","whisper-worker-cpu"]'` |
| Clippy | `& .\ci\guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/ui-transcriber-impl -CargoArgsJson '["clippy","--offline","--locked","--workspace","--all-targets","--message-format=json","--","-W","clippy::too_many_lines","-W","clippy::cognitive_complexity","-W","clippy::too_many_arguments","-W","clippy::type_complexity"]'` |

Les sélections `-p` refusées par la garde arrêtent T8 ; aucune commande Cargo directe ne les remplace. L’ajout du package-set whitelisté est le changement de gate précis proposé, sous accord utilisateur. `-DryRun` valide les profils avant leur exécution réelle, sans remplacer SelfTest.
- Comparer les commandes/effets CPU/GPU/Auto et les parcours micro/import avant/après. Ne pas exécuter les recettes natives suspendues.
- Fenêtre Windows par l’utilisateur : accueil, import immédiat/différé, doublon/annulation, file, liens, configuration, capture micro, publication, tailles 1536×864 et 1024×720, minimum 960×640, échelles 100/150/200 %. Résultat NOT RUN jusqu’à observation.
- Les tests d’interface ne prouvent pas la qualité de transcription. Le critère de fin porte sur l’existence et l’accessibilité des trois sorties au moment où l’application annonce la réussite ; aucune atomicité externe n’est requise.

## 7. Prérequis d’autorisation et readiness du plan

Le cadrage/plan ne donne pas GO d’implémentation. Avant T1/T2, il faut revue DESIGN/PLANS acceptables et accord utilisateur couvrant le lot, crates et fichiers. Avant T5 : accord écrit précis sur l’effet système Windows proposé en Q-01. Avant T8 : accord écrit précis sur le package-set whitelisté et changement limité de `ci/guard-level1.ps1`. Les extensions hors chemins listés, dépendances ou format durable alternatif exigent arbitrage/révision. Chaque lot Rust est délégué à `rust_implementer` en mode IMPLEMENT ; la revue post-implémentation est coordonnée par `rust_review_coordinator` selon AGENTS.

**Readiness PLANS : LIVRÉ AVEC RÉSERVES après deux cycles de revue/correction.** L’effet Q-01 et le changement précis de gate T8 restent à approuver avant leur exécution ; aucun comportement confirmé du brief n’est retiré. Aucun code, test ou validation visuelle n’a été exécuté.

**Clôture de revue :** cycle 1 FINDINGS, corrections v2 ; cycle 2 FINDINGS, corrections v3 appliquées par l’auteur aux UI-R01/03/04/05 et UI-R02 confirmé fermé. La limite de deux cycles est atteinte : aucune troisième revue indépendante n’est lancée ; ces corrections finales ne sont donc pas certifiées CLEAN. Livraison avec réserves et décision utilisateur requise.
