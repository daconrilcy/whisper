Avancée : la garde à périmètre figé a passé ses quatre auto-tests et le helper L3a-0 accepte la racine L3a canonique.
Blocage : la nouvelle mesure a été arrêtée sur une écriture historique hors `target/` ; la seconde commande n'a pas été lancée.
Décision : aucune nouvelle tolérance n'est ajoutée et L3a reste en attente de l'arbitrage utilisateur sur ce FAIL.

# Tolérance des écritures des tests existants — référence du 18/10/2026

## Campagne de référence

HEAD `cddd92733b5c6b8c9dfbde55b97ae77b02af2bb6`, branche `evo-startup-perf`, fichiers suivis propres avant/après. `models/` et `sound example/` étaient non suivis et n'ont pas été modifiés. Début `2026-10-18T07:38:15Z`, fin `2026-10-18T07:43:08Z`. Espace libre avant : **145,12 Gio**. Garde PowerShell : cas OK accepté ; cas KO (`TEMP` hors L3a) rejeté avant Cargo. `CARGO_TARGET_DIR=target/parakeet-tests/cargo-shared/`, `TEMP/TMP=target/parakeet-tests/l3/l3a/{temp,tmp}/`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`.

Commandes exécutées sur le HEAD inchangé :

```powershell
cargo test --offline --locked --workspace --exclude whisper-desktop --no-fail-fast
cargo test --offline --locked -p whisper-desktop --no-run
```

La première commande : **exit 101**, huit FAIL du seul `crates/whisper-adapters/tests/parakeet_archive.rs`, tous à `archive_root():66` : l'archive de campagne sous L3a n'est pas acceptée par le helper historique. Les autres cibles sont allées à leur terme. La seconde : **exit 0** ; les tests desktop ont été compilés, pas exécutés. La correction documentaire L3a prévoit le helper dans un chemin déjà ouvert, sans modifier la règle produit. Ce FAIL empêche de prétendre que la liste ci-dessous couvre les branches de test situées après ces huit refus.

Instantanés des fichiers et dossiers de `target/` (chemin, création, modification UTC et taille) : `target/parakeet-tests/l3/l3a/logs/target-before.csv`, `target-after.csv`, `target-changed-items.csv`, `target-changed-dirs.txt`. Journaux : `workspace-excluding-desktop.log`, `desktop-no-run.log`, `guard-ok.txt`, `guard-ko.txt`. Le diff brut contient 2 293 éléments et 465 dossiers parents, dont 2 026 éléments du Cargo partagé et 121 du lot L3a. Des métadonnées d'anciens lots L1/L2 portent des dates **antérieures au début de la campagne** ; elles ne prouvent aucune écriture actuelle et n'entrent pas dans la tolérance. Après filtrage par date de création ou de modification comprise dans la campagne, les emplacements historiques hors Cargo partagé/L3a sont ceux du tableau.

Une observation complémentaire des créations de dossiers durant les onze tests `direct_live::tests::*` (tous PASS) a capturé trois racines temporaires qui ne subsistent pas nécessairement après exécution. Son journal exact est `target/parakeet-tests/l3/l3a/logs/direct-live-created-events.txt` ; la commande supplémentaire est `cargo test --offline --locked -p whisper-adapters --lib direct_live::tests:: -- --nocapture`.

## Dossiers historiques constatés

Les suffixes PID/horodatage changent à chaque exécution. Les chemins concrets de la campagne figurent ci-dessous ; la tolérance porte sur **ces familles créées par les tests nommés**, et uniquement pendant les suites workspace existantes. Elle ne donne aucun droit d'écriture au nouveau code L3.

