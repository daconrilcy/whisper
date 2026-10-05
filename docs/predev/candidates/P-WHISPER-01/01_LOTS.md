# Lots ordonnés — P-WHISPER-01

Tous les lots sont `planned`. Les chemins ci-dessous sont proposés pour attribuer le travail, sous réserve du détail physique attribué avant L00 ; ils ne changent pas les frontières TECH-D18-08. Chaque lot applique le preflight de `02_VERIFICATION_AND_PREFLIGHT.md`, conserve ses sorties et résultats dans un ledger d'exécution distinct, et fait revoir tout candidat corrigé. Aucun essai produit n'est exécuté par ce document.

## L-WHISPER-00 — contrats et bootstrap

**Besoin technique** : TECH-D18-01/04/08 et RG-01/07/09/10. **Dépendance** : aucune ; DESIGN/PLANS exacts, mandat coding, railguard actif et détail de disposition sont des entrées DOCUMENT. **Ownership** : `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/whisper-core/**`, squelettes `crates/whisper-adapters/**`, `crates/whisper-worker-cpu/**`, `crates/whisper-worker-gpu/**`, `crates/whisper-desktop/**`, `crates/whisper-bootstrap/**`.

Définir identités job/génération/segment/plage, transitions pures, JobConfig, ports d'application, vues/façade, protocole IPC versionné et racines de composition. Livrer core compilable sans natif et builds CPU/GPU séparés sans unification de features CUDA. **Acceptation** : domain sans OS/IO/UI ; application sans adaptateurs concrets ; desktop/bootstrap sans dépendance Whisper/CUDA ; UI via façade ; ownership et unsafe vérifiables. Vérifier V-BASE/V-BOUNDARY. Aucune transcription revendiquée.

## L-WHISPER-01 — premier parcours WAV CPU

**Couverture** : REQ-03/06/12/13/24, UC-03/06/10/19, AC-03/06/10/19 ; TECH-D18-01/03/06/08. **Dépendance** : L00 CODE, API/lockfile réellement présents et hashés. **Ownership additionnel** : `crates/whisper-adapters/src/{decoder,archive,worker_ipc}.rs`, `crates/whisper-adapters/tests/import_cpu.rs`, `crates/whisper-core/tests/import_contract.rs`, worker CPU et consumer desktop.

Importer/prévalider un WAV réel avec modèle local hashé, worker CPU effectif attesté, langue choisie, progression visible dans le vrai desktop, TXT/SRT/manifeste/pointeur et scan historique minimal. La source reste à son emplacement ; aucun MP3 d'import. **Acceptation** : source SHA identique ; CPU effectif même sur PC GPU ; langue manuelle prioritaire ; fichiers cohérents et visibles ; erreur de modèle/dossier/corruption visible sans faux Complete. Un mock moteur ne valide pas ce parcours. Vérifier V-IMPORT/V-UI.

## L-WHISPER-02 — journal, FIFO et récupération

**Couverture** : REQ-03/04/12/13/16/17/18, UC-03/04/10/11/12/13, AC-03/04/10/11/12/13 ; TECH-D18-06. **Dépendance** : L01 EXECUTION avec réussite du parcours réel. **Ownership additionnel** : `crates/whisper-adapters/src/{journal,queue_store,recovery}.rs`, tests `durability.rs` et `scheduler_contract.rs`, ports, scheduler, archive, UI.

Confirmer audio+texte+record seulement après sync ; file FIFO versionnée et ACK durable ; préserver identité de source ; restaurer `AwaitingChoice` ou `Interrupted` sans exécuter ; Traiter/Retirer et confirmation avant reprise ; scan pending/générations idempotent. **Acceptation** : coupures avant/après sync et publication, préfixe confirmé conservé, double scan stable, aucun partiel Complete, ordre multi-import, source jamais supprimée et Start refusé pendant import. La preuve d'arrêt de processus ne promet pas toute panne électrique. Vérifier V-DURABLE/V-SCHEDULER.

## L-WHISPER-03 — live, VAD, MP3 et passages

**Couverture** : REQ-01/02/03/04/09/10/12/18/25, UC-01/02/03/04/08/10/13/20, AC-01/02/03/04/08/10/13/20 ; TECH-D18-04/05/06/07 et DEC-26/27/34/35. **Dépendance** : L02 EXECUTION ; détail capture/capacités et Q-07 avant preflight. **Ownership additionnel** : `crates/whisper-adapters/src/{capture,vad}.rs`, `crates/whisper-worker-cpu/src/encoder.rs`, tests live, journal/archives, desktop.

