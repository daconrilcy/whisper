Avancée : MVC raccordé ; finitions des fonctions longues et poursuite autorisées avec réserve V7 explicite.
Blocage : fonctions longues UI et contrôles affectés à terminer avant revue.
Décision : terminer dans les chemins prévus, sans corriger core ni neutraliser les avertissements ; revue et commits locaux, sans push.

# Refactorisation de ui.rs — plan PLAN-UI v1

## 1. Autorité, référence et limites

- Date : 31 octobre 2026. Auteur : session Codex principale ; proposition PLAN v1 de `rust_plan_writer`, `/root/cadrage_ui/plan_ui`, en lecture seule, intégrée ci-dessous sans revendication de copie littérale.
- Autorité utilisateur : « Je valide ce besoin. Lance le cadrage et le plan de refactorisation de ui.rs, enregistre-les dans docs/cadrage/ et commite localement, sans implémenter ni pousser. »
- Design parent : [refacto-ui.md](refacto-ui.md), DESIGN-UI v1, technique revue **CLEAN** sur `9dabf0f2da23fb4330b88f9363ce162b2a2bc86c` ; inscription du verdict dans `82ae582c386342d6bea97005d2360fb2232c6170`. UI-D01 et UI-D02 fermés par le reviewer indépendant.
- Base produit inspectée : `7154e3fee990ffc08e9bebb4bdf094aabaaa6e8e`, branche `evo-startup-perf`. Relever de nouveau Git avant exécution ; une divergence inexpliquée n'est pas une progression prévue.
- Sources du plan writer : design Git, AGENTS racine/desktop, manifeste desktop, lib.rs, sources/tests UI et interface réelle de la garde archive.

Objectif : façade ui.rs, MVC, tests/fixtures séparés, au plus 600 lignes physiques par fichier concerné, sans changement de logique/comportement. Préserver CA-01 à CA-06 et REQ/UC/AC-01 à11 du design.

Cette autorisation couvre les documents, leurs revues et commits locaux. Ce plan ne déclenche ni code, ni adaptation de garde, ni campagne, ni remise de binaire. Ces actions nécessiteront leur accord précis dans la session d'exécution.

Aucun changement de core/adapters/workers, dépendance, feature, manifeste, Cargo.lock, routage, format durable ou interface publique. Les accès infrastructure hérités reconnus par le cadrage ne sont pas élargis.

## 2. Périmètre exact des chemins

Les chemins du tableau sont relatifs à `crates/whisper-desktop/src/`. Les listes constituent les destinations prévues, sans autoriser leur écriture maintenant.

| Ensemble | Fichiers |
| --- | --- |
| P-UI, façade/modèle | `ui.rs`, `ui/model.rs`, `ui/model/preferences.rs` |
| P-C, contrôleur | `ui/model/controller.rs`, `ui/model/controller/actions.rs`, `ui/model/controller/observability.rs` |
| P-V, vues | `ui/model/view.rs`, puis `ui/model/view/settings.rs`, `actions.rs`, `dialogs.rs`, `microphone.rs`, `unified.rs`, `capture.rs`, `active_job.rs`, `notices.rs`, `queue.rs`, `history.rs` (tous ces noms courts sont sous `ui/model/view/`) |
| P-P, présentation | `ui/presentation.rs`, puis `ui/presentation/microphone.rs`, `gpu.rs`, `selection.rs`, `language.rs`, `paths.rs`, `text.rs`, `recovery.rs` (noms courts sous `ui/presentation/`) |
| P-T, tests/supports | `ui/tests.rs`, puis `ui/tests/support.rs`, `import_and_lifecycle.rs`, `microphone_controls.rs`, `unified_sessions.rs`, `compute_and_catalog.rs`, `document_presentation.rs`, `catalog_fixture.rs`, `root_support.rs` (noms courts sous `ui/tests/`) |

Sources historiques `ui/tests/support_and_import.rs` et `ui/tests/microphone_and_unified.rs` : modifiables puis supprimables uniquement après transfert intégral des contenus utiles et assertions.

**P-E, exécution future après accord précis :** `target/parakeet-tests/refacto-ui/` pour garde adaptée, cache Cargo local, temporaires, sorties et petites preuves ; `target/parakeet-tests/cargo-shared/` comme seule racine Cargo. Aucun autre emplacement d'écriture applicative volontaire. Garde proposée : `target/parakeet-tests/refacto-ui/guard.ps1`, non créée par cette mission.

**P-D, suivi documentaire :** les deux documents de cette mission, sans rapport supplémentaire.

Lecture indirecte, sans modification : `src/lib.rs`, `src/root.rs`, `src/reactivity_log.rs`, les cinq fichiers d'intégration desktop nommés section 8, manifestes/toolchain, contrats core et services consommés, garde archive de référence. Préserver les non suivis `docs/architecture/`, `models/`, `sound example/`. Tout chemin supplémentaire demande une modification du plan selon les règles du dépôt.

