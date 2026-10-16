Avancée : design CPU/import L2 relu CLEAN ; L1b-1 et L1b-2 sont committés.
Blocage : l'intégration native, la qualification du candidat Test et la VAD longue ne sont pas exécutées.
Décision : proposer L2a puis L2b, avec accords d'implémentation distincts ; ce plan ne les autorise pas.

# PLAN-PARA-L2 DRAFT v3 — import Parakeet CPU

**Parents Git :** design `docs/cadrage/evo-parakeet.md` §9 et `docs/cadrage/evo-engine-capabilities.md` §8 au commit `3c47f272`, verdict indépendant **CLEAN — DESIGN READY-L2** sur CPU/import FR/EN ; L1b-2 `84e5444d` et L1b-1 `7de28359`, tous deux revus CLEAN. Le bilan `bilan-parakeet-faisabilite-cloture.md` est une preuve de faisabilité isolée, pas une qualification produit. Contributions en lecture seule `ARCH-PARA-L2 v1`, `VAD-L2 v1`, `REQ-PARA-L2 v1` et `rust_plan_writer` ; rédaction/versionnement par la session principale. **Statut : PLANS READY-L2a au verdict indépendant v2 ; L2b livré avec réserve de revue sur la correction R2 apportée en v3.** Aucun code L2 n'est autorisé par ce document.

## 1. Résultat, limites et gates

L2a ajoute l'import Parakeet **CPU manuel**, WAV/MP3, Auto **langue** FR/EN, texte et TXT/SRT, pour une durée source ≤1 800 s. Avant qualification du worker Test exact, Parakeet reste grisé ; un harness privé permet la preuve sans contourner l'UI. Après PASS FR et EN sur le candidat exact, seule la fiche Test `import-single-pass` peut être versionnée et admise. L2b ajoute le parcours >1 800 s par VAD et sa recette de 60 min ; son échec laisse L2a utilisable et le long refusé avec cause. Les préférences et la matrice Turbo L1b sont préservées. GPU et Auto **calcul** Parakeet relèvent de L3, microphone de L4, acquisition de L5, paquet Final et `AC-PARA-16F` de L6, comparaison de qualité absolue de L7. Les 23 autres langues restent hors périmètre qualifié.

| Gate | Condition nécessaire | En absence |
| --- | --- | --- |
| G-L2-DESIGN | Parents et verdict exacts ci-dessus, décisions utilisateur du 16/10 sur le silence : prochain silence après 120 s, borne 30 min puis erreur, aucune coupe forcée. | `DESIGN_CHANGE_REQUIRED`. |
| G-L2-PLAN | Ce plan et ses chemins revus, acceptés, puis accord écrit précis **dans la session d'implémentation** pour L2a ou L2b. L'accord L1b et l'ancien spike ne couvrent pas L2. | Aucune écriture de code du sous-lot. |
| G-L2-NATIVE | Pour L2a, accord précis couvrant l'ABI C NeMo, Rust `unsafe` privé, chargement Windows de DLL existantes et copie bornée sous `target/parakeet-tests/l2/l2a/`. Aucune modification C++ ni acquisition. | Contrats purs possibles seulement si un sous-périmètre est accepté ; recette native `NOT RUN`, aucune fiche Test activée. |
| G-L2-TEST | Fichier Test Parakeet présent avec taille/SHA attendus ; worker, ABI, DLL, protocoles, sources FR/EN et sorties attribués à la clé exacte. | Recette affectée `NOT RUN`, sans téléchargement ni écriture sous `models/`; Parakeet grisé. |
| G-L2A-QUAL | Worker produit CPU et harness privé donnent `PASS` FR **et** EN sans langue imposée, temps valides, même build/artefact/runtime/protocole que la fiche. Tout changement de clé impose de refaire les deux essais courts. | Pas de fiche `Qualified` Test ni d'admission GUI ; `FAIL`/`INCONCLUSIVE` visibles. |
| G-L2B | L2a revu CLEAN ; algorithme VAD du design, référence Test-only 60 min et corpus local vérifiés ; accord écrit propre à L2b. | Import >30 min refusé ; L2a inchangé. |

