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

## Mise à jour DEC-30/31

Des intervalles d’activité acoustique candidats sur 26 WAV sont disponibles dans `39_AUTO_ANNOTATION.md` et `evidence/annotation-candidates.json`. Ils ne sont pas vérifiés humainement ; le seuil VAD et la validation de la cible DEC-26 demeurent ouverts.

## Mise à jour DEC-32 et essais E0/E3

Le gate reste DRAFT. Le protocole détaillé est dans `41_ARCH_008_PROTOCOL.md`. L’inventaire E0 et le smoke MP3 E3 figurent dans `42_TECHNICAL_FOLLOWUP.md`. Ils ne ferment pas installation, bascule, durabilité, charge, VAD ni rendu UI ; le reviewer doit requalifier 008 sur un nouveau candidat exact.

## Revue D14 et correction D15

La revue D14 maintient 008 ouvert et ouvre 014 sur la provenance du paquet détaillé de l’architecte. D15 relie le brut exact transporté dans `T-WHISPER-ARCH-008-DETAIL` et copié en `45_ARCH_008_RAW.md` ; seul le reviewer peut décider de fermer 014. Les essais E1/E2/E3 complet/E4/E5/E6 restent à conduire.

## Mise à jour D17 — volet VAD

Les 26 WAV et 99 intervalles ont été attestés par l'utilisateur ; les métriques du mode WebRTC 1 passent les seuils de conception acceptés sur ce corpus (`47/48`). Le test Rust, la cible micro EN/bruit courant et le produit final restent non exécutés. Les autres volets 008 sont inchangés. Seul le reviewer peut fermer ou restreindre 008 sur le manifeste D17.

## Proposition D18 à revue pour 008

E0/E1 livrent provenance, modèle pin/hash, acquisitions interrompues/invalides, package CPU/GPU local et prérequis VC documenté. E2 prouve strict absent/échec injecté et Auto→CPU avec deux EXE, offsets et stale rejeté. E3 intégré prouve archives MP3 réelles, groupe cumulatif et pending/reprise. E4 prouve publication/hash, file sync/restore et audio+texte par fragment avant ACK. E5 prouve arrêt/diagnostic sous saturation indépendante ; E6 reprend le VAD Rust. Voir `49/50` et sources brutes. Les limites déclarées ne deviennent pas PASS produit ; le reviewer indépendant seul peut fermer 008.

## D19 — deux sous-findings restants de R-WHISPER-DESIGN-18

INSTALL : AppContainer sans capacité réseau, témoin normal/restrictif et CPU/GPU du payload exact hashé transcrivant localement ; sources/logs `evidence/d19`. MODES : callback natif avant `state.full`, GPU absent refusé, erreur worker 73 effectivement traitée en strict/Auto, ancienne génération livrée au même `Controller.accept` puis rejetée avant `committed.push`. Reste à faire juger indépendamment sur D19 ; le coordinateur ne ferme pas 008.
