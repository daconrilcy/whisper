Avancée : plan de nettoyage établi à partir de l'audit des sources courantes.
Blocage : aucun pour la proposition ; l'implémentation et le retrait des parcours historiques ne sont pas autorisés.
Décision : recommander uniquement la phase 1 comme prochain lot ; préserver archives, attestation GPU et workers installés.

## Header

- Source audit : [audit-dependances-environnements-historiques.md](audit-dependances-environnements-historiques.md), EH-01 à EH-06.
- Planning date : 31 octobre 2026.
- Target workspace : `C:\dev\whisper` ; HEAD `998a732a33a7db7992f9c751cd29bdb029ef7456` et correctifs locaux décrits dans l'audit.
- Planning mode : plan only, no code changes.
- Confidence : **medium** ; les branches et contrats ont été lus, les nouveaux scénarios d'isolation n'ont pas encore été exécutés.
- Plan quality check : passed pour la structure et la traçabilité ; aucune revue indépendante de ce plan ni autorisation de coder implicite.

## 1. Audit Digest

Deux risques fonctionnels conditionnels sont prioritaires : configuration L3C/L3D des journaux GPU sélectionnée avant le parcours personnel (EH-01), et erreurs fatales d'un journal de mesure R6 hérité dans les workers (EH-02). Les parcours historiques et personnels restent imbriqués (EH-03). Les diagnostics facultatifs ont des protections hétérogènes (EH-04). Certaines identités de test sont aussi des contrats d'archives et de GPU à conserver (EH-05). Des tests historiques retournent prématurément sans environnement de campagne (EH-06).

Le prochain lot ferme l'indépendance du **lancement personnel** ; il n'annonce pas la disparition de toute trace de campagne dans tout le produit. La suite sépare la simplification structurelle, la décision sur les diagnostics et une éventuelle reconstruction/requalification des workers. Aucun nettoyage du disque ni création des anciens dossiers.

## 2. Freshness Check

Sources et manifestes inspectés : desktop `root.rs`, adapters `parakeet_import.rs`, `parakeet_live/trace.rs`, `diagnostics.rs`, `archive/{identity,import,recovery}.rs`, worker Parakeet `runtime_paths.rs`, `startup.rs`, `native_engine.rs`, `gpu_identity.rs`, `timing.rs`, core `model_registry.rs` et `application/diagnostics.rs`. Les modules existent. `parakeet_runtime_isolation.rs` et `historical_runtime.rs` ci-dessous sont des fichiers proposés, pas des capacités existantes.

L'audit a utilisé `rg`, lectures des sources/manifestes, état Git et inventaire statique sans Cargo. Les références de la carte `docs/architecture/` ont été confrontées aux sources ; la carte est un relevé ancien, non un mandat. Aucun plan de nettoyage de ces dépendances trouvé dans `docs/cadrage/` ; les bilans de nettoyage de `target` concernent le disque, pas ce changement produit.

À réactualiser avant tout code : état des quatre fichiers locaux corrigés (dont un fichier de tests nouveau), branche/HEAD, présence de ressources, usages encore actifs des parcours Test et de leurs scripts. Les correctifs de cette conversation ne doivent pas être écrasés ni absorbés dans un commit documentaire. Aucun Cargo n'a été exécuté pour rédiger ce plan.

## 3. Planning Assumptions

- **Safe** : les noms `PARAKEET_L2_RUNTIME_ROOT` et `PARAKEET_L3_ASR_LOG` restent des contrats parent/worker ; leur ancien nom ne justifie pas une migration.
- **Safe** : préserver les identités SHA, les clés persistantes, les refus de reprise L2a/R5-2, les bornes des files et la preuve d'usage GPU.
- **Validation needed** : les deux tests L3b locaux et le test de recyclage doivent rester présents ; leurs anciennes preuves sont distinctes de cette future campagne.
- **Decision gate** : aucun code avant acceptation du lot, de ses chemins et de sa racine de campagne dans la session d'implémentation (G0).
- **Decision gate** : retrait de routes Test ou diagnostics R6/R7 soumis à G1/G2 ; ne pas déduire leur inutilité de l'absence des dossiers sur cette machine.
- **Validation needed** : garde existante applicable à la racine autorisée, sans adaptation automatique ni nouveau script de surveillance.

