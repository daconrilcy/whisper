Avancée : cadrage du refactoring structurel établi sur les sources actuelles.
Blocage : aucune implémentation autorisée ; prérequis de campagne à résoudre avant Cargo.
Décision : cible indicative d’environ 600 lignes validée ; conception candidate pour revue indépendante.

# Refactoring du direct microphone — DESIGN-DL v1

## 1. Mandat, sources et contenu

Le 31 octobre 2026, l’utilisateur a donné cet accord dans la session :

> Je valide le besoin et la cible indicative d’environ 600 lignes. Passe au cadrage et prépare le plan de refactoring dans docs/cadrage/, avec revue indépendante et commit local, sans implémentation ni push.

Ce mandat autorise les présents documents, leur revue et leur versionnement local. Il ne vaut ni acceptation du plan à venir, ni autorisation de coder, de lancer une campagne ou de pousser. Aucun nouveau railguard, lanceur, service ou mécanisme de preuve n’est demandé.

Objectif : faciliter la maintenance du direct microphone dans Whisper Windows, en réduisant `crates/whisper-adapters/src/direct_live.rs` par des extractions cohérentes à comportement constant. Le mainteneur bénéficie du découpage ; l’utilisateur conserve capture, transcription, arrêt et sorties actuelles.

Base source : `17cd4927ea431ab7fb5b8c0fde7cda8c7ccd07e0`, branche `evo-startup-perf`. État initial : aucun changement suivi ; dossiers non suivis `docs/architecture/`, `models/`, `sound example/` à préserver. Espace libre C: relevé au cadrage : environ 165 Gio ; à relever de nouveau avant tout lot d’exécution.

Sources locales consultées : AGENTS racine et adapters, carte `docs/architecture/ARCHITECTURE.md` et fiche adapters, manifestes racine/adapters, `direct_live.rs`, références des consommateurs. Les cartes statiques non suivies ne constituent ni base d’acceptation ni preuve du code actuel.

Contributions en lecture seule, fusionnées par la session principale, auteur du document :

| Owner | Entrée/version | Contribution et état |
| --- | --- | --- |
| `rust_product_framer` | DL-REF-17cd4927-v1 | Brief, capacités et parcours ; proposition achevée |
| `rust_requirements_analyst` | REQ-DL-v1 | REQ/UC/AC et risques fonctionnels ; proposition achevée |
| `rust_domain_architect` | DOM-DL-v1 | Concepts, transitions et invariants ; proposition achevée |
| `rust_architect` | ARCH-DL-v1 | Découpage, visibilité et contrats ; contribution achevée |

Content map : §2 brief/exigences/acceptation ; §3 domaine/états ; §4 architecture/contrats ; §5 risques/preuves/décisions ; §6 readiness. Le plan associé sera `plan-refacto-direct-live.md` et le compte rendu indépendant `revue-refacto-direct-live.md`. Aucun document dans `docs/predev/` n’est modifié.

## 2. Périmètre et exigences

Le fichier compte **3 184 lignes physiques** (`Get-Content ... | Measure-Object -Line` ne doit pas être employé pour exclure les lignes vides : utiliser `(Get-Content -LiteralPath <chemin>).Count`). Les modules `tests` et `d2_corpus` commencent aux lignes 1445 et 2550 ; des méthodes `cfg(test)` existent également aux lignes 70–194. Il ne s’agit donc pas de 1 444 lignes de production.

Inclus : orchestration du direct, capture/gain/télémétrie, dialogue moteur, WAV/backlog et exports partagés Turbo/Parakeet, séparation des tests. Aucun objectif nouveau de rapidité, de qualité ou de latence. Hors changement : UI, réglages, installation, capture WASAPI elle-même, moteur natif, protocole, nouveaux modèles, dépendances, features, manifestes et `Cargo.lock`. Les reprises après panne et nouvelles fonctionnalités restent hors lot, conditionnées à une nouvelle demande.

