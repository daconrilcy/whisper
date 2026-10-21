# FEAS-PM-01 — zones critiques à contrôler après l’écoute complète

Toutes les heures et tous les mots ci-dessous viennent de Parakeet ou de l’analyse acoustique : **AUTO_NON_VALIDÉ**. La liste réunit les coupes A forcées et les bords B traversés par un mot ASR, sans exécuter A ni B. Les autres bords restent dans `auto_annotations.json` et ne demandent pas de contrôle manuel systématique pour cette première qualification.

Après avoir écouté le WAV entier et corrigé son texte, parcourir la table du même fichier. Un seul accord explicite peut couvrir toutes ses lignes si aucun écart n’a été trouvé ; sinon noter uniquement les exceptions et leurs temps. Pour rejouer une ligne : `ffplay -nodisp -autoexit -ss <temps-1,5> -t 3 "<WAV>"` (ramener le début à 0 si nécessaire).

Vérifier le **mot réellement audible** de part et d’autre de la frontière, sa présence unique, son orthographe et, s’il traverse la coupure, le côté où l’audio porte chaque partie. Les temps ASR ne constituent pas des temps approuvés.


## fr_pauses

| Temps (s) | Mot(s) attendu(s), temps ASR provisoires (s) | Motif |
| ---: | --- | --- |
| 0.00 | aucun mot ASR proche — écouter directement | début |
| 8.00 | **Présidente.** (7.52–8.16) | A 8 s, B mot traversant |
| 10.00 | **l'instrumentalisation** (9.68–10.80) | B mot traversant |
| 14.00 | **fausse** (13.92–14.32) | B mot traversant |
| 16.00 | **systémique** (15.44–16.00), **en** (16.00–16.24) | A 8 s |
| 18.00 | **scandaleuse.** (17.60–18.72) | B mot traversant |
| 22.00 | **Floyd,** (21.52–22.16) | B mot traversant |
| 24.00 | **des** (23.84–24.00), **sujets,** (24.00–24.32) | A 8 s |
| 26.00 | **lui** (25.84–26.16) | B mot traversant |
| 28.00 | **ségrégation** (27.44–28.08) | B mot traversant |
| 30.00 | **notre** (29.92–30.16) | B mot traversant |
| 32.00 | **particulier.** (31.60–32.56) | A 8 s, B mot traversant |
| 34.00 | **statuts** (33.92–34.16) | B mot traversant |
| 40.00 | **l'histoire** (39.92–40.32) | A 8 s, B mot traversant |
| 42.00 | **condamner** (41.68–42.16) | B mot traversant |
| 44.00 | **rappeler** (43.92–44.24) | B mot traversant |
| 46.00 | **première** (45.84–46.16) | B mot traversant |
| 48.00 | **l'esclavage** (47.28–48.08) | A 8 s, B mot traversant |
| 50.00 | **Plutôt** (49.84–50.16) | B mot traversant |
| 52.00 | **l'ordre** (51.76–52.16) | B mot traversant |
| 54.00 | **face** (53.92–54.08) | B mot traversant |
| 56.00 | **de** (56.00–56.08), **plus** (56.08–56.24) | A 8 s |
| 58.00 | **victimes,** (57.84–58.48) | B mot traversant |
| 62.00 | **reconnaître** (61.60–62.24) | B mot traversant |
| 64.00 | **l'indignation** (63.52–64.08) | A 8 s, B mot traversant |
| 65.02 | **géométrie.** (64.32–64.96) | fin |

À confirmer pour `fr_pauses` : toutes les lignes vérifiées **OUI / NON** ; exceptions (temps, mot attendu, temps corrigé) : **À RENSEIGNER**.


## en_pauses

