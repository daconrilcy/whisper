Avancée : les trois arbitrages utilisateur ferment R6-P1 à R6-P3 dans le plan v2 ; la recette 60 min reste informative et privée.
Blocage : le second et dernier verdict indépendant reste à obtenir avant toute implémentation.
Décision : si ce verdict est CLEAN, implémenter R6 dans les seuls chemins autorisés après préflight ; sinon arrêter sans code.

# PLAN-PARA-L2b-R6 v2 — normalisation et limite de passe unique

**Statut : DRAFT v2, second et dernier cycle de revue indépendante demandé.** Base : `wip/l2b-candidate`, diagnostic `2470d297`, verdict v1 `40920b49`, décisions utilisateur du 17/10/2026 répondant à R6-P1–P3. R5-2 A4 est Qualified Test pour l'ancienne clé ; B8 reste inactive. Ce plan couvre un seul lot R6, sans VAD adaptative. L'autorisation conditionnelle d'implémentation ne devient effective qu'après CLEAN.

## 1. Résultat produit et chemins

Résultat : traiter des mots natifs consécutifs sur une même trame sans `InvalidTimestamp` artificiel ; ne publier que des mots ordonnés, de durée strictement positive ; limiter l'import Parakeet à la plus longue durée passée par le parcours applicatif ; garder Turbo inchangé et refuser les anciennes clés avant pending. L'ancien pending reste conservé avec cause explicite, selon la règle R5-2.

Chemins d'écriture **exhaustifs** : `crates/whisper-worker-parakeet-cpu/src/native_engine.rs`, `crates/whisper-worker-parakeet-cpu/tests/native_import.rs`, `crates/whisper-adapters/src/worker_ipc.rs`, `crates/whisper-adapters/tests/parakeet_import.rs`, `crates/whisper-adapters/src/archive.rs`, `crates/whisper-adapters/tests/parakeet_archive.rs`, `crates/whisper-core/src/model_registry.rs`, et les fiches sous `docs/cadrage/`. Toutes les écritures d'exécution vont sous `target/parakeet-tests/l2/l2b/r6/`. Aucun changement dans C++, Cargo.toml/Cargo.lock, models/, sound example/, VAD ou B8 ; aucun réseau ni push. Si un autre chemin est nécessaire : `PLAN_CHANGE_REQUIRED`, arrêt sans code.

## 2. Préflight obligatoire avant toute écriture de code

Confirmer branche et HEAD, état Git, diff hors lot, plan CLEAN, modèle et DLL présents en lecture seule avec leurs SHA, worker R5-2 connu et fixtures FR87/FR27/EN15/spkr0/x2/x6/x12. Préserver `models/` et `sound example/` non suivis. Tester la garde PowerShell à échec fermé **avant le premier Cargo**, cas OK et cas KO avec témoin prouvant qu'aucun Cargo n'est atteint. `PARAKEET_L2_RUNTIME_ROOT` vaut exactement la racine canonique `…/l2b` et sert uniquement à la lecture. `CARGO_TARGET_DIR`, `CARGO_HOME`, `TEMP`, `TMP`, sorties, journaux, archives, pending, TXT/SRT et toute racine `PARAKEET_*` d'écriture sont canoniquement sous `…/l2b/r6`; les variables d'entrées en lecture sont explicitement classées, existent et ont les SHA attendus. Refuser les liens/jonctions sortant de la racine et toute variable non classée. Réexécuter la garde avant chaque Cargo. Ne pas nettoyer R5-1 ou R5-2.

## 3. Tâches et cartes

