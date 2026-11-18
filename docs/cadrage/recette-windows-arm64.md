# Recette Whisper sur Windows ARM64

Avancée : candidat x64, modèle et audio réunis pour une recette manuelle sur Windows ARM64.
Blocage : sur la machine ARM64, le démarrage de la fenêtre échoue avec une erreur OpenGL ; la qualification CI ARM64 reste ouverte.
Décision : tester le même candidat sous émulation x64, sans annoncer de version ARM native.

## Si Whisper ne s'ouvre pas : lancer le diagnostic

Si l'installation a déjà été tentée, commencez par ce diagnostic : il ne réinstalle rien et ne supprime aucun fichier.

1. Extrayez cette nouvelle archive et ouvrez le dossier extrait qui contient **DIAGNOSTIC.ps1**. Dans la barre d'adresse de l'explorateur, tapez `powershell` et appuyez sur Entrée.
2. Copiez la commande ci-dessous dans la fenêtre PowerShell puis appuyez sur Entrée :

   ```powershell
   powershell.exe -NoProfile -ExecutionPolicy Bypass -File ".\DIAGNOSTIC.ps1"
   ```

   Le paramètre `ExecutionPolicy` vaut seulement pour ce lancement de PowerShell ; il ne modifie pas le réglage permanent de Windows.
3. Attendez la vérification des fichiers et les 15 secondes d'observation du lancement. Ne fermez pas une éventuelle fenêtre d'erreur avant d'en noter le message.
4. Le programme indique un dossier `diagnostic-...` contenant `rapport.txt` et, si le desktop a été lancé, ses fichiers `desktop-stdout.txt` et `desktop-stderr.txt`. Communiquez ces fichiers et tout message affiché. Le rapport contient des informations Windows et des chemins de fichiers ; vous pouvez les consulter avant de les transmettre.

Le diagnostic contrôle l'identité du paquet et de l'installation, relève le code de sortie si le desktop se ferme et indique s'il est encore actif après 15 secondes. Il ne tue aucun processus. Si un desktop Whisper est déjà actif, il ne lance pas de seconde instance.

## 1. Extraire et installer

1. Copiez l'archive sur le PC Windows ARM64, puis faites clic droit sur l'archive et choisissez **Extraire tout**. Prévoyez au moins 6 Gio libres pour l'archive extraite et l'installation.
2. Dans le dossier extrait, ouvrez le dossier **paquet**. Dans la barre d'adresse de l'explorateur, tapez `powershell` puis appuyez sur Entrée. Une fenêtre PowerShell s'ouvre dans ce dossier.
3. Copiez cette ligne entière dans PowerShell et appuyez sur Entrée :

   ```powershell
   @((Resolve-Path '..\modele\ggml-large-v3-turbo.bin').Path,'o') | .\whisper-bootstrap.exe
   ```

   Le programme vérifie la compatibilité CPU, les fichiers du paquet et le modèle, puis installe Whisper dans `%LOCALAPPDATA%\WhisperTurbo`. Si ce dossier existe déjà, il refuse de l'écraser : ne le supprimez pas, notez le message.
4. Si PowerShell affiche **Installation terminée**, copiez cette ligne pour ouvrir Whisper :

   ```powershell
   & "$env:LOCALAPPDATA\WhisperTurbo\app\whisper-desktop.exe"
   ```

Si une étape échoue, conservez la fenêtre PowerShell ouverte et communiquez le message exact. Si le contrôle CPU refuse ou ne peut pas déterminer la compatibilité, ne cherchez pas à le contourner.

## 2. Tester un fichier

1. Dans Whisper, vérifiez que le modèle **Whisper large-v3-turbo** est disponible et sélectionnez **CPU**.
2. Importez `fr_passage_28-32.wav` depuis le dossier **audio** extrait, choisissez un dossier de sortie et cliquez sur **Traiter**.
3. Attendez la fin. Vérifiez que le texte est lisible et que des fichiers **TXT** et **SRT** sont créés. Notez tout message et, si possible, le temps écoulé.
4. Répétez avec **Auto**. Relevez le moteur affiché et vérifiez le texte ainsi que les fichiers TXT et SRT. Sans GPU NVIDIA utilisable, le moteur attendu est CPU.

## 3. Tester le microphone si possible

Sélectionnez **CPU**, démarrez le microphone, prononcez une phrase, faites une courte pause puis dites une phrase finale. Vérifiez qu'un bloc confirmé apparaît avant l'arrêt. À l'arrêt, vérifiez que les fichiers **WAV, TXT et SRT** sont créés et que la phrase finale figure dans le texte. Vous pouvez répéter en **Auto** si le temps le permet.

## 4. Résultats à communiquer

Indiquez : version et édition de Windows ; résultat de l'installation ; ouverture de la fenêtre ; moteur affiché en CPU et en Auto ; résultat de l'import WAV, présence de TXT/SRT et qualité du texte ; résultat du micro si testé ; messages d'erreur éventuels. Une capture des informations système et une capture de Whisper peuvent aider à comprendre un échec.

Cette recette manuelle vérifie le fonctionnement observé sur votre machine. Elle ne remplace pas à elle seule le job CI T42 ni la preuve technique des processus x64 sous émulation prévus par le plan. Le candidat n'est pas encore qualifié pour distribution.

## Retour de la première recette ARM64

L'utilisateur rapporte que l'application ne reste pas ouverte et que le diagnostic affiche `Error OpenGL(PainterError("egui_glow requires opengl 2.0+. "))`. Le desktop retenu utilise explicitement le moteur `eframe::Renderer::Glow` ; le code `egui_glow` 0.35.0 émet ce message lorsque le contexte OpenGL obtenu annonce une version majeure inférieure à 2. Le pilote, le contexte graphique et le mode de connexion de cette machine restent à identifier ; l'erreur ne permet pas à elle seule de déterminer pourquoi cette version est exposée. Le démarrage desktop et l'inférence sur ce PC sont en échec / non exécutés ; T42 reste `NOT RUN`. Aucun changement de pilote ni de moteur de rendu n'a été effectué.
