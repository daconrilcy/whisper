# Paquet brut ARCH-L01-INTEGRATION v2 — proposition documentaire

**Acteur :** `/root/whisper_l01_integration_coordination/l01_architecture_contract`. **Rôle :** rust_architect, lecture seule. **Date :** 2026-10-05, Europe/Paris. **Owner technique :** architecte ; owner de l’état et du transport : AUTH-COORD.

**ID :** CHANGE-L01-INTEGRATION-01, version 2, **PROPOSED**. Cette version remplace la proposition conversationnelle v1 ; elle choisit `root.rs`, borne les fichiers worker et corrige l’affirmation « aucune version nouvelle » : SHA2 0.11.0 est déjà choisi dans D19, mais son entrée n’existe pas dans le lockfile L00 actuel.

**Bases personnellement contrôlées par `verify-manifest`, Python `-B`, sans écriture :**

| Corpus | Digest | Résultat |
|---|---|---|
| D-WHISPER-19 | `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529` | PASS |
| P-WHISPER-02 | `8113649a38976e0c14e1c82a10ed1a04dff7c1022aaff5587962a0d94b8b2bad` | PASS |

HEAD lu : `e223166a08592048508865b480ecc53b101bdd0d`. `git status --short` : uniquement `?? target/`. Aucun code, test, ledger ou résultat V-IMPORT/V-UI produit par cet acteur. Aucun finding fermé.

**Persistance :** ce paquet est une source conversationnelle, non persistée. Il exige TRANSPORT et référence vérifiée au checkpoint avant handoff. Aucune égalité des octets export/proposition/TRANSPORT n’est revendiquée.

**Runtime :** permissions effectivement unrestricted ; lecture seule respectée par instruction. Isolation native et topologie HOST_PERSISTED **non qualifiées** par cette session.

## 1. Constat et classification

`desktop/src/main.rs` injecte actuellement `DesktopComposition`, qui accepte seulement `Refresh`. `core/src/application.rs` fournit les contrats/façade mais aucune application concrète. `adapters/src/lib.rs` exporte seulement `decoder`. Le manifeste desktop ne déclare pas adapters.

Modifier seulement `ui.rs` ne connecte donc pas le vrai binaire aux effets. Composer les adapters dans UI ou faire porter le scheduler par UI/worker enfreindrait RG-01, TECH-D18-04/08 et DETAIL-P01 v3.

**Architecture démontrable dans l’allowlist actuelle : NON ÉTABLIE.**

La correction proposée complète la disposition déjà acceptée dans DETAIL-P01 v3 : application pure core, bibliothèque desktop UI pure, composition privée au binaire desktop. Nature proposée : **détail physique et correction PLANS/mandat**, sans changement de besoin, frontières, garanties de durabilité ou politique d’arrêt D19. Un reviewer doit confirmer cette classification. Une modification de ces garanties exigerait DESIGN_CHANGE_REQUIRED.

Options :

| Option | Statut et justification |
|---|---|
| UI assemble et appelle adapters | REJECTED : RG-01 |
| Worker ou root possède scheduler et règles métier | REJECTED : TECH-D18-04/08 |
| Nouveau crate application ou nouvelle racine executable | NON RETENUE : les cibles existantes suffisent |
| Core implémente application pure ; root desktop injecte adapters | PROPOSED : conforme à DETAIL-P01 v3 et aux autorités lues |

## 2. Assemblage et matrice physique proposés

Le vrai consumer reste `whisper_desktop::run(application)` puis `ui::DesktopApp<A>`.

`desktop/src/main.rs` déclare `mod root;`, demande à `root` de construire les implémentations puis appelle `run`. `root.rs` construit les canaux bornés, le service IO, son worker enfant et `core::ImportApplication`. Il ne conserve pas un deuxième état métier.

