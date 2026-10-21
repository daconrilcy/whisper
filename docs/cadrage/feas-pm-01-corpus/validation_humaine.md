# FEAS-PM-01 — fiche de validation humaine

État : **quatre textes CANDIDAT** ; aucune approbation humaine enregistrée. Les temps de mots, frontières et étiquettes acoustiques restent `AUTO_NON_VALIDE`.

## 1. Écouter et corriger les textes

Depuis PowerShell, se placer dans `C:\dev\whisper`. Pour chaque ligne, ouvrir le texte dans le Bloc-notes, puis lancer l'audio dans un second terminal. Les deux restent visibles côte à côte. Éditer **la copie `candidate_*.txt` dans ce dossier**, puis l'enregistrer ; conserver les WAV et les textes sources de `sound example/` intacts. Une écoute complète de chaque WAV est indispensable pour attester que le texte correspond à l'audible.

| Cas | WAV à écouter | Texte à ouvrir et corriger |
| --- | --- | --- |
| Français avec pauses | [`fr_long_2020.wav`](<../../../sound example/voxpopuli/clips/long_examples/fr_long_2020.wav>) | [candidate_fr_pauses.txt](candidate_fr_pauses.txt) |
| Anglais avec pauses | [`en_long_2020.wav`](<../../../sound example/voxpopuli/clips/long_examples/en_long_2020.wav>) | [candidate_en_pauses.txt](candidate_en_pauses.txt) |
| Français continu | [`fr_continuous60_2020_CANDIDATE.wav`](<../../../sound example/voxpopuli/clips/long_examples/fr_continuous60_2020_CANDIDATE.wav>) | [candidate_fr_continu.txt](candidate_fr_continu.txt) |
| Anglais continu | [`en_continuous60_2020_CANDIDATE.wav`](<../../../sound example/voxpopuli/clips/long_examples/en_continuous60_2020_CANDIDATE.wav>) | [candidate_en_continu.txt](candidate_en_continu.txt) |

Commandes prêtes à copier, **une paire par cas** :

```powershell
notepad "docs\cadrage\feas-pm-01-corpus\candidate_fr_pauses.txt"
ffplay -nodisp -autoexit -i "sound example\voxpopuli\clips\long_examples\fr_long_2020.wav"

notepad "docs\cadrage\feas-pm-01-corpus\candidate_en_pauses.txt"
ffplay -nodisp -autoexit -i "sound example\voxpopuli\clips\long_examples\en_long_2020.wav"

notepad "docs\cadrage\feas-pm-01-corpus\candidate_fr_continu.txt"
ffplay -nodisp -autoexit -i "sound example\voxpopuli\clips\long_examples\fr_continuous60_2020_CANDIDATE.wav"

notepad "docs\cadrage\feas-pm-01-corpus\candidate_en_continu.txt"
ffplay -nodisp -autoexit -i "sound example\voxpopuli\clips\long_examples\en_continuous60_2020_CANDIDATE.wav"
```

Contrôler particulièrement les débuts, les fins, noms et nombres. Le français avec pauses n'a **aucun mot proposé par Parakeet avant 6,72 s** : écouter 0–7 s et corriger le texte, sans devoir horodater tous ces mots. Garder les répétitions réellement audibles. Signaler séparément les hésitations vocales sans mot lexical et les bruits ; ne pas ajouter de mots situés hors du WAV.

## 2. Fixer les deux préfixes exacts à 30 secondes

Les quatre extraits ci-dessous sont des coupes **à l'échantillon près** des WAV continus d'origine. Écouter d'abord 28–32 s pour comprendre la phrase, puis 0–30 s pour décider quel est le dernier mot entièrement audible avant la coupure. Si un mot traverse exactement 30 s, le noter comme **mot traversant** plutôt que de le deviner ou de copier tout le segment JSON.

| Langue | Contexte de la coupure | Audio exact 0–30 s | Préfixe textuel à fournir |
| --- | --- | --- | --- |
| Français | [28–32 s](fr_passage_28-32.wav) | [0–30 s](fr_prefix_0-30.wav) | Derniers mots et éventuel mot traversant : **À RENSEIGNER** |
| Anglais | [28–32 s](en_passage_28-32.wav) | [0–30 s](en_prefix_0-30.wav) | Derniers mots et éventuel mot traversant : **À RENSEIGNER** |

Les propositions automatiques de préfixe sont dans `parakeet_word_proposals.json`. Elles peuvent aider à repérer la zone, mais ne constituent pas la réponse à approuver.

## 3. Vérifier uniquement les frontières critiques

Après correction des textes, ouvrir [zones_critiques.md](zones_critiques.md). Sa table donne, pour chaque coupe A forcée et chaque bord B qu'un mot ASR traverse, le temps et le mot provisoire attendu. Une seule réécoute des portions difficiles suffit : noter **uniquement les exceptions** (mot absent, coupé, répété, différent ou temps manifestement faux). Le début FR 0–7 s et les deux coupures à 30 s sont prioritaires. Les autres mots et les bords B sans mot traversant ne demandent pas un contrôle manuel systématique pour cette préparation.

Le tableau de frontières sera recalcé si une correction du texte change les mots attendus. La validation des frontières est **distincte** de l'approbation des quatre textes.

## 4. Valider séparément l'acoustique

La section « Contrôle acoustique distinct » de [zones_critiques.md](zones_critiques.md) propose des intervalles à écouter. Indiquer pour chacun `PAUSE`, `HÉSITATION_VOISÉE`, `BRUIT`, `PAROLE_FAIBLE` ou `INCERTAIN`. Désigner, si possible, une courte plage contenant **seulement le bruit de fond** dans chaque WAV, ou dans le contexte immédiat du même OGG pour les continus. Dire explicitement `ABSENTE` si aucune plage comparable n'existe. Cette décision est séparée des textes et des mots de frontière ; aucun seuil relatif au bruit n'est retenu avant elle.

## 5. Donner un accord explicite

Après enregistrement des corrections, envoyer un message de cette forme, pour un cas à la fois ou pour les quatre ensemble :

> J'ai écouté intégralement les WAV `fr_pauses`, `en_pauses`, `fr_continu`, `en_continu`. J'approuve explicitement leurs textes `candidate_*.txt` actuellement enregistrés, avec les corrections que j'ai faites [ou sans correction]. Pour les continus, le préfixe exact à 30 s est : FR « … », EN « … » ; mots traversants éventuels : … . J'ai vérifié les lignes critiques de `zones_critiques.md` ; exceptions : … . Pour l'acoustique, mes étiquettes et plages de bruit seul sont : … .

L'accord peut être partiel : nommer seulement les cas ou catégories effectivement contrôlés. **Une simple ouverture, copie ou sauvegarde n'est pas un accord.** À réception de l'accord, la session principale recalcule les SHA-256 des WAV et des textes enregistrés, les lie à la décision humaine datée et met à jour les statuts ; si un fichier change ensuite, l'accord associé à son ancienne empreinte devient caduc. Les temps non contrôlés restent `AUTO_NON_VALIDE`. Les fichiers ne deviennent pas des références approuvées avant cette étape.
