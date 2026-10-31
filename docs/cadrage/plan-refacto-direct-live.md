Avancée : découpage réalisé ; nouvelle règle Temp appliquée et contrôles C1–C9 réussis sous garde.
Blocage : aucun pour les contrôles automatisés et la revue ; essais natifs et fenêtre Windows non exécutés.
Décision : refactoring CLEAN après validation ; règle Temp permanente révisée avec accord explicite.

# Plan de refactoring du direct microphone — PLAN-DL v1

## 1. Parent, mandat et statut

Parent : [DESIGN-DL v2](refacto-direct-live.md), révision Git `5071a8f0a4d31162d2208364dc25b199805794d1`. Sa conception a été déclarée DESIGN CLEAN sur `1a2522bfa8caececde517c898496bbbd035a3a03`, après fermeture de DL-D01 ; voir [la revue](revue-refacto-direct-live.md).

Mandat utilisateur du 31 octobre 2026 : besoin et cible indicative d’environ 600 lignes validés ; cadrage, plan, revue indépendante et commit local autorisés, **sans implémentation ni push**. Statut : PLANS READY documentaire après verdict PLANS CLEAN sur `d38a20e41fc8c21d2bf513a1de0d04d481142297`, sans constat résiduel ; retour brut dans la revue associée. Cette mise à jour de statut ne modifie pas le plan revu. L’acceptation utilisateur du lot et l’autorisation de coder ou de lancer Cargo restent distinctes et absentes.

Objectif L-DL : réduire et clarifier `direct_live.rs`, par les extractions du design, en conservant les comportements effectifs. Aucun nouveau modèle, format, canal, thread, protocole, dépendance, feature ou traitement d’erreur.

Owner de la proposition : `rust_plan_writer`, PLAN-DL v1 ; entrées DESIGN-DL v2 et ARCH-DL-v2, AGENTS et contrat de cadrage. Auteur des fichiers : session principale. Base produit inchangée `17cd4927ea431ab7fb5b8c0fde7cda8c7ccd07e0`.

## 2. Périmètre des chemins

Les huit chemins Rust suivants sont les seuls chemins de code proposés en écriture, tous relatifs à `crates/whisper-adapters/src/` :

| Chemin | Destination prévue |
| --- | --- |
| `direct_live.rs` | État et orchestration de session, constantes, window_size, déclarations/réexports |
| `direct_live/capture.rs` | Capture, télémétrie, drainage et diagnostics |
| `direct_live/worker.rs` | Événements, attestation, repli et démarrage worker |
| `direct_live/outputs.rs` | Finalisation incomplète et exports Turbo/Parakeet |
| `direct_live/wav.rs` | WAV et helper libre de remplissage du backlog |
| `direct_live/tests.rs` | Tests contrôlés actuels, éventuels témoins de caractérisation bornés |
| `direct_live/d2_corpus.rs` | Module D2 et ses six tests purs, sans changement d’activation |
| `direct_live/test_support.rs` | Cinq méthodes de test initiales, entièrement sous cfg(test) |

`lot.paths.read` : AGENTS, design/plan/revue Git, manifestes et Cargo.lock, cartes d’architecture disponibles, module source et consommateurs `worker_ipc/runtime/{mod,live,support}.rs`, `worker_ipc/tests.rs`, ses supports et `tests/microphone_and_finalization.rs`, `parakeet_live.rs`, `microphone_probe.rs`, capture/probe/transports existants nécessaires à la compréhension. Ces consommateurs restent sans édition.

Les trois documents déjà prévus dans `docs/cadrage/` peuvent recevoir le suivi attribué de leur propre mission par la session principale. Aucun autre rapport n’est imposé.

Une future campagne proposerait `target/parakeet-tests/refacto-direct-live/` pour résultats, temporaires, journaux et cache Cargo isolé, et exclusivement `target/parakeet-tests/cargo-shared/` pour la compilation. **Ces racines sont proposées pour une acceptation future ; aucune création ou campagne n’est autorisée maintenant.** Aucun autre dossier de lot, modèle, corpus, installation ou profil utilisateur ne devient une destination.

## 3. Préconditions et préflight

| ID/type | Condition avant consommation | Owner et état présent |
| --- | --- | --- |
| DOC-DESIGN / DOCUMENT | DESIGN-DL v2 Git et verdict CLEAN lisibles ; décisions et invariants inchangés | Session principale ; établi |
| DOC-PLAN / DOCUMENT | Plan et revue indépendants versionnés, périmètre accepté | Établi ; accord d’implémentation et décisions ultérieures consignés au §8–11 |
| CODE-BASE / CODE | Base produit et API des consommateurs revalidées ; différences ultérieures expliquées | Implémenteur ; à vérifier avant lot |
| EXEC-GO / EXECUTION | Accord écrit précis pour L-DL, huit chemins et vérifications ; livraison Git de code distincte | Acquis pour code et validations (§8–11), aucun commit de code ou push déduit |
| EXEC-GUARD / EXECUTION | Garde existante applicable et conforme, auto-tests et surveillance requis ; accord distinct si adaptation nécessaire | Établi après révision permanente et auto-tests PASS (§11–12) |
| EXEC-ENV / EXECUTION | Au moins 30 Gio libres ; variables canoniques, absence de jonction hors racine, outils/cache offline et prérequis de build disponibles | Établi pour les contrôles exécutés (§12) |
| EXEC-BASELINE / EXECUTION | Mesures et inventaire initiaux avant extraction ; témoins compilés initiaux | Mesures/inventaire établis ; baseline compilée antérieure NOT RUN, poursuite explicitement autorisée (§8), contrôles finaux exécutés (§12) |

