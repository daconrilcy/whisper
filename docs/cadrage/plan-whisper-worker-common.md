Avancée : plan documentaire proposé sur le design Git revu.
Blocage : code/campagnes non autorisés ; garde compatible non établie.
Décision : soumettre un lot séquentiel de neuf tâches à la revue PLANS.

# Plan d'implémentation — whisper-worker-common

**P-WCOMMON-01 v1, DRAFT**, 29 octobre 2026. Proposition `rust_plan_writer`, tâche common_plan, intégrée par la session principale seule auteure des fichiers. Contribution brute accessible dans son retour de cette session ; intégration utile sans revendication de copie verbatim.

Design parent : D-WCOMMON-01 v1, [whisper-worker-common.md](whisper-worker-common.md), contenu revu au commit `087af34797e91269b0614f95cd5e6c62cbe0a428`. Revue DESIGN CLEAN / DESIGN READY documentaire : [revue-whisper-worker-common.md](revue-whisper-worker-common.md), suivie au HEAD `466c756a5e26338117c3b86d9d7a8b8959ba4403`. Base code `7a4d20b2a115fc6dd57e3b40faa75731b35c1832`, branche evo-startup-perf.

Accord exact de session : « Je valide cette qualification. Lance le cadrage de whisper-worker-common et prépare le plan d’implémentation, sans modifier le code. » Le plan est autorisé ; son exécution ne l'est pas. Les choix/liste fermée restent à accepter avant code. Aucun Cargo, test, build ou moteur exécuté pendant la rédaction.

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

## 6. Commandes envisagées, toutes NOT RUN

Commandes **internes** à transmettre à la garde compatible future ; jamais autorisation de lancement direct. Cwd C:/dev/whisper, shell PowerShell, cible déclarée x86_64-pc-windows-msvc. CPU/GPU dans des invocations distinctes, pour éviter l'unification CUDA d'une commande groupée. L'invocation du lanceur, sa racine et les variables concrètes restent à établir avant exécution, pas des commandes actuellement disponibles.

| ID | Commande interne proposée | Moment et résultat attendu |
| --- | --- | --- |
| V0 | `cargo test --offline --locked -p whisper-worker-cpu --bin whisper-worker-cpu -- --nocapture --test-threads=1` | T1 ancien code : tests et témoins avant extraction, logs capturés |
| V1 | `cargo check --offline -p whisper-worker-common` | T2 seule cohérence locale nouveau package/lock, futur mandat doit couvrir l'absence de locked ; contrôler immédiatement diff local |
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

V0 avant T2 sur lock existant ; V1 seul contrôle sans locked, puis tous les contrôles qui l'acceptent offline/locked. Aucun téléchargement/mise à jour externe. Les contrôles sont échelonnés : V2/V4 après raccordements utiles, matrice finale T8 ; répéter seulement si nouveau changement/échec/inquiétude le justifie.

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
