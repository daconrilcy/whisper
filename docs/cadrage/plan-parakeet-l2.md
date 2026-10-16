Avancée : L2a-2 et sa fermeture R6–R10 sont commités à `4045c828` ; le candidat partiel L2b passe les contrats purs et la recette courte FR/EN.
Blocage : B4–B8 nécessitent le routeur et l'archive produit, absents des 17 chemins L2b v5 ; l'activation longue reste interdite.
Décision : étendre L2b à trois chemins adapters ciblés, relire ce delta puis reprendre les preuves du parcours produit avant toute fiche `import-vad`.

# PLAN-PARA-L2 v6 — import Parakeet CPU

**Parents Git :** design `docs/cadrage/evo-parakeet.md` §9 et `docs/cadrage/evo-engine-capabilities.md` §8 au commit `3c47f272`, verdict indépendant **CLEAN — DESIGN READY-L2** sur CPU/import FR/EN ; L1b-2 `84e5444d` et L1b-1 `7de28359`, tous deux revus CLEAN. Le bilan `bilan-parakeet-faisabilite-cloture.md` est une preuve de faisabilité isolée, pas une qualification produit. Contributions en lecture seule `ARCH-PARA-L2 v1`, `VAD-L2 v1`, `REQ-PARA-L2 v1` et `rust_plan_writer` ; rédaction/versionnement par la session principale. Le plan v4 était **CLEAN — PLANS READY-L2a-1 / L2a-2 / L2b** sur `e6e95050`. L2a-1 est commité à `842cb000` après revue CLEAN ; le delta v5 a reçu **CLEAN — PLANS READY-L2a-2** sur `0813336d`. L2a-2 et son lot distinct de fermeture R6–R10 sont commités à `4045c828`, avec verdict indépendant CLEAN pour ce dernier. **Statut v6 : CLEAN — PLANS READY-L2b**, revue indépendante du delta `36c63b61` sans REQUIRED. L'advisory V6-A1 sur les commandes d'archive et de B9 est corrigé en §5 et soumis à vérification du même reviewer. Le candidat L2b partiel reste non commité et non qualifié. L'utilisateur a autorisé dans cette session la révision du plan, sa revue, puis la reprise conditionnelle de L2b sur les trois chemins supplémentaires cités en §4 si la revue est CLEAN. Aucun autre lot n'est autorisé.

## 1. Résultat, limites et gates

L2a est scindé : **L2a-1** produit le worker CPU isolé, son IPC, son décodeur et le harness natif FR/EN ; **L2a-2** raccorde admission, archive, arrêt/reprise, fiche Test et interface seulement après PASS FR et EN et verdict CLEAN de L2a-1. Ensemble ils ajoutent l'import Parakeet **CPU manuel**, WAV/MP3, Auto **langue** FR/EN, texte et TXT/SRT, pour une durée source ≤1 800 s. Avant qualification du worker Test exact, Parakeet reste grisé ; un harness privé permet la preuve sans contourner l'UI. Après PASS FR et EN sur le candidat exact, seule la fiche Test `import-single-pass` peut être versionnée et admise. L2b ajoute le parcours >1 800 s par VAD et sa recette de 60 min ; son échec laisse L2a-2 utilisable et le long refusé avec cause. Les préférences et la matrice Turbo L1b sont préservées. GPU et Auto **calcul** Parakeet relèvent de L3, microphone de L4, acquisition de L5, paquet Final et `AC-PARA-16F` de L6, comparaison de qualité absolue de L7. Les 23 autres langues restent hors périmètre qualifié.

| Gate | Condition nécessaire | En absence |
| --- | --- | --- |
| G-L2-DESIGN | Parents et verdict exacts ci-dessus, décisions utilisateur du 16/10 sur le silence : prochain silence après 120 s, borne 30 min puis erreur, aucune coupe forcée. | `DESIGN_CHANGE_REQUIRED`. |
| G-L2-PLAN | Ce plan et les chemins de **chaque** sous-lot revus, acceptés, puis accord écrit précis **dans la session d'implémentation** pour L2a-1, L2a-2 ou L2b. L'accord L1b et l'ancien spike ne couvrent pas L2. | Aucune écriture de code du sous-lot. |
| G-L2-NATIVE | Pour L2a-1, accord précis couvrant l'ABI C NeMo, Rust `unsafe` privé, chargement Windows de DLL existantes et copie bornée sous `target/parakeet-tests/l2/l2a-1/`. Aucune modification C++ ni acquisition. | Contrats purs possibles seulement si un sous-périmètre est accepté ; recette native `NOT RUN`, aucune fiche Test activée. |
| G-L2-TEST | Fichier Test Parakeet présent avec taille/SHA attendus ; worker, ABI, DLL, protocoles, sources FR/EN et sorties attribués à la clé exacte. | Recette affectée `NOT RUN`, sans téléchargement ni écriture sous `models/`; Parakeet grisé. |
| G-L2A-1 | Worker produit CPU et harness privé donnent `PASS` FR **et** EN sans langue imposée, temps valides ; parité de décodage A11 documentée ; verdict indépendant `CLEAN` sur L2a-1. | L2a-2 ne démarre pas ; aucune fiche `Qualified` Test ni admission GUI. |
| G-L2A-QUAL | Le changement borné de garde de racine en L2a-2 modifie le SHA du worker L2a-1. Avant fiche Test ou admission GUI, reconstruire ce worker sous le runtime L2a-2 et refaire A3 FR/EN et A11 WAV/MP3 sur le binaire, les DLL, le modèle, le protocole et la racine d'exécution exacts du candidat L2a-2. Toute autre dimension changée impose la même requalification de sa clé exacte. | Pas de fiche `Qualified` Test ni d'admission GUI ; `FAIL`/`INCONCLUSIVE` visibles. |
| G-L2B | L2a-2 revu CLEAN ; algorithme VAD du design, référence Test-only 60 min et corpus local vérifiés ; accord écrit propre à L2b et aux 20 chemins du §4, avec revue CLEAN du delta v6 avant reprise du code. | Import >30 min refusé ; L2a-2 inchangé. |

