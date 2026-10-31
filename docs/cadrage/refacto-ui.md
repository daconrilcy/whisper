Avancée : cadrage MVC relu CLEAN ; deux constats fermés indépendamment.
Blocage : aucun constat DESIGN ouvert ; code et validations produit non exécutés.
Décision : DESIGN READY documentaire ; préparer le plan demandé, sans implémenter.

# Refactorisation de ui.rs — cadrage

## 1. Identité, autorité et livrables

- ID : DESIGN-UI v1, 31 octobre 2026, DESIGN READY documentaire après revue indépendante.
- Auteur des documents : session Codex principale. Contributions en lecture seule : product framer, requirements analyst, domain architect et rust architect, coordonnés par `/root/cadrage_ui`. Les sections ci-dessous intègrent leurs propositions ; elles ne revendiquent pas une copie littérale des contributions.
- Base produit inspectée : `7154e3fee990ffc08e9bebb4bdf094aabaaa6e8e`, branche `evo-startup-perf`.
- Besoin REF-UI v1 qualifié avec revue indépendante PASS, puis accepté par l'utilisateur dans cette session.
- Autorité précise : « Je valide ce besoin. Lance le cadrage et le plan de refactorisation de ui.rs, enregistre-les dans docs/cadrage/ et commite localement, sans implémenter ni pousser. »
- Livrables : ce document et `docs/cadrage/plan-refacto-ui.md`. Revues et suivi intégrés à ces documents, sans rapport supplémentaire.
- Mission actuelle : inspection statique, rédaction, revue des documents et commits locaux. Aucun code, campagne, test, Cargo, build ou push.
- Non suivis préexistants à préserver : `docs/architecture/`, `models/`, `sound example/`. Les cartes d'architecture sont des relevés anciens à confronter aux sources, pas un plan accepté.
- Espace C relevé avant la mission : 70 064 001 024 octets libres, soit environ 65,25 Gio. Aucune compilation engagée.

## 2. Besoin, version et critères acceptés

Version visée : refonte interne de l'interface Windows existante, pour rendre son code maintenable sans modifier l'expérience de transcription fichier ou microphone. Utilisateurs : utilisateurs actuels de Whisper et mainteneurs du desktop. Rust, egui/eframe et composition existants conservés.

| Critère | Résultat observable attendu après une implémentation autorisée |
| --- | --- |
| CA-01 | `ui.rs` reste une façade : modules, réexports explicites et délégations nécessaires ; usages publics compatibles. |
| CA-02 | Modèle, Vue et Contrôleur identifiables ; réutilisation des vues/commandes du cœur, sans dupliquer ses politiques. |
| CA-03 | Corps de tests et préparations de fixtures dans des fichiers dédiés ; instrumentation conditionnelle existante distinguée et justifiée. |
| CA-04 | Chaque fichier Rust du périmètre UI refactorisé, tests et supports inclus, fait au plus 600 lignes physiques, blancs/commentaires inclus, après formatage. |
| CA-05 | Mêmes états et entrées donnent les mêmes commandes et leur ordre, conditions d'activation, transitions, messages et résultats visibles. |
| CA-06 | Scénarios et assertions existants conservés, sans suppression/désactivation pour faciliter la refonte ; validation automatique distincte de l'observation Windows. |

Hors périmètre : nouvelles fonctionnalités, corrections fonctionnelles indépendantes, refonte visuelle, changement de moteur, de routage, de dépendance ou de contrat externe. Aucune migration, garantie durable supplémentaire, amélioration de performance, installation ou tray n'est ajoutée. Les comportements existants de ces chemins restent conservés lorsqu'ils sont traversés par l'UI.

La limite de 600 lignes porte sur les fichiers liés à cette refonte, pas sur tout le dépôt. Les cibles de fonctions/complexité des AGENTS restent applicables sans refonte automatique des autres modules.

## 3. Observations et sources primaires

Sources inspectées le 31 octobre : AGENTS racine et desktop ; manifestes workspace/desktop ; `src/ui.rs`, `src/ui/tests.rs` et sous-fichiers ; consommateurs `src/root.rs`, `src/lib.rs`, `tests/` desktop ; contrats du cœur, `path_settings.rs` et `microphone.rs`. Les chemins `src/` et `tests/` de ce document sont relatifs à `crates/whisper-desktop/`, sauf mention contraire. Numéros de ligne = base produit ci-dessus.