Avant le premier changement : relever Git et les responsabilités concurrentes, préserver les non-suivis, vérifier la stack et les entrées ci-dessus. L’espace libre observé au cadrage ne remplace pas le nouveau relevé. EXEC-BASELINE est la sortie de T1 requise avant T2/T3, pas un prérequis circulaire de T1.

Avant **chaque** commande Cargo : préflight de la garde applicable, CARGO_TARGET_DIR canonique sur cargo-shared, CARGO_HOME/TEMP/TMP canoniques sous la racine du lot, CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=line-tables-only. Cache utilisateur jamais utilisé en écriture ; copie offline sous lot seulement si nécessaire et autorisée. Toute variable manquante, fuite, jonction ou auto-test négatif interdit Cargo.

Les gardes archive et core/t10 ont des racines et comportements propres à leurs anciennes campagnes. Leur réutilisation ou une tolérance Temp ne sont pas déduites de ce plan. Si aucune garde applicable n’existe, arrêter la préparation d’exécution et demander uniquement l’accord borné nécessaire ; ne pas créer de lanceur ou de surveillance supplémentaire implicitement.

## 4. Index du lot L-DL

Owner de T1–T7 : `rust_implementer` en mode IMPLEMENT, uniquement après EXEC-GO. T8 : session principale pour orchestration/bilan, `rust_review_coordinator` puis reviewer indépendant pour la revue Rust. Tous les fichiers partagent l’état et les contrats du parent : aucune parallélisation des modifications.

Dépendances : `DOC/GO/GUARD/ENV → T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8`. Les sorties des tâches précédentes sont des prérequis CODE ; leurs vérifications concluantes sont des prérequis EXECUTION.

| Tâche | Besoins et chemins | Travail, sortie et preuve attendue |
| --- | --- | --- |
| **T1 — Établir la référence** | REQ-01 à 06 ; lecture des huit chemins/consommateurs | Relever mesures, symboles, signatures/visibilités, 25 tests recensés et conditions natives ; exécuter baseline ciblée C1–C4 et C6 sous garde. Distinguer 18 contrôlés, six purs D2 et un natif D2 non exécuté. Si échec, arrêter avant extraction. |
| **T2 — Séparer les tests** | REQ-02/06, AC-02/07, DEC-T03 ; parent, tests, d2_corpus, test_support | Déplacer sans perte les deux modules et cinq méthodes de support ; conserver noms complets/cfg. Cartographier la couverture sensible ; si une branche n’est pas couverte, ajouter seulement un témoin caractérisant le comportement actuel dans tests.rs, puis le vérifier avant extraction de production. Sortie : inventaire 25+n explicite, n justifié, baseline de caractérisation concluante. |
| **T3 — Extraire WAV/backlog** | REQ-02/03/05 ; parent, wav | Déplacer WavOutput/header/Drop/helper libre ; garder window_size et constantes parent. Préserver les API, utiliser le getter existant lorsque nécessaire. Premier parcours minimal démontrable : exactitude PCM→WAV, lecture backlog et arrêt avant moteur avec C5. |
| **T4 — Extraire les sorties** | REQ-02/05 ; parent, outputs | Déplacer exports/finalisation/timestamp et type outcome ; conserver signatures, réexports, noms, tri, compensations et JSON séparé. Contrôler la correspondance des corps et la visibilité des consommateurs ; preuves d’export finales en C2 et témoins T2. |
| **T5 — Extraire capture/télémétrie** | REQ-02/03 ; parent, capture | Déplacer pompage, drainage, télémétrie et diagnostics. pump_capture reste pub(crate) pour runtime/support ; ordre gain/PCM/recyclage/arrêt inchangé. Preuve statique des appels, puis C2/C4 en T7. |
| **T6 — Extraire le dialogue worker** | REQ-02/03/04 ; parent, worker | Déplacer handlers/helpers/spawn sans réordonner les bras ni ajouter de filtre. Conserver attestation, instance, repli Auto, compatibilité CPU historique et confirmation après WindowFinished. Nouveaux accès internes limités à pub(super), API existante inchangée. |
| **T7 — Consolider et vérifier** | REQ-01 à 06 ; huit chemins | Mesurer principal/extraits, justifier l’écart indicatif à 600 ; comparer inventaires/API/imports et exécuter C1–C4/C6–C9 sur candidat final. Produire résultats attribués, couverture AC et limites ; manifestes/lock/consommateurs inchangés. |
| **T8 — Revoir et clore** | REQ-01 à 06 ; diff et documents prévus | Faire revoir le candidat concret via le routage Rust du projet ; corrections dans les huit chemins par l’implémenteur, au plus deux cycles. Le reviewer seul ferme les constats. Bilan, espace libre final, preuves utiles et réserves ; commit de code seulement si accord précis applicable, jamais push implicite. |

T2 n’autorise aucun correctif fonctionnel : un défaut constaté est séparé. Une extraction requérant un neuvième chemin de code revient en PLAN_CHANGE ; une décision structurante nouvelle revient en DESIGN_CHANGE_REQUIRED.

## 5. Commandes proposées, jamais exécutées au cadrage