La preuve Test `AC-PARA-16-L2` est une `PRODUCT_VALIDATION` ; elle n'est pas transférée au paquet Final de `AC-PARA-16F` (`DELIVERY_QUALIFICATION`). `language_count=0` est attendu, sans critère d'échec. Aucun seuil nouveau de vitesse, RAM, VRAM ou écart global de transcription n'est introduit. Aucun worker Parakeet GPU ou live n'est lancé en L2.

## 2. Contrats communs à L2a et L2b

**Admission et profils.** La clé lie modèle/SHA, runtime/build natif, worker/build, protocole, backend CPU, profil et matériel pertinent. `import-single-pass` (≤1 800 s), `import-vad` (>1 800 s), live et référence de qualification ne partagent pas une preuve. La durée décodée et le parcours déterminent le profil avant effet, puis le même garde contrôle start, enqueue, process et reprise. La requête est liée au chemin du catalogue vérifié ; un chemin/empreinte arbitraire ou une dimension divergente est refusé. **Langue manuelle Parakeet**, GPU, Auto calcul et live restent refusés. La fiche Test est une donnée de code versionnée dans `model_registry.rs` **après** G-L2A-QUAL, sans nouveau manifeste ou fichier de qualification ; la composition vérifie sa clé à chaque lancement. Si le code ne peut représenter et vérifier cette clé exacte sans persistance ad hoc : `DESIGN_CHANGE_REQUIRED` avant activation.

**IPC et native.** Un protocole **Parakeet v1 séparé** porte modèle/artefact/révision/SHA, runtime/build, worker/protocole, backend/profil, source/SHA, nombre d'échantillons, job/génération/tentative, plages et identité de réponse. Le protocole Turbo v2 (`ipc.rs`) reste intact. Les messages `Prepare/Ready/BeginImport`, mots/progrès bornés, fin/erreur/Stop/Shutdown refusent version ou identité incohérentes avant audio ; les résultats tardifs ne franchissent pas la génération. Le worker Parakeet CPU est un nouveau processus Rust sans `whisper-rs`. Sa façade ABI C privée est la seule zone `unsafe`, avec invariants de layout, durée de vie des `CString`/PCM et propriété des pointeurs. Elle copie mots et texte en Rust possédé avant destruction des résultats. Le module est chargé **une fois par processus** et conservé jusqu'à sa fin : aucun `FreeLibrary` après inférence, même sur erreur. L'hôte ne charge aucune DLL NeMo. `target/parakeet-feasibility/spike02/worker.rs` est la référence de cycle de vie ; l'ancien `wrapper.rs` ne l'est pas.

**Transport et sorties.** Le worker décode la source/plage attestée en PCM mono 16 kHz ; jusqu'à 115,2 Mo de `f32` pour 30 min ne traversent pas les trames IPC de 1 Mio. Les résultats sont fractionnés en messages bornés. L'offset absolu est calculé en échantillons ; la trame TDT vaut 80 ms et le padding final n'allonge pas l'audio réel. Seul le **dernier mot du fichier**, si son dépassement brut est ≤80 ms, peut recevoir `end=min(end,durée_audio)`. Débordement interne, dépassement final >80 ms ou plage inversée interdisent la confirmation avec cause. Tout mot confirmé satisfait `0 ≤ start < end ≤ durée_audio` ; TXT/SRT sont ordonnés sur cette timebase. La source est préservée, `Complete` n'apparaît qu'après acquittements de publication. Stop/Quit avant engagement invalide la tentative, garde le pending récupérable et rejette les résultats tardifs ; après engagement, annonce que l'arrêt est trop tardif et attend l'issue réelle. `kill` demandé n'est pas une sortie physique constatée. Aucun IO, attente ou calcul natif bloquant sur le fil UI.

