Avancée : cadrage du découpage de `application.rs` établi sur les sources actuelles.
Blocage : aucun pour la revue documentaire ; implémentation et campagnes non autorisées.
Décision : extraction progressive par responsabilités, contrats inchangés et propriétaire d’état unique.

# Cadrage — refactorisation de l’application du cœur

- ID : D-REF-APP-CORE, version 1.0, 30 octobre 2026.
- Statut : CANDIDAT POUR REVUE DESIGN ; acceptation finale et autorisation d’implémentation distinctes.
- Auteur des documents : session Codex principale.
- Contributions : `rust_product_framer` — BF-APP-CORE v1 ; `rust_requirements_analyst` — RA-APP-CORE v1 ; `rust_domain_architect` — DOM-APP-CORE v1 ; `rust_architect` — ARCH-APP-CORE v1. Contributions proposées en lecture seule, sur la même base Git, assemblées par l’auteur principal ; cette synthèse ne prétend pas en être une copie exacte.
- Base inspectée : `30632da12a1eb0cf7b50465ee8ec30cd8174fa3f`, branche `evo-startup-perf`.
- Autorité : demande « découper application.rs dans whisper-core. Même méthode pour handle_event (1 121 lignes) et dispatch (934) », qualification PASS puis « ok fais le cadrage puis redige le plan de refacto avec les agents dediés ».
- Portée de cet accord : cadrage, plan, revues documentaires et versionnement local par l’auteur principal ; aucun code produit, campagne Cargo, compilation, test, remise de binaire ou push.
- Plan associé : `docs/cadrage/plan-refacto-application-core.md`, à produire après revue du présent design.

## 1. Besoin, périmètre et sources

Réduire le risque de régression et le coût des prochaines évolutions de transcription fichier et microphone en découpant progressivement `application.rs`, particulièrement `handle_event` et `dispatch`, selon leurs responsabilités. Les utilisateurs Windows retrouvent les mêmes commandes, messages, textes, fichiers, erreurs et règles d’arrêt ; les mainteneurs disposent de points de routage lisibles et de handlers attribués.

Aucun gain de vitesse, mémoire ou qualité audio n’est promis. Le périmètre comprend le découpage interne du fichier et ses helpers nécessaires ; aucune correction fonctionnelle, migration des exceptions de diagnostic/horodatage, nouvelle dépendance ou refonte d’un autre module n’est incluse.

Sources primaires : AGENTS racine/core, `crates/whisper-core/src/application.rs`, son arbre de tests, `lib.rs`, `ports.rs` et manifestes. La méthode précédente est documentée dans `docs/cadrage/refacto-worker-ipc.md`, son plan et son audit final ; leurs chiffres, arborescence et autorisations ne sont pas transférés. Les cartes `docs/architecture/` sont des relevés historiques confrontés aux sources.

| Élément | Base observée statiquement | Limite |
| --- | --- | --- |
| `application.rs` | 4 517 lignes physiques | Pas une mesure Clippy |
| `handle_event` | 796–1916, 1 121 lignes | Corps à décomposer, pas seulement déplacer |
| `dispatch` | 3412–4345, 934 lignes | Corps à décomposer, préambules conservés |
| `AppCommand` | 30 variants | `StartLive` ancien demeure refusé |
| Tests privés | `application/tests.rs` et quatre fragments, 44 déclarations `#[test]` | Aucun test découvert ou exécuté par Cargo |
| Manifestes | Rust 2024, minimum 1.98.1, huit crates, core : `serde =1.0.228` | Versions déclarées ; présence locale non exécutée |
| Git | Aucun changement suivi ; `docs/architecture/`, `models/`, `sound example/` non suivis | À préserver et recontrôler avant exécution |

Les cartes indiquant encore 7 550 lignes et des tests inline sont périmées. L’espace libre relevé au démarrage est d’environ 83,1 Gio ; ce relevé ne remplace pas le préflight de chaque lot futur.

## 2. Capacités, exigences et acceptation

Les capacités BF-01 à BF-08 sont incluses uniquement comme non-régression. BF-09 — nouvelle installation, intégration OS, tray, nouveaux réglages — reste hors mission et demande une demande produit distincte. BF-10 — nouvelle fonction, autre modèle, évolution de protocole, migration diagnostic/temps ou refonte extérieure — est exclue de cette refactorisation.

