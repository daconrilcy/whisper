Avancée : R6 qualifie 30 min en profil Test ; les journaux EN5 et EN30 donnent le temps applicatif et l'inférence native, mais pas leur ventilation complète.
Blocage : la suite workspace R6 a trois échecs acceptés comme réserves par l'utilisateur ; R7 devra les traiter et revérifier la suite complète.
Décision : R6 et le plan R7 v2 sont autorisés au commit local ; R7 est autorisé à l'implémentation après revue de l'amendement de tests.

# PLAN-PARA-L2b-R7 v2 — diagnostic du temps applicatif et indépendance du test VAD

**Statut : CLEAN — PLANS READY v2 au second et dernier cycle.** Base de la revue : branche `wip/l2b-candidate`, HEAD `8a5b49b0`, worker R6 `1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0`. Le commit R6 avec réserves et le plan R7 v2 ont été autorisés par l'utilisateur le 17/10/2026. L'implémentation R7 est autorisée conditionnellement à la revue d'un amendement de deux tests. B8 reste inactive. La limite Test de passe unique de 30 min est acceptée. R7 ne modifie ni la VAD, ni l'admission produit. La revue v1 a conclu NOT READY sur R7-P1/R7-P2 ; ces constats sont fermés par la revue indépendante v2. Les ajustements ultérieurs de statut ne changent aucune tâche technique.

## 1. Condition héritée de R6

La garde R6 à échec fermé a été testée OK/KO avant `cargo test --offline --locked --workspace`, puis avant la relance complète avec `--no-fail-fast`. Cette dernière a échoué sur **trois** tests. L'utilisateur a ensuite autorisé le commit R6 avec ces trois réserves explicites :

| Test | Cause observée | Traitement dans R7 |
| --- | --- | --- |
| `r5_2_vad_recipe_is_scoped_and_desktop_test_import_is_rejected_before_pending` (`whisper-adapters/tests/parakeet_archive.rs:335`) | La recette est créée sous R6, alors que `archive.rs:2999-3013` n'admet que la racine `r5-2`. | R7-B : rendre le test indépendant de la racine du lot tout en conservant la portée de recette. |
| `b9_reports_noise_floor_silence_counts_and_cut_plans` (`whisper-adapters/tests/parakeet_long.rs:48`) | `PARAKEET_L2_NOISE_MINUS40_WAV` manque dans l'environnement du test complet. | Hors correction R7 : préflight/fixture à arbitrer avant de revendiquer une suite verte ; aucune VAD nouvelle. |
| `only_the_recorded_l2a2_cpu_test_key_is_qualified` (`whisper-core/tests/parakeet_import_contract.rs:170`) | L'assertion attend l'empreinte historique `545a...`, le registre R6 expose `1d8f...`. | Hors correction R7 : test de contrat à requalifier dans un lot autorisé ; R7 ne change pas `whisper-core`. |

Ces deux derniers échecs supplémentaires sont rapportés tels quels ; ils n'étendent pas automatiquement les chemins d'écriture de R7. Aucun test ou artefact R5-1/R5-2 n'est nettoyé.

## 2. Mesures disponibles et limites du diagnostic

Les deux recettes concernent le **même worker R6**. Le chronomètre applicatif va de `StartImport` à `last_result` ; les assertions de présence de `transcript.txt`/`transcript.srt` suivent ce chronomètre (`crates/whisper-adapters/tests/parakeet_import.rs:204-239`). L'appel natif est chronométré autour de `recognizer.transcribe` (`crates/whisper-worker-parakeet-cpu/src/native_engine.rs:60-69`). Valeurs des dernières entrées de `target/parakeet-tests/l2/l2b/r6/logs/en5-application.jsonl`, `en30-application.jsonl`, `en5-r2-native.jsonl` et `en30-r2-native.jsonl` :

