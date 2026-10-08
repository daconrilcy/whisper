# Réponse utilisateur brute — Q-05

**Acteur :** HUMAN-USER. **Rôle :** user. **Date :** 2026-10-05 (Europe/Paris). **Question :** comportement du dossier de destination lorsqu’il change pendant un job.

**Réponse reçue :** « Oui, confirmer » à la question : « Q-05 — confirmes-tu le snapshot du dossier par job : les jobs actifs gardent leur destination, les suivants utilisent la nouvelle, et rien n’est déplacé ou supprimé automatiquement ? »

**Décision contrôlée :** snapshot du dossier par job ; un job actif conserve sa destination initiale, les jobs démarrés ensuite utilisent le nouveau dossier, et aucun fichier existant n’est déplacé ni supprimé automatiquement. Cette précision complète l’acceptation déjà persistée de suppression explicite de MP3/TXT/SRT après confirmation, sans jamais supprimer la source importée.

**Portée :** Q-05, REQ-20/22/23, lots L-WHISPER-01, L-WHISPER-02 et L-WHISPER-05. Réponse acceptée comme détail réversible ; aucune modification des octets des candidats READY D19/P02.
