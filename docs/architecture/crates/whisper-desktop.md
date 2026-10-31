Avancée : relevé statique actualisé au 31 octobre 2026.
Blocage : exécution native et outils absents [Non vérifié].
Décision : description de l’existant, sans proposition de transformation.

# whisper-desktop

## Rôle et type

Compose application et adaptateurs puis ouvre une fenêtre eframe/Glow. `crates/whisper-desktop/src/main.rs:1-33` ; `crates/whisper-desktop/src/root.rs:1-1656` ; `crates/whisper-desktop/src/lib.rs:1-56` ; `crates/whisper-desktop/src/ui/model/controller/actions.rs:1-353` ; `crates/whisper-desktop/src/ui/model/view.rs:1-51`

Les contrôleurs construisent les commandes import/micro et les vues présentent AppView. `crates/whisper-desktop/src/main.rs:1-33` ; `crates/whisper-desktop/src/root.rs:1-1656` ; `crates/whisper-desktop/src/lib.rs:1-56` ; `crates/whisper-desktop/src/ui/model/controller/actions.rs:1-353` ; `crates/whisper-desktop/src/ui/model/view.rs:1-51`

Type : **Binaire et bibliothèque de présentation**. `crates/whisper-desktop/src/main.rs:1-33` ; `crates/whisper-desktop/src/root.rs:1-1656` ; `crates/whisper-desktop/src/lib.rs:1-56` ; `crates/whisper-desktop/src/ui/model/controller/actions.rs:1-353` ; `crates/whisper-desktop/src/ui/model/view.rs:1-51`

Mesures : **5285 production / 4941 tests** ; lignes physiques, vides/commentaires inclus. Sources par fichier ci-dessous ; [méthode](../raw/METHOD.md).

## Dépendances déclarées

| Dépendance | Nature | Groupe/condition | Déclaration | Source |
| --- | --- | --- | --- | --- |
| whisper-core | interne | dependencies | {"workspace": true} | `crates/whisper-desktop/Cargo.toml:9` |
| whisper-adapters | interne | dependencies | {"workspace": true} | `crates/whisper-desktop/Cargo.toml:10` |
| eframe | externe | dependencies | {"version": "=0.35.0", "default-features": false, "features": ["accesskit", "default_fonts", "glow"]} | `crates/whisper-desktop/Cargo.toml:11` |

Metadata et Clippy workspace/all-targets ont terminé avec code 0 ; [résultats](../raw/cargo-results.json). [Non vérifié] Transitives (metadata sans dépendances), autres combinaisons de features et exécution des tests. Features déclarées : `crates/whisper-desktop/Cargo.toml:1-19`.

## Interfaces publiques et usages intercrates

[Non vérifié] Aucun import consommateur d’une autre crate relevé ; les autres usages ne sont pas exclus.

Interfaces principales avec définitions et sites d’usage (pour desktop, entre cibles du même package) :

| Interface | Définition | Usage observé |
| --- | --- | --- |
| `run` | `crates/whisper-desktop/src/lib.rs:18-56` | `crates/whisper-desktop/src/main.rs:5-33` |
| `DesktopStartup` | `crates/whisper-desktop/src/lib.rs:6-16` | `crates/whisper-desktop/src/main.rs:5-33` |

Définitions publiques détaillées par module ci-dessous ; réexportations de racine : `crates/whisper-desktop/src/lib.rs:1-56`. Imports textuels, pas un graphe de symboles résolu.

## Frontières constatées

| Frontière (repérage textuel) | Sites hors tests, sous leurs cfg |
| --- | --- |
| Disque | `crates/whisper-desktop/src/reactivity_log.rs:3` ; `crates/whisper-desktop/src/ui/model/view/capture.rs:208` |
| Processus | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Threads/canaux | `crates/whisper-desktop/src/lib.rs:11` ; `crates/whisper-desktop/src/reactivity_log.rs:9` ; `crates/whisper-desktop/src/root.rs:3` ; `crates/whisper-desktop/src/ui/model/preferences.rs:64` ; `crates/whisper-desktop/src/ui/model.rs:7` |
| Entrées/sorties standard | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| GPU/natif | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Réseau explicite | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |

Les sites désignent du code présent, pas une exécution ; [lignes brutes](../raw/rg-threads-processes-io.txt).

## Carte des fichiers et modules

| Module/cible | Responsabilité et source | Prod | Tests |
| --- | --- | --- | --- |
| crate | Déclare les modules et interfaces de bibliothèque. `crates/whisper-desktop/src/lib.rs:1-56` | 56 | 0 |
| crate | Compose le desktop et ouvre la fenêtre. `crates/whisper-desktop/src/main.rs:1-33` | 33 | 0 |
| crate::reactivity_log | Écrit les jalons de réactivité via canal et thread de fichier. `crates/whisper-desktop/src/reactivity_log.rs:1-245` | 153 | 92 |
| crate::root | Compose services, profils, catalogues et gardes d’admission des commandes. `crates/whisper-desktop/src/root.rs:1-1656` | 778 | 878 |
| crate::ui::model::controller::actions | Construit les demandes import/live et commandes gain/test micro. `crates/whisper-desktop/src/ui/model/controller/actions.rs:1-353` | 353 | 0 |
| crate::ui::model::controller::observability | Enregistre les jalons de rendu et de réactivité. `crates/whisper-desktop/src/ui/model/controller/observability.rs:1-99` | 99 | 0 |
| crate::ui::model::controller | Draine les entrées de frame et transmet les commandes à Application. `crates/whisper-desktop/src/ui/model/controller.rs:1-375` | 375 | 0 |
| crate::ui::model::preferences | Définit commandes/événements de préférences et leur canal. `crates/whisper-desktop/src/ui/model/preferences.rs:1-121` | 121 | 0 |
| crate::ui::model::view::actions | Affiche les actions démarrage/arrêt et transmet leurs commandes. `crates/whisper-desktop/src/ui/model/view/actions.rs:1-106` | 97 | 9 |
| crate::ui::model::view::active_job | Affiche métriques et texte du travail live actif. `crates/whisper-desktop/src/ui/model/view/active_job.rs:1-125` | 125 | 0 |
| crate::ui::model::view::capture | Affiche cartes de capture Parakeet, finalisation et sélection du texte. `crates/whisper-desktop/src/ui/model/view/capture.rs:1-352` | 352 | 0 |
| crate::ui::model::view::dialogs | Affiche confirmations et choix de doublons/récupération. `crates/whisper-desktop/src/ui/model/view/dialogs.rs:1-104` | 104 | 0 |
| crate::ui::model::view::history | Affiche historique des traitements et retraits de remplacement. `crates/whisper-desktop/src/ui/model/view/history.rs:1-146` | 146 | 0 |
| crate::ui::model::view::microphone | Dessine spectre, source, test, niveau et gain microphone. `crates/whisper-desktop/src/ui/model/view/microphone.rs:1-333` | 317 | 16 |
| crate::ui::model::view::notices | Affiche résultats publiés et notifications. `crates/whisper-desktop/src/ui/model/view/notices.rs:1-52` | 52 | 0 |
| crate::ui::model::view::queue | Affiche file et boutons Traiter, Reprendre et Retirer. `crates/whisper-desktop/src/ui/model/view/queue.rs:1-77` | 77 | 0 |
| crate::ui::model::view::settings | Affiche chemins, langue, calcul et préférence GPU. `crates/whisper-desktop/src/ui/model/view/settings.rs:1-208` | 208 | 0 |
| crate::ui::model::view::unified | Affiche sessions unifiées et pages document/audio. `crates/whisper-desktop/src/ui/model/view/unified.rs:1-424` | 424 | 0 |
| crate::ui::model::view | Implémente eframe::App et compose les panneaux de la fenêtre. `crates/whisper-desktop/src/ui/model/view.rs:1-51` | 51 | 0 |
| crate::ui::model | Définit DesktopApp et construit état d’interaction et canaux. `crates/whisper-desktop/src/ui/model.rs:1-285` | 247 | 38 |
| crate::ui::presentation::gpu | Construit libellés, options et contrôles de sélection GPU. `crates/whisper-desktop/src/ui/presentation/gpu.rs:1-243` | 243 | 0 |
| crate::ui::presentation::language | Construit les choix et libellés de langue selon le calcul. `crates/whisper-desktop/src/ui/presentation/language.rs:1-138` | 138 | 0 |
| crate::ui::presentation::microphone | Construit les libellés de sources et mesures présentées du microphone. `crates/whisper-desktop/src/ui/presentation/microphone.rs:1-170` | 170 | 0 |
| crate::ui::presentation::paths | Présente chemins, valeurs par défaut et empreintes de modèle. `crates/whisper-desktop/src/ui/presentation/paths.rs:1-97` | 97 | 0 |
| crate::ui::presentation::recovery | Affiche le widget de récupération des snapshots source. `crates/whisper-desktop/src/ui/presentation/recovery.rs:1-42` | 42 | 0 |
| crate::ui::presentation::selection | Projette les préférences et construit les sélections modèle/langue/calcul. `crates/whisper-desktop/src/ui/presentation/selection.rs:1-269` | 269 | 0 |
| crate::ui::presentation::text | Construit les libellés de phases et le texte provisoire à présenter. `crates/whisper-desktop/src/ui/presentation/text.rs:1-79` | 79 | 0 |
| crate::ui::presentation | Rassemble imports/réexportations des fonctions de présentation. `crates/whisper-desktop/src/ui/presentation.rs:1-59` | 59 | 0 |
| crate::ui::model::catalog_fixture | Tests ou supports : définit `install_test_model_catalog` `crates/whisper-desktop/src/ui/tests/catalog_fixture.rs:4-27`. | 0 | 28 |
| crate::ui::model::tests | Tests ou supports : définit `enqueue_then_stop_still_targets_the_live_identity` `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:2-21`, `import_uses_selected_gpu` `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:24-35`. | 0 | 436 |
| crate::ui::model::tests | Tests ou supports : définit `provisional_display_removes_only_a_proven_confirmed_prefix` `crates/whisper-desktop/src/ui/tests/document_presentation.rs:2-54`, `unified_ui_renders_one_bounded_page_preview_parts_outputs_and_wait_state` `crates/whisper-desktop/src/ui/tests/document_presentation.rs:57-165`. | 0 | 360 |
| crate::ui::model::tests | Tests ou supports : définit `empty_snapshot_recovery_result_does_not_render_a_blank_modal` `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:2-16`, `parakeet_preserves_the_selected_compute_preference` `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:19-30`. | 0 | 93 |
| crate::ui::model::tests | Tests ou supports : définit `auto_stop_is_disabled_by_default_and_uses_speech_sample_clock_when_enabled` `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:2-6`, `import_enqueue_waits_during_live_capture` `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:9-12`. | 0 | 486 |
| crate::ui::model::tests | Tests ou supports : définit `speech_limit_samples` `crates/whisper-desktop/src/ui/tests/root_support.rs:2-4`. | 0 | 4 |
| crate::ui::model::tests | Tests ou supports : définit `dispatch` `crates/whisper-desktop/src/ui/tests/support.rs:33-36`, `view` `crates/whisper-desktop/src/ui/tests/support.rs:38-40`. | 0 | 569 |
| crate::ui::model::tests | Tests ou supports : définit `desktop_parakeet_handlers_select_unified_routes_only_for_enabled_candidate` `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:2-82`, `unified_finalizing_capture_keeps_live_button_available_for_second_capture_only` `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:85-172`. | 0 | 798 |
| crate::ui::model::tests | Tests ou supports : déclare les sous-modules/réexportations de `crates/whisper-desktop/src/ui/tests.rs:1-7`. | 0 | 7 |
| crate::ui | Réexporte modèle UI et fonctions de présentation. `crates/whisper-desktop/src/ui.rs:1-20` | 20 | 0 |
| integration::gpu_inventory_ui | Tests ou supports : définit `inventory_status_does_not_render_the_removed_fixed_card_list` `crates/whisper-desktop/tests/gpu_inventory_ui.rs:58-80`, `status_area_distinguishes_loading_empty_partial_and_failure` `crates/whisper-desktop/tests/gpu_inventory_ui.rs:83-104`. | 0 | 275 |
| integration::gpu_selection_ui | Tests ou supports : définit `homonymous_devices_get_numbered_labels_and_keep_distinct_internal_identity` `crates/whisper-desktop/tests/gpu_selection_ui.rs:15-32`, `unique_names_have_no_suffix_and_real_combo_has_no_clear_preference_option` `crates/whisper-desktop/tests/gpu_selection_ui.rs:35-60`. | 0 | 224 |
| integration::microphone_level_ui | Tests ou supports : définit `meter_presentation_distinguishes_waiting_absent_silence_stale_and_old_capture` `crates/whisper-desktop/tests/microphone_level_ui.rs:42-98`, `microphone_labels_changed_default_notice_uses_names_and_keeps_identity_comparison` `crates/whisper-desktop/tests/microphone_level_ui.rs:101-164`. | 0 | 164 |
| integration::model_selection_ui | Tests ou supports : définit `snapshot_renders_catalog_models_and_all_turbo_languages_without_model_path_input` `crates/whisper-desktop/tests/model_selection_ui.rs:18-70`, `selection_ui_rendering_does_not_hash_model_or_runtime_files` `crates/whisper-desktop/tests/model_selection_ui.rs:73-104`. | 0 | 384 |
| integration::reprocessing_recovery_ui | Tests ou supports : définit `recovery_ui_displays_protected_and_recoverable_entries_without_implicit_recovery` `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:6-59`, `capacity_action_is_available_only_when_the_application_offers_recovery` `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:62-80`. | 0 | 80 |

