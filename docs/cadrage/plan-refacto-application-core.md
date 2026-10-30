Avancée : exécution T1–T10 autorisée ; garde T1 adaptée et cache copié, aucun code produit modifié.
Blocage : auto-tests E0 arrêtés sur événements hors racines ; baseline et T1 non commencées.
Décision : conserver les preuves et attendre l’arbitrage utilisateur avant toute reprise.

# Plan — refactorisation de l’application du cœur

- ID : P-REF-APP-CORE, version 1.0, 30 octobre 2026.
- Statut : PLANS READY — revue indépendante CLEAN ; acceptation finale utilisateur et autorisation d’exécuter distinctes.
- Auteur du document : session Codex principale ; contribution de planification : `rust_plan_writer`, PLAN-APP-CORE v1.
- Design parent : [refacto-application-core.md](refacto-application-core.md), D-REF-APP-CORE, Git `682e09e7d8260f9e454441f50b11dc9d848895bb`.
- Preuve de passage : DESIGN CLEAN indépendant sur `b258b1c44efc5e36467e287c1a3791215749c20a`, zéro REQUIRED/ADVISORY ; statut et preuve ajoutés sans modification technique dans `682e09e7`.
- Base du code : `30632da12a1eb0cf7b50465ee8ec30cd8174fa3f`, branche `evo-startup-perf`.
- Autorité documentaire : demande « ok fais le cadrage puis redige le plan de refacto avec les agents dediés ». Le code, les campagnes et le push restent soumis à leurs accords précis.
- Portée : dix tâches T1–T10 ; deux documents de cadrage seulement, aucun rapport ou handoff autonome supplémentaire.

## 1. Résultat et règles communes

Le résultat attendu est une façade `application.rs` déclarative, des dispatchers déléguant aux responsabilités du design et un unique propriétaire `ImportApplication<P>`. Les commandes, événements, erreurs, vues, messages, effets ordonnés et contrats publics restent ceux de la base.

Aucune dépendance, feature, modification de manifeste, lockfile ou protocole. Aucun nouveau thread, verrou, accès système, FFI, état parallèle, trait ou objet de contexte artificiel. Les exceptions historiques de diagnostic/temps sont déplacées sans migration fonctionnelle. Ancien `StartLive` toujours refusé.

Disposition et dépendances : DEC-01–06 et la carte du design sont la référence. Contrats → domaine/ports ; runtime → contrats/domaine/politiques/`ImportIoPort` ; handlers → état unique et helpers internes. Aucun lien vers adaptateur concret/UI/support de tests. Résolution Rust normale, sans `#[path]`.

Migration : état, constructeurs et `fail` restent dans `application.rs` jusqu’à T10. `runtime/mod.rs` déclare initialement les modules privés ; les implémentations descendantes utilisent l’état de leur ancêtre. Les méthodes partagées ont au plus `pub(in crate::application)` jusqu’à T10, puis une portée limitée à `runtime`. Aucun champ rendu public.

Retours : un helper transverse distingue consommation et poursuite ; aucune continuation après un retour qui quittait le dispatcher. Les deux réentrées `RequestClose → dispatch(CancelDuplicateImport)` et `RequestClose → dispatch(Stop)` restent réentrantes par l’API, avec ses gardes.

Owner d’exécution futur : `rust_implementer`, délégué par la session principale après GO précis ; revue par `rust_review_coordinator` et reviewer indépendant, au plus deux cycles. L’implémenteur ne s’attribue pas CLEAN.

## 2. Chemins et prérequis

Abréviations exactes utilisées dans les tâches :

| Alias | Chemin |
| --- | --- |
| A | `crates/whisper-core/src/application.rs` |
| B | `crates/whisper-core/src/application/` |
| R | `crates/whisper-core/src/application/runtime/` |
| U | `crates/whisper-core/src/application/tests/` |
| I | `crates/whisper-core/tests/` |
| D | `docs/cadrage/plan-refacto-application-core.md` |

Une concaténation comme `R/queue.rs` signifie le chemin relatif complet sous R, sans autoriser d’autre fichier. D est l’unique bilan d’exécution futur ; la session principale y consigne les résultats. Les budgets ci-dessous comptent les fichiers de production, séparément des tests et de D.

Lecture commune : AGENTS racine/core, design/plan Git, `lib.rs`, `ports.rs`, domaine/politiques et manifestes actuels ; consommateurs des chemins publics en lecture seule. Les cartes `docs/architecture/` sont confrontées aux sources, sans modification ni ajout à Git.

DOCUMENT D0 : design Git relu, présent plan relu/accepté dans sa portée, décisions applicables et accord précis de la session pour la tâche. Aucun consentement de code n’est déduit du statut documentaire.

CODE Cn : sortie de Tn conservée, attribuable et sans conflit avec les travaux d’autrui. Relever l’état Git avant chaque tâche ; préserver les non suivis `docs/architecture/`, `models/`, `sound example/`.