Chaque AC comportemental s’évalue par faux `ImportIoPort` en mémoire : mêmes commandes et événements, mêmes effets dans leur ordre, erreurs et vues après chaque action. Aucun masquage global des messages, erreurs ou champs ne doit dissimuler une divergence.

| Brief → REQ / UC / AC | Scénario et résultat observable |
| --- | --- |
| BF-01 → REQ-01 / UC-01 / AC-01 : import | Démarrage admissible, préparation, segments, accusés, fin et publication : mêmes effets et vues ; refus sans effet de traitement ; publication uniquement après conditions et accusés actuels ; refus de publication jamais présenté comme succès. |
| BF-02 → REQ-02 / UC-02 / AC-02 : file, doublons, reprise, snapshot | L’ajout n’est annoncé durable qu’après événement attendu et ne démarre pas seul un worker ; doublon périmé inoffensif ; annulation libérant seulement la nouvelle source ; reprise explicite et protections de récupération inchangées. |
| BF-03 → REQ-03 / UC-03 / AC-03 : microphone | Mêmes phases capture/moteur, compteurs et états en live/direct/unifié ; arrêt avant disponibilité ou après capture conservé ; `Ready` tardif sans redémarrage ; finalisation CPU déclenchée explicitement lorsqu’elle l’est aujourd’hui. |
| BF-04 → REQ-04 / UC-04 / AC-04 : Parakeet | Provisoire, confirmé et final automatique distincts ; final toujours non vérifié ; réponse de page périmée sans remplacement de la dernière sélection ; erreur affectant la bonne session ; mêmes bornes de cache. |
| BF-05 → REQ-05 / UC-05 / AC-05 : test micro, gain, mesures | Propriétaire, révisions, identités et invalidations conservés ; ancien événement sans écrasement du contexte récent ; mesures/source invalidées sur échec ; redémarrage après libération existante. |
| BF-06 → REQ-06 / UC-06 / AC-06 : historique, publication, nettoyage | Mêmes effets, indicateurs d’attente, résultats, messages et erreurs ; traitements transverses maintenus à leur position, avant le filtrage actif lorsque prévu. |
| BF-07 → REQ-07 / UC-07 / AC-07 : refresh et fermeture | Au plus quatre événements par refresh avec sorties anticipées actuelles ; admission interdite pendant fermeture sans effet ; `close_ready` seulement après tous états, retraits et accusés actuellement requis. |
| BF-08 → REQ-08 / UC-08 / AC-08 : compatibilité | Mêmes chemins publics, signatures, traits, types échangés et représentations ; aucune dépendance, feature, modification de manifeste, lockfile ou protocole. |
| BF-08 et CA-01/05 → REQ-09 / UC-09 / AC-09 : structure | Chaque branche et helper attribué ; état unique ; dispatchers délégant réellement ; façade finale déclarative ; témoins et métriques par extraction, écarts hérités nommés. |

REQ-01 à 07 couvrent les CA-02/03 de qualification ; REQ-08 couvre CA-04 ; REQ-09 couvre CA-01/05. Les tâches du futur plan renvoient à ces IDs.

## 3. Domaine et invariants à préserver

L’état métier (`JobState`, identités/générations) reste distinct de l’état opérationnel d’admission/persistance et de la projection `AppView`. Les captures unifiées et le final automatique conservent leurs identités et états dédiés ; ils ne deviennent pas un travail actif unique artificiel.

| ID | Invariant | Exigences |
| --- | --- | --- |
| DOM-01 | Décisions et mutations sur l’instance unique `ImportApplication<P>`, sans état parallèle dans les handlers. | REQ-08/09 |
| DOM-02 | L’ordre du routage et des traitements transverses d’événements est sémantique. | REQ-01 à 07 |
| DOM-03 | Événement étranger ignoré seulement après les traitements préalables actuels ; exceptions `Stopped`/`Failed` en arrêt et admission d’enqueue conservées. | REQ-02 à 05/07 |
| DOM-04 | Garde fermeture avant admission modèle ; réentrées actuelles par `dispatch` réappliquant ces gardes. | REQ-01/03/07 |
| DOM-05 | `PersistSegment` soumis ne signifie pas durable ; accusés et conditions propres au parcours gouvernent la publication. | REQ-01/03/04/06 |
| DOM-06 | Provisoire, confirmé et final automatique distincts ; pagination et mesures ne modifiant que leur propriétaire/révision courants. | REQ-04/05 |
| DOM-07 | Retours anticipés gardant leur portée : handler, dispatcher ou refresh ; aucune continuation ajoutée après refus, contrepression ou résultat terminal. | REQ-01 à 07/09 |

