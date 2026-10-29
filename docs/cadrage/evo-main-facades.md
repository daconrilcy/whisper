Avancée : cadrage des cinq points d'entrée établi sur les sources courantes.
Blocage : aucune décision produit bloquante identifiée ; vérifications du futur code NOT RUN.
Décision : proposition de design soumise à revue indépendante puis validation utilisateur ; aucune implémentation autorisée.

# Whisper — Des `main.rs` limités au rôle de façade

## 1. Identité, mandat et sources

Design **D-MAIN-FACADES-01**, version 1, 29 octobre 2026. Auteur des documents : session Codex principale. Base de code : `301fbbeaaba8cede4ce079828125ba271c9928af`, branche `evo-startup-perf`. Statut initial : **DRAFT — revue documentaire à effectuer**. Ce document regroupe brief, exigences, domaine, architecture, acceptation, risques et esquisse future ; aucune nouvelle fonctionnalité ne justifie des documents séparés.

Source produit : expression de besoin « Whisper — Des fichiers main.rs limités au rôle de façade », pièce jointe `Texte collé.txt` de la demande du 29 octobre 2026. La qualification fournie porte le verdict PASS ; elle n'a pas été refaite. Demande de cette session : « fais le cadrage de ce besoin avec les agents necessaires ». Le mandat couvre le cadrage, les contributions en lecture seule, sa revue et son enregistrement avec commit local conformément à AGENTS ; ni implémentation, ni Cargo, ni campagne, ni push.

Décisions acquises dans le besoin fourni : traiter les fichiers **entiers**, organiser les extractions **par responsabilité**, préserver le comportement et les points d'entrée déjà conformes. L'accord documentaire ne vaut pas acceptation du design proposé.

Sources techniques primaires : cinq `crates/<crate>/src/main.rs`, manifestes, `AGENTS.md` racine et locaux, `RAILGUARD.md`, modules existants et tests cités ci-dessous. La carte `docs/architecture/ARCHITECTURE.md` et ses fiches orientent la lecture ; certaines plages CPU/GPU précèdent les sources actuelles. Elles ne sont pas la référence de vérité ni modifiées par ce cadrage.

État initial : aucun changement suivi ; dossiers non suivis `docs/architecture/`, `models/`, `sound example/` préservés. Espace libre C observé : environ 104,3 Gio ; ce relevé documentaire ne remplace pas le préflight du futur lot.

### Contributions attribuées

| Apport | Owner / version | Contenu et limites |
|---|---|---|
| BRIEF-MAIN-FACADES-v1 | `rust_product_framer`, `/root/cadrage/brief` | Besoin intégral, capacités, périmètre et inventaire ; pas d'audit exhaustif des workers. |
| REQ-MAIN-FACADES-v1 | `rust_requirements_analyst`, `/root/cadrage/exigences` | Exigences, UC, acceptation, erreurs et variantes ; lecture ciblée, aucune preuve d'exécution. |
| DOM-MAIN-FACADES-v1 | `rust_domain_architect`, `/root/cadrage/domaine` | Propriétaires, caches, corrélations, transitions et ordre des contrôles observés. |
| ARCH-MAIN-FACADES-v1 | `rust_architect`, `/root/cadrage/architecture` | Modules, chemins, visibilités, configurations, risques et esquisse ; moteurs/FFI lus de façon ciblée. |

Coordination : `rust_predev_orchestrator`, `/root/cadrage`. Les apports bruts sont restitués dans la session avant intégration ; le présent texte est une synthèse attribuée, pas une copie revendiquée exacte. Tous reposent sur la même base de code ; aucun spécialiste n'a écrit dans le dépôt. La revue indépendante est distincte de ces contributions.

## 2. Objectif, périmètre et inventaire

Les mainteneurs doivent comprendre rapidement le lancement d'un exécutable et retrouver chaque traitement dans un module identifié. Le bénéfice produit est la possibilité de corriger et faire évoluer les workers sans modifier leurs résultats ou leur fonctionnement actuel.

