Avancée : cadrage complet relu indépendamment, CLEAN — DESIGN READY pour la planification.
Blocage : aucun pour le cadrage ; exécution Cargo et tests non autorisée.
Décision : préparer le refactor à logique constante, sans migration des contrats historiques.

# Cadrage — séparation des tests et diagnostics

ID : D-REF-TESTS-DIAG. Version : 1.0, 29 octobre 2026. Statut : DESIGN READY pour la planification.
Revue : `revue-refacto-tests-diagnostics.md`, verdict CLEAN sur la conception du commit `197d821ef98ea8c239d62f11cd40bfe9fdb1c74e` ; ajout de ce statut = EVIDENCE_ONLY, contenu technique inchangé.
Base produit : `f82063ae63b2cc075eaff54693eb62ea36a18c65`, branche `evo-startup-perf`.
Auteur documentaire : session Codex principale. Contributions proposées attribuées ci-dessous.

## 1. Sources, autorité et périmètre

S1 — demande originale de l'utilisateur dans cette session :

> ## déplacements mécaniques (risque quasi nul, gain immédiat)
>
> **1a. Sortir les tests de `worker_ipc.rs`** vers des fichiers séparés (`worker_ipc/tests/*.rs`).
>
> - Le fichier passe d'environ 29 000 à environ 12 500 lignes **sans toucher une ligne de logique**.
> - Toutes les étapes suivantes deviennent relisibles.
> - Faites de même pour `application.rs` (3 035 lignes de tests), `ui.rs` (2 410) et `archive.rs` (2 169).
>
> **1b. Renommer les identifiants de lots** (`record_r7_timing`, `l2a2_*`, `l04_*`...). C'est mécanique, et ça casse au passage le lien de dépendance entre `archive.rs` et `worker_ipc.rs` en déplaçant `record_r7_timing` dans un module `diagnostics`.

S2 — mission actuelle de l'utilisateur : « fais le cadrage puis le plan de refacto avec les agents dediés ».
S3 — `AGENTS.md` racine et des crates concernées, sources et manifestes actuels : contraintes applicables.
S4 — qualification dans cette session : revue indépendante PASS, pas autorisation d'implémenter.
S5 — `docs/architecture/` : relevés statiques non suivis, à confronter aux sources ; non intégrés à ce commit.

S2 autorise cadrage et planification. L'auteur principal écrit et commite localement les documents selon S3.
Aucun code, Cargo (même metadata), build, test, installation, push, élévation ou campagne n'est autorisé par cette mission.
Les choix techniques conformes sont délégués à l'architecte ; aucune acceptation de migration des contrats historiques n'est déduite de S2.
Les dossiers non suivis `docs/architecture/`, `models/` et `sound example/` sont préservés.

## 2. Brief produit

Contribution : `rust_product_framer`, `/root/cadrage_refacto/brief`, BRIEF-REF-TESTS-DIAG-v1 ; base S1–S4 et commit produit ci-dessus.

Objectif : rendre quatre unités Rust plus faciles à lire et faire évoluer par séparation des tests, clarification des noms internes et isolement du diagnostic commun exposé par worker_ipc.
Bénéficiaires : mainteneurs/relecteurs et utilisateurs Windows protégés contre régression.
Version : tranche « tests et diagnostics », aucune release déduite. Résultat utilisateur Windows : transcription, affichage, sauvegarde, erreurs et diagnostics identiques.