| Surface | Responsabilité et dépendances autorisées | Contrôle futur |
|---|---|---|
| `core/domain.rs` | Identités/transitions pures existantes | Imports sans application/IO/UI/native |
| `core/application.rs`, `ports.rs` | ImportApplication, snapshots, commandes/vues, ordre des effets et rejet obsolète ; domain/types purs | Compilation core seule ; revue imports et transitions |
| `adapters/archive.rs` | Fichiers, hashes, snapshots/manifeste/pointeur et scan | Contrats archive, coupures et double scan |
| `adapters/decoder.rs` | Proxy existant, identité/plage ; aucun codec natif parent | Imports et tests decoder |
| `adapters/worker_ipc.rs` | Admission IO, opérations hors UI, enfant, transport et supervision | Capacités/contrôle/EOF/erreurs ; aucun import UI |
| Worker CPU `main.rs`, `ipc.rs`, `decoder.rs`, `native_engine.rs` | Composition enfant, protocole, source/codec, contexte CPU exclusif | CPU build distinct ; attestation ; aucun CUDA |
| Desktop `lib.rs`, `ui.rs` | Framework et API application pure ; présentation et traduction des interactions | Pas d’import adapters/fs/process/native ; vrai consumer |
| Desktop `main.rs`, `root.rs` | Assemblage seulement | Revue wiring et absence d’état métier parallèle |

Cargo accorde les dépendances du package à ses différentes cibles ; l’ajout d’adapters au manifeste desktop ne prouve donc pas l’absence d’UI→adapters. Le contrôle doit inclure les imports/consumers de `lib.rs` et `ui.rs`. [Cargo Targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html), consulté le 2026-10-05.

## 3. Contrat application/effets proposé

Une famille de port réellement nécessaire suffit : `ImportIoPort`, **possédée par application**, avec admission non bloquante d’effets typés et polling non bloquant d’événements. Les noms Rust restent proposés ; les obligations suivantes sont le contrat.

**État métier :** ImportApplication possède job/génération, snapshot de langue/compute/destination, transitions, demande d’arrêt et décision de publication. UI possède champs/rendu. Adapter possède handles, processus et files techniques. Worker possède contexte natif et buffers.

**Commande de démarrage :** identité job/génération, référence source, langue/compute et destination sélectionnée. Le modèle local approuvé est une configuration pure injectée avec chemin/hash/taille. Les références de chemins sont des données ; seuls les adapters/worker les ouvrent.

**Snapshot :** le dossier est figé quand le job est accepté au démarrage. Modifier le choix UI affecte le suivant, sans déplacement ni suppression, conformément à Q-05. Aucun modèle n’est téléchargé dans ce parcours.

| Effet / sortie | Préconditions et erreurs | Effets, annulation, durabilité |
|---|---|---|
| Préparer source/modèle/destination → événement Prepared | Pas de job actif ; référence résoluble ; source et modèle ouverts en lecture seule ; modèle hash/taille approuvés ; erreurs absent, modifié, format, hash, accès, espace | Hors UI ; produire identité source et pending durable avant démarrage ; aucun accusé durable fondé sur RAM |
| Démarrer worker → Ready/Progress/Segment/End | Prepared valide, CPU demandé pour ce lot, protocole accepté, source revalidée | Enfant CPU sans contexte GPU ; Ready après initialisation/attestation ; chaque événement corrélé au job/génération/instance |
| Persister fragment → reçu de persistance | Identité courante ; plage/séquence valide ; segment non dupliqué | Append/version du staging, sync avant reçu ; source jamais modifiée |
| Publier archive → reçu vérifié | End réussi, tous fragments attendus reçus/persistés ; pas de cancel | TXT/SRT/manifeste vérifiés ; pointeur publié dernier ; Complete seulement après reçu cohérent |
| Scanner → entrées Complete/Recoverable | Racine autorisée, lecture des snapshots/manifeste/pending | Aucun démarrage restauré ; aucun nettoyage automatique ; double scan stable |
| Arrêter → Stopped/diagnostic | Job/génération correspondant | Admission indépendante du flux segments ; génération invalidée avant acceptation d’un résultat tardif ; conserver source/pending |