Contexte commun : **PowerShell**, cwd `C:\dev\whisper`, profil dev et features par défaut sauf mention. Chaque ligne ci-dessous est une **charge Cargo proposée à transmettre à la garde validée** ; elle n’est pas une instruction de lancement direct. --offline --locked est utilisé lorsque la sous-commande l’accepte.

Les processus de tests et les faux workers compilés par rustc héritent eux aussi des destinations isolées. Pour C3, fixer WHISPER_MIC_LEVEL_D2_ROOT sur un sous-dossier canonique neuf de la racine du lot ; ne pas activer WHISPER_MIC_LEVEL_D2_ENGINE. Vérifier les anciennes activations natives héritées et ne pas les réintroduire. Aucune recette native n’est déclenchée.

| ID | Charge proposée | Résultat attendu |
| --- | --- | --- |
| C1 | `cargo test --offline --locked -p whisper-adapters --lib -- --list` | Liste avant/après : mêmes noms actuels, ajouts T2 explicités ; aucune exécution D2 |
| C2 | `cargo test --offline --locked -p whisper-adapters --lib direct_live::tests:: -- --test-threads=1` | 18 scénarios actuels et éventuels témoins T2 passent avec leurs assertions |
| C3 | `cargo test --offline --locked -p whisper-adapters --lib direct_live::d2_corpus::tests:: -- --test-threads=1` | Six tests purs, diagnostics synthétiques inclus ; D2 natif non sélectionné |
| C4a | `cargo test --offline --locked -p whisper-adapters --lib worker_ipc::tests::microphone_gain_command_reaches_acquisition_and_finalized_wav_while_worker_is_delayed -- --exact --test-threads=1` | Gain transmis à l’acquisition, PCM/WAV et arrêt préservés dans le consommateur |
| C4b | `cargo test --offline --locked -p whisper-adapters --lib parakeet_live::microphone_spectrum_tests::parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav -- --exact --test-threads=1` | Spectre et WAV Parakeet inchangés |
| C4c | `cargo test --offline --locked -p whisper-adapters --lib microphone_probe::tests::probe_and_spectrum_reader_leave_stt_pcm_and_final_wav_byte_identical -- --exact --test-threads=1` | Identité des PCM et WAV avec sondes/télémétrie |
| C5 | `cargo test --offline --locked -p whisper-adapters --lib direct_live::tests::<nom> -- --exact --test-threads=1` | Trois appels séparés : nom = wav_output_keeps_the_exact_converted_samples, puis wav_backlog_fills_bounded_windows_without_drops_or_reordering, puis stop_before_engine_ready_flushes_captured_audio_to_the_wav ; un test réellement sélectionné par appel |
| C6 | `cargo test --offline --locked --workspace --no-run` | Compilation des tests et consommateurs sans lancer de recette ; prérequis natifs/offline vérifiés avant tentative |
| C7 | `cargo fmt --all -- --check` | Format conforme ; aucune réécriture pendant campagne |
| C8 | `cargo clippy --offline --locked -p whisper-adapters --all-targets -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity` | Pas d’erreur ni aggravation inexpliquée ; écarts structurels historiques signalés sans refonte automatique |
| C9 | `cargo build --offline --locked -p whisper-adapters -p whisper-desktop` | Compilation produit des consommateurs, sans ouverture de fenêtre ni remise de paquet |

C6 et C9 exigent les outils et dépendances natives déjà nécessaires aux cibles. Leur absence interdit d’improviser téléchargement, nouvelle technologie ou changement de profil : consigner le manque et faire arbitrer la vérification affectée. Aucun contrôle réussi ne prouve un moteur natif ou une fenêtre Windows.

## 6. Carte d’exécutabilité et couverture des critères

Owner des vérifications : implémenteur ; fermeture des constats : reviewer. Pour chaque ligne, lecture du code/fixtures existants, écriture de code limitée aux chemins T indiqués, écritures d’exécution seulement sous racine du lot et cargo-shared. Les données sont synthétiques/faux workers existants ; aucun modèle ni corpus utilisateur n’est écrit. Coûts indicatifs non mesurés : compilation initiale 10–30 min selon cache, tests contrôlés 1–5 min par groupe, lecture/mesures moins de 10 min ; ce ne sont ni délais contractuels ni résultats observés.

| Scénario / AC | Tâches et chemins directs ; consommateurs indirects | Données, observation et preuve proposée |
| --- | --- | --- |
| Taille/responsabilités, AC-01/02 | T1/T3–T7 ; huit fichiers, consommateurs §2 en lecture | Comptage physique identique avant/après, table symboles→fichier, diff des signatures et imports ; principal ~600 indicatif, API et visibilités inchangées ; mesures statiques + C6/C9 |
| Audio/gain/backlog/stop, AC-03 | T2/T3/T5/T7 ; parent/capture/wav/tests ; runtime et probe lus | Fixtures PCM et capture contrôlée ; octets exacts, ordre, gain appliqué une fois, bornes pending et vidage avant finalisation ; C2/C4/C5 |
| Confirmation/identité/télémétrie, AC-04 | T2/T5/T6/T7 ; parent/capture/worker/tests ; runtime/support lu | Événements Segment/WindowFinished, identités et séquences valides/périmées ; aucun DirectConfirmed prématuré, mêmes erreurs et progression ; C2 et témoins ciblés T2 si couverture manquante |
| CPU/GPU/Auto/compatibilité/échec, AC-05 | T2/T6/T7 ; worker/parent/tests/test_support ; transport lu | Faux workers et événements contrôlés : Ready/attestation, erreur GPU pré-Ready, arrêt précoce, échec après Ready, mode manuel, fallback CPU historique et interruption micro ; même backend/repli/messages/résultats ; C2 + témoins T2 pour branches absentes |
| Exports et erreurs, AC-06 | T2/T4/T7 ; outputs/wav/tests ; Parakeet lu | Segments/plages connus, collisions et échecs SRT/JSON contrôlés ; mêmes noms, contenu/temps/complete, compensations et diagnostics ; C2 + témoin T2 si échec JSON non couvert, C4b/C6 |
| Tests et conditions, AC-07 | T1/T2/T7 ; tests/d2_corpus/test_support/parent | Inventaire 25+n, mêmes noms/cfg/assertions, support absent du produit ; C1/C2/C3/C6/C9 |
| Moteurs natifs, matériel et fenêtre | Hors exécution L-DL proposée | NOT RUN : D2 natif non sélectionné par filtres positifs, pas de recette matérielle, ouverture Windows réservée à l’utilisateur si mission ultérieure |