## Déclarations de modules

| Déclaration | Forme | Contexte | Cible/plage | Source |
| --- | --- | --- | --- | --- |
| reactivity_log | externe | hors tests / cfg | crates/whisper-desktop/src/reactivity_log.rs | `crates/whisper-desktop/src/lib.rs:3` |
| ui | externe | hors tests / cfg | crates/whisper-desktop/src/ui.rs | `crates/whisper-desktop/src/lib.rs:4` |
| root | externe | hors tests / cfg | crates/whisper-desktop/src/root.rs | `crates/whisper-desktop/src/main.rs:3` |
| tests | inline | tests | plage locale | `crates/whisper-desktop/src/reactivity_log.rs:165-245` |
| tests | inline | tests | plage locale | `crates/whisper-desktop/src/root.rs:779-1656` |
| actions | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/controller/actions.rs | `crates/whisper-desktop/src/ui/model/controller.rs:2` |
| observability | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/controller/observability.rs | `crates/whisper-desktop/src/ui/model/controller.rs:3` |
| actions | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/actions.rs | `crates/whisper-desktop/src/ui/model/view.rs:2` |
| active_job | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/active_job.rs | `crates/whisper-desktop/src/ui/model/view.rs:3` |
| capture | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/capture.rs | `crates/whisper-desktop/src/ui/model/view.rs:4` |
| dialogs | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/dialogs.rs | `crates/whisper-desktop/src/ui/model/view.rs:5` |
| history | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/history.rs | `crates/whisper-desktop/src/ui/model/view.rs:6` |
| microphone | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/microphone.rs | `crates/whisper-desktop/src/ui/model/view.rs:7` |
| notices | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/notices.rs | `crates/whisper-desktop/src/ui/model/view.rs:8` |
| queue | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/queue.rs | `crates/whisper-desktop/src/ui/model/view.rs:9` |
| settings | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/settings.rs | `crates/whisper-desktop/src/ui/model/view.rs:10` |
| unified | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view/unified.rs | `crates/whisper-desktop/src/ui/model/view.rs:11` |
| controller | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/controller.rs | `crates/whisper-desktop/src/ui/model.rs:23` |
| preferences | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/preferences.rs | `crates/whisper-desktop/src/ui/model.rs:24` |
| view | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model/view.rs | `crates/whisper-desktop/src/ui/model.rs:31` |
| catalog_fixture | externe | tests | crates/whisper-desktop/src/ui/tests/catalog_fixture.rs | `crates/whisper-desktop/src/ui/model.rs:37-39` |
| tests | externe | tests | crates/whisper-desktop/src/ui/tests.rs | `crates/whisper-desktop/src/ui/model.rs:40-42` |
| gpu | externe | hors tests / cfg | crates/whisper-desktop/src/ui/presentation/gpu.rs | `crates/whisper-desktop/src/ui/presentation.rs:16` |
| language | externe | hors tests / cfg | crates/whisper-desktop/src/ui/presentation/language.rs | `crates/whisper-desktop/src/ui/presentation.rs:17` |
| microphone | externe | hors tests / cfg | crates/whisper-desktop/src/ui/presentation/microphone.rs | `crates/whisper-desktop/src/ui/presentation.rs:18` |
| paths | externe | hors tests / cfg | crates/whisper-desktop/src/ui/presentation/paths.rs | `crates/whisper-desktop/src/ui/presentation.rs:19` |
| recovery | externe | hors tests / cfg | crates/whisper-desktop/src/ui/presentation/recovery.rs | `crates/whisper-desktop/src/ui/presentation.rs:20` |
| selection | externe | hors tests / cfg | crates/whisper-desktop/src/ui/presentation/selection.rs | `crates/whisper-desktop/src/ui/presentation.rs:21` |
| text | externe | hors tests / cfg | crates/whisper-desktop/src/ui/presentation/text.rs | `crates/whisper-desktop/src/ui/presentation.rs:22` |
| model | externe | hors tests / cfg | crates/whisper-desktop/src/ui/model.rs | `crates/whisper-desktop/src/ui.rs:1` |
| presentation | externe | hors tests / cfg | crates/whisper-desktop/src/ui/presentation.rs | `crates/whisper-desktop/src/ui.rs:2` |

## Détail des modules

[Non vérifié] Relations `use crate::` textuelles : alias, réexportations, glob imports, chemins `self`/`super`, références hors `use`, macros et cibles distinctes ne forment pas ici un graphe complet.

### crate — `crates/whisper-desktop/src/lib.rs`

Déclare les modules et interfaces de bibliothèque. `crates/whisper-desktop/src/lib.rs:1-56`

Mesure : 56 production / 0 tests / 56 physiques. `crates/whisper-desktop/src/lib.rs:1-56`

Types : `DesktopStartup` (struct, hors tests) `crates/whisper-desktop/src/lib.rs:6-16`.

Fonctions avec corps : `run` `crates/whisper-desktop/src/lib.rs:18-56`.

Déclarations `pub` (visibilité sous conditions) : `pub mod ui;` `crates/whisper-desktop/src/lib.rs:4`; `pub struct DesktopStartup {` `crates/whisper-desktop/src/lib.rs:6`; `pub fn run<A: whisper_core::Application + 'static>(` `crates/whisper-desktop/src/lib.rs:18`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/lib.rs:1-56`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate — `crates/whisper-desktop/src/main.rs`

Compose le desktop et ouvre la fenêtre. `crates/whisper-desktop/src/main.rs:1-33`

Mesure : 33 production / 0 tests / 33 physiques. `crates/whisper-desktop/src/main.rs:1-33`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/main.rs:1-33`.

Fonctions avec corps : `main` `crates/whisper-desktop/src/main.rs:5-33`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/main.rs:1-33`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/main.rs:1-33`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::reactivity_log — `crates/whisper-desktop/src/reactivity_log.rs`

Écrit les jalons de réactivité via canal et thread de fichier. `crates/whisper-desktop/src/reactivity_log.rs:1-245`

Mesure : 153 production / 92 tests / 245 physiques. `crates/whisper-desktop/src/reactivity_log.rs:1-245`

Types : `Entry` (struct, hors tests) `crates/whisper-desktop/src/reactivity_log.rs:13-18`; `ReactivityLog` (struct, hors tests) `crates/whisper-desktop/src/reactivity_log.rs:20-25`.

Fonctions avec corps : `start` `crates/whisper-desktop/src/reactivity_log.rs:29-65`; `start` [tests] `crates/whisper-desktop/src/reactivity_log.rs:68-77`; `record_once` `crates/whisper-desktop/src/reactivity_log.rs:79-109`; `stop` `crates/whisper-desktop/src/reactivity_log.rs:111-118`; `drop` `crates/whisper-desktop/src/reactivity_log.rs:122-124`; `log_path` `crates/whisper-desktop/src/reactivity_log.rs:128-132`; `write_entries` `crates/whisper-desktop/src/reactivity_log.rs:134-143`; `format_entry` `crates/whisper-desktop/src/reactivity_log.rs:145-155`; `safe_field` `crates/whisper-desktop/src/reactivity_log.rs:157-159`; `should_record` `crates/whisper-desktop/src/reactivity_log.rs:161-163`; `formats_absolute_timestamp_and_duration_without_multiline_fields` [tests] `crates/whisper-desktop/src/reactivity_log.rs:172-183`; `records_each_milestone_once_per_session` [tests] `crates/whisper-desktop/src/reactivity_log.rs:186-192`; `writer_preserves_event_order` [tests] `crates/whisper-desktop/src/reactivity_log.rs:195-216`; `dropping_logger_drains_all_previously_sent_entries` [tests] `crates/whisper-desktop/src/reactivity_log.rs:219-244`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/reactivity_log.rs:1-245`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/reactivity_log.rs:1-245`.