| Dossier constaté sous `target/` | Test et source qui l'écrivent | Observation |
| --- | --- | --- |
| `gpu-selection-tests/` | Tests ci-dessous ; `crates/whisper-adapters/tests/gpu_settings.rs:19–27` et `crates/whisper-adapters/src/gpu_inventory.rs:809–819` | Dossier parent modifié ; deux familles historiques distinctes. |
| `gpu-selection-tests/gpu-inventory-108052-1792309237463257400-not-a-directory/`, avec sous-dossier `entry.jsonl/` | `inventory_log_write_failure_is_reportable_without_panicking`, `crates/whisper-adapters/src/gpu_inventory.rs:773–790` | Créés pendant la première suite. |
| `gpu-selection-tests/interleaved-111524-1792309283373045600/`, avec `output-updated/` | `interleaved_path_microphone_and_gpu_edits_preserve_one_latest_snapshot`, `crates/whisper-adapters/tests/gpu_settings.rs:141–180` | Créés pendant la première suite. |
| `gpu-selection-tests/migration-111524-1792309283417168400/`, avec `audio/` et `output/` | `old_json_migrates_additively_and_reopens_with_revision_acknowledgement`, `crates/whisper-adapters/tests/gpu_settings.rs:62–139` | Créés pendant la première suite. |
| `gpu-selection-tests/write-failure-111524-1792309283435946100/` | `write_failure_is_attributed_to_revision_and_reported`, `crates/whisper-adapters/tests/gpu_settings.rs:181–205` | Créé pendant la première suite. |
| `gpu-selection-tests/gpu-inventory-*-append.jsonl` et `gpu-inventory-*-dxgi-unjoined.jsonl` (fichiers dans le parent, pas des sous-dossiers) | `inventory_log_appends_timestamped_partial_snapshots_and_escapes_names` et `inventory_log_keeps_dxgi_adapters_that_failed_identity_join`, `crates/whisper-adapters/src/gpu_inventory.rs:729–808` | Fichiers créés ; leur dossier parent est la première ligne. |
| `direct-live-tests/` | `wav_output_keeps_the_exact_converted_samples`, `stop_before_engine_ready_flushes_captured_audio_to_the_wav`, `wav_backlog_fills_bounded_windows_without_drops_or_reordering`, `simulated_gpu_failure_keeps_wav_and_confirmed_text_with_incomplete_suffix`, `microphone_interruption_finalizes_cpu_outputs_without_gpu_message` ; `crates/whisper-adapters/src/direct_live.rs:1384–1614` | Dossier parent modifié pendant la première suite. |
| `direct-live-auto-startup-19852/` avec `.whisper-diagnostics/` et dossiers temporaires `rmeta*`, `rustc*` | `auto_microphone_falls_back_while_capture_is_open_and_keeps_auto_preference`, `crates/whisper-adapters/src/direct_live.rs:1175–1276` | Création capturée par observation complémentaire. |
| `direct-live-no-gpu-worker-19852/` avec `.whisper-diagnostics/` et dossiers temporaires `rmeta*`, `rustc*` | `auto_without_gpu_worker_clears_candidate_and_accepts_cpu_ready`, `crates/whisper-adapters/src/direct_live.rs:1277–1316` | Création capturée par observation complémentaire. |
| `direct-live-stop-before-ready-19852/` avec `.whisper-diagnostics/` et dossiers temporaires `rmeta*`, `rustc*` | `auto_microphone_stop_before_gpu_ready_keeps_cpu_retry_for_recorded_audio`, `crates/whisper-adapters/src/direct_live.rs:1317–1362` | Création capturée par observation complémentaire. |

`target/parakeet-tests/cargo-shared/` est la racine de compilation autorisée ; `target/parakeet-tests/l3/l3a/` est la racine du lot et de cette mesure. Ces deux racines ne sont **pas** des exceptions historiques. L'empreinte de présence/mtime relevée après la suite ne montre aucune écriture à partir du début de campagne dans `models/`, `sound example/`, `target/parakeet-feasibility/spike02/`, le dossier du worker CPU R6 ni `target/parakeet-tests/l2/l2b/native/`. Aucun nouveau fichier suivi par Git.

## Application de la tolérance aux campagnes suivantes