| Tâche | Travail borné | Preuve d'acceptation |
| --- | --- | --- |
| R6-1 | Dans `native_engine.rs`, conserver la fusion des tokens de ponctuation sans changer l'intervalle lexical. Pour une suite consécutive de `k` mots lexicaux dont les intervalles identiques ou imbriqués se chevauchent d'au plus 80 ms et ne peuvent garder une durée positive par la règle R5, partager l'intervalle commun de `D` millisecondes entières : chaque mot reçoit `floor(D/k)` ms, puis les `D mod k` premières parts reçoivent chacune 1 ms supplémentaire. Préserver l'ordre, les bornes communes et la durée positive. Si `D < k`, erreur attribuée avec bornes brutes ; un chevauchement >80 ms reste erreur. Tout message d'erreur de normalisation inclut index, texte et bornes **brutes** des mots concernés. Chronométrer l'appel natif dans le worker via un journal de recette opt-in sous R6, absent en usage normal. | Tests `80/2 = 40+40 ms`, `80/3 = 27+27+26 ms`, trois mots, intervalle imbriqué, cas réel `the/eye.` à 618,880 s, `D<k` et chevauchement >80 ms avec bornes brutes ; anciennes assertions R5 conservées ou adaptées explicitement. Journal natif distinct du temps applicatif. |
| R6-2 | Déclarer dans `model_registry.rs` une seule constante nommée de durée **qualifiée produit**, initialement 10 min (dernier PASS applicatif de R5-2). L'utiliser dans `worker_ipc.rs` pour refuser Parakeet **avant pending** après obtention de la durée réelle ; `archive.rs` applique la même constante avant création du pending, en appel direct et avant reprise d'un pending existant trop long, conservé sans modification. Message exact « Fichier trop long pour Parakeet, utilisez Turbo », avec cause explicite pour la reprise. Pas de substitution Turbo. La borne technique native 30 min ne devient pas automatiquement une admission produit. | Tests à borne, juste au-dessus, source inconnue résolue puis refusée, reprise/pending existant conservé, Turbo intact ; aucun nouveau pending Parakeet créé après refus. |
| R6-3 | Compiler le nouveau worker uniquement sous R6, relever SHA worker/modèle/cinq DLL chargées, refaire A4 par le parcours applicatif : FR87, FR27, EN15, TXT/SRT, zéro `InvalidTimestamp`, `faire? / Je` et FR27 sans U+FFFD. Pendant la recette, une admission transitoire strictement Test peut être utilisée uniquement pour la nouvelle clé exacte ; elle doit être retirée avant le diff final. Après A4 PASS, inscrire la fiche Qualified Test et la clé successeur dans `model_registry.rs`, `worker_ipc.rs`/`archive.rs` selon les clés déjà en place ; refuser toute ancienne clé pour nouvel import et reprise, avec pending conservé. Après A4 FAIL : retirer la voie provisoire, ne pas promouvoir la clé. | A4 PASS/FAIL, artefacts TXT/SRT, tests clé exacte, ancienne clé et autre clé, absence de voie provisoire finale. |
| R6-4 | Recette de passe unique, **même nouveau binaire** : `spkr0.wav` 5 min, x2 10 min et x6 30 min par le vrai parcours applicatif dans un test ciblé de `parakeet_import.rs` ; EN60 x12 informative par `--qualification-reference` dans `native_import.rs`, sans admission produit. Le délai d'un cas est `durée_audio/4 + 120 s`. Chronométrer dans `native_engine.rs` l'appel `recognizer.transcribe` et journaliser seulement si `PARAKEET_R6_TIMING_LOG` pointe canoniquement sous R6 ; le test applicatif mesure séparément `StartImport` → TXT/SRT écrits. Le test distingue expiration et erreur du worker ; tableau PASS/FAIL/INCONCLUSIF et premier mot refusé. Pour essayer 30 min alors que la constante vaut provisoirement 10 min, ouvrir une voie de **recette temporaire** uniquement pour l'entrée x6 exacte, la clé Test exacte et R6 ; retirer cette voie avant le diff final. Après résultat : si 30 min PASS, constante R6-2 = 30 min ; sinon 10 min. Refaire les tests de borne et A4 sur l'état final. EN60 ne promeut jamais la limite au-delà de 30 min. | Tableau 5/10/30/60, temps natif et temps applicatif séparés, premier refus, limite finale, même SHA ; aucune voie x6 provisoire dans le diff final. |
| R6-5 | Vérifier format, build/test hors ligne verrouillés et tests ciblés ; faire revoir le candidat par `rust_review_coordinator`, au plus deux cycles revue/correction, puis commiter sur `wip/l2b-candidate` uniquement après CLEAN. | Diff borné, sorties des commandes, SHA, A4, limite finale, verdict reviewer. |

## 4. Recettes et commandes exactes

Base PowerShell, après préflight et garde OK, à réappliquer avant **chaque** Cargo :

```powershell
$r6 = 'C:\dev\whisper\target\parakeet-tests\l2\l2b\r6'
$env:CARGO_TARGET_DIR = "$r6\cargo"
$env:CARGO_HOME = "$r6\cargo-home"
$env:TEMP = "$r6\temp"
$env:TMP = "$r6\temp"
$env:PARAKEET_L2_RUNTIME_ROOT = 'C:\dev\whisper\target\parakeet-tests\l2\l2b'
$env:PARAKEET_L2_ARCHIVE_ROOT = "$r6\archive"
$env:PARAKEET_L2_OUTPUT_DIR = "$r6\reports"
$env:PARAKEET_L2_NATIVE_DIR = 'C:\dev\whisper\target\parakeet-tests\l2\l2b\native'
$env:PARAKEET_L2_MODEL = 'C:\dev\whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'
$env:PARAKEET_L2_WORKER = "$r6\worker\whisper-worker-parakeet-cpu.exe"
$env:PARAKEET_L2_FR87_WAV = 'C:\dev\whisper\target\parakeet-feasibility\spike02\fixtures\fr87.wav'
$env:PARAKEET_L2_FR27_WAV = 'C:\dev\whisper\target\parakeet-feasibility\spike02\fixtures\fr27.wav'
$env:PARAKEET_L2_EN_WAV = 'C:\dev\whisper\target\parakeet-feasibility\spike02\fixtures\en15.wav'
```

