# Lots ordonnés — P-WHISPER-04

L00 est completed selon l’état courant et son ledger ; L01–L07 restent planned. Chaque lot conserve ses sorties dans un ledger distinct. Aucun essai produit n’est exécuté par le plan.

## L-WHISPER-00 — socle livré

Besoin technique : TECH-D18-01/04/08 et RG-01/07/09/10. L00 est completed selon `execution-ledgers/L-WHISPER-00-2026-10-05.md` et ses sorties CODE. Il a livré contrats/squelettes et builds isolés, mais ni `root.rs`, ni application d’import concrète, ni écrivain durable, ni parcours de transcription réel. Ne pas attribuer rétrospectivement ces tâches à L00. Les campagnes produit restent NOT RUN.

## L-WHISPER-01 — premier parcours WAV CPU intégré

Couverture : REQ-03/06/12/13/24, UC-03/06/10/19, AC-03/06/10/19 ; TECH-D18-01/03/04/06/08. Prérequis CODE : L00 et contrats/lockfile présents et hashés. Prérequis DOCUMENT : D19/P06/revues exacts, DETAIL-P01 v3, Q-05, Q-L01-BOUNDS-01, INTEGRATION v2, BOUNDS v1, COMPAT v2, mandat révisé et railguard actif.

**Allowlist exacte : 21 chemins, sans glob** (copiée de INTEGRATION v2) :

```text
Cargo.lock
crates/whisper-adapters/Cargo.toml
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/decoder.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/tests/import_cpu.rs
crates/whisper-core/src/application.rs
crates/whisper-core/src/ipc.rs
crates/whisper-core/src/lib.rs
crates/whisper-core/src/ports.rs
crates/whisper-core/tests/import_contract.rs
crates/whisper-desktop/Cargo.toml
crates/whisper-desktop/src/main.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-worker-cpu/Cargo.toml
crates/whisper-worker-cpu/src/decoder.rs
crates/whisper-worker-cpu/src/ipc.rs
crates/whisper-worker-cpu/src/main.rs
crates/whisper-worker-cpu/src/native_engine.rs
```

`desktop/src/lib.rs` est explicitement hors périmètre ; réutiliser son API `run(application)` et contrôler ses imports. `ui.rs` est inclus et peut changer pour la présentation, tout en restant pur.

**Ownership.** Core/application et ports possèdent états, transitions, snapshots, orchestration/ordre des effets et admission des résultats. Archive porte fichiers/hash/publication/scan ; decoder reste proxy ; worker_ipc porte IO/process/framing. Le worker CPU porte codec/contexte/buffers natifs. `main.rs` et `root.rs` composent seulement. L’UI consomme uniquement la façade. L’application n’a pas d’adaptateur concret ; root/UI/worker ne portent pas de second scheduler ou d’état métier parallèle.

**Travail L01.** Construire ImportApplication et ImportIoPort ; préparer source/modèle/destination en lecture seule et pending durable ; figer dossier au démarrage selon Q-05 ; exécuter worker CPU réel via IPC v2 ; persister segments avec reçu après sync ; publier TXT/SRT/manifeste puis pointeur ; afficher progression/erreurs/historique dans le vrai consumer. Appliquer tous les contrats de `06_L01_INTEGRATION_CONTRACTS.md`.

**Acceptation.** WAV transcrit réellement avec modèle local approuvé ; CPU attesté même sur PC GPU ; langue manuelle prioritaire ; source SHA inchangée ; sorties cohérentes et visibles. Complete seulement après End et reçu de publication vérifié. Modèle/source/dossier/protocole/worker/saturation en erreur restent visibles ; annulation ou résultat périmé ne publient pas. Double scan stable, sans départ ni nettoyage. V-IMPORT/V-UI/V-L01-CONTRACT/V-L01-BOUNDS : PRODUCT_VALIDATION / NOT RUN.

**Hors L01.** Import MP3 complet, journal/FIFO/recovery exhaustifs, live/VAD, GPU opérationnel/fallback, OS complet, installateur et qualification intégrée appartiennent aux lots suivants. L01 est WAV seul ; aucune qualification ne découle du mock ou des bancs D19.

## L-WHISPER-02 — journal, FIFO et récupération

**Couverture** : REQ-03/04/12/13/16/17/18, UC-03/04/10/11/12/13, AC-03/04/10/11/12/13 ; TECH-D18-06. **Dépendance** : L01 EXECUTION avec réussite du parcours réel. **Ownership additionnel** : `crates/whisper-adapters/src/{journal,queue_store,recovery}.rs`, tests `durability.rs` et `scheduler_contract.rs`, ports, scheduler, archive, `crates/whisper-desktop/src/root.rs` (câblage du journal, de la file et de la récupération), `ui.rs` (choix de reprise). `main.rs` reste hors du flux métier.

