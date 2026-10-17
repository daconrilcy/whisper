Avancée : le rejeu des sorties IPC L2a-2 FR reproduit le refus de l'adaptateur ; les sorties directes L2a et L2b étaient identiques sur FR/EN courts.
Blocage : L2a-2 est admis pour le FR mais sa garde de passe unique refuse « Je » ; R4 interdit la nouvelle clé worker avant une qualification explicite.
Décision : corriger les mots et l'adaptateur dans R5-1, puis faire arbitrer l'admission distincte nécessaire aux recettes applicatives de R5-2.

# PLAN-PARA-L2b-R5 v2 — temps, mots et parcours applicatif Parakeet

**Statut : DRAFT v2 — seconde revue indépendante requise ; aucun code R5 autorisé.** Base Git `f5c8394fc2847a9439bf32d239e58cb0a7c75632` sur `wip/l2b-candidate`, issu de `8a4385e9` ; WIP L2b avec réserves et R4 CLEAN, B8 inactive. La revue v1 du commit `595466fc` a rendu PLANS NOT READY-R5 (R5-P01 à P03) ; cette version sépare R5-1 réalisable dans les chemins indiqués et R5-2 dépendant d'une nouvelle décision sur l'admission. Ce plan tient compte de la décision utilisateur du 16/10/2026. Il ne réadmet pas la clé L2b courte et ne modifie pas l'archive par anticipation.

## 1. Diagnostic prioritaire et requalification L2a-2

Le rejeu déterministe des sorties IPC du worker **L2a qualifié** (`545a4830…`) contre `worker_ipc.rs` donne le même refus pour `fr87.wav` (mot 36) et `fr27.wav` (mot 35) : `?` = 17 920–18 080 ms, `Je` = 18 000–18 080 ms. À 16 kHz, le chevauchement est de **1 280 échantillons = 80 ms**. La constante actuelle `SINGLE_PASS_OVERLAP_TOLERANCE_SAMPLES = 80` n'admet que **5 ms** ; la garde des lignes 4 259–4 265 retourne `InvalidTimestamp`. En forçant seulement cette garde, le regroupement des lignes 79–98 avance le début de `Je` à 18 080 ms, égal à sa fin, et refuse aussi le mot. Le test existant `single_pass_punctuation_boundary_keeps_the_baseline_split_behavior` confirme cette seconde branche. Les IPC directs `fr87`, `fr27`, `en15` ont respectivement 177, 59, 38 mots et sont identiques entre les deux workers pour texte et chaque start/end. Aucun `InvalidTimestamp` n'est émis par le worker sur ces trois fichiers.

**Conclusion de portée :** l'import applicatif FR sur la clé L2a-2 déjà admise échouerait à cette garde lorsqu'il reçoit ces sorties. La relecture de code et le rejeu IPC le démontrent ; un test applicatif réel FR reste `NOT RUN` dans ce diagnostic. La qualification **historique** L2a-2 reste **FAIL pour FR sur le rejeu** ; le nouveau validateur strict ne pourra pas transformer son ancien worker non normalisé en PASS. Seul un **successeur reconstruit**, identifié par sa nouvelle clé et une fiche, pourra fermer cette régression après recette applicative. Les preuves EN indépendantes demeurent distinctes.

## 2. Périmètre de correction R5-1 proposé

Chemins d'écriture produit/test demandés, **au plus ceux-ci**, à détailler avant toute autorisation d'implémentation :

| Obligation | Chemin |
| --- | --- |
| Tolérance en échantillons, validation stricte, regroupement et tests internes existants | `crates/whisper-adapters/src/worker_ipc.rs` |
| Tests de route de l'adaptateur si les tests internes ne couvrent pas le parcours | `crates/whisper-adapters/tests/parakeet_import.rs` |
| Normalisation unique et tests unitaires du worker | `crates/whisper-worker-parakeet-cpu/src/native_engine.rs`, `crates/whisper-worker-parakeet-cpu/src/native_ffi.rs` |
| Harness natif FR/EN avec nouveau worker | `crates/whisper-worker-parakeet-cpu/tests/native_import.rs` |
| Contrat de `Word` et invariants documentés, uniquement si le contrat change | `crates/whisper-core/src/parakeet_ipc.rs` |

