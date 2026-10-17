Avancée : la passe unique applicative passe à 5 et 10 min ; le mot B6 et son attribution sont identifiés.
Blocage : 30 min expire dans le test applicatif, et le worker e767 ne sérialise pas les mots natifs lorsqu'il refuse B5.
Décision : retenir provisoirement 10 min comme dernière durée passée, conserver B8 inactive et attendre la validation utilisateur avant tout changement de code.

# Diagnostic R5-2 — B5 passe unique et B6 frontière

Base : commit R5-2 accepté `467f3bcef155426cda785ca402be1708538ae601`, worker CPU `e7672a520db87926b3da9b8e5b585a1e7e57ce53ff8b1d0fe95251046aa01f4d`, modèle `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`, runtime `d5a2cc8ef441dd5934972adae00f997241bea17a7bc5977a8c7f0122aaa958ff`. Aucun code source modifié. Les nouveaux WAV, IPC et sorties de recette sont sous `target/parakeet-tests/l2/l2b/r5-2`, hors Git.

## 1. B5 — mot 2030 et durée en passe unique

Le test B5 déjà intégré a échoué sur `spkr0_x12.wav` (60 min, 57 600 000 échantillons) en **322,98 s** : `InvalidTimestamp: word 2030 overlap cannot be normalized with positive durations`. Le worker ne produit alors qu'un événement `Failed` : le tableau natif de mots reste interne à `native_engine.rs::normalize_words` et n'est pas sérialisé. Sans changer de code, le texte et les bornes **exacts du run e767** ne sont donc pas directement observables.

Une trace native préexistante du banc `spike02/revised_cpu_3600s.log` porte sur le **même WAV SHA-256** `83a922126cc70ff18f9edc8a292960203dda241cc1f8cb2b5dcd75839cc9c22f`, le même modèle et la même DLL CPU. Elle utilise toutefois l'ancien wrapper et une structure d'options native différente ; ses valeurs sont une **corroboration**, pas une capture brute du worker e767.

| Source de preuve | Mot précédent | Mot 2030 | Chevauchement | Lecture |
| --- | --- | --- | ---: | --- |
| e767, B5 R5-2 | non émis | index 2030, texte/bornes non émis | au plus 80 ms d'après la branche de refus, valeur exacte non émise | Échec de normalisation : ni raccourcir le précédent ni avancer le suivant ne laisserait une durée positive (`native_engine.rs:109–145`). |
| Trace native antérieure, même audio/DLL | `the` 618 880–618 960 ms | `eye.` 618 880–618 960 ms | **80 ms**, intervalles identiques | Explique exactement le motif de refus si la même paire est retournée au worker e767. |

À 618,880 s, la jonction du montage la plus proche est **600 s**, soit **18,880 s** d'écart ; cette occurrence n'est donc **pas** à ±2 s d'une jonction. Le constat de branche du worker et cette comparaison temporelle sont établis ; l'identité textuelle et les bornes e767 restent à confirmer par un diagnostic natif explicitement autorisé si elles sont exigées comme preuve directe.

### Recette applicative complémentaire

Méthode : exécution du test applicatif existant `r5_2_a4_product_route_publishes_french_and_english_txt_srt` sans recompiler, en remplaçant uniquement sa variable d'entrée EN par chaque source et en séparant les destinations sous R5-2. Ce parcours exerce le vrai worker, l'adaptateur, l'archive, TXT et SRT ; le test repasse aussi ses deux cas FR inchangés. `spkr0.wav` est l'enregistrement réel ; `x2` et `x6` sont des répétitions synthétiques.

