Avancée : écart expliqué et différences de tests relues statiquement : +7 272 lignes sur le périmètre Git retenu.
Blocage : aucune conclusion « seuls des déplacements » possible pour toute la période ; limites sémantiques et natives explicites.
Décision : aucun correctif appliqué ; distinguer extraction fidèle des 235 tests, nouveaux témoins, adaptations et copies.

# Croissance des tests pendant les refactorings

## Conclusion et périmètre

L’augmentation ne vient **pas seulement des en-têtes/imports**. Entre `680cff123ed37d1c73bff8d07cd515af4b22c7e5` (parent de `f82063ae`, avant le premier commit de refactorisation des façades) et `88850e8afd45e3b4c827914fad4c55a796b479db`, les sources Rust passent de **58 215 à 65 487 lignes classées tests : +7 272**. Les fichiers Rust passent de **117 à 339 : 222 fichiers ajoutés**, dont le nombre de fichiers entièrement classés tests passe de **51 à 139 : +88**. Les 222 nouveaux fichiers ne sont donc pas tous des fichiers de tests. [Mesures par phase](raw/test-growth-phases.json), [méthode de la carte](raw/METHOD.md).

[Non vérifié] Le point de comparaison exact à l’origine du chiffre arrondi « 7 500 » n’est pas fourni. Le périmètre Git ci-dessus est explicite et reproductible. Toutes les mesures utilisent **le même classificateur lexical et les mêmes lignes physiques** qu’ARCHITECTURE.md : blancs/commentaires inclus, production et tests séparés. Elles ne comptent pas les fichiers générés ni plusieurs compilations d’un même fichier.

La comparaison historique utilise HEAD Git ; la correction locale ultérieure de deux SHA attendus dans `crates/whisper-core/tests/parakeet_import_contract.rs:224-239` est signalée séparément. Elle ne change ni nombre de lignes ni nombre de tests et n’est pas une modification découverte dans ces refactorings. [Correction et validation ultérieures](NIVEAU1.md).

## Décomposition comptable des 7 272 lignes

Les catégories ci-dessous sont disjointes. Priorité : plage physique d’une fonction portant #[test], puis autre fonction classée test, puis import hors fonction, puis reste. Chaque ligne n’est comptée qu’une fois, y compris en cas de fonction imbriquée. L’attribut et les lignes intérieures, y compris blancs/littéraux de fixtures, appartiennent à la plage de la fonction. [Partition complète par fichier](raw/test-growth-line-partition.json).

| Catégorie physique | Avant | Après | Différence |
| --- | ---: | ---: | ---: |
| Fonctions portant #[test], signature/attributs/corps | 40408 | 44629 | +4221 |
| Autres fonctions de support classées tests | 14341 | 16661 | +2320 |
| Imports hors des fonctions ci-dessus | 721 | 976 | +255 |
| Reste : types, constantes, modules, blancs et commentaires hors fonctions | 2745 | 3221 | +476 |
| **Total tests** | **58 215** | **65 487** | **+7 272** |

Les imports représentent **255 lignes nettes**. Les **476 autres lignes hors fonctions** ne sont pas toutes des en-têtes : elles incluent aussi types et constantes de fixtures. Les deux catégories de fonctions portent **6 541 lignes nettes**. Cette comptabilité décrit l’emplacement physique ; elle ne prétend pas que chaque ligne de fonction est un nouvel oracle. Les fonctions peuvent contenir des tables de données, des chaînes de faux workers ou un scénario extrait d’une ancienne fonction. Exemple d’attendus MP3 volumineux : `crates/whisper-worker-common/src/encoder/tests.rs:4-271`. [Partition](raw/test-growth-line-partition.json).

## Chronologie : extraction et autres travaux ne sont pas confondus

Les lignes sont des soldes entre jalons ; les intervalles agrègent parfois plusieurs commits de code. Les fonctions #[test] sont des identités physiques, toutes conditions cfg confondues, pas le nombre du harness d’une configuration. [Mesures](raw/test-growth-phases.json), [inventaires de cas](raw/test-growth-phase-cases.json).

