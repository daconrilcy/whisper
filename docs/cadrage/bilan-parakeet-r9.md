Avancée : R9 est implémenté sur la clé Test Parakeet gelée ; la revue indépendante finale rend CLEAN.
Blocage : aucun constat de code ouvert ; 31 tests du gate workspace restent ignorés et donc NOT RUN dans ce gate.
Décision : committer localement les huit chemins autorisés et ce bilan ; aucun push, aucune activation de B8.

# Bilan PLAN-PARA-R9 — 17 octobre 2026

Plan : `plan-parakeet-r9-sha-cache.md` v4, CLEAN — PLANS READY, statut au commit `d64260b9`. Base du candidat : `wip/l2b-candidate`, HEAD `d64260b97a5d13f33d4e4b74edb1332d57500d82` avant commit de code. Le code final modifie exactement huit chemins suivis : `Cargo.lock`, `crates/whisper-adapters/Cargo.toml`, `src/{archive,model_catalog,parakeet_import}.rs`, `tests/{model_catalog,parakeet_archive,parakeet_import}.rs`. Les modèles et `sound example/` non suivis ont été préservés. Worker produit gelé `1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0`, modèle `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`.

## Résultat des cartes

| Carte | Résultat | Statut |
| --- | --- | --- |
| R9-1 préflight | Branche/HEAD, espace >30 Gio, SHA modèle/worker/cinq DLL et fixtures contrôlés ; garde R9 OK, racine R8 KO avant Cargo. Cargo partagé, offline. | PASS |
| R9-2 cache | Cache de session synchronisé, identité physique `same-file` + taille + date recontrôlées ; un calcul complet par identité ; changements et erreurs refusés. `same-file = 1.0.6` était en cache, ajout direct au manifeste et une entrée dans `Cargo.lock`. | PASS |
| R9-3 raccord | Catalogue, qualification/délégation Parakeet et archive utilisent le cache. Le contrôle SHA du worker gelé reste indépendant ; aucun repli Turbo ajouté. | PASS |
| R9-4 tests | Tests de concurrence, réutilisation, identité changée et erreur PASS. Test réel de modèle modifié PASS : `ModelHashMismatch` avant destination/pending. | PASS |
| R9-5 recettes | EN5, EN30, A4 FR87/FR27/EN15 et les cinq scénarios R8 PASS sur la clé Test actuelle. | PASS |
| R9-6 gate | `cargo fmt --all -- --check`, `git diff --check`, `cargo test --offline --locked --workspace --no-fail-fast` PASS. Gate final : **366 PASS, 0 FAIL, 31 ignorés**, code de sortie 0. | PASS |
| R9-7 revue | Même `rust_reviewer` indépendant ; R9-R1 et R9-R2 corrigés par `rust_implementer` en deux cycles, V1 vérifié ; verdict final **CLEAN**, aucun constat ouvert. | PASS |

## Mesures de la même clé Test

| Recette | Inventaire catalogue avant demande | Demande → TXT/SRT | Inférence worker | Résultat |
| --- | ---: | ---: | ---: | --- |
| EN5 réel | 24,207 s | **38,598 s** | **22,134 s** | PASS : surcoût 16,464 s, sous la limite de 30 s. |
| EN30 synthétique `spkr0_x6.wav` | 23,495 s | 195,814 s | 146,085 s | PASS sous délai 570 s ; 28 800 000 échantillons. |
| FR87 | 25,014 s au premier essai | 14,970 s | 4,491 s | PASS ; TXT/SRT, `faire? Je`, durées positives, zéro U+FFFD. Recette corrigée ensuite repassée. |
| FR27 | 24,136 s | 10,884 s | 1,286 s | PASS ; dernier mot alphabétique, TXT/SRT. |
| EN15 | 24,650 s | 10,485 s | 0,846 s | PASS ; TXT/SRT, zéro U+FFFD. |

L’inventaire catalogue est séparé du temps **demande → TXT/SRT** : `DesktopComposition::compose` lance cet inventaire à la création, et l’admission est vérifiée avant la commande d’import. La recette attend explicitement les deux modèles `Ready` avant de démarrer ce chronomètre et rapporte le coût initial. Le démarrage du worker et son contrôle SHA sont compris dans le temps mesuré de l’import. Une mesure exploratoire démarrant avant l’admission a échoué au seuil ; elle n’a pas été présentée comme la mesure finale de la demande admissible.

Sur EN5, le compteur de lectures complètes **adaptateur** du modèle vaut 1 après l’inventaire, 1 après le premier import et 1 après le second import de la même session : **zéro lecture nouvelle au deuxième import**. Ce second import EN15 a pris 10,219 s. Le parcours lance un worker par import : deux démarrages sur cette session. Son SHA du modèle est contrôlé une fois par démarrage dans le worker gelé, hors plafond du cache adaptateur. La recette FR87 finale compte ses deux workers (vérification des mots et import) et deux contrôles SHA, avec deux enregistrements natifs bornés à l’exécution. La VRAM, la VAD et le GPU ne sont pas qualifiés par R9.