Carte d'architecture cible : desktop choisit toujours le profil ; adapters choisit le parcours et construit les arguments/environnement des processus ; workers valident et exécutent ; core conserve politiques et identités ; `tests/` porte les scénarios de lancement. Aucune dépendance nouvelle. Les helpers privés de compatibilité ne dépendent ni du desktop ni des tests et n'introduisent aucune écriture dans les anciens répertoires.

Convention : `parakeet_import.rs` plus `parakeet_import/*.rs`, déclarations `mod` idiomatiques et visibilité privée. Le fichier racine reste intentionnellement responsable du client IPC et du lancement : ce plan n'autorise pas sa conversion complète en façade. Toute extension de cette extraction exige un autre périmètre ; ne pas élargir les API pour simplifier les tests.

## 4. Finding Traceability

| Audit finding | Severity | Disposition | Task IDs / gate | Notes |
| --- | --- | --- | --- | --- |
| EH-01 | High / IMPORTANT | Planned | P1-T1 | Choix personnel avant racines de journaux historiques. |
| EH-02 | High / IMPORTANT | Planned | P1-T1, P3-T2 | Neutralisation à la frontière enfant, puis éventuel retrait dans le worker. |
| EH-03 | Medium / IMPORTANT | Planned | P2-T1, G1 | Isoler les chemins ; conserver les profils Test par défaut. |
| EH-04 | Low / MINEUR | Decision gate | G2, P3-T1, P3-T2 | Nettoyage R6/R7/trace après décision. Trace L3D du cœur différée à un lot diagnostic dédié. |
| EH-05 | Medium / IMPORTANT | Deferred | G1, P3-T3 | Pas de suppression des clés persistantes ni des preuves GPU ; seule mise à jour atomique des identités nouvelles si workers reconstruits. |
| EH-06 | Medium / IMPORTANT | Planned | P1-T1 | Nouvelle couverture autonome ; migration générale des anciens tests différée. |

## 5. Recommended Sequence

### Phase 1 : rendre le lancement personnel indépendant des variables historiques

- Objective : fermer EH-01/EH-02 au point de lancement sans reconstruire les workers figés.
- Closed invariant : variables de campagne héritées sans effet sur le worker choisi, sa racine effective et son journal ASR obligatoire dans le parcours personnel.
- Before metrics : deux branches d'influence confirmées par lecture ; zéro scénario autonome de pollution d'environnement couvrant ensemble ces deux branches.
- Target metrics : zéro consultation des racines L3C/L3D par la sélection personnelle du journal ; variables R6/R7 non héritées par ses enfants ; matrice CPU/GPU, import et lancement de session exercée sans L2/L3 historiques.
- Metric commands : V1 et V2 ; lecture du routage et du constructeur de commande.
- Stop conditions : celles de §7 ; ne pas commencer la phase 2 si un critère de phase 1 reste partiel.
- Dependencies : G0.
- Expected outcome : risque de chemins de campagne réduit au lancement, sans changement d'identité des workers.
- Tasks : P1-T1.

### Phase 2 : rendre la compatibilité historique identifiable

- Objective : sortir les exceptions L2/L3/R6 du routage personnel.
- Closed invariant : toutes les résolutions de racines historiques du client sont possédées par un module privé de compatibilité et sélectionnées après le profil personnel.
- Before metrics : exceptions réparties dans validation, qualification, journaux et helper R6 du fichier racine.
- Target metrics : zéro construction de chemin `target/parakeet-tests/l2` ou `l3` dans les fonctions communes/personnelles extraites ; exceptions historiques regroupées et comportements préservés.
- Metric commands : V0, V2 et V3 ; lecture des frontières et visibilité.
- Stop conditions : API publique à élargir, format d'archive à changer ou plus de trois fichiers de production nécessaires à cette tâche.
- Dependencies : phase 1 acceptée et vérifiée, G1.
- Expected outcome : frontière lisible, aucun retrait de route.
- Tasks : P2-T1.

### Phase 3 : retirer ou isoler les diagnostics devenus inutiles

