Avancée : A4 et les passes uniques applicatives de 5, 10 et 30 min passent sur la même clé R6 ; EN60 passe par la voie privée informative.
Blocage : la suite workspace complète présente trois échecs de tests liés à l'environnement ou à une attente historique ; le reviewer n'a pas rendu CLEAN après deux cycles.
Décision : R6 livré avec ces trois réserves et commit local autorisé par l'utilisateur le 17/10/2026 ; la limite Test de 30 min est acceptée. B8 reste inactive.

# Fiche R6 — Parakeet Qualified Test, passe unique CPU

## Identité et périmètre

Plan [PLAN-PARA-L2b-R6 v2](plan-parakeet-l2b-r6.md) revu **CLEAN — PLANS READY** au commit `922c6592`, décision utilisateur du 17/10/2026. Branche `wip/l2b-candidate`. Le binaire de recette et de Test est la copie figée `target/parakeet-tests/l2/l2b/r6/worker/whisper-worker-parakeet-cpu.exe`, SHA-256 `1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0`. Modèle `parakeet-tdt-0.6b-v3.q8_0.gguf`, SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`. Runtime natif CPU agrégé `d5a2cc8ef441dd5934972adae00f997241bea17a7bc5977a8c7f0122aaa958ff`.

| DLL réellement chargée | SHA-256 |
| --- | --- |
| `nemo_speech_asr_c.dll` | `9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764` |
| `nemo_speech_asr.dll` | `923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019` |
| `ggml.dll` | `2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794` |
| `ggml-base.dll` | `398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a` |
| `ggml-cpu.dll` | `f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0` |

Qualification **Test, CPU, import-single-pass** uniquement, en FR/EN sur le corpus indiqué ci-dessous, livrée avec les réserves ci-dessous. La constante acceptée est **30 min = 28 800 000 échantillons à 16 kHz** : plus longue durée passée par le parcours applicatif avec ce SHA. Au-delà, l'import Parakeet est refusé **avant pending** avec « Fichier trop long pour Parakeet, utilisez Turbo ». Aucune bascule silencieuse. Un pending antérieur au-delà de la borne reste conservé mais sa reprise est refusée avec cause explicite. Les anciennes clés Parakeet sont refusées pour un nouvel import ; leurs pending ne sont pas effacés. Turbo reste inchangé. **B8/import-vad reste inactive** côté produit ; la VAD adaptative est un lot ultérieur.

## Recettes sur la clé finale

Le temps **natif** est le chronométrage monotone de `recognizer.transcribe` dans le worker ; le temps **applicatif** va de `StartImport` au résultat. La présence des deux fichiers TXT/SRT est vérifiée juste après. Les nombres sont distincts. Tous les cas ci-dessous utilisent le binaire SHA `1d8f57b9…` ; les campagnes sur des SHA antérieurs ne qualifient pas cette fiche. Les journaux et sorties sont sous `target/parakeet-tests/l2/l2b/r6/`, hors Git.

| Cas | Audio et type | Délai | Temps natif | Temps applicatif / test | Résultat |
| --- | --- | ---: | ---: | ---: | --- |
| A4 | FR87, FR27, EN15 réels | 300 s **par fichier** | journal R6 par fichier | 406,62 s pour la série | **PASS** : TXT/SRT pour les trois, zéro `InvalidTimestamp`, `faire? / Je`, FR27 sans U+FFFD |
| EN5 | `spkr0.wav` réel, 4 800 000 échantillons | 195 s | 22,435 s | 176,102 s | **PASS** : TXT/SRT |
| EN10 | `spkr0_x2.wav` synthétique, 9 600 000 échantillons | 270 s | 47,020 s | 219,991 s | **PASS** : TXT/SRT |
| EN30 | `spkr0_x6.wav` synthétique, 28 800 000 échantillons | 570 s | 147,510 s | 394,755 s | **PASS** : TXT/SRT, sans admission provisoire x6 |
| EN60 | `spkr0_x12.wav` synthétique, 57 600 000 échantillons, `--qualification-reference` | 1 020 s | 306,337 s | 328,01 s **test privé**, temps applicatif sans objet | **PASS informatif** : 11 689 mots, non admis dans le produit et sans effet sur la constante |

EN10, EN30 et EN60 sont des répétitions de l'enregistrement anglais réel de 5 min ; leurs frontières de montage ne constituent pas trois enregistrements distincts. EN60 vérifie notamment la correction du refus `the / eye.` de B5. Le partage des mots sur une même trame est en **millisecondes entières** (`80/2 = 40+40`, `80/3 = 27+27+26`) ; quand `D<k`, l'erreur contient les bornes brutes. Cette règle ne mesure pas la qualité absolue de transcription.

## Vérifications et limites

Préflight R6 : modèle, worker, cinq DLL et fixtures vérifiés ; garde PowerShell testée **OK et KO avant le premier Cargo**, puis rejouée avant chaque commande. L'ancre `PARAKEET_L2_RUNTIME_ROOT` est exactement `…/l2b` en lecture ; Cargo, TEMP/TMP, journaux, archives, pending et sorties restent sous `…/l2b/r6`. Le cache hors ligne a été copié sous R6 ; aucun téléchargement, aucune écriture dans `models/` ou `sound example/`.

Tests ciblés du normaliseur, de la borne exacte et du dépassement d'un échantillon, des refus de reprise avec pending conservé, de l'ancienne clé et d'A4 : **PASS**. `cargo fmt --all -- --check`, `cargo build --offline --locked --workspace` et `git diff --check` : **PASS**. La suite `cargo test --offline --locked --workspace` a été relancée après R1/R2 sous garde R6, puis avec `--no-fail-fast` pour exécuter toutes les cibles. Elle présente trois échecs, listés ci-dessous ; aucun autre échec n'a été observé. Les tests `#[ignore]` n'ont pas été exécutés dans cette commande et ne sont pas comptés PASS.

