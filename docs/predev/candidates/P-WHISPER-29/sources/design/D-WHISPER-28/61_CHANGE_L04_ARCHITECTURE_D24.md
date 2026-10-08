# Architecture L04 — D26 DESIGN DRAFT

Owner AUTH-ARCH-L04-CHANGE. Les contrats et inventaire 14 étages sont adoptés pour revue DESIGN. Sources brutes T-WHISPER-L04-D25-SPECIALISTS-RAW. Code exact figé dans 52. Aucun test/qualification revendiqué.

# ARCH-L04-D25-CORRECTION-v1 — DRAFT

Auteur : `/root/d23_arch_delta`, rôle `rust_architect`, lecture seule. Date : 2026-10-07. Owner technique : `AUTH-ARCH-L04-CHANGE`.

D26 adopte ces contrats comme spécification DESIGN soumise à revue indépendante; validations restent NOT RUN. Ce paquet intègre `ARCH-L04-BOUNDS-v1` et la réconciliation domaine `DOM-L04-D25-RECOVERY-v2`, reçue de son auteur. Les invariants métier restent sous ownership domaine ; les propositions ci-dessous couvrent leur réalisation technique.

## 1. Base, provenance et patches proposés

Base vérifiée : **D-WHISPER-24**, digest `80e8bbfef4c0c671e7eba47d73f73ec15bf876477f71d580dea931e80bb27793`, manifeste SHA256 `b12f527b7f3c9fc155b5b46321e541adabb70165fd0d61ea5bcd3cdf86f21357`. État relu SHA256 `a4e3c71608290ef25ce825386ae9df9eb73cdf62571940115a0cb75171942fe4`.

Le TRANSPORT brut D24 a été relu et ses hashes recalculés :

- `T-WHISPER-L04-SPECIALISTS-RAW-D24/transport-manifest.json` : `a26a6bda4c6715e946078767142ec43ad30303271b38dd0f3e9ee93b1b94cbbc`;
- `ARCH-L04-CHANGE-v3.md` : `ed9543afaa0b0913d9fd1e8d387e10f58839d67e9268f3adff83172dad9a3360`;
- `DOM-L04-D24-v3.md` : `a0af62faddf7d7781617bc466bfbd3f30da119bcabdea4a0029602a5b6de1524`.

Patches documentaires proposés, tous sous owner architecture :

| Entrée | Base / version | Mutation proposée |
|---|---|---|
| `61_CHANGE_L04_ARCHITECTURE_D24.md` | SHA `e4ef1f63f80973d4ee65c2066e11dc64d278f7406f19be639ac31d3b2ef8690f`, version suivante D25 | Remplacer les résumés par les contrats des sections 3–7 ci-dessous et référencer l’inventaire |
| Nouvelle section/fichier d’inventaire D25 | D24 et sources section 2 | Persister les quatorze étages et leurs limites |
| `08_STATE_AND_PORT_CONTRACTS.md` | Sections architecture/ports seulement | Désigner explicitement les formulations remplacées ; conserver sections domaine sous leur owner |
| `10_QUESTIONS_RISKS_AND_READINESS.md` | Entrées techniques seulement | Ajouter risques/tâches section 8, sans fermer les findings |
| Q-D24-BOUND-01 | Owner technique architecture | Réponse technique proposée : réutilisation des capacités établies, comptabilité et politique de saturation définies ici |

Le coordinateur possède les modifications de state, versions REQ/UC/AC et dispositions GAP. Aucun registre partagé n’est remplacé en bloc.

## 2. Sources exactes de l’inventaire

`R = docs/predev/L04-PREFLIGHT/candidates/`. Les chemins `…` ci-dessous développent le préfixe de la même ligne précédente.