**Sondes.** Corréler release, modèle, worker, calcul demandé/effectif, job/génération/tentative, début/fin de vérification, préparation, premier audio, appel natif, confirmation, Stop, sortie et chaque publication. Une durée monotone n'est soustraite qu'à l'intérieur d'un même processus ; les sondes ne sont ni une mesure de charge, ni un benchmark. Réutiliser le journal local prévu sans nouveau fichier de preuve signé.

## 3. L2a-1 puis L2a-2 — passe unique CPU

| ID | Sortie démontrable | Cartes |
| --- | --- | --- |
| L2a.1 | Clés par profil/parcours/durée, refus Parakeet hors CPU/import/Auto langue et non-héritage Test/Final. | A1/A8 |
| L2a.2 | Contrat IPC Parakeet v1 versionné côté worker, erreurs d'identité et messages bornés ; Turbo v2 inchangé. Le raccordement du routeur hôte relève de L2a.5. | A2/A5 (worker) |
| L2a.3 | Worker CPU, façade ABI, chargement privé et cycle de vie sans déchargement après inférence. | A3/A5 |
| L2a.4 | Décodage WAV/MP3 côté worker, passe unique ≤1 800 s, mots possédés, temps validés et parité avec la voie `whisper-adapters/src/decoder.rs`. | A3/A4 (worker)/A11 |
| L2a.5 | Garde et routage à préparer/lancer/file/traitement/reprise, composition Test, refus explicite >30 min ; archive qualifiant l'identité Turbo historique ou Parakeet Test exacte. | A1/A5/A7/A8 |
| L2a.6 | Stop/Quit, sortie physique, résultat tardif, pending/reprise et publication TXT/SRT. | A6/A7/A9 |
| L2a.7 | Harness privé et recette native FR+EN du **worker produit exact**, sans admission GUI anticipée. | A3/A5 (worker) |
| L2a.8 | Seulement après G-L2A-QUAL : fiche Test versionnée, activation GUI Test et vérification de clé ; Final/live/GPU restent bloqués. | A1/A8 |
| L2a.9 | Régression Turbo, diff/tests/build et revue indépendante. | A10 |
| L2a.10 | Corriger uniquement la garde de racine du worker CPU pour admettre le runtime L2a-2 sans ouvrir un chemin arbitraire ; reconstruire sous cette racine, relever son nouveau SHA et refaire la recette FR/EN et la parité sur le candidat exact avant la fiche Test. | A3/A11/G-L2A-QUAL |

**Séquence et propriété.** L2a-1 exécute seulement L2a.2/.3/.4/.7 et les cartes A2, A3, A4/A5 **parties worker**, plus A11. Il ne modifie aucun fichier d'application, de registre, d'archive ou de bureau. Son livrable est le worker et son harness ; un PASS FR+EN et un verdict indépendant CLEAN sont nécessaires avant L2a-2. L2a-2 exécute L2a.1/.5/.6/.8/.9/.10 et A1, A4/A5 **parties produit**, A6–A10. Son changement de garde de racine crée une nouvelle clé worker : les PASS L2a-1 ouvrent le sous-lot, mais ne qualifient pas la fiche Test L2a-2 ; A3 et A11 sont rejouées sur le nouveau binaire avant L2a.8. Les deux sous-lots ont des accords, chemins et revues distincts.

**`lot.paths.write` L2a-1 :**

```text
Cargo.toml
Cargo.lock
crates/whisper-core/src/lib.rs
crates/whisper-core/src/parakeet_ipc.rs
crates/whisper-core/tests/parakeet_ipc.rs
crates/whisper-worker-parakeet-cpu/Cargo.toml
crates/whisper-worker-parakeet-cpu/src/main.rs
crates/whisper-worker-parakeet-cpu/src/native_engine.rs
crates/whisper-worker-parakeet-cpu/src/native_ffi.rs
crates/whisper-worker-parakeet-cpu/src/decoder.rs
crates/whisper-worker-parakeet-cpu/tests/native_import.rs
crates/whisper-worker-parakeet-cpu/tests/decoder_parity.rs
```

**`lot.paths.write` L2a-2 :**

```text
crates/whisper-core/src/application.rs
crates/whisper-core/src/model_registry.rs
crates/whisper-core/src/model_selection_app.rs
crates/whisper-core/tests/model_selection_app.rs
crates/whisper-core/tests/parakeet_import_contract.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/src/parakeet_import.rs
crates/whisper-adapters/tests/parakeet_archive.rs
crates/whisper-adapters/tests/parakeet_import.rs
crates/whisper-adapters/tests/parakeet_control.rs
crates/whisper-worker-parakeet-cpu/src/main.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-desktop/tests/model_selection_ui.rs
```

