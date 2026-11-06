Avancée : plan proposé en dix tâches, relié au cadrage revu CLEAN.
Blocage : revue PLANS à effectuer ; protocole et autorisation d'exécution restent requis.
Décision : préparer l'implémentation sans la lancer.

# INSPECT-MICRO-DEMARRAGE — plan v1

## 1. Parents et portée

Parent : [cadrage](evo-inspect-micro-demarrage.md), Git `09f5f976` ; contrats revus CLEAN sur `426c16ec` par `/root/design_review`, D-01 CLOSED après un cycle. [Relevé de revue](revue-inspect-micro-demarrage.md). Code produit de référence : `48827173`. Contribution PLAN v1 de `/root/plan`, lecture seule, intégrée par l'auteur principal. Autorité : demande du 6 novembre 2026 « ok defini le cadrage et le plan d'implementation a partir de ce besoin ».

Ce plan applique DEC-01 à DEC-05 : ciblage démarrage/micro, connaissance non inspectée explicite, SHA hors mutex global et revalidation, commandes explicites sans rejeu automatique. Il préserve CPU/GPU/Auto, sélection micro, intégrité, audio, qualité et sorties. Aucun nouveau moteur/modèle, dépendance, feature, unsafe, accès réseau, service, watcher, installation ou build natif. SHA workers, algorithmes file/reprise et formats durables inchangés.

**Planification autorisée ; implémentation non autorisée.** Avant exécution : plan accepté et accord écrit précis dans la session courante couvrant lot, chemins, campagnes et dossiers de remise. Push séparé. Aucun travail ci-dessous n'a été exécuté dans cette mission documentaire.

## 2. Dix tâches séquentielles

Même code/contrats partagés, aucune parallélisation d'édition. Coûts indicatifs, hors attente utilisateur et diagnostics.

| ID | Travail, owner et sortie | Prérequis typés | Couverture / coût |
|---|---|---|---|
| T1 — Préflight | Principal : Git/design/plan/revues, accord, responsabilités concurrentes, AGENTS, espace ≥30 Gio, outils/ressources. Arrêter Q-M01/03/04 et identifier provenance du témoin. | DOCUMENT : cadrage revu, plan accepté, accord courant | Tous ; 20–40 min |
| T2 — Garde et témoin | `rust_implementer` : garde N1 et auto-tests DEC-04, cible standard interne et mode sans variable, refus externe. Auto-tests avant Cargo. Préparer témoin Rust attribuable à `48827173` si aucun binaire équivalent vérifiable ; aucune modification produit ou reconstruction DLL. | EXECUTION : T1 ; ressources de build présentes | Préalable tous ; 45–90 min + build |
| T3 — Référence humaine | Utilisateur : mesurer avant avec protocole figé, vérifier signe de capture active. Principal : relever valeurs, configurations, profil et provenance. Aucun correctif produit avant cette référence. | CODE/ARTEFACT : témoin T2 ; EXECUTION : protocole/ressources vérifiés | REQ-05 / AC-10 ; 20–60 min |
| T4 — Cache SHA | `rust_implementer` : index global bref, coordination par clé, IO/SHA hors global, revalidation après attente et lecture, erreurs/poison/compteurs. Tests séparés concurrence, mutation et reprise après erreur. | EXECUTION : T2/T3 ; DOCUMENT : DEC-02 | REQ-03/04 / AC-05/06/07 ; 2–4 h |
| T5 — Quatre bindings | `rust_implementer` : résolveur commun sans lecture intégrale, inspection par ModelId, ciblage des quatre vrais points et contrôles aval/replis inchangés. Incrément vérifiable par tests synthétiques. | CODE : T4 ; EXECUTION : tests T4 réussis | REQ-02/06 / AC-03/04 ; 1–3 h |
| T6 — Démarrage/admission/UI | `rust_implementer` : incrément cohérent core/adapters/desktop : Uninspected/refus, catalogue complet, service extrait, boîte bornée sélection/commandes, versions et publication, contrôleur/widget, helpers Parakeet. DEC-05 vrai wrapper, avis de relance sans commande/proof GPU conservée. Tests vrais consommateurs. | CODE : T5 ; DOCUMENT : DEC-01/05 | REQ-01/06/08 / AC-01/02/09/11 ; 3–6 h |
| T7 — Vérifications | `rust_implementer` : diff, format, ciblés, référence N1 CPU, build Rust et lints via garde. PE des harnais workers avant exécution. Résultats attribuables au candidat, sans validation matérielle/fenêtre déduite. | CODE : T4–T6 ; EXECUTION : préflight N1 renouvelé | Tous automatisables ; 45–120 min + build |
| T8 — Revue/corrections | Principal confie le candidat à `rust_review_coordinator` ; reviewer indépendant, corrections par `rust_implementer`, deux cycles maximum. Rejouer contrôles affectés puis vérifications finales requises. Reviewer seul ferme/déclare CLEAN ; sinon LIVRÉ AVEC RÉSERVES. | EXECUTION : T7 réussi, diff/preuves attribuables | Tous ; 1–3 h |
| T9 — Remise/après humain | Principal : remise intra dépôt du profil réellement construit, routages/ressources contrôlés. Utilisateur : CPU/GPU/Auto accessibles, audio/sorties/interruption et mesures après comparables T3. Aucun lancement fenêtre ou installation par l'agent. | CODE : candidat revu ; EXECUTION : build/remise/ressources équivalents T3 | AC-08/09/10 ; 30–90 min + build |
| T10 — Clôture | Principal : bilan une page, résultats/limites, archivage durable hors target, rétention applicable, diff/commits locaux. Nettoyage seulement exact, sûr et autorisé selon AGENTS ; préserver caches/cible partagés. Aucun push. | EXECUTION : résultats T7–T9 ou réserves rapportées | Tous ; 20–45 min |