Confirmer audio+texte+record seulement après sync ; file FIFO versionnée et ACK durable ; préserver identité de source ; restaurer `AwaitingChoice` ou `Interrupted` sans exécuter ; Traiter/Retirer et confirmation avant reprise ; scan pending/générations idempotent. **Acceptation** : coupures avant/après sync et publication, préfixe confirmé conservé, double scan stable, aucun partiel Complete, ordre multi-import, source jamais supprimée et Start refusé pendant import. La preuve d'arrêt de processus ne promet pas toute panne électrique. Vérifier V-DURABLE/V-SCHEDULER.

## L-WHISPER-03 — live, VAD, MP3 et passages

**Couverture** : REQ-01/02/03/04/09/10/12/18/25, UC-01/02/03/04/08/10/13/20, AC-01/02/03/04/08/10/13/20 ; TECH-D18-04/05/06/07 et DEC-26/27/34/35. **Dépendance** : L02 EXECUTION ; détail capture/capacités et Q-07 avant preflight. **Ownership additionnel** : `crates/whisper-adapters/src/{capture,vad}.rs`, `crates/whisper-worker-cpu/src/encoder.rs`, tests live, journal/archives, `crates/whisper-desktop/src/root.rs` (capture, supervision et façade), `ui.rs` (commandes/états visibles). `main.rs` ne change que si le contrat de lancement évolue et après CHANGE.

Pending durable avant capture ; callback sans IO bloquante ; files bornées ; compteurs parole/total distincts ; texte provisoire/confirmé et horodatages capture/disponibilité/rendu ; Stop draine puis finalise ; AutoStop uniquement activé ; nouveau passage sur impulsion après Stop + Reprise, avec pause et IDs persistés ; un MP3 immuable par passage et TXT/SRT cumulés sur horloge PCM originale. **Acceptation** : un seul handle micro, import pendant live différé jusqu'à finalisation stable, silence gardé dans MP3, aucun départ automatique, Recoverable non Complete, retard CPU visible sans plafond temps réel, VAD mesuré contre DEC-35 sur les seuls 26 WAV annotés. Le micro réel représentatif reste une qualification future. Vérifier V-LIVE/V-VAD/V-DURABLE/V-UI.

## L-WHISPER-04 — calcul et supervision

**Couverture** : REQ-07/08/11/14/15/16/18/24, UC-07/09/11/13/19, AC-07/09/11/13/19 ; TECH-D18-01/03/04. **Dépendance** : L03 EXECUTION ; Q-04/Q-06/Q-09 avant preflight. **Ownership additionnel** : `crates/whisper-adapters/src/supervisor.rs`, `crates/whisper-worker-{cpu,gpu}/src/native_engine.rs`, tests `compute_policy.rs`/`worker_control.rs`, IPC, ports, `crates/whisper-desktop/src/root.rs` (sélection GPU/CPU et contrôle des générations), `ui.rs` (backend effectif et erreurs visibles). `main.rs` ne décide jamais du mode moteur.

Attester backend après contexte/state et avant inférence ; strict GPU en échec s'arrête sans CPU ; Auto invalide génération et reprend CPU au dernier offset durable ; rejeter ancien résultat avant journal ; contrôler indépendamment de l'IPC audio ; instrumenter saturation, cesser entrée et signaler plage non confirmée ; distinguer silence, inférence lente et panne. **Acceptation** : injections GPU absent/worker avant et après inférence, aucun CPU ni publication en strict, Auto sans doublon, CPU forcé sans CUDA, vieux résultat réellement livré puis refusé, Stop accessible pendant saturation ; ni timer seul ni kill automatique ne décide l'arrêt. Vérifier V-MODES/V-CONTROL/V-UI ; panne physique pilote encore à qualifier.

## L-WHISPER-05 — réglages, OS et archives

**Couverture** : REQ-06/14/15/16/19/20/21/22/23, UC-06/11/14..18, AC-06/11/14/15/16/17/18. **Dépendance** : L04 EXECUTION ; Q-03/Q-05/Q-06/Q-09 avant preflight. **Ownership additionnel** : `crates/whisper-adapters/src/{settings,os}.rs`, tests settings/Windows, historique, crates/whisper-desktop/src/root.rs (assemblage OS/settings/history et cycle de vie), ui.rs (consumers réels). main.rs reste un point d’entrée.