| Crate | Lignes physiques de main.rs, tests inclus | Traitement proposé |
|---|---:|---|
| whisper-desktop | 31 | Conserver ; horodatage initial, composition `root::compose` et lancement `whisper_desktop::run`. |
| whisper-bootstrap | 10 | Conserver ; références minimales de vue/protocole/transport, installation non activée. |
| whisper-worker-cpu | 1 497 | Extraire lancement, sessions, contrôles, instrumentation et tests. |
| whisper-worker-gpu | 2 573 | Même objectif, plus modules inline decoder/encoder/ipc et commandes GPU. |
| whisper-worker-parakeet-cpu | 1 246 | Extraire protocole, lancement, arguments, racines, intégrité, identité GPU, traitements et tests ; CPU/CUDA selon features. |

Ces comptes sont confirmés sur les fichiers actuels et ne mesurent pas la complexité. Aucun plafond de lignes arbitraire ne définit une façade.

Inclus : lancement lisible, responsabilités localisables, tests sortis des main entiers, préservation des modes et preuves de non-régression attribuées. Exclusions : nouvelle fonction, amélioration de performance, refonte du workspace, mutualisation systématique, nouvelle crate, nouvelle dépendance, changement de manifeste/lockfile, moteur, FFI, packaging ou ressources. Les réglages, installation, UI, archives et politiques moteur restent hors modification ; leurs contrats atteints restent préservés. Implémentation, campagne native et éventuelle recette sont différées à leurs autorisations propres.

## 3. Exigences, parcours et acceptation

| ID | Exigence | Besoin source / acceptation |
|---|---|---|
| REQ-01 | Les cinq main.rs ne portent que déclarations nécessaires, entrée, appel au lancement et traduction terminale éventuelle. | CA-01/05 ; AC-01 |
| REQ-02 | Extraire par responsabilité, avec consommateurs identifiés et tests dans des fichiers séparés ; ne pas déplacer un monolithe intact. | CA-02 ; AC-02 |
| REQ-03 | Préserver résultats, événements et ordre IPC, diagnostics, codes de sortie, fichiers et démarrage/attente/succès/arrêt/erreur. | CA-03 ; AC-03 |
| REQ-04 | Préserver versions, framing, identités, cache, racines, CPU/GPU, FFI et conditions d'activation. | CA-04 ; AC-04 |
| REQ-05 | Chaque changement sert extraction, liaison ou validation ; conserver desktop/bootstrap et éléments périphériques. | CA-05 ; AC-05 |
| REQ-06 | Distinguer analyse, compilation, tests contrôlés, exécution native et validation Windows, avec couverture réelle. | CA-03/04/05 ; AC-06 |

| UC | Données et action | Résultat observable / contrôle |
|---|---|---|
| UC-01 | Un mainteneur lit chaque entrée puis cherche un traitement. | AC-01 : revue du fichier entier, absence de boucle/traitement/test ; AC-02 : table origine→responsabilité→module→consommateur vérifiée par imports. |
| UC-02 | Référence puis candidat, mêmes entrées/configurations et ressources pour import/live/décodage disponibles. | AC-03 : mêmes données déterministes, ordre/corrélations/effets ; préserver les règles des valeurs variables. |
| UC-03 | Trame invalide, identité refusée, arrêt session, fermeture processus ou EOF selon moteur. | AC-03/04 : mêmes événements, poursuite ou fatal, codes et effets ; branches distinguées en section 4. |
| UC-04 | Construire et tester chaque configuration affectée. | AC-04 : conditions cfg, profils, refus et tests déplacés actifs ; matrice section 7, sans extrapoler CPU vers CUDA. |
| UC-05 | Examiner diff et bilan d'un candidat. | AC-05 : aucun chemin sans lien au besoin ; AC-06 : résultat par scénario/configuration, prérequis, limites et NOT RUN explicites. |

AC-03 ne signifie pas une égalité brute de PID, durées, noms temporaires ou empreinte du binaire reconstruit. Comparer les champs déterministes et préserver calcul, identité, corrélation et validité des valeurs variables. Toute normalisation doit être documentée sans masquer une différence de contenu, d'ordre, de diagnostic ou d'effet. Aucun nouveau délai d'arrêt ni seuil de performance.

## 4. Domaine et invariants observés

Les catégories suivantes décrivent le code existant ; aucune nouvelle machine d'états ou politique n'est introduite. Le parent possède la configuration et le choix/repli moteur. Les workers exécutent et contrôlent les commandes ; CPU n'initialise pas de GPU, GPU manuel ne reçoit aucun repli supplémentaire.