EXECUTION E0 — NON SATISFAIT : autorisation précise d’adaptation limitée de la garde existante et de campagne sur les racines du lot ; garde adaptée contrôlée, auto-tests et préflight conformes. La garde F3 actuelle ne satisfait pas E0.

EXECUTION En : contrôles requis de Tn réussis, non-régressions observées et revue indépendante de la tranche sans constat requis ouvert. Un test non exécuté ou filtre vide ne satisfait pas En.

Racines futures proposées, à autoriser avant création/utilisation : `target/parakeet-tests/refacto-application-core/t1/` à `t10/`, plus l’unique cible Cargo `target/parakeet-tests/cargo-shared/`. Aucun autre chemin de campagne n’est implicitement autorisé.

## 3. Index et ordre

| Tâche | Invariant fermé | Couverture dominante | Entrées |
| --- | --- | --- | --- |
| T1 | Contrats, métadonnées et diagnostics extraits avec API identique | REQ/AC-08/09 ; contrôles 01/07 | D0, E0 |
| T2 | File et transverses délégués à leur propriétaire | REQ/AC-02/06/07/09 | C1, E1 |
| T3 | Test micro, gain et mesures délégués sans déplacer les barrières | REQ/AC-05/09 | C2, E2 |
| T4 | Live/direct délégués sans changement des phases et arrêts | REQ/AC-03/04/09 | C3, E3 |
| T5 | Cycle import/persistance délégué avec publication conditionnée | REQ/AC-01/03/09 | C4, E4 |
| T6 | Unifié décomposé, identité/pagination/publication conservées | REQ/AC-03/04/05/09 | C5, E5 |
| T7 | Projections Parakeet décomposées et attribuées | REQ/AC-04/09 | C6, E6 |
| T8 | Final automatique décomposé, indépendant du travail actif | REQ/AC-04/09 | C7, E7 |
| T9 | Fermeture/poll et dispatchers à leurs propriétaires finaux | REQ/AC-01 à 07/09 | C8, E8 |
| T10 | État/tests transférés, visibilités resserrées, façade déclarative | REQ/AC-01 à 09 | C9, E9 |

Graphe unique T1 → T2 → … → T10. Aucune parallélisation : les tâches modifient le même état et les mêmes points de routage. Les budgets/campagnes de chaque tâche sont distincts, sans ajouter T0 ni étape documentaire supplémentaire.

## 4. Tâches

### T1 — Contrats, métadonnées, diagnostics et témoins

- Owner / entrées : implémenteur ; D0 et E0. Avant déplacement, découverte et baseline V2/V3 ; vérifier les témoins d’admission/fermeture S1/S7.
- Écriture : A, `B/contracts.rs`, `B/event_metadata.rs`, `B/diagnostics.rs` ; nouveau `I/application_routing.rs` et D. Budget : 4 production + 1 test, dépassement de trois justifié par le maintien atomique des déclarations et imports publics lors du premier découpage.
- Faire : déplacer les huit types de contrats publics, méthodes `AppCommand` et `AppFacade`, les deux impls de métadonnées `ImportEvent`, les deux fonctions de diagnostic/statistiques associées. Conserver `ImportApplication` dans A jusqu’à T10 ; les neuf chemins publics restent disponibles. Réexporter les chemins historiques depuis A ; conserver tous les autres corps dans A.
- Témoins : les suites existantes S1/S7/S8. Ajouter dans `application_routing.rs` un faux port local en mémoire et deux caractérisations proposées : `legacy_start_live_is_rejected_without_effects`, `history_event_is_applied_without_an_active_job`. Aucun support de production ni fichier de support partagé.
- Fermeture / métriques : avant : contrats, métadonnées et diagnostics définis dans A ; après : zéro définition de ces groupes dans A, neuf exports compatibles, corps/littéraux inchangés et fonctions/handlers de runtime encore dans A. M1/M2, revue des exports et V1–V6.
- Arrêt / retour : arrêt si une réexportation exige un changement consommateur ou si la baseline échoue ; annuler seulement le diff de T1 attribué et vérifié, après préservation du candidat, sans reset global.

### T2 — File, historique et traitements transverses

- Owner / entrées : implémenteur ; D0/E0/C1/E1. Baseline S2/S6/S7 avant extraction.
- Écriture : A, `R/mod.rs`, `R/queue.rs`, D. Budget : 3 production, aucun fichier de test supplémentaire.
- Faire : créer `mod runtime` privé et `runtime/mod.rs` déclaratif. Extraire commandes file/doublons/snapshot/historique/résolution de publication/reprise de nettoyage ; extraire les transverses `History`, `ReplacementCleanups`, snapshot, `Queue`, `QueueAdmission`, doublon et échecs de scan/retrait. Garder leur position avant filtre actif et distinguer consommé/poursuivre.
- Témoins : S2/S6, `history_event_is_applied_without_an_active_job`, fermeture avec retraits durables S7. Vérifier la branche doublon de final automatique sans déplacer sa politique dans queue.
- Fermeture / métriques : avant : ces corps sont dans les deux dispatchers ; après : zéro corps de ces familles dans A, uniquement appels et contrôles centraux ; définitions uniques dans `queue.rs`, état toujours A. M1/M3 et V1–V6.
- Arrêt / retour : arrêter sur consommation différente ou déplacement d’une garde ; reprendre uniquement T2 sur le dernier état C1 vérifié, en conservant le diff et les preuves.

