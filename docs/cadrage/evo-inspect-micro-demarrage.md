Avancée : cadrage INSPECT-MICRO-DEMARRAGE établi sur la base 48827173.
Blocage : revue indépendante à effectuer ; exécution produit non autorisée.
Décision : concevoir et planifier le périmètre demandé, sans coder ni lancer de campagne.

# INSPECT-MICRO-DEMARRAGE — cadrage v1

## 1. Autorité, périmètre et sources

Le 6 novembre 2026, l'utilisateur demande : « ok defini le cadrage et le plan d'implementation a partir de ce besoin ». Le besoin qualifié, revu PASS dans cette session, porte sur l'inspection ciblée au démarrage et dans les quatre bindings micro, le SHA calculé hors du mutex puis revalidé par identité, et deux mesures humaines avant/après. Cette demande autorise les livrables documentaires et leur versionnement local selon AGENTS.md ; elle n'autorise aucun code, compilation, installation ou push.

Résultat produit recherché : réduire les attentes évitables en préservant les choix existants, l'intégrité, l'audio annoncé comme capturé, la qualité et les sorties. Un gain reste à démontrer. Aucun seuil de 2 s/3 s n'est adopté par ce lot.

Inclus : démarrage/catalogue/sélection, les quatre points micro, cache d'intégrité partagé et préservation des parcours existants. Exclus : SHA internes aux workers, nouveau modèle/moteur, build natif, nettoyage. Différés : refonte générale du démarrage, capture anticipée, préchauffage général, tray, démarrage Windows, reprise après panne et nouveaux réglages. Leur reprise exige une autre décision produit.

Sources actuelles consultées, base `main` / `48827173`, état initial propre :

| Référence | Observation et limite |
|---|---|
| `AGENTS.md`, consignes des crates desktop/adapters | Frontières, garde N1, recette humaine ; suspension des builds natifs ; aucune dérogation ancienne transférée. |
| `docs/architecture/ARCHITECTURE.md`, fiches desktop/adapters | Cartes statiques confrontées aux sources ; aucune autorisation de refonte. |
| `crates/whisper-desktop/src/root.rs:43` | `compose` lance `inspect(1)` sur un thread catalogue, séparé des préférences ; fenêtre visible, interface utilisable et catalogue disponible sont distincts. |
| `crates/whisper-adapters/src/worker_ipc/backends/mod.rs:115` | Snapshot global encore utilisé par `bind_live`, `bind_native_unified_live`, `bind_parakeet_live`, `is_parakeet_live_request` ; `bind_import` possède déjà un ciblage réutilisable. |
| `crates/whisper-adapters/src/model_catalog.rs:81` | Lecture/SHA dans `cache.sha256`, appelée sous le mutex global ; identité actuelle : chemin canonique, handle, taille, modification. |
| `crates/whisper-core/src/model_catalog.rs`, `engine_capabilities.rs`, `model_selection_app.rs` | Catalogue fermé, disponibilités et admission ; aucun état non inspecté actuellement. |
| `crates/whisper-desktop/src/ui/model/controller.rs`, `preferences.rs` | Projection du catalogue et choix persisté distincts ; ordre des réponses asynchrones à préserver. |
| `docs/architecture/INSPECT-CIBLEE-RESERVES-COMPLEMENT.md` | Mutex et inspections micro/démarrage différés ; pas de gain release ou recette Windows démontré pour ce lot. |
| `ci/guard-level1.ps1:28` | Cible externe INSPECT-CIBLEE encore fixée dans la garde ; correction préalable nécessaire pour une campagne de ce nouveau lot. |

Contributions reçues en lecture seule : `/root/framing` FRAMING v1 (CAP-01 à 11), `/root/requirements` REQUIREMENTS v1 (REQ/UC/AC), `/root/domain` DOMAINE v1 (concepts et invariants), `/root/architecture` ARCHITECTURE v1 (contrats techniques). L'auteur principal intègre ces contributions, sans revendiquer une copie octet pour octet. Les paquets bruts sont conservés dans les échanges de cette session. Le domaine avait reçu les exigences intermédiaires R01–R08 ; leur correspondance avec les REQ finales est inchangée.