| ID / propriétaire | Invariant et transition à préserver | Sources principales actuelles |
|---|---|---|
| DOM-01 / lancement Turbo | Thread lecteur stdin → canal synchrone capacité 8 → boucle propriétaire cache/stdout ; même ordre, backpressure et flush. | CPU main:466–563 ; GPU main:1072–1174 |
| DOM-02 / cache Turbo | CPU : chemin+empreinte modèle. GPU : modèle/hash/cible/périphérique effectif ; attestation concordante et fraîche aussi lors d'un hit. | CPU main:565–585 ; GPU main:1176–1245 |
| DOM-03 / session Turbo | Corrélations job/génération/instance et contrôles d'arrêt inchangés. Inactif + Shutdown valide ou fermeture canal : succès sans ajout de Stopped. | CPU main:537–563 ; GPU main:1142–1174 |
| DOM-04 / import Turbo | Arrêt reconnu aux points existants ; Stopped corrélé, sans nouvelle interruption de l'appel natif. `take_stop` vide/déconnecté reste false ; mauvaise version émet échec puis true. | CPU main:789–810,1244–1282 ; GPU main:1710–1785,2224–2262 |
| DOM-05 / live Turbo | Stop applicable : finaliser MP3, paquets restants, puis Stopped. Shutdown live : retour selon branche existante, sans unifier avec ce flush. | CPU main:1168–1229 ; GPU main:2148–2209 |
| DOM-06 / boucle Parakeet | Moteur/identité/last_request_id/job_attempts possédés synchroniquement. last_request_id consommé avant certains refus ; job_attempts réservé avant transcription. | Parakeet main:30–306, notamment143–252 |
| DOM-07 / erreurs Parakeet | JSON/version invalides : Failed puis continuer. Framing invalide : fatal. Shutdown : Stopped puis sortie ; EOF propre : sortie sans Stopped ; fatal stderr et code2. | Parakeet main:23–28,103–131,1016–1053 |
| DOM-08 / lancement et transcription Parakeet | Mêmes canonicalisations, profils, racines runtime/modèle, empreintes, identité source/build/GPU ; même ordre avant Ready. InvalidTimestamp reste échec distinct ; NativeCallStarted CUDA reste au même point. | Parakeet main:30–108,350–619,696–954 |
| DOM-09 / transport et instrumentation | Trames1Mio séparateur compris, refus excessif/partiel avant JSON ; mêmes conditions de traces/hooks et frontières FFI. | CPU ipc:6–50,main:431–464 ; GPU main:514–646 ; Parakeet main:1016–1053 |

Les caches/compteurs sont opérationnels et non durables. L'état visible reste porté par les événements existants vers le parent. Persistance/migration/reprise nouvelle : non applicable ; préserver les écritures existantes, sources immuables et contrôles de destination. Les effets existants ne deviennent ni une nouvelle transaction ni une garantie supplémentaire de reprise après panne.

## 5. Architecture proposée

Choix techniques de l'architecte dans le mandat de conception, soumis à revue : **DEC-T01** modules privés dans les binaires existants, sans lib.rs ; **DEC-T02** séparer lancement/boucle/session/contrôles et parcours ; **DEC-T03** réutiliser les modules natifs/FFI sans refaire leurs frontières ; **DEC-T04** conserver les accès racine contractuels de qualification/timing ; **DEC-T05** sortir les tests appartenant aux main entiers, sans campagne de déplacement des tests externes.

Alternatives écartées : simple déplacement vers un gros worker.rs (CA-02), crate commune/librairie pour les tests ou nouveaux ports (hors besoin), uniformisation des moteurs/writers (risque CA-03/04). Aucun choix n'active une nouvelle règle produit.

### 5.1 Turbo CPU/GPU

Dans ce tableau, chemins relatifs à `crates/whisper-worker-cpu/src/` et `crates/whisper-worker-gpu/src/`, sauf mention. Cibles proposées, pas fichiers déjà créés.