- Objective : supprimer la dépendance de mesure historique retenue comme obsolète, sans supprimer l'attestation GPU.
- Closed invariant : release personnelle retenue sans lecture ni écriture de mesure de campagne R6/R7/ancienne trace micro ; journal ASR de preuve conservé.
- Before metrics : trois familles de diagnostics dans adapters/workers (R6, R7, trace micro), plus une trace L3D du cœur explicitement différée.
- Target metrics : zéro accès de ces trois familles dans la release personnelle finale ; diagnostic L3D du cœur inchangé tant qu'un lot distinct n'est pas autorisé.
- Metric commands : V0, V2, V4, V5 ; comparer les binaires/identités réellement remis.
- Stop conditions : G2/G3 non résolus, worker nouveau non qualifié, périmètre de distribution non autorisé.
- Dependencies : phases précédentes ; décision diagnostic puis requalification explicite.
- Expected outcome : suppression choisie, cohérente avec le routage et les identités enregistrées.
- Tasks : P3-T1, P3-T2, P3-T3. P3-T2/T3 forment un même candidat avant les vérifications d'admission : ne pas livrer l'état intermédiaire aux identités périmées.

## 6. Task Backlog

### P1-T1 : isoler l'environnement des lancements personnels

- Priority : High.
- Source finding : EH-01, EH-02, EH-06.
- Rationale : corriger les deux influences fonctionnelles sans changement des workers installés.
- Target files/modules : `crates/whisper-adapters/src/parakeet_import.rs`, nouveau `crates/whisper-adapters/tests/parakeet_runtime_isolation.rs`. Le filtrage de l'environnement reste une fonction privée locale au module de lancement ; pas de nouveau module de production en phase 1.
- Ownership bucket : infra/adapters ; tests d'intégration de lancement dans `tests/`.
- Scope budget : un fichier de production, plus le fichier de tests et support directement nécessaire. Aucun manifeste ou worker modifié.
- Closed invariant : celui de phase 1.
- Before metrics / Target metrics / Metric commands : phase 1, V1/V2 ; chaque cas doit atteindre réellement la frontière de lancement.
- Stop conditions : test préalable requis en échec, changement de contrat GPU/archives, nouvelle racine non autorisée, dépassement de budget.
- Dependencies : G0.
- Implementation outline :
  1. Choisir le parcours personnel avant de lire les variables L3C/L3D dans la journalisation. Garder les refus des parcours historiques.
  2. Pour les enfants personnels seulement, retirer explicitement `PARAKEET_R6_TIMING_LOG`, `PARAKEET_R6_CASE`, les trois variables `PARAKEET_R7_TIMING_ACTIVE`, `PARAKEET_R7_EXEC_ROOT`, `PARAKEET_R7_TIMING_LOG` et les racines de campagne L3C/L3D de l'environnement hérité ; vérifier les consommateurs avant de figer cette liste.
  3. Toujours fournir explicitement `PARAKEET_L2_RUNTIME_ROOT` et, pour le GPU, `PARAKEET_L3_ASR_LOG`. Préserver `LOCALAPPDATA`, les ressources installées, CUDA, l'identité et le profil demandés.
  4. Tester par sous-processus afin d'éviter les variables globales mutables ; absence des véritables dossiers L2/L3, valeurs héritées invalides et valeurs simultanées L3C/L3D. Faux worker Rust observant arguments/environnement et échange IPC minimal, sans simuler une réussite native.
- Tests to add or update : CPU et GPU personnels ; `spawn` sans journal de session et parcours de lancement avec journal de session ; environnement propre/pollué ; commande enfant sans variables de mesure ; journal produit dans sa racine ; route historique refusant toujours chemins et identités incorrects.
- Verification commands : V1, V2, V4, puis V5 pour la recette autorisée.
- Command provenance : V1 Source inferred (nouveau test), V2/V4 Repo docs et Source inferred, V5 Repo docs.
- Acceptance criteria : zéro cas sautant silencieusement ; mêmes chemins produit et choix CPU/GPU avec ou sans pollution ; absence L3b réelle conservée ; pas de mécanisme de suppression d'environnement appliqué indistinctement aux campagnes de qualification ; SHA/attestation/archives inchangés ; budget respecté.
- Rollback/fallback notes : revenir uniquement sur le delta de cette tâche après état antérieur vérifié ; conserver preuves et correctifs locaux précédents. Aucun changement de réglages Windows ni recréation L3b.
- Notes for implementation agent : ne pas confondre `spawn_live` déjà isolé pour certains journaux avec `spawn` ; ne pas exécuter volontairement un test rouge sous la règle d'arrêt de campagne. Ajouter le test et le correctif avant la première exécution autorisée.