La preuve Test `AC-PARA-16-L2` est une `PRODUCT_VALIDATION` ; elle n'est pas transférée au paquet Final de `AC-PARA-16F` (`DELIVERY_QUALIFICATION`). `language_count=0` est attendu, sans critère d'échec. Aucun seuil nouveau de vitesse, RAM, VRAM ou écart global de transcription n'est introduit. Aucun worker Parakeet GPU ou live n'est lancé en L2.

## 2. Contrats communs à L2a et L2b

**Admission et profils.** La clé lie modèle/SHA, runtime/build natif, worker/build, protocole, backend CPU, profil et matériel pertinent. `import-single-pass` (≤1 800 s), `import-vad` (>1 800 s), live et référence de qualification ne partagent pas une preuve. La durée décodée et le parcours déterminent le profil avant effet, puis le même garde contrôle start, enqueue, process et reprise. La requête est liée au chemin du catalogue vérifié ; un chemin/empreinte arbitraire ou une dimension divergente est refusé. **Langue manuelle Parakeet**, GPU, Auto calcul et live restent refusés. La fiche Test est une donnée de code versionnée dans `model_registry.rs` **après** G-L2A-QUAL, sans nouveau manifeste ou fichier de qualification ; la composition vérifie sa clé à chaque lancement. Si le code ne peut représenter et vérifier cette clé exacte sans persistance ad hoc : `DESIGN_CHANGE_REQUIRED` avant activation.

**IPC et native.** Un protocole **Parakeet v1 séparé** porte modèle/artefact/révision/SHA, runtime/build, worker/protocole, backend/profil, source/SHA, nombre d'échantillons, job/génération/tentative, plages et identité de réponse. Le protocole Turbo v2 (`ipc.rs`) reste intact. Les messages `Prepare/Ready/BeginImport`, mots/progrès bornés, fin/erreur/Stop/Shutdown refusent version ou identité incohérentes avant audio ; les résultats tardifs ne franchissent pas la génération. Le worker Parakeet CPU est un nouveau processus Rust sans `whisper-rs`. Sa façade ABI C privée est la seule zone `unsafe`, avec invariants de layout, durée de vie des `CString`/PCM et propriété des pointeurs. Elle copie mots et texte en Rust possédé avant destruction des résultats. Le module est chargé **une fois par processus** et conservé jusqu'à sa fin : aucun `FreeLibrary` après inférence, même sur erreur. L'hôte ne charge aucune DLL NeMo. `target/parakeet-feasibility/spike02/worker.rs` est la référence de cycle de vie ; l'ancien `wrapper.rs` ne l'est pas.

**Transport et sorties.** Le worker décode la source/plage attestée en PCM mono 16 kHz ; jusqu'à 115,2 Mo de `f32` pour 30 min ne traversent pas les trames IPC de 1 Mio. Les résultats sont fractionnés en messages bornés. L'offset absolu est calculé en échantillons ; la trame TDT vaut 80 ms et le padding final n'allonge pas l'audio réel. Seul le **dernier mot du fichier**, si son dépassement brut est ≤80 ms, peut recevoir `end=min(end,durée_audio)`. Débordement interne, dépassement final >80 ms ou plage inversée interdisent la confirmation avec cause. Tout mot confirmé satisfait `0 ≤ start < end ≤ durée_audio` ; TXT/SRT sont ordonnés sur cette timebase. La source est préservée, `Complete` n'apparaît qu'après acquittements de publication. Stop/Quit avant engagement invalide la tentative, garde le pending récupérable et rejette les résultats tardifs ; après engagement, annonce que l'arrêt est trop tardif et attend l'issue réelle. `kill` demandé n'est pas une sortie physique constatée. Aucun IO, attente ou calcul natif bloquant sur le fil UI.