Pour les témoins T2, comparer le comportement **avant extraction de production**, en conservant les gardes effectives, y compris Failed non universellement filtré et Err pour réponse inattendue/périmée. Ne pas ajouter d’infrastructure de production pour tester.

Cette carte couvre toutes les tâches T1–T8 et tous les chemins écrits : WAV→AC-03/06, outputs→AC-06, capture→AC-03/04, worker→AC-04/05, parent→AC-01 à 05, tests/support/D2→AC-07 et critères exercés. Les consommateurs indirects sont contrôlés sans édition. Aucun scénario obligatoire ne dépend d’un chemin de code supplémentaire.

## 7. Budget de vérification et reprise

Ordre proposé : baseline unique T1 ; inventaire et caractérisations nécessaires T2 ; trois témoins WAV T3 ; campagne finale ciblée T7. Ne pas relancer la suite entière après chaque déplacement. Répéter uniquement les contrôles affectés par une correction ou un échec nouveau. Pas de suite workspace exécutée à l’aveugle, de --all-features, de --fix, de téléchargement ou d’exclusion héritée silencieuse.

Toute fuite hors racine attribuée, autre violation surveillée hors du traitement Temp UNATTRIBUTED révisé au §11, violation de préflight, erreur de garde ou échec de commande impose l’arrêt prévu par AGENTS, conservation du chemin/heure/résultats et arbitrage avant reprise. Ne pas effacer la fuite ni restaurer un état inconnu. Une compilation manquante ou un test non exécuté reste NOT RUN.

Récupération : conserver diff et données, isoler la dernière extraction en cause par lecture Git, proposer la correction limitée ; pas de reset global, suppression récursive ou nettoyage de cargo-shared. Retirer uniquement des temporaires explicitement couverts par un manifeste de suppression approuvé ; sinon les conserver. Archiver les petites preuves utiles avec arborescence, comparaison du nombre et SHA-256 comme demandé par AGENTS, sans créer une campagne de qualification ou un manifeste de preuves supplémentaire.

La clôture distingue diff, vérifications, revue et validation Windows. Après deux cycles maximum, les constats résiduels sont livrés avec réserves à l’utilisateur. Le présent mandat se termine après revue et versionnement local des documents ; toute exécution attend son accord précis.

## 8. Suivi d’exécution du 31 octobre 2026

Les états « absent/non établi » du préflight et le mandat documentaire ci-dessus décrivent la référence revue. Ils sont complétés par les décisions et résultats suivants, sans modifier le découpage ni les critères.

Autorité actuelle : après « ok implemente la refactorisation », l’utilisateur a précisé :

> Oui, j’autorise l’adaptation de la garde PowerShell dans target/parakeet-tests/refacto-direct-live/, avec cache et temporaires isolés, auto-tests et sans tolérance Temp. Poursuis ensuite la refactorisation et les vérifications du plan.

EXEC-GO est donc acquis pour l’implémentation et les vérifications prévues. L’adaptation bornée de la garde et les racines proposées sont autorisées ; cela ne crée pas de tolérance Temp. Aucun commit de code ou push supplémentaire n’a été déduit de cet accord.

Owner exécutant : `rust_implementer`, agent `implementation_direct_live`, mode IMPLEMENT ; base `102a3f8d44cfdb52355d70abc0f263f9ff5af86f`, branche evo-startup-perf. Les trois familles de non-suivis initiaux sont préservées. La session principale assure uniquement le suivi documentaire.

Préparation réalisée : garde adaptée et cache copié sous `target/parakeet-tests/refacto-direct-live/`, temporaires et CARGO_HOME isolés, compilation prévue dans cargo-shared. La tolérance Temp de l’ancienne garde a été retirée. Les nettoyages internes des tests ciblés ont été examinés : ils concernent leurs propres fixtures créées sous les répertoires temporaires isolés ; aucun nettoyage manuel de campagne n’est autorisé par ce constat. Outils de build présents selon l’implémenteur ; cette présence ne vaut pas compilation.

Résultat observé : `pwsh -NoProfile -File target/parakeet-tests/refacto-direct-live/guard.ps1 -Mode SelfTest` a terminé en FAIL, code 1, au premier test de processus écrivant dans la racine autorisée. Le watcher a reçu un événement réel `Created` pour `C:\Users\cyril\AppData\Local\Temp\etilqs_CZNOPQLtgFiQagj` le **31 octobre 2026 à 11:50:41.5065092 +01:00**, puis a arrêté le processus surveillé. L’origine de l’écriture n’est pas attribuée ; le constat ne démontre ni une écriture Cargo (aucun Cargo lancé), ni une écriture autorisée. Le fichier n’était plus présent au contrôle en lecture rapporté par l’implémenteur ; aucune suppression ou restauration n’a été faite par l’équipe.