Graphe sans cycle : T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9 → T10. Un exécutable installé ancien non attribuable à la base ne prouve pas la comparaison du lot. T2 peut construire les exécutables Rust utiles via garde corrigée ; T3 reste antérieur au premier correctif produit T4.

## 3. Chemins proposés

Relatifs à `C:/dev/whisper`. Aucun chemin supplémentaire implicite. Les nouveaux tests restent séparés du code de production ; déclarations incluses ci-dessous.

| Groupe | Chemins modifiables | Tâche / contrat |
|---|---|---|
| G1 | `ci/guard-level1.ps1`, auto-tests intégrés | T2 / DEC-04 |
| G2 | `crates/whisper-core/src/model_catalog.rs` ; `crates/whisper-core/src/engine_capabilities.rs` ; `crates/whisper-core/tests/model_catalog.rs` | T6 / connaissance, projection/admission |
| G3 | `crates/whisper-adapters/src/model_catalog.rs` ; `crates/whisper-adapters/src/model_catalog/tests.rs` ; nouveau `crates/whisper-adapters/src/model_catalog/concurrency_tests.rs` | T4/T5 / SHA, résolution, inspection ; déclaration dans model_catalog.rs |
| G4 | `crates/whisper-adapters/src/worker_ipc/backends/mod.rs` ; `crates/whisper-adapters/src/worker_ipc/tests.rs` ; nouveau `crates/whisper-adapters/src/worker_ipc/tests/live_inspection.rs` | T5 / vrais quatre points |
| G5 | `crates/whisper-desktop/src/root.rs` ; nouveaux `crates/whisper-desktop/src/root/model_catalog.rs` et `crates/whisper-desktop/src/root/model_catalog/tests.rs` | T6 / service compose et vrai wrapper |
| G6 | `crates/whisper-desktop/src/ui.rs` ; `crates/whisper-desktop/src/ui/model.rs` ; `crates/whisper-desktop/src/ui/model/preferences.rs` ; `crates/whisper-desktop/src/ui/model/controller.rs` ; `crates/whisper-desktop/src/ui/presentation/selection.rs` | T6 / transport, intentions, présentation |
| G7 | `crates/whisper-desktop/src/ui/tests.rs` ; nouveau `crates/whisper-desktop/src/ui/tests/catalog_inspection.rs` ; `crates/whisper-desktop/tests/model_selection_ui.rs` | T6/T7 / contrôleur et widget |
| G8 | `docs/cadrage/evo-inspect-micro-demarrage.md` ; présent plan ; `docs/cadrage/revue-inspect-micro-demarrage.md` ; futur `docs/cadrage/bilan-inspect-micro-demarrage.md` ; future archive `docs/cadrage/raw/inspect-micro-demarrage.zip` | Principal seul ; suivi et clôture ; cinq livrables, aucun manifeste séparé |
| G9 | `target/parakeet-tests/inspect-micro-demarrage/` ; cible/cache partagés N1 ; proposition `deliverables/inspect-micro-demarrage/avant/` et `apres/` | Campagne/remise intra dépôt, binaires non commités |