| Temps (s) | Mot(s) attendu(s), temps ASR provisoires (s) | Motif |
| ---: | --- | --- |
| 0.00 | **You** (0.00–0.24), **can't** (0.24–0.56) | début |
| 4.00 | **opposition** (3.44–4.08) | B mot traversant |
| 6.00 | **afraid** (5.84–6.08) | B mot traversant |
| 8.00 | **work** (7.84–8.00), **won't** (7.44–7.84) | A 8 s |
| 10.00 | **are** (9.84–10.16) | B mot traversant |
| 12.00 | **enough.** (11.92–12.64) | B mot traversant |
| 14.00 | **including** (13.92–14.48) | B mot traversant |
| 16.00 | **months** (15.92–16.24) | A 8 s, B mot traversant |
| 18.00 | **beaten,** (17.92–18.56) | B mot traversant |
| 20.00 | **cannons,** (19.52–20.24) | B mot traversant |
| 24.00 | **stronger.** (23.44–24.08) | A 8 s, B mot traversant |
| 28.00 | **elections.** (27.76–28.32) | B mot traversant |
| 30.00 | **that** (29.92–30.24) | B mot traversant |
| 32.00 | **elections** (31.84–32.40) | A 8 s, B mot traversant |
| 34.00 | **the** (33.84–34.08) | B mot traversant |
| 36.00 | **Bulgarian** (35.52–36.16) | B mot traversant |
| 38.00 | **This** (37.84–38.16) | B mot traversant |
| 40.00 | **demands** (39.84–40.24) | A 8 s, B mot traversant |
| 44.00 | **personage** (43.84–44.32) | B mot traversant |
| 48.00 | **nor** (47.92–48.16) | A 8 s, B mot traversant |
| 50.00 | **with.** (49.68–50.08) | B mot traversant |
| 52.00 | **funds,** (51.76–52.32) | B mot traversant |
| 56.00 | **with** (55.84–56.00), **former** (56.00–56.40) | A 8 s |
| 58.00 | **judges** (57.68–58.16) | B mot traversant |
| 60.00 | **judges,** (59.68–60.16) | B mot traversant |
| 64.00 | **goes.** (63.44–64.00), **But** (64.00–64.24) | A 8 s |
| 65.00 | **what?** (64.56–64.96), **know** (64.40–64.56) | fin |

À confirmer pour `en_pauses` : toutes les lignes vérifiées **OUI / NON** ; exceptions (temps, mot attendu, temps corrigé) : **À RENSEIGNER**.


## fr_continu

| Temps (s) | Mot(s) attendu(s), temps ASR provisoires (s) | Motif |
| ---: | --- | --- |
| 0.00 | **J'observerai** (0.08–0.64) | début |
| 4.00 | **seulement,** (3.68–4.32) | B mot traversant |
| 6.00 | **posés.** (5.60–6.08) | B mot traversant |
| 8.00 | **d'avoir** (7.92–8.32) | A 8 s, B mot traversant |
| 10.00 | **totalement** (9.92–10.40) | B mot traversant |
| 12.00 | **relation** (11.84–12.16) | B mot traversant |
| 16.00 | **dialogue** (15.76–16.32) | A 8 s, B mot traversant |
| 22.00 | **clarté** (21.84–22.16) | B mot traversant |
| 24.00 | **Xi,** (23.92–24.40) | A 8 s, B mot traversant |
| 28.00 | **parle** (27.92–28.08) | B mot traversant |
| 30.00 | **à** (29.84–30.00), **Haute-Gone,** (30.00–30.80) | préfixe 30 s |
| 32.00 | **le** (31.92–32.08) | A 8 s, B mot traversant |
| 36.00 | **Nous** (35.92–36.08) | B mot traversant |
| 38.00 | **croyons** (37.92–38.24) | B mot traversant |
| 40.00 | **que** (39.92–40.08) | A 8 s, B mot traversant |
| 44.00 | **climatique.** (43.60–44.16) | B mot traversant |
| 46.00 | **Sud,** (45.92–46.24) | B mot traversant |
| 48.00 | **climatique,** (48.00–48.48), **neutralité** (47.52–47.92) | A 8 s |
| 50.00 | **rester** (49.76–50.08) | B mot traversant |
| 52.00 | **l'on** (51.92–52.16) | B mot traversant |
| 56.00 | **voudrais** (55.84–56.24) | A 8 s, B mot traversant |
| 58.00 | **qui** (57.92–58.16) | B mot traversant |
| 60.00 | **sens.** (59.68–59.92), **faire** (59.52–59.60) | fin |

À confirmer pour `fr_continu` : toutes les lignes vérifiées **OUI / NON** ; exceptions (temps, mot attendu, temps corrigé) : **À RENSEIGNER**.


## en_continu