| Jalon Git | Tests physiques | Delta depuis le jalon précédent | Fonctions #[test] | Delta de fonctions |
| --- | ---: | ---: | ---: | ---: |
| `f82063ae^` (680cff12) | 58215 | +0 | 751 | +0 |
| `f82063ae` (f82063ae) | 60506 | +2291 | 781 | +30 |
| `7a4d20b2` (7a4d20b2) | 60529 | +23 | 781 | +0 |
| `2d9c3fe8` (2d9c3fe8) | 61072 | +543 | 792 | +11 |
| `c753f7c6` (c753f7c6) | 61532 | +460 | 799 | +7 |
| `2829ce45` (2829ce45) | 61610 | +78 | 801 | +2 |
| `7154e3fe` (7154e3fe) | 62672 | +1062 | 826 | +25 |
| `90ead44e` (90ead44e) | 62669 | -3 | 826 | +0 |
| `17cd4927` (17cd4927) | 63000 | +331 | 832 | +6 |
| `b09a8b66` (b09a8b66) | 63165 | +165 | 835 | +3 |
| `998a732a` (998a732a) | 63162 | -3 | 835 | +0 |
| `778ef0bc` (778ef0bc) | 65487 | +2325 | 854 | +19 |
| `HEAD` (88850e8a) | 65487 | +0 | 854 | +0 |

Les deux plus grands apports sont les façades/workers (`f82063ae`, +2 291 lignes et +30 fonctions de test) et les corrections micro/isolation (`998a732a` → `778ef0bc`, +2 325 lignes et +19 fonctions). Les caractérisations d’archives apportent +1 062 lignes et +25 fonctions entre les jalons core et archive. Ce sont des ajouts réels de tests/supports, pas uniquement du découpage. Exemples : `crates/whisper-worker-cpu/tests/worker_contract.rs:31-92` ; `crates/whisper-adapters/src/worker_ipc/tests/unified_restart_and_pages.rs:1-564` ; `crates/whisper-adapters/src/archive/tests/publication_characterization.rs:1-264`. Les exigences de caractérisation sont explicites dans `docs/cadrage/plan-whisper-worker-common.md:53-73` et les tâches/bilans de `docs/cadrage/plan-refacto-application-core.md:335-335`. [Chronologie mesurée](raw/test-growth-phases.json).

### Vérification distincte de la règle « tests déplacés »

Le lot d’extraction dédié est la comparaison `f82063ae` → `7a4d20b2`. Le plan prévoit **235 tests**, sur application (44), UI (40), archive (22) et IPC (129), avec conservation des scénarios/assertions et seuls renommages DEC-T03 autorisés. `docs/cadrage/plan-refacto-tests-diagnostics.md:60-65` ; `docs/cadrage/plan-refacto-tests-diagnostics.md:301-307` ; `docs/cadrage/plan-refacto-tests-diagnostics.md:385-385`.

Vérification refaite sur les sources Git : **228 items de test identiques en tokens** (espacement/commentaires exclus, littéraux conservés) et **7 items ne différant que par les renommages de helpers prévus**. Après application de cette liste fermée de renommages aux anciens tokens, **235/235 items sont identiques**, zéro ajout, zéro suppression, zéro nouveau groupe de corps dupliqués sur ce lot. Solde de lignes tests du lot : **+23**, et non +7 272. [Comparaison et liste des renommages](raw/test-growth-extraction-cases.json), [diff des sept items](raw/test-growth-extraction-changed-cases.diff).

Ce constat est cohérent avec le bilan versionné de cette extraction, qui documente sa propre comparaison des tokens et des supports. `docs/cadrage/plan-refacto-tests-diagnostics.md:440-442`. [Non vérifié] La nouvelle comparaison d’items ne démontre pas seule l’équivalence de tous les helpers ni des cfg/imports englobants ; elle ne se substitue pas à leur analyse.

## Inventaire historique complet des fonctions de test

Sur l’ensemble de la période : **751 → 854 fonctions #[test]**, soit **+103 nettes**. L’appariement trouve **706 fonctions identiques en tokens**, **43 fonctions anciennes modifiées**, **105 fonctions nouvelles**, et **2 anciennes occurrences supprimées**. Les égalités 706 + 43 + 2 = 751 et 706 + 43 + 105 = 854 sont vérifiées. [Inventaire nominatif et références avant/après](raw/test-growth-cases.json).