## 3. Lots, dépendances et responsabilités

Huit tâches séquentielles. Chemins partagés, notamment ui.rs et raccords tests : aucune parallélisation d'écriture proposée. Implémentation future par `rust_implementer` en mode IMPLEMENT après GO conforme aux AGENTS ; revue par `rust_review_coordinator`, reviewer indépendant et corrections par l'implémenteur. La principale reste auteur du bilan documentaire.

| Lot | Tâches | Prérequis typés | Sortie |
| --- | --- | --- | --- |
| L-UI-0, préparation | T1–T2 | DOCUMENT : design/plan relus, GO précis ; CODE : base actuelle ; EXECUTION : garde conforme avant Cargo | Référence caractérisée et campagne praticable, ou arrêt documenté |
| L-UI-1, refactorisation | T3–T5 | DOCUMENT : sorties L-UI-0 ; CODE : tests/supports disponibles ; EXECUTION : baseline obtenue | MVC raccordé, façade, fichiers ≤600 et couverture conservée |
| L-UI-2, vérification/remise | T6–T8 | CODE : candidat L-UI-1 ; EXECUTION : contrôles consolidés puis revue | Candidat revu, preuves et limites ; observation Windows distincte |

Graphe : **T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8**. Les corrections T7 reviennent dans les chemins concernés sans créer une nouvelle étape de revue. Maximum dix tâches et dix fichiers de rapport par lot ; ce plan compte huit tâches et deux documents.

## 4. Les huit tâches

| ID | Travail et chemins | Critères / résultat observable |
| --- | --- | --- |
| **T1** | Préflight et, si expressément autorisée, adaptation minimale de la garde dans P-E. Lire la garde archive sans l'exécuter telle quelle. Racines UI et Cargo partagée ; aucune tolérance Temp historique transférée. | Préalable des vérifications : auto-tests/contrôles canoniques satisfaits, ou STOP avant Cargo. La surveillance n'est pas un confinement préventif. |
| **T2** | Avant toute modification de production : inventaire API publique, fichiers, tests/scénarios/assertions et ordre des effets ; contrôles baseline autorisés. Ajouter seulement si nécessaire une caractérisation ciblée dans P-T pour un invariant non observé, avec production inchangée. | CA-01/05/06, AC-01/05–11 : référence attribuée au SHA/configuration, lacunes et échecs existants explicites. Aucun test miroir du découpage futur. |
| **T3** | Découper tests/supports P-T ; déplacer install_test_model_catalog vers support conditionnel et speech_limit_samples dans contexte tests. Adapter includes/imports de ui.rs/tests.rs ; conserver le propriétaire actuel jusqu'au raccord T4. | CA-03/04/06, AC-03/04/11 : mêmes scénarios/assertions ; tests finalisés ≤600 et suite desktop compilable au point de contrôle T3. Pas de champ public/hook nouveau. |
| **T4** | Migration cohérente P-UI/P-C/P-V/P-P et raccord P-T : état model, impl descendants, présentation, façade/réexports explicites. | CA-01/02/04/05, AC-01/02/04–10 : MVC complet, code compilable au terme de la tâche ; pas de politique déplacée dans core ni nouvel effet. Voir raccord atomique ci-dessous. |
| **T5** | Réconcilier API, ownership/imports, définitions uniques, ordre frame/dispatch, IDs/scopes egui, scénarios/assertions et tailles. Compléter uniquement les caractérisations utiles manquantes P-T et corriger les écarts de refonte dans les chemins prévus. | Tous CA, AC-01–11 : rapprochement avant/après montrant mêmes commandes/paramètres/ordre/refus/résultats. Pas de correctif fonctionnel indépendant. |
| **T6** | Validation consolidée sous garde : format, tests desktop par défaut et personal-install, build desktop dans ces configurations, Clippy applicable et lints structurels. | CA-01/04/05/06 : résultats attribués au candidat ; distinguer défaut baseline/nouveau, aucun Cargo hors garde. |
| **T7** | Revue du candidat par rust_review_coordinator ; reviewer indépendant, corrections rust_implementer, même reviewer pour fermeture. Répéter les vérifications affectées par les corrections. | Tous CA : deux cycles revue/correction maximum ; CLEAN ou LIVRÉ AVEC RÉSERVES et constats ouverts. Aucun verdict Windows déduit. |
| **T8** | Si remise demandée et autorisée, préparer recette du profil réellement construit ; utilisateur observe la fenêtre. Clore P-D avec fichiers, SHA, résultats/réserves et espace final ; commits locaux seulement si autorisés. | CA-05/06 : rendu/dialogues/fermeture observés par utilisateur ou NOT RUN. Aucun push, ouverture automatique ou paquet autonome annoncé sans qualification. |

