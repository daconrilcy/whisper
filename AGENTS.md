# Whisper
Objectif produit : dans l'application Windows existante, transcrire sur CPU un fichier WAV/MP3 ou la voix captée par le micro ; afficher le texte micro confirmé par blocs et enregistrer TXT, SRT et WAV de même nom à l'arrêt. Le micro est livré avec une réserve de vitesse : sur CPU, le traitement prend environ 2,25 fois la durée audio et fonctionne comme une dictée avec retard.
Différés : vitesse en temps réel dans une évolution GPU séparée, mode Auto, tray, reprise après panne, sortie MP3.
Évolution micro : livrée avec réserves, validée par l'utilisateur dans la fenêtre Windows ; bilan dans `docs/cadrage/bilan-evo-micro-direct.md`. Le cadrage initial reste dans `docs/cadrage/evo-micro-direct.md`.
État du produit : MVP validé le 13 octobre 2026 (`v0.1.0-mvp`) ; évolution micro locale `v0.2.0-micro`, livrée avec réserve de vitesse.
Historique : design D-WHISPER-30, plan P-WHISPER-31 et state.json en phase IMPLEMENTATION ; L00–L03 completed, L04–L07 planned. Ces documents décrivent une V1 plus large et ne donnent pas d'autorisations nouvelles.
Les nouveaux livrables de cadrage vont dans `docs/cadrage/`, suivis par Git. `docs/predev/` reste une archive historique en lecture.
Stack déclarée : Rust ; autre technologie ou code système seulement sur accord précis dans la session en cours.
G1 est arrêté définitivement. Une tâche Whisper doit servir l'objectif produit ; utiliser Git et les tests utiles, sans broker ni campagne documentaire ajoutée.
