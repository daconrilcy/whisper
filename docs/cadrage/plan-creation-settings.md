Avancée : correctif implémenté ; 359 tests PASS, zéro échec ; revue indépendante CLEAN.
Blocage : aucun pour le code ; recette native et nouvelle remise non exécutées.
Décision : livrer en commit local ; garde et lanceur inchangés, aucune installation ni push.

# Création du dossier settings par le produit

Autorisation explicite de la session : « ok implemente jusqu'a Clean », après
versionnement du plan dans `4eef6c09`. Les limites et arrêts prévus restent
applicables ; deux cycles de revue/correction maximum. Espace libre relevé
au lancement : 80,89 Gio. Aucun autre changement suivi préalable.

## Objectif et constat

Une installation personnelle ou une remise portable doit pouvoir préparer les
journaux Parakeet sans que la garde, un installateur ou le lanceur ait créé
`Whisper/settings` au préalable. Ce correctif ne doit pas créer de préférences
par défaut ni modifier un `settings.json` existant.

Base observée : `main`, `1b76a7ea` (message de démarrage déjà commité et revu
CLEAN). Aucun fichier suivi modifié ; `models/` et `sound example/` non suivis
à préserver. Espace libre observé : environ 81,93 Gio. Aucun Cargo exécuté
pour rédiger ce plan.

Preuves dans les sources actuelles :

- `crates/whisper-adapters/src/parakeet_import.rs:679-733`,
  `worker_stderr_log` : pour un runtime personnel, le parent est
  `runtime/settings`, puis seul `create_dir(log_root.join("logs"))` est appelé.
  L'absence de `settings` provoque `WorkerLogPath` avant le lancement du worker.
- `parakeet_import/legacy_runtime_tests.rs:53` crée déjà `settings` dans
  `personal_cpu_and_gpu_do_not_require_l3b`, masquant ce prérequis implicite.
- `ci/guard-native.ps1:1105-1113` prépare ce parent dans le RunRoot ; le bilan
  `docs/cadrage/GARDE-NVML.md:27` consigne l'échec puis la reprise réussie après
  ce contournement. Le lanceur portable prépare aussi ce parent.
- `path_settings.rs:299` crée déjà les parents lors d'une sauvegarde de
  préférences. Cela ne garantit pas leur existence avant le premier journal.

## Périmètre et comportement attendu

Corriger le propriétaire de l'écriture du journal, dans les adaptateurs,
plutôt qu'ajouter un ordre d'initialisation obligatoire au desktop. Pour la
branche personnelle, préparer le parent `settings` avant `logs`, puis ouvrir
le journal comme aujourd'hui. Une petite fonction privée est possible si
elle rend les tests directs et évite une duplication ; aucune API publique
nouvelle uniquement pour les tests.

Conserver la sélection du runtime, la priorité d'un `session_log_root`
explicite, les chemins historiques L3, les préfixes et noms uniques de journaux,
`create_new`, les contrôles canoniques et le traitement des erreurs. Ne pas
remplacer globalement les créations historiques par `create_dir_all` ni
relâcher les vérifications de racine. Ne jamais considérer `AlreadyExists`
comme preuve suffisante qu'un composant est un dossier utilisable.

La création ne concerne que les répertoires utiles au chemin de journal actif :
pas d'initialisation globale des profils CPU qui ne demandent aucun journal.
Le runtime personnel doit déjà exister et être reconnu par les contrôles
actuels ; le correctif ne crée pas une nouvelle installation.

## Critères d'acceptation

| ID | Critère vérifiable |
| --- | --- |
| AC1 | Runtime personnel valide sans `settings` : la préparation du journal crée `settings/logs` et un journal ouvrable, sans erreur de parent absent. |
| AC2 | `settings` existe sans `logs` : création réussie ; dossiers et journal déjà présents : nouvel appel réussi sans troncature ni remplacement. |
| AC3 | Un `settings.json` et des journaux existants conservent leur contenu ; aucun fichier de préférences n'est créé par ce correctif. |
| AC4 | `settings` ou `logs` est un fichier ordinaire : refus explicite, sans suppression, remplacement, ni tentative de lancement natif. |
| AC5 | Le journal reste sous sa racine canonique autorisée ; conserver les refus existants de chemins hors racine et ne pas accepter un lien redirigeant les écritures hors runtime. Aucune ACL, élévation ni création de jonction requise pour les tests N1. |
| AC6 | Parcours personnel CPU sans journal, racine de session explicite et routages historiques inchangés ; aucun effet sur CPU/GPU/Auto, IPC, modèles, qualifications ou politique de repli. |
| AC7 | Le test personnel existant ne précrée plus `settings` : avec son faux exécutable invalide, il atteint toujours `WorkerStart` pour les deux variantes et démontre que le GPU a préparé son journal auparavant. Aucun moteur natif n'est exécuté. |

