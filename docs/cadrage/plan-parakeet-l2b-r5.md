Avancée : le rejeu des sorties IPC L2a-2 FR reproduit le refus de l'adaptateur ; les sorties directes L2a et L2b étaient identiques sur FR/EN courts.
Blocage : L2a-2 est admis pour le FR mais sa garde de passe unique refuse « Je » ; l'ABI native ne fournit que des mots déjà assemblés et R4 interdit la nouvelle clé worker.
Décision : proposer R5 pour la cause réelle, puis faire arbitrer les prérequis de qualification que les chemins d'écriture annoncés ne couvrent pas.

# PLAN-PARA-L2b-R5 v1 — temps, mots et parcours applicatif Parakeet

**Statut : DRAFT — revue indépendante requise ; aucun code R5 autorisé.** Base Git `f5c8394fc2847a9439bf32d239e58cb0a7c75632` sur `wip/l2b-candidate`, issu de `8a4385e9` ; WIP L2b avec réserves et R4 CLEAN, B8 inactive. Ce plan tient compte de la décision utilisateur du 16/10/2026. Il ne réadmet pas la clé L2b courte et ne modifie pas l'archive par anticipation.

## 1. Diagnostic prioritaire et requalification L2a-2

Le rejeu déterministe des sorties IPC du worker **L2a qualifié** (`545a4830…`) contre `worker_ipc.rs` donne le même refus pour `fr87.wav` (mot 36) et `fr27.wav` (mot 35) : `?` = 17 920–18 080 ms, `Je` = 18 000–18 080 ms. À 16 kHz, le chevauchement est de **1 280 échantillons = 80 ms**. La constante actuelle `SINGLE_PASS_OVERLAP_TOLERANCE_SAMPLES = 80` n'admet que **5 ms** ; la garde des lignes 4 259–4 265 retourne `InvalidTimestamp`. En forçant seulement cette garde, le regroupement des lignes 79–98 avance le début de `Je` à 18 080 ms, égal à sa fin, et refuse aussi le mot. Le test existant `single_pass_punctuation_boundary_keeps_the_baseline_split_behavior` confirme cette seconde branche. Les IPC directs `fr87`, `fr27`, `en15` ont respectivement 177, 59, 38 mots et sont identiques entre les deux workers pour texte et chaque start/end. Aucun `InvalidTimestamp` n'est émis par le worker sur ces trois fichiers.

**Conclusion de portée :** l'import applicatif FR sur la clé L2a-2 déjà admise échouerait à cette garde lorsqu'il reçoit ces sorties. La relecture de code et le rejeu IPC le démontrent ; un test applicatif réel FR reste `NOT RUN` dans ce diagnostic. La qualification L2a-2 est **à requalifier pour FR / statut de la carte import FR : FAIL sur le rejeu**, sans invalider les preuves EN indépendantes. Ne pas déclarer L2a-2 globalement CLEAN pour l'import FR tant que le parcours complet ne passe pas.

## 2. Périmètre de correction proposé

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

Remplacer l'unité erronée par une constante nommée `TDT_FRAME_OVERLAP_SAMPLES = 80 * 16 = 1_280`, documentée comme 80 ms à 16 kHz, avec tests à 1 280 et 1 281 échantillons. À la réception IPC de passe unique, vérifier chaque mot normalisé : texte non vide, `0 <= start < end <= durée`, débuts ordonnés et **aucun chevauchement** ; toute violation est attribuée avant génération de segment. Le regroupement de phrase ne doit jamais produire un intervalle vide. Conserver les contrôles de source, génération, identité et Turbo.

### R5.2 — normalisation unique dans le worker et contrat IPC v1

La sortie `Event::Result.words` doit satisfaire ces invariants avant sérialisation : UTF-8 valide sans U+FFFD émis ; ponctuation seule attachée au mot lexical précédent sans intervalle propre ; `start < end`, ordre et absence de chevauchement. Pour deux mots consécutifs qui se chevauchent de 0 à 80 ms, avancer le début du second à la fin du premier si sa durée demeure positive ; sinon ramener la fin du précédent au début du second **si les deux durées demeurent positives**. Au-delà de 80 ms, erreur attribuée. Zéro chevauchement reste inchangé. Une correction impossible ne fabrique pas une durée positive : erreur attribuée.

Les octets d'un mot sont décodés **une fois après assemblage de tous ses tokens**, sans `String::from_utf8_lossy`. Une séquence invalide produit une erreur avec index du mot et cause, sans remplacement silencieux. Le texte global ne doit pas être fabriqué depuis une version lossy différente des mots ; le contrat IPC v1 décrit le rapport entre `text` et les mots normalisés. Tests unitaires : ponctuation seule, chevauchements 0/80/81 ms, second mot contenu dans le premier, UTF-8 multioctet coupé entre deux tokens et octets réellement invalides.

