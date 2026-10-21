# FEAS-PM-01 — validation humaine du corpus

Statut : **CANDIDAT**. Aucun texte, mot horodaté ou seuil acoustique n’est encore approuvé.

## Mode opératoire

1. Ouvrir chaque WAV indiqué ci-dessous et écouter sa durée entière en suivant le texte candidat lié. Corriger le texte si nécessaire, y compris les mots tronqués hors WAV, les hésitations réellement audibles et les noms/nombres. Ne pas déduire les mots du seul ASR.
2. Rejouer les 2 premières et 2 dernières secondes. Pour les continus, écouter en plus 28–32 s et fixer le texte exact de 0–30 s : un mot traversant 30 s appartient au préfixe seulement s’il est audible avant la coupure selon la convention retenue.
3. Écouter les coupes A et les bords B signalés ci-dessous, en lisant les mots attendus. Marquer chaque mot proche `entier`, `coupé`, `incertain` ou corriger son temps ; vérifier aussi silence réel, hésitation voisée et bruit sur les pauses candidates. Les listes de temps sont des suggestions automatiques.
4. Pour chaque enregistrement, déclarer explicitement « approuvé tel quel » ou « corrigé et approuvé », en indiquant l’identifiant du fichier, la date, la convention de transcription et le nom de la personne. Le SHA-256 du WAV et du texte final est alors calculé et inscrit dans l’attestation ; l’absence de correction ne vaut jamais validation tacite.
5. Ne créer un `REFERENCE.txt` approuvé et des temps `VALIDÉ_HUMAIN` qu’après cette décision explicite. Si le texte change, recalculer les zones dépendantes et réexaminer les mots concernés.

Convention proposée à valider : conserver les mots réellement audibles, les répétitions et les interjections lexicales ; signaler séparément les hésitations vocales sans mot lexical, les bruits et l’inaudible ; ne pas restituer un mot coupé hors de l’extrait. Ponctuation et casse peuvent être normalisées sans inventer de contenu.

## Fiches d’écoute

### fr_pauses

- Audio : `sound example/voxpopuli/clips/long_examples/fr_long_2020.wav` (65.02 s ; SHA-256 `541285a38fb6927c1bb4384825d2a7565a4a41efeade91c5f13de60bfefc5c0e`).
- Texte à relire : [`candidate_fr_pauses.txt`](candidate_fr_pauses.txt) ; SHA-256 initial `9db2295a9c02df03db8cea42cdd3743f4bde36c63299ef9e65a282687ed952e3`. Statut **CANDIDAT**.
- Écoute prioritaire : 0–7 s ; 63.02–65.02 s. Parakeet ne propose aucun mot avant 6,72 s alors que le texte candidat commence dès l’ouverture : vérifier et horodater humainement cette portion.
- A, coupes forcées candidates : 8, 16, 24, 32, 40, 48, 56, 64 s. Elles ne représentent pas les coupes effectives d’un essai.
- B, bords de cadence où un mot ASR traverse le temps proposé : 8, 10, 14, 18, 22, 26, 28, 30, 32, 34, 40, 42, 44, 46, 48, 50, 52, 54, 58, 62, 64 s. Vérifier ces mots à l’écoute ; les autres bords sont consignés dans `auto_annotations.json`.
- Pauses fixes exploratoires −35 dBFS : 4.74–6.43, 12.88–13.41, 26.88–27.46, 32.39–32.80, 38.34–38.98, 49.51–50.01, 58.23–58.65, 63.39–63.89. Ne pas les approuver par défaut.

Décision humaine : **EN ATTENTE**. Personne : **À RENSEIGNER**. Date : **À RENSEIGNER**. Texte final et SHA-256 : **À RENSEIGNER**.


### en_pauses

- Audio : `sound example/voxpopuli/clips/long_examples/en_long_2020.wav` (65.00 s ; SHA-256 `d73e3c08972ccb39eb3fc70ffb202c95c2b386fb3e59b5fafb753dab2c2ec18a`).
- Texte à relire : [`candidate_en_pauses.txt`](candidate_en_pauses.txt) ; SHA-256 initial `c349f550b76ab3f1b6071cea80d61a09fa2a83578f6208c8263309d4a6d4a8c7`. Statut **CANDIDAT**.
- Écoute prioritaire : 0–2 s ; 63.00–65.00 s.
- A, coupes forcées candidates : 8, 16, 24, 32, 40, 48, 56, 64 s. Elles ne représentent pas les coupes effectives d’un essai.
- B, bords de cadence où un mot ASR traverse le temps proposé : 4, 6, 10, 12, 14, 16, 18, 20, 24, 28, 30, 32, 34, 36, 38, 40, 44, 48, 50, 52, 58, 60 s. Vérifier ces mots à l’écoute ; les autres bords sont consignés dans `auto_annotations.json`.
- Pauses fixes exploratoires −35 dBFS : 12.21–12.67. Ne pas les approuver par défaut.