Les 105 fonctions nouvelles occupent **4 341 lignes physiques de plages** (attributs inclus). Ce chiffre brut n’est pas à additionner au solde des fonctions/supports : les plages historiques déplacées, les deux suppressions de copies et les variations de format/longueur sont déjà prises en compte dans la partition. Il ne signifie pas 105 scénarios sémantiquement indépendants : certains sont des wrappers, des fixtures ou des copies entre backends. [Inventaire](raw/test-growth-cases.json).

Les deux occurrences retirées sont les copies GPU de `long_source_is_streamed_as_bounded_contiguous_windows` et `lame_gapless_delay_and_padding_are_trimmed_once`. Leurs tokens sont identiques aux cas retenus dans common : il s’agit d’une mutualisation du même test de décodage, pas de la disparition de leurs assertions. `crates/whisper-worker-common/src/decoder/tests.rs:17-120`. Les anciens chemins/plages sont dans [l’inventaire](raw/test-growth-cases.json). [Non vérifié] Ce constat de conservation textuelle ne qualifie pas à lui seul toutes les routes natives CPU/GPU.

### Relecture des 43 différences

Les deltas des 43 items ont été relus ; pour les modifications de fixture/harness/oracle, les sites appelés ci-dessous ont aussi été consultés. **Aucun affaiblissement d’assertion n’a été mis en évidence dans cette relecture ciblée**, mais il existe des modifications effectives des scénarios, de la synchronisation, des fixtures et de deux oracles conditionnels. Il serait inexact de présenter toute la période comme « tests seulement déplacés ». [Deltas token par token](raw/test-growth-token-deltas.txt), [diff complet](raw/test-growth-changed-cases.diff), [classement annoté](raw/test-growth-cases.json).

| Groupe relu | Nombre | Constat |
| --- | ---: | --- |
| Renommages de helpers | 6 | Appels remplacés, arguments/assertions inchangés hors renommage. |
| Adaptation d’API, types, arguments | 26 | Box/déréférencement, contextes nommés et WorkerRuntimeConfig ; valeurs des anciens arguments retrouvées dans les champs. Le Default de cette configuration conserve None/false : `crates/whisper-adapters/src/worker_ipc.rs:124-131`. |
| Virgules de formatage | 3 | D2, son auto-test de statut et le témoin WAV ; les différences de tokens sont des virgules terminales. |
| Autres modifications effectives | 8 | Détail ci-dessous ; elles ne sont pas assimilées à des déplacements. |

| Cas relu | Différence constatée et portée |
| --- | --- |
| `retry_epoch_discards_audio_acquired_before_the_retry_barrier` | Timestamp ancien décalé de 1 ms, attente de processed_chunks à la place de sleep(10 ms), assertions de rejet/epoch ajoutées ; renforcement du témoin de concurrence. `crates/whisper-adapters/src/microphone_probe.rs:878-960`. |
| `parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav` | Initialisation du champ TraceSink désactivé retirée avec le champ de production ; assertions de spectre/WAV conservées, plus virgules de formatage. `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:10-216`. |
| `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` | Modèle lu sous WHISPER_L1B_RUNTIME_ROOT/models et racine du catalogue adaptée ; invariant de reprise conservé mais prérequis de fixture changé. C’est la source du prérequis L1B manquant observé pendant la validation, pas une simple extraction. `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1155-1280` ; [historique des essais](NIVEAU1.md). |
| `disconnected_preference_worker_reports_failed_model_and_compute_saves` | Deux assertions explicites sur PreferenceChannelError::Disconnected ajoutées avant les assertions de messages existantes. `crates/whisper-desktop/tests/model_selection_ui.rs:306-346`. |
| `cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache` | Corps délégué à run_for_exe ; child borné, capture des événements et racine de travail explicite ajoutés. Les deux imports, les contrôles Ready/End et les assertions cache load/hit sont retrouvés dans le helper, pas supprimés. Test toujours ignored. `crates/whisper-worker-cpu/tests/reusable_imports.rs:60-115` ; `crates/whisper-worker-cpu/tests/reusable_imports.rs:117-220`. |
| `accepts_only_the_two_canonical_l2_test_roots` | Fixture de dépôt isolée, et attentes devenues conditionnelles à parakeet-personal et parakeet-qualification-reference. Ce sont des oracles modifiés. Les conditions correspondent aux branches actuelles de la fonction testée. `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:1-48` ; `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:101-123`. |
| `gpu_worker_allows_only_the_canonical_l3_runtime_roots` | Même oracle de racines, mais sur les répertoires créés par fixture_repository au lieu du vrai dépôt. Test cfg CUDA hors personal. `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:50-77`. |
| `qualification_profiles_are_confined_to_l2b` | Fixture isolée ; assertion import-single-pass devenue dépendante de CUDA/personal, en accord avec allowed_runtime_root_candidates. `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:79-106` ; `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:89-123`. |