La garde vérifie **avant Cargo** le disque libre ≥30 Gio, les chemins canoniques de Cargo partagé et des sorties L3a/L3b, ainsi que `TEMP/TMP` dans la racine du lot ; un cas volontairement hors racine doit être rejeté. Avant et après chaque campagne, elle compare l'inventaire des fichiers et dossiers de `target/`, de `models/`, de `sound example/`, de `spike02/`, du worker CPU R6 et de ses DLL. Dans `target/`, seules les racines Cargo partagé, lot courant et les familles historiques du tableau sont admises ; un autre emplacement nouveau ou modifié est **FAIL immédiat**. Les familles historiques restent réservées aux tests existants ; tout nouveau code L3 écrit exclusivement dans sa racine de lot. Le contrôle de l'environnement et du périmètre de surveillance figé ci-dessous refuse toute écriture hors `target/` qu'il observe, dont le TEMP utilisateur réel hors lot. Les dossiers transitoires doivent être observés pendant l'exécution, en plus des instantanés avant/après ; le seul instantané final les manquerait. Aucun répertoire de `models/`, `sound example/`, `spike02/`, du worker CPU R6 ou de ses DLL ne bénéficie d'une tolérance.

## Arbitrage L3-03 et garde du 18/10/2026

La décision utilisateur remplace l'exigence de preuve générale par un périmètre de détection **figé**. La garde PowerShell d'exécution est `target/parakeet-tests/l3/l3a/guard-campaign.ps1` ; elle arme des `FileSystemWatcher` récursifs **avant** le processus Cargo, traite créations, modifications, suppressions et renommages, puis les arrête **après** sa fin. L'événement interdit provoque `taskkill /T /F` sur l'arbre de processus Cargo et un résultat FAIL avec horodatage, type et chemin dans `logs/guard-<commande>.json`. Une erreur de surveillance est elle-même un FAIL. Le script et les journaux sont sous la racine d'exécution L3a, hors Git.

Périmètres surveillés : dépôt `C:\dev\whisper` en excluant seulement `target/parakeet-tests/cargo-shared/`, la racine L3a et les familles historiques du tableau pour les tests existants ; `models/`, `sound example/`, `target/parakeet-feasibility/spike02/`, `target/parakeet-tests/l2/l2b/r6/worker/` et `target/parakeet-tests/l2/l2b/native/` sans exclusion ; `%LOCALAPPDATA%\Temp` réel, distinct de `TEMP/TMP` de campagne ; réglages `%LOCALAPPDATA%\Whisper` et diagnostics `<destination>\.whisper-diagnostics`. `root.rs:153–160` construit les réglages avec `WindowsPathPort`, dont le chemin est défini par `path_settings.rs:28–34`. `root.rs:153–160` utilise `ui::default_destination()` (`ui.rs:2020–2032`) ou la destination de `settings.json` ; `supervisor.rs:152–154` place les diagnostics sous cette destination. Sur cet hôte, la destination enregistrée est `C:\Users\cyril\Documents\WhisperTranscriptions`, donc les diagnostics surveillés sont `C:\Users\cyril\Documents\WhisperTranscriptions\.whisper-diagnostics`. Le journal GPU `gpu_inventory.rs:301–308` et le journal de réactivité `reactivity_log.rs:129–131` relèvent aussi de `%LOCALAPPDATA%\Whisper`.

Les écritures ailleurs sur le volume sont un **risque résiduel accepté** par l'utilisateur. La redirection systématique des tests historiques demeure L6-perso P7. La garde vérifie les variables de campagne avant Cargo ; les SHA du modèle Parakeet, du worker R6 et des cinq DLL CPU sont comparés à [la fiche R6](fiche-parakeet-r6.md) avant et après la campagne. Les dossiers protégés ne sont jamais utilisés pour les auto-tests.

### Auto-test de la garde

