Avancée : le candidat L2a-2 fonctionne en FR/EN CPU et ses corrections R1 à R5 sont fermées.
Blocage : R6 à R8 restent ouverts faute de scénarios natifs complets ; L2a-2 est LIVRÉ AVEC RÉSERVES.
Décision : proposer un lot distinct de fermeture des preuves A6, A7 et A5, sans démarrer L2b.

# PLAN-PARA-L2a-2-ACCEPT v1 — fermeture R6 à R8

**Statut : CLEAN — PLANS READY-L2a-2-ACCEPT, sans autorisation de code.** Revue indépendante du commit `4007e52b` : aucun REQUIRED, un advisory d'exécution R8 (distinguer refus de qualification avant lancement et erreur ABI/exports atteinte par le chargeur). Auteur : session Codex principale. Base : `PLAN-PARA-L2 v5` et son verdict CLEAN enregistré à `ddecb546`, candidat L2a-2 non commité sur `evo-startup-perf`, verdict indépendant après trois passes et deux corrections : R1–R5 fermés, R6–R8 ouverts, `LIVRÉ AVEC RÉSERVES`. La demande utilisateur de préparer ce plan est donnée dans la session courante. Un accord écrit distinct sur ce lot et ses chemins précède toute implémentation. Le candidat L2a-2 reste intact pendant le cadrage ; son verdict antérieur n'est pas transformé rétroactivement en CLEAN.

## 1. Périmètre et preuve de départ