| Source | Observation |
| --- | --- |
| `src/ui.rs` | 3 869 lignes physiques ; état, actualisation, interactions, rendu et helpers concentrés. |
| `src/ui.rs:320–397` | `DesktopApp<A>` porte l'état de présentation unique et des champs d'observation `#[cfg(test)]`. |
| `src/ui.rs:193–279,612–775` | Mailbox préférences, polling et dispatch à conserver. |
| `src/ui.rs:777–1402,1795–2880` | Vues capture/unifiée, réglages micro et grande méthode `show_ui` à extraire par sections. |
| `src/ui.rs:578–602,3801–3802,3868–3869` | Préparation catalogue de test encore inline, inclusion de support conditionnel et déclaration du module de tests. |
| `src/ui/tests.rs` | 4 lignes, quatre `include!` ; corps des tests déjà externalisés. |
| `src/ui/tests/support_and_import.rs` | 668 lignes ; contexte partagé, faux services, import et cycle de vie. |
| `src/ui/tests/microphone_and_unified.rs` | 856 lignes ; doit être divisé sémantiquement. |
| `src/ui/tests/compute_and_catalog.rs` | 436 lignes. |
| `src/ui/tests/document_presentation.rs` | 360 lignes. |
| `src/ui/tests/root_support.rs` | 4 lignes ; helper de test à garder dans le contexte des tests. |
| `src/lib.rs:4`, `src/root.rs`, `tests/` | Module `ui` public, consommateurs à compiler sans changement d'import. |

Socle conservé : Rust 1.98.1, édition 2024, cible Windows `x86_64-pc-windows-msvc` ; `eframe = 0.35.0`, features `accesskit`, `default_fonts`, `glow`. Ces informations sont des relevés de configuration, pas une preuve de compilation de la refonte.

## 4. Capacités, exigences et acceptation

Chaque ligne porte CAP-nn, REQ-nn, UC-nn et AC-nn de même numéro. Owner du brief : product framer ; REQ/UC/AC : requirements analyst. AC décrit un contrôle futur, pas un résultat obtenu.

| IDs | Scénario / exigence de conservation | Acceptation future | Liens |
| --- | --- | --- | --- |
| 01 | Consommateur importe `ui::DesktopApp` et helpers existants. | Compilation avec imports et signatures inchangés ; façade inspectée. | CA-01, DOM-02, DEC-T01/T03 |
| 02 | Une interaction micro/import/sélection traverse Vue puis Contrôleur vers les services existants. | Responsabilités et dépendances traçables, aucune politique métier dupliquée. | CA-02, DOM-01/02, DEC-T02 |
| 03 | Lancer un test UI nécessitant un catalogue ou un faux service. | Fixtures hors production ; observations conditionnelles conservées. | CA-03/06, INV-06 |
| 04 | Mesurer tous les fichiers Rust de l'arbre cible après formatage. | Aucun fichier >600 lignes ; pas de découpage artificiel par tranches. | CA-04, INV-07 |
| 05 | Ouverture, attente asynchrone, actualisation, refus, annulation et fermeture occupée/libre. | Ordre de frame, erreurs, `CancelClose` et arrêt coopératif identiques. | CA-05, DOM-01/04, INV-02 |
| 06 | Ajouter, traiter, reprendre, annuler ou retirer un import/file. | Commandes, identités, génération et gardes identiques, retrait Running toujours refusé. | CA-05, DOM-05, INV-02/03/04 |
| 07 | Choisir/tester une source, régler gain, démarrer/arrêter, observer niveau et textes micro. | Même source effective, contrôles/mesures périmées, textes confirmés/provisoires et erreurs. | CA-05, DOM-03/05, INV-03 |
| 08 | Choisir calcul/modèle/langue puis recevoir catalogue/préférence/GPU tardif. | Choix touché prioritaire, mêmes révisions/notices et disponibilité ; mailbox conservée. | CA-05, DOM-04, INV-04/05 |
| 09 | Saisir/choisir un chemin, annuler le dialogue ou quitter pendant sa sauvegarde. | Mêmes valeurs, messages et confirmations de persistance ; pas de succès anticipé. | CA-05, DOM-03, INV-05 |
| 10 | Consulter historique, copies retenues, publication/reprise/nettoyage puis confirmer ou annuler. | Inspection sans effet implicite, mêmes commandes explicites et gardes d'admission. | CA-05, DOM-01/05, INV-04/05 |
| 11 | Comparer les suites et assertions avant/après déplacement. | Scénarios préservés, aucun nouvel ignore, résultats attribués au candidat ; Windows séparé. | CA-06, INV-06/07 |