Une admission acceptée signifie uniquement « opération admise », jamais « archive durable » ou « job terminé ». Une saturation refuse la nouvelle opération avec erreur visible, conserve le dernier état prouvé et arrête l’entrée concernée. Aucune commande critique ni fragment confirmé n’est silencieusement supprimé.

L’application ordonne ces effets et valide leurs reçus. L’adapter exécute leur mécanisme technique ; il ne décide ni second démarrage, ni fallback, ni Complete.

Erreurs publiques à distinguer : Busy, InvalidInput, SourceMissing/Changed, ModelMissing/HashMismatch, UnsupportedLanguage/Format, StorageUnavailable/Full/Corrupt, ProtocolMismatch, WorkerExited, BackendMismatch, Saturated, Cancelled et StaleResponse. Leur codage exact reste DETAIL, mais une chaîne générique ne doit pas effacer la distinction nécessaire aux transitions/récupération.

## 4. Concurrence, limites et arrêt

**Décisions techniques proposées à revoir :** un service IO parent dédié, un enfant CPU actif maximum, canaux `std::sync::mpsc::sync_channel` ; aucun nouveau runtime async ni crate de concurrence. API UI : `try_send`/`try_recv`, jamais `recv`, `join`, hash, lecture disque ou inférence. [Canal borné std](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html), consulté le 2026-10-05.

Bornes proposées : 8 effets en attente, 64 événements persistables en attente, trame IPC plafonnée à 1 MiB, un bloc PCM de décodage limité par `max_samples` validé avant allocation. Une trame trop grande est rejetée explicitement ; le texte n’est pas tronqué. Les segments à persister subissent la rétropression. Le rendu provisoire peut être remplacé par sa dernière valeur ; aucun fragment durable ne l’est.

Le canal de contrôle ne partage pas la capacité data : état d’arrêt dédié par identité/instance, consultable indépendamment par le service IO et le lecteur contrôle enfant. Les opérations disque et inférence restent bloquantes hors UI. Les fenêtres PCM/inférence et leur mapping SRT doivent être bornés sans imposer une limite globale de durée du fichier.

**Arrêt borné réel :** l’admission Stop/Quit et le polling UI sont non bloquants et effectuent un nombre fini d’opérations par tick. Aucun délai maximum de terminaison FFI/disque n’est promis. Une attente ou panne est affichée ; Quitter reste ouvert jusqu’à résolution ou action manuelle selon D19. Aucun kill automatique ni `Drop` faisant un join infini sur UI.

Un arrêt coopératif reconnu donne Stopped, puis libération des handles. EOF/erreur de protocole/exit inattendu est supervisé et mène à Recoverable/diagnostic. Le vieux processus, sa génération ou un événement après annulation ne peuvent publier d’archive.

**Q-L01-BOUNDS-01**, reversible_detail, owner architecte/AUTH-COORD, échéance avant code L01 : valider ces capacités et préciser la taille des fenêtres PCM à partir des WAV autorisés et des contrats d’offsets existants. Les nombres proposés sont des limites de ressources internes, sans promesse de latence ni réduction du périmètre produit.

## 5. IPC et compatibilité proposés

Le V1 L00 est un socle non connecté. Il manque notamment une fin de transcription explicite et une corrélation complète de l’initialisation/erreur.

Proposer **IPC_PROTOCOL_VERSION = 2**, avec :

- préparation/démarrage corrélés à job/génération/instance et source hashée ;
- backend attesté ;
- segments identifiés et plages sur axe source ;
- événement End portant dernier offset/séquence attendue ;
- erreurs et Stopped corrélés ;
- contrôle indépendant du flux data.

