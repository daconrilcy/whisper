Avancée : T1–T7 validées, revues indépendantes CLEAN ; deux extractions terminées.
Blocage : aucun pour T8 ; échec optionnel recette-hooks préexistant consigné.
Décision : engager l’extraction identité/sources T8, sans push.

# Plan de refactorisation d’archive.rs

## 1. Identité, autorité et portée

- ID : PLAN-ARCHIVE v1, 30 octobre 2026, PLANS READY documentaire ; aucune exécution autorisée.
- Auteur : session Codex principale ; contribution rust_plan_writer `/root/plan_archive`, synthèse de son candidat en lecture seule.
- Design parent : [refacto-archive.md](refacto-archive.md), DESIGN-ARCHIVE v1, Git `0bec1a3ad79cd3334a14ad73262b38c7a8e3b54e` ; CLEAN confirmé indépendamment par `/root/revue_design_archive`, zéro REQUIRED. Revue initiale sur `6263391c`, preuve dans `be549cb2` ; VIS-01 confirmé et advisory de décompte fermé sur `0bec1a3a`.
- Base produit : `2829ce4569a90f494e6ff5f73a8887ee452074cf`, branche evo-startup-perf.
- Autorité : « tu peux faire le cadrage puis rediger le plan de refacto avec les agents dediés ». Documents et commits locaux seulement, aucun code/campagne/test/push.
- Sources inspectées par le plan writer : design Git, AGENTS racine/adapters, archive.rs, tests archive/live/Parakeet/reprocessing, registre modèles, manifeste adapters, garde F3.
- Toutes commandes/validations futures : **NOT RUN**.

Dix tâches globales, séquentielles. Ce fichier est l’unique bilan futur, sans rapport autonome ni manifeste de qualification. Résultat : publication, reprise, identité et retrait séparés selon design, comportement caractérisé avant extraction. Pas de correction implicite, migration, garantie crash/powerfail nouvelle, dépendance/feature/API publique/politique retrait/injection nouvelle.

## 2. Chemins et propriétaires

Alias relatifs au dépôt. Une concaténation désigne le chemin complet, sans extension implicite.

| Alias | Chemin |
| --- | --- |
| A | crates/whisper-adapters/src/archive.rs |
| M | crates/whisper-adapters/src/archive/ |
| UT | crates/whisper-adapters/src/archive/tests.rs |
| U | crates/whisper-adapters/src/archive/tests/ |
| I | crates/whisper-adapters/tests/ |
| P | docs/cadrage/plan-refacto-archive.md |
| R | target/parakeet-tests/refacto-archive/ |
| G | target/parakeet-tests/refacto-archive/guard.ps1 |
| C | target/parakeet-tests/cargo-shared/ |

R/C sont proposés, non autorisés par la mission documentaire. R reçoit garde adaptée/cache local/temporaires/fixtures/résultats/preuves. Les sources et P sont modifiés hors commande de campagne ; pendant la commande seules R/C sont destinations applicatives volontaires.

| Extraction propriétaire | Modules |
| --- | --- |
| T6 | M/campaign.rs, storage.rs, import.rs, live.rs, publication.rs, publication/catalog.rs, publication/exports.rs |
| T7 | M/journal.rs, recovery.rs, recovery/live.rs |
| T8 | M/source.rs, identity.rs |
| T9 | M/cleanup.rs, cleanup/proofs.rs, cleanup/execution.rs |

Quinze enfants. A est raccordé T6–T9 par un propriétaire unique. UT/tests sont adaptés uniquement selon chaque tâche. Types/records restent dans A, pas de champ public nouveau ; API/réexports/cfg conservés. VIS-01 : pub(super) au parent immédiat, pub(in crate::archive) seulement pour helpers imbriqués nécessaires, chemins intermédiaires accessibles ou réexports explicites. Rien d’exposé hors archive.

Lecture indirecte : lib.rs, journal.rs, recovery.rs, queue_store.rs, source_binding.rs, staging.rs, windows_file_identity.rs, model_catalog.rs, model_registry.rs, consommateurs worker_ipc/, manifestes et AGENTS. Aucun de ces chemins n’est ouvert en modification, sauf tests nommés ci-dessous.

## 3. Prérequis et ordre

- DOCUMENT D0 : design Git/revue, plan revu/accepté, décisions applicables, GO précis tâche/chemins/campagne.
- CODE Cn : sortie Tn attribuable et disponible sans conflit.
- EXECUTION E0 : T1 terminée, garde autorisée/conforme, auto-tests/préflight/ressources validés.
- EXECUTION En : contrôles Tn réellement exercés, observations attendues et revue indépendante sans constat requis ouvert.

Graphe : T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9 → T10. Aucune parallélisation.

**T2–T5 réussies sur le code produit antérieur aux extractions avant T6.** Chaque nouveau test passe d’abord contre ce code inchangé ; une anomalie est rapportée, pas corrigée dans le baseline.

Avant chaque tâche : Git, préalables « dernier chantier », responsabilités concurrentes et espace libre ; préserver docs/architecture/, models/, sound example/ non suivis. Sous30Gio : aucune compilation/campagne. Relever espace final. Exécution future rust_implementer IMPLEMENT ; revue rust_review_coordinator/reviewer indépendant, deux cycles au plus. Principale seule écrit bilan P. Aucun GO par ce DRAFT.

## 4. Garde et ressources futures

### T1 : adaptation bornée proposée

Source lue : target/parakeet-tests/refacto-worker-ipc-f3/guard.ps1. Racine F3 codée dans contrôles/snapshots/surveillance, variables et fixture MP3 historiques imposées, tolérances UNATTRIBUTED Temp/Downloads/.git/objects non transférables. Préflight chemins/variables/reparse/espace/arguments/surveillance existants à conserver.

Sous accord précis futur : adapter cette garde dans G, R remplaçant F3 partout ; C reste cible Cargo unique. Retirer fixtures/variables sans rapport ; ne reconduire aucune tolérance externe. Événement externe détecté ou erreur surveillance : arrêt/conservation chemin+heure. Préserver chemins canoniques/variables/reparse/arguments/sous-processus. Auto-tests positifs internes et refus cible externe, variables absentes/externes/reparse, événement externe Temp/Downloads/objets Git, renommage traversant frontière. Événements simulés existants : aucune écriture externe pour tester le refus. Jonction éventuelle créée uniquement dans R, sans écriture dans sa cible, retrait ciblé. Pas nouveau service/isolation/surveillant/manifeste/dérogation.