Preuves conservées : `target/parakeet-tests/refacto-direct-live/logs/selftest.log` et `guard-events.jsonl`. Aucun auto-test n’a été relancé après cet échec. EXEC-GUARD reste BLOCKED, T1 n’a pas commencé ses commandes Cargo, T2–T8 restent NOT RUN. Aucun fichier Rust modifié ; aucune revue de code lancée faute de candidat. Espace libre final relevé : environ 164 Gio.

La règle d’arrêt du dépôt et du §7 impose un nouvel arbitrage avant reprise. Une nouvelle tentative resterait stricte, sans tolérance ni exclusion supplémentaire ; elle ne peut être lancée sur la seule base de l’accord antérieur après cet échec.

### Nouvelle tentative explicitement autorisée

L’utilisateur a ensuite autorisé : « Oui, j’autorise une nouvelle tentative des auto-tests avec la garde stricte inchangée, sans tolérance Temp. S’ils passent, poursuis la refactorisation et les vérifications du plan. »

Cette unique nouvelle tentative a terminé en FAIL, code 1. La garde est restée inchangée. Le témoin d’écriture interne et les quatre contrôles synthétiques Temp Created/Changed/Deleted/Renamed ont passé ; la suite globale n’a pas abouti. Un nouvel événement réel Created, d’origine non attribuée, a été reçu pour `C:\Users\cyril\AppData\Local\Temp\etilqs_7z56XBMS6rgySEX` à **11:52:21.5984749 +01:00** le 31 octobre 2026. Le processus surveillé a été arrêté. Les enregistrements de refus synthétiques du même journal ne sont pas des écritures externes réelles.

Journal supplémentaire conservé : `target/parakeet-tests/refacto-direct-live/logs/selftest-retry-1.log`, avec événements dans guard-events.jsonl. Aucun Cargo, aucune modification Rust, aucune troisième tentative. EXEC-GUARD reste BLOCKED et les travaux T1–T8 n’ont pas commencé. La répétition ne démontre toujours pas l’origine de ces écritures et ne justifie aucune tolérance implicite ; l’étape utile suivante serait un diagnostic borné, à autoriser avant son engagement hors plan.

### Diagnostic et tentative de capture autorisés

L’utilisateur a autorisé successivement le diagnostic en lecture seule des processus et journaux existants, puis : « J’autorise une capture courte reliant les écritures Temp aux processus, sans installation ni élévation, pour identifier leur origine et débloquer la refactorisation. La garde doit rester stricte. »

Le diagnostic a confirmé les événements PowerShell aux deux instants. Les PID 106636 et 26864 portés par ces messages identifient les processus de garde qui ont journalisé les erreurs, pas les auteurs des écritures. Les deux fichiers temporaires ne sont plus présents lors des lectures. Aucun événement Application/System correspondant dans la fenêtre examinée ; Sysmon absent, journal Security inaccessible sans élévation. Le traçage Kernel-Process Analytic est désactivé et n’a pas été activé. Openfiles ne dispose pas du suivi local activé et refuse la consultation distante ; aucune configuration n’a été changée.

La famille de noms et l’inventaire des modules indiquent une piste SQLite, pas une attribution : plusieurs processus actifs chargent une bibliothèque SQLite, dont le processus principal de l’application Codex, OneDrive et plusieurs composants iCloud. La présence d’une bibliothèque n’établit pas qu’un processus a créé l’un des fichiers. Référence primaire sur les noms temporaires SQLite : https://www.sqlite.org/forum/forumpost/86225d47ab, réponse de Richard Hipp du 6 décembre 2022. Aucun contenu de base de données utilisateur consulté ou modifié.

Capture : Windows Performance Recorder, Logman et Tracerpt sont présents ; aucun Procmon/Handle/Process Explorer trouvé dans les emplacements examinés. Trois démarrages bornés ont été tentés avec les droits ordinaires, sans installation, élévation, service ou tâche planifiée :

| Tentative | Paramètres de capture pertinents | Résultat |
| --- | --- | --- |
| WPR FileIO, mémoire | Instance dédiée WhisperDirectLiveTemp, temporaires dirigés sous lot ; arrêt après 30 s prévu seulement si démarrage réussi | Refus immédiat 0x800705aa |
| WPR FileIO.light, disque | Tampons plus petits (20 × 1 Mio par collecteur au lieu de milliers en mémoire), même instance et racine de sortie | Même refus immédiat 0x800705aa |
| Logman Microsoft-Windows-Kernel-File | Session éphémère dédiée via -ets sans enregistrement/planification ; 2–8 tampons de 64 Kio, fichier circulaire limité à 16 Mio sous lot ; durée prévue 30 s | Même refus immédiat 0x800705aa |

Le journal Windows `Microsoft-Windows-Kernel-EventTracing/Admin`, événement 2, confirme les échecs de démarrage avec statut **0xC000009A**, à 11:58:03.803, 11:58:39.287 et 11:59:20.123 (+01:00) le 31 octobre 2026. Le message signale des ressources système insuffisantes ; la cause précise de ce refus n’est pas démontrée et il n’est pas requalifié en refus d’accès. Aucun redémarrage, arrêt de processus tiers, réglage de traçage ou changement de privilège tenté.