**Raccord atomique T4 :** les champs privés exigent de déplacer ensemble leur propriétaire et les impl qui y accèdent. Les opérations internes ne sont pas annoncées compilables individuellement ; point de validation = raccord complet. Ne pas compenser par pub(crate), copie d'état ou fichier provisoire géant. Aucun sous-plan de tâches supplémentaire caché.

Les fichiers de production non encore extraits peuvent conserver leur taille de référence pendant T2–T3. Les fichiers créés/finalisés respectent leur budget ; après T4 tout le périmètre final est ≤600. Les extractions restent sémantiques, pas des tranches de lignes.

## 5. Contrats particuliers

- DesktopApp garde ses champs privés ; controller/view/tests descendants de ui::model. Méthodes interbranches limitées à `pub(in crate::ui::model)`.
- window_must_remain_open : définition pure unique dans presentation/selection, réexport interne borné à ui. Aucun réexport public nouveau.
- Présentation peut utiliser seulement les deux types passifs nommés SelectionWidgetState et ModelPreferenceEvent du modèle, via les imports du design ; aucun accès à DesktopApp/services/runtime.
- Même ordre de frame, dispatch immédiat, révisions et protections *_touched. Mailbox sync_channel(1), coalescence et restitution modèle puis calcul inchangées.
- Aucun scope/ID egui ajouté, effet sorti de sa closure ou changement d'ordre pour résoudre un emprunt.
- Reprises/nettoyage explicites : inspection et annulation n'émettent pas la confirmation.
- Rapprocher noms courts/scénarios/assertions ; préfixe des tests unitaires ui::tests → ui::model::tests prévu, sans suppression de scénario.

## 6. Préflight obligatoire de chaque lot futur

PowerShell, cwd `C:\dev\whisper`. Tout est proposé, **NOT RUN** ici. Vérifier documents Git, verdicts, GO et chemins ; relever git status --short, branche/SHA et espace libre C. Sous 30 Gio, arrêter avant compilation/test. Coordonner les responsabilités avant modifications concurrentes.

Avant le premier Cargo, fixer/vérifier canoniquement les variables et l'absence de jonction sortante :

| Variable | Valeur |
| --- | --- |
| CARGO_HOME | `C:\dev\whisper\target\parakeet-tests\refacto-ui\cargo-home` |
| TEMP | `C:\dev\whisper\target\parakeet-tests\refacto-ui\temp` |
| TMP | `C:\dev\whisper\target\parakeet-tests\refacto-ui\tmp` |
| CARGO_TARGET_DIR | `C:\dev\whisper\target\parakeet-tests\cargo-shared` |
| CARGO_INCREMENTAL | `0` |
| CARGO_PROFILE_DEV_DEBUG | `line-tables-only` |

Si cache hors ligne nécessaire, copier son contenu sous CARGO_HOME du lot avant campagne dans la portée autorisée ; aucune écriture au cache du profil utilisateur, aucun téléchargement ni changement Cargo.lock. Les processus enfants héritent de l'environnement contrôlé.

Référence réelle : garde `target/parakeet-tests/refacto-archive/guard.ps1`, modes SelfTest/Run et CargoArguments, racine archive codée et tolérances Temp inapplicables à UI. T1 ne reconduit aucune dérogation. La garde future doit refuser variables absentes/externes et jonctions sortantes avant Cargo, auto-tester ces refus, puis surveiller pendant la commande. Auto-tests avec événements simulés, sans créer une fuite externe pour la tester.

L'observation après écriture ne fournit pas de confinement préventif. Conformité non démontrable : STOP/arbitrage avant Cargo, pas de dérogation implicite. Aucun nouveau compte/service/tâche planifiée/surveillance supplémentaire n'est déduit de ce plan.

## 7. Commandes de vérification proposées

Toutes **NOT RUN**, après T1 et GO ; PowerShell, cwd `C:\dev\whisper`. La garde future doit exister et être conforme : ces lignes ne prouvent pas sa disponibilité. Les lignes Cargo sont exclusivement les arguments du lanceur gardé, jamais des commandes directes.

| ID | Commande proposée | Observation attendue |
| --- | --- | --- |
| V0 | `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode SelfTest` | Refus des mauvaises racines/variables et contrôles requis ; aucun Cargo préalable |
| V1 | `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode Run -CargoArguments @('fmt','--all','--','--check')` | Format conforme ; fmt n'accepte pas offline/locked |
| V2 | `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-desktop','--lib','--tests','--','--list')` | Inventaires baseline/final, préfixes renommés rapprochés |
| V3 | `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-desktop','--lib','--tests')` | Unitaires/intégration desktop, mêmes comportements/assertions |
| V4 | `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-desktop','--lib','--tests','--features','personal-install')` | Couverture dans configuration personnelle existante |
| V5 | `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode Run -CargoArguments @('build','--offline','--locked','-p','whisper-desktop','--bins')` | Composition/binaire/consommateurs compilent |
| V6 | `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode Run -CargoArguments @('build','--offline','--locked','-p','whisper-desktop','--bins','--features','personal-install')` | Profil personnel compile, routage conservé |
| V7 | `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode Run -CargoArguments @('clippy','--offline','--locked','-p','whisper-desktop','--all-targets','--','-D','warnings')` | Lints usuels sans nouveaux avertissements |
| V8 | `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode Run -CargoArguments @('clippy','--offline','--locked','-p','whisper-desktop','--all-targets','--features','personal-install','--','-W','clippy::too_many_lines','-W','clippy::cognitive_complexity','-W','clippy::too_many_arguments','-W','clippy::type_complexity')` | Écarts structurels attribués, hérités hors périmètre distingués |
| V9 | `git diff --check` | Aucun défaut d'espacement |