| Étape | EN5 (s) | EN30 (s) | Nature de la preuve |
| --- | ---: | ---: | --- |
| Parcours applicatif total, demande → résultat | 176,102 | 394,755 | Mesuré, inclut admission, worker, archive et polling du test ; TXT/SRT vérifiés ensuite. |
| Inférence native `recognizer.transcribe` | 22,435 | 147,510 | Mesuré dans le worker ; la normalisation vient après ce chronomètre. |
| Tout le reste du parcours applicatif, différence des deux lignes précédentes | 153,667 | 247,245 | Calculé ; **pas** une mesure d'une étape. |
| Démarrage du worker et attente du Ready | non mesuré | non mesuré | À instrumenter. |
| Chargement du modèle et des DLL | non mesuré | non mesuré | À instrumenter dans le worker. |
| SHA modèle, worker, cinq DLL, source | non mesuré | non mesuré | Sites d'appel identifiés, nombre dynamique de lectures et durée non mesurés. |
| Inspection/décodage audio | non mesuré | non mesuré | À distinguer entre adaptateur, archive et worker. |
| Normalisation et validation des mots | non mesuré | non mesuré | À instrumenter autour de `normalize_words` et de la validation adaptateur. |
| Archive : admission, pending, segments, publication | non mesuré | non mesuré | À instrumenter par sous-étapes. |
| TXT/SRT | non mesuré | non mesuré | À instrumenter jusqu'à la publication ; la vérification du harness doit rester séparée. |
| Harness : compilation, préparation/copies | hors chronomètre applicatif ; durée non mesurée | hors chronomètre applicatif ; durée non mesurée | `started` est pris juste avant `StartImport` ; Cargo et copies du worker sont antérieurs. |

Les diagnostics `archive/recipes/{en5,en30}/.whisper-diagnostics/worker-0.jsonl` enregistrent notamment `backend_ready` puis `progress`, mais ni le départ de la demande ni les frontières entre chargement, hash, décodage et archive. Leur écart Ready→progress (EN5 22,950 s ; EN30 153,606 s) confirme l'ordre de grandeur de l'inférence, **sans** prouver à lui seul la durée de chacune des étapes. Les causes probables, à tester et non affirmées comme mesurées, sont les lectures répétées du modèle/fichiers pour SHA, le décodage et les écritures d'archive qui croissent avec la durée ou le nombre de mots, ainsi que le chargement fixe du modèle/DLL. Le polling à 100 ms peut ajouter une faible latence, sans expliquer seul 153 s.

Inspection statique initiale : `parakeet_import.rs:67-78,98-112,127-143,151-163` lit le modèle, le worker et cinq DLL pour la clé/qualification ; `archive.rs:1557-1562,1604-1626` relit la source et le modèle pendant l'identification et la préparation ; `whisper-worker-parakeet-cpu/src/main.rs:63-72,315-337,667-680` relit le modèle, le worker, cinq DLL et la source. Ces **sites** ne donnent pas le nombre d'appels effectifs par recette : R7 le comptera par fichier, phase et octets lus. Aucune suppression de vérification SHA n'est présumée.

## 3. R7-A — diagnostic chronométré, sans optimisation

But : obtenir deux tableaux EN5/EN30 dont la somme des phases, avec les attentes et arrondis documentés, recouvre le temps demande→résultat, puis mesurer séparément la vérification TXT/SRT. Garder les mesures séparées : **produit** (adaptateur, worker, archive) ; **harness** (garde, compilation, copies, lancement du test, vérification finale). Rejouer une fois EN5 et EN30, mêmes audio, modèle, worker/DLL et profil Test, uniquement après préflight et autorisation R7. Aucun changement de texte, timestamps, clés ou critères de qualification.

**Stratégie de mesure sans nouvelle admission :** deux voies complémentaires sur chaque audio, à ne jamais confondre. (1) Le parcours applicatif utilise le **worker R6 gelé**, SHA `1d8f...`, et la clé Qualified Test existante ; seules l'adaptateur, l'archive et le harness reçoivent des sondes opt-in. Aucun binaire worker différent ne lui est présenté. (2) Une **copie diagnostique privée** du worker est compilée avec des sondes dans `main.rs`/`native_engine.rs` ; elle s'exécute directement depuis `native_import.rs`, en profil `import-single-pass`, avec les mêmes sources, modèle et DLL, mais **sans** `ImportApplication`, archive ou `ArchiveQualificationKey`. Son SHA distinct est relevé et indiqué dans les résultats ; il ne devient jamais une clé admissible. Ainsi, les sous-étapes worker privées sont indicatives et ne sont pas additionnées comme si elles appartenaient exactement au parcours du worker gelé. Le tableau distingue les deux voies ; toute discordance supérieure à la précision des sondes reste `INCONCLUSIF`.