Les messages V1 sont refusés avec ProtocolMismatch. Aucune interprétation ambiguë ni compatibilité silencieuse. Parent et worker CPU sont livrés/revus ensemble. Les données durables n’utilisent pas ces DTO IPC comme schéma de stockage.

Le worker GPU L00 n’est pas un consumer opérationnel ; L04 devra respecter V2 lors de son implémentation. Cela ne qualifie pas le futur GPU. Le reviewer doit vérifier que cette complétion de protocole versionné reste un détail de l’architecture déjà acceptée.

## 6. Publication, recovery et migration

Import : TXT/SRT plus référence hashée à la source intacte, sans copie audio ni MP3. Écrire pending et génération immuable ; fermer/synchroniser chaque artefact ; vérifier tailles/hashes/identités ; produire manifeste puis pointeur courant dernier.

La publication concerne **cette génération d’archive sous la racine du job**. Elle n’est pas une transaction entre plusieurs fichiers ni une garantie contre toute panne électrique. [File::sync_all](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all), consulté le 2026-10-05.

Après coupure : pending sans pointeur complet, pointeur invalide, artefact absent ou hash différent → Recoverable. Un ancien snapshot validé reste lisible ; la tentative courante ne devient pas Complete. Réconciliation idempotente, sans suppression automatique ni reprise de traitement.

Migration de **données produit préexistantes : N/A pour ce delta**, car L00 n’a livré ni écrivain d’archives/file ni paramètres durables. Schémas archive/pending versionnés à leur première écriture ; version inconnue refusée sans réécriture. Le changement IPC V1→V2 est une rupture des contrats de code, à couvrir dans les tests et le ledger futur, même en l’absence de migration de fichiers.

L02 garde le journal/FIFO/reprise complète et injections systématiques de coupures ; le scan minimal L01 ne ferme pas tous AC-10/12/13.

## 7. Chemins exacts à faire autoriser

**Allowlist complète proposée pour L01 révisé ; tous les ajouts hors mandat actuel restent non autorisés :**

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

`desktop/lib.rs`, `core/domain.rs`, manifeste racine, bootstrap, worker GPU et railguard actif ne nécessitent pas de delta.

Dépendances proposées : desktop ajoute `whisper-adapters.workspace`; adapters ajoute `serde_json.workspace` et `sha2 = "=0.11.0"` ; worker CPU ajoute SHA2 exact pour revalidation modèle/source. Serde JSON reste 1.0.145 ; toolchain 1.98.1/édition 2024/x64 MSVC et eframe 0.35.0, whisper-rs 0.16.0 CPU sans CUDA, Symphonia 0.5.5 restent inchangés.