La surveillance détecte, elle ne confine pas préventivement. Toute limite incompatible avec AGENTS impose arrêt/arbitrage, sans tolérance implicite.

### Environnement

Windows/PowerShell, cwd C:/dev/whisper. CARGO_TARGET_DIR=C:/dev/whisper/target/parakeet-tests/cargo-shared ; CARGO_HOME=R/cargo-home, TEMP=R/temp, TMP=R/tmp ; CARGO_INCREMENTAL=0 ; CARGO_PROFILE_DEV_DEBUG=line-tables-only. Créer/vérifier canoniquement avant Cargo ; cache local copié en lecture depuis dépendances disponibles, aucun téléchargement/cache utilisateur écrit/manifeste-lock modifié/Cargo direct. Les enfants héritent de cet environnement.

Interface conservée G -Mode SelfTest|Run -CargoArguments. Commandes §5 : arguments du lanceur gardé seulement. Aucun lancement maintenant.

### Données

- Fixtures synthétiques R/t2/…t5/, temporaires résolvant dans R, sources/tiers/destinations internes, aucune archive utilisateur.
- Modèle Turbo C:/dev/whisper/models/ggml-large-v3-turbo.bin, modèle Parakeet C:/dev/whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf : lecture seule, présence/taille/SHA attendus du registre vérifiés avant suites. Absence/divergence bloque la suite.
- PARAKEET_L2_RUNTIME_ROOT=R/t4/parakeet-runtime ; PARAKEET_L2_ARCHIVE_ROOT=R/t4/parakeet-runtime/r6/archive. Créer r6 : fallback actuel archive_root local, sans racines historiques l3/r7/r11.
- PARAKEET_L2_MODEL=modèle Parakeet ci-dessus ; PARAKEET_L2_EN_WAV=R/t4/fixtures/source.wav, WAV PCM synthétique valide assez long pour les plages réellement lues.
- PARAKEET_RTC02_EXEC_ROOT=R/t5/rtc02 pour cleanup ; pour invocation reprocessing_create remplacer PARAKEET_L2_ARCHIVE_ROOT par R/t5/create.
- Reprocessing compile faux worker par rustc dans ses racines : enfant sous même garde/TEMP/TMP ; aucun moteur natif lancé par ce faux worker.
- Variables de qualification historiques inutiles absentes ; jamais WHISPER_L04_CAMPAIGN_BUILD=1, --ignored ou destination volontaire C:/WhisperLive.

À chaque réexécution, destinations fixtures distinctes sous racine de tâche. Modèles entrées seulement ; suppressions internes des tests bornées par préflight à leurs fixtures propres.

## 5. Vérifications proposées

Toutes NOT RUN. PowerShell/cwd dépôt ; & G -Mode Run -CargoArguments @(...) ; offline/locked quand acceptés. Résultats observables exigés en plus exit0.

| ID | Arguments Cargo | Observation |
| --- | --- | --- |
| V-U | test --offline --locked -p whisper-adapters --lib archive::tests -- --nocapture --test-threads=1 | Scénarios nommés effectivement exécutés/assertions |
| V-L | test --offline --locked -p whisper-adapters --test live_archive -- --nocapture --test-threads=1 | Huit existants à recompter + compléments ; snapshots/CURRENT/coverage/alias |
| V-K | test --offline --locked -p whisper-adapters --test parakeet_archive NOM -- --exact --nocapture --test-threads=1 | Un scénario exact par appel, aucun NOT RUN/retour précoce |
| V-R | test --offline --locked -p whisper-adapters --test reprocessing_cleanup -- --nocapture --test-threads=1 | Six existants à recompter + compléments ; faux worker/source/tiers/successeur |
| V-C | test --offline --locked -p whisper-adapters --test reprocessing_create -- --nocapture --test-threads=1 | CreateNew et publications préexistantes préservées |
| V-F | fmt --all -- --check | Format sans réécriture |
| V-B | build --offline --locked -p whisper-adapters -p whisper-desktop | Consommateur desktop compilable, pas remise binaire/fenêtre |
| V-S | clippy --offline --locked -p whisper-adapters --all-targets -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity | Résultats attribués, écarts historiques nommés/non aggravés, pas suppression globale |

V-K, neuf appels nommés :

1. parakeet_model_identity_prepares_persists_and_publishes_txt_srt
2. parakeet_archive_refuses_model_sha_divergence_before_pending_creation
3. pending_scan_and_resume_require_the_exact_qualified_test_key
4. registered_personal_pending_cpu_and_gpu_scan_and_resume_without_qualification_feature
5. retired_l2a_pending_remains_visible_but_cannot_be_resumed
6. retired_r5_2_pending_is_rejected_with_retirement_cause_and_preserved
7. gpu_pending_is_bound_to_the_qualified_physical_identity
8. parakeet_pending_queue_resumes_only_the_same_model_source_and_prefix
9. resume_refuses_over_limit_pending_and_preserves_its_bytes

Avant V-K : noms confirmés par liste des tests sous garde, environnement canonique et préconditions satisfaites. Libtest seul ne prouve pas l’exercice : pas NOT RUN, paramètres complets et effets/assertions réellement atteints. Variable absente/filtre vide échoue la vérification. Nouveaux tests nommés archive_characterization_* avec correspondance nom→PF→assertions dans P ; aucun identifiant de lot en production.

Pas --all-features ni tests ignorés. Branches recette-hooks déplacées T6 comparées statiquement ; compilation distincte feature seulement après contrôle variables, sans campagne L04. Native/GGUF modifié/longues imports/kill/powerfail/fenêtre Windows restent NOT RUN.

## 6. Dix tâches

### T1 — Préflight et garde

REQ/AC07,09, INV08. D0 et accord précis adaptation G/racines/campagne. Écriture future G, R/cargo-home,temp,tmp,logs,t1/ et P ; lectures garde F3/règles/design/plan/modèles/cache. Aucune source Rust. Sortie E0 : §4 appliqué, auto-tests/préflight/données/variables réussis et liste des suites/limites établie. Témoin : test archive nominal existant identifié au préflight exécuté sous garde conforme ; pas campagne entière déduite de ce témoin. G -Mode SelfTest puis V-U filtré sur ce nom. Arrêt fuite/non-conformité/ressource absente/accord manquant. Estimation non mesurée : préparation1–3h, auto-tests/premier build5–45min selon cache.

