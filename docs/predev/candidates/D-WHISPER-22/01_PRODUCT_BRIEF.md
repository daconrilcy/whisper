# Brief produit — V1

## Besoin et utilisateur

Application Windows locale à usage personnel sur le PC cible : transcription live progressive depuis le microphone et import WAV/MP3. Les captures live produisent un MP3 par passage et des TXT/SRT cumulés dans un dossier de transcription choisi ; les sources importées restent à leur emplacement et les TXT/SRT associés sont archivés sans copie audio. L’application démarre avec Windows sans fenêtre ni capture, est accessible via tray, fournit réglages et raccourci global. Ces fonctions sont incluses en V1 selon la confirmation utilisateur du 2026-10-05 (`06`).

## Cible

Ce PC : Windows 11 Pro x64 build 26200 ; Core i9-14900K ; 95,8 Go RAM ; NVIDIA GTX 1080 Ti ; Intel UHD Graphics 770. Parsec Virtual Display Adapter est virtuel, pas un accélérateur de calcul. L’utilisateur rapporte avoir testé manuellement Whisper sur ce matériel avec succès en CPU et GPU ; il a constaté que le GPU était effectivement utilisé.

## Inclus

- Microphone seulement ; pas d’audio système ni Teams/Zoom ; une capture live au maximum.
- Import demandé pendant live mis en attente jusqu’à la fin du live.
- Modèle téléchargé pendant l’installation pour disponibilité au premier usage.
- Langue manuelle prioritaire, sinon détection automatique.
- Modes Auto, CPU et GPU ; retours explicites sur le mode effectif et les replis.
- Historique reconstruit depuis les fichiers du dossier d’archives choisi ; conservation jusqu’à suppression manuelle confirmée, qui efface les archives et jamais la source importée.
- File durable d’imports non commencés, séparée de l’historique.
- Stop + Reprise permet un nouveau passage dans la même transcription sur impulsion utilisateur ; un dossier avec MP3 par passage et TXT/SRT cumulés.
- Plusieurs imports pendant live sont traités un par un en ordre de demande après finalisation ; un import interrompu par crash requiert confirmation avant reprise.
- En cas de Quitter bloqué par le moteur/encodeur, rester ouvert jusqu'à résolution ou action manuelle. Les logs locaux sont techniques, sans audio ni texte transcrit.

## Exclusions

Pas de capture audio système, pas de multi-live, pas de suppression de source lors du retrait/annulation, pas d’exécution automatique d’import restauré sans choix utilisateur. Aucun plafond de latence ni limite de perte chiffrée n’est accepté à ce stade.

## Décisions de reprise D10

La validation V1 doit représenter des micros réels FR/EN, en calme et bruit courant (DEC-26). Le live CPU reste disponible avec retard visible et sans promesse de temps réel (DEC-27). Le corpus disponible est limité aux WAV déjà présents (DEC-28) : cette contrainte laisse la validation représentative ouverte. Voir `32_USER_DECISIONS.md`.

## Cible de validation après DEC-34

Pour le gate DESIGN, l'utilisateur accepte les 26 WAV existants comme corpus borné. La cible micro réel FR/EN en calme et bruit courant de DEC-26 devient une qualification ultérieure du produit, non une preuve acquise par D17. Voir `47/48`.
