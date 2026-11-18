Avancée : procédure préparée pour vérifier le bootstrap graphique du futur candidat Turbo sur Snapdragon.
Blocage : le candidat et ses empreintes doivent être renseignés après implémentation, revue et build gardé.
Décision : démarrer sur une image où le pack Microsoft est absent et observer l’installateur livré.

## Préparation du candidat

Lot proposé sur la branche de reprise. Le bootstrap orchestre les contrôles en lecture seule puis appelle son installation existante après confirmation. Les dialogues Windows en français et la sélection du modèle réutilisent la bibliothèque rfd des adaptateurs. Le desktop propose une sonde du moteur eframe/Glow, appelée sur son image du paquet vérifiée avant exécution. Elle doit initialiser le rendu réel puis quitter, sans charger de modèle, lancer de worker ou créer une installation. La consultation du pack concerne le compte interactif. Un échec de cette consultation doit rester distinguable d’une absence confirmée.

Périmètre proposé : bootstrap (point d’entrée, validation des entrées et tests), adaptateurs (dialogues et consultation du prérequis), desktop (mode sonde et tests), puis adaptation des appels du banc qui alimentent actuellement le bootstrap sur son entrée standard. Conserver les protections de destination et revalider les entrées au moment de l’installation. Avant remise, faire relire le delta, exécuter N1 et Clippy via la garde, construire et identifier les exécutables du candidat puis vérifier le contenu du paquet. Une modification de l’identité du desktop ou du bootstrap doit être répercutée dans la fiche et le pilote de cette recette.

## Sur la réservation Snapdragon

1. Utiliser une image fraîche et le bureau du compte de recette. Relever le build de Windows, le modèle de la machine, la version du pilote graphique et l’état initial du dossier WhisperTurbo. Vérifier en lecture seule que le pack Microsoft.D3DMappingLayers est absent pour ce compte. Si le pack est déjà installé, choisir une autre image. Conserver le modèle hors ligne et le paquet extrait. Écarter le lanceur T42 qui tente de préinstaller le pack.
2. Double-cliquer sur le bootstrap du candidat. Relever le résultat de la sonde et photographier ou capturer la fenêtre. Si le pilote fournit déjà le rendu requis, l’admission graphique est attendue et la branche « rendu manquant » reste à exercer sur une autre image. Si le rendu échoue, vérifier le message en français et les actions Store, revérification et annulation avant toute copie dans WhisperTurbo. L’action d’installation doit être inaccessible.
3. Annuler. Comparer l’état du dossier WhisperTurbo avec l’état initial. Relancer le bootstrap, revérifier avant correction et constater que l’installation reste bloquée. Cliquer sur l’action Store, vérifier la fiche Microsoft ouverte et installer le pack dans le même compte. Relever son identité et sa version.
4. Revenir au bootstrap resté ouvert. Revérifier le rendu et constater le passage à l’état prêt après validation des autres entrées. Choisir le modèle fourni si demandé, confirmer l’installation puis ouvrir le desktop installé. Vérifier que sa fenêtre est utilisable.
5. Rattacher au même essai les empreintes du candidat, les résultats de sonde avant/après, les observations de l’opérateur, l’état avant/après annulation et les preuves de démarrage. Une alerte jamais observée ou une installation anticipée du pack laisse la branche essentielle non qualifiée.

## Contrôles complémentaires

Sur le Dell sans NVIDIA, vérifier que le CPU reste admissible lorsque le rendu et les ressources conviennent. Sur une copie de recette dédiée, présenter un modèle incorrect puis un modèle valide, et vérifier le diagnostic d’un fichier de paquet absent. Consigner séparément les résultats de chaque parcours. [Critères acceptés](prerequis-graphiques-installation-turbo.md)
