Avancée : R11 et son correctif élargi R7 sont revus CLEAN ; R1 à R8 fermés, gate workspace vert sous garde.
Blocage : recette D-A8 dans la fenêtre Windows non exécutée par l'utilisateur.
Décision : candidat technique prêt ; commit local du code et du bilan, sans installation ni push.

# Bilan provisoire R11 — retraitement d'un audio déjà publié

Date : 19/10/2026. Branche : `evo-startup-perf`. Base : `e3918aab2fe7702e1f2e93e8fea7c3d7338c7446`. Candidat : arbre de travail, patch SHA-256 `D2F2CA61563A39CEE9D4F76489B5AC6E08DEC856CC000A22A82C46A7E6636DF7`.

## Réalisation et vérification

Le candidat ajoute la détection par SHA-256 des octets de l'audio dans la destination choisie, un dialogue Annuler / Remplacer / Créer un nouveau, une réservation avant file, des noms TXT/SRT numérotés, et une visibilité durable des publications remplacées. Les neuf fichiers modifiés sont exactement ceux du [plan R11](plan-retraitement-audio.md). Les ressources non suivies `models/` et `sound example/` sont préservées.

Le diagnostic R11.1 a obtenu deux publications distinctes pour deux imports successifs dans le même processus ; la réutilisation forcée du même couple job/génération reproduit `generation is already published`. La cause exacte du cas observé dans l'application installée n'est donc pas attestée par ce diagnostic.

Vérifications finales rapportées par l'implémenteur : `cargo fmt --all -- --check` PASS ; test ciblé de collision avec fichier tiers aux mêmes octets PASS ; suite `whisper-adapters` PASS ; `cargo test --offline --locked --workspace --no-fail-fast` PASS sous la garde R11, 52 groupes de résultats, 0 échec, 34 tests ignorés ; `git diff --check` PASS. La garde n'a relevé aucun écart d'écriture. Les tests ignorés ne sont pas des preuves de réussite. La recette D-A8 dans la fenêtre Windows est NOT RUN.

## Revue indépendante et réserve

Deux cycles de revue/correction ont été utilisés. Les constats R1 à R4 sont fermés. Le verdict final est **LIVRÉ AVEC RÉSERVES** en raison de R5, classé BLOQUANT pour D-A5.

Dans `archive.rs`, `commit_publish` appelle `install_publication_exports` avant d'écrire `CURRENT`. Si le processus s'interrompt après la création d'un export et avant son marqueur de propriété, la reprise dans une nouvelle génération retrouve ce chemin sans provenance et le refuse comme collision. La coupure `import.before_current` et la reprise du pending donnent le scénario de reproduction. Le refus protège les fichiers tiers, mais empêche de terminer cette reprise. La correction devra récupérer uniquement les exports dont la propriété peut être prouvée, même si un fichier tiers a exactement les mêmes octets ; elle devra préserver les anciennes publications actives jusqu'au commit.

Références de revue : `crates/whisper-adapters/src/archive.rs::commit_publish` lignes 2533–2536 ; `install_publication_exports` lignes 2887–2935 ; `resume_from_with_qualification` lignes 2273–2307. Un échec après le premier des deux exports peut créer le même blocage.

## Reprise demandée par l'utilisateur après le premier verdict

L'utilisateur a autorisé de nouveaux cycles de revue/correction pour lever les réserves. Le correctif R5 a été limité à `archive.rs` et `import_cpu.rs`. Il couvre deux interruptions : après la création du seul TXT, puis après les deux exports et avant `CURRENT`. La reprise dans une nouvelle génération aboutit, les anciennes publications restant actives jusqu'au commit. Le refus d'un fichier tiers ayant les mêmes octets reste testé.

La première nouvelle revue a fermé R5, puis ouvert R6 : un lien physique direct entre export et artefact interne permettait à l'édition du TXT exporté de corrompre l'artefact vérifié par le manifeste. Le correctif a isolé l'artefact interne avec une copie préparée distincte. Le test ciblé vérifie l'édition de l'export, le SHA interne, le scan d'archive et la recherche de doublon. La seconde revue a fermé R6 et confirmé R1 à R5 fermés.

La seconde revue a ouvert **R7 BLOQUANT** : `archive.rs::install_publication_exports`, lignes 2937–2940, exige `fs::hard_link` sans alternative. Une destination exFAT/FAT32 reste sélectionnable, mais ne permet pas cette publication TXT/SRT, contrairement à D-A3/D-A4. La preuve de fermeture demandée est : premier import et Remplacer sur une destination sans liens physiques, TXT/SRT vérifiés, reprise avant `CURRENT`, et fichier tiers en collision intact.

