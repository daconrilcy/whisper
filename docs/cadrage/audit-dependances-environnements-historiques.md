Avancée : audit statique des dépendances historiques terminé ; six constats classés.
Blocage : aucun pour l'audit ; les scénarios identifiés n'ont pas été exécutés dans cette passe.
Décision : isoler en priorité le lancement personnel des variables de campagne ; aucune implémentation autorisée par ce document.

## Scope

Audit du 31 octobre 2026, dépôt `C:\dev\whisper`, branche `evo-startup-perf`, HEAD `998a732a33a7db7992f9c751cd29bdb029ef7456`. Demande : rechercher les dépendances aux anciens environnements de test dans le code de production et proposer un nettoyage sans modifier ce code.

L'état examiné inclut les deux correctifs locaux non committés de cette conversation : recyclage dans `parakeet_live/capture.rs` et son test, puis contrôle R6/L3b dans `parakeet_import.rs` et `parakeet_import/legacy_runtime_tests.rs`. Ils ont reçu séparément une revue CLEAN et ne sont pas réaudités ici. Les éléments non suivis `docs/architecture/`, `models/` et `sound example/` restent préservés. La release locale est un instantané de cet état, pas du seul HEAD.

Recherche sur les huit packages de `crates/`, puis lecture ciblée des appels de production, manifestes et conditions de compilation. Termes : `parakeet-tests`, chemins L1/L2/L3, variables `PARAKEET_L*`, `PARAKEET_R*`, `CARGO_MANIFEST_DIR`, qualification, recette et traces. Les résultats dans `tests/`, les modules `cfg(test)` et les fonctionnalités désactivées sont distingués des chemins livrés. Cela établit les branches visibles dans les sources, pas une preuve exhaustive sur chaque binaire installé.

Aucun Cargo, test, moteur natif ou fenêtre Windows exécuté pour cet audit. L'inventaire du skill a été utilisé sur `crates/` en désactivant son appel interne à `cargo metadata`, conformément à la règle de campagne. Résultat : 333 fichiers Rust, 123 156 lignes physiques tests compris, huit manifestes. Une tentative d'inventaire à la racine, inutilement lente dans les sorties de compilation, a été interrompue ; l'inventaire `crates/` est celui retenu.

## Architecture Snapshot

| Responsabilité | Sources actuelles | Constat |
| --- | --- | --- |
| Composition du produit | `whisper-desktop/src/root.rs:322-403` | `personal-install` choisit l'installation personnelle ; le profil par défaut lit les variables historiques. |
| Lancement et transport Parakeet | `whisper-adapters/src/parakeet_import.rs` | Validation produit, exceptions historiques, journaux et processus partagent le même module. |
| Exécution du moteur | `whisper-worker-parakeet-cpu/src/{startup,runtime_paths,native_engine,gpu_identity,timing}.rs` | `parakeet-personal` sépare les racines personnelles des racines Test ; certaines mesures historiques restent communes. |
| Identité et compatibilité des archives | `whisper-adapters/src/archive/{identity,import,recovery}.rs`, `whisper-core/src/model_registry.rs` | Les identités historiques ont aussi un rôle de compatibilité et de refus de reprise. |
| Diagnostics | `whisper-adapters/src/diagnostics.rs`, `parakeet_live/trace.rs`, `whisper-core/src/application/diagnostics.rs` | Mesures R7/L3D et trace micro encore configurables dans le code courant. |
| Support de recette | `worker_ipc/runtime/support.rs`, `scripts/recette-build.rs` | Les hooks destructifs/de synchronisation examinés sont sous `recette-hooks`, refusé en release. |

Direction observée : desktop → adapters + core, adapters → core, worker Parakeet → core. Pas de nouvel import vers les tests identifié dans les parcours examinés. Les variables d'environnement de diagnostic dans le cœur constituent une responsabilité technique résiduelle, pas une dépendance Cargo vers les adaptateurs.

