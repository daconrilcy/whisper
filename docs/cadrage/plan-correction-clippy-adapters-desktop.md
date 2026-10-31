Avancée : lot P réalisé et relu CLEAN ; 17 sites fermés, 68 avertissements adapters lib restants.
Blocage : aucun pour P ; sélection contrôlée S à établir avant édition.
Décision : commit local P puis lot S, revues et autres arrêts maintenus, aucun push.

# Analyse et plan de correction Clippy — adaptateurs et desktop

## Header

- Source audit : présent document et journal `target/parakeet-tests/clippy-core/logs/Run-20261031T053220129-3da3f339.stderr.log`, commande observée `clippy --offline --locked -p whisper-desktop --all-targets`, exit 0.
- Planning date : 2026-10-31.
- Target workspace : `C:\dev\whisper`, branche `evo-startup-perf`, HEAD `d9f9b859207312d4db39775d94557bd9ecece130`.
- Planning mode : plan only, no code changes.
- Confidence : medium — inventaire exhaustif du journal fourni et inspection ciblée des sources/consommateurs, mais profils supplémentaires et tests adaptateurs non relancés ; certains usages cfg doivent être confirmés avant suppression.
- Plan quality check : passed, auto-contrôle documentaire ; aucune revue indépendante du plan revendiquée.
- Notation : `adapters/` signifie `crates/whisper-adapters/`, `desktop/` signifie `crates/whisper-desktop/` ; les chemins abrégés de tests se rattachent à la crate de la tâche.

## 1. Audit Digest

Objectif produit : rendre les avertissements usuels de Clippy exploitables et les contrôles stricts des deux crates possibles, tout en préservant audio capturé, identités des sessions, préférences, verrous et publication durable. Aucun ajout fonctionnel n'est proposé.

### Décompte exact et limites

Le journal contient **100 diagnostics localisés émis**, soit **98 emplacements distincts** après dédoublonnage exact chemin/ligne/colonne : **85 adaptateurs lib**, **13 desktop**. Les résumés Clippy ne sont pas des diagnostics supplémentaires. Desktop lib annonce 10 warnings, lib test 10 dont 7 dupliqués, bin test 2 ; deux emplacements réapparaissent explicitement dans le texte du journal (paramètre model et tuple audio). Les 13 sites se répartissent en dix de bibliothèque et trois supplémentaires exclusivement dans les supports/tests. Ne pas additionner les résumés comme si les cibles étaient disjointes.

La commande cible desktop all-targets mais ne qualifie pas **adapters all-targets** : les tests de la dépendance ne sont pas lintés comme ses propres cibles. Le profil observé est le défaut, sans personal-install. Les éventuels diagnostics propres aux tests adaptateurs, features de qualification, workers ou profils supplémentaires sont **non mesurés**, pas zéro. La bibliothèque core a déjà son strict PASS au lot précédent ; ne pas la remodifier.

Les lints `too_many_arguments` et `type_complexity` proviennent ici de clippy::all. Ce relevé n'active pas `too_many_lines` ni `cognitive_complexity` ; il ne constitue pas un audit de toutes les tailles de fichiers/fonctions.

### Causes et priorités

- **IMPORTANT, conservation disque/verrous** : trois `suspicious_open_options` concernent des fichiers de verrou, et non des fichiers de données à écraser. Choix minimal : **truncate(false)**. La suggestion truncate(true) ne doit pas être appliquée mécaniquement. Le comportement actuel n'est pas une corruption démontrée ; il est implicite.
- **IMPORTANT, durée de vie** : `OperationGuardProbe::Acquired(File)` est un gardien RAII. Remplacer sa charge par () supprimerait prématurément le verrou. Le handle `VerifiedSource._file` et l'appel de validation d'identité doivent aussi survivre au nettoyage.
- **IMPORTANT, frontières** : trois enums internes volumineux (272, 576 et 840 octets au moins dans le journal), douze fonctions trop argumentées, six types complexes et deux erreurs unitaires publiques. Leurs corrections ont des consommateurs et des invariants de propriété ; elles ne sont pas des opérations de formatage.
- **MINEUR, formes locales** : closures unitaires, if imbriqués, conversions/emprunts superflus, idiomes I/O, champs mut et variables non utilisés. Le risque devient concret si une simplification réordonne acquisition, validation, émission ou arrêt.
- **MINEUR, cfg(test)** : PID/rectangles observés seulement en tests ; wrappers de confirmation appelés par les tests ; constructeur de runtime atteint uniquement via un constructeur cfg(test). Les supprimer pour rendre la lib silencieuse casserait la couverture.
- **Desktop, résultat ignoré** : trois selectable_label sont rendus sans exploiter leur Response ; le plan conserve exactement leur apparence et interaction par consommation explicite du retour. Le paramètre model inutilisé accompagne une fonction qui renvoie toujours true : aucune nouvelle règle de compatibilité live ne doit être inventée.

### Carte d'architecture, avant = après