Les fonctions d'installation, le tray et les nouvelles cibles de performance n'ont pas de REQ nouvelle dans cette refonte. L'objectif est de préserver la version actuelle, pas de qualifier à nouveau tout le produit.

## 5. Domaine, états et invariants

Contribution DOMAIN v1, owner `rust_domain_architect` ; architecture v1 owner `rust_architect`.

- **DOM-01 :** `Application` via `AppFacade<A>` est l'autorité métier ; `AppView` en est la projection. L'UI soumet les commandes sans reproduire les transitions du cœur.
- **DOM-02 :** une instance `DesktopApp<A>` possède l'état de présentation : sélections, notices, confirmations, caches et identités suivies. Pas de seconde source de vérité.
- **DOM-03 :** `PathSettingsApp` et `MicrophoneSettings` gardent leur rôle ; une saisie ou une notice n'est pas un succès durable.
- **DOM-04 :** catalogue/préférences/GPU gardent leurs récepteurs, révisions, protections `*_touched` et mailbox ; pas de nouveau canal, verrou ou thread.
- **DOM-05 :** les identités complètes `(JobId, Generation)` restent celles des requêtes et de la vue ; sélection de capture et source active restent distinctes.

| Flux | Métier / durable / visible et transitions conservées |
| --- | --- |
| Actualisation | État métier dans le cœur ; dernière `AppView` en UI. Succès de dispatch remplace la vue au même point ; refus conserve sa réponse visible. Aucun engagement durable par le rendu. |
| Préférences | Événement `Saved` confirme la sauvegarde ; une révision ancienne n'actualise pas la notice courante. Déconnexion catalogue signale l'erreur et retire le récepteur. Un choix touché n'est pas écrasé au chargement. |
| File | `Reprendre…` ouvre une confirmation ; confirmer émet `ResumeInterrupted`, annuler efface la confirmation. Retrait interdit pour Running. Même calcul de génération, y compris cas limite. |
| Microphone | Gardes d'admission existantes avant démarrage ; arrêt avec identité active. Rafraîchir la source prévue ne change pas implicitement le flux actif. Mesure périmée ou d'une autre capture reste distinguée. |
| Publication/reprise | État durable détenu par services ; résultat principal seulement pour Complete. Inspection ne libère pas les copies ; reprise/nettoyage nécessitent leurs actions explicites. Aucune nouvelle atomicité ou garantie après panne. |
| Fermeture | `close_requested/close_ready` viennent de l'application ; `quit_pending` appartient à l'UI. Occupé : CancelClose et demande coopérative existante. `on_exit` garde arrêt test, commits audio/destination puis fermeture inventaire dans l'ordre. |

Invariants : **INV-01** autorité métier unique ; **INV-02** mêmes commandes/paramètres/ordre/refus ; **INV-03** mêmes identités/générations ; **INV-04** priorité aux choix touchés et confirmations explicites ; **INV-05** confirmation seulement aux points existants (dispatch accepté, Saved, Complete) ; **INV-06** mêmes fixtures/scénarios/assertions et observations conditionnelles ; **INV-07** même API et au plus 600 lignes par fichier.

## 6. Décisions techniques et ownership MVC

**DEC-T01 — État privé et accès descendants.** Définir `DesktopApp<A>` dans `ui/model.rs` et conserver ses champs privés. `controller`, `view` et `tests` sont des enfants logiques de `ui::model`. L'accès descendant évite de rendre les champs publics. Méthodes utilisées entre branches : seulement `pub(in crate::ui::model)` ; helpers partagés avec présentation : au plus `pub(in crate::ui)`. Ne pas élargir à `pub(crate)` par commodité. L'option état dans un module frère nécessitant d'exposer tous ses champs est écartée.