Évaluation des axes du skill : placement des tests **improved but incomplete** ; discipline des façades **improved but incomplete** ; responsabilités **problematic** dans `parakeet_import.rs` ; direction des dépendances **clean** dans le périmètre ; erreurs typées **improved but incomplete** (chaînes préfixées conservées) ; surface publique **improved but incomplete** (`spawn_for_qualification` public mais caché à la documentation).

Contrôle du placement des tests (test placement) : l'inventaire compte **30 production src files with inline test modules** et **178 inline `#[test]` functions** dans ces modules. Cette mesure heuristique reconnaît les modules inline nommés `tests` ; elle ne compte pas tous les fichiers de tests sous `src/`. Exemple : `root.rs` contient 878 lignes de module de tests selon cet inventaire. `parakeet_import.rs` dépasse 1 300 lignes physiques et conserve une orchestration significative malgré son sous-dossier de tests. Cela complique la distinction produit/recette, mais ne justifie pas une migration générale des tests dans ce lot.

## Key Findings

### EH-01 [High] IMPORTANT — des variables L3C/L3D peuvent imposer les anciens dossiers à un lancement GPU personnel

- Preuves : `parakeet_import.rs:894-990`, fonction `worker_stderr_log`. Sans `session_log_root`, les variables `PARAKEET_L3C_EXEC_ROOT` et `PARAKEET_L3D_EXEC_ROOT` sont traitées avant la sélection du journal personnel. Elles provoquent la résolution des racines canoniques L3c/L3d puis L3b. Cette sélection n'est pas protégée par `recette-hooks`.
- Appel de production : `worker_ipc/runtime/import/worker_start.rs:374` appelle `ParakeetProcess::spawn`, qui ne fournit pas de racine de journal de session. Les parcours unifiés avec `spawn_live` fournissent cette racine et évitent ces deux branches ; ce constat n'annonce donc pas un nouvel échec systématique du micro unifié.
- Risque : un environnement hérité peut faire échouer le lancement GPU sur un ancien chemin absent ou sur le conflit L3C/L3D. Dépendance conditionnelle confirmée par lecture, scénario non reproduit pendant l'audit.
- Direction minimale : sélectionner le parcours personnel avant toute configuration historique ; les variables de campagne ne doivent pas influencer ce parcours. Conserver le journal ASR nécessaire à l'attestation GPU.

### EH-02 [High] IMPORTANT — une mesure R6 peut faire échouer une transcription de production

- Preuves : `whisper-worker-parakeet-cpu/src/native_engine.rs:95-96` appelle `log_native_timing(...)?` si `PARAKEET_R6_TIMING_LOG` existe. La fonction vers `331-378` impose `PARAKEET_L2_RUNTIME_ROOT/r6`, puis propage les erreurs de chemin ou d'écriture. Aucun `cfg(test)` ni feature de recette n'encadre cet appel.
- `parakeet_import.rs:613-617` configure explicitement la racine du worker, mais laisse hériter les autres variables du processus parent.
- Risque : avec une variable R6 héritée, une inférence native réussie peut être suivie d'un échec de transcription pour un ancien journal manquant. Ce mécanisme concerne les builds personnels CPU et CUDA issus de cette source ; les ressources installées n'ont pas été désassemblées.
- Direction minimale immédiate : empêcher les variables de campagne de parvenir aux workers personnels lancés par le produit. Nettoyage ultérieur du worker conditionné à la requalification de ses nouvelles empreintes.

### EH-03 [Medium] IMPORTANT — routages historiques et routage personnel cohabitent dans les fonctions ordinaires

