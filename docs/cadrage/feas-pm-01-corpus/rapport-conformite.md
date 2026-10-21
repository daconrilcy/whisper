Avancée : les quatre textes `_copie` ont reçu l'approbation humaine et leurs SHA-256 sont liés aux WAV ; les mots aux frontières listées ont reçu « OK avec le tableau ».
Blocage : l'étape acoustique n'a pas été faite ; aucun bruit seul, pause ou seuil relatif n'est qualifié humainement.
Décision : corpus non qualifié pour FEAS-PM-01 ; l'étude reste interdite.

# Qualification préparatoire du corpus FEAS-PM-01

## Livrables et portée

- [Manifeste](manifest.json) : quatre triplets WAV/JSON/texte, OGG d'origine, membre et SHA-256 de l'archive, plages source et égalité PCM. Les fichiers sources du dossier `sound example/` sont inchangés.
- [Quatre textes approuvés et fiche d'écoute](validation_humaine.md) : les fichiers `_copie.txt` corrigés par l'utilisateur sont les références à retenir ; leurs SHA-256 exacts et les WAV associés sont dans le manifeste. Les anciens `candidate_*.txt` restent des brouillons.
- [Mots horodatés Parakeet](parakeet_word_proposals.json) : hypothèses ASR `AUTO_NON_VALIDE`, issues du worker CPU NeMo-Speech.cpp 0.2.0 déjà présent. Ce n'est pas un alignement forcé sur un texte relu.
- [Annotations automatiques](auto_annotations.json) : bords candidats A/B, mots ASR voisins, pauses exploratoires, énergie acoustique et file de vérification. Aucune stratégie A/B n'a été exécutée.
- [Fiche de validation humaine](validation_humaine.md) et [zones critiques](zones_critiques.md) : écoute intégrale avec texte directement modifiable, puis contrôle ciblé des mots et de l'acoustique. Quatre extraits WAV exacts de 0–30 s et 28–32 s, dérivés des deux continus, facilitent la décision sur le préfixe ; leurs identités et plages figurent dans le manifeste.

## Résultat par prérequis

| Prérequis | État | Preuve et travail restant |
| --- | --- | --- |
| Quatre WAV, JSON et textes initiaux | **PASS** | Présents ; WAV PCM16 mono 16 kHz, deux durées de 60 s et deux d'environ 65 s. |
| Filiation et identité des sources | **PASS local** | Les quatre OGG extraits égalent leurs membres de `original_2020.tar` ; les plages JSON décodées égalent octet pour octet les WAV. SHA-256 et recette dans le manifeste. L'origine externe de l'archive n'a pas d'attestation amont vérifiée. |
| Référence textuelle humaine pour les quatre WAV | **PASS** | L'utilisateur a corrigé les quatre `_copie.txt` et a répondu explicitement « J'approuve les quatre textes » après l'écoute. Le manifeste enregistre date, chemin et SHA-256 exact du texte et du WAV pour chaque cas. Les anciens fichiers nommés `REFERENCE.txt` dans `sound example/` ne sont toujours pas les références retenues. |
| Mots horodatés | **PARTIAL** | Parakeet a fourni 189 mots FR pauses, 185 EN pauses, 227 FR continu et 181 EN continu ; tous provisoires. Le premier mot du FR avec pauses commence à 6,72 s alors que le texte candidat contient de la parole initiale : la zone 0–7 s manque dans ces temps ASR et exige un contrôle humain. Les mots non reconnus et les temps proches des coupes restent à corriger. |
| Préfixes exacts 0–30 s des deux continus | **PASS pour la frontière lexicale ; temps ASR provisoires** | L'utilisateur a confirmé que `situation` (FR) et `that` (EN) sont acoustiquement tronqués à la coupure. Le manifeste conserve les mots entiers précédents, dérivés des textes `_copie` approuvés, et annote séparément ces deux mots tronqués. Les extraits audio sont coupés à 480 000 échantillons. Les temps Parakeet proches de 30 s ne prévalent pas sur cette décision humaine. |
| Pauses, hésitations vocales et bruit | **PARTIAL** | L'utilisateur a explicitement indiqué ne pas avoir fait cette étape et a demandé de « considérer OK ». Cela évite une nouvelle longue écoute, mais ne constitue pas une étiquette acoustique humaine. FFmpeg détecte à −35 dBFS huit pauses FR et une EN ≥400 ms dans les longs, aucune dans les continus ; ces résultats restent exploratoires. La note FR « 0–6 : pause » ne désigne pas un intervalle non ambigu, puisque 0–4 s est aussi déclaré en anglais. |
| Seuil relatif au bruit pour A | **FAIL pour la qualification** | Aucune plage de bruit seul n'a été approuvée, donc aucun seuil opérationnel figé. Les proxys automatiques des continus (−36,05 et −34,33 dBFS, contre environ −65 dBFS dans les longs) pourraient être de la parole faible et produisent des coupes candidates incompatibles avec la qualification par seuil fixe. |
| Frontières des stratégies A/B | **PARTIAL** | L'utilisateur a répondu « OK avec le tableau » pour les quatre cas et a signalé le texte anglais 0–4 s du FR pauses, absent des temps Parakeet avant 6,72 s. Les mots aux frontières listées sont ainsi acceptés ; leurs bornes ASR exactes restent provisoires. Les coupes A/B n'ont pas été exécutées ; `omis`, `répété` et `altéré` ne seront constatés qu'après une comparaison autorisée. |
| Éligibilité des deux passages continus de 60 s et de leurs 30 premières secondes | **PARTIAL** | Absence préliminaire de pause ≥400 ms à −35/−30 dBFS ; une détection relative au bruit non calibrée peut trouver des pauses différentes. L'écoute et le seuil figé doivent confirmer l'absence de pause admissible, ainsi que les textes 0–30 s. |
| Prêt à lancer FEAS-PM-01 | **FAIL** | Les références textuelles sont approuvées, mais le détecteur de pause relatif au bruit et l'éligibilité acoustique des continus ne sont pas qualifiés. Autorisation distincte toujours nécessaire. |

## Calibration reproductible proposée

1. Écouter et désigner dans chaque WAV une plage réellement limitée au bruit de fond, ainsi que des exemples de pause, de reprise, d'hésitation voisée et de bruit transitoire. Si un continu de 60 s n'offre aucune plage de bruit seul, rechercher un court contexte adjacent **dans le même OGG**, puis confirmer à l'écoute qu'il est comparable ; sinon le seuil demeure non qualifié.
2. Calculer l'énergie RMS sur trames de 20 ms et le 95e percentile des trames de bruit seul. Comparer des marges préfixées de +6, +9 et +12 dB au-dessus de ce niveau, avec une pause candidate ≥400 ms et au moins 1 s entre deux coupes. Les valeurs sont une grille de calibration, pas des seuils retenus.
3. Confronter chaque variante aux exemples approuvés : pause détectée, hésitation non coupée, bruit non confondu avec une reprise. Choisir et figer avant FEAS la marge qui respecte ces exemples, avec son SHA-256 des fichiers et les intervalles d'étalonnage. Si aucune marge ne convient, rendre **FAIL** pour la détection de pause de A et demander une autre méthode de détection ; ne pas régler le seuil sur les résultats A/B.

## Interventions humaines restantes

La [fiche d'écoute](validation_humaine.md) rapporte les décisions reçues. Les quatre textes et les mots du tableau de frontières ont été traités. L'utilisateur a dispensé la revue acoustique détaillée ; les étiquettes de pause, hésitation et bruit et le seuil relatif demeurent donc inconnus. Aucun mot individuel supplémentaire n'exige une vérification systématique à ce stade. Une résolution minimale ultérieure du seuil acoustique ou une décision explicite de modifier ce prérequis du protocole sera nécessaire avant une demande d'exécution FEAS-PM-01.

L'alignement forcé indépendant reste différé. Si les corrections des temps Parakeet deviennent trop nombreuses ou si la zone manquante initiale se répète, en mesurer le volume après la première revue humaine et demander une décision distincte sur l'outil ; aucun aligneur ni modèle supplémentaire n'a été installé.