Preuves des refus conservées uniquement sous `target/parakeet-tests/refacto-direct-live/temp-attribution/` : wpr-start.txt, wpr-file-start.txt, logman-start.txt. Les captures n’ont pas démarré : aucun ETL d’attribution exploitable, aucune observation fichier→PID obtenue. Le contrôle final indique WPR non en cours et session Logman dédiée introuvable ; aucune nouvelle capture n’a été laissée active. Au terme de cette tentative, la garde était inchangée, aucun Cargo ni code Rust n’avait été modifié. EXEC-GUARD restait BLOCKED ; la capacité de capture n’était pas rétablie.

### Reprise du code sans attendre le diagnostic

L’utilisateur a ensuite demandé : « continue la refacto sans te soucier de ce probleme ». Cette instruction actuelle permet de poursuivre le découpage des huit chemins Rust et leur revue indépendante malgré l’absence de baseline compilée et l’échec du prérequis de garde. Elle remplace, pour la poursuite du code, l’arrêt précédemment consigné ; les critères fonctionnels et le découpage accepté restent inchangés.

La session principale a annoncé la séparation du code et des vérifications bloquées. La garde reste stricte et inchangée : aucun Cargo direct, aucune tolérance Temp, aucune nouvelle tentative automatique de campagne ou de capture déduite de cette instruction. Les vérifications compilées non réalisées restent NOT RUN, jamais PASS. Les contrôles statiques utiles et le formatage ciblé des seuls chemins autorisés sont effectués ; leurs limites seront soumises à la revue indépendante. L’implémenteur reprend T2–T7 ; la session principale organise T8 sur le candidat concret.

## 9. Bilan du candidat local

Auteur du code : `rust_implementer`, agent implementation_direct_live. Base Git `102a3f8d44cfdb52355d70abc0f263f9ff5af86f`, candidat dans l’arbre de travail, sans commit de code ni push. Les huit chemins prévus sont les seuls chemins Rust changés ; manifestes, Cargo.lock et consommateurs restent inchangés. Les changements documentaires sont ceux de la session principale. Les éléments non suivis antérieurs sont préservés.

| Chemin sous crates/whisper-adapters/src/ | Lignes physiques finales | Résultat structurel |
| --- | ---: | --- |
| direct_live.rs | 470 | État, orchestration et API conservés ; déclarations/réexports |
| direct_live/capture.rs | 148 | Capture, drainage, télémétrie et diagnostics |
| direct_live/worker.rs | 336 | Événements worker, attestation, repli et démarrage |
| direct_live/outputs.rs | 267 | Exports Turbo/Parakeet et finalisation incomplète |
| direct_live/wav.rs | 134 | Écriture/lecture WAV et remplissage du backlog |
| direct_live/tests.rs | 1262 | 18 tests existants et trois témoins supplémentaires |
| direct_live/d2_corpus.rs | 622 | Sept tests D2 conservés, dont un natif et six purs |
| direct_live/test_support.rs | 129 | Méthodes de support sous cfg(test) |

Le principal passe de 3 184 à 470 lignes. Le résultat est inférieur à l’estimation ~500–600 parce que les responsabilités et le support de tests sont entièrement sortis ; aucun code n’est ajouté ou compacté pour approcher artificiellement la cible indicative de 600.

Vérifications observées, attribuées : l’implémenteur a exécuté `rustfmt.exe --edition 2024 --check crates/whisper-adapters/src/direct_live.rs` avec TEMP/TMP isolés, résultat PASS (analyse des modules déclarés incluse), puis git diff --check PASS et contrôles des nouveaux fichiers. La session principale a recontrôlé les décomptes et git diff --check à la remise. La comparaison mécanique des blocs a été rapportée concluante par l’implémenteur puis examinée indépendamment par le reviewer. Ce sont des preuves statiques, pas des résultats de compilation ou de tests.

Les 25 tests initiaux sont conservés et trois témoins ont été ajoutés pour la confirmation après WindowFinished, l’attestation GPU et l’échec JSON. Inventaire source attendu : 28 tests ; découverte réelle par le runner, exécution des tests et nouveaux témoins NOT RUN. Aucun changement du traitement des événements Failed/périmés, de la politique de repli ou des formats n’est revendiqué. La revue n’a pas relevé de défaut actionnable dans les déplacements.

| Critères | Preuve disponible | Limite restante |
| --- | --- | --- |
| CA-01/02, AC-01/02 | Mesures, correspondance des responsabilités, revue statique API/visibilités et consommateurs, pump_capture pub(crate) conservée | Compilation des nouveaux modules non démontrée |
| CA-03/04, AC-03/04/05 | Corps et gardes déplacés comparés statiquement ; témoins existants conservés et deux témoins ciblés ajoutés | Audio, ordre, gain, arrêt, attestation/repli non vérifiés par exécution |
| CA-05, AC-06 | Exports partagés et API préservés à la lecture ; témoin JSON ajouté | Contenu/compensations et consommateurs non vérifiés par exécution |
| CA-06, AC-07 | Inventaire et noms/cfg conservés à la lecture, support séparé | Liste compilée et exécution des 28 tests non obtenues |