- Preuves : `ParakeetWorkerPaths::{validate,is_canonical_l3b_reference,qualification_key,gpu_qualification_key,is_l2b_runtime}`, `spawn_for_qualification` et le helper R6 dans `parakeet_import.rs` ; `root.rs:322-403` ; `runtime_paths.rs:31-65,93-143` du worker.
- Le correctif local évite désormais L3b pour le personnel dans le helper R6. Les méthodes de qualification choisissent déjà le personnel avant les branches L2b/L3b. Les workers compilés avec `parakeet-personal` vérifient l'installation personnelle ; leurs builds par défaut sont volontairement Test. Ces références ne sont pas toutes des bugs.
- Risque : l'imbrication rend facile le retour d'une résolution historique avant le choix du parcours, comme le défaut corrigé. Le défaut Test du build n'est pas équivalent au profil personnel livré.
- Direction : extraire une frontière privée de compatibilité historique, sans nouvelle feature ni élargissement d'API dans le premier lot ; conserver les validations et identités existantes.

### EH-04 [Low] MINEUR — diagnostics de campagne encore présents, avec des effets différents

- `parakeet_live.rs:45-46` et `parakeet_live/trace.rs:82-122` : trace de confirmation liée à une racine absolue `C:\dev\whisper\target\parakeet-tests\parakeet-micro-lot1`. Elle est inactive sans variable ; son échec désactive la trace sans arrêter la capture.
- `whisper-adapters/src/diagnostics.rs:42-102` et `whisper-worker-parakeet-cpu/src/timing.rs:3-54` : mesures R7 activables par environnement, écritures synchrones dans une racine configurable, erreurs traitées sans échec produit. Leurs coûts restent possibles si activées.
- `whisper-core/src/application/diagnostics.rs:10-30` : variable L3D active des sorties stderr, sans résolution de dossier historique.
- `parakeet_live/trace.rs:3-69` : diagnostics D2 réellement limités à `cfg(test)` ; pas de dépendance active à nettoyer en urgence.
- Direction : traiter séparément trace produit éventuelle et mesures de campagne. Aucun retrait de diagnostic demandé implicitement ; reporter la décision sur leur utilité et garder les bornes existantes.

### EH-05 [Medium] IMPORTANT — les identités dites Test portent aussi la compatibilité des données

- Preuves : `archive/identity.rs:100-184` accepte les clés personnelles via `is_parakeet_import_test` et reconnaît les anciennes clés L2a/R5-2 ; `archive/recovery.rs:81-103` refuse explicitement leur reprise et conserve les pending.
- `archive/import.rs:405-409` réserve le profil VAD R5-2 à une destination de recette ; `archive/identity.rs:851-867` vérifie les deux racines d'environnement. Le profil personnel single-pass ne dépend pas de ce contrôle.
- `model_registry.rs:39-57` et `gpu_identity.rs:47-84` contiennent aussi des identités et un journal ASR historiquement nommés L3, toujours utilisés pour l'intégrité et l'attestation. `PARAKEET_L2_RUNTIME_ROOT` est aujourd'hui un contrat parent/worker personnel malgré son nom.
- Risque : supprimer mécaniquement les noms L2/L3, les empreintes ou les clés peut casser les archives, l'admission ou l'attestation GPU. Pas de preuve d'un accès obligatoire aux anciens dossiers pour le profil personnel dans ces contrôles d'archives.
- Direction : préserver le format persistant, les contrôles SHA et le refus des anciennes reprises ; clarifier les noms internes seulement après tests de compatibilité. Ne pas renommer les variables de protocole dans le premier lot.

### EH-06 [Medium] IMPORTANT — une partie de la couverture historique peut afficher `ok` sans exercer le contrôle

- Preuves : `parakeet_import.rs:1260-1286`, helper `not_run_without_campaign_variables` et test `paths_reject_a_worker_outside_the_runtime_root`. Sans variable de campagne, le test retourne prématurément.
- Limite déjà constatée lors de la vérification précédente : ancien contrôle fonctionnellement NOT RUN ; les deux nouveaux tests du correctif L3b ont effectivement été exécutés.
- Risque : une suite verte ne démontre pas l'indépendance des parcours de production envers les anciens dossiers.
- Direction : scénarios autonomes avec faux worker et environnement isolé par sous-processus ; garder les campagnes natives explicitement non exécutées lorsqu'elles manquent. Ne pas annoncer une couverture native à partir des tests de chemins.

