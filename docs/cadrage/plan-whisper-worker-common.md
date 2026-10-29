Avancée : refactoring en commit local 1e8e3944 ; format et revue statique sans anomalie ; garde adaptée et auto-tests réussis.
Blocage : nouvelle interruption de V2 sur fichier temporaire hors racines non couvert ; V3/V9C non lancés.
Décision : validation suspendue, traces conservées, aucune extension des exceptions ni push.

# Plan d'implémentation — whisper-worker-common

**P-WCOMMON-01 v1, DRAFT**, 29 octobre 2026. Proposition `rust_plan_writer`, tâche common_plan, intégrée par la session principale seule auteure des fichiers. Contribution brute accessible dans son retour de cette session ; intégration utile sans revendication de copie verbatim.

Design parent : D-WCOMMON-01 v1, [whisper-worker-common.md](whisper-worker-common.md), contenu revu au commit `087af34797e91269b0614f95cd5e6c62cbe0a428`. Revue DESIGN CLEAN / DESIGN READY documentaire : [revue-whisper-worker-common.md](revue-whisper-worker-common.md), suivie au HEAD `466c756a5e26338117c3b86d9d7a8b8959ba4403`. Base code `7a4d20b2a115fc6dd57e3b40faa75731b35c1832`, branche evo-startup-perf.

Accord initial de rédaction : « Je valide cette qualification. Lance le cadrage de whisper-worker-common et prépare le plan d’implémentation, sans modifier le code. » Les statuts d'entrée et de remise documentaire ci-dessous décrivent cet état initial. Les accords ultérieurs de réalisation et les résultats courants sont consignés au §10 ; aucun moteur natif ni fenêtre Windows n'a été exécuté dans ce lot.

## 1. Résultat et responsabilités

Un lot **L-WCOMMON-01**, neuf tâches séquentielles, pour partager decoder Symphonia, encoder MP3 interne, transport IPC et vérification SHA-256. Common possède les algorithmes ; les petites façades CPU/GPU conservent signatures et appelants. Sessions/moteurs/politiques/attestation/schémas core/interface restent en place.

Quatre rapports seulement : design, revue DESIGN, présent plan, revue PLANS. Le futur bilan actualise le présent plan ; aucun rapport, manifeste d'empreintes ou mécanisme documentaire supplémentaire. La limite de dix tâches et dix fichiers de rapport est respectée ; elle n'est pas une limite de dix fichiers Rust.

Après accord précis : `rust_implementer` mode IMPLEMENT est responsable du code. Le candidat va à `rust_review_coordinator`, qui mandate un reviewer indépendant et renvoie les corrections à l'implémenteur, au maximum deux cycles. Le reviewer seul prononce CLEAN. Si profil indisponible, arbitrage utilisateur avant changement de responsable. La session principale écrit les documents ; aucun code concurrent sur les mêmes chemins.

## 2. Prérequis et autorisations à l'entrée

| Type | Prérequis | État actuel |
| --- | --- | --- |
| DOCUMENT | Design parent et revue exacte ; plan relu indépendamment | Design disponible ; revue PLANS à effectuer |
| DOCUMENT | Acceptation DEC-WC-01 et plan ; accord précis de coder/modifier manifestes et lockfile | Non accordé |
| CODE | Algorithmes/appelants de base identiques à référence ou delta expliqué | À revérifier avant T1 |
| EXECUTION | Garde compatible, racines autorisées, cache privé, auto-tests et surveillance AGENTS | Non établi ; bloque tout Cargo, baseline comprise |
| EXECUTION | ≥30 Gio libres ; CARGO_HOME/TEMP/TMP canoniques dans racine autorisée ; CARGO_TARGET_DIR canonique target/parakeet-tests/cargo-shared/ | À vérifier avant chaque campagne |
| EXECUTION | Rust 1.98.1, cible Windows MSVC, prérequis natifs CPU/GPU disponibles sans téléchargement | À vérifier sans installation |
| EXECUTION | Corpus MP3 gapless lisible/autorisé, délai et padding vérifiés | Non attesté ; scénario concerné NOT RUN sans corpus |

Aucune racine de campagne, adaptation de garde ou nouvelle surveillance créée par ce plan. Pas de réutilisation implicite d'un ancien lot ou de ses dérogations. Avant exécution, mandat concret et invocation compatible nécessaires ; sinon arrêt avant Cargo. CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only ; pas de cache utilisateur en écriture ni Cargo direct. Ces prérequis ne bloquent pas la préparation/revue documentaire du plan.

## 3. Portée fermée des chemins

Préfixes de notation : C = `crates/whisper-worker-common`, P = `crates/whisper-worker-cpu`, G = `crates/whisper-worker-gpu`. Une liste avec P et G désigne chacun des deux chemins, pas une autorisation d'écrire dans toute la sous-arborescence.

