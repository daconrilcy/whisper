Avancée : lots A à F3 clos après vérifications et revue CLEAN ; clôture structurelle et bilan final au §38.
Blocage : aucun dans le périmètre de refactorisation autorisé.
Décision : F3 et le plan de refactorisation clos sans réserve dans ce périmètre ; commits locaux sans push.

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
- Target files/modules : racine, `backends/mod.rs`, `backends/native.rs`, support.rs, `runtime/import/worker_start.rs` et sa déclaration dans `runtime/import.rs` ; création et budget ajusté explicitement autorisés le 30 octobre 2026 (§33).
- Ownership bucket : infra/adapters sélection et lancement ; orchestration reste runtime.
- Scope budget : 6 production + tests correspondants, après accord précis du 30 octobre 2026 ; le sixième fichier reçoit uniquement la déclaration du sous-module. Exception pour le helper SHA transversal, l'orchestration et son câblage, sans famille d'effets supplémentaire.
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
## 12. Exécution du lot A — 29 octobre 2026

Avancée : candidat A/3a implémenté et revu ; aucun défaut de code actionnable relevé.
Blocage : suite recette-hooks seule en échec sur le test de spectre ; lot A non clos.
Décision : disposition utilisateur requise sur cette réserve ; aucun lot suivant autorisé ni démarré.

### Autorité et périmètre effectifs

Dans la session d'exécution, l'utilisateur a demandé la refactorisation selon ces deux documents puis autorisé précisément le démarrage du seul lot A, sa campagne, l'adaptation d'une copie de la garde existante et les commits locaux, sans push. Base Git : `fa4772c26a66ad7f4347c095ae13260c55ce216b`. Les autres lots restent conditionnés à leurs autorisations et à la clôture des prérequis ; cet enregistrement ne vaut pas GO global.

Accords complémentaires de cette même session :

- Helper de fixtures `worker_ipc/tests/microphone_and_finalization.rs` : racine obtenue par `std::env::temp_dir()` au lieu de `CARGO_TARGET_DIR`, avec TEMP sous la racine du lot.
- Exclusion du seul scénario natif `direct_live::d2_corpus::d2_corpus_real_cpu_turbo_and_parakeet`, puis du seul témoin historique `l3c_acceptance_fault_is_confined_to_its_canonical_execution_root` qui exige des écritures dans l3c/l3d. Ils restent NOT RUN ; aucune destination volontaire l3c/l3d autorisée.
- Pour A uniquement, événements Temp utilisateur sans attribution démontrée consignés `UNATTRIBUTED` sans arrêt à eux seuls. Aucune destination volontaire supplémentaire autorisée.
- Pour tous les lots, événements non attribués dans `.git/objects/` consignés sans arrêt à eux seuls. HEAD, références, index, configuration, sources et toute écriture hors racine attribuée à la campagne restent soumis à l'arrêt.
- Reproductions de tous les échecs autorisées. Aucun accord de correction fonctionnelle ou d'exclusion supplémentaire n'en découle.

### Candidat et vérifications

Implémentation déléguée à `rust_implementer` ; la session principale n'a pas modifié le code en parallèle. Transport et lecture/admission déplacés vers `ipc/mod.rs` (398 lignes physiques), framing et limite vers `ipc/framing.rs` (106), diagnostics et compteurs partagés vers `support.rs` (41). Façade publique conservée avec réexports et liaisons internes transitoires ; helpers live laissés dans la racine pour les lots prévus. Seul ajustement hors transport : helper de fixtures explicitement autorisé ci-dessus. Aucun manifeste, dépendance ou Cargo.lock modifié.

Racine d'exécution : `target/parakeet-tests/refacto-worker-ipc-a/`, garde `guard.ps1`, journaux sous `logs/` ; compilation sous `target/parakeet-tests/cargo-shared/`. Cache existant copié sous la racine A, sans téléchargement. La fixture MP3 existante exigée par un test a été copiée sous A et son SHA-256 vérifié. Les arrêts initiaux sur événement Temp et objet Git sont conservés ; leur origine n'est pas démontrée. Les auto-tests de la garde ont réussi après les dérogations écrites. Les sources, modèles, corpus et dossiers tiers ont été préservés.

| Vérification du candidat | Résultat réellement observé |
| --- | --- |
| `git diff --check`, format gardé | PASS |
| `check --offline --locked --workspace` gardé | PASS |
| Cinq témoins T-A gardés | 5/5 exécutés et réussis |
| Suite par défaut séquentielle | PASS |
| Découverte et suites l01-memory / personal séquentielles | PASS : chacune 282 découverts, 265 réussis, 16 ignorés, 1 D2 filtré |
| Découverte et suite recette-hooks,l3-gpu séquentielle | PASS : 287 découverts, 269 réussis, 16 ignorés, 2 filtrés autorisés |
| Suite recette-hooks seule séquentielle | FAIL deux fois : 268 réussis, 1 échec, 16 ignorés, 1 filtré ; test de spectre isolé PASS |
| Clippy avec les quatre lints structurels du plan | PASS commande ; 143 avertissements, contre 142 en baseline ; nouvel import inutilisé `process::Command` |
| Natifs et fenêtre Windows | NOT RUN |

Les cinq configurations avaient obtenu une baseline séquentielle réussie avant extraction, après reproduction d'échecs. `microphone_probe::tests::retry_epoch_discards_audio_acquired_before_the_retry_barrier` a échoué avant extraction dans d'autres configurations et deux fois après extraction dans recette-hooks seule : `fresh retry spectrum did not arrive`. Le succès isolé et celui de la combinaison de features ne ferment pas le contrôle recette-hooks seule ; cause et lien éventuel avec le candidat non établis. Aucun code microphone corrigé. L'échec initial de configuration MP3 et les autres échecs de baseline restent consignés, sans être convertis en succès.

Les 16 tests ignorés ne sont pas des tests exécutés. Quatre corps conditionnels natifs avaient signalé NOT RUN en baseline ; les sorties capturées des suites complémentaires ne permettent pas de les identifier individuellement sur le candidat. Aucun succès natif implicite revendiqué.

### Revue et état de sortie

Revue par `rust_review_coordinator` et un même `rust_reviewer` indépendant en lecture seule : deux passes, zéro correction, une passe de vérification complémentaire, candidat working tree à base fa4772c2. Demandes WIPC-A-V1/V2 exécutées puis appréciées par le reviewer. Verdict final **BLOCKED**, aucun constat de code actionnable ouvert, aucune déclaration CLEAN. Les corps déplacés ont été comparés à la baseline par le reviewer ; compteurs uniques, chemins publics et témoins transport préservés. Le nouvel avertissement d'import reste signalé comme limite de propreté.

Le candidat est conservé en commit local avec ce bilan ; cette conservation ne clôt pas A et n'autorise pas B. Une disposition utilisateur sur le contrôle recette-hooks en échec, ou une preuve nouvelle obtenue dans des conditions matériellement différentes, reste nécessaire. Aucune recette Windows ni publication distante effectuée. L'archivage de clôture n'est pas déclaré réalisé : lot non clos ; aucun nettoyage de campagne ou cargo-shared effectué.