## Six tâches d'implémentation après accord

1. **Préflight, session principale.** Vérifier HEAD, changements concurrents,
   consignes racine/adapters, espace libre supérieur à 30 Gio et périmètre
   accepté. Lire ce plan versionné ; préserver tous les non-suivis.
2. **Correctif, `rust_implementer` en mode IMPLEMENT.** Ajouter la création
   manquante au seul chemin personnel de journal et vérifier l'ordre des
   contrôles de type/racine ; garder les erreurs contextualisées existantes.
3. **Tests, même implémenteur.** Ajouter des tests séparés pour AC1-6,
   puis modifier le témoin personnel existant pour AC7. Utiliser des fichiers
   factices et les temporaires isolés du lot ; si `LOCALAPPDATA` doit varier,
   reprendre le sous-processus de harnais existant avec environnement propre,
   sans mutation globale concurrente ni nouveau programme auxiliaire.
4. **Vérification N1.** Via `ci/guard-level1.ps1`, racine de lot
   `target/parakeet-tests/creation-settings`, cible canonique
   `target/parakeet-tests/cargo-shared`, CARGO_HOME/TEMP/TMP/LOCALAPPDATA isolés.
   SelfTest préalable ; transmettre les arguments/variables en vrais tableaux
   PowerShell. Exécuter `cargo fmt --check`,
   `cargo test --offline --locked -p whisper-adapters`,
   `cargo check --offline --locked -p whisper-desktop` et
   `cargo clippy --offline --locked -p whisper-adapters --all-targets`.
   Aucun `native-tests`, `--ignored`, modèle ou worker natif valide exécuté.
   Un contrôle statique des limites complète les cas N1 sans annoncer une
   qualification Windows des jonctions. Arrêt et arbitrage selon AGENTS après
   violation du préflight ou échec de compilation/test ; aucune exception
   de reprise du lot précédent n'est reconduite tacitement.
5. **Revue, `rust_review_coordinator`.** Reviewer indépendant en lecture seule,
   corrections par le même implémenteur ; deux cycles maximum, verdict CLEAN
   ou réserves explicites. Il vérifie notamment AC5 et l'absence de changement
   aux routages historiques. Une exigence nouvelle dépassant ce plan revient
   à la session principale pour arbitrage.
6. **Clôture, session principale.** Compléter le présent document avec fichiers,
   résultats réels, limites et espace libre ; vérifier le diff et commiter
   localement. Conserver les journaux dans la racine du lot, sans nouvelle
   campagne documentaire ni archive d'empreintes imposée.

## Chemins autorisés et exclusions

Trois fichiers Rust maximum :

- `crates/whisper-adapters/src/parakeet_import.rs` : correction et déclaration
  du nouveau module de tests, sans refonte du fichier ;
- `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs` : témoin
  personnel existant sans précréation du parent ;
- `crates/whisper-adapters/src/parakeet_import/worker_log_tests.rs` : nouveaux
  tests privés séparés, si nécessaires.

Le présent document est le seul rapport prévu. Tout autre chemin ou dépassement
de dix tâches/fichiers de rapport nécessite arbitrage. Pas de manifeste,
Cargo.lock, dépendance, unsafe, code de worker, changement de garde, lanceur,
installateur, ressource installée, modèle ou audio. Aucun nettoyage, push,
compilation release, installation ou qualification matérielle dans ce lot.

## Remise ultérieure et retour arrière

La garde et le lanceur peuvent conserver leur préparation redondante : leur
suppression n'est pas nécessaire pour corriger le produit et n'est pas incluse.
Les tests doivent donc prouver le correctif sans s'appuyer sur cette préparation.
Une recette native réelle, puis un nouveau paquet incluant aussi le correctif
du message de démarrage, feront l'objet d'un accord distinct. Leur scénario
devra partir d'un profil de recette sans `settings`, sans supprimer les
préférences de l'installation utilisateur ; ils ne sont pas exécutés par ce plan.

Si le correctif doit être abandonné après commit, proposer un `git revert`
ciblé à l'utilisateur. Aucune restauration destructive ni suppression de
réglages pour revenir en arrière.

## Arrêt de vérification et arbitrage

