Avancée : dix tâches définies, de la caractérisation préalable aux extractions puis à la clôture.
Blocage : aucune exécution autorisée ; garde et caractérisation baseline non exécutées.
Décision : soumettre le candidat à la revue indépendante PLANS.

# Plan de refactorisation d’archive.rs

## 1. Identité, autorité et portée

- ID : PLAN-ARCHIVE v1, 30 octobre 2026, DRAFT.
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
