Avancée : phases 1 à 3 et correctifs micro validés par l'utilisateur, commit local 778ef0bc ; lot L3D du cœur autorisé.
Blocage : aucun pour préparer et exécuter le lot L3D décrit en section 12.
Décision : retirer uniquement cette trace historique, vérifier sous garde, faire relire et reconstruire une release locale distincte, sans installation ni push.

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

- Révision acceptée le 31 octobre 2026 : après inventaire et arrêt avant édition, l'utilisateur a autorisé le retrait complet sur les 21 fichiers de production explicitement proposés, avec tests concernés, vérifications sous la garde existante, revue indépendante et reconstruction locale sans workers ni push. Cette révision remplace le budget initial de trois fichiers pour ce seul lot ; elle n'autorise pas P3-T2/P3-T3.
- Extension précise acceptée ensuite dans cette même conversation : ajouter `parakeet_confirmation.rs` uniquement pour retirer les métadonnées/calculs de trace, sans changer règles, segments, plages ni ordre de confirmation ; ajouter `archive.rs` uniquement pour retirer l'import `Instant` inutilisé. Total autorisé : 23 fichiers de production, neuf tâches ci-dessous. Les vérifications, la revue et la reconstruction restent soumises à la garde existante, sans workers ni push.
- Priority : Low.
- Source finding : EH-04.
- Rationale : retirer les chemins de campagne et les coûts optionnels des diagnostics jugés inutiles.
- Target files/modules : sous `crates/whisper-adapters/src/`, les 23 chemins des tâches ci-dessous ; tests correspondants sous `tests/`, adaptation du test privé existant `parakeet_live/microphone_spectrum_tests.rs` et petits invariants privés séparés seulement si inaccessibles sans élargir l'API.
- Ownership bucket : diagnostics infra/adapters.
- Scope budget : 23 fichiers de production maximum, répartis en neuf tâches ci-dessous, plus tests directement correspondants ; aucune nouvelle dépendance. La suppression de la trace micro est une exception atomique de six fichiers : champs, appels et définitions doivent disparaître ensemble pour conserver un candidat compilable. La tâche séparée de confirmation retire seulement les producteurs de métadonnées, avec les mêmes règles de décision.
- Closed invariant : zéro lecture/écriture R7/ancienne trace micro dans la release personnelle selon la décision G2 ; D2 `cfg(test)` préservé.
- Before metrics : trace micro à racine absolue et mesures R7 facultatives compilées dans les adaptateurs.
- Target metrics : zéro accès aux anciennes destinations depuis le produit personnel, attestation ASR intacte.
- Metric commands : V0, V2, V4 ; contrôle des appels et imports résiduels.
- Stop conditions : besoin d'un nouveau produit de diagnostic, d'une feature, d'un 24e fichier de production ou d'un changement de worker ; arrêter pour arbitrage. Ne pas inventer une API de télémétrie.
- Dependencies : P2-T1, G2.
- Implementation outline : supprimer les mesures R7 des adaptateurs et l'ancienne trace micro, leurs appels, champs et calculs dédiés devenus sans objet ; conserver les diagnostics D2 `cfg(test)` et le journal ASR/attestation GPU ; ne pas modifier le comportement de capture ou de confirmation. Les champs `published` et `unresolved_before/after` de confirmation restent nécessaires aux événements produit ; seuls les curseurs/refus détaillés et autres données exclusives à la trace peuvent disparaître. Conserver strictement les prédicats de refus, leur ordre et les résultats de finalisation. Aucun relais no-op permanent pour les mécanismes retirés.
- Tests to add or update : activation des anciennes variables sans effet de fichier pour le produit ; comportements audio identiques ; contrats de traces conservées vérifiés séparément.
- Verification commands : V0, V2, V4.
- Command provenance : Observed pour recherche ; Repo docs/Source inferred pour futures commandes.
- Acceptance criteria : invariant atteint sans nouvelle écriture implicite ; zéro nouvelle masse inline ; budget respecté.
- Rollback/fallback notes : différer si utilité diagnostic non tranchée ; P1 reste suffisante pour protéger le lancement.
- Notes for implementation agent : trace L3D dans le cœur explicitement hors tâche ; pas de renommage global de variables.

