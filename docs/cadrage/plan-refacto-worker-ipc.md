Avancée : plan proposé en treize lots bornés, suivant l'ordre 3a–3f demandé.
Blocage : aucune exécution autorisée ; l'acceptation du cadrage et l'accord précis du lot restent requis.
Décision : une seule famille d'effets par lot ; aucun changement de comportement entre les extractions.

# Plan de refactorisation — worker_ipc

## Header

- ID : P-REF-WORKER-IPC, v1.0 ; date : 29 octobre 2026.
- Source audit : indications « Étape 3 » fournies dans cette session, complétées par inspection ciblée ; aucun audit global ni mesure Clippy nouvelle revendiqués.
- Design : [refacto-worker-ipc.md](refacto-worker-ipc.md), D-REF-WORKER-IPC v1.0, proposé à acceptation.
- Target workspace : `C:\dev\whisper`, crate `whisper-adapters`, branche `evo-startup-perf`.
- Base produit : `939468382dacfff1c119c8acd79f08b3b7385392`.
- Planning mode : plan only, no code changes.
- Confidence : medium — responsabilités et tests repérés ; compilation, conditions de fixtures et faisabilité d'une campagne stricte non vérifiées.
- Plan quality check : passed documentaire ; revue indépendante CLEAN sur `704cb179775979d7217024bbdfeba5539308c0d7`, sans assimilation à l'acceptation utilisateur.
- Owner : auteur documentaire = session principale ; implémentation future = `rust_implementer` ; revue de candidat = `rust_review_coordinator` et reviewer indépendant.

## 1. Audit Digest

Le fichier actuel compte 13 214 lignes physiques. `effect` commence à 6 305, `poll_unified_sessions` à 4 719, `poll` à 10 958. Le transport n'est pas une plage autonome à déplacer aveuglément : `spawn_live_attempt`, réservation d'essai et remapping live s'intercalent entre ses fonctions. `PublicationGate` est une petite machine atomique. Le service comporte des helpers d'exports automatiques qui doivent aussi recevoir un propriétaire.

L'indication de complexité 101 appartient à la demande, sans remesure. Aucun classement de gravité issu d'un audit externe n'est inventé. Les risques principaux sont les sorties précoces de `effect`, les compteurs/ACK de persistance, le filtrage d'identités, la saturation et les règles d'arrêt/repli. Le gain rapide est l'isolement du framing et de la gate, avant les responsabilités couplées.

Carte d'architecture et contrats normatifs de la proposition : cadrage §3–6. Le domaine reste dans core ; tous les déplacements sont internes aux adaptateurs. La façade publique reste `worker_ipc`. Pas de couche décorative ni d'interface nouvelle entre crates.

## 2. Freshness Check

Inspection statique de `worker_ipc.rs`, son arbre de tests, `lib.rs`, manifestes, AGENTS racine/adapters, carte et fiche architecture. Tests déjà séparés : `worker_ipc/tests.rs` inclut sept fragments ; supports supplémentaires présents. Pas de nouveaux `ipc/`, `publication/`, `service/`, `runtime/`, `backends/`, `parakeet_unified/` dans ce sous-arbre à la base inspectée.

Les anciens plans `plan-refacto-tests-diagnostics.md` et cadrage associé confirment un chantier antérieur de séparation des tests ; aucune de ses autorisations ni dérogations de campagne n'est transférée. Pas de brouillon de refactorisation worker_ipc trouvé dans `docs/cadrage/`.

État initial : aucun changement suivi ; non suivis `docs/architecture/`, `models/`, `sound example/` préservés. Espace libre initial environ 115 Gio. Recontrôler sources, symboles, tests, Git, responsables concurrents et espace avant chaque lot. Le nombre de tests exécutables et les métriques Clippy sont à établir lors d'une campagne autorisée ; ne pas recopier de compte historique comme baseline actuelle.

## 3. Planning Assumptions

- Safe : l'ordre de la demande est conservé ; 3d comprend exactement cinq familles successives.
- Safe : aucune nouvelle dépendance, feature, modification du protocole, de core, des autres adaptateurs ou de desktop n'est nécessaire ; une découverte contraire suspend le lot affecté.
- Safe : les méthodes sont extraites sur le même état ImportRuntime ; pas de copie de ressources, remplacement de mécanisme concurrent ni généralisation de traits.
- Validation needed : la visibilité privée requiert des ajustements internes. Préférer `pub(super)` ou `pub(in crate::worker_ipc)` et imports explicites ; ne pas ouvrir l'API publique.
- Validation needed : les corps longs déplacés gardent une dette héritée attribuée ; tout nouveau fichier reste ≤1 500 lignes de production. Les cibles fonctions/complexité/arguments/types du dépôt sont examinées à chaque extraction.
- Decision gate : cadrage/plan acceptés, lot/chemins et campagne précisément autorisés avant tout changement produit/Cargo. Ces contrôles viennent des AGENTS, sans nouvelle gate de qualification.
- Decision gate : aucune garde compatible et autorisée pour ces nouveaux lots n'a été établie ici. Le script historique observé contient des racines et exceptions propres à un autre lot. Son adaptation éventuelle nécessite un accord précis ; elle n'est ni réalisée ni implicitement autorisée par ce plan.

### Convention et portée des tests

Convention `worker_ipc.rs` + sous-dossiers avec `mod.rs` et enfants, sans `#[path]`. `lib.rs` reste inchangé. Les reexports publics existants conservent leurs signatures et chemins ; nouveaux modules privés.

Les tests existants restent dans leur arbre actuel : migration des scénarios vers des tests d'intégration différée pour éviter de modifier leur accès privé et leur découverte à la fois que le code. Aucun ajout de volume inline. Les tests privés existants peuvent importer des helpers avec visibilité limitée à `worker_ipc` ; cela ne justifie aucune visibilité crate/pub supplémentaire. Nouvelle caractérisation privée uniquement si nécessaire à un invariant inaccessible via l'API publique, dans `worker_ipc/<responsabilité>/tests.rs` ; exception motivée dans le lot. Nouveaux scénarios publics éventuels : `crates/whisper-adapters/tests/worker_ipc_transport.rs`, `worker_ipc_service.rs` ou `worker_ipc_workflows.rs`, avec support sous `tests/support/worker_ipc/`. Ces chemins sont proposés, sans obligation de créer des tests redondants.

## 4. Finding Traceability