Quatre processus factices écrivent chacun sous `target/parakeet-tests/l3/l3a/guard-selftest/`. Les trois premiers sous-dossiers sont classés artificiellement comme protégés par la même fonction de décision ; le quatrième est autorisé. Résultat final consigné dans `guard-selftest/results.json` :

| Témoin | Attendu | Observé | Arbre du processus |
| --- | --- | --- | --- |
| `protected/` | FAIL | FAIL, événement `Changed` | Arrêté avant ses 30 s d'attente |
| `outside-target/` | FAIL | FAIL, événement `Changed` | Arrêté avant ses 30 s d'attente |
| `temp-real/` | FAIL | FAIL, événement `Changed` | Arrêté avant ses 30 s d'attente |
| `allowed/` | PASS | PASS, aucun événement interdit | Terminé normalement |

Ces noms représentent des **classes simulées** ; les quatre chemins physiques sont sous L3a. La garde a été reprise après un premier arrêt dû à un événement de renommage à ancien chemin vide, puis l'auto-test complet a été rejoué avec succès. Le résultat final est celui du journal `guard-selftest/results.json`.

### Reprise de la mesure après L3a-0

Sur le HEAD `783273cb`, le seul code source modifié est `crates/whisper-adapters/tests/parakeet_archive.rs::archive_root()` : `PARAKEET_L3A_EXEC_ROOT` doit se résoudre exactement vers `target/parakeet-tests/l3/l3a`, et l'archive canonique doit être dessous. Les branches R6/R7/R8/R9 restent inchangées. Le préflight relève **145,23 Gio** libres et les sept SHA modèle/worker/cinq DLL conformes à la fiche R6.

La première tentative gardée a été interrompue par un événement de renommage à chemin ancien vide que la garde avait interprété comme la racine du dépôt ; elle a été corrigée dans le script d'exécution, et les quatre auto-tests ont été rejoués. La nouvelle tentative de `cargo test --offline --locked --workspace --exclude whisper-desktop --no-fail-fast` a démarré à `2026-10-18T08:44:40Z` et a été **arrêtée à 08:44:42Z** : événement `Changed` sur `C:\dev\whisper\crates\whisper-adapters\archive\.whisper-diagnostics\worker-0.jsonl`. `logs/guard-Adapters.json` donne le chemin, l'horodatage et `terminated=true` ; `logs/Adapters.stdout.log` et `.stderr.log` montrent les tests exécutés avant l'arrêt. Ce dossier n'est pas dans la tolérance et se trouve **hors `target/`**. Il contenait déjà un fichier, dont le `LastWriteTimeUtc` a changé pendant la campagne. La cause source est `worker_ipc.rs:7882–7891` (`test_request()` fixe `destination: "archive"`), transmis à `worker_ipc.rs:2315` (`start_diagnostics`) ; `supervisor.rs:152–154` écrit `.whisper-diagnostics/worker-0.jsonl` relativement au répertoire de la crate. Plusieurs tests `worker_ipc` utilisent cette requête ; leur exécution parallèle ne permet pas d'attribuer avec certitude l'événement à un seul test. Les tests concernés incluent `auto_without_gpu_starts_cpu_with_normalized_compute_and_same_identity` (`worker_ipc.rs:6409–6450`) et les tests JobId(709) (`worker_ipc.rs:6454–6482`, `:6528–6555`).

**La seconde commande** `cargo test --offline --locked -p whisper-desktop --no-run` est **NOT RUN**, conformément à l'ordre d'arrêt. Les huit tests `parakeet_archive` n'ont pas été atteints dans cette tentative : leur résultat après la correction L3a-0 reste **NOT RUN**, et la mesure complète reste **FAIL**. Aucun chemin n'est ajouté au tableau de tolérance. Les SHA après arrêt sont identiques aux sept SHA du préflight (`logs/protected-sha-before.json` et `logs/protected-sha-after.json`). Aucune correction de `worker_ipc.rs` ni suppression du fichier historique n'a été entreprise.