| ID | Fichier / SHA256 recalculé |
|---|---|
| S01 | `R/P-WHISPER-28/10_L03_INTEGRATION_CONTRACTS.md` — `1079689e79414c2d700eda2a1ac9a2f90632c35dc0b4709f3e379f44abcfcaae` |
| S02 | `R/P-WHISPER-28/sources/ARCH-DETAIL-P02-v1-R2.md` — `85ebaa418dbc9d6168ef7381634d709aae4bf49a53139cf0b443d143c6103879` |
| S03 | `R/P-WHISPER-28/sources/architecture/ARCH-L01-BOUNDS-v1.md` — `fee3f0dbcb5ecdbc9f70082aba311bca3daaa34bf5efaa12d6b90b71a10f2efce` |
| S04 | `R/D-WHISPER-23/sources/L04-change-baseline/crates/whisper-adapters/src/worker_ipc.rs.txt` — `a97e3bc66566eccfe168ed20a8134f3b5428df7656be0dadb7dfd51258d26462` |
| S05 | `R/D-WHISPER-23/sources/L04-change-baseline/crates/whisper-worker-cpu/src/main.rs.txt` — `25d88755bed8bd3afeed539f028c2471301d8391ccdd2567b3ada7c4499009f1` |
| S06 | `R/D-WHISPER-23/sources/L04-change-baseline/crates/whisper-adapters/src/archive.rs.txt` — `043dd959835e350bf96bfecab13b501dd9b142763c05ed6d99f78845b668a8ce` |
| S07 | Code courant `crates/whisper-adapters/src/capture.rs` — `6473322c054b2aead741a1c7e97567f1666e7fb4deedd7865423ead57396677c` |
| S08 | Code courant `crates/whisper-adapters/src/staging.rs` — `cdbb8953032cccdc7e8401b763defcc452bef66e0da0e610b0db123f8a3e5507` |
| S09 | Code courant `crates/whisper-worker-cpu/src/encoder.rs` — `25d59e138604f9d58a4a3f6952eee47d33a10104706b038ed7c19bb4b50680cd` |
| S10 | Code courant `crates/whisper-worker-cpu/src/ipc.rs` — `1b06be095f4586f8e2cddc128fcc9e7307ab208d87323e18f97eddca97f06f97` |
| S11 | D23 snapshot `…/crates/whisper-worker-gpu/src/main.rs.txt` — `e83d7fb7bd6636b8ee32dd821c104042dab8d7d246fd24254ee5012e91d682c4` |
| S12 | D23 snapshot `…/crates/whisper-core/src/application.rs.txt` — `0ce0e32c0b2e92f8a747d065f30a8f89c022ac2ce5e2848ae323bc897c55ea4b` |

S04–S06 correspondent au code courant. S07/S08 ne figurent pas parmi les 21 snapshots D23 ; leurs hashes concordent avec les sorties L03 de `R/P-WHISPER-28/sources/progress/L03-CLOSURE-04/execution-evidence.json`, SHA `247966a803d10445597dd9fc3915e855409a54cb09ff9b4e17997fc0f4c4ec53`. C’est une preuve de version, pas une nouvelle exécution.

## 3. Inventaire des quatorze étages

Les valeurs sont soit **établies par contrat/code**, soit marquées **non établies**. Une capacité de vecteur initiale n’est pas un plafond d’allocation.