| Écritures prévues | Chemins |
| --- | --- |
| Manifestes et lock | Cargo.toml, Cargo.lock, P/Cargo.toml, G/Cargo.toml, C/Cargo.toml |
| Production common | C/src/lib.rs, decoder.rs, encoder.rs, ipc.rs, integrity.rs, observation.rs |
| Façades | Dans P et G : src/decoder.rs, src/encoder.rs, src/ipc.rs, src/integrity.rs ; en plus P/src/ipc_output.rs |
| Tests common | C/src/decoder/tests.rs, C/src/encoder/tests.rs, C/src/ipc/tests.rs, C/src/integrity/tests.rs, C/src/observation/tests.rs |
| Tests CPU | P/src/decoder/tests.rs, P/src/encoder/tests.rs, P/src/ipc/tests.rs, P/src/ipc_output/tests.rs |
| Tests GPU | G/src/decoder/tests.rs, G/src/ipc/tests.rs |
| Suivi documentaire | Présent plan, actualisation finale sans nouveau rapport |

Lecture sans modification : dans P et G main.rs/startup.rs/diagnostics.rs/session/mod.rs/session/decode.rs/session/import.rs/session/live.rs/memory_qualification.rs/qualification.rs ; GPU gpu_commands.rs/diagnostics/tests.rs/native_engine.rs/native_ffi.rs/attestation ; scripts/recette-build.rs, rust-toolchain.toml, .cargo/config.toml et manifestes actuels. Tous ces noms s'entendent sous leur src sauf les trois fichiers racine explicités.

Core/desktop/adapters/bootstrap/Parakeet ne sont pas modifiables. Chemin supplémentaire → PLAN_CHANGE ou DESIGN_CHANGE selon effet, sans extension automatique. Les fichiers de tests ne sont retirés qu'après inventaire des scénarios conservés.

## 4. Neuf tâches, graphe et sorties

| Tâche | Objet / critères / chemins | Prérequis → sortie observable |
| --- | --- | --- |
| **T1 caractériser l'existant** | WC02/03/04/06/08. Séparer CPU tests inline decoder/ipc dans tests séparés ; ajouter encoder/tests et déclarations cfg(test), algorithmes inchangés. Inventorier GPU et CPU ipc_output | DOCUMENT + CODE baseline + G-EXEC → table test→destination→AC, témoins déterministes réellement observés avant substitution |
| **T2 introduire common** | WC01/05, DEC02/05. Manifeste/lib/type observation ; members/workspace/liens workers ; dépendances directes conservées jusqu'à migration ; lock local cohérent | T1 réussi → common compilable, arêtes locales attendues, aucun package/version/checksum externe changé |
| **T3 partager IPC** | WC01/04/05/08, DEC03/04. Common ipc/tests ; façades P/G ipc et CPU ipc_output/tests. Incoming local ; CPU actif None, GPU recorder sous feature | T2 + témoins T1 → premier parcours partagé utilisé par les deux binaires, mêmes octets/refus |
| **T4 partager décodage** | WC01/02/05/08. Common decoder/tests ; façades/tests P/G ; union utile des tests et observation par appel | T3 + témoins PCM T1 → mêmes valeurs/fenêtres/plages/erreurs/arrêt callback |
| **T5 partager encodage** | WC01/03/05/06. Common encoder/tests ; façades P/G ; test CPU préalable. Attentes observées T1 figées et transférées | T4 + oracle paquets T1 → mêmes séquences par appel/finalisation/refus |
| **T6 partager intégrité et finir manifestes** | WC01/05/07, DEC01/05. Common integrity/tests et façades P/G ; retirer Symphonia/rusty_mp3 directs inutiles après migration tests ; **garder sha2 production** pour recette-hooks ; ajuster arêtes lock | T5 → quatre outils partagés, mêmes points de contrôle SHA et erreurs |
| **T7 conserver hooks/raccordement** | WC01/05/08. Common observation/tests et tests locaux listés ; revue façades/appelants ; forward l01-memory-qualification, signatures common stables | T6 → None sans collecte/JSON/trace ; Some sous feature ; mêmes stages/champs/distinctions CPU/GPU |
| **T8 valider et revoir** | WC01 à 08. Format/tests ciblés/matrice features/builds CPU et GPU distincts/lints/diff ; revue indépendante et corrections bornées mêmes chemins | T7 + G-EXEC → résultats attribués au candidat ; REQUIRED fermés ou réserves explicites |
| **T9 clore/remettre** | WC05/06. Actualiser présent plan avec fichiers/résultats/limites/espace libre ; préserver petites preuves selon AGENTS | T8 → bilan réel et NOT RUN, aucun push/install/paquet autonome/ouverture automatique fenêtre |

Graphe : T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9. Sorties CODE de chaque tâche servent la suivante ; résultats EXECUTION des contrôles utiles doivent être acquis avant poursuite selon AGENTS. Aucun parallélisme de code : fichiers et contrats se recoupent.

## 5. Témoins avant/après

PCM : WAV déterministe non silencieux ; débuts/tailles des fenêtres, fin source, f32::to_bits puis i16 des plages comparés. Le test existant 2 000 123 échantillons conserve 25 fenêtres de 80 000 puis 123. Requêtes invalides/source inaccessible-invalide/callback Err vérifiés séparément.