**DEC-T02 — MVC adapté à l'existant.** Modèle métier dans le cœur ; modèle de présentation unique dans DesktopApp. Contrôleur pour polling/interactions/commandes ; Vue pour egui. Une vue appelle immédiatement le contrôleur au point actuel du clic : pas de collecte différée en fin de frame, qui changerait la logique observée par les widgets suivants.

**DEC-T03 — Façade compatible.** Conserver le fichier `ui.rs`, pas `ui/mod.rs`. Réexporter explicitement les symboles publics existants, constructeurs, types, fonctions et constantes, sans `pub use *` global. Les consommateurs conservent leurs imports ; aucun nouveau contrat public.

**DEC-T04 — Extraction sémantique du rendu.** Découper `show_ui` en méthodes appelées dans le même ordre, avec les mêmes closures/scopes et le même `Ui`. Aucun nouveau group/horizontal/ScrollArea/push_id ou wrapper de layout. Les noms locaux de méthodes sont des détails réversibles ; nouveaux chemins et changements de contrat reviennent au plan/design.

**Types partagés de présentation.** `SelectionWidgetState` et `ModelPreferenceEvent`, déjà publics, restent définis dans `model/preferences.rs`. `model.rs` les réexporte explicitement ; `presentation/selection.rs` importe ces deux types par `crate::ui::model::{SelectionWidgetState, ModelPreferenceEvent}`. Le module model reste privé dans ui : aucun chemin public externe nouveau. Cette dépendance nommée aux deux types passifs préserve les signatures de `apply_initial_model`, `apply_preference_event`, `selected_language`, `model_selection_combo` et `language_selection_combo` ; elle n'autorise aucun accès présentation à DesktopApp, ses champs, ses services, ses canaux ou son runtime. `PathDefaults`, `SaveStatus`, `AppView`, `JobState` et les types moteur/micro sont importés directement du cœur ; `LanguageChoices` relève de presentation/language. Les helpers privés partagés, dont `MicrophoneLabelProjection`, gardent un accès interne borné à ui, sans API publique supplémentaire.

### Arborescence cible et budgets

Budgets estimatifs après extraction, non preuves de taille. Tous les chemins ci-dessous sous `src/`. L'implémentation devra mesurer après formatage, imports inclus, et rester dans les chemins acceptés.

| Fichier cible | Responsabilité / repères source ui.rs | Budget lignes |
| --- | --- | ---: |
| `ui.rs` | Façade et réexports explicites | 100 |
| `ui/model.rs` | État, types privés, constructeurs/builders : 320–458,489–577,604–610 | 400 |
| `ui/model/preferences.rs` | Commandes/événements/runtime et mailbox : 193–292 | 160 |
| `ui/model/controller.rs` | Poll/dispatch 459–487,612–775 ; préframe 2014–2121 ; arrêt/refresh 1411–1437 ; gardes 3804–3817 | 500 |
| `ui/model/controller/actions.rs` | Import/live/test/gain 1496–1775 ; IDs 298–318 ; conversions 3437–3466 | 450 |
| `ui/model/controller/observability.rs` | Jalons 1439–1494 ; horodatages et premier rendu 3833–3866 | 180 |
| `ui/model/view.rs` | eframe::App 1777–1792 ; orchestration courte de show_ui | 160 |
| `ui/model/view/settings.rs` | Entête/chemins/modèle/langue/calcul/GPU 2126–2334 | 300 |
| `ui/model/view/actions.rs` | Actions import/live 2336–2400 et stop 2501–2524 | 180 |
| `ui/model/view/dialogs.rs` | Doublons 2402–2491 et récupération 2493–2499 | 180 |
| `ui/model/view/microphone.rs` | Réglages 1795–2013 et spectre 30–105 | 350 |
| `ui/model/view/unified.rs` | Vue Parakeet unifiée 777–1084 | 400 |
| `ui/model/view/capture.rs` | Vue capture 1086–1402 | 400 |
| `ui/model/view/active_job.rs` | État/progression/textes, blocs autour de 2526–2630 | 220 |
| `ui/model/view/notices.rs` | Messages/sauvegarde/résultat autour de 2631–2677 ; published_result 1404–1409 | 140 |
| `ui/model/view/queue.rs` | File et confirmation reprise 2679–2749 | 140 |
| `ui/model/view/history.rs` | Scan, nettoyage et historique 2750–2880 | 190 |
| `ui/presentation.rs` | Index des helpers et réexports vers façade | 100 |
| `ui/presentation/microphone.rs` | Mesure/source 21–28,108–191 ; labels 3164–3227 et constantes | 300 |
| `ui/presentation/gpu.rs` | GPU 2922–2931,2949–3108,3125–3162,3244–3271 | 280 |
| `ui/presentation/selection.rs` | Choix 2933–2947,3370–3435 ; admission/widgets 3467–3626 ; prédicat pur window_must_remain_open 3819–3831 | 370 |
| `ui/presentation/language.rs` | Choix et labels des langues 3628–3762 | 220 |
| `ui/presentation/paths.rs` | Sauvegarde 3110–3123, édition 3229–3242, chemins 3273–3275/3306–3317 ; défauts/hash 3364–3368/3764–3799 | 180 |
| `ui/presentation/text.rs` | Labels 3277–3304 et texte provisoire 3319–3362 | 120 |
| `ui/presentation/recovery.rs` | Widget récupération 2883–2920 | 100 |