### T3 — Test microphone, gain et acquisition

- Owner / entrées : implémenteur ; D0/E0/C2/E2 ; baseline S3.
- Écriture : A, `R/mod.rs`, `R/microphone.rs`, D. Budget : 3 production ; aucun nouveau test prévu.
- Faire : extraire commandes démarrage/arrêt test/gain et événements gain/test/mètre/spectre. Isoler l’invalidation d’acquisition appelée à sa position actuelle avant le filtre d’identité ; conserver identité propriétaire, révisions demandée/appliquée et gestion de l’ouverture asynchrone.
- Témoins : S3 ; contrôler anciennes réponses, arrêt puis reprise live et échec de test invalidant source/mesure.
- Fermeture / métriques : avant : corps micro mêlés aux dispatchers ; après : zéro corps micro dans A, barrières centrales inchangées et handlers uniques `microphone.rs` ; aucune nouvelle I/O. M1/M3, V1–V6.
- Arrêt / retour : arrêter si le transfert contourne une révision ou nécessite un champ public ; retour limité au diff T3, aucun changement de politique micro.

### T4 — Live et direct

- Owner / entrées : implémenteur ; D0/E0/C3/E3 ; baseline S4 et S1.
- Écriture : A, `R/mod.rs`, `R/live.rs`, D. Budget : 3 production ; tests existants seulement.
- Faire : extraire `StartLiveConfigured`, `StartDirectLiveConfigured`, `FinishLiveOnCpu`, phases/compteurs/direct/provisoires/reprises CPU/`WindowFinished` et projections Parakeet actives. Garder `Diagnostic` et les autres branches de ce groupe au même emplacement logique ; conserver leurs messages et mise à jour de vue.
- Témoins : S4 et diagnostics de `I/live_contract.rs` ; vérifier arrêt avant `Ready`, génération et persistance finales ; `legacy_start_live_is_rejected_without_effects`.
- Fermeture / métriques : avant : corps live/direct dans A ; après : zéro corps de ce groupe dans A, routage vers `live.rs`, refus ancien `StartLive` inchangé. M1/M3, V1–V6.
- Arrêt / retour : arrêter sur redémarrage tardif, changement d’admission ou final CPU automatique ; retour limité T4, sans unification de branches.

### T5 — Import, séquences et persistance

- Owner / entrées : implémenteur ; D0/E0/C4/E4 ; baseline S5.
- Écriture : A, `R/mod.rs`, `R/import.rs`, D. Budget : 3 production.
- Faire : extraire `StartImport`, `ProcessQueued`, `ResumeInterrupted`, cycle actif `ResumeCheckpoint`/`Prepared`/`Ready`/`Progress`/`Segment`/`Persisted`/`End`/`Published`/`Failed`/`Stopped`, `maybe_publish`, `accept_segment`, `retry_pending_segment`. Conserver les différences import/live, annulation, plages, contrepression et erreurs.
- Témoins : S5, notamment publication après fin/accusés, neuvième segment sous contrepression, déconnexion fatale, arrêt et segments tardifs.
- Fermeture / métriques : avant : cycle et trois helpers dans A ; après : zéro corps de ce cycle et zéro définition des trois helpers dans A, appels uniques vers import ; publication jamais avant conditions actuelles. M1/M3 et V1–V6.
- Arrêt / retour : arrêter si un accusé est traité comme une simple soumission ou si un résultat terminal poursuit le dispatcher ; conserver candidat et revenir seulement sur T5 après analyse.

### T6 — Captures et documents unifiés

- Owner / entrées : implémenteur ; D0/E0/C5/E5 ; baseline S6U.
- Écriture : A, `R/mod.rs`, `R/unified.rs`, D. Budget : 3 production ; pas de déplacement de tests.
- Faire : transférer `add_unified_session_card`, handler unifié, deux retries et publication ; commandes start/stop/sélection/lectures ; projections de sessions d’import unifié. Décomposer le gros handler en routage et handlers de branches dans ce fichier, sans recopier le monolithe tel quel.
- Témoins : S6U ; séquences `Prepared`/progress/end, dernières requêtes de page, backend effectif, erreur de total et révision de gain tardive.
- Fermeture / métriques : avant : helpers et branches unifiées dans A ; après : zéro définition de ces helpers et zéro corps unifié dans A, `handle_unified_capture_event` réellement délégant ; publication conditionnée par arrêt, séquences et accusés. M1/M3, V1–V6.
- Arrêt / retour : arrêter si le fichier dépasse 1 500 lignes de production ou si une décomposition exige un propriétaire supplémentaire ; retour limité T6, sans ajouter un module hors carte.

### T7 — Vues des captures Parakeet