L’isolement des fixtures runtime_paths fait l’objet d’un accord enregistré dans le bilan du lot ; ce changement n’est pas présenté comme implicite dans la règle de déplacement. `docs/cadrage/plan-nettoyage-environnements-historiques.md:251-255`.

### Liste exhaustive des 43 items relus

Chaque ligne renvoie à la source courante du test ; les anciens chemins et plages sont dans le JSON. [Inventaire](raw/test-growth-cases.json).

| Test | Source courante | Catégorie de relecture |
| --- | --- | --- |
| `recette_policy_l04_requires_the_feature` | `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:4-14` | renommages |
| `campaign_snapshot_skips_an_active_writer_lock` | `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:690-715` | renommages |
| `l04_campaign_gate_rejects_escape_and_duplicate_trigger` | `crates/whisper-adapters/src/archive/tests/campaign.rs:1-146` | renommages |
| `r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision` | `crates/whisper-adapters/src/archive/tests/recovery.rs:1-327` | adaptation_api_types_arguments |
| `callback_reuses_recycled_slots_without_blocking_or_saturating` | `crates/whisper-adapters/src/capture.rs:839-881` | adaptation_api_types_arguments |
| `callback_marks_partial_channel_frames_as_failed` | `crates/whisper-adapters/src/capture.rs:883-914` | adaptation_api_types_arguments |
| `callback_stamps_each_block_with_the_gain_active_at_its_acquisition_boundary` | `crates/whisper-adapters/src/capture.rs:916-975` | adaptation_api_types_arguments |
| `gain_published_during_sample_copy_belongs_to_the_next_callback_block` | `crates/whisper-adapters/src/capture.rs:977-1034` | adaptation_api_types_arguments |
| `d2_corpus_real_cpu_turbo_and_parakeet` | `crates/whisper-adapters/src/direct_live/d2_corpus.rs:142-282` | virgules_formatage |
| `test_status_writer_preserves_diagnostics` | `crates/whisper-adapters/src/direct_live/d2_corpus.rs:539-558` | virgules_formatage |
| `inventory_log_appends_timestamped_partial_snapshots_and_escapes_names` | `crates/whisper-adapters/src/gpu_inventory.rs:744-790` | adaptation_api_types_arguments |
| `inventory_log_write_failure_is_reportable_without_panicking` | `crates/whisper-adapters/src/gpu_inventory.rs:792-811` | adaptation_api_types_arguments |
| `inventory_log_keeps_dxgi_adapters_that_failed_identity_join` | `crates/whisper-adapters/src/gpu_inventory.rs:813-841` | adaptation_api_types_arguments |
| `retry_epoch_discards_audio_acquired_before_the_retry_barrier` | `crates/whisper-adapters/src/microphone_probe.rs:878-959` | synchronisation_et_assertions_renforcees |
| `parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav` | `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:10-216` | fixture_structure_trace_retiree |
| `native_auto_gpu_ready_failure_retries_once_on_cpu_before_first_call` | `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:116-561` | adaptation_api_types_arguments |
| `native_personal_cpu_finalizes_a_synthetic_capture_without_opening_hardware` | `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3134-3294` | adaptation_api_types_arguments |
| `native_queued_gpu_resume_uses_current_attestation_and_rebinds_snapshot` | `crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3310-3945` | adaptation_api_types_arguments |
| `stdin_frames_preserve_live_start_and_admitted_window_before_normal_stop` | `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:1451-1520` | adaptation_api_types_arguments |
| `r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication` | `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:403-424` | renommages |
| `l3c_acceptance_fault_is_confined_to_its_canonical_execution_root` | `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:942-995` | renommages |
| `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` | `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1155-1271` | fixture_modele_deplacee |
| `unified_runtime_keeps_reserved_final_turn_through_preparation_with_peer_preview_ready` | `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2022-2472` | adaptation_api_types_arguments |
| `queued_create_only_publication_paths_reach_app_view` | `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:871-973` | adaptation_api_types_arguments |
| `resumed_create_only_publication_paths_reach_app_view` | `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:975-1405` | adaptation_api_types_arguments |
| `reprocessing_create_only_publication_paths_reach_app_view_and_preserve_earlier_result` | `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:1407-1559` | adaptation_api_types_arguments |
| `import_application_resumes_a_verified_interrupted_unified_import` | `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:1561-1964` | adaptation_api_types_arguments |
| `failed_unified_resume_preserves_interrupted_source_binding_and_queue_identity` | `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:1966-2119` | adaptation_api_types_arguments |
| `failed_unified_resume_after_binding_transfer_restores_registry_and_queue_identity` | `crates/whisper-adapters/src/worker_ipc/tests/unified_import.rs:2121-2364` | adaptation_api_types_arguments |
| `recette_policy_l2a2_requires_the_feature` | `crates/whisper-adapters/src/worker_ipc/tests/unified_support.rs:11-23` | renommages |
| `r11_duplicate_import_prompts_before_queue_and_replace_keeps_all_old_archives` | `crates/whisper-adapters/tests/import_cpu.rs:25-345` | adaptation_api_types_arguments |
| `replace_all_publishes_first_stops_partial_cleanup_and_resumes_only_on_request` | `crates/whisper-adapters/tests/reprocessing_cleanup.rs:844-1598` | adaptation_api_types_arguments |
| `application_create_new_processes_the_bound_snapshot_and_preserves_prior_publication` | `crates/whisper-adapters/tests/reprocessing_create.rs:238-403` | adaptation_api_types_arguments |
| `close_blocks_direct_and_indirect_admissions_without_submitting_effects` | `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs:26-116` | adaptation_api_types_arguments |
| `stale_duplicate_dialog_response_cannot_submit_a_replacement` | `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs:790-837` | adaptation_api_types_arguments |
| `transcribe_command_carries_complete_build_runtime_profile_and_attempt_identity` | `crates/whisper-core/tests/parakeet_ipc.rs:15-49` | adaptation_api_types_arguments |
| `disconnected_preference_worker_reports_failed_model_and_compute_saves` | `crates/whisper-desktop/tests/model_selection_ui.rs:306-340` | assertions_erreur_typee_ajoutees |
| `cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache` | `crates/whisper-worker-cpu/tests/reusable_imports.rs:112-116` | extraction_scenario_et_harnais_processus_modifies |
| `reference_wav_decoder_matches_product_decoder_and_reads_sixty_minutes` | `crates/whisper-worker-parakeet-cpu/src/reference_decoder/tests.rs:56-70` | virgules_formatage |
| `accepts_only_the_two_canonical_l2_test_roots` | `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:14-50` | fixture_et_oracle_conditionnel_features |
| `gpu_worker_allows_only_the_canonical_l3_runtime_roots` | `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:52-77` | fixture_racine_simulee |
| `qualification_profiles_are_confined_to_l2b` | `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:79-106` | fixture_et_oracle_conditionnel_features |
| `gpu_worker_rejects_wrong_device_before_audio` | `crates/whisper-worker-parakeet-cpu/tests/native_import.rs:1078-1146` | adaptation_api_types_arguments |