Écriture de tests/exécution uniquement sous `target/parakeet-tests/l2/l2b/r5/` ; sources `models/`, `sound example/`, fixtures et DLL lues sans les modifier. Aucun réseau, téléchargement, code C++, changement de Cargo, publication ou push. Deux cycles revue/correction R5 au maximum après un plan accepté et un accord précis.

### R5.1 — unité et validation à l'entrée de l'adaptateur

Remplacer l'unité erronée par une constante nommée `TDT_FRAME_OVERLAP_SAMPLES = 80 * 16 = 1_280`, documentée comme 80 ms à 16 kHz. Son test d'unité vérifie **la conversion et la frontière de classification** : 1 280 est dans une trame, 1 281 la dépasse. Ce test n'admet pas un IPC chevauchant. À la réception IPC de passe unique, vérifier chaque mot **après normalisation worker** : texte non vide, `0 <= start < end <= durée`, débuts ordonnés et **aucun chevauchement résiduel, même de 1 échantillon** ; toute violation est attribuée avant génération de segment. Un chevauchement résiduel de 80 ms est donc refusé par l'adaptateur comme défaut du worker, bien que le normaliseur du worker puisse le corriger ; celui de 81 ms doit déjà échouer dans le worker. Le regroupement de phrase ne doit jamais produire un intervalle vide. Conserver les contrôles de source, génération, identité et Turbo.

### R5.2 — normalisation unique dans le worker et contrat IPC v1

La sortie `Event::Result.words` doit satisfaire ces invariants avant sérialisation : UTF-8 valide sans U+FFFD émis ; ponctuation seule attachée au mot lexical précédent sans intervalle propre ; `start < end`, ordre et absence de chevauchement. Pour deux mots consécutifs qui se chevauchent de 0 à 80 ms, avancer le début du second à la fin du premier si sa durée demeure positive ; sinon ramener la fin du précédent au début du second **si les deux durées demeurent positives**. Au-delà de 80 ms, erreur attribuée. Zéro chevauchement reste inchangé. Une correction impossible ne fabrique pas une durée positive : erreur attribuée.

Les octets d'un mot sont décodés **une fois après assemblage de tous ses tokens**, sans `String::from_utf8_lossy`. Une séquence invalide produit une erreur avec index du mot et cause, sans remplacement silencieux. Le texte global ne doit pas être fabriqué depuis une version lossy différente des mots ; le contrat IPC v1 décrit le rapport entre `text` et les mots normalisés. Tests unitaires : ponctuation seule, chevauchements 0/80/81 ms, second mot contenu dans le premier, UTF-8 multioctet coupé entre deux tokens et octets réellement invalides.

**Faisabilité ABI lue en revue :** `rnnt_greedy_decoder.cpp:460–468` concatène `sp_piece_text(piece)` dans `cur_.word` avant export ; `decoder.h:123–124` retire seulement le marqueur de frontière ; `src/asr/c_api.cpp:437–439` expose le mot assemblé par `word.c_str()`. Rust peut donc copier les octets du mot assemblé depuis `native_ffi.rs:277–280`, puis décoder strictement **au niveau de ce mot** au lieu du `String::from_utf8_lossy` actuel (`native_ffi.rs:325–337`). Les six JSON diagnostiques ne contiennent **aucun U+FFFD** ; le dernier mot de `fr27` est déjà `à...` (`U+00E0 U+002E U+002E U+002E`) dans les deux workers. L'affichage `�...` du premier compte rendu provenait de la console, pas de l'IPC. Si un futur résultat natif donne effectivement des octets invalides ou un U+FFFD littéral, le worker retourne une erreur attribuée ; il ne fabrique pas un mot français.

### R5.3 — validation et parcours

La validation stricte de l'adaptateur reçoit uniquement les mots normalisés du worker, puis regroupe sans déplacer un début au-delà de sa fin. Les erreurs de normalisation sont propagées comme échecs attribués, jamais converties en SRT tronqué. Tester le couple réel `? / Je` : `?` est suffixe du mot précédent ; `Je` est un mot distinct de durée strictement positive. Les tests de route couvrent la même demande que l'interface applicative, pas seulement un harness de worker.

## 3. Cartes d'acceptation et ordre