**Sondes.** Corréler release, modèle, worker, calcul demandé/effectif, job/génération/tentative, début/fin de vérification, préparation, premier audio, appel natif, confirmation, Stop, sortie et chaque publication. Une durée monotone n'est soustraite qu'à l'intérieur d'un même processus ; les sondes ne sont ni une mesure de charge, ni un benchmark. Réutiliser le journal local prévu sans nouveau fichier de preuve signé.

## 3. L2a — passe unique CPU, neuf tâches

| ID | Sortie démontrable | Cartes |
| --- | --- | --- |
| L2a.1 | Clés par profil/parcours/durée, refus Parakeet hors CPU/import/Auto langue et non-héritage Test/Final. | A1/A8 |
| L2a.2 | IPC Parakeet v1 versionné et routeur distinct, erreurs d'identité, messages bornés, Turbo v2 inchangé. | A2/A5/A10 |
| L2a.3 | Worker CPU, façade ABI, chargement privé et cycle de vie sans déchargement après inférence. | A3/A5 |
| L2a.4 | Décodage WAV/MP3, passe unique ≤1 800 s, mots possédés, temps validés. | A3/A4 |
| L2a.5 | Garde et routage à préparer/lancer/file/traitement/reprise, composition Test, refus explicite >30 min ; archive qualifiant l'identité Turbo historique ou Parakeet Test exacte. | A1/A5/A7/A8 |
| L2a.6 | Stop/Quit, sortie physique, résultat tardif, pending/reprise et publication TXT/SRT. | A6/A7/A9 |
| L2a.7 | Harness privé et recette native FR+EN du **candidat produit exact**, sans admission GUI anticipée. | A3/A8 |
| L2a.8 | Seulement après G-L2A-QUAL : fiche Test versionnée, activation GUI Test et vérification de clé ; Final/live/GPU restent bloqués. | A1/A8 |
| L2a.9 | Régression Turbo, diff/tests/build et revue indépendante. | A10 |

**`lot.paths.write` L2a, exclusifs :**

```text
Cargo.toml
Cargo.lock
crates/whisper-core/src/lib.rs
crates/whisper-core/src/application.rs
crates/whisper-core/src/model_registry.rs
crates/whisper-core/src/model_selection_app.rs
crates/whisper-core/src/parakeet_ipc.rs
crates/whisper-core/tests/model_selection_app.rs
crates/whisper-core/tests/parakeet_import_contract.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/src/parakeet_import.rs
crates/whisper-adapters/tests/parakeet_archive.rs
crates/whisper-adapters/tests/parakeet_import.rs
crates/whisper-adapters/tests/parakeet_control.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-desktop/tests/model_selection_ui.rs
crates/whisper-worker-parakeet-cpu/Cargo.toml
crates/whisper-worker-parakeet-cpu/src/main.rs
crates/whisper-worker-parakeet-cpu/src/native_engine.rs
crates/whisper-worker-parakeet-cpu/src/native_ffi.rs
crates/whisper-worker-parakeet-cpu/src/decoder.rs
crates/whisper-worker-parakeet-cpu/tests/native_import.rs
```

Le manifeste racine et `Cargo.lock` ne changent que pour enregistrer la nouvelle crate et des versions déjà présentes en cache offline ; aucun téléchargement. `engine_capabilities.rs`, `ports.rs`, `ipc.rs`, les workers Turbo et les manifestes existants restent en lecture. `archive.rs` est ouvert seulement pour qualifier l'identité du modèle selon le catalogue vérifié à la préparation, au pending, au scan/publication et à la reprise : le Turbo historique garde exactement ses validations SHA/taille, Parakeet accepte seulement sa clé Test prouvée, tout artefact inconnu est refusé. Le test `parakeet_archive.rs` couvre les deux identités, leurs divergences, la préparation, la reprise et la publication. `WorkerPaths { cpu, gpu }` conserve ses constructeurs : une configuration Parakeet distincte est injectée. Un besoin réel hors liste entraîne `PLAN_CHANGE_REQUIRED` avant édition. Les nouveaux fichiers n'existent pas encore et sont autorisés uniquement par un futur accord L2a sur cette liste.