| Étage, résidence et owner | Capacité/payload | Admission, refus et saturation ; référence |
|---|---|---|
| **1. WASAPI/CPAL**, buffers OS ; capture adapter | Format effectif `Fs/C`; buffers OS non inventoriés. Code exige `Fs≥16000`, `C>0`, sans maximum global de formats. | Erreur CPAL latch failure. Dimensionner au format réellement choisi ; aucun RSS OS garanti. S07:191–301. |
| **2. Pool callback**, RAM capture | **100 Vec f32 partagés entre deux rings**, pas 200 payloads. `S=ceil(Fs×20/1000)×C`; payload demandé `100×S×4` octets. Le bloc détenu par le consommateur appartient au même pool. | Pool libre vide ⇒ saturation latch ; frame incohérente ⇒ failure. Callback sans IO. S07:12,159–188,209–223,304–324. |
| **3. Converter/rubato**, RAM convertisseur | `K=input_frames_next()`; réservation initiale pending=`2K`, non plafond. Avant conversion longueur mono≤`K−1+S/C`, après boucle `<K`. FFT, capacités sortie/résultat et tail non chiffrés. | Erreur conversion ⇒ cessation. Compter coexistence input/output/result, mesurer dimensions réelles rubato. S07:28–151. |
| **4. Réassemblage/VAD**, RAM runtime | Trame **320 i16=640 octets** ; reliquat après extraction `<320`. Pic avant extraction dépend du converter. | VAD conserve l’audio ; erreur ⇒ cessation. `converted` et `frame_pending` peuvent coexister. S04:1252–1348. |
| **5. Writer PCM**, file RAM et disque ; stockage | **50 commandes queued**. Frame320 i16, payload queued≤**32 000 octets** ; writer actif et trame producteur séparés. Tail<320; Drain sans PCM. | `try_send` plein⇒WouldBlock, fermé⇒BrokenPipe ; entrée cesse. Stop send/drain hors UI, sans délai maximal. S08:12–13,147–157,279–343. |
| **6. Spool durable**, disque ; PcmStaging | **32 000 octets/s** PCM mono16k, header/checkpoint en supplément ; sync nominale toutes **25 trames**, puis drain. **Aucun quota applicatif ni durée maximale.** | Croissance durable ; IO/sync échoué⇒pas de nouvel ACK valide, cessation et Recoverable. Limites filesystem/offset contrôlées ; aucune continuité garantie pendant une durée donnée. S01/S08. |
| **7. Lecture replay**, RAM PcmWriter | Caller≤**80 000 échantillons**, PCM160 000 octets. Bytes+i16 simultanés peuvent représenter **320 000 octets de payload** hors capacité/overhead. | Plage durable uniquement, offsets checked ; erreur⇒Recoverable. API ne vérifie pas elle-même80k : caller responsable. S08:242–275; S04:1353–1428. |
| **8. Commandes parent→worker**, RAM/pipe ; IPC adapter | **1 data queued**, **1 Stop queued**, dispatcher actif et **1 pending_window** distincts. | Plein conserve/réessaie fenêtre ; déconnexion⇒erreur. Replay ne crée aucune nouvelle file accumulatrice. S04:170–187,244–307,1353–1450. |
| **9. Framing**, RAM sérialisation/lecture ; transports | **1 048 576 octets/frame**, newline comprise. 80k i16 :160k binaire ; borne JSON conservatrice560k hors métadonnées. | Reader contrôle accumulation avant désérialisation. Writer actuel sérialise intégralement avant contrôle : **wire admis borné, allocation préalable non bornée par ce check**. S03/S04:353–415/S10. |
| **10. Entrée worker**, RAM enfant | **8 commandes queued**, reader en transfert, une active. Fenêtre live valide≤80k ; f32≤320k octets. | Queue pleine bloque reader. Validation80k après désérialisation/queue. « Une active et une suivante » n’est pas prouvé pour toute la chaîne. S05:125,156–194,578–651; S11:743. |
| **11. Native/codec**, RAM/VRAM enfant | Encodeur reçoit chunks≤16k i16 ; moteur une fenêtre≤80k. Paquets, textes/segments, contexte/modèle et allocations internes non chiffrés. | Erreurs typées ; aucune déduction d’allocation depuis bitrate64kbps. S09/S05:578–717; native_engine snapshot D23. |
| **12. Effets**, RAM application/service | **8 queued**, contrat wire≤8MiB ; un actif séparé, deferred_effect à compter, Stop1 distinct. | Plein refuse ; pas d’effet accepté puis perdu. Sérialisation préalable a le gap de l’étage9. S04:1924–2035. |
| **13. Événements**, RAM parent/application | Contrat **64 globalement**,21×3+1, wire≤64MiB. | **Écart actuel :** guard pending<21 puis effet produisant3 événements peut faire20→23. Clones/transfert/événements locaux à compter. Admission globale des bursts requise avant insertion. S04:605–655,2061–2150. |
| **14. Archives/texte/vues**, RAM et disque ; archive/application/UI | Dernière vue provisoire remplaçable. Journaux, segments cumulés, TXT/SRT, Queue/History croissent avec corpus. | **Pas de plafond RSS global établi.** Recovery/publisher/journal de tentatives nouveaux doivent scanner/écrire progressivement sans copier tout le PCM/groupe. Gap Queue/History hérité reste explicitement distinct. S06:92–101,253–304,337–362; S12; S01 P02-GAP-BYTES-01. |

Q09 borne les diagnostics uniquement ; ses limites ne bornent aucun de ces étages PCM.

### Dimensionnement et politique sous saturation

Charge cible : **un live actif**, format réel `Fs/C`, normalisation mono16k, worker unique, fenêtres≤80k ; importer reste exclu pendant ce live. À48kHz stéréo, exemple contractuel : slot7 680 octets, pool768 000 octets. Ce n’est pas une nouvelle restriction de device.

Pour une durée `t`, charge PCM nominale=`32 000×t+header`. Backlog à transcrire=`A−T`, charge durable correspondante=`2×(A−T)` ; le curseur des fenêtres envoyées ne prouve pas T.

Comptabiliser occupations simultanées, `.len()` et `.capacity()`, transfert, données actives, f32 et buffers OS/native. Aucun chiffre wire n’est présenté comme RSS.

**Contrat requis d’admission :**

- réserver **slots et budget wire** pour tout le burst avant insertion ;
- si20 slots sont occupés et un burst en demande3, ne rien insérer avant capacité suffisante ;
- garder le burst dans une résidence active explicitement comptée ou ne pas le construire encore ; aucune file cachée et aucune troncature ;
- utiliser un compteur global couvrant les événements en transfert, pas seulement chaque `sync_channel`;
- sérialiser dans un writer/builder plafonné à la limite wire existante, qui refuse avant dépassement ; éviter `to_vec` illimité suivi d’un check ;
- Stop conserve son admission indépendante ;
- si capture/writer ne peut conserver l’entrée, fermer celle-ci, préserver confirmations et rendre perte connue/inconnue explicite.