MP3 interne : séquence i16 déterministe, par exemple `((i * 997) % 60_001) - 30_000`, donnée et découpage fixes. Vide/multiples push/franchissement 16 000/finish/second finish/push refusé. Capturer par cas/appel/paquet ordre, longueur et SHA-256, y compris sorties vides/erreurs ; figer les valeurs **observées sur l'ancien algorithme** dans les attentes common. Jamais d'attentes calculées par le nouveau code ni auto-update. Identité des encodeurs CPU/GPU revérifiée avant T1 ; divergence nouvelle → coordinateur avant choix baseline.

IPC : comparer writers actifs, pas helper CPU historique ; compléter refus/limites manquants. Les fixtures utilisent exclusivement TEMP validé ; logs ordinaires du lanceur et attentes des tests suffisent. Aucun outil ni manifeste d'empreintes de qualification supplémentaire.

## 6. Commandes prévues — résultats d'exécution au §10

Commandes **internes** à transmettre à la garde compatible future ; jamais autorisation de lancement direct. Cwd C:/dev/whisper, shell PowerShell, cible déclarée x86_64-pc-windows-msvc. CPU/GPU dans des invocations distinctes, pour éviter l'unification CUDA d'une commande groupée. L'invocation du lanceur, sa racine et les variables concrètes restent à établir avant exécution, pas des commandes actuellement disponibles.

| ID | Commande interne proposée | Moment et résultat attendu |
| --- | --- | --- |
| V0 | `cargo test --offline --locked -p whisper-worker-cpu --bin whisper-worker-cpu -- --nocapture --test-threads=1` | T1 ancien code : tests et témoins avant extraction, logs capturés |
| V1 | `cargo check --offline --locked -p whisper-worker-common` | T2 cohérence locale ; lockfile préparé et vérifié hors campagne avant contrôle, ajustement explicitement validé dans cette session |
| V2 | `cargo test --offline --locked -p whisper-worker-common --lib` | Après T3 puis fin : contrats communs défaut |
| V3 | `cargo test --offline --locked -p whisper-worker-common --lib --features l01-memory-qualification` | Hooks Some/None et contrats sous feature |
| V4-P | `cargo test --offline --locked -p whisper-worker-cpu --bin whisper-worker-cpu` | Unitaire/raccordement CPU ; pas de modèle requis par cette sélection |
| V4-G | `cargo test --offline --locked -p whisper-worker-gpu --bin whisper-worker-gpu` | Même contrôle GPU séparément ; pas une exécution modèle |
| V5-P | `cargo test --offline --locked -p whisper-worker-cpu --bin whisper-worker-cpu --features l01-memory-qualification` | Unitaire/raccordement instrumenté CPU |
| V5-G | `cargo test --offline --locked -p whisper-worker-gpu --bin whisper-worker-gpu --features l01-memory-qualification` | Unitaire/raccordement instrumenté GPU séparé |
| V6-P | `cargo check --offline --locked -p whisper-worker-cpu --features recette-hooks` | Debug hooks + sha2 ; aucune campagne historique activée |
| V6-G | `cargo check --offline --locked -p whisper-worker-gpu --features recette-hooks` | Équivalent GPU séparé |
| V7-P | `cargo build --offline --locked --release -p whisper-worker-cpu` | Binaire CPU défaut ; pas preuve native |
| V7-G | `cargo build --offline --locked --release -p whisper-worker-gpu` | Binaire GPU défaut séparé ; pas preuve native |
| V8 | `cargo fmt --all -- --check` | Format ; écart préexistant hors scope distingué |
| V9-C | `cargo clippy --offline --locked -p whisper-worker-common --lib -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity` | Lints common ; aucun --fix |
| V9-P | `cargo clippy --offline --locked -p whisper-worker-cpu --bin whisper-worker-cpu -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity` | Lints CPU |
| V9-G | `cargo clippy --offline --locked -p whisper-worker-gpu --bin whisper-worker-gpu -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity` | Lints GPU séparés |
| V10 | `git diff --check` et lectures Git noms/diff manifestes-lock | Statique portée et dépendances, sans campagne |

V0 avant T2 sur lock existant ; tous les contrôles qui l'acceptent sont désormais offline/locked, y compris V1 après arbitrage utilisateur. Les éditions de Cargo.lock se font hors campagne ; aucune commande Cargo n'est autorisée à le modifier. Aucun téléchargement/mise à jour externe. Les contrôles sont échelonnés : V2/V4 après raccordements utiles, matrice finale T8 ; répéter seulement si nouveau changement/échec/inquiétude le justifie.

Refus release recette-hooks : contrôle statique de scripts/recette-build.rs et liens build des deux workers inchangés. Essai Cargo volontairement négatif NOT RUN ; aucun échec artificiel ni dérogation ajouté. Les variables historiques L01/L02/L04 et sorties traces ne sont pas activées automatiquement. Le test gapless ne vaut preuve que sur corpus autorisé avec délai/padding vérifiés ; son retour anticipé ne vaut pas PASS audio.