Transitions sensibles : admission explicite → préparation → moteur prêt → segments/persistance → fin → publication demandée → publication confirmée. File durable seulement après événement d’acquittement. Reprise explicite avec génération suivante. `PortError::Committing` pendant l’arrêt conserve message et finalisation, sans devenir un arrêt réussi.

Pour les captures unifiées, la publication attend l’arrêt soumis, l’absence de publication antérieure, les segments/accusés épuisés et la concordance de dernière séquence. Le final automatique conserve sa file, sa saturation et ses échecs indépendamment du travail actif.

Aucune nouvelle garantie d’atomicité TXT/SRT/WAV, de conservation après panne ou de reprise après interruption brutale n’est ajoutée.

## 4. Décisions techniques et disposition finale

Choix techniques proposés par `rust_architect`, autorité déléguée par la mission de cadrage ; ces choix ne modifient pas le produit.

- DEC-01 : `application.rs` + `application/`, façade finale déclarative ; écarter le simple déplacement du monolithe.
- DEC-02 : état unique, méthodes réparties entre modules privés ; aucun objet de contexte artificiel, trait ou copie d’état.
- DEC-03 : contrats publics, métadonnées, diagnostics historiques et orchestration séparés ; réexports maintenant les chemins actuels, sans changement nécessaire de `lib.rs`.
- DEC-04 : propriétaire central des gardes et chaînes de contrôle ; handlers par familles et branches, sans uniformiser les retours et erreurs.
- DEC-05 : tests privés descendants du propriétaire final d’état ; ni champs publics, ni accès de test dans la production, ni `#[path]`.
- DEC-06 : traces et horodatage historiques déplacés sans correction, avec conditions, littéraux et points d’appel inchangés.

Tous les chemins suivants sont relatifs à `crates/whisper-core/src/`.

| Destination | Attribution exhaustive des symboles et responsabilités actuels |
| --- | --- |
| `application.rs` | Déclarations privées et réexports ciblés seulement ; zéro corps algorithmique de production à la clôture. |
| `application/contracts.rs` | `CommandIdentity`, `AppCommand` et `identity`/`targets`, `AppView`, phases directes, `ApplicationError`, trait `Application` et défaut, `AppFacade` et méthodes ; sources 249–510 et 4465–4514. |
| `application/event_metadata.rs` | Implémentations `ImportEvent::kind_name` et `ImportEvent::identity`, valeurs et exhaustivité conservées. |
| `application/diagnostics.rs` | `record_application_timing`, `trace_application_event`, statiques et littéraux associés ; sources 10–191. |
| `application/runtime/mod.rs` | `ImportApplication<P>`, `UnifiedCaptureAdmission`, `AutomaticFinalJobState`, quatre bornes actuelles, `new`, `with_model_guard`, `fail` et déclarations ; sources 512–629. |
| `application/runtime/commands.rs` | Unique `impl Application for ImportApplication<P>` : `set_gpu_candidate`, `dispatch`, `view` ; gardes fermeture/modèle et routage des 30 variants ; `refuse_model_command`. |
| `application/runtime/events.rs` | Orchestration centrale `handle_event`, filtre d’identité et routage final ; conditions et sorties centrales. |
| `application/runtime/poll.rs` | `poll_once`, borne de quatre événements, traces et sorties de refresh. |
| `application/runtime/closing.rs` | `update_close_ready`, `drive_close_queue_removals`, `RequestClose`, arrêt partagé `Stop`/`Quit`, cas `Committing`. |
| `application/runtime/queue.rs` | Transverses historique/nettoyages/snapshot/file/admission/doublons/échecs de scan-retrait ; commandes correspondantes, résolution de publication et reprise de nettoyage. |
| `application/runtime/import.rs` | `StartImport`, `ProcessQueued`, `ResumeInterrupted` ; cycle actif `ResumeCheckpoint`/`Prepared`/`Ready`/`Progress`/`Segment`/`Persisted`/`End`/`Published`/`Failed`/`Stopped` et asymétries ; `maybe_publish`, `accept_segment`, `retry_pending_segment`. |
| `application/runtime/live.rs` | `StartLiveConfigured`, `StartDirectLiveConfigured`, `FinishLiveOnCpu` ; événements direct/live/provisoires, phases/compteurs/reprises CPU, `WindowFinished`, projections Parakeet actives. Ancien `StartLive` toujours refusé. |
| `application/runtime/microphone.rs` | Commandes test/gain ; événements gain/test/mesures/spectre ; invalidation d’acquisition à sa position actuelle avant filtre d’identité. |
| `application/runtime/unified.rs` | `add_unified_session_card`, `handle_unified_capture_event`, retries unifiés, `maybe_publish_unified_capture` ; démarrage/arrêt unifiés, sélection/lectures document-audio, projections des sessions d’import unifié. |
| `application/runtime/parakeet_views.rs` | `track_parakeet_capture_event`, `find_parakeet_capture_mut`, `trim_parakeet_capture_views`, `append_parakeet_confirmed` et closure de création de capture. |
| `application/runtime/automatic_final.rs` | `track_automatic_final_event`, `set_capture_automatic_final_status`, `mark_automatic_final_failed`, `start_next_automatic_final`, `retry_automatic_final_segment`, `handle_automatic_final_event`, `maybe_publish_automatic_final`. |
| `application/runtime/tests.rs` et `runtime/tests/*.rs` | Déplacement normal des cinq fichiers de tests privés actuels, sans changement de finalité ni migration globale des tests d’intégration. |