La saturation IPC peut freiner l’inférence tout en laissant le spool progresser. Une attente longue ne ferme pas le micro. Une saturation réelle de capture, déconnexion writer ou erreur disque le ferme. Aucun quota de spool ou délai de panne n’est inventé.

## 4. Identités, couverture et formats

Les contrats D25 ont priorité sur les anciennes formulations « erreur worker⇒cesser capture », « génération worker=passage » et « checkpoint=dernier mot » pour les scénarios corrigés.

Identités :

- durable : groupe/passage ou import, `storage_generation`, configuration/source/axe stables;
- capture : flux/handle attaché au passage, indépendant du worker;
- moteur : `(namespace, attempt_generation, instance_id)`.

Un vieux progrès moteur ne rafraîchit pas la capture; un nouveau compteur capture ne confirme pas une ancienne inférence.

**T** : couverture transcription confirmée, silence inclus. **A** : PCM contigu durable. **C** : capture connue. **E** : curseur d’encodage du pending de la tentative. `0≤T≤A≤C`; E ne prouve jamais T.

Schémas techniques proposés :

| Schéma | Contenu obligatoire / owner |
|---|---|
| `RecoveryCheckpoint v2` | type, identité durable, namespace/tentative, sample rate et group offset, modèle/configuration/source vérifiés, T/A, capture continue ou fin scellée, dernier commit/segment, références d’intégrité ; archive produit, application adopte |
| `CoverageConfirmation v2` | commit monotone, plage complète `[start,end)`, provenance tentative, zéro ou plusieurs segments identifiés, référence audio/source, contrôle d’intégrité ; writer archive |
| `AttemptJournal v1` | séquence, identité durable, namespace, Reserved/Started/Retired, génération/instance connue, checkpoint et mode de reprise, contrôle d’intégrité ; writer archive exclusif |

Physique proposé : records JSON UTF-8 newline, sérialisation et lecture plafonnées par la limite existante1MiB, checksum SHA256 des données canoniques du record. Lecture progressive. Version inconnue, corruption interne ou séquence impossible⇒refus Recoverable; seul suffixe final incomplet peut être ignoré comme non validé.

`WindowFinished` IPC associe fenêtre soumise, plage, identité et résultats, même avec **zéro segment**. Archive valide plage et résultats, écrit/synchronise le payload nécessaire, puis le record de couverture en dernier et le synchronise avant DurableAck. T avance uniquement sur les records valides formant un préfixe contigu. Texte intermédiaire reste provisoire.

Ce protocole ne prétend aucune transaction multi-fichiers. Commit valide avec ACK perdu : scan adopte le commit. ACK sans record valide : aucune nouvelle couverture confirmée.

## 5. Barrière finie et opérations

L’application ferme l’admission des **résultats d’inférence anciens**, puis fixe `M`, dernier ticket des transactions de confirmation **complètes déjà admises**. Ce watermark reste fixe. Les nouveaux samples PCM Auto continuent indépendamment.

Pour chaque ticket≤M : adopter commit durable valide, établir absence définitive de commit, ou rendre l’erreur. Issue inconnue⇒attente. Ne pas attendre tous les futurs writes PCM ni supposer qu’une annulation retire un commit déjà durable.

Drainer/rejeter les vieux messages IPC pour libérer le transport, tout en acceptant exclusivement la confirmation d’arrêt de l’ancien enfant dans le contrôleur de barrière. Ces messages ne modifient pas la tentative suivante.

| Opération / thread | Préconditions | Résultat, erreurs et annulation |
|---|---|---|
| `freeze_inference(M)` / application | panne éligible corrélée | admission ancienne fermée, ensemble fini; répétition idempotente |
| `settle(M)` / archive hors UI | writer exclusif, tickets connus | T autoritatif et snapshot A; corruption/IO⇒Recoverable; Stop reste latched |
| `reserve_attempt` / archive hors UI | C vérifié, ancienne tentative arrêtée/sortie, intentions compatibles | Reserved sync avant spawn; overflow/IO interdit spawn; numéro jamais réutilisé |
| `spawn_resume` / adaptateur | réservation valide, intentions revérifiées | child compatible puis CPU attesté; échec⇒Recoverable sans boucle |
| `rebuild_mp3` / enfant+archive | tentative admissible, même passage, E=0 | `[0,T)` encodage seul, `[T,A)` encodage/inférence; nouvelle capture Auto ensuite; erreur préserve PCM/T |
| `confirm_window` / archive | identité/plage/segments/audio valides | T avance silence compris après durable; stale/refus ne modifie rien |
| `publish` / archive | capture fermée, E couvre audio scellé, couverture T terminée, encodeur fini, fichiers vérifiés | manifeste/pointeur dernier; erreur⇒Recoverable, publication précédente préservée |