Le nouveau candidat conserve exactement les neuf chemins de code R11. Formatage et suite workspace complète sous garde : PASS, 52 groupes, 0 échec, 34 tests ignorés. La recette D-A8 dans la fenêtre Windows reste NOT RUN. Le diff de ces neuf chemins par rapport à `e3918aab` est identifié par le SHA-1 Git `5344ad96ef3526998978e1a0a1894dcba88669e6` selon le reviewer. Les deux cycles de ce nouveau passage sont épuisés ; aucune correction supplémentaire n'a été lancée.

Conformément au plan, R11 n'est pas livrable tant que D-A1 à D-A7 ne sont pas satisfaits ensemble. Le candidat reste non commité et l'installation personnelle n'est pas modifiée.

## Lot correctif R7 demandé ensuite

Le correctif R7 a été limité à `crates/whisper-adapters/src/archive.rs` et `crates/whisper-adapters/tests/import_cpu.rs`. Une injection force l'absence de liens physiques ; le repli crée les sorties par `create_new` sans écraser un fichier tiers. Les tests adaptateur passent (107 tests de bibliothèque, 2 ignorés), de même que le formatage, sous garde. Ils couvrent la reprise après un ou deux exports dont la propriété est attestée et le refus d'une collision avec un fichier tiers aux mêmes octets.

Une coupure immédiatement après `create_new`, avant la persistance de l'identité du fichier final, laisse toutefois un export sans provenance. La reprise refuse ce fichier et conserve le pending et les anciennes publications ; le test n'aboutit qu'après suppression manuelle du fichier ambigu. Le reviewer indépendant classe donc **R7 BLOQUANT** pour D-A5/D-A7. Le gate workspace complet de ce candidat est **NOT RUN**, arrêté sur ce défaut. Aucun code R7 n'a été commité.

L'adjudication du reviewer situe le refus dans `archive.rs::install_owned_export` vers la ligne 3224 et la reprise vers la ligne 3176. Elle conclut qu'une résolution sans adoption d'un fichier tiers demande une nouvelle confirmation visible et la transmission de ce choix. Les chemins supplémentaires minimaux sont `crates/whisper-desktop/src/ui.rs`, `crates/whisper-core/src/ports.rs`, `crates/whisper-core/src/application.rs` et `crates/whisper-adapters/src/worker_ipc.rs`. Ils figurent dans les neuf chemins du plan R11, mais pas dans le sous-périmètre R7 autorisé à ce stade. Aucune édition supplémentaire n'a été engagée.

## Correctif R7 élargi et clôture technique

L'utilisateur a autorisé l'extension aux quatre chemins ci-dessus. Le candidat réutilise la confirmation visible de l'application : après une coupure qui laisse un export final sans identité persistée, il garde ce fichier intact, demande un nouveau nom et n'engage la génération suivante qu'après le choix explicite. La voie sans liens physiques publie par création exclusive. Les artefacts internes restent indépendants des TXT/SRT modifiables ; une collision avec un fichier tiers aux mêmes octets ne lui transfère aucune propriété.

Un test intégré déterministe couvre premier import et Remplacer : absence forcée de liens physiques, coupure après création exclusive du SRT, redémarrage, `ResumeInterrupted`, dialogue visible, choix du nom suffixé, reprise du préfixe et publication de la génération suivante. Il vérifie TXT/SRT, anciennes publications encore actives avant le commit et fichier tiers inchangé. La première revue du candidat élargi demandait cette preuve intégrée et le rétablissement des fins de ligne LF de `worker_ipc.rs` ; la seconde revue a déclaré **CLEAN**, R7/R8 fermés, R1–R6 toujours fermés. Deux revues et une correction ont été utilisées.

Vérifications finales sous garde R11 : formatage PASS ; `git diff --check` PASS ; tests adapters PASS (109 tests de bibliothèque réussis, 2 ignorés) ; `cargo test --offline --locked --workspace --no-fail-fast` PASS (code 0, aucun échec). La garde n'a relevé aucun écart d'écriture. Espace libre final rapporté : 53,73 Gio. L'empreinte SHA-256 indépendante du diff brut du candidat, avant commit, est `391d9b1bf0886941807aca7c841de5acd77163adc956c1d6486af25a3ec257b4` selon le reviewer.

Les 34 tests ignorés/NOT RUN du gate workspace sont listés ci-dessous ; leurs recettes natives ou matérielles ne sont pas requalifiées par R11 :

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

La recette D-A8 avec un vrai audio déjà traité dans la fenêtre Windows reste **NOT RUN**. Le code est prêt pour cette vérification utilisateur ; l'installation personnelle `%LOCALAPPDATA%\Whisper` n'est pas modifiée par R11.