| Indication / risque | Gravité | Disposition | Tâches | Couverture |
| --- | --- | --- | --- | --- |
| 3a transport, lecture/écriture, threads | structure demandée | Planned | A | REQ-01/03 |
| 3b PublicationGate | structure demandée | Planned | B | REQ-02 |
| 3c AsyncImportIo/service_loop | structure demandée | Planned | C | REQ-02/03 |
| 3d file, doublons, direct, reprise, import | structure demandée | Planned | D1–D5 | REQ-02–05/07 |
| 3e poll et poll_unified_sessions après handlers | structure demandée | Planned | E1–E2 | REQ-01–05 |
| 3f backends et parakeet_unified en dernier | structure demandée | Planned | F1–F3 | REQ-01/05–07 |
| Sorties précoces et épilogue de effect | risque de régression produit | Planned | D1–D5 | contrôle de flot et témoins avant/après |
| Tests déjà séparés | observation actuelle | Already resolved | tous | conserver arbre et scénario |
| Migration globale des tests / réduction de toute complexité héritée | hors extraction demandée | Deferred | aucun | aucun ajout inline ; dette déplacée identifiée |
| Campagne stricte non établie | contrainte d'exécution | Decision gate | tous | conditions AGENTS §8 |

## 5. Recommended Sequence

Treize lots successifs, une tâche par lot, au plus deux fichiers de compte rendu existants mis à jour : ce plan et le cadrage. Les dix tâches maximum s'apprécient par lot, pas sur la somme de cette feuille de route. Aucun lot suivant ne démarre tant que l'invariant du précédent n'est pas clos et ses vérifications requises terminées. Aucun GO global n'est déduit de cette proposition.

Pour chaque phase ci-dessous, l'objectif, l'invariant fermé, les métriques avant/cible, les commandes de métriques et conditions d'arrêt sont ceux de sa tâche homonyme §6 ; ils constituent le contrat de phase, sans deuxième invariant implicite.

| Phase / lot | Tâche | Objectif et invariant fermé | Avant → cible | Dépend de | Résultat démontrable |
| --- | --- | --- | --- | --- | --- |
| 3a | A | transport autonome | transport dans racine → 0 corps transport dans racine | aucun | mêmes frames et ordre Stop |
| 3b | B | gate isolée | 1 définition racine → 0, 1 définition publication | A | mêmes transitions/atomiques |
| 3c | C | service isolé | AsyncImportIo et boucle racine → 0 corps service racine | B | même admission et publication finale |
| 3d-file | D1 | famille file déléguée | 4 bras dans racine → 4 appels | C | même FIFO et erreurs |
| 3d-doublons | D2 | famille doublons déléguée | 1 bras → 1 appel | D1 | mêmes prompts/confirmations |
| 3d-direct | D3 | famille direct/micro déléguée | 8 bras → 8 appels | D2 | même capture et arrêt |
| 3d-reprise | D4 | famille reprise déléguée | 5 bras → 5 appels | D3 | même restauration/identité |
| 3d-import | D5 | dispatch complet délégué | 12 bras résiduels → 0 corps métier dans match | D4 | mêmes sorties et épilogue |
| 3e-poll | E1 | polling principal isolé | poll dans racine → 0 corps polling principal racine | D5 | même ordre d'événements |
| 3e-unifié | E2 | polling unifié isolé | poll_unified_sessions/document racine → 0 | E1 | mêmes drains/ACK |
| 3f-backends | F1 | primitives moteur isolées | catalogue/routes/worker conservé racine → 0 | E2 | mêmes choix et tentatives |
| 3f-parakeet | F2 | primitives unifiées isolées | sources/exécuteur/transcripteur racine → 0 | F1 | mêmes sessions et récoltes |
| 3f-clôture | F3 | façade finale fine | helpers résiduels racine → 0 corps algorithmique | F2 | API publique compatible |

Les comptes 4+1+8+5+12 portent sur les 30 bras du match principal, non sur ses pré-calculs ni ses closures. À revalider sur la base de chaque lot ; changement du nombre/contrat = mise à jour du plan avant extraction concernée. F3 clôt la même étape 3f, sans nouvelle fonctionnalité.

## 6. Task Backlog

### Contrat commun obligatoire de chaque tâche

Tous les chemins abrégés ci-dessous sont sous `crates/whisper-adapters/src/worker_ipc/`, sauf `racine` = `crates/whisper-adapters/src/worker_ipc.rs`. Les chemins proposés sont autorisables, pas déjà autorisés. Chaque tâche couvre une seule extraction structurale, conserve REQ-01–07 et le comportement sur sa tranche.

- Owner : rust_implementer après GO précis ; la session principale ne code pas en parallèle. Ne pas annuler les changements d'autres tâches ; conflit direct = arrêt.
- Priority : High pour A–E2 (frontières avant moteurs), Medium pour F1–F3 ; ordre de dépendance obligatoire, sans urgence produit inventée.
- Metric commands : M1/M2/M3 §7 avant/après ; revue du diff par symboles pour les métriques non comptables par regex. Pas de métrique de performance inventée.
- Stop conditions : baseline en échec, critère de la tranche partiel, changement de comportement, visibilité publique requise, conflit avec travail tiers, budget dépassé sans justification prévue, fichier nouveau >1 500 lignes de production, préflight ou campagne en échec. Ne pas poursuivre en cascade.
- Implementation outline commun : établir témoins et inventaire à même configuration ; déplacer les symboles et ajuster uniquement câblage/imports/visibilités internes ; comparer les chemins de retour et mutations ; exécuter contrôles autorisés ; soumettre le candidat au coordinateur de revue ; mettre à jour le bilan dans ce plan.
- Acceptance criteria communs : invariant fermé de la tâche, budget tenu, aucun changement d'API/format/messages/ordre/atomiques/bornes/politique, tests réellement exécutés et limites nommées. Pas de CLEAN autoattribué par l'implémenteur.
- Rollback/fallback commun : avant commit, retirer uniquement le diff propre à la tâche après vérification d'absence de conflit ; après commit, proposer un revert limité au commit du lot, jamais reset/clean général. Si isolation impossible, arrêter avec le diff conservé. Revenir au dernier lot clos, sans commencer le suivant.
- Notes pour l'implémenteur : méthodes sur le même ImportRuntime ; aucune nouvelle architecture métier ; si une signature interne transporte trop d'arguments, préférer méthode utilisant l'état existant à un contexte artificiel.
- Verification commands / provenance : V0, V1–V6 et M1–M3 §7, commandes déduites des sources/manifestes et bornées par la garde ; T-A à T-F §7 selon tâche. Toutes les commandes Cargo restent NOT RUN ici.

Les champs ci-dessous complètent ce contrat commun, qui s'applique à chacune des treize tâches.

### A — Extraire le transport (lot 3a)