La reconstruction crée un pending propre à la tentative; elle ne réappend pas le flux défaillant. Une reconstruction interrompue perd E, **pas T** : prochaine reprise explicitement autorisée reconstruit depuis E=0 avec T courant vérifié.

## 6. Exclusivité et migration

Writer unique sur archive/journaux. Proposition concrète : verrou exclusif non bloquant sur fichier stable par job, acquis hors UI et conservé durant mutations; absence de verrou⇒aucune mutation. Ne pas supprimer/recréer le fichier verrouillé ni transmettre son handle aux enfants.

`std::fs::File::try_lock`, stable depuis1.89, permet cette exclusion entre participants coopérants; fermeture des handles libère le verrou. Documentation consultée le2026-10-07, distincte d’une qualification Windows exécutée. Un ancien writer qui n’utilise pas ce protocole nécessite une preuve de cessation; le verrou neuf ne suffit pas à le déclarer absent. [Source primaire Rust](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).

Migration :

- Complete historique : lire sa version, sans réécriture;
- pending connu : vérifier identité, PCM/source et records;
- couverture contiguë silence compris démontrable : produire checkpointv2 et namespace neuf, frontière tentative0, première réservation1; préserver originaux;
- couverture absente/ambiguë : garder Recoverable et fragments accessibles, **refuser nouveau replay du même passage**, sans inventer T depuis fins de mots;
- version inconnue/corruption : préserver et refuser traitement non supporté.

Une réservation complète sans Started signifie **spawn possible, état inconnu**. Elle consomme son numéro; avant reprise, établir exclusivité et absence d’ancien enfant susceptible d’agir sur le staging. Aucun restart automatique.

Namespace neuf persistant et jamais réutilisé; création exclusive et contrôle de collision. Sa représentation/génération concrète reste détail réversible, sous cet invariant. Ancien protocole/namespace refusé.

## 7. Transitions et supervision

Auto GPU éligible : même capture→barrièreM→ancien enfant arrêté/sorti→checkpoint/réservation→CPU attesté→reconstruction/replay. A/C peuvent augmenter pendant la barrière.

GPU forcé : fermer/drainer→PCM scellé→attente du choix; clic CPU→`FinishPreservedPassage`→finalisation, **aucun micro rouvert**.

Stop/Quit avant spawn empêche nouvelle tentative automatique. Quit en attente GPU forcé n’est pas un choix CPU. Après CPU lancé : fermer capture si nécessaire, drainer/finaliser courant; CPU échoué⇒Recoverable, sans boucle. Import annulé préserve source/confirmation. Masquer ne change rien.

Supervision indépendante : capture, audio durable, inférence, commit, réservation, encodeur/publication, arrêt. Q04 reste applicable; aucun timer ne stoppe, ne bascule ou ne publie. Q09 reste diagnostique.

## 8. Traçabilité, risques et travail restant

| Contrat / risque technique | REQ/UC/AC liés | Owner et preuve attendue |
|---|---|---|
| `RISK-T-D25-BOUND-01` résidence/bursts/sérialisation | REQ07/11/18, UC07/09/13, AC07/09/13 | architecture; inventaire ci-dessus puis occupation, burst20→23 refusé avant insertion, allocation capped |
| `RISK-T-D25-COVERAGE-01` silence/T | REQ10/18, UC08/13, AC08/13 | domaine+architecture; fenêtre silencieuse confirmée, coupure/ACK perdu |
| `RISK-T-D25-BARRIER-01` M fini | REQ07/15/16/18, UC07/11/13, AC07/11/13 | application/archive; PCM continue mais settling finit, ticket inconnu maintient attente |
| `RISK-T-D25-ENCODE-01` E/T confondus | REQ10/18/25, AC07/08/13/20 | archive/worker; crash encodage seul, MP3 sans doublon, T conservé |
| `RISK-T-D25-MIGRATION-01` ancien pending | REQ12/18, UC10/13, AC10/13 | architecture/domaine; legacy migrable/refusé, namespace ancien rejeté |
| `RISK-T-D25-LIFECYCLE-01` lancement après Stop | REQ08/15/16, AC07/11 | application; Stop avant/après réservation et spawn, strict CPU sans micro |