Découpage borné du lot P3-T1 élargi (neuf tâches ; tous les chemins ci-dessous sont relatifs à `crates/whisper-adapters/src/`) :

| Tâche | Chemins / responsabilité | Budget production |
| --- | --- | --- |
| P3-T1-A | Retirer appels R7 et contexte dans `model_catalog.rs`, `parakeet_import.rs`, `worker_ipc.rs` | 3 |
| P3-T1-B | Retirer appels R7 dans `archive/import.rs`, `archive/storage.rs`, `archive/recovery.rs` | 3 |
| P3-T1-C | Retirer appels R7 dans `archive/source.rs`, `archive/journal.rs`, `archive/publication.rs` | 3 |
| P3-T1-D | Retirer appels R7 dans `worker_ipc/runtime/duplicates.rs`, `worker_ipc/runtime/import.rs`, `worker_ipc/runtime/queue.rs` | 3 |
| P3-T1-E | Retirer appels R7 dans `worker_ipc/runtime/poll/parakeet.rs` ; retirer seulement l'import `Instant` inutilisé dans `archive.rs` | 2 |
| P3-T1-F | Retirer `diagnostics.rs` et sa déclaration dans `lib.rs`, après les consommateurs | 2 |
| P3-T1-G | Retirer trace micro dans `parakeet_live.rs`, `parakeet_live/{events,lifecycle,trace,inference,windows}.rs`, garder D2 | 6, exception atomique motivée ci-dessus |
| P3-T1-H | Retirer uniquement métadonnées et calculs de trace dans `parakeet_confirmation.rs` ; vérifier décisions identiques | 1 |
| P3-T1-I | Adapter/ajouter tests directs, vérifier V0/V2/V4 et variables historiques sans fichier, revue puis release locale | 0 hors corrections des chemins déjà ouverts |

Préflight : référence exacte pré-P3 de chaque fichier déjà modifié ou non suivi qui sera touché, tests compris, sous `target/parakeet-tests/refacto-parakeet-live/pre-p3-t1/` ; aucun manifeste d'empreintes ajouté. Racines autorisées inchangées : lot `target/parakeet-tests/refacto-parakeet-live/` et compilation `target/parakeet-tests/cargo-shared/`, garde existante uniquement. Ne pas lancer de vérification Cargo sur un état de suppression intermédiaire incomplet ; produire d'abord le candidat cohérent. Première erreur de compilation, test ou garde : arrêt et arbitrage selon AGENTS.

Acceptation du lot élargi : zéro appel `record_stage_timing`/`TimingContextGuard` de R7 adaptateurs, zéro lecture/écriture de l'ancienne variable de trace micro dans la production, aucune évaluation résiduelle dédiée à ces mécanismes ; D2 de test conservé ; ASR GPU et ses contrôles inchangés ; identités, archives persistées, workers, manifests et garde inchangés. Les scénarios publics restent dans `tests/`, le test privé existant n'est adapté que pour retirer la construction de `TraceSink` devenue sans objet. Les ressources de recette et SHA sont vérifiées avant remise, sans installation ni lancement de la fenêtre Windows par l'agent.

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

### Préparation G3 après validation utilisateur de P3-T1 — proposition à confirmer

L'utilisateur a autorisé la continuation de la prochaine phase. Le préflight confirme les six fichiers worker de P3-T2 et les ressources locales CPU/GPU ; 164,3 Gio libres relevés avant cette étape. G3 reste à confirmer sur le paquet précis ci-dessous : le plan initial laissait volontairement les commandes natives et les racines de recette à définir avant exécution.

Point de compatibilité révélé par les sources : `archive/identity.rs::is_registered_personal_import` compare les clés aux seules identités personnelles courantes. `archive/recovery.rs::validate_pending` s'en sert pour les pending Parakeet version 2. Remplacer les empreintes courantes sans préserver la reconnaissance historique ferait perdre cette reconnaissance aux anciennes clés personnelles. Les archives unifiées version 4 sans qualification suivent une autre branche. Cette observation ne signifie pas que les archives actuelles sont endommagées.