- Owner / entrées : implémenteur ; D0/E0/C6/E6 ; baseline S7P.
- Écriture : A, `R/mod.rs`, `R/parakeet_views.rs`, D. Budget : 3 production.
- Faire : extraire et décomposer `track_parakeet_capture_event`, la closure de création de capture, `find_parakeet_capture_mut`, `trim_parakeet_capture_views`, `append_parakeet_confirmed`. Garder l’appel de suivi avant les transverses et le filtre actif ; ne pas rendre ce suivi consommateur exclusif des événements.
- Témoins : S7P, cache de huit cartes, confirmation tardive avec bonne identité/ordre et échec avant live-started.
- Fermeture / métriques : avant : cinq groupes dans A ; après : zéro définition de ces groupes dans A, routage de suivi décomposé, provisoires/confirmés distincts et cache borné inchangé. M1/M3, V1–V6.
- Arrêt / retour : arrêter sur suppression d’une seconde projection ou sur regroupement d’identités distinctes ; retour T7 uniquement.

### T8 — Final automatique indépendant

- Owner / entrées : implémenteur ; D0/E0/C7/E7 ; baseline S8A.
- Écriture : A, `R/mod.rs`, `R/automatic_final.rs`, D. Budget : 3 production.
- Faire : extraire les sept helpers de final automatique de la carte du design ; décomposer `handle_automatic_final_event` en branches. Garder le routage dédié avant le filtre du travail actif, limites de file/segments, statut non vérifié, erreurs et passage au suivant.
- Témoins : S8A, saturation/erreurs inspectées par comparaison des branches et constantes ; aucune preuve de moteur natif déduite de ces tests.
- Fermeture / métriques : avant : sept helpers dans A ; après : zéro définition des sept helpers dans A, état toujours dans A, final distinct d’un import actif et des confirmés. M1/M3, V1–V6.
- Arrêt / retour : arrêter si une finalisation peut modifier le travail actif indépendant ou si la gestion d’erreur change ; retour T8 seulement, sans correction fonctionnelle.

### T9 — Fermeture, poll et dispatchers finaux

- Owner / entrées : implémenteur ; D0/E0/C8/E8 ; baseline S1/S5/S7.
- Écriture : A, `R/mod.rs`, `R/closing.rs`, `R/poll.rs`, `R/commands.rs`, `R/events.rs`, D. Budget : 6 production, justifié par le transfert cohérent des deux points d’entrée et de leur cycle de fermeture/poll ; aucune autre famille modifiée.
- Faire : extraire readiness/retraits/`RequestClose`/`Stop`/`Quit` et `Committing`, puis `poll_once`. Transférer les deux dispatchers maintenant réduits, l’unique impl `Application` et `refuse_model_command`. Conserver les chaînes centrales du design et les deux réentrées `RequestClose`.
- Témoins : S1/S5/S7, notamment refus indirects, publication déjà engagée, retraits durables, quatre événements, `Ready` séparé de l’échec suivant et contrepression.
- Fermeture / métriques : avant : wrappers centraux encore A ; après : zéro définition de `handle_event`/`dispatch`/`poll_once`/readiness/retraits/`refuse_model_command` dans A, un impl `Application`, 30 dispositions conservées ; aucun corps métier réintroduit dans `commands`/`events`. M1–M4, V1–V6.
- Arrêt / retour : arrêter si une réentrée est remplacée par appel direct de handler ou si la chaîne poll change ; retour limité T9 au dernier C8/E8, sans fusionner les retries.

### T10 — État, tests privés et façade

- Owner / entrées : implémenteur ; D0/E0/C9/E9 ; baseline de toutes les suites et découverte V2 avant déplacement.
- Écriture production : A et les douze fichiers de R : `mod.rs`, `commands.rs`, `events.rs`, `poll.rs`, `closing.rs`, `queue.rs`, `import.rs`, `live.rs`, `microphone.rs`, `unified.rs`, `parakeet_views.rs`, `automatic_final.rs`. Budget : 13 production, justifié par le transfert unique d’état et le resserrement simultané des visibilités intermodules ; aucun corps fonctionnel remodelé.
- Écriture tests : déplacer `B/tests.rs` vers `R/tests.rs` et les quatre fragments `closing_and_recovery.rs`, `selection_and_microphone.rs`, `live_lifecycle.rs`, `parakeet_sessions.rs` de U vers `R/tests/`. Les cinq sources et cinq destinations sont les seuls chemins de migration ; pas de nouveau support de test.
- Faire : transférer les trois structs d’état, quatre bornes, constructeurs et `fail` vers `runtime/mod.rs` ; importer les symboles nécessaires aux tests descendants ; resserrer les méthodes à runtime, garder champs privés et neuf exports. A devient pure façade.
- Fermeture / métriques : zéro corps algorithmique de production dans A, un propriétaire `ImportApplication`, zéro ancien fichier de test, 44 déclarations historiques conservées ; deux caractérisations de T1 présentes séparément. Comparer les identités de tests hors préfixe de module, car `application::tests` devient `application::runtime::tests`. M1–M5, V1–V7.
- Arrêt / retour : arrêter sur test perdu, champ public, `#[path]`, API déplacée sans réexport ou dépassement non justifié ; retour limité au transfert T10 en conservant T1–T9 et les preuves, sans restauration globale.