**`lot.paths.read` L2a principaux :** `AGENTS.md`, `RAILGUARD.md`, `Cargo.toml`, `Cargo.lock`, manifests concernés ; ce plan et parents Git ; `crates/whisper-core/src/{audio,domain,ports,ipc,model_catalog,engine_capabilities}.rs`, `crates/whisper-adapters/src/{archive,queue_store,staging,recovery,journal,supervisor,decoder,model_catalog}.rs`, tests import/MP3/worker/legacy existants, `crates/whisper-desktop/src/main.rs`, workers Turbo ; `models/` et `sound example/` en lecture seule ; banc `target/parakeet-feasibility/spike02/worker.rs`, `NeMo-Speech.cpp/include/nemo_speech/asr.h`, `build-cpu/bin/`, `build-cpu/vcpkg_installed/x64-windows/bin/`, et fixtures `fr87.wav`, `en15.wav` en lecture seule. Le header réel a été vérifié au préflight documentaire. L'inventaire transitif des DLL CPU est un résultat du préflight d'exécution ; les répertoires du banc ne sont pas modifiés. Aucun secours depuis PATH, Downloads ou CUDA.

**`lot.paths.runtime_write` L2a :** `target/parakeet-tests/l2/l2a/` exclusivement, avec sous-dossiers Cargo, temporaires, fixtures, copies **non déplacées** des DLL CPU, archives, réglages, files, logs et résultats. Les fichiers modèle et audio d'origine restent en lecture. Rediriger toute racine de test, `CARGO_TARGET_DIR`, `TEMP`, `TMP`, `WHISPER_L1B_RUNTIME_ROOT` et les racines L2 définies en §5 vers cette racine pour la campagne, puis restaurer ; aucune écriture dans Documents/AppData utilisateur.

### Cartes L2a, toutes `NOT RUN`

| Carte | Stimulus et observation exigée | Limite |
| --- | --- | --- |
| A1 | Variantes de modèle/SHA/runtime/build/worker/protocole/profil, CPU/GPU/Auto calcul, Auto/manuelle et Test/Final : seule la clé qualifiée Test import-single-pass CPU/Auto langue est admise, sans effet moteur pour les autres. | Avant A3/G-L2A-QUAL, même cette clé reste grisée. |
| A2 | Mauvaise version/identité, ancienne génération/tentative et trame au-dessus de la borne : refus avant audio, Turbo v2 inchangé. | Contrats IPC. |
| A3 | FR `fr87.wav` et EN `en15.wav` parlés, worker produit exact, sans langue imposée ; vingt inférences CPU alternées dans le même processus : textes intelligibles, CPU effectif, résultats attribués, hôte vivant, arrêt constaté sans unload. | Recette Test réelle ; transcript historique ≠ vérité humaine. |
| A4 | WAV `spkr0.wav`, MP3 dérivé local, 1 800 s puis 1 800 s + 1 échantillon, silence et queue/padding : routage et sortie corrects ; long refusé sans appel natif ; zéro temps confirmé invalide. | Fichiers synthétiques signalés. |
| A5 | Source/modèle modifiés, DLL manquante/incompatible, résultat native incomplet ou worker mort : cause explicite, aucune substitution, source intacte, aucun faux `Complete`. | Aucun fallback Turbo. |
| A6 | Stop/Quit à barrières préparation, inférence et publication ; avant engagement pending et source intacts, résultats tardifs ignorés, sortie physique observée ; après engagement résultat réel annoncé. | Contrat puis scénario natif ; pas de délai inventé. |
| A7 | Enqueue/process/reprise de pending et checkpoint Parakeet, divergences de source/SHA modèle et audio/job/génération/FIFO/préfixe : refus avant inférence si divergent ; sinon préfixe stable, pas de doublon, publication unique. Préparation, scan et reprise valident l'identité Parakeet exacte ; identité inconnue refusée. | Ne promet pas une reprise générale après panne. |
| A8 | Avant preuves Parakeet grisé ; après PASS FR+EN et fiche Test exacte, seule la portée L2a s'ouvre, sans fuite vers Final/GPU/live/manuel. Une clé modifiée regrise ; la fenêtre Windows est une validation distincte. | AC-PARA-16-L2 ≠ AC-PARA-16F. |
| A9 | Archive réelle Parakeet : source SHA inchangé, TXT/SRT de même base, `Complete` après les deux acquittements ; échec SRT après TXT déclaré incomplet avec chemins réels. Rejouer les validations de préparation/pending/scan/reprise Turbo, dont SHA/taille historiques inchangés. | Pas d'atomicité multi-fichiers. |
| A10 | Rejouer la matrice Turbo L1b (1 206 admissions/commandes), tests de file et import existants ; aucune option historique nouvelle grisée/refusée, aucun changement Turbo v2. | Tests natifs Turbo conditionnels listés `NOT RUN` s'ils n'ont pas d'entrées. |