Pour une caractérisation ciblée, V3 peut recevoir après `--` un filtre de nom court identifié section 8. Ne pas répéter toutes les campagnes sans changement pertinent.

Comptage statique : relever `[IO.File]::ReadAllLines($chemin).Length` sur ui.rs et tous les fichiers *.rs sous src/ui/, après formatage. Aucun >600 accepté à l'état final. Ce relevé n'est pas un manifeste de qualification.

T2 : V2/V3/V4 et les contrôles baseline nécessaires pour attribuer d'éventuels défauts. T3 : tests/tailles des tests ; T4 : V1/V3/V5 au raccord complet. T6 : jeu final V1–V9 ; corrections T7 : contrôles affectés seulement.

## 8. Carte d'exécutabilité et traçabilité

Owner automatique : implémenteur/validateur désigné ; owner fenêtre : utilisateur. Données = mêmes états, faux services, événements et interactions que la référence. Aucun modèle natif nécessaire aux tests UI n'est supposé sans inspection des prérequis. Toutes observations **NOT RUN**. Coût indicatif : quelques dizaines de minutes d'examen par famille, hors compilation ; durée totale dépend du cache et de la machine, sans cible produit nouvelle.

| AC / tâches | Chemins ; données et stimulus | Observation attendue / méthode |
| --- | --- | --- |
| AC-01 / T2,T4–T6 | P-UI/P-P ; lib/root et intégration en lecture ; imports/constructeurs/helpers existants | Mêmes symboles/signatures, façade sans traitement ; V3/V5/V6 et revue exports |
| AC-02 / T4,T5 | P-UI/P-C/P-V/P-P ; clic start micro, ajout import, choix sélection | Chaîne état→widget→contrôleur→commande→vue sans politique parallèle ; revue ciblée/vrais widgets V3 |
| AC-03 / T3,T5 | P-T/raccord P-UI ; catalogue et faux services des tests | Fixtures hors production, observation cfg(test) conservée ; inventaire statique et V3 |
| AC-04 / T3–T5 | Tous fichiers finaux après formatage | Aucun >600, imports/blancs inclus ; V1 et compte physique |
| AC-05 / T2,T4,T5,T8 | P-C/controller, P-V/view, tests lifecycle ; événements async, ouverture, refus, fermeture busy/idle | Même polling/ordre, CancelClose et sortie coopérative ; refus ne fige pas quit_pending. V3 : window_stays_open_through_capture_and_drain, rejected_quit_admission_does_not_latch_pending_close ; caractérisation ordre si lacune et fenêtre utilisateur |
| AC-06 / T2,T4,T5 | P-C/actions, P-V/actions/queue/dialogs, tests import ; import valide/invalide/doublon, Running, confirmation/annulation | Mêmes gardes, génération, compute durable ; aucune reprise implicite. V3 : queued_import_uses_current_compute_and_preserves_the_durable_request, resumed_import_uses_its_durable_compute_instead_of_the_global_selector ; compléter confirmation si lacune |
| AC-07 / T2,T4,T5,T8 | P-C/actions, P-V/microphone/unified/capture, P-P/microphone, P-T ; source présente/absente/changée, identité ancienne, mesure périmée, test/gain/start/stop | Mêmes identité/activation/niveau/spectre/textes/commandes. V3/V4 : microphone_test_button_stops_after_neutral_gain_ack_and_reopens, microphone_saturation_badge_requires_current_identity_and_fresh_limiter_data, unified_capture_gain_slider_targets_the_current_source_and_revision ; fenêtre utilisateur |
| AC-08 / T2,T4,T5 | P-UI/preferences, P-C, P-P/selection/gpu/language, tests compute/intégration ; événement tardif, choix touché, ancienne révision, mailbox pleine/déconnectée | Même priorité/disponibilité/notices/coalescence. V3/V4 : late_saved_preferences_do_not_overwrite_a_newer_user_choice, preference_mailbox_is_bounded_and_keeps_the_latest_choice_per_field, disconnected_preference_worker_reports_failed_model_and_compute_saves |
| AC-09 / T2,T4,T5,T8 | P-V/settings, P-P/paths, P-T ; PathSettingsApp/dialogues en lecture ; saisie/choix/annulation, chemin manquant, sauvegarde en attente/erreur | Mêmes valeurs/messages/confirmation durable, annulation conserve valeur ; V3, caractérisation ciblée si manque et dialogues Windows utilisateur |
| AC-10 / T2,T4,T5 | P-V/history/dialogs/notices, P-P/recovery, tests documents/intégration ; historique protégé/récupérable, inspection/annulation/confirmation | Aucun effet implicite, mêmes pages/commandes/résultat Complete. V3 : recovery_ui_displays_protected_and_recoverable_entries_without_implicit_recovery, unified_ui_requests_only_the_selected_identity_pages |
| AC-11 / T2,T3,T5,T6 | P-T et intégration inchangée ; inventaires baseline/final, diff assertions | Aucun scénario/ignore/suppression dissimulé ; résultats attribués SHA/config ; V2/V3/V4 et revue diff |

