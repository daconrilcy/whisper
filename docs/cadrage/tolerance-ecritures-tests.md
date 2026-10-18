Avancée : le gate L3a a été mesuré sur `cddd9273` sans modification du code, avec instantanés avant/après et observation des dossiers temporaires micro.
Blocage : huit tests `parakeet_archive` échouent sur le helper de racine historique ; la mesure des chemins qu'ils auraient créés après ce refus reste incomplète.
Décision : tolérer seulement les racines historiques constatées ci-dessous pour les tests existants ; tout nouveau chemin exige une nouvelle mesure et un arbitrage.

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

La garde vérifie **avant Cargo** le disque libre ≥30 Gio, les chemins canoniques de Cargo partagé et des sorties L3a/L3b, ainsi que `TEMP/TMP` dans la racine du lot ; un cas volontairement hors racine doit être rejeté. Avant et après chaque campagne, elle compare l'inventaire des fichiers et dossiers de `target/`, de `models/`, de `sound example/`, de `spike02/`, du worker CPU R6 et de ses DLL. Dans `target/`, seules les racines Cargo partagé, lot courant et les familles historiques du tableau sont admises ; un autre emplacement nouveau ou modifié est **FAIL immédiat**. Les familles historiques restent réservées aux tests existants ; tout nouveau code L3 écrit exclusivement dans sa racine de lot. Le contrôle de l'environnement et des chemins surveillés refuse toute écriture hors `target/`, dont `%TEMP%` hors lot. Les dossiers transitoires doivent être observés pendant l'exécution, en plus des instantanés avant/après ; le seul instantané final les manquerait. Aucun répertoire de `models/`, `sound example/`, `spike02/`, du worker CPU R6 ou de ses DLL ne bénéficie d'une tolérance.

La mesure ne permet pas d'attribuer avec certitude à ce processus toutes les modifications de métadonnées anciennes, ni d'observer une écriture arbitraire ailleurs sur le volume. La garde agit à échec fermé sur les destinations qu'elle contrôle ; si une écriture extérieure est observée ou si la couverture des chemins ne peut pas être démontrée, la campagne est **FAIL**, sans extension tacite de la liste.