## 7. Carte d'exécutabilité et couverture réciproque

Owner réalisation : implémenteur ; jugement : reviewer. Coûts indicatifs, aucune cible produit nouvelle : tests courts hors compilation, build natif initial potentiellement dominant et durée non mesurée. Chaque commande ci-dessous reste conditionnée à G-EXEC.

| Scénario / AC | Tâches / chemins directs et indirects | Données, commande, observation et attendu | Coût / limite |
| --- | --- | --- | --- |
| WC01 source unique | T2–T7 common/façades ; main/startup/session/diagnostics en lecture | V10 puis V2/V4 : une implémentation par outil, deux raccordements effectifs | Revue courte ; builds G-EXEC |
| WC02 PCM/plages | T1/T4/T8 tests decoder CPU→common/façade GPU ; session/decode en lecture | WAV silence existant + non silencieux, V0 puis V2/V4 : 25×80 000+123, mêmes bits/valeurs/positions, EOF et refus | Secondes hors compilation ; TEMP validé |
| WC02 gapless | T1/T4/T8 decoder/tests common et raccordement P/G | Corpus WHISPER_L02_GAPLESS_MP3 vérifié ; V0/V2 : délai/padding retirés une fois, chronologie conservée | Corpus absent = NOT RUN |
| WC03 MP3 interne | T1/T5/T8 encoder/tests CPU→common ; session/live en lecture | PCM/découpage fixes, V0 puis V2 : mêmes paquets par appel, vide/finish répété/refus | Court hors compilation ; oracle initial figé |
| WC04 transport | T1/T3/T8 ipc/tests common/P/G, CPU ipc_output/tests ; startup/diagnostics/session en lecture | V2/V3/V4 : valide/successives/exact 1 Mio LF inclus/oversize/EOF/incomplet/JSON/serialization Err sans sortie/I/O Err propagée | Faible volume ; pas atomicité pipe |
| WC05 isolation | T2–T8 manifestes/lock/façades ; exclusions/config natifs en lecture | V6/V7/V10 : sources exclues intactes, versions externes fixes, CPU sans CUDA et GPU inchangé | Coût natif variable ; pas preuve moteur |
| WC06 couverture réelle | T1/T8/T9 plan/inventaire/tests listés | Ancien test→destination→AC ; commande/données/candidat/résultats et retour conditionnel explicite | Revue/bilan ; actuel NOT RUN |
| WC07 empreinte | T6/T8 integrity/tests common ; session/decode/import et GPU gpu_commands en lecture | V2 : contenu connu/digest bon, mismatch/inaccessible ; mêmes points d'appel | Millisecondes/secondes ; pas de modèle |
| WC08 hooks | T3/T4/T7/T8 observation/tests et façades/tests IPC-decoder P/G ; recorders/qualification en lecture | V2–V6 : défaut inactif, Some/None sous feature, mêmes stages/champs, CPU actif sans trace serialize/GPU avec, refus release conservé statiquement | Court hors build ; aucune nouvelle destination trace |

Toute écriture §3 appartient à une tâche §4 et à un scénario ci-dessus. Les lectures indirectes n'autorisent pas de modification cachée. Les quatre documents restent les seuls rapports. Une preuve manquante limite l'AC correspondant, sans succès global inventé.

## 8. Arrêt, récupération, clôture

Avant chaque tâche : état Git, design/plan/revues et accord courant ; préserver non suivis docs/architecture/, models/, sound example/ et tout changement concurrent. Arrêt si baseline diverge, attente non capturée, contrat/chemin supplémentaire, lock externe dérive, garde/prérequis absent. Ne jamais changer l'oracle pour faire passer le candidat.

Échec/écriture hors racine : arrêt AGENTS et conservation des traces, arbitrage requis avant reprise. Pas d'effacement/restauration externe sans état antérieur vérifié. Pas de reset global, nettoyage cargo-shared, suppression récursive ou installation. Commits locaux circonscrits possibles après accord d'implémentation.

Clôture T9 : relever espace libre ; archiver petites preuves selon AGENTS avec arborescence, nombre/SHA-256 lorsqu'applicable à la campagne ; copies/WAV temporaires retirés uniquement sur manifeste approuvé. Aucune suppression prévue par ce plan documentaire. Native Turbo réel et fenêtre Windows NOT RUN sans mandat distinct ; aucun paquet/recette autonome dans cette mission. Une remise Windows éventuelle appliquera les règles de profil du dépôt, sans copies de ressources inutilisées.

## 9. État de remise documentaire

Découpage/chemins/dépendances/scénarios/commandes proposés ; revue PLANS à effectuer. Aucun propre CLEAN/READY déclaré par le plan writer. Après verdict indépendant, le suivi courant sera dans revue-plan-whisper-worker-common.md sans modifier les exigences du plan revu.

Restent avant réalisation : acceptation utilisateur liste/design/plan, accord précis d'implémentation/manifestes/lockfile/campagne, résolution concrète G-EXEC. Tous les tests/builds/native/fenêtre : NOT RUN. Aucun code modifié.