Les plages servent à retrouver les responsabilités ; les frontières exactes suivent les accolades. Les imports et petites constantes suivent leur consommateur. Les fonctions longues sont divisées en helpers sémantiques dans les fichiers prévus ; le respect de 600 lignes ne suffit pas à justifier une fonction géante.

Le prédicat partagé `window_must_remain_open(JobState) -> bool` conserve une définition unique et son corps dans `presentation/selection.rs`, visible `pub(in crate::ui)`. `presentation.rs` le réexporte avec cette même visibilité interne, sans exposition par la façade publique. `presentation/microphone.rs` l'utilise via son parent ; le contrôleur et l'import privé de model pour les tests passent par `crate::ui::presentation::window_must_remain_open`. L'admission unifiée l'appelle localement. Ainsi présentation ne dépend pas du contrôleur.

### Tests, includes et visibilité

Dans `model.rs`, déclarer `#[cfg(test)] #[path = "tests.rs"] mod tests;`, en conservant le fichier physique `ui/tests.rs`. Ce module descendant accède aux champs privés du modèle. Garder les includes textuels dans son contexte lexical partagé. Rendre les noms privés nécessaires accessibles par des imports privés dans `model`, sans réexport public supplémentaire.

| Fichier cible sous src/ui/ | Traitement / budget |
| --- | --- |
| `tests.rs` | Index et contexte partagé, au plus 100 lignes |
| `tests/support.rs` | Fakes/helpers de support_and_import, au plus 500 |
| `tests/import_and_lifecycle.rs` | Tests import/fermeture du même fichier, au plus 350 |
| `tests/microphone_controls.rs` | Tests microphone, au plus 500 |
| `tests/unified_sessions.rs` | Tests sessions unifiées, au plus 500 |
| `tests/compute_and_catalog.rs` | Scénarios/assertions conservés, 436 actuellement |
| `tests/document_presentation.rs` | Scénarios/assertions conservés, 360 actuellement |
| `tests/catalog_fixture.rs` | install_test_model_catalog déplacé dans impl conditionnel de DesktopApp, accessible seulement dans model, au plus 80 |
| `tests/root_support.rs` | Helper speech_limit_samples dans contexte tests, au plus 20 |

Supprimer les anciens `support_and_import.rs` et `microphone_and_unified.rs` seulement après transfert intégral. Déplacer l'inclusion root_support hors du niveau production. Les champs/branches conditionnels existants observant les rectangles/activation des widgets restent conditionnels ; ils ne sont pas des corps de test à supprimer. Aucun hook nouveau ni feature ajoutée.

Le préfixe Rust des tests unitaires change de `ui::tests` à `ui::model::tests`. Comparer scénarios, noms courts et assertions avec correspondance explicite, pas les seuls préfixes. Les `use super::{…}` et références `super::` des includes doivent être adaptés de manière cohérente ; préserver les supports partagés.

## 7. Matrice des dépendances et contrats