Q-L04-AUDIO-CAPACITY-01 est répondu comme détail technique par AUTH-ARCH-L04-CHANGE; aucun plafond RSS, quota spool ou durée garantie. Le reviewer décide de D24-002/003 sur D25 exact.

Restent réversibles au plan : noms types/fichiers techniques, représentation namespace, instrumentation et datasets de validation. Restent indispensables au design : limites/résidences établies, admission des bursts, identité séparée, watermark, confirmation silence, exclusivité, schémas/migration/refus, priorités Stop et publication.

**DESIGN_FEASIBILITY** : analyse des contrats/primitives existants, pas nouveau succès expérimental. Pas de SPIKE supplémentaire identifié pour ces contrats. Si un plafond RSS, quota spool ou durée de continuité garantie est ajouté, il exige décision/preuve distincte.

**PRODUCT_VALIDATION NOT RUN** : saturation réelle, refus/budgets, watermark, silence, crash/migration, MP3 et vraie UI. **DELIVERY_QUALIFICATION NOT RUN** : package IPC homogène, native CPU/GPU et cible Windows. Les tests futurs ne sont pas remplacés par cet inventaire.

Portée physique conservée : **21 chemins D24**, sans nouveau crate/DLL. Capture/staging sont sources héritées de validation, pas nouveaux chemins autorisés. Tout besoin de les modifier retourne au coordinateur avant code. Railguard actif adapté; aucune activation ou exception proposée.

D26 adopte ces contrats pour revue; seul le reviewer ferme un finding sur manifeste exact.



## Inventaire des quatorze étages intégré au contrat

# ARCH-L04-BOUNDS-v1 — inventaire technique pour D24-002

Auteur : `/root/d23_arch_delta`, rôle `rust_architect`, lecture seule, 2026-10-07. Aucun fichier édité ni banc exécuté. Inventaire fondé sur les snapshots **D23** et les contrats **P28/L03** ; il ne ferme pas lui-même D24-002.

## Sources et fidélité

Racine documentaire `R = docs/predev/L04-PREFLIGHT/candidates/`.

| Référence | Source / SHA256 recalculé |
|---|---|
| B01 | `R/P-WHISPER-28/10_L03_INTEGRATION_CONTRACTS.md` — `1079689e79414c2d700eda2a1ac9a2f90632c35dc0b4709f3e379f44abcfcaae` |
| B02 | `R/P-WHISPER-28/sources/ARCH-DETAIL-P02-v1-R2.md` — `85ebaa418dbc9d6168ef7381634d709aae4bf49a53139cf0b443d143c6103879` |
| B03 | `R/P-WHISPER-28/sources/architecture/ARCH-L01-BOUNDS-v1.md` — `fee3f0dbcb5ecdbc9f70082aba311bca3daaa34bf5efaa12d6b90b71a10f2efce` |
| B04 | D23 snapshot `sources/L04-change-baseline/crates/whisper-adapters/src/worker_ipc.rs.txt` — `a97e3bc66566eccfe168ed20a8134f3b5428df7656be0dadb7dfd51258d26462` |
| B05 | D23 snapshot `…/crates/whisper-worker-cpu/src/main.rs.txt` — `25d88755bed8bd3afeed539f028c2471301d8391ccdd2567b3ada7c4499009f1` |
| B06 | D23 snapshot `…/crates/whisper-adapters/src/archive.rs.txt` — `043dd959835e350bf96bfecab13b501dd9b142763c05ed6d99f78845b668a8ce` |
| B07 | Code courant `crates/whisper-adapters/src/capture.rs` — `6473322c054b2aead741a1c7e97567f1666e7fb4deedd7865423ead57396677c` |
| B08 | Code courant `crates/whisper-adapters/src/staging.rs` — `cdbb8953032cccdc7e8401b763defcc452bef66e0da0e610b0db123f8a3e5507` |
| B09 | Code courant `crates/whisper-worker-cpu/src/encoder.rs` — `25d59e138604f9d58a4a3f6952eee47d33a10104706b038ed7c19bb4b50680cd` |
| B10 | Code courant `crates/whisper-worker-cpu/src/ipc.rs` — `1b06be095f4586f8e2cddc128fcc9e7307ab208d87323e18f97eddca97f06f97` |

B04–B06 sont identiques au code courant relu. B07/B08 n’ont pas de snapshot dans la baseline D23 des 21 chemins ; leurs hashes correspondent aux sorties L03 enregistrées dans `R/P-WHISPER-28/sources/progress/L03-CLOSURE-04/execution-evidence.json`, hash `247966a803d10445597dd9fc3915e855409a54cb09ff9b4e17997fc0f4c4ec53`. Cette égalité est une preuve de version, pas une nouvelle exécution.