Intégration lue/exécutée sans modification : `crates/whisper-desktop/tests/gpu_inventory_ui.rs`, `gpu_selection_ui.rs`, `microphone_level_ui.rs`, `model_selection_ui.rs`, `reprocessing_recovery_ui.rs` (tous sous ce même dossier). Nouvelles caractérisations uniquement dans P-T et d'abord sur production inchangée en T2 ; compléments T5 limités aux lacunes de conservation.

Sens inverse : P-UI→AC-01/02/05 ; P-C→AC-02/05–09 ; P-V→AC-02/05–10 ; P-P→AC-01/02/07–10 ; P-T→AC-03/11 et scénarios associés ; P-E préalable d'exécution ; P-D bilan prévu. Chaque CA du design est couvert : CA-01→AC-01, CA-02→AC-02, CA-03→AC-03, CA-04→AC-04, CA-05→AC-05–10, CA-06→AC-11.

## 9. Arrêt, récupération, recette et clôture

Arrêter le lot concerné : autorisation manquante, espace <30 Gio, variable/racine/jonction invalide, garde non conforme, fuite attribuée, modification inexpliquée, échec test/build ou découverte demandant un changement fonctionnel/structurel hors design. Préserver les preuves utiles ; ne pas effacer une écriture suspecte, restaurer un fichier tiers, nettoyer cargo-shared ou faire un reset global. Retour arrière par examen du diff/commits et uniquement sur changements attribuables/chemins autorisés.

Remise éventuelle T8 : suivre le profil réellement compilé et les AGENTS. Pour personal-install, desktop avec seuls workers Turbo CPU/GPU adjacents ; workers Parakeet, modèles et DLL depuis l'installation stable, présence et SHA-256 vérifiés avant remise. Pas de copies que ce profil n'ouvre pas. Les ressources nécessaires sont des prérequis, pas une autorisation de reconstruire d'autres crates. Si elles manquent, arrêter la seule remise concernée et demander la décision utile. Aucun paquet autonome ni qualification native déduit des builds desktop.

L'utilisateur observe rendu, activation des commandes, dialogues, micro/textes, historique et fermeture. Si cette observation n'a pas lieu, NOT RUN et réserve explicite ; le reviewer de code ne la remplace pas. Aucun binaire ni fenêtre remis/ouvert dans cette mission documentaire.

À la clôture future : archiver les petites preuves avec arborescence, contrôler nombre/SHA-256 selon AGENTS ; ne retirer que copies/temporaires d'un manifeste de nettoyage précisément approuvé. Relever espace final, pas de nettoyage cargo-shared ni nouveau manifeste de qualification. Rapporter commandes/résultats réellement obtenus, distingués des propositions.

## 10. Revue et état du plan

Statut initial : candidat à revue **PLANS**, sur cette version Git contre le design parent. Vérifier couverture AC-01–11, chemins, prérequis, dépendances et commandes. Le rédacteur ne déclare pas son propre CLEAN. Deux cycles revue/correction maximum ; au-delà LIVRÉ AVEC RÉSERVES, constats ouverts et décision utilisateur.

La mission actuelle s'arrête aux documents et commits locaux. Un verdict PLANS CLEAN ne vaut pas GO code/campagne/recette/push. Toutes vérifications produit proposées restent NOT RUN.

### Verdict indépendant et bilan documentaire

Reviewer `/root/cadrage_ui/review_ui`, indépendant de la rédaction/correction du plan, lecture seule : **PLANS CLEAN** sur `68639422ef37cd79f7c727ea1587d84f10dd5e2e`, contre DESIGN technique `9dabf0f2` et verdict consigné dans `82ae582c`. Fichier Git inchangé avant/après revue ; aucun REQUIRED ouvert ni suggestion, aucun cycle de correction PLANS consommé. Le reviewer confirme la couverture bidirectionnelle CA-01–06/AC-01–11, les huit tâches, le raccord atomique T4, la baseline sur production inchangée et la séparation des autorisations futures. Les tests nommés et les cinq fichiers d'intégration existent. UI-D01/UI-D02 restent fermés.