Le manifeste racine et `Cargo.lock` ne changent qu'en L2a-1 pour enregistrer la nouvelle crate et des versions déjà présentes en cache offline ; aucun téléchargement. En L2a-1, `application.rs`, `model_selection_app.rs`, `model_registry.rs`, `archive.rs`, tout `whisper-desktop/` et les adaptateurs restent en lecture seule. `main.rs` du worker est le seul chemin rouvert en L2a-2 : seul le contrôle de racine d'exécution peut changer, sans toucher le protocole, le décodage, la façade native ou le cycle de vie DLL. Le worker accepte exclusivement les racines canoniques Test L2a-1 et L2a-2 de ce dépôt ; une racine arbitraire reste refusée. `engine_capabilities.rs`, `ports.rs`, `ipc.rs`, les workers Turbo et les manifestes existants restent en lecture dans les deux sous-lots. En L2a-2, `archive.rs` est ouvert seulement pour qualifier l'identité du modèle selon le catalogue vérifié à la préparation, au pending, au scan/publication et à la reprise : le Turbo historique garde exactement ses validations SHA/taille, Parakeet accepte seulement sa clé Test prouvée, tout artefact inconnu est refusé. Le test `parakeet_archive.rs` couvre les deux identités, leurs divergences, la préparation, la reprise et la publication. `WorkerPaths { cpu, gpu }` conserve ses constructeurs : une configuration Parakeet distincte est injectée. Un besoin réel hors de la liste du sous-lot entraîne `PLAN_CHANGE_REQUIRED` avant édition. Les nouveaux fichiers n'existent pas encore et sont autorisés uniquement par l'accord sur leur sous-lot.

**`lot.paths.read` L2a principaux :** `AGENTS.md`, `RAILGUARD.md`, `Cargo.toml`, `Cargo.lock`, manifests concernés ; ce plan et parents Git ; `crates/whisper-core/src/{audio,domain,ports,ipc,model_catalog,engine_capabilities}.rs`, `crates/whisper-adapters/src/{archive,queue_store,staging,recovery,journal,supervisor,decoder,model_catalog}.rs`, tests import/MP3/worker/legacy existants, `crates/whisper-desktop/src/main.rs`, workers Turbo ; `models/` et `sound example/` en lecture seule ; banc `target/parakeet-feasibility/spike02/worker.rs`, `NeMo-Speech.cpp/include/nemo_speech/asr.h`, `build-cpu/bin/`, `build-cpu/vcpkg_installed/x64-windows/bin/`, et fixtures `fr87.wav`, `en15.wav` en lecture seule. Le header réel a été vérifié au préflight documentaire. L'inventaire transitif des DLL CPU est un résultat du préflight d'exécution ; les répertoires du banc ne sont pas modifiés. Aucun secours depuis PATH, Downloads ou CUDA.

**`lot.paths.runtime_write` :** L2a-1 écrit uniquement sous `target/parakeet-tests/l2/l2a-1/` (Cargo, temporaires, fixtures, copies **non déplacées** des DLL CPU, logs et résultats du harness). L2a-2 écrit uniquement sous `target/parakeet-tests/l2/l2a-2/` (Cargo, temporaires, fixtures, copies des DLL CPU, archives, réglages, files, logs et résultats). Le binaire et les preuves L2a-1 sont lus sans modification par L2a-2 ; le binaire L2a-2 est reconstruit sous sa propre racine et identifié par son nouveau SHA. Les DLL sont copiées sans déplacement et leurs SHA vérifiés. Les fichiers modèle et audio d'origine restent en lecture. Rediriger toute racine de test, `CARGO_TARGET_DIR`, `TEMP`, `TMP`, `WHISPER_L1B_RUNTIME_ROOT` et les racines L2 définies en §5 vers la racine du sous-lot pour la campagne, puis restaurer ; aucune écriture dans Documents/AppData utilisateur.

### Cartes L2a-1 et L2a-2, toutes `NOT RUN`

