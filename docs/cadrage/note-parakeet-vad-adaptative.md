Avancée : le seuil RMS fixe 64 est rejeté ; B9 montre zéro silence admissible sur les trois fonds bruités.
Blocage : B5 échoue en passe unique et B6 relève un mot dont l'intervalle traverse une coupure ; la nouvelle VAD n'est pas encore qualifiée.
Décision : proposer une VAD relative au bruit avec un secours déterministe au minimum d'énergie ; aucune implémentation ni activation B8 avant décision utilisateur.

# Note de conception — VAD adaptative Parakeet

## Problème et portée

La VAD actuelle classe chaque trame PCM de 20 ms avec un seuil RMS absolu de 64 et exige 400 ms consécutives sous ce seuil (`parakeet_windows.rs`). B9 donne zéro silence admissible et `NoAdmissibleSilence` sur `spkr0_x12.wav` mêlé à un bruit rose de −40, −30 ou −20 dBFS. Le seuil fixe 64 est **rejeté par décision utilisateur**. La présente note propose le remplacement de la sélection des coupures, sans modifier le code.

Le défaut B5 est distinct : le worker refuse un chevauchement de mots dans la **passe unique** EN60, avant toute VAD. Changer la VAD ne répare pas ce refus. B6 montre un intervalle de mot qui traverse le milieu d'un silence ; un nouveau point de coupure doit être jugé sur les mots réellement émis, pas seulement sur l'énergie.

## Règle proposée

1. Décoder le fichier en PCM mono 16 kHz. Calculer l'énergie RMS de chaque trame de 20 ms en dBFS, avec une borne numérique pour les trames nulles. Mesurer le plancher du fichier par un percentile bas robuste (valeur candidate : **P10**) des trames. Pour éviter qu'une portion initiale exceptionnellement calme fixe tout le seuil, réestimer aussi le plancher dans chaque plage de recherche et retenir la valeur locale lorsqu'elle est plus élevée.
2. Une trame candidate au silence est sous le plancher retenu **plus une marge en dB** (valeur candidate : **+6 dB**). Les valeurs P10 et +6 dB sont des paramètres **à qualifier**, non des constantes produit déjà acceptées. Exiger au moins **400 ms** consécutives candidates. Parmi les silences admissibles, choisir celui dont le centre est le plus proche de la cible de 90 s dans la plage normale de **60 à 120 s**. Préserver les **200 ms de marge contextuelle** de chaque côté et l'attribution d'un mot à l'unique fenêtre contenant son centre.
3. Si aucun silence admissible n'est présent jusqu'à 120 s, poursuivre la recherche selon la règle déjà décidée, au plus jusqu'à la **limite native de 30 min**, marges déduites. Le centre de coupure maximal est donc à 29 min 59,8 s pour le premier segment (marge de sortie de 200 ms) et à 29 min 59,6 s pour les suivants (marges d'entrée et de sortie). Si aucun silence n'est trouvé dans cette plage autorisée, **ne pas produire `NoAdmissibleSilence`** : calculer l'énergie moyenne de chaque fenêtre glissante de 400 ms dont le centre est dans cette plage et couper au centre de la fenêtre de plus faible énergie. À égalité, choisir d'abord le point le plus proche de 90 s, puis le plus précoce. Cette branche de secours est signalée dans la recette avec le niveau d'énergie et sa distance à la cible.
4. Avant admission produit, vérifier que chaque segment natif, marges incluses, reste sous la limite de 30 min et que toutes les bornes sont dans l'audio. Une coupe de secours peut tomber dans de la parole continue : les 200 ms de contexte et l'attribution unique limitent les doublons, mais **ne garantissent pas** à eux seuls l'absence de mot coupé. Tout mot traversant une coupure est rapporté avec son texte, ses bornes, son centre et sa fenêtre propriétaire.

## Recette envisagée

| Corpus | Contrôles attendus |
| --- | --- |
| B5, EN60 synthétique | Mesurer silences, positions de coupure et recours au minimum d'énergie. Comparer VAD et référence seulement si le refus de passe unique B5 a été corrigé ou si une référence valide séparée est explicitement acceptée ; ne pas attribuer à la VAD un échec antérieur à la découpe. |
| B6, FR long synthétique | Mesurer toutes les coupures, leur énergie et les mots à ±2 s. Vérifier zéro doublon, zéro perte de mot et attribution unique ; documenter les intervalles qui traversent le silence, dont `41.` à 262,810 s, avant de fixer un critère mécanique révisé. Un enregistrement FR long réel reste nécessaire pour confirmer la portée. |
| B9, EN60 + bruit rose −40, −30, −20 dBFS | À chaque niveau : plancher global/local, seuil calculé, nombre de silences ≥400 ms, positions des coupures, nombre de secours, énergie aux coupures et absence de `NoAdmissibleSilence`. Comparer la stabilité des mots aux frontières avec la source non bruitée ; signaler toute coupe dans la parole. |

La recette compare le même worker, le même modèle et les mêmes DLL pour les variantes. Elle conserve les sorties et mesures sous le banc isolé. **B8 reste inactive** : le résultat de cette note et les nouvelles mesures seront présentés à l'utilisateur avant tout plan de correction ou activation.
