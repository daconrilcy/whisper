Avancée : cadrage proposé du découpage progressif de worker_ipc, fondé sur les sources du 29 octobre 2026.
Blocage : aucun pour la rédaction ; exécution et campagnes non autorisées par ce document.
Décision : conserver l'ordre 3a–3f demandé et le comportement existant à chaque lot ; proposition soumise à acceptation.

# Cadrage — refactorisation de worker_ipc

- ID : D-REF-WORKER-IPC, version 1.0, 29 octobre 2026.
- Auteur : session Codex principale ; propriétaire des décisions produit : utilisateur.
- Statut : PROPOSÉ ; aucune autorisation d'implémentation, de campagne ou de push.
- Source d'autorité : demande de cette session « rédige le cadrage et le plan », tableau de l'étape 3 et principe de l'étrangleur. Cette demande autorise les deux documents et leur versionnement local ; elle ne constitue pas leur acceptation finale.
- Base inspectée : `939468382dacfff1c119c8acd79f08b3b7385392`, branche `evo-startup-perf`.
- Plan associé : [plan-refacto-worker-ipc.md](plan-refacto-worker-ipc.md).

## 1. Besoin et résultat produit

Réduire le risque de régression lors des prochaines évolutions de transcription fichier et microphone : isoler le transport, la publication, le service, les familles d'effets, le polling et les moteurs aujourd'hui réunis dans un même fichier. L'utilisateur de l'application doit retrouver exactement les mêmes commandes, événements, erreurs, fichiers et règles d'arrêt après chaque extraction.

Les utilisateurs concernés sont l'utilisateur Windows actuel et les mainteneurs Rust. Aucun nouvel écran, format, moteur, mode de capture ou comportement de reprise n'est demandé. Le bénéfice attendu est une modification ultérieure plus localisée et vérifiable, sans promesse de gain de vitesse, mémoire ou qualité de transcription.

## 2. Existant vérifié et limites des observations

Sources primaires : `AGENTS.md`, `crates/whisper-adapters/AGENTS.md`, manifestes, `crates/whisper-adapters/src/lib.rs`, `worker_ipc.rs` et son arbre de tests. Carte et fiche `docs/architecture/` consultées comme relevés statiques non suivis, confrontées aux sources, sans les ajouter à Git.

Dans `crates/whisper-adapters/src/worker_ipc.rs` à cette base :

| Élément | Ancres actuelles | Observation |
| --- | --- | --- |
| Racine | 1–13 214 | 13 214 lignes physiques ; dernière déclaration `mod tests` séparée |
| PublicationGate | 526–582 | machine atomique, appelée par le runtime et le service |
| Transport | 584–1 331, par symboles | `ChildWorkerTransport`, commandes, lecture, framing ; helpers d'essai live intercalés |
| État | 1 332–1 420 | `ImportRuntime`, nombreuses ressources et identités de sessions |
| Parakeet unifié | 1 421–2 741, puis méthodes dispersées | source, route, transcripteur, exécuteur, restauration et sessions |
| Catalogue et routes | 2 913–3 583 notamment | modèle, candidat GPU, reprise CPU, worker conservé |
| Polling unifié | 4 719–5 685 | `poll_unified_sessions`, avec drain du document en amont |
| Effets | 6 305–8 219 | `effect`, préambule, dispatch, épilogue d'erreur/persistance |
| Polling principal | 10 958–12 056 | `poll`, plus capture/VAD/worker en amont |
| Service | 12 077–13 211 | `AsyncImportIo`, `ImportIoPort`, `ServiceRuntime`, boucle, helpers d'export final |

Les plages sont des repères de lecture, jamais des instructions de copier-coller. La complexité 101 de `effect()` est donnée par la demande ; aucune mesure Clippy n'a été exécutée pour la confirmer. La racine contient déjà zéro fonction `#[test]` : les tests sont dans `worker_ipc/tests.rs` et ses fragments `tests/*.rs`. Leur migration globale n'appartient pas à ce chantier.

État Git initial : aucun changement suivi ; `docs/architecture/`, `models/`, `sound example/` non suivis, à préserver. Espace C: relevé : 123 519 447 040 octets libres, environ 115 Gio. Ce relevé documentaire ne remplace pas le préflight de chaque future campagne. Aucun Cargo, build, test, worker natif ou essai Windows lancé ici.