| Capacité | Résultat / sort | Source |
| --- | --- | --- |
| CAP-01 | Tests worker_ipc vers worker_ipc/tests/*.rs, chaque test/auxiliaire/cfg conservé | S1 |
| CAP-02 | Séparation tests application.rs, destinations locales à concevoir | S1 |
| CAP-03 | Séparation tests ui.rs, mêmes scénarios/observations | S1 |
| CAP-04 | Séparation tests archive.rs, sélections/dispositifs préservés | S1 |
| CAP-05 | Noms internes explicites ; liste finie, pas renommage global par motif | S1, S3 |
| CAP-06 | Diagnostic commun indépendant, suppression archive vers worker_ipc ; contexte/garde/effets préservés | S1 |
| CAP-07 | Démontrer conservation/non-régression lors réalisation future ; pas exécuté au cadrage | S1, S3 |
| CAP-08 | Migration env/protocoles/formats/dossiers/interfaces campagnes exclue faute accord ; reprise par mission distincte | S3 |
| CAP-09 | Refonte autres responsabilités production/changements comportement hors mission | S1, S2 |
| CAP-10 | Aucun nouveau chantier installation/démarrage OS/réglages/tray/archives ; contrats actuels et différés inchangés | S1–S3 |

Parcours : lecture/localisation tests ; découverte/sélection sans disparition ni ignoré compté succès ; diagnostic depuis archive+IPC identique ; usage Windows inchangé sans nouvel écran/état/annulation/reprise.
« Risque quasi nul », « gain immédiat », ~12 500 lignes et volumes initiaux sont appréciations/estimations, aucun seuil accepté. Pas de nouvelle API publique, dépendance ou décision produit nécessaire.

## 3. Observations actuelles

| Source | Lignes | Début principal module tests |
| --- | ---: | ---: |
| crates/whisper-adapters/src/worker_ipc.rs | 29 509 | 13 513 |
| crates/whisper-core/src/application.rs | 7 550 | 4 516 |
| crates/whisper-desktop/src/ui.rs | 6 195 | 3 870 |
| crates/whisper-adapters/src/archive.rs | 10 092 | 7 968 |

Des auxiliaires cfg(test) dispersés nécessitent un inventaire distinct. Ces lignes ne sont pas une mesure des seules lignes de production.
`archive.rs` contient neuf appels à `crate::worker_ipc::record_r7_timing` ; worker_ipc dépend déjà d'archive.
Le diagnostic possède aussi R7TimingContext, un stockage local au thread et R7TimingContextGuard (`worker_ipc.rs:175–205`).
Aucun module diagnostics dans adapters/lib.rs ; ceux des workers CPU/GPU sont dans d'autres crates.
Les contrats historiques incluent `PARAKEET_R7_*`, `PARAKEET_L2_*`, `WHISPER_L04_*`, `.l2a2-acceptance`, `whisper-l04-fault/1` et les sélections `--exact`.
`archive.rs:9610–9670` comporte notamment des sous-processus et écritures vers `C:\WhisperLive\L04-campaign\runs` : conserver ces tests n'autorise pas leur exécution.
Les gardes repérées sous `target/parakeet-tests/micro-level/` et `retraitement-cible/rtc-05/` sont historiques, pas des lanceurs génériques de ce refactor.

## 4. Exigences, scénarios et acceptation

Contribution : `rust_requirements_analyst`, `/root/cadrage_refacto/requirements`, REQ-REF-TESTS-DIAG-v1 ; base S1–S4 et commit produit ci-dessus.
Chaque ligne établit CAP-n ↔ REQ-n ↔ UC-n ↔ AC-n ; REQ-07 est transverse. Tous les tests produit : NOT RUN.

| IDs | Exigence et scénario d'acceptation observable | Contrôle futur |
| --- | --- | --- |
| CAP-01 / REQ-01 / UC-01 / AC-01 | Inventaire IPC puis extraction : chaque test/auxiliaire exactement une fois après correspondance de noms, aucune assertion/cfg perdu ; fragments attachés aux types restants recensés/justifiés | Diff et inventaires puis harness si autorisé |
| CAP-02 / REQ-02 / UC-02 / AC-02 | Application : mêmes entrées, commandes, événements, assertions/cfg et transitions après déplacement | Diff, découverte/exécution futures |
| CAP-03 / REQ-03 / UC-03 / AC-03 | UI : mêmes scénarios/observations, accès privés conservés ; aucune API publique ni branche produit pour tests | Diff, consommateurs, tests futurs ; pas preuve rendu Windows |
| CAP-04 / REQ-04 / UC-04 / AC-04 | Archive : scénarios durables/attributs/sélections conservés ; chaque --exact sélectionne le test attendu, sans zéro-test silencieux ni parent réinvoqué | Revue et exécution autorisée uniquement |
| CAP-05 / REQ-05 / UC-05 / AC-05 | Liste finie ancien/nouveau : tous consommateurs traités ; env/formats/dossiers/protocoles/messages/interfaces historiques identiques, reliquats motivés | Recherche et diff, glob insuffisant |
| CAP-06 / REQ-06 / UC-06 / AC-06 | Timing activé admissible, désactivé, config absente/invalide, contextes emboîtés et fin portée : émission/refus/champs/messages/restauration identiques ; diagnostics indépendant d'IPC, aucune destination nouvelle | Comparaison fonction/TLS/garde/appels, vérification ciblée future |
| CAP-07 / REQ-07 / UC-07 / AC-07 | Comparaison bases/configs/listes : cardinalités identiques après table noms ; aucun perdu/dupliqué/nouvel ignored/logique changée ; manifests/lock/contrats/fonctions préservés | Sources, listes et résultats réels avec statut par scénario |

Raccordements autorisables au futur plan : modules, imports, chemins et visibilité minimale nécessaires ; pas de nouvelle API publique pour tests.
Mesures : cardinalité identique par configuration ; zéro reliquat test inexpliqué ; zéro appel timing archive via IPC et zéro dépendance diagnostics vers IPC ; zéro évolution hors raccordements/liste noms.
Une comparaison statique n'est ni une découverte conditionnelle du harness ni une exécution.

### Cas dégradés et états

Aucun état produit nouveau. Test introuvable/dupliqué/inaccessible ou --exact non résolu : arrêt de tranche.
Élargissement d'API publique nécessaire : arbitrage. Échec de vérification préalable/après : aucune suite dépendante.
Diagnostic désactivé/config invalide : conserver non-écriture/messages ; fin de portée : conserver restauration TLS sans nouvelle propagation interthread.
Interruption édition/validation : conserver diff et résultat partiel, pas reset. Tests hors racines autorisées : NOT RUN jusqu'au mandat, sans changement de logique pour contourner.

### Risques fonctionnels

| ID | Risque et contrôle |
| --- | --- |
| RISK-F01 | Perte/sélection vide test → AC-01 à AC-04, AC-07 |
| RISK-F02 | Élargissement privé altérant API/données/synchronisation → AC-01 à AC-03, AC-07 |
| RISK-F03 | Perte contexte/refus/champs timing → AC-06 |
| RISK-F04 | Renommage modifiant contrat de campagne → AC-05 |
| RISK-F05 | Test écrivant hors racines, notamment C:\WhisperLive\L04-campaign\runs → AC-04/07, prérequis EXECUTION |
| RISK-F06 | Lignes retirées présentées comme performance/non-régression prouvée → séparation des preuves |

## 5. Domaine et architecture

### 5.1 Domaine — DOMAIN-REF-TESTS-DIAG-v1

Owner : `rust_domain_architect`, `/root/cadrage_refacto/domain`, contribution proposée sur base documentaire `5a767076254c5ec09fa0b70e2e0ce1f8b6f393ac` et sources produit initiales. Sources directes : AGENTS racine/adapters/core/desktop, worker_ipc.rs:175–205,260–316 ; couverture REQ/UC/AC-01 à07.

| Concept / propriétaire | Responsabilité préservée | Liens |
| --- | --- | --- |
| Application / cœur | Commandes, événements, états et effets ; aucune transition ou règle changée | REQ-02, AC-02/07 |
| Présentation / UI | Interactions et représentation de l'état ; aucun accès public pour fixtures | REQ-03, AC-03/07 |
| Exécution / IPC | Effets, filtrage des identités ; mêmes bornes, ordres et confirmations | REQ-01, AC-01/07 |
| Archive durable | Persistance, intégrité, publication et reprise existantes ; aucun nouveau format/chemin utilisateur | REQ-04, AC-04/07 |
| Contexte de diagnostic | Attribution descriptive d'une mesure à opération/identifiant/destination ; pas de politique métier | REQ-06, AC-06 |
| Scénario / support de test | Mêmes stimuli, observations, données, oracle et préconditions | REQ-01 à05/07 |

États métier/durables : inchangés, pas de nouvelle machine d'états. État visible : même représentation. Contexte diagnostic : temporaire et propre au thread appelant.

| Événement / précondition | Transition ou résultat conservé | Effet / erreur |
| --- | --- | --- |
| Entrée de portée, précédent absent ou présent | Sauvegarder précédent et installer courant | Aucune trace ni transition métier à cette seule entrée |
| Mesure, activation absente ou différente de 1 | Contexte inchangé, aucune trace | Aucun nouveau refus/repli |
| Mesure activée, config/chemin rejeté | Mêmes refus et messages | Aucun nouvel emplacement/récupération |
| Mesure activée, chemin admissible | Lire contexte sans le consommer, product ou unscoped conservé | Même tentative d'append, champs et traitement des échecs |
| Destruction de la garde | Restaurer précédent, y compris erreurs exécutant le Drop | Aucune propagation interthread ni garantie après arrêt brutal |

- INV-01 : scénarios/auxiliaires/assertions/cfg/synchronisation présents une fois ; seuls raccordements et renommages listés changent. AC-01 à05/07.
- INV-02 : mêmes stimuli, décisions, événements, états visibles et effets ; aucune nouvelle API/politique produit. AC-01 à04/07.
- INV-03 : filtres d'identités, ACK, PublicationGate, intégrité et publication gardent emplacement logique et ordre ; aucune atomicité/déduplication nouvelle. AC-01/02/04/07.
- INV-04 : activation, refus, contexte, restauration, trace et influence existante sur exécution inchangés ; aucune idempotence/reprise ajoutée. AC-06/07.
- INV-05 : noms externes, formats, chemins, données et sélections historiques compatibles ; aucune migration déduite du renommage. AC-04/05/07.
- INV-06 : interruption/non-exécution/effet hors racine donnent résultat partiel ou NOT RUN, pas conservation démontrée ; aucun reset/nettoyage nouveau. AC-07.

### 5.2 Architecture — ARCH-REF-TESTS-DIAG-v1

Owner : `rust_architect`, `/root/cadrage_refacto/architecture`, choix techniques proposés dans la mission S2, base documentaire `5a767076254c5ec09fa0b70e2e0ce1f8b6f393ac`. Sources : AGENTS racine et trois crates, manifestes workspace/crates, quatre sources cibles et consommateurs model_catalog.rs/parakeet_import.rs/staging.rs/supervisor.rs ; fiches statiques confrontées aux sources le 29 octobre 2026.

| Propriétaire | Actuel | Cible / dépendance |
| --- | --- | --- |
| whisper-core | Domaine/application, dépendance externe serde | Identique, tests extraits et deux noms locaux changés |
| whisper-adapters | Dépend de core ; IPC utilise archive, archive utilise timing IPC | Cargo identique ; archive et IPC utilisent diagnostics qui ne dépend d'aucun des deux |
| whisper-desktop | Dépend de core et adapters, compose et affiche | Identique ; déplacement tests seulement |
| Tests unitaires | Supports/modules dans les quatre sources | Même crate et portée logique, fichiers séparés, aucun accès public ajouté |
| Workers/bootstrap | Routages/crates existants | Hors tranche, homonymes non renommés |

Aucune nouvelle couche/crate. Les dépendances desktop → adapters + core restent celles d'AGENTS : pas de refonte des écarts historiques. Les accès système du cœur déjà présents restent présents, seuls noms internes clarifiés.

**DEC-T01 — Layout à portée constante.** `foo.rs` + `foo/tests.rs` + `foo/tests/*.rs`. Racines : `#[cfg(test)] mod tests;`. Agrégateur : fragments via `include!("tests/nom.rs")`, sans nouveau sous-module logique. Imports super, accès privés, noms harness et --exact restent identiques. Alternative sous-modules thématiques écartée car elle change les identités et les raccordements privés. Aucun #[path]. Items complets, attributs/commentaires et ordre conservés ; agrégateur sans logique. Racines production intentionnellement volumineuses : façade fine hors mission. Tests unitaires dans src selon S1/S3, sans API publique pour les migrer en intégration.

### 5.3 Destinations finies

Tous chemins de fragments ci-dessous sont relatifs au dossier src de la crate de leur racine. Frontières par items complets, jamais par découpage aveugle de lignes.

| Racine | Agrégateur | Fragments dans son sous-dossier tests/, en ordre |
| --- | --- | --- |
| adapters/worker_ipc.rs | worker_ipc/tests.rs | unified_support.rs ; unified_import.rs ; unified_capture.rs ; native_execution.rs ; microphone_and_finalization.rs ; reprocessing_and_transport.rs ; resume_and_admission.rs |
| core/application.rs | application/tests.rs | closing_and_recovery.rs ; selection_and_microphone.rs ; live_lifecycle.rs ; parakeet_sessions.rs |
| desktop/ui.rs | ui/tests.rs | support_and_import.rs ; microphone_and_unified.rs ; compute_and_catalog.rs ; document_presentation.rs |
| adapters/archive.rs | archive/tests.rs | archive_and_contracts.rs ; recovery.rs ; campaign.rs ; replacement_and_segmented.rs |

Ancres de début, attributs précédents inclus ; chaque fragment s'arrête juste avant l'item-ancre suivant ou à la fin du module :

- IPC : imports module principal ; unified_capture_runtime_rejects_third_before_capture_factory ; import_application_finalizes_stopped_capture_a_while_capture_b_pumps_and_publishes_segmented_audio ; native_window_pcm_requires_a_bounded_matching_riff_range ; controlled_worker_and_invalid_times_leave_no_complete_archive ; reprocessing_candidate ; compile_r7_resume_worker.
- Application : imports ; unified_page_responses_and_errors_follow_latest_selection_request ; repeated_start_is_identifiable_and_stale_stop_does_not_target_new_generation ; parakeet_provisional_never_becomes_confirmed_without_a_confirmed_event.
- UI : imports ; auto_stop_is_disabled_by_default_and_uses_speech_sample_clock_when_enabled ; enqueue_then_stop_still_targets_the_live_identity ; provisional_display_removes_only_a_proven_confirmed_prefix.
- Archive : imports ; r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision ; l04_campaign_gate_rejects_escape_and_duplicate_trigger ; r11_replace_deactivates_every_matching_publication_and_keeps_other_audio.

Supports supplémentaires inclus sous cfg(test) à leur portée racine d'origine :

- worker_ipc/tests/stopped_ack.rs : module complet parakeet_stopped_ack_tests, nom logique inchangé.
- worker_ipc/tests/runtime_support.rs : items racine entièrement cfg(test) NativeQualificationEventKind, NativeQualificationWork, NativeQualificationEvent, NativeQualificationObserver et son impl, ControlledChildObservation, observe_controlled_child, ScheduledWindowTranscriber et son impl de trait.
- archive/tests/publication_support.rs : deux constantes APPROVED_MODEL_*, PublicationCopyFault, PUBLICATION_COPY_FAULT et quatre fonctions de faute de copie, visibilités existantes.
- ui/tests/root_support.rs : speech_limit_samples.

Reliquats intentionnels : champs, initialisations, branches et méthodes test enchâssés dans structs/impls mixtes, notamment injections/observations IPC, constructions contrôlées, positions widgets, install_test_model_catalog, closures conditionnelles archive. Les extraire restructurerait types/impls. Le plan doit lister leurs emplacements ; tout autre reliquat inexpliqué interdit clôture.

Contrôle macros/chemins : zéro include!/include_str!/include_bytes!/file!/line!/module_path! dans les quatre sources ; zéro assert_snapshot!/insta::/expect_test/snapbox/current_dir. IPC possède huit CARGO_MANIFEST_DIR, autres zéro : préserver leurs constructions ancrées au manifeste. panic! : IPC84/application6/UI1/archive0, messages conservés ; site fichier/ligne normalement déplacé. Snapshots d'états/archives applicatifs inchangés. Pas d'obstacle statique observé à include! à portée constante.

### 5.4 Diagnostic commun : extraction atomique

**DEC-T02.** Nouveau `crates/whisper-adapters/src/diagnostics.rs`, déclaré par `mod diagnostics;` dans lib.rs. Extraire ensemble contexte, TLS, garde enter/Drop, fonction de mesure et son PROCESS_START. Aucun autre code production déplacé.
record_stage_timing, TimingContextGuard et enter : pub(crate). Contexte/champs/TLS privés. Aucun réexport public ou relais IPC.
Consommateurs : worker_ipc.rs, neuf appels archive.rs, model_catalog.rs:172, cinq appels parakeet_import.rs:489,620,703,715,812.
diagnostics utilise std et serde_json déjà disponibles ; aucune dépendance archive/IPC/support test.
Entrées : opération/identifiant/destination pour portée ; stage/durée/détails pour mesure. Sorties/erreurs/effets : tableau domaine §5.1.
PARAKEET_R7_TIMING_ACTIVE, PARAKEET_R7_TIMING_LOG, PARAKEET_R7_EXEC_ROOT, JSON, stages, messages et règles chemins restent littéralement identiques. Append JSON + nouvelle ligne, product/unscoped et traitement échec conservés. Aucun thread/verrou/async/timeout/rattrapage/durabilité/atomicité ajouté. Un seul TLS et PROCESS_START ; restauration emboîtée inchangée, aucune propagation interthread.

### 5.5 Nomenclature finie

**DEC-T03.** Symboles seulement ; aucune substitution globale par préfixe, aucun remplacement de littéraux. Les variantes cfg(feature)/cfg(not(feature)) du même symbole sont toutes concernées.

| Source | Ancien | Nouveau |
| --- | --- | --- |
| IPC → diagnostics | record_r7_timing | record_stage_timing |
| IPC → diagnostics | R7TimingContext | TimingContext |
| IPC → diagnostics | R7_TIMING_CONTEXT | TIMING_CONTEXT |
| IPC → diagnostics | R7TimingContextGuard | TimingContextGuard |
| IPC | l2a2_acceptance_barrier | acceptance_barrier |
| IPC | l2a2_acceptance_take_worker_fault | take_acceptance_worker_fault |
| IPC | canonical_l3_acceptance_root | canonical_extended_acceptance_root |
| IPC | l3_gpu_candidate_allowed | gpu_qualification_candidate_allowed |
| IPC, faute | l3c_execution_root | extended_execution_root |
| IPC, faute | in_l2a2 | in_primary_runtime |
| IPC, faute | in_l3_test_root | in_extended_runtime |
| IPC, canonisation | l3c | primary_root |
| IPC, canonisation | l3d | secondary_root |
| Archive | L04_CAMPAIGN_OCCURRENCE | CAMPAIGN_HOOK_OCCURRENCE |
| Archive | l04_campaign_after | campaign_after_hook |
| Archive | l04_campaign_is_after | campaign_is_after_hook |
| Archive | l04_campaign_assert_archive | assert_campaign_archive |
| Archive | l04_campaign_write_marker | write_campaign_marker |
| Archive | l04_campaign_snapshot | campaign_archive_snapshot |
| Archive | l04_campaign_operation_result | record_campaign_operation_result |
| Archive | l04_campaign_gate | campaign_gate |
| Application | l3d_timing_line | record_application_timing |
| Application | l3d_event_trace | trace_application_event |
| Support tests IPC | compile_r7_resume_worker | compile_copy_resume_worker |
| Support tests IPC | write_r7_resume_wav | write_copy_resume_wav |
| Support tests IPC | r7_resume_request | copy_resume_request |
| Support tests IPC | r7_hex | copy_resume_hex |
| Support tests IPC | r7_generation_path | copy_resume_generation_path |
| Support tests IPC | wait_r7_prompt | wait_copy_resume_prompt |
| Support tests IPC | run_r7_integrated_copy_resume_case | run_integrated_copy_resume_case |

Raccordements L04 hors quatre racines limités à staging.rs et supervisor.rs (dans adapters/src), plus IPC ; aucun déplacement de logique. Homonymes workers CPU/GPU et timing worker Parakeet hors tranche.
Conservés : noms des tests recette_policy_l2a2_requires_the_feature, recette_policy_l04_requires_the_feature, l04_campaign_gate_rejects_escape_and_duplicate_trigger, r7_*, r11_*, l3c_* ; harness/--exact, faux exécutables/whisper_r7_resume_worker, variables, formats, dossiers, messages et labels historiques. Le préfixe seul ne justifie rien.

### 5.6 Faisabilité et risques techniques

Déclarations lues : édition2024, rust-version1.98.1, serde_json=1.0.145 et eframe=0.35.0 inchangés ; features recette-hooks/qualification/personal-install inchangées. Ce ne sont pas des versions d'outils ni configs compilées constatées. Aucune dépendance native, FFI, politique unsafe ou redistribution nouvelle.

| Preuve / risque | Classe et état proposé | Contrôle / suite |
| --- | --- | --- |
| Items à portée constante, pas nouveau consommateur public | DESIGN_FEASIBILITY — PASS statique proposé | Revue items/layout |
| Diagnostic autonome, consommateurs recensés | DESIGN_FEASIBILITY — PASS statique proposé | Revue TLS/garde/fonction/raccordements |
| RISK-T01 : frontière fragment/attribut/support perdu | PRODUCT_VALIDATION — NOT RUN | Corps/attributs/inclusion unique puis harness autorisé |
| RISK-T02 : support compilé en produit/visibilité élargie | PRODUCT_VALIDATION — NOT RUN | Revue cfg/visibilité puis builds variantes utiles autorisés |
| RISK-T03 : TLS/horloge dupliqué | PRODUCT_VALIDATION — NOT RUN | Un seul TLS/PROCESS_START, mêmes appels/portées |
| RISK-T04 : cfg/consommateur renommé partiellement | PRODUCT_VALIDATION — NOT RUN | Table et appels, recherche symboles/littéraux |
| RISK-T05 : rustc/processus/écritures historiques par tests | PRODUCT_VALIDATION — NOT RUN | Cartographie effets avant commande, sous-suites admissibles |
| EXE/fenêtre | DELIVERY_QUALIFICATION — NOT RUN, non requise ici | Aucun binaire ni recette remis |

Pas de nouvelle technologie ni faisabilité demandant SPIKE : déplacements à portée constante et raccordements recensés ; exécution à prouver après code.
Mesures lexicales observées, pas harness : #[test] IPC129/archive22/application44/UI40, total235 ; #[cfg(test)] respectivement131/12/1/22. Cible mêmes235 tests source, zéro #[test] dans quatre racines, zéro inclusion double, zéro timing via IPC, aucun item racine test oublié ; cfg résiduels conservés et justifiés.
Contrôles proposés : git diff --check ; comparaison Git corps/attributs/littéraux/table ; rg -n 'crate::worker_ipc::record_r7_timing' crates/whisper-adapters/src attendu absence ; rg -n 'worker_ipc' crates/whisper-adapters/src/diagnostics.rs attendu absence ; comptage lexical récursif fragments/inclusions. Absence rg = code1 attendu.
Gate exécution : aucun lanceur admissible de cette tranche établi. Gardes historiques non transposables par simple choix technique. Avant tout Cargo/processus test : accord précis, racines/garde/auto-tests conformes, ≥30Gio, variables/surveillance AGENTS. Tests vers C:\WhisperLive\L04-campaign\runs, tests compilant faux workers et natifs : NOT RUN sans prérequis compatibles. Ne bloque pas comparaison statique/plan ; aucune garde/dérogation créée.

## 6. Traçabilité, applicabilité et readiness

Aucune nouvelle migration/persistance ; effets existants à préserver. Concurrence inchangée, dont TLS/gardes/bornes.
Windows, pile native et routage inchangés ; aucun nouveau réseau, donnée ou destination. Aucun EXE livré ni UI validée.
Pas de SPIKE demandé. Pas de nouveau railguard ; AGENTS existants demeurent applicables.

| REQ/UC/AC | Invariants | Architecture | Risques / acceptation |
| --- | --- | --- | --- |
| 01 | INV-01/02/03 | IPC/support même portée | F01/02, T01/02/05 ; conservation/harness |
| 02 | INV-01/02 | Application | F01/02, T01/02 ; transitions/tests |
| 03 | INV-01/02 | UI | F01/02, T01/02 ; privé/observations |
| 04 | INV-01/03/05 | Archive/--exact | F01/05, T01/05 ; zéro sélection vide |
| 05 | INV-01/05 | Nomenclature/consommateurs | F04,T04 ; littéraux identiques |
| 06 | INV-04 | Diagnostics/TLS/effets | F03,T03 ; diagnostics identiques |
| 07 | INV-01 à06 | Inventaires/diff/prérequis | F01 à06,T01 à05 ; preuves distinctes |

Scope, use_cases, errors, domain, dependencies, platform, acceptance, risks couverts et soumis à revue. Persistence/concurrency : conservation des effets/gardes/ordres existants. Nouveaux états, migration, packaging et nouveau railguard : non applicables sans demande correspondante.
Arrêt futur : test perdu/dupliqué/filtre non résolu ; logique/cfg/littéral imprévu ; API publique élargie pour fixture ; consommateur nouveau étendant périmètre ; validation préalable manquante/échouée ; conflit concurrent. Conserver diff et arrêter tranche dépendante.
Terminé : quatre contributions, traçabilité, layout/nomenclature, revue indépendante CLEAN. Restant : plan puis revue PLANS. Ce statut ne vaut pas autorisation d'implémenter.

## 7. Validation et remise

Lectures et recherches uniquement ; git diff --check exécuté. Cargo, build, tests, natif, fenêtre Windows : NOT RUN.
Le futur plan doit couvrir AC-01 à AC-07 en dix tâches maximum et justifier chaque chemin.
Les autorisations d'implémentation, de campagne et de publication restent distinctes de la readiness documentaire.