| Cible | Responsabilité extraite / origine CPU ; GPU | Consommateurs |
|---|---|---|
| startup.rs | Ancien main, buffers/canal/thread/trace et fatal ; diagnostic système CPU préservé. main:466 ;1072 | main |
| worker.rs | Boucle run et possession/réutilisation cache. :537 ;1142 | startup |
| session/mod.rs | Admission, version et dispatch première commande. run_session:587 ;1247 | worker |
| session/import.rs | Branche import : validation, plages, progression, segments et fin, même ordre | admission |
| session/live.rs | run_live_service, fenêtres, paquets MP3 et arrêt. :992 ;1971 | admission |
| session/decode.rs | handle_decode/run_decode_service. :1284/1374 ;2264/2354 | admission |
| session/control.rs | take_stop et corrélations. :1244 ;2224 | import |
| engine_cache.rs | cached_cpu_engine ; CachedGpuEngine/same_gpu_target/cached_gpu_engine | worker possède ; session emprunte |
| integrity.rs | verify_sha256. :1431 ;2411 | parcours qui vérifient déjà |
| diagnostics.rs | emit_failure ; erreurs initialisation/revalidation GPU. :1448 ;2428–2482 | parcours et fatal |
| qualification.rs | Variantes historiques l04_campaign_*, mêmes cfg. :8–312 ;649–953 | appelants actuels |
| memory_qualification.rs | Module inline racine, même feature l01-memory-qualification. :313–408 ;954–1049 | appelants actuels |
| CPU : ipc_output.rs | Writer borné du main:431–464, séparé du writer existant ipc::write_frame | appelants du writer borné |
| GPU : decoder.rs / encoder.rs / ipc.rs | Externaliser modules inline :1–453 /454–513 /514–646 | mêmes parcours |
| GPU : gpu_commands.rs | ProbeCuda, AttestGpuCandidate, QualifyGpus au début de run_session | admission |

Réutilisés CPU : decoder.rs, encoder.rs, ipc.rs, native_engine.rs. Réutilisés GPU : native_engine.rs, native_ffi.rs, gpu_attestation.rs, gpu_qualification.rs. Le writer CPU déjà dans ipc.rs est conservé avec son comportement distinct ; aucune fusion ou suppression. L'alias Incoming peut appartenir à ipc pour éviter une dépendance de la session vers la boucle.

Matrice de dépendances internes proposée : main→startup→worker→session ; worker/session→engine_cache→native_engine ; session→decoder/encoder/integrity/diagnostics/qualification ; startup/worker/session→ipc et CPU ipc_output ; session GPU→gpu_commands→gpu_attestation/gpu_qualification/native_ffi. Les modules techniques ne remontent pas vers startup/worker/session. Les appels existants native_engine→memory_qualification racine sont conservés ; ce helper ne devient pas un propriétaire de session.

### 5.2 Parakeet

Chemins relatifs à `crates/whisper-worker-parakeet-cpu/src/`.

| Cible | Origine actuelle dans main.rs / responsabilité | Consommateurs |
|---|---|---|
| startup.rs | run:30, branche decode-only, args/contrôles/charge/identité avant Ready | main |
| session.rs | Boucle, compteurs, corrélations, refus/arrêt de run | startup |
| args.rs | parse_args:621 et Args:689, syntaxe inchangée | startup |
| runtime_paths.rs | shared_model_runtime_allowed:460 ; required_runtime_root:696 et helpers jusqu'à ensure_inside:949 | startup/decode-only |
| gpu_identity.rs | worker_gpu_identity:432, configured_backend:449, build_gpu_evidence:484, validate_expected_gpu:537, requête/parser NVIDIA:580/597 | startup/session |
| integrity.rs | runtime_sha256:1075, sha256_file:1101, constant_time_eq:1163 | appelants actuels |
| transcription.rs | transcribe_one:350 ; contrôles source, erreurs et callback | session |
| decode_only.rs | decode_only_mode:308 ; create_new/sync_all et résumé | startup |
| reference_decoder.rs | decode_reference_wav:957 sous parakeet-qualification-reference | transcription de référence |
| ipc.rs | read_frame:1016, write_event:1041, failed:1055 | startup/session |
| timing.rs | record_r7_timing:1119 ; import racine de déclaration pour accès crate::record_r7_timing existant | appelants existants dont native_engine |

Conserver decoder.rs, native_engine.rs, native_ffi.rs. Dépendances : startup→args/runtime_paths/integrity/gpu_identity/native_engine/session ; session→ipc/transcription/gpu_identity/integrity ; transcription→decoder/reference_decoder/native_engine/timing. Les helpers ne dépendent pas du lancement ou de la session. Aucun relais fonctionnel ajouté seulement pour remplacer l'accès racine timing.

