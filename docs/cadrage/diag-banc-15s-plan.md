Avancée : plan du lot préparé avant instrumentation et campagnes ; worktree dédié créé.
Blocage : reprise des campagnes soumise à arbitrage après les deux échecs consignés ci-dessous.
Décision : corriger et relire le candidat sans relancer de campagne avant cet accord.

# DIAG-BANC-15S — plan du 4 novembre 2026

## Mandat et état initial

Objectif : localiser dans le corps Rust du banc Auto les coûts fixes et bimodaux,
pour rendre fiables les mesures natives de transcription. La demande utilisateur
de cette session autorise le banc seul, N1, puis six essais N2 Auto au maximum ;
aucune correction de performance, modification produit/garde/machine,
fusion ou publication. Les mesures humaines et exclusions de pistes sont des
entrées utilisateur, pas des validations reproduites par ce lot.

Worktree `C:/wt/diag-banc`, branche `diag-banc-15s`, depuis main `9943493d`
(main a avancé depuis DIAG-AUTO-15S sur `4f32d029`). État initial propre dans
le worktree ; deux documents non suivis de main préservés. Espace initial :
555 914 850 304 octets libres. Budget : 90 minutes de travail actif au total,
six corps N2 maximum, deux cycles de correction d'instrumentation maximum.
L'attente d'un arbitrage utilisateur ne vaut pas poursuite autorisée.

## Périmètre et responsabilités

- Session principale : ce plan versionné, orchestration des commandes sous la
  garde existante, analyse, archivage et bilan demandé explicitement dans
  `docs/architecture/DIAG-BANC-15S-BILAN.md` ; commits locaux seulement.