Périmètre de production proposé (neuf fichiers) :

- P3-T2 : `crates/whisper-worker-parakeet-cpu/src/{native_engine,timing,main,integrity,startup,transcription}.rs`, suppression R6/R7 et de leurs seuls appels/calculs dédiés. Exception atomique de six fichiers déjà motivée dans cette tâche ; préserver moteur, décodeur, protocole, bornes et attestation ASR GPU.
- P3-T3 : `crates/whisper-core/src/model_registry.rs` et, extension précise proposée, `crates/whisper-adapters/src/archive/{identity,recovery}.rs`. Budget de trois fichiers pour cette tâche. Séparer reconnaissance des clés d'archive antérieures et admission des exécutables actuels : aucune ancienne empreinte ne devient une autorisation d'exécuter un worker différent. Pas de modification de fichiers d'archives utilisateur, de schéma persistant ni de règle de reprise implicite.
- Tests directs : fichiers existants de contrat et de native worker, `tests/support/worker.rs`, tests d'archives/registre concernés ; nouveau `crates/whisper-worker-parakeet-cpu/tests/native_diagnostics_absence.rs` proposé pour les scénarios réels sans diagnostic. Petits tests privés séparés seulement si nécessaires à un invariant inaccessible autrement.

Découpage proposé du prochain lot : (1) préflight/snapshots et copies de référence, (2) retrait worker sur les six fichiers, (3) builds CPU puis CUDA et gel des EXE avant modification des empreintes, (4) qualification native différentielle sur les copies, (5) enregistrement des nouvelles empreintes et compatibilité des anciennes clés, (6) tests de contrats/archives et revue indépendante, (7) desktop release et dossier de remise. Sept tâches ; ne pas démarrer d'autre lot ou de migration de données.

Racines proposées : garde existante `target/parakeet-tests/refacto-parakeet-live/guard.ps1`, lot inchangé, `CARGO_TARGET_DIR=target/parakeet-tests/cargo-shared/`. Toutes les nouvelles sorties, copies de ressources, fixtures et réglages sous `target/parakeet-tests/refacto-parakeet-live/p3-workers/`. Les sous-dossiers `reference/`, `localappdata/Whisper/{worker,native,models,settings}/`, `input/` et `outputs/` y séparent références figées et candidat. Aucun remplacement sous le véritable `%LOCALAPPDATA%` ni sous l'environnement local de recette précédemment validé.

Entrées lues uniquement : workers/DLL/modèles personnels déjà vérifiés sous `target/parakeet-tests/parakeet-unified/lot-04/isolated-localappdata/Whisper/`, et `sound example/spkr0.wav`. Copier les ressources nécessaires dans la nouvelle racine ; produire en Rust une fixture PCM de 30 secondes de ce WAV sous `input/`. Pas de téléchargement ni de corpus long historique absent. Si cet extrait ne permet pas une comparaison de parole exploitable, arrêter la qualification et faire arbitrer le corpus au lieu d'en inventer une preuve.

Commandes proposées, toutes via `guard.ps1 -Mode Run -CargoArguments @(...)` après son `SelfTest` et le préflight permanent (provenance : features observées dans le manifeste, nouvelles sélections de tests inférées du support à implémenter) :