| Besoin/capacité | Exigence | UC et acceptation observable | Responsabilité cible |
| --- | --- | --- | --- |
| CA-01 / CAP-01 maintenance | REQ-01 : cible indicative ~600 lignes physiques, sans compression artificielle | UC-01 / AC-01 : même comptage avant/après pour principal et extraits ; expliquer l’écart éventuel à 600, sans inventer un plafond | fichier principal et revue |
| CA-02 / CAP-01 | REQ-02 : responsabilités identifiables et contrats préservés | UC-02 / AC-02 : table ancien→nouveau, imports sans cycle, aucune API publique créée pour les tests | §4 et tous extraits |
| CA-03 / CAP-02 capture et CAP-03 gain/télémétrie | REQ-03 : préserver conversion, ordre, gain, bornes, arrêt et gardes d’identité effectives | UC-03 / AC-03 : mêmes PCM/WAV, gain au bloc attendu, backlog borné et vidage avant finalisation ; AC-04 : mêmes confirmations, erreurs d’ordre/identité et progression niveau/spectre/texte | principal, capture, worker, wav |
| CA-04 / CAP-04 moteur et CAP-05 erreur/arrêt | REQ-04 : conserver CPU/GPU/Auto, attestations et compatibilité historique | UC-04 / AC-05 : mêmes backend/motifs/erreurs/résultats incomplets lors du repli avant Ready, arrêt précoce, échec moteur ou interruption micro ; pas de nouveau repli | principal et worker |
| CA-05 / CAP-06 sorties | REQ-05 : préserver les contrats Turbo/Parakeet | UC-05 / AC-06 : mêmes noms, TXT/SRT, horodatages, JSON et indicateur complete ; mêmes compensations et diagnostics sous échec SRT/JSON | outputs et wav |
| CA-06 / CAP-07 tests | REQ-06 : conserver scénarios, assertions et conditions d’activation | UC-06 / AC-07 : inventaire avant/après, noms des tests conservés, aucun scénario retiré ou désactivé ; séparer PASS/FAIL/ignoré/NOT RUN | tests, d2_corpus, test_support |

Les AC comparent les mêmes fixtures et stimuli avant/après, pas seulement un code retour. Le détail des commandes et des scénarios est porté par le plan. Une lacune de couverture avérée appelle un témoin ciblé dans les chemins de tests prévus ; aucun refactoring fonctionnel supplémentaire.

## 3. Domaine, états et garanties actuelles

DOM-01 : `DirectLive` possède la requête, l’identité JobId/Generation/instance worker, la préférence Auto et le moteur effectif, les ressources, les compteurs et événements. DOM-02 : `WavOutput` possède fichier/chemin/compteur ; le WAV est le backlog, la télémétrie est distincte de l’audio enregistré. DOM-03 : une fenêtre inflight, ses segments provisoires, puis les segments confirmés. DOM-04 : sorties WAV, TXT/SRT et JSON ont des contrats distincts.

| Déclencheur / précondition | Transition et effet à préserver | Source `direct_live.rs` de base |
| --- | --- | --- |
| start valide | Acquisition worker/capture/convertisseur/probe/WAV ; capture peut avancer pendant l’attente moteur ; erreurs libèrent les ressources acquises selon le code actuel | 196–349 |
| Premier PCM écrit puis poll | DirectCaptureActive suivant l’ordre actuel ; l’ouverture seule ne vaut pas admission PCM | 403–456, 493–558 |
| Attestation puis Ready concordants | Candidat GPU/backend contrôlés ; Ready expose moteur effectif et cause éventuelle de repli | 655–754 |
| Prêt, pas de fenêtre active ni Stop envoyé | Lecture du WAV vers pending borné puis envoi d’une seule fenêtre | 603–653 |
| Segment admissible | Contrôle séquence, plage, fréquence et ordre ; ajout provisoire, sans DirectConfirmed | 756–793 |
| WindowFinished cohérent | Libère inflight, transfère les segments et émet DirectConfirmed | 794–817 |
| stop | Idempotent ; invalide probe, arrête capture, draine avec les bornes existantes et ajoute la queue du convertisseur ; Stop worker après backlog traité | 384–401, 603–645 |
| Stopped concordant après stop_sent | Termine WAV puis exporte TXT/SRT, marque done et émet DirectPublished ; mêmes erreurs intermédiaires | 818–833 |
| fail / interruption | Avorte worker, tente de sauver l’audio restant et les sorties incomplètes au mieux ; messages et erreurs conservés | 907–998 |

États visibles : capture active pendant attente moteur, moteur prêt, blocs confirmés/télémétrie, arrêt en cours, publication des chemins, erreur et résultats incomplets. Les booléens `worker_ready`, `stop_requested`, `stop_sent`, `done` restent une représentation opérationnelle ; aucune nouvelle machine d’états produit n’est imposée. La fermeture conserve le routage du parent.

Invariants de non-régression :

- INV-01 (REQ-03) : conversion/gain par bloc, révisions acquittées, ordre et quotas de drainage inchangés ; saturation/panne interrompent, xrun reste avertissement (355–366, 493–558).
- INV-02 (REQ-03) : pending ≤128 000 échantillons ; fenêtres 80 000–128 000 avec recherche de pause, queue courte à l’arrêt ; une seule inflight (23–29, 1274–1308).
- INV-03 (REQ-04) : repli Auto avant Ready selon les branches actuelles, incrément d’instance et effacement des preuves GPU ; aucune bascule nouvelle pendant traitement ou en mode manuel (196–269, 841–893).
- INV-04 (REQ-03/04) : les gardes d’identité réellement présentes restent identiques. Les événements inattendus/périmés aboutissent actuellement à Err ; plusieurs bras Failed n’ont pas de filtrage universel. Ne transformer ni le rejet en ignorance silencieuse, ni ces bras en nouvelle politique de filtrage (655–860).
- INV-05 (REQ-05) : create_new, limite PCM/WAV, en-têtes et nettoyages compensatoires conservés. JSON séparé et flush n’offrent pas une transaction multi-fichiers ni une garantie contre coupure (1000–1224, 1310–1424).