## 3. Périmètre et décisions de conception proposées

Ordre imposé par la demande : 3a transport → 3b PublicationGate → 3c service → 3d file, doublons, direct, reprise, import, une famille par lot → 3e polling → 3f backends et Parakeet unifié.

DEC-01 : extraction dans la même crate, sans dépendance nouvelle, sans modifier les manifestes, `Cargo.lock`, le protocole IPC ni le cœur métier. `worker_ipc.rs` reste le point d'entrée public compatible.

DEC-02 : convention Rust `worker_ipc.rs` + `worker_ipc/<responsabilité>/mod.rs` et fichiers enfants. Aucun `#[path]` pour contourner la résolution normale. Les sous-modules restent privés ; utiliser les visibilités minimales vers `crate::worker_ipc`, sans nouveaux exports au niveau de la crate pour faciliter les tests. Les `pub use` de la façade maintiennent les chemins publics existants de `ChildWorkerTransport`, `AsyncImportIo`, `probe_cuda_worker`, `qualify_gpu_worker` et la visibilité existante de `gpu_evidence_matches`.

DEC-03 : `ImportRuntime` reste un seul propriétaire d'état pendant la migration. Les handlers sont des méthodes `impl ImportRuntime` réparties par responsabilité, sans cloner son état ni introduire de traits génériques ou d'objets de contexte artificiels. Les enfants peuvent accéder à l'état privé de leur ancêtre ; après déplacement dans `runtime/mod.rs`, limiter la visibilité des méthodes et des types réellement partagés entre modules frères. Aucun champ n'est rendu public à l'extérieur de `worker_ipc`.

DEC-04 : `effect(&mut self, ImportEffect, &mut VecDeque<ImportEvent>)` garde sa signature. Son préambule et son épilogue restent sous un propriétaire unique. Chaque bras extrait devient un appel à son handler. Préserver la différence entre retour du dispatcher, retour d'une closure et résultat qui doit passer par l'épilogue. Si nécessaire, un petit résultat interne explicite distingue « terminé, quitter effect » de « poursuivre l'épilogue » ; pas de transformation mécanique de tous les `return` en `Result`.

DEC-05 : la façade devient fine à la clôture, avec déclarations de modules et réexports ciblés. Pendant les lots intermédiaires, son contenu métier résiduel est intentionnel et décroissant ; il doit être nommé dans le bilan du lot. Le plan attribue aussi les helpers périphériques pour ne pas finir avec une façade encore monolithique.

DEC-06 : le périmètre nommé `publication/` extrait ici uniquement `PublicationGate` et ses constantes. Les fonctions de publication de l'archive et les exports automatiques restent rattachés à leur propriétaire d'exécution ; aucune dépendance `archive → worker_ipc` n'est introduite. La consigne locale « préserver PublicationGate dans worker_ipc.rs » décrit l'existant ; le contrat et son chemin interne sont préservés par réexport privé lors du déplacement explicitement demandé.

## 4. Carte des responsabilités et dépendances

Tous les chemins suivants sont sous `crates/whisper-adapters/src/worker_ipc/`.

| Propriétaire | Éléments actuels | Destination et dépendances permises |
| --- | --- | --- |
| Transport technique | enfant, stdin/stdout, bornes, framing, attestation des événements | `ipc/mod.rs`, `ipc/framing.rs` ; std, serde, core IPC, ports et WorkerTransport ; pas d'archive, service ou runtime |
| Interverrou de publication | PublicationGate, constantes GATE | `publication/mod.rs` ; atomiques std et PortError uniquement |
| Service d'adaptateur | AsyncImportIo, ServiceRuntime, service_loop, admission et exports automatiques | `service/mod.rs`, `service/automatic_final.rs` ; runtime via contrat existant, publication, core, archive/file pour les exports déjà présents ; pas de dépendance inverse du transport |
| Exécution d'effets | état, dispatch, IO et événements | `runtime/mod.rs`, `runtime/effects.rs`, `runtime/{queue,duplicates,live,recovery,import}.rs` ; core, archive et adaptateurs existants, publication, transport, backends et sessions |
| Polling | poll et aides capture/VAD, poll_unified_sessions/document | `runtime/poll.rs`, `runtime/poll_unified.rs` ; mêmes propriétaires d'état et handlers, sans nouvelle machine d'états |
| Choix et démarrage moteur | RuntimeModelCatalog, routes, preuves GPU, worker réutilisable et tentatives | `backends/mod.rs`, `backends/native.rs` ; core, catalogue/supervisor et transport ; aucune dépendance vers service |
| Exécution Parakeet unifiée | sources, drain, exécuteur, transcripteur, sessions | `parakeet_unified/mod.rs`, `parakeet_unified/executor.rs`, `parakeet_unified/native.rs` ; core, transport et modules Parakeet existants ; aucune dépendance vers service |
| Helpers du runtime | identité de source, journal, WAV, diagnostics et helpers de préparation | `runtime/source_binding.rs`, `runtime/support.rs` ; adaptateurs existants et core ; pas de nouveau domaine métier |
| Tests | tests.rs et fragments existants | conservés à portée privée ; nouveaux petits tests privés dans fichiers séparés ; nouveaux scénarios publics dans `crates/whisper-adapters/tests/worker_ipc_*.rs` si réellement nécessaires |