1. Tests worker sans moteur : `test --offline --locked -p whisper-worker-parakeet-cpu --features parakeet-personal`, puis la variante `parakeet-personal,parakeet-cuda` ; distinguer ignorés et scénarios réellement exécutés.
2. Build CPU : `build --release --offline --locked -p whisper-worker-parakeet-cpu --features parakeet-personal` ; copier l'EXE résultant vers le candidat CPU avant l'étape suivante.
3. Build CUDA : même commande avec `--features parakeet-personal,parakeet-cuda` ; copier/nommer le candidat `whisper-worker-parakeet-gpu.exe`. Ne pas mélanger leurs DLL. Les deux EXE sont ensuite figés ; une recompilation ultérieure liée au registre ne remplace pas ces copies sans nouvelle qualification.
4. Native : `test --release --offline --locked -p whisper-worker-parakeet-cpu --features parakeet-personal,parakeet-cuda --test native_diagnostics_absence -- --ignored --test-threads=1`. Nouveau harnais proposé : pour chacun des deux backends, référence propre, candidat propre et candidat avec R6/R7 invalides ; vrais chargement et inférence, handshake/identités, texte exploitable, mots/plages et nombre d'échantillons comparés sur le même backend, journal ASR et identité GPU conservés. Aucun faux worker ne remplace ces cas. Aucun échec volontaire de la référence avec variables polluées.
5. Régression après enregistrement des clés : garde avec tests core/archives/isolation déjà présents et nouveaux cas de compatibilité ; filtres définitifs à relever dans les sources avant exécution, pas de zéro test annoncé PASS. Format et Clippy sur worker/core/adapters concernés.
6. Desktop : `build --release --offline --locked -p whisper-desktop --features personal-install`, puis lanceur pointant uniquement vers les nouvelles ressources qualifiées de ce lot. Import/micro CPU/GPU, exports et comportement dans la fenêtre restent à confirmer par l'utilisateur ; les preuves natives et cette recette ne sont pas confondues.

Arrêts : espace <30 Gio, préflight/garde/compilation/test en échec, ressource absente ou divergente, texte non exploitable, divergence de résultat à expliquer, attestation GPU absente, identité reconnue différente des copies remises, autre fichier de production nécessaire. Conserver preuves et références, aucun nettoyage/restauration ni relance après échec sans arbitrage. Deux cycles de revue/correction maximum. Le bilan utilisera ce document et la réponse de remise, sans campagne documentaire supplémentaire.

### Bilan d'exécution P3-T2/P3-T3 — 31 octobre 2026

Le paquet G3 ci-dessus a reçu un GO écrit précis dans cette conversation : lot couplé, compatibilité des anciennes archives, essais natifs CPU/GPU sous garde, revue indépendante et release sous `p3-workers`, sans remplacement de la version validée, installation ni push. Les neuf fichiers de production prévus et les tests directs ont été modifiés ; les copies exactes pré-source des neuf fichiers et du test de racines existant restent dans `p3-workers/pre-source/`. Le test historique `runtime_paths/tests.rs` a été adapté, après accord précis, à une fixture isolée au lieu des répertoires L2/L3 réels absents.

Trois arrêts ont été conservés : compilation avec références résiduelles au module timing, événement Git réel d'origine non établie pendant un SelfTest, puis échec du test historique faute de dossiers. L'utilisateur a autorisé chaque correction/reprise correspondante ; aucun fichier Git n'a été supprimé ou restauré. Une invocation `--p` invalide a aussi été conservée comme commande refusée avant compilation, pas comme un build exécuté ; l'invocation valide `--package` a ensuite été exécutée sur arbitrage local. La garde n'a pas été assouplie. Sa surveillance sans PID ne constitue pas une preuve exhaustive d'absence de fuite.

Les deux workers ont été construits séparément et figés **avant** l'édition des empreintes actives dans le registre :

| Backend | EXE réellement qualifié et remis, sous `p3-workers/localappdata/Whisper/worker/` | SHA-256 |
| --- | --- | --- |
| CPU | `whisper-worker-parakeet-cpu.exe` | `a965c9503f909124f8ddedad09e83e5d187e70edd1869e880e9f1ad5ae2fb99d` |
| GPU | `whisper-worker-parakeet-gpu.exe` | `7ffb0b4a022ec26909a8939b37787de6df6c9bdb2903edf14d3ffb32579eefc1` |

Les essais natifs portent sur ces copies, pas sur les EXE éventuellement reconstruits dans le cache Cargo après mise à jour des clés. Leurs SHA ont été revérifiés après le build desktop et concordent avec les identités actives. Les cinq DLL CPU, neuf DLL GPU et deux modèles correspondent aux ressources vérifiées d'origine ; les workers de la version validée gardent leurs anciennes empreintes.