**Mode distinct à préserver :** --decode-only produit déjà un résumé JSON stdout (main:338–347) et son PCM via création exclusive puis sync_all. « stdout réservé au protocole » concerne le mode worker IPC et ne supprime pas ce résumé du mode distinct.

### 5.3 Visibilités, frontières et structure

Modules racine privés ; entrées/types transversaux limités à pub(crate), helpers privés ou pub(super), champs privés sauf besoin concret. Pas d'API publique entre crates ni trait/champ de production réservé aux tests. Les trois main résiduels portent déclarations et délégation ; traduction terminale actuelle permise (Parakeet diagnostic fatal/code2, Turbo événement Failed/code2 déléguable au lancement).

Matrice workspace inchangée : desktop→adapters+core ; adapters→core ; workers→core ; bootstrap→adapters+core. Ni core→worker/adapters/desktop ni worker→desktop/adapters ; politiques métier restent au parent/core, I/O et FFI dans leurs responsabilités existantes. Contrôle futur : imports/visibilités/manifestes et revue des consommateurs, pas un nouveau contrôle CI.

Cibles AGENTS pour le code nouveau : 1500 lignes production/fichier, fonction100, complexité25, arguments7, type250. Ne pas accroître les concentrations ; répartir les grosses fonctions aux frontières de responsabilité sans réordonner les branches. Tout dépassement résiduel doit être justifié dans le bilan prévu et ne donne pas mandat de refonte supplémentaire.

## 6. Tests extraits

| Worker | Tests appartenant à main.rs à placer dans fichiers séparés |
|---|---|
| CPU | ipc_output/tests.rs : writer borné ; worker/tests.rs : arrêt inactif |
| GPU | decoder/tests.rs : tests decoder inline ; ipc/tests.rs : framing ; worker/tests.rs : arrêt ; diagnostics/tests.rs : erreurs GPU corrélées |
| Parakeet | runtime_paths/tests.rs ; runtime_paths/personal_tests.rs sous cfg initial ; gpu_identity/tests.rs sous CUDA ; ipc/tests.rs ; reference_decoder/tests.rs sous feature initiale |

Les tests des modules GPU inline font partie des main entiers. Déclarations cfg(test) et conditions de features préservées. Les tests déjà dans les modules externes ne sont pas déplacés systématiquement ; seules liaisons directement nécessaires peuvent être adaptées. Supports partagés selon visibilité ; tests/ n'est pas supposé accessible automatiquement aux tests unitaires.

## 7. Vérifications futures et preuves

Toutes les commandes et observations de cette section sont **proposées, NOT RUN**. Cwd : C:\dev\whisper ; PowerShell. Après plan/autorisation/préflight, commandes par package `cargo test/build/clippy --offline --locked -p <worker>` avec features ci-dessous et commandes de format prévues par le plan. Pas de --fix, téléchargement ou modification Cargo.lock. Un succès sans scénario atteint n'est pas une preuve native.

| ID / scénario et données | Vérification attendue | Couverture |
|---|---|---|
| V-01 cinq façades et tous modules extraits | Revue fichiers complets, imports, chemins, diff ; aucune responsabilité/test restant dans main ; chaque diff relié | AC-01/02/05 |
| V-02 framing valide, limite1Mio, limite+1, séparateur, EOF partiel | Tests contrôlés reader/writer et erreurs avant JSON ; writers CPU distincts conservés | AC-03/04 |
| V-03 Turbo inactif, import/live, arrêt/version/corrélation | Référence/candidat : ordre/events/diagnostics/codes et MP3 final distinct Stop/Shutdown ; réutilisation cache | AC-03/04 |
| V-04 GPU sonde, refus d'identité/attestation et cache | Tests corrélations/refus, attestation sur hit ; séparer sonde et inférence native | AC-03/04 |
| V-05 Parakeet requête refusée puis suivante, répétition/génération/tentative | Consommation identique compteurs, mêmes refus/poursuite, Ready/Result/Failed/Stopped ; JSON/version/framing/EOF distingués | AC-03/04 |
| V-06 racines runtime/modèle et profils Parakeet | Tests canonicalisation/refus/personnel/référence, decode-only sortie et création exclusive, empreintes | AC-03/04 |
| V-07 transcription native mêmes modèles/audio/config CPU et CUDA | Résultats et erreurs dont InvalidTimestamp/callback ; observer moteur effectif, ne pas déduire natif de compilation | AC-03/04/06 |
| V-08 bilan et éventuelle recette Windows | Configuration, entrée, scénario atteint, résultat PASS/FAIL/NOT RUN et limites ; recette réservée utilisateur si prévue | AC-05/06 |