## 4. L2b — VAD longue CPU, six tâches

L2b réutilise le même worker/candidat CPU que L2a et n'est pas parallèle. Un échec ou `NOT RUN` des cartes longues laisse `import-vad` non qualifié. La décision utilisateur est : 60–120 s visés, silence ≥400 ms, marge 200 ms ; après 120 s sans silence, prolonger jusqu'au prochain admissible, **au plus 1 800 s d'entrée native marges comprises**, puis erreur explicite sans coupe forcée. Le reliquat EOF est traité même s'il dure <60 s.

| ID | Sortie démontrable | Cartes |
| --- | --- | --- |
| L2b.1 | Fenêtres VAD pures et déterministes du design §9, plages centrales jointives et marges bornées. | B1/B2 |
| L2b.2 | Exécution séquentielle des fenêtres, offsets, padding et propriétaire unique de chaque mot. | B3 |
| L2b.3 | Fin courte, silence intégral et absence de silence jusqu'à borne : refus sans coupe forcée ni faux succès. | B2/B3 |
| L2b.4 | Stop/Quit/reprise pendant et entre fenêtres, préfixe confirmé invariant. | B4 |
| L2b.5 | Référence privée ≤3 600 s du même candidat, recette EN60 et montage FR synthétique >1 800 s, contrôle de toutes les coupures. | B5/B6/B7 |
| L2b.6 | Après PASS : fiche Test `import-vad` exacte, activation >30 min ; sinon refus maintenu ; régression L2a/Turbo et revue. | B8 |

**`lot.paths.write` L2b, exclusifs :**

```text
crates/whisper-core/src/lib.rs
crates/whisper-core/src/model_registry.rs
crates/whisper-core/src/model_selection_app.rs
crates/whisper-core/src/parakeet_windows.rs
crates/whisper-core/tests/parakeet_windows.rs
crates/whisper-core/tests/parakeet_import_contract.rs
crates/whisper-adapters/src/parakeet_import.rs
crates/whisper-adapters/tests/parakeet_import.rs
crates/whisper-adapters/tests/parakeet_control.rs
crates/whisper-adapters/tests/parakeet_long.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-desktop/tests/model_selection_ui.rs
crates/whisper-worker-parakeet-cpu/Cargo.toml
crates/whisper-worker-parakeet-cpu/src/main.rs
crates/whisper-worker-parakeet-cpu/src/native_engine.rs
crates/whisper-worker-parakeet-cpu/tests/native_import.rs
```

