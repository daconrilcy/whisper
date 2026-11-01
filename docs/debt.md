Avancée : dette des tests mixtes identifiée dans le lot tests autonomes.
Blocage : aucun pour le classement natif des scénarios entiers ; extraction différée.
Décision : conserver les assertions et les tests entiers dans ce lot, sans les découper.

# Dette des tests mixtes

## DEBT-1 — Reprise L1B et ressources réelles

Test : `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution`, dans `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs`.

Le scénario vérifie la conservation de l'identité durable et la résolution du chemin d'exécution, mais exige aussi `WHISPER_L1B_RUNTIME_ROOT/models/ggml-large-v3-turbo.bin` et `sound example/spkr0.wav`. Le lot actuel conserve tout le test dans la catégorie native. Une évolution ultérieure devra extraire le contrat synthétique de reprise/identité dans une fixture temporaire indépendante, en conservant séparément la preuve avec modèle et corpus réels. Elle devra démontrer la conservation des assertions et faire approuver le changement du nombre de tests ; aucune extraction n'est autorisée ici.

## DEBT-2 — Liaison historique et catalogue réel

Test : `known_historical_turbo_path_rebinds_to_the_verified_test_catalog_path`, dans `crates/whisper-adapters/tests/model_catalog.rs`.

La liaison au modèle vérifié du catalogue et le refus d'un chemin inconnu partagent un scénario dépendant du catalogue réel sous `models/`. Le lot actuel conserve le test entier dans la catégorie native. Extraire ultérieurement le refus de chemin inconnu et les invariants purement synthétiques, en gardant la vérification du catalogue réel et toutes les assertions. Aucun découpage n'est réalisé dans ce lot.

## DEBT-3 — Doublon R11 et inférence Turbo

Test : `r11_duplicate_import_prompts_before_queue_and_replace_keeps_all_old_archives`, dans `crates/whisper-adapters/tests/import_cpu.rs`. Extraire ultérieurement le flux synthétique de doublon, remplacement et conservation des archives. Le scénario actuel exige le modèle Turbo réel et un worker CPU ; il reste entier en natif.

## DEBT-4 — WAV long et métriques d'inférence

Test : `generated_long_wav_runs_eight_full_native_windows_with_stage_memory_samples`, dans `crates/whisper-adapters/tests/import_cpu.rs`. La génération du WAV et les invariants synthétiques des fenêtres pourront être isolés des mesures d'inférence/mémoire exigeant modèle et worker Turbo réels. Aucun découpage dans ce lot.

## DEBT-5 — Identité MP3 et publication native

Test : `r11_mp3_content_identity_is_independent_of_filename_and_destination`, dans `crates/whisper-adapters/tests/import_mp3.rs`. Extraire l'identité, la duplication et les invariants synthétiques d'archives du scénario qui exige aujourd'hui `WHISPER_L02_MP3_FIXTURE`, `WHISPER_L01_MODEL` et un worker natif. Le test reste entier.

## DEBT-6 — Gate L04 et environnement de campagne

Test : `l04_campaign_gate_rejects_escape_and_duplicate_trigger`, dans `crates/whisper-adapters/src/archive/tests/campaign.rs`. Isoler les assertions synthétiques de gate et de défaut des racines L04 et prérequis de campagne à la compilation et à l'exécution. Les hooks et la protection release ne sont pas modifiés.

## DEBT-7 — Défauts de synchronisation PCM

Test : `periodic_pcm_sync_has_before_and_after_fault_points`, dans la section de tests de `crates/whisper-adapters/src/staging.rs`. Extraire les injections synthétiques PCM de l'orchestration et des racines L04. Le scénario actuel reste entier, avec ses prérequis de hooks.

## DEBT-8 — Purge de diagnostics et hooks L04

Test : `campaign_startup_purge_can_fail_before_deleting_old_diagnostics`, dans la section de tests de `crates/whisper-adapters/src/supervisor.rs`. Extraire la purge et l'injection synthétiques des hooks et racines L04. Conserver l'ensemble des assertions du scénario natif actuel.

## DEBT-9 — Audio artificiel long et inférence Parakeet

Deux occurrences de `fr_synthetic_long_reference_and_vad_use_same_candidate`, dans `crates/whisper-adapters/tests/parakeet_long.rs` et `crates/whisper-adapters/tests/parakeet_archive.rs`. Séparer ultérieurement les vérifications d'audio artificiel long et de frontières VAD de l'inférence Parakeet réelle. Le scénario actuel exige `PARAKEET_L2_FR_LONG_WAV`, modèle/runtime, worker CPU et DLL natives ; les deux occurrences restent entières.

Les entrées DEBT-3 à DEBT-9 reprennent les contributions statiques de l'implémenteur ; les TODO correspondants et la conservation du contenu vérifié restent à confirmer par la revue indépendante. Ces entrées décrivent la dette liée à la mission acceptée ; elles ne constituent pas un plan d'implémentation supplémentaire.