| Carte | Critère PASS | Ordre / état actuel |
| --- | --- | --- |
| R5-A1 | Conversion 80 ms = 1 280 échantillons ; classification 1 280 dans la borne / 1 281 hors borne ; IPC normalisé sans chevauchement accepté, tout chevauchement résiduel refusé. | Avant inférence ; NOT RUN. |
| R5-A2 | Tests de chaque règle : ponctuation, 0/80/81 ms, UTF-8 coupé entre tokens, octets invalides ; aucun U+FFFD émis. | Avant nouvel exécutable ; faisabilité ABI à prouver. |
| R5-A3 | FR et EN courts rejoués sur le nouveau binaire ; SHA du binaire, modèle et DLL relevés. La fiche **historique** L2a-2 garde son FAIL FR jusqu'à qualification applicative du successeur. | Après A1–A2 ; NOT RUN. |
| R5-A4 | `fr87`, `fr27`, `en15` par **parcours applicatif complet** : TXT et SRT produits, zéro `InvalidTimestamp` ; `fr27` sans U+FFFD et dernier mot en français valide ; `? / Je` conforme. | Après A3 et admission explicite ; NOT RUN. |
| R5-A5 | B5, B6 puis B9 exécutées sur **le même binaire** ; résultats séparés, B9 informative selon L2b. | Après A4 ; NOT RUN. |
| R5-A6 | Tests Turbo inchangés ; aucune admission nouvelle de clé L2b avant la carte applicable PASS et une **fiche explicite**. | Tout au long ; NOT RUN. |

**R5-1 s'arrête après A3. R5-2 reste à arbitrer :** R4 a retiré l'admission du worker L2b court dans `crates/whisper-adapters/src/archive.rs`, et la clé VAD y est liée à l'ancien hash. Le nouveau binaire R5 aura un autre hash. `worker_ipc.rs:1229–1232` refuse aussi une clé VAD non reconnue. Le parcours applicatif prépare l'archive avant de lancer le worker : **A4 et A5 ne peuvent pas passer sur ce binaire dans les seuls chemins de la section 2**. Une fiche documentaire seule ne modifie pas ces gardes. Après A3, il faut une décision écrite distincte pour un sous-lot d'admission borné incluant au minimum `archive.rs` et la garde VAD de `worker_ipc.rs`, une nouvelle fiche explicite pour la clé du successeur et les recettes applicatives A4 puis B5/B6/B9 du même binaire. Ces écritures **ne sont pas autorisées par le périmètre R5-1** et ne figurent pas parmi ses chemins d'écriture. R4 reste en place jusque-là : aucune réadmission implicite. La fiche historique L2a-2 demeure FAIL pour FR ; la fiche du successeur ne pourra rendre l'import FR admissible qu'après A4 PASS.

## 4. Préflight, vérifications et sortie

Avant code R5-1 : vérifier HEAD, état Git et chemins, binaires exacts L2a/L2b, SHA modèle/DLL, les trois WAV et l'absence de travaux concurrents ; inspecter la chaîne native assemblée sans décodage lossy. Si une de ces conditions échoue, arrêter `PLAN_CHANGE_REQUIRED`, sans édition source. Aucun code C++ n'est permis. Avant code R5-2 : obtenir la décision de périmètre sur les chemins d'admission et une fiche du nouveau hash ; sans elle, A4/A5 restent `NOT RUN` et R5 global n'est pas déclaré livré.

Vérifications prévues après éventuel accord : `cargo fmt --all -- --check`, tests unitaires core/adapters/worker pertinents, `cargo test --offline --locked -p whisper-adapters --test parakeet_import`, `cargo test --offline --locked -p whisper-worker-parakeet-cpu`, `cargo build --offline --locked --workspace`, `git diff --check`, puis recettes natives/applicatives exactes et inventaire des sorties TXT/SRT sous la racine R5. Les tests ignorés sont rapportés séparément. Les variables Cargo, temporaires et racines de sortie sont résolues sous `target/parakeet-tests/l2/l2b/r5/`, sans fichier de modèle suivi dans Git.

Le handoff R5-1 rapporte diff, tests réellement exécutés, hashes, A1–A3 et le FAIL FR historique L2a-2. R5-2 rapporte A4–A6 et B5/B6/B9 seulement après son accord écrit. Le reviewer indépendant rend CLEAN ou des constats attribués ; après deux cycles au plus par livrable, LIVRÉ AVEC RÉSERVES. **Le présent DRAFT ne vaut ni admission de clé ni accord d'implémentation.**
