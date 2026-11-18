Avancée : sur le Snapdragon testé, Whisper a pu démarrer après installation du pack graphique Microsoft. Le bootstrap actuel interroge l’utilisateur dans le terminal.
Blocage : le candidat ne démontre pas qu’un utilisateur est averti du manque graphique avant installation.
Décision proposée : intégrer au véritable installateur le parcours de détection, de correction guidée et de revérification décrit ci-dessous.

## Parcours attendu

Au double-clic sur l’installateur, une fenêtre Windows en français doit fonctionner même si le rendu OpenGL de Whisper est indisponible. Avant la première écriture dans l’installation Whisper, vérifier la compatibilité CPU, les fichiers requis du paquet, le modèle local attendu et la capacité de créer le contexte graphique requis par le desktop dans la session courante. Le choix du mécanisme graphique et de la sonde reste à valider avant implémentation.

| Situation | Réponse de l’installateur |
|---|---|
| Rendu indisponible sur Windows ARM64 et pack de compatibilité absent | Expliquer pourquoi Whisper ne pourra pas afficher sa fenêtre. Proposer « Ouvrir le Microsoft Store », « Vérifier à nouveau » et « Annuler ». L’ouverture du Store se fait à la demande de l’utilisateur. |
| Pack déjà présent, mais rendu toujours indisponible | Expliquer que la vérification graphique échoue encore. Proposer de vérifier à nouveau et rendre le détail technique consultable. |
| Rendu fonctionnel | Admettre le contrôle graphique, y compris si le pilote fournit directement la capacité nécessaire. |
| Modèle absent ou incorrect | Proposer « Choisir le modèle », vérifier son empreinte et indiquer clairement si le fichier convient. |
| Fichier livré manquant ou invalide | Nommer le fichier et inviter à récupérer le paquet complet. |
| CPU incompatible | Expliquer la limite matérielle. Une absence de GPU NVIDIA sur un PC compatible CPU laisse l’installation CPU accessible. |

Activer « Installer Whisper » lorsque les contrôles requis réussissent. Annuler, fermer le Store ou échouer à une revérification conserve l’installation existante et laisse une installation neuve sans fichiers Whisper.

## Recette d’acceptation proposée

Sur une image Snapdragon initialement dépourvue du pack et où la sonde graphique échoue, lancer le véritable installateur par double-clic. Observer l’alerte avant toute installation Whisper. Vérifier l’annulation, puis suivre le lien Store, installer le pack, revenir à la même fenêtre et revérifier. Constater que l’installation devient disponible et que le desktop s’ouvre. Le banc doit observer les actions du produit ; préinstaller lui-même le pack masquerait précisément le comportement à qualifier.

Sur un PC Intel sans NVIDIA dont le rendu fonctionne, vérifier que l’installation CPU est admise. Tester séparément le modèle incorrect et un paquet incomplet. Rattacher les observations au candidat modifié. L’import Snapdragon déjà documenté reste la preuve de fonctionnement sur une machine équipée. [État de qualification](etat-qualification-l4.md) · [Plan de distribution](plan-implementation-distribution-cpu-x64.md)