Espace libre : relevé initial implémenteur 122 922 348 544 octets ; relevé après candidat et revue 122 279 821 312 octets (environ 113,9 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit.
## 13. Reprise corrective autorisée du lot A

Avancée : l'utilisateur refuse la clôture avec réserves et demande les corrections nécessaires.
Blocage : origine du délai de spectre à établir ; clôture conditionnée aux vérifications et à la revue.
Décision : corriger dans le lot A les causes établies, sans lancer de lot suivant ni ajouter d'exclusion.

Demande écrite de la même session : « non je souhaite que qu el lot A soit cloturé sans reserve. Fais les corrections necessaire ». Elle autorise l'investigation et la correction du test de spectre identifié, ainsi que de l'import inutilisé, dans la continuité du candidat `f7b0cd38`. Le périmètre correctif comprend `crates/whisper-adapters/src/microphone_probe.rs`, ses tests existants et, si nécessaire, un fichier de tests séparé sous `microphone_probe/`, ainsi que l'import devenu inutile dans `worker_ipc.rs`. Le test microphone de finalisation déjà observé en échec peut être investigué et stabilisé dans son fichier de tests existant si une cause commune est démontrée. Pas de nouvelle dépendance, feature, exclusion, modification de Cargo.lock ou autre évolution produit. Les règles de campagne et accords précédents restent applicables.

L'implémenteur doit établir la cause observable, préserver les assertions sur le rejet des anciens blocs audio et la publication d'un spectre frais, et privilégier une synchronisation déterministe du test si le défaut est dans son ordonnancement. Augmenter simplement un délai, ignorer le test ou répéter jusqu'au succès ne constitue pas une correction suffisante. Toute correction de production doit être justifiée par le défaut démontré et accompagnée du témoin pertinent. Les profils de revue restent indépendants ; les deux passes précédentes sans correction ne valent pas deux cycles revue/correction accomplis. Deux cycles correctifs maximum pour cette reprise autorisée.

Vérifications attendues : reproduction discriminante du défaut et contrôle après correction, cinq témoins T-A, suites des cinq configurations à paramètres identiques et sorties permettant d'identifier les retours NOT RUN, format, compatibilité workspace et Clippy sans nouvel avertissement introduit par A. Le reviewer décide de la fermeture des constats. Les exclusions natives D2/l3c déjà acceptées demeurent hors de cette clôture structurelle ; aucun succès natif ni Windows implicite. À clôture effective seulement : archivage des petites preuves sous la racine autorisée avec contrôle de nombre et SHA-256, aucun nettoyage sans manifeste de suppression approuvé, relevé final de l'espace libre et bilan actualisé par la session principale. Commit local uniquement, aucun push.
## 14. Clôture corrective du lot A — 29 octobre 2026

Avancée : lot A/3a corrigé, vérifié et revu CLEAN ; petites preuves archivées.
Blocage : aucun dans le périmètre structurel A et sa reprise corrective autorisée.
Décision : lot A clos sans réserve dans ce périmètre ; aucun lot suivant démarré ou autorisé.

### Corrections et preuve du défaut de témoin

Reprise sur `fca4b8339ab5a98bf148f6ad586599cbe62ffdf7`, candidat A précédent `f7b0cd38ff9e2e9e9b44e5afe566f436882136c3`. L'implémenteur a modifié seulement `microphone_probe.rs`, ajouté `microphone_probe/retry_epoch_regression.rs` et ajusté l'import `Command` de `worker_ipc.rs` pour les tests. Aucun changement de logique produit, de protocole, dépendance, feature ou manifeste.

Le test de barrière de reprise construisait ses horodatages par deux lectures rapprochées de l'horloge ; leur égalité pouvait faire admettre le bloc supposé ancien. Ses 4 800 échantillons deviennent cinq fragments pour une file de quatre places : une saturation avance alors l'époque et invalide le préfixe de spectre. Une reproduction instrumentée a observé l'époque 2 au lieu de 1 avant le flux frais, avec une perte et aucun bloc traité. Le nouveau test séparé impose l'égalité temporelle et reproduit déterministement saturation et changement d'époque, puis vérifie le spectre après un flux frais. Cette preuve ne permet pas d'attribuer individuellement chaque ancien échec au même mécanisme.

Le test existant fournit désormais une acquisition strictement antérieure à la barrière (`Instant::now() - 1 ms`), attend le compteur de traitement de chaque bloc et contrôle l'absence de perte. Les assertions de rejet de l'audio ancien, d'identité d'époque et de production d'un spectre frais sont maintenues. Il n'est ni ignoré ni rendu passant par simple augmentation du délai. L'échec intermédiaire précédant ce dernier changement et les cinq suites finales qui le suivent restent distingués dans les preuves.

### Vérifications du candidat final

Toutes les commandes Cargo ont utilisé la garde et les racines autorisées, sans téléchargement. Format, `git diff --check`, compilation workspace, cinq témoins T-A et témoin déterministe : PASS. Clippy : PASS commande, 142 avertissements, identiques au nombre de baseline ; avertissement d'import nouveau supprimé.

| Configuration séquentielle finale | Tests réussis | Ignorés | Filtrés autorisés | Résultat |
| --- | --- | --- | --- | --- |
| défaut | 266 | 16 | 1 D2 | PASS |
| recette-hooks | 270 | 16 | 1 D2 | PASS |
| l01-memory-qualification | 266 | 16 | 1 D2 | PASS |
| personal-qualification | 266 | 16 | 1 D2 | PASS |
| recette-hooks,l3-gpu-qualification | 270 | 16 | 2 D2/l3c | PASS |

Les sorties visibles identifient les retours anticipés. Trois scénarios auparavant NOT RUN ont en outre été exécutés isolément avec variables bornées, sans lancement natif, et réussissent : `paths_reject_a_worker_outside_the_runtime_root`, `over_limit_parakeet_import_is_refused_before_pending_creation` et `injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume`.

Le scénario `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` demeure NOT RUN : il créerait un lien physique vers le modèle installé hors racine A. Le reviewer l'a jugé hors du critère de clôture A/T-A (reprise D4). Les 16 tests ignorés, les exclusions D2/l3c précisément acceptées, les campagnes natives et la fenêtre Windows ne sont pas revendiqués exécutés ; ils ne constituent pas une validation implicite du produit. Aucun nouveau test exclu pour obtenir cette clôture.

Journaux des cinq suites finales sous `target/parakeet-tests/refacto-worker-ipc-a/logs/` : `Run-20261029T171800388-ad4c1ef7`, `Run-20261029T171906452-7c360fea`, `Run-20261029T172019253-81a7a7b0`, `Run-20261029T172131191-8f4427d1`, `Run-20261029T172243038-d471a92d` (fichiers stdout/stderr associés).

### Revue indépendante et conservation

Le même `rust_review_coordinator` a réactivé le même `rust_reviewer` indépendant sur le candidat correctif, epoch 1. Après examen du diff, des résultats et clarification en lecture seule de la chronologie du journal intermédiaire, le reviewer rend **CLEAN** : aucun constat ouvert, aucune vérification requise manquante et aucun blocage dans le périmètre A corrigé. Les exclusions et limites hors périmètre ci-dessus restent explicites ; le verdict ne vaut pas recette native ni validation Windows. La session principale n'a pas modifié le code et consigne seule ce bilan.

Archive des petites preuves : `target/parakeet-tests/refacto-worker-ipc-a/archive/closure-20261029/proofs.zip`, 175 entrées, 644 586 octets. Arborescence relative préservée ; compte et unicité contrôlés, SHA-256 de chaque entrée comparé à sa source. SHA-256 du ZIP : `BECD1690904BB351D8A9C4F7E532AB02478E99FC18CFF5C8AD2791B953A2CE05`. Aucun cache, modèle, build ou WAV ajouté à cette archive, aucun manifeste d'empreintes créé. Aucun nettoyage ni suppression effectué faute de manifeste de suppression approuvé ; cargo-shared et entrées préexistantes conservés.

Espace libre final : 121 987 014 656 octets (environ 113,6 Gio). Le code relu et ce bilan sont conservés dans un commit local de clôture, sans push. Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés. Cette clôture remplace le blocage du premier candidat décrit au §12 ; elle ne donne aucune autorisation de commencer B.
## 15. Démarrage autorisé du lot B — 29 octobre 2026

Avancée : prérequis A clos CLEAN sur `1fd0843f08e73aaafca5b474ace05d8970895746` ; démarrage B autorisé.
Blocage : aucun obstacle constaté au préflight documentaire ; garde et baseline B à vérifier avant extraction.
Décision : exécuter B seul, vérifier et faire relire indépendamment avant clôture et commit local sans push.

Accord écrit de la session : « Passe au lot B selon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » Périmètre : `worker_ipc.rs`, `worker_ipc/publication/mod.rs` et tests directement correspondants, notamment `publication/tests.rs` pour l'état après erreur si nécessaire. Une seule machine PublicationGate et ses cinq constantes, sous publication, sans changement de transitions, atomiques, messages ou API publique. L'implémentation est déléguée au profil prévu, la revue au coordinateur et reviewer indépendants.

Racine de campagne B : `target/parakeet-tests/refacto-worker-ipc-b/`, copie adaptée de la garde existante, cache et temporaires sous cette racine ; cible Cargo partagée inchangée. Aucun mécanisme supplémentaire de surveillance ou d'isolation. Préflight, résolutions canoniques, auto-tests et surveillance obligatoires. La dérogation `.git/objects/` explicitement donnée pour tous les lots reste applicable ; la dérogation Temp donnée pour A uniquement ne l'est pas pour B. Les deux scénarios natif D2 et historique l3c restent hors des vérifications non natives de cette extraction comme précédemment accepté ; aucune nouvelle exclusion, campagne native ou recette Windows implicite. Tous les échecs peuvent être reproduits selon l'accord de session ; une violation non couverte impose arrêt et arbitrage. Espace libre initial B : 121 781 956 608 octets (environ 113,4 Gio). Les non suivis préexistants restent préservés.
## 16. Clôture du lot B — 29 octobre 2026

Avancée : PublicationGate extraite, vérifications réussies et revue indépendante CLEAN.
Blocage : aucun dans le périmètre B/REQ-02.
Décision : lot B clos sans réserve dans son périmètre autorisé ; commit local sans push, aucun lot C démarré.

### Autorité, campagne et delta

Base Git relue et testée : `2d523240a96836124830afa6045e1ba42027f374`, après clôture A. La session principale a délégué l'implémentation à `rust_implementer` puis la revue à `rust_review_coordinator` avec le même reviewer indépendant ; elle n'a pas modifié le code en parallèle. Trois fichiers du candidat : `worker_ipc.rs`, nouveau `worker_ipc/publication/mod.rs` (73 lignes physiques) et nouveau `worker_ipc/publication/tests.rs` (29).

Une seule définition de PublicationGate et de ses cinq constantes se trouve désormais sous publication. Les transitions, ordres mémoire, résultats et messages sont identiques ; réexport interne et visibilités limitées aux appelants et tests existants, sans nouvelle API publique. Les deux tests séparés ajoutés couvrent le maintien COMMITTING après une erreur de commit, l'absence de réactivation par rollback dans cet état et le rollback d'une annulation. Aucune fonction de publication d'archive déplacée ; aucun manifeste, Cargo.lock, feature, dépendance ou code unsafe modifié.

La campagne B utilise `target/parakeet-tests/refacto-worker-ipc-b/` et la cible Cargo partagée canonique. Cache et fixture MP3 copiés sous B ; variables et chemins canoniques vérifiés, auto-tests de refus et surveillance existante appliqués. Une première découverte a été interrompue par un événement `Created` sur `C:\Users\cyril\AppData\Local\Temp\etilqs_0WQsUTun2fV2LiE` à 17:30:58.001 +01:00, d'origine indéterminée, sans attribution à Cargo. La trace est conservée, aucun effacement ou restauration. L'utilisateur a ensuite autorisé explicitement pour B la même dérogation Temp que A : consignation UNATTRIBUTED sans arrêt à elle seule pour les événements sans attribution démontrée, aucune destination volontaire supplémentaire ; toute écriture attribuée à la campagne ou autre violation impose l'arrêt. Les auto-tests ont été repris et réussis avant la baseline. L'accord `.git/objects/` pour tous les lots reste inchangé.

### Résultats réellement exécutés

Baseline puis candidat : format, contrôle workspace, Clippy avec les quatre lints de structure, découverte et cinq suites séquentielles PASS via la garde. Clippy : 142 avertissements en baseline comme sur le candidat, aucun avertissement nouveau introduit par B. `git diff --check` PASS.

T-B : `stop_before_commit_atomically_prevents_publication` et `stop_during_blocked_commit_returns_promptly_and_reports_finalizing` exécutés et réussis avant/après. Les deux nouveaux tests privés sont également exécutés et réussis. Les découvertes du candidat ont exactement deux tests de plus que la baseline.

| Configuration séquentielle | Réussis baseline | Réussis candidat | Ignorés | Filtrés autorisés | Résultat |
| --- | --- | --- | --- | --- | --- |
| défaut | 266 | 268 | 16 | 1 D2 | PASS |
| recette-hooks | 270 | 272 | 16 | 1 D2 | PASS |
| l01-memory-qualification | 266 | 268 | 16 | 1 D2 | PASS |
| personal-qualification | 266 | 268 | 16 | 1 D2 | PASS |
| recette-hooks,l3-gpu-qualification | 270 | 272 | 16 | 2 D2/l3c | PASS |

Trois contrôles conditionnels non natifs ont été exécutés isolément avec variables et ressources bornées et réussissent : `paths_reject_a_worker_outside_the_runtime_root`, `over_limit_parakeet_import_is_refused_before_pending_creation` et `injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume`. L'index des commandes/résultats est `target/parakeet-tests/refacto-worker-ipc-b/logs/guard.jsonl`, avec les journaux stdout/stderr associés.

Les 16 tests ignorés, les exclusions D2/l3c déjà acceptées et les campagnes natives/Windows ne sont pas revendiqués exécutés. Le test `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` reste NOT RUN car il créerait un lien physique vers le modèle hors racine autorisée ; le reviewer l'a jugé sans lacune matérielle pour B/REQ-02. Aucun nouveau skip ajouté pour obtenir le résultat ; cette clôture structurelle ne vaut pas qualification native ou recette Windows.

### Revue et archivage

Le reviewer indépendant rend **CLEAN** sur le candidat working tree, epoch 0, après comparaison avec la base exacte : une passe de revue, zéro correction et zéro demande de vérification complémentaire. Aucun constat ouvert, aucune lacune requise ni blocage dans le périmètre du lot. La machine, les cinq constantes, les transitions atomiques et les témoins ont été vérifiés ; les limites hors périmètre sont conservées ci-dessus.

Archive : `target/parakeet-tests/refacto-worker-ipc-b/archive/closure-20261029/proofs.zip`, 75 entrées, 296 006 octets. Arborescence relative préservée ; nombre et unicité vérifiés, SHA-256 de chaque entrée comparé à sa source. SHA-256 du ZIP : `7DBAE43DDA0C74C47FF08AE746BFB5550A0991B1987ECCCFED95435B82B2690A`. Aucun manifeste créé, aucun cache/build/WAV/modèle archivé, aucune suppression ni nettoyage (absence de manifeste de suppression approuvé), cargo-shared conservé.

Espace libre final : 120 848 478 208 octets (environ 112,6 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan conservés dans le commit local de clôture ; aucun push. B est clos ; C nécessite sa propre autorisation.
## 17. Démarrage autorisé du lot C — 29 octobre 2026

Avancée : prérequis B clos CLEAN sur `fbe2ca7749d6aa13ff9fbb658b23e687fbdfac18` ; C autorisé.
Blocage : aucun obstacle documentaire constaté ; garde et baseline C à vérifier avant extraction.
Décision : exécuter C seul, revue indépendante puis commit local sans push si clôture sans réserve démontrée.

Accord écrit de la session : « Passe au lot C selon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » Périmètre production : `worker_ipc.rs`, `worker_ipc/service/mod.rs`, `worker_ipc/service/automatic_final.rs` ; tests directement correspondants. Extraire AsyncImportIo, le contrat ServiceRuntime, service_loop et les helpers d'export automatique selon C, en conservant le câblage minimal de l'impl ServiceRuntime sur ImportRuntime, les compteurs partagés uniques et le chemin public AsyncImportIo. Aucun nouveau comportement, changement d'admission/bornes/attente/Start/Stop, dépendance, feature ou manifeste.

Racine de campagne C : `target/parakeet-tests/refacto-worker-ipc-c/`, copie adaptée de la garde existante avec cache, temporaires et sorties sous C ; cible Cargo partagée inchangée. Préflight, auto-tests et surveillance existante obligatoires, sans nouvelle isolation ou surveillance. La seule dérogation généralisée aux lots est celle des événements non attribués dans `.git/objects/` ; les dérogations Temp limitées à A et B ne s'étendent pas à C. Les scénarios natif D2 et historique l3c restent hors de la campagne non native déjà délimitée ; aucun nouveau skip ni campagne native/Windows implicite. Autorisation de reproduire les échecs conservée, sans contourner une violation hors racine non couverte. Espace libre initial : 120 439 889 920 octets (environ 112,2 Gio). Non suivis préexistants préservés.
## 18. Clôture du lot C — 29 octobre 2026

Avancée : service asynchrone et helpers d'export final extraits, vérifications réussies et revue indépendante CLEAN.
Blocage : aucun dans le périmètre C/REQ-02/03 après disposition explicite de l'incident de garde.
Décision : lot C clos sans réserve dans son périmètre autorisé ; commit local sans push, aucun lot D1 démarré.

### Delta et préservation des contrats

Base de la revue et de la campagne : `31ff24fe2820acfce72afae079ec3b740884f725`, après B clos. Implémentation déléguée à `rust_implementer`, revue par `rust_review_coordinator` et le même reviewer indépendant en lecture seule ; la session principale n'a pas modifié le code en parallèle.

Trois fichiers production : façade `worker_ipc.rs`, nouveaux `worker_ipc/service/mod.rs` (833 lignes physiques) et `worker_ipc/service/automatic_final.rs` (320). AsyncImportIo, impl ImportIoPort, contrat ServiceRuntime et service_loop appartiennent au service ; exports/status/stages/nettoyage de fin automatique et NEXT_AUTO_EXPORT_STAGE à automatic_final. L'impl ServiceRuntime sur ImportRuntime reste câblée minimalement. Le chemin public AsyncImportIo est conservé par réexport, les visibilités internes restent limitées et NEXT_IMPORT_JOB_ID vient du propriétaire support unique.

Le reviewer a comparé les corps déplacés à la base : réservations, bornes, Start/Stop, déconnexion, ACK et chemin de publication conservés ; `EVENTS_PER_STAGE` reste 21 et l'attente 5 ms. Aucun compteur dupliqué, canal bloquant ajouté, changement fonctionnel, dépendance, feature, manifeste ou Cargo.lock modifié. Les nouveaux modules restent sous la limite de 1 500 lignes ; noms et nombre de tests inchangés.

### Campagne et dispositions des incidents

Garde existante adaptée et utilisée sous `target/parakeet-tests/refacto-worker-ipc-c/`, cache, fixture et temporaires sous C, compilation sous la seule cible Cargo partagée canonique. Préflight, auto-tests et surveillance ont été exécutés ; commandes offline/locked, aucun téléchargement. Les premiers arrêts n'ont pas été convertis en succès :

- Événement Temp créé à 17:54:38.060 +01:00 : origine indéterminée, garde initialement stricte, arrêt avant extraction. L'utilisateur a ensuite autorisé explicitement la dérogation Temp pour C, les événements non attribués étant consignés sans arrêt à eux seuls. Aucune destination volontaire hors racine ni écriture attribuée à la campagne autorisée.
- Événement Downloads supprimé à 18:22:15.501 +01:00 : fichier `~$Business Case Profit Recovery - Leonardo EN.pptx`, origine indéterminée. Après arrêt, l'utilisateur a autorisé la reprise et demandé de ne pas interrompre pour les fichiers extérieurs sans lien projet. La garde consigne les documents personnels concernés `UNATTRIBUTED_EXTERNAL`, avec auto-tests de refus maintenus pour les fichiers Whisper, modèles, cache Cargo, index et renommages entre zones.
- Événement `.git/index.lock` créé à 18:46:22.025 +01:00, après réussite des vérifications prévues, pendant un contrôle conditionnel supplémentaire. L'implémenteur avait lancé un `git status --short` concurrent de cette commande gardée, contrairement à la règle de coordination ; son lien exact avec l'événement n'est pas démontré. Arrêt immédiat, index sans diff, verrou absent et aucun Cargo actif aux contrôles suivants. L'utilisateur a autorisé précisément la reprise et la clôture après cet incident, trace conservée et protection de l'index inchangée. Cette disposition ne crée aucune dérogation générale de l'index et ne permet aucune lecture Git concurrente d'une campagne future.

Les journaux des événements sont conservés sous `logs/guard-events.jsonl` et les résultats sous `logs/guard.jsonl`. Aucun fichier détecté n'a été effacé ou restauré par cette intervention. La dérogation antérieure `.git/objects/` pour les événements sans attribution démontrée reste distincte de l'incident d'index.

### Résultats effectivement obtenus

Baseline puis candidat : format, découverte, cinq témoins T-C exécutés individuellement, cinq suites séquentielles, contrôle workspace et Clippy PASS via la garde. `git diff --check` PASS. Clippy : 142 avertissements en baseline comme sur le candidat, aucun nouvel avertissement introduit.

Les cinq témoins T-C couvrent les bornes de file et de persistance, la distinction saturation/déconnexion, l'admission Stop sous saturation, l'ordre Start/Stop et l'absence de démarrage tardif d'un travail arrêté.

| Configuration séquentielle du candidat | Réussis | Échoués | Ignorés | Filtrés autorisés | Résultat |
| --- | --- | --- | --- | --- | --- |
| défaut | 268 | 0 | 16 | 1 D2 | PASS |
| recette-hooks | 272 | 0 | 16 | 1 D2 | PASS |
| l01-memory-qualification | 268 | 0 | 16 | 1 D2 | PASS |
| personal-qualification | 268 | 0 | 16 | 1 D2 | PASS |
| recette-hooks,l3-gpu-qualification | 272 | 0 | 16 | 2 D2/l3c | PASS |

Journaux des cinq suites finales : `Run-20261029T183842365-c1d9aa08`, `Run-20261029T184009314-d5ee7651`, `Run-20261029T184139902-f06557ba`, `Run-20261029T184307154-926871cd`, `Run-20261029T184434384-7aaee56d` sous `target/parakeet-tests/refacto-worker-ipc-c/logs/` (stdout/stderr associés).

Deux contrôles conditionnels complémentaires ont été exécutés isolément et réussis : `paths_reject_a_worker_outside_the_runtime_root` et `injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume` (ce dernier : `Run-20261029T184549092-d8de94e1`, code 0). Le contrôle `over_limit_parakeet_import_is_refused_before_pending_creation` a été interrompu par la garde (`Run-20261029T184550198-5220fa3b`, code 1) : il n'est pas déclaré PASS. Le reviewer a établi qu'il exerce RuntimeModelCatalog::bind_import inchangé et ne crée pas de lacune matérielle pour C/REQ-02/03 ; aucune répétition inutile demandée après disposition de l'incident.

Le test D4 de reprise qui créerait un lien physique vers un modèle hors racine reste NOT RUN et hors critère C. Les 16 tests ignorés, les exclusions D2/l3c déjà convenues, les campagnes natives et les essais dans la fenêtre Windows ne sont pas revendiqués exécutés. Aucun nouveau skip n'a été ajouté pour obtenir la clôture. Celle-ci porte sur le périmètre structurel C et ne vaut pas qualification native/Windows.

### Revue finale et conservation

Premier paquet de revue : aucun défaut de code actionnable, blocage uniquement sur la disposition de l'incident d'index. Après l'accord utilisateur précis et sans nouvelle mutation du candidat, le même reviewer rend **CLEAN** : aucun constat ouvert, aucune vérification requise manquante et aucun blocage dans C. Aucun cycle correctif nécessaire ; le changement reste un déplacement littéral avec adaptation du câblage interne.

Archive : `target/parakeet-tests/refacto-worker-ipc-c/archive/closure-20261029/proofs.zip`, 112 entrées uniques, 356 565 octets. Arborescence relative conservée ; nombre, unicité et SHA-256 de chaque entrée comparés aux sources. SHA-256 du ZIP : `B511112F54E3891A02DFF70A4E5A596727699948F5A183B687A25B81203FC821`. Aucun cache/build/modèle/WAV inclus et aucun manifeste d'empreintes créé. Aucun nettoyage ni suppression sans manifeste de suppression approuvé ; cargo-shared et entrées préexistantes conservés.

Espace libre final : 119 998 652 416 octets (environ 111,8 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan conservés dans le commit local de clôture, aucun push. C est clos ; aucun lot suivant n'est autorisé par cette clôture.
## 19. Démarrage autorisé du lot D1 — 29 octobre 2026

Avancée : prérequis C clos CLEAN sur `8eb5bbb8c97c9287d4f7f1c4f189c5759d5cbde0` ; D1 autorisé.
Blocage : aucun obstacle documentaire constaté ; préflight et baseline D1 requis avant extraction.
Décision : exécuter uniquement D1, revue indépendante puis commit local sans push si clôture démontrée.

Accord écrit de la session : « Passe au lot D1 selon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » Périmètre : `worker_ipc.rs`, `worker_ipc/runtime/mod.rs` et `worker_ipc/runtime/queue.rs`, avec tests directement correspondants. Déplacer le propriétaire ImportRuntime, ses constructeurs et injections existantes sans changer son état ; déléguer les seuls bras Enqueue, RemoveQueued, ProcessQueued et ScanQueue. Conserver identités, validations, FIFO, refus avant préparation, préambule/épilogue et retours précoces du dispatcher. ConfirmDuplicateImport reste pour D2. Aucune nouvelle API, dépendance, feature ou modification de manifeste.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-d1/` et cible Cargo partagée canonique ; copie adaptée de la garde existante, cache, fixture, temporaires et sorties sous D1. Préflight, auto-tests, surveillance et commandes offline/locked obligatoires, sans nouvelle surveillance ou isolation. La consigne de session de consigner sans arrêt les événements sur fichiers extérieurs sans lien démontré avec le projet reste applicable, ainsi que l'accord `.git/objects/` pour tous les lots. Elle n'autorise aucune destination applicative volontaire hors racines ni écriture attribuée à la campagne ; sources, index/HEAD/références/configuration, modèles, corpus, cache Cargo et ressources Whisper restent protégés. La disposition de l'incident index C est ponctuelle, sans exception générale ; aucune lecture Git concurrente d'une commande gardée. Les dispositions hors périmètre D2 natif/l3c et D4 hardlink restent explicites, sans nouveau skip ni campagne native/Windows implicite. Espace libre initial : 120 403 664 896 octets (environ 112,1 Gio). Non suivis préexistants préservés.
## 20. Clôture du lot D1 — 29 octobre 2026

Avancée : état ImportRuntime et quatre traitements de file extraits, vérifications réussies et revue indépendante CLEAN.
Blocage : aucun dans le périmètre D1/REQ-03/04.
Décision : lot D1 clos sans réserve dans son périmètre autorisé ; commit local sans push, aucun lot D2 démarré.

### Delta et invariants

Base Git exacte : `07554be8caf7c0e7a609b9dceafc33784c691775`, après C clos. Implémentation déléguée à `rust_implementer`, revue par le coordinateur et le même reviewer indépendant en lecture seule. La session principale n'a pas modifié le code en parallèle.

Trois fichiers production : façade `worker_ipc.rs`, nouveaux `worker_ipc/runtime/mod.rs` (325 lignes physiques) et `worker_ipc/runtime/queue.rs` (449). Un seul ImportRuntime avec le même état, ses constructeurs et injections existants ; alias privé dans la façade pour les impls non encore extraits. Champs et méthodes partagés limités à worker_ipc, aucun nouvel export externe. Les quatre bras Enqueue, RemoveQueued, ProcessQueued et ScanQueue délèguent aux méthodes de file sur cet état unique. ConfirmDuplicateImport reste dans la façade pour D2.

Le reviewer a comparé les quatre corps, constructeurs et injections à la base : mutations, FIFO, identités, validations, refus avant préparation et source binding préservés. Les retours demeurent dans les mêmes closures ; préambule, sorties précoces et épilogue de `effect` inchangés. Aucun changement d'algorithme métier, dépendance, feature, manifeste ou Cargo.lock. Les deux nouveaux modules respectent le plafond de 1 500 lignes.

### Caractérisation et résultats exécutés

L'inventaire ne montrait pas de témoin direct des effets RemoveQueued et ScanQueue dans les tests worker_ipc existants. Une caractérisation privée séparée a donc été ajoutée dans `tests/queue_characterization.rs`, déclarée par une ligne dans `tests.rs`. Le test `queue_effect_scan_and_remove_preserve_remaining_fifo_entry` exerce lecture, retrait et nouvelle lecture persistée en vérifiant l'entrée FIFO restante. Exécution avant extraction PASS (`Run-20261029T191356043-7456a453`), puis après extraction PASS (`Run-20261029T191506713-c8288643`).

Témoins complémentaires existants : les deux refus GPU/capacité du plan, `pending_compute_change_persists_only_the_choice_and_keeps_fifo` et `pending_compute_change_cannot_skip_the_fifo_head`, tous exécutés et réussis avant/après. Les cinq témoins ciblés passent. Découverte : exactement un test de caractérisation supplémentaire, sans renommage ou perte de scénario existant.

Baseline et candidat : format, contrôle workspace et cinq suites séquentielles PASS via garde, `git diff --check` PASS. Les cinq suites du candidat rapportent :

| Configuration séquentielle | Réussis baseline | Réussis candidat | Échoués | Ignorés | Filtrés autorisés |
| --- | --- | --- | --- | --- | --- |
| défaut | 268 | 269 | 0 | 16 | 1 D2 |
| recette-hooks | 272 | 273 | 0 | 16 | 1 D2 |
| l01-memory-qualification | 268 | 269 | 0 | 16 | 1 D2 |
| personal-qualification | 268 | 269 | 0 | 16 | 1 D2 |
| recette-hooks,l3-gpu-qualification | 272 | 273 | 0 | 16 | 2 D2/l3c |

Clippy réussit avec 144 avertissements contre 142 en baseline. Les deux nouveaux diagnostics `too_many_lines` sont les handlers Enqueue (178 lignes) et ProcessQueued (217), corps hérités déplacés à l'identique. Aucun `allow` ajouté. Le plan autorise l'attribution des longs corps déplacés comme dette héritée ; le reviewer a confirmé cette équivalence et n'en fait pas un constat actionnable de D1. Les cibles de taille des fonctions ne sont donc pas revendiquées atteintes pour ces deux corps, et aucune refonte fonctionnelle n'est ajoutée à ce lot.

Trois contrôles conditionnels non natifs isolés ont été exécutés avec variables bornées et réussissent : `paths_reject_a_worker_outside_the_runtime_root`, `injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume` et `over_limit_parakeet_import_is_refused_before_pending_creation` (journaux `Run-20261029T192330813-31d5e419`, `Run-20261029T192331980-892e0485`, `Run-20261029T192333124-b6436116`). Le scénario de reprise D4 impliquant un hardlink sur le modèle extérieur reste NOT RUN, hors critères D1. Les 16 tests ignorés, les exclusions D2 natif/l3c déjà convenues et les essais natifs/Windows ne sont pas revendiqués exécutés ; aucun nouveau skip ajouté.

Journaux des cinq suites finales : `Run-20261029T191619097-5b34454c`, `Run-20261029T191748391-e91156f3`, `Run-20261029T191914219-77655504`, `Run-20261029T192043003-eac93aa3`, `Run-20261029T192211622-3029e9f4`, sous `target/parakeet-tests/refacto-worker-ipc-d1/logs/` avec stdout/stderr et index `guard.jsonl`.

### Garde, revue et conservation

Campagne sous la racine D1, cache/fixture/temporaires et sorties bornés, unique cible Cargo partagée canonique, préflight et auto-tests de refus avant Cargo, commandes offline/locked et surveillance existante. Un événement Created Temp `etilqs_y8iri21xqko1SJH` à 19:03:58 a d'abord interrompu un témoin ; ce résultat reste non acquis et sa trace est conservée, sans attribution à la campagne ni suppression/restauration. La session principale a ensuite appliqué la directive utilisateur générale de cette même session de ne pas interrompre pour les fichiers Downloads ou autres extérieurs sans lien démontré avec le projet : ces événements non attribués sont consignés sans arrêt à eux seuls. Les auto-tests ont confirmé le maintien de l'arrêt pour les fichiers suspects ou attribués à la campagne et les ressources du projet. Aucune destination applicative volontaire hors racines n'est autorisée. Les protections sources, index/HEAD/références/configuration, modèles, corpus, ressources Whisper et cache Cargo restent actives ; aucune lecture Git pendant les commandes gardées. Aucun mécanisme de surveillance ou d'isolation supplémentaire créé.

Le reviewer indépendant rend **CLEAN** sur le candidat working tree, epoch 0 : une passe de revue, zéro correction, aucun constat ouvert, aucune vérification requise manquante ni blocage dans D1. La dette héritée des deux longs corps est attribuée explicitement ci-dessus conformément au plan ; la clôture ne prétend ni la supprimer ni valider le produit nativement ou dans Windows.

Archive : `target/parakeet-tests/refacto-worker-ipc-d1/archive/closure-20261029/proofs.zip`, 88 entrées uniques, 313 468 octets. Arborescence relative conservée, compte/unicité et SHA-256 de chaque entrée vérifiés contre les sources. SHA-256 du ZIP : `A0700399C3CE39AD22853559CEB5CBCE0063BA5306CCB179494E785284B22442`. Aucun cache/build/modèle/WAV inclus, aucun manifeste créé, aucune suppression ni nettoyage sans manifeste de suppression approuvé ; cargo-shared conservé.

Espace libre final : 119 496 704 000 octets (environ 111,3 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan conservés dans le commit local de clôture ; aucun push. Aucun lot D2 n'est autorisé par cette clôture.
## 21. Démarrage autorisé du lot D2 — 29 octobre 2026

Avancée : D1 clos CLEAN et déjà commité sur `1a6bb628e7c26e882083a59f89b97dfb58ef06e7` ; D2 autorisé.
Blocage : aucun obstacle documentaire constaté ; préflight et baseline requis avant extraction.
Décision : exécuter uniquement D2 selon le plan, revue indépendante puis commit local sans push.

Accord écrit de la session : « Ok commit et passe au D2 selon le plan ». Le commit D1 est acquis ; cette demande autorise la suite D2 avec les vérifications et la revue prévues, sans modifier l'interdiction de push. Périmètre : `worker_ipc.rs`, `worker_ipc/runtime/mod.rs` et `worker_ipc/runtime/duplicates.rs`, tests directement correspondants. Extraire ConfirmDuplicateImport, confirm_interrupted_publication, résolution des cibles et revalidation des publications. Préserver prompts, choix dernier/tous/cible exacte, refus/annulation, ordre des mutations et source bindings, retours courts, différence entre revalidation proposant un prompt et erreur terminale. Ne pas exécuter un épilogue auparavant évité et ne pas fusionner les politiques fichier/direct.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-d2/`, cible Cargo partagée canonique inchangée, copie adaptée de la garde existante, cache/fixture/temporaires et sorties bornés sous D2. Préflight, auto-tests, surveillance et commandes offline/locked requis. La directive utilisateur générale sur les fichiers extérieurs sans lien démontré avec le projet et l'accord `.git/objects/` pour tous les lots restent applicables ; aucune destination volontaire hors racines, écriture attribuée à la campagne ou modification de ressource projet n'est autorisée. Protection index/HEAD/références/configuration inchangée, aucune lecture Git concurrente d'une commande gardée. Exclusions D2 natif/l3c et limites hors périmètre D4 hardlink conservées explicitement, sans nouveau skip ni qualification native/Windows implicite. Espace libre initial : 119 925 055 488 octets (environ 111,7 Gio). Non suivis préexistants préservés.
## 22. Clôture du lot D2 — 30 octobre 2026

Avancée : traitement des doublons extrait, vérifications réussies et revue indépendante CLEAN.
Blocage : aucun dans le périmètre D2 autorisé.
Décision : D2 clos sans réserve dans ce périmètre ; commit local sans push, aucun lot D3 démarré.

### Autorité et delta

Après le GO initial du §21, l'utilisateur a confirmé explicitement l'implémentation, les vérifications, le commit local sans push et la clôture sans réserve après revue indépendante. Base exacte : `7ee653ccb7033679561c460a3ed4d9645b0c8eaa`, après D1 clos. Implémentation déléguée au profil prévu, revue par le coordinateur et le même reviewer indépendant ; la session principale n'a pas modifié le code en parallèle.

Trois fichiers production : `worker_ipc.rs`, déclaration dans `runtime/mod.rs`, nouveau `runtime/duplicates.rs` (447 lignes physiques). ConfirmDuplicateImport délègue à un seul handler ; confirm_interrupted_publication, resolve_reprocessing_targets et verified_present_publications ont leur propriétaire sous duplicates. Les helpers restent privés à worker_ipc ; aucune nouvelle API externe, dépendance, feature, modification de manifeste ou Cargo.lock. La politique directe n'est pas fusionnée avec celle du doublon fichier.

Le résultat interne `DuplicateEffect::Return` représente les deux retours du dispatcher préexistants : confirmation interrompue et refus CreateNew sans registre. `Continue(result)` préserve les chemins passant par l'épilogue ; les retours de revalidation ou d'erreur restent dans leurs closures d'origine. Le reviewer a vérifié les mutations, source bindings, identités, prompts, décisions et distinction entre revalidation proposant un prompt et erreur terminale. Aucun succès de handler n'active un épilogue auparavant évité.

### Baseline, reprise et campagne

Campagne sous `target/parakeet-tests/refacto-worker-ipc-d2/`, cache/fixture/temporaires et sorties sous D2, cible Cargo partagée canonique, préflight, auto-tests et surveillance existante, commandes offline/locked. Aucun téléchargement ni nouvelle isolation/surveillance.

Le 29 octobre, la suite défaut de baseline avait échoué sur trois scénarios Auto utilisant réellement le microphone Windows par défaut, absent. Un échec de nettoyage de répertoire était aussi survenu puis avait réussi isolément. Ces résultats restent conservés. Le 30 octobre, l'utilisateur a confirmé « Microphone rétabli, tu peux reprendre » ; la suite défaut puis les autres configurations ont réussi, les trois scénarios Auto inclus sans exclusion ni correction de code microphone.

Un événement Created dans Downloads sur un GUID.tmp à 08:16:43 a interrompu recette-hooks (`Run-20261030T081616295-8b50dd01`), sans attribution à la campagne ni lien projet démontré. La garde, encore limitée aux documents Office/PDF, a été alignée sur la consigne utilisateur générale déjà donnée de ne pas interrompre pour les fichiers extérieurs sans lien projet, sans limitation d'extension. Les fichiers non attribués concernés sont consignés `UNATTRIBUTED_EXTERNAL` ; fichiers suspects/attribués, ressources Whisper, cache Cargo, modèles, corpus, sources, index/HEAD/références/configuration et renommages entre zones restent protégés. Auto-tests du GUID synthétique et des refus projet réussis ; recette-hooks relancée PASS. Aucune destination volontaire hors racines ni effacement/restauration de l'événement autorisé.

Un relevé Git avait été lancé par erreur en parallèle de l'auto-test initial de reprise, sans Cargo alors et sans incident d'index observé. La séquence a été corrigée avant la suite des campagnes : aucun Git pendant une commande gardée, y compris SelfTest ; aucun assouplissement de protection de l'index. Les auto-tests et les événements historiques restent archivés, sans conversion des commandes interrompues en succès.

### Vérifications exécutées

Les trois témoins T-D2 du plan, les parcours de retraitement et de reprise unifiée ont été exécutés et réussis avant/après. Une caractérisation privée séparée a été ajoutée dans `tests/duplicate_characterization.rs`, déclarée dans `tests.rs` : refus courts, identité et événements, avant extraction (`Run-20261030T082325938-ccf50c55`) puis après (`Run-20261030T082435738-0c6c3e68`), codes 0. Six témoins ciblés candidat réussis. Le témoin d'annulation du cœur `cancelling_duplicate_import_releases_only_the_new_source_snapshot` réussit (`Run-20261030T083302959-e911e1e8`).

Baseline et candidat : format, contrôle workspace, cinq suites séquentielles et `git diff --check` PASS. Les découvertes comptent exactement un test supplémentaire de caractérisation, sans perte de scénario.

| Configuration séquentielle | Réussis baseline | Réussis candidat | Échoués | Ignorés | Filtrés autorisés |
| --- | --- | --- | --- | --- | --- |
| défaut | 269 | 270 | 0 | 16 | 1 D2 natif |
| recette-hooks | 273 | 274 | 0 | 16 | 1 D2 natif |
| l01-memory-qualification | 269 | 270 | 0 | 16 | 1 D2 natif |
| personal-qualification | 269 | 270 | 0 | 16 | 1 D2 natif |
| recette-hooks,l3-gpu-qualification | 273 | 274 | 0 | 16 | 2 D2 natif/l3c |

Clippy réussit : 145 avertissements contre 144 en baseline. Le nouveau `too_many_lines` concerne le corps de confirmation hérité extrait, 226 lignes pour une cible de 100 ; aucun `allow` ajouté. Le signalement préexistant de confirm_interrupted_publication (110 lignes) est conservé. Le reviewer confirme l'attribution en dette héritée permise par le plan, sans constat actionnable D2 ; l'atteinte du seuil de taille n'est pas revendiquée pour ce corps déplacé.

Trois contrôles conditionnels non natifs isolés ont été réellement exécutés avec ressources bornées et réussissent : chemins de runtime, routage de catalogue et refus d'import trop long (`Run-20261030T083311086-ef04aa3a`, `Run-20261030T083312482-cb42fba2`, `Run-20261030T083313792-ad705178`). Le scénario de reprise legacy impliquant un hardlink sur le modèle extérieur reste NOT RUN et hors critères D2. Les 16 tests ignorés, les exclusions D2 corpus natif/l3c déjà convenues, les campagnes natives et la fenêtre Windows ne sont pas revendiqués exécutés. Aucun nouveau skip ajouté pour clôturer ce lot de refactorisation D2.

Journaux des cinq suites finales : `Run-20261030T082546994-51891753`, `Run-20261030T082716109-e8ce41c0`, `Run-20261030T082845010-77274ead`, `Run-20261030T083014671-621154a6`, `Run-20261030T083144167-25eb7924`, sous `target/parakeet-tests/refacto-worker-ipc-d2/logs/` avec stdout/stderr et index `guard.jsonl`.

### Revue et conservation

Le reviewer indépendant rend **CLEAN** sur le candidat working tree, epoch 0 : une passe de revue, zéro correction, aucun constat ouvert, aucune vérification requise manquante ni blocage dans D2. Les retours courts, les closures, les identités et les mutations restent identiques ; la dette de taille héritée est explicitement attribuée ci-dessus conformément au plan. La clôture porte sur ce périmètre structurel, sans qualification native/Windows implicite.

Archive : `target/parakeet-tests/refacto-worker-ipc-d2/archive/closure-20261030/proofs.zip`, 100 entrées uniques, 354 755 octets. Arborescence relative conservée, nombre/unicité et SHA-256 de chaque entrée comparés à sa source. SHA-256 du ZIP : `81C63A06BDCFBBC3BCF3328D7A3819D6567094453BEE76ABFCF1EE24AB53185C`. Aucun cache/build/modèle/WAV archivé, aucun manifeste créé ni nettoyage/suppression sans manifeste de suppression approuvé ; cargo-shared conservé.

Espace libre final : 116 739 747 840 octets (environ 108,7 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan conservés dans un commit local de clôture, aucun push. Aucun lot D3 autorisé par cette clôture.
## 23. Démarrage autorisé du lot D3 — 30 octobre 2026

Avancée : D2 clos CLEAN sur `c6844a8519babe7fa005731d40d5b9df7adf99c8` ; D3 autorisé.
Blocage : aucun obstacle documentaire constaté ; préflight et baseline requis avant extraction.
Décision : exécuter uniquement D3, revue indépendante puis commit local sans push si clôture démontrée.

Accord écrit de la session : « Passe au lot D3 selon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » Périmètre : `worker_ipc.rs`, `runtime/mod.rs`, `runtime/live.rs`, subdivision autorisée `runtime/live/capture.rs` si nécessaire ; quatre fichiers production maximum et tests directement correspondants. Extraire les huit bras StartMicrophoneTest, StartDirectLive, StartUnifiedLive, StartLive, SetMicrophoneGain, FinishLiveOnCpu, StopUnifiedLive et StopMicrophoneTest, avec helpers live/capture/activation/tentative/replay nécessaires. Conserver primitives moteur jusqu'à F1 et source/exécuteur unifiés jusqu'à F2. Stop générique partagé reste D5. Préserver retours de closures et du dispatcher, événements différés, capture/durabilité/ACK, identités, gain, ordre Start/Stop et politique de repli existante. Pas de réécriture des modules capture.rs/direct_live.rs extérieurs ni nouvelle API, dépendance, feature ou manifeste.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-d3/`, copie adaptée de la garde existante, cache/fixture/temporaires et sorties bornés sous D3, cible Cargo partagée canonique inchangée. Préflight, auto-tests, surveillance, commandes offline/locked et séquencement sans Git pendant commande gardée obligatoires. Les consignes de session sur les fichiers extérieurs non attribués sans lien démontré avec le projet et sur `.git/objects/` restent applicables ; aucune destination volontaire hors racines, écriture attribuée à la campagne ou modification de ressource projet n'est autorisée. Index/HEAD/références/configuration, sources, ressources Whisper, modèles, corpus et cache Cargo restent protégés. Exclusions D2 corpus natif/l3c et limite hors périmètre D4 hardlink explicites, aucun nouveau skip ni campagne native/Windows implicite. Espace libre initial : 116 398 514 176 octets (environ 108,4 Gio). Non suivis préexistants préservés.
## 24. Clôture du lot D3 — 30 octobre 2026

Avancée : huit traitements direct/micro et helpers associés extraits, vérifications réussies et revue indépendante CLEAN.
Blocage : aucun dans le périmètre D3 autorisé.
Décision : D3 clos sans réserve dans ce périmètre ; commit local sans push, aucun lot D4 démarré.

### Delta et contrats préservés

Base Git exacte : `7b895ae871d48a3e785e7ce645200437a1f82d53`, après D2 clos. Implémentation déléguée au profil prévu ; revue par le coordinateur et le même reviewer indépendant en lecture seule. La session principale n'a pas modifié le code en parallèle.

Trois fichiers production : `worker_ipc.rs`, déclaration dans `runtime/mod.rs` et nouveau `runtime/live.rs` (968 lignes physiques). La subdivision capture autorisée n'a pas été nécessaire. Les huit bras StartMicrophoneTest, StartDirectLive, StartUnifiedLive, StartLive, SetMicrophoneGain, FinishLiveOnCpu, StopUnifiedLive et StopMicrophoneTest délèguent. MicrophoneTestCapture, spawn_live_attempt, start_live, activate_live_after_ready, les tentatives CPU liées au direct et finish_live_on_cpu sont sous live. Stop générique, polling et primitives prévues pour F1/F2 restent en place. Aucun fichier capture.rs/direct_live.rs extérieur, test, dépendance, feature, manifeste ou Cargo.lock modifié.

Le résultat interne LiveEffect conserve les deux retours anticipés du dispatcher StartDirectLive ; les returns de StartUnifiedLive restent locaux aux mêmes closures. Le reviewer a comparé les corps : préambule/épilogue, mutations, état unique ImportRuntime, identités, activation, réservation/remapping, événements différés, gain et ACK de persistance préservés. Aucun repli ajouté ni politique de sélection moteur modifiée. Visibilités limitées à worker_ipc, aucun nouvel export externe.

### Vérifications avant/après

Préflight, auto-tests et surveillance de la garde sous `target/parakeet-tests/refacto-worker-ipc-d3/`, cache/fixture/temporaires et sorties sous D3, cible Cargo partagée canonique, commandes offline/locked. Consignes existantes sur les événements extérieurs non attribués et objets Git conservées ; protections des fichiers liés au projet inchangées. Aucun Git concurrent des commandes gardées.

Le premier check d'extraction avait relevé deux champs de capture encore privés pour le polling historique. Leur visibilité interne a été ajustée, puis un import réservé aux tests borné à cfg(test), avant les contrôles finaux. Les échecs intermédiaires restent dans les preuves ; l'état final et tous ses contrôles sont distingués, sans prétendre au succès des premières commandes en erreur.

Six témoins ciblés ont réellement passé avant et après : les trois T-D3 nommés du plan (ordre Start/Stop, attente des ACK de persistance, gain et cible obsolète), le Start/Stop immédiat, les deux captures/finalisations concurrentes et la fermeture globale. Un témoin supplémentaire de capture MicrophoneTestCapture réussit après extraction. Les tests existants sont conservés sans renommage, retrait ou ajout ; les découvertes et comptes avant/après sont identiques.

| Configuration séquentielle | Réussis baseline et candidat | Échoués | Ignorés | Filtrés autorisés | Résultat |
| --- | --- | --- | --- | --- | --- |
| défaut | 270 | 0 | 16 | 1 D2 corpus natif | PASS |
| recette-hooks | 274 | 0 | 16 | 1 D2 corpus natif | PASS |
| l01-memory-qualification | 270 | 0 | 16 | 1 D2 corpus natif | PASS |
| personal-qualification | 270 | 0 | 16 | 1 D2 corpus natif | PASS |
| recette-hooks,l3-gpu-qualification | 274 | 0 | 16 | 2 D2 corpus natif/l3c | PASS |

Format, `git diff --check`, contrôle workspace et Clippy PASS. Clippy final : 145 avertissements, comme la baseline. Le reviewer a comparé les messages au-delà de leur nombre : aucun nouveau lint ; les seuls changements de diagnostics hérités sont la réduction de taille du dispatcher (1 247 à 969 lignes selon Clippy) et de sa complexité cognitive (82 à 63). Les cibles structurelles globales ne sont pas revendiquées atteintes pour tous les corps historiques, et aucun avertissement n'est masqué.

Trois contrôles conditionnels non natifs isolés sont exécutés avec ressources bornées et réussissent : chemins de runtime, routage de catalogue, refus d'import trop long (`Run-20261030T090413525-e696342f`, `Run-20261030T090414746-426d36b3`, `Run-20261030T090415969-356de6a6`). Le test D4 impliquant un hardlink vers le modèle extérieur reste NOT RUN, hors critères D3. Les 16 tests ignorés, exclusions D2 corpus natif/l3c et campagnes natives/Windows ne sont pas revendiqués exécutés. Aucun nouveau skip.

Journaux des cinq suites finales : `Run-20261030T085710879-343eb8da`, `Run-20261030T085836840-a86cd7fd`, `Run-20261030T090007302-032db68c`, `Run-20261030T090135885-c09fba85`, `Run-20261030T090300870-e06e18d0`, sous `target/parakeet-tests/refacto-worker-ipc-d3/logs/` avec stdout/stderr et index `guard.jsonl`.

### Revue et conservation

Le même reviewer indépendant rend **CLEAN** sur le candidat working tree, epoch 0 : une passe de revue, zéro correction, aucun constat ouvert, aucune vérification requise manquante ni blocage dans D3. L'équivalence des huit handlers, des retours anticipés et des helpers live a été confirmée, ainsi que l'absence d'élargissement vers F1/F2. Cette clôture structurelle ne vaut pas qualification native ni recette dans la fenêtre Windows.

Archive : `target/parakeet-tests/refacto-worker-ipc-d3/archive/closure-20261030/proofs.zip`, 100 entrées uniques, 346 820 octets. Arborescence relative conservée ; compte/unicité et SHA-256 de chaque entrée comparés aux sources. SHA-256 du ZIP : `DF1C378CFD972027EE46D31A7B1864F4BD7ABD6A315863F102381B0A43109DD5`. Aucun cache/build/modèle/WAV archivé, aucun manifeste créé, aucun nettoyage ni suppression sans manifeste de suppression approuvé ; cargo-shared conservé.

Espace libre final : 115 486 175 232 octets (environ 107,6 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan conservés dans le commit local de clôture, aucun push. Aucun lot D4 autorisé par cette clôture.
## 25. Démarrage autorisé du lot D4 — 30 octobre 2026

Avancée : D3 clos CLEAN sur `f627cc811a3737aec1d3c8e5f67f6d9dd4cce294` ; D4 autorisé.
Blocage : aucun obstacle documentaire ; fixture legacy à borner avant sa baseline obligatoire.
Décision : exécuter D4 seul, revue indépendante puis commit local sans push si clôture démontrée.

Accord écrit de la session : « Passe au lot D4 selon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » Périmètre : `worker_ipc.rs`, `runtime/mod.rs`, `runtime/recovery.rs`, `runtime/source_binding.rs`, tests directement correspondants. Extraire ReleaseSourceBinding, InspectSourceSnapshotRecovery, ConfirmSourceSnapshotRecovery, ResumeInterrupted et ResumeReplacementCleanup ; méthodes de liaison ensure/rebind/transfer/restore/release, active_snapshot_bindings et inspect_source_snapshot_recovery vers leur propriétaire source_binding. Préserver transferts et restitution sur erreur, checkpoints/générations, fichiers durables, source non libérée prématurément, refus et choix du modèle. Aucun recalcul universel de SHA, politique core déplacée ou nouvelle reprise après panne.

Le test obligatoire `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution`, jusqu'ici hors critères des lots précédents, devient une preuve D4 à exécuter. Sa fixture doit être bornée : copie du modèle réel sous la racine de campagne puis liens physiques entre cette copie canonique et le chemin legacy sous cette même racine. Adapter uniquement le test correspondant et son catalogue/attentes de chemins pour conserver les assertions d'identité, SHA et checkpoint ; ne pas remplacer le lien physique par une simple copie des deux fichiers, ni toucher au modèle installé. Établir cette preuve avant extraction, puis après. Les ressources installées et corpus servent en lecture seule, sans qualification native implicite.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-d4/`, copie adaptée de la garde existante, cache/fixture/temporaires/sorties sous D4, cible Cargo partagée canonique. Préflight, auto-tests, espace libre ≥30 Gio, surveillance, offline/locked et absence de Git concurrent obligatoires. Consignes existantes sur événements extérieurs sans lien projet et `.git/objects/` maintenues ; aucune destination volontaire externe ni écriture attribuée à la campagne autorisée. Sources, modèles, corpus, cache Cargo, ressources Whisper et index/HEAD/références/configuration protégés. Exclusions D2 corpus natif/l3c explicites ; aucune exclusion du test legacy D4, aucun nouveau skip ou campagne native/Windows implicite. Espace libre initial : 116 899 463 168 octets (environ 108,9 Gio). Non suivis préexistants préservés.
## 26. Clôture du lot D4 — 30 octobre 2026

Avancée : reprise et liaisons sources extraites, legacy réellement vérifié, revue indépendante CLEAN.
Blocage : aucun dans le périmètre D4 autorisé.
Décision : D4 clos sans réserve dans ce périmètre ; commit local sans push, aucun lot D5 démarré.

### Delta et préservation des invariants

Base Git exacte : `165471791b734f1676bf6130eb161bc82956640c`, après D3 clos. Implémentation déléguée au profil prévu ; revue par le coordinateur et le même reviewer indépendant en lecture seule. La session principale n'a pas modifié le code en parallèle.

Quatre fichiers production : `worker_ipc.rs`, déclaration dans `runtime/mod.rs`, nouveaux `runtime/recovery.rs` (369 lignes physiques) et `runtime/source_binding.rs` (529). Les cinq bras ReleaseSourceBinding, InspectSourceSnapshotRecovery, ConfirmSourceSnapshotRecovery, ResumeInterrupted et ResumeReplacementCleanup délèguent. Les méthodes de liaison, rebind/transfert/restauration/libération, active_snapshot_bindings et inspect_source_snapshot_recovery sont sous leur propriétaire source_binding, sur le même ImportRuntime.

Le reviewer a comparé les corps déplacés : les treize retours du dispatcher ResumeInterrupted sont mappés sur RecoveryEffect::Return ; les closures et l'épilogue conservent leurs chemins. Transferts, restitution sur erreur, checkpoints, générations, identités, fichiers durables, refus et modèle choisi sont préservés ; aucune source libérée prématurément, aucun recalcul universel de SHA ni politique de recovery du cœur déplacée. Aucune nouvelle reprise après panne, API, dépendance, feature ou modification de manifeste/Cargo.lock. Nouveaux modules sous 1 500 lignes.

### Fixture legacy et modèle installé

Le test `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` n'est plus NOT RUN : son corps est réellement exécuté avant et après extraction et dans les cinq configurations de référence et du candidat. La seule adaptation de test est dans `tests/resume_and_admission.rs` : modèle canonique et catalogue sous la racine D4, lien physique legacy vers cette copie bornée. Une copie unique du modèle réel est réutilisée, sans remplacer la sémantique de lien physique par deux copies indépendantes. Assertions SHA, checkpoint, identité/génération, file et pending conservées, confirmées par le reviewer.

Le modèle installé reste en lecture seule. Son SHA-256 final est identique au préflight : `1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69`, taille 1 624 555 275 octets et date relevée du 6 octobre 2026 à 08:12:43 inchangées. Les liens sont créés entre fichiers D4 ; aucun lien vers le modèle installé ni écriture volontaire sur celui-ci.

Les exécutions isolées legacy avant/après et finale sont réussies : `Run-20261030T092034416-7f5a0a20`, `Run-20261030T094110060-9e712b87`, `Run-20261030T095757567-2c225882`, code 0 avec test exécuté, aucun retour NOT RUN. Après les cinq suites, un reformatage global involontaire du fragment inclus a été retiré : indentation et retours ligne remis comme HEAD, mêmes adaptations de chemins conservées. Le reviewer confirme la sémantique inchangée ; format final et legacy isolé final PASS sur ce dernier état.

### Vérifications obtenues

Préflight, auto-tests et surveillance de la garde D4 appliqués ; cache/fixture/modèle copié/temporaires et sorties sous D4, cible Cargo partagée canonique, commandes offline/locked, aucun Git concurrent. Règles de session sur événements extérieurs sans lien projet et objets Git conservées sans lever les protections des ressources projet. Aucun téléchargement, mécanisme de surveillance ou isolation supplémentaire.

Le témoin R7 copie/reprise/publication passe avant/après. Les témoins `failed_unified_resume_preserves_interrupted_source_binding_and_queue_identity` et `failed_unified_resume_after_binding_transfer_restores_registry_and_queue_identity` sont réellement exécutés, passent isolément sur candidat et dans les suites ; ils couvrent conservation et restitution de liaison/identité en échec. Format, `git diff --check`, workspace et cinq suites séquentielles baseline/candidat PASS ; pas de nouveau test, nom ou compte modifié.

| Configuration séquentielle | Réussis baseline et candidat | Échoués | Ignorés | Filtrés autorisés | Legacy exécuté |
| --- | --- | --- | --- | --- | --- |
| défaut | 270 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks | 274 | 0 | 16 | 1 D2 corpus natif | oui |
| l01-memory-qualification | 270 | 0 | 16 | 1 D2 corpus natif | oui |
| personal-qualification | 270 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks,l3-gpu-qualification | 274 | 0 | 16 | 2 D2 corpus natif/l3c | oui |

Clippy réussit : 146 avertissements contre 145 en référence. Le nouveau `too_many_lines` concerne ResumeInterrupted extrait, corps hérité de 332 lignes pour une cible de 100, sans `allow`. Le reviewer confirme l'équivalence et l'attribution en dette héritée admise par le plan, sans constat actionnable D4. Les diagnostics hérités du dispatcher diminuent de 969 à 584 lignes et de complexité cognitive 63 à 45 ; l'atteinte globale des cibles de structure n'est pas revendiquée.

Les trois contrôles conditionnels non natifs sont acquis : catalogue injecté dans les suites grâce à WHISPER_L1B_RUNTIME_ROOT et les deux autres contrôles ciblés réussis. Les 16 tests ignorés, exclusions D2 corpus natif/l3c déjà convenues, campagnes natives et fenêtre Windows ne sont pas revendiqués exécutés ; aucun nouveau skip. Le legacy D4, désormais requis et exécuté, ne figure plus parmi ces limites hors périmètre.

Journaux des cinq suites finales : `Run-20261030T094247000-c4f22f4c`, `Run-20261030T094527660-ea5d787c`, `Run-20261030T094822356-cac5293d`, `Run-20261030T095120954-a2e3788e`, `Run-20261030T095419486-bc1afc3b`, sous `target/parakeet-tests/refacto-worker-ipc-d4/logs/` avec stdout/stderr et index `guard.jsonl`. Les suites incluent le SHA réel du modèle ; leur durée accrue n'a pas été contournée.

### Revue et conservation

Le reviewer indépendant rend **CLEAN** sur le candidat working tree, epoch 0 : une passe de revue, zéro correction, aucun constat ouvert, aucune vérification requise manquante ni blocage. Il confirme notamment la reprise legacy réellement exécutée, la sémantique du diff final de fixture et les treize sorties précoces. La clôture reste celle de la refactorisation D4 et ne vaut pas qualification native/Windows.

Archive : `target/parakeet-tests/refacto-worker-ipc-d4/archive/closure-20261030/proofs.zip`, 80 entrées uniques, 328 907 octets. Arborescence relative conservée, compte/unicité et SHA-256 de chaque entrée comparés aux sources. SHA-256 du ZIP : `55B01E71A7ABDFEF2514DB1A7609E8D54EFD97527EE5A41E339099A359F15356`. Aucun modèle/corpus/WAV/cache/build inclus, aucun manifeste créé, aucun nettoyage ou suppression sans manifeste de suppression approuvé. Copie du modèle, liens bornés et cargo-shared conservés.

Espace libre final : 114 376 765 440 octets (environ 106,5 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan conservés dans le commit local de clôture, aucun push. Aucun lot D5 autorisé par cette clôture.
## 27. Démarrage autorisé du lot D5 — 30 octobre 2026

Avancée : D4 clos CLEAN sur `ebd65199ae5739a8514e2c08f22f39b67b8a268b` ; D5 autorisé.
Blocage : aucun obstacle documentaire constaté ; préflight et baseline requis avant extraction.
Décision : exécuter uniquement D5, revue indépendante puis commit local sans push si clôture démontrée.

Accord écrit de la session : « Passe au lot D5 selon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » Périmètre production : `worker_ipc.rs`, `runtime/mod.rs`, `runtime/effects.rs`, `runtime/import.rs` ; si import dépasse 1 500 lignes, subdivision déjà autorisée en `runtime/import/preparation.rs`, `publication.rs`, `worker_start.rs`, plafond atomique sept production pour ce lot. Tests directement correspondants. Extraire les douze variants résiduels SetGpuCandidate, ReadUnifiedDocumentPage, ReadUnifiedAudioPartsPage, Prepare, StartWorker, StartAutomaticFinalTranscription, ScanHistory, ResolveImportPublication, ResolveLivePublication, PersistSegment, Publish et Stop, ainsi que préparation/persistance/publication/stop et wrappers moteur nécessaires, sans déplacer les primitives F1. Déplacer ensemble le préambule et l'épilogue dans effects.rs ; chacun des trente bras ne fait plus que déléguer, match exhaustif sans fourre-tout. Préserver signature, sorties précoces, mutations, nombre et ordre des événements, ACK, nettoyage, publication et compagnon JSON ; aucune nouvelle API, dépendance, feature ou modification de manifeste.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-d5/`, copie adaptée de la garde existante, cache/fixture/modèle copié/temporaires/sorties bornés sous D5, cible Cargo partagée canonique. Le test legacy borné reste réellement activé dans les suites avec sa copie modèle sous D5 ; aucun lien physique vers le modèle installé ou une racine de lot extérieure. Préflight, auto-tests, espace ≥30 Gio, surveillance, commandes offline/locked et absence de Git concurrent obligatoires. Consignes de session sur fichiers extérieurs non attribués sans lien projet et `.git/objects/` maintenues ; ressources projet et écritures suspectes/attribuées restent protégées, aucune destination volontaire hors racines. Seules exclusions D2 corpus natif/l3c convenues, aucun nouveau skip ni qualification native/Windows implicite. Espace libre initial : 114 140 852 224 octets (environ 106,3 Gio). Non suivis préexistants préservés.
## 28. Clôture du lot D5 — 30 octobre 2026

Avancée : import extrait et dispatcher exhaustif fermé, correction de revue vérifiée, verdict indépendant CLEAN.
Blocage : aucun dans le périmètre D5 autorisé.
Décision : D5 clos sans réserve dans ce périmètre ; commit local sans push, aucun lot E1 démarré.

### Delta et fermeture de l'étape 3d

Base Git exacte : `8ec1f8e79fa35c8a30aad7189f5613aec75c57a7`, après D4 clos. Implémentation déléguée au profil prévu, revue par le coordinateur et le même reviewer indépendant en lecture seule ; session principale seule auteure du bilan et des commits, sans modification du code en parallèle.

Quatre fichiers production : `worker_ipc.rs`, déclaration dans `runtime/mod.rs`, nouveaux `runtime/effects.rs` (237 lignes physiques finales) et `runtime/import.rs` (1 016). Les subdivisions conditionnelles du plan n'ont pas été nécessaires. Une seule méthode effect, signature conservée ; préambule et épilogue déplacés ensemble. Le match principal contient trente variantes explicites, sans fourre-tout, chacune déléguée. Les douze variants résiduels d'import, lecture de pages, préparation, démarrage, persistance, publication et Stop ont leurs handlers ; les contrôles internes de retour D2 à D4 sont conservés. Aucun corps métier restant dans ce match, aucune nouvelle API externe, dépendance, feature, modification de manifeste ou Cargo.lock. Polling E1/E2 et primitives F1/F2 restent hors de ce lot.

Le reviewer a comparé séparément les trente bras, la signature, le préambule et l'épilogue à la base, puis les helpers de préparation, persistance, publication et Stop. Équivalence confirmée hors câblage et mise en forme : retours dispatcher/closures, mutations, nombre/ordre des événements, ACK et compteurs, identités, nettoyage et chemins d'échec préservés.

### Caractérisation et preuves initiales — époque 0

Un test privé séparé `automatic_final_binding_failure_updates_companion_and_clears_identity` a été ajouté dans `tests/effect_characterization.rs`, déclaré par `tests.rs`. Il vérifie l'échec de binding automatique, Failed, le statut failed_or_unavailable du JSON compagnon, son diagnostic et le nettoyage des identités/sorties. PASS avant extraction (`Run-20261030T102957586-f39cb100`) puis après (`Run-20261030T103102615-a59faa15`), codes 0.

Les trois témoins T-D5 nommés (publication échouée sans rétention, message worker échoué sans masquer Stop corrélé, Stopped obsolète sans masquer Stop corrélé), ainsi que l'ACK de persistance, ont été réellement exécutés et réussis avant/après. Baseline et premier candidat : format, workspace, diff check et cinq suites séquentielles PASS avec legacy réel. Une caractérisation supplémentaire dans la découverte à cette étape :

| Configuration | Réussis baseline | Réussis candidat époque 0 | Échoués | Ignorés | Filtrés autorisés |
| --- | --- | --- | --- | --- | --- |
| défaut | 270 | 271 | 0 | 16 | 1 D2 corpus natif |
| recette-hooks | 274 | 275 | 0 | 16 | 1 D2 corpus natif |
| l01-memory-qualification | 270 | 271 | 0 | 16 | 1 D2 corpus natif |
| personal-qualification | 270 | 271 | 0 | 16 | 1 D2 corpus natif |
| recette-hooks,l3-gpu-qualification | 274 | 275 | 0 | 16 | 2 D2 corpus natif/l3c |

Ces cinq suites appartiennent à l'époque 0 ; elles n'ont pas été relancées après la correction ciblée décrite ci-dessous. Journaux : `Run-20261030T103208167-add0ee56`, `Run-20261030T103504237-a4719f62`, `Run-20261030T103801406-48d239ca`, `Run-20261030T104058473-0bfe119e`, `Run-20261030T104356916-f913e829`, sous `target/parakeet-tests/refacto-worker-ipc-d5/logs/` avec stdout/stderr et index `guard.jsonl`.

Les contrôles conditionnels purs sont acquis : catalogue injecté dans les suites grâce à WHISPER_L1B_RUNTIME_ROOT et deux contrôles ciblés (`Run-20261030T104639248-692b5b00`, `Run-20261030T104640395-83e55217`), codes 0. Le legacy borné reste réellement exécuté dans les suites ; aucun nouveau retour NOT RUN ou skip de ce témoin.

### Constat de revue, correction et preuves finales — époque 1

Première revue : aucun défaut fonctionnel observé, mais constat requis MINEUR `WIPC-D5-R01` sur deux nouvelles signatures de pagination à neuf arguments. Elles aplatissaient des champs homogènes request_id/révision/offset, avec risque d'inversion qui continuerait à compiler ; ces signatures n'ont pas été présentées comme dette héritée. Le même implémenteur a corrigé le constat : les commandes ImportEffect existantes restent groupées jusqu'aux deux handlers, puis sont déstructurées une fois. Signatures self + effet + événements, aucun nouveau contexte artificiel, API publique ou suppression de lint. Match principal toujours exhaustif.

Le candidat corrigé ajoute un témoin de dispatcher `pagination_effects_preserve_request_and_generation_identity_on_error` dans le même fichier privé. Le test cible les identités et erreurs des deux événements paginés (`Run-20261030T105820655-550b73de`, PASS). Le test du cœur `unified_page_responses_and_errors_follow_latest_selection_request` passe (`Run-20261030T105904263-a9092c92`). Les contrôles réellement réexécutés sur l'époque 1 passent également : JSON/binding/nettoyage (`105843347-4eaec5f5`), publication échouée (`105844678-f7630d08`), Stop corrélé après événement obsolète (`105846124-6338a259`), ACK de persistance (`105847414-3b81f77f`), workspace (`105848705-e667d58f`), Clippy (`105852720-a70664a1`), format (`105902562-37197e4a`) ; préfixe `Run-20261030T` pour chaque référence.

Le reviewer a vérifié la correction, fermé lui-même WIPC-D5-R01 et jugé ces contrôles ciblés suffisants pour les deux frontières modifiées, sans lacune matérielle. La preuve finale est donc composée : suites complètes époque 0, contrôles ciblés époque 1 et relecture de l'équivalence du changement borné. Ce bilan ne prétend pas à une exécution des cinq suites sur le commit final.

Clippy initial : 148 avertissements contre 146 en baseline (deux nouvelles signatures de pagination, corps Stop extrait et disparition d'un diagnostic du dispatcher). Clippy corrigé : 146, les deux too_many_arguments de pagination ont disparu, sans allow. Dette de taille héritée explicitement retenue : effect 227 lignes et Stop 111 pour la cible 100. Le reviewer ne relève plus de constat actionnable et n'assimile pas cette dette à l'atteinte des cibles de structure.

### Campagne, revue et conservation

Préflight, auto-tests et surveillance de la garde sous D5, cache/fixture/modèle copié/temporaires et sorties bornés, cible Cargo partagée canonique, commandes offline/locked, aucun Git concurrent ni nouvelle isolation/surveillance. Consignes de session pour événements extérieurs non attribués sans lien projet et `.git/objects/` maintenues sans lever les protections des ressources projet. Modèle installé en lecture seule, SHA et métadonnées finaux identiques au préflight : `1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69`, 1 624 555 275 octets, date relevée du 6 octobre 2026 à 08:12:43. Copie et liens physiques restent sous D5.

Les 16 tests ignorés, exclusions D2 corpus natif/l3c déjà convenues, campagnes natives et fenêtre Windows ne sont pas revendiqués exécutés ; aucun nouveau skip ajouté. La clôture est celle du périmètre structurel D5, sans qualification native/Windows implicite.

Verdict final indépendant **CLEAN** sur le candidat working tree époque 1 : deux passes de revue, un cycle correctif, WIPC-D5-R01 fermé, aucun constat ouvert ni vérification requise manquante. La transmission de correction n'avait initialement pas réactivé le reviewer ; le coordinateur a corrigé le déclenchement via le même reviewer avant le paquet final. Aucun changement de source ou de preuve pendant cette correction de coordination.

Archive : `target/parakeet-tests/refacto-worker-ipc-d5/archive/closure-20261030/proofs.zip`, 104 entrées uniques, 378 726 octets. Arborescence relative conservée, compte/unicité et SHA-256 de chaque entrée comparés aux sources. SHA-256 du ZIP : `9C0FE209F3F76F461F25F2682F784A1B68C56CABC25D39A1742A57B4AD978477`. Suites époque 0 et contrôles époque 1 inclus avec leurs journaux distincts. Aucun modèle/corpus/WAV/cache/build archivé, aucun manifeste créé ni nettoyage/suppression sans manifeste de suppression approuvé ; copie modèle, liens bornés et cargo-shared conservés.

Espace libre final : 110 231 945 216 octets (environ 102,7 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan conservés dans le commit local de clôture, aucun push. Aucun lot E1 autorisé par cette clôture.
## 29. Démarrage autorisé du lot E1 — 30 octobre 2026

Avancée : D5 clos CLEAN sur `85ce279e9aec75969692a73e849076d140315d88` ; E1 autorisé.
Blocage : aucun obstacle documentaire constaté ; préflight et baseline requis avant extraction.
Décision : exécuter uniquement E1, revue indépendante puis commit local sans push si clôture démontrée.

Accord écrit de la session : « Passe au lot E1 selon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » Périmètre : `worker_ipc.rs`, `runtime/mod.rs`, `runtime/poll.rs`, `runtime/poll/capture.rs`, `runtime/poll/parakeet.rs`, cinq fichiers production maximum et tests directement correspondants. Déplacer poll, poll_live_capture, pump_live_windows, progression/résultat VAD, poll_parakeet_worker et télémétrie dans poll et ses enfants, sans réordonner capture, worker, ACK ou événements différés. Les aides de gestion d'échec suivent leur propriétaire runtime prévu sans nouvelle politique ; pas de transfert anticipé de backends F1 ni de polling unifié E2. Préserver retours Option/Result, identités, règles d'arrêt et seuil LIVE_CAPTURE_EVENTS_BEFORE_WORKER_POLL, sans nouvelle boucle active. Aucun nouveau comportement, API, dépendance, feature ou manifeste.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-e1/`, copie adaptée de la garde existante, cache/fixture/modèle copié/temporaires et sorties bornés sous E1, cible Cargo partagée canonique. Préflight, auto-tests, espace ≥30 Gio, surveillance, offline/locked et absence de Git concurrent obligatoires. Test legacy borné réellement activé dans les suites, copie et liens physiques sous E1 seulement, état du modèle installé contrôlé en lecture seule. Consignes de session sur événements extérieurs non attribués sans lien projet et `.git/objects/` maintenues sans lever les protections des ressources projet ou des écritures suspectes/attribuées. Seules exclusions D2 corpus natif/l3c déjà convenues, aucun nouveau skip ni qualification native/Windows implicite. Espace libre initial : 109 666 598 912 octets (environ 102,1 Gio). Non suivis préexistants préservés.
## 30. Clôture du lot E1 — 30 octobre 2026

Avancée : polling principal et auxiliaires extraits, vérifications réussies et revue indépendante CLEAN.
Blocage : aucun dans le périmètre E1 autorisé.
Décision : E1 clos sans réserve dans ce périmètre ; commit local sans push, aucun lot E2 démarré.

### Delta et invariants

Base Git exacte : `e6e9fe68c823f80f78241f5981ac37d059dba45a`, après D5 clos. Implémentation déléguée au profil prévu ; revue par le coordinateur et le même reviewer indépendant en lecture seule. La session principale n'a pas modifié le code en parallèle.

Cinq fichiers production : `worker_ipc.rs`, déclaration dans `runtime/mod.rs`, nouveaux `runtime/poll.rs` (1 107 lignes physiques), `runtime/poll/capture.rs` (254) et `runtime/poll/parakeet.rs` (819). Neuf méthodes déplacées, aucun corps correspondant restant à la racine ; aucun test, manifeste, Cargo.lock, dépendance ou feature modifié. Les corps sont identiques à la base hors fins de ligne, signatures et visibilités internes adaptées sans nouvelle API publique.

Le reviewer a confirmé l'ordre des phases : sessions/document unifiés ; événements Parakeet différés ; micro/direct/Parakeet live ; reprises et diagnostics ; télémétrie/capture ; attente ACK ; VAD ; worker Parakeet ou transport ; événements et arrêts différés. Les méthodes de polling unifié et primitives moteur restent à leur place pour E2/F1/F2 ; seuls leurs appels dans le cycle sont préservés. Retours Option/Result, identités job/génération/instance/tentative, Ready/Failed/Stop, ACK et événements différés inchangés, aucune nouvelle boucle active ou politique de repli.

La constante `LIVE_CAPTURE_EVENTS_BEFORE_WORKER_POLL` vaut **2**, identique à la base. Le témoin `microphone_legacy_capture_budget_forces_worker_poll_within_four_ui_events` garantit une borne de **quatre événements UI** : ce 4 ne désigne pas la constante. Un compte rendu intermédiaire avait confondu les deux ; il a été corrigé explicitement sans changement de code.

### Incident de formatage et reprise autorisée

Après les cinq suites de baseline réussies et un check d'extraction réussi, l'implémenteur a lancé par erreur `cargo fmt --all` en écriture sous garde (`Run-20261030T112658530-cfb3e212`). La garde a détecté une modification source de `runtime/poll/capture.rs` à 11:26:59 et arrêté le processus, code 1. Les trois nouveaux fichiers polling portaient cet horodatage et ont pu être formatés. C'est une écriture de campagne hors racines, attribuée et non couverte par les dérogations extérieures ; ni le résultat ni l'incident n'ont été présentés comme un contrôle réussi. Aucun fichier restauré ou supprimé, candidat conservé et aucun processus restant actif.

La session principale a demandé la reprise sur cet état pour revue, format uniquement --check et sans lever la protection des sources. L'utilisateur a répondu précisément : « je t'autorise la reprise sur le candidat conservé ». La garde existante E1 a été renforcée pour refuser avant Cargo tout fmt sans --check et tout --fix ; auto-tests synthétiques de ces refus et auto-tests existants PASS, sans nouvelle surveillance ou isolation. Une ligne vide signalée par fmt --check a été corrigée manuellement entre commandes, puis les contrôles finaux ont été exécutés sur le candidat conservé. Le reviewer confirme l'absence de changement parasite et l'équivalence des neuf corps après reprise. La protection des sources reste inchangée, sans permission générale de formatage Cargo en écriture.

Le premier check d'extraction avait auparavant signalé une accolade déplacée ; elle avait été corrigée avant le check réussi qui précédait l'incident de formatage. Les premières commandes en erreur restent dans les preuves et sont distinguées de la validation finale.

### Résultats exécutés

Préflight, auto-tests et surveillance sous `target/parakeet-tests/refacto-worker-ipc-e1/`, cache/fixture/modèle copié/temporaires et sorties bornés sous E1, cible Cargo partagée canonique, commandes offline/locked, absence de Git concurrent. Consignes de session sur fichiers extérieurs non attribués sans lien projet et `.git/objects/` maintenues sans lever les protections des ressources projet ou des écritures suspectes/attribuées.

Quatre témoins T-E nommés du plan et trois témoins Ready/ACK/anti-famine réellement exécutés et réussis avant/après. Format --check, diff check, workspace et cinq suites séquentielles baseline/candidat PASS. Tous les résultats candidat ci-dessous sont postérieurs à la reprise autorisée :

| Configuration séquentielle | Réussis rapportés baseline et candidat | Échoués | Ignorés | Filtrés autorisés | Legacy réel |
| --- | --- | --- | --- | --- | --- |
| défaut | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks | 276 | 0 | 16 | 1 D2 corpus natif | oui |
| l01-memory-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| personal-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks,l3-gpu-qualification | 276 | 0 | 16 | 2 D2 corpus natif/l3c | oui |

La baseline E1 porte sur les sources D5 finales et constitue une nouvelle exécution complète ; elle ne transforme pas rétrospectivement les seules preuves ciblées de l'époque 1 D5 en suites anciennes relancées. Les noms et comptes de tests E1 avant/après sont inchangés.

Clippy PASS, diagnostics identiques par crate et multiensemble des en-têtes identique : **146 diagnostics adapters et 23 core** avant/après, soit 169 diagnostics réels. Le compte brut intermédiaire de 171 incluait deux lignes de résumé ; il ne s'agit pas d'une hausse depuis les 146 adapters rapportés aux lots précédents. Dette héritée conservée : poll 1 087 lignes/complexité 101, polling Parakeet 501/30, capture 125 lignes, VAD 102 et signature VAD préexistante à 9 arguments. Aucun nouveau diagnostic, aucune suppression de lint ; les seuils de structure globaux ne sont pas revendiqués atteints.

Deux contrôles conditionnels signalent NOT RUN dans les suites : `parakeet_import::tests::paths_reject_a_worker_outside_the_runtime_root` attend PARAKEET_L2_RUNTIME_ROOT ; `worker_ipc::tests::over_limit_parakeet_import_is_refused_before_pending_creation` attend PARAKEET_L2_MODEL, PARAKEET_L2_RUNTIME_ROOT, PARAKEET_L2_EN_WAV, PARAKEET_L2_WORKER et PARAKEET_L2_NATIVE_DIR. Les comptes libtest ne prouvent pas l'exécution de ces deux corps. Le reviewer les a jugés sans lacune matérielle E1 car leurs frontières n'ont pas été modifiées. Catalogue injecté et legacy sont réellement exécutés et réussis. Les 16 ignorés, exclusions D2 corpus natif/l3c convenues, campagnes natives et fenêtre Windows ne sont pas revendiqués exécutés ; aucun nouveau skip ajouté.

Journaux des cinq suites finales : `Run-20261030T113052658-fcafe801`, `Run-20261030T113339641-28fcefc4`, `Run-20261030T113542460-231d3202`, `Run-20261030T113746321-91fdddfb`, `Run-20261030T113950683-f8546b84`, sous `target/parakeet-tests/refacto-worker-ipc-e1/logs/` avec stdout/stderr, index `guard.jsonl` et incidents `guard-events.jsonl`.

### Revue et conservation

Verdict indépendant **CLEAN** sur le candidat working tree, epoch 0 : une passe de revue, zéro correction, aucun constat ouvert, aucune vérification requise manquante ni blocage dans E1. Le reviewer confirme ordre, retours, identités, événements différés, seuil, absence d'extraction E2/F1/F2 et validation finale après reprise explicitement autorisée. Les limites conditionnelles sont nommées ci-dessus ; la clôture structurelle ne vaut pas qualification native/Windows.

Le modèle installé conserve le SHA-256 `1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69`, taille 1 624 555 275 octets et date de préflight ; copie et liens physiques restent bornés sous E1. Archive : `target/parakeet-tests/refacto-worker-ipc-e1/archive/closure-20261030/proofs.zip`, 94 entrées uniques, 330 684 octets. Arborescence relative conservée, compte/unicité et SHA-256 de chaque entrée comparés aux sources. SHA-256 du ZIP : `AB2D79C9ACFE3CB9A49B3BF924F3E23D8C095A19948AD98B3D80B636F88DD418`. Aucun modèle/corpus/WAV/cache/build archivé, aucun manifeste créé ni nettoyage/suppression sans manifeste de suppression approuvé ; copie modèle, liens et cargo-shared conservés.

Espace libre final : 106 728 566 784 octets (environ 99,4 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan conservés dans le commit local de clôture, aucun push. Aucun lot E2 autorisé par cette clôture.
## 31. Démarrage autorisé du lot E2 — 30 octobre 2026

Avancée : E1 clos CLEAN sur `5b60ac943d9882e3a17e129f179bcb505f557337` ; E2 autorisé.
Blocage : aucun obstacle documentaire constaté ; préflight et baseline requis avant extraction.
Décision : exécuter E2 seul selon le plan, revue indépendante puis commit local sans push si clôture démontrée.

Accord écrit de la session : « Passe au lot suivant elon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » Le prochain lot est E2, dépendant de E1 clos. Périmètre : `worker_ipc.rs`, `runtime/mod.rs`, `runtime/poll_unified.rs`, subdivision conditionnelle `runtime/poll_unified/document.rs` si nécessaire ; quatre production maximum et tests directement correspondants. Extraire poll_unified_sessions, poll_unified_document et aides de libération/admission directement requises, sans changer le séquencement. Attribuer à poll_unified l'orchestration retry_native_gpu_startup_on_cpu et ses types NativeAutoCpuRetry, FailedManualGpuPeer et manual_gpu_failure_event. Le prédicat pur can_retry_native_auto_on_cpu reste racine jusqu'à F1 ; types source/exécuteur restent racine jusqu'à F2. Conserver ACK, points de libération, limites/concurrence, identités, drain et règles CPU/GPU existants, sans fusion de machines session/worker ni nouvelle politique.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-e2/`, copie adaptée de la garde E1 renforcée (refus fmt sans --check et --fix), cache/fixture/modèle copié/temporaires et sorties bornés sous E2, cible Cargo partagée canonique. Préflight, auto-tests, espace ≥30 Gio, surveillance, commandes offline/locked et absence de Git concurrent obligatoires. Legacy borné réellement actif dans les suites ; modèle installé en lecture seule, copie et liens sous E2, état pré/post contrôlé. Consignes extérieurs non attribués sans lien projet et `.git/objects/` maintenues sans lever les protections des ressources projet ou des écritures suspectes/attribuées. Seules exclusions D2 corpus natif/l3c déjà convenues, aucun nouveau skip ni qualification native/Windows implicite. Espace libre initial : 106 408 243 200 octets (environ 99,1 Gio). Non suivis préexistants préservés.
## 32. Clôture du lot E2 — 30 octobre 2026

Avancée : polling unifié extrait, correction d'encodage vérifiée et revue indépendante CLEAN.
Blocage : aucun dans le périmètre E2 autorisé.
Décision : E2 clos sans réserve dans ce périmètre ; commit local sans push, aucun lot F1 démarré.

### Delta et invariants

Base Git exacte : `2c73a5f7194695ece718172444cefc6841c550e2`, après E1 clos. Implémentation déléguée au profil prévu ; revue par le coordinateur et le même reviewer indépendant en lecture seule. La session principale n'a pas modifié le code en parallèle.

Trois fichiers production : `worker_ipc.rs`, déclaration dans `runtime/mod.rs`, nouveau `runtime/poll_unified.rs` (1 334 lignes physiques). La subdivision document n'a pas été nécessaire. poll_unified_sessions, poll_unified_document, aides de libération/admission directement requises, retry_native_gpu_startup_on_cpu, NativeAutoCpuRetry, FailedManualGpuPeer et manual_gpu_failure_event possèdent leur définition unique sous poll_unified. Le prédicat pur can_retry_native_auto_on_cpu et les types source/exécuteur restent à la racine jusqu'aux lots F1/F2. Aucun test, API publique, manifeste, Cargo.lock, dépendance ou feature modifié.

Le reviewer a comparé sept corps déplacés, identiques hors fins de ligne, et deux types identiques hors visibilité. Ordre du drain et du polling, ACK, identités/générations/tentatives, points de libération, restitutions, capacités, concurrence et repli préservés ; aucune fusion de machines session/worker, boucle active ou nouvelle politique. E1 et son seuil restent inchangés. Après correction des douze littéraux ci-dessous, la façade restante est équivalente à la base en dehors des définitions déplacées et des imports internes nécessaires.

### Preuves initiales — époque 0

Préflight, auto-tests et surveillance de la garde sous `target/parakeet-tests/refacto-worker-ipc-e2/`, cache/fixture/modèle copié/temporaires et sorties bornés sous E2, cible Cargo partagée canonique, commandes offline/locked, aucun Git concurrent. Garde renforcée E1 conservée : fmt sans --check et --fix refusés avant Cargo. Consignes de session sur événements extérieurs non attribués sans lien projet et objets Git maintenues sans lever les protections des ressources projet ou des écritures suspectes/attribuées. Aucune violation de garde signalée dans E2.

Les quatre T-E passent avant/après. Trois témoins synthétiques supplémentaires candidat sont réellement exécutés et réussis, sans variable native conditionnelle : `native_auto_gpu_ready_failure_retries_once_on_cpu_before_first_call` (retry Auto et refus du pair GPU manuel), `controlled_document_drain_reads_one_segment_per_poll_and_preserves_all_pages` (drain borné et pages) et `import_application_finalizes_stopped_capture_a_while_capture_b_pumps_and_publishes_segmented_audio` (finalisation concurrente). Le test du cœur `unified_page_responses_and_errors_follow_latest_selection_request` passe après extraction (`Run-20261030T122853564-8b764eea`) sur core inchangé ; il n'a pas été exécuté séparément avant extraction et n'est pas présenté comme tel.

Baseline puis premier candidat : format --check, workspace, diff check, Clippy et cinq suites séquentielles PASS, sans ajout ou retrait de test. Catalogue injecté et legacy réellement RUN/PASS dans les dix suites. Les cinq suites candidat appartiennent à l'époque 0, avant la restauration des douze littéraux :

| Configuration | Réussis rapportés baseline et candidat époque 0 | Échoués | Ignorés | Filtrés autorisés | Legacy réel |
| --- | --- | --- | --- | --- | --- |
| défaut | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks | 276 | 0 | 16 | 1 D2 corpus natif | oui |
| l01-memory-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| personal-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks,l3-gpu-qualification | 276 | 0 | 16 | 2 D2 corpus natif/l3c | oui |

Journaux des suites candidat : `Run-20261030T121346485-21946b76`, `Run-20261030T121653051-052229bf`, `Run-20261030T121948294-ce6133f7`, `Run-20261030T122252434-7b41c570`, `Run-20261030T122548211-46244c60`, sous `target/parakeet-tests/refacto-worker-ipc-e2/logs/` avec stdout/stderr et index `guard.jsonl`.

Clippy époque 0 : core 23 diagnostics et adapters 146, identiques avant/après, multiensemble des en-têtes identique. Dette héritée inchangée : longueur/complexité 960/64 du polling unifié et signature de repli à dix arguments préexistante. Aucun nouveau lint ou allow ; l'atteinte globale des cibles de structure n'est pas revendiquée.

### Constat de revue et preuve finale — époque 1

Première revue : constat requis MINEUR `WIPC-E2-R01`. Douze littéraux français restés dans `worker_ipc.rs`, hors code extrait, avaient subi une corruption d'encodage réelle : par exemple modèle devenu modÃ¨le, délai devenu dÃ©lai. Le reviewer l'a confirmé par comparaison UTF-8/code points avec la base ; ce n'était pas un simple affichage et les tests ne l'avaient pas détecté. Les diagnostics concernés touchaient qualification GPU, catalogue, démarrage micro et repli Auto. La clôture n'a pas été prononcée sur cet état malgré les suites vertes.

Le même implémenteur a restauré exactement les douze lignes de littéraux de la base, sans réencodage global, modification d'autre fichier ou changement des corps extraits. Comparaison individuelle UTF-8 avec HEAD et absence de changement parasite confirmées par le même reviewer. Après correction : auto-tests de garde, `git diff --check`, format (`Run-20261030T123352109-43a556a9`) et workspace (`Run-20261030T123353804-b8b4d4b7`) PASS.

Les cinq suites, les T-E, les autres tests cités et Clippy demeurent des preuves de l'époque 0 ; ils n'ont pas été relancés après la correction limitée aux douze littéraux. La preuve de fermeture demandée par le reviewer prévoyait explicitement la comparaison des littéraux et les contrôles format/diff/workspace, sans répétition des cinq suites si aucun autre changement. Le reviewer a confirmé cette condition et fermé lui-même WIPC-E2-R01. La preuve finale est composée, et ce bilan ne prétend pas à une campagne complète sur le commit final.

### Limites, revue et conservation

Deux corps conditionnels restent NOT RUN faute de variables : `paths_reject_a_worker_outside_the_runtime_root` attend PARAKEET_L2_RUNTIME_ROOT ; `over_limit_parakeet_import_is_refused_before_pending_creation` attend PARAKEET_L2_MODEL, PARAKEET_L2_RUNTIME_ROOT, PARAKEET_L2_EN_WAV, PARAKEET_L2_WORKER et PARAKEET_L2_NATIVE_DIR. Les comptes libtest ne prouvent pas l'exécution de ces deux corps. Le reviewer les a jugés sans lacune matérielle E2, leurs frontières restant inchangées. Les 16 ignorés, exclusions D2 corpus natif/l3c déjà convenues, campagnes natives et fenêtre Windows ne sont pas revendiqués exécutés ; aucun nouveau skip.

Verdict final indépendant **CLEAN** sur le candidat working tree, époque 1 : deux passes de revue et un cycle correctif, WIPC-E2-R01 fermé, aucun constat ouvert, aucune vérification requise manquante ni blocage. Les limites ci-dessus restent explicites ; la clôture structurelle ne vaut pas qualification native/Windows.

Le modèle installé conserve le SHA-256 `1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69`, taille 1 624 555 275 octets et date du préflight ; copie et liens sont bornés sous E2. Archive : `target/parakeet-tests/refacto-worker-ipc-e2/archive/closure-20261030/proofs.zip`, 90 entrées uniques, 331 436 octets. Arborescence relative conservée, compte/unicité et SHA-256 de chaque entrée comparés aux sources. SHA-256 du ZIP : `E3C09D2F5E50B6174D018A9E9548A05800EBE7D43A5A6AAAA4D286DC693AA0B3`. Suites et Clippy époque 0, contrôles époque 1 distincts dans l'archive. Aucun modèle/corpus/WAV/cache/build archivé, aucun manifeste créé ni nettoyage/suppression sans manifeste de suppression approuvé ; copie modèle, liens et cargo-shared conservés.

Espace libre final : 103 308 873 728 octets (environ 96,2 Gio). Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan conservés dans le commit local de clôture, aucun push. Aucun lot F1 autorisé par cette clôture.
## 33. Démarrage autorisé du lot F1 — 30 octobre 2026

Avancée : E2 clos CLEAN sur `50236dc8d2f62ad16f7884b2bc9b74aec7cef8ca` ; périmètre F1 ajusté et autorisé.
Blocage : aucun obstacle documentaire restant ; préflight et baseline requis avant extraction.
Décision : F1 seul, six fichiers production, revue indépendante et commit local sans push si clôture démontrée.

L'utilisateur a autorisé F1, ses vérifications et le commit local sans push, avec clôture sans réserve après revue indépendante. Il a ensuite autorisé précisément la création de `runtime/import/worker_start.rs`, puis le budget de six fichiers après constat que sa déclaration requiert aussi une modification du parent. L'accord courant « oui je t'autorise » répond à la proposition explicite de six fichiers, le sixième limité à la déclaration du sous-module. Le plan F1 est ajusté ci-dessus avant code.

Périmètre production exact : `worker_ipc.rs`, `backends/mod.rs`, `backends/native.rs`, `support.rs`, `runtime/import/worker_start.rs` et `runtime/import.rs` (déclaration seule). Les méthodes d'orchestration encore racine totalisent environ 661 lignes ; les ajouter aux 1 016 lignes d'import.rs dépasserait le plafond. Elles restent sur ImportRuntime dans le sous-module runtime, sans entrer dans les backends. Catalogue/bindings, cache IdleWorker, routes/attestations et probes deviennent des primitives backend sans dépendance service/ImportRuntime ; aides SHA vers support. Le prédicat pur can_retry_native_auto_on_cpu va backend ; l'orchestration de retry et ses pairs reste poll_unified, les sources/exécuteur restent F2. Réexports publics historiques et comportements CPU/GPU/Auto, preuves, identités, limites et réutilisation conservés. Aucun nouveau fallback, API, dépendance, feature ou manifeste.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-f1/`, garde existante adaptée, cache/fixture/modèle copié/temporaires et sorties bornés, cible Cargo partagée canonique. Préflight, auto-tests, seuil ≥30 Gio, surveillance, offline/locked et absence de Git concurrent obligatoires ; refus fmt sans --check et --fix maintenu. Legacy borné réellement actif, état du modèle installé contrôlé en lecture seule. Consignes extérieurs sans lien projet et objets Git maintenues sans lever les protections des ressources ou écritures suspectes/attribuées. Vérifications T-F et intégrations publiques worker_control/gpu_qualification_transport ; les corps conditionnels de catalogue/routes pertinents à F1 doivent être examinés et exécutés avec ressources bornées si nécessaires, sans campagne native implicite ni nouveau skip. Seules exclusions D2 corpus natif/l3c convenues ; pas de qualification native/Windows implicite.

Lecture/écriture UTF-8 explicite, sans conversion globale aveugle ; comparaison des littéraux/messages restants et déplacés avec la base pour prévenir la régression corrigée en E2. Espace libre initial : 102 804 127 744 octets (environ 95,7 Gio). Non suivis préexistants préservés.

## 34. Clôture du lot F1 — 30 octobre 2026

Avancée : primitives backend et démarrage runtime extraits, vérifications réussies et revue indépendante CLEAN.
Blocage : aucun dans le périmètre F1 autorisé.
Décision : F1 clos sans réserve dans ce périmètre ; commit local sans push, aucun lot F2 démarré.

### Delta et invariants

Base Git exacte : `d89d534be90422ba950617fe587d225e9c645a65`, après le démarrage ajusté du §33. Implémentation déléguée à rust_implementer ; revue confiée à rust_review_coordinator et au même rust_reviewer indépendant en lecture seule. La session principale n'a pas modifié le code en parallèle.

Six fichiers production, conformément à l'ajustement explicitement accepté : `worker_ipc.rs`, `support.rs`, `runtime/import.rs` (seule déclaration du sous-module), nouveaux `backends/mod.rs` (468 lignes physiques), `backends/native.rs` (215) et `runtime/import/worker_start.rs` (665). Le parent import passe de 1 016 à 1 018 lignes physiques, sans ajout de corps. Aucun test, manifeste, Cargo.lock, dépendance ou feature modifié.

Catalogue et bindings, cache IdleWorker, probes et primitives de sélection sont sous backends ; routes, attestations et prédicat pur can_retry_native_auto_on_cpu sous backends/native. Les backends ne dépendent ni du service ni d'ImportRuntime et n'utilisent aucun import wildcard. decode_sha256 et format_sha256 rejoignent support. Les sept méthodes start_worker_with_candidate, start_worker_with_cpu_override, start_worker_with_candidate_override, start_parakeet_worker, replace_parakeet_pending_with_cpu_key, fallback_parakeet_to_cpu et handle_worker_failure restent des méthodes ImportRuntime, désormais dans runtime/import/worker_start. L'orchestration retry GPU reste poll_unified ; sources et exécuteur restent à la racine pour F2.

Le reviewer a confirmé l'équivalence de 31 corps déplacés, les structures et signatures, les visibilités internes adaptées, la direction des imports et les réexports publics historiques probe_cuda_worker, qualify_gpu_worker et gpu_evidence_matches. Politique CPU/GPU/Auto, identités, preuves, limites de repli et réutilisation du cache conservées ; aucun nouveau fallback, recalcul de hash ou changement d'API. Comparaison explicite UTF-8 du multiensemble des 19 littéraux non ASCII avant/après : aucune valeur ajoutée ou retirée, messages exacts préservés.

### Campagne, incidents et reprises

Préflight et auto-tests sous `target/parakeet-tests/refacto-worker-ipc-f1/`, cache Cargo, modèles copiés, fixtures, sorties et temporaires bornés sous F1 ; seule cible de compilation partagée `target/parakeet-tests/cargo-shared/`. Variables canoniques, absence de jonction sortante, CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=line-tables-only et commandes offline/locked contrôlés. Refus fmt sans --check et --fix maintenu. Consignes de session sur événements extérieurs sans lien projet et objets Git maintenues sans lever les protections des ressources projet, index/HEAD/refs/config ou des écritures suspectes/attribuées.

Le premier T-F de baseline a été interrompu avant son exécution : run `20261030T130615204-c8fe63ed`, code 1. La garde a observé deux événements Changed sous l'ancien lot C : `target/parakeet-tests/refacto-worker-ipc-c/tmp/worker-ipc-tests` à 13:06:33.8058670 et `target/parakeet-tests/refacto-worker-ipc-c/tmp/r10-queue-store-tests` à 13:06:33.9057987. Auteur non établi, aucun résultat PASS revendiqué. L'implémenteur avait lancé simultanément une recherche récursive dans les anciennes racines ; cette corrélation temporelle ne prouve pas l'origine des événements. Aucun effacement ni restauration ; sources/index inchangés, arrêt des processus constaté. L'utilisateur a précisément autorisé la reprise avec garde inchangée et sans inspection concurrente. Le témoin a ensuite réussi et aucun nouvel incident de garde n'a été signalé. Les événements de refus synthétiques SelfTest restent distingués des événements Run. Cette disposition ne crée aucune exception générale pour les anciens lots.

Le premier check d'extraction `20261030T132935357-dbf879a7` a échoué sur des visibilités internes de réexports ; elles ont été ajustées avant le check réussi `20261030T132954171-f9526134` et les vérifications finales. Cette erreur de compilation intermédiaire reste dans les preuves.

Premier essai l01 du candidat `20261030T133805564-627a2f6d` : 271 réussis et un échec de r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication, le linker ne pouvant ouvrir l'EXE du faux worker (LNK1104). La reproduction isolée puis une relance intégrale l01 ont réussi sans modification du candidat. L'échec initial est conservé ; aucune cause extérieure précise n'est inférée et aucun nouveau skip n'a été ajouté.

### Résultats réellement exécutés

Sept témoins T-F exécutés individuellement et réussis avant/après :

- auto_without_gpu_starts_cpu_with_normalized_compute_and_same_identity ;
- auto_gpu_ready_is_attested_and_explicitly_admitted ;
- auto_import_gpu_exit_after_ready_before_begin_retries_once_on_cpu ;
- forced_gpu_initialization_failure_waits_for_explicit_cpu_choice ;
- stop_latched_after_gpu_failure_prevents_cpu_worker_launch ;
- retained_worker_reads_two_correlated_import_sessions_and_hits_model_cache ;
- prepared_native_routes_without_start_are_bounded_and_stop_releases_identity.

Intégrations publiques réellement exécutées avant/après : worker_control, 5 tests PASS ; gpu_qualification_transport, 1 test PASS. Format --check, git diff --check, check workspace et Clippy PASS avant/après. Check workspace final `20261030T133203952-49f41521`, Clippy final `20261030T133208289-e3309d7a` avec les quatre lints de structure prévus. Clippy : 23 diagnostics core et 146 adapters avant/après, soit 169 hors lignes de résumé. Dette héritée déplacée conservée sans nouveau diagnostic ni suppression de lint ; les cibles globales de structure ne sont pas revendiquées atteintes.

Cinq suites séquentielles complètes avant/après, sur le candidat final sans correction de revue. Catalogue injecté et legacy réellement RUN/PASS dans les dix suites abouties :

| Configuration | Réussis baseline et candidat abouti | Échoués dans ces exécutions | Ignorés | Filtrés autorisés | Legacy réel |
| --- | --- | --- | --- | --- | --- |
| défaut | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks | 276 | 0 | 16 | 1 D2 corpus natif | oui |
| l01-memory-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| personal-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks,l3-gpu-qualification | 276 | 0 | 16 | 2 D2 corpus natif/l3c | oui |

Journaux candidat aboutis : `Run-20261030T133213563-3a79ab38`, `Run-20261030T133510521-091b4249`, `Run-20261030T134139642-b2cc3ff3` (relance l01), `Run-20261030T134435193-235b989e`, `Run-20261030T134730997-a6173c46`. L'essai l01 initial en échec est distinct de ce tableau.

Les deux contrôles conditionnels ont cette fois été exécutés réellement, en sessions isolées avec ressources bornées sous F1, avant et après extraction : paths_reject_a_worker_outside_the_runtime_root (avant `20261030T132833724-8c2d0eb2`, après `20261030T135008156-b5042aae`) et over_limit_parakeet_import_is_refused_before_pending_creation (avant `20261030T132834917-8ddb60f1`, après `20261030T135009364-c625adb9`). Tous PASS ; leur exécution est prouvée par ces sessions, pas déduite des seuls comptes libtest des suites générales. Le test de refus anticipé utilise le modèle réel mais ne lance aucun worker natif. Les 16 ignorés et exclusions historiques D2/l3c ne sont pas revendiqués exécutés. Qualification native CPU/GPU et fenêtre Windows non réalisées ; aucune qualification produit implicite.

### Revue et conservation

Verdict indépendant **CLEAN**, époque 0 : une passe de revue, zéro correction, aucun constat ouvert, aucune vérification requise manquante ni blocage dans F1. Les incidents et dispositions ci-dessus ont été examinés par le reviewer ; aucune causalité non démontrée n'est revendiquée. Cette clôture ne démarre ni n'autorise F2.

Modèles originaux et copies F1 contrôlés, originaux inchangés par rapport au préflight de copie : Turbo 1 624 555 275 octets, SHA-256 `1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69` ; Parakeet 713 975 456 octets, SHA-256 `E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E`.

Archive `target/parakeet-tests/refacto-worker-ipc-f1/archive/closure-20261030/proofs.zip` : 114 entrées uniques, 376 174 octets, arborescence relative préservée, SHA-256 de chaque entrée comparé en mémoire à sa source. SHA-256 ZIP : `97ECB2C68F27A4DEEA0082D50488984626FA70B64AF96A0055E3EDA959486D51`. Aucun modèle/cache/build/corpus/WAV inclus, aucun nouveau manifeste, aucune suppression ni restauration. Copies modèles/liens et cargo-shared conservés.

Espace libre final : 99 491 045 376 octets (environ 92,7 Gio), au-dessus du seuil de 30 Gio. Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Code relu et présent bilan réunis dans le commit local de clôture, sans push.
## 35. Démarrage autorisé du lot F2 — 30 octobre 2026

Avancée : F1 clos CLEAN sur `10cb0f117e3bc0e48c3247269de6f4c3892b3ccb` ; F2 est le prochain lot du plan et son inventaire est établi.
Blocage : aucun obstacle identifié en lecture seule ; préflight et baseline requis avant extraction.
Décision : exécuter F2 seul, revue indépendante puis commit local sans push si clôture démontrée.

Accord écrit courant : « Passe au lot suivant selon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » Le prochain lot est F2, dépendant de F1 clos. Cinq fichiers production maximum : `worker_ipc.rs`, nouveaux `parakeet_unified/mod.rs`, `parakeet_unified/executor.rs`, `parakeet_unified/native.rs`, et `runtime/mod.rs` pour imports d'état si nécessaires ; tests directement correspondants uniquement. Inventaire préliminaire : environ 182 lignes état/source/drain/restauration, 438 exécuteur/commandes/résultats/Drop, 451 transcripteur natif/validation PCM. Aucun dépassement ou sixième fichier identifié ; toute découverte contraire appelle l'arbitrage prévu avant extension.

Extraire les types UnifiedDocumentDrain, UnifiedSessionSource/State et restore_unified_document_prefix vers mod ; commandes/résultats/UnifiedInferenceExecutor, start/stop/reap/restart et destruction vers executor ; NativeParakeetWindowTranscriber et validation PCM vers native. Primitives sans dépendance service/runtime/ImportRuntime, routes backends F1 réutilisées. L'admission et le démarrage orchestrés sur ImportRuntime restent runtime. Canaux bornés, barrières Ready/Stopped, propriété et durée de vie threads/enfants, sessions, récoltes et restauration conservés. Pas de nouvelle pool, planification, politique, API, protocole, dépendance, feature ou modification de Cargo.lock. F3 non démarré.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-f2/`, adaptation de la garde existante F1, cache/fixtures/modèles copiés/temporaires/sorties bornés sous F2, seule cible Cargo `target/parakeet-tests/cargo-shared/`. Préflight canonique, auto-tests, seuil ≥30 Gio, surveillance, offline/locked, CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only obligatoires ; refus fmt sans --check et --fix maintenu. Aucune inspection Git/recherche récursive concurrente à la campagne. Dispositions de session sur événements extérieurs sans lien projet et objets Git maintenues, sans lever les protections des ressources, sources, index/HEAD/refs/config ou écritures suspectes/attribuées. Les reprises ponctuelles des incidents antérieurs ne créent pas d'exception générale pour les anciens lots.

V1–V6 et M1–M3 avant/après, cinq configurations séquentielles avec legacy réellement actif et borné ; T-F et témoins unifiés panique/déconnexion/Ready/recyclage/drain/reprise. Corps conditionnels pertinents examinés et exécutés avec ressources bornées lorsque requis ; les comptes libtest ne prouvent pas un corps ayant retourné tôt. Seules exclusions D2 corpus natif/l3c déjà convenues, 16 ignorés historiques distingués ; aucune qualification native ni Windows implicite. Modèles originaux en lecture seule, copies et liens bornés, état pré/post contrôlé. UTF-8 explicite, comparaison exacte des messages restants/déplacés et des corps, aucun réencodage global. Archive de petites preuves avec nombre et SHA-256 à la clôture ; aucun nettoyage sans manifeste approuvé.

Espace libre initial : 98 702 929 920 octets (environ 91,9 Gio). Aucun changement suivi avant démarrage ; non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Implémentation déléguée au même rust_implementer, revue confiée au coordinateur et à un reviewer indépendant, deux cycles maximum ; session principale auteur du bilan et du commit.
## 36. Clôture du lot F2 — 30 octobre 2026

Avancée : primitives Parakeet unifiées extraites, vérifications réussies et revue indépendante CLEAN sur le candidat final.
Blocage : aucun dans le périmètre F2 autorisé.
Décision : F2 clos sans réserve dans ce périmètre ; commit local sans push, aucun lot F3 démarré.

### Delta et invariants

Base Git exacte : `0ce476a7f15f7cb297c858e506cb2c56c51cc1a5`, après le démarrage autorisé du §35. Implémentation déléguée au même rust_implementer, revue confiée à rust_review_coordinator et au même rust_reviewer indépendant en lecture seule. La session principale n'a pas modifié le code en parallèle.

Quatre fichiers production dans le budget de cinq : `worker_ipc.rs` et les nouveaux `parakeet_unified/mod.rs` (200 lignes physiques), `parakeet_unified/executor.rs` (462) et `parakeet_unified/native.rs` (463). Aucun changement nécessaire dans runtime/mod.rs. Aucun test, manifeste, Cargo.lock, dépendance ou feature modifié ; inclusion du support de tests historique runtime_support.rs conservée à la racine.

UnifiedDocumentDrain, UnifiedSessionSource/State et restore_unified_document_prefix rejoignent mod.rs ; commandes/résultats, UnifiedInferenceExecutor et ses opérations start/stop/reap/restart/Drop rejoignent executor ; NativeParakeetWindowTranscriber et validation PCM rejoignent native. Les routes backend F1 sont utilisées, sans dépendance vers service/runtime/ImportRuntime ni import wildcard. L'admission et le démarrage orchestrés restent sur ImportRuntime dans runtime. Aucun nouveau pool, planificateur, canal ou choix métier.

Le reviewer confirme 16 déclarations/impls équivalents et 33 corps identiques à la base hors fins de ligne. Canaux bornés, barrières Ready/Stopped, propriété et durée de vie des threads/enfants, récoltes, reprise et identités conservés. API et réexports publics inchangés ; types extraits possédés uniquement par leur nouveau module. Comparaison explicite UTF-8 du multiensemble des deux littéraux non ASCII dans la façade et les blocs déplacés : valeurs inchangées. Aucune conversion globale d'encodage.

### Campagne et preuves avant la dernière retouche de visibilité

Préflight canonique et auto-tests de la garde F2 réussis avant Cargo, cache/fixtures/modèles copiés/temporaires/sorties sous `target/parakeet-tests/refacto-worker-ipc-f2/`, seule cible de compilation `target/parakeet-tests/cargo-shared/`. CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only, commandes offline/locked, fmt limité à --check et refus --fix maintenus. Aucune inspection Git/recherche concurrente aux commandes gardées. Consignes de session sur événements extérieurs sans lien projet et objets Git maintenues sans lever les protections des sources, ressources, index/HEAD/refs/config ou écritures suspectes/attribuées. Aucun incident de garde F2.

Le premier format --check après extraction a signalé des retouches de présentation ; elles ont été appliquées manuellement hors commande gardée, puis le contrôle a réussi. Aucune commande Cargo de formatage en écriture.

Sept T-F du plan réellement exécutés individuellement et réussis avant/après extraction : Auto sans GPU, GPU Ready attesté, repli Auto avant Begin, échec GPU manuel, Stop empêchant retry, worker conservé/cache et routes natives préparées bornées. Huit témoins unifiés supplémentaires réellement exécutés et réussis avant/après :

- panicking_unified_worker_terminates_the_inflight_session_and_its_peer ;
- disconnected_unified_inference_result_fails_the_inflight_session_and_preserves_its_peer ;
- failed_ready_is_reaped_before_the_executor_recycles ;
- native_executor_backend_changes_only_after_the_prior_instance_is_reaped ;
- unified_child_retains_only_two_job_ids_until_real_worker_recycle ;
- controlled_document_drain_reads_one_segment_per_poll_and_preserves_all_pages ;
- import_application_resumes_a_verified_interrupted_unified_import ;
- import_application_finalizes_stopped_capture_a_while_capture_b_pumps_and_publishes_segmented_audio.

Intégrations publiques réellement exécutées avant/après extraction : worker_control, 5 tests PASS ; gpu_qualification_transport, 1 test PASS. Format --check, git diff --check et check workspace PASS. Cinq suites séquentielles complètes baseline puis candidat ; catalogue injecté et legacy réellement RUN/PASS dans les dix suites :

| Configuration | Réussis baseline et candidat avant retouche finale | Échoués | Ignorés | Filtrés autorisés | Legacy réel |
| --- | --- | --- | --- | --- | --- |
| défaut | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks | 276 | 0 | 16 | 1 D2 corpus natif | oui |
| l01-memory-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| personal-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks,l3-gpu-qualification | 276 | 0 | 16 | 2 D2 corpus natif/l3c | oui |

Journaux des suites candidat : `Run-20261030T143649866-347d1fe2`, `Run-20261030T143857014-c7dc07a3`, `Run-20261030T144105513-a9e903e9`, `Run-20261030T144309070-4a6f7be2`, `Run-20261030T144523971-af41c081`, sous F2/logs avec stdout/stderr et index guard.jsonl.

Deux contrôles conditionnels réellement exécutés isolément avant/après extraction avec ressources bornées F2 : paths_reject_a_worker_outside_the_runtime_root, avant `20261030T143203965-4f2fee22`, après `20261030T144757804-fe3d81f1` ; over_limit_parakeet_import_is_refused_before_pending_creation, avant `20261030T143205182-38b47e58`, après `20261030T144758983-3fcda2ba`. Tous PASS ; le compte libtest seul n'est pas présenté comme preuve d'exécution du corps. Le refus anticipé ne lance aucun worker natif.

### Retouche avant revue et preuve finale composée

Le bilan Clippy du candidat a révélé un avertissement private_interfaces supplémentaire : le champ interne commands avait reçu une visibilité plus large que son type. L'implémenteur a rétabli sa visibilité privée, comme dans la base, avant la première revue. Aucun corps, contrat ou autre champ modifié par cette retouche. Les usages de commands et son enum restent locaux à executor, sans accès externe ni condition de feature affecté, ce que le reviewer a confirmé.

Après cette seule restriction de visibilité : format --check `20261030T144846792-232a2d3a`, check workspace `20261030T144848640-6e195b5d`, Clippy `20261030T144852736-a76d1245` et témoin panicking_unified_worker_terminates_the_inflight_session_and_its_peer `20261030T144856503-23129b27` PASS. Clippy final : 23 diagnostics core et 146 adapters, identiques à la baseline, soit 169 hors résumés. Dette de structure héritée déplacée, aucun nouveau diagnostic final ni suppression de lint ; cibles globales non revendiquées atteintes.

Les cinq suites, les autres témoins, les intégrations et les deux conditionnels n'ont PAS été relancés après cette retouche. Le reviewer a jugé leurs résultats toujours applicables : seul un champ redevient privé, aucun corps ni contrat ne change, et les contrôles finaux compilent/vérifient le candidat final. La preuve de clôture est composée et ne prétend pas à cinq suites relancées sur le commit final.

### Revue, limites et conservation

Verdict indépendant **CLEAN** sur le candidat final working tree, époque 0 : une passe de revue, zéro correction de revue, aucun constat ouvert, aucune demande de vérification, aucun gap matériel ni blocage dans F2. Les 16 ignorés et exclusions D2 corpus natif/l3c ne sont pas revendiqués exécutés. Campagnes natives CPU/GPU/Auto et validation dans la fenêtre Windows non réalisées ; clôture structurelle seulement. Aucun lot F3 démarré ou autorisé par ce verdict.

Modèles originaux/copies contrôlés, tailles, SHA-256 et dates pré/post identiques : Turbo 1 624 555 275 octets, `1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69` ; Parakeet 713 975 456 octets, `E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E`. Originaux utilisés en lecture seule, copies et liens bornés sous F2 conservés.

Archive `target/parakeet-tests/refacto-worker-ipc-f2/archive/closure-20261030/proofs.zip` : 148 entrées uniques, 429 535 octets ; arborescence relative préservée et SHA-256 de chaque entrée vérifié en mémoire contre sa source. SHA-256 ZIP : `3CA01FB67BF4D594797BC4F25D20BD3D95430B0A9EFAB17472DD2CF88798DB76`. Aucun modèle/cache/build/corpus/WAV archivé, aucun nouveau manifeste, aucun nettoyage/suppression ou restauration. Cargo-shared conservé.

Espace libre final : 94 572 056 576 octets (environ 88,1 Gio), supérieur au seuil de 30 Gio. Aucun processus Cargo/rustc/rustfmt actif. Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Sources relues et bilan réunis dans le commit local de clôture, sans push.
## 37. Démarrage autorisé du lot F3 — 30 octobre 2026

Avancée : F2 clos CLEAN sur `571a4db445fb63c6632ccd6fb624935de223b2c6` ; inventaire exhaustif préliminaire du reliquat F3 établi.
Blocage : aucun obstacle certain en lecture seule ; visibilités, supports de tests et tailles finales à confirmer sous garde.
Décision : exécuter F3 seul selon son budget et ses propriétaires, revue indépendante puis commit local sans push si clôture démontrée.

Accord écrit courant : « Passe au lot suivant selon le plan. Je t’autorise l’implémentation, les vérifications et le commit local, sans push. Clôture-le sans réserve après revue indépendante. » F3 est le prochain et dernier lot de ce plan, dépendant de F2 clos. La clôture attendue est structurelle, à comportement constant, sans qualification native ou Windows implicite.

La façade comporte encore 1 607 lignes physiques : trois constantes, 26 helpers libres hors variantes cfg, six types/enum et des impls de VAD/live/ImportRuntime et du contrat ServiceRuntime ; deux includes de support de tests préexistants. runtime/mod.rs comporte 341 lignes dont environ 100 lignes d'injections de test/commandes à affecter à un propriétaire. Ces relevés seront confrontés aux définitions finales, sans déduire la couverture d'un simple comptage lexical.

Budget autorisé : huit fichiers production maximum, dans les chemins F3 du plan. Inventaire estimé à six : `worker_ipc.rs`, `runtime/mod.rs`, nouveau `runtime/support.rs`, `support.rs`, `runtime/live.rs`, `runtime/import.rs` ; subdivisions déjà créées utilisables seulement dans le même budget. Aucun autre chemin, nouveau mécanisme ou lot ajouté. Affecter admission/démarrage/exécution d'import unifié au propriétaire import, capture/admission/arrêt et helpers live/VAD au propriétaire live/poll pertinent ; helpers WAV/temps/hooks/préparation/identité/diagnostics à runtime/support selon leurs consommateurs. Les aides techniques transversales restent dans support, sans dépendance runtime/service/archive. runtime/mod reste état, constructeurs et déclarations ; l'implémentation du contrat ServiceRuntime reste runtime, sans déplacer ce contrat dans les modules techniques. Import estimé à environ 1 432 lignes et live à 1 285 avant imports et répartition finale : vérifier les plafonds, ne pas comprimer le code ni étendre le périmètre si ces estimations sont insuffisantes.

Cible stricte : façade composée de déclarations et réexports ciblés, zéro corps algorithmique résiduel, symboles uniques avec propriétaire explicite, API publique et features inchangées. Aucun nouveau #[path] ordinaire, conservation des supports de tests historiques et de leurs accès privés nécessaires. Bornes, ordres, identités, atomiques, messages, reprise, ACK, publication, capture et choix/repli moteur conservés. Aucune dépendance/feature/unsafe, changement Cargo.lock, politique, protocole ou optimisation ajouté. Bilan des dettes héritées et carte finale des propriétaires dans ce plan, sans nouveau rapport.

Campagne sous `target/parakeet-tests/refacto-worker-ipc-f3/`, garde F2 adaptée sans assouplissement, cache/fixtures/modèles copiés/temporaires/sorties bornés sous F3, cible Cargo unique `target/parakeet-tests/cargo-shared/`. Préflight canonique et auto-tests avant tout Cargo, espace ≥30 Gio, offline/locked, CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only ; fmt limité à --check et --fix refusé. Aucune inspection Git/recherche récursive concurrente aux commandes gardées. Dispositions de session sur événements extérieurs sans lien projet et objets Git maintenues, sans lever les protections des ressources, sources, index/HEAD/refs/config ou écritures suspectes/attribuées. Aucune exception générale issue des incidents ponctuels antérieurs.

V0–V6, T-A à T-F et M1–M3 sur tout le sous-arbre avant/après, revue exhaustive des symboles résiduels racine et runtime/mod. Cinq configurations séquentielles, mêmes données et legacy réellement actif et borné ; témoins unifiés/Ready/ACK/reprise et intégrations publiques pertinents. Corps conditionnels examinés et exécutés avec ressources bornées lorsque requis ; pas de succès déduit d'un retour anticipé. Seules exclusions D2 corpus natif/l3c historiques, 16 ignorés distingués ; aucune nouvelle exclusion. Modèles originaux lecture seule, copies/liens bornés et état pré/post contrôlé. UTF-8 explicite, comparaison exacte des messages et des corps, aucun réencodage global. Archivage des petites preuves avec compte et SHA-256 à la clôture, aucun nettoyage sans manifeste approuvé.

Espace libre initial : 93 876 150 272 octets (environ 87,4 Gio). Suivi propre avant démarrage ; `docs/architecture/`, `models/`, `sound example/` préservés et exclus des commits. Même rust_implementer pour l'implémentation, coordinateur et même reviewer indépendant pour la revue ; deux cycles maximum. Session principale auteur du bilan et des commits locaux, aucun push.
## 38. Clôture du lot F3 et bilan structurel final — 30 octobre 2026

Avancée : façade fine, responsabilités attribuées et revue indépendante finale CLEAN après correction de frontière WAV.
Blocage : aucun constat ouvert ni vérification requise manquante dans le périmètre autorisé.
Décision : F3 et le plan de refactorisation clos sans réserve dans ce périmètre ; commit local sans push.

### Delta final et carte des propriétaires

Base Git exacte : `c36d6380a3c48386d60fe452e9e44e0349b9796e`, après le démarrage du §37. Implémentation par le même rust_implementer, revue par rust_review_coordinator et le même rust_reviewer indépendant en lecture seule. La session principale n'a pas modifié le code en parallèle. Huit fichiers production, budget respecté ; aucun test, manifeste, Cargo.lock, dépendance ou feature modifié.

| Fichier touché sous worker_ipc | Lignes physiques finales | Responsabilité |
| --- | --- | --- |
| façade `worker_ipc.rs` | 130 | déclarations, imports et réexports ciblés ; deux includes exclusivement test préexistants |
| `runtime/mod.rs` | 264 | état, quatre constructeurs et déclarations |
| nouveau `runtime/support.rs` | 458 | hooks recette, temps, préflight/collision, scan historique, identités unifiées, diagnostics, injections de tests et impl du contrat ServiceRuntime |
| `support.rs` | 94 | aides techniques transversales : SHA, compteurs/capacités et writer WAV pur |
| `runtime/live.rs` | 1 386 | état/capture/admission live, démarrage/arrêt unifié, binding et confirmation de fenêtre |
| `runtime/import.rs` | 1 444 | routes/admission/démarrage import, traitement contrôlé et binding import |
| `runtime/poll.rs` | 1 234 | polling principal et filtrage identité/tentative/arrêt |
| `runtime/poll/parakeet.rs` | 996 | polling Parakeet, état et aides de validation/VAD |

Les aides live/VAD rejoignent live/poll selon l'outline F3 ; aucune subdivision supplémentaire créée en dehors de runtime/support prévu. Chaque fichier touché est sous 1 500 lignes physiques, donc aussi sous le plafond de lignes de production. runtime/mod ne porte plus les injections de tests ou les helpers algorithmiques. La façade passe de 1 607 à 130 lignes physiques et ne contient aucun fn/struct/enum/impl/const de production ; ses includes historiques restent exclusivement sous cfg(test), sans nouveau #[path] ordinaire.

Carte finale de l'ensemble de la refactorisation :

| Responsabilité | Propriétaire final |
| --- | --- |
| Transport enfant et framing | `ipc/mod.rs`, `ipc/framing.rs` |
| Interverrou atomique PublicationGate | `publication/mod.rs` |
| Service asynchrone, admission et finale automatique | `service/mod.rs`, `service/automatic_final.rs` |
| État et constructeurs runtime | `runtime/mod.rs` |
| Dispatch et préambule/épilogue uniques | `runtime/effects.rs` |
| File, doublons, reprise et identité de source | `runtime/queue.rs`, `duplicates.rs`, `recovery.rs`, `source_binding.rs` |
| Capture/direct, import et démarrage worker | `runtime/live.rs`, `runtime/import.rs`, `runtime/import/worker_start.rs` |
| Polling principal, capture, Parakeet et sessions unifiées | `runtime/poll.rs` et ses enfants, `runtime/poll_unified.rs` |
| Catalogue, bindings, sélection, cache et preuves/routes moteur | `backends/mod.rs`, `backends/native.rs` |
| Sources/état/drain, exécuteur et transcripteur unifiés | `parakeet_unified/mod.rs`, `executor.rs`, `native.rs` |
| Aides runtime et techniques transversales | `runtime/support.rs` et `support.rs`, avec frontières distinctes |

Le reviewer a confirmé l'identité de 105 corps de fonctions et de leurs conditions de compilation, et la façade fine. La comparaison mécanique de 42 noms de fonctions racine retrouve les corps de la base en normalisant seulement espaces et virgules finales de présentation ; aucun mismatch. Les 22 littéraux non ASCII conservent exactement leurs valeurs. Tous littéraux : 379 vers 381, seuls deux recette-hooks ajoutés dans des attributs de réexport, aucun message supprimé ou modifié. API/réexports publics, identités, ordre, ACK, publication, atomiques, bornes et politique de repli inchangés. EVENTS_PER_STAGE reste 21, anti-famine 2 et capacité unifiée 2 ; aucune pool, planification ou dépendance nouvelle. support transversal ne dépend ni de runtime, ni de service, ni d'archive.

### Campagne et preuves complètes — époque 0

Préflight canonique, variables, auto-tests et garde F3 réussis avant Cargo ; cache/fixtures/modèles copiés/temporaires/sorties sous `target/parakeet-tests/refacto-worker-ipc-f3/`, seule cible de compilation partagée `target/parakeet-tests/cargo-shared/`. Offline/locked, CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only. Refus fmt sans --check et --fix conservés ; aucune inspection Git/recherche concurrente à la campagne. Dispositions de session sur événements extérieurs sans lien projet et objets Git maintenues sans lever les protections des sources, ressources, index/HEAD/refs/config ou écritures suspectes/attribuées. Aucune violation de garde F3.

Les premiers essais d'extraction ont signalé une accolade, des visibilités/imports et des conditions cfg à corriger, puis des accès privés de supports tests : checks `20261030T152440293-2eda68db`, `152455985-1c52fac3`, découverte `152541904-1761fe39`, en erreur et conservés dans les preuves. Les retouches ont précédé la validation aboutie. Les fmt --check en erreur de présentation ont été suivis de corrections manuelles hors commande gardée, avant les contrôles réussis ; aucune commande de formatage Cargo en écriture. Ces essais intermédiaires ne sont pas présentés comme PASS.

36 filtres T-A à T-F du §7, chacun non nul, réellement exécutés et réussis avant/après extraction. Intégrations publiques worker_control (5 tests) et gpu_qualification_transport (1) PASS avant/après. Les huit témoins unifiés F2 de panique, déconnexion, Ready/reap, recyclage, drain, reprise et publication concurrente sont réellement PASS dans les dix suites. Format --check, workspace, diff check et Clippy de l'époque 0 PASS.

| Configuration | Réussis baseline et candidat époque 0 | Échoués | Ignorés | Filtrés autorisés | Legacy réel |
| --- | --- | --- | --- | --- | --- |
| défaut | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks | 276 | 0 | 16 | 1 D2 corpus natif | oui |
| l01-memory-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| personal-qualification | 272 | 0 | 16 | 1 D2 corpus natif | oui |
| recette-hooks,l3-gpu-qualification | 276 | 0 | 16 | 2 D2 corpus natif/l3c | oui |

Journaux des cinq suites candidat époque 0 : `Run-20261030T153335468-886122d9`, `Run-20261030T153641826-91373de9`, `Run-20261030T153934616-ec915d8c`, `Run-20261030T154236358-7ebec17f`, `Run-20261030T154511158-33280d28`, sous F3/logs avec stdout/stderr et index guard.jsonl.

Deux conditionnels réellement exécutés isolément avant/après extraction avec ressources bornées F3 : paths_reject_a_worker_outside_the_runtime_root (avant `20261030T152321058-6d90fd1c`, après `20261030T154753291-557023e3`) et over_limit_parakeet_import_is_refused_before_pending_creation (avant `20261030T152322204-b9a9db22`, après `20261030T154754483-f6e0b5bc`). Tous PASS ; pas de déduction de l'exécution du corps à partir du seul compte libtest. Le refus anticipé ne lance aucun worker natif.

### Constat de revue, correction et preuve finale — époque 1

Première revue : WIPC-F3-R01 MINEUR. write_pcm16_wav avait été affecté à runtime/support, alors que le transcripteur natif de parakeet_unified le consommait via l'alias racine. Malgré l'identité fonctionnelle et les tests réussis, cette résolution introduisait une dépendance de la primitive native vers runtime, contraire à la frontière F2. Le reviewer a demandé son affectation au support technique transversal, avec corps/signature/messages et alias interne inchangés.

Le même implémenteur a déplacé ce writer pur dans support.rs, ajusté l'alias façade et retiré son réexport runtime. native.rs reste inchangé. Le même reviewer a vérifié l'unicité, la résolution vers support, l'absence de définition/réexport runtime et l'identité du corps, de la signature et des messages avec la base, puis fermé lui-même WIPC-F3-R01.

Contrôles après correction, sous garde : format --check `20261030T160547953-2827a9e7`, test réellement exécuté native_window_wav_is_short_pcm16_mono_16khz_riff `20261030T160549733-2a34de77`, workspace `20261030T160613585-c62820f1`, check adapters avec l01-memory-qualification `20261030T160617695-1a0a9347`, Clippy `20261030T160621168-dac84796` et git diff --check : tous PASS.

Les cinq suites, les 36 filtres, les intégrations et les conditionnels restent des preuves de l'époque 0 ; ils n'ont PAS été relancés après ce déplacement pur. Le reviewer a expressément jugé suffisante la comparaison du writer et les contrôles ciblés demandés. La preuve de clôture est composée, et ne revendique pas cinq suites sur le commit final.

### Dette héritée, verdict et limites de qualification

Clippy final : 23 diagnostics core et 146 adapters, soit 169 hors résumés ; mêmes libellés en multiensemble que la baseline. Aucun nouveau diagnostic final ni suppression de lint. Les fonctions longues/complexes et signatures au-delà des cibles, déjà présentes et déplacées littéralement, constituent la dette héritée conservée : cette extraction ne revendique pas la conformité globale aux seuils fonctions/complexité/arguments/types. Les bilans précédents attribuent notamment les handlers de reprise, le polling principal et unifié, les helpers de capture et les signatures VAD/repli. Aucune correction métier ou campagne de réduction globale de cette dette n'a été ajoutée à F3.

Verdict indépendant final **CLEAN**, époque 1 : deux passes de revue, un cycle de correction, WIPC-F3-R01 fermé, aucun constat ouvert, aucune demande de vérification requise ni gap matériel restant. Le plan de treize lots A, B, C, D1–D5, E1–E2, F1–F3 est structurellement clos dans le périmètre autorisé, avec bilans et preuves propres à chaque lot. Les incidents et dispositions antérieurs restent historiques ; cette clôture ne transforme pas les preuves composées des lots précédents en suites rétrospectivement rejouées.

Les 16 tests ignorés et exclusions historiques D2 corpus natif/l3c ne sont pas revendiqués exécutés ; aucun nouveau skip. Qualification native CPU/GPU/Auto, qualité/vitesse d'inférence et validation dans la fenêtre Windows non réalisées par cette refactorisation. CLEAN porte sur le périmètre structurel relu et ses preuves automatisées, sans validation produit implicite.

### Conservation et état final

Modèles originaux et copies F3 contrôlés, tailles/dates/SHA-256 identiques pré/post : Turbo 1 624 555 275 octets, `1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69` ; Parakeet 713 975 456 octets, `E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E`. Originaux en lecture seule, copies et liens bornés conservés.

Archive `target/parakeet-tests/refacto-worker-ipc-f3/archive/closure-20261030/proofs.zip` : 262 entrées uniques, 667 622 octets, arborescence relative conservée ; SHA-256 de chaque entrée comparé en mémoire à sa source. SHA-256 ZIP : `94500FC481E916C88BA9ED8AB450985CAB65609FAAED85D58CEFD69BA4825FF4`. Preuves des deux époques distinctes. Aucun modèle/cache/build/corpus/WAV archivé, aucun manifeste nouveau ni nettoyage/suppression/restauration. Cargo-shared conservé.

Espace libre final : 90 242 392 064 octets (environ 84,0 Gio), supérieur au seuil de 30 Gio. Aucun processus Cargo/rustc/rustfmt actif. Non suivis préexistants `docs/architecture/`, `models/`, `sound example/` préservés et exclus du commit. Huit fichiers production relus et présent bilan réunis dans le commit local de clôture, aucun push.
