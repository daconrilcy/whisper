# Matrice de fermeture du finding WHISPER-DESIGN-008 — D10

Source du finding : `T-WHISPER-REVIEW-09/review-D9.json`, sur le manifeste D9. La fermeture appartient au reviewer indépendant sur un futur candidat exact. Cette matrice ordonne le travail restant ; elle n'attribue aucun PASS.

| Volet | Preuve actuelle | Preuve encore nécessaire | Owner / décision |
|---|---|---|---|
| Installation V1 sur PC cible | EXE portable CPU/GPU avec PATH réduit, modèle et DLL ; `19` | Installateur local : acquisition et hash du modèle, licences/redistribution du paquet, installation interrompue, premier lancement offline installé ; logs et liste des octets livrés. PC propre différé par CHANGE-001. | Architecte Rust ; réduction de V1 seulement sur décision utilisateur. |
| Calcul effectif | GPU CUDA et CPU forcé observés sur un échantillon ; GPU demandé peut tomber silencieusement sur CPU. | Test GPU forcé absent puis panne pendant job : arrêt et proposition CPU sans bascule ; Auto→CPU durant job, identités/offsets uniques et état effectif visible. | Architecte Rust ; AC-07/19. |
| MP3 et passages | Décodeur lit deux profils ; encodeur produit un MP3 décodable. | Encodage live multi-passages, un MP3 par passage, TXT/SRT cumulés, correction du padding/alignement, reprise après interruption et qualité sur WAV existants. | Architecte Rust ; AC-03/10/20. |
| Durabilité et charge | 21 cas de publication après arrêt de processus ; 100 pertes de fragments ouverts, P95 480 ms à confirmation 0,5 s. | Injection autour de chaque confirmation et publication, saturation sous charge longue, worker/encodeur bloqué, arrêt manuel et état récupérable ; borne de perte à décider. Coupure électrique réelle non démontrée. | Architecte Rust et domaine ; AC-09/11/12/13. |
| VAD | 24 clips FLEURS et deux WAV initiaux ; fractions de blocs, sans annotation fine. | Annotation temporelle de voix/silence et bruit réel FR/EN micro pour précision/rappel ; choix du mode et seuil produit. Avec DEC-28, cette cible n'est pas réalisable à partir du corpus disponible. | Analyste + utilisateur, Q-08. |
| Délai live | GPU : 585 ms médiane et 704 ms P95 après fenêtre, console/modèle préchargé ; CPU : file max 29,618 s sur 4 fenêtres. | Horodatages capture→premier rendu dans l'application, charge longue et files bornées. CPU : retard et backend effectifs visibles, politique de saturation sans perte silencieuse ; aucune promesse temps réel selon DEC-27. | Architecte Rust + analyste ; AC-02. |
| Seuils | Aucun seuil V1 accepté. | Définir métriques, échantillonnage, règle d'échec et valeurs acceptables après mesures sur conditions cibles DEC-26 ; si DEC-28 reste, accepter explicitement une cible réduite ou laisser DESIGN DRAFT. | Utilisateur, Q-08. |

## Ordre et gate

1. Réaliser les essais techniques autorisés de SPIKE-01/02 sur le PC cible, en gardant les preuves et limites reproductibles.
2. Faire mesurer dans le vrai produit les propriétés qui en dépendent ; avant code produit, conserver leur méthode et la faisabilité démontrée distinctes de `NOT RUN`.
3. Présenter les seuils et le corpus effectivement disponibles à l'utilisateur. L'absence de WAV micro EN/bruit courant, d'annotation fine et d'application instrumentée interdit de conclure que la cible DEC-26 est vérifiée.
4. Soumettre un nouveau candidat manifesté au reviewer ; lui seul peut fermer 008. DESIGN READY et PLANS restent bloqués tant que les rubriques concernées sont GAP.

## Mise à jour DEC-29

Les plafonds provisoires acceptés sont atteints par les SPIKE de perte et de replay GPU sur les WAV existants ; voir `38_PROVISIONAL_THRESHOLDS.md`. Cette constatation ne ferme aucun autre volet du finding 008. La mesure dans l’application et le seuil VAD restent ouverts.