- Source finding / rationale : 3a, REQ-01/03 ; isoler la frontière enfant avant le runtime.
- Target files/modules : racine, `ipc/mod.rs`, `ipc/framing.rs`, `support.rs` ; tests existants `tests/reprocessing_and_transport.rs`.
- Ownership bucket : infra/adapters transport.
- Scope budget : 4 fichiers production, tests directement correspondants ; exception justifiée par memory_trace partagé entre transport, framing et service, à extraire atomiquement pour préserver la configuration memory sans dépendance transport vers runtime ; aucun changement lib.rs.
- Closed invariant : transport complet possédé par ipc, chemins publics conservés par réexport.
- Before metrics : 1 ChildWorkerTransport et corps framing/lecture/commandes dans racine ; cible ipc absente.
- Target metrics : zéro définition correspondante dans racine ; exactement une définition par symbole dans ipc ; framing sans dépendance runtime/service/archive.
- Metric commands / Stop conditions : contrat commun ; rechercher `ChildWorkerTransport|read_events|read_frame|write_frame|serialize_frame|BoundedFrameWriter|WorkerReaderMessage` dans M1.
- Dependencies : aucune.
- Implementation outline : extraire le struct, ExpectedGpuSession, impls request/receive/Drop, admission de commandes et lecture des événements dans ipc/mod.rs ; framing et MAX_FRAME_BYTES dans framing.rs ; déplacer memory_trace et MEMORY_REPORT avec leurs imports cfg dans support.rs, module privé std/serde_json sans runtime/service/archive. Les compteurs partagés NEXT_IMPORT_JOB_ID et NEXT_INSTANCE rejoignent ce même propriétaire transversal, sans duplication ni changement des ordres atomiques. Les appelants encore dans racine utilisent une liaison interne transitoire. Conserver helpers de tentative live dans racine jusqu'à D3/F1 ; préserver filtres et attestations.
- Tests to add or update : T-A ; conserver noms/logique de tests, imports privés explicites ; caractériser seulement une branche non couverte découverte par inventaire.
- Verification commands : V1–V6, T-A ; provenance §7.
- Acceptance criteria : framing à limite identique, version/EOF/attestation identiques, Stop retenu en saturation, aucune duplication de corps ni élargissement public ; façade encore épaisse intentionnellement.
- Rollback/fallback / notes : contrat commun ; ne pas copier les lignes 584–1 331 en bloc, car helpers de reprise live intercalés.

### B — Extraire PublicationGate (lot 3b)

- Source finding / rationale : 3b, REQ-02 ; isoler l'interverrou sans refaire la publication.
- Target files/modules : racine, `publication/mod.rs` ; tests de gate dans reprocessing_and_transport et resume_and_admission.
- Ownership bucket : adaptateur/publication, primitive de synchronisation existante.
- Scope budget : 2 production + tests correspondants.
- Closed invariant : une seule machine PublicationGate et ses constantes, sous publication.
- Before metrics : 1 définition gate + 5 constantes GATE dans racine.
- Target metrics : 0 définition gate/constantes GATE dans racine, 1 gate + mêmes 5 constantes dans publication.
- Metric commands / Stop conditions : M1 `PublicationGate|GATE_` ; contrat commun.
- Dependencies : A.
- Implementation outline : déplacement littéral des transitions et ordres mémoire, réexport interne ; imports tests sans API externe.
- Tests to add or update : T-B, contrôle état après commit en erreur ; petit test privé séparé publication/tests.rs justifié si absence de témoin de ce dernier état.
- Verification commands : V1–V6, T-B ; provenance §7.
- Acceptance criteria : mêmes résultats stop/commit/rollback ; aucune réactivation implicite après erreur, aucune migration des fonctions d'archive.
- Rollback/fallback / notes : contrat commun ; ne pas simplifier les atomiques ou messages.

### C — Extraire le service asynchrone (lot 3c)

- Source finding / rationale : 3c, REQ-02/03 ; isoler le consommateur de fin de chaîne.
- Target files/modules : racine, `service/mod.rs`, `service/automatic_final.rs` ; tests service existants.
- Ownership bucket : service/application d'adaptateur.
- Scope budget : 3 production + tests correspondants.
- Closed invariant : AsyncImportIo, contrat ServiceRuntime et boucle possédés par service.
- Before metrics : 1 AsyncImportIo, service_loop et impl ImportIoPort dans racine, helpers finaux dispersés.
- Target metrics : 0 corps service dans racine ; un exemplaire dans service ; réexport public AsyncImportIo inchangé.
- Metric commands / Stop conditions : M1 `AsyncImportIo|ServiceRuntime|service_loop|automatic_final|effect_event_reservation` ; contrat commun.
- Dependencies : B.
- Implementation outline : déplacer façade et boucle dans mod.rs ; helpers queue finale/exports/status/stages/cleanup et NEXT_AUTO_EXPORT_STAGE dans automatic_final.rs ; importer NEXT_IMPORT_JOB_ID depuis support.rs pour conserver son partage avec collision_free_job_id, sans copie du compteur ; garder l'impl ServiceRuntime sur le runtime via un câblage minimal ; conserver utilitaires de réservation/identité avec leur appelant. Adapter seulement visibilité interne des fonctions finales encore utilisées par le runtime.
- Tests to add or update : T-C ; pas de réécriture de fixtures concurrentes.
- Verification commands : V1–V6, T-C ; provenance §7.
- Acceptance criteria : capacités et réservations constantes (EVENTS_PER_STAGE = 21 à cette base), Start/Stop/déconnexion et attente 5 ms identiques ; aucune publication automatique contournée ; modules ≤1 500 lignes chacun.
- Rollback/fallback / notes : contrat commun ; ne pas remplacer try_send/try_recv par des appels bloquants.

### D1 — Extraire la file et installer le propriétaire d'état (lot 3d-file)

- Source finding / rationale : 3d file, REQ-03/04 ; première famille indépendante.
- Target files/modules : racine, `runtime/mod.rs`, `runtime/queue.rs` ; tests resume_and_admission et reprocessing_and_transport.
- Ownership bucket : runtime/application d'adaptateur.
- Scope budget : 3 production + tests correspondants ; déplacement d'état/constructeurs nécessaire à la portée commune des futurs impls, sans redéfinition de l'état.
- Closed invariant : les quatre bras de file délèguent à runtime/queue sur le même état.
- Before metrics : Enqueue, RemoveQueued, ProcessQueued, ScanQueue avec corps dans match ; ImportRuntime dans racine.
- Target metrics : 4 bras à appel, 0 corps file dans dispatcher ; 1 ImportRuntime dans runtime/mod.rs, constructeur inchangé.
- Metric commands / Stop conditions : M1 noms des quatre variants + ImportRuntime ; contrat commun.
- Dependencies : C.
- Implementation outline : déplacer état, constructeurs et méthodes d'injection existantes dans runtime/mod.rs ; maintenir alias privé racine pour les impls non encore extraits ; visibilité de champs limitée à worker_ipc lorsque requise par impls/tests historiques ; extraire file, validations FIFO/identité et helpers strictement associés dans queue.rs. Ne pas extraire le corps ConfirmDuplicateImport avant D2.
- Tests to add or update : T-D1 ; cas queue pleine, ordre FIFO, refus GPU avant préparation, admission unifiée refusée sans perte d'entrée durable.
- Verification commands : V1–V6, T-D1 ; provenance §7.
- Acceptance criteria : quatre bras délégués, même source binding/validation/preflight ; préambule/épilogue toujours dans effect et sorties précoces préservées.
- Rollback/fallback / notes : contrat commun ; aucun champ exposé en dehors du sous-arbre worker_ipc.

