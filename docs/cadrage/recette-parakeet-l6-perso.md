Avancée : l'exécutable personnel et ses deux workers Parakeet sont installés sous `%LOCALAPPDATA%\Whisper`.
Blocage : la validation dans la fenêtre Windows et le rollback entre générations distinctes restent à faire.
Décision : essayer l'application publiée avec des fichiers réels et conserver les sauvegardes tant que la recette n'est pas acceptée.

# Recette Windows — installation personnelle L6-perso

Cette procédure porte sur **l'application installée**, distincte du profil Test du dépôt. La [fiche L6-perso](fiche-parakeet-l6-perso.md) fixe les SHA admis et le [bilan](bilan-parakeet-l6-perso.md) sépare les PASS automatisés des deux contrôles encore ouverts.

## Ouvrir et vérifier

1. Fermer toute instance de Whisper puis lancer `%LOCALAPPDATA%\Whisper\app\whisper-desktop.exe` depuis l'Explorateur Windows. Aucun `PARAKEET_*` ni chemin `C:\dev\whisper\target` n'est nécessaire pour cette application personnelle.
2. Dans le profil personnel, vérifier que Turbo et Parakeet sont visibles ; Parakeet doit être sélectionnable pour **Import**, en CPU et en GPU sur cette GTX 1080 Ti. Le microphone conserve Turbo. L'interface ne doit pas qualifier un GPU inconnu ni effectuer un repli silencieux en mode GPU manuel.
3. Importer un fichier français **réel** de quelques minutes avec Parakeet CPU, puis avec Parakeet GPU. Choisir une sortie de TXT/SRT hors de `%LOCALAPPDATA%\Whisper` (par exemple un dossier personnel de transcriptions). Lire le TXT et ouvrir le SRT avec l'audio : mots intelligibles, sous-titres dans l'ordre et sans temps nul ni chevauchement visible. Relever le nom du fichier et tout écart concret.
4. Importer `C:\dev\whisper\sound example\spkr0.wav` (anglais, 5 min) en GPU manuel, puis en Auto. Les deux doivent produire TXT/SRT. En GPU manuel, une panne GPU doit être expliquée, sans substitution CPU ; en Auto, le GPU est privilégié et un seul secours CPU Parakeet est permis avant l'appel natif. La préférence de modèle doit rester Parakeet.
5. Vérifier qu'un audio de plus de 30 min est refusé **avant** création de pending avec « Fichier trop long pour Parakeet, utilisez Turbo ». Vérifier que Turbo reste utilisable pour les modes et parcours déjà présents. Ne pas modifier le modèle, les DLL ou les workers installés pour provoquer cette erreur.

Critère de recette : application ouvrable sans le dépôt, Parakeet CPU/GPU utilisables sur les deux langues, TXT/SRT cohérents, Turbo intact, refus >30 min visible, réglages conservés après fermeture/réouverture. Les tests automatisés ne remplacent pas votre appréciation de la qualité du texte français réel.

## Mise à jour et retour arrière

Le script local `C:\dev\whisper\scripts\install-personal.ps1` n'utilise pas le réseau. Avant une **nouvelle** version, compiler et qualifier ses deux workers exacts et le desktop release selon [PLAN-PARA-L6-PERSO v4](plan-parakeet-l6-perso.md) ; choisir un identifiant inédit et fermer l'application. Dans PowerShell depuis `C:\dev\whisper`, les étapes du script sont :

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\install-personal.ps1 -StageOnly -CandidateId <nouvel-id>
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\install-personal.ps1 -Publish -CandidateId <nouvel-id>
```

La première commande prépare et vérifie `%LOCALAPPDATA%\Whisper\.staging\<nouvel-id>` ; la seconde sauvegarde l'installation active sous `%LOCALAPPDATA%\Whisper\backups\before-<nouvel-id>` avant remplacement. Le script refuse un staging ou backup existant, un SHA inattendu et une application encore ouverte. Après publication, refaire les imports CPU/GPU et la vérification des réglages.

Pour demander le retour à une sauvegarde complète **vérifiée**, application fermée :

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\install-personal.ps1 -RollbackId before-<nouvel-id>
```

Le script vérifie le backup et conserve l'état remplacé dans `backups/`. **Limite actuelle :** l'essai réel L6-perso n'a restauré que C→C ; la restauration d'une autre génération reste NOT RUN. Conserver tous les backups et le dossier transactionnel pending relevé dans le bilan ; aucune suppression n'est prévue par cette recette.