Instrumentation minimale opt-in et confinée à R7 : horloge monotone par processus, événements de début/fin ; identifiant de recette ; pour chaque SHA, rôle du fichier, nom ou identité déjà connue, taille, nombre de lectures et durée. Aucun contenu audio ou modèle journalisé. La copie privée indique chargement du modèle/DLL, préparation Ready, hash, décodage, appel natif et normalisation. L'adaptateur/son archive du parcours gelé indiquent vérification de clé, spawn/Ready, validation des mots, identité source, préparation/pending, persistance et publication TXT/SRT. Le harness relève séparément garde, Cargo, copie du binaire **privé** et vérification finale. Les durées du worker et de l'hôte utilisent chacune leur horloge monotone ; aucune soustraction directe entre processus. Une phase `attente/IPC/autre` porte le résiduel applicatif, sans l'attribuer arbitrairement au modèle ou à l'archive.

**Chemins de code proposés, à confirmer au préflight R7 :** `crates/whisper-adapters/src/parakeet_import.rs`, `src/worker_ipc.rs`, `src/archive.rs`, `tests/parakeet_import.rs`, `crates/whisper-worker-parakeet-cpu/src/main.rs`, `src/native_engine.rs`, `tests/native_import.rs`. `src/decoder.rs` n'est pas ouvert : un chronomètre autour de son appel depuis `main.rs` suffit. La source `archive.rs` peut recevoir uniquement des sondes opt-in pour R7-A ; sa règle VAD reste byte-for-byte inchangée. Toute modification de `model_registry.rs`, du contrôle d'admission, de l'FFI ou d'un autre chemin exige `PLAN_CHANGE_REQUIRED`. Aucun ajout de dépendance, aucun changement Cargo, aucun réseau. Écriture d'exécution proposée : `target/parakeet-tests/l2/l2b/r7/` uniquement ; worker R6, modèle, DLL et audio sont en lecture seule. La garde à échec fermé est testée OK/KO avant le premier Cargo.

**Acceptation R7-A :** tableau EN5/EN30 par étape en secondes, compte SHA et fichier, colonnes `produit gelé`, `worker privé` et `harness` distinctes. Les sommes des phases hôte et du résiduel recouvrent le nouveau total applicatif ; les sous-phases worker privé restent une décomposition diagnostique séparée. Comparer les totaux nouveaux aux 176,102/394,755 s historiques et les inférences privées aux 22,435/147,510 s, en signalant l'effet éventuel des sondes ; ne pas traiter l'écart comme qualité ou admission. Si l'écart interdit d'attribuer le résiduel, conclure `INCONCLUSIF` pour la cause précise. Aucun changement de clé ni nouvelle fiche produit.

## 4. R7-B — test R4 indépendant du nom du lot

Le test `r5_2_vad_recipe_is_scoped_and_desktop_test_import_is_rejected_before_pending` construit aujourd'hui `recipe_destination` sous `archive_root()` R6 (`parakeet_archive.rs:26-37,325-335`) ; la règle produit `is_r5_2_vad_recipe_destination` exige `PARAKEET_L2_ARCHIVE_ROOT` sous `PARAKEET_L2_RUNTIME_ROOT/r5-2` et la destination sous cette archive (`archive.rs:2999-3013`). Le plan conserve cette règle et ses assertions : clé VAD exacte admise dans la recette, import Test ordinaire refusé avant création du dossier/pending.