Les blocs pas encore écrits et les erreurs de sauvegarde ne reçoivent aucune nouvelle garantie. DirectConfirmed n’est pas un ACK de persistance. Pas de migration de format ni de reprise après panne.

## 4. Architecture proposée et frontières

DEC-T01 : extraire des modules privés sous `src/direct_live/`, en conservant l’état de session dans le parent et plusieurs impl DirectLive. Choix technique délégué par la mission de cadrage, à revoir indépendamment. Alternative rejetée : sortir seulement les tests laisserait environ 1 300 lignes de production ; créer un framework/trait générique ne répondrait à aucun besoin présent.

| Chemin relatif à `crates/whisper-adapters/src/` | Responsabilité / symboles de base | Ordre de grandeur, pas un plafond accepté |
| --- | --- | --- |
| direct_live.rs | État, constantes, start/stop/poll/send_next/fail/export, wrapper de remplissage, petits accès crate, Drop, window_size ; déclarations et réexports | ~500–600, cible ~600 |
| direct_live/capture.rs | Télémétrie, pump_capture, drain_capture, diagnostics et microphone_error | ~180 |
| direct_live/worker.rs | handle_worker, retry_cpu, backend/identity/attestation et spawn_direct_worker | ~340 |
| direct_live/outputs.rs | finalize_incomplete_outputs, export_segments, automatic_final_output_paths, export_parakeet_manifest, ParakeetExportOutcome, export_parakeet_outputs, timestamp | ~300 |
| direct_live/wav.rs | WavOutput, header, Drop et helper libre fill_pending_from_wav | ~150–170 |
| direct_live/tests.rs | Module tests déplacé, mêmes noms/scénarios/assertions | ~1100 |
| direct_live/d2_corpus.rs | Module D2 et tests imbriqués déplacés, même cfg et noms | ~635 |
| direct_live/test_support.rs | Cinq méthodes de construction/manipulation réservées aux tests actuellement 70–194 | ~125 |

Les estimations ne sont pas des décomptes après extraction. `window_size` reste dans le parent car il décide l’admission ; le helper libre `fill_pending_from_wav` va dans wav.rs et peut être importé au parent pour conserver le nom utilisé par les tests. Aucun déplacement hors de cette famille n’est nécessaire. Ne pas compacter pour atteindre 600.

DEC-T02 : préserver les chemins crate existants via réexports `pub(crate)` explicites dans le parent, notamment WavOutput et les exports Parakeet. État DirectLive privé parent, méthodes transverses d’impl en `pub(super)` au plus. Modules de tests/support sous cfg(test), jamais inclus dans le produit. Pas d’API publique ou de trait créé pour les fixtures. Le getter existant `sample_count` permet d’éviter d’exposer les champs WAV.

DEC-T03 : conserver les modules nommés `tests` et `d2_corpus` et les noms de fonctions, donc les filtres complets existants. Déplacer les cinq méthodes initiales de test dans test_support.rs entièrement cfg(test). Le relevé lexical actuel contient 25 attributs test : 18 dans tests, un scénario natif D2 et six tests purs imbriqués dans D2 ; à comparer au vrai inventaire de compilation lors du lot, ce relevé n’étant pas un test exécuté.

Consommateurs à préserver sans édition : `worker_ipc/runtime/mod.rs:79`, `worker_ipc/runtime/live.rs:1149`, `worker_ipc/tests/microphone_and_finalization.rs:482`, `parakeet_live.rs:14,930,1557,1638,3109`, `microphone_probe.rs:716`. Les numéros décrivent la base, à revalider sur le candidat.

| Consommateur | Dépendance autorisée | Contrôle futur |
| --- | --- | --- |
| worker_ipc, parakeet_live, microphone_probe | API crate direct_live inchangée | Imports, compilation des consommateurs et tests concernés |
| Parent DirectLive | capture/worker/outputs/wav privés | Visibilités minimales et revue des appels |
| capture/worker | État parent, adaptateurs existants et types core | Pas d’état session parallèle, pas de cycle de responsabilité |
| outputs/wav | Types core et IO existantes | Aucune dépendance retour vers worker_ipc ni appel UI |
| Tests/support cfg(test) | API privée par hiérarchie du module | Pas de chemin ou branche de production ajouté pour tester |