### D2 — Extraire les doublons (lot 3d-doublons)

- Source finding / rationale : 3d doublons, REQ-04 ; préserver choix utilisateur et revalidation.
- Target files/modules : racine, runtime/mod.rs (déclaration), `runtime/duplicates.rs`.
- Ownership bucket : runtime/application.
- Scope budget : 3 production + tests correspondants.
- Closed invariant : ConfirmDuplicateImport délégué, ses décisions inchangées.
- Before metrics : 1 corps ConfirmDuplicateImport et confirm_interrupted_publication dans racine.
- Target metrics : 1 appel de bras, 0 corps doublon racine ; helpers résolution de cibles et vérification de publications dans duplicates.rs.
- Metric commands / Stop conditions : M1 `ConfirmDuplicateImport|confirm_interrupted_publication|resolve_reprocessing_targets|verified_present_publications` ; contrat commun.
- Dependencies : D1.
- Implementation outline : extraire handler, confirmation interrompue, résolution/revalidation des publications et cibles ; distinguer revalidation qui repropose un prompt et erreur terminale ; garder l'ordre des mutations/source bindings.
- Tests to add or update : T-D2 ; historique sans fichiers, dernier/tous, cible exacte, confirmation interrompue, refus/annulation.
- Verification commands : V1–V6, T-D2 ; provenance §7.
- Acceptance criteria : même prompt, cible, choix et retour court ; la réussite du handler ne déclenche pas un épilogue auparavant évité.
- Rollback/fallback / notes : contrat commun ; ne pas fusionner les politiques de doublon fichier et direct.

### D3 — Extraire le direct et le micro (lot 3d-direct)

- Source finding / rationale : 3d direct, REQ-02/05 ; isoler orchestration capture et arrêt.
- Target files/modules : racine, runtime/mod.rs, `runtime/live.rs` ; subdivision autorisée `runtime/live/capture.rs` si nécessaire.
- Ownership bucket : runtime/application, capture existante.
- Scope budget : 4 production au maximum, exception justifiée par la séparation capture/commande si le déplacement excède 1 500 lignes ; pas de réécriture capture.rs/direct_live.rs externes.
- Closed invariant : les huit bras direct/micro délèguent.
- Before metrics : StartMicrophoneTest, StartDirectLive, StartUnifiedLive, StartLive, SetMicrophoneGain, FinishLiveOnCpu, StopUnifiedLive, StopMicrophoneTest contiennent logique.
- Target metrics : 8 appels ; start_live/activation et MicrophoneTestCapture déplacés ; 0 corps correspondant racine.
- Metric commands / Stop conditions : M1 ces variants + `start_live|activate_live_after_ready|MicrophoneTestCapture|spawn_live_attempt` ; contrat commun.
- Dependencies : D2.
- Implementation outline : extraire handlers et helpers live nécessaires, dont tentative/replay live lorsque lié à capture ; conserver primitives route moteur jusqu'à F1 et source/exécuteur unifiés jusqu'à F2 ; conserver exactement retours de closures et du dispatcher.
- Tests to add or update : T-D3 ; Start suivi Stop immédiat, compteur de persistance, gain obsolète, deux captures et finalisation concurrente existants.
- Verification commands : V1–V6, T-D3 ; provenance §7.
- Acceptance criteria : même démarrage/capture/arrêt, aucun nouveau repli, pas de perte des événements différés, tous les fichiers sous plafond.
- Rollback/fallback / notes : contrat commun ; Stop générique partagé reste famille import D5, traitement runtime identique jusque-là.

### D4 — Extraire la reprise et les liaisons sources (lot 3d-reprise)

- Source finding / rationale : 3d reprise, REQ-04 ; conserver identité et restauration.
- Target files/modules : racine, runtime/mod.rs, `runtime/recovery.rs`, `runtime/source_binding.rs`.
- Ownership bucket : runtime/application et IO de source existants.
- Scope budget : 4 production, exception justifiée par l'isolation des transferts de binding ; tests correspondants.
- Closed invariant : les cinq bras reprise délèguent avec mêmes liaisons source.
- Before metrics : ReleaseSourceBinding, InspectSourceSnapshotRecovery, ConfirmSourceSnapshotRecovery, ResumeInterrupted, ResumeReplacementCleanup dans match.
- Target metrics : 5 appels ; méthodes ensure/rebind/transfer/restore/release_source_binding et active_snapshot_bindings/inspect_source_snapshot_recovery dans source_binding.rs ; corps reprise dans recovery.rs.
- Metric commands / Stop conditions : M1 ces cinq variants et `source_binding|ResumeInterrupted` ; contrat commun.
- Dependencies : D3.
- Implementation outline : préserver transfert/restitution sur erreur, checkpoints et génération ; fonctions de catalogue appelées sans changer le modèle sélectionné ; aucune nouvelle reprise après panne.
- Tests to add or update : T-D4 ; remplacement/copie, legacy checkpoint, identité source changée, relâchement sur échec et reprise interrompue.
- Verification commands : V1–V6, T-D4 ; provenance §7.
- Acceptance criteria : mêmes fichiers durables, source non libérée prématurément, refus identiques et aucun recalcul universel des SHA.
- Rollback/fallback / notes : contrat commun ; ne pas déplacer les politiques de recovery du cœur vers adapters.

### D5 — Extraire l'import et fermer le dispatcher (lot 3d-import)