`parakeet_windows.rs` est pur ; il ne remplace pas `adapters/vad.rs`, destiné au live. La feature `parakeet-qualification-reference` produit le **candidat Test L2b** : le même binaire exact exécute le parcours VAD Test et la référence ≤3 600 s. Le profil référence est accessible seulement au harness privé sous la racine runtime, jamais aux commandes UI/file/reprise ; la feature ne change ni décodage, ni ABI, ni inférence. La build L2a sans cette feature et la build L2b ont des clés worker distinctes : refaire FR/EN courts sur L2b avant sa fiche, sans transférer la preuve de L2a. Un binaire Final sans cette feature reste à qualifier en L6. Le test compare référence et VAD du **même candidat Test L2b exact** ; si l'identité varie entre les deux exécutions, résultat `INCONCLUSIVE` et révision du protocole. La référence n'étend pas la limite du profil applicatif.

**`lot.paths.read` L2b :** chemins L2a et leurs bilans, `docs/cadrage/bilan-parakeet-limite-vad-revisee.md`, `bilan-parakeet-limite-decoupe-longue.md`, `crates/whisper-adapters/src/vad.rs`, son test, et fixtures synthétiques existantes `target/parakeet-feasibility/spike02/fixtures/synthetic/{spkr0_x12.wav,fr_concat_318.wav}`. Toutes en lecture seule. `fr_concat_318.wav` dure environ 319 s : il sert de source, pas de preuve du parcours >30 min. Construire sous runtime un montage FR >1 800 s par répétitions concaténées et vérifier sa durée décodée avant la recette. Les jonctions et frontières de ce montage restent déclarées artificielles.

**`lot.paths.runtime_write` L2b :** `target/parakeet-tests/l2/l2b/` exclusivement, y compris Cargo, temporaires, copies des fixtures et DLL CPU, PCM/sorties/référence, archives et logs. Le modèle original sous `models/` reste en lecture seule ; aucune copie de modèle n'est autorisée. Aucun fichier du profil Final.

### Cartes L2b, toutes `NOT RUN`

| Carte | Stimulus et observation exigée | Limite |
| --- | --- | --- |
| B1 | PCM au-dessous/égal/au-dessus seuil RMS 64, silences 380/400/420 ms, plusieurs candidats et égalité : classification reproductible, 20 ms/320 échantillons, 20 trames, coupure près de 90 s puis ancienne en égalité. | Pur, seuil candidat non qualifié. |
| B2 | Silence après 120 s, aucun jusqu'à la borne, fin courte et silence intégral : prolongation correcte ou erreur sans coupe forcée ; entrée native ≤1 800 s marges comprises. | Décision du 16/10. |
| B3 | Mots des marges, offsets non alignés sur 80 ms, padding et dépassements : chaque mot publié une fois, zéro temps invalide confirmé ; dernier seul ≤80 ms borné. | Contrat + réel. |
| B4 | Stop/Quit/reprise à frontière et pendant natif, ancien résultat et dernière fenêtre en erreur : préfixe/source préservés, pending récupérable, aucun faux complet. | Supervision hors UI. |
| B5 | EN60 `spkr0_x12.wav`, référence même candidat puis vrai parcours VAD, inventaire de toutes les coupures : à ±2 s, ≤2 éditions normalisées par coupure ; zéro doublon, mot tronqué ou temps invalide. | Écart global informatif ; pas de seuil 3 %. |
| B6 | FR long **synthétique** >1 800 s dérivé par répétition de `fr_concat_318.wav` sous runtime, durée décodée attestée et véritable profil produit `import-vad` : même critère mécanique ; montage et frontières artificielles signalés. | FR long réel demeure réserve. |
| B7 | Demander le profil référence via commandes UI, file/reprise, source hors racine ou >3 600 s : refus avant effet ; seul le harness privé y accède. | Aucun mode produit >30 min en passe unique. |
| B8 | Après PASS B1–B7 et FR/EN courts sur la build Test L2b exacte, fiche `import-vad` de cette clé seulement ; dimension modifiée regrise, L2a et Turbo inchangés ; `FAIL/INCONCLUSIVE/NOT RUN` laisse long refusé. | Final/GPU/live non qualifiés ; preuve L2a non héritée. |