Références écartées comme défaut actif : hooks `worker_ipc/runtime/support.rs` et injection GPU dans `worker_ipc/backends/native.rs` sous `recette-hooks`, avec refus release dans `scripts/recette-build.rs` ; tests inline de `root.rs` et `source_binding.rs` ; mentions L1 dans les commentaires et libellés d'erreur du catalogue. Les fonctionnalités L01 de qualification CPU/GPU sont également explicitement activables, distinctes des parcours par défaut ; aucune migration implicite proposée.

## Principle Review (SOLID / DRY / KISS / YAGNI)

- SOLID : le principal problème est la responsabilité mêlée du lancement produit et des exceptions de campagnes ; pas de nouvelle couche de traits justifiée.
- DRY : les contrôles de racines parent/worker ont des finalités différentes. Les fusionner sans contrat commun affaiblirait les protections.
- KISS : une sélection de parcours unique avant les résolutions de chemins serait plus lisible que des exceptions réparties dans la journalisation et la qualification.
- YAGNI : supprimer ou isoler les anciennes mesures selon leur usage confirmé ; ne pas créer de nouvelle infrastructure de qualification pour ce nettoyage.

## Refactoring Roadmap

1. **Stabiliser** : neutraliser l'influence des variables de campagne sur les lancements personnels et vérifier sans anciens dossiers (EH-01, EH-02, EH-06).
2. **Simplifier** : isoler la compatibilité de chemins historiques dans un module privé distinct (EH-03), sans retirer les contrats de données (EH-05).
3. **Nettoyer après décision** : diagnostics historiques et workers personnels, avec requalification exacte des binaires si leur source change (EH-04, EH-05).

### Next Closed Slice

Invariant : lancer le worker personnel CPU/GPU doit choisir les mêmes ressources et journaux de produit, que les variables L3C/L3D/R6/R7 soient absentes ou pointent vers des racines historiques absentes. Périmètre : frontière de lancement dans `parakeet_import.rs`, éventuel helper privé d'environnement, tests d'intégration autonomes. Arrêt : zéro résolution de dossier de campagne attribuable au lancement personnel, variables ASR/racine du worker explicites conservées, validation SHA et archivage inchangés. Vérifications proposées sous garde : `cargo test --offline --locked -p whisper-adapters --test parakeet_runtime_isolation -- --test-threads=1`, puis suite Parakeet ciblée, format, Clippy et build release. Le nom du nouveau test est proposé, il n'existe pas encore. Voir le plan associé pour les commandes, budgets et autorisations futures.

## Quick Wins

Prioriser la sélection personnelle dans les journaux et la maîtrise de l'environnement enfant : cela traite deux risques sans modifier les workers installés ni leurs empreintes. Conserver les noms d'environnement contractuels ; corriger les messages historiques uniquement lorsque cela accompagne une modification utile. Ne pas créer L3b pour masquer un accès indésirable.

## Open Questions / Validation Needed

- Faut-il conserver l'exécution des anciens parcours Test ? Recommandation : les isoler d'abord ; leur suppression nécessite une décision distincte et un inventaire de leurs utilisateurs.
- Les traces R6/R7/L3D doivent-elles rester disponibles dans les releases personnelles ? Recommandation : préserver le diagnostic ASR nécessaire au GPU, puis retirer les mesures devenues inutiles après décision. Aucun nouvel accès volontaire hors des racines autorisées.
- Les variables historiques n'étaient pas présentes dans l'environnement de cette session d'audit. Les risques EH-01/EH-02 sont conditionnels ; ils ne sont pas présentés comme la cause des flashs signalés.
- À vérifier lors de l'implémentation autorisée : lancement réel CPU/GPU et import/micro sans L2/L3 historiques, arrêt et TXT/SRT/WAV, identité GPU et absence de régression archives. Aucun de ces essais n'a été exécuté ici.

Plan de nettoyage : [plan-nettoyage-environnements-historiques.md](plan-nettoyage-environnements-historiques.md).