- `rust_implementer`, mode IMPLEMENT :
  `crates/whisper-adapters/tests/parakeet_unified_native.rs` et nouveau support
  `crates/whisper-adapters/tests/support/native_import_timing.rs`, et unique
  descripteur harnessTrace du profil PersonalAutoFallback dans
  `ci/native-profiles.json` (accord précis utilisateur « oui je t'autorise »).
  Aucun changement des assertions, noms/nombres de tests, dépendances ou produit.
- Candidat concret : revue par `rust_review_coordinator`, reviewer indépendant,
  corrections par l'implémenteur ; revue indépendante des conclusions avant bilan.
- Preuves et temporaires sous `target/diag-banc-15s/` du worktree ; cache N1
  persistant `target/cargo-home-shared/`, cible Cargo partagée existante
  `C:/dev/whisper/target/parakeet-tests/cargo-shared/`. Racine N2 sous le lot si
  le profil existant l'accepte, sinon emplacement imposé par la garde à constater.
  Aucun accès en écriture aux archives `target/poll-preserved/` ou à
  `target/parakeet-feasibility/`. Aucune suppression automatique des target du lot.

## Huit tâches séquentielles et critères

1. **Préflight N0 et activation.** Lire les consignes/carte, vérifier Git,
   chemins, espace et transmission de l'activation. Ne pas contourner un
   filtrage d'environnement. Si incompatible, arrêter les tâches dépendantes
   et obtenir uniquement l'arbitrage nécessaire.
2. **Instrumentation du banc.** Horloge monotone unique par test, identifiant
   d'essai/PID, enregistrements JSONL début/fin/durée en ms. Tracer l'entrée et
   la sortie du test et de ses helpers, validation des chemins/empreinte source,
   préparation des dossiers du banc, personal_paths (incluant validations CPU/GPU),
   construction du service/application/requête, StartImport, chaque Refresh et
   chaque changement visible d'événement/vue, sortie de boucle, vérifications
   archive/exports/WAV/source/index, destruction des objets et fin du test.
   Les événements internes non exposés par ImportApplication ne sont pas inventés.
   Activée explicitement, inactive par défaut ; aucun état produit précréé.
   Le staging de la garde est antérieur au corps et reste mesuré séparément.
3. **Revue du candidat.** Vérifier scope, activation, absence de changement
   d'assertion, exhaustivité et coût des traces. Les appels produit opaques
   restent un intervalle unique. Deux cycles de correction maximum.
4. **Vérifications N1.** Garde N1 et auto-tests existants, inventaire de
   référence sur la base si nécessaire, format, cargo test standard sans
   activation, comparaison des identités/nombres/résultats ; build et Clippy
   utiles du banc. Compilation native-tests seulement par une entrée de la
   garde existante qui l'autorise sans exécution native ; sinon arbitrage.
   Aucun contrôle PE requis sans worker/manifeste/lock/script de build touché.
   Tout échec arrête la campagne selon AGENTS.md. Aucun téléchargement réseau.
5. **Préflight N2 puis essais.** SelfTest/Preflight existants, profil exact
   PersonalAutoFallback, ressources épinglées identiques, en-short.mp3 de
   24 237 octets et Auto sans candidat. Jusqu'à six Run ; arrêt dès un rapide
   (<30 s) et un lent (>=30 s). Un intermédiaire est relevé s'il survient,
   sans prolonger la campagne après obtention de la paire. Compter chaque
   corps engagé dans le plafond, conserver les erreurs et traces partielles.
6. **Analyse par mode.** Tableau rapide/lent avec identifiants, phases et
   écarts, sans moyenne ni médiane globale. Distinguer corps Rust, Cargo,
   staging et post-traitement de la garde. Ne pas transformer l'écart avec
   les mesures humaines en durée directement mesurée d'une phase commune.
   Si un intervalle produit reste opaque : le dire et proposer seulement le
   marqueur minimal du lot suivant. Si environnement suspect : une commande
   utilisateur hors garde est proposée, jamais exécutée par l'agent.
7. **Revue indépendante des conclusions.** Vérifier les traces et la portée
   du verdict BANC/GARDE/PRODUIT (installation neuve)/NON DÉTERMINÉ. Évaluer
   au plus deux corrections possibles avec coût, risque, effet sur la mesure ;
   aucune correction appliquée. Choix garder/jeter l'instrumentation à l'utilisateur.
8. **Clôture.** Archiver les petites traces, reçus et paramètres utiles sous
   la racine du lot ; vérifier nombre et empreintes de l'archive conformément
   à la rétention demandée. Proposer les commandes de suppression des seuls
   intermédiaires du lot après contrôle de chemins, liens et usages ; l'utilisateur
   les exécute. Préserver caches partagés/persistants et sources. Bilan figé
   d'une page au chemin demandé, versionné localement ; aucun push ni fusion.

## Incompatibilité établie à la tâche 1

`ci/guard-native.ps1:2002-2035` construit ProcessStartInfo en effaçant
EnvironmentVariables, puis ne réinjecte que ses listes explicites.
`Get-ProfileEnvironment`, lignes 1044-1087, construit la table exclusivement
à partir du profil et de ses valeurs/entrées/sorties, puis ajoute les racines.
`ci/native-profiles.json:733-755` : PersonalAutoFallback n'a que CUDA_PATH comme
valeur, aucune variable d'activation des traces. Les clés de ValuesJson doivent
correspondre exactement au profil (`Get-ValuesDocument`, lignes 970-980).
Exporter WHISPER_DIAG_BANC_TRACE dans le terminal ne la transmettrait pas.
Réutiliser CUDA_PATH, détourner une racine ou une variable système, activer
implicitement via native-tests ou lancer hors garde violerait le mandat.

### Arbitrage accepté dans cette session avant implémentation

Option recommandée : exception limitée à **un descripteur littéral** ajouté
uniquement aux values de PersonalAutoFallback dans ci/native-profiles.json :

```json
{"key":"harnessTrace","environmentVariable":"WHISPER_DIAG_BANC_TRACE","required":true,"kind":"literal"}
```

Le manifeste d'exécution propre au lot fournit `harnessTrace: "1"` ; le support
du banc n'active les traces que pour la valeur exacte `1`, sinon aucune trace.
Le chemin JSONL est dérivé de WHISPER_L04_CAMPAIGN_RUN, jamais d'un chemin
ambiant libre. La garde et les autres profils resteraient inchangés. Cela
élargit explicitement le périmètre à ce descripteur ; l'utilisateur l'a autorisé
après présentation du blocage. Coût attendu faible ; les anciens
ValuesJson de ce profil devraient fournir la nouvelle clé (0 ou 1).

Alternative : maintenir strictement le périmètre et arrêter avant
instrumentation/campagnes. Aucun mécanisme conforme de transmission de la
variable dédiée n'a été trouvé. Un fichier d'activation ne constitue pas une
solution établie : le staging épingle exactement 21 fichiers runtime et un MP3 ;
sans ajout au manifeste il ne serait pas copié, avec ajout il ferait échouer
le nombre de fichiers requis. Ne pas utiliser un marqueur implicite.

Les deux observations signalées par l'utilisateur (provenance des DLL sous
spike02 et messages ignore décalés) seront rapportées comme telles, sans
correction ni prétention de vérification de provenance.