Les trois chemins courts désignent les fixtures du banc R5-2 vérifiées en lecture ; le préflight vérifie leurs SHA et l'absence de variable non classée. Copier le cache Cargo R5-2 existant, par chemins canoniques vérifiés, vers `$r6\cargo-home` avant le premier Cargo ; cette lecture est autorisée, et toutes les écritures de cache restent R6. Les sorties propres sont créées sous R6. La garde canonique vérifie également `PARAKEET_R6_RECIPE_WAV` comme **entrée en lecture seule**, `PARAKEET_R6_TIMING_LOG` comme écriture R6, et `PARAKEET_R6_CASE` comme identifiant sans chemin. Elle est testée OK puis KO avant le premier Cargo et invoquée à nouveau avant chaque ligne Cargo ci-dessous ; un KO empêche la commande. Aucune instrumentation de temps n'écrit si la variable de journal est absente. La valeur du journal doit être un chemin R6 existant ou dont le parent canonique est sous R6, sans lien sortant.

```powershell
# Binaire unique, avec la voie privée de référence pour EN60. Garde avant chacun.
cargo build --offline --locked -p whisper-worker-parakeet-cpu --features parakeet-qualification-reference
New-Item -ItemType Directory -Force "$r6\worker" | Out-Null
Copy-Item -LiteralPath "$r6\cargo\debug\whisper-worker-parakeet-cpu.exe" -Destination $env:PARAKEET_L2_WORKER
cargo test --offline --locked -p whisper-worker-parakeet-cpu --features parakeet-qualification-reference --test native_import r6_reference_en60 --no-run
cargo test --offline --locked -p whisper-adapters --test parakeet_import r6_application_single_pass_recipe --no-run
```

Après gel de cette copie privée R6 et relevé de son SHA, A4 court utilise la même clé et les fixtures FR87/FR27/EN15. Les constructions Cargo ultérieures peuvent remplacer `cargo/debug/…` mais ne modifient pas la copie qualifiée ; si le source worker change, reconstruire, recopier, rehasher et **refaire toutes les recettes de la nouvelle clé** :

```powershell
cargo test --offline --locked -p whisper-adapters --test parakeet_archive r5_2_a4_product_route_publishes_french_and_english_txt_srt -- --ignored --exact --nocapture
```

Pour chaque cas long, le test `r6_application_single_pass_recipe` prend la source indiquée, impose le profil import-single-pass, calcule le délai depuis la durée décodée (`durée_audio/4 + 120 s` : **195, 270, 570 s**) et mesure `StartImport` → présence de TXT et SRT. Le temps natif est écrit par le worker autour de `recognizer.transcribe`, en millisecondes monotones, dans un journal par cas ; vérifier une seule mesure pour le cas et l'identité de la source. Les chemins d'archive par cas sont sous `$r6\archive\{case}`. L'éventuel dépassement du délai est `FAIL délai de test` distinct de `InvalidTimestamp` et ne prétend pas qualifier le worker. La source x6 est testée avec la seule admission temporaire R6 décrite en R6-4, puis cette admission est retirée.

```powershell
$env:PARAKEET_R6_CASE = 'en5';  $env:PARAKEET_R6_RECIPE_WAV = 'C:\dev\whisper\sound example\spkr0.wav'; $env:PARAKEET_R6_TIMING_LOG = "$r6\logs\en5-native.jsonl"
cargo test --offline --locked -p whisper-adapters --test parakeet_import r6_application_single_pass_recipe -- --ignored --exact --nocapture
$env:PARAKEET_R6_CASE = 'en10'; $env:PARAKEET_R6_RECIPE_WAV = 'C:\dev\whisper\target\parakeet-feasibility\spike02\fixtures\synthetic\spkr0_x2.wav'; $env:PARAKEET_R6_TIMING_LOG = "$r6\logs\en10-native.jsonl"
cargo test --offline --locked -p whisper-adapters --test parakeet_import r6_application_single_pass_recipe -- --ignored --exact --nocapture
$env:PARAKEET_R6_CASE = 'en30'; $env:PARAKEET_R6_RECIPE_WAV = 'C:\dev\whisper\target\parakeet-tests\l2\l2b\r5-2\fixtures\spkr0_x6.wav'; $env:PARAKEET_R6_TIMING_LOG = "$r6\logs\en30-native.jsonl"
cargo test --offline --locked -p whisper-adapters --test parakeet_import r6_application_single_pass_recipe -- --ignored --exact --nocapture
```