## 2. Exigences et couverture

| Besoin/capacité | Exigence | Parcours | Acceptation | Responsabilité |
|---|---|---|---|---|
| CA-01 / CAP-01 | REQ-01 : ciblage au démarrage, catalogue et sélection préservés sans faux statut | UC-01 | AC-01/02 | Catalogue, root, présentation et garde core |
| CA-02 / CAP-02 | REQ-02 : cibler les quatre points micro, sans lecture intégrale étrangère | UC-02/03 | AC-03/04 | Backends et catalogue |
| CA-03 / CAP-03 | REQ-03 : lecture/SHA hors mutex global ; autre artefact indépendant de ce calcul | UC-04 | AC-05 | Cache adapters |
| CA-04 / CAP-04 | REQ-04 : revalidation d'identité et cache concurrent cohérent | UC-04 | AC-06/07 | Cache et admission |
| CA-05 / CAP-05 | REQ-05 : deux délais humains comparables avant/après, sans seuil inventé | UC-06 | AC-10 | Utilisateur, recette |
| CA-06 / CAP-06 | REQ-06 : CPU/GPU/Auto, replis et moteur effectif identifiables conservés | UC-02/03 | AC-04/08 | Politiques existantes |
| CA-06 / CAP-07 | REQ-07 : source micro, audio capturé, qualité et TXT/SRT/WAV conservés | UC-02/05 | AC-08/09 | Capture/publication existantes |
| CA-06 / CAP-08 | REQ-08 : interruption et générations existantes préservées | UC-05 | AC-09 | Application et admission |

UC-01 : lancement → fenêtre → interface utilisable ; catalogue vérifié progressivement à la demande. UC-02 : choix modèle/mode/source → préparation → capture active → transcription → arrêt/finalisation. UC-03 : inconnu, absent, altéré ou route indisponible → cause et repli existants. UC-04 : demandes simultanées et remplacement pendant inspection. UC-05 : arrêt/annulation/fermeture selon les commandes existantes ; aucune nouvelle reprise après panne. UC-06 : comparaison humaine sur le même PC et les mêmes configurations.

Les états visibles de capture et de moteur restent distincts. La vue unifiée expose déjà un état de démarrage, un arrêt/annulation et des compteurs capturés/durables ; le libellé de traitement ne prouve pas seul le début réel de capture.

## 3. Domaine et invariants

La définition connue d'un modèle ne dépend pas de sa présence sur disque. Le choix utilisateur ne constitue pas une preuve de disponibilité. Une observation d'intégrité porte sur une identité précise ; le cache est une optimisation volatile et ne déclenche aucune capture.

Transitions d'observation : non inspecté → inspection demandée → preuve stable ou erreur. Un cache valide peut fournir la preuve après contrôle de l'identité actuelle ; une identité différente nécessite une nouvelle inspection. Une classification de route n'est pas une admission. L'application admet uniquement une demande encore actuelle, selon les contrôles du parcours. La capture devient active sur son événement existant, distinct de l'état moteur.

- INV-01/02 [REQ-01/02/04] : toutes les définitions restent connues ; non inspecté ne signifie ni prêt, ni absent, ni corrompu, ni annulé. Un choix ultérieur peut demander sa vérification avant admission.
- INV-03 [REQ-03] : le SHA de A ne sérialise pas B sous le mutex global. Aucune durée murale particulière sur un disque partagé n'est garantie.
- INV-04/05 [REQ-04] : un changement détecté invalide l'observation ; les demandes sur une même clé convergent sans supprimer la déduplication actuelle ni bloquer les autres artefacts.
- INV-06/07 [REQ-06/08] : politiques et générations existantes préservées ; une réponse ancienne ne change pas le choix utilisateur ni ne réactive une session arrêtée.
- INV-08/09 [REQ-05/07] : aucune modification de la confirmation audio ou des mécanismes de publication ; mêmes bornes observables avant/après.

Persistance : aucun nouveau format, migration ou cache disque. Un arrêt de processus peut perdre un calcul et imposer son recalcul. Aucune promesse nouvelle de récupération ou d'annulation d'un appel disque n'est ajoutée.