Pending durable avant capture ; callback sans IO bloquante ; files bornées ; compteurs parole/total distincts ; texte provisoire/confirmé et horodatages capture/disponibilité/rendu ; Stop draine puis finalise ; AutoStop uniquement activé ; nouveau passage sur impulsion après Stop + Reprise, avec pause et IDs persistés ; un MP3 immuable par passage et TXT/SRT cumulés sur horloge PCM originale. **Acceptation** : un seul handle micro, import pendant live différé jusqu'à finalisation stable, silence gardé dans MP3, aucun départ automatique, Recoverable non Complete, retard CPU visible sans plafond temps réel, VAD mesuré contre DEC-35 sur les seuls 26 WAV annotés. Le micro réel représentatif reste une qualification future. Vérifier V-LIVE/V-VAD/V-DURABLE/V-UI.

## L-WHISPER-04 — calcul et supervision

**Couverture** : REQ-07/08/11/14/15/16/18/24, UC-07/09/11/13/19, AC-07/09/11/13/19 ; TECH-D18-01/03/04. **Dépendance** : L03 EXECUTION ; Q-04/Q-06/Q-09 avant preflight. **Ownership additionnel** : `crates/whisper-adapters/src/supervisor.rs`, `crates/whisper-worker-{cpu,gpu}/src/native_engine.rs`, tests `compute_policy.rs`/`worker_control.rs`, IPC, ports, desktop.

Attester backend après contexte/state et avant inférence ; strict GPU en échec s'arrête sans CPU ; Auto invalide génération et reprend CPU au dernier offset durable ; rejeter ancien résultat avant journal ; contrôler indépendamment de l'IPC audio ; instrumenter saturation, cesser entrée et signaler plage non confirmée ; distinguer silence, inférence lente et panne. **Acceptation** : injections GPU absent/worker avant et après inférence, aucun CPU ni publication en strict, Auto sans doublon, CPU forcé sans CUDA, vieux résultat réellement livré puis refusé, Stop accessible pendant saturation ; ni timer seul ni kill automatique ne décide l'arrêt. Vérifier V-MODES/V-CONTROL/V-UI ; panne physique pilote encore à qualifier.

## L-WHISPER-05 — réglages, OS et archives

**Couverture** : REQ-06/14/15/16/19/20/21/22/23, UC-06/11/14..18, AC-06/11/14/15/16/17/18. **Dépendance** : L04 EXECUTION ; Q-03/Q-05/Q-06/Q-09 avant preflight. **Ownership additionnel** : `crates/whisper-adapters/src/{settings,os}.rs`, tests settings/Windows, historique, desktop et ports.

Persister paramètres versionnés et snapshot par job ; dossier effectif validé sans publication ailleurs ; scan historique depuis fichiers, ouverture et suppression des seules archives après aperçu/confirmation ; tray, fenêtre masquée sans arrêt, instance unique, autostart sans micro ni fenêtre, raccourci configurable et conflit visible. **Acceptation** : valeurs invalides gardent l'ancien état, job actif inchangé, source importée intacte, dossier indisponible signalé, un seul processus inactif au login, hotkey sans déclenchement caché, vrai consumer montre états Recoverable. Vérifier V-OS/V-HISTORY/V-UI.

## L-WHISPER-06 — installation et package

**Couverture** : REQ-05/07/08/24, UC-05/07/19, AC-05/07/19 ; TECH-D18-01/02/03 et CHANGE-001. **Dépendance** : L05 EXECUTION. **Ownership additionnel** : `crates/whisper-bootstrap/src/{acquisition,prerequisites}.rs`, test `install_contract.rs`, inventaire/notices sous `docs/release/`, manifests ; aucune acquisition dans worker.

Construire payload CPU/GPU isolés, imports PE et DLL CUDA privées ; vérifier VC Redist ; acquérir via WinHTTP avec TLS/redirections/200/206/416 et reprise contrôlée ; modèle fixé au commit `6034871ec87c84e342efab769d4c5c06cd126db3`, taille 1 624 555 275, SHA256 `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` ; staging/hash/prérequis avant Ready. **Acceptation** : interruption, hash faux, troncature et runtime absent/refusé ne font pas Ready ; 200 n'est pas concaténé ; 206/Range et identité vérifiés ; worker lit modèle local sans téléchargement. PC propre reste différé par CHANGE-001. Vérifier V-INSTALL/V-PE/V-OFFLINE. Aucune publication autorisée.

## L-WHISPER-07 — qualification intégrée

**Couverture** : REQ-01..25, UC/AC-01..20. **Dépendance** : L06 EXECUTION ; mandat sur corpus micro représentatif avant ces mesures. **Ownership** : rapports attribués sous `docs/qualification/`, sans réécrire DESIGN/PLANS. Tester le payload et les vrais consumers : live/import, interruptions/reprise, modes/langues, réglages/OS, disque/micro/worker, FR/EN calme/bruit et sessions longues selon mandat. Mesurer capture→rendu, retard, perte/buffers et VAD ; distinguer validation produit et qualification livraison. **Acceptation** : chaque AC a résultat/preuve/limite pour le candidat exact ; bancs D18/D19 ne deviennent jamais tests produit ; PC propre et panne électrique restent explicitement différés/non prouvés. Toutes les campagnes V restent NOT RUN jusqu'à exécution autorisée.