Persister paramètres versionnés et snapshot par job ; dossier effectif validé sans publication ailleurs ; scan historique depuis fichiers, ouverture et suppression des seules archives après aperçu/confirmation ; tray, fenêtre masquée sans arrêt, instance unique, autostart sans micro ni fenêtre, raccourci configurable et conflit visible. **Acceptation** : valeurs invalides gardent l'ancien état, job actif inchangé, source importée intacte, dossier indisponible signalé, un seul processus inactif au login, hotkey sans déclenchement caché, vrai consumer montre états Recoverable. Vérifier V-OS/V-HISTORY/V-UI.

## L-WHISPER-06 — installation et package

**Couverture** : REQ-05/07/08/24, UC-05/07/19, AC-05/07/19 ; TECH-D18-01/02/03 et CHANGE-001. **Dépendance** : L05 EXECUTION. **Ownership additionnel** : `crates/whisper-bootstrap/src/{acquisition,prerequisites}.rs`, test `install_contract.rs`, inventaire/notices sous `docs/release/`, manifests ; aucune acquisition dans worker ni dans `crates/whisper-desktop/src/root.rs` ; le binaire bootstrap et ses adapters gardent l’acquisition.

Construire payload CPU/GPU isolés, imports PE et DLL CUDA privées ; vérifier VC Redist ; acquérir via WinHTTP avec TLS/redirections/200/206/416 et reprise contrôlée ; modèle fixé au commit `6034871ec87c84e342efab769d4c5c06cd126db3`, taille 1 624 555 275, SHA256 `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` ; staging/hash/prérequis avant Ready. **Acceptation** : interruption, hash faux, troncature et runtime absent/refusé ne font pas Ready ; 200 n'est pas concaténé ; 206/Range et identité vérifiés ; worker lit modèle local sans téléchargement. PC propre reste différé par CHANGE-001. Vérifier V-INSTALL/V-PE/V-OFFLINE. Aucune publication autorisée.

## L-WHISPER-07 — qualification intégrée

**Couverture** : REQ-01..25, UC/AC-01..20. **Dépendance** : L06 EXECUTION ; mandat sur corpus micro représentatif avant ces mesures. **Ownership** : rapports attribués sous `docs/qualification/`, sans réécrire DESIGN/PLANS. Tester le payload et les vrais consumers : live/import, interruptions/reprise, modes/langues, réglages/OS, disque/micro/worker, FR/EN calme/bruit et sessions longues selon mandat. Mesurer capture→rendu, retard, perte/buffers et VAD ; distinguer validation produit et qualification livraison. **Acceptation** : chaque AC a résultat/preuve/limite pour le candidat exact ; bancs D18/D19 ne deviennent jamais tests produit ; PC propre et panne électrique restent explicitement différés/non prouvés. Toutes les campagnes V restent NOT RUN jusqu'à exécution autorisée.

## Correction contraignante de L-WHISPER-02 : import MP3

Cette section complète L02 ; elle prévaut sur toute lecture de L01 qui limiterait définitivement REQ-03 au WAV. **Réalisation**, owner L02 : compléter `crates/whisper-adapters/src/decoder.rs`, le `DecoderPort`, les manifests/features du décodeur, et créer `crates/whisper-adapters/tests/import_mp3.rs`. Le choix concret de décodeur et profils/bitrate est résolu par DETAIL-P01 et Q-07 avant ce lot. Exécuter le décodage bloquant hors UI, avec buffers bornés et source en lecture seule ; convertir selon le contrat PCM16 mono 16 kHz sans perdre l'axe temporel source pour SRT. Réutiliser le vrai worker CPU, le publisher TXT/SRT et le scan du premier parcours WAV. Aucun MP3 importé n'est recopié ou archivé.

L02 couvre aussi REQ-06/24 et vérifie AC-06/19 pour cette variante, en plus de ses REQ/AC existants. **Acceptation** : un MP3 de profil accepté est transcrit réellement avec langue manuelle prioritaire, CPU effectif attesté, offsets sur l'axe original, TXT/SRT visibles et hash source identique avant/après. MP3 invalide/tronqué/profil refusé, source modifiée après enqueue, annulation et crash donnent une erreur ou reprise explicite, jamais faux Complete ni suppression source ; aucun import ne tourne durant live. Vérifier V-IMPORT-MP3, V-DURABLE, V-SCHEDULER et V-UI. Le test de prévalidation seul et un moteur mocké ne ferment pas AC-03.

La campagne L07 reprend explicitement le MP3 implémenté en L02 ; elle ne remplace pas sa réalisation. Chaque lot suit les `requires_details` du tableau P2 dans `00_PLANS_INDEX.md` et le registre `04_OPEN_DETAILS.md`.