Références de tests existants : CPU tests/live_worker.rs et tests/reusable_imports.rs ; GPU tests/gpu_selection_native.rs ; Parakeet tests/native_import.rs et tests/decoder_parity.rs. Plusieurs tests natifs/parité Parakeet sont ignorés et requièrent des ressources explicites. Un test de sonde GPU peut réussir avec refus CUDA : cela ne prouve pas l'inférence GPU.

| Configuration affectée | Vérifications futures distinctes |
|---|---|
| Turbo CPU puis GPU | Défaut, l01-memory-qualification, recette-hooks, combinaison des deux : compilation et tests ciblés, branches no-op préservées |
| Turbo release | Sans hooks : construction ; avec recette-hooks : refus attendu du script existant, pas livraison |
| Parakeet | Défaut, parakeet-cuda, parakeet-qualification-reference, parakeet-personal ; personnel+CUDA, CUDA+référence et autres combinaisons touchées actuellement supportées à inventorier au plan ; aucun all-features global remplaçant cette matrice |
| Natif Windows | Turbo CUDA et Parakeet CUDA séparément ; CPU et ressources/profils effectifs distincts |

Préconditions déclarées dans le dépôt, installation non vérifiée : Rust1.98.1/édition2024, whisper-rs0.16.0, Symphonia0.5.5, rusty_mp3 0.8.0, Parakeet libloading0.8.9 et runtime natif. Aucun changement de stack/version/redistribution. Ressources de référence/candidat et coûts/durées de campagne restent à fixer au plan selon scénarios disponibles, avant exécution ; aucune nouvelle cible chiffrée n'est acceptée ici.

Avant **tout Cargo**, y compris metadata : design/plan acceptés, lot/chemins et accord précis ; espace ≥30Gio ; garde existante applicable avec auto-tests et surveillance ; CARGO_HOME/TEMP/TMP canoniques dans racine du lot, CARGO_TARGET_DIR sur target/parakeet-tests/cargo-shared/ ; CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only. Anciennes dérogations non réutilisables. Racine de campagne et applicabilité de la garde sont des conditions d'exécution non satisfaites, sans bloquer la conception. Ni garde nouvelle ni contournement proposés ; si l'outil existant ne convient pas, arrêt avant Cargo et arbitrage.

Si une remise Windows est prévue plus tard, respecter le profil réellement construit et les ressources effectivement chargées selon AGENTS ; pas de paquet dit autonome sans preuve. La validation dans la fenêtre appartient à l'utilisateur.

## 8. Risques, décisions et readiness

| ID | Risque concret / fermeture future | Liens |
|---|---|---|
| RISK-01 | Imports/visibilités/cfg cassés ou tests devenus inactifs ; contrôle chemins et matrice par configuration | AC-01/02/04/06 ; V-01/02 |
| RISK-02 | Ordre événements/validations/compteurs/flush changé ; scénarios de refus suivi et arrêt différencié | AC-03/04 ; V-03/05 |
| RISK-03 | Cache, attestation ou racine élargis ; réutilisation/refus et témoins négatifs | AC-03/04 ; V-04/06 |
| RISK-04 | Fusion writers CPU ou uniformisation moteurs ; table correspondance et revue ciblée | AC-02/03/04 ; V-01/02 |
| RISK-05 | Comparaison masquante ou fausse preuve native ; valeurs variables explicites et scénario réellement atteint | AC-03/06 ; V-07/08 |

Faisabilité de conception : preuve documentaire de modules externes déjà utilisés dans ces mêmes binaires, aucune technologie/dépendance nouvelle ; aucun SPIKE structurant nécessaire identifié. Cette preuve ne prouve ni compilation du futur découpage ni non-régression. PRODUCT_VALIDATION et DELIVERY_QUALIFICATION restent NOT RUN. Revue DESIGN indépendante à obtenir sur la version Git réelle.