## 4. Architecture et décisions techniques

### DEC-01 — Catalogue complet, inspection à la demande

Choix technique délégué par la mission de conception : conserver toutes les définitions avec un état explicite `ModelAvailability::Uninspected` et un refus d'admission `RefusalCause::ModelNotInspected`. Distinguer la sélection d'une intention de l'admission d'une transcription. Une entrée non vérifiée peut être choisie pour demander son inspection ; elle ne fournit jamais un artefact exécutable avant `Ready` et les qualifications requises.

Le snapshot initial contient les deux définitions du registre, chemins et empreintes attendus, observations absentes et état `Uninspected` ; sa construction ne lit aucun modèle intégralement. `CatalogSnapshot::engine_snapshot` traduit une observation absente en `Uninspected`, jamais en `Missing` par défaut.

Le modèle effectif issu du reducer de préférences existant, ou d'un choix utilisateur plus récent, est seul inspecté. Un changement de choix demande une inspection ciblée. Aucun SHA des autres modèles n'est lancé en arrière-plan sans besoin. Le catalogue complet demeure présenté ; seules les observations réellement produites sont fusionnées. Les générations de publication sont monotones ; avant publication, une réponse dont la révision d'intention est dépassée est ignorée pour la présentation/admission courante. Le cache peut conserver son calcul valide ; le choix et la session ne changent pas.

Étendre `ModelSelectionRuntime` avec un canal dédié `(ModelId, revision)` : boîte du dernier choix et réveil de capacité 1, regroupement des demandes, aucun thread par clic. Le service catalogue reste un thread séparé, extrait de `compose` vers `root/model_catalog.rs`. Il publie le catalogue non inspecté puis attend une intention, inspecte uniquement le modèle choisi et ses ressources de qualification nécessaires, vérifie la révision et publie le snapshot complet. La déconnexion arrête la boucle après le calcul engagé ; aucune préemption de lecture disque n'est promise.

Les helpers Parakeet `qualify_parakeet_catalog_entry` et `apply_parakeet_key_availability` ne s'appliquent qu'à une observation Parakeet effectivement réalisée. Ils ne transforment pas `Uninspected` en `Missing`. Leur surveillance existante est limitée au modèle actif déjà vérifié, sans SHA étranger.

Alternatives écartées : catalogue tronqué (perte des choix), faux Ready/Missing (admission ou diagnostic incorrect), inspection exhaustive différée sans demande (lecture étrangère conservée). Aucun changement de préférence persistée ou de format IPC n'est requis.

### DEC-02 — SHA coordonné par clé, hors verrou global

L'ouverture et la lecture de l'artefact sont séparées de l'index partagé. Le verrou global sert seulement à trouver/créer l'état associé à une clé et à accéder aux métadonnées mémoire. Un calcul en cours pour la même clé est partagé ; les autres clés progressent indépendamment. Aucune lecture intégrale, attente d'un autre calcul ou notification bloquante sous le verrou global.

La clé conserve le contrat existant chemin canonique + identité de handle + taille + modification. La déduplication existante sur une même clé reste requise ; aucun nouveau regroupement de chemins durs distincts n'est promis. Séquence : ouvrir hors global ; trouver/créer l'emplacement partagé sous global bref ; libérer global puis verrouiller l'emplacement ; revalider après toute attente ; retourner le digest valide ou calculer ; contrôler le nombre d'octets, réouvrir/revalider puis publier. Aucun emprunt du registre global ne traverse une IO. Ne jamais acquérir l'emplacement en tenant le global. Poison global/local : erreur explicite, sans récupération silencieuse. Une erreur de lecture n'est pas mémorisée comme succès ; les attentes sont libérées et une nouvelle demande peut retenter selon l'état réel, sans boucle infinie.

La revalidation garde la portée du mécanisme actuel : changements observables par l'identité et les métadonnées contrôlées. Elle ne prétend pas éliminer toute fenêtre TOCTOU ni détecter une mutation hostile restaurant toutes ces métadonnées. L'admission et les contrôles ultérieurs existants demeurent requis.

### DEC-03 — Bindings ciblés, frontières conservées

