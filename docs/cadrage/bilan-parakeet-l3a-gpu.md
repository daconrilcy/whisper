Avancée : cartes A1 à A4 PASS, revue indépendante CLEAN et commit local L3a préparé.
Blocage : aucun pour L3a ; `live_archive` a produit une erreur système 3 intermittente au premier passage du gate.
Décision : clôturer L3a sur son worker GPU figé et attendre un accord séparé pour L3b.

# Bilan L3a — GPU Parakeet pour l'import, worker natif

Base `d1fa9249`, plan [PLAN-PARA-L3 v5](plan-parakeet-l3-gpu-import.md), identité et mesures dans la [fiche L3a](fiche-parakeet-l3a-gpu.md). Le candidat touche uniquement les onze chemins de code L3a autorisés : cinq dans `whisper-worker-parakeet-cpu`, deux dans `whisper-core`, quatre dans `whisper-adapters`. Diff avant documentation : **1 285 insertions, 49 suppressions**. `Cargo.lock` et le worker CPU R6 n'ont pas changé. Le binaire GPU figé a le SHA-256 `5054612a05fbfd1039e108def3aa4e598268d2b4274b8c1dc96cd6b8e3f2cd6d` ; ses neuf DLL et le modèle sont détaillés dans la fiche. Toutes les écritures d'exécution sont sous `target/parakeet-tests/l3/l3a/` et le Cargo partagé.

| Carte | Résultat | Statut |
| --- | --- | --- |
| A1 | Feature `parakeet-cuda`, ABI NeMo `gpu=0`, chargement unique des DLL sans `FreeLibrary`, copie release figée ; CPU R6 inchangé. | **PASS** |
| A2 | Journal `[asr] backend=CUDA0` dans le PID worker, carte Windows/NVIDIA unique et cohérente, EN5 natif `6,396 s` ≤ `11,1 s`, 45 mesures GPU pendant l'appel. | **PASS** |
| A3 | FR87, FR27, EN15, EN5 réel, EN30 synthétique, vingt appels et mauvais périphérique passent sur le même worker. Zéro `InvalidTimestamp`. | **PASS** |
| A4 | `fmt`, build workspace CPU, tests workspace hors desktop, compilation des tests desktop : dernier passage de chaque commande sous garde avec exit code `0`, aucun écart hors tolérance ; revue indépendante **CLEAN** après deux passes et une correction. | **PASS** |

## Garde et diagnostic Git

Recherche dans les scripts de build et `build.rs` du workspace : **aucun appel Git** (`Command::new("git")`, `git describe`, `rev-parse`, `status`). La garde d'exécution ignorée par Git, `target/parakeet-tests/l3/l3a/guard-campaign.ps1`, classe les événements de `.git/` en bruit VCS et compare avant/après HEAD, SHA-256 de `git show-ref`, `git status --porcelain=v1 -uall` et `git stash list`. L'instantané de la racine exclut `.git`. Les neuf auto-tests ont réussi : sept cas historiques, création/suppression de `index.lock` dans un dépôt témoin (**PASS, bruit VCS**) et commit témoin (**FAIL détecté**). Aucune opération de cet auto-test ne touche le vrai `.git` du dépôt.

Avant chaque recette GPU, la garde vérifie la carte Windows et `nvidia-smi` avant le lancement du worker et le SHA du worker figé. Chaque campagne compare avant/après les sept SHA protégés (modèle, worker CPU R6, cinq DLL CPU), identiques. Les campagnes finales `Fmt`, `BuildWorkspace`, `Adapters`, `Desktop`, `GpuSingle`, `GpuTwenty` et `GpuWrong` rapportent toutes `exitCode=0`, aucun écart observé et aucun événement hors tolérance. Les journaux sont sous `target/parakeet-tests/l3/l3a/logs/guard-*.json` et l'auto-test sous `guard-selftest/results.json`.

## Gate A4 et réserve

| Commande sous garde | Dernier résultat |
| --- | --- |
| `cargo fmt --all -- --check` | exit `0` |
| `cargo build --offline --locked --workspace` | exit `0` ; build sans feature CUDA |
| `cargo test --offline --locked --workspace --exclude whisper-desktop --no-fail-fast` | exit `0` ; 46 cibles, **317 PASS, 0 FAIL, 33 ignorés** ; `parakeet_archive` : 9 PASS, 5 ignorés |
| `cargo test --offline --locked -p whisper-desktop --no-run` | exit `0` ; compilation seulement |

Les 33 tests ignorés sont **NOT RUN dans ce gate** ; ils comprennent des recettes dépendant des fixtures ou de l'environnement, et ne sont pas présentés comme PASS. Les trois recettes GPU A3 ont été exécutées séparément avec `--ignored --exact --nocapture` sur le binaire figé et sont PASS. Un premier passage de la commande adapters a échoué sur `live_archive::attempt_journal_reserves_monotonically_and_enforces_transitions` : `StorageUnavailable`, « Le chemin d'accès spécifié est introuvable » (`os error 3`). Le nouveau passage complet, sous la même garde, est vert. Aucun correctif de `live_archive` n'est inclus dans L3a ; l'intermittence reste un risque de test à suivre.

Noms des **33 NOT RUN de ce gate** (les trois `gpu_worker_*` ont leur PASS distinct dans A3) :

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

## Revue et suite

Le reviewer indépendant a fermé R1 (confinement des racines GPU), R2 (ordre de validation de durée et marqueur d'appel natif) et R3 (préflight GPU avant worker). Verdict final **CLEAN**, deux passes de revue et une passe de correction. Le SHA du diff de code revu est `d973e4c3…7d25a74`. Les documents de clôture ont été rédigés après ce verdict conformément au plan. Aucun push. L3b, l'admission GPU dans le produit et la validation dans la fenêtre Windows restent à traiter séparément après accord écrit.