Rubriques couvertes : scope§2 ; use_cases/errors§3–4 ; domain§4 ; dependencies§5 ; persistence§4 (aucune migration nouvelle) ; concurrency§4–5 (ownership/canal/arrêt conservés) ; platform§7 ; acceptance§3/7 ; risks§8. Railguard existant seulement analysé : invariants applicables préservés ; aucun nouveau railguard, manifeste ou mécanisme G1. Les règles AGENTS actuelles gouvernent les autorisations.

Aucune question produit bloquante identifiée. Q-EXEC-01 : responsable futur coordinateur/architecte, échéance avant première campagne, fixer racine autorisée et applicabilité de la garde existante ; refus = aucune exécution. Q-DETAIL-01 : architecte/plan writer, échéance avant plan exécutable, fixer données, commandes détaillées et combinaisons réellement supportées ; ne pas réduire silencieusement CA-03/04. Ce sont des conditions futures, pas des accords supposés.

## 9. Esquisse de réalisation future — huit tâches

Cette esquisse répond à l'activité6 du besoin ; **ce n'est pas un plan accepté ou prêt à exécuter**. Après validation du design, un plan détaillé autorisé précisera lots, prérequis DOCUMENT/CODE/EXECUTION, chemins, ressources, commandes et critères de sortie. Aucun code n'est autorisé par le présent texte.

| Tâche | Chemins / dépendance | Sortie et couverture |
|---|---|---|
| T1 | Baseline et scénarios de§7 ; autorisations/racines/garde avant modification | Conditions réelles et référence disponible ; CA-03/04/05 |
| T2 | CPU src/ipc_output.rs,qualification.rs,memory_qualification.rs et tests directs ; T1 | Transport/instrumentation distincts, cfg et bornes ; CA-02/03/04 |
| T3 | CPU startup/worker/engine_cache/session/integrity/diagnostics/main et tests§6 ; T2 | Façade CPU, sessions/réutilisation/arrêts conservés ; CA-01 à05 |
| T4 | GPU decoder/encoder/ipc/qualification/memory_qualification et tests§6 ; T3 | Extractions inline sans report mécanique CPU ; CA-01/02/04 |
| T5 | GPU startup/worker/engine_cache/session/gpu_commands/integrity/diagnostics/main et tests ; T4 | Façade GPU, corrélations/cache/attestation ; CA-01 à05 |
| T6 | Parakeet args/runtime_paths/gpu_identity/integrity/ipc/timing/decode_only/reference_decoder et tests ; T5 | Contrôles et modes distincts conservés ; CA-02/03/04 |
| T7 | Parakeet startup/session/transcription/main et tests ; T6 | Façade, compteurs/refus/arrêt/timestamps ; CA-01 à05 |
| T8 | Vérifications croisées§7, revue/corrections bornées et bilan ; T3/T5/T7 | Couverture réelle par configuration, réserves et recette si prévue ; CA-01 à05 |

Propriété future : code à rust_implementer après autorisation ; revue via rust_review_coordinator et reviewer indépendant, maximum deux cycles ; recette utilisateur si prévue. Pas de modifications concurrentes du même contrat sans coordination. Modules existants réutilisés ne sont modifiables que pour liaisons nécessaires explicitement incluses dans le futur lot. Aucun chemin hors des trois workers et de la documentation du lot sans arbitrage.

Arrêt futur sur divergence inexpliquée, nécessité de contrat/manifestes/hors-périmètre, garde refusée, écriture hors racine attribuée, échec de contrôle ou dépassement dix tâches/dix rapports. Préserver l'état et demander l'arbitrage ciblé, sans reset, nettoyage ou extension automatique. Ne pas pousser sans accord précis.

## 10. Bilan du cadrage et revue

Vérifications réalisées : lectures et recherches statiques, inventaire et état Git. Cargo, compilation, tests, modèles, applications et recette Windows : NOT RUN. Un seul document de cadrage créé ; aucun code produit, manifeste, lockfile, ressource ou dossier non suivi préexistant modifié.

Revue indépendante : en attente sur le candidat Git. La version, le verdict et les éventuels constats seront consignés ici après retour du reviewer. Les décisions de périmètre acquises ne sont pas rouvertes ; la validation utilisateur du design et l'autorisation de la suite restent distinctes.
