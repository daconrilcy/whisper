# Résultat VAD sur annotations acceptées humainement — D17

## Contrôle de l'apport

`evidence/human-annotations.json` est la copie octet pour octet du fichier fourni par l'utilisateur. SHA-256 : `422955209e88dbcf628e38d0859e20b7fe7c47f3195b121d240f7fb684931607`. Schéma `whisper-human-annotations/1`, 26 fichiers distincts tous `verified=true`, 99 intervalles ordonnés et bornés, 26 hashes WAV concordants. Aucun intervalle n'a été modifié par rapport aux candidats automatiques. L'utilisateur a attesté avoir vérifié l'ensemble (DEC-33). La machine peut contrôler l'intégrité et la géométrie des intervalles, pas prouver qu'une écoute a réellement eu lieu.

## Méthode et mesures

Décodage FFmpeg mono PCM16 16 kHz, trames de 20 ms. Une trame est étiquetée parole lorsque son milieu appartient à un intervalle accepté. WebRTC VAD Python `webrtcvad` a été exécuté sur ces mêmes trames ; le mode 1 est celui retenu par DEC-35 pour le gate DESIGN. Le script exact et le rapport complet sont dans `evidence/compare_vad_human.py.txt` et `evidence/vad-human-comparison.json`.

| Groupe | WAV | Précision | Rappel | TP | FP | FN | TN |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| fr_fr | 12 | 0.8215 | 1.0000 | 3894 | 846 | 0 | 870 |
| en_us | 12 | 0.8873 | 0.9107 | 3313 | 421 | 325 | 821 |
| fr_micro | 1 | 0.8346 | 0.9953 | 424 | 84 | 2 | 219 |
| en_public | 1 | 0.6963 | 1.0000 | 337 | 147 | 0 | 66 |
| all | 26 | 0.8417 | 0.9606 | 7968 | 1498 | 327 | 1976 |

Critères acceptés : total précision ≥ 0,80, rappel ≥ 0,95 ; chaque groupe précision ≥ 0,65, rappel ≥ 0,90. Les cinq lignes passent ces valeurs sur ces 26 WAV. Les mêmes propositions RMS ont été validées sans correction, donc les erreurs communes ou limites d'annotation demeurent possibles malgré l'attestation humaine.

## Portée restante

Ces WAV ne représentent pas un microphone EN ni le bruit courant réel de DEC-26. La qualification du produit final sur ces conditions est `NOT RUN`. L'exécution VAD ici utilise le wrapper Python ; le moteur VAD Rust retenu et son comportement doivent encore être démontrés. La visualisation UI, la charge et les autres volets de 008 ne sont pas couverts. Aucun résultat ne ferme 008 sans revue indépendante du candidat exact.