- Source finding / rationale : 3d import, REQ-02/04/07 ; terminer le principe de l'étrangleur.
- Target files/modules : racine, `runtime/effects.rs`, `runtime/import.rs`, runtime/mod.rs.
- Ownership bucket : runtime/application.
- Scope budget : 4 production, exception justifiée : déplacement final du dispatcher avec sa déclaration et la famille résiduelle, pour contrôler atomiquement son épilogue.
- Closed invariant : chacun des 30 bras ne fait plus que déléguer ; préambule/épilogue uniques.
- Before metrics : 12 bras résiduels ; dispatcher encore dans racine.
- Target metrics : 0 corps métier dans match ; 1 effect dans effects.rs, signature initiale inchangée.
- Metric commands / Stop conditions : M1 `fn effect|ImportEffect::` + lecture de chaque bras et contrôle de flot ; contrat commun.
- Dependencies : D4.
- Implementation outline : traiter les douze variants SetGpuCandidate, ReadUnifiedDocumentPage, ReadUnifiedAudioPartsPage, Prepare, StartWorker, StartAutomaticFinalTranscription, ScanHistory, ResolveImportPublication, ResolveLivePublication, PersistSegment, Publish et Stop. Déplacer préparation/persistance/publication/stop et wrappers d'orchestration moteur dans import.rs ; conserver primitives moteur F1. Le préambule et l'épilogue sont déplacés ensemble dans effects.rs ; résultat interne de contrôle seulement si nécessaire pour conserver les sorties précoces.
- Tests to add or update : T-D5 ; échec avant/après binding, échec publication, erreur de persistance et ACK, échec automatique avec compagnon JSON, Stop terminal obsolète.
- Verification commands : V1–V6, T-D5 ; provenance §7.
- Acceptance criteria : toutes les variantes couvertes par match exhaustif, aucun bras fourre-tout masquant un nouveau variant, même nombre/ordre d'événements et mêmes chemins de nettoyage ; import.rs doit être subdivisé en `runtime/import/{preparation,publication,worker_start}.rs` si >1 500 lignes, avec plafond atomique porté à 7 production pour ce seul lot (corps existants interdépendants, pas une autre famille).
- Rollback/fallback / notes : contrat commun ; la preuve de fermeture porte sur l'intégralité des bras et pas seulement la baisse de taille de effect.

### E1 — Extraire le polling principal (lot 3e-poll)

- Source finding / rationale : 3e, REQ-01–05 ; attendre la fermeture des handlers.
- Target files/modules : racine, runtime/mod.rs, `runtime/poll.rs`, `runtime/poll/{capture,parakeet}.rs`.
- Ownership bucket : runtime/application.
- Scope budget : 5 production, exception justifiée par les aides capture/VAD et worker nécessaires au même cycle de polling, sans en changer l'ordre.
- Closed invariant : polling principal possédé par runtime/poll.
- Before metrics : poll, poll_live_capture, pump_live_windows, progression/résultat VAD, poll_parakeet_worker et telemetry dans racine.
- Target metrics : zéro corps de ces méthodes dans racine, un exemplaire chacune dans poll et ses enfants, mêmes priorités.
- Metric commands / Stop conditions : M1 `fn poll\(|poll_live_capture|pump_live_windows|poll_parakeet_worker|take_legacy_live_telemetry` ; contrat commun.
- Dependencies : D5.
- Implementation outline : déplacer poll et ses auxiliaires sans réordonner capture, worker, ACK ni événements différés ; isoler aides dans fichiers ≤1 500 lignes. Les aides de traitement d'échec non encore extraites rejoignent le propriétaire runtime existant, sans changement de politique.
- Tests to add or update : T-E ; événements tardifs, panne après Ready, anti-famine, arrêt avec persistance en attente.
- Verification commands : V1–V6, T-E ; provenance §7.
- Acceptance criteria : mêmes retours Option/Result, pas de boucle active nouvelle ni modification de seuil LIVE_CAPTURE_EVENTS_BEFORE_WORKER_POLL.
- Rollback/fallback / notes : contrat commun ; aucune extraction de backends anticipée.

### E2 — Extraire le polling unifié (lot 3e-unifié)

- Source finding / rationale : 3e, REQ-02/05 ; isoler l'orchestration des sessions.
- Target files/modules : racine, runtime/mod.rs, `runtime/poll_unified.rs` ; subdivision `runtime/poll_unified/document.rs` si nécessaire.
- Ownership bucket : runtime/application sessions.
- Scope budget : 4 production, exception de subdivision seulement si nécessaire au plafond ; tests correspondants.
- Closed invariant : poll_unified_sessions et poll_unified_document possédés par runtime.
- Before metrics : 2 méthodes dans racine (dont une ~967 lignes physiques).
- Target metrics : 0 définition dans racine, un exemplaire dans poll_unified ; séquencement inchangé.
- Metric commands / Stop conditions : M1 `poll_unified_sessions|poll_unified_document` ; contrat commun.
- Dependencies : E1.
- Implementation outline : déplacer les deux méthodes et helpers de libération/admission directement requis dans runtime ; attribuer explicitement à poll_unified.rs l'orchestration `retry_native_gpu_startup_on_cpu` et ses types `NativeAutoCpuRetry`, `FailedManualGpuPeer` et `manual_gpu_failure_event`, qui manipulent scheduler/sessions/archives/exécuteur. Le prédicat pur can_retry_native_auto_on_cpu reste provisoirement racine puis backends F1 ; garder les types source/exécuteur côté racine jusqu'à F2. Utiliser document.rs si ce regroupement dépasse 1 500 lignes.
- Tests to add or update : T-E ; deux sessions, publication finale pendant autre capture, fermeture globale, événements document périmés et drain final.
- Verification commands : V1–V6, T-E ; provenance §7.
- Acceptance criteria : aucun ACK perdu, libération aux mêmes points, aucune limite ou concurrence modifiée.
- Rollback/fallback / notes : contrat commun ; ne pas fusionner les machines de session/worker.

### F1 — Extraire les backends (lot 3f-backends)

- Source finding / rationale : 3f, REQ-06 ; primitives de sélection en dernier.
- Target files/modules : racine, `backends/mod.rs`, `backends/native.rs`, support.rs ; runtime/import/worker_start.rs si déjà créé en D5.
- Ownership bucket : infra/adapters sélection et lancement ; orchestration reste runtime.
- Scope budget : 5 production + tests correspondants ; exception pour le helper SHA transversal et l'appelant moteur existant, sans famille d'effets supplémentaire.
- Closed invariant : catalogue, routes et primitives moteur sans dépendance service/runtime.
- Before metrics : RuntimeModelCatalog, IdleWorker, routes et attestations dans racine.
- Target metrics : 0 corps de ces primitives dans racine ; backends n'importe ni service ni ImportRuntime.
- Metric commands / Stop conditions : M1 `RuntimeModelCatalog|IdleWorker|native_unified_route|native_cpu_route|gpu_evidence_matches|qualify_gpu_worker|probe_cuda_worker` ; contrat commun.
- Dependencies : E2.
- Implementation outline : catalogue/bindings et sélection dans mod.rs, routes/preuves et prédicat pur can_retry_native_auto_on_cpu dans native.rs ; decode_sha256 et format_sha256 dans support.rs avec imports explicites des consommateurs, sans dépendance backend vers runtime. Conserver méthodes start_worker*, start_parakeet_worker et handle_worker_failure sur ImportRuntime dans runtime ; retry_native_gpu_startup_on_cpu et sa gestion des pairs restent runtime/poll_unified.rs. Déplacer uniquement les aides techniques, pas la politique. Réexports historiques des probes et gpu_evidence_matches conservés.
- Tests to add or update : T-F, worker_control et gpu_qualification_transport publics existants ; Auto sans GPU, panne pré-audio, GPU manuel, Stop empêchant retry, cache worker conservé.
- Verification commands : V1–V6, T-F ; provenance §7.
- Acceptance criteria : mêmes preuves/identités/repli et limites ; aucun nouveau fallback ou recalcul, pas de dépendance inverse.
- Rollback/fallback / notes : contrat commun ; ne pas qualifier CPU/GPU natifs à partir des faux workers.