**Verdict du reviewer après deux cycles : LIVRÉ AVEC RÉSERVES ; R1–R3 fermés, R4 ouvert.** L'utilisateur a accepté le commit local avec les trois réserves de test suivantes, sans les requalifier en PASS :

| Réserve | Test et échec observé | Portée |
| --- | --- | --- |
| R4 | `r5_2_vad_recipe_is_scoped_and_desktop_test_import_is_rejected_before_pending` : archive de recette sous R6 alors que la règle `archive.rs:2999–3013` attend R5-2. | Test historique lié à la racine du lot ; aucune inférence VAD lancée et B8 reste inactive. |
| B9 | `b9_reports_noise_floor_silence_counts_and_cut_plans` : `PARAKEET_L2_NOISE_MINUS40_WAV is required`. | Fixture/variable d'environnement absente de la suite workspace ; B9 n'est pas établi par cette relance. |
| Contrat de clé L2a-2 | `only_the_recorded_l2a2_cpu_test_key_is_qualified` attend `545a…` alors que le registre R6 expose `1d8f…`. | Assertion historique obsolète après R5-2/R6 ; elle ne prouve pas un refus du nouveau worker. |

Ces trois réserves concernent les tests et leur environnement. Aucun échec produit additionnel n'a été démontré par cette relance. L'amendement R7 prévoit de traiter chacune dans son périmètre autorisé, sans modifier la règle VAD ni activer B8. Aucun push.

Réserves : seuls le français court et l'anglais jusqu'à 30 min sont éprouvés par ces recettes, avec 10/30 min synthétiques ; les autres langues, GPU Parakeet, qualité contre vérité terrain, VAD adaptative et paquet final restent hors de cette qualification. Les 30 min constituent une limite **Test** fondée sur le plus long PASS, pas une garantie universelle de performance ou de qualité.
