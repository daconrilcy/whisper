Avancée : quatre sources identifiées et reliées à l'archive ; quatre brouillons de mots horodatés et une liste de frontières produits.
Blocage : aucune transcription, aucun préfixe 0–30 s et aucune pause n'ont été approuvés humainement.
Décision : corpus non qualifié pour FEAS-PM-01 ; l'étude reste interdite.

# Qualification préparatoire du corpus FEAS-PM-01

## Livrables et portée

- [Manifeste](manifest.json) : quatre triplets WAV/JSON/texte, OGG d'origine, membre et SHA-256 de l'archive, plages source et égalité PCM. Les fichiers sources du dossier `sound example/` sont inchangés.
- [Quatre textes candidats et fiche d'écoute](validation_humaine.md) : copies à contenu identique avec fins de ligne LF des textes fournis, avec procédure d'approbation et préfixes de 30 s proposés par Parakeet pour les deux continus.
- [Mots horodatés Parakeet](parakeet_word_proposals.json) : hypothèses ASR `AUTO_NON_VALIDE`, issues du worker CPU NeMo-Speech.cpp 0.2.0 déjà présent. Ce n'est pas un alignement forcé sur un texte relu.
- [Annotations automatiques](auto_annotations.json) : bords candidats A/B, mots ASR voisins, pauses exploratoires, énergie acoustique et file de vérification. Aucune stratégie A/B n'a été exécutée.
- [Fiche de validation humaine](validation_humaine.md) et [zones critiques](zones_critiques.md) : écoute intégrale avec texte directement modifiable, puis contrôle ciblé des mots et de l'acoustique. Quatre extraits WAV exacts de 0–30 s et 28–32 s, dérivés des deux continus, facilitent la décision sur le préfixe ; leurs identités et plages figurent dans le manifeste.

## Résultat par prérequis

| Prérequis | État | Preuve et travail restant |
| --- | --- | --- |
| Quatre WAV, JSON et textes initiaux | **PASS** | Présents ; WAV PCM16 mono 16 kHz, deux durées de 60 s et deux d'environ 65 s. |
| Filiation et identité des sources | **PASS local** | Les quatre OGG extraits égalent leurs membres de `original_2020.tar` ; les plages JSON décodées égalent octet pour octet les WAV. SHA-256 et recette dans le manifeste. L'origine externe de l'archive n'a pas d'attestation amont vérifiée. |
| Référence textuelle humaine pour les quatre WAV | **FAIL** | Quatre textes candidats, zéro attestation humaine. Les deux fichiers déjà nommés `REFERENCE.txt` pour les continus sont identiques aux candidats et n'établissent aucune validation. Écouter chaque WAV entier, corriger puis approuver explicitement le texte lié au SHA-256 du WAV. |
| Mots horodatés | **PARTIAL** | Parakeet a fourni 189 mots FR pauses, 185 EN pauses, 227 FR continu et 181 EN continu ; tous provisoires. Le premier mot du FR avec pauses commence à 6,72 s alors que le texte candidat contient de la parole initiale : la zone 0–7 s manque dans ces temps ASR et exige un contrôle humain. Les mots non reconnus et les temps proches des coupes restent à corriger. |
| Préfixes exacts 0–30 s des deux continus | **PARTIAL** | Audio présent ; propositions ASR dans la fiche. Les segments source traversent la coupure à 30 s : FR 23,009–30,789 s, EN 24,408–34,167 s. L'écoute doit fixer les derniers mots réellement audibles avant 30 s. |
| Pauses, hésitations vocales et bruit | **PARTIAL** | FFmpeg détecte à −35 dBFS huit pauses FR et une EN de durée ≥400 ms dans les longs ; aucune dans les deux continus. Les seuils −35/−30 dBFS sont exploratoires. Les courtes baisses d'énergie et les plages les plus calmes sont des candidats, pas des étiquettes `pause`, `hésitation` ou `bruit`. |
| Seuil relatif au bruit pour A | **PARTIAL** | Procédure de calibration ci-dessous prête ; aucune plage de bruit seul ni pause n'est approuvée, donc aucun seuil opérationnel figé. Le proxy automatique est notamment −36,05 dBFS en FR continu et −34,33 dBFS en EN continu, contre environ −65 dBFS dans les longs : il pourrait représenter de la parole faible et produit de fausses coupes. |
| Frontières des stratégies A/B | **PARTIAL** | Coupes A forcées toutes les 8 s, pauses candidates, bords B toutes les 2 s et mots proches listés. Ces temps décrivent le protocole, pas les coupes effectives d'une exécution. Écouter les mots traversant les bords et corriger les intervalles ; `omis`, `répété` et `altéré` ne seront constatés qu'après une comparaison autorisée. |
| Éligibilité des deux passages continus de 60 s et de leurs 30 premières secondes | **PARTIAL** | Absence préliminaire de pause ≥400 ms à −35/−30 dBFS ; une détection relative au bruit non calibrée peut trouver des pauses différentes. L'écoute et le seuil figé doivent confirmer l'absence de pause admissible, ainsi que les textes 0–30 s. |
| Prêt à lancer FEAS-PM-01 | **FAIL** | Les références humaines, frontières critiques et seuil de pause restent ouverts. Autorisation distincte toujours nécessaire. |

## Calibration reproductible proposée

1. Écouter et désigner dans chaque WAV une plage réellement limitée au bruit de fond, ainsi que des exemples de pause, de reprise, d'hésitation voisée et de bruit transitoire. Si un continu de 60 s n'offre aucune plage de bruit seul, rechercher un court contexte adjacent **dans le même OGG**, puis confirmer à l'écoute qu'il est comparable ; sinon le seuil demeure non qualifié.
2. Calculer l'énergie RMS sur trames de 20 ms et le 95e percentile des trames de bruit seul. Comparer des marges préfixées de +6, +9 et +12 dB au-dessus de ce niveau, avec une pause candidate ≥400 ms et au moins 1 s entre deux coupes. Les valeurs sont une grille de calibration, pas des seuils retenus.
3. Confronter chaque variante aux exemples approuvés : pause détectée, hésitation non coupée, bruit non confondu avec une reprise. Choisir et figer avant FEAS la marge qui respecte ces exemples, avec son SHA-256 des fichiers et les intervalles d'étalonnage. Si aucune marge ne convient, rendre **FAIL** pour la détection de pause de A et demander une autre méthode de détection ; ne pas régler le seuil sur les résultats A/B.

## Interventions humaines restantes

La [fiche d'écoute](validation_humaine.md) regroupe les passages prioritaires. Une écoute complète des quatre WAV reste indispensable pour établir les textes. Les écoutes ciblées portent ensuite sur le début FR 0–7 s, les débuts/fins de chaque fichier, les tranches 28–32 s des continus, les coupes A, les mots ASR traversant les bords B et les plages acoustiques retenues. Une approbation explicite est nécessaire même si aucun mot ne change. Elle doit lier personne, date, convention, SHA-256 du WAV et SHA-256 du texte approuvé ; les corrections de temps de mots et les étiquettes de pause sont approuvées séparément.

L'alignement forcé indépendant reste différé. Si les corrections des temps Parakeet deviennent trop nombreuses ou si la zone manquante initiale se répète, en mesurer le volume après la première revue humaine et demander une décision distincte sur l'outil ; aucun aligneur ni modèle supplémentaire n'a été installé.