### P2-T1 : extraire la compatibilité des racines historiques

- Priority : Medium.
- Source finding : EH-03 ; protection EH-05.
- Rationale : rendre explicite la frontière qui a causé le défaut L3b.
- Target files/modules : `parakeet_import.rs`, nouveau `parakeet_import/historical_runtime.rs`, tests `tests/parakeet_runtime_isolation.rs` et `tests/parakeet_import.rs`.
- Ownership bucket : infra/adapters ; compatibilité historique privée.
- Scope budget : deux fichiers de production plus tests correspondants ; pas de nouveau `pub` externe.
- Closed invariant / Before metrics / Target metrics : ceux de phase 2.
- Metric commands : V0/V3 et revue manuelle des responsabilités.
- Stop conditions : mêmes que phase 2 ; ne pas absorber le transport IPC ni tous les tests du gros fichier.
- Dependencies : P1-T1, G1.
- Implementation outline : déplacer les exceptions de chemins L2/L3 et l'adaptation CPU R6 ; passer les données nécessaires explicitement ; maintenir la sélection personnelle en amont et les noms de protocole existants. Vérifier les méthodes de qualification et de journalisation, pas seulement `validate`.
- Tests to add or update : conserver les tests L3b de cette conversation, couverture personnelle P1, refus historiques ; scenarios publics sous `tests/`. Ne pas ajouter de masse inline ; tests privés existants différés, avec appels adaptés uniquement si requis.
- Verification commands : V0, V2, V3, V4.
- Command provenance : Observed pour les recherches V0 ; Repo docs/Source inferred pour la validation future.
- Acceptance criteria : chemin personnel sans résolution de campagne ; module historique sans dépendance vers tests/desktop ; noms et signatures publiques inchangés ; racine `parakeet_import.rs` encore non-façade intentionnelle mais sans exception de chemin historique dispersée ; budget respecté.
- Rollback/fallback notes : abandonner l'extraction entière si elle oblige une modification de protocole ; conserver la phase 1 fonctionnelle.
- Notes for implementation agent : extraction de responsabilité seulement ; aucune feature nouvelle et aucun retrait d'anciens workers.

### P3-T1 : nettoyer les diagnostics historiques des adaptateurs

- Priority : Low.
- Source finding : EH-04.
- Rationale : retirer les chemins de campagne et les coûts optionnels des diagnostics jugés inutiles.
- Target files/modules : `crates/whisper-adapters/src/parakeet_live.rs`, `src/parakeet_live/trace.rs`, `src/diagnostics.rs` ; tests correspondants sous `tests/` si boundary publique, petits tests privés séparés seulement si l'invariant est inaccessible sans élargir l'API.
- Ownership bucket : diagnostics infra/adapters.
- Scope budget : trois fichiers de production plus tests correspondants ; aucune nouvelle dépendance.
- Closed invariant : zéro lecture/écriture R7/ancienne trace micro dans la release personnelle selon la décision G2 ; D2 `cfg(test)` préservé.
- Before metrics : trace micro à racine absolue et mesures R7 facultatives compilées dans les adaptateurs.
- Target metrics : zéro accès aux anciennes destinations depuis le produit personnel, attestation ASR intacte.
- Metric commands : V0, V2, V4 ; contrôle des appels et imports résiduels.
- Stop conditions : besoin d'un nouveau produit de diagnostic ou d'une feature non acceptée ; ne pas inventer une API de télémétrie.
- Dependencies : P2-T1, G2.
- Implementation outline : appliquer le retrait décidé ou proposer le périmètre révisé si diagnostic conservé ; supprimer les appels devenus sans objet, garder les bornes si une trace produit est explicitement conservée ; ne pas modifier la capture ni la confirmation.
- Tests to add or update : activation des anciennes variables sans effet de fichier pour le produit ; comportements audio identiques ; contrats de traces conservées vérifiés séparément.
- Verification commands : V0, V2, V4.
- Command provenance : Observed pour recherche ; Repo docs/Source inferred pour futures commandes.
- Acceptance criteria : invariant atteint sans nouvelle écriture implicite ; zéro nouvelle masse inline ; budget respecté.
- Rollback/fallback notes : différer si utilité diagnostic non tranchée ; P1 reste suffisante pour protéger le lancement.
- Notes for implementation agent : trace L3D dans le cœur explicitement hors tâche ; pas de renommage global de variables.