SelfTest N1 : 30 positifs et 58 négatifs PASS. Formatage final et
`git diff --check` PASS. Le filtre `parakeet_import::` prévu dans la commande
de tests n'est pas accepté par la liste d'autorisation N1 ; refus avant
lancement des tests, campagne arrêtée. Aucun check/Clippy exécuté ensuite.
Proposition : remplacer la commande filtrée par
`cargo test --offline --locked -p whisper-adapters`, autorisée par la garde,
sans modifier celle-ci ni activer les tests natifs. Cela élargit l'exécution
aux tests N1 de la crate et nécessite accord explicite de reprise.
Accord explicite reçu ensuite dans la session : reprise autorisée avec toute
la suite N1 de `whisper-adapters`, sans garde modifiée ni tests natifs activés.

Cette suite a ensuite exécuté 279 tests unitaires : 278 PASS, un échec,
48 ignorés. Le nouveau journal est créé sous `settings/logs` ; le test compare
son chemin canonique Windows (préfixe étendu) à un attendu lexical.
Première revue indépendante : R1 BLOQUANT, canonicaliser aussi le dossier
attendu et conserver l'assertion de confinement. Aucun autre constat
actionnable ; check desktop et Clippy non exécutés après cet arrêt.
Accord de correction/reprise demandé conformément à AGENTS ; aucun CLEAN
revendiqué pour ce candidat intermédiaire.
Accord explicite reçu ensuite : correction de la comparaison par
canonicalisation du chemin attendu, conservation de l'assertion de
confinement, puis reprise tests, compilation, Clippy et même reviewer.

## Bilan final — 2 novembre 2026

Le client Parakeet prépare maintenant `settings` avant `logs` lorsque son
chemin personnel de journal est actif. Il vérifie les répertoires canoniques
et refuse un fichier à la place d'un dossier. Réglages et journaux existants
ne sont ni remplacés ni tronqués ; le correctif ne crée pas de préférences.
Les branches session explicite et historiques restent inchangées. Le CPU
personnel qui ne demande pas de journal n'initialise pas ce dossier inutilement.

Fichiers modifiés :

- `parakeet_import.rs` : préparation du parent personnel, contrôles de dossier
  et de racine, déclaration du module de tests ;
- `parakeet_import/legacy_runtime_tests.rs` : retrait de la précréation du
  parent dans le témoin personnel CPU/GPU et vérification des effets attendus ;
- `parakeet_import/worker_log_tests.rs` : tests séparés de création, répétition,
  préservation, refus des fichiers et variantes de routage ;
- le présent document : accords, résultats et limites du lot.

Vérifications finales sous `ci/guard-level1.ps1`, lot
`target/parakeet-tests/creation-settings`, cible partagée canonique,
environnement isolé, offline/locked lorsque applicable :

| Commande | Résultat | Dossier final sous `scratch/` |
| --- | --- | --- |
| SelfTest | 30 positifs / 58 négatifs PASS, acquis avant Cargo | journaux du lot |
| `cargo fmt --check` | PASS | `20261102T143059213-0eecb39163234cc3b0c3a2b828cdf4f0` |
| `cargo test --offline --locked -p whisper-adapters` | 359 PASS / 0 échec / 118 ignorés, 33 résumés | `20261102T143110469-6f9c82c270af458eab2618a73b303614` |
| `cargo check --offline --locked -p whisper-desktop` | PASS | `20261102T143150624-dc0e1068dbd3471b9262ff0ec4893869` |
| `cargo clippy --offline --locked -p whisper-adapters --all-targets` | PASS ; avertissements hors delta, aucun sur les fichiers modifiés | `20261102T143221078-ab36766a2cb54c94abf44bf1d38ae182` |

Les dossiers finaux contiennent `cargo-command.stdout.log` et
`cargo-command.stderr.log`. `git diff --check` PASS. Revue indépendante :
deux passes, un cycle de correction, R1 fermé, verdict **CLEAN**.
Espace libre de clôture : environ 77,95 Gio. Modèles et audios non suivis
préservés ; aucun manifeste, lockfile, garde ou lanceur modifié.

Limites conservées : les tests utilisent des fichiers factices et les harnais,
sans moteur natif, modèle ni transcription réelle. Le confinement des chemins
est revu statiquement ; aucune qualification de jonction Windows réelle.
Aucune compilation release, ouverture de fenêtre, installation ou nouvelle
archive portable. La recette de premier démarrage sans `settings` sur le
paquet livré reste à faire dans le lot de remise ultérieur décrit plus haut.