| Carte | Sous-lot | Stimulus et observation exigée | Limite |
| --- | --- | --- | --- |
| A1 | L2a-2 | Variantes de modèle/SHA/runtime/build/worker/protocole/profil, CPU/GPU/Auto calcul, Auto/manuelle et Test/Final : seule la clé qualifiée Test import-single-pass CPU/Auto langue est admise, sans effet moteur pour les autres. | Avant A3/G-L2A-QUAL, même cette clé reste grisée. |
| A2 | L2a-1 | Mauvaise version/identité, ancienne génération/tentative et trame au-dessus de la borne : refus côté worker avant audio, Turbo v2 inchangé. Le refus par le routeur hôte est repris en L2a-2/A5. | Contrats IPC. |
| A3 | L2a-1, rejouée en L2a-2 après changement du worker | FR `fr87.wav` et EN `en15.wav` parlés, worker produit exact, sans langue imposée ; vingt inférences CPU alternées dans le même processus : textes intelligibles, CPU effectif, résultats attribués, hôte vivant, arrêt constaté sans unload. L2a-2 utilise sa nouvelle racine et son binaire reconstruit, et relève les identités de chaque composant. | Recette Test réelle ; transcript historique ≠ vérité humaine. |
| A4 | L2a-1 puis L2a-2 | Worker : WAV `spkr0.wav`, MP3 dérivé local, 1 800 s, silence et queue/padding, temps valides. Produit : routage à 1 800 s puis 1 800 s + 1 échantillon ; long refusé sans appel natif ; zéro temps confirmé invalide. | Fichiers synthétiques signalés. |
| A5 | L2a-1 puis L2a-2 | Worker : source/modèle modifiés, DLL manquante/incompatible, résultat natif incomplet ou worker mort donnent une erreur attribuée, sans substitution. Produit : le routeur refuse la mauvaise identité avant audio et préserve la source sans faux `Complete`. | Aucun fallback Turbo. |
| A6 | L2a-2 | Stop/Quit à barrières préparation, inférence et publication ; avant engagement pending et source intacts, résultats tardifs ignorés, sortie physique observée ; après engagement résultat réel annoncé. | Contrat puis scénario natif ; pas de délai inventé. |
| A7 | L2a-2 | Enqueue/process/reprise de pending et checkpoint Parakeet, divergences de source/SHA modèle et audio/job/génération/FIFO/préfixe : refus avant inférence si divergent ; sinon préfixe stable, pas de doublon, publication unique. Préparation, scan et reprise valident l'identité Parakeet exacte ; identité inconnue refusée. | Ne promet pas une reprise générale après panne. |
| A8 | L2a-2 | Avant preuves Parakeet grisé ; après PASS FR+EN L2a-1 et fiche Test exacte, seule la portée L2a-2 s'ouvre, sans fuite vers Final/GPU/live/manuel. Une clé modifiée regrise ; la fenêtre Windows est une validation distincte. | AC-PARA-16-L2 ≠ AC-PARA-16F. |
| A9 | L2a-2 | Archive réelle Parakeet : source SHA inchangé, TXT/SRT de même base, `Complete` après les deux acquittements ; échec SRT après TXT déclaré incomplet avec chemins réels. Rejouer les validations de préparation/pending/scan/reprise Turbo, dont SHA/taille historiques inchangés. | Pas d'atomicité multi-fichiers. |
| A10 | L2a-2 | Rejouer la matrice Turbo L1b (1 206 admissions/commandes), tests de file et import existants ; aucune option historique nouvelle grisée/refusée, aucun changement Turbo v2. | Tests natifs Turbo conditionnels listés `NOT RUN` s'ils n'ont pas d'entrées. |
| A11 | L2a-1, rejouée en L2a-2 après changement du worker | Parité du décodeur worker avec le parcours existant via `whisper-adapters/src/decoder.rs` (proxy vers `whisper-worker-cpu/src/decoder.rs`) : sur les mêmes WAV et MP3, même nombre d'échantillons mono 16 kHz et écart maximal échantillon par échantillon ≤1 unité `i16` (≤1/32768 en PCM normalisé). Rapporter le maximum observé, le codec/source et l'alignement ; tout dépassement ou décalage est FAIL, sans relâcher la borne silencieusement. | Le proxy n'effectue pas lui-même le décodage ; les deux voies complètes sont comparées. |

## 4. L2b — VAD longue CPU, six tâches

L2b réutilise le même worker/candidat CPU que L2a et n'est pas parallèle. Un échec ou `NOT RUN` des cartes longues laisse `import-vad` non qualifié. La décision utilisateur est : 60–120 s visés, silence ≥400 ms, marge 200 ms ; après 120 s sans silence, prolonger jusqu'au prochain admissible, **au plus 1 800 s d'entrée native marges comprises**, puis erreur explicite sans coupe forcée. Le reliquat EOF est traité même s'il dure <60 s.

| ID | Sortie démontrable | Cartes |
| --- | --- | --- |
| L2b.1 | Fenêtres VAD pures et déterministes du design §9, plages centrales jointives et marges bornées. | B1/B2 |
| L2b.2 | Exécution séquentielle des fenêtres, offsets, padding et propriétaire unique de chaque mot. | B3 |
| L2b.3 | Fin courte, silence intégral et absence de silence jusqu'à borne : refus sans coupe forcée ni faux succès. | B2/B3 |
| L2b.4 | Stop/Quit/reprise pendant et entre fenêtres, préfixe confirmé invariant. | B4 |
| L2b.5 | Référence privée ≤3 600 s du même candidat, recette EN60 et montage FR synthétique >1 800 s, contrôle de toutes les coupures ; observation informative sur fond bruité. | B5/B6/B7/B9 |
| L2b.6 | Après PASS : fiche Test `import-vad` exacte, activation >30 min ; sinon refus maintenu ; régression L2a/Turbo et revue. Présenter B9 à l'utilisateur avant l'activation. | B8/B9 |