### T2 — Stockage/publication

C-PUB, PF03/04/05/08/09/21, REQ/AC03,04,07. Entrées D0,E0. Écritures U/publication_characterization.rs, UT pour inclusion ; U/publication_support.rs seulement réemploi helper existant sans injection produit ; R/t2/, P. A inchangé. Sorties C2/E2 : tests passing baseline et observations.

Scénarios : closure streamed en erreur, obstacle Windows au remplacement par fixture/handle/structure existante, TXT seul, manifeste absent/tronqué/divergent, intention seule/staging partiel, CURRENT avant index, index absent/tronqué/tmp résiduel, répétition. Observer final/tmp/catalogue/ancien/successeur/erreurs réelles. Marquer états reconstruits ; sync non reproductible reste limite, pas nouvel injecteur. Réutiliser R11 sans doublon. V-U,V-F ; V-B si support compilation exige. Estimation2–5h, tests2–15min cache chaud. Arrêt si correction fonctionnelle nécessaire.

### T3 — Possession et fin de publication

C-OWN, PF06/07/10/11, REQ/AC03,05,07. Entrées C2/E2. Écritures U/export_ownership_characterization.rs, UT, U/publication_support.rs si nécessaire ; R/t3/, P. A inchangé. Sorties C3/E3.

Branches ownership v1/v2/v3 effectivement présentes, JSON tronqué/version inconnue, finale avant identité persistée par seam existant, réconciliation TXT/SRT partielle, réservation résiduelle, tiers mêmes nom/octets et identité différente. Observer identité/octets tiers, CURRENT/catalogue, confirmation nom, deuxième résolution et cleanup effectif. Conserver R7. V-U,V-F. Estimation2–4h, tests2–15min. Arrêt branche supposée inexistante ou nouveau mécanisme produit.

### T4 — Reprise, journaux, micro, identité de référence

C-REC, PF01/02/12–18, REQ/AC01,02,04,05,07. Entrées C3/E3, modèles/WAV validés. Écritures U/recovery_characterization.rs, UT, I/live_archive.rs ; R/t4/, P. Pas modification I/parakeet_archive.rs pour routage : fallback §4. A inchangé. Sorties C4/E4.

Pending absent/tronqué/divergent, fragment JSONL initial/queue>8Kio/ligne complète corrompue au milieu ; checksum/ordre tentative/coverage ; segment durable avant coverage ; snapshots partiels/manifeste-CURRENT live invalides. Observer séquence/préfixe durable-confirmé/octets/diagnostics/deux scans-reprises, mutations actuelles conservées. V-U,V-L,V-K,V-F. Faux MP3 ne qualifie pas encodage ; neuf V-K exercés requis E4. Estimation3–6h, tests5–30min+hash modèles. Aucun modèle modifié/copié. NOT RUN ou modèle divergent : arrêt.

### T5 — Retrait et témoin complet avant extraction

C-RET, PF19/20 et interactions publication, REQ/AC03,05,06,07. Entrées C4/E4, Windows/modèle Turbo/rustc gardé. Écritures U/cleanup_characterization.rs, UT, I/reprocessing_cleanup.rs pour compléter fixtures existantes ; R/t5/, P. A inchangé. Sorties C5/E5.

Plan tronqué ; objet disparu avant progrès et inverse ; successeur/index divergents ; découverte réparant index/preuves sans suppression ; reprise explicite ; source modifiée/tiers/conteneur/parent legacy/inconnus préservés. Réutiliser scénarios cleanup. V-U,V-R,V-C puis V-L/V-K seulement si preuves antérieures ne sont plus attribuables à même révision tests/code ; V-F,V-B,V-S sur témoin complet avant extraction.

P relie chaque PF à test/assertions exercés ou limite design. Toute lacune obligatoire caractérisable non fermée bloque T6 ; seules limites déjà bornées (sync non injectable, absence preuve kill/powerfail notamment) demeurent. Deux cycles revue/correction max ; aucune élévation ni contournement Windows. Estimation3–6h, contrôles10–45min.

### T6 — Publication et supports

REQ/AC01,02,03,07,08, PF03–11,16–18,21, DEC-T01–04. Entrées C5/E5, code A baseline inchangé sauf divergence analysée. Écritures A et sept modules T6 §2 ; UT et fichiers U existants pour imports uniquement (assertions inchangées) ; R/t6/, P. Sorties C6/E6 compilables.

Ordre interne : storage/campaign puis imports ; catalog/exports ; orchestration publication ; préparation import et impl LiveArchive/résolution live. Types/records/état et court load_or_migrate restent A. Reprise/identité/cleanup encore A sont accessibles par imports internes. Conserver retour/sync/verrou/suppression/erreur, cfg, fonctions longues. Ces mouvements sont une tâche à propriétaire unique. V-U,V-L,V-R,V-C,V-F,V-B,V-S ; V-K si raccordement touche validation identité. Observer exports avant CURRENT/index, erreur tardive malgré commit, alias best effort et AC03 ; revue indépendante diff. Estimation3–6h, contrôles10–45min ; différence avant/après : arrêt.

### T7 — Reprise et journaux

REQ/AC01,02,04,07,08, PF01/02/12/13/16–18, INV02/04/05. Entrées C6/E6. Écritures A, trois modules T7 ; imports M/import.rs,live.rs,publication.rs, UT et U uniquement raccordements ; R/t7/,P. Sorties C7/E7.

Journal : persistance/lecture/visite/réparation segments/préfixe unified. Recovery : resume/scans import ; recovery/live : scans/alias/manifeste/CURRENT/préfixe/JSONL borné. load_or_migrate reste parent. Conserver réparations/ordre/ligne partielle versus complète/confirmations/non-duplication. V-U,V-L,V-K,V-F,V-B,V-S ; V-R si scan cleanup affecté. Revue imports/cfg/signatures/diff mécanique. Estimation2–5h, contrôles10–40min.

### T8 — Identité et sources

REQ/AC01,03,05,07,08, PF13–15,20, INV01/03/07. Entrées C7/E7. Écritures A, deux modules T8 ; imports seulement des modules T6/T7 §2, UT/U pour raccordements ; R/t8/,P. Sorties C8/E8.

