# Preuves techniques et risques

## Cible rapportée

Windows 11 Pro x64 build 26200 ; Core i9-14900K, 95,8 Go RAM, NVIDIA GTX 1080 Ti (nvidia-smi rapporté pilote 581.29), Intel UHD 770. Parsec Virtual Display Adapter exclu du calcul. Cette cible ne garantit pas d’autres PC.

## Options techniques proposées, non approuvées

| Sujet | Candidat antérieur | À qualifier |
|---|---|---|
| Toolchain | Rust/Cargo 1.99.0, édition 2024, MSVC x64 | Disponibilité, build reproductible et support. |
| Moteur | whisper-rs 0.16.0 | Révision native, features, ABI, build Windows. |
| GPU | CUDA candidat GTX 1080 Ti | Toolkit/runtime/DLL, compatibilité et preuve backend dans le binaire livré. |
| Capture | CPAL 0.18.2 | Périphériques, configurations, débranchement. |
| Décodage | Symphonia 0.5.5 | Variantes WAV/MP3, canaux, fréquence, erreurs. |
| Modèle | large-v3-turbo multilingue non quantifié candidat | Format compatible, licence, hash, taille, téléchargement. |
| MP3 output | Media Foundation candidat, crate windows 0.62.2 | Paramètres, disponibilité cible, distribution, reprise. |
| VAD | À choisir | Modèle, seuils, corpus et taux d’erreur. |

## Matrice de preuve

- WhisperLive/faster-whisper Python CPU : exécuté avec succès selon l’utilisateur.
- Même pile GPU : réussi, GPU effectivement utilisé selon observation manuelle utilisateur.
- CTranslate2 4.8.2, un device CUDA et types float32/int8_float32/int8 : rapporté dans le cadrage antérieur ; ne qualifie pas whisper-rs/ggml.
- `whisper-rs 0.16.0` CPU et CUDA : build Rust et exécution sur GTX 1080 Ti observés ; modèle local vérifié par SHA-256. Binaire portable avec DLL CUDA testé sur le même PC. Le détail et les logs sont dans `09`/`evidence/`.
- Décodage Symphonia 0.5.5 : un WAV et deux profils MP3 lus ; marge de padding MP3 observée.
- Durabilité : 21 points de sortie forcée de processus vérifiés ; ce n’est pas une panne de courant.
- Premières mesures bornées de VAD, perte du fragment ouvert et délai de texte en replay : `21_MEASUREMENTS.md` ; encodeur MP3 Rust décodable dans `T-WHISPER-MP3-SPIKE-04`. Aucune cible de qualité acceptée.
- Installateur sur ce PC, capture CPAL, VAD retenu et validé sur corpus annoté, charge longue, arrêt worker bloqué et produit intégré : **NOT RUN**. La qualification sur PC propre est différée par `CHANGE-001`.
- Commandes/logs de l’essai manuel : non attachés.

## Risques

Compatibilité moteur/CUDA/pilote/redistribution ; modèle/licence/hash/taille ; pertes dans fragment ouvert, fsync et atomicité ; import commencé interrompu ; backpressure et doublons de segments au fallback ; erreurs VAD ; sortie tripartite incohérente ; formats exacts ; confidentialité des données/logs/localisation à définir.

Des sources primaires ont été consultées et datées dans `09_TECHNICAL_SOURCES_AND_SPIKES.md`. Elles attestent des API/options documentées, pas un build ni une livraison fonctionnelle sur la cible. Les versions et revendications exactes restent à vérifier par les SPIKE ; des essais Rust bornés sont attachés, sans qualification du produit final.