Décision humaine : **EN ATTENTE**. Personne : **À RENSEIGNER**. Date : **À RENSEIGNER**. Texte final et SHA-256 : **À RENSEIGNER**.


### fr_continu

- Audio : `sound example/voxpopuli/clips/long_examples/fr_continuous60_2020_CANDIDATE.wav` (60.00 s ; SHA-256 `322757782eda5332bdec4ff8bc7bbd87a5bbb76330d0d4da7f6e4d271f5a0df2`).
- Texte à relire : [`candidate_fr_continu.txt`](candidate_fr_continu.txt) ; SHA-256 initial `ad13e548f22aa163449009fe4beab64e3d93467fc902d9e0bfca9a2607e55dc4`. Statut **CANDIDAT**.
- Écoute prioritaire : 0–2 s ; 58.00–60.00 s ; 28–32 s et coupure exacte 30 s.
- A, coupes forcées candidates : 8, 16, 24, 32, 40, 48, 56 s. Elles ne représentent pas les coupes effectives d’un essai.
- B, bords de cadence où un mot ASR traverse le temps proposé : 4, 6, 8, 10, 12, 16, 22, 24, 28, 32, 36, 38, 40, 44, 46, 50, 52, 56, 58 s. Vérifier ces mots à l’écoute ; les autres bords sont consignés dans `auto_annotations.json`.
- Pauses fixes exploratoires −35 dBFS : aucune ≥400 ms. Ne pas les approuver par défaut.

Préfixe 0–30 s proposé par Parakeet, **AUTO_NON_VALIDÉ** :

> J'observerai que depuis ce nouveau cycle institutionnel, il y a quelques mois seulement, quelques choix clairs ont été posés. La Chine, par exemple. Moi je me souviens d'avoir constaté pendant longtemps que l'Europe était totalement divisée. Comment aborder la relation avec la Chine? En quelques mois, grâce à un travail politique, un dialogue entre nous, nous avons aligné une position claire, stratégique, et nous exprimons, cher M. Lambert, avec clarté le message. Quand je m'adresse au président Xi, je ne baisse pas les yeux. S'agissant de valeur fondamentale, les droits de l'homme. Je parle des Ouïghours, je parle de la situation à

Texte du préfixe approuvé après écoute : **À RENSEIGNER**.

Décision humaine : **EN ATTENTE**. Personne : **À RENSEIGNER**. Date : **À RENSEIGNER**. Texte final et SHA-256 : **À RENSEIGNER**.


### en_continu

- Audio : `sound example/voxpopuli/clips/long_examples/en_continuous60_2020_CANDIDATE.wav` (60.00 s ; SHA-256 `1fdcf0d8c00c426c7ed78c6724087db857b3acde465ea1f8bd77a3be8ca4b7c8`).
- Texte à relire : [`candidate_en_continu.txt`](candidate_en_continu.txt) ; SHA-256 initial `e570618d32aaf2dbebc0a6b1e754ba684eec68a5310a8b713a8b0d109e06ee37`. Statut **CANDIDAT**.
- Écoute prioritaire : 0–2 s ; 58.00–60.00 s ; 28–32 s et coupure exacte 30 s.
- A, coupes forcées candidates : 8, 16, 24, 32, 40, 48, 56 s. Elles ne représentent pas les coupes effectives d’un essai.
- B, bords de cadence où un mot ASR traverse le temps proposé : 2, 4, 8, 12, 16, 20, 22, 24, 28, 32, 34, 36, 38, 40, 42, 46, 48, 52, 54, 58 s. Vérifier ces mots à l’écoute ; les autres bords sont consignés dans `auto_annotations.json`.
- Pauses fixes exploratoires −35 dBFS : aucune ≥400 ms. Ne pas les approuver par défaut.

Préfixe 0–30 s proposé par Parakeet, **AUTO_NON_VALIDÉ** :

> This is why we Socialists and Democrats urge Member States to stop considering public services as a cost, but at the contrary as an investment to guarantee a decent life for all of us in the Union. And a decent life means well being that includes access to quality education and jobs, as well as social security, health care, housing, justice and social services, and also especially for vulnerable groups. To achieve this, it's important to change the mindset of many EU leaders and to come forward with a model that

Texte du préfixe approuvé après écoute : **À RENSEIGNER**.

Décision humaine : **EN ATTENTE**. Personne : **À RENSEIGNER**. Date : **À RENSEIGNER**. Texte final et SHA-256 : **À RENSEIGNER**.