Déplacer impl qualification/registre, identité job, lecture/fichiers unified, artefacts/chemins/intention/provenance/PCM ; source/hash/manifeste PCM segmenté/output samples. Types partagés A, API/réexports/visibilités inchangés, pas recalcul SHA universel/cache modifié. V-U,V-K,V-R,V-F,V-B,V-S ; V-L si vérifications live déplacées. Observer refus avant effets/bornes/provenance/versions pending-stratégie/identité physique vs contenu. Estimation2–5h, contrôles10–40min.

### T9 — Retrait contrôlé

REQ/AC03,05,06,07,08, PF19/20, INV03/06/07. Entrées C8/E8. Écritures A et trois modules T9 ; imports seulement M/publication.rs,publication/catalog.rs,recovery.rs,identity.rs ; UT/U pour raccordements ; R/t9/,P. Sorties C9/E9/façade finale.

API/plans, capture/reprise preuves, exécution Windows/non-Windows selon design. run_cleanup_plan_locked déplacé mécaniquement malgré longueur. Préserver propriété/conditions retrait/ordre suppression-progrès/ancêtres-conteneurs/legacy. V-U,V-R,V-C,V-F,V-B,V-S ; observer découverte sans retrait/successeur visible/anciens encore présents/reprise explicite/SourceChanged/tiers-inconnus préservés. Revue tranche. Estimation2–5h, contrôles10–45min.

### T10 — Delta cumulé et clôture

REQ/AC01–09, tous PF/INV. Entrées C9/E9. Écritures P,R/t10/ seulement ; aucune correction source furtive : retourner tâche/propriétaire dans limite cycles.

Une fois V-U,V-L,V-K,V-R,V-C,V-F,V-B,V-S sur final, sauf contrôles déjà attribuables exactement à ce candidat. Examiner delta Git/signatures/réexports/cfg/formats/littéraux/ordre effets/dépendances/tailles réelles ; fonctions longues historiques non aggravées. Revue cumulée indépendante via routage AGENTS ; reviewer seul CLEAN. Après deux cycles : LIVRÉ AVEC RÉSERVES et décision utilisateur.

Archiver petites preuves avec arborescence/compte/SHA-256 selon règle campagne, sans manifeste de qualification supplémentaire. Conserver fixtures tant que suppression non inscrite dans manifeste de nettoyage précisément approuvé ; cargo-shared préservé. Espace final relevé. Aucun push/binaire de recette/UI. Estimation contrôles15–60min plus revue, non mesurée.

## 7. Carte d’exécutabilité

Owner futur rust_implementer ; observations/bilan principale, jugement reviewer. Durées indicatives §6. Lecture modèles uniquement, aucun modèle/corpus utilisateur modifié. Tous contrôles proposés NOT RUN.

| Scénarios / AC | Baseline | Écritures directes/indirectes | Observation répétée | Commandes |
| --- | --- | --- | --- | --- |
| PF03–05,08–09,21 / AC03,04,07 | T2 | U/UT/support,R/temp,tmp,t2,C/cache local | Final/tmp/manifeste/CURRENT/index/actifs/erreurs/résolution | V-U |
| PF06–07,10–11 / AC03,05,07 | T3 | U/UT/support,R/t3,temp,tmp,C/cache | Octets-identité tiers/réservation/confirmation/retry | V-U |
| PF01–02,12–14 / AC01,04,05,07 | T4 | U/UT,deux racines fixtures R,C/cache | Séquence/diagnostics/mutations/deux reprises | V-U |
| PF16–18 / AC02,04,07 | T4 | U/UT/I-live,R/t4,temp,tmp,C/cache | Curseurs/coverage/snapshots/CURRENT/alias/refus | V-U,V-L |
| PF15 / AC05 | T4 | R/t4/parakeet-runtime/r6/archive,C/cache ; modèles/WAV lus | Neuf scénarios exercés, aucun NOT RUN, pending/source/refus | V-K |
| PF19–20 / AC03,05,06,07 | T5 | U/UT/I-cleanup,R/t5/rtc02 et create,faux worker.rs/exe,C/cache ; modèle lu | Découverte sans retrait/source/tiers/successeur/progrès/rejeu | V-U,V-R,V-C |
| Tous / AC01–08 après extraction | T6–T10 | Sources attribuées hors campagne ; R/C seuls pendant commande | Mêmes assertions/fixtures baseline, API/formats inchangés | §6 puis ensemble T10 |
| Préalables/concurrence / AC09 | T1 et suivants | Git lu/P modifié hors campagne | Responsabilité/préflight/décision avant collision | Git/lecture statique |

Couverture inverse : modules→REQ08 et responsabilité §2/6 ; nouveaux tests→REQ07 et groupe C ; garde/racines→préalables AGENTS. Documents : design et P seulement. Chemin nécessaire absent : PLAN_CHANGE avant exécution, pas élargissement tacite.

## 8. Arrêt, reprise, retour arrière

Arrêt tâche affectée : garde/préflight/auto-test échoué/espace<30Gio/écriture hors racine ; filtre vide/NOT RUN/ignored involontaire/scénario manquant ; différence inexpliquée/contrat modifié/Git inattendu ; nouveau hook/trait/API/dépendance/chemin nécessaire ; conflit mission/préalable dernier chantier non satisfait.

Préserver preuves/changements d’autrui ; pas reset global/suppression racine/restauration externe/effacement données utilisateur. Fuite : chemin+heure puis arbitrage avant reprise. Retour extraction : rétablir uniquement delta source attribuable après Git/coordination, sans rollback/migration archives puisque formats inchangés. Ne pas supprimer baseline/fixtures incident avant clôture ; reprendre dernier Cn/En prouvé après préflight. Répéter campagne seulement pour changement/échec/préoccupation non résolue.

## 9. Limites et passage

Signatures finales/lints/build/tests/campagne restent NOT RUN. Erreurs/états reconstruits et seam existant ne démontrent ni kill ni powerfail ; MP3 réel/moteur natif/GPU/fenêtre/paquet non qualifiés.

Après revue PLANS : GO d’exécution doit nommer tâche/lot, chemins source/tests, adaptation G, racines R/C et campagne utile, dont accord précis pour adaptation PowerShell existante. Acceptation plan/CLEAN ne suffit pas. Push soumis à accord distinct.

## 10. Revue PLANS et état final