## 10. Bilan de réalisation — 29 octobre 2026

Auteur du bilan : session principale. Base du candidat : `7d06cd0fa0a284248ad837585dd618c4a71e0951`, branche `evo-startup-perf`, modifications de travail non committées. Les quatre rapports prévus sont conservés ; aucun rapport supplémentaire créé. Le présent bilan et l'ajustement V1 remplacent les statuts historiques d'attente des sections précédentes pour cette réalisation.

### Accords et exécution

L'utilisateur a demandé la mise en œuvre de ce plan, puis autorisé précisément l'adaptation de la garde PowerShell et V0–V10 avec `target/parakeet-tests/worker-common/` pour les sorties/cache/temporaire et `target/parakeet-tests/cargo-shared/` pour la compilation. Il a ensuite accepté la consignation `UNATTRIBUTED` des événements sans attribution sous `%LOCALAPPDATA%/Temp`, sans destination volontaire supplémentaire et avec arrêt conservé pour les écritures attribuées hors racines. Aucune dérogation historique n'est héritée.

Les incidents Temp puis `.git/objects/pack/tmp_pack_gbctSt` ont provoqué des arrêts et leurs traces ont été conservées. L'investigation a relié l'activité Git observée aux captures de différences de l'application desktop ; reprise dans Codex CLI indépendant, desktop fermé. Avant reprise, aucune activité Git ni changement des packs n'a été observé pendant environ 39 secondes ; cela ne prouve pas une inactivité future. L'extension VS Code était encore chargée. Les auto-tests de garde ont ensuite réussi et la surveillance est restée active pendant les commandes.

La première tentative V1 sans locked a été arrêtée après un événement sur Cargo.lock. L'ajustement accepté explicitement est : « Je valide la préparation du lockfile hors campagne, V1 avec --locked, puis la reprise du plan et de la revue indépendante, sans push ». Seule la commande V1 attendue de la garde a été verrouillée ; aucune destination supplémentaire n'a été ouverte. La comparaison statique finale confirme les mêmes 440 packages externes, versions, sources et checksums ; les changements du lockfile portent sur les liens locaux et l'ordre normalisé de rubato/rustfft.

### Fichiers et comportement livrés

- Nouvelle crate `crates/whisper-worker-common/` : Cargo.toml, lib.rs, decoder.rs, encoder.rs, ipc.rs, integrity.rs, observation.rs et les cinq fichiers de tests séparés prévus.
- Cargo.toml et Cargo.lock racine ; manifestes CPU/GPU pour les liens locaux, le transfert des dépendances de décodage/encodage et le forwarding de feature.
- Façades CPU : decoder.rs, encoder.rs, ipc.rs, ipc_output.rs, integrity.rs et les trois nouveaux fichiers decoder/tests.rs, encoder/tests.rs, ipc/tests.rs.
- Façades GPU : decoder.rs, encoder.rs, ipc.rs, integrity.rs et adaptation de decoder/tests.rs.
- Garde et sorties dans la racine autorisée ignorée par Git ; aucun changement des autres crates, moteurs, schémas core ou interface.

Les signatures et appelants des façades sont conservés. Le writer CPU actif passe `None` à l'observation ; le GPU utilise son recorder sous feature. Les signatures communes restent stables avec/sans instrumentation. `sha2` reste une dépendance directe de production des deux workers pour recette-hooks. Les éléments préexistants non suivis `docs/architecture/`, `models/` et `sound example/` ont été préservés.

### Inventaire de conservation des tests

| Scénario ou test initial | Destination et preuve conservées | Critère |
| --- | --- | --- |
| CPU/GPU `long_source_is_streamed_as_bounded_contiguous_windows` | Test commun : 25 fenêtres de 80 000 puis 123 ; façades raccordées | WC02 |
| CPU/GPU `lame_gapless_delay_and_padding_are_trimmed_once` | Test commun conservé, NOT RUN sans corpus | WC02 |
| CPU/GPU `rejects_oversized_and_incomplete_frames_before_deserialization` | Tests locaux conservés et common `frames_and_limits_are_preserved` | WC04 |
| CPU `rejects_serialized_frames_over_one_mibibyte` et GPU `writer_accepts_limit_and_rejects_limit_plus_one` | Tests locaux et commun ; limite LF inclus | WC04 |
| CPU actif `writer_accepts_limit_and_rejects_limit_plus_one_without_growing_past_it` | Test local ipc_output conservé | WC04 |
| Témoins T1 sur anciens algorithmes | Common `capture_non_silent_pcm_baseline` et `packet_sequence_matches_original_encoder` ; valeurs PCM et 57 paquets MP3 figés selon résultats observés | WC02/03 |
| Cas ajoutés après revue | Source 16 échantillons, plage [12,20) : Some, longueur 4, PCM attendu ; source existante invalide : UnsupportedFormat | WC02 |

### Vérifications réellement exécutées