| Contrôle exécuté sous garde | Résultat | Référence du run, préfixe `20261031T` |
| --- | --- | --- |
| Builds CPU puis CUDA figés | PASS | `183236170-67c48488`, `183251382-78485ca8` |
| Native : référence propre, candidat propre, candidat avec R6/R7 invalides, chaque backend | Un test exécutant six inférences PASS ; 480 000 échantillons, 114 mots et texte/plages identiques par backend ; ASR et identité GPU vérifiés | `183357009-82447c74` |
| Anciennes/nouvelles clés d'archives CPU/GPU, clés altérées, admission et refus de reprise | PASS ; anciennes clés acceptées seulement lors de validation d'archive, pas d'exécution ; refus de reprise sans réécrire le pending ; refus L2a/R5-2 conservés | `183718108-141813e3` |
| Cœur, bibliothèque | 80 PASS | `183618349-427a9728` |
| Archives, bibliothèque | 38 PASS, 1 ignoré | `183628138-72ca9e5d` |
| Parakeet Live | 39 PASS | `183646305-557f35a7` |
| Isolation import / session | 2 PASS / 1 PASS | `183655922-c0208f19`, `183700716-e40844f2` |
| Worker personnel CPU / CUDA après édition des clés | 24 PASS / 24 PASS ; copies qualifiées non remplacées | `183733810-af0c0750`, `183740651-ba86c674` |
| Format, Clippy worker et adaptateurs, build adaptateurs | PASS ; avertissements existants et helper de fixture inutilisé en configuration CUDA signalés | `183750070-f08954bb`, `183754932-41d6c12b`, `183800846-95641493`, `183810777-09804344` |

Les journaux exacts et arguments sont conservés sous `target/parakeet-tests/refacto-parakeet-live/logs/`. Les preuves des builds, tests, arrêts et six journaux ASR ont été archivées dans `p3-workers/archive/`, avec arborescence, nombre (46 fichiers) et SHA-256 des copies vérifiés. Les copies des journaux globaux de garde contiennent aussi les campagnes précédentes ; elles ne sont pas présentées comme un journal exclusif de ce lot. Aucune source, référence, copie WAV ou racine Cargo partagée n'a été nettoyée.

La revue indépendante du candidat courant, comparé aux snapshots pré-source sur le HEAD documentaire `d15fdeb403d1462f2332216bce119c3b53a1404c`, conclut **CLEAN**, un passage, aucune correction demandée. Elle confirme le retrait des seules mesures R6/R7, la conservation de la durée d'inférence contractuelle, du moteur, du protocole et de l'attestation GPU, et la séparation entre reconnaissance des anciennes archives et admission courante.

Après revue, le desktop personnel a été reconstruit sous garde (`build --release --offline --locked --package whisper-desktop --features personal-install`) avec succès. Remise : `p3-workers/release/Lancer-Whisper.cmd`, vérifié avec `--check` sans ouvrir la fenêtre. Il utilise `p3-workers/localappdata/Whisper/` pour les workers Parakeet, modèles, DLL et réglages ; seuls les deux workers Turbo conservés sont placés à côté du desktop. Le dossier release ne doit pas être déplacé seul hors de son arborescence de ressources.

Limites : `worker_contract` autonome NOT RUN car il impose une racine `main-facades` hors de ce lot ; les six inférences IPC ne prétendent pas couvrir tous ses cas. Un test archive est ignoré. Aucun nouveau résultat de performance ni validation de cette nouvelle release dans la fenêtre Windows n'est revendiqué. L'utilisateur avait déclaré P3-T1 fonctionnel avant ce lot ; cette validation ne vaut pas pour le nouveau binaire. Espace libre final relevé : 160,7 Gio. Sources produit laissées non committées ; seuls les documents de cadrage sont commités localement par la session principale. Aucun push ni installation.

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

## 12. Lot distinct : retrait de la trace L3D du cœur

Autorisation : le 31 octobre 2026, après validation utilisateur de la release `p3-workers/release-fix-parakeet-micro/` et commit local `778ef0bc`, l'utilisateur confirme le périmètre proposé : retrait de la trace L3D du cœur, plan existant actualisé, implémentation déléguée, tests sous garde, revue indépendante et nouvelle release locale, sans installation ni push. Cette section remplace uniquement le report de L3D pour ce lot ; les autres exclusions et contrats restent applicables.