## Inventaire par étage

| Étage / owner | Capacité et payload établis | Admission, saturation, limites |
|---|---|---|
| WASAPI / CPAL | Format device effectif `Fs`, canaux `C`; buffers OS non inventoriés. | Erreur CPAL latch `CAPTURE_FAILED`. Aucune borne globale des formats admis au-delà de `Fs≥16000`, `C>0`. B07 `start_device`. |
| Pool callback / capture adapter | **100 vecteurs f32 partagés entre deux rings**, pas 200 payloads. Taille demandée du slot `S=ceil(Fs×20/1000)×C`; payload demandé `100×S×4` octets. | Callback découpe sans IO. Pool libre vide ⇒ saturation latch ; format partiel/incohérent ⇒ failure. Le vecteur détenu par le consommateur appartient à ces mêmes 100 slots. B07:12,159–188,209–223,304–324. |
| Conversion mono/rubato / `Pcm16Converter` | `K=input_frames_next()` ; `pending` initialement réservé à `2K`; buffers sortie alloués par rubato. Bloc entrant ≤`S/C` frames. | `Vec::with_capacity` n’est pas un plafond. Avant traitement, longueur mono ≤`K−1+S/C`; après boucle `<K`. Résultat PCM, capacité Vec, FFT interne et drainage final non chiffrés. Erreur conversion cesse capture. B07:28–151. |
| Réassemblage/VAD / live runtime | Trame **320 i16 =640 octets** ; reliquat après extraction `<320`. | Le pic avant extraction dépend du résultat converter ; `frame_pending` et vecteur `converted` peuvent coexister. VAD ne supprime aucun audio archivé. Erreur ⇒ cessation. B04:1252–1348. |
| Writer PCM / stockage | File **50 commandes** ; Frame=320 i16 ; payload queued ≤**32 000 octets**, plus une commande writer active et la trame détenue par le producteur, comptées séparément. Tail `<320`; Drain sans PCM. | `try_send` plein ⇒ `WouldBlock`; déconnexion ⇒ `BrokenPipe`. Runtime cesse capture. Stop utilise send/drain bloquants hors UI, sans délai maximal promis. B08:12–13,147–157,279–343. |
| Spool durable / `PcmStaging` | PCM16 mono16k : **32 000 octets/s**, plus header/checkpoint. Sync nominale toutes **25 trames**, puis drain. | **Aucun quota applicatif de spool ni durée maximale** dans B08. Croissance sur disque jusqu’à Stop/erreur filesystem/limite d’offset contrôlée. Ce n’est pas une file RAM. IO/sync échoué ⇒ aucun nouvel ACK durable valide. B01/B08. |
| Lecture replay / `PcmWriter` | Appel runtime ≤**80 000 échantillons** : PCM **160 000 octets**. Lecture garde temporairement bytes et i16 simultanément : jusqu’à **320 000 octets de payload**, hors capacités/overhead. | Lecture seulement plage durable, checked offsets; erreur ⇒ Recoverable. API `read_durable_samples` ne vérifie pas elle-même 80k : cette borne vient de son caller. B08:242–275; B04:1353–1428. |
| Commandes parent→enfant / IPC adapter | **1 commande data queued**, **1 Stop queued**, plus dispatcher actif et un `pending_window` runtime. | Plein ⇒ fenêtre conservée et réessayée ; pas de seconde file de replay autorisée. Déconnexion ⇒ cessation/erreur. B04:170–187,244–307,1353–1450. |
| Framing / parent et worker | **1 048 576 octets/frame**, newline incluse. 80k i16 : 160k payload binaire; borne conservatrice JSON valeurs/séparateurs **560k**, métadonnées en supplément. | Reader contrôle accumulation avant désérialisation. Writer sérialise **avant** contrôle taille : frame admise bornée, allocation préalable non démontrée bornée à 1MiB. B03/B04:353–415/B10. |
| Entrée worker CPU/GPU / enfant | **8 commandes queued**, une commande traitée, plus reader bloqué en transfert. Fenêtre live valide ≤80k; conversion moteur f32 ≤**320k octets**. | `send` bloque reader si queue pleine. Validation 80k dans service, après désérialisation/admission en queue. Donc « une fenêtre active + une suivante » de B01 n’est pas démontré sur toute la chaîne. B05:125,156–194,578–651; GPU courant `main.rs:743`. |
| Codec/native / enfant | Encodeur reçoit chunks ≤16k i16 ; fenêtre moteur ≤80k. | Vecteurs de paquets, segments/textes natifs, modèle/contexte et mémoire FFT/codec non bornés par ces chiffres. Un bitrate de64kbps ne constitue pas une borne allocation/packet démontrée. B09/B05/native_engine. |
| Effets application / service | **8 queued**, budget contractuel wire ≤8MiB, un actif séparé, contrôle Stop1. | Effet sérialisé puis refusé si frame trop grande ; `try_send` plein refuse. `deferred_effect` et effet actif doivent être comptés dans l’inventaire, sans confusion avec queued. B04:1924–2035. |
| Événements / application | Contrat **64 globalement**, soit21×3+1, wire≤64MiB. Code : child events21, service pending21 visés, app channel21. | **Limite globale non démontrée pour tous les effets** : guard `pending.len()<21` avant un effet pouvant produire plusieurs événements ; ResumeInterrupted peut faire20→23. Clone en transfert et objets locaux à compter. B04:605–655,2061–2150. |
| Texte/archive/UI / archive+application | Dernière vue provisoire remplaçable. | `LiveArchive.segments`, lecture des journaux, TXT/SRT cumulés et Queue/History peuvent croître avec le corpus. Aucun budget RSS global ni plafond de texte démontré. B06:92–101,253–304,337–362; P02-GAP-BYTES-01 dans B01. |

