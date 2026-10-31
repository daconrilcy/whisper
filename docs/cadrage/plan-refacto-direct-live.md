Avancée : plan de refactoring établi à partir de DESIGN-DL v2.
Blocage : aucun GO d’implémentation ; garde de campagne applicable non établie.
Décision : PLANS READY documentaire après revue CLEAN ; lot de huit tâches séquentielles.

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
| DOC-PLAN / DOCUMENT | Plan et revue indépendants versionnés, périmètre accepté | Session principale pour versionnement/revue ; utilisateur pour acceptation, à produire |
| CODE-BASE / CODE | Base produit et API des consommateurs revalidées ; différences ultérieures expliquées | Implémenteur ; à vérifier avant lot |
| EXEC-GO / EXECUTION | Accord écrit précis pour L-DL, huit chemins, vérifications et éventuelle livraison Git de code | Utilisateur ; absent |
| EXEC-GUARD / EXECUTION | Garde existante applicable et conforme, auto-tests et surveillance requis ; accord distinct si adaptation nécessaire | Session principale ; non établi |
| EXEC-ENV / EXECUTION | Au moins 30 Gio libres ; variables canoniques, absence de jonction hors racine, outils/cache offline et prérequis de build disponibles | Implémenteur ; non vérifié |
| EXEC-BASELINE / EXECUTION | Mesures, inventaire et témoins initiaux concluants avant extraction de production | Implémenteur ; NOT RUN |

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

Toute fuite hors racine, violation de préflight, erreur de garde ou échec de commande impose l’arrêt prévu par AGENTS, conservation du chemin/heure/résultats et arbitrage avant reprise. Ne pas effacer la fuite ni restaurer un état inconnu. Une compilation manquante ou un test non exécuté reste NOT RUN.

Récupération : conserver diff et données, isoler la dernière extraction en cause par lecture Git, proposer la correction limitée ; pas de reset global, suppression récursive ou nettoyage de cargo-shared. Retirer uniquement des temporaires explicitement couverts par un manifeste de suppression approuvé ; sinon les conserver. Archiver les petites preuves utiles avec arborescence, comparaison du nombre et SHA-256 comme demandé par AGENTS, sans créer une campagne de qualification ou un manifeste de preuves supplémentaire.

La clôture distingue diff, vérifications, revue et validation Windows. Après deux cycles maximum, les constats résiduels sont livrés avec réserves à l’utilisateur. Le présent mandat se termine après revue et versionnement local des documents ; toute exécution attend son accord précis.