| Consommateur | Dépendances conservées/autorisées | Interdit / contrôle futur |
| --- | --- | --- |
| Façade ui | model/presentation via réexports explicites | Traitement ou état parallèle ; inspection |
| model | Types/API core, services déjà détenus et helpers de présentation existants | Nouvelle politique métier/effet ; revue champs et constructeurs |
| controller | État parent, AppFacade et services déjà utilisés | Nouveau worker/thread/verrou/routage ; comparaison points d'effets |
| view | État parent, helpers, méthodes contrôleur à visibilité bornée | Nouvelle politique/IO/scopes egui ; revue ordre et vrais widgets |
| presentation | Types core, egui, helpers purs de presentation ; seulement SelectionWidgetState et ModelPreferenceEvent de model, selon les imports nommés ci-dessus | DesktopApp/son état vivant, services, canaux/runtime-mailbox, contrôleur/root ou nouvel IO |
| core/adapters/workers | Aucun changement | Dépendance vers UI, changement de contrat/manifeste |

Écarts hérités à la cible de séparation : CatalogSnapshot vient de whisper_adapters ; ArchiveStore::browse_treatment_folder est appelé dans ui.rs:2471–2477. Les consignes desktop reconnaissent ces accès directs existants. Cette refonte les conserve sans les étendre et sans revendiquer une séparation infrastructure/UI totale. Leur suppression ne fait pas partie du mandat.

Contrats de frontière :

- Frame : polling modèle → sonde GPU conditionnelle → inventaire GPU → notices chemins → Refresh → jalons/réconciliation identités → busy/fermeture → widgets → scan historique à sa position actuelle. Délais de repaint 33/50/100 ms conservés.
- Dispatch : mêmes paramètres et refus ; actualisation AppView au même point avant les widgets suivants. Pas de file d'actions différées.
- Préférences : réveil sync_channel(1), derniers choix coalescents, restitution modèle puis calcul ; plein/déconnecté et comportement mutex inchangés. Révisions et événements tardifs traités comme actuellement.
- egui : IDs explicites, ordre d'allocation, labels, scopes et formes observées conservés. Ne pas déplacer un effet hors de sa closure pour simplifier un emprunt.
- Durabilité : mêmes services, formats et confirmations ; pas de migration, nouvelle atomicité ou reprise après panne. Annuler une confirmation n'émet pas sa commande.
- Fermeture : même protocole coopératif et ordre de on_exit ; pas de destruction immédiate remplaçant une attente.
- Plateforme et sécurité : aucun nouvel unsafe/FFI/native, réseau, packaging, dépendance, feature ou modification Cargo.lock.

## 8. Risques et preuves attendues

| ID / owner | Risque / critères | Fermeture future |
| --- | --- | --- |
| RISK-F01/T02, analyste/architecte | Réordonner frame ou allocation egui ; CA-05 | Comparer ordre des appels, IDs/scopes, tests vrais widgets et observation Windows |
| RISK-F03, domaine | État parallèle ou succès anticipé ; CA-02/05 | Revue ownership et confirmations, refus et événements obsolètes |
| RISK-F04/T04, analyste/architecte | Tests/assertions perdus lors des includes ; CA-03/06 | Inventaire avant/après, correspondance des scénarios, aucun ignore ajouté |
| RISK-F05, analyste | Confirmation rendue implicite ; CA-05 | Tester inspection/annulation/confirmation et commandes émises |
| RISK-T01, architecte | Visibilité cassée ou élargie ; CA-01/03 | Modules descendants, compilation des consommateurs et tests sans API publique nouvelle |
| RISK-T03, architecte | Fichier >600 après formatage ; CA-04 | Comptage physique de tout le périmètre final et marge par module |
| RISK-T05, architecte | Garde historique impropre à la campagne UI | Préflight futur conforme ; arrêter avant Cargo si conformité non établie |

Faisabilité du découpage : analyse statique des blocs et de leur ownership ; budgets estimés. Compilation de la refonte NOT RUN. Pas de nouvelle technologie ou hypothèse produit nécessitant un spike supplémentaire.

