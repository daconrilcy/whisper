Avancée : la recette Dell T33 fournit un parcours automatisé de référence, avec ses entrées, ses exports et sa preuve de fermeture.
Blocage : les preuves propres aux runners T41 et T42 restent à obtenir.
Décision proposée : réutiliser le parcours T33 et éprouver ses échecs localement avant de mobiliser un témoin matériel.

# Préparer une recette automatisée de distribution

L’objectif est qu’un opérateur puisse partir d’une machine vierge, lancer une fois le paquet et remettre un seul ZIP exploitable. Ce cadrage guide la préparation des prochains essais. Les critères et mandats d’exécution restent ceux du [plan L4](plan-implementation-distribution-cpu-x64.md).

## Avant de réserver la machine

| Travail de préparation | Résultat à vérifier sur le poste de préparation |
|---|---|
| Partir du parcours qui a réussi sur le Dell | Reprendre le pilote, les entrées et le contrat du candidat T33. Énumérer seulement les preuves supplémentaires de la tâche visée. Utiliser la [recette réussie](T33-dcbf572ce7.zip) comme exemple de résultat attendu. |
| Assembler le ZIP réellement remis | Extraire ce ZIP dans un dossier neuf. Contrôler la présence et l’identité du bootstrap, du desktop, des workers, des DLL et notices prévues, du modèle hors ligne, du WAV et des scripts appelés. Le lanceur résout ses chemins depuis le dossier extrait. |
| Tester le lanceur tel que l’opérateur l’utilisera | Exercer le lanceur sous Windows PowerShell 5.1, avec un chemin contenant espaces et caractères spéciaux. Vérifier les réponses au bootstrap sur son entrée standard, son code de retour, le calcul SHA-256 et l’héritage du profil temporaire. L’installation démarre dans un profil vide et le produit crée ses propres données. |
| Rejouer les défauts déjà rencontrés | Tester une entrée manquante, un modèle d’empreinte incorrecte, une fenêtre recréée, un worker lent sans premier progrès, une sortie prématurée et un journal encore ouvert. Pour chaque cas vérifier l’étape, la cause conservée, le verdict et les pièces récupérables. Utiliser une horloge simulée pour les attentes longues. |

## Pendant l’essai

1. **Admettre le témoin.** Consigner l’OS, le candidat et les capacités requises. Pour T41, vérifier le prédicat matériel exact de T01. Pour T42, identifier le mécanisme CI autorisé, le bureau interactif de la réservation, le pack Microsoft OpenGL et la destination des résultats. Sur Snapdragon, prévoir le pack avant le créneau d’inférence. Le [mandat T42](mandat-t42-windows-arm64.md) détaille la mesure OS/PE à réaliser pendant la vie des deux processus.
2. **Suivre le vrai parcours.** Retrouver la fenêtre par le PID du desktop, rafraîchir l’arbre UI Automation et identifier le bouton audio par son libellé voisin. Vérifier le chemin du WAV dans la requête. Invoquer « Traiter » uniquement si la file expose cet état. En cas d’écart, enregistrer les contrôles disponibles et leur état au premier échec.
3. **Attendre une preuve observable.** Borner le démarrage, les dialogues, la publication et la fermeture séparément. Pendant le chargement ou le calcul CPU, afficher la phase, le temps écoulé, le PID et le dernier événement. Une période sans progrès peut durer plusieurs minutes. Prévoir l’interruption explicite du banc avec conservation du diagnostic.
4. **Décider sur le même import.** Relier requête, génération, moteur effectif, WAV, modèle, échantillons, manifeste et exports. Contrôler texte attendu, empreintes et bornes SRT. Consigner l’état du worker juste avant la fermeture normale du desktop puis sa terminaison. Le cas de sortie 1 observé sur le Dell est admissible uniquement sous les conditions vérifiées par le pilote T33 après publication.
5. **Remettre la preuve en une fois.** Donner au ZIP l’identifiant de l’essai et placer son rapport à la racine avec verdict, étape, cause première, temps et PID. Inclure journaux, identités, manifeste et exports disponibles. Rouvrir le ZIP, vérifier ses pièces et afficher son chemin exact. Sur une réservation distante, récupérer cette archive et vérifier la copie avant de libérer la machine.

## Décision de remise

Le paquet est prêt pour le témoin quand le lanceur et le contenu réellement remis ont été vérifiés ensemble, que les scénarios négatifs produisent un diagnostic exploitable et que l’on sait récupérer l’archive. Un prérequis manquant laisse l’essai matériel non exécuté. Une preuve obligatoire absente laisse l’essai incomplet. Un défaut fonctionnel observé sur le candidat est rapporté comme tel. Seule l’exécution sur le témoin prévu peut qualifier le critère matériel.
