# Whisper
Objectif produit : dans l'application Windows existante, transcrire sur CPU un fichier WAV/MP3 ou la voix captée en direct par le micro ; afficher le texte micro au fil de l'eau et enregistrer la transcription en TXT et SRT à l'arrêt.
Différés : GPU et mode Auto, tray, reprise après panne, sortie MP3.
Évolution micro : cadrage en cours dans `docs/cadrage/evo-micro-direct.md` ; son parcours n'est pas encore validé dans la fenêtre Windows.
État du produit : MVP atteint et validé dans la fenêtre Windows le 13 octobre 2026 ; version locale `v0.1.0-mvp`.
Historique : design D-WHISPER-30, plan P-WHISPER-31 et state.json en phase IMPLEMENTATION ; L00–L03 completed, L04–L07 planned. Ces documents décrivent une V1 plus large et ne donnent pas d'autorisations nouvelles.
Les nouveaux livrables de cadrage vont dans `docs/cadrage/`, suivis par Git. `docs/predev/` reste une archive historique en lecture.
Stack déclarée : Rust ; autre technologie ou code système seulement sur accord précis dans la session en cours.
G1 est arrêté définitivement. Une tâche Whisper doit servir l'objectif produit ; utiliser Git et les tests utiles, sans broker ni campagne documentaire ajoutée.