### F2 — Extraire Parakeet unifié (lot 3f-parakeet)

- Source finding / rationale : 3f, REQ-01/05/06 ; isoler les ressources les plus couplées après orchestration.
- Target files/modules : racine, `parakeet_unified/mod.rs`, `parakeet_unified/executor.rs`, `parakeet_unified/native.rs` ; runtime/mod.rs pour imports d'état.
- Ownership bucket : infra/adapters exécution unifiée ; orchestration runtime conservée.
- Scope budget : 5 production, exception nécessaire aux trois responsabilités source/état, thread/exécuteur et transcripteur natif ; tests correspondants.
- Closed invariant : types/primitives de sessions unifiées possédés par parakeet_unified, sans dépendance service/runtime.
- Before metrics : UnifiedDocumentDrain, UnifiedSessionSource/State, NativeParakeetWindowTranscriber, UnifiedInferenceExecutor et résultats/commandes dans racine.
- Target metrics : 0 définition correspondante racine ; un exemplaire dans sous-arbre cible, chaque fichier ≤1 500 lignes.
- Metric commands / Stop conditions : M1 ces symboles ; contrat commun.
- Dependencies : F1.
- Implementation outline : déplacer types de source/état et restore_unified_document_prefix dans mod.rs, commandes/résultats/exécuteur et start/stop/reap/restart dans executor.rs, transcripteur natif et validation PCM dans native.rs ; utiliser types de routes backends ; méthodes d'admission/démarrage sur ImportRuntime restent runtime.
- Tests to add or update : T-F et témoins unifiés existants de panique, déconnexion, Stop/Ready, recyclage enfant, restauration/reprise ; ajuster uniquement accès privés nécessaires.
- Verification commands : V1–V6, T-F ; provenance §7.
- Acceptance criteria : mêmes canaux bornés, barrières Ready/Stopped, propriété et durée de vie des threads/enfants, aucune ouverture de visibilité publique.
- Rollback/fallback / notes : contrat commun ; aucune nouvelle pool, aucun nouveau mécanisme de planification.

### F3 — Fermer la façade et les helpers résiduels (lot 3f-clôture)

- Source finding / rationale : fermeture de l'étrangleur, REQ-07 ; ne pas laisser une seconde orchestration dans racine.
- Target files/modules : racine, runtime/mod.rs, `runtime/support.rs`, support.rs, runtime/live.rs, runtime/import.rs et leurs subdivisions déjà créées.
- Ownership bucket : runtime/application et helpers techniques à responsabilité explicite.
- Scope budget : au plus 8 production, exceptionnel pour le câblage final des propriétaires déjà créés et la suppression des imports/reexports transitoires ; aucun nouveau fichier hors `worker_ipc/`.
- Closed invariant : racine façade seule, sans corps algorithmique ni responsabilité résiduelle non attribuée.
- Before metrics : à relever après F2 ; inventaire des fonctions/types/statics encore dans racine, pas une estimation arbitraire.
- Target metrics : 0 corps algorithmique racine, zéro `#[path]` ordinaire ajouté, réexports publics identiques ; runtime/mod.rs = état, constructeurs et déclarations seulement, helpers répartis.
- Metric commands / Stop conditions : M1/M2/M3 sur tout sous-arbre et revue exhaustive des symboles racine ; contrat commun.
- Dependencies : F2.
- Implementation outline : affecter helpers WAV/temps/hooks et validations audio au propriétaire runtime/support.rs ou responsabilité déjà nommée ; helpers techniques transversaux restent dans support.rs (memory_trace et SHA déjà déplacés en A/F1), avec zéro dépendance vers runtime/service/archive. Helpers live/VAD vers live/poll, méthodes start/run/admission unifiées restantes vers runtime/import ou live selon appelant. Maintenir les features et tests sans exposer l'API. Retirer imports transitoires inutiles et vérifier la façade depuis ses consommateurs.
- Tests to add or update : mêmes scénarios, V4/V5 sur toutes configurations prévues ; aucun test de simple déplacement de lignes.
- Verification commands : V0–V6, T-A à T-F et M1–M3 ; provenance §7.
- Acceptance criteria : tous les propriétaires de la carte D §4 ont leurs symboles, pas de logique perdue dans façade, pas de fichier monolithique recréé, bilan des réserves héritées ; tests/default/features et compilation consommateurs clos avant livraison structurelle.
- Rollback/fallback / notes : contrat commun ; si reliquat ne tient pas dans les chemins/budget ou nécessite autre choix d'architecture, ne pas improviser un lot 14 : exposer l'écart pour arbitrage.

## 7. Verification Matrix

### Lectures et métriques, sans Cargo

À lancer depuis la racine du dépôt. Les lectures Git utilisent `--no-optional-locks`, sans lecture Git concurrente pendant une commande gardée.

| ID / zone | Commande ou contrôle | Provenance | Résultat attendu | Tâches |
| --- | --- | --- | --- | --- |
| V0 état | `git --no-optional-locks status --short` ; `git rev-parse HEAD` ; `Get-PSDrive C` | Observed (équivalents sans optional-locks pour le relevé initial) | changements attribués, base connue, ≥30 Gio avant campagne | toutes |
| M1 symboles | bloc M1 ci-dessous, puis motif ciblé de chaque tâche | Source inferred | symboles uniques, propriétaire attendu, appelants vérifiés | toutes |
| M2 taille | bloc M2 ci-dessous | Source inferred | tri par fichier puis distinguer production/tests/commentaires ; >1 500 physiques déclenche comptage production relu | toutes |
| M3 diff | `git diff --check` ; `git diff --stat` ; `git diff -- crates/whisper-adapters/src/worker_ipc.rs crates/whisper-adapters/src/worker_ipc` | Source inferred | pas d'espace invalide ; déplacement et câblage seuls ; visibilité/direction revues manuellement | toutes |

M1 (PowerShell, lecture seule) :