Toutes les commandes Cargo ont utilisé la garde et les racines du lot ; CPU/GPU séparés, variables privées et réglages de compilation conservés. Les commandes exactes, dates, codes et chemins stdout/stderr sont dans `target/parakeet-tests/worker-common/logs/guard.jsonl`.

| Contrôle | Résultat et portée |
| --- | --- |
| V0 ancien code | PASS, 11 tests ; témoin `V0-20261029T145304123-373fd4ed.stdout.log` avant extraction |
| V1 corrigé avec locked | PASS sur la crate initiale T2 ; les contrôles ultérieurs couvrent le candidat final |
| V2/V3 common défaut/instrumenté | PASS, 12 tests par configuration après correction ; runs `20261029T151258767-8511b76d` et `20261029T151302802-fdd0f3f9` |
| V4/V5 CPU et GPU défaut/instrumentés | PASS, 9 tests CPU et 19 tests GPU par configuration |
| V6 CPU/GPU recette-hooks dev | PASS ; refus release contrôlé statiquement dans scripts/recette-build.rs, aucun échec artificiel lancé |
| V7 builds release CPU/GPU | PASS ; changements postérieurs limités aux tests communs, sans changement du code de production |
| V8 format | PASS sur candidat corrigé, run `20261029T151308112-91fca393` |
| V9 Clippy common/CPU/GPU | PASS avec avertissements ; dernier delta après ces commandes limité à deux tests decoder communs, validés par V2/V3/V8 |
| V10 Git/lockfile | git diff --check PASS ; comparaison externe lockfile identique, portée vérifiée |

### Revue indépendante et clôture

Le coordinateur `common_review_loop` a mandaté un `rust_reviewer` indépendant en lecture seule. Deux passes de revue, une correction confiée au même implémenteur ; aucun code concurrent par la session principale. Le finding IMPORTANT `R-WC-01` portait sur deux scénarios manquants de WC02, sans défaut fonctionnel établi : source lisible invalide et plage chevauchant EOF. Seul decoder/tests.rs commun a été modifié pour cette correction. Le même reviewer a fermé le finding et rendu **CLEAN sur le candidat de travail epoch 1**, sans demande de vérification ouverte.

Limites conservées : MP3 gapless NOT RUN sans corpus ; aucune exécution audio native ni validation dans la fenêtre Windows ; decode_wav_windows reste volumineuse (271 lignes signalées par Clippy), sans défaut nouveau établi par la revue. Aucun paquet ni installation préparé dans ce mandat.

Petites preuves copiées en préservant l'arborescence vers `target/parakeet-tests/worker-common/archive/20261029-worker-common/logs/` : 83 fichiers, 156 597 octets ; nombre identique et SHA-256 de chaque copie vérifié contre sa source, sans manifeste supplémentaire. Journaux d'origine et traces des incidents conservés ; aucun WAV/cache/copieur temporaire supprimé, cargo-shared intact. Espace libre final relevé : 122 242 064 384 octets (environ 113,8 Gio). Aucun push ; candidat source non committé, bilan documentaire destiné au commit local prévu par AGENTS.

## 11. Complément accepté — réduction de decode_wav_windows (29 octobre 2026)

Avancée : complément précisément autorisé dans la session courante par « oui j'autorise ce complement » après présentation du périmètre.
Blocage : aucun pour le code ; campagne conditionnée à la compatibilité de la garde existante avec les règles courantes.
Décision : lot complémentaire autonome de quatre tâches, commits locaux autorisés, aucun push.

Objectif produit : faciliter la maintenance du décodage commun WAV/MP3 sans modifier la transcription ni les contrats WC02/WC08. Base source : 2d9c3fe8, branche evo-startup-perf ; seuls docs/architecture/, models/ et sound example/ sont non suivis avant ce complément et restent préservés.

Chemins autorisés : crates/whisper-worker-common/src/decoder.rs et src/decoder/tests.rs ; présent plan écrit par la session principale. Aucune dépendance, manifeste, Cargo.lock, façade worker ou autre module modifié. Helpers privés dans decoder.rs ; aucun nouveau fichier de production requis.

1. Préparer les extractions privées et conserver les invariants ; implémentation par rust_implementer en mode IMPLEMENT.
2. Vérifier sous la garde existante uniquement : V8 format, V2 tests communs par défaut, V3 avec l01-memory-qualification, V9C Clippy structure ; git diff --check hors campagne. Tests complémentaires seulement si nécessaires à la préservation des contrats.
3. Revue par rust_review_coordinator avec reviewer indépendant et corrections par le même implémenteur, au maximum deux cycles.
4. Bilan dans cette section et commits locaux circonscrits ; pas de push, installation, paquet ou fenêtre Windows.

Acceptation : decode_wav_windows et les helpers nouveaux restent sous 100 lignes selon Clippy, sans allow/expect du lint ; signatures publiques, profils WAV/MP3 admis/refusés, valeurs PCM et interpolation, bornes/positions des fenêtres, compteurs/EOF, erreurs et arrêt immédiat callback inchangés ; observations mémoire (stages/champs/ordre/Some/None et durées de vie pertinentes) conservées. Pas de changement d'oracles pour masquer une différence.

