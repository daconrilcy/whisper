Avancée : la recette cible le guidage graphique de l’installateur x64 exécuté sur le Snapdragon.
Blocage : le parcours avec le Microsoft Store doit être observé sur une réservation où le pack est absent.
Décision : arrêter la recette dès que la revérification réussit et que le bouton de confirmation de l’installation de Whisper devient disponible.

## Sur le bureau du Snapdragon

1. Utiliser une réservation fraîche, avec le pack graphique Microsoft absent pour le compte connecté et sans installation existante dans `%LOCALAPPDATA%\WhisperTurbo`. Extraire entièrement le nouveau ZIP dans un dossier distinct et double-cliquer sur **whisper-bootstrap.exe**. Le dossier **modele** contient déjà le modèle.
2. Quand l’affichage de Whisper échoue, vérifier que l’alerte explique la situation et propose explicitement **Oui : ouvrir le Microsoft Store**. Sur Windows en anglais, le bouton porte le libellé **Yes**. Conserver une capture de cette alerte. Si seule la revérification est proposée, relever le message et arrêter la recette.
3. Cliquer sur **Oui / Yes**. Vérifier que la fiche Microsoft du pack de compatibilité graphique s’ouvre. Installer le pack dans le Store et attendre la fin. Laisser le bootstrap ouvert.
4. Revenir à l’assistant. Cette fois **Oui / Yes** signifie **vérifier à nouveau**. Cliquer dessus. La sonde de Whisper doit réussir et l’assistant proposer d’installer WhisperTurbo. Conserver une capture de cette confirmation. À ce stade **Non / No** permet de terminer la recette sans installer Whisper.

Si le rendu fonctionne dès le premier lancement, consigner ce résultat : l’image ne permet pas d’exercer le scénario du pack manquant. Pour transmettre le résultat, préciser si l’alerte proposait le Store, si le clic a ouvert la bonne fiche et si la confirmation d’installation est apparue après revérification. Aucun import ni essai micro n’est demandé.

## Référence du diagnostic

Le rapport graphique antérieur sur le SC8480XP indique Windows ARM64 et la valeur numérique 12 pour l’architecture du Snapdragon X2 Elite. L’ancien candidat présentait l’action de revérification seule. Sa recette et sa capture sont conservées dans le dépôt, dans la fiche `recette-bootstrap-graphique-snapdragon.md`.

## Identité du nouvel installateur

Archive à extraire : WhisperTurbo-Snapdragon-Store-ARM64.zip. Empreinte SHA-256 de whisper-bootstrap.exe : 6560131dc0bbc6e6bdd45a2728f719db7a6079324dc3fd06b816cd6ca22f21f6.

## Retour du nouveau créneau Snapdragon

Parcours Store en échec. L’opérateur signale avoir utilisé le nouveau paquet. La [capture](preuves-bootstrap-snapdragon/alerte-store-b7-signalee.png) montre uniquement Oui pour réessayer et Non pour quitter. Non ferme bien la fenêtre. L’identité du processus exécuté sur la VM et la valeur d’architecture qu’il a effectivement obtenue restent à établir. La revérification après passage par le Store reste non exercée.

Le contrôle de la fenêtre ouverte sur SC8480XP a identifié le bootstrap du dossier WhisperTurbo-Snapdragon-Store-ARM64 du bureau. Son SHA-256 diffère de B7 (CandidatB7=False). Windows annonce ArchitectureCPU=12 et Snapdragon X2 Elite. L’alerte observée ne qualifie donc pas B7. La version exacte lancée n’a pas été identifiée. Reprendre avec le bootstrap vérifié du ZIP.