```powershell
rg -n '^(pub.*|struct|enum|impl|fn|mod|const|static)|^    (pub.*fn|fn )' crates/whisper-adapters/src/worker_ipc.rs crates/whisper-adapters/src/worker_ipc
```

M2 (PowerShell, lecture seule) :

```powershell
Get-ChildItem crates/whisper-adapters/src/worker_ipc -Recurse -Filter *.rs | ForEach-Object {
    [pscustomobject]@{Path=$_.FullName; Lines=(Get-Content -LiteralPath $_.FullName).Count}
}
```

Le comptage physique ne remplace pas le comptage des lignes de production. Aucun nouveau script de métriques ou manifeste n'est requis.

### Commandes internes futures, exclusivement via garde autorisée

Les commandes suivantes sont les **arguments Cargo internes** à transmettre au lanceur gardé après autorisation. Ne jamais les exécuter directement. Le chemin exact du lanceur compatible reste non établi ; aucune commande d'invocation prétendument opérationnelle n'est inventée. L'interface observée d'une ancienne garde est `-Mode SelfTest`, puis `-Mode Run -CargoArguments @(...)` ; cela ne prouve ni sa réutilisabilité ni son autorisation.

| ID | Arguments / contrôle | Provenance | Résultat attendu et couverture |
| --- | --- | --- | --- |
| V1 | `fmt --all -- --check` | Source inferred, workspace Rust | format sans modification, toutes tâches ; Cargo fmt également gardé |
| V2 découverte | `test --offline --locked -p whisper-adapters --lib -- --list` | Source inferred, bibliothèque observée | établir liste réelle avant/après, mêmes scénarios, noms ajustés seulement si déplacement privé justifié |
| V3 témoins | `test --offline --locked -p whisper-adapters --lib <filtre> -- --nocapture` | Placeholder pour filtre exact, noms observés ci-dessous | exécutions pertinentes non nulles ; pas de simple PASS d'un test qui a skippé son corps |
| V4 baseline/lot | `test --offline --locked -p whisper-adapters --lib` | Source inferred | baseline puis après extraction ; aucune régression, ignored/conditions de campagne séparées |
| V5 configurations | V2 et V4 avec respectivement `--features recette-hooks`, `--features l01-memory-qualification`, `--features personal-qualification`, `--features recette-hooks,l3-gpu-qualification` avant `--` | Source inferred, features du manifeste | compilation/découverte et témoins configurés avant/après ; ressources/variables manquantes = NOT RUN, jamais succès implicite |
| V6 consommateurs | `check --offline --locked --workspace` ; `clippy --offline --locked -p whisper-adapters --lib -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity` | Source inferred, manifestes/AGENTS | compatibilité consommateurs ; comparer warnings hérités/nouveaux, sans prétendre -D warnings existant |
| T-public F1 | `test --offline --locked -p whisper-adapters --test worker_control --test gpu_qualification_transport` | Source inferred, fichiers observés | sélection et frontière publique inchangées |
| Natifs | campagnes CPU/GPU/Auto et tests opt-in `parakeet_unified_native` | Repo docs/source, autorisation distincte | NOT RUN ici ; aucune levée de #[ignore] ni construction de fixture native implicite |
| Fenêtre Windows | recette import WAV/MP3, direct, file/doublon/reprise, Stop/fermeture, CPU/GPU/Auto selon installation | cadrage/AGENTS | réservée à l'utilisateur après remise autorisée conforme au profil ; pas de nouvelle qualification imposée à la rédaction |

V4/V5 peuvent appeler des rustc/faux workers, écrire des fixtures ou retourner tôt si variables absentes. Avant toute exécution, relever les variables et chemins réels de chaque témoin depuis son helper ; confiner toutes ses écritures dans la racine autorisée du lot. Aucune variable ou répertoire de fixture inventé ne vaut configuration. Si un témoin obligatoire est inopérant sans environnement, arrêter avant de déplacer le code concerné ou demander l'arbitrage précis sur la validation manquante.

### Témoins repérés, à exécuter avant/après leur extraction

Chaque nom ci-dessous est un filtre V3 observé dans les sources. Vérifier le nombre de tests lancés et le chemin réellement parcouru. Pas de nouveaux tests systématiques si ces témoins couvrent la tranche.

| Groupe | Filtres existants / scénarios à compléter si non couverts | Tâches |
| --- | --- | --- |
| T-A | `frame_limit_counts_newline_and_rejects_oversize_before_write`, `framed_ipc_rejects_incomplete_oversized_and_v1_messages`, `gpu_attestation_ack_is_bound_to_the_current_session`, `stdin_frames_preserve_live_start_and_admitted_window_before_normal_stop`, `slow_worker_backpressure_keeps_command_queue_bounded_and_retries_window` | A |
| T-B | `stop_before_commit_atomically_prevents_publication`, `stop_during_blocked_commit_returns_promptly_and_reports_finalizing` ; état après erreur/rollback à vérifier explicitement | B |
| T-C | `effect_queue_accepts_eight_and_rejects_the_ninth_without_blocking`, `persist_segment_queue_full_is_distinct_from_disconnection`, `stop_control_is_admitted_when_effect_queue_is_full`, `queued_live_start_is_applied_before_an_immediate_stop`, `saturated_pending_queue_cannot_start_a_live_job_after_stop_and_allows_next_start` | C |
| T-D1 | `queued_unified_gpu_refusal_preserves_durable_entry_and_source_before_preparation`, `queued_unified_capacity_refusal_keeps_fifo_entry_without_preparing_route` ; FIFO/RemoveQueued/ScanQueue à sélectionner par inventaire | D1 |
| T-D2 | `emptied_transcriptions_keeps_history_but_offers_no_duplicate_targets`, `reprocessing_latest_requires_known_unique_order_and_all_includes_inactive_present`, `browse_accepts_only_exact_verified_treatment_in_current_destination` ; réponses doublon/annulation à relever | D2 |
| T-D3 | `direct_micro_start_and_single_stop_are_applied_in_order`, `stopped_worker_waits_until_live_segment_persistence_is_acknowledged`, `unified_capture_gain_changes_apply_to_the_next_written_pcm_block_and_reject_stale_targets` | D3 |
| T-D4 | `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution`, `r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication` ; échecs de transfert/restauration à relever | D4 |
| T-D5 | `failed_publish_drops_pending_worker_without_starting_retention`, `failed_worker_message_does_not_hide_later_correlated_stop`, `stale_stopped_does_not_hide_later_correlated_stop` ; épilogue/retours anticipés/JSON à vérifier par chemin | D5 |
| T-E | `unified_capture_updates_preview_and_final_document_before_stop_then_publishes_the_tail`, `global_close_waits_for_both_capture_publications_and_refuses_new_admission`, `stop_counters_reach_application_before_final_window`, `stale_mp3_and_failed_events_have_old_attempt_identity` | E1/E2 |
| T-F | `auto_without_gpu_starts_cpu_with_normalized_compute_and_same_identity`, `auto_gpu_ready_is_attested_and_explicitly_admitted`, `auto_import_gpu_exit_after_ready_before_begin_retries_once_on_cpu`, `forced_gpu_initialization_failure_waits_for_explicit_cpu_choice`, `stop_latched_after_gpu_failure_prevents_cpu_worker_launch`, `retained_worker_reads_two_correlated_import_sessions_and_hits_model_cache`, `prepared_native_routes_without_start_are_bounded_and_stop_releases_identity` | F1/F2 |