Statut courant : **PLANS READY documentaire**. Inscription du verdict uniquement, sans modification technique du plan revu. Limite résiduelle d'exécution : garde UI à préparer sous accord précis et conformité à démontrer avant tout Cargo ; aucun transfert de dérogation ni GO de campagne.

Vérifications réellement exécutées pour cette livraison : lectures/analyse statique, git diff --check (travail, index et comparaison depuis la base), contrôle de la liste des fichiers modifiés, commits locaux et revues documentaires indépendantes. Seuls `docs/cadrage/refacto-ui.md` et `docs/cadrage/plan-refacto-ui.md` changent depuis la base produit ; non suivis préexistants préservés. Aucun code, test, build, Cargo, moteur natif, fenêtre Windows ou push. Espace final relevé : **70 596 972 544 octets** libres sur C, environ 65,75 Gio ; aucun nettoyage effectué.

### Tentative d'exécution du 31 octobre 2026 — arrêt en T1

Autorisation de cette session : « Je confirme T1–T7, l’adaptation de la garde UI, les campagnes prévues, l’implémentation déléguée et la revue indépendante, puis le bilan T8 et les commits locaux, sans remise de binaire ni push. Lance. » Cette autorisation remplace la limite documentaire pour ces actions précises ; le design et le plan techniques restent inchangés.

Base d'exécution : `95fd9687ff6fa5d2d6bbd02a3d99dffcffb793a3`, branche `evo-startup-perf`. Implémenteur délégué : `/root/implement_ui`, mode IMPLEMENT. Les non suivis `docs/architecture/`, `models/` et `sound example/` sont préservés. Espace avant délégation : **64,56 Gio** ; espace final rapporté par l'implémenteur : **64,47 Gio**.

- T1 : garde adaptée dans `target/parakeet-tests/refacto-ui/guard.ps1`, cache hors ligne copié sous la racine UI, analyse syntaxique PowerShell réussie. Aucune tolérance Temp historique reconduite ; contrôle supplémentaire de `WHISPER_DESKTOP_TEST_ROOT` pour refuser une destination externe.
- V0 réellement exécuté : `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode SelfTest`, **FAIL, exit 1**. Événement `Created` enregistré pour `C:\Users\cyril\AppData\Local\Temp\etilqs_AeH9uTaTdcWausw`, horodatage du journal `2026-10-31T04:44:10.2707828+01:00`. Origine indéterminée ; aucune attribution à Cargo ou à la campagne démontrée. Journal conservé : `target/parakeet-tests/refacto-ui/logs/guard-events.jsonl`.
- T2 : inventaires statiques fichiers/API/tests/assertions conservés dans la racine UI (`baseline-files.txt`, `baseline-public-api.txt`, `baseline-tests.txt`). Aucune baseline exécutée par Cargo ; la référence comportementale reste à établir.
- T3–T7 : **NOT RUN** ; aucun changement du code produit ou des tests. Aucune revue de candidat Rust puisque le candidat n'existe pas. Aucun build, moteur natif, remise de binaire, ouverture de fenêtre ou push.

La garde a déclenché l'arrêt ; aucun nouvel auto-test ni Cargo n'a suivi. Fichier signalé, cache, temporaires et journaux ne sont ni effacés ni restaurés. Les éléments d'exécution restent dans la racine UI ignorée par Git ; seul ce bilan documentaire est commité. L'archivage de clôture de campagne n'est pas annoncé effectué : la campagne reste arrêtée en T1.

Décision nécessaire avant reprise : conserver l'arrêt pour rechercher l'origine de l'événement, ou autoriser précisément une dérogation limitée au traitement des événements Temp non attribués. Recommandation : examiner d'abord les éléments disponibles en lecture seule ; aucune dérogation d'une autre campagne ne s'applique automatiquement. La refonte UI n'est pas livrée.

### Reprise autorisée du 31 octobre 2026

À la demande précise « Autorisez-vous une dérogation limitée à cette campagne pour consigner les événements Temp non attribués sans arrêter, tout en maintenant l'arrêt pour les écritures attribuées hors racine et les autres violations ? », l'utilisateur répond : « oui je t'autoriser ».

Cette dérogation concerne seulement la campagne UI : un événement observé dans le Temp utilisateur sans attribution démontrée est conservé comme `UNATTRIBUTED` sans provoquer à lui seul l'arrêt. Il ne devient ni une écriture Cargo prouvée ni une écriture autorisée. Aucune destination applicative volontaire hors des racines UI et Cargo partagée n'est autorisée. Une écriture attribuée hors racine, une autre violation du préflight ou une modification inexpliquée d'un emplacement sensible impose toujours l'arrêt. La surveillance ne constitue pas un confinement préventif et son absence de PID ne prouve pas l'origine des événements.