## Copies exactes identifiées et relues

Huit groupes de corps identiques ont augmenté depuis la référence, pour **19 occurrences physiques** et **330 lignes de fonctions** ; après conservation comptable d’une occurrence par groupe, **197 lignes** sont des copies additionnelles. Ce sont des corps normalisés, pas une preuve d’identité de tout le contexte importé. [Groupes complets](raw/test-growth-cases.json).

| Famille | Occurrences physiques | Contexte relu |
| --- | ---: | --- |
| Ouverture source inaccessible | 2 | Deux façades decoder CPU/GPU appelées via super ; chaque test vise sa façade. `crates/whisper-worker-cpu/src/decoder/tests.rs:3-11` ; `crates/whisper-worker-gpu/src/decoder/tests.rs:3-11`. |
| Canaux vides/déconnectés, Stop corrélé, protocole invalide | 6 (3×2) | Les modules CPU/GPU possèdent chacun leur take_stop ; les trois mêmes scénarios s’appliquent aux deux implémentations. `crates/whisper-worker-cpu/src/session/control/tests.rs:3-47` ; `crates/whisper-worker-gpu/src/session/control/tests.rs:3-47` ; `crates/whisper-worker-cpu/src/session/control.rs:1-10` ; `crates/whisper-worker-gpu/src/session/control.rs:1-10`. |
| Fixture enfant, arrêt au délai, stdout conservé au timeout | 9 (3×3) | Auto-tests du harnais BoundedChild, copiés dans les trois workers ; ce sont des tests d’outillage de test, pas neuf comportements produit distincts. `crates/whisper-worker-cpu/tests/support/worker.rs:213-282` ; `crates/whisper-worker-gpu/tests/support/worker.rs:213-282` ; `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:213-282`. |
| Stop/Shutdown live comparé référence/candidat | 2 | Même test texte, mais live_case est local à chacun des backends et porte sa configuration CPU/GPU. Les deux sont ignored faute de lancement explicite natif. `crates/whisper-worker-cpu/tests/worker_contract.rs:230-246` ; `crates/whisper-worker-gpu/tests/worker_contract.rs:268-284`. |