Reviewer indépendant rust_design_reviewer `/root/revue_design_archive`, sans participation à rédaction/synthèse/correction : **CLEAN**, zéro REQUIRED et zéro ADVISORY, sur le candidat Git `e843a97a9522a15a7dfc2fcc059b124e31ed1a9c`, copie lue identique au fichier Git. Design parent `0bec1a3a` CLEAN préservé.

Contrôles rendus : neuf CAP/REQ et 21 familles PF reliées aux tâches/chemins/observations ; couverture inverse, graphe sans cycle, baseline T2–T5 avant T6, quinze modules attribués et VIS-01 ; huit tests live, six cleanup et neuf noms V-K recoupés ; fallback r6, racines cleanup/create et faux workers internes confirmés ; lecture seule modèles ; garde source à adapter et absence de transfert de dérogations ; contrats sensibles observables et arrêt/retour arrière/permissions conformes. Lecture/comparaison statique et git diff --check réussis ; aucun test ni Cargo exécuté.

Le coordinateur consigne PLANS READY sur cette preuve. Le présent ajout et le statut consignent le verdict sans modifier les tâches revues. Adaptation de garde, ressources, témoins baseline, comparaisons après extraction et validation produit restent NOT RUN. Aucun push réalisé.

## 11. Exécution autorisée le 30 octobre 2026

Accord utilisateur dans la session d’exécution : « GO T1–T10 selon le plan, avec les chemins Rust/tests prévus, l’adaptation PowerShell de la garde, les campagnes sous les deux racines indiquées, la caractérisation avant extraction, les revues et les commits locaux, sans push. » Cet accord remplace l’absence de GO documentaire mentionnée dans les sections historiques ; les prérequis et conditions d’arrêt restent applicables.

Base de départ : `7fdd329b`, branche `evo-startup-perf`. Aucun fichier suivi modifié ; non suivis `docs/architecture/`, `models/`, `sound example/` conservés. Espace initial relevé par la principale : 68,45 Gio. Responsabilités : `/root/implementation_archive` (`rust_implementer`, IMPLEMENT) propriétaire unique des sources/tests et de la garde prévus ; session principale auteur du présent bilan. Aucune modification source concurrente par la principale. T1 seule engagée initialement ; aucune extraction avant caractérisation T2–T5 et gates prévus. Résultats d’exécution à consigner après réception des preuves.

### T1 — Arrêt sur événement externe

Préflight de l’implémenteur : HEAD `7fdd329b`, branche inchangée, 67,49 Gio libres ; rustc/MSVC/CMake/CUDA et LIBCLANG_PATH présents. Tailles et SHA-256 des deux modèles conformes au registre. Garde adaptée dans G : racine R substituée, tolérances F3 Temp/Downloads/objets Git retirées, renommages entre racines dans les deux sens contrôlés ; syntaxe PowerShell validée par Parser.ParseFile. Cache local copié sous R, aucune dépendance téléchargée.

Premier `G -Mode SelfTest` : **FAIL**, sortie 1. Événement réel `Changed` sur `C:/Users/cyril/AppData/Local/Temp/NGL/NGLClient_CoreSync17.8.10.1.log` à `2026-10-30T20:07:42.6464160+01:00`, détecté à `20:07:42.7017951+01:00`. Origine non attribuée ; ni écriture de campagne prouvée ni destination autorisée. Journal conservé : `target/parakeet-tests/refacto-archive/logs/guard-events.jsonl`. Aucun transfert de tolérance historique, aucune relance après l’échec.

Conformément aux §§4/8, arrêt et arbitrage avant reprise. Surveillance terminée. La jonction temporaire de l’auto-test a été retirée par son nettoyage prévu ; cache, garde, répertoires selftest et preuves conservés. Aucun fichier Rust modifié. Espace final rapporté : 67,80 Gio. Témoin T1, T2–T10, campagnes Cargo et revue d’implémentation : **NOT RUN**. E0 non acquis ; aucune extraction autorisée à démarrer en cet état. Aucun push. Reprise seulement après décision utilisateur puis auto-tests conformes et témoin T1.

### Investigation et tentative de reprise T1

Investigation en lecture seule autorisée : journal périodique toutes les 30 secondes, thread 4036 également présent dans CoreSync PID 50064, lancé le 20 octobre ; exécutable `C:/Program Files (x86)/Adobe/Adobe Sync/CoreSync/CoreSync.exe`, version 7.8.10.1, signature Adobe Inc. valide. Origine Adobe fortement étayée sans trace système attribuant rétrospectivement chaque écriture.

Accord utilisateur : arrêt uniquement de CoreSync après vérification d’identité, puis reprise T1–T10 sans assouplir la garde et sans push. La principale a vérifié chemin, signature et stabilité PID/date de création puis arrêté seulement PID 50064. Aucun autre processus ou service arrêté. Préflight de reprise sur `f1637234` : 67,15 Gio libres, CoreSync absent, garde inchangée.

Second SelfTest : **FAIL**, événement `Changed` du même journal à `2026-10-30T20:13:14.8069630+01:00`, détecté à `20:13:14.8130888+01:00`. Arrêt sans Cargo et preuves conservées dans le journal existant. Lecture ultérieure : CoreSync relancé à `20:13:09.521904+01:00`, PID 140924, parent 22808 (`Adobe Desktop Service.exe`, déjà présent). Aucun nouvel arrêt effectué. Espace final 66,84 Gio. La garde reste inchangée ; E0, témoin et T2–T10 restent non acquis/NOT RUN. Nouvel arbitrage requis pour traiter le relanceur extérieur au périmètre autorisé.

### Troisième tentative T1 — Auto-tests réussis, témoin interrompu

Accord utilisateur pour arrêter après vérification Creative Cloud.exe, Adobe Desktop Service.exe puis CoreSync.exe, et reprendre T1–T10 sans assouplir la garde ni pousser. Creative Cloud.exe était déjà absent ; chemins attendus, signatures Adobe Inc. valides et stabilité d’identité vérifiés avant arrêt ciblé d’Adobe Desktop Service.exe PID 22808 puis CoreSync.exe PID 140924. Aucun autre processus ou service arrêté. Les trois étaient absents après l’action et au préflight.

Préflight sur `8cf827a6` : 67,08 Gio libres, outils et syntaxe présents. Garde inchangée. `G -Mode SelfTest` **PASS**, sortie 0 : chemins/variables hors racines refusés, test positif réussi et événements externes simulés refusés, y compris les deux sens des renommages traversant les racines.