## 5. Préflight et vérifications proposés

Avant toute écriture de **chaque sous-lot**, vérifier branche/HEAD et ancêtres design/L1b, statut/diff, plan Git exact et verdict indépendant, puis obtenir l'accord écrit précis du sous-lot et de ses chemins. L2a exige en plus l'accord explicite ABI C/`unsafe`/chargement et copie de DLL CPU ; le mandat du spike n'y supplée pas. L2b exige L2a CLEAN. Si un chemin, une technologie, une dépendance ou une écriture indirecte manque : `PLAN_CHANGE_REQUIRED` avant édition. Les dossiers locaux `models/`, `sound example/`, le spike et les réglages utilisateur restent en lecture seule ; aucun téléchargement/installateur/service/ACL/push. Toute recette native sur un modèle absent/SHA divergent ou des DLL manquantes est `NOT RUN`, jamais remplacée par une fixture qui réussirait silencieusement.

Le préflight inventorie les DLL CPU et leurs imports transitifs, le header `nemo_speech/asr.h`, le modèle, les fixtures, l'outil ffmpeg présent et l'espace cible. Les versions de crates doivent être déjà en cache ; `cargo metadata --offline --locked --no-deps --format-version 1` vérifie le workspace pré-existant. Après ajout de la crate, actualiser `Cargo.lock` par `cargo update --offline --workspace`, inspecter le diff de résolution et seulement ensuite lancer les commandes `--offline --locked` ; si une dépendance n'est pas en cache, arrêter sans téléchargement. Pour les commandes de vérification, rediriger temporairement `CARGO_TARGET_DIR`, `TEMP`, `TMP`, `WHISPER_L1B_RUNTIME_ROOT` et les racines L2 sous `lot.paths.runtime_write`, vérifier les chemins absolus avant les tests puis restaurer les valeurs précédentes, y compris l'absence de valeur ; retirer/restaurer `WHISPER_L01_*`, `WHISPER_L02_*`, `WHISPER_L04_*`. Pas de nettoyage global. Définir `PARAKEET_L2_RUNTIME_ROOT`, `PARAKEET_L2_ARCHIVE_ROOT`, `PARAKEET_L2_SETTINGS_ROOT`, `PARAKEET_L2_QUEUE_ROOT`, `PARAKEET_L2_OUTPUT_DIR` sous le runtime du sous-lot ; les nouveaux tests doivent refuser une valeur absente ou hors racine. Les tests adapters historiques `model_catalog`, `model_preferences` et `legacy_model_requests` utilisent obligatoirement `WHISPER_L1B_RUNTIME_ROOT`, à fixer sous ce même runtime. Le protocole de test natif utilisera des entrées explicites : `PARAKEET_L2_MODEL`, `PARAKEET_L2_NATIVE_DIR`, `PARAKEET_L2_WORKER`, `PARAKEET_L2_FR_WAV`, `PARAKEET_L2_EN_WAV`, `PARAKEET_L2_OUTPUT_DIR` ; modèles/audio d'origine restent en lecture, et le worker et sa cible Cargo sont sous runtime. Un test d'acceptation manquant une entrée n'annonce pas PASS. Le MP3 est dérivé localement sous runtime de `spkr0.wav` avec ffmpeg ; si l'outil est absent, la carte MP3 est `NOT RUN` et la qualification WAV ne la couvre pas.

