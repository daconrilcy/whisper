# Seuils provisoires sur les essais WAV existants — DEC-29

Source produit : réponse directe de l'utilisateur du 2026-10-05 : « Oui, comme seuils provisoires sur ces essais (recommandé) » à la proposition explicite : perte ≤ 0,5 s après arrêt de processus avec confirmation à 0,5 s ; GPU ≤ 1 s au P95 après chaque fenêtre de 5 s ; délai CPU seulement affiché. Cette décision autorise ces valeurs **pour les essais bornés existants**. Elle ne valide pas la qualité du futur produit ni la cible micro FR/EN calme/bruit courant DEC-26.

| Mesure | Méthode et seuil accepté provisoirement | Observation actuelle | Statut de la preuve |
|---|---|---|---|
| Perte de fragment ouvert | Arrêts indépendants de processus simulant trames 20 ms, confirmation toutes les 0,5 s ; perte = capturé moins confirmé ; maximum ≤ 500 ms sur le jeu observé. | 50 cas à 0,5 s : maximum 480 ms, P95 480 ms, médiane 260 ms (`21`, `loss-results.json`). | PASS **sur le SPIKE et cette fixture seulement**. Capture réelle, panne électrique, synchro de répertoire et longue charge non démontrées. |
| GPU après fenêtre | Replay WAV FR/EN FLEURS sur ce PC, modèle préchargé, fenêtres 5 s ; P95 nearest-rank de fin de fenêtre→sortie console ≤ 1 000 ms. | 45 fenêtres : P95 704 ms, médiane 585 ms, max 804 ms ; file max 0 (`29`, `live-fleurs-summary.json`). | PASS **sur le replay console seulement**. Ce n'est ni capture→rendu UI, ni qualité du texte, ni garantie sur un micro. |
| CPU live | Mode CPU permis ; backend et retard mesuré affichés ; aucun plafond de temps réel. | Replay de 4 fenêtres : file max 29,618 s, sans UI (`29`). | Politique produit acceptée ; comportement visible du produit NOT RUN. |
| VAD | Pas de seuil accepté par DEC-29. Les WAV ne possèdent pas d'annotation temporelle fine ; la précision et le rappel ne sont pas calculables. | Fractions de blocs et bruit seul dans `21/29`, insuffisantes pour un seuil qualité. | OPEN ; annoter les WAV existants avant proposition d'une valeur, ou décision produit explicite. |

Les deux PASS mesurent la conformité des **prototypes** aux plafonds provisoires sur les données présentes. Ils n'annulent pas WHISPER-DESIGN-008 : installateur, GPU forcé/Auto durant job, MP3 cumulatif, durabilité/charge, VAD et instrumentation UI restent à démontrer. Le test sur PC propre demeure différé par CHANGE-001 ; l'installation locale reste incluse.