Témoin lancé par G : `test --offline --locked -p whisper-adapters --lib archive::tests::create_only_publication_returns_verified_internal_unified_artifacts_without_external_exports -- --exact --nocapture --test-threads=1`. Garde **FAIL**, sortie 1 : création de `C:/Users/cyril/AppData/Local/Temp/etilqs_0nJY2ZTRJ16D26D` à `2026-10-30T20:15:18.8651898+01:00`, détection à `20:15:18.9572266+01:00`. Attribution inconnue ; ne pas conclure à une fuite Cargo ou à une activité extérieure prouvée. Commande interrompue, stdout/stderr vides ; témoin non démontré. Run `20261030T201516416-624f193a`, preuves dans R/logs/guard.jsonl et guard-events.jsonl. Aucun effacement, aucune relance.

Espace final : 67,40 Gio ; `git diff --check` réussi. Aucune source Rust modifiée, non suivis initiaux préservés. T1 reste non validée, T2–T10 et revue d’implémentation NOT RUN. Reprise soumise à arbitrage conformément au plan ; aucune tolérance ajoutée et aucun push.

### Dérogation Temp autorisée pour la reprise

Accord utilisateur dans cette session : « Ne t'arrete plus a cause de modif/creation dans temp/ Je t'autorise la reprise T1–T10 ». Pour cette campagne archive uniquement, cet accord remplace la règle d’arrêt sur les événements `Created`/`Changed` sous `C:/Users/cyril/AppData/Local/Temp/`, y compris les cas précédemment non attribués. Ces événements restent consignés avec chemin/heure sans interrompre la campagne ; leur origine ne devient pas prouvée. L’accord ne crée aucune destination volontaire de campagne hors R/C : CARGO_HOME/TEMP/TMP restent internes, préflight canonique, auto-tests et surveillance maintenus. Aucun transfert aux autres projets/campagnes. Les autres événements et contrôles, dont racines sensibles, variables hors racine, erreurs de surveillance et échecs de commande, conservent leur traitement prévu. Les auto-tests doivent distinguer créations/modifications Temp consignées sans arrêt des refus toujours applicables. Reprise par le même implémenteur ; aucun push.

Première traduction trop étroite de « modifications » en Created/Changed seulement : auto-tests réussis, témoin interrompu sur `Deleted C:/Users/cyril/AppData/Local/Temp/_MEI770962/api-ms-win-core-console-l1-1-0.dll` à `2026-10-30T20:19:45.6912782+01:00` (détection `20:19:45.7482398`). Run `20261030T201939048-abd71749`, sortie 1 ; compilation de dépendances commencée, aucun test attesté. 67,32 Gio libres. Preuves conservées dans R/logs/ ; événements Temp admis consignés dans allowed-temp-events.jsonl, attribution UNKNOWN.

La principale rectifie son interprétation pour appliquer l’intention explicite de ne plus s’arrêter sur les modifications du Temp : créations, changements, suppressions et renommages entièrement internes au Temp utilisateur sont consignés sans arrêt. Un renommage dont l’autre extrémité sort du Temp reste contrôlé normalement. Cette précision remplace la restriction Created/Changed du paragraphe précédent ; aucune nouvelle destination volontaire ni tolérance hors Temp, erreur de surveillance ou variable incorrecte n’est accordée. L’implémenteur adapte les auto-tests avant reprise du témoin.

### T1 validée — E0 acquis

Préflight sur `0d88375b` : 67,31 Gio libres. SelfTest **PASS**, sortie 0 ; les quatre catégories de changements internes Temp sont consignées sans arrêt, les traversées et autres événements externes simulés sont refusés. Témoin nominal exact ci-dessus **PASS**, sortie 0, un test exécuté/réussi, zéro ignoré, 288 filtrés ; commande 44 s. Run `20261030T202052854-d4c76281` dans R/logs/guard.jsonl, stdout/stderr associés conservés. 26 avertissements de compilation rapportés, pas de succès de suite entière déduit du témoin. Garde SHA-256 `16750A3D5E39609330364AE25F1416B6D037687BF7AC62F923F4B78B26CA00DF` ; espace final 67,22 Gio. Git suivi intact, `git diff --check` réussi.

Revue indépendante routée par `/root/review_archive` (`rust_review_coordinator`) à un `rust_reviewer` en lecture seule : **CLEAN**, zéro constat, demande de vérification ou lacune matérielle ; une revue, aucune correction. Cible : base `7fdd329b`, candidat `0d88375b` et garde identifiée ci-dessus, scope T1. Comparaison G/F3, racines, retrait des tolérances historiques, dérogation Temp et preuves E0 revus. Limites conservées : surveillance sans confinement préventif, origine des événements Temp inconnue. T1 seule validée ; T2–T5 restent préalables aux extractions, T2 peut commencer. Aucun fichier Rust modifié à ce stade.

### T2 validée — Caractérisation stockage/publication

Base `8515d6c7`, neuf nouveaux tests dans U/publication_characterization.rs, inclusion UT ; A inchangé. Préflight 67,21 Gio, final 67,20 Gio. V-U run `20261030T202618192-60d1812a` : **28 réussis, zéro échec, un ignoré historique L04 explicitement exclu**, 269 filtrés ; neuf nouveaux scénarios et R11 existants exercés. V-F run `20261030T202643221-859a9917` et `git diff --check` **PASS**. Fixtures R/t2 via ARCHIVE_CHARACTERIZATION_ROOT ; temporaires des autres tests internes à R.

| PF | Nouveaux tests archive_characterization_* (suffixes) / observations |
| --- | --- |
| 03,21 | stream_error, replace_obstacle, txt_only_prepare : erreur, final/tmp, préparation partielle et reprise |
| 04 | missing_truncated_and_divergent_manifest : refus, restauration et scan complet |
| 05 | intent_and_partial_staging, divergent_staged_srt : intention seule, staging partiel/divergent, absence publication puis reprise |
| 08 | current_before_index_internal_resolution : deux résolutions, index/ordinal stables ; R11 existant couvre ancien actif et successeur |
| 09 | index_absent_truncated_and_temporary_retry, index_rename_obstacle : erreurs répétées, fichiers conservés, reprise ; R11 nominal exercé |