Campagne autorisée dans les racines existantes target/parakeet-tests/worker-common/ (cache, TEMP/TMP, sorties) et target/parakeet-tests/cargo-shared/ (Cargo). Réutiliser guard-worker-common.ps1 sans modification ni surveillance supplémentaire. Préflight >=30 Gio, chemins canoniques, cache privé, réglages AGENTS, auto-tests et surveillance obligatoires avant Cargo ; offline/locked. Aucun accord nouveau de dérogation aux écritures hors racines : si la garde exige une exception historique, ne pas la réutiliser implicitement ; arrêter la campagne et rapporter le besoin précis. Ne pas effacer de traces ni de temporaires non approuvés. MP3 gapless sans corpus attesté et validation Windows restent NOT RUN.

### Bilan intermédiaire du complément

Implémentation confiée à rust_implementer (decoder_implement), revue confiée à rust_review_coordinator (decoder_review_loop), avec un rust_reviewer indépendant. Un seul fichier source modifié : crates/whisper-worker-common/src/decoder.rs ; tests existants et leurs oracles conservés. Signatures publiques inchangées, extractions privées de la préparation, de la lecture/conversion et de l'émission. Corps principal : 75 lignes physiques ; helpers : 22 à 62 lignes physiques selon l'implémenteur ; ces mesures ne remplacent pas Clippy.

Contrôles réellement exécutés : rustfmt ciblé --edition 2024 --config skip_children=true --check PASS ; git diff --check ciblé PASS. Revue indépendante statique : aucune anomalie concrète identifiée sur PCM/interpolation/compteurs/erreurs/callback/observations mémoire (stages, champs, ordre et durées de vie). Verdict global BLOCKED, aucun finding de code ouvert, un passage et zéro correction ; aucun CLEAN global annoncé.

V8 via Cargo, V2, V3 et V9C NOT RUN : guard-worker-common.ps1 lignes 180–205 laisse poursuivre les événements Temp utilisateur UNATTRIBUTED selon une exception d'une ancienne session, non héritée par §11. Aucun Cargo, auto-test de garde, adaptation de garde ou surveillance supplémentaire lancé pour ce complément. La poursuite proposée exige un accord précis pour adapter cette garde existante à l'arrêt sur toute détection hors racines, avec auto-tests, puis exécuter V8/V2/V3/V9C. Corpus MP3 gapless et fenêtre Windows NOT RUN. Aucun nettoyage ni archive de campagne nouvelle : aucune campagne lancée. Les non-suivis restent préservés. Candidat consigné localement, sans push ; objectif complet encore en attente des vérifications.

Espace libre final de cette étape : 124 057 853 952 octets (environ 115,5 Gio).

### Reprise autorisée — garde stricte

Accord courant reçu après proposition précise d'adaptation de la garde existante à l'arrêt sur toute détection hors racines, puis auto-tests et vérifications prévues : « Oui je t'autorise l'adaptation ».

Le complément autorise désormais uniquement la modification ciblée de target/parakeet-tests/worker-common/guard-worker-common.ps1 : supprimer la poursuite UNATTRIBUTED et adapter son auto-test au refus, conserver les protections existantes et les deux racines autorisées. Aucun nouveau mécanisme de surveillance, aucune écriture volontaire hors racines, aucune dérogation Temp ni installation. Les événements synthétiques des auto-tests ne créent pas les chemins interdits représentés. Garde locale ignorée par Git ; son adaptation et ses résultats sont consignés dans le présent plan, sans nouvel artefact de gouvernance.

Le même implémenteur réalise l'adaptation puis SelfTest, V8, V2, V3, V9C séquentiellement. Toute détection réelle hors racines ou échec impose l'arrêt et la conservation des traces avant arbitrage ; les refus synthétiques attendus des auto-tests ne sont pas des fuites réelles. La session principale suspend ses écritures et opérations Git pendant la surveillance. Revue reprise ensuite par le même coordinateur et le même reviewer sur le candidat source 1e8e3944 et les nouvelles preuves ; aucun nouveau cycle de correction engagé sans finding.

Espace libre avant reprise : 123 701 194 752 octets (environ 115,2 Gio).

### Résultat de la reprise — arrêt conservatoire

Garde locale adaptée par le même implémenteur : suppression de Get-TempEventClassification et de la branche de poursuite UNATTRIBUTED ; remplacement de l'auto-test de poursuite du Temp inconnu par un refus synthétique avec arrêt du processus. La garde demeure locale sous target et n'est pas ajoutée à Git. Aucun code Rust ni oracle modifié depuis 1e8e3944.

SelfTest PASS (sortie 0) : variables absentes/hors racines/jonctions refusées, écriture autorisée admise, événements synthétiques hors racines et Temp suspect/inconnu arrêtés sans créer les fichiers interdits. V8 PASS (sortie 0), cargo fmt --all -- --check via garde ; logs V8-20261029T152706205-e6bdb224.