**Exécution fixée :** le test parent, lancé sous la garde R7 avec l'ancre normale L2b, prépare `R7/recipe-scope` après contrôle canonique de R7 et des parents existants. Il démarre **son propre exécutable de test** (`current_exe`) en processus enfant avec `--exact r5_2_vad_recipe_is_scoped_and_desktop_test_import_is_rejected_before_pending --nocapture`, via `Command::env` pour `PARAKEET_R7_VAD_CHILD=1`, `PARAKEET_L2_RUNTIME_ROOT=R7/recipe-scope` et `PARAKEET_L2_ARCHIVE_ROOT=R7/recipe-scope/r5-2/archive`. Aucune mutation globale de l'environnement parent. Dans la branche enfant, avant toute création : canonicaliser l'ancre, son enfant `r5-2`, l'archive et la destination future ; exiger ancre sous R7, archive sous `ancre/r5-2`, destination acceptée sous archive. Le helper `archive_root()` qui exige `runtime/r6` est **contourné uniquement dans ce scénario enfant** par la racine injectée après validation ; tous les autres tests gardent le helper et l'ancre R6 actuelle. L'enfant exécute les assertions R4 existantes, puis rend son code de sortie au parent. Le parent échoue si l'enfant échoue. La destination `ordinary-desktop-test-import` est créée seulement comme chemin à tester, sans `create_dir_all`, et doit rester absente après refus. `archive.rs` et son prédicat `is_r5_2_vad_recipe_destination` ne changent pas.

**Chemin proposé pour R7-B :** `crates/whisper-adapters/tests/parakeet_archive.rs` uniquement. `archive.rs` reste inchangé pour R7-B. Test ciblé sous garde OK, puis contrôle explicite du pending accepté et de l'absence de pending/destination rejetés. Aucun changement de la logique VAD ou B8. L'exception d'ancre synthétique est interne à l'enfant du test d'archive, après validation sous R7 : elle n'est pas une exception aux variables du processus Cargo ou du parcours applicatif.

## 5. Commandes prévues et confinement

Ces commandes sont **planifiées, non exécutées**. Le script de garde R7 à créer sous R7 classe `PARAKEET_L2_RUNTIME_ROOT=.../l2b` comme ancre de lecture, toutes les racines d'écriture (`CARGO_TARGET_DIR`, `CARGO_HOME`, `TEMP`, `TMP`, archive, logs, worker privé) sous R7, modèle/DLL/audio/worker R6 gelé comme lectures avec SHA attendus. Il teste les cas OK et KO avant le premier Cargo ; chaque invocation ci-dessous réapplique la garde. L'exception enfant R7-B n'est créée qu'après ce contrôle, via `Command::env` et les validations test-only ci-dessus.

```powershell
$r7 = 'C:\dev\whisper\target\parakeet-tests\l2\l2b\r7'
& "$r7\guarded-cargo-r7.ps1" -GuardOnly
& "$r7\guarded-cargo-r7.ps1" -GuardOnly -GuardTestBad   # doit échouer avant Cargo
& "$r7\guarded-cargo-r7.ps1" test --offline --locked -p whisper-adapters --test parakeet_import r7_application_timing --no-run
& "$r7\guarded-cargo-r7.ps1" test --offline --locked -p whisper-worker-parakeet-cpu --test native_import r7_private_worker_timing --no-run
$env:PARAKEET_L2_WORKER = 'C:\dev\whisper\target\parakeet-tests\l2\l2b\r6\worker\whisper-worker-parakeet-cpu.exe' # lecture, voie applicative uniquement
& "$r7\guarded-cargo-r7.ps1" build --offline --locked -p whisper-worker-parakeet-cpu
New-Item -ItemType Directory -Force "$r7\worker-private" | Out-Null
Copy-Item -LiteralPath "$r7\cargo\debug\whisper-worker-parakeet-cpu.exe" -Destination "$r7\worker-private\whisper-worker-parakeet-cpu.exe"
$env:PARAKEET_R7_PRIVATE_WORKER = "$r7\worker-private\whisper-worker-parakeet-cpu.exe" # test natif privé seulement, SHA relevé
$env:PARAKEET_R7_CASE = 'en5'; $env:PARAKEET_R7_RECIPE_WAV = 'C:\dev\whisper\sound example\spkr0.wav'
& "$r7\guarded-cargo-r7.ps1" test --offline --locked -p whisper-adapters --test parakeet_import r7_application_timing -- --ignored --exact --nocapture
& "$r7\guarded-cargo-r7.ps1" test --offline --locked -p whisper-worker-parakeet-cpu --test native_import r7_private_worker_timing -- --ignored --exact --nocapture
$env:PARAKEET_R7_CASE = 'en30'; $env:PARAKEET_R7_RECIPE_WAV = 'C:\dev\whisper\target\parakeet-tests\l2\l2b\r5-2\fixtures\spkr0_x6.wav'
& "$r7\guarded-cargo-r7.ps1" test --offline --locked -p whisper-adapters --test parakeet_import r7_application_timing -- --ignored --exact --nocapture
& "$r7\guarded-cargo-r7.ps1" test --offline --locked -p whisper-worker-parakeet-cpu --test native_import r7_private_worker_timing -- --ignored --exact --nocapture
& "$r7\guarded-cargo-r7.ps1" test --offline --locked -p whisper-adapters --test parakeet_archive r5_2_vad_recipe_is_scoped_and_desktop_test_import_is_rejected_before_pending -- --exact --nocapture
```