Revue indépendante T2, même coordinator/reviewer : **CLEAN**, zéro constat/demande/lacune ; une revue, aucune correction. Candidat worktree sur base `8515d6c7`, nouveau fichier SHA-256 `08EDA0B5A0953FFCDA60D808530F41687A1FCB8B3D074380604831688DD95FBA`. Aucun code de production/manifeste/lock modifié. États reconstruits : pas de panne sync_all forcée, kill ou powerfail démontré. C2/E2 acquis ; T3 peut commencer.

### T3 validée — Possession et fin de publication

Base `ca958a7b`, six tests U/export_ownership_characterization.rs et inclusion UT, production inchangée. Préflight/final : 66,85 Gio. V-U run `20261030T202955772-97555c86` : **34 réussis, zéro échec, un ignoré historique L04**, 269 filtrés ; V-F run `20261030T203015362-9e40dfe4` et `git diff --check` **PASS**. Fixtures R/t3.

| PF | Tests archive_characterization_* (suffixes) / observations |
| --- | --- |
| 06 | ownership_versions_one_two_three_follow_existing_paths : v1 hardlink, migration v2→v3, v3, deux essais ; ownership_truncated_or_unknown_refuses_without_mutation : erreurs, octets et identités conservés |
| 07 | v3_final_link_before_identity_is_recovered : final avant identité persistée ; R7 existant exercé pour seam copie exclusive/reconfirmation/tiers/ancien actif |
| 10 | reconcile_partial_skips_byte_identical_third_party_then_retries : TXT déplacé, SRT tiers mêmes octets/identité différente préservé, répétition puis SRT propriétaire déplacé, index/CURRENT conservés |
| 11 | residual_reservation_cleanup_refusal_and_retry : refus puis suppression et répétition, index/CURRENT inchangés ; second_external_resolution_preserves_committed_publication : deuxième résolution baseline refuse DuplicateStateChanged, données préservées |

Même reviewer indépendant : **CLEAN**, zéro constat/demande/lacune ; une revue, aucune correction. Candidat nouveau fichier SHA-256 `1490EAFFE12C8489F093E5DA73CE3150303E499B7F645556690A63B5839A64F5` sur base ci-dessus. Coupures reconstruites/simulation, pas kill/powerfail. C3/E3 acquis ; T4 peut commencer.

### T4 validée — Reprise, journaux, micro et identité

Base `48976cfa`, trois nouveaux tests U/recovery_characterization.rs et quatre nouveaux I/live_archive.rs, inclusion UT ; production inchangée. Préflight 66,83 Gio, final 66,76 Gio. Modèles : tailles/SHA-256 conformes au registre ; WAV synthétique PCM mono 16 kHz, 32 000 échantillons, 64 044 octets. Neuf racines V-K distinctes internes R/t4/r6/vk1 à vk9, aucun routage de test Parakeet modifié.

| PF | Tests archive_characterization_* (suffixes) / observations |
| --- | --- |
| 01 | pending_absent_truncated_and_divergent : états pending et refus conservés |
| 02 | segment_journal_repairs_initial_and_large_partial_tail ; complete_corrupt_middle_segment_is_not_repaired : fragments initiaux, queue >8 Kio, ligne complète corrompue, réparations/refus et préfixe |
| 12–14 | Tests existants scan de corruption, refus inter-destinations, copie/reprise préfixe exercés avec V-U |
| 15 | Neuf noms V-K de §5 exercés chacun exactement : **1 réussi, zéro ignoré**, aucun NOT RUN ; modèles/WAV/préconditions contrôlés |
| 16 | attempt_journal_corruption_is_not_read_by_recovery : checksum et ordre écrits vérifiés, puis corruption ; deux scans identiques sans mutation. Limite baseline explicite : le parcours recovery ne lit pas le journal attempts.jsonl, aucune correction introduite |
| 17 | live_coverage_checksum_and_order_are_refused ; live_segment_before_coverage_stays_unconfirmed : coverage invalide refusée, segment durable non confirmé sans coverage |
| 18 | live_current_manifest_and_snapshot_corruption_are_stable : CURRENT/manifeste/snapshots invalides diagnostiqués sur deux scans |

V-U run `20261030T203305213-6fc3f066` : **37 réussis, un ignoré L04 historique** ; V-L run `20261030T203325181-1df36b16` : **12 réussis, zéro ignoré** ; neuf V-K et V-F run `20261030T203536116-f492f82b` **PASS**. Journaux détaillés par commande dans R/logs/guard.jsonl ; `git diff --check` réussi. Même reviewer indépendant : **CLEAN**, zéro constat/demande/lacune, une revue/aucune correction, sur candidat worktree base ci-dessus et fichiers de tests identifiés. Limites : états reconstruits, faux MP3 sans qualification encodage, pas moteur natif/kill/powerfail. C4/E4 acquis ; T5 peut commencer.

V-K T4, références des neuf runs dans l’ordre §5 : `20261030T203357199-0cf23557`, `20261030T203406324-66122c0c`, `20261030T203415420-e0b59153`, `20261030T203431925-7c6a0021`, `20261030T203441019-cf85b129`, `20261030T203449919-1db5b42d`, `20261030T203506809-67dedd90`, `20261030T203516006-43171a8a`, `20261030T203525034-de85e41c`.

### T5 validée — Retrait et témoin complet avant extraction

Base `a7c2690f`, ajout U/cleanup_characterization.rs et inclusion UT, compléments I/reprocessing_cleanup.rs ; production toujours inchangée. Préflight 66,76 Gio, final 66,60 Gio. Modèle Turbo taille/SHA conformes au registre ; faux workers/fixtures sous R/t5, rustc enfant gardé. PF19/20 : plan tronqué refusé en découverte/reprise deux fois sans altérer les preuves ; `archive_characterization_cleanup_missing_object_before_progress_is_adopted` et `archive_characterization_cleanup_recorded_removed_but_present_is_preserved` caractérisent les deux sens fichier/progrès. Les six tests cleanup existants exercent découverte réparant index/preuves sans retrait, preuve successeur/index divergente, SourceChanged, reprise explicite, tiers/inconnus/conteneurs/legacy préservés.

