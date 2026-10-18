Avancée : L3b a passé ses recettes, son gate workspace et sa seconde revue indépendante CLEAN.
Blocage : aucun constat de code ouvert ; la recette visuelle dans l'application Windows reste à faire par l'utilisateur.
Décision : clôturer L3b en commits locaux sans push, puis arrêter avant l'essai dans la fenêtre Windows.

# Bilan L3b — Parakeet GPU pour l'import

Base `11163657` (L3a CLEAN), [plan L3 v5](plan-parakeet-l3-gpu-import.md), [fiche GPU Test](fiche-parakeet-l3-gpu-test.md). Candidat revu : empreinte du diff `6ed9a67a4a677a920dfc3f1a398e5966572cdb4f`. Le worker GPU L3a `5054612a…` et ses neuf DLL n'ont pas été modifiés ; le worker CPU R6 `1d8f57b9…` et ses cinq DLL restent inchangés. L3b touche 13 chemins autorisés dans `whisper-core`, `whisper-adapters` et `whisper-desktop`, sans `Cargo.lock`, C++ ni réseau. Le diff exact des quatre tests ouverts par décision utilisateur est conservé sous `target/parakeet-tests/l3/l3b/l3b-four-tests.diff`.

| Carte | Résultat | Statut |
| --- | --- | --- |
| B1 | Admission GPU par la clé exacte (modèle, worker, 9 DLL, CUDA0, profil import, carte physique) après qualification réelle ; injection de candidat limitée au binaire de test et retirée du build final. | **PASS** |
| B2 | GPU manuel fixe ; Auto privilégie le GPU et n'autorise qu'un essai CPU Parakeet avant `NativeCallStarted` ; aucun repli après ce marqueur, après mort sans marqueur ou vers Turbo. Reprise avec backend différent refusée sans perte. | **PASS** |
| B3 | Interface : disponibilité GPU recalculée sur les artefacts et la carte ; sélection manuelle refusée avec cause si la clé manque ou diverge. CPU Parakeet, matrice Turbo et micro Turbo conservés. | **PASS** |
| B4 | FR87, FR27, EN15, EN5 et EN30 sur la même copie worker ; TXT/SRT produits, zéro `InvalidTimestamp`, refus >30 min avant pending ; cinq scénarios R8 CPU et GPU réussis. | **PASS** |
| B5 | `fmt`, build workspace et suite complète `cargo test --offline --locked --workspace --no-fail-fast` sous garde : 377 résultats `ok` dont deux recettes sans sélecteur non exécutées, zéro échec, 34 ignorés. SHA protégés identiques, zéro événement hors tolérance. | **PASS** |

## Recettes et garde