## 8. Decision Gates

Ces entrées reprennent les conditions des AGENTS ; elles ne créent pas une campagne ou un artefact de qualification supplémentaire.

| Condition | Statut | Bloque | Décision / contrôle | Valeur recommandée | Validation |
| --- | --- | --- | --- | --- | --- |
| Acceptation et GO précis | Blocking avant code | lot concerné | accepter version Git, tâche et chemins ; autorisation de campagne distinctement délimitée | commencer seulement A après acceptation | réponse écrite dans session d'exécution |
| Garde et racines | Blocking avant Cargo | vérifications et fermeture du lot | racine de campagne à autoriser ; garde existante compatible ou adaptation autorisée | aucune ancienne dérogation héritée | préflight et auto-tests obligatoires, surveillance existante ; pas de contournement |
| Détails privés | Validation | clôture du lot concerné | visibilités et accès tests | portée worker_ipc minimale, sans API publique | compilation et revue |
| Témoins/fixtures | Validation avant extraction | tranche sans preuve baseline | vérifier variables/ressources et absence de retour anticipé silencieux | conserver scénarios réels, compléter seulement un manque avéré | liste puis exécution attribuée |

Règles de campagne à appliquer intégralement après accord : ≥30 Gio ; CARGO_TARGET_DIR canonique `target/parakeet-tests/cargo-shared/` ; CARGO_HOME/TEMP/TMP sous racine de lot autorisée, résolutions canoniques et absence de jonctions sortantes ; cache copié sous lot si nécessaire, pas de cache utilisateur en écriture ; CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=line-tables-only. Auto-tests de refus avant premier Cargo, même metadata/fmt ; surveillance existante pendant commande, arrêt et conservation sur violation. Ni nouvelle isolation/ACL/service/surveillance ni dérogation Temp/Git/NVIDIA automatique. Si les règles strictes ne peuvent être satisfaites, arrêter sans lancer Cargo.

Clôture future : petites preuves archivées avec arborescence, nombre et SHA-256 selon AGENTS ; suppression limitée aux copies/WAV d'un manifeste approuvé, sinon aucun nettoyage ; espace libre à nouveau relevé. Aucun nouveau manifeste de qualification du produit. Pas de nettoyage cargo-shared.

## 9. Out of Scope

Ne pas faire : correction fonctionnelle découverte en route, refonte du domaine/core, modification du protocole ou messages, concurrence/pool/canaux nouveaux, nouvelle dépendance/feature/unsafe, remplacement des moteurs, optimisation, changement UI ou modèle, migration des hooks, nettoyage général des tests, nouvelle CI distante, qualification native implicite, push, modification des non suivis préexistants. Une divergence produit reste un constat attribué et appelle un périmètre distinct.

## 10. Final Self-Check

| Contrôle | Résultat | Limites |
| --- | --- | --- |
| Plan réel et sections du modèle présentes | Pass documentaire | proposition, pas autorisation |
| Chaque indication 3a–3f tracée | Pass documentaire | §4–6 |
| Tâches bornées, chemins, dépendances, tests, acceptation et retour arrière | Pass documentaire | contrat commun + spécificités de chaque tâche |
| Invariants/métriques/arrêts pour phases et tâches | Pass documentaire | §5 renvoie aux contrats §6 ; mesures dynamiques futures |
| Budgets explicites et exceptions motivées | Pass documentaire | dépassement non prévu = arrêt |
| Carte d'architecture et tests sans API artificielle | Pass documentaire | cadrage §4 ; tests historiques conservés |
| Commandes, provenance et contrôles manquants distingués | Pass documentaire | garde non établie ; aucune commande exécutée |
| Conditions de départ explicites | Pass documentaire | §8 |
| Revue indépendante | CLEAN documentaire | même reviewer, cycle 2 sur 704cb179 ; aucun REQUIRED ouvert |
| Acceptation et code | Non acquis | aucun accord implicite |

Revue du 29 octobre 2026 : cycle 1 sur `4daa59a71feff5d8d58014ae135f338beb55782f`, deux REQUIRED IMPORTANT, aucun BLOQUANT ; corrections par l'auteur principal et cycle 2 sur `704cb179775979d7217024bbdfeba5539308c0d7`, WIPC-R01 (helpers partagés) et WIPC-R02 (orchestration de reprise native) fermés par rust_design_reviewer. Commandes copiables et description d'activate également vérifiées. Verdict CLEAN sur les deux propositions documentaires ; aucune exécution produit. L'ajout de ce résultat est EVIDENCE_ONLY. Détail de fermeture : cadrage §9. Le plan reste proposé et non exécutable sans acceptation/GO et conditions §8.

## 11. Implementation Agent Prompt

À utiliser uniquement après acceptation et GO écrit précis :

« Mode IMPLEMENT. Exécuter uniquement A/3a du plan `docs/cadrage/plan-refacto-worker-ipc.md` et du cadrage `docs/cadrage/refacto-worker-ipc.md` dans leurs versions Git acceptées. Relire AGENTS racine/adapters, état Git actuel et sources. Vous n'êtes pas seul dans le dépôt : préserver changements suivis/non suivis, coordonner les responsabilités et ne pas annuler les modifications des autres. Autorité limitée aux quatre fichiers production et tests correspondants du lot A ; aucun autre lot. Avant code, établir les prérequis et témoins réels ; avant toute commande Cargo, disposer de l'accord de campagne, d'une garde compatible, des racines/env canoniques, auto-tests et espace libre requis. Exécuter M1–M3, V1–V6 et T-A à travers le lanceur autorisé pour Cargo. Stop si baseline en échec, garde non conforme, critère partiel, conflit ou élargissement requis. Conserver contrat/public API, bornes, ordre Stop, framing et identités. Rapporter diff, contrôles réellement exécutés et limites ; ne pas déclarer CLEAN. La session principale soumettra ensuite le candidat au rust_review_coordinator ; deux cycles maximum, validation Windows distincte. Aucun push. »