### P3-T2 : nettoyer les mesures des workers après décision

- Priority : Medium.
- Source finding : EH-02, EH-04.
- Rationale : supprimer à la source le journal R6 fatal et les mesures R7 si leur retrait est accepté.
- Target files/modules : sous `crates/whisper-worker-parakeet-cpu/src/`, `native_engine.rs`, `timing.rs`, `main.rs`, `integrity.rs`, `startup.rs`, `transcription.rs`. Ces six fichiers sont les consommateurs et la définition R7 observés par recherche.
- Ownership bucket : worker, exécution/diagnostic.
- Scope budget : exception explicite de six fichiers de production plus tests, à accepter à G3. Justification : retirer le module R7 et sa réexportation requiert de retirer simultanément tous ses imports et appels pour conserver un candidat compilable ; conserver des relais sans effet uniquement pour fractionner cette suppression ajouterait une transition inutile. Aucun septième fichier de production ouvert par cette tâche.
- Closed invariant : zéro dépendance R6/R7 de mesure dans les workers personnels reconstruits ; preuves d'inférence et attestation ASR conservées.
- Before metrics : `log_native_timing(...)?` actif conditionnellement dans le moteur ; module R7 commun.
- Target metrics : absence de ce point d'échec et des accès de mesure retirés.
- Metric commands : V0, V4, V5 ; nouvelle identité de chaque worker mesurée après build, jamais supposée identique.
- Stop conditions : G2/G3, budget, changement de sémantique GPU ou décodeur ; aucun écrasement de worker installé.
- Dependencies : P3-T1 et autorisation du lot exact de six fichiers ; réactualiser l'inventaire des appels avant code ; couplage de livraison avec P3-T3.
- Implementation outline : supprimer les diagnostics choisis et leurs appels, conserver résultat natif, modèle, protocole et événements ; construire séparément CPU personnel puis CUDA personnel, copier les binaires sous la racine autorisée avant de changer de combinaison de features.
- Tests to add or update : sous-processus avec variables R6/R7 invalides ; vérification native du résultat, nécessaire pour démontrer la conservation après inférence. Une erreur OS de faux worker ne qualifie pas ce point du moteur.
- Verification commands : V4 et V5 ; commandes précises CPU/CUDA à fixer à G3, pas de campagne sans ce périmètre.
- Command provenance : Repo docs pour features existantes ; Source inferred pour matrice à autoriser.
- Acceptance criteria : aucun changement de modèle, protocole ou qualité ; pas de nouvelle feature sans accord ; ne pas livrer des workers non reconnus par le registre ; six fichiers maximum et exception justifiée acceptée avant code.
- Rollback/fallback notes : conserver anciens binaires sans les réinstaller ; si requalification indisponible, ne pas livrer ce delta et conserver P1.
- Notes for implementation agent : les workers personnels sont identifiés par SHA exact ; un simple rebuild constitue un nouveau candidat.

### P3-T3 : qualifier les nouvelles identités avant toute remise

- Priority : High pour toute livraison issue de P3-T2.
- Source finding : EH-05.
- Rationale : garantir admission, archives et identité GPU après reconstruction.
- Target files/modules : `crates/whisper-core/src/model_registry.rs`, tests d'identité/archives existants ; sorties de recette sous une racine à autoriser, sans nouvelle publication dans le profil utilisateur.
- Ownership bucket : registre de domaine et validation produit.
- Scope budget : un fichier de production plus tests correspondants et sorties autorisées ; aucun changement de format persistant.
- Closed invariant : identités reconnues exactement égales aux binaires qualifiés remis, compatibilité et refus historiques préservés.
- Before metrics : empreintes enregistrées pour les workers actuellement qualifiés.
- Target metrics : zéro identité supposée ou décalée entre dossier de recette, registre et preuves ; zéro reprise d'une ancienne clé retirée.
- Metric commands : V2, V4, V5 ; SHA-256 des ressources réellement utilisées, exigé par la recette existante.
- Stop conditions : absence qualification CPU/GPU, ressources manquantes, changement demandé d'archives ou écriture hors dépôt sans accord précis.
- Dependencies : P3-T2, G3 ; aucune mise à jour partielle du registre livrée.
- Implementation outline : vérifier artefacts et clés, mettre à jour uniquement les identités nouvelles autorisées, exécuter cas archives anciennes et nouvelle transcription ; remettre un lanceur conforme au profil réel.
- Tests to add or update : import et micro CPU/GPU, TXT/SRT/WAV, contrôle du GPU effectif, conservation des pending retirés ; Windows réservé à l'utilisateur.
- Verification commands : V2, V4, V5.
- Command provenance : Repo docs pour recette et hashes, Source inferred pour sélection finale des tests.
- Acceptance criteria : clés persistantes anciennes inchangées, refus explicites conservés, signatures IPC inchangées, validation native distincte des tests statiques ; budget respecté.
- Rollback/fallback notes : retrait de la nouvelle recette locale uniquement sur inventaire approuvé ; pas de restauration ou nettoyage de l'installation personnelle automatique.
- Notes for implementation agent : aucune installation, publication, nouveau manifeste d'empreintes ou push implicite.