| Campagne sous `guard-campaign.ps1` | Résultat final |
| --- | --- |
| `Fmt` (`cargo fmt --all -- --check`) | exit `0`, aucun écart |
| `BuildWorkspace` (`cargo build --offline --locked --workspace`, sans feature d'injection) | exit `0`, aucun écart |
| `Recipe-gpu_app_fr_en_single_pass` | exit `0`, FR87/FR27/EN15, `185,29 s` de test, TXT/SRT |
| `Recipe-gpu_app_en5` | exit `0`, EN5, `132,34 s` de test, TXT/SRT |
| `Recipe-gpu_app_en30` | exit `0`, EN30, `220,80 s` de test, TXT/SRT |
| `Recipe-gpu_manual_and_auto_fallback` et `Recipe-gpu_rejects_bad_identity_before_pending` | exit `0` chacun ; repli et refus prouvés |
| Cinq campagnes `R8-*` | exit `0` chacune ; variantes CPU et GPU exercées |
| `Workspace` | exit `0` ; 377 `ok` dont 2 recettes non exécutées, 0 FAIL, 34 ignorés ; `observedDeviation=False`, 0 événement interdit, 70 événements de bruit tiers sans attribution |

La garde a été auto-testée sur neuf cas après la dernière modification de son script ; 9/9 conformes. `Workspace-sha-before.json` et `Workspace-sha-after.json` sont identiques. Les journaux des campagnes sont sous `target/parakeet-tests/l3/l3b/logs/`. Dans le gate workspace, les tests `gpu_app_fr_en_single_pass` et `gpu_app_en5` retournent `ok` sans exécuter la recette quand le sélecteur `PARAKEET_L3B_RECIPE_CASE` est absent : ils sont **NOT RUN dans ce gate**, et leur PASS provient exclusivement des campagnes ciblées ci-dessus, avec sélecteur et sorties réelles. Les 34 tests ignorés de la suite sont également **NOT RUN dans ce gate** ; leurs noms sont consignés dans `Workspace.stdout.log`. Les cinq scénarios R8, ignorés dans la suite générale, ont chacun leur PASS ciblé.

Les 34 tests ignorés du gate, tous **NOT RUN dans ce gate**, sont :

```text
capture::tests::native_selected_microphone_survives_initial_discontinuity
capture::tests::native_wasapi_input_catalog_has_stable_parseable_ids
modified_parakeet_model_is_refused_before_pending_after_cache_revalidation
r5_2_a4_product_route_publishes_french_and_english_txt_srt
r5_2_b9_reports_noise_floor_silences_and_cut_plans
r5_2_long_tests::en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy
r5_2_long_tests::fr_synthetic_long_reference_and_vad_use_same_candidate
resumed_native_import_keeps_confirmed_prefix_once_and_publishes_once
resumed_native_vad_import_keeps_confirmed_prefix_once_and_publishes_once
stop_and_quit_at_preparation_and_publication_barriers_preserve_truthful_state
stop_during_parakeet_inference_waits_for_exit_and_preserves_pending_without_publication
stop_queue_journal_failure_settles_runtime_and_allows_next_import
incompatible_native_dll_is_refused_by_product_and_fails_at_loader_boundary
incomplete_result_frame_is_rejected_as_transport_decode_failure
killed_native_worker_is_reported_as_exit_without_result
product_route_incomplete_result_preserves_pending_source_and_rejects_completion
product_route_worker_death_preserves_pending_source_and_rejects_completion
r6_application_single_pass_recipe
r7_application_timing
r9_application_recipe
b9_reports_noise_floor_silence_counts_and_cut_plans
en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy
fr_synthetic_long_reference_and_vad_use_same_candidate
root::tests::parakeet_worker_key_changes_regray_and_readmit_without_restarting
cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache
worker_decoder_matches_turbo_decoder_for_wav_and_mp3_within_one_i16_unit
cpu_worker_accepts_fr_and_en_twenty_times_in_one_process_and_exits_without_unload
cpu_worker_normalizes_fr87_fr27_and_en15_short_fixtures
gpu_worker_fr_en_single_pass
gpu_worker_rejects_wrong_device_before_audio
gpu_worker_twenty_calls
r6_reference_en60
r7_private_worker_timing
r7_private_worker_timing_child
```

## Revue et limites

Deux passes de revue indépendante ont été effectuées. La première a relevé quatre constats : recettes ajoutées en `#[ignore]`, identité physique insuffisante dans l'admission, indication GPU trop optimiste dans l'interface, et remplacement des contrôles CPU R8. Les corrections ont retiré les nouveaux `#[ignore]`, lié la clé à la carte physique, vérifié worker/DLL dans l'interface et apparié CPU/GPU dans les cinq R8. La seconde passe a déclaré **CLEAN** sur le candidat `6ed9a67a…`, sans constat ouvert. Les assertions des quatre fichiers de tests autorisés ont été remplacées et conservées, sans affaiblir les assertions Turbo ou CPU.

La validation dans la fenêtre Windows n'a pas été réalisée par Codex et reste à l'utilisateur. EN5 applicatif a respecté le délai de recette de `195 s`, mais la campagne ne démontre pas l'objectif de `≤ 15 s` de bout en bout. Les autres limites de périmètre figurent dans la fiche : FR long réel et vérité terrain absents, GPU limité à cette carte et ces artefacts, L6-perso à amender, VAD et micro Parakeet hors lot.