**Delta v6 — nécessité et ordre.** L2b v5 ouvrait le harness mais pas deux gardes du parcours produit : `worker_ipc.rs` refuse >1 800 s à la demande et au démarrage, lance une seule fenêtre, n'accepte que `import-single-pass` à Ready et traite un seul résultat ; `archive.rs` refuse >1 800 s et ne persiste/compare que la clé single-pass. B4, B5/B6 en véritable parcours produit et B8 ne sont donc pas démontrables par le harness seul. Ajouter le routeur, l'archive et leur test à **L2b.2/.4/.5/.6**, sans toucher au protocole, au décodeur, à la FFI ni aux workers Turbo. Le routeur choisit le profil selon la durée décodée avant effet, exécute les fenêtres en séquence, confirme une seule fois leurs plages centrales, invalide les résultats tardifs et reprend depuis le préfixe confirmé. L'archive enregistre et vérifie séparément la clé `import-vad` du candidat exact au prepare/pending/scan/reprise, tout en conservant les anciens pending `import-single-pass` et Turbo. Tester Stop/Quit à la frontière et pendant l'appel natif, erreur de la dernière fenêtre, divergence de clé/source et reprise sans doublon ni faux `Complete`. B5/B6 doivent passer par `AsyncImportIo`/`ImportApplication` avec le même candidat que la référence privée, et publier une seule génération. B8 reste suspendue jusqu'aux PASS requis ; B9 est présenté à l'utilisateur avant son activation.

**`lot.paths.write` L2b v6, exclusifs (20 chemins) :**

