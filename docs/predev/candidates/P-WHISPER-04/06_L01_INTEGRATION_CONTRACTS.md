# Contrats d’intégration — L-WHISPER-01 / P-WHISPER-04

Sources : DETAIL-P01 v3, ARCH-L01-INTEGRATION v2, ARCH-L01-BOUNDS v1 et revue de compatibilité D19 v2, liées dans `05_SOURCE_AND_RULE_INDEX.md`. COMPAT est bornée à INTEGRATION et ne ferme pas la revue PLANS ni n’approuve les nombres BOUNDS.

## Application, port et effets

`ImportApplication` possède job/génération, snapshots langue/compute/destination, transitions, ordre des effets, arrêt, validation des résultats et décision de publication. `ImportIoPort`, possédé par application, admet des effets typés sans blocage et retourne des événements par polling non bloquant. UI possède l’état de présentation ; adapter les handles/processus/files techniques ; worker le contexte natif et buffers.

Commande de démarrage : job/génération, référence source, langue/compute/destination et modèle local approuvé (chemin/hash/taille comme données). Les adapters ouvrent les chemins ; aucun téléchargement en L01. Le dossier est snapshoté quand le job est accepté, selon Q-05 ; les modifications UI touchent les jobs suivants seulement.

| Effet | Contrat et événements |
|---|---|
| Préparer → Prepared | Pas de job actif ; source/modèle lus sans mutation, identités/hashes/taille validés ; erreur explicite ; pending durable avant worker |
| Démarrer → Ready/Progress/Segment/End | CPU demandé ; Ready après initialisation/attestation ; événements corrélés au job/génération/instance |
| Persister → reçu | Plage/séquence courante sans doublon ; staging append/version et sync avant reçu |
| Publier → reçu vérifié | End réussi et fragments requis persistés ; TXT/SRT/manifeste vérifiés, pointeur publié dernier ; Complete après reçu |
| Scanner → Complete/Recoverable | Snapshots/manifeste/pending vérifiés ; idempotent ; aucun démarrage ou nettoyage automatique |
| Arrêter → Stopped/diagnostic | Contrôle indépendant ; génération invalidée avant admission tardive ; source/pending conservés |

Admission n’est jamais un ACK durable ou Complete. L’application décide ; adapter exécute. Distinguer Busy, InvalidInput, SourceMissing/Changed, ModelMissing/HashMismatch, UnsupportedLanguage/Format, StorageUnavailable/Full/Corrupt, ProtocolMismatch, WorkerExited, BackendMismatch, Saturated, Cancelled et StaleResponse. Les codes sont détails d’implémentation ; les distinctions gouvernent transitions et récupération.

## Composition et frontières

`desktop/src/main.rs` déclare `mod root`; `root.rs` assemble `ImportApplication`, ports/adapters, service IO, canaux et worker CPU, puis injecte l’application dans `whisper_desktop::run(application)`. `desktop/lib.rs` réutilise l’API/run existante, n’expose pas root, et reste sans adapters/fs/process/native. `ui.rs` reste présentation pure via façade; Cargo rend les deps du package disponibles à toutes cibles, donc vérifier imports/consumers réels. Aucun scheduler ou état métier parallèle dans root/UI/worker.

## IPC v2

IPC_PROTOCOL_VERSION=2 corrèle préparation/démarrage avec job, génération, instance, source hashée; atteste backend; identifie segments/plages; porte End explicite (dernier offset/séquence), erreurs et Stopped corrélés; fournit contrôle indépendant du flux data. Refuser V1 par ProtocolMismatch. Parent et CPU sont livrés/revus ensemble ; GPU suit v2 en L04. DTO IPC séparés des schémas durables. Vérifier version/job/génération/requête/séquence/plage avant mutation/publication. EOF sans End n’est pas une réussite.

## Bornes et saturation

| Ressource | Borne |
|---|---|
| Effets en attente | 8, ≤8 MiB wire ; 1 actif hors file |
| Événements persistables | 64, ≤64 MiB wire ; aucun événement persistant évincé |
| Frame IPC | ≤1 048 576 octets enveloppe incluse, validation avant allocation/désérialisation |
| Décodage | 1 requête active, 0 ou 1 bloc ; 0 = EOF explicite |
| Inférence | 1 fenêtre active |
| Contrôle | 1 Stop par génération active et 1 Shutdown, indépendants/coalescés |
| UI provisoire | 1 dernier snapshot remplaçable seulement |

Fenêtre PCM 5 s : mono/16 kHz, max 80 000 samples. PCM16 ≤160 000 octets, f32 moteur ≤320 000. Valider `1 ≤ max_samples ≤ 80_000` avant allocation/décodage. Dernière fenêtre réelle 1..80 000, sans padding compté comme source. Plages [start,end), `end-start=samples.len()`, séquences continues et offsets remappés/vérifiés sur l’axe source. JSON PCM16 évalué à 560 000 octets hors métadonnées : vérifier la taille wire complète avant envoi. Pas de frame multisegment implicite ni cumul du fichier entier.

Ces budgets ne bornent pas RSS global : mesurer/inventorier contexte/modèle, codec, sérialisation, objets et buffers OS. `max_samples` seul ne prévient pas une allocation native antérieure. Saturation arrête les demandes de blocs et signale une erreur visible, sans perte silencieuse. UI reste réactive. Pas de délai maximum FFI/disque, kill automatique ou join UI infini ; Quitter reste ouvert si l’enfant ne confirme pas.

## Publication, récupération, migration

Importer WAV sans modifier/coller la source, sans copie audio ni MP3. Pending/génération, sync artefacts, vérification tailles/hashes/identités, manifeste et pointeur dernier. Après incohérence/coupure, état Recoverable ; ancien snapshot vérifié lisible ; tentative courante jamais Complete. Réconciliation idempotente, sans reprise/suppression automatique. Publication porte cette génération seulement ; aucune transaction multi-fichier ou garantie panne électrique générale.

Migration de fichiers préexistants N/A pour ce delta : L00 n’a pas livré d’écrivain durable. Archive/pending versionnés à première écriture ; version inconnue refusée sans réécriture. Rupture IPC V1→V2 reste testée. L02 porte FIFO/journal/reprise exhaustive.

## Vérifications futures

V-L01-CONTRACT et V-L01-BOUNDS : PRODUCT_VALIDATION / NOT RUN. Tester neufième effet, 65e événement, budget bytes, frame >1 MiB/incomplète/V1, max_samples 0/80_001, plage invalide/PCM vide/EOF court, génération ancienne vraiment livrée, Stop file pleine/inférence bloquée, End absent, publication refusée, double scan, vrai worker et vraie UI réactive, mémoire par étage et absence de cumul. Stimulus, branche, effet, témoin, sorties/hash/limites conservés dans les deux tests allowlistés. Un mock ne ferme pas le parcours réel.
