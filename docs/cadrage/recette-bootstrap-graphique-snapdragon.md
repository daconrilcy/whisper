Avancée : le candidat graphique Turbo a été compilé et assemblé. Le contrôle du rendu utilise le desktop livré.
Blocage : le parcours Store avant/après reste à observer sur un Snapdragon dont le pack est initialement absent.
Décision : lancer le bootstrap depuis le bureau sur une image fraîche et conserver les observations de la recette.

## Candidat à utiliser

| Fichier | SHA-256 |
|---|---|
| `whisper-bootstrap.exe` | `bd8ce23641f884eb09bd0901393599181691cd4883ae4841f0b3fe9105a23eac` |
| `app/whisper-desktop.exe` | `6602055e1e57092f22c32e3966ea36d723a8e79ff0e67f6a1ab9a7cc82df6ad4` |
| `modele/ggml-large-v3-turbo.bin` | `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` |

Le modèle est déjà dans le kit. La sonde ouvre brièvement une fenêtre WhisperTurbo. Sur l’alerte initiale qui propose le Store, **Oui** ouvre la fiche Microsoft, **Non** vérifie à nouveau, **Annuler** quitte. Après ouverture du Store, revenir à la boîte de dialogue et cliquer sur **Oui** pour relancer la vérification après installation du pack. Quand l’installateur annonce que le PC est prêt, **Oui** confirme l’installation de WhisperTurbo.

## Démarrer depuis le bureau

Extraire entièrement l’archive de recette dans un dossier. Ouvrir ce dossier dans l’Explorateur et double-cliquer sur `whisper-bootstrap.exe`. Garder `app`, `notices-candidate` et `modele` à côté de cet exécutable. Le dossier `modele` contient le fichier `ggml-large-v3-turbo.bin` fourni hors ligne. Utiliser la session graphique du même compte jusqu’à la fin du test.

Avant le premier lancement, ouvrir les applications installées dans les Paramètres Windows et constater l’absence du pack de compatibilité graphique Microsoft. Dans l’Explorateur, ouvrir `%LOCALAPPDATA%` et relever si `WhisperTurbo` existe. Pour tester une première installation utiliser un profil vierge. Conserver une capture de l’état initial. Le test porte sur l’alerte graphique, son annulation et la reprise après installation du pack. Il ne demande aucune nouvelle transcription.

## Sur la réservation Snapdragon

1. Utiliser une image fraîche et le bureau du compte de recette. Relever le build de Windows, le modèle de la machine, la version du pilote graphique et l’état initial du dossier WhisperTurbo. Vérifier en lecture seule que le pack Microsoft.D3DMappingLayers est absent pour ce compte. Si le pack est déjà installé, choisir une autre image. Conserver le modèle hors ligne et le paquet extrait. Écarter le lanceur T42 qui tente de préinstaller le pack.
2. Double-cliquer sur le bootstrap du candidat. Relever le résultat de la sonde et photographier ou capturer la fenêtre. Si le pilote fournit déjà le rendu requis, l’admission graphique est attendue et la branche « rendu manquant » reste à exercer sur une autre image. Si le rendu échoue, vérifier le message en français et les actions Store, revérification et annulation avant toute copie dans WhisperTurbo. L’action d’installation doit être inaccessible.
3. Annuler. Comparer l’état du dossier WhisperTurbo avec l’état initial. Relancer le bootstrap, revérifier avant correction et constater que l’installation reste bloquée. Cliquer sur l’action Store, vérifier la fiche Microsoft ouverte et installer le pack dans le même compte. Relever son identité et sa version.
4. Revenir au bootstrap resté ouvert. Revérifier le rendu et constater le passage à l’état prêt après validation des autres entrées. Choisir le modèle fourni si demandé, confirmer l’installation puis ouvrir le desktop installé. Vérifier que sa fenêtre est utilisable.
5. Rattacher au même essai les empreintes du candidat, les résultats de sonde avant/après, les observations de l’opérateur, l’état avant/après annulation et les preuves de démarrage. Une alerte jamais observée ou une installation anticipée du pack laisse la branche essentielle non qualifiée.

## Contrôles complémentaires

Sur le Dell sans NVIDIA, vérifier que le CPU reste admissible lorsque le rendu et les ressources conviennent. Sur une copie de recette dédiée, présenter un modèle incorrect puis un modèle valide, et vérifier le diagnostic d’un fichier de paquet absent. Consigner séparément les résultats de chaque parcours. [Critères acceptés](prerequis-graphiques-installation-turbo.md)
