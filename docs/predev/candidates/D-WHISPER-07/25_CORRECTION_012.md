# Correction du finding WHISPER-DESIGN-012

Base : R-WHISPER-DESIGN-05 sur D-WHISPER-05, digest
`63168e8521a61e385bc2d8dc821103e32191fc16fd28c7a696d03d94c53eb3e6`.
Source brute : `24_REVIEW_RAW_D5.json` et TRANSPORT `T-WHISPER-REVIEW-05`.

Le candidat D6 conserve les 100 cas et 21 fenetres D5 sans nouvel essai.
Le script de perte versionne utilise maintenant la mediane usuelle et P95 nearest
rank. Le JSON et le tableau `21` concordent : pour la cadence 1 s, mediane 350 ms
et P95 980 ms. Pour 0,5 s, mediane 260 ms et P95 480 ms.

`21` distingue desormais les replays GPU sur fixtures FR/EN repetees trois fois
et le replay CPU sur le WAV FR original de 14,592 s. Les agregats live ne changent pas.

La fermeture de 012 appartient au reviewer sur le manifeste D6 exact.
WHISPER-DESIGN-008 reste ouvert ; aucun seuil V1 n'est accepte par ce correctif.