Les trois fichiers `tests/support/worker.rs` des workers sont **entièrement identiques**, 282 lignes chacun : **846 lignes physiques**, dont **564 lignes de copies supplémentaires** par rapport à un seul fichier. Ce constat dépasse le simple effet d’en-têtes. Leurs tests de délai sont inclus dans les 330 lignes ci-dessus : ces deux mesures se recouvrent et ne doivent pas être additionnées. `crates/whisper-worker-cpu/tests/support/worker.rs:1-282` ; `crates/whisper-worker-gpu/tests/support/worker.rs:1-282` ; `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs:1-282`. [Mesure annotée](raw/test-growth-cases.json).

Ces supports sont aussi inclus dans plusieurs cibles d’intégration. Dans la dernière sortie workspace, chacun des trois noms support::tests de délai apparaît **cinq fois** ; ce sont des répétitions d’exécution par cible et non des fichiers Rust physiques supplémentaires. [Sortie de validation](raw/niveau1-post-fix-workspace.stdout.txt), [compteur annoté](raw/test-growth-cases.json). [Déduction] La répétition des scénarios locaux CPU/GPU est cohérente avec les deux implémentations visées ; elle n’est pas une preuve de duplication accidentelle du même parcours dans une seule cible.

## Limites et vérifications réalisées

- Analyse statique Git, comparaison des sources en mémoire, même classificateur que la carte, partition des lignes, appariement des fonctions, lecture des différences et des copies identifiées. **Aucun Cargo ni test relancé** pour cette analyse ; aucun fichier de code, test, manifeste ou garde modifié.
- [Non vérifié] Aucun type-check historique par feature, aucune expansion de macros ni preuve complète d’équivalence sémantique des helpers. Une fonction identique peut changer de résolution d’import ou de cfg englobant ; les 706 égalités de tokens ne prouvent pas à elles seules l’identité du comportement compilé.
- [Non vérifié] Détection exhaustive des quasi-doublons, extractions partielles et algorithmes équivalents sous noms différents non revendiquée. Les huit groupes concernent les corps entiers identiques après exclusion des blancs/commentaires, littéraux conservés. Les variantes CPU/GPU de contrats ont aussi des différences explicites de backend hors de ces groupes.
- Les fonctions physiques incluent des cas cfg/ignored ; elles ne sont pas assimilées aux 773 réussites consolidées du harness actuel. Le statut des tests natifs/NOT RUN reste celui du [bilan de validation](NIVEAU1.md).
- L’inventaire des 105 nouvelles fonctions figure dans [test-growth-cases.json](raw/test-growth-cases.json), avec leurs chemins et plages ; leur présence est vérifiée, mais aucune revue exhaustive de tous leurs helpers et de toutes leurs branches n’est revendiquée ici.

Reproduction brute : `python -B docs/architecture/raw/test-growth-audit.py` pour les phases, puis le même avec `--compare` pour l’appariement global ; les annotations de relecture et la vérification des 235 items constituent des résultats complémentaires manuels/outillés documentés ici. Le script ne fait ni checkout ni écriture des snapshots : seuls les rapports sous docs/architecture/raw sont écrits. [Script](raw/test-growth-audit.py).