**Après candidat L2a**, commandes proposées, non exécutées :

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-core
cargo test --offline --locked -p whisper-worker-parakeet-cpu
cargo test --offline --locked -p whisper-adapters --test parakeet_import --test parakeet_control --test legacy_model_requests --test model_catalog --test model_preferences
cargo test --offline --locked -p whisper-adapters --test parakeet_archive
cargo test --offline --locked -p whisper-desktop --test model_selection_ui
cargo build --offline --locked --workspace
cargo test --offline --locked -p whisper-worker-parakeet-cpu --test native_import -- --ignored --nocapture
git diff --check
git diff --name-only
```

Le dernier test natif requiert ses variables/chemins vérifiés au préflight. Un résultat Cargo vert avec sous-scénario sauté ne ferme aucune carte native. Les tests Turbo natifs conditionnels restent `NOT RUN` si leurs entrées sont absentes, avec sous-scénarios dénombrés. L2a rapporte le build du worker et le CPU réellement attesté, pas une preuve GPU.

**Après candidat L2b**, commandes proposées, non exécutées :

```powershell
cargo fmt --all -- --check
cargo build --offline --locked -p whisper-worker-parakeet-cpu --features parakeet-qualification-reference
cargo test --offline --locked -p whisper-core --test parakeet_windows --test parakeet_import_contract
cargo test --offline --locked -p whisper-adapters --test parakeet_import --test parakeet_control
cargo test --offline --locked -p whisper-worker-parakeet-cpu --test native_import --features parakeet-qualification-reference -- --ignored --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_long -- --ignored --nocapture
cargo test --offline --locked -p whisper-core
cargo test --offline --locked -p whisper-desktop --test model_selection_ui
cargo build --offline --locked --workspace --target-dir target/parakeet-tests/l2/l2b/cargo-regression/
git diff --check
git diff --name-only
```

Avant la première commande L2b, fixer `CARGO_TARGET_DIR` à `target/parakeet-tests/l2/l2b/cargo/`, construire la build Test L2b avec feature, fixer `PARAKEET_L2_WORKER` au binaire de cette cible et vérifier sa clé worker/runtime/protocole ainsi que celle des DLL copiées. La commande `native_import` rejoue FR et EN parlés sur **ce binaire**, avec comptes et résultats explicites ; sans ses deux PASS, ne pas lancer une recette qui prétend qualifier B8. `parakeet_long` lance ensuite référence et VAD via le même chemin et archive les identités observées de chaque exécution. Le build workspace de régression utilise sa cible distincte `cargo-regression/` même si `CARGO_TARGET_DIR` pointe encore vers `cargo/` ; constater que le worker qualifié conserve les mêmes octets avant/après cette régression. La régression finale ne doit ni reconstruire ni substituer le worker qualifié ; tout changement pertinent de build, modèle, DLL ou protocole périme la preuve et impose de rejouer courts puis long. La build Test L2b avec feature et sa cible Cargo restent séparées du worker L2a ordinaire. Les temps de campagne proposés ne sont pas des critères produit. Les tests VAD 60 min sont de la `PRODUCT_VALIDATION`, non une répétition du spike clos.

## 6. Livrables et statut

Chaque sous-lot rapporte diff, sorties des commandes, nombre de scénarios exécutés, état `PASS/FAIL/NOT RUN/INCONCLUSIVE` de ses cartes, identité des octets natifs réellement chargés et limites. `rust_review_coordinator` orchestre un `rust_reviewer` indépendant, corrections à `rust_implementer`, au plus deux cycles par sous-lot. CLEAN documentaire du présent plan ne vaut pas autorisation de code, ni CLEAN d'un candidat, ni validation dans la fenêtre Windows. Aucun push ni acquisition de modèle.

Récupération : si une fiche Test ou un profil long échoue, conserver Turbo et le profil antérieur qualifié, laisser la nouvelle clé non admissible ; garder source, pending, résultats et diagnostics. Toute suppression éventuelle se limite aux sorties inventoriées dans runtime et attend une autorité précise. Ne pas réinitialiser le dépôt ni modifier les modèles pour faire passer la recette.

**Sortie attendue de cette étape :** revue indépendante du plan versionné dans Git, puis présentation d'un accord séparé pour L2a, ses chemins et l'ABI native. L2b attendra L2a CLEAN. Aucun travail d'implémentation L2 n'est demandé par la présente rédaction.