| Responsabilité | Modules | Direction à préserver |
| --- | --- | --- |
| Domaine / contrats | whisper-core, lecture seule | Aucune dépendance vers adapters/UI, aucun nouveau champ pour le lint |
| Persistance / identité | archive/*, source_binding, windows_file_identity, queue_store | Verrous, SHA et publication conservés ; pas de nouveau lien archive→worker_ipc |
| Capture / Parakeet | capture, microphone_probe, parakeet_* | Canaux bornés, callback sans nouvelle allocation/lock, ACK et durabilité inchangés |
| Orchestration technique | worker_ipc/ipc, runtime, service, parakeet_unified | Même routage, générations, incarnation, stop/reap et priorité des commandes |
| Composition / interface | desktop lib/main/root, ui/model/view/presentation | Affichage et commandes, politiques restent dans core |
| Tests | tests/ de chaque crate, supports privés existants | Nouveaux scénarios publics dans tests/, pas de nouvelle masse inline |

Aucun déplacement de module ni réarchitecture générale. Convention actuelle `foo.rs` + `foo/` préservée. Les façades UI restent des déclarations/réexports ; archive.rs et worker_ipc.rs conservent intentionnellement leur rôle actuel, sans profiter des alias pour leur ajouter une nouvelle orchestration.

## 2. Freshness Check

Lectures : AGENTS racine/adapters/desktop, manifestes, carte d'architecture et fiches des deux crates, journal complet indexé, sources ciblées des cas risqués et recherche des constructeurs/appels. Les cartes sous docs/architecture sont non suivies et anciennes : elles donnent l'orientation, pas les chemins actuels de preuve. Les sources montrent déjà archive/publication et worker_ipc/runtime ; les anciennes cartes AGENTS qui les décrivent comme proposés ne justifient pas de les recréer.

Aucun plan résiduel précédent trouvé ; `docs/cadrage/plan-correction-clippy-core.md` ferme seulement les dix erreurs core et rapporte la dette aval. Aucun source suivi sale au début ; non-suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés.

Le relevé appartient au candidat qui a été commité en HEAD ; aucune correction source ultérieure dans ce chat. Inspection ciblée, pas revue exhaustive de tous les corps. Aucune commande Cargo, rustfmt, test, build, outil natif, UI ou accès réseau pendant cette analyse.

### Consommateurs à coordonner

| Frontière | Producteurs/lecteurs constatés | Correction retenue et preuve |
| --- | --- | --- |
| WorkerCommandPoll | worker_ipc/ipc/mod.rs : take_next_worker_command, writer loop ; tests de transport | Box seulement dans l'enveloppe interne de polling ; déballer avant le transport ; canaux WorkerCommand et JSON Turbo inchangés |
| PreflightResult::Duplicate | runtime/queue.rs et duplicates.rs ; observation puis émission événement | Box interne, déballer au passage dans ImportEvent ; ne pas changer le prompt du cœur |
| UnifiedSessionSource::Capture | construction runtime/live.rs ; méthodes ReadablePcmSource/pump/stop/gain dans parakeet_unified/mod.rs ; fixtures unified_capture/native_execution | Box du pump possédé ; Closed inchangé ; même Drop et mêmes ACK |
| reconfirm_interrupted_publication | archive/publication/catalog.rs ; runtime/duplicates.rs ; archive/tests/recovery.rs | Paramètre public nommé, API source Rust coordonnée ; aucun format durable changé |
| Installation export | publication/exports.rs : quatre appels install_owned_export ; archive.rs callback export_hard_linker | Alias du même callback et contexte emprunté, injection et instant d'appel conservés |
| Capture callback | capture.rs : callbacks f32/i16/u16 et tests locaux | Contexte de références, pas d'allocation ou mutex nouveau dans la boucle |
| Live/VAD/repli | inference_thread spawn et transcribe_window dans parakeet_live.rs ; Event::Result vers handler VAD ; deux appels retry_native_gpu_startup_on_cpu et tests native_execution | Regrouper données/références, pas les politiques ; preserve compute/identity/cancel |
| Runtime/service | service constructors vers runtime constructor ; nombreux tests native_execution/unified_import/unified_capture | Alias de gates partagé et config interne ; façades publiques inchangées si possible |
| Préférences UI | ui/model/preferences.rs send/recv ; root boucle recv ; settings transmet résultat à presentation/selection.rs ; tests/model_selection_ui.rs | Erreur nommée réexportée ; Full reste succès, Disconnected/Poisoned restent échec, même notice |
| Démarrage desktop | lib.rs::run ; unique appel main.rs | Struct possédée des options existantes, aucun changement de root::compose/routage |
| Requête audio UI | ui/model.rs et view/unified.rs comparaisons/pagination | Alias privé du tuple, égalité et ordre des sept composantes conservés |

L'absence de consommateurs externes au dépôt n'est pas démontrée. Les ajouts de paramètres publics nommés changent l'API Rust interne au workspace ; tout consommateur découvert hors dépôt impose un arbitrage de compatibilité avant la tâche correspondante.

## 3. Planning Assumptions

- Safe : aucune dépendance, feature, seuil de lint, protocole, format durable ou règle produit ne change.
- Safe : pas de tests miroirs pour les réécritures syntaxiques ; réutiliser les tests utiles et ajouter seulement les preuves de verrou/canal/effets nécessaires.
- Safe : migration des anciens tests inline différée ; aucun nouveau scénario volumineux dans src.
- Validation needed : recherche usages toutes features et compilation tests avant suppression d'un membre privé ; cfg(test) uniquement si tous ses usages sont réellement test.
- Validation needed : regroupement de paramètres sans clone ou mem::take artificiel et sans modification de portée des gardiens ; seuil réel vérifié par Clippy.
- Decision gate : ce plan documentaire ne vaut ni GO des six lots ni autorisation de nouvelle campagne. Le GO core antérieur n'autorise pas automatiquement cette nouvelle correction.
- Decision gate : la tolérance Temp accordée à **clippy-core seulement** ne s'étend pas à une campagne distincte. Faire confirmer expressément la règle de campagne retenue avant son lancement ; ne pas recopier une garde permissive sous un nouveau nom sans cette décision.

## 4. Finding Traceability

Chaque emplacement a un ID stable A01–A85 ou D01–D13 et une tâche. Les numéros de ligne sont ceux de HEAD lors de l'analyse ; la recherche par symbole prime après modification. Tous les constats du journal sont **Planned**. La dette des autres profils/cibles non mesurés reste **Deferred** jusqu'au relevé autorisé, sans promesse de nettoyage implicite. Les risques de conservation sont IMPORTANT ; les autres diagnostics sont MINEUR, sans panne fonctionnelle affirmée.

| Audit finding | Severity | Disposition | Task IDs / gate | Notes |
| --- | --- | --- | --- | --- |
| A01 `crates/whisper-adapters/src/parakeet_live.rs:11` | MINEUR | Planned | S6 | unused import: `MicrophoneSpectrumReader` |
| A02 `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:247` | MINEUR | Planned | T1 | `unused_unit` |
| A03 `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:249` | MINEUR | Planned | T1 | `unused_unit` |
| A04 `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:22` | MINEUR | Planned | T1 | `unused_unit` |
| A05 `crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:28` | MINEUR | Planned | T1 | `unused_unit` |
| A06 `crates/whisper-adapters/src/archive/cleanup/proofs.rs:310` | MINEUR | Planned | P3 | unused variable: `existing` |
| A07 `crates/whisper-adapters/src/archive/journal.rs:185` | MINEUR | Planned | P3 | variable does not need to be mutable |
| A08 `crates/whisper-adapters/src/archive/publication.rs:228` | MINEUR | Planned | P3 | value assigned to `plan` is never read |
| A09 `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:231` | MINEUR | Planned | T2 | unused variable: `pid` |
| A10 `crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs:256` | MINEUR | Planned | T2 | unused variable: `pid` |
| A11 `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:234` | MINEUR | Planned | T2 | unused variable: `session` |
| A12 `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:453` | MINEUR | Planned | T3 | variable does not need to be mutable |
| A13 `crates/whisper-adapters/src/worker_ipc/service/automatic_final.rs:54` | MINEUR | Planned | T2 | unused variable: `generation` |
| A14 `crates/whisper-adapters/src/parakeet_audio.rs:348` | MINEUR | Planned | S5 | field `index_path` is never read |
| A15 `crates/whisper-adapters/src/parakeet_live.rs:921` | MINEUR | Planned | S6 | methods `session_id` and `destination` are never used |
| A16 `crates/whisper-adapters/src/parakeet_confirmation.rs:166` | MINEUR | Planned | S6 | methods `finalize` and `ingest` are never used |
| A17 `crates/whisper-adapters/src/source_binding.rs:79` | IMPORTANT | Planned | P1 | field `0` is never read |
| A18 `crates/whisper-adapters/src/source_binding.rs:1133` | IMPORTANT | Planned | P4 | method `release_record` is never used |
| A19 `crates/whisper-adapters/src/windows_file_identity.rs:38` | IMPORTANT | Planned | P2 | field `identity` is never read |
| A20 `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:155` | IMPORTANT | Planned | C6 | associated function `new_with_workers_and_parakeet` is never used |
| A21 `crates/whisper-adapters/src/archive/cleanup/execution.rs:328` | MINEUR | Planned | S1 | `collapsible_if` |
| A22 `crates/whisper-adapters/src/archive/identity.rs:223` | MINEUR | Planned | S1 | `manual_is_multiple_of` |
| A23 `crates/whisper-adapters/src/archive/identity.rs:311` | MINEUR | Planned | S1 | `manual_is_multiple_of` |
| A24 `crates/whisper-adapters/src/archive/publication/catalog.rs:17` | IMPORTANT | Planned | P1 | `suspicious_open_options` |
| A25 `crates/whisper-adapters/src/archive/publication/catalog.rs:348` | IMPORTANT | Planned | C7 | `too_many_arguments` |
| A26 `crates/whisper-adapters/src/archive/publication/catalog.rs:633` | IMPORTANT | Planned | C7 | `too_many_arguments` |
| A27 `crates/whisper-adapters/src/archive/publication/catalog.rs:921` | MINEUR | Planned | S2 | `manual_range_patterns` |
| A28 `crates/whisper-adapters/src/archive/publication/catalog.rs:933` | MINEUR | Planned | S2 | `format_in_format_args` |
| A29 `crates/whisper-adapters/src/archive/publication/exports.rs:329` | IMPORTANT | Planned | C8 | `too_many_arguments` |
| A30 `crates/whisper-adapters/src/archive/recovery.rs:650` | MINEUR | Planned | S1 | `manual_range_patterns` |
| A31 `crates/whisper-adapters/src/archive.rs:336` | IMPORTANT | Planned | C8 | `type_complexity` |
| A32 `crates/whisper-adapters/src/capture.rs:263` | IMPORTANT | Planned | C2 | `too_many_arguments` |
| A33 `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:55` | MINEUR | Planned | S3 | `field_reassign_with_default` |
| A34 `crates/whisper-adapters/src/gpu_inventory/windows_native.rs:78` | MINEUR | Planned | S3 | `field_reassign_with_default` |
| A35 `crates/whisper-adapters/src/gpu_inventory.rs:38` | MINEUR | Planned | C1 | `type_complexity` |
| A36 `crates/whisper-adapters/src/gpu_inventory.rs:324` | MINEUR | Planned | S3 | `needless_return` |
| A37 `crates/whisper-adapters/src/gpu_inventory.rs:328` | MINEUR | Planned | C1 | `too_many_arguments` |
| A38 `crates/whisper-adapters/src/microphone_probe.rs:264` | MINEUR | Planned | S3 | `collapsible_if` |
| A39 `crates/whisper-adapters/src/model_catalog.rs:59` | MINEUR | Planned | S4 | `io_other_error` |
| A40 `crates/whisper-adapters/src/model_catalog.rs:98` | MINEUR | Planned | S4 | `io_other_error` |
| A41 `crates/whisper-adapters/src/model_catalog.rs:168` | MINEUR | Planned | S4 | `io_other_error` |
| A42 `crates/whisper-adapters/src/model_catalog.rs:191` | MINEUR | Planned | S4 | `io_other_error` |
| A43 `crates/whisper-adapters/src/model_catalog.rs:201` | MINEUR | Planned | S4 | `io_other_error` |
| A44 `crates/whisper-adapters/src/parakeet_audio.rs:735` | MINEUR | Planned | S5 | `chunks_exact_to_as_chunks` |
| A45 `crates/whisper-adapters/src/parakeet_document.rs:171` | MINEUR | Planned | S5 | `manual_is_multiple_of` |
| A46 `crates/whisper-adapters/src/parakeet_import.rs:150` | MINEUR | Planned | S6 | `cmp_owned` |
| A47 `crates/whisper-adapters/src/parakeet_live.rs:179` | MINEUR | Planned | S6 | `needless_question_mark` |
| A48 `crates/whisper-adapters/src/parakeet_live.rs:379` | MINEUR | Planned | S6 | `derivable_impls` |
| A49 `crates/whisper-adapters/src/parakeet_live.rs:1438` | MINEUR | Planned | S6 | `collapsible_if` |
| A50 `crates/whisper-adapters/src/parakeet_live.rs:1721` | MINEUR | Planned | S6 | `collapsible_if` |
| A51 `crates/whisper-adapters/src/parakeet_live.rs:1722` | MINEUR | Planned | S6 | `collapsible_if` |
| A52 `crates/whisper-adapters/src/parakeet_live.rs:2193` | IMPORTANT | Planned | C3 | `too_many_arguments` |
| A53 `crates/whisper-adapters/src/parakeet_live.rs:2548` | IMPORTANT | Planned | C3 | `too_many_arguments` |
| A54 `crates/whisper-adapters/src/parakeet_pipeline.rs:479` | MINEUR | Planned | S5 | `needless_borrow` |
| A55 `crates/whisper-adapters/src/path_settings.rs:166` | MINEUR | Planned | S4 | `new_without_default` |
| A56 `crates/whisper-adapters/src/queue_store.rs:492` | MINEUR | Planned | S4 | `collapsible_if` |
| A57 `crates/whisper-adapters/src/source_binding.rs:761` | IMPORTANT | Planned | P4 | `cmp_owned` |
| A58 `crates/whisper-adapters/src/source_binding.rs:762` | IMPORTANT | Planned | P4 | `cmp_owned` |
| A59 `crates/whisper-adapters/src/source_binding.rs:887` | IMPORTANT | Planned | P4 | `question_mark` |
| A60 `crates/whisper-adapters/src/source_binding.rs:1245` | IMPORTANT | Planned | P1 | `suspicious_open_options` |
| A61 `crates/whisper-adapters/src/source_binding.rs:1344` | IMPORTANT | Planned | P4 | `collapsible_if` |
| A62 `crates/whisper-adapters/src/source_binding.rs:1366` | IMPORTANT | Planned | P1 | `suspicious_open_options` |
| A63 `crates/whisper-adapters/src/source_binding.rs:1436` | IMPORTANT | Planned | P4 | `cmp_owned` |
| A64 `crates/whisper-adapters/src/windows_file_identity.rs:137` | IMPORTANT | Planned | P2 | `unnecessary_cast` |
| A65 `crates/whisper-adapters/src/windows_file_identity.rs:155` | IMPORTANT | Planned | P2 | `unnecessary_cast` |
| A66 `crates/whisper-adapters/src/windows_file_identity.rs:168` | IMPORTANT | Planned | P2 | `unnecessary_cast` |
| A67 `crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:303` | IMPORTANT | Planned | E1 | `large_enum_variant` |
| A68 `crates/whisper-adapters/src/worker_ipc/parakeet_unified/mod.rs:105` | IMPORTANT | Planned | E3 | `large_enum_variant` |
| A69 `crates/whisper-adapters/src/worker_ipc/runtime/live.rs:1318` | MINEUR | Planned | T2 | `let_unit_value` |
| A70 `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:387` | IMPORTANT | Planned | C4 | `too_many_arguments` |
| A71 `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs:465` | MINEUR | Planned | T3 | `collapsible_if` |
| A72 `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:347` | MINEUR | Planned | T3 | `question_mark` |
| A73 `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:21` | IMPORTANT | Planned | C5 | `too_many_arguments` |
| A74 `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:667` | MINEUR | Planned | T3 | `collapsible_if` |
| A75 `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:886` | MINEUR | Planned | T3 | `collapsible_if` |
| A76 `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:53` | MINEUR | Planned | T4 | `needless_borrow` |
| A77 `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:281` | MINEUR | Planned | T4 | `collapsible_if` |
| A78 `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:15` | MINEUR | Planned | T4 | `map_entry` |
| A79 `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:3` | IMPORTANT | Planned | E2 | `large_enum_variant` |
| A80 `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:416` | MINEUR | Planned | T1 | `collapsible_if` |
| A81 `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:448` | MINEUR | Planned | T1 | `collapsible_if` |
| A82 `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:71` | IMPORTANT | Planned | C6 | `type_complexity` |
| A83 `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:174` | IMPORTANT | Planned | C6 | `too_many_arguments` |
| A84 `crates/whisper-adapters/src/worker_ipc/service/mod.rs:35` | IMPORTANT | Planned | C6 | `type_complexity` |
| A85 `crates/whisper-adapters/src/worker_ipc/service/mod.rs:159` | IMPORTANT | Planned | C6 | `too_many_arguments` |
| D01 `crates/whisper-desktop/src/ui/model/view/microphone.rs:246` | MINEUR | Planned | D1 | unused variable: `response` |
| D02 `crates/whisper-desktop/src/ui/presentation/selection.rs:86` | MINEUR | Planned | D1 | unused variable: `model` |
| D03 `crates/whisper-desktop/src/ui/model/preferences.rs:62` | IMPORTANT | Planned | D4 | `result_unit_err` |
| D04 `crates/whisper-desktop/src/ui/model/preferences.rs:78` | IMPORTANT | Planned | D4 | `result_unit_err` |
| D05 `crates/whisper-desktop/src/ui/model/view/capture.rs:319` | MINEUR | Planned | D2 | `collapsible_if` |
| D06 `crates/whisper-desktop/src/ui/model.rs:95` | MINEUR | Planned | D3 | `type_complexity` |
| D07 `crates/whisper-desktop/src/lib.rs:6` | IMPORTANT | Planned | D5 | `too_many_arguments` |
| D08 `crates/whisper-desktop/src/ui/model/view/unified.rs:287` | MINEUR | Planned | D1 | unused return value of `eframe::egui::Ui::selectable_label` that must be used |
| D09 `crates/whisper-desktop/src/ui/model/view/unified.rs:337` | MINEUR | Planned | D1 | unused return value of `eframe::egui::Ui::selectable_label` that must be used |
| D10 `crates/whisper-desktop/src/ui/model/view/unified.rs:341` | MINEUR | Planned | D1 | unused return value of `eframe::egui::Ui::selectable_label` that must be used |
| D11 `crates/whisper-desktop/src/ui/tests/support.rs:434` | MINEUR | Planned | D2 | `type_complexity` |
| D12 `crates/whisper-desktop/src/root.rs:837` | MINEUR | Planned | D2 | `op_ref` |
| D13 `crates/whisper-desktop/src/root.rs:844` | MINEUR | Planned | D2 | `op_ref` |
| Autres warnings tests adaptateurs/profils/workers non mesurés | À qualifier | Deferred | G-SCOPE | Ne pas annoncer zéro ni les corriger sans extension précise |

## 5. Recommended Sequence

Six lots successifs et un seul rapport. Les tâches comprennent leurs tests/contrôles ; préflight et revue/clôture constituent chacun une étape commune par lot. Comptes maximaux : P=4+2, S=6+2, T=4+2, E=3+2, C=8+2, D=5+2 ; aucun lot ne dépasse dix tâches. Ne pas traiter ces six lots comme un lot d'implémentation monolithique. Revue indépendante et commit local de chaque lot fermé avant le suivant, au plus deux cycles par livrable.

| Phase | Objective / Closed invariant | Before metrics → Target metrics | Metric commands | Dependencies / Stop conditions / Expected outcome |
| --- | --- | --- | --- | --- |
| P — verrous et preuves | Nettoyage sans perte de gardien ni modification disque | 17 sites ciblés → 0, trois locks explicitement non tronquants | V-LOCK/V-SOURCE/V-UNIT/V-BASE, inspection durée de vie | GO P et campagne ; arrêter si exclusion/contenu/reprise diverge ; tranche fermée de persistance |
| S — simplifications locales | Zéro diagnostic dans les 30 sites S | 30 → 0 | V-UNIT/V-BASE/V-F et sous-tests par tâche | P fermé ; arrêter si effet déplacé ou test manquant ; pas de changement d'API |
| T — effets et polling | Même ordre des effets avec les 19 formes simplifiées | 19 → 0 | V-DUP/V-UNIT/V-SOURCE/V-BASE | S fermé ; arrêter sur erreur de routage/génération/ACK ; aucune enum modifiée |
| E — représentation interne | Trois grandes variantes réduites, propriété et format inchangés | 3 → 0 ; tailles journalisées272/576/840 sans engagement ABI | V-UNIT/V-BASE ; recherche constructeurs et lecture Drop | T fermé ; arrêter si plus de Box que prévu, format ou canal changé ; trois enums fermés |
| C — signatures et types adapters | Huit tâches nomment les données sans nouvelle politique | 16 → 0, fonctions ciblées ≤7 arguments et types complexes nommés | V-UNIT/V-BASE/V-ASTRICT ; inspection callbacks/borrows | E fermé ; arrêter si allocation callback, alias change Send/Sync, API non inventoriée ; adapters lib strict visé |
| D — desktop et tests observés | Même interface et préférences, contrats Rust coordonnés | 13 → 0, zéro site inventorié restant sur 98 | V-DESKTEST/V-BASE/V-DSTRICT/V-F | C fermé ; arrêter si choix UI/routage/notice change ; strict desktop all-targets de référence visé |

Les comptes sont des **sites du relevé de référence**, pas un engagement sur le nombre exact de warnings intermédiaires : compilation/test peut ajouter ou dédupliquer des diagnostics. Tout diagnostic nouveau doit être attribué, inscrit dans ce rapport et soumis au gate de périmètre, pas corrigé automatiquement.

## 6. Task Backlog

Contrat commun à chaque tâche :
- Périmètre = fichiers nommés + adaptations des tests directement consommateurs. Budget par défaut ≤3 fichiers production, précisé ci-dessous ; toute extension non listée exige arbitrage. Un fichier cité comme test n'est pas de production.
- Dépendance = lot précédent fermé puis tâches du lot dans l'ordre de leur numéro. Avant le premier Cargo du lot, préflight/garde/auto-tests conformes et baseline du lot ; une autorisation passée ne dispense pas de ces entrées.
- Closed invariant : les sites de la tâche disparaissent et son critère de conservation reste vrai. Before metrics : nombre de sites issu du tableau ; Target metrics : zéro pour ces sites, aucun nouveau diagnostic non attribué. Metric commands : V-BASE et commandes listées.
- Stop conditions : test/build/commande requise en échec inattendu, préflight/garde invalide, écriture externe bloquante, conflit de travail tiers, critère partiel, besoin d'élargir API/format/visibilité ou budget. Ne pas passer à la tâche/phase suivante avec une fermeture partielle.
- Rollback/fallback : revenir seulement sur le delta attribuable et coordonné de cette tâche si nécessaire, conserver journaux ; pas de reset global, nettoyage de caches, allow/expect ou diminution des seuils. Rendre les réserves après deux cycles.
- Notes for implementation agent : preserve non-suivis et changements tiers ; ne pas ajouter de helper/outillage autonome. Aucun nouveau scénario inline ; exceptions privées existantes conservées, nouveaux tests séparés. Les actions décrites ci-dessous sont proposées, non exécutées.

### P1 — Expliciter les trois ouvertures de verrou et leur garde RAII

- Priority : High. Source finding : A17, A24, A60, A62. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/archive/publication/catalog.rs ; adapters/src/source_binding.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 2 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 4 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : phase précédente fermée et préflight du lot (P1 : GO initial).
- Implementation outline : Ajouter truncate(false) aux trois OpenOptions de fichiers .lock, sans toucher aux ouvertures de données déjà truncate(true). Transformer OperationGuardProbe::Acquired(File) en variante nommée Acquired { _guard: File }, adapter constructeur et matches, conserver l'objet probe dans sa portée actuelle. Traiter le contenu du lock comme conservé et son handle comme gardien de durée de vie.
- Tests to add or update : Nouveau adapters/tests/publication_lock_contract.rs : précréer un lock avec contenu sentinelle, acquisition sans modification, seconde tentative non bloquante refusée tant que le gardien existe, puis possible après Drop. Réutiliser tests/source_binding.rs et le test privé existant recovery_refuses_reserved_operation_while_acquire_holds_the_cross_process_lock.
- Verification commands : V-LOCK, V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Trois suspicious_open_options et A17 absents ; contenu et exclusion inchangés. Ne jamais remplacer File par () ni raccourcir sa portée.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### P2 — Conserver la preuve d'identité Windows en supprimant les conversions superflues

- Priority : High. Source finding : A19, A64, A65, A66. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/windows_file_identity.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 4 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Renommer le champ stocké inutilisé VerifiedSource.identity en _identity, avec construction adaptée, pour conserver l'appel identity_for_handle et sa possibilité d'échec. Retirer les trois casts de pointeur identiques, sans modifier valeurs, flags, signatures FFI ni blocs unsafe.
- Tests to add or update : Réutiliser tests privés windows_file_identity (dont deletion_is_applied_to_the_open_identity) et tests/source_binding.rs ; pas de nouvelle visibilité pour test.
- Verification commands : V-UNIT, V-SOURCE, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : A19 et trois casts absents ; handle _file, hashing et validations d'identité préservés. Aucun élargissement unsafe.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### P3 — Nettoyer les liaisons inutiles de publication

- Priority : Medium. Source finding : A06, A07, A08. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/archive/cleanup/proofs.rs ; adapters/src/archive/journal.rs ; adapters/src/archive/publication.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 3 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Remplacer le find de l'ancêtre suivi d'un binding unused par any ou un test de présence équivalent ; retirer mut sur document ; retirer l'affectation plan=existing après avoir conservé lecture, comparaison et erreur DuplicateStateChanged, puis mut du plan si devenu inutile. Ne pas retirer lecture de plan durable ni write_cleanup_plan dans l'autre branche.
- Tests to add or update : Réutiliser archive/tests/recovery.rs et tests privés archive de journal/cleanup ; aucune nouvelle matrice native.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : A06–A08 absents ; validation durable et séquence de publication identiques.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### P4 — Simplifier les vérifications de snapshots sans changer leur identité

- Priority : Medium. Source finding : A18, A57, A58, A59, A61, A63. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/source_binding.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 6 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Comparer les chemins par Path::new sur chaînes/OsStr empruntés ; conserver les comparaisons de chemins, pas de comparaison textuelle approximative ni canonicalize supplémentaire. Remplacer le retour d'erreur inchangé par ?. Fusionner le if ciblé en conservant sa branche d'erreur. Supprimer release_record seulement après recherche confirmant zéro appel dans toutes configurations/tests ; la libération active finish_retirement/release reste intacte.
- Tests to add or update : tests/source_binding.rs ; conserver scénarios de corruption, reprise et disposition ; pas d'appel de release_record artificiel pour taire Clippy.
- Verification commands : V-SOURCE, V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Six sites fermés ; mêmes erreurs et capacités de snapshots, aucune suppression durable déplacée.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### S1 — Simplifier les prédicats d'archive

- Priority : Medium. Source finding : A21, A22, A23, A30. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/archive/cleanup/execution.rs ; adapters/src/archive/identity.rs ; adapters/src/archive/recovery.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 4 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : phase précédente fermée et préflight du lot (P1 : GO initial).
- Implementation outline : Chaîne let pour le if imbriqué ; is_multiple_of pour les deux modulos avec diviseur identique ; motif plage inclusif pour les codes d'erreur contigus, sans élargir les autres codes.
- Tests to add or update : Tests privés archive cleanup/identity/recovery existants ; pas de nouveau test miroir de syntaxe.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Quatre diagnostics fermés ; mêmes branches/erreurs, mêmes checks de corruption.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### S2 — Simplifier l'affichage d'erreur du catalogue de publication

- Priority : Low. Source finding : A27, A28. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/archive/publication/catalog.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 2 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Remplacer le motif OR par sa plage exacte ; fusionner format! imbriqués sans changer le texte, les identifiants ou le rendu du chemin.
- Tests to add or update : Tests privés archive/recovery existants ; comparer les messages avant/après dans le diff.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : A27/A28 absents, messages et choix des codes inchangés.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### S3 — Nettoyer l'inventaire GPU et la sonde micro

- Priority : Medium. Source finding : A33, A34, A36, A38. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/gpu_inventory/windows_native.rs ; adapters/src/gpu_inventory.rs ; adapters/src/microphone_probe.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 4 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Initialiser les deux structures Windows avec champs et ..Default::default() ; enlever le return terminal inutile, pas son eprintln ; chaîne let dans la sonde micro sans changer reprise ou sélection par défaut. Les types/signatures C1 sont différés au lot C.
- Tests to add or update : tests/gpu_inventory.rs, tests/microphone_probe.rs et tests privés inventaire existants ; ne pas appeler inventaire machine réel pour un test synthétique.
- Verification commands : V-GPU, V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Quatre diagnostics fermés ; mêmes flags Windows, diagnostics et politique micro.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### S4 — Simplifier erreurs I/O, préférences et file

- Priority : Medium. Source finding : A39, A40, A41, A42, A43, A55, A56. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/model_catalog.rs ; adapters/src/path_settings.rs ; adapters/src/queue_store.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 7 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Remplacer les cinq Error::new(Other,...) par Error::other avec charge utile conservée ; Default for WindowsModelPreferencePort délègue à new() sans nouvel accès environnement supplémentaire ; réunir le if de queue_store en gardant ordre de vérification.
- Tests to add or update : tests/model_catalog.rs, tests/model_preferences.rs ; tests privés queue_store. Aucun téléchargement, modèle ou test d'installation réel.
- Verification commands : V-MODEL, V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Sept diagnostics fermés ; chemins, message et moment d'accès environnement identiques.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### S5 — Simplifier stockage et lecture PCM

- Priority : High. Source finding : A14, A44, A45, A54. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/parakeet_audio.rs ; adapters/src/parakeet_document.rs ; adapters/src/parakeet_pipeline.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 4 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Retirer seulement le champ PcmPartWriter.index_path et son stockage : conserver la variable locale qui crée le fichier, le handle index et le champ distinct de PcmPartStore utilisé par read_page. Passer chunks_exact(2) à as_chunks::<2>().0.iter(), conserver décodage little endian et traitement du reste. is_multiple_of et retrait d'emprunt inutiles aux deux autres sites.
- Tests to add or update : Tests privés PCM/document/pipeline existants, en particulier limites de partie, reprise, index corrompu et couverture ; aucun nouvel accès au corpus.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Quatre sites fermés ; format WAV/index, découpage et acceptation/rejet des restes identiques.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### S6 — Distinguer production et supports privés Parakeet

- Priority : High. Source finding : A01, A15, A16, A46, A47, A48, A49, A50, A51. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/parakeet_live.rs ; adapters/src/parakeet_import.rs ; adapters/src/parakeet_confirmation.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 9 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Retirer import MicrophoneSpectrumReader inutilisé. Pour session_id/destination, rechercher consommateurs toutes features/tests et supprimer si zéro, sinon cfg(test) uniquement si usage exclusivement test. finalize/ingest de Confirmation ont de nombreux appels dans tests : les marquer cfg(test), garder finalize_observed/ingest_observed de production. Simplifier Ok(...?), dériver Default seulement pour D2WorkerDiagnostics non-test (préserver impl cfg(test) et mutex), réunir les trois if sans déplacer effets. Comparer native_dir sans PathBuf alloué, même égalité OsStr/Path.
- Tests to add or update : Tests privés confirmation/live/import et tests/parakeet_command_wire.rs ; conserver les scénarios de confirmation existants. Fonctions d'inférence à neuf/douze paramètres restent pour C3.
- Verification commands : V-UNIT, V-WIRE, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Neuf diagnostics fermés ; aucun test supprimé, fermeture/confirmation/cancel inchangés, Default test inchangé.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### T1 — Simplifier les expressions des effets de file

- Priority : Medium. Source finding : A02, A03, A04, A05, A80, A81. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc/runtime/duplicates.rs ; adapters/src/worker_ipc/runtime/queue.rs ; adapters/src/worker_ipc/runtime/support.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 6 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : phase précédente fermée et préflight du lot (P1 : GO initial).
- Implementation outline : Retirer les quatre () terminaux de closures en gardant affectation request et chaîne and_then ; réunir les deux if de support sans modifier les refus. Ne pas traiter PreflightResult avant E2.
- Tests to add or update : Réutiliser duplicate_confirmation_short_refusals_preserve_runtime_identity et tests privés reprise/file.
- Verification commands : V-DUP, V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Six diagnostics fermés ; ordre binding/validation/persistance inchangé.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### T2 — Limiter aux tests les observations de PID

- Priority : High. Source finding : A09, A10, A11, A13, A69. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc/parakeet_unified/native.rs ; adapters/src/worker_ipc/runtime/live.rs ; adapters/src/worker_ipc/service/automatic_final.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 5 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Garder les deux lectures pid sous cfg(test), avant terminate et observer Reaped ; ne jamais supprimer terminate. Retirer le SchedulerSession local réellement inutilisé ; ignorer generation dans le destructuring de l'effet automatique ; remplacer Ok(applied)/let _=applied par Ok(()) dans set_gain sans changer son appel ni les événements de refus.
- Tests to add or update : Tests privés native_execution/unified_capture existants ; vérification des observations d'incarnation et des rejets de gain.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Cinq sites fermés ; ordre stop/reap et gestion de gain identiques en test et production.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### T3 — Simplifier les branches de polling

- Priority : High. Source finding : A12, A71, A72, A74, A75. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc/runtime/poll_unified.rs ; adapters/src/worker_ipc/runtime/poll/parakeet.rs ; adapters/src/worker_ipc/runtime/poll.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 5 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Retirer mut executor ; chaînes let aux trois sites ciblés, conserver retraits/réinsertion de sessions et génération ; utiliser ? sur self.active.as_ref() dans la fonction Option sans changer ses effets antérieurs. Contextes de paramètres différés C4/C5.
- Tests to add or update : Tests privés worker IPC native_execution/unified_import/unified_capture existants ; aucun worker natif réel.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Cinq sites fermés ; même drainage/ack et rejet des événements anciens.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### T4 — Simplifier reprise et acquisition de source

- Priority : High. Source finding : A76, A77, A78. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc/runtime/recovery.rs ; adapters/src/worker_ipc/runtime/source_binding.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 2 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 3 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Retirer l'emprunt immédiatement redéréférencé ; chaîne let ciblée ; utiliser Entry::Vacant pour acquire+insert. acquire doit rester paresseux et appelé une seule fois pour une clé absente, jamais pour une clé occupée ; conserver les validations même lorsque binding déjà présent.
- Tests to add or update : Tests privés resume_and_admission et tests/source_binding.rs ; ajouter si absent un test public de réutilisation sans nouvel acquire et d'échec sans insertion, sans visibilité nouvelle.
- Verification commands : V-UNIT, V-SOURCE, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Trois sites fermés ; pas de copie snapshot ni I/O supplémentaire, mêmes refus.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### E1 — Réduire WorkerCommandPoll sans changer les canaux

- Priority : High. Source finding : A67. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc/ipc/mod.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 1 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : phase précédente fermée et préflight du lot (P1 : GO initial).
- Implementation outline : Boxer seulement WorkerCommandPoll::Command. Adapter take_next_worker_command (quatre retours Command) et déballer au writer avant sérialisation. Canaux existants continuent à transporter WorkerCommand ; pas de changement du protocole Turbo ou des capacités.
- Tests to add or update : Tests privés IPC/worker_control existants ; adapter constructeurs de tests, vérifier Stop/Shutdown prioritaire, Empty/Disconnected et drains sans hang.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Enum au moins272 octets → diagnostic absent ; même priorité et fermeture, une allocation par commande, aucune allocation sur Idle/Closed.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### E2 — Réduire PreflightResult à charge utile possédée

- Priority : High. Source finding : A79. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc/runtime/support.rs ; adapters/src/worker_ipc/runtime/duplicates.rs ; adapters/src/worker_ipc/runtime/queue.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 1 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Boxer seulement Duplicate(DuplicateImportPrompt), adapter tous les constructeurs des deux producteurs et déballer à la création ImportEvent::DuplicateImportDetected qui garde son type. Queued, journaux et types core restent inchangés.
- Tests to add or update : Tests privés doublons/file/resume ; adapter assertions si nécessaire, ne pas modifier leurs oracles.
- Verification commands : V-DUP, V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Enum au moins576 octets → diagnostic absent ; prompts/identités et événements métier inchangés.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### E3 — Réduire UnifiedSessionSource en conservant la capture

- Priority : High. Source finding : A68. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc/parakeet_unified/mod.rs ; adapters/src/worker_ipc/runtime/live.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 2 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 1 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Boxer uniquement Capture(UnifiedCapturePump), constructeur runtime/live et constructeurs de tests ; les méthodes read_window/pump/stop/set_gain/take_gain_events conservent la délégation par autoderef. Closed(PcmPartStore) inchangé, pas d'Arc ni clone du pump.
- Tests to add or update : Tests privés unified_capture et native_execution ; vérifier arrêt/Drop, gain et derniers ACK ; compilation tests ignorés sans exécution.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Enum au moins840 octets → diagnostic absent ; un propriétaire et une allocation à création session, aucun changement de durée de vie/audio annoncé durable.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### C1 — Nommer le résultat et le contexte de journal GPU

- Priority : Medium. Source finding : A35, A37. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/gpu_inventory.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 2 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : phase précédente fermée et préflight du lot (P1 : GO initial).
- Implementation outline : Alias privé pour (generation, Result<Vec<GpuRuntimeIdentity>,String>) et Receiver associé ; struct empruntée InventoryLogSnapshot pour les sept données de append_inventory_log, chemin séparé. Adapter appel production et appels de tests, ne pas changer le JSON ni timestamp.
- Tests to add or update : Tests de append_inventory_log existants (lignes737–800), tests/gpu_inventory.rs ; comparer valeurs JSON avec génération/errors/options.
- Verification commands : V-GPU, V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Deux diagnostics absents, fonction ≤7 arguments, mêmes messages/JSON et pas de clone de collections.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### C2 — Regrouper l'état du callback audio

- Priority : High. Source finding : A32. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/capture.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 1 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Contexte privé emprunté CaptureCallbackState pour capacity, free/ready, status, gain_state et next_frame ; data, convert et channels restent explicites. Adapter trois callbacks de formats et les tests locaux. Ne pas allouer/locker/cloner dans le callback ; conserver atomiques et leurs Ordering.
- Tests to add or update : Tests privés capture f32/i16/u16, saturation, gain/révision et frontière trame ; tests/live_capture.rs compilés, exécution uniquement si prérequis autorisés.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Neuf paramètres → ≤7 ; aucune allocation/latence/contre-pression nouvelle dans boucle temps réel.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### C3 — Regrouper les paramètres de l'inférence live

- Priority : High. Source finding : A52, A53. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/parakeet_live.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 2 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Créer un contexte possédé de démarrage thread pour paths/request/candidate/session_log_root ; garder cancel, requests, events, trace_enabled et diagnostics explicites (six paramètres après groupement). Créer WindowRequestContext<'a> avec request, identity, native_call_marker_required, expected_gpu, attempt et trace_enabled ; garder process &mut, cancel, path, sha256, expected_samples et global_window_start_sample explicites (sept paramètres après groupement). Conserver les callbacks d'observation aux mêmes instants. Adapter unique spawn inference_thread et appel transcribe_window et tests.
- Tests to add or update : Tests privés live/confirmation et routage existants ; contrôles Auto/GPU/CPU synthétiques, natif NOT RUN.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Deux fonctions neuf/douze paramètres → chacune ≤7 ; mêmes identités, cancel, timeouts et séquence d'appel natif.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### C4 — Nommer les données du résultat VAD

- Priority : High. Source finding : A70. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc/runtime/poll/parakeet.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 1 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Struct locale de résultat portant request_id/job_id/generation/attempt/identity/source_sha256/duration_samples/words ; construire dans l'arm Event::Result puis transmettre à handle_parakeet_vad_result. Ne pas changer Event core sérialisé ni extraire une validation plus tard.
- Tests to add or update : Tests privés VAD/worker IPC et tests/vad_contract.rs ; conserver checks exacts d'identité/durée et rejet stale.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Neuf arguments → ≤7 ; mêmes refus avant publication.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### C5 — Regrouper les références du repli de démarrage

- Priority : High. Source finding : A73. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc/runtime/poll_unified.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 1 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Contexte emprunté NativeRetryState regroupant scheduler/sessions/archives/executor_route/executor ; garder current_state/call/cause et les deux fabriques injectées explicites. Adapter les deux appels production et worker_ipc/tests/native_execution.rs. Aucun mem::take ou clonage d'état pour contourner un borrow.
- Tests to add or update : Tests privés native_execution (appel de ligne302), scénarios Auto partagé, GPU manuel, moteur déjà prêt et appel audio commencé.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Dix arguments → ≤7 ; une seule tentative CPU avant audio, aucun repli manuel ou après démarrage audio.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### C6 — Nommer la configuration et les gates des workers

- Priority : High. Source finding : A20, A82, A83, A84, A85. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/worker_ipc.rs ; adapters/src/worker_ipc/runtime/mod.rs ; adapters/src/worker_ipc/service/mod.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 5 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Définir au parent un alias PublicationGateRegistry conservant exactement Arc<Mutex<HashMap<(JobId,u64),Arc<PublicationGate>>>> et une configuration interne empruntée/possédée selon usages pour routes Parakeet et registre de snapshots. Les deux helpers à huit arguments reçoivent ce groupement, adapter appels service et tests. new_with_workers_and_parakeet est utilisé via new_with_catalog cfg(test) : le marquer cfg(test), ne pas supprimer la chaîne de constructeurs de tests. Ne pas modifier les façades publiques AsyncImportIo si non nécessaire.
- Tests to add or update : Tous constructeurs tests native_execution/unified_import/unified_capture compilés ; tests privés startup et publication/gates.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Cinq diagnostics absents ; mêmes booléens/racines/options, capacités des canaux et Arc partagé, pas nouvel état global.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### C7 — Nommer les paramètres de confirmation de publication

- Priority : High. Source finding : A25, A26. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/archive.rs ; adapters/src/archive/publication/catalog.rs ; adapters/src/worker_ipc/runtime/duplicates.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 2 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Créer un paramètre public PublicationReconfirmation<'a> minimal pour source_sha256/source_samples/expected_active/choice/reserved_basename/confirmed_basename, défini dans archive.rs ; root et previous restent séparés. Adapter l'appel duplicates et archive/tests/recovery.rs. Pour write_publication_reservation_with_storage utiliser un paramètre privé de réservation (valeurs possédées) ; préserver sérialisation PublicationIntent et ordre des contrôles. Pas de nouveau format ni nouvelle politique.
- Tests to add or update : Tests privés archive/tests/recovery.rs et tests/publication_lock_contract.rs du lot P ; tests reprocessing_* seulement compilés sauf recette distincte.
- Verification commands : V-UNIT, V-LOCK, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Deux fonctions à huit arguments → ≤7 ; rupture API source Rust bornée à la confirmation, JSON réservations/nom/export et revalidation inchangés.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### C8 — Nommer le callback d'export et son contexte

- Priority : High. Source finding : A29, A31. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : adapters/src/archive.rs ; adapters/src/archive/publication/exports.rs.
- Ownership bucket : adaptateurs, responsabilité des modules nommés.
- Scope budget : 2 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 2 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Alias interne ExportHardLinker pour le champ Arc<dyn Fn(&Path,&Path)->io::Result<()>+Send+Sync>, sans modifier dyn/Send/Sync. Regrouper hard_linker et after_final_copy dans un contexte emprunté ExportInstallOps pour install_owned_export ; adapter quatre appels. Garder les callbacks au même instant et l'ordre copie finale/identité/ownership.
- Tests to add or update : Tests privés publication/exports existants, injection hard_linker et after_final_copy ; perte/corruption et reprise testées sans modèle.
- Verification commands : V-UNIT, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Deux diagnostics absents ; même propriété des exports et atomicité, aucune nouvelle dépendance archive→worker_ipc.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### D1 — Conserver les réponses UI utiles aux tests et expliciter les autres

- Priority : Medium. Source finding : D01, D02, D08, D09, D10. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : desktop/src/ui/model/view/microphone.rs ; desktop/src/ui/presentation/selection.rs ; desktop/src/ui/model/view/unified.rs.
- Ownership bucket : interface/composition et tests.
- Scope budget : 3 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 5 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : phase précédente fermée et préflight du lot (P1 : GO initial).
- Implementation outline : Nommer _response le résultat colored_label et utiliser _response.rect sous cfg(test) : conserver le widget et son rectangle. Renommer le paramètre inutilisé model en _model, garder model_supports_live toujours true. Pour trois selectable_label, garder le widget inchangé et consommer explicitement Response par let _ = ; ne pas créer une interaction ou navigation nouvelle.
- Tests to add or update : Tests/microphone_level_ui.rs et model_selection_ui.rs ; tests privés UI affichage unifié. Pas de test miroir du let _ ; fenêtre Windows NOT RUN.
- Verification commands : V-DESKTEST, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Cinq sites absents ; mêmes textes/boutons et sémantique de sélection, instrumentation test préservée.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### D2 — Simplifier prédicat UI et supports de tests

- Priority : Medium. Source finding : D05, D11, D12, D13. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : desktop/src/ui/model/view/capture.rs ; desktop/src/root.rs ; desktop/src/ui/tests/support.rs.
- Ownership bucket : interface/composition et tests.
- Scope budget : 3 dont un fichier test fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 4 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Chaîne let du bloc capture sans changer la gestion d'hypothèse provisoire ; comparer configured==expected sans emprunt redondant dans les deux tests de racine ; alias privé ControlledGpuResponseQueue de même Rc<RefCell<VecDeque<...>>> dans support tests.
- Tests to add or update : Tests privés UI/root et tests/gpu_inventory_ui.rs ; préserver les racines de fixtures existantes plutôt que les élargir pour lancer la suite.
- Verification commands : V-DESKTEST, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Quatre sites fermés (un lib, trois test) ; mêmes racines et comportement provisoire.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### D3 — Nommer la clé de requête audio unifiée

- Priority : Medium. Source finding : D06. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : desktop/src/ui/model.rs.
- Ownership bucket : interface/composition et tests.
- Scope budget : 1 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 1 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Alias privé UnifiedAudioRequestKey de l'actuel tuple à sept composantes ; garder Option, ordre, Eq et les constructeurs/comparaisons tuple de model/view/unified.rs inchangés. Aucune nouvelle struct si un alias suffit.
- Tests to add or update : Tests privés UI unifiée couvrant pagination, révision et requêtes périmées ; pas de migration du modèle UI.
- Verification commands : V-DESKTEST, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Type_complexity absent ; cache invalide aux mêmes changements de job/génération/révision/frontières/phase.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### D4 — Typer les erreurs du canal de préférences

- Priority : High. Source finding : D03, D04. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : desktop/src/ui/model/preferences.rs ; desktop/src/ui/presentation/selection.rs ; desktop/src/ui/model.rs ; desktop/src/ui.rs.
- Ownership bucket : interface/composition et tests.
- Scope budget : 4 (exception atomique API publique) fichiers production au maximum, plus tests directement correspondants ; exception de quatre fichiers justifiée par la signature et ses réexports/consommateurs à changer atomiquement.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 2 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Enum PreferenceChannelError { Poisoned, Disconnected }, derive Debug et impl Display/Error ; send/recv renvoient ce type, map_err aux deux causes existantes, Full reste succès. Réexporter via façades model/ui et adapter record_preference_delivery(Result<(),PreferenceChannelError>), sans changer message utilisateur ni sa décision is_err. root boucle while let Ok et settings appels par inférence doivent rester inchangés si compilent.
- Tests to add or update : Tests/model_selection_ui.rs : conserver mailbox bornée/latest-choice/disconnected ; ajouter assertions sur Disconnected et fermeture receiver, aucun nouveau bulk inline. Le type opaque pending ne doit pas être exposé pour provoquer poison artificiellement.
- Verification commands : V-DESKTEST, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Deux result_unit_err absents ; API coordonnée, dernière valeur conservée, aucun canal non borné ; exception de quatre fichiers justifiée par signature/reexports/consommateur publics inséparables.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

### D5 — Regrouper la composition de démarrage desktop

- Priority : High. Source finding : D07. Rationale : fermer ces diagnostics en préservant l'invariant décrit dans l'acceptation.
- Target files/modules : desktop/src/lib.rs ; desktop/src/main.rs.
- Ownership bucket : interface/composition et tests.
- Scope budget : 2 fichiers production au maximum, plus tests directement correspondants ; aucun dépassement prévu.
- Closed invariant / Before metrics / Target metrics : critère ci-dessous ; 1 site(s) → zéro. Metric commands : V-BASE et contrôles ci-dessous. Stop conditions : contrat commun et divergence du critère ci-dessous.
- Dependencies : tâche précédente du même lot fermée.
- Implementation outline : Struct publique DesktopStartup contenant les neuf paramètres actuels hors application, avec noms et types identiques ; run(application,startup), destructurer avant run_native et conserver les builders et leur ordre. Mettre à jour l'unique appel main identifié. Pas de changement de root::compose, routage personnel/Test ou démarrage du moteur.
- Tests to add or update : Compilation bin/lib tous profils prévus ; tests root et UI existants ; pas d'ouverture de fenêtre automatique.
- Verification commands : V-DESKTEST, V-BASE, V-F (§7). Command provenance : Source inferred ; aucun Cargo exécuté lors de la rédaction.
- Acceptance criteria : Dix arguments →2 ; mêmes ressources, workers, timestamps et options, aucun déplacement d'I/O ou délai.
- Rollback/fallback notes : annuler le seul delta coordonné de la tâche si nécessaire ; conserver les preuves. Notes for implementation agent : contrat commun ci-dessus, pas de cleanup voisin.

## 7. Verification Matrix

**Toutes les commandes Cargo de cette section : proposées, NOT RUN pendant la planification.** Le seul run observé est le journal de référence. N'exécuter aucune de ces commandes avant GO précis, environnement et garde.

Racine future proposée : `target/parakeet-tests/clippy-adapters-desktop/`, cible unique `target/parakeet-tests/cargo-shared/`. Depuis `C:\dev\whisper`, `$g = '.\target\parakeet-tests\clippy-adapters-desktop\guard.ps1'` désigne la future copie adaptée d'une garde existante, **pas un fichier aujourd'hui disponible**. CARGO_HOME, TEMP, TMP canoniques sous cette racine, cache copié sans écriture au profil ; CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only. Aucun Cargo direct, même fmt/metadata ; aucun --fix, téléchargement ou changement Cargo.lock. Relever au moins 30 Gio libres avant chaque lot.

La copie de la garde clippy-core contient une exception Temp spécifique : soit l'autoriser explicitement pour cette nouvelle campagne en maintenant toutes ses limites et auto-tests, soit la retirer et adapter les auto-tests à l'arrêt strict. Le plan ne choisit pas silencieusement une dérogation. Pas de nouveau service, compte, ACL, tâche planifiée, watcher supplémentaire ni confinement annoncé. Une notification UNATTRIBUTED n'est jamais une écriture Cargo prouvée.

Avant de lancer les tests, faire l'inventaire de leurs écritures et variables : les chemins doivent rester dans la racine de campagne. Les variables effectivement observées sont `PARAKEET_L2_ARCHIVE_ROOT` (tests/source_binding.rs), `PARAKEET_RTC02_EXEC_ROOT` (reprocessing_cleanup) et les racines historiques des fixtures desktop. Ne pas fixer de variable inconnue ni lancer en masse des scénarios exigeant modèle, DLL, workers réels ou racine historique incompatible. L'inventaire compilé `-- --list` n'exécute pas les tests. Pour une suite lib dont les prérequis ne peuvent être satisfaits sans élargir les chemins, arrêter le lot et faire arbitrer une sélection précise ; aucun « PASS » à partir d'un filtre vide ou d'un test ignoré. La compilation --all-targets couvre les constructeurs des tests ignorés sans les exécuter.

| ID / Area | Command or check | Provenance | Expected result | Covers tasks | Notes |
| --- | --- | --- | --- | --- | --- |
| V-P préflight | `git status --short` ; `git rev-parse HEAD` ; `Get-PSDrive C` ; vérification des variables/racines et syntaxe PowerShell de la garde adaptée ; `& $g -Mode SelfTest` | Repo docs | Git attribuable, ≥30 Gio, syntaxe et auto-tests PASS | Tous, avant premier Cargo de chaque lot | Pas de nouvelle exception implicite |
| V-I inventaire tests | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--lib','--','--list')` puis même commande avec `whisper-desktop` | Source inferred | Liste, prérequis et racines examinés | Tous | N'est pas une exécution réussie de tests |
| V-BASE relevé référence | `& $g -Mode Run -CargoArguments @('clippy','--offline','--locked','-p','whisper-desktop','--all-targets')` | Observed | Exit 0 ; baseline 85 adapters / 13 sites desktop ou drift expliqué ; comparer chaque ID | Tous, avant/après lot | Commande interne reproduit le run observé ; wrapper adapté futur |
| V-UNIT lib adapters | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--lib')` | Source inferred | Baseline et candidat PASS sur les mêmes tests admissibles ; bilan des ignorés | P,S,T,E,C | Prérequis V-I obligatoire ; aucune inférence native par effet de bord |
| V-LOCK | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--test','publication_lock_contract')` | Source inferred | Tests de non-troncature et durée de verrou PASS | P1,C7 | Nouvelle cible proposée, garder sous tests/ |
| V-SOURCE | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--test','source_binding')` | Source inferred | Snapshot/identité/capacité/reprise PASS | P,T4 | Variable réelle `PARAKEET_L2_ARCHIVE_ROOT`, sous racine autorisée |
| V-GPU | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--test','gpu_inventory')` | Source inferred | Scénarios synthétiques PASS | S3,C1 | N'invoque pas une qualification GPU réelle |
| V-MODEL catalogue | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--test','model_catalog')` | Source inferred | Scénarios locaux PASS | S4 | Prérequis/racines inspectés avant lancement |
| V-MODEL préférences | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--test','model_preferences')` | Source inferred | Persistance existante PASS | S4 | Même règle de racines |
| V-DUP | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--lib','duplicate_confirmation_short_refusals_preserve_runtime_identity')` | Source inferred | Un test exécuté, PASS | T1,E2 | Déjà utilisé dans la campagne core |
| V-WIRE | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--test','parakeet_command_wire')` | Source inferred | Deux tests JSON v1 PASS | S6,E | Non-régression protocole, pas d'inférence |
| V-DESKTEST défaut | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-desktop','--all-targets')` | Source inferred | Baseline/candidat mêmes scénarios PASS | D | Pas d'ouverture de fenêtre ; fixtures de test à contrôler |
| V-DESKTEST personnel | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-desktop','--all-targets','--features','personal-install')` | Source inferred | PASS distinct du profil défaut | D | Ne pas activer recette-hooks |
| V-CHECK | `& $g -Mode Run -CargoArguments @('check','--offline','--locked','--workspace','--all-targets')` | Source inferred | Consommateurs et tests ignorés compilés | Tous | Pas de lancement des suites natives |
| V-CHECK personnel | `& $g -Mode Run -CargoArguments @('check','--offline','--locked','-p','whisper-desktop','--all-targets','--features','personal-install')` | Source inferred | PASS | C,D | Routage personnel préservé |
| V-CORE | `& $g -Mode Run -CargoArguments @('clippy','--offline','--locked','-p','whisper-core','--lib','--','-D','warnings')` | Repo docs | PASS inchangé | E,C,D | Réutiliser preuve valide si aucun changement ne l'invalide |
| V-ASTRICT | `& $g -Mode Run -CargoArguments @('clippy','--offline','--locked','-p','whisper-adapters','--lib','--','-D','warnings')` | Source inferred | Exit 0, aucun warning bibliothèque | Fin C | Ne pas le lancer en connaissance d'avertissements résiduels des lots antérieurs |
| V-DSTRICT | `& $g -Mode Run -CargoArguments @('clippy','--offline','--locked','-p','whisper-desktop','--all-targets','--','-D','warnings')` | Source inferred | Exit 0 sur configuration de référence | Fin D | Différent du strict adapters all-targets ou workspace |
| V-F | `& $g -Mode Run -CargoArguments @('fmt','--all','--','--check')` ; `git diff --check` | Repo docs | PASS | Tous | Pas de rustfmt isolé sur fragments include! |
| Revue et livraison | Diff borné, comparaison journal par emplacement, revue indépendante, commit local du lot et bilan dans ce document | Repo docs | CLEAN ou réserves selon limite deux cycles | Chaque lot | Main auteur documentaire ; rust_implementer auteur Rust ; rust_review_coordinator orchestre reviewer lecture seule |
| Non exécuté / non qualifié | Strict adapters --all-targets, all-features/workspace ; tests natives/ignored ; fenêtre Windows | Source inferred | Limites explicites | Bilan final | Nouvelle portée si demandée, pas de PASS inféré |

Les commandes V-UNIT/V-DESKTEST sont des objectifs soumis au gate de prérequis, pas une permission de faire écrire des tests dans une ancienne racine hors campagne. Les suites d'intégration reprocessing/import réel ont des corpus/runtimes propres et ne sont pas requises par défaut pour les formes syntaxiques. Les tests réellement indispensables d'identité, publication, stop/reap, ACK et repli doivent toutefois être exécutables avant fermeture des tâches à risque ; si ces preuves ne sont pas accessibles, la tâche reste bloquée, pas livrée « verte » par simple compilation.

En fin de lot : archiver petites preuves avec arborescence, vérifier nombre et SHA-256 sans manifeste supplémentaire, conserver l'arrêt éventuel et les sorties, relever espace ; ne supprimer que des temporaires précisément autorisés, ne pas nettoyer cargo-shared.

## 8. Decision Gates

| Gate | Status | Blocks | Question | Recommended default | How to validate |
| --- | --- | --- | --- | --- | --- |
| G-GO | Blocking | Toute édition produit/campagne | Quel lot et quelles racines/campagnes sont précisément autorisés ? | P seul d'abord, puis lots dans l'ordre fermé | Plan Git et accord écrit courant sur le lot |
| G-TEMP | Blocking | Premier Cargo nouvelle campagne | Tolérance Temp à reprendre pour cette campagne spécifique ou arrêt strict ? | Tolérance purement Temp non attribué, explicite et bornée comme clippy-core, avec tous les autres arrêts maintenus | Accord précis ; branche et auto-tests correspondants de la garde |
| G-TEST | Validation | Fermeture de chaque tâche | Tests utiles et racines d'écriture admissibles sans ancienne recette ? | Exécuter scénarios contrôlés, préserver tests natifs ignorés ; ne pas élargir les fixtures | V-I + lecture variables/chemins et préflight avant V-UNIT |
| G-API | Validation | C7,D4,D5 | Autres consommateurs des signatures publiques que ceux recensés ? | Adapter les consommateurs workspace ensemble, aucun format modifié | Recherche et V-CHECK ; consommateur externe ou budget dépassé ⇒ arbitrage |
| G-SCOPE | Defaultable | Aucun lot déjà défini | Traiter aussi les diagnostics propres aux cibles non mesurées ? | Différer et consigner, fermer les 98 sites d'abord | Relevé distinct autorisé et nouvelle portée avant correction |
| G-UI | Defaultable | Aucune correction purement technique | Ajouter des actions de sélection/copie aux chemins ou modifier compatibilité live ? | Conserver exactement le comportement actuel | D1, aucun changement fonctionnel implicitement déduit de must_use |

Seuls les gates de produit/périmètre doivent être soumis à l'utilisateur. G-TEST et la recherche G-API sont des vérifications techniques ; les effectuer dans l'autorité déjà accordée, sans nouvelle demande pour chaque expression.

## 9. Out of Scope

- Ne pas faire : allow/expect, relèvement des seuils, changement de lints/CI, --fix, nouvelle dépendance, Cargo.lock, protocole/Serde ou format de publication modifié.
- Ne pas faire : supprimer un File gardien, modifier truncate des fichiers de données par propagation, déplacer ACK/commit/rename/cleanup, rendre SHA ou identité optionnels.
- Ne pas faire : changer core, workers, CPU/GPU/Auto/repli, politique de confirmation, routes ou installation stable ; ajouter de l'I/O en UI pour résoudre un warning.
- Ne pas faire : refaire MVC, archive ou worker IPC, migrer tous tests inline, introduire une couche générique, traits spéculatifs, canal non borné, Arc/clone pour contourner un borrow.
- Ne pas faire : supprimer instrumentation utilisée par cfg(test), ajouter des champs/hooks de production uniquement pour tests, activer les features recette-hooks/qualification.
- Ne pas faire : réutiliser tacitement la dérogation clippy-core, lancer moteurs natifs/UI, écrire hors dépôt, supprimer des fichiers tiers, créer une surveillance supplémentaire ou pousser sans accord précis.
- Aucun nouveau rapport d'audit, manifeste de signatures ou journal de qualification : analyse, inventaire et plan réunis ici. Les journaux d'exécution utiles vont uniquement dans les racines de campagne autorisées.
- Si un lot dépasse dix tâches ou dix fichiers de rapport, arrêter pour arbitrage ; les budgets comptés §5 satisfont cette limite.

## 10. Final Self-Check

| Check | Result | Notes |
| --- | --- | --- |
| Real plan content, not placeholder output | Pass | Analyse des causes et 30 tâches dans six lots ≤10 chacune avec étapes communes |
| All template sections present and in order | Pass | Header puis 1–11 |
| Every audit finding mapped | Pass | 85 A + 13 D, 98 sites uniques ; cibles non mesurées explicitement différées |
| Each task has targets/dependencies/outline/tests/verification/acceptance/rollback | Pass | Contrat commun et détail de chaque tâche |
| Structural phases/tasks have invariant, before/target metrics, stop conditions | Pass | Enums, contextes et API ; comptes de sites et seuil réel |
| Scope budgets and overruns justified | Pass | ≤3 production ; seule exception D4 quatre fichiers API coordonnée |
| Architecture, dependency direction and facades explicit | Pass | Carte §1 et absence de déplacement/couche nouvelle |
| Rust test placement explicit | Pass | Nouveau scénario lock dans tests/, conservation des supports privés ; migrations différées |
| Commands have provenance and known limitations | Pass | Aucun Cargo ici ; prérequis/racines des tests non présumés acquis |
| Decision gates name what they block | Pass | GO/règle campagne séparés des validations techniques |
| Paths real or proposed explicitly | Pass | Nouvelle campagne/test/paramètres sont des cibles futures nommées |
| Phase-by-phase scope narrow | Pass | Chaque lot ferme ses sites et invariants avant suite |

Vérifications réellement réalisées pour cette livraison : lecture des sources, journal et manifestes ; extraction/dédoublonnage en mémoire des diagnostics ; recherches des consommateurs ; état Git, lecture des consignes et `git diff --check` documentaire. Aucun Cargo, modification Rust/test/configuration/garde, modèle, moteur, fenêtre ou push. L'auteur principal écrit et commite ce seul document. L'analyse n'est pas une nouvelle preuve d'exécution.

## 11. Implementation Agent Prompt

### Bilan du lot P

Implémenteur `/root/implementation_clippy`, baseline `f41d7cca66fa62a8367d2774c6627cae23b4def7`, candidat worktree relu : six fichiers de production et nouveau `tests/publication_lock_contract.rs` conformes P1–P4. Les trois fichiers de verrou sont explicitement non tronquants ; les gardiens File et validation d'identité sont conservés, aucune lecture/comparaison de plan durable retirée. V-BASE : **85 → 68** avertissements adapters lib, les **17 sites P disparus**.

Toutes commandes Cargo via garde `target/parakeet-tests/clippy-adapters-desktop/guard.ps1`, cwd dépôt, cache/temp/TMP sous campagne et cible canonique cargo-shared, offline/locked, environnement prescrit. SelfTest PASS avec tolérance bornée et autres arrêts. Runs baseline/candidat (suffixes de `logs/guard.jsonl` et `Run-*`) : archive `304a6607`/`ac4ff369` **38 réussis et 1 ignoré** ; recovery snapshot `661f6306`/`4589989a` **3 réussis** ; identité Windows `f0b70a38`/`a1d005e5` **3 réussis** ; intégration source `04fecdc9`/`d6d652d9` **13 réussis** ; nouveau contrat verrou avant/après `8ab51dcf`/`181b847d` **1 réussi**. V-BASE `92638e65`/`1fd1f8b7` exit 0 ; check workspace/all-targets `562e0c03`, format final `c2862d15` et diff-check PASS. Les suites non sélectionnées, recettes natives et fenêtre Windows restent NOT RUN ; aucun PASS global lib ou strict global inféré.

Échecs intermédiaires conservés : `be418bad`, format de deux expressions du nouveau test, corrigé sans changement d'assertion ; `af841d19`, erreur E0596 car mut avait été retiré au mauvais document de journal.rs, réparée en rétablissant la première liaison mutable et en retirant seulement celle du site Clippy. Chaque arrêt a été signalé ; l'auteur du plan a arbitré les réparations strictement dans le périmètre du GO, puis contrôles affectés rejoués. Aucun échec ni défaut connu transmis comme fermé sans vérification.

Reviewer `/root/review_clippy/rust_reviewer_clippy_a`, via même coordinateur : **CLEAN** sur ce candidat et cette baseline, scope préflight/P1–P4 ; un passage, zéro correction de revue, aucun constat/demande/gap ouvert. Archivage par l'implémenteur sous `target/parakeet-tests/clippy-adapters-desktop/archive/lot-p-20261031/` : **76 fichiers source/76 copies**, **687 396 octets**, tous SHA-256 concordants ; aucun manifeste ni nettoyage. Espace final **64 609 779 712 octets**. Temp autorisé reste UNATTRIBUTED ; non-suivis tiers préservés. Aucun push.

Autorisation écrite de l'utilisateur dans cette session après livraison du plan : « Implémente les six lots du plan dans l’ordre, avec revues indépendantes et commits locaux, sans push. J’autorise la tolérance limitée au Temp Windows non attribué pour cette campagne, avec les autres arrêts maintenus. » Cet accord couvre P, S, T, E, C puis D et les campagnes prévues. Pour cette seule campagne `clippy-adapters-desktop`, les événements Created/Changed/Deleted purement sous `%LOCALAPPDATA%\Temp` non attribués et les renommages dont les deux chemins y restent sont consignés UNATTRIBUTED sans arrêt ; aucune attribution ni destination volontaire externe n'en est déduite. Les événements attribués à la campagne, sensibles, erreurs de surveillance, renommages franchissant la frontière et les autres arrêts du plan restent bloquants. Une revue et un commit de chaque lot fermé précèdent le suivant. L'autorisation ne vaut pas extension aux warnings/cibles hors périmètre ni aux recettes natives/UI.

### Sélection technique G-TEST au préflight P

V-I sous garde : adapters **308 tests inventoriés** (run `d1d2839c`), desktop lib **44** (`f0e3e3b2`), pas de corps exécuté ; V-BASE `92638e65` exit 0. L'audit a trouvé des tests natifs non ignorés et des fixtures qui exigent une autre campagne ou retournent prématurément si leurs variables sont absentes. Pour respecter l'exclusion explicite des recettes natives et le gate G-TEST, l'auteur du plan retient une sélection technique contrôlée, identique avant/après chaque lot, sans modifier ni ignorer les tests dans les sources : V-UNIT avec les neuf filtres `--skip` ci-dessous. Les tests déjà ignored restent ignorés, jamais exécutés avec --ignored. Le test privé de verrou et les trois tests windows_file_identity restent inclus. Fixer `PARAKEET_L2_ARCHIVE_ROOT` sous `target/parakeet-tests/clippy-adapters-desktop/temp/`, après vérification canonique de la garde, pour les tests source_binding ; TEMP/TMP restent sous la campagne. Aucune autre ancienne racine n'est autorisée par cette sélection.

- `direct_live::d2_corpus::d2_corpus_real_cpu_turbo_and_parakeet` : moteurs réels.
- `direct_live::d2_corpus::tests::test_status_writer_preserves_diagnostics` : racine D2 d'une autre recette.
- `gpu_inventory::windows_native::tests::live_dxgi_enumeration_has_ordered_ranks_and_setupapi_identity_join` : inventaire matériel réel.
- `worker_ipc::tests::live_automatic_final_gate_does_not_block_next_session_stop_or_late_publish` : modèle réel.
- `worker_ipc::tests::r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication` : modèle réel.
- `parakeet_import::tests::paths_reject_a_worker_outside_the_runtime_root` : fixture de campagne absente, retour anticipé.
- `worker_ipc::tests::over_limit_parakeet_import_is_refused_before_pending_creation` : fixture de campagne absente, retour anticipé.
- `worker_ipc::tests::injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume` : fixture de campagne absente, retour anticipé.
- `worker_ipc::tests::resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` : fixture de campagne absente, retour anticipé.

Cette première proposition d'exclusions a été **supersédée avant tout lancement de V-UNIT** : l'audit complémentaire a trouvé d'autres dépendances modèle/MP3 et racines RTC-02 dans des tests de worker IPC non concernés par P. Aucune baseline partielle ainsi filtrée n'est revendiquée. Pour éviter une liste d'exclusions fragile, la sélection retenue est désormais positive et propre à chaque lot.

Pour **P seulement**, trois exécutions V-UNIT avec les filtres exacts `archive::tests::`, `source_binding::recovery_tests::` et `windows_file_identity::tests::`, identiques baseline/candidat, plus V-SOURCE et le nouveau V-LOCK. Ces modules contrôlés couvrent les invariants publication/reprise/verrou/identité sans moteur ni modèle ; les racines requises sont vérifiées avant lancement et `PARAKEET_L2_ARCHIVE_ROOT` reste sous le temp de campagne. Les autres tests lib sont NOT RUN pour P ; aucun résultat global de bibliothèque n'est inféré. Avant les lots suivants, sélectionner de même les modules/scénarios pertinents après audit des fixtures, consigner la sélection et la conserver avant/après. Les tests de recette exclus ne sont jamais des PASS. Un échec des tests retenus impose l'arrêt ; toute preuve de conservation essentielle inaccessible bloque la tâche concernée.

À utiliser uniquement après accord précis sur le lot et la campagne :

> Mode IMPLEMENT, rust_implementer. Lire docs/cadrage/plan-correction-clippy-adapters-desktop.md et les AGENTS applicables. Commencer par préflight du lot expressément autorisé, baseline admissible puis ses seules tâches, P1–P4 pour le premier lot recommandé. Les 98 IDs et les six lots bornent l'objectif, pas un nettoyage général. Respecter fichiers/budgets, gardiens File/identités/format/ordre des effets, canaux bornés et configuration des workers. Préserver les changements tiers et docs/architecture, models, sound example. Les variables Cargo/Temp et la règle Temp doivent être explicitement autorisées pour la nouvelle campagne ; aucun Cargo direct ni --fix. Exécuter les vérifications §7 réellement utiles et fermer tous critères du lot ; arrêter sur les conditions du plan. Ne pas modifier le document de cadrage : rendre à main les contributions de bilan, diff, résultats/logs et limites. Ne pas passer au lot suivant avant revue indépendante via rust_review_coordinator et commit local du lot fermé. Deux cycles maximum par livrable ; aucune correction externe au plan, aucun push/native/UI sans accord précis.
