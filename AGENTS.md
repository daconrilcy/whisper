# Whisper
Objectif produit : importer un fichier audio WAV ou MP3 dans l'application Windows existante et obtenir sa transcription en TXT et SRT, sur CPU.
Différés : micro en direct, GPU et mode Auto, tray, reprise après panne, sortie MP3.
État du produit : MVP atteint et validé dans la fenêtre Windows le 13 octobre 2026 ; version locale `v0.1.0-mvp`.
Historique : design D-WHISPER-30, plan P-WHISPER-31 et state.json en phase IMPLEMENTATION ; L00–L03 completed, L04–L07 planned. Ces documents décrivent une V1 plus large et ne donnent pas d'autorisations nouvelles.
Les nouveaux livrables de cadrage vont dans `docs/cadrage/`, suivis par Git. `docs/predev/` reste une archive historique en lecture.
Stack déclarée : Rust ; autre technologie ou code système seulement sur accord précis dans la session en cours.
G1 est arrêté définitivement. Une tâche Whisper doit servir l'objectif produit ; utiliser Git et les tests utiles, sans broker ni campagne documentaire ajoutée.