EN5 et EN30 ont respectivement un délai applicatif de 195 et 570 s (`durée/4 + 120`) ; le test privé a le même garde-fou, chronométré séparément. Sorties : journaux structurés et TXT/SRT applicatifs sous `$r7\archive\{en5,en30}`, rapports de temps sous `$r7\logs`, copie du worker diagnostique sous `$r7\worker-private`, et fixture d'archive sous `$r7\recipe-scope`. La préparation du worker privé, les copies et chaque commande `--no-run` sont chronométrées par le harness, hors total applicatif. Les noms `r7_application_timing` et `r7_private_worker_timing` désignent les tests à créer dans R7, pas des tests déjà présents.

Le test applicatif lit exclusivement `PARAKEET_L2_WORKER` (copie R6 gelée) et vérifie son SHA `1d8f...` avant `StartImport`. Le test natif privé lit exclusivement `PARAKEET_R7_PRIVATE_WORKER`, vérifie son SHA consigné, lance directement son IPC et n'utilise ni la clé du registre ni `ArchiveStore`. La garde classe la première variable comme lecture R6 et la seconde comme lecture/écriture sous R7 ; toute inversion est un échec de préflight. `Copy-Item` est chronométré et refuse d'écraser une copie privée dont le SHA diffère sans nouveau run complet EN5/EN30. La commande `build` et la copie sont exécutées une seule fois après le gel du code instrumenté, avant les deux recettes.

## 6. Préflight, ordre et revue

1. Avant tout code : constater branche/HEAD et le commit R6 livré avec trois réserves, conserver `models/` et `sound example/` en lecture, vérifier clés/SHA et entrées EN5/EN30. L'implémentation R7 reste soumise au verdict de la revue de son amendement de tests.
2. Après accord écrit R7 seulement : installer et tester la garde à échec fermé pour toutes les variables de lecture/écriture, cas OK et KO avant le premier Cargo ; isoler Cargo, TEMP/TMP, logs, archives et sorties sous R7.
3. R7-A : garder le worker R6 gelé pour la voie applicative ; compiler séparément le worker de diagnostic, sans l'introduire dans une clé d'archive ; relever EN5/EN30 une fois par voie. Documenter temps produit/worker privé/harness et comptes SHA. Si le résultat manque une phase majeure, signaler `INCONCLUSIF` sans inférer une cause certaine.
4. R7-B : injecter la racine de test, exécuter les assertions R4, vérifier la règle inchangée dans le diff. Ne pas transformer les autres échecs workspace en PASS ; les reporter séparément.
5. Faire relire indépendamment le candidat par les rôles de revue Rust, au plus deux cycles revue/correction. R7 ne résout pas à lui seul les deux échecs hors périmètre de la suite workspace. Pas de push ni d'activation B8.

Cette version documentaire est soumise à la revue indépendante. Aucun code R7 n'est autorisé avant la décision écrite de l'utilisateur.
