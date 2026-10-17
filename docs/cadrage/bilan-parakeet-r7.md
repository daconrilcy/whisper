Avancée : R7 instrumente les temps sans changer l'admission produit ; EN30 et le dernier gate workspace passent.
Blocage : EN5 corrigé a expiré deux fois avant le worker ; la revue reste LIVRÉ AVEC RÉSERVES.
Décision : aucune correction du surcoût n'a été faite ; le candidat reste non commité, B8 inactive.

# Bilan R7 — 17 octobre 2026

Base Git `wip/l2b-candidate` HEAD `319a7737f8a213ecc9ed076c7651cab9c5d98868`. Le candidat est le diff non commité de dix fichiers autorisés. Worker produit R6 gelé : SHA-256 `1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0`. Worker privé de diagnostic : SHA-256 `5098b21f50e84d80324836b4b71586602bec34e940b2d8f8a62574a738f212a6`. Même modèle et cinq DLL, mais le worker privé n'est pas une admission applicative.

## Ventilation chronométrée

EN5 est la **dernière passe complète avant la correction de sonde** ; deux passes après correction ont chacune expiré à 195 s dans `Preparing` et ne fournissent aucune mesure des mots. EN30 est la passe complète **après correction**. Cette asymétrie interdit de présenter le tableau comme une comparaison contrôlée unique des deux durées. Les durées de phases peuvent se recouvrir, notamment quand l'application et le worker agissent en parallèle ; ne pas additionner les lignes.

| Étape | EN5 (s) | EN30 (s) | Portée |
| --- | ---: | ---: | --- |
| Application, demande → résultat | 176,927 (avant sonde R7-R1) ; **FAIL 195** deux fois après | 504,720 (après sonde R7-R1) | TXT/SRT vérifiés sur les passes réussies. Délai EN30 : 570 s. |
| SHA adaptateur, total 56 lectures | 65,619 | 123,468 | 8 modèle, 8 worker, 8 de chacune des 5 DLL. |
| Dont SHA modèle adaptateur | 64,855 | 121,925 | Huit lectures du même GGUF de 714 Mo. |
| SHA archive, total 11 lectures | 8,121 | 16,863 | 2 source, 1 modèle, 4 TXT, 4 SRT. |
| Dont SHA modèle archive | 7,909 | 14,642 | Cette durée est comprise dans `préparation pending` ci-dessous. |
| Préparation archive/pending | 8,129 | 16,867 | Inclut un hash modèle ; ne pas additionner aux SHA archive. |
| Démarrage du processus worker | 0,004 | 0,007 | Temps du seul appel de création, hors attente Ready. |
| Démarrage → Ready | 8,226 | 12,445 | Worker produit gelé. |
| Demande worker → résultat | 22,137 | 148,511 | Inclut décodage, inférence, normalisation et transfert ; pas le démarrage. |
| Validation des mots applicative | **NOT MEASURED** après correction | 0,001112 / 5 903 mots | Sonde ajoutée après les premiers PASS EN5. |
| Regroupement des mots applicatif | **NOT MEASURED** après correction | 0,001661 / 548 groupes | Même limite. |
| Persistance des segments | 0,235 / 99 segments | 1,665 / 548 segments | Écritures et sync mesurés, hors intervalles entre segments. |
| Préparation et commit de publication | 0,022 | 0,035 | TXT/SRT/manifest ; les hash TXT/SRT sont comptés aussi dans SHA archive. |
| Worker privé : processus total | 40,48 | 171,30 | Voie directe diagnostique ; non substituable au temps produit. |
| Worker privé : SHA, chargement, décodage, inférence, normalisation | 8,363 ; 0,019 ; 0,423 ; 22,829 ; <0,001 | 8,921 ; 0,020 ; 2,448 ; 150,216 ; 0,002 | Durées internes du worker privé, pas du binaire produit gelé. |
| Harness : sommeil demandé par polling | 176,5 / 1 766 Refresh | 395,0 / 3 951 Refresh | Mesure sur les passes complètes pré-correction ; **non additive** : le worker travaille pendant ces attentes. |
| Harness : compilation `cargo` affichée | 2,32 initial puis 0,22 incrémental | 0,24 incrémental | Temps affichés par Cargo, hors demande→résultat. La durée dédiée `--no-run` est **NOT MEASURED**. |
| Harness : garde, copie worker privé | **NOT MEASURED** | **NOT MEASURED** | Opérations hors chronomètre applicatif ; aucune valeur inventée. |
| Harness : vérification finale TXT/SRT | **NOT MEASURED** après correction | 0,000 à la résolution milliseconde | EN5 n'a pas atteint cette vérification après correction. |