Extraire la résolution de chemin de `inspect_requested_model` en résolveur partagé sans lecture intégrale : mêmes chemins du registre et alias Turbo historique, aucun deuxième résolveur divergent. Ajouter une inspection ciblée par `ModelId` pour le démarrage, réutilisée par `inspect_requested_model`.

`bind_live` obtient un snapshot ciblé puis conserve le binding Turbo. `bind_parakeet_live` résout sans snapshot global puis réutilise `bind_import` ciblé, avec qualifications, langue, choix effectif et repli Auto inchangés. `bind_native_unified_live` résout sans lecture exhaustive et conserve modèle Parakeet requis, SHA demandé, chemin personnel et contrôles aval. `is_parakeet_live_request` résout sans lecture intégrale : classification seulement. Aucun contrôle aval n'est supprimé et une classification ne vaut pas admission.

### DEC-04 — Préalable de garde N1

Résoudre dans `ci/guard-level1.ps1` et ses auto-tests la racine partagée depuis le dépôt : `target/parakeet-tests/cargo-shared/`. Les cibles fraîches restent sous la racine du lot via `-FreshTarget`. Le mode sans variable utilise la même cible standard par `--config build.target-dir=...` injecté exclusivement par la garde, avec contrôle du `target_directory` effectif. Retirer l'exigence externe INSPECT-CIBLEE sans autoriser de cible externe arbitraire. Préserver catégories, contrôles canoniques/jonctions/environnement/PE et refus. Auto-tests positifs/négatifs variable/sans variable, standard/fraîche et refus externe avant toute campagne Cargo. Cette résolution est une tâche technique du plan proposé ; aucune garde n'est modifiée aujourd'hui, aucun watcher ajouté.

### Matrice de responsabilités

| Consommateur | Dépendances et rôle permis | Contrôle futur |
|---|---|---|
| Core | Types purs, état non inspecté, refus, politique/admission ; aucune IO/adapters/UI | Tests de projection/admission ; imports et manifestes inchangés |
| Adapters | Core ; observation disque, SHA/cache, binding | Tests concurrence/ciblage ; pas de dépendance desktop |
| Desktop UI | API application/types pour intentions et présentation ; pas de SHA/disque nouveau | Tests contrôleur/choix et refus avant Ready |
| Root | Assemble services/catalogue/préférences ; pas de politique métier parallèle | Tests chemin réel de démarrage/composition |
| Workers/bootstrap/capture/publication | Contrats existants consommés, sans refonte | Non-régression ciblée, aucune nouvelle dépendance |

Pas de dépendance, crate, runtime, code unsafe ou réseau ajouté. Rust 1.98.1, édition 2024, cible Windows x64 MSVC selon les fichiers versionnés ; sha2 0.10.9 et same-file 1.0.6 déjà déclarés. Aucune compatibilité matérielle nouvelle n'est revendiquée. L'implémentation devra extraire les responsabilités nouvelles des fichiers déjà volumineux plutôt que les agrandir sans borne.

## 5. Acceptation et risques

Toutes les exécutions produit restent **NOT RUN** dans cette mission documentaire.

| AC | Scénario / observation attendue |
|---|---|
| AC-01 | Vrai chemin de démarrage, configurations synthétiques : zéro lecture intégrale des modèles sans besoin ; ne pas tester seulement un helper isolé. |
| AC-02 | Catalogue complet ; choisir un modèle non inspecté présent, absent puis altéré : inspection ciblée, disponibilité finale exacte, admission interdite avant preuve, choix conservé. |
| AC-03 | Exercer chacun des quatre points réels avec modèles/routes compatibles : compteurs par artefact sans lecture étrangère, résultat lié au modèle demandé. |
| AC-04 | Modèle inconnu/absent/altéré, requête incompatible et route indisponible : causes et replis actuels préservés. |
| AC-05 | Suspendre le SHA froid de A ; B sur une autre clé termine avant la libération de A. Barrières déterministes ; timeout uniquement contre blocage. |
| AC-06 | Modifier/remplacer A pendant son calcul ou avant revalidation : digest périmé rejeté, aucune entrée invalide réutilisable, nouvel accès conforme à l'état réel. |
| AC-07 | Requêtes concurrentes même clé et alias, cache chaud, erreur de lecture puis nouvelle demande : résultats cohérents, déduplication existante, aucun attente résiduelle. |
| AC-08 | Phrase courte/silence, mêmes réglages et routes CPU/GPU/Auto : source/moteur/repli corrects, qualité comparable et sorties cohérentes. Preuve CPU ne vaut pas GPU. |
| AC-09 | Arrêt/annulation pendant attente puis capture, nouvelle génération : pas d'activation par résultat ancien ; audio annoncé capturé préservé selon contrats actuels. |
| AC-10 | Deux délais avant/après, bornes identiques approuvées et conditions déclarées : valeurs individuelles, médiane, min/max et limites ; absence de gain possible et rapportée. |

