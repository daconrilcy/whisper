# FEAS-PM-01 — fiche de validation humaine

État au 21 octobre 2026 : les quatre textes `_copie` ont été **approuvés explicitement par l'utilisateur** après correction. Les mots aux frontières de [zones_critiques.md](zones_critiques.md) ont reçu « OK avec le tableau » ; leurs temps ASR restent provisoires. Huit zones acoustiques ciblées ont ensuite été étiquetées dans la [revue acoustique](acoustic-review.md), sans qualifier de seuil relatif au bruit.

## 1. Écouter et corriger les textes

Les fichiers à retenir sont **les quatre `_copie.txt` corrigés par l'utilisateur**. Les premiers `candidate_*.txt` restent des brouillons historiques. Le manifeste lie les octets exacts des `_copie.txt` approuvés aux SHA-256 des WAV. Les WAV et les textes sources de `sound example/` restent intacts.

| Cas | WAV à écouter | Texte à ouvrir et corriger |
| --- | --- | --- |
| Français avec pauses | [`fr_long_2020.wav`](<../../../sound example/voxpopuli/clips/long_examples/fr_long_2020.wav>) | [candidate_fr_pauses_copie.txt](candidate_fr_pauses_copie.txt) |
| Anglais avec pauses | [`en_long_2020.wav`](<../../../sound example/voxpopuli/clips/long_examples/en_long_2020.wav>) | [candidate_en_pauses_copie.txt](candidate_en_pauses_copie.txt) |
| Français continu | [`fr_continuous60_2020_CANDIDATE.wav`](<../../../sound example/voxpopuli/clips/long_examples/fr_continuous60_2020_CANDIDATE.wav>) | [candidate_fr_continu_copie.txt](candidate_fr_continu_copie.txt) |
| Anglais continu | [`en_continuous60_2020_CANDIDATE.wav`](<../../../sound example/voxpopuli/clips/long_examples/en_continuous60_2020_CANDIDATE.wav>) | [candidate_en_continu_copie.txt](candidate_en_continu_copie.txt) |

Commandes prêtes à copier, **une paire par cas** :

```powershell
notepad "docs\cadrage\feas-pm-01-corpus\candidate_fr_pauses_copie.txt"
ffplay -nodisp -autoexit -i "sound example\voxpopuli\clips\long_examples\fr_long_2020.wav"

notepad "docs\cadrage\feas-pm-01-corpus\candidate_en_pauses_copie.txt"
ffplay -nodisp -autoexit -i "sound example\voxpopuli\clips\long_examples\en_long_2020.wav"

notepad "docs\cadrage\feas-pm-01-corpus\candidate_fr_continu_copie.txt"
ffplay -nodisp -autoexit -i "sound example\voxpopuli\clips\long_examples\fr_continuous60_2020_CANDIDATE.wav"

notepad "docs\cadrage\feas-pm-01-corpus\candidate_en_continu_copie.txt"
ffplay -nodisp -autoexit -i "sound example\voxpopuli\clips\long_examples\en_continuous60_2020_CANDIDATE.wav"
```

L'utilisateur a précisé que le début du français avec pauses (0–4 s) est en anglais. Parakeet n'avait proposé aucun mot avant 6,72 s : ses temps ne couvrent donc pas ce début. L'écoute ciblée F1 confirme maintenant une vraie pause vers 5–6 s et la reprise du français autour de 6,5 s, sans attester les bornes exactes à l'échantillon.

## 2. Fixer les deux préfixes exacts à 30 secondes

Les quatre extraits ci-dessous sont des coupes **à l'échantillon près** des WAV continus d'origine. Écouter d'abord 28–32 s pour comprendre la phrase, puis 0–30 s pour décider quel est le dernier mot entièrement audible avant la coupure. Si un mot traverse exactement 30 s, le noter comme **mot traversant** plutôt que de le deviner ou de copier tout le segment JSON.

| Langue | Contexte de la coupure | Audio exact 0–30 s | Préfixe textuel à fournir |
| --- | --- | --- | --- |
| Français | [28–32 s](fr_passage_28-32.wav) | [0–30 s](fr_prefix_0-30.wav) | `situation` tronqué selon l'utilisateur ; dernier mot entier dérivé du texte approuvé : `la`. |
| Anglais | [28–32 s](en_passage_28-32.wav) | [0–30 s](en_prefix_0-30.wav) | `that` tronqué selon l'utilisateur ; dernier mot entier dérivé du texte approuvé : `model`. |

Décision humaine sur la coupure **exacte de 30 s** : `situation` (FR) et `that` (EN) sont des **mots acoustiquement tronqués**. Le texte des mots entièrement audibles avant eux, dérivé des textes `_copie` approuvés, est dans `manifest.json` ; il finit respectivement par `… je parle de la` et `… a model`. Les mots tronqués sont annotés à part, sans les compter comme des mots complets du préfixe. Les observations faites sur le clip 28–32 s (`Hong Kong`/`le` en FR ; `social`/`challenges` en EN) décrivent ce contexte plus large et ne remplacent pas la coupure à 30 s. Les temps Parakeet proches de 30 s restent `AUTO_NON_VALIDE`.

## 3. Vérifier uniquement les frontières critiques

Après correction des textes, ouvrir [zones_critiques.md](zones_critiques.md). Sa table donne, pour chaque coupe A forcée et chaque bord B qu'un mot ASR traverse, le temps et le mot provisoire attendu. Une seule réécoute des portions difficiles suffit : noter **uniquement les exceptions** (mot absent, coupé, répété, différent ou temps manifestement faux). Le début FR 0–7 s et les deux coupures à 30 s sont prioritaires. Les autres mots et les bords B sans mot traversant ne demandent pas un contrôle manuel systématique pour cette préparation.

L'utilisateur a répondu « OK avec le tableau » pour les quatre cas, après avoir signalé le début FR 0–4 s en anglais. Cet accord porte sur les **mots aux frontières listées**, pas sur la précision de tous les temps Parakeet ni sur les étiquettes acoustiques. Si le texte approuvé change, ces validations devront être réexaminées.

## 4. Valider séparément l'acoustique

Après avoir dispensé la revue acoustique longue, l'utilisateur a écouté huit extraits ciblés (15,6 s cumulées) et donné les étiquettes consignées dans la [revue acoustique](acoustic-review.md) et le [manifeste](manifest.json). F1, F2 et E1 contiennent de vraies pauses ; FC2 et EC3 n'en contiennent pas ; FC1, EC1 et EC2 contiennent des reprises de souffle ou micro-pauses. Les plages les plus calmes des continus ne sont pas validées comme bruit seul. Le contrôle acoustique est **PARTIEL** et aucun seuil relatif au bruit n'est retenu. Cette réserve est indépendante de l'approbation des textes.

## 5. Donner un accord explicite

La réponse explicite « **J'approuve les quatre textes** » reçue dans cette session lie les quatre fichiers `_copie` aux SHA-256 des WAV et des textes inscrits dans le [manifeste](manifest.json). Cette approbation a été enregistrée le 21 octobre 2026. Une simple ouverture ou sauvegarde n'aurait pas suffi. Si l'un des fichiers approuvés change, son accord lié à l'ancienne empreinte devient caduc. L'approbation du texte ne transforme pas les temps de mots ou les annotations acoustiques automatiques en références humaines.
