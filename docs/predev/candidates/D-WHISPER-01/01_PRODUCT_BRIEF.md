# Brief produit — V1

## Besoin et utilisateur

Application Windows locale à usage personnel sur le PC cible : transcription live progressive depuis le microphone et import WAV/MP3. Les sorties MP3, TXT et SRT sont archivées dans un dossier choisi. L’application démarre avec Windows, est accessible via tray, fournit réglages et raccourci global.

## Cible

Ce PC : Windows 11 Pro x64 build 26200 ; Core i9-14900K ; 95,8 Go RAM ; NVIDIA GTX 1080 Ti ; Intel UHD Graphics 770. Parsec Virtual Display Adapter est virtuel, pas un accélérateur de calcul. L’utilisateur rapporte avoir testé manuellement Whisper sur ce matériel avec succès en CPU et GPU ; il a constaté que le GPU était effectivement utilisé.

## Inclus

- Microphone seulement ; pas d’audio système ni Teams/Zoom ; une capture live au maximum.
- Import demandé pendant live mis en attente jusqu’à la fin du live.
- Modèle téléchargé pendant l’installation pour disponibilité au premier usage.
- Langue manuelle prioritaire, sinon détection automatique.
- Modes Auto, CPU et GPU ; retours explicites sur le mode effectif et les replis.
- Historique reconstruit depuis les fichiers du dossier d’archives choisi ; conservation jusqu’à suppression manuelle.
- File durable d’imports non commencés, séparée de l’historique.

## Exclusions

Pas de capture audio système, pas de multi-live, pas de suppression de source lors du retrait/annulation, pas d’exécution automatique d’import restauré sans choix utilisateur. Aucun plafond de latence ni limite de perte chiffrée n’est accepté à ce stade.