Utilisé par, via ces imports : hors tests : `crate::ui::model` `crates/whisper-desktop/src/ui/model.rs:1`.

#### Inline `tests`

Tests/auxiliaires : `formats_absolute_timestamp_and_duration_without_multiline_fields` `crates/whisper-desktop/src/reactivity_log.rs:172-183`, `records_each_milestone_once_per_session` `crates/whisper-desktop/src/reactivity_log.rs:186-192`, `writer_preserves_event_order` `crates/whisper-desktop/src/reactivity_log.rs:195-216`, `dropping_logger_drains_all_previously_sent_entries` `crates/whisper-desktop/src/reactivity_log.rs:219-244`. `crates/whisper-desktop/src/reactivity_log.rs:165-245`

Mesure locale : 0 production / 81 tests ; sous-ensemble non additionnable. `crates/whisper-desktop/src/reactivity_log.rs:165-245`

Types locaux : aucun relevé dans `crates/whisper-desktop/src/reactivity_log.rs:165-245`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-desktop/src/reactivity_log.rs:165-245`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::root — `crates/whisper-desktop/src/root.rs`

Compose services, profils, catalogues et gardes d’admission des commandes. `crates/whisper-desktop/src/root.rs:1-1656`

Mesure : 778 production / 878 tests / 1656 physiques. `crates/whisper-desktop/src/root.rs:1-1656`

Types : `DesktopComposition` (struct, hors tests) `crates/whisper-desktop/src/root.rs:32-41`; `DesktopAdmissionApplication` (struct, hors tests) `crates/whisper-desktop/src/root.rs:562-574`; `RecordingIo` (struct, tests) `crates/whisper-desktop/src/root.rs:791-794`.

Fonctions avec corps : `compose` `crates/whisper-desktop/src/root.rs:43-319`; `configured_parakeet_worker` `crates/whisper-desktop/src/root.rs:321-342`; `configured_gpu_parakeet_worker` `crates/whisper-desktop/src/root.rs:344-360`; `personal_install_root` `crates/whisper-desktop/src/root.rs:362-369`; `personal_model_root` `crates/whisper-desktop/src/root.rs:371-373`; `personal_settings_path` `crates/whisper-desktop/src/root.rs:375-377`; `reprocessing_snapshot_registry_root` `crates/whisper-desktop/src/root.rs:379-385`; `personal_worker_paths` `crates/whisper-desktop/src/root.rs:387-405`; `qualify_parakeet_catalog_entry` `crates/whisper-desktop/src/root.rs:407-433`; `parakeet_gpu_route_is_qualified` `crates/whisper-desktop/src/root.rs:435-451`; `gpu_worker_file_metadata` `crates/whisper-desktop/src/root.rs:453-475`; `parakeet_worker_runtime_is_qualified` `crates/whisper-desktop/src/root.rs:477-492`; `file_metadata` `crates/whisper-desktop/src/root.rs:494-499`; `parakeet_artifact_matches` `crates/whisper-desktop/src/root.rs:501-508`; `parakeet_available` `crates/whisper-desktop/src/root.rs:510-514`; `apply_parakeet_key_availability` `crates/whisper-desktop/src/root.rs:516-539`; `apply_parakeet_gpu_availability` `crates/whisper-desktop/src/root.rs:541-560`; `guarded_application` `crates/whisper-desktop/src/root.rs:576-619`; `poll_admission_catalog` `crates/whisper-desktop/src/root.rs:622-649`; `admission_result` `crates/whisper-desktop/src/root.rs:651-691`; `core_refuses_before_effect` `crates/whisper-desktop/src/root.rs:693-717`; `dispatch` `crates/whisper-desktop/src/root.rs:721-762`; `view` `crates/whisper-desktop/src/root.rs:764-766`; `set_gpu_candidate` `crates/whisper-desktop/src/root.rs:768-776`; `submit` [tests] `crates/whisper-desktop/src/root.rs:797-800`; `poll` [tests] `crates/whisper-desktop/src/root.rs:802-804`; `fixture_root` [tests] `crates/whisper-desktop/src/root.rs:807-831`; `is_allowed_campaign_root` [tests] `crates/whisper-desktop/src/root.rs:833-847`; `fixture_root_uses_canonical_l3b_campaign_root_when_configured` [tests] `crates/whisper-desktop/src/root.rs:850-861`; `fixture_root_accepts_only_the_canonical_l3c_campaign_root` [tests] `crates/whisper-desktop/src/root.rs:864-888`; `artifact_metadata_detects_replacement_with_same_size_and_modified_time` [tests] `crates/whisper-desktop/src/root.rs:891-922`; `turbo_sha` [tests] `crates/whisper-desktop/src/root.rs:924-934`; `model_sha` [tests] `crates/whisper-desktop/src/root.rs:936-943`; `request` [tests] `crates/whisper-desktop/src/root.rs:945-961`; `admission_snapshot` [tests] `crates/whisper-desktop/src/root.rs:963-994`; `admission_receiver` [tests] `crates/whisper-desktop/src/root.rs:996-1000`; `parakeet_selection_and_admission_require_the_exact_test_key` [tests] `crates/whisper-desktop/src/root.rs:1003-1101`; `valid_cpu_route_remains_ready_when_gpu_route_is_missing_or_mismatched` [tests] `crates/whisper-desktop/src/root.rs:1104-1140`; `parakeet_worker_key_changes_regray_and_readmit_without_restarting` [tests] `crates/whisper-desktop/src/root.rs:1144-1268`; `composed_guard_rejects_direct_unknown_model_before_any_effect` [tests] `crates/whisper-desktop/src/root.rs:1271-1297`; `observed_missing_model_is_refused_before_any_effect` [tests] `crates/whisper-desktop/src/root.rs:1300-1339`; `unified_live_start_applies_its_staged_candidate_before_the_start_effect` [tests] `crates/whisper-desktop/src/root.rs:1342-1399`; `refused_ui_request_drops_staged_gpu_candidate_and_refresh_keeps_notice` [tests] `crates/whisper-desktop/src/root.rs:1402-1448`; `active_job_refuses_interrupted_resume_without_gpu_candidate_effect` [tests] `crates/whisper-desktop/src/root.rs:1451-1490`; `terminal_job_state_allows_new_gpu_start_with_candidate_before_start_effect` [tests] `crates/whisper-desktop/src/root.rs:1493-1565`; `composed_catalog_rejects_turbo_changed_after_admission_before_worker_start` [tests] `crates/whisper-desktop/src/root.rs:1568-1628`; `personal_profile_uses_only_the_stable_model_worker_and_settings_layout` [tests] `crates/whisper-desktop/src/root.rs:1632-1655`.

Déclarations `pub` (visibilité sous conditions) : `pub struct DesktopComposition {` `crates/whisper-desktop/src/root.rs:32`; `pub fn compose() -> DesktopComposition {` `crates/whisper-desktop/src/root.rs:43`; `pub struct DesktopAdmissionApplication<P> {` `crates/whisper-desktop/src/root.rs:562`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/root.rs:1-1656`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

#### Inline `tests`

Tests/auxiliaires : `submit` `crates/whisper-desktop/src/root.rs:797-800`, `poll` `crates/whisper-desktop/src/root.rs:802-804`, `fixture_root` `crates/whisper-desktop/src/root.rs:807-831`, `is_allowed_campaign_root` `crates/whisper-desktop/src/root.rs:833-847`. `crates/whisper-desktop/src/root.rs:779-1656`

Mesure locale : 0 production / 878 tests ; sous-ensemble non additionnable. `crates/whisper-desktop/src/root.rs:779-1656`

Types locaux : `RecordingIo` `crates/whisper-desktop/src/root.rs:791-794`.

Imports locaux : aucun `use crate::` relevé dans `crates/whisper-desktop/src/root.rs:779-1656`. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.

### crate::ui::model::controller::actions — `crates/whisper-desktop/src/ui/model/controller/actions.rs`

Construit les demandes import/live et commandes gain/test micro. `crates/whisper-desktop/src/ui/model/controller/actions.rs:1-353`

Mesure : 353 production / 0 tests / 353 physiques. `crates/whisper-desktop/src/ui/model/controller/actions.rs:1-353`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/controller/actions.rs:1-353`.

Fonctions avec corps : `next_import_job_id` `crates/whisper-desktop/src/ui/model/controller/actions.rs:3-23`; `queued_request_for_compute` `crates/whisper-desktop/src/ui/model/controller/actions.rs:25-32`; `command_compute_choice` `crates/whisper-desktop/src/ui/model/controller/actions.rs:34-45`; `gpu_selection_identity` `crates/whisper-desktop/src/ui/model/controller/actions.rs:47-58`; `start_import` `crates/whisper-desktop/src/ui/model/controller/actions.rs:61-101`; `start_microphone_test` `crates/whisper-desktop/src/ui/model/controller/actions.rs:103-142`; `set_microphone_gain` `crates/whisper-desktop/src/ui/model/controller/actions.rs:144-194`; `active_unified_capture_identity` `crates/whisper-desktop/src/ui/model/controller/actions.rs:196-213`; `stop_microphone_test` `crates/whisper-desktop/src/ui/model/controller/actions.rs:215-220`; `start_live` `crates/whisper-desktop/src/ui/model/controller/actions.rs:222-305`; `microphone_for_live` `crates/whisper-desktop/src/ui/model/controller/actions.rs:307-352`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/controller/actions.rs:1-353`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/controller/actions.rs:1-353`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::controller::observability — `crates/whisper-desktop/src/ui/model/controller/observability.rs`

Enregistre les jalons de rendu et de réactivité. `crates/whisper-desktop/src/ui/model/controller/observability.rs:1-99`

Mesure : 99 production / 0 tests / 99 physiques. `crates/whisper-desktop/src/ui/model/controller/observability.rs:1-99`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/controller/observability.rs:1-99`.

Fonctions avec corps : `unix_time_ms` `crates/whisper-desktop/src/ui/model/controller/observability.rs:3-9`; `process_log_session` `crates/whisper-desktop/src/ui/model/controller/observability.rs:11-13`; `live_log_session` `crates/whisper-desktop/src/ui/model/controller/observability.rs:15-25`; `record_first_render` `crates/whisper-desktop/src/ui/model/controller/observability.rs:27-40`; `record_reactivity_milestones` `crates/whisper-desktop/src/ui/model/controller/observability.rs:43-98`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/controller/observability.rs:1-99`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/controller/observability.rs:1-99`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::controller — `crates/whisper-desktop/src/ui/model/controller.rs`

Draine les entrées de frame et transmet les commandes à Application. `crates/whisper-desktop/src/ui/model/controller.rs:1-375`

Mesure : 375 production / 0 tests / 375 physiques. `crates/whisper-desktop/src/ui/model/controller.rs:1-375`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/controller.rs:1-375`.

Fonctions avec corps : `poll_gpu_probe` `crates/whisper-desktop/src/ui/model/controller.rs:6-34`; `poll_model_selection` `crates/whisper-desktop/src/ui/model/controller.rs:36-105`; `selected_model_artifact` `crates/whisper-desktop/src/ui/model/controller.rs:107-122`; `selected_model_id` `crates/whisper-desktop/src/ui/model/controller.rs:124-128`; `selected_model_is_selectable` `crates/whisper-desktop/src/ui/model/controller.rs:130-140`; `dispatch` `crates/whisper-desktop/src/ui/model/controller.rs:142-201`; `stop_active` `crates/whisper-desktop/src/ui/model/controller.rs:203-207`; `request_quit` `crates/whisper-desktop/src/ui/model/controller.rs:209-211`; `refresh_microphones` `crates/whisper-desktop/src/ui/model/controller.rs:213-229`; `prepare_frame` `crates/whisper-desktop/src/ui/model/controller.rs:231-312`; `poll_frame_inputs` `crates/whisper-desktop/src/ui/model/controller.rs:314-353`; `import_enqueue_enabled` `crates/whisper-desktop/src/ui/model/controller.rs:355-357`; `manual_stop_available` `crates/whisper-desktop/src/ui/model/controller.rs:359-368`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/controller.rs:1-375`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/controller.rs:1-375`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::preferences — `crates/whisper-desktop/src/ui/model/preferences.rs`

Définit commandes/événements de préférences et leur canal. `crates/whisper-desktop/src/ui/model/preferences.rs:1-121`

Mesure : 121 production / 0 tests / 121 physiques. `crates/whisper-desktop/src/ui/model/preferences.rs:1-121`

Types : `ModelPreferenceCommand` (enum, hors tests) `crates/whisper-desktop/src/ui/model/preferences.rs:4-7`; `ModelPreferenceEvent` (enum, hors tests) `crates/whisper-desktop/src/ui/model/preferences.rs:9-19`; `ModelSelectionRuntime` (struct, hors tests) `crates/whisper-desktop/src/ui/model/preferences.rs:21-25`; `PendingPreferenceChoices` (struct, hors tests) `crates/whisper-desktop/src/ui/model/preferences.rs:28-31`; `PreferenceChannelError` (enum, hors tests) `crates/whisper-desktop/src/ui/model/preferences.rs:34-37`; `ModelPreferenceCommandSender` (struct, hors tests) `crates/whisper-desktop/src/ui/model/preferences.rs:52-55`; `ModelPreferenceCommandReceiver` (struct, hors tests) `crates/whisper-desktop/src/ui/model/preferences.rs:57-60`; `SelectionWidgetState` (struct, hors tests) `crates/whisper-desktop/src/ui/model/preferences.rs:118-121`.

Fonctions avec corps : `fmt` `crates/whisper-desktop/src/ui/model/preferences.rs:40-45`; `model_preference_command_channel` `crates/whisper-desktop/src/ui/model/preferences.rs:62-76`; `send` `crates/whisper-desktop/src/ui/model/preferences.rs:79-94`; `recv` `crates/whisper-desktop/src/ui/model/preferences.rs:98-114`.

Déclarations `pub` (visibilité sous conditions) : `pub enum ModelPreferenceCommand {` `crates/whisper-desktop/src/ui/model/preferences.rs:4`; `pub enum ModelPreferenceEvent {` `crates/whisper-desktop/src/ui/model/preferences.rs:9`; `pub struct ModelSelectionRuntime {` `crates/whisper-desktop/src/ui/model/preferences.rs:21`; `pub enum PreferenceChannelError {` `crates/whisper-desktop/src/ui/model/preferences.rs:34`; `pub struct ModelPreferenceCommandSender {` `crates/whisper-desktop/src/ui/model/preferences.rs:52`; `pub struct ModelPreferenceCommandReceiver {` `crates/whisper-desktop/src/ui/model/preferences.rs:57`; `pub fn model_preference_command_channel()` `crates/whisper-desktop/src/ui/model/preferences.rs:62`; `pub fn send(&self, command: ModelPreferenceCommand) -> Result<(), PreferenceChannelError> {` `crates/whisper-desktop/src/ui/model/preferences.rs:79`; `pub fn recv(&self) -> Result<Vec<ModelPreferenceCommand>, PreferenceChannelError> {` `crates/whisper-desktop/src/ui/model/preferences.rs:98`; `pub struct SelectionWidgetState {` `crates/whisper-desktop/src/ui/model/preferences.rs:118`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/preferences.rs:1-121`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::actions — `crates/whisper-desktop/src/ui/model/view/actions.rs`

Affiche les actions démarrage/arrêt et transmet leurs commandes. `crates/whisper-desktop/src/ui/model/view/actions.rs:1-106`

Mesure : 97 production / 9 tests / 106 physiques. `crates/whisper-desktop/src/ui/model/view/actions.rs:1-106`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/actions.rs:1-106`.

Fonctions avec corps : `start_actions_ui` `crates/whisper-desktop/src/ui/model/view/actions.rs:4-75`; `stop_action_ui` `crates/whisper-desktop/src/ui/model/view/actions.rs:76-105`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/actions.rs:1-106`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/actions.rs:1-106`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::active_job — `crates/whisper-desktop/src/ui/model/view/active_job.rs`

Affiche métriques et texte du travail live actif. `crates/whisper-desktop/src/ui/model/view/active_job.rs:1-125`

Mesure : 125 production / 0 tests / 125 physiques. `crates/whisper-desktop/src/ui/model/view/active_job.rs:1-125`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/active_job.rs:1-125`.

Fonctions avec corps : `active_job_ui` `crates/whisper-desktop/src/ui/model/view/active_job.rs:4-41`; `active_live_metrics_ui` `crates/whisper-desktop/src/ui/model/view/active_job.rs:43-81`; `active_live_text_ui` `crates/whisper-desktop/src/ui/model/view/active_job.rs:83-124`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/active_job.rs:1-125`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/active_job.rs:1-125`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::capture — `crates/whisper-desktop/src/ui/model/view/capture.rs`

Affiche cartes de capture Parakeet, finalisation et sélection du texte. `crates/whisper-desktop/src/ui/model/view/capture.rs:1-352`

Mesure : 352 production / 0 tests / 352 physiques. `crates/whisper-desktop/src/ui/model/view/capture.rs:1-352`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/capture.rs:1-352`.

Fonctions avec corps : `render_parakeet_capture_ui` `crates/whisper-desktop/src/ui/model/view/capture.rs:4-62`; `capture_details_ui` `crates/whisper-desktop/src/ui/model/view/capture.rs:64-158`; `capture_automatic_final_ui` `crates/whisper-desktop/src/ui/model/view/capture.rs:160-234`; `capture_text_selector_ui` `crates/whisper-desktop/src/ui/model/view/capture.rs:236-293`; `capture_text_ui` `crates/whisper-desktop/src/ui/model/view/capture.rs:295-351`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/capture.rs:1-352`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/capture.rs:1-352`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::dialogs — `crates/whisper-desktop/src/ui/model/view/dialogs.rs`

Affiche confirmations et choix de doublons/récupération. `crates/whisper-desktop/src/ui/model/view/dialogs.rs:1-104`

Mesure : 104 production / 0 tests / 104 physiques. `crates/whisper-desktop/src/ui/model/view/dialogs.rs:1-104`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/dialogs.rs:1-104`.

Fonctions avec corps : `dialogs_ui` `crates/whisper-desktop/src/ui/model/view/dialogs.rs:4-103`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/dialogs.rs:1-104`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/dialogs.rs:1-104`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::history — `crates/whisper-desktop/src/ui/model/view/history.rs`

Affiche historique des traitements et retraits de remplacement. `crates/whisper-desktop/src/ui/model/view/history.rs:1-146`

Mesure : 146 production / 0 tests / 146 physiques. `crates/whisper-desktop/src/ui/model/view/history.rs:1-146`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/history.rs:1-146`.

Fonctions avec corps : `history_ui` `crates/whisper-desktop/src/ui/model/view/history.rs:4-21`; `replacement_cleanup_ui` `crates/whisper-desktop/src/ui/model/view/history.rs:23-52`; `history_entries_ui` `crates/whisper-desktop/src/ui/model/view/history.rs:54-145`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/history.rs:1-146`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/history.rs:1-146`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::microphone — `crates/whisper-desktop/src/ui/model/view/microphone.rs`

Dessine spectre, source, test, niveau et gain microphone. `crates/whisper-desktop/src/ui/model/view/microphone.rs:1-333`

Mesure : 317 production / 16 tests / 333 physiques. `crates/whisper-desktop/src/ui/model/view/microphone.rs:1-333`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/microphone.rs:1-333`.

Fonctions avec corps : `paint_microphone_spectrum` `crates/whisper-desktop/src/ui/model/view/microphone.rs:3-76`; `microphone_settings_ui` `crates/whisper-desktop/src/ui/model/view/microphone.rs:79-86`; `microphone_source_ui` `crates/whisper-desktop/src/ui/model/view/microphone.rs:88-154`; `microphone_test_ui` `crates/whisper-desktop/src/ui/model/view/microphone.rs:156-190`; `microphone_meter_ui` `crates/whisper-desktop/src/ui/model/view/microphone.rs:192-288`; `microphone_gain_ui` `crates/whisper-desktop/src/ui/model/view/microphone.rs:290-332`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/microphone.rs:1-333`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/microphone.rs:1-333`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::notices — `crates/whisper-desktop/src/ui/model/view/notices.rs`

Affiche résultats publiés et notifications. `crates/whisper-desktop/src/ui/model/view/notices.rs:1-52`

Mesure : 52 production / 0 tests / 52 physiques. `crates/whisper-desktop/src/ui/model/view/notices.rs:1-52`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/notices.rs:1-52`.

Fonctions avec corps : `published_result` `crates/whisper-desktop/src/ui/model/view/notices.rs:4-9`; `notices_ui` `crates/whisper-desktop/src/ui/model/view/notices.rs:11-51`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/notices.rs:1-52`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/notices.rs:1-52`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::queue — `crates/whisper-desktop/src/ui/model/view/queue.rs`

Affiche file et boutons Traiter, Reprendre et Retirer. `crates/whisper-desktop/src/ui/model/view/queue.rs:1-77`

Mesure : 77 production / 0 tests / 77 physiques. `crates/whisper-desktop/src/ui/model/view/queue.rs:1-77`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/queue.rs:1-77`.

Fonctions avec corps : `queue_ui` `crates/whisper-desktop/src/ui/model/view/queue.rs:4-76`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/queue.rs:1-77`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/queue.rs:1-77`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::settings — `crates/whisper-desktop/src/ui/model/view/settings.rs`

Affiche chemins, langue, calcul et préférence GPU. `crates/whisper-desktop/src/ui/model/view/settings.rs:1-208`

Mesure : 208 production / 0 tests / 208 physiques. `crates/whisper-desktop/src/ui/model/view/settings.rs:1-208`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/settings.rs:1-208`.

Fonctions avec corps : `settings_ui` `crates/whisper-desktop/src/ui/model/view/settings.rs:4-14`; `path_settings_ui` `crates/whisper-desktop/src/ui/model/view/settings.rs:16-66`; `language_and_compute_ui` `crates/whisper-desktop/src/ui/model/view/settings.rs:68-104`; `compute_options_ui` `crates/whisper-desktop/src/ui/model/view/settings.rs:106-165`; `selection_notices_ui` `crates/whisper-desktop/src/ui/model/view/settings.rs:167-186`; `gpu_preference_ui` `crates/whisper-desktop/src/ui/model/view/settings.rs:188-207`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/settings.rs:1-208`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/settings.rs:1-208`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view::unified — `crates/whisper-desktop/src/ui/model/view/unified.rs`

Affiche sessions unifiées et pages document/audio. `crates/whisper-desktop/src/ui/model/view/unified.rs:1-424`

Mesure : 424 production / 0 tests / 424 physiques. `crates/whisper-desktop/src/ui/model/view/unified.rs:1-424`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view/unified.rs:1-424`.

Fonctions avec corps : `render_unified_parakeet_ui` `crates/whisper-desktop/src/ui/model/view/unified.rs:4-64`; `unified_session_status` `crates/whisper-desktop/src/ui/model/view/unified.rs:66-143`; `request_unified_pages` `crates/whisper-desktop/src/ui/model/view/unified.rs:145-255`; `unified_document_ui` `crates/whisper-desktop/src/ui/model/view/unified.rs:257-314`; `unified_audio_ui` `crates/whisper-desktop/src/ui/model/view/unified.rs:316-423`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view/unified.rs:1-424`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view/unified.rs:1-424`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::view — `crates/whisper-desktop/src/ui/model/view.rs`

Implémente eframe::App et compose les panneaux de la fenêtre. `crates/whisper-desktop/src/ui/model/view.rs:1-51`

Mesure : 51 production / 0 tests / 51 physiques. `crates/whisper-desktop/src/ui/model/view.rs:1-51`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/model/view.rs:1-51`.

Fonctions avec corps : `on_exit` `crates/whisper-desktop/src/ui/model/view.rs:14-23`; `ui` `crates/whisper-desktop/src/ui/model/view.rs:25-27`; `show_ui` `crates/whisper-desktop/src/ui/model/view.rs:31-50`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/model/view.rs:1-51`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/model/view.rs:1-51`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model — `crates/whisper-desktop/src/ui/model.rs`

Définit DesktopApp et construit état d’interaction et canaux. `crates/whisper-desktop/src/ui/model.rs:1-285`

Mesure : 247 production / 38 tests / 285 physiques. `crates/whisper-desktop/src/ui/model.rs:1-285`

Types : `UnifiedAudioRequestKey` (type, hors tests) `crates/whisper-desktop/src/ui/model.rs:48-56`; `DesktopApp` (struct, hors tests) `crates/whisper-desktop/src/ui/model.rs:58-127`; `CaptureTextSelection` (enum, hors tests) `crates/whisper-desktop/src/ui/model.rs:130-134`.

Fonctions avec corps : `new` `crates/whisper-desktop/src/ui/model.rs:137-139`; `with_startup_compute` `crates/whisper-desktop/src/ui/model.rs:141-152`; `with_paths` `crates/whisper-desktop/src/ui/model.rs:154-167`; `with_gpu_probe` `crates/whisper-desktop/src/ui/model.rs:169-172`; `with_gpu_inventory` `crates/whisper-desktop/src/ui/model.rs:174-177`; `with_process_started` `crates/whisper-desktop/src/ui/model.rs:179-182`; `with_unified_candidate_enabled` `crates/whisper-desktop/src/ui/model.rs:184-187`; `with_microphones` `crates/whisper-desktop/src/ui/model.rs:189-276`; `with_model_selection_runtime` `crates/whisper-desktop/src/ui/model.rs:278-284`.

Déclarations `pub` (visibilité sous conditions) : `pub use preferences::{` `crates/whisper-desktop/src/ui/model.rs:32`; `pub struct DesktopApp<A> {` `crates/whisper-desktop/src/ui/model.rs:58`; `pub fn new(application: A) -> Self {` `crates/whisper-desktop/src/ui/model.rs:137`; `pub fn with_startup_compute(` `crates/whisper-desktop/src/ui/model.rs:141`; `pub fn with_paths(` `crates/whisper-desktop/src/ui/model.rs:154`; `pub fn with_gpu_probe(mut self, gpu_probe: Receiver<Result<(), String>>) -> Self {` `crates/whisper-desktop/src/ui/model.rs:169`; `pub fn with_gpu_inventory(mut self, gpu_inventory: GpuInventoryApp) -> Self {` `crates/whisper-desktop/src/ui/model.rs:174`; `pub fn with_process_started(mut self, started_at_unix_ms: u64) -> Self {` `crates/whisper-desktop/src/ui/model.rs:179`; `pub fn with_microphones(` `crates/whisper-desktop/src/ui/model.rs:189`; `pub fn with_model_selection_runtime(mut self, runtime: ModelSelectionRuntime) -> Self {` `crates/whisper-desktop/src/ui/model.rs:278`.

Dépend de, via `use crate::` : hors tests : `crate::reactivity_log::ReactivityLog` → `crate::reactivity_log` `crates/whisper-desktop/src/ui/model.rs:1`<br>hors tests : `crate::ui::presentation::*` → `crate::ui::presentation` `crates/whisper-desktop/src/ui/model.rs:2`.

Utilisé par, via ces imports : hors tests : `crate::ui::presentation` `crates/whisper-desktop/src/ui/presentation.rs:1`<br>hors tests : `crate::ui::presentation` `crates/whisper-desktop/src/ui/presentation.rs:1`<br>hors tests : `crate::ui::presentation` `crates/whisper-desktop/src/ui/presentation.rs:1`.

### crate::ui::presentation::gpu — `crates/whisper-desktop/src/ui/presentation/gpu.rs`

Construit libellés, options et contrôles de sélection GPU. `crates/whisper-desktop/src/ui/presentation/gpu.rs:1-243`

Mesure : 243 production / 0 tests / 243 physiques. `crates/whisper-desktop/src/ui/presentation/gpu.rs:1-243`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/presentation/gpu.rs:1-243`.

Fonctions avec corps : `parakeet_gpu_catalog_ready` `crates/whisper-desktop/src/ui/presentation/gpu.rs:3-12`; `gpu_qualified_status` `crates/whisper-desktop/src/ui/presentation/gpu.rs:14-44`; `qualified_devices` `crates/whisper-desktop/src/ui/presentation/gpu.rs:46-56`; `gpu_preference_labels` `crates/whisper-desktop/src/ui/presentation/gpu.rs:58-79`; `gpu_preference_combo` `crates/whisper-desktop/src/ui/presentation/gpu.rs:81-105`; `gpu_preference_options` `crates/whisper-desktop/src/ui/presentation/gpu.rs:107-127`; `gpu_preference_selectable` `crates/whisper-desktop/src/ui/presentation/gpu.rs:129-137`; `gpu_selection_resolution_label` `crates/whisper-desktop/src/ui/presentation/gpu.rs:139-175`; `gpu_qualified_presentation` `crates/whisper-desktop/src/ui/presentation/gpu.rs:177-214`; `gpu_inventory_status` `crates/whisper-desktop/src/ui/presentation/gpu.rs:216-220`; `gpu_inventory_status_message` `crates/whisper-desktop/src/ui/presentation/gpu.rs:222-243`.

Déclarations `pub` (visibilité sous conditions) : `pub fn gpu_qualified_status(ui: &mut egui::Ui, state: &GpuQualifiedInventoryState) {` `crates/whisper-desktop/src/ui/presentation/gpu.rs:14`; `pub fn gpu_preference_labels(devices: &[GpuQualifiedDevice]) -> Vec<String> {` `crates/whisper-desktop/src/ui/presentation/gpu.rs:58`; `pub fn gpu_preference_combo(` `crates/whisper-desktop/src/ui/presentation/gpu.rs:81`; `pub fn gpu_preference_options(` `crates/whisper-desktop/src/ui/presentation/gpu.rs:107`; `pub fn gpu_preference_selectable(device: &whisper_core::gpu_inventory::GpuQualifiedDevice) -> bool {` `crates/whisper-desktop/src/ui/presentation/gpu.rs:129`; `pub fn gpu_selection_resolution_label(` `crates/whisper-desktop/src/ui/presentation/gpu.rs:139`; `pub fn gpu_qualified_presentation(` `crates/whisper-desktop/src/ui/presentation/gpu.rs:177`; `pub fn gpu_inventory_status(ui: &mut egui::Ui, state: &GpuInventoryState) {` `crates/whisper-desktop/src/ui/presentation/gpu.rs:216`; `pub fn gpu_inventory_status_message(state: &GpuInventoryState) -> Option<String> {` `crates/whisper-desktop/src/ui/presentation/gpu.rs:222`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/presentation/gpu.rs:1-243`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::presentation::language — `crates/whisper-desktop/src/ui/presentation/language.rs`

Construit les choix et libellés de langue selon le calcul. `crates/whisper-desktop/src/ui/presentation/language.rs:1-138`

Mesure : 138 production / 0 tests / 138 physiques. `crates/whisper-desktop/src/ui/presentation/language.rs:1-138`

Types : `LanguageChoices` (struct, hors tests) `crates/whisper-desktop/src/ui/presentation/language.rs:4-8`.

Fonctions avec corps : `language_choices_for_compute` `crates/whisper-desktop/src/ui/presentation/language.rs:12-104`; `language_choice_labels` `crates/whisper-desktop/src/ui/presentation/language.rs:106-121`; `language_option_labels` `crates/whisper-desktop/src/ui/presentation/language.rs:123-138`.

Déclarations `pub` (visibilité sous conditions) : `pub struct LanguageChoices {` `crates/whisper-desktop/src/ui/presentation/language.rs:4`; `pub fn language_choices_for_compute(` `crates/whisper-desktop/src/ui/presentation/language.rs:12`; `pub fn language_option_labels(` `crates/whisper-desktop/src/ui/presentation/language.rs:123`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/presentation/language.rs:1-138`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::presentation::microphone — `crates/whisper-desktop/src/ui/presentation/microphone.rs`

Construit les libellés de sources et mesures présentées du microphone. `crates/whisper-desktop/src/ui/presentation/microphone.rs:1-170`

Mesure : 170 production / 0 tests / 170 physiques. `crates/whisper-desktop/src/ui/presentation/microphone.rs:1-170`

Types : `MicrophoneMeterPresentation` (enum, hors tests) `crates/whisper-desktop/src/ui/presentation/microphone.rs:4-11`; `MicrophoneLabelProjection` (struct, hors tests) `crates/whisper-desktop/src/ui/presentation/microphone.rs:101-103`.

Fonctions avec corps : `microphone_source_change_notice` `crates/whisper-desktop/src/ui/presentation/microphone.rs:16-33`; `microphone_meter_presentation` `crates/whisper-desktop/src/ui/presentation/microphone.rs:35-65`; `microphone_saturation_is_current` `crates/whisper-desktop/src/ui/presentation/microphone.rs:67-82`; `microphone_test_start_enabled` `crates/whisper-desktop/src/ui/presentation/microphone.rs:84-100`; `new` `crates/whisper-desktop/src/ui/presentation/microphone.rs:106-108`; `device_label` `crates/whisper-desktop/src/ui/presentation/microphone.rs:110-140`; `choice_label` `crates/whisper-desktop/src/ui/presentation/microphone.rs:142-147`; `actual_line` `crates/whisper-desktop/src/ui/presentation/microphone.rs:149-151`; `planned_line` `crates/whisper-desktop/src/ui/presentation/microphone.rs:153-155`; `microphone_device_label` `crates/whisper-desktop/src/ui/presentation/microphone.rs:158-163`; `microphone_choice_label` `crates/whisper-desktop/src/ui/presentation/microphone.rs:165-170`.

Déclarations `pub` (visibilité sous conditions) : `pub enum MicrophoneMeterPresentation {` `crates/whisper-desktop/src/ui/presentation/microphone.rs:4`; `pub fn microphone_source_change_notice(` `crates/whisper-desktop/src/ui/presentation/microphone.rs:16`; `pub fn microphone_meter_presentation(` `crates/whisper-desktop/src/ui/presentation/microphone.rs:35`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/presentation/microphone.rs:1-170`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::presentation::paths — `crates/whisper-desktop/src/ui/presentation/paths.rs`

Présente chemins, valeurs par défaut et empreintes de modèle. `crates/whisper-desktop/src/ui/presentation/paths.rs:1-97`

Mesure : 97 production / 0 tests / 97 physiques. `crates/whisper-desktop/src/ui/presentation/paths.rs:1-97`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/presentation/paths.rs:1-97`.

Fonctions avec corps : `path_save_status_label` `crates/whisper-desktop/src/ui/presentation/paths.rs:5-18`; `path_text_edit` `crates/whisper-desktop/src/ui/presentation/paths.rs:20-37`; `path_is_file` `crates/whisper-desktop/src/ui/presentation/paths.rs:39-41`; `path_if_file` `crates/whisper-desktop/src/ui/presentation/paths.rs:43-45`; `copyable_path` `crates/whisper-desktop/src/ui/presentation/paths.rs:47-54`; `default_model_path` `crates/whisper-desktop/src/ui/presentation/paths.rs:56-60`; `default_audio_directory` `crates/whisper-desktop/src/ui/presentation/paths.rs:62-66`; `default_destination` `crates/whisper-desktop/src/ui/presentation/paths.rs:68-78`; `approved_model_sha256` `crates/whisper-desktop/src/ui/presentation/paths.rs:80-82`; `parse_sha256` `crates/whisper-desktop/src/ui/presentation/paths.rs:84-90`; `hex_digit` `crates/whisper-desktop/src/ui/presentation/paths.rs:91-97`.

Déclarations `pub` (visibilité sous conditions) : `pub const APPROVED_MODEL_SHA256: &str =` `crates/whisper-desktop/src/ui/presentation/paths.rs:3`; `pub fn path_save_status_label(status: &SaveStatus) -> String {` `crates/whisper-desktop/src/ui/presentation/paths.rs:5`; `pub fn default_model_path() -> String {` `crates/whisper-desktop/src/ui/presentation/paths.rs:56`; `pub fn default_audio_directory() -> String {` `crates/whisper-desktop/src/ui/presentation/paths.rs:62`; `pub fn default_destination() -> String {` `crates/whisper-desktop/src/ui/presentation/paths.rs:68`; `pub fn approved_model_sha256() -> [u8; 32] {` `crates/whisper-desktop/src/ui/presentation/paths.rs:80`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/presentation/paths.rs:1-97`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::presentation::recovery — `crates/whisper-desktop/src/ui/presentation/recovery.rs`

Affiche le widget de récupération des snapshots source. `crates/whisper-desktop/src/ui/presentation/recovery.rs:1-42`

Mesure : 42 production / 0 tests / 42 physiques. `crates/whisper-desktop/src/ui/presentation/recovery.rs:1-42`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/presentation/recovery.rs:1-42`.

Fonctions avec corps : `source_snapshot_recovery_widget` `crates/whisper-desktop/src/ui/presentation/recovery.rs:5-42`.

Déclarations `pub` (visibilité sous conditions) : `pub fn source_snapshot_recovery_widget(` `crates/whisper-desktop/src/ui/presentation/recovery.rs:5`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/presentation/recovery.rs:1-42`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::presentation::selection — `crates/whisper-desktop/src/ui/presentation/selection.rs`

Projette les préférences et construit les sélections modèle/langue/calcul. `crates/whisper-desktop/src/ui/presentation/selection.rs:1-269`

Mesure : 269 production / 0 tests / 269 physiques. `crates/whisper-desktop/src/ui/presentation/selection.rs:1-269`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/presentation/selection.rs:1-269`.

Fonctions avec corps : `record_compute_selection_interaction` `crates/whisper-desktop/src/ui/presentation/selection.rs:3-11`; `record_preference_delivery` `crates/whisper-desktop/src/ui/presentation/selection.rs:13-20`; `apply_startup_preference` `crates/whisper-desktop/src/ui/presentation/selection.rs:22-33`; `apply_initial_model` `crates/whisper-desktop/src/ui/presentation/selection.rs:35-43`; `apply_preference_event` `crates/whisper-desktop/src/ui/presentation/selection.rs:45-71`; `selected_language` `crates/whisper-desktop/src/ui/presentation/selection.rs:73-80`; `compute_for_model` `crates/whisper-desktop/src/ui/presentation/selection.rs:82-87`; `model_supports_live` `crates/whisper-desktop/src/ui/presentation/selection.rs:89-91`; `unified_live_admission_available` `crates/whisper-desktop/src/ui/presentation/selection.rs:93-135`; `model_selection_combo` `crates/whisper-desktop/src/ui/presentation/selection.rs:137-177`; `language_selection_combo` `crates/whisper-desktop/src/ui/presentation/selection.rs:179-255`; `window_must_remain_open` `crates/whisper-desktop/src/ui/presentation/selection.rs:257-269`.

Déclarations `pub` (visibilité sous conditions) : `pub fn record_compute_selection_interaction(` `crates/whisper-desktop/src/ui/presentation/selection.rs:3`; `pub fn record_preference_delivery(` `crates/whisper-desktop/src/ui/presentation/selection.rs:13`; `pub fn apply_startup_preference(` `crates/whisper-desktop/src/ui/presentation/selection.rs:22`; `pub fn apply_initial_model(` `crates/whisper-desktop/src/ui/presentation/selection.rs:35`; `pub fn apply_preference_event(` `crates/whisper-desktop/src/ui/presentation/selection.rs:45`; `pub fn compute_for_model(` `crates/whisper-desktop/src/ui/presentation/selection.rs:82`; `pub fn model_supports_live(_model: Option<whisper_core::model_catalog::ModelId>) -> bool {` `crates/whisper-desktop/src/ui/presentation/selection.rs:89`; `pub fn model_selection_combo(` `crates/whisper-desktop/src/ui/presentation/selection.rs:137`; `pub fn language_selection_combo(` `crates/whisper-desktop/src/ui/presentation/selection.rs:179`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/presentation/selection.rs:1-269`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::presentation::text — `crates/whisper-desktop/src/ui/presentation/text.rs`

Construit les libellés de phases et le texte provisoire à présenter. `crates/whisper-desktop/src/ui/presentation/text.rs:1-79`

Mesure : 79 production / 0 tests / 79 physiques. `crates/whisper-desktop/src/ui/presentation/text.rs:1-79`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/presentation/text.rs:1-79`.

Fonctions avec corps : `capture_state_label` `crates/whisper-desktop/src/ui/presentation/text.rs:3-13`; `unified_session_phase_label` `crates/whisper-desktop/src/ui/presentation/text.rs:15-26`; `compute_choice_label` `crates/whisper-desktop/src/ui/presentation/text.rs:28-34`; `provisional_without_confirmed_prefix` `crates/whisper-desktop/src/ui/presentation/text.rs:36-72`; `normalized_word` `crates/whisper-desktop/src/ui/presentation/text.rs:74-79`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/presentation/text.rs:1-79`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/presentation/text.rs:1-79`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::presentation — `crates/whisper-desktop/src/ui/presentation.rs`

Rassemble imports/réexportations des fonctions de présentation. `crates/whisper-desktop/src/ui/presentation.rs:1-59`

Mesure : 59 production / 0 tests / 59 physiques. `crates/whisper-desktop/src/ui/presentation.rs:1-59`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/presentation.rs:1-59`.

Fonctions avec corps : aucune relevée dans `crates/whisper-desktop/src/ui/presentation.rs:1-59`.

Déclarations `pub` (visibilité sous conditions) : `pub use gpu::{` `crates/whisper-desktop/src/ui/presentation.rs:24`; `pub use language::{LanguageChoices, language_choices_for_compute, language_option_labels};` `crates/whisper-desktop/src/ui/presentation.rs:31`; `pub use microphone::{` `crates/whisper-desktop/src/ui/presentation.rs:37`; `pub use paths::{` `crates/whisper-desktop/src/ui/presentation.rs:40`; `pub use recovery::source_snapshot_recovery_widget;` `crates/whisper-desktop/src/ui/presentation.rs:47`; `pub use selection::{` `crates/whisper-desktop/src/ui/presentation.rs:48`.

Dépend de, via `use crate::` : hors tests : `crate::ui::model::ModelPreferenceEvent` → `crate::ui::model` `crates/whisper-desktop/src/ui/presentation.rs:1`<br>hors tests : `crate::ui::model::SelectionWidgetState` → `crate::ui::model` `crates/whisper-desktop/src/ui/presentation.rs:1`<br>hors tests : `crate::ui::model` → `crate::ui::model` `crates/whisper-desktop/src/ui/presentation.rs:1`.

Utilisé par, via ces imports : hors tests : `crate::ui::model` `crates/whisper-desktop/src/ui/model.rs:2`.

### crate::ui::model::catalog_fixture — `crates/whisper-desktop/src/ui/tests/catalog_fixture.rs`

Tests ou supports : définit `install_test_model_catalog` `crates/whisper-desktop/src/ui/tests/catalog_fixture.rs:4-27`.

Mesure : 0 production / 28 tests / 28 physiques. `crates/whisper-desktop/src/ui/tests/catalog_fixture.rs:1-28`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/tests/catalog_fixture.rs:1-28`.

Fonctions avec corps : `install_test_model_catalog` [tests] `crates/whisper-desktop/src/ui/tests/catalog_fixture.rs:4-27`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/tests/catalog_fixture.rs:1-28`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/tests/catalog_fixture.rs:1-28`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::tests — `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs`

Tests ou supports : définit `enqueue_then_stop_still_targets_the_live_identity` `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:2-21`, `import_uses_selected_gpu` `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:24-35`.

Mesure : 0 production / 436 tests / 436 physiques. `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:1-436`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:1-436`.

Fonctions avec corps : `enqueue_then_stop_still_targets_the_live_identity` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:2-21`; `import_uses_selected_gpu` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:24-35`; `pending_and_late_gpu_inventory_does_not_change_compute_or_gate_transcription` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:38-94`; `auto_preference_is_sent_to_both_paths_without_becoming_cpu` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:97-111`; `cuda_startup_selects_gpu_for_import_and_microphone` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:114-127`; `asynchronous_gpu_probe_sets_the_existing_startup_choice` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:130-145`; `unavailable_cuda_leaves_cpu_usable_and_explains_choice` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:148-166`; `microphone_uses_selected_gpu` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:169-179`; `microphone_labels_changed_windows_default_notice_keeps_identity_and_gain` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:182-236`; `gain_gesture_during_capture_open_targets_the_pinned_identity` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:239-266`; `new_microphone_start_gets_a_new_group_while_continuation_keeps_its_identity` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:269-283`; `first_render_timestamp_is_recorded_after_the_label_in_an_egui_frame` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:286-303`; `first_effective_action_is_not_recorded_when_application_rejects_start` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:306-314`; `changing_catalog_snapshots_regray_and_restore_parakeet_in_the_open_ui` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:317-406`; `gpu_compute_control_requires_a_ready_qualified_runtime_catalog_entry` [tests] `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:409-436`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:1-436`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/tests/compute_and_catalog.rs:1-436`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::tests — `crates/whisper-desktop/src/ui/tests/document_presentation.rs`

Tests ou supports : définit `provisional_display_removes_only_a_proven_confirmed_prefix` `crates/whisper-desktop/src/ui/tests/document_presentation.rs:2-54`, `unified_ui_renders_one_bounded_page_preview_parts_outputs_and_wait_state` `crates/whisper-desktop/src/ui/tests/document_presentation.rs:57-165`.

Mesure : 0 production / 360 tests / 360 physiques. `crates/whisper-desktop/src/ui/tests/document_presentation.rs:1-360`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/tests/document_presentation.rs:1-360`.

Fonctions avec corps : `provisional_display_removes_only_a_proven_confirmed_prefix` [tests] `crates/whisper-desktop/src/ui/tests/document_presentation.rs:2-54`; `unified_ui_renders_one_bounded_page_preview_parts_outputs_and_wait_state` [tests] `crates/whisper-desktop/src/ui/tests/document_presentation.rs:57-165`; `unified_ui_requests_only_the_selected_identity_pages` [tests] `crates/whisper-desktop/src/ui/tests/document_presentation.rs:168-242`; `unified_wait_tracking_prunes_finished_evicted_sessions_and_keeps_selected_wait` [tests] `crates/whisper-desktop/src/ui/tests/document_presentation.rs:245-360`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/tests/document_presentation.rs:1-360`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/tests/document_presentation.rs:1-360`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::tests — `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs`

Tests ou supports : définit `empty_snapshot_recovery_result_does_not_render_a_blank_modal` `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:2-16`, `parakeet_preserves_the_selected_compute_preference` `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:19-30`.

Mesure : 0 production / 93 tests / 93 physiques. `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:1-93`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:1-93`.

Fonctions avec corps : `empty_snapshot_recovery_result_does_not_render_a_blank_modal` [tests] `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:2-16`; `rendered_shape_count` [tests] `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:3-13`; `parakeet_preserves_the_selected_compute_preference` [tests] `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:19-30`; `queued_import_uses_current_compute_and_preserves_the_durable_request` [tests] `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:33-63`; `resumed_import_uses_its_durable_compute_instead_of_the_global_selector` [tests] `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:66-93`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:1-93`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/tests/import_and_lifecycle.rs:1-93`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::tests — `crates/whisper-desktop/src/ui/tests/microphone_controls.rs`

Tests ou supports : définit `auto_stop_is_disabled_by_default_and_uses_speech_sample_clock_when_enabled` `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:2-6`, `import_enqueue_waits_during_live_capture` `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:9-12`.

Mesure : 0 production / 486 tests / 486 physiques. `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:1-486`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:1-486`.

Fonctions avec corps : `auto_stop_is_disabled_by_default_and_uses_speech_sample_clock_when_enabled` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:2-6`; `import_enqueue_waits_during_live_capture` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:9-12`; `window_stays_open_through_capture_and_drain` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:15-22`; `manual_stop_is_available_while_micro_is_capturing` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:25-29`; `microphone_test_button_stops_after_neutral_gain_ack_and_reopens` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:32-251`; `actual_probe_spectra_stay_visible_across_discarded_egui_passes` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:254-322`; `microphone_saturation_badge_requires_current_identity_and_fresh_limiter_data` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:325-403`; `starting_a_new_microphone_job_hides_the_previous_publication` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:406-417`; `refused_ui_model_command_has_no_gpu_candidate_io_effect` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:420-457`; `direct_live_request_uses_explicit_microphone_without_changing_compute` [tests] `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:460-486`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:1-486`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/tests/microphone_controls.rs:1-486`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::tests — `crates/whisper-desktop/src/ui/tests/root_support.rs`

Tests ou supports : définit `speech_limit_samples` `crates/whisper-desktop/src/ui/tests/root_support.rs:2-4`.

Mesure : 0 production / 4 tests / 4 physiques. `crates/whisper-desktop/src/ui/tests/root_support.rs:1-4`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/tests/root_support.rs:1-4`.

Fonctions avec corps : `speech_limit_samples` [tests] `crates/whisper-desktop/src/ui/tests/root_support.rs:2-4`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/tests/root_support.rs:1-4`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/tests/root_support.rs:1-4`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::tests — `crates/whisper-desktop/src/ui/tests/support.rs`

Tests ou supports : définit `dispatch` `crates/whisper-desktop/src/ui/tests/support.rs:33-36`, `view` `crates/whisper-desktop/src/ui/tests/support.rs:38-40`.

Mesure : 0 production / 569 tests / 569 physiques. `crates/whisper-desktop/src/ui/tests/support.rs:1-569`

Types : `RecordingApplication` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:30`; `MicrophoneCycleApplication` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:44-48`; `UiFrameReport` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:275-279`; `RejectQuitApplication` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:420`; `RejectStartApplication` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:421`; `RefusingStagedCandidateApplication` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:424-427`; `OneMicrophone` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:429`; `ChangingDefaultMicrophone` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:431`; `ControlledGpuInventory` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:434`; `ControlledGpuResponseQueue` (type, tests) `crates/whisper-desktop/src/ui/tests/support.rs:436`; `HomonymousMicrophones` (struct, tests) `crates/whisper-desktop/src/ui/tests/support.rs:462`.

Fonctions avec corps : `dispatch` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:33-36`; `view` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:38-40`; `dispatch` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:51-212`; `view` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:214-216`; `apply_microphone_cycle_event` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:219-273`; `run_ui_frame` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:281-287`; `run_ui_frame_at_size` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:289-296`; `run_ui_frame_with_discard` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:298-340`; `actual_probe_spectrum` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:342-396`; `click_ui_button` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:398-418`; `start` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:439-441`; `poll` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:443-445`; `list_inputs` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:449-455`; `default_input_identity` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:457-459`; `list_inputs` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:465-483`; `default_input_identity` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:485-487`; `list_inputs` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:491-498`; `default_input_identity` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:500-502`; `dispatch` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:506-512`; `view` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:514-516`; `dispatch` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:520-522`; `view` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:524-526`; `dispatch` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:530-550`; `view` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:552-554`; `set_gpu_candidate` [tests] `crates/whisper-desktop/src/ui/tests/support.rs:556-568`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/tests/support.rs:1-569`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/tests/support.rs:1-569`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::tests — `crates/whisper-desktop/src/ui/tests/unified_sessions.rs`

Tests ou supports : définit `desktop_parakeet_handlers_select_unified_routes_only_for_enabled_candidate` `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:2-82`, `unified_finalizing_capture_keeps_live_button_available_for_second_capture_only` `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:85-172`.

Mesure : 0 production / 798 tests / 798 physiques. `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:1-798`

Types : `LifecycleIo` (struct, tests) `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:376`.

Fonctions avec corps : `desktop_parakeet_handlers_select_unified_routes_only_for_enabled_candidate` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:2-82`; `unified_finalizing_capture_keeps_live_button_available_for_second_capture_only` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:85-172`; `unified_capture_gain_slider_targets_the_current_source_and_revision` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:175-239`; `microphone_labels_number_homonyms_by_sorted_identity` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:242-272`; `microphone_labels_keep_unique_names_and_hide_missing_names` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:275-320`; `microphone_labels_selection_keeps_the_full_identity_in_commands` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:323-356`; `rejected_quit_admission_does_not_latch_pending_close` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:359-365`; `turbo_completed_capture_allows_unified_parakeet_button_and_dispatch` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:368-512`; `submit` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:378-380`; `poll` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:381-383`; `unified_admission_keeps_direct_capture_and_other_busy_guards` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:515-558`; `stale_audio_page_retries_are_delayed_and_corruption_does_not_retry` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:561-688`; `unified_audio_page_stays_visible_at_same_position_while_progress_refreshes` [tests] `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:691-798`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:1-798`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/tests/unified_sessions.rs:1-798`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui::model::tests — `crates/whisper-desktop/src/ui/tests.rs`

Tests ou supports : déclare les sous-modules/réexportations de `crates/whisper-desktop/src/ui/tests.rs:1-7`.

Mesure : 0 production / 7 tests / 7 physiques. `crates/whisper-desktop/src/ui/tests.rs:1-7`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui/tests.rs:1-7`.

Fonctions avec corps : aucune relevée dans `crates/whisper-desktop/src/ui/tests.rs:1-7`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/src/ui/tests.rs:1-7`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui/tests.rs:1-7`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### crate::ui — `crates/whisper-desktop/src/ui.rs`

Réexporte modèle UI et fonctions de présentation. `crates/whisper-desktop/src/ui.rs:1-20`

Mesure : 20 production / 0 tests / 20 physiques. `crates/whisper-desktop/src/ui.rs:1-20`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/src/ui.rs:1-20`.

Fonctions avec corps : aucune relevée dans `crates/whisper-desktop/src/ui.rs:1-20`.

Déclarations `pub` (visibilité sous conditions) : `pub use model::{` `crates/whisper-desktop/src/ui.rs:4`; `pub use presentation::{` `crates/whisper-desktop/src/ui.rs:9`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/src/ui.rs:1-20`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::gpu_inventory_ui — `crates/whisper-desktop/tests/gpu_inventory_ui.rs`

Tests ou supports : définit `inventory_status_does_not_render_the_removed_fixed_card_list` `crates/whisper-desktop/tests/gpu_inventory_ui.rs:58-80`, `status_area_distinguishes_loading_empty_partial_and_failure` `crates/whisper-desktop/tests/gpu_inventory_ui.rs:83-104`.

Mesure : 0 production / 275 tests / 275 physiques. `crates/whisper-desktop/tests/gpu_inventory_ui.rs:1-275`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/tests/gpu_inventory_ui.rs:1-275`.

Fonctions avec corps : `rendered_text` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:9-27`; `collect` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:12-21`; `rendered_status` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:29-47`; `collect` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:32-41`; `inventory` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:49-55`; `inventory_status_does_not_render_the_removed_fixed_card_list` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:58-80`; `status_area_distinguishes_loading_empty_partial_and_failure` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:83-104`; `qualified_loading_and_uncertain_support_are_accessible_and_cpu_remains_available` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:106-123`; `collect` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:111-117`; `qualified_catalogue_uses_green_gray_and_neutral_text_without_color_only_meaning` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:126-213`; `disabled_ambiguous_or_unranked_devices_are_never_green` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:216-249`; `widget_shows_disabled_and_unknown_device_properties` [tests] `crates/whisper-desktop/tests/gpu_inventory_ui.rs:252-275`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/tests/gpu_inventory_ui.rs:1-275`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/tests/gpu_inventory_ui.rs:1-275`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::gpu_selection_ui — `crates/whisper-desktop/tests/gpu_selection_ui.rs`

Tests ou supports : définit `homonymous_devices_get_numbered_labels_and_keep_distinct_internal_identity` `crates/whisper-desktop/tests/gpu_selection_ui.rs:15-32`, `unique_names_have_no_suffix_and_real_combo_has_no_clear_preference_option` `crates/whisper-desktop/tests/gpu_selection_ui.rs:35-60`.

Mesure : 0 production / 224 tests / 224 physiques. `crates/whisper-desktop/tests/gpu_selection_ui.rs:1-224`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/tests/gpu_selection_ui.rs:1-224`.

Fonctions avec corps : `homonymous_devices_get_numbered_labels_and_keep_distinct_internal_identity` [tests] `crates/whisper-desktop/tests/gpu_selection_ui.rs:15-32`; `unique_names_have_no_suffix_and_real_combo_has_no_clear_preference_option` [tests] `crates/whisper-desktop/tests/gpu_selection_ui.rs:35-60`; `selected_and_resolution_text_use_the_same_numbered_name` [tests] `crates/whisper-desktop/tests/gpu_selection_ui.rs:63-90`; `only_confirmed_available_and_unambiguous_gpu_is_selectable` [tests] `crates/whisper-desktop/tests/gpu_selection_ui.rs:93-125`; `qualified_device` [tests] `crates/whisper-desktop/tests/gpu_selection_ui.rs:127-143`; `selection_messages_are_plain_and_never_expose_internal_identity` [tests] `crates/whisper-desktop/tests/gpu_selection_ui.rs:146-198`; `uncertain_selection_is_neutral_and_save_acknowledgement_is_revisioned` [tests] `crates/whisper-desktop/tests/gpu_selection_ui.rs:201-224`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/tests/gpu_selection_ui.rs:1-224`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/tests/gpu_selection_ui.rs:1-224`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::microphone_level_ui — `crates/whisper-desktop/tests/microphone_level_ui.rs`

Tests ou supports : définit `meter_presentation_distinguishes_waiting_absent_silence_stale_and_old_capture` `crates/whisper-desktop/tests/microphone_level_ui.rs:42-98`, `microphone_labels_changed_default_notice_uses_names_and_keeps_identity_comparison` `crates/whisper-desktop/tests/microphone_level_ui.rs:101-164`.

Mesure : 0 production / 164 tests / 164 physiques. `crates/whisper-desktop/tests/microphone_level_ui.rs:1-164`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/tests/microphone_level_ui.rs:1-164`.

Fonctions avec corps : `meter` [tests] `crates/whisper-desktop/tests/microphone_level_ui.rs:6-39`; `meter_presentation_distinguishes_waiting_absent_silence_stale_and_old_capture` [tests] `crates/whisper-desktop/tests/microphone_level_ui.rs:42-98`; `microphone_labels_changed_default_notice_uses_names_and_keeps_identity_comparison` [tests] `crates/whisper-desktop/tests/microphone_level_ui.rs:101-164`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/tests/microphone_level_ui.rs:1-164`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/tests/microphone_level_ui.rs:1-164`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::model_selection_ui — `crates/whisper-desktop/tests/model_selection_ui.rs`

Tests ou supports : définit `snapshot_renders_catalog_models_and_all_turbo_languages_without_model_path_input` `crates/whisper-desktop/tests/model_selection_ui.rs:18-70`, `selection_ui_rendering_does_not_hash_model_or_runtime_files` `crates/whisper-desktop/tests/model_selection_ui.rs:73-104`.

Mesure : 0 production / 384 tests / 384 physiques. `crates/whisper-desktop/tests/model_selection_ui.rs:1-384`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/tests/model_selection_ui.rs:1-384`.

Fonctions avec corps : `snapshot_renders_catalog_models_and_all_turbo_languages_without_model_path_input` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:18-70`; `selection_ui_rendering_does_not_hash_model_or_runtime_files` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:73-104`; `language_availability_uses_selected_backend_and_auto_intersection` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:107-173`; `parakeet_language_projection_follows_qualified_gpu_or_auto_preference` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:176-208`; `late_saved_preferences_do_not_overwrite_a_newer_user_choice` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:211-270`; `same_value_cpu_click_blocks_late_saved_auto_preference` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:273-281`; `preference_mailbox_is_bounded_and_keeps_the_latest_choice_per_field` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:284-304`; `disconnected_preference_worker_reports_failed_model_and_compute_saves` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:307-340`; `preference_receiver_reports_disconnected_after_last_sender_drops` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:343-350`; `rendered_text` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:352-361`; `parakeet_preserves_import_compute_preferences_and_legacy_live_availability_without_changing_turbo` [tests] `crates/whisper-desktop/tests/model_selection_ui.rs:364-384`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/tests/model_selection_ui.rs:1-384`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/tests/model_selection_ui.rs:1-384`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

### integration::reprocessing_recovery_ui — `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs`

Tests ou supports : définit `recovery_ui_displays_protected_and_recoverable_entries_without_implicit_recovery` `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:6-59`, `capacity_action_is_available_only_when_the_application_offers_recovery` `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:62-80`.

Mesure : 0 production / 80 tests / 80 physiques. `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:1-80`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:1-80`.

Fonctions avec corps : `recovery_ui_displays_protected_and_recoverable_entries_without_implicit_recovery` [tests] `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:6-59`; `capacity_action_is_available_only_when_the_application_offers_recovery` [tests] `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:62-80`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:1-80`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs:1-80`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

## Comptages textuels

| Motif | Production | Tests |
| --- | --- | --- |
| builder_spawn | 1 | 0 |
| cfg_feature | 0 | 1 |
| cfg_test | 0 | 26 |
| command_new | 0 | 0 |
| expect | 2 | 21 |
| thread_spawn | 3 | 1 |
| unsafe | 0 | 0 |
| unwrap | 2 | 164 |

Chaque occurrence sourcée : [occurrences.tsv](../raw/occurrences.tsv). Chaînes/commentaires inclus ; appels effectifs non résolus.

## Diagnostics Clippy de cette crate

Commande workspace/all-targets, code 0, aucun test exécuté. 13 messages warning concernant les sources de cette crate, répétitions entre cibles incluses : 4 sur plages hors tests et 9 sur plages tests. [Sortie brute](../raw/cargo-clippy.stdout.jsonl), [détail sourcé](../raw/clippy-diagnostics.tsv).
| Diagnostic | Plages hors tests | Plages tests |
| --- | --- | --- |
| clippy::cognitive_complexity | 0 | 1 |
| clippy::too_many_lines | 4 | 8 |
