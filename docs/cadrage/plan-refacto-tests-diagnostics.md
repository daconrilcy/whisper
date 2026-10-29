Avancée : refactor implémenté et vérifié ; revue indépendante du candidat CLEAN.
Blocage : aucun constat requis ouvert dans le périmètre ; limites d'exécution précisées au bilan §12.
Décision : publication du lot autorisée par la demande « commit et push » ; conserver les données, caches et preuves.

# Plan de refactorisation — tests séparés et diagnostics

## Header

### Amendement de session — autorisation et garde (v1.1)

Sources d'autorité : après la remise du plan, l'utilisateur a écrit « tu as mon accord pour implementer », puis « oui je t'autorise » en réponse aux deux arbitrages explicites : adaptation de la garde sous target/parakeet-tests/refacto-tests-diagnostics/ sans ses anciennes dérogations, et implémentation exceptionnelle par la session principale si rust_implementer reste indisponible. Le runtime a refusé sa création avec « agent thread limit reached », y compris après tentative de libération des anciennes sessions. La session principale est donc responsable du code ; aucune implémentation concurrente déléguée.

CHANGE-AUTH-01 : G-CODE levé pour le lot. Extension autorisée G0 (une tâche préalable, portant le total à dix) : adaptation de target/parakeet-tests/main-facades/guard-main-facades.ps1 en target/parakeet-tests/refacto-tests-diagnostics/guard-refacto-tests-diagnostics.ps1. Racine d'exécution unique target/parakeet-tests/refacto-tests-diagnostics/, cible Cargo inchangée target/parakeet-tests/cargo-shared/. Pas de nouvelle surveillance : reprise des emplacements déjà surveillés, aucune exception Temp/Git objects/CUDA héritée. Tout événement hors racines arrête ; surveillance observatrice, pas confinement préventif. Aucun service, ACL, élévation ou nouvelle dépendance. La garde d'origine est préservée.

G0 : zéro production modifiée ; un script de campagne dans la racine autorisée, préflight, auto-tests synthétiques de refus hors racine/variables absentes/erreur watcher/état Git changé et observation réelle d'un fichier dans la racine. Auto-tests PASS avant Cargo. CARGO_HOME/TEMP/TMP fixés sous racine, CARGO_TARGET_DIR sur cible partagée, CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only. Cache hors ligne copié en lecture seule depuis le cache historique sous la nouvelle racine ; aucune utilisation du cache utilisateur en écriture. Les petits contrôles de garde restent dans target et ne constituent pas un manifeste de qualification du produit.

Invocation effective (PowerShell, cwd dépôt) : `& ./target/parakeet-tests/refacto-tests-diagnostics/guard-refacto-tests-diagnostics.ps1 -Mode SelfTest`, puis le même script avec `-Mode Run -CargoArguments @('test','--offline','--locked',...)` pour chaque commande V6/V7/V8 autorisée. Aucun Cargo direct. G-EXEC levé uniquement après auto-tests, cache privé disponible et préflight de chaque invocation ; T1 reste requis avant T2. Arrêt sur toute violation/erreur, conservation des traces et arbitrage avant reprise. G-HIST inchangé. Le statut CLEAN précédent porte sur v1.0 ; cet amendement enregistre l'accord de session sans lui attribuer de nouvelle revue indépendante.

Le reste du plan v1.0 ci-dessous décrit les neuf tâches produit, inchangées. Aucun push/commit produit ou qualification Windows déduit de l'accord.

### Résultat de la première tentative d'exécution

G0 : garde adaptée, syntaxe vérifiée, auto-tests PASS (refus variables absentes/hors racines, observation réelle dans racine, événements synthétiques Temp/Git/erreur watcher refusés). Cache privé copié depuis main-facades/cargo-home, sans écriture dans sa source. Espace avant environ98,7Gio, après copie/arrêt environ97,6Gio.
T1 : première commande interne `cargo test --offline --locked -p whisper-core --lib -- --list` lancée par la garde puis interrompue. La compilation des dépendances avait commencé ; aucune liste de tests ni résultat de témoin n'a été produit. À 2026-10-29T11:34:47.1162980+01:00, le watcher a observé Created sur `C:\Users\cyril\AppData\Local\Temp\etilqs_YEpsdpwIe6Klco3`. Absence de PID : origine et attribution indéterminées, ni écriture Cargo démontrée ni écriture autorisée. La garde stricte a arrêté la commande et enregistré STOP.json. Aucun contournement, reprise, nettoyage ou modification de ce fichier.
Traces conservées dans target/parakeet-tests/refacto-tests-diagnostics/ : STOP.json, logs/guard-events.jsonl et logs/8437fdf69f0040d793800165d1459ded.{stdout,stderr,git-before,git-after}.txt. Les événements synthétiques des auto-tests sont distincts de l'événement réel de la commande ; ils ne prouvent aucune écriture externe.
T1 non clos, T2 àT9 NOT RUN, sources Rust inchangées. G-EXEC de nouveau bloquant : arbitrage de l'utilisateur nécessaire avant toute reprise conformément AGENTS et G0. Revue indépendante encore non exécutée, capacité d'agents indisponible ; aucun CLEAN de code revendiqué.

### Arbitrage de reprise — dérogation Temp limitée à ce lot

Après ce constat, l'utilisateur a répondu « oui je t'autorise » à la demande explicite de consigner les événements du seul Temp Windows sans attribution démontrée comme UNATTRIBUTED, sans arrêt automatique pour ce seul motif. Cette dérogation de session est propre à ce lot ; elle n'autorise aucune destination volontaire hors racines. Toute écriture extérieure attribuée à la campagne ou modification sensible inexpliquée impose toujours l'arrêt. Le watcher sans PID laisse origine/nature indéterminées ; ses observations ne prouvent ni autorisation ni attribution Cargo, et la surveillance n'est pas un confinement.
La garde conserve STOP.json et autorise uniquement la reprise du constat exact ci-dessus (chemin/type/date), pas un acquittement global des futurs arrêts. Auto-tests mis à jour avant reprise : Temp synthétique consigné, Git objects/autres chemins/erreurs watcher refusés. Aucun autre assouplissement.

Deuxième arrêt : cœur défaut découvert avec succès (80 tests du harness), puis baseline adapters interrompue le 29 octobre à 11:37:43.1529194+01:00 sur Changed de .git/objects/pack/tmp_pack_yLcAWX. États Git avant/après strictement identiques ; aucune attribution de processus démontrée. L'utilisateur a ensuite accepté « Oui, avec ces limites » à la dérogation explicite : événements non attribués du seul .git/objects consignés sans arrêt automatique, sous réserve de conservation des références/index/sources/statut. Toute autre modification sensible ou écriture extérieure attribuée à la campagne impose toujours arrêt. Cette autorisation propre au lot ne transfère pas une ancienne dérogation. Trace de l'arrêt préservée, auto-tests actualisés pour cette classification ; refs/index/état Git restent à refus fermé.