Les événements à plusieurs projections restent traités aux positions actuelles ; la carte n’impose pas une consommation exclusive par le premier handler. Les nouveaux handlers portent un nom de responsabilité et restent dans leur famille.

Migration : garder initialement l’état dans `application.rs`, ancêtre des modules extraits, puis déplacer état et tests sous `runtime` à la clôture. Les méthodes appelées depuis l’ancien propriétaire reçoivent seulement `pub(in crate::application)` ; resserrer ensuite à `runtime`. Aucun champ d’état public. Métadonnées et diagnostics : `pub(super)` seulement si nécessaire. Réexporter uniquement les neuf types publics historiques.

## 5. Chaînes de contrôle et contrats

`handle_event` : identifier → router pages/publication unifiées et captures connues → suivre Parakeet par référence → transverses et retours → invalidation acquisition → final automatique séparé → filtre actif avec exceptions arrêt/échec/enqueue → projection session unifiée et invalidation pages → refus de segment après arrêt lorsque prévu → traitement métier.

Un helper transverse peut retourner `Result<bool, ApplicationError>` pour distinguer consommé/quitter de poursuivre. Un simple `Ok(())` ne doit pas devenir implicitement une consommation.

`dispatch` : garde fermeture → admission modèle/refus visible → routage → vue. Préserver les retours précoces ; un handler peut retourner directement `Result<AppView, ApplicationError>` pour éviter continuation ou clonage supplémentaire. Réentrées observées : `RequestClose → dispatch(CancelDuplicateImport)` ligne 3577 et `RequestClose → dispatch(Stop)` ligne 3616.

`poll_once` : retry unifié → retry actif avec sortie si `false` → retry final automatique dont le booléen reste ignoré → quatre polls maximum. Trace avant traitement ; sorties `None`, segments actifs pendants hors unifié, puis `Ready` et appels à `update_close_ready` inchangés.

`ImportIoPort::submit` et `poll` gardent leurs signatures et erreurs ; runtime décide, adaptateurs exécutent. `Backpressure` conserve les traitements distincts par parcours ; `Committing` ne devient pas un arrêt réussi ; `fail` garde son message et `PortUnavailable`. Aucun changement de threads, verrous, processus, FFI, `Send/Sync` ou `unsafe`.

## 6. Dépendances et contrôles structurels

| Consommateur | Dépendances permises | Contrôle futur |
| --- | --- | --- |
| Contrats | Domaine et ports actuels | Imports/réexports, aucun adaptateur concret |
| Runtime | Contrats, domaine/politiques, `ImportIoPort` | Imports et manifestes inchangés, aucune UI |
| Handlers | État unique et helpers internes | Pas d’état parallèle, accès système nouveau ou support de tests |
| Diagnostics historiques | Exceptions actuelles | Corps, littéraux, conditions et points d’appel comparés |
| Tests | Faux port mémoire, API et invariants privés descendants | Pas de champs ou branches de production créés pour les tests |