Objectif produit : supprimer la lecture de la variable de chronométrage et les sorties historiques sur stderr pendant les commandes, événements et rafraîchissements, sans changer les états visibles, les effets métier ou le journal ASR GPU. Aucune amélioration chiffrée de performances n'est revendiquée.

### P4-T1 : retirer la trace et ses appels

- Auteur Rust : `rust_implementer`, mode IMPLEMENT ; baseline produit `778ef0bc`.
- Six fichiers de production autorisés, tous sous `crates/whisper-core/src/` : `application.rs`, `application/diagnostics.rs`, `application/runtime/mod.rs`, `application/runtime/commands.rs`, `application/runtime/import.rs`, `application/runtime/poll.rs`.
- Retirer le module de diagnostic, ses imports et ses appels ; ne pas laisser de fonctions vides ni de nouvelle branche de configuration. Préserver l'ordre et le contenu des commandes, événements, effets et transitions.
- Critères : aucune référence à `PARAKEET_L3D_TIMING_ACTIVE`, `L3D_APP_TRACE`, `record_application_timing` ou `trace_application_event` dans le code de production du cœur ; API, formats persistants, protocoles et dépendances inchangés.
- Tests associés seulement dans les supports existants ou fichiers de tests séparés du cœur si une lacune de couverture concrète le justifie ; pas de hook de production ni de nouveau framework.

### P4-T2 : vérifier et faire relire

- Préflight : relever l'espace libre (>30 Gio), l'état Git et l'absence d'un écrivain Git concurrent avant lancement de la garde. Préserver les non-suivis `docs/architecture/`, `models/` et `sound example/`.
- Racines inchangées : garde `target/parakeet-tests/refacto-parakeet-live/guard.ps1`, sorties et temporaires dans sa racine autorisée avec les preuves du lot sous `p3-workers/core-l3d/`, compilation uniquement dans `target/parakeet-tests/cargo-shared/`. Variables isolées, auto-tests et surveillance obligatoires ; aucune modification de garde.
- Vérifications utiles : contrôle statique des quatre identifiants et `git diff --check` ; garde SelfTest puis Run pour `fmt --all -- --check`, `test --offline --locked -p whisper-core --lib`, suites desktop pertinentes, `clippy --offline --locked -p whisper-core --lib --tests` et compilation personnelle du desktop. Exercer la suite cœur avec la variable historique à `1` dans l'environnement de campagne pour vérifier qu'elle ne réactive aucune trace ; ne pas modifier les variables persistantes Windows.
- Revue via `rust_review_coordinator` et reviewer indépendant en lecture seule ; corrections exclusivement par le même implémenteur, deux cycles maximum. Un échec réel de compilation, test, préflight ou garde arrête la campagne ; diagnostic en lecture seule et arbitrage utilisateur avant correction/reprise.
- Aucun essai natif CPU/GPU nouveau : workers, modèles, moteurs et routage inchangés. Distinguer les tests contrôlés de la validation dans la fenêtre Windows.

### P4-T3 : remettre une release locale distincte

- Après revue, compilation via garde : `build --release --offline --locked -p whisper-desktop --features personal-install`.
- Nouveau dossier sous `target/parakeet-tests/refacto-parakeet-live/p3-workers/`, sans remplacer `release-fix-parakeet-micro/` ni les autres versions validées. Réutiliser les workers Turbo livrés et les ressources Parakeet qualifiées de `p3-workers/localappdata/Whisper/` ; aucune reconstruction de worker.
- Vérifier présence et SHA-256 des ressources effectives et des copies remises ; contrôler le lanceur avec `--check` sans ouvrir la fenêtre. Ne pas annoncer un paquet autonome.
- Archiver les petites preuves en conservant leur arborescence et en vérifiant nombre et SHA-256 ; relever l'espace libre final. Aucun nettoyage de cache ou WAV sans manifeste approuvé.
- Compte rendu dans ce document et la remise : fichiers changés, checks réellement exécutés, verdict, réserves et validation Windows restant à l'utilisateur. Aucun nouveau rapport séparé, installation, push ou modification d'archive utilisateur. Le commit du code de ce nouveau lot nécessite un accord distinct ; seul le plan est versionné localement avant implémentation.