## 7. Verification Matrix

Les commandes ci-dessous sont **proposées pour une future implémentation**, pas exécutées pour cet audit. Toute commande Cargo passe par `target/parakeet-tests/refacto-parakeet-live/guard.ps1` seulement si G0 autorise cette racine et confirme son applicabilité. Ne pas créer/copier une garde pour contourner ce point. Préflight >30 Gio, variables isolées, auto-tests et surveillance obligatoires ; `CARGO_INCREMENTAL=0`, debug `line-tables-only`, cible partagée, `--offline --locked` quand acceptés. Au premier échec ou violation : arrêt, preuves conservées, arbitrage avant reprise.

| Area | Command or check | Provenance | Expected result | Covers tasks | Notes |
| --- | --- | --- | --- | --- | --- |
| V0 statique | `git status --short`, `git diff --check` ; `rg -n 'parakeet-tests|PARAKEET_L[123]|PARAKEET_R[67]' crates` | Observed | État préservé, chaque référence résiduelle classée par contexte et cfg | Toutes | Une occurrence en test ou un contrat GPU ne constitue pas un échec. |
| V1 isolation | `& ./target/parakeet-tests/refacto-parakeet-live/guard.ps1 -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--test','parakeet_runtime_isolation','--','--test-threads=1')` | Source inferred | Tous les scénarios nouveaux réellement exécutés | P1-T1, P2-T1 | Cible nouvelle à créer ; pas de retour silencieux `ok`. |
| V2 régression | Même garde, arguments `@('test','--offline','--locked','-p','whisper-adapters','--lib','parakeet_import::','--','--test-threads=1')`, puis filtre `parakeet_live::` | Repo docs | Suites ciblées réussies ; NOT RUN explicitement distinct des contrôles exercés | P1/P2/P3 | Chaque appel séquentiel ; ne pas transformer des tests non exécutés en PASS fonctionnel. |
| V3 frontières | Lecture des modules et `rg -n 'target/parakeet-tests/l[23]|CARGO_MANIFEST_DIR' crates/whisper-adapters/src/parakeet_import` et fichier racine | Source inferred | Résolutions historiques possédées par le module privé ; pas de dépendance vers tests/desktop | P2-T1 | Revue manuelle du choix de parcours obligatoire. |
| V4 format/lint/build | Garde `-Mode SelfTest`, puis `-Mode Run -CargoArguments @('fmt','--all','--','--check')` ; `@('clippy','--offline','--locked','-p','whisper-adapters','--lib','--tests')` ; `@('build','--release','--offline','--locked','-p','whisper-desktop','--features','personal-install')` | Repo docs | Exit 0 ; warnings existants distingués des nouveaux | P1/P2/P3 | Ajouter worker/core concernés pour P3 après G3 ; jamais de Cargo direct. |
| V5 recette et requalification | Ressources effectives et SHA-256 ; CPU/GPU, import/micro, arrêt, exports ; comparaison contrôle CPU/GPU et attestation | Repo docs | Fonctionnement dans Windows et aucun accès requis aux anciens dossiers | P1 et P3 | P1 ne reconstruit pas les workers ; P3 nécessite commandes natives et chemins acceptés avant exécution. Fenêtre : utilisateur. |

