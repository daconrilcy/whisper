Avancée : la campagne temporelle complète sur les extraits disponibles passe avec la borne d'une trame ; la passe unique FR atteint 87,65 s sur CPU et CUDA0.
Blocage : aucun corpus FR/EN de six fichiers de 3 à 5 min ni source de 2 à 20 min n'est disponible pour établir la limite maximale et qualifier la découpe longue.
Décision : point 2 GO dans la portée testée ; point 3 NO-GO faute de preuve, donc GO global non acquis. Les points 1 et 4 restent acquis sans nouvel essai.

# Parakeet FR/EN — critères révisés du 15 octobre 2026

Cette campagne prolonge le [bilan initial](bilan-parakeet-fr-en-go.md) uniquement pour les points 2 et 3. L'utilisateur a demandé de ne pas tester des WAV manquants ; aucun audio long n'a été fabriqué par répétition. Les scripts et résultats bruts sont dans `target/parakeet-feasibility/spike02/`, sans modification du code produit.

| Point et critère révisé | Résultat | Statut |
| --- | --- | --- |
| **1. Worker/DLL**, acquis | Résultat antérieur conservé : 20/20 inférences CPU et 20/20 CUDA0, FR/EN, hôte intact, worker sans `FreeLibrary`. Aucun changement ni nouvel essai de ce point. | **GO acquis** |
| **2. Horodatages** : dépassement brut ≤80 ms, puis aucune violation après bornage du dernier mot | Le mot fautif est **le dernier mot du fichier**, `fr27.wav` en passe entière : `à...`, **26 720–27 040 ms** pour 27 000 ms d'audio sur CPU et CUDA0. Le passage antérieur à 27 120 ms venait du padding explicite supplémentaire du banc ; la nouvelle campagne transmet le PCM réel au moteur, qui gère ses propres trames de 80 ms. Le worker borne seulement `end` du dernier mot final à 27 000 ms lorsque le dépassement est ≤80 ms. Sur **7 extraits × entier/fenêtres × CPU/CUDA0** : 14 parcours, 41 appels audio par calcul ; **0 dépassement >80 ms, 0 mot dépassant une frontière interne, 0 violation après borne, 1 borne appliquée** par calcul (`timing_summary.json`, `timing_*.stdout.log`). Les fenêtres utilisent l'offset cumulé en échantillons ; aucun index de trame n'est utilisé comme offset. | **GO sur les extraits disponibles** |
| **3A. Passe unique** : déterminer la limite CPU/CUDA0 aux paliers 1, 2, 5, 10, 20 min | Le palier **1 min FR** réussit sur les deux calculs. La plus longue passe disponible, **87,65 s FR**, réussit aussi. L'anglais fourni dure **15,151 s**. Les paliers 2/5/10/20 min et les longs fichiers EN sont **NOT RUN**, faute de sources. Temps, facteur et mémoire observés ci-dessous. Aucune durée maximale de passe unique n'est trouvée. | **NO-GO pour fixer une limite** |
| **3B. VAD long** : seulement au-delà de la limite A, segments 60–120 s, silence ≥400 ms, marge 200 ms | **NOT RUN** : la limite A est inconnue et aucun fichier plus long que cette limite n'est disponible. La précédente VAD sur des segments courts ne qualifie pas ce nouveau contrat. | **NO-GO / non qualifié** |
| **3C. Chevauchement éventuel** : fusion au milieu selon le centre temporel des mots, zéro doublon | Le banc précédent appliquait déjà cette règle avec 2 s de chevauchement (fenêtres FR de 20 s et EN de 10 s). Il a observé des doublons « article » en FR et « life » en EN, ainsi que des écarts de 6,99–8,06 % FR et 2,56 % EN à la passe entière. Cette preuve **ne satisfait pas** zéro doublon ; aucun nouvel essai sur le corpus long absent n'a été fait. Le chevauchement n'est pas retenu comme solution qualifiée. | **NO-GO** |
| **4. Langue**, acquis | Résultat antérieur conservé : FR/EN sans langue imposée, identifiant de langue non requis. Aucun changement ni nouvel essai de ce point. | **GO acquis** |

## Passe unique : observations, pas limite maximale

Le temps de calcul est chronométré dans le processus autour de l'inférence, hors lecture WAV ; le facteur temps réel vaut durée audio / temps de calcul. La RAM est le pic `PeakWorkingSetSize` observé pour le worker Windows. Sous WDDM, `nvidia-smi` renvoie `[N/A]` pour la VRAM par processus : la colonne GPU est le **pic global de la carte moins son niveau avant l'essai**, pouvant inclure d'autres applications ; ce n'est pas la VRAM attribuée exactement à Parakeet.

| Calcul | Audio disponible | Temps de calcul | Facteur temps réel | RAM worker au pic | VRAM GPU, delta global observé |
| --- | ---: | ---: | ---: | ---: | ---: |
| CPU | EN 15,151 s | 0,778 s | 19,48× | 806,6 Mio | — |
| CPU | FR 60 s | 2,884 s | 20,80× | 1 122,7 Mio | — |
| CPU | FR 87,65 s | 4,656 s | 18,83× | 1 318,2 Mio | — |
| CUDA0 / GTX 1080 Ti | EN 15,151 s | 0,384 s | 39,45× | 316,4 Mio | 1 014 Mio |
| CUDA0 / GTX 1080 Ti | FR 60 s | 0,720 s | 83,36× | 319,9 Mio | 1 316 Mio |
| CUDA0 / GTX 1080 Ti | FR 87,65 s | 0,912 s | 96,15× | 324,0 Mio | 1 538 Mio |

**Limite de durée en passe unique : inconnue pour CPU et CUDA0.** La plus longue durée **réussie et observée** est 87,65 s en français sur les deux ; en anglais, seulement 15,151 s. Aucun de ces succès n'établit un maximum ni une garantie sur l'autre langue. Le corpus requis de **trois fichiers FR et trois fichiers EN distincts de 3 à 5 min** manque ; le seuil d'écart ≤2 %, zéro doublon et zéro mot coupé n'est pas qualifié pour le nouveau parcours long.

## Proposition de design conditionnelle

Orienter les fichiers vers une **passe unique tant qu'ils restent sous une limite mesurée sur CPU et CUDA0**, puis vers des **segments VAD de 60–120 s**, silence ≥400 ms et marge 200 ms, au-delà. La valeur de cette limite ne peut pas être fixée à partir des sources présentes. Le chevauchement reste écarté tant que sa fusion produit des doublons. Cette proposition n'ouvre ni `DESIGN READY` ni l'implémentation produit. Les 23 autres langues et le paquet final restent hors de cette qualification.