## Arrêt de campagne avant tout corps natif

Le 4 novembre 2026, N1 SelfTest 35/79 et fmt --check ont réussi. La première
commande standard cargo test, sous garde et sans activation, a échoué à la
compilation (101, E0061) : appel existant à huit arguments dans
tests/parakeet_unified_native/voxpopuli.rs:262, helper passé à neuf arguments.
Aucun résultat de test exécuté. Le reviewer a demandé R1 (compatibilité)
et R2 (découpage plus fin des phases) ; correction cycle 1 par l'implémenteur
dans les trois fichiers autorisés, sans Cargo, puis relecture indépendante.

Erreur d'orchestration distincte : la session principale a lancé Native
SelfTest pendant N1. Le watcher natif a vu une écriture VSApplicationInsights
dans target/diag-banc-15s/standard/scratch/.../localappdata/, hors de sa racine
N2, et a arrêté SelfTest. Ce n'est pas une preuve de fuite hors dépôt ni une
écriture attribuée au produit. Les logs sont conservés dans
target/diag-banc-15s/native-selftest.log et
target/n2-native/logs/selftest-nvidia-diagnostic/success/guard-events.jsonl.
Le résultat N1 (E0061) est indépendant de cet arrêt ; aucune nouvelle campagne
n'est lancée. Zéro corps N2 sur les six autorisés. Reprise proposée : commandes
strictement séquentielles, uniquement après accord utilisateur post-échec
conformément à AGENTS.md. Pas de restauration ni suppression des preuves.

### Reprise autorisée et nouvel arrêt

Après l'accord utilisateur « oui je t'autorise », les commandes séquentielles
sur d6482e95 ont réussi : N1 standard 705 PASS / 128 ignorés / 0 échec,
fmt --check, build adapters, Clippy adapters/all-targets (avertissements
conservés), puis Native SelfTest. R1/R2 sont fermés statiquement ; la preuve
de compilation native reste requise par le reviewer.

PreparationNoRun a ensuite été arrêté par la garde à 17:21:06 locale sur
une écriture VSApplicationInsights dans l'ancien LOCALAPPDATA isolé N1
standard-reprise, bien que N1 ait fini à 17:16:39. Les commandes Cargo n'étaient
pas concurrentes. Aucun diagnostic compilateur avant cet arrêt, aucun corps
Auto engagé. Logs conservés sous target/n2-native/logs/
PreparationNoRun-ba0dcfdaccbd4cba910bf68383adca89/ ; V2 reste non démontrée.

Lecture des processus : seul VCTIP.EXE PID 82512, créé à 17:16:04.504851 locale,
parent 167388 déjà disparu. Son chemin est
C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.44.35207/bin/HostX64/x64/VCTIP.EXE.
Cette concordance n'attribue pas l'écriture à ce PID. Proposition d'arbitrage :
arrêter uniquement cette incarnation de VCTIP, après revérification du PID,
nom, chemin et date de création, puis reprendre la compilation et les essais
selon les mêmes gardes. Aucun arrêt de processus, modification de garde ou
configuration machine effectué pour cette proposition. Quota Auto : 0/6.