Le lot ferme seulement les trois constats indépendants ci-dessous sur **import Parakeet Test, CPU, FR/EN, WAV/MP3, passe unique ≤ 1 800 s**. Il utilise le worker figé sous `target/parakeet-tests/l2/l2a-2/worker/parakeet-worker.exe`, SHA-256 `545a4830755da613a1255be97fbc6a6a513ef837d669d8a9870d3bbee9dfd587`, le modèle Test SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e` et les cinq DLL CPU privées de la campagne L2a-2. A3 (20 inférences FR/EN), A11 (WAV/MP3 : 4 800 000 échantillons chacun, écart PCM maximal 0), Stop pendant l'inférence et publication réelle TXT/SRT sont des preuves déjà acquises **pour cette clé** ; elles ne prouvent pas les scénarios manquants.

| Constat | Critère source dans `PLAN-PARA-L2 v5` | Manque exact | Résultat exigé ici |
| --- | --- | --- | --- |
| R6 | A6 : Stop/Quit aux barrières de préparation, inférence et publication, sortie physique et état durable correct. | Préparation et publication non exécutées ; inférence déjà passée. | Deux scénarios distincts, avant et après engagement de publication, avec état visible, pending, source, sorties et fin de processus vérifiés. |
| R7 | A7 : reprise du pending/checkpoint sur identité exacte, préfixe stable, sans doublon, publication unique. | Les tests de file/archive passent, mais aucune nouvelle inférence native sur une reprise produit n'a été exécutée. | Reprise par le routeur produit jusqu'à `Complete`, comparaison des segments et publications avec le préfixe confirmé et refus des divergences avant inférence. |
| R8 | A5 : erreur attribuée pour DLL incompatible, résultat natif incomplet ou worker mort ; aucune substitution. | Ces trois entrées négatives natives/transport sont non exécutées. | Trois refus identifiés par cause et phase, source/pending préservés, aucune sortie `Complete`, aucun repli Turbo. |

R6–R8 sont des **critères d'acceptation bloquants** : un test absent, ignoré, lancé sur une autre clé ou qui ne traverse pas le routeur produit reste `NOT RUN`. Les 23 autres langues, GPU/Auto calcul Parakeet, live, VAD longue, paquet Final, qualité absolue et validation manuelle de la fenêtre Windows restent hors de ce lot. Aucune nouvelle mesure de vitesse ou de charge n'est introduite.

## 2. Tâches, chemins et ordre

| Tâche | Propriétaire | Action et preuve attendue |
| --- | --- | --- |
| F0 — préflight | Session principale, lecture seule | Relever branche/HEAD, diff suivi et non suivi du candidat, plan v5/ce plan et leurs verdicts, empreintes worker/modèle/DLL, présence des fixtures FR/EN et de la racine Test. Confirmer que la clé qualifiée n'a pas changé. Aucun nettoyage de `target/debug` : l'écart d'exécution antérieur reste signalé. |
| F1 — R6 | `rust_implementer` | Ajouter des barrières de test déterministes dans le parcours produit pour arrêter **pendant préparation** puis **pendant publication** ; exercer aussi Quit si son chemin diffère. Avant engagement : résultat tardif ignoré, pending et source conservés, aucune publication complète, sortie physique constatée hors UI. Après engagement : l'arrêt est annoncé trop tardif, la publication réelle se termine ou échoue avec ses chemins exacts ; jamais de `Cancelled` fictif ni de `Complete` partiel. Aucun délai d'attente inventé comme garantie produit. |
| F2 — R7 | `rust_implementer` | Construire un pending avec un préfixe réellement confirmé, provoquer l'interruption avant `Complete`, puis reprendre par la commande produit sur la génération suivante et lancer le worker natif. Vérifier la même source et clé qualifiée, FIFO et génération, préfixe byte-identique, aucun segment doublé, une seule paire TXT/SRT et un seul `Complete`. Les variantes SHA modèle/source audio, job, génération, FIFO et préfixe divergent sont refusées avant appel natif. |
| F3 — R8 | `rust_implementer` | Sur des copies privées sous le runtime, remplacer la DLL C par une DLL du jeu qui n'expose pas l'ABI attendue et vérifier l'erreur de chargement/exports avant audio ; tuer un worker engagé et constater sa sortie ; injecter un événement de résultat incomplet à la frontière de transport contrôlée par le test. Chaque cas doit traverser la garde et le routeur pertinents, porter une cause attribuée, laisser source/pending intacts, ne produire ni faux `Complete` ni repli Turbo. Le cas d'événement incomplet ne prétend pas modifier la bibliothèque NeMo. |
| F4 — convergence | `rust_implementer`, puis `rust_review_coordinator` | Relancer les scénarios F1–F3 sur la clé exacte, les tests de régression Turbo, format/build/diff ; rapporter chaque sous-cas `PASS/FAIL/NOT RUN`. Soumettre le candidat à un `rust_reviewer` indépendant, au plus deux cycles revue/correction **pour ce nouveau lot**. |

**Chemins d'écriture source du nouveau lot, et seulement ceux-ci :**

```text
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/src/parakeet_import.rs
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/tests/parakeet_control.rs
crates/whisper-adapters/tests/parakeet_import.rs
crates/whisper-adapters/tests/parakeet_archive.rs
```

Les trois fichiers source ne sont ouverts qu'à une correction révélée par F1–F3 ; commencer par les tests et garder les invariants Turbo et la qualification exacte. `crates/whisper-worker-parakeet-cpu/`, `whisper-core/`, `whisper-desktop/`, `Cargo.toml`, `Cargo.lock`, le protocole, le décodeur, la façade FFI et le railguard restent en lecture. Si une preuve exige de modifier l'un de ces chemins, la DLL native ou le contrat produit, arrêt `PLAN_CHANGE_REQUIRED` avant édition. Si le worker, le modèle, une DLL ou le protocole change malgré tout dans un plan révisé, G-L2A-QUAL impose une nouvelle recette FR/EN et A11 avant toute admission.

**Écritures d'exécution :** `target/parakeet-tests/l2/l2a-2/` seulement, avec sous-dossiers distincts par scénario ; copies de worker et DLL exclusivement dans cette racine, sans déplacer ni modifier la copie figée. Les copies fautives sont étiquetées par scénario et ne deviennent jamais la fiche Test. Modèles, `sound example/`, `spike02/fixtures/`, runtime L2a-1, documents utilisateur et `target/debug` restent en lecture. Pas de réseau, téléchargement, code C++, installation, service, ACL ou push.

## 3. Contrôles et décision de fermeture

Avant chaque commande Cargo, résoudre et vérifier les chemins absolus de `CARGO_TARGET_DIR`, `TEMP`, `TMP`, `WHISPER_L1B_RUNTIME_ROOT`, `PARAKEET_L2_RUNTIME_ROOT`, `PARAKEET_L2_ARCHIVE_ROOT`, `PARAKEET_L2_SETTINGS_ROOT`, `PARAKEET_L2_QUEUE_ROOT` et `PARAKEET_L2_OUTPUT_DIR` sous la racine d'exécution autorisée ; restaurer ensuite leurs anciennes valeurs, y compris l'absence de valeur. Écarter/restaurer les variables `WHISPER_L01_*`, `WHISPER_L02_*` et `WHISPER_L04_*` pendant la campagne. Le modèle, le worker, les cinq DLL et les WAV FR/EN sont fournis par chemins explicites ; une entrée absente rend son essai `NOT RUN`, sans source de secours. Ne pas utiliser un timeout d'inférence comme oracle d'arrêt ou de reprise.

Commandes proposées après candidat, avec les variables ci-dessus vérifiées et le worker figé attesté avant/après :

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-adapters --test parakeet_control -- --ignored --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_import -- --ignored --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_archive -- --ignored --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_control --test parakeet_import --test parakeet_archive --test legacy_model_requests --test model_catalog --test model_preferences
cargo test --offline --locked -p whisper-core
cargo test --offline --locked -p whisper-desktop --lib --test model_selection_ui
cargo build --offline --locked --workspace
git diff --check
git status --short
```

Les commandes `--ignored` n'annoncent aucun PASS global par leur seul code 0 : dénombrer les scénarios réellement lancés et conserver leur résultat attribué à R6, R7 ou R8. Les essais d'inférence FR/EN et A11 déjà acquis sont conservés uniquement si le worker figé et les dimensions de sa clé restent identiques. Les scénarios de ce lot sont ensuite relus avec le diff exact. **CLEAN** demande R6, R7 et R8 fermés par le reviewer, zéro faux `Complete`, zéro doublon de reprise et aucune régression Turbo ; sinon `LIVRÉ AVEC RÉSERVES` après deux cycles avec sous-cas ouverts et décision de l'utilisateur. Le lot ne donne aucun GO à L2b et ne vaut pas qualification Final.