Premier échec et éléments conservés ; nouveaux auto-tests requis avant Cargo. Base de reprise `bf5a54a47e57dc5cffbd6c8dfa5f6d682a00181c` (bilan documentaire seulement depuis la base précédente), mêmes chemins et responsable d'implémentation. Espace relevé avant reprise : **64,12 Gio**. Aucun transfert général de dérogation ni changement du design MVC.

### Résultat de reprise — arrêt en T2

- T1/V0 : **PASS, exit 0**. Auto-tests des variables/racines/jonctions et des événements simulés : Temp non attribué consigné `UNATTRIBUTED` ; Temp attribué, autres chemins externes et renommages transfrontaliers refusés. Premier échec conservé.
- T2/V2 exécuté sous garde : `& .\target\parakeet-tests\refacto-ui\guard.ps1 -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-desktop','--lib','--tests','--','--list')`. **FAIL, exit 1**, après environ 24 secondes : événement `Changed` sur `C:\dev\whisper\.git\objects\pack\tmp_pack_rHYElW`, horodatage du journal `2026-10-31T04:46:26.7047987+01:00`, détection `2026-10-31T04:46:26.8519485+01:00`. La garde rapporte l'arrêt de l'arbre de processus. Aucun inventaire exécuté complet ni résultat de test obtenu.
- Journaux conservés : `target/parakeet-tests/refacto-ui/logs/guard-events.jsonl` et `guard.jsonl`, identifiant d'exécution `20261031T044603839-3a847c9b` ; sorties de compilation incomplète conservées.
- Inspection ultérieure en lecture seule : un processus `git.exe` était présent, puis absent lors du relevé suivant. Cela ne démontre ni son lien avec le fichier observé ni l'origine exacte de l'événement. Aucune configuration Git modifiée et aucun processus Git arrêté par la session principale.

La dérogation Temp ne couvre pas `.git`. T2 reste incomplet ; T3–T7 non exécutés, aucun code ni test produit modifié. Espace final rapporté : **63,69 Gio**. Aucun nettoyage/restauration, aucune relance Cargo après l'arrêt. Le bilan documentaire est commité localement ; pas de push. Arbitrage nécessaire avant reprise ; option recommandée : autoriser une nouvelle tentative après disparition de l'activité Git observée, sans tolérance supplémentaire pour `.git`.

### Nouvelle tentative autorisée après l'arrêt T2

L'utilisateur répond « oui je t'autorise » à la demande de reprise après disparition du processus Git observé, sans élargir la dérogation. Les contrôles sur `.git` et les autres emplacements sensibles restent donc inchangés ; l'origine du précédent événement demeure indéterminée. Même responsable et périmètre T1–T7 puis bilan T8/commits locaux ; pas de remise de binaire ni push. Ce bilan d'autorisation est commité avant de relancer la garde pour ne pas provoquer nous-mêmes d'écritures Git concurrentes à la campagne. Prochaine action : préflight, puis nouvelle tentative de baseline sous garde avant toute modification de production.

### Baseline réussie, puis arrêt au contrôle T3

Après disparition du processus Git observé, V2, V3 et V4 ont terminé avec **exit 0**, production inchangée. Par défaut : **81 tests réussis et 1 ignoré préexistant** ; profil `personal-install` : **82 réussis et 1 ignoré préexistant**. Avertissements préexistants conservés dans les journaux, sans revendication de Clippy réussi. Espace avant refactorisation : **62,57 Gio**.

Le découpage T3 des tests a ensuite été effectué, mais son contrôle de compilation desktop a échoué avec **exit 101, 21 erreurs**, sans fuite signalée. Diagnostic de l'implémenteur : l'attribut `#[derive(Clone, Default)]` de `RecordingApplication` est resté à la fin de `ui/tests/import_and_lifecycle.rs`, alors que la structure a été déplacée dans `ui/tests/support.rs`. Erreurs E0599 et E0614 en cascade rapportées ; leur fermeture reste à vérifier après correction.

Correction proposée dans les seuls chemins P-T : déplacer cet attribut au-dessus de la structure, puis reprendre le contrôle T3 sous garde ; examiner les diagnostics résiduels et poursuivre T4–T6 seulement après succès. Le candidat et les journaux sont conservés ; aucun reset ni nouvelle commande Cargo après l'arrêt. Le raccord MVC T4, la validation consolidée T6 et la revue indépendante T7 restent non exécutés. Arbitrage demandé en application du §9 (arrêt sur échec test/build), sans nouvelle dérogation de chemins.

### Autorisation de corriger les erreurs de raccord

