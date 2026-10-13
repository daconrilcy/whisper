# Bilan — évolution micro direct

**Statut : LIVRÉ AVEC RÉSERVES — 13 octobre 2026.** L'utilisateur a validé le parcours dans la fenêtre Windows. La vitesse du CPU reste la réserve acceptée ; le micro sert de dictée avec retard, sans promesse de temps réel.

## Livré

La capture micro affiche des blocs de texte confirmés pendant la séance. Un clic sur **Arrêter**, ou la fermeture de la fenêtre, termine le traitement de l'audio restant avant de créer un TXT, un SRT et le WAV de la séance, de même nom. La dernière phrase est présente. Le micro et l'import sont exclusifs ; les imports WAV/MP3 du MVP sont préservés. Le modèle local est chargé une seule fois par session. Aucun modèle n'a été téléchargé.

## Réserve de vitesse et essais écartés

Sur le même extrait de référence de 30 s, l'import demandait initialement **46,8 fois** la durée audio. Après optimisation de la compilation native et utilisation de 24 fils, l'import demande **2,25 fois** cette durée (67,6 s de traitement) ; le chemin micro en six blocs de 5 s demande **2,24 fois** (67,3 s). Le seuil d'une minute de calcul pour une minute d'audio n'est pas atteint. Un bloc micro de 5 s prend environ 11 s ; son coût vient surtout de l'encodage et du décodage du modèle, non du chargement répété.

Des blocs plus courts multiplient ce coût presque fixe et n'accélèrent pas la transcription globale. Réduire `audio_ctx` à la durée du bloc accélère le calcul mais altère nettement le texte ; des valeurs intermédiaires changent la ponctuation et restent au-dessus du seuil. Ces réglages n'ont pas été retenus. Le décodage simple était déjà actif.

## Suite possible

La vitesse relève d'une évolution séparée. Options encore ouvertes : **GPU**, version compressée du modèle actuel, ou modèle plus léger. Chacune demandera une décision de périmètre et des mesures propres avant adoption ; aucun téléchargement n'est autorisé par cette clôture.

Compilation release et tests existants réussis ; la validation du parcours dans la fenêtre relève de l'essai final de l'utilisateur. Pas de push.