```text
crates/whisper-core/src/lib.rs
crates/whisper-core/src/model_registry.rs
crates/whisper-core/src/model_selection_app.rs
crates/whisper-core/src/parakeet_windows.rs
crates/whisper-core/tests/parakeet_windows.rs
crates/whisper-core/tests/parakeet_import_contract.rs
crates/whisper-adapters/src/parakeet_import.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/tests/parakeet_import.rs
crates/whisper-adapters/tests/parakeet_control.rs
crates/whisper-adapters/tests/parakeet_archive.rs
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

Le candidat partiel au préflight v6 ne touche que huit chemins de la liste ci-dessus ; `worker_ipc.rs`, `archive.rs` et `parakeet_archive.rs` n'ont pas été modifiés pour L2b. Ses huit tests purs B1–B3 et vingt inférences alternées FR/EN sur le worker Test L2b sont des preuves provisoires, à rattacher de nouveau à la clé exacte après le candidat final. Elles ne ferment ni B4–B8 ni B9, tous `NOT RUN` à ce stade. Le runtime demeure exclusivement `target/parakeet-tests/l2/l2b/` ; modèle, sources audio et banc restent en lecture.

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
| B9 | **Informative, non bloquante pour B8** : sous runtime, mélanger `spkr0_x12.wav` avec un fond rose constant produit par ffmpeg `anoisesrc`, à −40, −30 et −20 dBFS. Pour chaque niveau, rapporter la durée, le nombre de silences admissibles ≥400 ms, les coupures réalisées ou l'erreur explicite « aucun silence ». Présenter ces trois résultats à l'utilisateur avant activation de `import-vad`, sans en faire un seuil d'admission. | Source synthétique bruitée ; aucun nouveau critère de qualité ou vitesse. |

## 5. Préflight et vérifications proposés

Avant toute écriture de **chaque sous-lot**, vérifier branche/HEAD et ancêtres design/L1b, statut/diff, plan Git exact et verdict indépendant, puis obtenir l'accord écrit précis du sous-lot et de ses chemins. L2a-1 exige en plus l'accord explicite ABI C/`unsafe`/chargement et copie de DLL CPU ; le mandat du spike n'y supplée pas. L2a-2 exige PASS FR et EN sur L2a-1 et son verdict CLEAN ; L2b exige L2a-2 CLEAN. Si un chemin, une technologie, une dépendance ou une écriture indirecte manque : `PLAN_CHANGE_REQUIRED` avant édition. Les dossiers locaux `models/`, `sound example/`, le spike et les réglages utilisateur restent en lecture seule ; aucun téléchargement/installateur/service/ACL/push. Toute recette native sur un modèle absent/SHA divergent ou des DLL manquantes est `NOT RUN`, jamais remplacée par une fixture qui réussirait silencieusement.

Le préflight inventorie les DLL CPU et leurs imports transitifs, le header `nemo_speech/asr.h`, le modèle, les fixtures, l'outil ffmpeg présent et l'espace cible. Les versions de crates et de décodage doivent être déjà en cache ; `cargo metadata --offline --locked --no-deps --format-version 1` vérifie le workspace pré-existant. En L2a-1, après ajout de la crate, actualiser `Cargo.lock` par `cargo update --offline --workspace`, inspecter le diff de résolution et seulement ensuite lancer les commandes `--offline --locked` ; si une dépendance de décodage ou autre manque dans le cache hors ligne : `PLAN_CHANGE_REQUIRED`, sans téléchargement. L2a-2 ne modifie ni `Cargo.toml` ni `Cargo.lock`. Pour les commandes de vérification, rediriger temporairement `CARGO_TARGET_DIR`, `TEMP`, `TMP`, `WHISPER_L1B_RUNTIME_ROOT` et les racines L2 sous `lot.paths.runtime_write` du sous-lot, vérifier les chemins absolus avant les tests puis restaurer les valeurs précédentes, y compris l'absence de valeur ; retirer/restaurer `WHISPER_L01_*`, `WHISPER_L02_*`, `WHISPER_L04_*`. Pas de nettoyage global. Définir `PARAKEET_L2_RUNTIME_ROOT`, `PARAKEET_L2_ARCHIVE_ROOT`, `PARAKEET_L2_SETTINGS_ROOT`, `PARAKEET_L2_QUEUE_ROOT`, `PARAKEET_L2_OUTPUT_DIR` sous le runtime du sous-lot ; les nouveaux tests doivent refuser une valeur absente ou hors racine. Les tests adapters historiques `model_catalog`, `model_preferences` et `legacy_model_requests` utilisent obligatoirement `WHISPER_L1B_RUNTIME_ROOT`, à fixer sous ce même runtime. Le protocole de test natif utilisera des entrées explicites : `PARAKEET_L2_MODEL`, `PARAKEET_L2_NATIVE_DIR`, `PARAKEET_L2_WORKER`, `PARAKEET_L2_FR_WAV`, `PARAKEET_L2_EN_WAV`, `PARAKEET_L2_OUTPUT_DIR` ; A11 exige aussi `PARAKEET_L2_TURBO_DECODE_WORKER` et les mêmes WAV/MP3 pour les deux voies. Modèles/audio d'origine restent en lecture, et le worker et sa cible Cargo sont sous runtime. Un test d'acceptation manquant une entrée n'annonce pas PASS. Le MP3 est dérivé localement sous runtime de `spkr0.wav` avec ffmpeg ; si l'outil est absent, la carte MP3 et A11 MP3 sont `NOT RUN`, et la qualification WAV ne les couvre pas.

**Après candidat L2a-1**, commandes proposées, non exécutées ; `CARGO_TARGET_DIR=target/parakeet-tests/l2/l2a-1/cargo/` et autres racines du même sous-lot :

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-core --test parakeet_ipc
cargo test --offline --locked -p whisper-worker-parakeet-cpu
cargo build --offline --locked --workspace
cargo test --offline --locked -p whisper-worker-parakeet-cpu --test native_import -- --ignored --nocapture
cargo test --offline --locked -p whisper-worker-parakeet-cpu --test decoder_parity -- --ignored --nocapture
git diff --check
git diff --name-only
```

Les deux tests natifs exigent leurs variables/chemins vérifiés au préflight et leurs scénarios FR/EN, WAV/MP3 réellement exécutés. A11 compare les blocs PCM issus du proxy `whisper-adapters/src/decoder.rs` et du décodeur Parakeet, avec le worker Turbo de décodage inchangé ; le proxy n'est pas lui-même un codec. Un résultat Cargo vert avec sous-scénario sauté ne ferme aucune carte native. L2a-1 rapporte le build du worker et le CPU réellement attesté, pas une preuve GPU. Le verdict CLEAN sur L2a-1 vient après les PASS FR/EN et la parité documentée.