Cibles finales : zéro corps algorithmique dans `application.rs`, un propriétaire d’état, tous les symboles attribués, 30 dispositions de commande préservées, fichiers nouveaux ≤1 500 lignes de production. Handlers nouveaux : cibles Clippy 100 lignes, complexité 25, sept arguments, complexité de type 250.

`trace_application_event` et `ImportEvent::identity`, copiés sans restructuration, sont des exceptions héritées à mesurer et nommer : leur taille physique dépasse 100, sans préjuger du comptage Clippy. Les gros handlers unifiés/Parakeet/final automatique sont réellement décomposés en routage et branches dans leur famille.

Contrôles proposés : inventaire `rg`, comparaison Git des corps déplacés et littéraux, revue des dispositions, lignes physiques et absence de `#[path]`. Clippy NOT RUN ; aucune baseline ou CI nouvelle.

## 7. Vérification, risques et séquence du futur plan

Témoins existants à réutiliser : contrats import/live/scheduler/modèle ; fragments `closing_and_recovery`, `selection_and_microphone`, `live_lifecycle`, `parakeet_sessions`. Notamment gardes d’admission avant effet, matrices d’import, pagination suivant la dernière sélection, fermeture refusant les admissions indirectes et attente d’une publication déjà engagée.

Le plan précise commandes, données, résultats attendus, chemins, prérequis et limites. L’horodatage système historique exige une comparaison adaptée de présence/position et bornes ; aucune égalité artificielle entre deux exécutions, aucun masquage global.

Séquence technique proposée, dix tranches séquentielles sans parallélisation des mêmes dispatchers :

1. Témoins utiles, contrats, métadonnées et diagnostics.
2. File et transverses.
3. Microphone.
4. Live/direct.
5. Import et persistance.
6. Unifié.
7. Vues Parakeet.
8. Final automatique.
9. Fermeture/poll et clôture des dispatchers.
10. Transfert final état/tests, visibilité, façade et bilan global.

Chaque extraction fait décroître le résidu racine. Le plan writer détaille les budgets, invariants fermés, métriques et contrôles sans ajouter de décision structurante.

| Risque | Couverture |
| --- | --- |
| RISK-F01 / T01 : gardes, retours ou effets réordonnés | DOM-02/04/07, AC-01/06/07, témoins et inspection des chaînes |
| RISK-F02 : publication/fermeture prématurée | DOM-05, AC-01/03/07, séquences d’accusés |
| RISK-F03 : confusion identité/session/révision | DOM-03/06, AC-02 à 05 |
| RISK-F04 : témoins lacunaires | REQ-09, inventaire des branches et caractérisations utiles |
| RISK-T02 : accès privés des tests cassé | Tests descendants et compilation gardée |
| RISK-T03 : second monolithe | Carte exhaustive, routage et métriques finales |
| RISK-T04 : exemptions historiques transférées | Prérequis EXECUTION non satisfait et règle permanente |

## 8. Readiness, autorisations et limites

- DESIGN_FEASIBILITY : étayée statiquement. Extraction interne, réexports et accès descendants sans nouvelle pile. Les builds futurs vérifieront la réalisation ; aucun SPIKE requis pour cette faisabilité structurelle.
- PRODUCT_VALIDATION : NOT RUN. Tests mémoire et contrats futurs ; ils prouvent l’orchestration, pas l’écriture physique TXT/SRT/WAV ni le moteur natif.
- DELIVERY_QUALIFICATION : NOT RUN. Moteurs natifs et fenêtre Windows distincts ; aucune remise de binaire prévue.

La garde observée `target/parakeet-tests/refacto-worker-ipc-f3/guard.ps1` est liée à ses racines et dérogations historiques. Elle n’est pas exécutable telle quelle pour ce chantier. Une adaptation limitée de l’existant, des racines autorisées et des exemptions doit être précisément autorisée avant campagne. Aucun nouveau dispositif ou surveillance n’est demandé.

Avant tout Cargo futur : espace ≥30 Gio, racines canoniques et environnement requis, self-tests et surveillance conformément à AGENTS. Aucune invocation directe ni dérogation importée. Ce prérequis EXECUTION non satisfait bloque les campagnes, pas la revue documentaire.

Aucun railguard supplémentaire, manifeste G1 ou journal de qualification n’est proposé. Aucune question produit nouvelle bloquante. Le reviewer est seul propriétaire du verdict DESIGN ; CLEAN ne vaut ni autorisation de coder ni validation produit.