`Cargo.toml`, `Cargo.lock`, AGENTS, protocoles workers, handlers queue/recovery, capture/publication ne sont pas en édition. Les scripts PE existants, leurs fixtures, manifestes et toolchain sont lus/utilisés sans modification. Tout chemin nouveau nécessite un PLAN_CHANGE ; décision structurante → architecte/DESIGN_CHANGE_REQUIRED. Ne pas étendre automatiquement le lot. Respecter les cibles de structure AGENTS et réduire `root.rs` par extraction bornée.

## 4. Préflight et commandes futures

**Tout est NOT RUN.** PowerShell 7, cwd `C:/dev/whisper`. Cible standard `target/parakeet-tests/cargo-shared/`, cache persistant `target/cargo-home-shared/` : présence observée le 6 novembre, contenu/canonicalité/jonctions à vérifier avant exécution. Aucune cible externe INSPECT-CIBLEE transférée. Les cibles fraîches restent sous G9 via `-FreshTarget` ; elles ne sont pas nécessaires par défaut.

Avant premier Cargo : contrôle des responsabilités Git et changements, outil/cible/features/dépendances natives par commande ; états READY/MISSING/NOT CHECKED/NOT REQUIRED attribués. La garde isole TEMP/TMP/LOCALAPPDATA, contrôle racines/environnement/espace, fixe `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, capture stdout/stderr/codes. Aucun Cargo direct ; aucune modification de profil/config utilisateur. Le cache hors ligne absent donne un diagnostic, pas un téléchargement implicite.

Après correction T2, commandes proposées :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/inspect-micro-demarrage
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode SelfTest -WithoutCargoTargetDir -LotRoot target/parakeet-tests/inspect-micro-demarrage
pwsh -NoProfile -File ci/check-worker-imports.ps1 -SelfTest
```

Attendu : cas positifs acceptés, chemins externes/évasions/jonctions/arguments interdits refusés, cible effective contrôlée sans variable. Témoin PE réel facultatif : faute de sélection explicite et SHA, annoncer « témoin réel : NON EXÉCUTÉ ».

Préfixe commun, remplacer seulement la valeur JSON selon le tableau :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -LotRoot target/parakeet-tests/inspect-micro-demarrage -CargoArgsJson '["fmt","--check"]'
```

| Usage | Valeur de `-CargoArgsJson` |
|---|---|
| Format | `["fmt","--check"]` |
| Cache/concurrence | `["test","--offline","--locked","-p","whisper-adapters","concurrency_tests"]` |
| Bindings réels | `["test","--offline","--locked","-p","whisper-adapters","live_inspection"]` |
| Catalogue adapters | `["test","--offline","--locked","-p","whisper-adapters","model_catalog"]` |
| Core projection/admission | `["test","--offline","--locked","-p","whisper-core","--test","model_catalog"]` |
| Service réel/wrapper | `["test","--offline","--locked","-p","whisper-desktop","root::model_catalog::tests"]` |
| Contrôleur | `["test","--offline","--locked","-p","whisper-desktop","catalog_inspection"]` |
| Widget | `["test","--offline","--locked","-p","whisper-desktop","--test","model_selection_ui"]` |
| Clippy | `["clippy","--offline","--locked","-p","whisper-core","-p","whisper-adapters","-p","whisper-desktop","--all-targets"]` |
| Build desktop personnel | `["build","--offline","--locked","--release","-p","whisper-desktop","--features","personal-install"]` |
| Workers Turbo remise | `["build","--offline","--locked","--release","-p","whisper-worker-cpu","-p","whisper-worker-gpu"]` |

Les modules/tests nouveaux portent ces noms de filtres ; vérifier nombre et noms réellement exécutés, jamais zéro test présenté comme succès. Le test root compile le module consommé par le vrai binaire/composition, sans remplacer le wrapper par un helper isolé. L'injection de lecteur/qualification reste privée, sans API publique ou champ/branche produit réservé aux tests.

N1 final : une campagne, dans un shell dédié, sans `CARGO_TARGET_DIR`, cible fixée par la seule config interne de garde :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -WithoutCargoTargetDir -LotRoot target/parakeet-tests/inspect-micro-demarrage -CargoArgsJson '["test","--offline","--locked","--no-fail-fast"]'
```