Les couches restent celles du dépôt : adapters exécute les IO, core possède les politiques, desktop compose/affiche, workers exécutent IPC. Aucun changement de manifeste ou de dépendance. Cette matrice locale ne prétend pas réarchitecturer tout le workspace.

Contrats : capture reçoit les blocs CPAL, écrit le PCM et expose la télémétrie ; worker reçoit WorkerEvent, modifie la même session et produit Option<ImportEvent>/String avec gardes identiques ; outputs reçoit requête/segments/plages et conserve chemins/diagnostics ; WAV conserve create_new, append, read_samples, finish et Drop. Le parent reste propriétaire des ressources et de leur ordre d’arrêt. Pas de thread, canal, verrou, runtime async ou FFI ajouté. Le coût bloquant des IO reste inchangé ; la backpressure actuelle et les transports existants sont conservés.

## 5. Faisabilité, risques et décisions

Plateforme : application Windows existante, CPU et CUDA selon installation ; Rust minimal déclaré 1.98.1, édition 2024. Dépendances/features actuelles conservées ; `unsafe_code=deny` pour adapters, pas de nouveau unsafe. Aucune recherche de bibliothèque ni faisabilité moteur nouvelle : la faisabilité du déplacement repose sur les frontières source et la visibilité Rust ; sa compilation reste à démontrer lors de l’implémentation.

| Preuve / risque | Classe, état et owner | Réponse / échéance |
| --- | --- | --- |
| Découpage sans nouveau composant externe | DESIGN_FEASIBILITY : examen statique, contribution architecte ; revue à venir | Vérifier cohérence des accès et budget dans DESIGN puis PLANS |
| RISK-F01 ordre drain/arrêt/confirmation modifié | PRODUCT_VALIDATION : NOT RUN, implémenteur | AC-03/04, témoins audio/stop/backlog et revue du diff |
| RISK-F02 attestation/identité/repli modifiés | PRODUCT_VALIDATION : NOT RUN, implémenteur | AC-05 et branches non couvertes caractérisées sans changer la politique |
| RISK-F03 rupture sorties Parakeet ou compensations | PRODUCT_VALIDATION : NOT RUN, implémenteur | AC-06, tests export et compilation consommateurs |
| RISK-T01 visibilité/résolution module/tests | PRODUCT_VALIDATION : NOT RUN, implémenteur | Même API, noms et cfg ; compilation tests et inventaire |
| RISK-T02 campagne non conforme | EXECUTION : prérequis ouvert, session principale | Résoudre avant Cargo, sans contournement ni nouvel outillage implicite |
| Recette native et fenêtre Windows | DELIVERY_QUALIFICATION : NOT RUN, utilisateur pour fenêtre | Hors campagne actuelle ; ne pas prétendre validation produit |

RISK-T02 est concret : `target/parakeet-tests/refacto-archive/guard.ps1` a une racine archive codée en dur et une tolérance d’événements Temp `UNKNOWN` ; la garde `refacto-application-core/t10/guard.ps1` est également propre à son lot. Ces gardes ne sont pas annoncées compatibles avec un nouveau lot. Aucune ancienne dérogation n’est étendue. Avant une campagne, identifier une garde existante conforme et applicable, ou demander l’accord précis nécessaire à une adaptation bornée et à son périmètre. Aucune écriture ni exécution de garde réalisée au cadrage.

Décisions : DEC-P01 cible indicative ~600 acceptée par l’utilisateur ; DEC-P02 comportement constant et mandat documentaire seulement acceptés dans la session ; DEC-T01/02 choix techniques proposés sous mandat de cadrage. Aucune question produit structurante ouverte. Un écart substantiel à la cible ou un chemin supplémentaire relève de l’acceptation du plan ou d’un PLAN_CHANGE avant exécution ; jamais d’une autorisation implicite.

## 6. Readiness et limites

Rubriques applicables couvertes : scope §1–2 ; use_cases/acceptance §2 ; errors/domain §3 ; dependencies/concurrency/persistence/platform §3–5 ; risks §5. Migration N/A : formats et contrats inchangés. Nouveau packaging N/A : aucun paquet livré. Railguard nouveau N/A : aucun prévu par le besoin ; les règles AGENTS existantes demeurent applicables. Pas de SPIKE : aucun nouveau mécanisme technique à établir pour le découpage.

Statut candidat : DESIGN REVIEW PENDING. La revue indépendante doit juger ces contenus et les limites d’exécution ; elle seule déclare CLEAN. Les tests produit et recettes restent NOT RUN. Même DESIGN READY puis PLANS READY ne vaudront pas autorisation de coder. La session s’arrête après plan, revue et commits locaux prévus au mandat.