## Identités natives et preuves

Les cinq DLL réellement attendues sous `target/parakeet-tests/l2/l2b/native/` ont été revérifiées :

| DLL | SHA-256 |
| --- | --- |
| `nemo_speech_asr_c.dll` | `9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764` |
| `nemo_speech_asr.dll` | `923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019` |
| `ggml.dll` | `2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794` |
| `ggml-base.dll` | `398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a` |
| `ggml-cpu.dll` | `f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0` |

Toutes les écritures d’exécution R9 sont sous `target/parakeet-tests/l2/l2b/r9/`, sauf l’unique journal natif autorisé `target/parakeet-tests/l2/l2b/r6/logs/r9-native-inference.jsonl`. Cargo écrit sous `target/parakeet-tests/cargo-shared/`. Log final workspace : `r9/logs/workspace-no-fail-fast-review-cycle2-final.log`, SHA-256 `ffeb5b344e2863031881d4457766cf1c4679ff3407de4e7a36a699055b4e90cb`. Journaux FR87 final et refus modèle modifié : `r9/logs/fr87-r9-review-cycle2-final.log` et `r9/logs/v1-r9-review-cycle1.log`.

La dernière relance workspace a code de sortie 0. Une tentative antérieure avait échoué sur deux variables de campagne mal définies ; la garde et l’environnement ont été corrigés sans modifier le produit. Une invocation de `cargo fmt --check` en lecture seule a eu lieu sans rappel de la garde ; elle a été répétée sous garde avec PASS. Ces écarts de procédure n’ont pas été masqués par le gate final.

**Limite d’intégrité :** une modification volontaire conservant taille, date et identifiant de fichier n’invalide pas le cache adaptateur. Le worker gelé recalcule son propre SHA au démarrage et refuse alors le modèle avant inférence. Cela ne prouve pas un refus avant pending pour ce cas particulier ; R9 prouve le refus avant pending quand l’identité détectable change.

## NOT RUN du gate workspace final (31 tests ignorés)

Ces cas ne sont pas comptés comme PASS du gate. Les recettes A4, R9 et R8 demandées ont été exécutées séparément avec leurs journaux ; B9 reste sans fixtures bruit dans ce lot.

1. `capture::tests::native_selected_microphone_survives_initial_discontinuity`
2. `capture::tests::native_wasapi_input_catalog_has_stable_parseable_ids`
3. `modified_parakeet_model_is_refused_before_pending_after_cache_revalidation`
4. `r5_2_a4_product_route_publishes_french_and_english_txt_srt`
5. `r5_2_b9_reports_noise_floor_silences_and_cut_plans`
6. `r5_2_long_tests::en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy`
7. `r5_2_long_tests::fr_synthetic_long_reference_and_vad_use_same_candidate`
8. `resumed_native_import_keeps_confirmed_prefix_once_and_publishes_once`
9. `resumed_native_vad_import_keeps_confirmed_prefix_once_and_publishes_once`
10. `stop_and_quit_at_preparation_and_publication_barriers_preserve_truthful_state`
11. `stop_during_parakeet_inference_waits_for_exit_and_preserves_pending_without_publication`
12. `stop_queue_journal_failure_settles_runtime_and_allows_next_import`
13. `incompatible_native_dll_is_refused_by_product_and_fails_at_loader_boundary`
14. `incomplete_result_frame_is_rejected_as_transport_decode_failure`
15. `killed_native_worker_is_reported_as_exit_without_result`
16. `product_route_incomplete_result_preserves_pending_source_and_rejects_completion`
17. `product_route_worker_death_preserves_pending_source_and_rejects_completion`
18. `r6_application_single_pass_recipe`
19. `r7_application_timing`
20. `r9_application_recipe`
21. `b9_reports_noise_floor_silence_counts_and_cut_plans`
22. `en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy`
23. `fr_synthetic_long_reference_and_vad_use_same_candidate`
24. `root::tests::parakeet_worker_key_changes_regray_and_readmit_without_restarting`
25. `cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache`
26. `worker_decoder_matches_turbo_decoder_for_wav_and_mp3_within_one_i16_unit`
27. `cpu_worker_accepts_fr_and_en_twenty_times_in_one_process_and_exits_without_unload`
28. `cpu_worker_normalizes_fr87_fr27_and_en15_short_fixtures`
29. `r6_reference_en60`
30. `r7_private_worker_timing`
31. `r7_private_worker_timing_child`

Les **164 petits fichiers de preuve** (1 052 503 octets), dont le journal natif R6 autorisé, sont archivés dans `C:\dev\whisper-evidence\l2b-r9\` avec l’arborescence conservée. Le nombre de fichiers et le SHA-256 de chaque copie ont été vérifiés. L’espace libre du disque C: après archivage est de **133,16 Gio**. Aucune copie temporaire n’a été supprimée : une suppression éventuelle exigera un manifeste nominatif approuvé. Aucun modèle, WAV source, `spike02` ou worker admis n’a été supprimé.