Baseline suivante : adapters défaut282, recette-hooks286, hooks+l3-gpu287 tests découverts. l01-memory-qualification échoue avant refactor avec trois E0252 (Mutex/SystemTime/UNIX_EPOCH importés deux fois dans worker_ipc.rs). Journal d7c9ef14ff9649d987716fc6d40b009b.stderr.txt conservé, arrêt de la campagne. Accord explicite de l'utilisateur « Oui, corriger puis poursuivre » à la suppression de ces trois imports redondants et reprise. Correction préalable bornée dans la tâche T1 (mêmes types déjà importés sans condition, aucun changement de logique), pas de tâche produit supplémentaire ; baseline de cette configuration à rétablir après correction. Les autres configurations n'activent pas cet import conditionnel.

T1 ensuite fermé : neuf configurations découvertes (core80 ; adapters282/hooks286/hooks+gpu287/memory282/personal282 ; desktop44 pour défaut/hooks/personal) et quatorze exécutions de témoins ciblés PASS, chaque test exécuté une fois sans ignored. T2 application44, T3 UI40 et T4 archive22 extraits ; listes et témoins de leurs configurations identiques à la baseline. T5 IPC129 extrait avec supports, aucun #[test] dans les quatre racines ; défaut/hooks/gpu/memory après extraction PASS. Dernière configuration personal interrompue sur Created de .git/index.lock à 11:53:12.2212725+01:00 : état Git avant/après strictement identique, verrou disparu. Un préflight concurrent de l'agent avait lancé git status sans --no-optional-locks et est une origine possible (pas de PID dans le watcher). Arbitrage de reprise demandé ; pas d'exception index.lock proposée. Désormais aucune lecture Git concurrente aux commandes gardées et lectures git --no-optional-locks. Capacité d'agents libérée en clôturant les missions documentaires terminées ; rust_implementer /root/refactor_remaining reprend T6–T9 après clôture T5, main cesse toute édition Rust parallèle.

Arbitrage index.lock : l'utilisateur a accepté « Oui, reprendre ainsi » après constat d'identité Git et disparition du verrou. Reprise limitée à cet arrêt exact acquitté dans la garde, aucun assouplissement pour de futurs événements index.lock ; auto-tests réexécutés. Les lectures Git concurrentes sont interdites entre intervenants pendant les commandes gardées.

- ID : P-REF-TESTS-DIAG, v1.0, PLANS READY documentaire.
- Revue : revue-plan-refacto-tests-diagnostics.md, CLEAN sur `05f82cc9a536350fde5360010bb8e011cb78295c` ; ajout de statut/référence = EVIDENCE_ONLY, tâches et contenu technique inchangés.
- Source audit : observations/analyse ciblée de `refacto-tests-diagnostics.md`, D-REF-TESTS-DIAGv1.0 ; aucun audit global revendiqué.
- Design parent : `d7f35143c5a4329513ba592efa0d03c3c038aa21`.
- Revue parent : `revue-refacto-tests-diagnostics.md`, CLEAN sur `197d821ef98ea8c239d62f11cd40bfe9fdb1c74e` ; commit suivant limité au statut et résultat de revue.
- Base produit : `f82063ae63b2cc075eaff54693eb62ea36a18c65`.
- Planning date : 29 octobre 2026.
- Target workspace : C:\dev\whisper, PowerShell, Windows.
- Planning mode : plan only, no code changes.
- Confidence : medium — destinations et consommateurs vérifiés, mais aucun lanceur compatible ni résultat Cargo de cette tranche établi.
- Plan quality check : auto-contrôle §10 effectué ; revue indépendante CLEAN, aucun REQUIRED ouvert.
- Auteur de proposition : rust_plan_writer, /root/plan_refacto ; écriture documentaire réservée à la session principale.
- Autorité : « fais le cadrage puis le plan de refacto avec les agents dediés » ; code, campagne et push non autorisés.

## 1. Audit Digest

Objectif : extraire les tests de quatre unités sans changer leur logique, isoler le timing commun, remplacer une liste finie de noms internes.
Quatre sources, 235 attributs #[test] lexicaux : application44/UI40/archive22/IPC129. Ce n'est pas le nombre de tests compilés par configuration. Supports cfg(test) dispersés.
Le diagnostic dans IPC possède fonction/contexte/TLS/garde/horloge. Archive l'appelle neuf fois ; model_catalog une fois, parakeet_import cinq fois. Déplacement atomique requis.
Noms historiques aussi interfaces : environnement/fichiers/formats/messages/hooks/--exact. Seuls les trente symboles DEC-T03 sont renommables.
Risques RISK-F01 à06 et RISK-T01 à05 repris sans reclassement en défauts prouvés. « Risque quasi nul » et ~12 500 lignes indicatifs.
Premier résultat fermé : extraction tests application avec noms/assertions/cfg conservés. Autres extractions séparées du diagnostic et des renommages.

| Ownership bucket | Sources actuelles | Cible et dépendances |
| --- | --- | --- |
| domaine/application | crates/whisper-core/src/application.rs | Même responsabilité ; tests dédiés et deux noms internes changés |
| adapters/exécution | crates/whisper-adapters/src/worker_ipc.rs et consommateurs | Même responsabilité ; core/archive puis diagnostics |
| adapters/stockage | crates/whisper-adapters/src/archive.rs | Même stockage ; plus de dépendance timing vers IPC |
| adapters/diagnostics | Timing aujourd'hui IPC | Nouveau diagnostics.rs, std/serde_json existants ; aucun IPC/archive/tests |
| présentation/composition existantes | crates/whisper-desktop/src/ui.rs | Dépendances core/adapters conservées |
| tests/support | Modules/items racine des quatre sources | Même crate/portée logique, fichiers séparés sans API publique |