## 5. Vérifications et provenance

Toutes les commandes de cette section sont proposées, NOT RUN, sauf les observations statiques de la base citées par le design. Cwd : `C:\dev\whisper` ; shell : PowerShell. Aucun test n’utilise de modèle/audio natif : les données sont les fixtures Rust et faux ports en mémoire observés.

Les chaînes V1–V7 sont uniquement les arguments Cargo internes à transmettre au lanceur compatible après D0/E0. Le chemin de ce lanceur et son invocation opérationnelle restent à établir ; ne jamais les exécuter directement. L’interface F3 observée `-Mode SelfTest` puis `-Mode Run -CargoArguments @(...)` n’est pas une autorisation ni une commande réutilisable telle quelle.

| ID | Arguments proposés / provenance | Quand et résultat attendu |
| --- | --- | --- |
| V1 | `fmt --all -- --check` — inféré du workspace/AGENTS | Chaque tâche ; présentation conforme sans écriture Cargo |
| V2 | `test --offline --locked -p whisper-core -- --list` — inféré des cibles observées | Baseline T1 et clôture T10 ; mêmes tests historiques, aucun filtre vide, préfixes de déplacement identifiés |
| V3 | `test --offline --locked -p whisper-core` — inféré des sources/tests | Baseline puis chaque tâche ; tous les contrats et 44 tests privés passent, nouveaux tests T1 inclus, skipped/ignored explicités |
| V4 | `test --offline --locked -p whisper-core --lib NOM` ou `test --offline --locked -p whisper-core --test CIBLE NOM` — forme inférée, NOM/CIBLE issus de S1–S8A | Avant/après tranche ; au moins le témoin attendu exécuté, bonnes branches/observations |
| V5 | `build --offline --locked -p whisper-core` — inféré du manifeste | Chaque tâche ; bibliothèque construite, sans prétendre qualifier les consommateurs natifs |
| V6 | `clippy --offline --locked -p whisper-core --all-targets -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity` — AGENTS/manifeste | Chaque tâche ; écarts nouveaux corrigés ou justifiés, héritage comparé, aucun `-D warnings` inventé |
| V7 | `check --offline --locked --workspace` — manifestes et commande du plan worker_ipc observés | T1 et T10 ; huit crates compatibles. Dépendance native/prérequis absent = arrêt du contrôle et résultat NOT RUN/FAIL explicite, sans téléchargement ni changement de périmètre |

Les compilations/checks d’autres crates servent seulement à vérifier la compatibilité de l’API ; leurs sources restent en lecture. Les exigences natives de V7 doivent être vérifiées avant lancement ; aucune installation de SDK/CUDA n’est induite.

Contrôles statiques hors campagne, toujours séquentiels aux commandes gardées :

| ID | Commande proposée et provenance | Constat attendu |
| --- | --- | --- |
| M1 | `git diff --check` ; `git diff --stat` ; `git diff --name-only` — Git projet observé | Pas d’erreur de présentation, chemins contenus dans la tâche, résidu racine décroissant |
| M2 | `rg -n '^(pub (struct|enum|trait)|pub use|impl |fn )' crates/whisper-core/src/application.rs crates/whisper-core/src/application` — déclarations observées | Examiner carte et unicité des définitions ; neuf exports compatibles et groupes déplacés à leur propriétaire |
| M3 | `rg -n 'fn (handle_event|dispatch|poll_once|maybe_publish|accept_segment|retry_pending_segment|handle_unified_capture_event|track_parakeet_capture_event|handle_automatic_final_event)' crates/whisper-core/src/application.rs crates/whisper-core/src/application` — symboles observés | Compléter par revue des branches de la tâche : absence de corps déplacé dans A, définition unique et délégation réelle, pas simple déménagement du monolithe |
| M4 | `rg -n '#\[path|pub .*mod|pub .*io|pub .*view|self\.dispatch' crates/whisper-core/src/application.rs crates/whisper-core/src/application` — risques du design | Lookup normal, modules privés, champs non publics, deux réentrées exactes conservées |
| M5 | `rg -n '#\[test\]' crates/whisper-core/src/application` ; `Get-ChildItem crates/whisper-core/src/application -Recurse -Filter *.rs \| ForEach-Object { [pscustomobject]@{Path=$_.FullName;PhysicalLines=(Get-Content -LiteralPath $_.FullName).Count} }` — mesures statiques inférées | 44 déclarations historiques privées ; fichiers de production ≤1 500 lignes, lecture manuelle séparant tests/production ; aucune mesure physique appelée « mesure Clippy » |

M2–M4 sont des repérages pour revue, pas des preuves automatiques de sémantique. Chaque tâche relève avant/après le nombre de corps de sa famille restant dans A : zéro à sa clôture ; les occurrences de variants dans les appels de routage ne sont pas des corps résiduels.