RISK-F01 : faux statut catalogue → DEC-01, AC-01/02. RISK-F02 : contrôle/repli perdu dans un binding → DEC-03, AC-03/04/08. RISK-F03 : résultat obsolète → DEC-02, AC-06/07/09. RISK-F04 : comparaison de bornes/caches différents → protocole et AC-10. RISK-F05 : capture anticipée introduite implicitement → exclusion et AC-08/09. RISK-T01 : verrouillage circulaire/attente après erreur → ordre de verrous et AC-05/07. RISK-T02 : garde N1 encore liée à l'ancien lot → préalable de plan, aucun Cargo avant sa résolution.

| Classe de preuve | État et propriétaire |
|---|---|
| DESIGN_FEASIBILITY | PARTIAL avant revue : primitives et inspections ciblées déjà présentes, pas de nouvelle pile ni build natif ; contrats précis soumis au reviewer. Aucun essai ou gain revendiqué. |
| PRODUCT_VALIDATION | NOT RUN : implémenteur pour tests synthétiques, utilisateur pour capture/qualité et deux délais. |
| DELIVERY_QUALIFICATION | NOT RUN : profil réellement compilé, routages et SHA des ressources à contrôler avant remise ; aucune reconstruction native permise. |

## 6. Mesures et décisions restantes

**DEC-P01 acceptée le 6 novembre dans cette session :** à la question capture réellement active ou moteur prêt, l'utilisateur répond « Capture réellement active (recommandé) ». La borne micro est donc clic → capture active de la session courante, constatée par progression du compteur d'échantillons capturés ; le signe exact par route est vérifié avant référence. Ne pas remplacer cette borne par le premier texte ou l'état moteur.

| Question | Classe / owner | Proposition et échéance |
|---|---|---|
| Q-F01 | Technique structurante / architecte | Résolue par DEC-01, soumise à revue ; pas de catalogue tronqué ni de faux statut. |
| Q-M01 | Détail mesure / utilisateur | Lancement → interface utilisable, apparition de fenêtre secondaire ; validation avant référence. |
| Q-M03 | Détail mesure / utilisateur | Comparaison sans seuil imposé ; toute cible absolue reste une décision explicite avant référence. |
| Q-M04 | Détail protocole / utilisateur, préparation implémenteur | Six mesures par configuration retenue, même PC/profil/modèle/mode/source ; validation avant référence. Distinguer nouveau processus et cache OS chaud : aucun disque froid prétendu sans protocole. |

Le protocole sera arrêté avant toute modification produit affectant les mesures. Conserver versions des exécutables et ressources, configuration et méthode humaine. Aucun outil de chronométrage, hook de produit ou campagne supplémentaire n'est créé par ce cadrage.

## 7. État et passage au plan

DESIGN : DRAFT, en attente de revue indépendante des fichiers Git. Les questions de mesure ne bloquent pas la conception ; elles bloquent seulement la référence humaine. Aucun essai technique natif ni nouveau railguard n'est nécessaire à cette évolution bornée. Les règles applicables restent celles des AGENTS existants.

Le plan doit rester à dix tâches maximum, couvrir chaque AC et nommer les chemins exacts, le préalable de garde, la référence humaine avant code, les vérifications, la revue indépendante et la remise. La demande de planification est explicite ; ni un verdict CLEAN ni le futur plan ne donnent l'autorisation de coder.