V2 FAIL (sortie lanceur 1) : processus interrompu par la garde après début de compilation, aucun résultat de test exploitable. Événement Changed sur C:\Users\cyril\AppData\Local\Temp\NGL\NGLClient_CoreSync17.8.10.1.log, event_at 2026-10-29T15:27:10.4742973+01:00, detected_at 2026-10-29T15:27:10.5165829+01:00. Origine et attribution indéterminées ; ce constat ne prouve pas une écriture Cargo. Arrêt obligatoire respecté, aucune reprise, V3/V9C NOT RUN. Traces conservées dans target/parakeet-tests/worker-common/logs/guard-events.jsonl et V2-20261029T152710219-3631573a.{stderr,stdout}.log. Aucun nettoyage ni archive de clôture créé après arrêt ; aucun fichier externe effacé/restauré.

Le verdict de revue global reste BLOCKED faute des vérifications V2/V3/V9C ; la revue statique précédente demeure sans finding concret, aucun nouveau cycle de correction. Une reprise nécessite un nouvel arbitrage selon AGENTS. Espace libre après arrêt : 123 447 803 904 octets (environ 115,0 Gio). Aucun push.

### Reprise autorisée — exception au journal exact

Accord courant « oui je t'autorise » en réponse à la proposition explicite : exception limitée au journal C:\Users\cyril\AppData\Local\Temp\NGL\NGLClient_CoreSync17.8.10.1.log, événements consignés sans arrêt, afin de reprendre les tests et Clippy. Autorisation limitée à cette campagne ; aucune conclusion sur l'origine de ce journal, aucune destination volontaire supplémentaire pour Cargo ni exception du dossier NGL/Temp en général.

Le même implémenteur adapte la garde locale existante par comparaison canonique exacte de ce chemin (sans préfixe/glob ; renommage vers/depuis autre chemin reste refusé), consigne les événements concernés et conserve l'arrêt pour tout autre événement hors racines et toute erreur du watcher. Auto-tests synthétiques : journal exact consigné sans arrêt ; fichier voisin/autre chemin et Temp inconnu refusés. Puis SelfTest et V2/V3/V9C séquentiels ; V8 déjà PASS sur source inchangée. Tout autre incident impose arrêt/conservation/arbitrage, sans relance automatique. Aucun changement Rust prévu. Main suspend écritures/Git durant la surveillance. Espace libre avant reprise : 123 944 775 680 octets (environ 115,4 Gio).

### Résultat de la reprise avec exception exacte

Le même implémenteur a adapté la garde locale pour le seul journal NGLClient_CoreSync17.8.10.1.log accepté, avec comparaison exacte, contrôle des jonctions, journalisation et refus des voisins/renommages vers ou depuis un autre chemin. SelfTest PASS (sortie 0) : exception exacte et refus synthétiques, sans création des chemins interdits. L'origine des événements du journal n'est pas établie par cette exception.

V2 à nouveau interrompu (sortie lanceur 1), aucune validation de test acquise : événement Created sur C:\Users\cyril\AppData\Local\Temp\dbcb27b0-fe1d-4d90-8046-93252bd41a22.tmp à 2026-10-29T15:40:51.1785040+01:00, détecté à 2026-10-29T15:40:51.2212108+01:00. Origine et attribution indéterminées, sans conclusion d'écriture Cargo. Ce chemin n'appartient pas à l'exception acceptée. Arrêt/conservation respectés ; V3 et V9C NOT RUN, aucune relance. Logs conservés dans target/parakeet-tests/worker-common/logs/guard-events.jsonl et V2-20261029T154050944-e0a5a2eb.{stdout,stderr}.log. Aucun nettoyage ni archivage de clôture après cet arrêt. Espace libre final : 124 146 872 320 octets (environ 115,6 Gio).

Le candidat source 1e8e3944 est inchangé : décomposition effectuée, format PASS et revue statique indépendante sans finding, mais critère Clippy et tests non confirmés. Le verdict global reste BLOCKED. Pas de nouvelle passe de revue ou de correction en l'absence de nouveau candidat ou de résultat de validation. L'exception n'est pas élargie aux fichiers Temp aléatoires ni au dossier Temp. Validation suspendue selon AGENTS ; reprise soumise à arbitrage avant nouvelle campagne. Aucun push.

### Nouvelle demande de terminer les vérifications

L'utilisateur demande dans la session courante : « termines les tests pour etre sur que tout soit fonctionnel ». Cette instruction autorise la reprise explicite de V2/V3/V9C sur le même candidat et sous la garde actuelle, avec l'unique exception au journal exact déjà accordée. Elle n'est pas interprétée comme une dérogation générale au Temp ni comme autorisation d'ignorer un nouvel incident. Même implémenteur, aucune modification produit ou garde supplémentaire. L'arrêt antérieur et ses traces restent conservés ; toute nouvelle détection non couverte impose à nouveau l'arrêt selon AGENTS. Libre avant reprise : 123 561 951 232 octets. Main suspend ses écritures/Git pendant la surveillance.