DEC-T01 impose foo.rs + foo/tests.rs + foo/tests/*.rs, fragments include! à portée constante. Demande/AGENTS imposent ici unitaires séparés dans src : aucune migration via API publique vers intégration.
Racines production intentionnellement non-façades ; refonte hors mission. Aucun ajout de tests inline volumineux.

## 2. Freshness Check

Sources lues par plan writer : design/revue, AGENTS racine/adapters/core/desktop, manifestes workspace/trois crates, sources/consommateurs concernés.
HEAD vérifié d7f35143c5a4329513ba592efa0d03c3c038aa21, evo-startup-perf. Aucun changement suivi ; non suivis docs/architecture/, models/, sound example/ préservés.
Quatre racines/consommateurs trouvés ; diagnostics.rs nouveau prévu. Packages whisper-core/adapters/desktop et cibles bibliothèque présents.
Features adapters : recette-hooks, l01-memory-qualification, l3-gpu-qualification, personal-qualification. Desktop : recette-hooks, personal-install.
scripts/recette-build.rs refuse recette-hooks en release ; pas --all-features ni release/hooks.
Aucun brouillon antérieur de ce plan trouvé parmi fichiers suivis. Sources produit inchangées depuis base ; recontrôler ancres/consommateurs/compteurs/effets avant implémentation.
Aucun Cargo/test/build/exécutable natif/réseau lancé en planification.

## 3. Planning Assumptions

| ID | Classe | Hypothèse / prérequis et échéance |
| --- | --- | --- |
| A01 | Safe | DEC-T01 à03 référence ; pas migration contrats historiques |
| A02 | Validation needed | Ancres/compteurs inchangés ou évolution expliquée avant T1 |
| A03 | Validation needed | Variantes applicables recensées T1 sans activer recettes par environnement |
| A04 | Decision gate | Accord implémentation absent : interdit T2 àT9 et édition produit |
| A05 | Decision gate | Lanceur/racines/mandat absents : interdit Cargo/processus test et clôture exécutable T1 |
| A06 | Safe | Aucun test supplémentaire nécessaire au déplacement pur, scénarios conservés |
| A07 | Validation needed | Chaque --exact découvert une fois avant lancement ; zéro-test pas succès |
| A08 | Safe | Natifs/recettes historiques non activés, restent NOT RUN |

Contrôles statiques T1 préparent le périmètre en lecture seule ; ne remplacent pas baseline exécutable obligatoire. T2 ne commence pas si T1 partiel.

## 4. Finding Traceability

Observations/risques du design, pas nouvel audit de défauts.

| Audit finding / source | Severity | Disposition | Tâches / gate | Notes |
| --- | --- | --- | --- | --- |
| Quatre modules tests mêlés, REQ-01 à04 | IMPORTANT | Planned | T2 àT5 | Conservation complète/portée constante |
| Supports racine dispersés, T01/02 | IMPORTANT | Planned | T1 àT5 | Reliquats imbriqués explicités |
| Timing IPC partagé, REQ-06 | IMPORTANT | Planned | T6 | Atomique, six fichiers production |
| Trente noms internes, REQ-05 | IMPORTANT | Planned | T6 àT8 | DEC-T03 exhaustive |
| Identités --exact, F01 | IMPORTANT | Planned | T1/T4/T5/T8/T9 | Harness préservé |
| Logique/API, F02/T01/T02 | IMPORTANT | Planned | T2 àT9 | Diff exhaustif, pas API fixtures |
| Contrats env/formats/dossiers/messages, F04 | IMPORTANT | Planned | T6 àT9 | Littéraux identiques |
| Lanceur/racines absents, F05/T05 | IMPORTANT | Decision gate | G-EXEC,T1 | Dérogations non transposables |
| Recettes L04/natifs/plus de1Gio | IMPORTANT | Deferred | G-HIST | Pas déclenchement implicite, NOT RUN |
| ~12 500 lignes, risque quasi nul | Indicatif | Deferred | T9 | Volume réel, pas garantie performance |
| Refonte autres responsabilités | Hors mission | Deferred | §9 | Nouvelle mission |
| Revue conception | — | Already resolved | Revue parent | CLEAN documentaire |

## 5. Recommended Sequence

Un lot, neuf tâches strictement séquentielles : T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9.
Pas parallélisation de fichiers/contrats partagés. Chaque invariant fermé avant suite ; critère requis partiel arrête dépendantes.

### Phase 0 — Baseline

- Objective / Closed invariant : baseline source/exécutable comparable des variantes retenues.
- Before metrics :235 attributs source, admissibilité exécutable non établie.
- Target metrics : inventaire complet, listes/témoins avant disponibles, prérequis levés.
- Metric commands : V0 àV3,V6 àV8 selon autorisation.
- Stop conditions : mandat/garde absents, baseline échouée, effet interdit/conflit.
- Dependencies : DOCUMENT design/revue/plan approuvé ; EXECUTION G-EXEC avant Cargo.
- Expected outcome / Tasks : base comparable,T1.

### Phase 1 — Quatre extractions fermées

- Objective / Closed invariant : tests/supports déplaçables une fois à portée d'origine.
- Before metrics :44/40/22/129 attributs dans racines.
- Target metrics : zéro racines, mêmes44/40/22/129 dédiés, total235.
- Metric commands : V1 àV4,V6 àV8, ciblés par tâche.
- Stop conditions : perte/duplication/cfg/logique changé, API élargie, prérequis partiel.
- Dependencies : T1 fermé,G-CODE.
- Expected outcome / Tasks : T2 application,T3 UI,T4 archive,T5 IPC, sans renommage.

### Phase 2 — Diagnostic et noms

- Objective / Closed invariant : raccordements/noms prévus conformes DEC-T02/03 sans contrat changé.
- Before metrics : fonction/TLS/garde IPC, neuf appels archive via IPC, trente symboles anciens.
- Target metrics : un propriétaire diagnostics, zéro timing via IPC, trente correspondances closes.
- Metric commands : V3 àV8, littéraux/consommateurs.
- Stop conditions : consommateur hors budget, logique/littéral changé, TLS/horloge dupliqué, cfg incomplet.
- Dependencies : T5 fermé, vérifications précédentes réussies.
- Expected outcome / Tasks : T6 diagnostic atomique,T7 noms IPC/application/support,T8 noms archive/raccordements.

### Phase 3 — Clôture

- Objective / Closed invariant : chaque AC possède résultats attribuables et limites explicites.
- Before metrics : preuves par tranche, pas revue candidat global.
- Target metrics : AC-01 à07 couverts, périmètre exact, aucun requis partiel dissimulé.
- Metric commands : V0 àV8 selon changements, revue code indépendante autorisée.
- Stop conditions : requis non fermé, pas annoncer CLEAN/produit validé.
- Dependencies : T8 fermé, revue AGENTS.
- Expected outcome / Tasks : T9, aucun push/nettoyage implicite.

## 6. Task Backlog

Préfixes réels : A = crates/whisper-adapters/src/ ; C = crates/whisper-core/src/ ; U = crates/whisper-desktop/src/.
Règles communes : rust_implementer après accord ; main coordonne ; rust_review_coordinator/reviewer indépendant jugent candidat. Budgets plafonds, pas invitation à tout toucher. Commandes V du §7 ; arrêt sur échec/prérequis partiel. Retour ciblé de ses seuls changements après inspection, préserver diff ; jamais reset/clean global. Suivi futur dans ce plan/document de lot existant, sans rapport supplémentaire.

### T1 — Établir baseline et admissibilité

- Priority / source / rationale : High,REQ-07/tous risques ; empêcher comparaison trompeuse/exécution hors mandat.
- Targets / Ownership bucket : lecture quatre racines/supports/consommateurs ; suivi document de lot existant.
- Scope budget : zéro production modifiée, un document existant, aucun script/garde nouveau.
- Closed invariant : baseline/configuration/effet/statut connus pour chaque contrôle obligatoire.
- Before / Target metrics :235 source/admissibilité inconnue → inventaire235/supports/reliquats, listes/quatre témoins avant réussis par config applicable.
- Metric / verification commands / provenance : V0 àV3 observés/inférés ; V6/V7/V8 proposés derrière G-EXEC.
- Dependencies : DOCUMENT design/revue/plan ; EXECUTION mandat/garde/auto-tests/racines/environnement AGENTS.
- Implementation outline : Git/espace, cartographie cfg/tests/supports/effets, lecture témoins/configurations ; seulement après gate compiler/lister/témoins avant déplacement.
- Tests : W-C/W-U/W-A/W-I §7, identités exactes.
- Acceptance : baseline source/listes/témoins réussis, aucune écriture applicative hors racine, aucun transfert autorisation historique.
- Stop / rollback / notes : baseline exécutable absente = PARTIAL,T2 bloquée ; conserver constat, aucun contournement/outillage créé.

### T2 — Extraire tests application

- Priority / source / rationale : High,REQ-02/07, première tranche minimale démontrable.
- Targets : C/application.rs ; C/application/tests.rs ; C/application/tests/{closing_and_recovery,selection_and_microphone,live_lifecycle,parakeet_sessions}.rs.
- Ownership bucket / Scope budget : application/tests ; un production + cinq tests.
- Closed invariant : application::tests inchangé, scénarios une fois.
- Before / Target metrics :44 racine → zéro racine,44 dédiés ; aucun support racine supplémentaire.
- Metric / verification commands : V1/V2/V3/V4,V6/V7/V8 core ; Source inferred.
- Dependencies : T1 fermé,DOCUMENT DEC-T01,EXECUTION G-CODE/G-EXEC.
- Implementation outline : ancres design§5.3, items complets sans noms changés, include! sans logique, ordre/imports/cfg(test) conservés.
- Tests : conserver44,W-C avant/après,découverte application::tests::*.
- Acceptance : mêmes corps/assertions/cfg/noms, accès privés, pas pub nouveau, budget tenu ; racine non-façade intentionnelle.
- Stop / rollback / notes : raccordement insuffisant nécessitant autre adaptation → arrêt/retour ciblé T2.

### T3 — Extraire tests UI

- Priority / source / rationale : High,REQ-03/07, présentation isolée sans changement comportement.
- Targets : U/ui.rs ; U/ui/tests.rs ; U/ui/tests/{support_and_import,microphone_and_unified,compute_and_catalog,document_presentation,root_support}.rs.
- Ownership bucket / Scope budget : présentation/tests ; un production + six tests/support.
- Closed invariant : ui::tests et speech_limit_samples racine gardent portée.
- Before / Target metrics :40 racine → zéro racine,40 dédiés ; helper root_support inclus cfg(test).
- Metric / verification commands : V1 àV4,V6/V7/V8 desktop ; Source inferred.
- Dependencies : T2 fermé,DEC-T01,G-CODE/G-EXEC.
- Implementation outline : ancres, helper entier à portée racine, champs/initialisations/méthodes cfg imbriqués conservés/recensés.
- Tests :40,W-U,découverte ui::tests::*,aucune fenêtre.
- Acceptance : aucune politique/UI/API nouvelle, mêmes observations/assertions, budget/cfg tenus, racine non-façade intentionnelle.
- Stop / rollback / notes : test/rendu automatique pas qualification Windows ; retour ciblé T3.

### T4 — Extraire tests archive

- Priority / source / rationale : High,REQ-04/07, préserver durabilité/sélections.
- Targets : A/archive.rs ; A/archive/tests.rs ; A/archive/tests/{archive_and_contracts,recovery,campaign,replacement_and_segmented,publication_support}.rs.
- Ownership bucket / Scope budget : stockage/tests ; un production + six tests/support.
- Closed invariant : archive::tests et supports publication gardent portée/cfg.
- Before / Target metrics :22 racine → zéro racine,22 dédiés ; noms harness inchangés.
- Metric / verification commands : V1 àV4,V6/V7/V8 adapters ; Source inferred.
- Dependencies : T3 fermé,DEC-T01,G-CODE/G-EXEC.
- Implementation outline : ancres, publication_support inclus cfg(test) racine, closures cfg anciennes lignes5592/5594/5608 conservées ; vérifier --exact sans exécuter.
- Tests : W-A,discovery par variante ; recettes/scénarios disque conservés sans déclenchement automatique.
- Acceptance : constantes/fautes copie privées identiques,22 source ; l04_campaign_gate_rejects_escape_and_duplicate_trigger et filtre enfant identiques.
- Stop / rollback / notes : L04 vers C:\WhisperLive\L04-campaign\runs NOT RUN sans G-HIST ; pas modifier chemin/assertion pour contourner ; retour ciblé T4.

### T5 — Extraire tests et supports IPC

- Priority / source / rationale : High,REQ-01/07, plus gros bloc sans refonte runtime.
- Targets : A/worker_ipc.rs ; A/worker_ipc/tests.rs ; A/worker_ipc/tests/{unified_support,unified_import,unified_capture,native_execution,microphone_and_finalization,reprocessing_and_transport,resume_and_admission,stopped_ack,runtime_support}.rs.
- Ownership bucket / Scope budget : exécution/tests ; un production + dix tests/support.
- Closed invariant : une inclusion par test/support, portée logique inchangée.
- Before / Target metrics :129 racine → zéro racine,129 dédiés ; parakeet_stopped_ack_tests même niveau.
- Metric / verification commands : V1 àV4,V6/V7/V8 adapters ; Source inferred.
- Dependencies : T4 fermé,DEC-T01,G-CODE/G-EXEC.
- Implementation outline : sept fragments principaux ; stopped_ack module entier inclus racine ; runtime_support items entiers cfg(test) inclus racine ; huit CARGO_MANIFEST_DIR/attributs ignore conservés.
- Tests :129,W-I,découverte worker_ipc::tests::* et worker_ipc::parakeet_stopped_ack_tests::*.
- Acceptance : aucune assertion/fixture/champ/type/synchronisation nouvelle, reliquats enchâssés recensés, zéro item racine inexpliqué, budget tenu, runtime racine non-façade.
- Stop / rollback / notes : aucun filtre large déclenchant faux workers/natifs/plus1Gio ; retour ciblé T5.

### T6 — Extraire atomiquement le diagnostic partagé

- Priority / source / rationale : High,REQ-06/05/07,DEC-T02 ; supprimer timing archive→IPC.
- Targets : A/{diagnostics.rs,lib.rs,worker_ipc.rs,archive.rs,model_catalog.rs,parakeet_import.rs}.
- Ownership bucket / Scope budget : adapters/diagnostics, six production,zéro test nouveau. Dépassement trois justifié : propriétaire/déclaration/quatre consommateurs compilent ensemble sans relais temporaire.
- Closed invariant : un propriétaire timing autonome, comportement identique.
- Before / Target metrics : fonction/TLS/garde IPC,neuf archive via IPC → zéro via IPC,un TLS/PROCESS_START diagnostics,quatre correspondances DEC-T03 closes.
- Metric / verification commands : V3/V4/V5/V6/V7/V8 ; Source inferred.
- Dependencies : T5 fermé,DEC-T02/03,G-CODE/G-EXEC.
- Implementation outline : transférer contexte/TLS/garde/enter/Drop/fonction/horloge ensemble,mod privé,quatre noms nouveaux ; IPC+archive9+model_catalog1+parakeet_import5 raccordés.
- Tests : témoins adapters/compilation consommateurs ; comparaison exhaustive branches timing désactivé/absent/invalide/admissible,contextes absent/présent/imbriqué,Drop ; recette native non activée.
- Acceptance : mêmes prédicats/messages/littéraux/champs/stages/ordre ; fonction/garde/enter pub(crate),contexte/TLS privés ; aucune dépendance IPC/archive/tests ; budget six exact.
- Stop / rollback / notes : pas nouvelle durabilité/mutex/propagation thread ; consommateur nouveau demande portée révisée ; retour atomique six fichiers.

### T7 — Renommer symboles IPC, application et supports

- Priority / source / rationale : Medium,REQ-05/07,noms de responsabilité sans contrat changé.
- Targets : A/worker_ipc.rs,C/application.rs,A/worker_ipc/tests/resume_and_admission.rs et seulement fragments/supports référant ces symboles.
- Ownership bucket / Scope budget : application/exécution/tests,deux production + tests consommateurs directs parmi T2/T5.
- Closed invariant : dix-huit correspondances du groupe closes sans littéral changé.
- Before / Target metrics : anciens symboles présents → zéro référence symbolique ancienne dans groupe ; noms tests/littéraux historiques restent.
- Metric / verification commands : V3/V4/V6/V7/V8 ; Source inferred.
- Dependencies : T6 fermé,DEC-T03,G-CODE/G-EXEC.
- Implementation outline : neuf noms IPC,deux application,sept helpers ci-dessous ; variantes cfg/appels tous traités,comparer littéraux.
- Tests : aucun scénario renommé,helpers seuls,W-C/W-I,recherche consommateurs crates/.
- Acceptance : aucune substitution glob/migration env/trace/worker ; budget tenu,chaque ligne du groupe close.
- Stop / rollback / notes : l3c/l3d seulement paramètres précis canonisation design ; retour ciblé T7.

### T8 — Renommer symboles archive et raccordements

- Priority / source / rationale : Medium,REQ-05/04/07,noms internes explicites/hooks compatibles.
- Targets : A/{archive.rs,staging.rs,supervisor.rs,worker_ipc.rs},fragments tests archive/IPC consommant huit symboles.
- Ownership bucket / Scope budget : adapters/stockage/exécution/tests,quatre production + tests directs. Dépassement trois justifié : définitions/trois consommateurs atomiques sans alias provisoire.
- Closed invariant : huit correspondances archive closes,contrats hooks identiques.
- Before / Target metrics : huit anciens présents → zéro référence symbolique ancienne ; noms tests/env/formats conservés.
- Metric / verification commands : V3/V4/V6/V7/V8,notamment recette-hooks ; Source inferred.
- Dependencies : T7 fermé,DEC-T03,G-CODE/G-EXEC.
- Implementation outline : occurrence+sept fonctions,cfg positifs/négatifs/appels trois consommateurs ; filtres --exact/littéraux vérifiés sans campagne.
- Tests : W-A/W-I,discovery défaut/hooks,nom/logique test gate L04 conservés.
- Acceptance : zéro hook/message/env/format/destination changé,pas API nouvelle,budget quatre exact,trente correspondances globales closes.
- Stop / rollback / notes : workers homonymes intacts,G-HIST indépendant ; retour atomique T8.

### T9 — Clore les preuves et faire revoir le candidat

- Priority / source / rationale : High,REQ-07/toutes AC,résultat vérifiable/limites explicites.
- Targets / Ownership bucket : union lot.paths en lecture,document du lot,revue AGENTS.
- Scope budget : zéro nouveau changement production,document prévu,pas nouveau rapport/garde opportuniste.
- Closed invariant : périmètre/critères requis attribuables au candidat réel.
- Before / Target metrics : preuves par tâche → AC-01 à07,235 source,zéro test quatre racines,zéro reliquat inexpliqué,trente noms,diagnostic indépendant.
- Metric / verification commands : V0 àV8 utiles selon candidat,pas répétition sans changement/échec justifié.
- Dependencies : T8 fermé,CODE candidat,EXECUTION résultats requis ; main mandate rust_review_coordinator.
- Implementation outline : confronter paths/diff/inventaires/littéraux,résultats par config/témoins,revue indépendante/corrections rust_implementer,deux cycles max.
- Tests : résultats existants,lints/build appropriés ; natifs/recettes sans mandat NOT RUN,aucune fenêtre qualifiée/EXE remis.
- Acceptance : reviewer seul CLEAN,exécuté/ignoré/partiel séparés ; requis partiel interdit clôture sans réserve ; après deux cycles livrer réserves/arbitrage.
- Stop / rollback / notes : aucun commit/push produit déduit du plan ; préserver données/diff/preuves,pas nettoyage auto.

### Index des chemins et renommages

lot.paths = union exacte T2 àT8 : quatre racines,quatre agrégateurs,dix-neuf fragments principaux,quatre supports racine,diagnostics.rs/lib.rs,consommateurs model_catalog.rs/parakeet_import.rs/staging.rs/supervisor.rs. Autre chemin produit nécessite plan révisé.
Ancres/contenus design§5.3,inchangés ; destinations ci-dessus exhaustives.

| Groupe / tâche | Correspondances DEC-T03 exactes |
| --- | --- |
| Diagnostic T6,4 | record_r7_timing→record_stage_timing ; R7TimingContext→TimingContext ; R7_TIMING_CONTEXT→TIMING_CONTEXT ; R7TimingContextGuard→TimingContextGuard |
| IPC T7,9 | l2a2_acceptance_barrier→acceptance_barrier ; l2a2_acceptance_take_worker_fault→take_acceptance_worker_fault ; canonical_l3_acceptance_root→canonical_extended_acceptance_root ; l3_gpu_candidate_allowed→gpu_qualification_candidate_allowed ; l3c_execution_root→extended_execution_root ; in_l2a2→in_primary_runtime ; in_l3_test_root→in_extended_runtime ; paramètres l3c→primary_root et l3d→secondary_root |
| Application T7,2 | l3d_timing_line→record_application_timing ; l3d_event_trace→trace_application_event |
| Helpers T7,7 | compile_r7_resume_worker→compile_copy_resume_worker ; write_r7_resume_wav→write_copy_resume_wav ; r7_resume_request→copy_resume_request ; r7_hex→copy_resume_hex ; r7_generation_path→copy_resume_generation_path ; wait_r7_prompt→wait_copy_resume_prompt ; run_r7_integrated_copy_resume_case→run_integrated_copy_resume_case |
| Archive T8,8 | L04_CAMPAIGN_OCCURRENCE→CAMPAIGN_HOOK_OCCURRENCE ; l04_campaign_after→campaign_after_hook ; l04_campaign_is_after→campaign_is_after_hook ; l04_campaign_assert_archive→assert_campaign_archive ; l04_campaign_write_marker→write_campaign_marker ; l04_campaign_snapshot→campaign_archive_snapshot ; l04_campaign_operation_result→record_campaign_operation_result ; l04_campaign_gate→campaign_gate |

Reliquats cfg(test) à pointer avant/après : application,déclaration module seulement ; UI champs385–395,initialisations553–566,méthode578,observations1880/1915/1945/2001/2386/2517 ; archive closures5592/5594/5608. IPC nombreux champs/méthodes/expressions imbriqués : inventaire V2 repères696–873,1498–1987,2132–2477,2600–2971,4074–4774,5042–6883,7899/8061/8809/12626 ; retirer seulement items entiers runtime_support et module stopped_ack. Repères de base,pas frontières. Chaque reliquat V2 attribué à son item,tout oubli interdit clôture.

## 7. Verification Matrix

cwd=C:\dev\whisper,shell PowerShell. Commandes futures proposées NOT RUN,sauf lectures Git/sources §2.
Aucune ligne Cargo n'est à lancer directement : commande interne à transmettre au lanceur après G-EXEC. Invocation réelle Placeholder,aucun chemin/adaptateur compatible établi. Pas créer/contourner garde.

| ID / zone | Commande ou contrôle | Provenance | Attendu / tâches |
| --- | --- | --- | --- |
| V0 état | git status --short ; git rev-parse HEAD ; git diff --check ; git diff --name-only | Observed | Base/diff expliqués,espace avant/après,T1/T9 |
| V1 métriques | rg -n '^\s*#\[test\]' sur quatre racines puis arbres dédiés,compter Measure-Object | Source inferred |235 total,zéro racines après T5,T1 àT5/T9 |
| V2 cfg/support | rg -n '^\s*#\[cfg\(test\)\]' sur racines/arbres,lire item/contexte | Source inferred | Supports une fois,reliquats justifiés,T1 àT5/T9 |
| V3 conservation | git diff --color-moved=plain --color-moved-ws=allow-indentation-change <avant-tranche> -- <cibles-tranche> ; git show <avant-tranche>:<source> ; comparaison intégrale corps/attributs/imports/littéraux avec fragments/inclusions | Placeholder révision future,Source inferred méthode | Zéro perte logique/cfg/assertion,ordre/portée conservés,toutes tâches |
| V4 contrats | rg -n -- '--exact\|PARAKEET_\|WHISPER_\|CARGO_MANIFEST_DIR\|whisper-l04-fault/1\|\.l2a2-acceptance' <cibles> puis diff littéraux et recherche de chaque symbole ancien/nouveau dans crates/ | Source inferred | Littéraux/harness identiques,correspondances complètes,T1 àT9 |
| V5 dépendances | rg -n 'worker_ipc\|archive' crates/whisper-adapters/src/diagnostics.rs ; rg -n 'record_r7_timing\|R7_TIMING_CONTEXT\|R7TimingContext' crates/whisper-adapters/src | Source inferred | Absence attendue code1 normal,un TLS/PROCESS_START,T6/T9 |
| V6 compilation | Commandes internes ci-dessous,par package/config applicable | Source inferred / Repo docs | Compilation produit/tests utile,pas exécution large,T1 àT9 |
| V7 découverte | cargo test --offline --locked -p <package> --lib <options-feature> -- --list | Source inferred,paramètres ci-dessous | Même liste filtrée par module avant/après,découverte≠exécution |
| V8 témoins | cargo test --offline --locked -p <package> --lib <options-feature> <nom-complet> -- --exact --test-threads=1 | Source inferred,W ci-dessous | running1test,1passed,zéro ignored,T1/tranche/T9 |
| Lanceur | <invocation du lanceur existant accepté,commande interne et racines du lot> | Placeholder | G-EXEC bloquant,jamais commande directe |

V1/V2 arguments : quatre chemins complets §1 et leurs seuls arbres dédiés §6. Vérifier inclusion réelle : fichier orphelin pas preuve test présent.
V3 : lecture exhaustive items,pas seules lignes colorées/total235. Fichier/ligne panic déplacés,messages conservés.

Commandes internes V6 :

- Format : `cargo fmt --all -- --check` ; préexistants signalés,pas correction globale.
- Produit : `cargo build --offline --locked -p whisper-core -p whisper-adapters -p whisper-desktop --lib`.
- V7 compile/liste harness,pas --no-run redondant.
- Lints : `cargo clippy --offline --locked -p whisper-core -p whisper-adapters -p whisper-desktop --lib -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity`.
- Warnings historiques attribués,pas refonte ; aucun manifest/lock modifié/téléchargement.

Configurations avant/après : core défaut ; adapters défaut,`--features recette-hooks`,`--features recette-hooks,l3-gpu-qualification`,`--features l01-memory-qualification`,`--features personal-qualification` ; desktop défaut,`--features recette-hooks`,`--features personal-install`.
V6/V7 variantes seulement package applicable via -p/--lib,pas feature core inexistante. T1 établit listes avant utiles ; répétitions limitées package/config affecté et fermeture.
Pas release/hooks,recette/natif activé par variable ; mandat précise variables ambiantes à neutraliser/conserver sans changer contrats.

Témoins réellement lus,identités conservées :

| ID | Package / nom complet pour V8 | Données,observation,effets du corps |
| --- | --- | --- |
| W-C | whisper-core / application::tests::repeated_start_is_identifiable_and_stale_stop_does_not_target_new_generation | Commandes/identités mémoire,start égal et refus génération ancienne,pas I/O corps |
| W-U | whisper-desktop / ui::tests::auto_stop_is_disabled_by_default_and_uses_speech_sample_clock_when_enabled | bool/minutes,None/1_920_000/960_000,calcul pur,pas fenêtre |
| W-A | whisper-adapters / archive::tests::bounded_resume_accepts_a_current_backend_for_the_same_model_profile | Clés CPU/GPU mémoire,même modèle/profil accepté,altérations refusées,pas I/O corps |
| W-I | whisper-adapters / worker_ipc::parakeet_stopped_ack_tests::stopped_ack_waits_for_exit_then_allows_completion | None/Some(true)/Some(false)→Wait/Success/Failure,calcul pur |

Ces témoins ne remplacent pas comparaison exhaustive235. Compilation soumise G-EXEC même si corps sans disque/processus. Présence unique V7 avant chaque V8,zéro-test arrête.

### Carte d'exécutabilité des scénarios obligatoires

Owner futur rust_implementer,main suit mandat,reviewer juge constats. Coût/durée non mesurés : lecture bornée aux cibles,compilation dépend cache ; aucun budget natif long engagé.

| AC / tâches | Chemins directs/indirects et données | Observation / résultat | Prérequis / limites |
| --- | --- | --- | --- |
| AC-01,T1/T5/T9 | IPC+9fragments/supports+agrégateur,source baseline/listes variantes | V1 àV4 exhaustifs,129 uniques/noms identiques,W-I1pass | G-CODE/G-EXEC,faux workers/natifs/plus1Gio NOT RUN |
| AC-02,T1/T2/T7/T9 | Application+4fragments+agrégateur,commandes/faux ports existants |44,mêmes assertions/transitions,W-C1pass,deux noms seulement | G-CODE/G-EXEC,pas adaptateur pour témoin |
| AC-03,T1/T3/T9 | UI+4fragments+root_support+agrégateur,états existants |40,privé/cfg identiques,W-U1pass | G-CODE/G-EXEC,pas preuve rendu Windows |
| AC-04,T1/T4/T8/T9 | Archive+4fragments+publication_support+agrégateur,filtres --exact |22,noms/filtre/cfg conservés,W-A1pass,découverte conditionnelle | G-CODE/G-EXEC,campagne enfant interdite sans G-HIST |
| AC-05,T6 àT9 |30symboles,model_catalog/parakeet_import/staging/supervisor/IPC/tests | Table close,cfg complets,littéraux identiques | V3/V4+compilation variantes,pas workers renommés |
| AC-06,T6/T9 | Diagnostics/lib+4consommateurs,fonction/TLS/garde/horloge source | Comparaison exhaustive activation/refus/append/contextes/Drop,propriétaire unique,compilation/témoins | Recette timing NOT RUN sans mandat,comparaison non fidèle bloque T6 |
| AC-07,T1 àT9 | Union lot.paths,manifest/lock lus,inventaires/listes/résultats |235,inclusion unique,listes identiques par config,statuts explicites | Requis partiel/échoué arrête dépendantes,natif/Windows non revendiqués |

AC-06 conservation par transfert intégral comparé,aucun changement de contrôle ; aucune branche I/O prétendue exercée. Adaptation fonctionnelle découverte : arrêt T6,réviser design/validation.

## 8. Decision Gates

| Gate | Status | Blocks | Question / autorité | Recommended default | How to validate |
| --- | --- | --- | --- | --- | --- |
| G-CODE | Blocking | T2 àT9,édition produit | Accord précis lot/périmètre,utilisateur | Attendre,pas déduire du plan | Citation session/chemins/lot |
| G-EXEC | Blocking | Cargo/processus test,clôture T1/donc suite | Mandat/lanceur conforme tranche,utilisateur pour accord/écart | Aucun lancement/transfert dérogation | Lanceur réel,auto-tests/préflight/garde/racines/variables/surveillance AGENTS,≥30Gio,Placeholder remplacé |
| G-BASE | Validation | Fermeture T1 et tâche consommatrice | Sources/cfg/noms/effets conformes,implémenteur | Vérifier | V0 àV4 puis baseline V6 àV8 |
| G-HIST | Blocking | Recettes/natifs/hors racines historiques uniquement | Mission distincte éventuelle,utilisateur | NOT RUN | Mandat/racines/données/garde compatibles |
| G-REVIEW | Validation | Clôture sans réserve candidat | Reviewer indépendant AGENTS | Deux cycles max | Verdict/constats reliés diff |

G-HIST ne bloque pas déplacements conservateurs et contrôles obligatoires,mais bloque recettes historiques. Si recette indispensable pour résultat requis,critère reste partiel/interdit clôture correspondante.

## 9. Out of Scope

Ne pas modifier logique produit/test,assertions/scénarios,synchronisation,ACK/PublicationGate/intégrité,env/protocoles/formats/dossiers/messages/hooks,noms harness/--exact,manifests/lock,routage/installations/workers.
Pas API publique fixture,crate/couche/trait,garde/lanceur/script,isolation/service/tâche/ACL,CI,manifeste qualification,fichier dette/nouveau chantier documentaire.
Pas refonte façades/écarts structure historiques,fenêtre automatisée,recette native nouvelle,téléchargement,nettoyage target/cache/données,push.

## 10. Final Self-Check

| Check | Result | Notes |
| --- | --- | --- |
| Plan réel,pas placeholder global | Pass |9 tâches,lanceur explicitement bloquant |
| Sections template dans ordre | Pass | Header puis1à11 |
| Observations/risques→tâches/gates/différés | Pass | §4/carteAC |
| Cibles/deps/outline/tests/contrôles/critères/retour | Pass | T1 àT9/règles communes |
| Invariants/métriques/commandes/arrêts | Pass | §5/6 |
| Scope budgets/dépassements justifiés | Pass | Diagnostics6,archive4 atomiques |
| Tests dédiés/architecture justifiée | Pass | Unitaires src demandés,aucune API intégration |
| Commandes/provenance | Pass | V0àV8,arguments futurs/lanceur identifiés |
| Gates bornés,autorisation distincte | Pass | G-CODE/G-EXEC bloquent actions,pas revue docs |
| Chemins réels ou design | Pass | Union lot.paths/destinations finies |
| Première tranche fermée/ordre sans cycle | Pass | T2 application,séquentiel |
| Readiness/exécution distinguées | Pass | Pas CLEAN/PLANS READY auto-déclaré |

Auto-contrôle de proposition ; reviewer seul verdict. Toutes validations produit NOT RUN.

## 11. Implementation Agent Prompt

Après accord écrit précis utilisateur,agir IMPLEMENT dans C:\dev\whisper pour P-REF-TESTS-DIAG selon ce plan/design/revues Git. T1 seulement au départ : sources/Git/responsabilités concurrentes/espace/périmètre/prérequis campagne. Aucun Cargo/processus test avant G-EXEC/invocation réelle lanceur ; T2 seulement si baseline T1 complète/réussie. Puis T2→T9,tranches fermées,V0àV8/budgets exacts. Préserver autrui/non-suivis,235tests/noms/cfg/assertions/contrats. Renommages seulement trente DEC-T03. Aucune API/dépendance/logique/garde/campagne native/fenêtre automatisée nouvelle. Arrêt si partiel/perte/duplication/logique/contrat/conflit/écriture interdite. Rendre diff/résultats/NOT RUN ; main mandate revue indépendante AGENTS. Ce prompt/plan/verdicts ne valent accord implémentation/publication.

## 12. Bilan d'implémentation et revue du candidat

Date : 29 octobre 2026. Candidat : arbre de travail sur evo-startup-perf, base c355414c1096c1669639dc926bc0257a8ce14a40. Le bilan initial précédait la publication. La correction EOF décrite ci-dessous a fait l'objet d'une confirmation indépendante ; aucun changement Rust depuis ce dernier verdict.
Responsabilités : session principale pour G0/T1 et les extractions T2–T5 sous exception de responsabilité autorisée ; rust_implementer /root/refactor_remaining pour T6–T8 et contrôles finaux T9, sans édition Rust parallèle. L'agent était de nouveau disponible après clôture des missions documentaires terminées.

| Tâche | Résultat et preuve |
| --- | --- |
| G0 | Garde adaptée et auto-tests PASS, avec seuls arbitrages de session ci-dessus ; aucune suppression de STOP historique ni transfert implicite de dérogation |
| T1 | Baselines neuf configurations et quatorze exécutions de témoins PASS ; correction préalable des trois imports redondants autorisée et isolée |
| T2 | 44 tests application déplacés, listes et témoin conservés |
| T3 | 40 tests UI et helper racine déplacés, trois variantes conservées |
| T4 | 22 tests archive et supports de publication déplacés, filtres --exact inchangés |
| T5 | 129 tests IPC et supports déplacés, dont module stopped_ack à portée constante ; cinq variantes conservées |
| T6 | Fonction/contexte/TLS/garde/Drop/horloge dans diagnostics privé ; consommateurs raccordés, aucun relais IPC ni API publique ajouté |
| T7 | Dix-huit correspondances internes IPC/application/supports, sans remplacement des littéraux ou noms de tests |
| T8 | Huit correspondances archive et raccordements atomiques dans les consommateurs prévus |
| T9 | Format/build/Clippy et comparaisons finales PASS ; revue indépendante CLEAN, aucun correctif demandé |

### Fichiers et conservation

37 fichiers Rust du plan : neuf fichiers suivis modifiés et 28 nouveaux (diagnostics.rs et 27 fichiers de tests/supports/agrégateurs). Le seul document modifié est ce plan/bilan. Cargo.toml, Cargo.lock, AGENTS et autres fonctions hors lot sont préservés. Les non-suivis préexistants docs/architecture/, models/ et sound example/ restent présents.

| Fichier | Lignes avant | Lignes après |
| --- | ---: | ---: |
| crates/whisper-core/src/application.rs | 7 550 | 4 517 |
| crates/whisper-desktop/src/ui.rs | 6 195 | 3 869 |
| crates/whisper-adapters/src/archive.rs | 10 092 | 7 914 |
| crates/whisper-adapters/src/worker_ipc.rs | 29 509 | 13 214 |

Il s'agit des tailles des fichiers, pas d'une mesure de performance. Les 235 attributs de test sont dans les fichiers dédiés, zéro dans les quatre racines. Les éléments cfg(test) enchâssés dans des types/impls mixtes restent intentionnellement en place. Les fragments conservent la portée logique et les chaînes brutes des faux workers.
Le reviewer a reconstruit les quatre modules et comparé leurs tokens Rust au baseline après inversion des seuls renommages DEC-T03 : 44 application, 40 UI, 22 archive, 128 IPC principal et un stopped_ack. Il a aussi confirmé les supports et les 6 308 littéraux identiques sur les racines/consommateurs/diagnostic. Contexte, garde, restauration, fonction et horloge de diagnostic conservés ; un PROCESS_START, 17 appels qualifiés, aucun ancien symbole Rust ciblé. Les noms historiques préservés ne sont pas des reliquats à renommer.

### Vérifications finales réellement exécutées

PowerShell, cwd C:\dev\whisper, toolchain 1.98.1-x86_64-pc-windows-msvc. Toutes les commandes Cargo ont été exécutées par la garde adaptée, avec cache/TEMP/TMP privés, cible partagée, --offline --locked quand accepté, incremental=0 et debug=line-tables-only.

| Contrôle | Résultat | Journal sous target/parakeet-tests/refacto-tests-diagnostics/logs/ |
| --- | --- | --- |
| cargo fmt --all -- --check | Exit 0 ; seules quatre sorties de fichiers du lot avaient été appliquées avant le contrôle, aucune correction globale hors périmètre | 0bf73376fa874ec39d135808324528ea |
| cargo build --offline --locked -p whisper-core -p whisper-adapters -p whisper-desktop --lib | Exit 0, bibliothèques compilées | 68feef963a2348daaf0e79358d5a1065 |
| cargo clippy --offline --locked -p whisper-core -p whisper-adapters -p whisper-desktop --lib avec les quatre lints structurels du §7 | Exit 0 avec avertissements | 35a2853fa74d4ee682090633bb0383e3 |
| Listes compilées finales | Neuf listes strictement identiques aux baselines : core80, adapters282/hooks286/hooks+gpu287/memory282/personal282, desktop44 dans chacune des trois variantes | baseline-*.txt, after-*.txt et journaux stdout correspondants |
| Témoins W-C/W-U/W-A/W-I | Quatre tests distincts, quatorze invocations selon configuration ; chacune 1 passed, zéro ignored | Journaux stdout des commandes exactes |
| git --no-optional-locks diff --check | PASS | Contrôle local final |

Clippy résume 23 avertissements core, 142 adapters et 17 desktop, soit 182 (185 lignes warning avec les trois synthèses). Aucune baseline Clippy comparable ne permet d'affirmer leur antériorité complète. Aucun avertissement diagnostics.rs repéré. Ils ne sont pas masqués et ne déclenchent aucune refonte hors lot.

### Revue indépendante — résultat brut utile

Coordinateur : rust_review_coordinator /root/review_refactor, sans édition ni revue du code par lui-même. Reviewer : rust_reviewer /root/review_refactor/independent_review, indépendant des auteurs, en lecture seule. Une revue, zéro correction, zéro demande VERIFY supplémentaire. Le reviewer n'a lancé aucun Cargo.

Verdict : CLEAN. Objet : arbre de travail sur evo-startup-perf, base c355414c1096c1669639dc926bc0257a8ce14a40, P-REF-TESTS-DIAG union T2–T8 (37 fichiers Rust). Findings : aucun. Constats ouverts : aucun. Demandes de vérification et gaps requis : aucun. Blocage : aucun.
Il confirme modules/supports/cfg/harness, transfert du diagnostic, conservation des littéraux, neuf listes identiques, quatorze témoins réussis, rapprochement des preuves format/build/Clippy/diff-check avec le candidat, y compris les nouveaux fichiers.

### Limites, preuves et clôture

NOT RUN : autres tests au-delà des témoins ciblés, recettes historiques supplémentaires, qualifications natives et fenêtre Windows. Aucun EXE ou dossier de recette livré ; la compilation vérifiée porte sur les bibliothèques. Ce bilan ne revendique ni campagne native ni validation fonctionnelle Windows.
La garde consigne 1 350 événements Temp (auto-tests inclus), sans PID ni attribution démontrée ; les trois événements du journal .git/objects sont synthétiques. Ces observations ne prouvent aucune écriture Cargo et ne deviennent pas des destinations autorisées. Les seuls incidents ayant imposé arrêt/reprise et leurs arbitrages sont conservés ci-dessus et dans les journaux.

Petites preuves archivées sans supprimer les originaux, caches ou copies : target/parakeet-tests/refacto-tests-diagnostics/archives/refacto-tests-diagnostics-evidence-20261029.zip. Arborescence conservée, 305 entrées vérifiées par nombre et SHA-256 de chacune contre l'original, sans manifeste séparé. SHA-256 de l'archive : DD58BBA259E9F8CF855056BBC79A621B0AAB0B43B1CEC15239E97C0B8F1A12B2. Les gros snapshots Git et sorties dépassant 2 Mio restent disponibles dans la racine brute sans duplication dans cette archive.
Espace libre avant : environ98,7Gio ; après clôture/archivage : 101882564608 octets, environ94,9Gio, au-dessus du seuil30Gio. Aucun nettoyage cargo-shared, modèle, corpus, WAV ou fichier temporaire externe effectué.

### Préparation du commit et du push

L'utilisateur a ensuite demandé explicitement « commit et push ». Périmètre de publication : 37 fichiers Rust et ce bilan, sur evo-startup-perf ; les six commits documentaires locaux du cadrage/plan font aussi partie de cette publication. Les non-suivis préexistants docs/architecture/, models/ et sound example/ sont exclus.
Le contrôle complet de l'index a signalé EOF-GIT-CHECK : quinze nouveaux fragments avaient un LF superflu à EOF, non couvert par le précédent diff-check des seuls fichiers suivis. rust_implementer /root/refactor_remaining a retiré uniquement ce dernier LF de chacun des quinze fichiers, sans modifier le préfixe ni l'index. Aucun Cargo relancé pour cette correction sans changement de tokens.
Le même reviewer /root/review_refactor/independent_review a confirmé indépendamment working_tree == index[:-1] == index.rstrip(CR/LF)+LF pour les quinze chemins, a fermé EOF-GIT-CHECK et maintenu CLEAN. Les conclusions structurelles et les 6 308 littéraux sont inchangés ; les vérifications fonctionnelles antérieures restent applicables. Bilan de revue final : deux passages, une correction, zéro demande VERIFY, aucun finding ouvert. L'archive de preuves ci-dessus précède cette correction de présentation, sans changement de sa valeur fonctionnelle. Le contrôle de l'index est à répéter après réindexation avant commit, puis synchronisation locale/suivi/distant à vérifier après push.