SHA2 0.11.0 reprend TECH-D18-02. Documentation primaire consultée : [SHA2 0.11.0](https://docs.rs/sha2/0.11.0/sha2/), 2026-10-05. Pas de nouveau runtime codec ou moteur/DLL proposé. Notices/transitives SHA2 et diff exact lockfile devront être conservés dans les preuves et repris par L06 ; aucune qualification de packaging obtenue ici.

## 8. Couverture et preuves attendues

| REQ → UC/AC | Owner | Risque lié | Preuve L01 attendue |
|---|---|---|---|
| REQ-03 → UC-03/AC-03 | Application, decoder/worker | R-03 ; source/format/obsolescence | WAV réel → segments → sorties ; hash source avant/après ; entrée invalide visible |
| REQ-06 → UC-06/AC-06 | Snapshot application, moteur, UI | R-04 | Langue manuelle transmise et effective ; choix suivant sans mutation job actif |
| REQ-12/13 → UC-10/AC-10 | Archive/application/history/UI | R-02 ; RISK-T-D18-03 | TXT/SRT/manifeste/pointeur cohérents ; source référencée ; scan Complete/Recoverable |
| REQ-24 → UC-19/AC-19 | CPU worker/application/UI | R-01 ; RISK-T-D18-02 | Sur PC GPU, worker CPU effectif attesté, aucune initialisation GPU, CPU affiché |
| Q-05 → REQ-20/22/23 | Snapshot application/UI | Destination changée en cours | Job A garde racine A, job B prend B, aucun move/delete |

Nouveaux risques techniques proposés : **RISK-T-L01-01** consumer non câblé ; **02** UI bloquée ou commandes perdues ; **03** EOF assimilé à réussite/protocole ambigu ; **04** Complete publié avant reçu durable. Tous PROPOSED, owner architecte/AUTH-COORD ; aucun risque fermé par cette rédaction.

**DESIGN_FEASIBILITY :** disposition Cargo/core/UI/root étayée par les sources et la disposition acceptée. Aucun nouveau SPIKE requis à ce stade pour déplacer le wiring. Une incompatibilité native/codec découverte ne sera pas masquée : retour architecte et SPIKE distinct si faisabilité structurante non couverte.

**PRODUCT_VALIDATION, NOT RUN :**

- V-BOUNDARY : compilation core sans adapters ; Cargo/features, imports UI/root et CPU sans CUDA.
- V-IMPORT : véritable worker CPU, modèle hashé, WAV autorisé et source intacte ; langue, progression, sorties vérifiées ; modèle absent/faux, WAV corrompu, dossier refusé/espace insuffisant, événement obsolète.
- V-UI : EXE desktop exact assemblé par `main/root`, interaction dans DesktopApp réel, progression/CPU/langue/historique/erreurs observés ; aucun mock moteur.
- Contrats : double Start, Stop stale, saturation, Quitter en attente sans blocage UI, End incomplet refusé, scan répété stable.

Chaque résultat doit lier stimulus, branche réellement atteinte, effet et témoin. Un test pur du port ne prouve ni IO réel, ni worker, ni UI.

**DELIVERY_QUALIFICATION, NOT RUN :** imports PE/payload final, VC runtime/DLL/notices, installation, offline et environnement cible. Aucun résultat L00 ou banc D19 n’est recyclé en qualification du produit L01.

## 9. Impact documentaire et suite

D19 reste immuable et sa référence technique est conservée tant qu’aucune garantie ne change. DETAIL-P01 reçoit un complément versionné ; un nouveau candidat PLANS, proposé **P-WHISPER-03**, reprend P02 avec L01/ownership/IPC/contrats et dépendances CODE actualisés. P02 ne doit pas être réécrit.

L02 est directement affecté par ports application, schémas archive et DTO IPC ; L03–L05 consommeront ces contrats. L04 worker GPU devra implémenter V2. L06 reprend lockfile/notices et appariement parent/workers. L07 conserve qualification finale. Aucun travail parallèle sur ces contrats sans ownership coordonné.

L00 completion/preuves restent conservées. Les fichiers L00 à modifier deviennent préconditions CODE hashées du nouveau scope ; les fichiers absents restent des chemins à créer, sans hash fictif. Actualiser allowlist, autorisation sourcée et `code_state` avant un nouveau préflight. Le PASS historique L01 ne démontre pas ce scope révisé.

**Questions/tâches restantes, owner AUTH-COORD :** persister ce paquet et son attribution ; faire confirmer classification DETAIL/PLANS ; résoudre Q-L01-BOUNDS-01 ; faire produire puis revoir P03 exact ; obtenir/enregistrer l’autorisation utilisateur des chemins supplémentaires ; contrôler la vue documentaire/code réelle avant `check-state --lot L-WHISPER-01`. L01 reste suspendu pour son intégration jusqu’à ces gates. Ledger L01 et V-IMPORT/V-UI seulement après gate et exécution autorisée.

## 10. Empreintes fondatrices personnellement lues

| Fichier | SHA256 |
|---|---|
| `docs/predev/state.json` | `92924345e6dd3f133cfdd95a97d834d0a283342d04cd1a50153a58475eb5faed` |
| `docs/predev/USER_AUTHORIZATION_L01.md` | `b6088b12a1e8aa221db1f1b4ca9e29a5e867fce35590f217ff82164548cc3151` |
| `DETAIL-P01-raw.md` | `c554c748d888eb91aef8cff024ba4eb6b3b7d0317f333f0395111c084cae8086` |
| `Q-05-answer-raw.md` | `89538c0beebfe221883bb9e260d415818a992e58273bb870ba5dbaddea0f45c6` |
| D19 `08_STATE_AND_PORT_CONTRACTS.md` | `ffa9efb206c42941a9c846cfdf6a1783b766781472c556e1aad4187958406240` |
| D19 `50_ARCH_DECISIONS_D18.md` | `bac81d0eca399cd7cef48a0e87eb778aa60c0981daf35e03a959dd265dec6b80` |
| D19 `07_COVERAGE_AND_ACCEPTANCE.md` | `03e88ece134bfa7f388c60468e31046b5d591238c24769a4a37f6f356c61e0cf` |
| P02 `01_LOTS.md` | `c9e1b6ef5dfa2e46ecf7b306f587fec603b83322f103753809f1530f2c08b4d5` |
| P02 `02_VERIFICATION_AND_PREFLIGHT.md` | `bdc40168bdeffd0ffb1aea469a3058c5e2802dc414ac4c0f07e30ae179a91ef5` |
| `RAILGUARD.md` | `9fed5e976f9f49a994718637e3489acfe7d341911645969d0f5e4fc8a9107c34` |
| `core/src/application.rs` | `f9aa136fccebe741f4ff8e3bb1c0f2a04c0562d1e9d8fedd09115c79f6e88cb0` |
| `core/src/ports.rs` | `03fdef88124b4cc7c4cb7a65d76c3cc92cf067be774026b88b162469401d9b03` |
| `core/src/ipc.rs` | `9c64a6af41a1fb5fe22ffb8624441fb9e530e5c11b06328ffb2b8f6269d22e10` |
| `desktop/src/main.rs` | `04818ef9fdc8d37d0166b71b989da8d63b69a8b372af480c425e24f2aff7e38b` |
| `desktop/src/ui.rs` | `9317b88831780f53cc69aefdc8fb0b3c5d196fa7c9729233a8840c5b5064e4c8` |
| `desktop/Cargo.toml` | `0528886e5b8575b30ada28910a50dbfe9519a7c364d33043161e046de9121d15` |
| `adapters/src/lib.rs` | `6329856cb79a1ca8712dc6f549ce7d003abbc1ed7452a89be496b01d0750fbff` |
| `adapters/Cargo.toml` | `812d0dbc03e1359dc1bd19f5576d03a91dcaa0878022380e2f7e61901c75f344` |
| `worker-cpu/Cargo.toml` | `3fc393931d7d16f705b7d37fee9b0821b21f48cd3b696af2394960c3aa159bb9` |
| `Cargo.lock` | `447abf23220b0a3cb5c58ea2222f0e04355acb5f2d179c33aacc771c8ba616b8` |

Règles centrales lues : les cinq `agent-rules`, les deux SKILL et références handoff/engineering/deliverable-quality/workflow/runtime. Empreintes clés : constitution `74865dcb4e69173d43a2e38f3dad134fe0816c9da4a7a50029c7445ca3d85b38`, RUST_RULES `ed8789932c0ad8abdc6422c7730a802f160a27b210ed0dafc6cd6318d90751fe`, engineering-contract `1f5af0cc54a55339ed2f8946860b008383015c9301d311266104da895e94c5ed`.

<oai-mem-citation>
<citation_entries>
MEMORY.md:139-139|note=[transport and checkpoint provenance workflow]
</citation_entries>
<rollout_ids>
</rollout_ids>
</oai-mem-citation>