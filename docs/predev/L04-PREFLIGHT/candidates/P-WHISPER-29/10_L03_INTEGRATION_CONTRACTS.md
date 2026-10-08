# Contrats d’intégration L03 — P-WHISPER-10

Parent DESIGN : D-WHISPER-19 (`903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`). Réponse DETAIL-P02 acceptée ; Q-07 demeure inchangée. Ce document est un contrat de planification, pas preuve d’implémentation ou de qualification.

## Dépendances attribuées

- `whisper-adapters`: `cpal = "=0.18.2"`, `default-features = false`, entrée WASAPI.
- `whisper-adapters`: `rubato = "=0.16.2"`, `default-features = false`, feature `fft_resampler`.
- `whisper-adapters`: `webrtc-vad = "=0.4.0"`.
- `whisper-worker-cpu`: `rusty_mp3 = "=0.8.0"`.

Ces versions/features sont des propositions attribuées à DETAIL-P02, à vérifier par source officielle et préflight package avant édition. Core reste sans dépendance native.

## Capture, conversion et segmentation

- CPAL 0.18.2 via WASAPI pour l’entrée Windows.
- Conversion vers PCM16 mono 16 kHz via rubato 0.16.2 ; VAD webrtc-vad 0.4.0, mode 1, blocs de 320 échantillons / 20 ms.
- Fenêtre d’inférence limitée à 80 000 échantillons / 5 s. Le backlog lent s’accumule sur disque ; aucun arrêt fondé uniquement sur la durée.
- Callback de capture borné à 100 slots, sans IO bloquante. Saturation, erreur de conversion ou rupture de capture terminent explicitement l’entrée avec diagnostic ; aucune perte silencieuse.
- Stop reste indépendant du canal data, VAD et débit worker.

## Durabilité et reprise

Stager le PCM16 mono 16 kHz sur disque durable, croissance nominale 32 000 octets/s. Synchroniser au plus toutes les 25 trames (0,5 s) et lors du drain ; cette cadence ne promet pas un maximum universel de perte totale. Séparer `captured`, `admitted`, `audio_durable` et `confirmed_fragment`. L’audio durable seul ne confirme pas de fragment : les préconditions audio/texte/record du journal D19 demeurent obligatoires. Préserver Recoverable à l’erreur/arrêt, exposer cause et plages non confirmées.

Le ring de 100 slots est dimensionné par `ceil(Fs × 20 ms) × channels × sizeof(f32)` plus un slot local. À 48 kHz stéréo, la proposition donne 768 000 + 7 680 octets. File PCM writer : au plus 50 trames de 320 PCM16 (32 000 octets) plus une trame locale de 640 octets. Worker : une fenêtre de 80 000 échantillons et un buffer suivant au plus. Ces capacités doivent être confirmées par le code/test sans overflow.

## Capacités et comptabilité mémoire

DETAIL-P02 conserve les budgets wire du contrat P09 `06_L01_INTEGRATION_CONTRACTS.md` :

| Ressource | Contrat |
|---|---|
| Effets en attente | Au plus 8 ; somme des représentations wire ≤8 MiB |
| Effet actif | Un actif hors du canal des effets en attente ; comptabilisé séparément |
| Frame IPC | ≤1 048 576 octets enveloppe incluse ; vérifier avant allocation/désérialisation et avant émission |
| Événements persistables | Au plus 64 globalement ; budget wire global ≤64 MiB ; aucune éviction d’événement durable |
| Contrôle | Stop indépendant du canal data ; contrôle conservé pendant saturation |
| Vue provisoire UI | Dernier snapshot remplaçable ; aucun fragment durable ou commande critique silencieusement jeté |

L02 utilise `sync_channel(8)` pour les effets. Sa répartition des événements persistables est de 21 places par étage sur trois étages, plus un événement en transfert, soit 64 globalement. L03 préserve cette comptabilité et vérifie événements en transfert, événements locaux et rétropression réelle.

Ces valeurs bornent les représentations wire concernées, pas le RSS. Mesurer séparément buffers PCM/f32, sérialisation, objets locaux, contexte/modèle natif, codec, allocations de conversion et buffers OS. Aucune valeur wire n’est une promesse de mémoire totale.

## P02-GAP-BYTES-01 — événements locaux Queue/History

Nature `PRODUCT_VALIDATION`; statut `PROPOSED / NOT RUN`; owner exécution L03, contrôle indépendant après exécution. Les bornes de nombre d’événements et frames IPC ne prouvent pas le budget des événements construits localement dans le parent ; Queue/History peuvent croître sans validation de frame IPC, aucun compteur global de bytes n’est établi.

Inventorier construction, stockage, transfert et consommation ; mesurer le nombre global en vol, wire et allocations locales Queue/History ; tester histoire/file assez grande, limites et dépassement avec rétropression. Si nécessaire, admission bornée ou pagination par curseurs, préservant identité, ordre, accès à toutes les entrées durables et absence de démarrage implicite. Aucune éviction ou troncature silencieuse. Enregistrer fixture/hash, candidat, stimulus, branche, occupation, wire, allocations locales et effet observable. Un mock ou `sync_channel(8)` seul ne clôt pas ce point. Toute extension hors allowlist retourne au coordinateur.

Source attribuée : `sources/ARCH-DETAIL-P02-v1-R2.md`; acceptation : `sources/user-acceptance.md`.