Vérifications non revendiquées : modèle natif réel pendant l'audit, absence exhaustive de toute écriture OS, qualification des anciens environnements absents, validation des flashs UI. La garde observe des emplacements ; elle n'est pas un confinement préventif.

## 8. Decision Gates

| Gate | Status | Blocks | Question | Recommended default | How to validate |
| --- | --- | --- | --- | --- | --- |
| G0 | Blocking | Tout code / Cargo de ce plan | Accepter phase 1, chemins P1-T1, garde et racines existantes dans la session d'implémentation ? | Phase 1 seule, sans push | Accord écrit précis, préflight, vérification du plan versionné et garde. |
| G1 | Defaultable | Retrait des routes historiques uniquement | Conserver ou supprimer l'exécution Test historique ? | Conserver et isoler ; P2 peut être préparée sur cette base mais nécessite son GO | Inventorier utilisateurs/scripts avant tout retrait ; archives restent compatibles. |
| G2 | Blocking | Phase 3 | Retirer R6/R7/ancienne trace micro ou garder un diagnostic produit explicitement défini ? | Retrait des mesures obsolètes ; garder ASR GPU | Décision utilisateur ; sinon phase 3 différée, phase 1 indépendante. |
| G3 | Blocking | Build/qualification/livraison des workers nouveaux | Accepter le lot exact après inventaire des appels, racines et recette natives ? | Ne reconstruire aucun worker pendant phase 1 | Plan de lot ≤10 tâches, budgets vérifiés, ressources et critères CPU/GPU disponibles. |

## 9. Out of Scope

Pas de code dans cette tâche de planification. Pas de suppression L2/L3/R5/R6 sur disque ; pas de nettoyage `target` ou de l'installation personnelle. Pas de renommage des contrats d'environnement, clés d'archives ou protocole. Pas de changement de modèle, runtime natif, dépendance ou algorithme. Pas de migration générale des tests inline ni de refonte totale de `parakeet_import.rs`. Pas de nouvelle feature, garde, service, technologie, compte Windows ou outil de surveillance sans accord précis. Pas de validation UI automatique à la place de l'utilisateur, ni push.

## 10. Final Self-Check

| Check | Result | Notes |
| --- | --- | --- |
| Plan réel, sections présentes dans l'ordre | Pass | Phases, tâches, commandes et gates explicites. |
| Tous les constats tracés | Pass | Six EH ; identités et trace L3D du cœur différées explicitement. |
| Cibles, dépendances, tests, vérification, acceptation et repli par tâche | Pass | Budgets et dépassement conditionnel P3-T2 explicités. |
| Invariant, métriques et arrêt par phase/tâche structurelle | Pass | Choix de parcours, frontière historique, diagnostics. |
| Budgets définis | Pass | Maximum trois fichiers de production par tâche, sauf exception atomique motivée de six pour P3-T2, soumise à G3. |
| Tests d'intégration sous `tests/` | Pass | Petits invariants privés seulement si frontière inaccessible sans élargissement d'API ; aucune nouvelle masse inline. |
| Provenance des commandes | Pass | Nouveau test signalé comme proposé ; aucune campagne présentée exécutée. |
| Gates et permissions | Pass | Seuls produit/périmètre soumis à décision ; validation technique distincte. |
| Chemins et structure Rust | Pass | Existant/proposé distingué ; modules privés sans `#[path]`. |
| Périmètre borné | Pass | Cinq tâches, deux documents ; phase 1 seule recommandée au prochain GO. |

## 11. Implementation Agent Prompt

Après G0 seulement, déléguer à `rust_implementer` en mode IMPLEMENT : « Appliquer uniquement P1-T1 du plan `docs/cadrage/plan-nettoyage-environnements-historiques.md`, appuyé par l'audit associé. Préserver les correctifs et tous les changements préexistants, notamment non suivis. Budget : `parakeet_import.rs` et tests directs nommés au plan. Ne pas modifier workers, manifestes, identités, archives ou garde. Vérifier V1/V2/V4 sous garde autorisée et signaler tout NOT RUN. Arrêter au premier échec de campagne. Ne pas commencer P2/P3, committer ni pousser sans autorisation. » Puis revue via `rust_review_coordinator`, au plus deux cycles ; recette Windows séparée réservée à l'utilisateur.
