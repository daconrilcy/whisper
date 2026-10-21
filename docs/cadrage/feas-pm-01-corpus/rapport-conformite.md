Avancée : les quatre textes `_copie` sont approuvés ; huit zones acoustiques ciblées sont maintenant étiquetées humainement et liées aux SHA-256 des sources.
Blocage : aucune plage de bruit seul n'est certifiée dans les continus ; la grille de seuils relatifs manque une vraie pause FR et coupe au moins deux zones sans pause.
Décision : détecteur de pauses non qualifié pour FEAS-PM-01 ; l'étude reste interdite. L'évaluation préparatoire du VAD existant conclut PARTIAL.

# Qualification préparatoire du corpus FEAS-PM-01

## Livrables et portée

- [Manifeste](manifest.json) : quatre triplets WAV/JSON/texte, OGG d'origine, membre et SHA-256 de l'archive, plages source et égalité PCM. Les fichiers sources du dossier `sound example/` sont inchangés.
- [Quatre textes approuvés et fiche d'écoute](validation_humaine.md) : les fichiers `_copie.txt` corrigés par l'utilisateur sont les références à retenir ; leurs SHA-256 exacts et les WAV associés sont dans le manifeste. Les anciens `candidate_*.txt` restent des brouillons.
- [Mots horodatés Parakeet](parakeet_word_proposals.json) : hypothèses ASR `AUTO_NON_VALIDE`, issues du worker CPU NeMo-Speech.cpp 0.2.0 déjà présent. Ce n'est pas un alignement forcé sur un texte relu.
- [Annotations automatiques](auto_annotations.json) : bords candidats A/B, mots ASR voisins, pauses exploratoires, énergie acoustique et file de vérification. Aucune stratégie A/B n'a été exécutée.
- [Fiche de validation humaine](validation_humaine.md) et [zones critiques](zones_critiques.md) : écoute intégrale avec texte directement modifiable, puis contrôle ciblé des mots et de l'acoustique. Quatre extraits WAV exacts de 0–30 s et 28–32 s, dérivés des deux continus, facilitent la décision sur le préfixe ; leurs identités et plages figurent dans le manifeste.
- [Revue acoustique ciblée](acoustic-review.md) : huit extraits de 15,6 s cumulées, leurs étiquettes humaines, le balayage des marges RMS +6/+9/+12 dB et la confirmation du début anglophone du WAV FR.
- [Étude préparatoire du VAD](vad-prep-report.md) : 48 passages des WAV complets, décisions brutes et coupes candidates ; `PARTIAL` car E1 n'est pas détectée par une règle stricte de 400 ms. Le protocole FEAS-PM-01 demeure inchangé.

## Résultat par prérequis

| Prérequis | État | Preuve et travail restant |
| --- | --- | --- |
| Quatre WAV, JSON et textes initiaux | **PASS** | Présents ; WAV PCM16 mono 16 kHz, deux durées de 60 s et deux d'environ 65 s. |
| Filiation et identité des sources | **PASS local** | Les quatre OGG extraits égalent leurs membres de `original_2020.tar` ; les plages JSON décodées égalent octet pour octet les WAV. SHA-256 et recette dans le manifeste. L'origine externe de l'archive n'a pas d'attestation amont vérifiée. |
| Référence textuelle humaine pour les quatre WAV | **PASS** | L'utilisateur a corrigé les quatre `_copie.txt` et a répondu explicitement « J'approuve les quatre textes » après l'écoute. Le manifeste enregistre date, chemin et SHA-256 exact du texte et du WAV pour chaque cas. Les anciens fichiers nommés `REFERENCE.txt` dans `sound example/` ne sont toujours pas les références retenues. |
| Mots horodatés | **PARTIAL** | Parakeet a fourni 189 mots FR pauses, 185 EN pauses, 227 FR continu et 181 EN continu ; tous provisoires. L'écoute F1 confirme l'anglais initial et le début du français vers 6,5 s ; les temps ASR commencent seulement à 6,72 s et n'alignent pas les mots anglais initiaux. Les mots non reconnus et les temps proches des coupes restent provisoires. |
| Préfixes exacts 0–30 s des deux continus | **PASS pour la frontière lexicale ; temps ASR provisoires** | L'utilisateur a confirmé que `situation` (FR) et `that` (EN) sont acoustiquement tronqués à la coupure. Le manifeste conserve les mots entiers précédents, dérivés des textes `_copie` approuvés, et annote séparément ces deux mots tronqués. Les extraits audio sont coupés à 480 000 échantillons. Les temps Parakeet proches de 30 s ne prévalent pas sur cette décision humaine. |
| Pauses, hésitations vocales et bruit | **PARTIAL** | Huit zones ciblées ont été écoutées et étiquetées explicitement. F1, F2 et E1 contiennent de vraies pauses ; FC2 et EC3 n'en contiennent pas ; FC1, EC1 et EC2 contiennent de courtes respirations ou micro-pauses dont l'admissibilité à 400 ms n'est pas établie. Les autres zones des 60 s n'ont pas d'étiquette humaine. Aucune plage de bruit seul des continus n'est certifiée. |
| Seuil relatif au bruit pour A | **FAIL pour la qualification** | Aucune base de bruit seul n'est validée dans les continus. Avec les bases automatiques, la grille +6/+9/+12 dB manque la vraie pause FR F2 ; +9 dB coupe FC2 (emphase sans pause) et +12 dB coupe EC3 (sans pause). Les niveaux des zones les plus calmes des continus (−36,05 et −34,33 dBFS au percentile 95 de 200 ms) ne constituent donc pas un étalonnage confirmé. Aucun seuil n'est retenu. |
| Frontières des stratégies A/B | **PARTIAL** | L'utilisateur a répondu « OK avec le tableau » pour les quatre cas et confirmé à l'écoute l'anglais initial du FR pauses. Les mots aux frontières listées sont acceptés ; leurs bornes ASR exactes restent provisoires. Les coupes A/B n'ont pas été exécutées ; `omis`, `répété` et `altéré` ne seront constatés qu'après une comparaison autorisée. |
| Éligibilité des deux passages continus de 60 s et de leurs 30 premières secondes | **PARTIAL** | Les textes et frontières à 30 s sont approuvés ; les clips ciblés ont été écoutés. La grille relative produit des coupes dans des zones sans pause ; les autres coupes des 60 s ne sont pas toutes étiquetées. L'absence de pause admissible sur toute la minute et les 30 premières secondes n'est donc pas démontrée avec un détecteur qualifié. |
| Prêt à lancer FEAS-PM-01 | **FAIL** | Les références textuelles et les huit étiquettes acoustiques ciblées sont approuvées. L'étude VAD couvre les 60 s complets sans coupe ≥400 ms, mais manque E1 et ne qualifie aucun profil ; le protocole reste inchangé. Autorisation distincte toujours nécessaire. |

## Calibration reproductible examinée

1. Le calcul RMS par trames de 20 ms, le 95e percentile d'une plage candidate de bruit, les marges +6/+9/+12 dB, la durée minimale de 400 ms et l'espacement de 1 s ont été fixés pour la vérification préparatoire. Les huit extraits et leurs SHA-256 sont dans le manifeste ; les valeurs et les intervalles détectés sont dans la [revue acoustique](acoustic-review.md).
2. Les plages les plus calmes des continus ne sont pas certifiées « bruit seul ». Avec ces bases provisoires, les marges testées manquent F2 et coupent des zones sans pause humainement étiquetées. Aucun seuil relatif n'est donc figé. Un seuil fixe −35 dBFS donne zéro coupe RMS sur les continus, mais ne satisfait pas le prérequis de calibration relative au bruit.
3. Le VAD déjà présent dans les adaptateurs Rust a été [évalué hors produit](vad-prep-report.md) sur les quatre WAV complets, sans installation. Le résultat `PARTIAL` ne qualifie ni une nouvelle règle de coupe, ni son raccordement à Parakeet, ni FEAS-PM-01.

## Interventions humaines restantes

La [fiche d'écoute](validation_humaine.md) rapporte la validation des textes et des frontières ; la [revue acoustique](acoustic-review.md) rapporte les huit étiquettes nouvelles. Aucune nouvelle écoute intégrale ni vérification de tous les mots horodatés n'est demandée. L'[étude VAD](vad-prep-report.md) n'a proposé aucune coupe sur les deux minutes continues, mais E1 reste manquée par la règle stricte. Une règle de coupe validée et une décision distincte sont nécessaires avant FEAS-PM-01.

L'alignement forcé indépendant reste différé. Si les corrections des temps Parakeet deviennent trop nombreuses ou si la zone manquante initiale se répète, en mesurer le volume après la première revue humaine et demander une décision distincte sur l'outil ; aucun aligneur ni modèle supplémentaire n'a été installé.