Q09 borne uniquement les **diagnostics**. Ses 8MiB ne bornent ni PCM, ni spool, ni archive.

## Charge et méthode de dimensionnement

La charge de référence est celle du device effectivement choisi : relever `Fs/C/format`, dériver `S`, interroger `K` et dimensions rubato, mesurer `.len()` **et** `.capacity()` des vecteurs. À **48kHz stéréo**, exemple contractuel B01 : slot7 680 octets, pool768 000 octets. Ce n’est pas une nouvelle garantie pour tous les devices.

Pour la capture normalisée, `N=16 000×T`, stockage PCM=`2N+header`. Pour le backlog inférence, suivre `B=durable_samples−inference_cursor`; pendant un blocage, croissance nominale32kB/s. Pour un replay au checkpoint, charge à traiter=`durable_samples−confirmed_offset`. Ne charger cette différence entière ni dupliquer le spool; lire par fenêtres≤80k.

Dimensionner par **occupations simultanées**, pas par addition de plafonds théoriques incompatibles : callback, converter, frame producer, writer queue/active, read buffers, pending window, dispatcher, enfant queue/reader/active, f32, frames sérialisées et buffers OS. Compter aussi mémoire texte/native, indépendante de PCM.

La campagne future doit exercer writer ralenti, inférence bloquée, UI consommatrice ralentie et disque défaillant, avec occupation/capacités, counters captured/admitted/durable/confirmed, bytes spool, backend/identités, refus et Stop. Aucun seuil de délai, RSS ou réserve disque n’est inventé.

## Avis sur migration et journal

La séparation passage durable/tentative est appropriée. Préférer un journal technique séparé **versionné, checksummé, streamé**, de réservations de tentative, pour préserver les pending/manifests historiques. Réservation sync avant spawn ; compteur jamais recyclé ; un record complet mais sans enfant lancé reste consommé.

Préciser la réconciliation entre journal technique, checkpoint PCM et fragments : aucun record d’intention ne confirme audio/texte ni Complete. Suffixe incomplet ignoré ; divergence d’identité/checkpoint rend Recoverable. Aucun auto-start au scan. Éviter une lecture intégrale du journal ou du groupe pour calculer le high-watermark.

## Analyse suffisante ou SPIKE ?

**L’analyse suffit pour corriger la conception de D24-002**, à condition de consigner ces valeurs, inconnues et écarts, et de prescrire :

- aucune nouvelle file cachée de replay;
- admission par capacité réelle, erreurs explicites et cessation sur saturation;
- comptabilité globale des événements/bursts;
- publication/recovery streamés lorsqu’une accumulation entière ferait échouer la garantie;
- distinction mémoire finie du transport / spool durable croissant / allocations natives inconnues.

Les garanties actuelles ne permettent pas d’affirmer « RSS entièrement borné » ou « une fenêtre active et une suivante sur toute la chaîne ». Retirer ces affirmations ou définir leurs corrections vérifiables.

**Pas de SPIKE préalable requis identifié ici** : aucune nouvelle pile ni garantie de débit continu n’est acceptée. Les mesures du pipeline intégré relèvent de **PRODUCT_VALIDATION NOT RUN**. Un SPIKE devient nécessaire si le successor exige un plafond RSS ou une continuité garantie pendant une durée de panne déterminée malgré les allocations opaques ; ces garanties ne sont pas établies par les sources présentes.

La fermeture de D24-002 appartient au reviewer sur le successor exact, après intégration de cet inventaire et des contrats corrigés.