**Après candidat L2a-2**, commandes proposées, non exécutées ; `CARGO_TARGET_DIR=target/parakeet-tests/l2/l2a-2/cargo/` et autres racines du même sous-lot. Reconstruire d'abord le worker, copier les DLL sous `l2a-2/native/`, vérifier leurs SHA et lancer A3 puis A11 sous `PARAKEET_L2_RUNTIME_ROOT=target/parakeet-tests/l2/l2a-2/`. Les fixtures FR/EN du spike sont lues sans modification ; le MP3 A11 est produit seulement sous `l2a-2/`. La fiche Test ne peut référencer que les SHA et identités réellement obtenus sur cette campagne :

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-worker-parakeet-cpu --test native_import -- --ignored --nocapture
cargo test --offline --locked -p whisper-worker-parakeet-cpu --test decoder_parity -- --ignored --nocapture
cargo test --offline --locked -p whisper-core
cargo test --offline --locked -p whisper-adapters --test parakeet_import --test parakeet_control --test legacy_model_requests --test model_catalog --test model_preferences
cargo test --offline --locked -p whisper-adapters --test parakeet_archive
cargo test --offline --locked -p whisper-desktop --test model_selection_ui
cargo build --offline --locked --workspace
git diff --check
git diff --name-only
```

L2a-2 exécute les scénarios d'import et d'archive avec son worker reconstruit après le changement borné de garde de racine ; la preuve L2a-1 demeure un prérequis historique et ne qualifie pas cette nouvelle clé. Une clé modifiée exige de rejouer FR/EN avant admission. Les tests Turbo natifs conditionnels restent `NOT RUN` si leurs entrées sont absentes, avec sous-scénarios dénombrés.

**Après candidat L2b**, commandes proposées, non exécutées :

```powershell
cargo fmt --all -- --check
cargo build --offline --locked -p whisper-worker-parakeet-cpu --features parakeet-qualification-reference
cargo test --offline --locked -p whisper-core --test parakeet_windows --test parakeet_import_contract
cargo test --offline --locked -p whisper-adapters --test parakeet_import --test parakeet_control
cargo test --offline --locked -p whisper-adapters --test parakeet_archive
cargo test --offline --locked -p whisper-worker-parakeet-cpu --test native_import --features parakeet-qualification-reference -- --ignored --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_long -- --ignored --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_long b9_reports_noise_floor_silence_counts_and_cut_plans -- --exact --nocapture
cargo test --offline --locked -p whisper-core
cargo test --offline --locked -p whisper-desktop --test model_selection_ui
cargo build --offline --locked --workspace --target-dir target/parakeet-tests/l2/l2b/cargo-regression/
git diff --check
git diff --name-only
```

Avant la première commande L2b, fixer `CARGO_TARGET_DIR` à `target/parakeet-tests/l2/l2b/cargo/`, construire la build Test L2b avec feature, fixer `PARAKEET_L2_WORKER` au binaire de cette cible et vérifier sa clé worker/runtime/protocole ainsi que celle des DLL copiées. La commande `native_import` rejoue FR et EN parlés sur **ce binaire**, avec comptes et résultats explicites ; sans ses deux PASS, ne pas lancer une recette qui prétend qualifier B8. `parakeet_long` lance ensuite référence et VAD via le même chemin et archive les identités observées de chaque exécution. Le build workspace de régression utilise sa cible distincte `cargo-regression/` même si `CARGO_TARGET_DIR` pointe encore vers `cargo/` ; constater que le worker qualifié conserve les mêmes octets avant/après cette régression. La régression finale ne doit ni reconstruire ni substituer le worker qualifié ; tout changement pertinent de build, modèle, DLL ou protocole périme la preuve et impose de rejouer courts puis long. La build Test L2b avec feature et sa cible Cargo restent séparées du worker L2a ordinaire. Les temps de campagne proposés ne sont pas des critères produit. Les tests VAD 60 min sont de la `PRODUCT_VALIDATION`, non une répétition du spike clos.

Pour B9, créer les trois variantes **uniquement** sous `target/parakeet-tests/l2/l2b/` à partir de `spkr0_x12.wav` en mixant par ffmpeg `anoisesrc` de couleur rose constante aux niveaux −40, −30 et −20 dBFS ; vérifier le niveau produit et la durée avant le passage VAD. Le harness `parakeet_long` rapporte séparément pour chaque variante les silences admissibles, les coupures ou l'erreur « aucun silence ». L'issue B9 n'entre pas dans le PASS B8, mais son rapport (ou `NOT RUN` motivé) est présenté à l'utilisateur **avant** toute activation de `import-vad`.

## 6. Livrables et statut

Chaque sous-lot rapporte diff, sorties des commandes, nombre de scénarios exécutés, état `PASS/FAIL/NOT RUN/INCONCLUSIVE` de ses cartes, identité des octets natifs réellement chargés et limites. `rust_review_coordinator` orchestre un `rust_reviewer` indépendant, corrections à `rust_implementer`, au plus deux cycles par sous-lot. CLEAN documentaire du présent plan ne vaut pas autorisation de code, ni CLEAN d'un candidat, ni validation dans la fenêtre Windows. Aucun push ni acquisition de modèle.

Récupération : si une fiche Test ou un profil long échoue, conserver Turbo et le profil antérieur qualifié, laisser la nouvelle clé non admissible ; garder source, pending, résultats et diagnostics. Toute suppression éventuelle se limite aux sorties inventoriées dans runtime et attend une autorité précise. Ne pas réinitialiser le dépôt ni modifier les modèles pour faire passer la recette.

**Sortie attendue de cette étape :** revue indépendante du delta v5 versionné dans Git avant reprise de L2a-2 ; l'accord utilisateur du 16/10/2026 couvre l'ajout de `main.rs` et la requalification FR/EN sur le nouveau worker. L2b attend CLEAN de L2a-2 et un accord propre. La présente rédaction n'autorise aucune autre extension de périmètre.