**Gate de faisabilité avant code :** `native_ffi.rs:277–280` appelle l'ABI `nemo_speech_asr_result_word_text` puis `copy_c_string`, qui effectue actuellement `String::from_utf8_lossy` à `native_ffi.rs:325–337`. L'ABI inspectée (`include/nemo_speech/asr.h:269`, `src/asr/c_api.cpp:437`) expose un mot C déjà assemblé, **aucun tableau de tokens**. Il faut démontrer que les octets de tous les tokens du mot sont encore présents dans cette chaîne et que le texte global peut être reconstruit sans U+FFFD. Si l'ABI a déjà perdu des octets ou émet elle-même U+FFFD, l'exigence `fr27` ne peut pas être fermée dans les chemins Rust permis : `PLAN_CHANGE_REQUIRED`, sans contournement ni substitution de texte.

### R5.3 — validation et parcours

La validation stricte de l'adaptateur reçoit uniquement les mots normalisés du worker, puis regroupe sans déplacer un début au-delà de sa fin. Les erreurs de normalisation sont propagées comme échecs attribués, jamais converties en SRT tronqué. Tester le couple réel `? / Je` : `?` est suffixe du mot précédent ; `Je` est un mot distinct de durée strictement positive. Les tests de route couvrent la même demande que l'interface applicative, pas seulement un harness de worker.

## 3. Cartes d'acceptation et ordre

| Carte | Critère PASS | Ordre / état actuel |
| --- | --- | --- |
| R5-A1 | Constante 1 280 échantillons, frontières 1 280 PASS / 1 281 FAIL, et validation d'adaptateur stricte. | Avant inférence ; NOT RUN. |
| R5-A2 | Tests de chaque règle : ponctuation, 0/80/81 ms, UTF-8 coupé entre tokens, octets invalides ; aucun U+FFFD émis. | Avant nouvel exécutable ; faisabilité ABI à prouver. |
| R5-A3 | FR et EN courts rejoués sur le nouveau binaire ; SHA du binaire, modèle et DLL relevés ; fiche L2a requalifiée si affectée. | Après A1–A2 ; NOT RUN. |
| R5-A4 | `fr87`, `fr27`, `en15` par **parcours applicatif complet** : TXT et SRT produits, zéro `InvalidTimestamp` ; `fr27` sans U+FFFD et dernier mot en français valide ; `? / Je` conforme. | Après A3 et admission explicite ; NOT RUN. |
| R5-A5 | B5, B6 puis B9 exécutées sur **le même binaire** ; résultats séparés, B9 informative selon L2b. | Après A4 ; NOT RUN. |
| R5-A6 | Tests Turbo inchangés ; aucune admission nouvelle de clé L2b avant la carte applicable PASS et une **fiche explicite**. | Tout au long ; NOT RUN. |

**Gate d'admission bloquant pour A4 :** R4 a retiré l'admission du worker L2b court dans `crates/whisper-adapters/src/archive.rs`. Le hash du nouveau worker R5 sera encore différent. Le parcours applicatif complet prépare l'archive avant de lancer ce worker ; avec les chemins d'écriture listés en section 2, A4 **ne peut pas passer sur ce nouveau binaire**. Une fiche documentaire seule ne modifie pas cette garde. Il faut une décision explicite de périmètre et un plan autorisant l'admission de la nouvelle clé dans `archive.rs` **après** preuve de normalisation/harness, ou un sous-lot de qualification séparé ; aucune réadmission implicite pendant R5. Une recette applicative sur L2a historique peut requalifier L2a mais ne prouve pas R5-A4 pour le nouveau worker.

## 4. Préflight, vérifications et sortie

Avant code : vérifier HEAD, état Git et chemins, binaires exacts L2a/L2b, SHA modèle/DLL, les trois WAV et l'absence de travaux concurrents ; établir la possibilité réelle de lire les octets natifs complets et obtenir une décision sur l'admission du nouveau hash. Si une de ces conditions échoue, arrêter `PLAN_CHANGE_REQUIRED`, sans édition source. Si la source du U+FFFD est NeMo lui-même, arrêter sans modifier C++ sous ce plan.

Vérifications prévues après éventuel accord : `cargo fmt --all -- --check`, tests unitaires core/adapters/worker pertinents, `cargo test --offline --locked -p whisper-adapters --test parakeet_import`, `cargo test --offline --locked -p whisper-worker-parakeet-cpu`, `cargo build --offline --locked --workspace`, `git diff --check`, puis recettes natives/applicatives exactes et inventaire des sorties TXT/SRT sous la racine R5. Les tests ignorés sont rapportés séparément. Les variables Cargo, temporaires et racines de sortie sont résolues sous `target/parakeet-tests/l2/l2b/r5/`, sans fichier de modèle suivi dans Git.

Le handoff rapporte diff, tests réellement exécutés, hashes, résultats A1–A6, statut L2a-2 requalifié et B5/B6/B9. Le reviewer indépendant rend CLEAN ou des constats attribués ; après deux cycles au plus, LIVRÉ AVEC RÉSERVES. **Le présent DRAFT ne vaut ni admission de clé ni accord d'implémentation.**