**Cause étayée du surcoût :** l'adaptateur relit huit fois le GGUF pour SHA, puis l'archive une fois. Sur EN5 passé, cela représente au moins 72,764 s mesurées ; sur EN30 corrigé, au moins 136,567 s. Le coût d'une lecture modèle a varié d'environ 8 à 15 s entre campagnes. Ce fait explique une part substantielle du temps de préparation et de la variation EN30. Il ne couvre pas à lui seul le total : les traces montrent aussi des intervalles non instrumentés avant certaines lectures SHA et entre persistance de segments. Leur propriétaire exact reste **INCONCLUSIF**. Les 100 ms de sommeil du harness cadencent l'observation mais se chevauchent avec le traitement asynchrone ; ils ne peuvent pas être soustraits comme coût ajouté. Aucune optimisation ni suppression de vérification SHA n'est autorisée ou faite dans R7.

## Recettes et revue

- EN5 applicatif avant correction R7-R1 : PASS 176,927 s ; deux essais corrigés : FAIL à 195 s chacun dans `Preparing`, sans `spawn/Ready` ni mesure de validation/regroupement. Pas de troisième essai.
- EN30 applicatif corrigé : PASS 504,720 s, TXT/SRT présents. Sonde validation 1,1116 ms, regroupement 1,6612 ms. Le temps d'inférence directe du worker privé reste une mesure séparée.
- Test R4 de racine injectée : PASS ; prédicat produit de recette VAD inchangé. Ancienne clé `545a…` refusée avant pending, pending historique conservé et non reprenable ; clé `1d8f…` admise en Test. B9 est ignoré par défaut, donc NOT RUN sans fixtures ; règle et seuils inchangés.
- Garde R7 à échec fermé testée OK (exit 0) et KO (exit 1 avant Cargo). Écart de procédure consigné : `cargo --version` a été appelé une fois avant la garde, sans build ni écriture.
- `cargo fmt --all -- --check` et `git diff --check` : PASS. Dernier `cargo test --offline --locked --workspace --no-fail-fast` sous garde : exit 0, zéro FAIL, 29 ignorés. Sortie complète : `logs/workspace-r7-v2-full.log`, SHA-256 `706094dbc05e66936f074c2edf8f16a9326fedc77efaca409cac99d02ed03ca6`.
- Un passage immédiatement antérieur du même gate a échoué une fois dans `live_archive::tests::attempt_journal_reserves_monotonically_and_enforces_transitions` avec `StorageUnavailable` / erreur Windows 3. La relance identique a passé. L'échec ponctuel est conservé comme réserve d'environnement, sans preuve de cause racine.
- Reviewer indépendant : **LIVRÉ AVEC RÉSERVES** après deux cycles revue/correction. Constat ouvert R7-R1 MEDIUM : la ventilation validation/regroupement EN5 corrigée n'est pas démontrée. Vérification complémentaire V1/V2 sans changement de source.

## NOT RUN du gate workspace : 29 tests ignorés

1. `capture::tests::native_selected_microphone_survives_initial_discontinuity`
2. `capture::tests::native_wasapi_input_catalog_has_stable_parseable_ids`
3. `r5_2_a4_product_route_publishes_french_and_english_txt_srt`
4. `r5_2_b9_reports_noise_floor_silences_and_cut_plans`
5. `r5_2_long_tests::en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy`
6. `r5_2_long_tests::fr_synthetic_long_reference_and_vad_use_same_candidate`
7. `resumed_native_import_keeps_confirmed_prefix_once_and_publishes_once`
8. `resumed_native_vad_import_keeps_confirmed_prefix_once_and_publishes_once`
9. `stop_and_quit_at_preparation_and_publication_barriers_preserve_truthful_state`
10. `stop_during_parakeet_inference_waits_for_exit_and_preserves_pending_without_publication`
11. `stop_queue_journal_failure_settles_runtime_and_allows_next_import`
12. `incompatible_native_dll_is_refused_by_product_and_fails_at_loader_boundary`
13. `incomplete_result_frame_is_rejected_as_transport_decode_failure`
14. `killed_native_worker_is_reported_as_exit_without_result`
15. `product_route_incomplete_result_preserves_pending_source_and_rejects_completion`
16. `product_route_worker_death_preserves_pending_source_and_rejects_completion`
17. `r6_application_single_pass_recipe`
18. `r7_application_timing`
19. `b9_reports_noise_floor_silence_counts_and_cut_plans`
20. `en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy`
21. `fr_synthetic_long_reference_and_vad_use_same_candidate`
22. `root::tests::parakeet_worker_key_changes_regray_and_readmit_without_restarting`
23. `cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache`
24. `worker_decoder_matches_turbo_decoder_for_wav_and_mp3_within_one_i16_unit`
25. `cpu_worker_accepts_fr_and_en_twenty_times_in_one_process_and_exits_without_unload`
26. `cpu_worker_normalizes_fr87_fr27_and_en15_short_fixtures`
27. `r6_reference_en60`
28. `r7_private_worker_timing`
29. `r7_private_worker_timing_child`

Ces tests sont NOT RUN **dans le gate workspace**. Certains ont été lancés séparément lors des recettes R6/R7 ; cela ne les transforme pas en PASS du gate.