Les `impl ImportRuntime` de démarrage/admission restent dans `runtime/` et appellent les primitives extraites de `backends/` ou `parakeet_unified/`. Ne pas créer un cycle en déplaçant une méthode d'orchestration dans un module technique. Le service compose le runtime ; le runtime peut implémenter le contrat ServiceRuntime existant sans que les modules techniques importent le service. Dépendances inter-crates inchangées : adapters → core, desktop → adapters + core. Les politiques restent dans core.

## 5. Exigences, parcours et acceptation

| Exigence | Parcours / cas d'usage | Critère observable et preuve attendue | Lots |
| --- | --- | --- | --- |
| REQ-01 Identité et transport inchangés | UC-01 démarrage worker, trame invalide, EOF, événement tardif | AC-01 mêmes commandes/événements et erreurs ; rejet de trames surdimensionnées/incomplètes ; anciens JobId/Generation/instance_id filtrés comme avant | 3a, 3e, 3f |
| REQ-02 Arrêt/publication inchangés | UC-02 Stop avant commit, pendant commit, arrêt déjà demandé | AC-02 Stop avant commit interdit publication ; pendant commit PortError::Committing ; rollback et état après échec identiques ; ACK de persistance avant confirmation conservés | 3b, 3c, 3d, 3e |
| REQ-03 Admission bornée inchangée | UC-03 file pleine, Stop concurrent au démarrage, déconnexion | AC-03 bornes, priorités, résultat full/disconnected, ordre et nombre des événements identiques ; aucun canal audio/worker non borné ajouté | 3a, 3c, 3d, 3e |
| REQ-04 Transcription et reprise inchangées | UC-04 import WAV/MP3, doublon, file FIFO, reprise et source modifiée | AC-04 mêmes choix, identités, chemins, journal, confirmations, nettoyage et erreurs ; aucun export TXT/SRT/WAV différent à entrée identique | 3d, 3e, 3f |
| REQ-05 Capture et session unifiée inchangées | UC-05 direct, gain, Stop, import pendant capture, fermeture | AC-05 audio annoncé capturé conservé selon contrat existant ; mêmes admissions, pending, drains, publication de queue finale et refus de nouveaux travaux à la fermeture | 3d direct, 3e, 3f |
| REQ-06 Sélection et réutilisation moteur inchangées | UC-06 CPU/GPU/Auto, panne avant/après début audio, cache prêt | AC-06 même attestation, choix fixe manuel, repli Auto existant borné, identités des tentatives, réutilisation compatible ; aucun repli ajouté après début audio | 3f |
| REQ-07 Extraction sans élargissement | UC-07 appelant desktop/bootstrap/tests et configurations | AC-07 chemins publics et signatures identiques ; nouveaux modules privés, aucune dépendance/feature ni unsafe ajouté ; tous les bras délégués au terme de 3d, façade fine au terme de 3f | tous |

Les exigences héritées portent sur la préservation du comportement réellement présent ; une divergence préexistante entre produit et spécification doit être rapportée et ne peut être corrigée silencieusement dans cette refactorisation. La validation automatisée compare les témoins avant/après par lot, avec même configuration et mêmes données ; elle ne démontre ni la qualité native CPU/GPU ni une validation dans la fenêtre Windows.

## 6. États et contrats critiques à conserver

