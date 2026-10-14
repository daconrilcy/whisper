# Whisper
Objectif produit : dans l'application Windows existante, transcrire un fichier WAV/MP3 ou la voix captée par le micro avec le modèle local large-v3-turbo et un choix explicite CPU ou GPU ; afficher le texte micro confirmé par blocs et enregistrer TXT, SRT et WAV de même nom à l'arrêt. Le GPU est présélectionné si CUDA est confirmé au démarrage, sinon le CPU est présélectionné avec un message clair ; le choix reste manuel et fixe pendant une session. Le GPU traite l'audio plus vite que sa durée et affiche le premier texte micro en moins de 15 secondes, avec une qualité comparable au CPU. Le CPU reste disponible.
Différés : mode Auto et bascule automatique, supervision CPU/GPU, tray, reprise après panne, changement de modèle, sortie MP3.
Évolution micro : livrée avec réserves, validée par l'utilisateur dans la fenêtre Windows ; bilan dans `docs/cadrage/bilan-evo-micro-direct.md`. Le cadrage initial reste dans `docs/cadrage/evo-micro-direct.md`.
Évolution GPU : livrée et validée par l'utilisateur dans la fenêtre Windows ; bilan dans `docs/cadrage/bilan-evo-gpu.md`. Le cadrage initial est dans `docs/cadrage/evo-gpu.md`.
État du produit : MVP validé le 13 octobre 2026 (`v0.1.0-mvp`) ; évolution micro locale `v0.2.0-micro`, livrée avec réserve de vitesse ; évolution GPU locale `v0.3.0-gpu`.
Historique : design D-WHISPER-30, plan P-WHISPER-31 et state.json en phase IMPLEMENTATION ; L00–L03 completed, L04–L07 planned. Ces documents décrivent une V1 plus large et ne donnent pas d'autorisations nouvelles.
Les nouveaux livrables de cadrage vont dans `docs/cadrage/`, suivis par Git. `docs/predev/` reste une archive historique en lecture.
Stack déclarée : Rust ; autre technologie ou code système seulement sur accord précis dans la session en cours.
G1 est arrêté définitivement. Une tâche Whisper doit servir l'objectif produit ; utiliser Git et les tests utiles, sans broker ni campagne documentaire ajoutée.
