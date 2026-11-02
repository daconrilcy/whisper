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


## DEBT-10 — Scénarios unifiés contrôlés et garde de modèle réel

Le diagnostic V1 et le reviewer confirment que les 27 scénarios ci-dessous utilisent des données, contrôleurs ou workers factices mais traversent une préparation qui lit metadata et SHA-256 d’un modèle enregistré réel. Ils restent entiers en natif dans cette reprise ; aucune assertion n’est modifiée ni scénario découpé. Les contrats synthétiques à extraire ultérieurement couvrent l’ordonnancement et le recyclage, les identités et liaisons source, les transitions capture/Stop/gain, les documents et la publication. L’extraction future devra garder la preuve distincte de la garde réelle et toutes les assertions ; elle exige son propre périmètre accepté.

Le prérequis est models/parakeet-tdt-0.6b-v3.q8_0.gguf enregistré, sauf live_automatic_final_gate_does_not_block_next_session_stop_or_late_publish qui utilise models/ggml-large-v3-turbo.bin. Trois cas unified_restart_and_pages exigent également le parent historique target/parakeet-tests/refacto-parakeet-live/p3-workers dans leur recette native. Aucun de ces prérequis n’est préparé ou exécuté dans le lot actuel.

| Test conservé entier | Source |
| --- | --- |
| controlled_bounded_gpu_prepare_uses_identity_without_legacy_session_qualification | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| controlled_runtime_recycles_scheduler_slots_across_many_unified_imports | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| controlled_worker_and_invalid_times_leave_no_complete_archive | crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs |
| export_collision_keeps_existing_file_and_does_not_publish_unified_archive | crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs |
| capture_with_empty_snapshot_registry_publishes_complete_source_without_registry_state | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| global_close_waits_for_both_capture_publications_and_refuses_new_admission | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| failed_unified_resume_after_binding_transfer_restores_registry_and_queue_identity | crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs |
| import_application_finalizes_stopped_capture_a_while_capture_b_pumps_and_publishes_segmented_audio | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| failed_unified_resume_preserves_interrupted_source_binding_and_queue_identity | crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs |
| import_application_import_a_and_capture_b_share_one_held_backend_until_b_is_published | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| import_application_processes_durable_queued_unified_imports_for_cpu_gpu_auto_and_long_audio | crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs |
| capture_identity_does_not_mask_existing_import_source_binding | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| import_application_uses_real_archive_path_for_controlled_short_and_long_unified_imports | crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs |
| live_automatic_final_gate_does_not_block_next_session_stop_or_late_publish | crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs |
| queued_create_only_publication_paths_reach_app_view | crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs |
| native_auto_gpu_ready_failure_retries_once_on_cpu_before_first_call | crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs |
| reprocessing_create_only_publication_paths_reach_app_view_and_preserve_earlier_result | crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs |
| unified_capture_after_direct_stop_uses_its_own_gate_and_publishes_audio | crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs |
| unified_capture_alone_routes_probe_and_rejects_old_or_wrong_source_events | crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs |
| unified_capture_updates_preview_and_final_document_before_stop_then_publishes_the_tail | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| unified_capture_runtime_rejects_third_before_capture_factory | crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs |
| import_application_resumes_a_verified_interrupted_unified_import | crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs |
| unified_start_and_immediate_stop_keep_the_new_session_identity | crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs |
| unified_document_finalization_failure_releases_session_and_allows_replacement | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| resumed_create_only_publication_paths_reach_app_view | crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs |
| unified_runtime_keeps_reserved_final_turn_through_preparation_with_peer_preview_ready | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |
| unified_capture_gain_changes_apply_to_the_next_written_pcm_block_and_reject_stale_targets | crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs |

Les TODO(DEBT-10) sont posés par l’implémenteur dans les sections de tests ; leur cohérence sera confirmée par le même reviewer. Cette entrée ne transforme pas les échecs V1 historiques en réussites.


## DEBT-11 — Reprocessing et catalogue Turbo réel

Les sept scénarios ci-dessous mêlent contrôles synthétiques de nettoyage, récupération, admission et publication avec un catalogue Turbo réel. Le helper create_failed_cleanup_fixture ou la préparation directe vérifie models/ggml-large-v3-turbo.bin par LocalModelCatalog, hash et canonicalize avant les assertions concernées. Ils restent entiers en natif, avec TODO(DEBT-11) et toutes leurs assertions. Une évolution ultérieure pourra isoler les contrats synthétiques de remplacement, préservation d’objets et reprise de nettoyage, en conservant séparément la preuve du catalogue réel. Cette extraction n’est pas réalisée ici.

| Test conservé entier | Source |
| --- | --- |
| application_create_new_processes_the_bound_snapshot_and_preserves_prior_publication | crates/whisper-adapters/tests/reprocessing_create.rs |
| reparse_container_is_rejected_before_any_old_artifact_is_removed | crates/whisper-adapters/tests/reprocessing_cleanup.rs |
| replaced_container_identity_is_rejected_and_both_directories_are_preserved | crates/whisper-adapters/tests/reprocessing_cleanup.rs |
| durable_legacy_cleanup_replay_keeps_empty_parent_without_container_proof | crates/whisper-adapters/tests/reprocessing_cleanup.rs |
| archive_characterization_cleanup_missing_object_before_progress_is_adopted | crates/whisper-adapters/tests/reprocessing_cleanup.rs |
| archive_characterization_cleanup_recorded_removed_but_present_is_preserved | crates/whisper-adapters/tests/reprocessing_cleanup.rs |
| replace_all_publishes_first_stops_partial_cleanup_and_resumes_only_on_request | crates/whisper-adapters/tests/reprocessing_cleanup.rs |

Le prérequis est le fichier Turbo enregistré réel sous models/ ; les racines de sortie déjà synthétiques restent propres aux tests. Aucun modèle n’est fourni dans cette reprise.