## 6. Carte d’exécutabilité des scénarios

Owner commun : implémenteur futur, reviewer indépendant pour le verdict. Écritures de source : chemins Tn seulement ; lecture indirecte : core domaine/ports/politiques et données Rust des tests. Écritures d’exécution : racine du Tn et cible Cargo partagée seulement, jamais les dossiers `models/` ou `sound example/`.

Les coûts ci-dessous sont des estimations indicatives avec cache préparé, pas des seuils produit ni engagements : V3/témoins 1–10 min par tranche ; build/lint 2–15 min ; V7 peut prendre 10–60 min selon les prérequis natifs. Baseline avant le premier déplacement, pas de répétition sans changement/échec pertinent.

| ID / tâches / AC | Témoins réellement observés et chemins de lecture | Données, observation et résultat attendu |
| --- | --- | --- |
| S1 — T1/T4/T9/T10, AC-01/07/08 | `U/selection_and_microphone.rs` : `model_guard_blocks_each_import_and_live_effect_route_before_submission`, `turbo_existing_matrix_dispatches_all_1206_requests_to_effect_submission` ; `U/closing_and_recovery.rs` : `close_blocks_direct_and_indirect_admissions_without_submitting_effects` | Matrices et requêtes des tests ; compter effets, comparer refus/vues après chaque commande ; admissions gardées et aucun effet sur refus |
| S2 — T2/T10, AC-02 | `I/scheduler_contract.rs` : `enqueue_is_acknowledged_only_after_the_durable_queue_event`, `interrupted_resume_requires_an_explicit_command_and_advances_generation` ; U sélection : `cancelling_duplicate_import_releases_only_the_new_source_snapshot`, `stale_duplicate_dialog_response_cannot_submit_a_replacement` | Faux port, enqueue/ack/doublon/générations ; mêmes effets ordonnés, pas de démarrage implicite, ancienne réponse sans remplacement |
| S3 — T3/T10, AC-05 | U sélection : `microphone_gain_command_is_scoped_to_the_active_direct_capture`, `microphone_test_gain_requested_during_open_survives_started_event`, `microphone_test_owns_one_capture_then_releases_before_live_restart`, `microphone_test_failure_invalidates_its_meter_and_source` ; `I/microphone_level.rs` : `spectrum_context_binds_source_and_rejects_stale_foreign_and_future_updates` | Identités/révisions/source des fixtures ; mêmes propriétaires, gain demandé conservé pendant ouverture, mesure périmée ignorée, source invalidée sur échec |
| S4 — T4/T10, AC-03 | `U/live_lifecycle.rs` : `immediate_live_stop_does_not_reenter_capturing_on_late_ready`, `direct_capture_and_engine_progress_independently_through_pre_ready_stop`, `live_stop_keeps_generation_until_segments_are_persisted_and_published` ; `I/live_contract.rs` : `forced_gpu_recovery_requires_an_explicit_cpu_finish_command` | Séquences capture/stop/Ready/segments ; mêmes phases et compteurs, pas de redémarrage tardif ni final CPU implicite |
| S5 — T5/T9/T10, AC-01/03/07 | `I/import_contract.rs` : `application_publishes_only_after_end_and_synced_segment_receipt`, `persist_backpressure_retains_ninth_segment_until_queue_capacity_returns`, `disconnected_persist_queue_is_fatal_and_does_not_retry_the_segment`, `refresh_drains_at_most_four_events_and_waits_for_every_persisted_ack`, `ready_and_following_failure_are_observable_on_separate_refreshes`, `stop_invalidates_generation_and_rejects_late_segment_but_accepts_stop_receipt`, `refused_publish_never_becomes_complete` | Segments/plages/ack/erreurs des tests ; effets conservés dans l’ordre, publication conditionnée, contrepression retenue et déconnexion fatale, refresh borné |
| S6 — T2/T10, AC-06 | U sélection : `destination_scan_stays_pending_until_history_and_queue_are_consumed`, `queue_scan_failure_clears_pending_state_and_is_visible` ; U fermeture : `snapshot_recovery_completion_closes_dialog_and_keeps_error_and_protected_reason_visible` | Événements transverses, récupération protégée et erreur ; mêmes indicateurs et message, sans filtrage prématuré par identité active |
| S6U — T6/T10, AC-03/04/05 | U sélection : `unified_page_responses_and_errors_follow_latest_selection_request`, `unified_session_card_cache_stays_bounded_at_eight` ; `U/parakeet_sessions.rs` : `unified_capture_ready_records_the_effective_native_backend`, `unified_capture_prepared_progress_end_publishes_only_after_total_is_known`, `unified_capture_rejects_progress_with_a_total_different_from_prepared_source`, `unified_capture_rejects_end_until_prepared_total_arrives`, `late_unified_gain_rejection_does_not_rollback_newer_capture_view` | Totaux/identités/requêtes de page ; même backend visible, anciennes réponses ignorées, aucune publication avant total et accusés applicables |
| S7 — T2/T9/T10, AC-07 | U fermeture : `close_removes_durable_queued_work_before_reporting_ready`, `close_keeps_failed_queue_removal_recoverable_and_retries_it`, `request_close_waits_for_an_already_committing_import_result` | File durable et `PortError::Committing` ; retraits/arrêts ordonnés, readiness retardée, publication engagée conservée |
| S7P — T7/T10, AC-04 | U Parakeet : `parakeet_provisional_never_becomes_confirmed_without_a_confirmed_event`, `parakeet_late_confirmation_keeps_identity_and_reorders_display_chronologically`, `direct_start_failure_before_live_started_keeps_a_capture_row_without_wav`, `turbo_start_failure_before_live_started_does_not_create_parakeet_capture` ; cache S6U | Hypothèses/confirmations/échecs ; aucune promotion implicite, identité et ordre conservés, même création/non-création de cartes |
| S8A — T8/T10, AC-04 | U Parakeet : `parakeet_automatic_final_import_stays_unverified_and_separate_from_confirmed_text`, `late_automatic_import_failure_updates_only_its_capture_row` | Capture et import final indépendants ; final non vérifié, confirmé intact, erreur attribuée à sa ligne |
| S8 — T1/T10, AC-08/09 | V2/V3/V5/V6/V7, M1–M5 ; `I/application_routing.rs` proposé T1 | API inchangée et suites historiques conservées ; deux nouveaux témoins sans modèle natif ; neuf exports, un propriétaire et façade déclarative |