L'utilisateur répond « oui je t'autorise » à la demande précise : « Autorisez-vous la correction des erreurs de raccord liées aux déplacements, avec relance des contrôles jusqu'à réussite, puis poursuite T4–T7 ? » La demande précisait que cela permet de traiter ces erreurs ordinaires sans nouvelle confirmation à chaque essai, les autres motifs d'arrêt restant applicables.

Pour cette refonte et dans les seuls chemins prévus, les erreurs de raccord introduites par les déplacements peuvent donc être corrigées puis les contrôles affectés relancés jusqu'à réussite. Cette décision aménage l'arrêt du §9 pour ce seul cas ; elle ne dispense d'aucune validation, n'élargit ni les chemins ni les fonctionnalités et ne couvre ni une fuite d'écriture, ni une modification inexpliquée, ni un défaut fonctionnel indépendant. Les corrections de revue T7 restent soumises à la limite de deux cycles. Le bilan est commité avant la reprise des commandes gardées.

### Candidat MVC intermédiaire — finitions et décision V7

Après correction du derive et des fins de ligne, T3 passe : **81 tests réussis, 1 ignoré préexistant**. Le raccord MVC est compilé ; façade explicite, modèle privé, contrôleur/vues descendants et présentation répartis dans les chemins du plan. Rapprochement statique : **39 symboles publics**, **40 tests UI**, mêmes comptes d'assertions (107 `assert`, 79 `assert_eq`, 1 `assert_ne`, 45 `matches`), aucun nouvel ignore UI. Inventaire exécuté V2 : **82 noms correspondants** après renommage prévu du préfixe. Maximum physique UI après formatage : **567 lignes**.

Commandes du §7 exécutées depuis `C:\dev\whisper`, avec les variables prévues et la garde UI : V1 format PASS (`20261031T045541772-0a0a7267`) ; V2 inventaire PASS (`20261031T045613655-4a260657`) ; V3 défaut PASS, **81 réussis/1 ignoré hérité** (`20261031T045628036-81592e22`) ; V4 personnel PASS, **82 réussis/1 ignoré hérité** (`20261031T045632624-8b77e480`) ; V5 build défaut PASS (`20261031T045546531-58611bfc`) ; V6 build personnel PASS (`20261031T045642002-cef92069`) ; V8 Clippy structurel exit 0 avec avertissements (`20261031T045645340-fda82342`) ; V9 `git diff --check` PASS. Journaux sous `target/parakeet-tests/refacto-ui/logs/`. Aucun événement hors racine bloquant signalé pour ces commandes.

V7 strict : **FAIL, exit 101**, run `20261031T045723893-798bc559`, dix erreurs dans `whisper-core` inchangé, dont import `SAMPLE_RATE` dans `parakeet_session.rs:6` et `large_enum_variant`. Core est hors périmètre ; aucune correction ni neutralisation de lint effectuée. Ce contrôle s'arrête avant de qualifier le desktop : il ne prouve pas l'absence de nouveaux avertissements UI. Les cinq avertissements usuels du build UI de référence sont encore présents dans le build final, sans que ce seul compte suffise à qualifier tous les lints.

Exigence encore inachevée du design §6 : découper sémantiquement les fonctions longues. V8 relève notamment `prepare_frame` et `active_job_ui` (111 lignes), `history_ui` (126), `settings_ui` (206), et les fonctions de microphone/unifié/capture (214/302/308). Le respect de la taille des fichiers ne ferme pas cette exigence. T4/T5 ne sont pas déclarés terminés ; candidat non soumis à T7. Finitions attendues dans les seuls fichiers prévus, en conservant ordre, closures/scopes et effets.

Arrêt après V7 ; modifications UI et preuves conservées, aucun changement core/adapters/manifeste/Cargo.lock, pas de revue consommée, pas de validation Windows ni remise de binaire. Espace final rapporté : **62,55 Gio**. Décision nécessaire : autoriser la poursuite des finitions prévues et conserver V7 en réserve héritée hors périmètre, puis contrôles affectés et revue T7 ; aucune revendication de V7 PASS.

### Disposition V7 et autorisation des finitions

L'utilisateur répond « oui je t'autorise » à la demande précise : « Autorisez-vous à terminer ce découpage, refaire les contrôles affectés, puis poursuivre la revue et les commits en conservant l'échec Clippy strict comme réserve explicite ? » Il était précisé que cela n'autoriserait aucune modification de `whisper-core` ni neutralisation de ses avertissements.

V7 demeure donc FAIL sur le candidat mesuré, avec disposition utilisateur autorisant la poursuite et la livraison assortie de cette réserve. Aucun PASS n'en est déduit, aucun correctif hors périmètre ni `allow` de contournement n'est autorisé. Les finitions sémantiques déjà prévues restent obligatoires, ainsi que les vérifications affectées et la revue indépendante T7. Les autres motifs d'arrêt et la limite de deux cycles de revue/correction restent applicables. Le présent accord est commité avant reprise sous garde.
