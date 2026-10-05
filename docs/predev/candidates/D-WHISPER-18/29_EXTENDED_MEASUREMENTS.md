# Mesures etendues VAD et delai live - 2026-10-05

## Corpus et methode

Ajout au rapport borne `21_MEASUREMENTS.md`. Source publique : [Google FLEURS](https://huggingface.co/datasets/google/fleurs), licence CC BY 4.0 ; split validation. L'API `first-rows` a fourni les 100 premiers enregistrements FR et EN ; selection deterministe des 6 premiers clips de chaque code de genre 0/1 par langue. Cela donne **24 clips** (12 FR, 12 EN), 112.2 s FR et 97.6 s EN avant pauses. Le corpus ne garantit ni 24 locuteurs distincts, ni les conditions acoustiques du micro utilisateur. IDs, durees, empreintes SHA-256 et recette dans `evidence/fleurs/sample-metadata.json` et les scripts. Les nouveaux fichiers `evidence/fleurs/` ne contiennent ni audio ni texte transcrit. Les journaux historiques de prototype conserves dans `evidence/` contiennent parfois une ligne de transcription du WAV de test ; ils ne sont pas des logs du futur produit. Leurs versions brutes demeurent manifestes dans les candidats precedents.

Les clips source PCM float 16 kHz mono sont decodes en PCM16. VAD WebRTC sur trames de 20 ms, modes 0..3, sous trois conditions : propre, attenuation de 12 dB, et bruit blanc synthetique ajoute a -30 dBFS avec graine fixe. Une seconde de silence numerique encadre chaque clip. Les 288 combinaisons individuelles et les controles silence/bruit sont dans `vad-fleurs-results.json`. La plage de chaque clip est traitee comme **bloc vocal approximatif** ; les pauses internes n'ont pas d'annotation humaine.

## VAD : part mediane du bloc signalee comme parole

| Langue | Condition | Mode 1 | Mode 2 |
|---|---|---:|---:|
| FR | propre | 91.2 % | 88.1 % |
| FR | -12 dB | 89.9 % | 85.2 % |
| FR | bruit -30 dBFS | 98.1 % | 55.0 % |
| EN | propre | 71.4 % | 59.2 % |
| EN | -12 dB | 50.4 % | 15.8 % |
| EN | bruit -30 dBFS | 95.0 % | 1.9 % |

Sur 5 s de bruit blanc seul a -30 dBFS, le mode 1 signale 4,80 s et le mode 2 0,08 s ; sur 5 s de silence numerique, les deux signalent 0. Le mode 2 supprime presque tout ce bruit isole, mais signale une faible part de certains blocs EN attenues ou bruites. Le mode 1 suit davantage les blocs vocaux, tout en confondant le bruit isole avec de la parole. Ces fractions ne sont **ni rappel ni precision VAD**, puisque les paroles et pauses internes ne sont pas annotes. Aucun mode V1 n'est retenu.

## Replay live cadence

Chaque langue assemble ses 12 clips avec 500 ms de silence entre clips : fixtures FR 117,7 s et EN 103,1 s, hashes dans `live-fixtures.json`. Le meme binaire CUDA que `21` lit les fixtures a cadence de 20 ms, par fenetres de 5 s. Le modele reste precharge. Les journaux stderr manifestes montrent explicitement `CUDA0` pour les deux passages ; l'identite du binaire, du modele et des fixtures est dans `live-fleurs-summary.json`. Les logs JSONL et la recette de synthese sont conserves. La mesure est entre la fin de capture de chaque fenetre et la sortie console apres inférence ; elle ne mesure pas l'apparition d'un mot dans une UI.

| Replay | Fenetres | Callback | Mediane apres fenetre | P95 nearest rank | Maximum | Attente file max |
|---|---:|---:|---:|---:|---:|---:|
| GPU FR+EN | 45 | 45 | 585 ms | 704 ms | 804 ms | 0 ms |
| CPU EN (clip 16,38 s) | 4 | 4 | 26.694 s | 43.167 s | 43.167 s | 29.618 s |

Le replay CPU emploie le binaire sans CUDA et un seul clip FLEURS EN (validation row 1, id 1620), converti en PCM16 ; il n'est pas le flux concatene des 12 clips. Sur GPU, aucun backlog ne se forme dans ces 220,8 s de replay ; le maximum mesure depuis le debut d'une fenetre de 5 s jusqu'a la sortie console est 5,804 s. Sur CPU, le court clip montre deja une attente de file de 29,618 s. Aucune promesse de temps reel CPU ne peut en etre deduite.

## Perte et decisions restantes

La perte reste mesuree par les **100 arrets independants** de `21` : mediane/P95 260/480 ms pour 0,5 s de confirmation, 350/980 ms pour 1 s. Ce phenomene depend de la politique de confirmation, pas du contenu linguistique du clip ; aucun nouvel essai de durabilite electrique ou de capture reelle n'est ajoute ici.

Ces essais etendent la diversite des voix et la duree du replay ; ils ne couvrent pas les microphones, les bruits reels, deux heures de live, l'UI, la saturation du moteur, ni l'annotation fine des silences. Les seuils V1 de VAD, perte et delai restent OPEN jusqu'a une decision sur corpus annote et conditions cibles, selon la demande utilisateur. AC-02/08/13 restent NOT RUN pour le produit.