Cette sélection couvre les membres par défaut. Ajouter les trois sélections CPU de référence ; commencer par compilation seule via le même préfixe de garde :

```text
["test","--offline","--locked","-p","whisper-worker-cpu","--no-run"]
["test","--offline","--locked","-p","whisper-worker-parakeet-cpu","--no-run"]
["test","--offline","--locked","-p","whisper-worker-common","--no-run"]
```

Relever les chemins exacts de tous les harnais affichés. Avant leur première exécution : `pwsh -NoProfile -File ci/check-worker-imports.ps1 -Kind Cpu -Path '<harnais exact>'` pour CPU/common, `-Kind ParakeetCpu` pour Parakeet CPU. Puis lancer via garde les trois `test` correspondants sans `--no-run`. Les inventaires éventuels utilisent `-- --list` après PE, aucun corps GPU en N1. Aucun triplement de campagne historique. Aucun `native-tests`, `recette-hooks` release, argument inconnu ou téléchargement ajouté.

La présence des ressources natives nécessaires à un build Rust est vérifiée avant celui-ci ; absence → blocage de cette tâche, aucune reconstruction native. Un exécutable préexistant utilisé pour la remise est identifié par provenance/profil et contrôlé avant/après ; la présence d'un fichier ne suffit pas.

## 5. Carte d'exécutabilité

Toutes les lignes sont NOT RUN. Petites fixtures synthétiques pour les tests automatiques. Coût après compilation : quelques secondes par sélection, sauf recette humaine.

| AC / tâches / REQ | Chemins directs/indirects et données | Observation attendue / owner et commande |
|---|---|---|
| AC-01 / T6–7 / R01 | G3/G5/G6 ; service appelé par compose, lecteur/qualification privés injectés | Catalogue complet ; seul besoin lu, qualification incluse, zéro lecture étrangère ; implémenteur, filtre root |
| AC-02 / T6–7 / R01 | G2/G3/G5/G6/G7 ; présent/absent/altéré non inspecté | Choix possible, refus avant preuve, état exact ; core + root + contrôleur/widget |
| AC-03 / T5–7 / R02 | G3/G4 ; demandes Turbo/Parakeet compatibles, alias historique | Chacun des quatre points réels, compteurs distincts et bon modèle ; `live_inspection` |
| AC-04 / T5–7 / R02/R06 | G3/G4 et gardes core ; inconnu/absent/altéré, langue/SHA/route incompatibles | Refus/repli existants, contrôles aval conservés ; `live_inspection` et tests existants utiles |
| AC-05 / T4–7 / R03 | G3 ; A froid suspendu, B autre clé | B termine avant libération A, timeout anti-blocage seulement ; `concurrency_tests` |
| AC-06 / T4–7 / R04 | G3 ; remplacement/modification après attente et pendant lecture | Rejet identité et aucun digest périmé réutilisé ; `concurrency_tests` |
| AC-07 / T4–7 / R04 | G3 ; même clé/alias/cache chaud/erreur lecture/panic maîtrisée | Dédup actuelle, erreurs explicites, pas attente résiduelle ; concurrence + catalogue existant |
| AC-08 / T7/T9 / R06/R07 | G4/G5/G6 ; capture/publication/workers consommés, phrase/silence fixes | Source/moteur/repli, qualité et TXT/SRT/WAV cohérents ; tests utiles puis utilisateur Windows, 10–30 min/config |
| AC-09 / T6–7/T9 / R07/R08 | G5/G6/G7 ; application/générations existantes, arrêt/fermeture/retour tardif | Pas de lancement ancien, audio préservé sans promesse panne ; root/contrôleur puis utilisateur 10–20 min |
| AC-10 / T3/T9 / R05 | G9, mêmes ressources/PC/profil/modèle/mode/source/cache déclaré | Deux délais, valeurs individuelles/médiane/min/max, limites, gain ou absence ; utilisateur 20–60 min |
| AC-11 / T6–7 / R06/R08 | G5/G6/G7 ; vrai DesktopAdmissionApplication, requêtes durables en fixtures A choisi/B non inspecté | File et reprise séparées : demande B/avis sans effet ni proof GPU ; publication sans démarrage ; nouveau clic recontrôlé, A inchangé ; variants cadrage ; root + contrôleur |