Garde observée : `target/parakeet-tests/refacto-archive/guard.ps1`, modes SelfTest/Run et CargoArguments ; racine archive codée et tolérances Temp non transférables. Elle ne peut pas être utilisée telle quelle pour UI. Une adaptation minimale pourra être une tâche explicite du plan futur, sous accord précis, racines proposées `target/parakeet-tests/refacto-ui/` et `target/parakeet-tests/cargo-shared/`. Aucun transfert des dérogations historiques.

La surveillance après écriture n'est pas un confinement préventif. Si la garde ne permet pas de respecter les règles permanentes applicables, arrêter avant Cargo et demander un arbitrage ; le cadrage ne délivre aucune dérogation. Aucun auto-test de garde lancé ici.

## 9. Readiness et revue

| Rubrique DESIGN | Applicabilité / preuve documentaire |
| --- | --- |
| scope/use_cases/errors | Sections 2–4 ; nominal, attente, refus, annulation, fermeture et reprise existantes |
| domain | Section 5, responsabilités, transitions et points de confirmation |
| dependencies/platform | Sections 6–7, chemins/visibilité et socle conservé |
| persistence | Préservation des contrats existants ; aucune migration/nouvelle garantie à concevoir |
| concurrency | Polling/mailbox existants conservés ; aucun nouveau thread/file/verrou |
| acceptance/risks | CA-01–06, AC-01–11 et section 8 ; preuves produit futures NOT RUN |
| railguard | Analyse des règles AGENTS déjà actives, pas activation d'un nouvel artefact de gouvernance |

Questions produit structurantes : aucune. Détails réversibles : noms des méthodes d'extraction, owner implémenteur, avant le déplacement concerné, dans les chemins/contrats définis. Un nouveau chemin ou changement de contrat est un changement de plan/design à faire revoir, pas une permission implicite.

État initial : candidat documentaire, revue indépendante à effectuer sur Git. Seul le reviewer peut déclarer CLEAN et fermer ses constats, avec deux cycles revue/correction maximum. Le plan est déjà demandé par l'utilisateur ; il pourra être préparé après revue DESIGN satisfaisante. DESIGN READY et PLANS READY resteront distincts d'un GO de code.

Revue DESIGN initiale par `/root/cadrage_ui/review_ui`, indépendant de la rédaction, sur `9b69ce9fcf35d4e50ee6ec6c49636200f9ff1656` : **FINDINGS**, un constat UI-D01 IMPORTANT/Medium REQUIRED. Preuve : `microphone_test_start_enabled` et `unified_live_admission_available` appellent le prédicat alors placé dans controller, en contradiction avec la matrice. Condition de fermeture : placement partagé pur, définition unique, imports/visibilités/mapping/budgets alignés. Correction proposée par l'architecte : placement dans presentation/selection et accès internes décrits ci-dessus. Fermeture réservée au reviewer lors de la relecture Git ; aucun autre constat initial.

Relecture sur `057ac834196f44d12fd0ba7ef2ce48bbec7fb9f0` : UI-D01 **CLOSED** par le reviewer ; **FINDINGS**, nouveau UI-D02 IMPORTANT/Medium REQUIRED. Preuve : la matrice interdisait toute dépendance presentation vers model alors que les helpers de sélection prennent SelectionWidgetState et ModelPreferenceEvent placés dans model/preferences. Correction finale de l'architecte, deuxième cycle : autoriser seulement ces deux types passifs avec imports explicites, et préciser les accès aux helpers et types core comme ci-dessus. Aucun nouvel accès à l'état vivant ni changement d'API ; fermeture UI-D02 réservée au reviewer sur la version corrigée Git.

Verdict final brut de `/root/cadrage_ui/review_ui` sur `9dabf0f2da23fb4330b88f9363ce162b2a2bc86c` : **CLEAN** ; UI-D01 et UI-D02 **CLOSED**, aucun REQUIRED ouvert ni suggestion. Révision Git contrôlée inchangée pendant la revue. Deux cycles de correction consommés. Statut courant : **DESIGN READY documentaire** ; la présente inscription du verdict est une preuve de revue, pas une modification du design technique ni une autorisation de code/campagne/push.

Contrôles de cette mission : lectures ciblées et git diff --check ; aucun test, build, Cargo, moteur natif ou ouverture Windows. La future recette Windows restera celle de l'utilisateur ; aucun binaire remis ici.