EN60 est une **recette privée informative**, sans import applicatif ni calcul de constante. Le test `r6_reference_en60` lance le **même exécutable** avec `--qualification-reference`, vérifie son identité, donne un délai de **1020 s** (`3600/4+120`) et mesure l'appel natif dans le worker. Le temps total de parcours applicatif est **sans objet**, indiqué ainsi dans le tableau, et le temps total de cette voie privée peut être rapporté séparément.

```powershell
$env:PARAKEET_R6_CASE = 'en60'; $env:PARAKEET_R6_RECIPE_WAV = 'C:\dev\whisper\target\parakeet-feasibility\spike02\fixtures\synthetic\spkr0_x12.wav'; $env:PARAKEET_R6_TIMING_LOG = "$r6\logs\en60-native.jsonl"
cargo test --offline --locked -p whisper-worker-parakeet-cpu --features parakeet-qualification-reference --test native_import r6_reference_en60 -- --ignored --exact --nocapture
```

Après résultat 30 min, fixer **l'unique constante produit** à 30 min si PASS, sinon 10 min ; reconstruire le worker seulement si son code change, et refaire A4, tests d'admission/borne, format/build/tests ciblés avec `cargo fmt --all -- --check`, `cargo build --offline --locked --workspace`, `cargo test --offline --locked --workspace`. Les recettes longues peuvent être `NOT RUN` si A4 échoue ; ne pas promouvoir la nouvelle clé dans ce cas. Les journaux R6 conservent commandes, codes de sortie et temps natif/applicatif ; le livrable indique les tests réellement exécutés, pas les commandes seulement projetées.

## 5. Arbitrages fermés et garde-fous

**R6-P1, fermé par décision utilisateur.** EN60 informative emploie la voie privée existante. Elle n'est ni un PASS applicatif ni une qualification. `main.rs` et `decoder.rs` restent intacts.

**R6-P2, fermé par décision utilisateur.** `parakeet_ipc.rs` reste inchangé : ses bornes sont des millisecondes entières. Le partage R6-1 donne `80/3 = 27+27+26 ms`. Si `D<k`, l'erreur attribuée contient les bornes brutes ; aucune durée nulle n'est émise.

**R6-P3, fermé par décision utilisateur.** `archive.rs` et `tests/parakeet_archive.rs` peuvent changer **uniquement** pour la nouvelle clé, la garde de durée avant pending et le refus de reprise d'un pending trop long, laissé en place avec cause explicite.

## 6. Sortie de revue et arrêt

Le reviewer compare chaque exigence utilisateur au chemin et à l'essai qui la prouvent. `CLEAN` seulement si les trois arbitrages, le chronométrage, les recettes et la promotion conditionnelle de la nouvelle clé ont un chemin exécutable dans le périmètre. En cas de `NOT READY` au second cycle, arrêter à `PLAN_CHANGE_REQUIRED`, rapporter sans code et demander la décision de périmètre correspondante. Deux cycles de plan au maximum. B6 reste bénin et différé au lot VAD séparé ; B8 reste inactive.

### Verdict indépendant du cycle 1

**NOT READY — PLAN_CHANGE_REQUIRED.** Le reviewer confirme :

| ID | Critère concerné | Écart constaté | Arbitrage nécessaire |
| --- | --- | --- | --- |
| R6-P1, BLOQUANT | EN60 par parcours applicatif en passe unique | Au-delà de 30 min, l'application route vers VAD ; le décodeur ordinaire et le worker ordinaire plafonnent à 30 min. La voie `qualification-reference` 60 min existante n'est pas applicative. `main.rs` et `decoder.rs` sont hors chemins. | Autoriser EN60 informative par la voie privée directe, ou étendre précisément les chemins et contrats d'une recette applicative 60 min. |
| R6-P2, BLOQUANT | Parts égales, durée positive jusqu'à l'échantillon | L'IPC expose des millisecondes, soit 16 échantillons à 16 kHz. Un partage exact de 80 ms entre trois mots donne 426/427 échantillons et ne survit pas à la sérialisation actuelle. `parakeet_ipc.rs` est hors chemins. | Accepter une quantification en millisecondes avec son domaine de validité, ou autoriser une évolution de l'IPC et des consommateurs aux échantillons. |
| R6-P3, BLOQUANT | Refus de durée avant pending, également à l'archive | R6-2 prévoit une garde de durée dans `archive.rs`, mais ce fichier est autorisé seulement pour le changement de clé. | Étendre l'autorisation de `archive.rs` et de son test à la garde de durée/reprise, ou réduire explicitement la garantie aux seules entrées applicatives gardées dans `worker_ipc.rs`. |

Le reviewer demande aussi, à titre **IMPORTANT non bloquant**, de préciser les commandes, les features et la mesure du temps de calcul. Il n'a modifié aucun fichier. Le prochain cycle de revue attend les trois décisions ci-dessus ; une reformulation du plan ne peut pas créer les permissions manquantes.