T2–T6 sont réalisés dans le code ; T1/T7 n’ont pas leurs preuves Cargo et ne sont pas déclarés validés. T8 a fourni une revue statique indépendante, dont la clôture est BLOCKED : aucun finding actionnable, mais absence de preuves exécutables. Un passage de revue, zéro correction, zéro vérification compilée ; pas de déclaration CLEAN Rust. Retour attribué dans revue-refacto-direct-live.md. Tous les contrôles Cargo C1–C9 restent NOT RUN, y compris Cargo fmt/Clippy ; le rustfmt direct n’est pas présenté comme l’exécution de C7. Moteurs natifs et fenêtre Windows NOT RUN.

Espace libre relevé à la remise : environ 162 Gio. Aucun nettoyage manuel, suppression de cache partagé, nouvelle tentative de garde ou capture réalisé pendant la reprise du code. Les traces précédentes sont conservées. Livraison du découpage avec réserve de validation ; la demande de poursuivre n’est pas transformée en preuve de non-régression exécutée.

## 10. Validation demandée sur le candidat final

Après la remise, l’utilisateur a demandé « fais les validations necessaires ». La session principale a confié à l’implémenteur initial le mode VERIFY, sans droit de modifier les sources, tests, garde, configuration ou Git. La demande couvre les contrôles C1–C9 et leur préflight strict ; aucune capture ou nouvelle tolérance Temp n’en a été déduite.

Préflight : mêmes huit chemins Rust du candidat, base 102a3f8d, changements documentaires de la session principale distingués des sources ; environ 161 Gio libres. Les résultats statiques antérieurs restent applicables, aucune source n’ayant changé depuis la revue.

Tentative observée : `pwsh -NoProfile -File target/parakeet-tests/refacto-direct-live/guard.ps1 -Mode SelfTest`, FAIL, code 1. Les quatre refus synthétiques Temp ont passé, mais la suite globale a été interrompue par un événement réel Created pour `C:\Users\cyril\AppData\Local\Temp\etilqs_qvbBfXiciDwaSiu` à **13:15:21.7861363 +01:00**, le 31 octobre 2026. L’origine reste non attribuée et ce constat n’est pas une écriture Cargo démontrée, aucun Cargo n’ayant été lancé. Garde inchangée, arrêt du processus surveillé, aucun nettoyage ou relancement après l’échec.

Preuves : `target/parakeet-tests/refacto-direct-live/logs/selftest-validation-20261031.log` et guard-events.jsonl. L’exécutant retourne mode VERIFY, statut BLOCKED ; C1–C9 restent NOT RUN. Aucun code, test, consommateur, manifeste ou lockfile modifié par cette tentative. Les événements synthétiques de refus enregistrés dans le même journal ne représentent pas de fichiers externes réellement créés.

La clôture de revue reste BLOCKED faute de preuves compilées ; aucun nouveau verdict CLEAN n’est annoncé. Le prérequis concret reste l’applicabilité de la garde dans l’environnement observé. Une nouvelle tentative identique ou une capture supplémentaire n’est pas programmée automatiquement.

## 11. Révision permanente de la règle Temp et reprise autorisée

L’utilisateur a demandé « revise la regle de temp qui est un blocage inutile », puis a explicitement choisi **« Modification permanente dans AGENTS.md »** à la question de portée. La proposition acceptée précise : événements non attribués sous le seul `%LOCALAPPDATA%\Temp` réel journalisés sans arrêt automatique, origine restant inconnue ; écritures volontaires hors racine et écritures attribuées à la campagne toujours interdites.

Cette décision AUTHORITY_CHANGE remplace les restrictions précédentes « sans tolérance Temp » uniquement pour le traitement des événements non attribués. Elle autorise la modification permanente du fichier AGENTS.md du dépôt et l’alignement de la garde du lot ; aucune règle de profil ou d’un autre projet n’est modifiée. Les exigences produit, les huit chemins Rust et les critères CA/AC ne changent pas.

Implémentation attendue dans la garde existante : classification UNATTRIBUTED avec chemin/type/horodatages pour Created/Changed/Deleted sous le Temp réel et Renamed entièrement interne à ce Temp ; pas de liste blanche de noms. Attribution démontrée à la campagne, renommage traversant, autre destination extérieure, emplacement protégé, erreur watcher ou préflight restent bloquants. Les auto-tests doivent vérifier ces deux familles par injection synthétique, sans créer de fichier externe. Aucun PID n’est inventé et aucune capture supplémentaire n’est ajoutée.

La demande de validations du candidat reste applicable : une fois la garde alignée et ses auto-tests passés, reprendre C1–C9 sous les mêmes racines et variables isolées. Les observations UNATTRIBUTED et leur risque résiduel figurent dans le bilan, sans devenir une preuve d’innocuité ni une autorisation d’écriture. La session principale reste auteur du suivi documentaire ; l’implémenteur initial possède l’adaptation de la garde puis l’exécution des vérifications, sans mutation du candidat Rust en mode VERIFY.

## 12. Résultats exécutés après révision Temp

Exécutant : implementation_direct_live ; adaptation de garde limitée au traitement Temp et aux auto-tests, puis mode VERIFY sur le candidat Rust inchangé. Cwd `C:\dev\whisper`, PowerShell, toolchain `1.98.1-x86_64-pc-windows-msvc`, profil dev, features par défaut. CARGO_HOME sous refacto-direct-live/cargo-home, TEMP/TMP sous refacto-direct-live/temp et tmp, CARGO_TARGET_DIR canonique cargo-shared, CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only. Toutes les charges Cargo sont passées exclusivement par la garde existante en mode Run ; aucun téléchargement ou commande Cargo directe.