PublicationGate : IDLE → ACTIVE par activate ; ACTIVE → CANCELLED par request_stop ; rollback_stop ne fait que CANCELLED → ACTIVE ; ACTIVE → COMMITTING par commit, puis COMMITTED si succès. Un commit en erreur ne réactive pas automatiquement la gate. Conserver exactement les ordres mémoire Release/Acquire/AcqRel et les messages d'erreur existants ; ne pas « réparer » cet état à l'occasion du déplacement.

Effets : entrée consommée une seule fois, mutations du même ImportRuntime, événements ajoutés dans la même VecDeque ; captures d'identité calculées au même moment. Le compteur de persistance live, la confirmation différée et la centralisation des échecs doivent s'exécuter sur les mêmes chemins qu'avant. Les sorties précoces du dispatcher restent précoces.

Transport : conserver limite de trame avec newline, framing et version, ordre du canal normal et du canal Stop, saturation, corrélation et attestation GPU, destruction/abort du processus et terminaison lecteur. `spawn_live_attempt`, la réservation d'essai et le remapping de séquences relèvent de l'exécution live, pas du framing.

Service : conserver réservations d'événements, traitement Start/Stop, déférés, pompe temps réel, traitement des déconnexions, attente de 5 ms et non-blocage des soumissions. Les constantes de capacité restent identiques, même si leur propriétaire change.

Sessions/moteurs : conserver admission, route préparée, démarrage, ACK Ready/Stopped, arrêt et récolte, drain document/audio, rejet d'identité obsolète, restauration de préfixe et libération des ressources. Aucune nouvelle garantie de durabilité ou reprise après panne n'est créée.

## 7. Risques et traitement

| Risque concret | Traitement et arrêt |
| --- | --- |
| Retour anticipé converti en simple succès, exécutant un épilogue auparavant évité | inventorier les sorties à chaque famille ; témoin ciblé ; arrêt si équivalence non établie |
| Visibilité Rust et fixtures privées cassées | réexports internes minimaux et imports explicites dans tests ; aucun élargissement public pour sauver les fixtures |
| Déplacement mélangeant transport et reprise | extraction par symboles et responsabilité, jamais par plage |
| Inversion de dépendance ou nouvelle politique dans adapters | revue de la carte ci-dessus ; garder orchestration dans runtime, aucun changement core/archive |
| Test déclaré réussi mais non exécuté faute de variables | distinguer découvert, exécuté, ignoré et NOT RUN ; vérifier sorties et conditions de skip |
| Ancienne garde réutilisée avec dérogations historiques | aucune réutilisation automatique ; accord précis et préflight strict avant toute future commande Cargo |
| Gros sous-module recréant le monolithe | taille de production ≤1 500 lignes par nouveau fichier ; subdivision au sein de la responsabilité et du budget ; dépassement de périmètre soumis à arbitrage |

Les seuils des AGENTS (100 lignes/fonction, complexité 25, 7 arguments, type 250) sont des cibles de revue. Les longs corps déplacés sans changement peuvent conserver une dette héritée précisément attribuée ; aucune exemption globale ni objectif chiffré de complexité 101 → X inventé. Le dispatch final doit être délégué, même si les handlers gardent provisoirement des branches complexes.

## 8. Livrables, exclusions et readiness

Deux fichiers suivis uniquement : ce cadrage et le plan associé. Les retours de revue documentaire sont intégrés à ces fichiers, sans campagne de rapports supplémentaires. Pas de nouveaux rails, manifeste de qualification, CI distante, service Windows, ACL, technologie, nettoyage de target ni modification des sources pendant cette mission.

Le plan est proposé dans la même remise parce que l'utilisateur l'a demandé explicitement ; cela ne transforme pas le design proposé en design accepté. Avant implémentation : acceptation écrite des documents versionnés, lot et chemins précis autorisés, responsable `rust_implementer`, prérequis et campagne explicitement autorisés. Revue du candidat par `rust_review_coordinator`, fermeture des constats par reviewer indépendant, deux cycles maximum. Validation dans la fenêtre Windows réservée à l'utilisateur lorsqu'une recette sera autorisée.

Couverture documentaire : brief §1/3 ; exigences/parcours/acceptation §5 ; domaine et états §6 ; architecture et contrats §3/4/6 ; risques/contraintes §2/7 ; readiness §8. Aucun SPIKE ni architecture inter-crates requis : extraction interne à comportement constant. Statut proposé, revue indépendante à consigner après examen de cette version Git.