| Source EN | Durée ; échantillons | SHA-256 source | Résultat applicatif | Premier mot refusé |
| --- | ---: | --- | --- | --- |
| `spkr0.wav` réel | 5 min ; 4 800 000 | `e9561af7da6f2838cd5f298c98179dbe7abdb4ed1bc91d2b22471cb3bea40862` | **PASS** ; TXT/SRT publiés, archive complète | Aucun |
| `spkr0_x2.wav` synthétique | 10 min ; 9 600 000 | `8a9eeb88b1965926a0e4f65a30566396ff43c4ad818ab0651966c597476192de` | **PASS** ; TXT/SRT publiés, archive complète | Aucun |
| `spkr0_x6.wav` synthétique | 30 min ; 28 800 000 | `acca7bacdbfa9047bceb23b654a644fdeb4ea07b1847709a8153da59838b2d5c` | **FAIL du test applicatif** : délai de 300 s par cas dépassé ; `AppView` encore `Running`, aucun TXT/SRT | Aucun mot refusé observé : le worker n'avait pas rendu son résultat |

Les journaux et sorties des deux premiers cas sont respectivement `r5-2/logs/diagnostic-b5-{5min,10min}.log` et `r5-2/archive/diagnostic-b5-{5min,10min}/a4/en15`. Le troisième journal est `r5-2/logs/diagnostic-b5-30min.log` ; son échec est un **timeout du test**, sans `InvalidTimestamp` observé. Le worker resté actif après la fin du test a été arrêté explicitement. Le nom `en15` est celui du cas de test réutilisé ; la source effective figure dans le pending et dans les SHA ci-dessus.

Le défaut `InvalidTimestamp` de B5 est constaté à **60 min** sur un montage, hors jonction. Il **n'est pas constaté sur l'enregistrement réel de 5 min** ni sur le montage de 10 min. Le cas de 30 min ne permet pas de conclure sur les horodatages : son délai de recette expire avant le résultat. La plus longue durée ayant réellement obtenu un PASS du parcours applicatif est donc **10 min**, avec la réserve que cette source est synthétique. La seule durée longue réelle passée est **5 min**.

## 2. B6 — coupure à 262,810 s

La sortie IPC e767 a été rejouée **sans modification de code** sur les fenêtres VAD 2 et 3, ainsi que sur la référence FR synthétique de 1 915,8 s. Le silence retenu par la règle actuelle RMS 64 va de **262,340 à 263,280 s** (940 ms) ; la coupure est à **262,810 s**, son milieu.

| Parcours / mot | Start–end | Centre | Rapport à la coupure | Attribution |
| --- | ---: | ---: | --- | --- |
| Référence passe unique : `41.` | 261,760–263,200 s | 262,480 s | Dépasse la coupure de **390 ms** | Pas de fenêtres |
| VAD fenêtre 2 : `41.` | 261,700–262,900 s | 262,300 s | Dépasse la coupure de **90 ms** | **Fenêtre 2** : centre avant 262,810 s |
| VAD fenêtre 3 : `527` (mot suivant) | 263,570–264,450 s | 264,010 s | Commence après le silence | **Fenêtre 3** |

`41.` apparaît une seule fois près de cette frontière dans la VAD et existe aussi en passe unique. Il n'est **ni perdu ni dupliqué** par l'attribution centrale ; son intervalle est étiré dans un silence, de manière plus courte en VAD qu'en passe unique. Le critère strict « aucun mot ne traverse une coupure » échoue mécaniquement, mais cette observation seule ne prouve pas une dégradation textuelle. Les deux coupures précédentes, à 91,540 et 181,580 s, n'avaient aucune édition/troncature dans l'essai B6.

Preuves hors Git : `r5-2/logs/b6-ipc-output.jsonl`, `b6-reference-ipc-output.jsonl` et WAV de fenêtres sous `r5-2/archive/long/fr-long/vad`.

## 3. Décision produit provisoire à confirmer

La limite provisoire proposée est **10 min** : c'est la plus longue durée effectivement passée, mais sur un montage synthétique ; 5 min est la seule durée réelle vérifiée. Au-delà : refus **avant pending** avec « Fichier trop long pour Parakeet, utilisez Turbo » ; aucune bascule silencieuse. Cette restriction est **une décision produit demandée mais non codée** : la fiche Qualified Test actuelle reste en place tant qu'un lot correctif n'est pas autorisé. B8 reste inactive. La [note VAD adaptative](note-parakeet-vad-adaptative.md) décrit le remplacement demandé du seuil RMS 64 et sa future évaluation.