La garde adaptée a passé ses auto-tests : les quatre types Temp non attribués sont journalisés sans arrêt ; l’attribution démontrée, les renommages traversants et les autres sorties interdites arrêtent le processus surveillé. Les contrôles de variables absentes, chemins hors racine et jonctions sont conservés. Journal `target/parakeet-tests/refacto-direct-live/logs/selftest-temp-rule-20261031.log`. Aucun fichier externe intentionnellement créé pour ces tests.

| Contrôle | Résultat actuel observé |
| --- | --- |
| C1 : compilation et liste adapters | PASS, code 0 ; 313 tests listés, dont les 28 tests direct_live : 21 directs et sept D2 |
| C2 : direct_live::tests:: | PASS, code 0 ; 21 tests réussis, aucun ignoré, dont les trois témoins ajoutés |
| C3 : direct_live::d2_corpus::tests:: | PASS, code 0 ; six tests purs réussis, D2 natif non sélectionné |
| C4a/b/c : consommateurs exacts du §5 | PASS, code 0 chacun ; un test sélectionné et réussi par commande |
| C5 : exactitude WAV, backlog et arrêt avant moteur | PASS par les trois mêmes témoins exécutés dans C2 sur le même candidat, sans double comptage |
| C6 : cargo test --workspace --no-run, offline/locked | PASS, code 0 ; tous les tests du workspace compilent, aucune exécution induite |
| C7 : cargo fmt --all -- --check | PASS, code 0 |
| C8 : Clippy adapters all-targets et quatre lints de structure | PASS avec avertissements, code 0 ; 184 avertissements lib test dont 61 doublons selon la sortie. Deux réexports inutilisés attribuables au candidat : ParakeetExportOutcome et export_parakeet_manifest. Les fonctions longues préexistantes déplacées produisent également des avertissements ; aucun succès sans avertissement n’est revendiqué |
| C9 : build adapters et desktop, offline/locked | PASS, code 0 ; aucune fenêtre ouverte |
| git diff --check | PASS, code 0 |

Total de tests exécutés sans double comptage : **30** (21 directs + six D2 purs + trois consommateurs). Pas d’exécution générale de la suite workspace. Recette D2 native, moteurs/micro/GPU réels et fenêtre Windows restent NOT RUN.

Les commandes ont observé **92 événements Temp UNATTRIBUTED** (30 Created, 32 Changed, 30 Deleted), journalisés selon la règle permanente acceptée. Leur origine reste inconnue, ils ne sont pas présentés comme des écritures de campagne autorisées ou comme une preuve d’absence de fuite. Aucun autre échec signalé par la garde, aucune mutation source/configuration/Git pendant VERIFY. Les non-suivis antérieurs sont préservés.

Les 37 petits journaux ont été archivés avec arborescence, nombre et SHA-256 individuels vérifiés dans `target/parakeet-tests/refacto-direct-live/archive/validation-20261031.zip` ; les originaux sont conservés, aucun nettoyage manuel. Espace libre final rapporté : environ 158,7 Gio. Les résultats NOT RUN/échecs précédents restent historiques et sont complétés, non effacés, par ces preuves nouvelles. La clôture de revue revient au même reviewer indépendant, notamment pour l’examen des deux avertissements nouveaux ; l’exécutant ne déclare pas CLEAN.

## 13. Clôture après revue des preuves

Le même reviewer indépendant, rust_reviewer_direct_live, a repris le candidat Rust inchangé et déclaré **CLEAN** après examen des preuves du §12. Coordination : rust_review_coordinator, agent review_refactor_direct_live. Base 102a3f8d, candidat arbre de travail epoch 0 ; deux passages de revue, zéro correction, un passage de vérification, aucun finding ou gap de vérification ouvert, blocker null. Retour attribué consigné dans revue-refacto-direct-live.md.

Les deux avertissements de réexport inutilisé sont ADVISORY : ParakeetExportOutcome et export_parakeet_manifest sont conservés pour le contrat de compatibilité DEC-T02. Aucun correctif ou masquage de lint n’est requis par le reviewer. Le succès de Clippy signifie code 0 avec les avertissements décrits, pas absence d’avertissements.

Résultat livré : découpage de 3 184 à 470 lignes, sept modules privés, 25 tests conservés et trois témoins ajoutés ; 30 tests ciblés réussis, compilation workspace des tests, format, Clippy et build adapters/desktop réussis. La règle permanente AGENTS distingue désormais l’observation Temp non attribuée d’une écriture de campagne démontrée ; les événements observés restent d’origine inconnue. Pas de validation native ou Windows inférée de ces contrôles. Pas de commit de code, push ou nettoyage manuel. Les éléments non suivis antérieurs sont conservés.

### Livraison Git autorisée après validation

L’utilisateur a ensuite demandé : « Fais le commit local de la refactorisation de direct_live.rs, de la règle Temp et des documents de suivi, sans push ni inclusion des fichiers étrangers à cette tâche. » Le commit de livraison porte uniquement les huit fichiers Rust du découpage, AGENTS.md et les deux documents de suivi modifiés. La mention précédente d’absence de commit décrit l’état de remise avant cet accord. Les dossiers non suivis docs/architecture/, models/ et sound example/ restent hors commit ; les artefacts locaux de campagne sous target/ ne sont pas ajoutés à Git. Aucun code n’est modifié à cette étape, les preuves de validation restent applicables.