| Contrôle | Run / résultat |
| --- | --- |
| V-U | `20261030T203835728-b838ce9c` : 38 réussis, zéro échec, un ignoré L04 historique |
| V-R | `20261030T203854600-f4960520` : huit réussis, zéro ignoré |
| V-C | `20261030T203939361-1b394ca9` : deux réussis, zéro ignoré |
| V-F | `20261030T204008397-20fcdd7b` : PASS |
| V-B | `20261030T204012923-fc578477` : adapters et desktop compilés, PASS |
| V-S | `20261030T204052504-bb5f7b2e` : PASS avec avertissements historiques ; notamment run_cleanup_plan_locked 500 lignes/complexité 33, type archive.rs:943 ; production inchangée, aucun lint nouveau T5 constaté |

`git diff --check` PASS. V-L et neuf V-K T4 restent attribuables au même code produit et aux mêmes tests, inchangés. Même reviewer indépendant : **CLEAN**, zéro constat/demande/lacune, une revue/aucune correction ; contrôle inverse des PF01–21 et des lacunes obligatoires T2–T5 : aucune lacune caractérisable obligatoire identifiée avant T6. C5/E5 acquis. Limites maintenues : sync_all non injectable, journal de tentatives non relu par recovery caractérisé, états reconstruits/faux MP3, pas kill/powerfail/natif/fenêtre Windows. Aucune extraction antérieure à cette validation.

Correspondance inverse PF→preuves baseline : PF01/02/12–18 §T4 (tests reprise, scan sain/corrompu, refus inter-destination, schémas/stratégie et neuf V-K) ; PF03–05/08–09/21 §T2 ; PF06–07/10–11 §T3 avec R7 ; PF19–20 §T5 avec huit V-R. Chaque groupe relie tests/assertions et limites ; aucun exit 0 sans scénario exercé ne ferme une famille.

### T6 validée — Publication et supports

Base `620e26a0`, A et sept modules prévus déplacés, tests/assertions inchangés. Imports nommés, visibilité interne minimale VIS-01, types/états et responsabilités T7–T9 conservés dans A. Préflight 66,60 Gio, final 66,59 Gio. Lignes physiques relevées après extraction (pas décompte de production Clippy) : A 4 321 ; campaign 484 ; storage 75 ; import 529 ; live 714 ; publication 305 ; publication/catalog 989 ; publication/exports 619. Façade intermédiaire encore volumineuse avant T7–T9.

Contrôles gardés : V-U `20261030T205239329-04773b13` **38 réussis, un ignoré L04** ; V-L `20261030T205304174-4a04c265` **12 réussis** ; V-R `20261030T205320031-1b14dca1` **8 réussis** ; V-C `20261030T205356166-f351d9f6` **2 réussis** ; neuf V-K exacts chacun un réussi/zéro ignoré/aucun NOT RUN après extraction et avant derniers changements d’imports uniquement. V-B `20261030T205430673-fee00f35`, V-S `20261030T205446074-6ff169ad`, V-F final `20261030T205557356-67aeeba2` et diff --check **PASS**. Un V-U intermédiaire après imports nommés avait échoué sur quatre imports dupliqués ; corrigés puis contrôles rafraîchis. Avertissements normaux revenus à la baseline, imports tests sous cfg(test) ; dernier delta seulement retrait de trois imports inutilisés sous recette-hooks, format final rafraîchi.

Contrôle distinct optionnel `check --offline --locked -p whisper-adapters --features recette-hooks`, run `20261030T205454890-37383c39` : **FAIL**, exit 101, `thread` et `Duration` non résolus dans worker_ipc/runtime/support.rs:90. Fichiers support.rs, worker_ipc.rs et runtime/mod.rs inchangés depuis baseline, anomalie hors périmètre non corrigée. Aucun succès de cette configuration revendiqué ; pas de campagne L04.

Même reviewer indépendant : **CLEAN**, zéro constat/demande/lacune, une revue/aucune correction reviewer. Comparaison de 194 corps de fonctions après normalisation espaces/commentaires : inchangés ; six différences de signatures dues aux virgules finales rustfmt seulement. Types/statics/attributs/API/réexports/cfg, ordre des effets et visibilités VIS-01 préservés. Échec optionnel recette-hooks jugé préexistant par preuve statique des fichiers et chaînes d’imports inchangés, non attribuable à T6. Candidat stable campaign SHA-256 `D24F4EE65AA886DF24B5DB07DB64896472FE71EF9296B1F02E4AAAB6CB001EE6`. C6/E6 acquis ; T7 peut commencer. Fonctions longues historiques conservées ; aucune validation native/Windows/kill/powerfail déduite.

### T7 validée — Reprise et journaux

Base `0f5dedc9`, A et trois modules journal.rs/recovery.rs/recovery/live.rs seulement ; tests et modules T6 inchangés. Imports explicites, réexport public scan_live_recoveries conservé, load_or_migrate_publication_index reste façade. Préflight 66,59 Gio, final 66,57 Gio. Lignes non vides rapportées (autre métrique que les lignes physiques T6) : A 2 778 ; journal 306 ; recovery 748 ; recovery/live 398.

V-U `20261030T210126452-88453588` : **38 réussis, un ignoré L04 historique** ; V-L `20261030T210145045-bea8ec0d` : **12 réussis** ; V-R `20261030T210203595-da992c3e` : **8 réussis** (rejoué car le scan déplacé peut servir cleanup). V-F `20261030T210418640-bbb9bb45`, V-B `20261030T210423919-2481db8e`, V-S `20261030T210439375-e0768d2d`, diff --check **PASS**. Avertissements build adapters 16/desktop 5 ; Clippy 260 test/136 doublons, historiques. Premier V-U réussi avec import inutile ensuite retiré ; les preuves ci-dessus portent sur le candidat stabilisé.

Neuf V-K exacts chacun **un réussi, zéro ignoré, aucun NOT RUN**, dans l’ordre §5 : `20261030T210241113-6e248540`, `20261030T210251807-63ec59db`, `20261030T210301285-945f64d8`, `20261030T210315268-a48d50f8`, `20261030T210324358-d14e5449`, `20261030T210333512-066d9b11`, `20261030T210349557-7aa23821`, `20261030T210358487-019567c2`, `20261030T210407948-ca72a49b`.

Même reviewer indépendant : **CLEAN**, zéro constat/demande/lacune ; une revue/aucune correction. 103 corps identiques après normalisation lexicale espaces/commentaires ; types/attributs conservés, quatre différences de virgules finales de signatures seulement ; VIS-01 et réparations/ordre préservés. C7/E7 acquis ; T8 peut commencer. recette-hooks non requalifié (échec préexistant T6), pas natif/UI/kill/powerfail.