| Temps (s) | Mot(s) attendu(s), temps ASR provisoires (s) | Motif |
| ---: | --- | --- |
| 0.00 | **This** (0.08–0.24), **is** (0.24–0.32) | début |
| 2.00 | **Democrats** (1.44–2.08) | B mot traversant |
| 4.00 | **considering** (3.60–4.24) | B mot traversant |
| 8.00 | **investment** (7.92–8.40) | A 8 s, B mot traversant |
| 12.00 | **life** (11.84–12.16) | B mot traversant |
| 16.00 | **quality** (15.68–16.24) | A 8 s, B mot traversant |
| 20.00 | **housing,** (19.68–20.16) | B mot traversant |
| 22.00 | **and** (21.84–22.08) | B mot traversant |
| 24.00 | **groups.** (23.76–24.40) | A 8 s, B mot traversant |
| 28.00 | **leaders** (27.84–28.32) | B mot traversant |
| 30.00 | **that** (29.76–30.00), **puts** (30.00–30.40) | préfixe 30 s |
| 32.00 | **challenges** (31.68–32.16) | A 8 s, B mot traversant |
| 34.00 | **ones.** (33.84–34.56) | B mot traversant |
| 36.00 | **align** (35.92–36.32) | B mot traversant |
| 38.00 | **dimensions** (37.84–38.32) | B mot traversant |
| 40.00 | **avoid** (39.92–40.16) | A 8 s, B mot traversant |
| 42.00 | **we** (41.84–42.08) | B mot traversant |
| 46.00 | **in** (45.92–46.16) | B mot traversant |
| 48.00 | **consider** (47.84–48.16) | A 8 s, B mot traversant |
| 52.00 | **instruments** (51.68–52.32) | B mot traversant |
| 54.00 | **child** (53.84–54.24) | B mot traversant |
| 56.00 | **guarantee** (55.44–56.00), **can** (56.00–56.24) | A 8 s |
| 58.00 | **allocate** (57.76–58.24) | B mot traversant |
| 60.00 | **are** (59.60–59.84), **we** (59.52–59.60) | fin |

À confirmer pour `en_continu` : toutes les lignes vérifiées **OUI / NON** ; exceptions (temps, mot attendu, temps corrigé) : **À RENSEIGNER**.

## Contrôle acoustique distinct

Ne valider aucune pause ou plage de bruit par déduction du seuil fixe. Pendant l’écoute complète, noter toute hésitation voisée, bruit transitoire ou véritable pause ≥400 ms observée, y compris si la liste automatique l’a manquée. Pour chaque candidat ci-dessous, répondre `PAUSE`, `HÉSITATION_VOISÉE`, `BRUIT`, `PAROLE_FAIBLE` ou `INCERTAIN`. Désigner séparément une plage **bruit seul** comparable au clip pour calibrer le seuil relatif ; si aucune n’existe, écrire `ABSENTE`.

### fr_pauses

- Pauses fixes exploratoires −35 dBFS : 4.74–6.43, 12.88–13.41, 26.88–27.46, 32.39–32.80, 38.34–38.98, 49.51–50.01, 58.23–58.65, 63.39–63.89.
- Coupes produites par le seuil relatif **non calibré**, à confirmer ou rejeter : 5.02–6.10.
- Trois courtes baisses d’énergie à écouter pour distinguer hésitation, bruit et parole faible : 38.62–38.94, 6.18–6.42, 63.60–63.78.
- Plage la plus calme proposée comme « bruit seul » : 5.20–5.40 s, p95 -65.17 dBFS. **Son contenu n’est pas connu automatiquement.**
- Étiquettes et plage de bruit seul approuvées : **EN ATTENTE**.

### en_pauses

- Pauses fixes exploratoires −35 dBFS : 12.21–12.67.
- Coupes produites par le seuil relatif **non calibré**, à confirmer ou rejeter : 12.26–12.66.
- Trois courtes baisses d’énergie à écouter pour distinguer hésitation, bruit et parole faible : 27.00–27.26, 25.20–25.44, 63.94–64.14.
- Plage la plus calme proposée comme « bruit seul » : 12.40–12.60 s, p95 -63.29 dBFS. **Son contenu n’est pas connu automatiquement.**
- Étiquettes et plage de bruit seul approuvées : **EN ATTENTE**.

### fr_continu

- Pauses fixes exploratoires −35 dBFS : aucune.
- Coupes produites par le seuil relatif **non calibré**, à confirmer ou rejeter : 6.90–7.30, 12.52–12.96, 32.78–33.20, 54.78–55.18.
- Trois courtes baisses d’énergie à écouter pour distinguer hésitation, bruit et parole faible : 6.90–7.30, 54.78–55.18, 39.20–39.58.
- Plage la plus calme proposée comme « bruit seul » : 33.00–33.20 s, p95 -36.05 dBFS. **Son contenu n’est pas connu automatiquement.**
- Étiquettes et plage de bruit seul approuvées : **EN ATTENTE**.

### en_continu

- Pauses fixes exploratoires −35 dBFS : aucune.
- Coupes produites par le seuil relatif **non calibré**, à confirmer ou rejeter : 4.06–4.56, 6.16–6.62, 13.08–13.48, 24.10–24.50, 34.08–34.66, 41.08–41.74, 49.12–49.54, 52.62–53.16.
- Trois courtes baisses d’énergie à écouter pour distinguer hésitation, bruit et parole faible : 24.10–24.50, 54.62–55.00, 9.76–10.12.
- Plage la plus calme proposée comme « bruit seul » : 54.80–55.00 s, p95 -34.33 dBFS. **Son contenu n’est pas connu automatiquement.**
- Étiquettes et plage de bruit seul approuvées : **EN ATTENTE**.