Avant compilation, rechercher tous les consommateurs de `ModelAvailability` et `RefusalCause`, notamment transformation snapshot/engine, projection/admission, présentation et qualification Parakeet, puis les rapprocher de G2/G3/G5/G6. Tout consommateur nécessitant un chemin absent est un PLAN_CHANGE, pas une permission implicite.

## 6. Protocole, remise, arrêt et clôture

DEC-P01 acceptée : clic micro → **capture réellement active**, constatée sur la session/source courante par progression des échantillons. Avant T3 : identifier précisément cet indicateur pour chaque route retenue. Restent à valider avec l'utilisateur : Q-M01 lancement → interface utilisable ; Q-M03 comparaison sans seuil ; Q-M04 configurations et six répétitions proposées. Même méthode avant/après, distinction processus neuf/cache OS, aucun disque froid supposé. Aucune installation ou fenêtre n'est lancée par cette mission documentaire.

Profil proposé identique avant/après : `personal-install`. Dossiers G9 : desktop et deux workers Turbo à côté. Workers Parakeet, leurs DLL et modèles chargés depuis l'installation stable effective selon AGENTS : vérifier présence, routage et SHA requis avant remise. Ne pas ajouter de copies Parakeet/DLL que le profil n'ouvre pas. Préserver le dossier avant pour éviter écrasement par le build après. Aucun binaire commité. Les accès/écritures nécessaires à la recette Windows sont réalisés par l'utilisateur ; toute installation agent hors dépôt demanderait un accord distinct.

Ni garde ni lanceur ne créent l'état produit (settings/logs/config) pour masquer un défaut de premier lancement. Un manque est un constat produit ; toute correction hors chemins prévus revient à l'arbitrage approprié.

Arrêt avant compilation/campagne : espace <30 Gio, racine/variable/jonction invalide, garde ou prérequis défaillant. Sur compilation/test en échec ou changement invalidant les entrées, arrêter la campagne, préserver les preuves et demander l'arbitrage nécessaire ; aucun reset/restauration/suppression global. Un git.exe inconnu ou une sortie temporairement silencieuse ne suffit pas à déclencher l'arrêt ; appliquer la politique de concurrence AGENTS. Diagnostic : trois essais ou trente minutes maximum.

Avant clôture, les preuves utiles quittent target vers G8 ; archive unique, vérification nombre/SHA selon AGENTS, sans manifeste séparé. Identifier chemins exacts, appartenance au lot, absence de liens/processus consommateurs avant tout nettoyage ; préserver cible/cache partagés, modèles, sources, livrables et ressources de contrôles permanents. Aucun nettoyage n'est requis pour annoncer un résultat de mesure. Bilan d'une page figé à clôture, tout complément dans un nouveau document.

Le bilan distingue diff, vérifications, revue, validation humaine, performance et limites NON EXÉCUTÉES. N1 vert ne prouve ni GPU matériel ni qualité Windows. La revue documentaire du présent plan ne confère aucune autorisation de coder.