Après T10, les chemins `U/...` deviennent `R/tests/...` par le déplacement exact décrit ; les noms courts des tests restent identiques. Les chemins I ne changent pas. Les fragments U sont en lecture avant T10, puis seulement déplacés à T10 ; aucune permission générale de les réécrire n’est créée.

Tous les scénarios et commandes ci-dessus sont NOT RUN dans cette mission. Si la couverture d’une branche obligatoire n’est pas démontrée, suspendre la seule tâche concernée et proposer le changement précis du plan ; ne pas ajouter silencieusement un fichier/test ou déclarer équivalence exhaustive.

## 7. Campagnes, arrêt et reprise

La garde F3 observée possède racines codées et exemptions historiques `UNATTRIBUTED`/Temp/Git/extérieur. Ne pas la lancer telle quelle ni transférer ses dérogations. E0 exige un accord précis pour l’adaptation limitée de l’existant aux racines du lot et aux exigences actuelles ; aucun compte, ACL, service, isolation ou surveillance nouvelle.

Avant chaque commande Cargo, y compris découverte/fmt : espace ≥30 Gio ; chemins canoniques et jonctions contrôlés ; `CARGO_TARGET_DIR=target/parakeet-tests/cargo-shared/` ; `CARGO_HOME`, `TEMP`, `TMP` sous racine de tâche ; aucun cache utilisateur en écriture ; `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. Copier un cache nécessaire sous la racine uniquement dans l’accord d’exécution applicable. Refus `fmt` sans `--check` et `--fix` conservés.

Auto-tests de refus hors racine avant Cargo, surveillance existante pendant commande, aucune inspection Git/recherche concurrente aux commandes gardées. Si les règles strictes ne sont pas satisfaites, ne pas lancer Cargo. Toute écriture hors racine détectée arrête la campagne ; conserver chemin/heure/preuves et demander arbitrage avant reprise. Une détection après écriture ne crée pas une autorisation.

À clôture de tâche : conserver les petits résultats utiles, archiver en préservant l’arborescence et vérifier nombre/SHA-256 selon AGENTS ; relever l’espace libre et consigner les résultats dans D. Aucun nouveau manifeste d’empreintes. Aucun nettoyage de `cargo-shared` ; ne retirer que des temporaires explicitement autorisés, sans présumer d’un manifeste de suppression approuvé.

Arrêts communs : baseline requise en échec ; contrôle de tâche partiel ; filtre ne lançant aucun témoin ; nouveau changement public/manifeste ; chemin ou budget dépassé ; état concurrent conflictuel ; besoin d’une décision structurante ; garde violée. Pas de passage à la tâche suivante.

Retour arrière : conserver le candidat et les preuves ; identifier le diff exact de la tâche ; demander arbitrage si l’état antérieur n’est pas établi ou si d’autres modifications se superposent. Aucun reset global ni suppression récursive automatique. Une correction fonctionnelle découverte devient une demande distincte.

## 8. Trame de future délégation et clôture

Avant délégation future, la session principale renseigne : tâche Tn, révision Git courante, preuve D0/E0/Cn-1/En-1, accord précis de code/campagne, chemins/budget de Tn, contrôles V/M et scénarios S attendus. L’implémenteur travaille uniquement dans ce périmètre, préserve les autres changements et remet diff, mesures et résultats sans verdict de revue.

Après candidat concret : revue indépendante par le circuit Rust prévu, corrections par l’implémenteur, au plus deux cycles. Constats ouverts après plafond : LIVRÉ AVEC RÉSERVES et décision utilisateur. La validation des moteurs/fichiers physiques et de la fenêtre Windows reste distincte et NOT RUN ; aucune remise d’EXE ni recette supplémentaire n’est imposée par ce plan.

Le plan est prêt sur le plan documentaire ; ses commandes restent non exécutables avant E0 et GO applicable. Readiness documentaire, acceptation finale, autorisation de code, campagnes, validation produit et push sont des décisions distinctes.

## 9. Revue indépendante et clôture documentaire

Le 30 octobre 2026, `rust_design_reviewer` (`/root/revue_design_plan`), indépendant de l’auteur et du plan writer, a rendu PLANS CLEAN sur `98b33ac15025cc2dab3666b89c151caadd2b538c`, avec design parent `682e09e7d8260f9e454441f50b11dc9d848895bb`. Zéro constat REQUIRED. Il a vérifié les dix tâches, chemins, budgets, prérequis, scénarios et arguments proposés ; les 42 noms de témoins existants cités dans la carte ont été retrouvés dans les sources. Les deux témoins T1 sont des ajouts futurs clairement identifiés.

Son unique suggestion ADVISORY P-APP-01 (mineure) précisait le comptage des contrats T1. L’auteur l’a appliquée : huit types de contrats déplacés, `ImportApplication` conservé jusqu’à T10 et neuf chemins publics disponibles. À la seconde passe ciblée, sur `ba5465a9bba6d02d43c61e735d1af007434acb12`, le même reviewer a fermé P-APP-01 et maintenu CLEAN ; aucun constat ouvert. Aucun autre contenu n’a été réaudité lors de cette passe. La présente consignation de statut et de preuve ne change pas le plan technique.

La mission a produit seulement ce plan et son cadrage, versionnés localement. Lectures et contrôles Git de présentation/périmètre effectués ; aucun code produit modifié, aucune campagne Cargo, compilation, exécution de test, validation native/Windows, remise d’EXE ou push. Les non suivis préexistants ont été préservés. Aucun résultat historique de campagne n’est revendiqué comme validation de ce refactoring futur.

## 10. Exécution — arrêt E0 du 30 octobre 2026

La demande d’exécution du plan a été confirmée dans la session par : « Oui, j’autorise l’adaptation de la garde sans anciennes exemptions et les campagnes T1–T10 dans les racines proposées, avec copie du cache Cargo et cible partagée. Exécute le plan avec les agents et revues prévus, sans push. » Cette autorisation couvre D0 et l’adaptation E0 ; elle ne dispense pas de réussir les auto-tests et préflights.

La session principale a délégué uniquement T1 à `rust_implementer`, sur la base `94720718c40945ed7f5482aaa5a738bb6b7810ff`, branche `evo-startup-perf`. La garde historique a été adaptée sous `target/parakeet-tests/refacto-application-core/t1/guard.ps1`, sans anciennes exemptions, et le cache Cargo copié sous `t1/cargo-home/` (29 837 fichiers). Les sorties et temporaires de préparation sont conservés sous T1. Aucun fichier Rust ou manifeste n’a été modifié. Les non suivis préexistants `docs/architecture/`, `models/` et `sound example/` sont préservés.

Préflight rapporté par l’implémenteur : Rust 1.98.1, rustfmt, Clippy, SDK Windows, MSVC, CUDA 12.9 et libclang présents ; analyse syntaxique PowerShell de la garde réussie. Espace libre avant préparation : 85 248 016 384 octets ; relevé après arrêt par la session principale : 85 525 430 272 octets. Ces relevés ne prouvent pas l’attribution des variations d’espace.

Commande exécutée : `pwsh -NoProfile -File target/parakeet-tests/refacto-application-core/t1/guard.ps1 -Mode SelfTest`. Résultat : FAIL, sortie 1. Le journal `target/parakeet-tests/refacto-application-core/t1/logs/guard-events.jsonl`, relu par la session principale, contient deux événements réels hors racines sous `C:\Users\cyril\Downloads\linkedin_cv_tool_PRGX\CV_LinkedIn\_session_chromium\Default\Code Cache\js` : suppression du fichier `959e6672afd8ef39_0` à `2026-10-30T18:16:50.1570099+01:00`, puis modification du dossier à `2026-10-30T18:16:50.3756455+01:00`. Le journal indique l’arrêt de l’arbre de processus surveillé. Le watcher ne fournit pas de PID : l’origine des écritures reste inconnue ; elles ne sont pas attribuées à Cargo ou à l’implémenteur.

E0 demeure non satisfait. Aucun Cargo, baseline V2/V3, contrôle V1–V7, test produit, compilation, revue de candidat Rust, recette native/Windows ou push n’a été exécuté. T1–T10 restent à réaliser. Aucun nettoyage ni exemption supplémentaire n’a été appliqué ; les preuves sont conservées. La reprise requiert l’arbitrage utilisateur prévu par AGENTS et la section 7 du plan.
