Avancée : plan du lot préparé avant instrumentation et campagnes ; worktree dédié créé.
Blocage : cause interne non déterminée ; aucun mode lent parmi les six essais autorisés.
Décision : lot clos au quota ; aucun essai supplémentaire ni fusion sans nouvelle décision utilisateur.

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

## Reprise finale et clôture du 4 novembre 2026

L'utilisateur a autorisé explicitement l'arrêt de cette seule instance et la
reprise. À 17:24:51 locale, nom, chemin et date de création de VCTIP 82512 ont
été revérifiés identiques, puis cette instance seule a été arrêtée. L'origine
PID des anciennes écritures n'est pas rétroactivement démontrée.
PreparationNoRun-3f8b5076ec184be8b19c60af5fdc3242 a ensuite terminé
NO_RUN_PASS (aucun corps natif), puis le préflight PersonalAutoFallback a réussi.

Six essais Auto séquentiels, tous PASS et R :
fab3b20dd9174888b02d021ea3f0b29d 25,26 s ;
07b8d73430c44df5827bd3e4c226f887 25,10 s ;
0b1e1300008e47378d4f3c2f62cab2ed 25,16 s ;
a55b7d0cf94949b7b2d20ff775db4922 25,23 s ;
cfbb41330c0a4080949c6fe46dafae11 25,16 s ;
7eae9edfcfbf471ab1827d85b33613b0 25,12 s.
Arrêt au quota 6/6, aucun essai lent ni intermédiaire obtenu, aucune correction
de performance. Les traces monotones sont sous chaque runs/<id>/writes/l04/ ;
un Refresh est enregistré à chaque interrogation, les changements de vue
seulement lorsqu'ils sont observés. Pas d'observation des événements internes.

Instrumentation source d6482e95 : revue indépendante finale CLEAN, R1/R2 fermés,
V1/V2/V3 validés ; deux revues et un cycle de correction. Conclusions relues
indépendamment : préciser observation de Prepared, périmètres non identiques à
l'application normale et absence de preuve A/B du coût des traces. Aucun gain
de performance ni disparition de bimodalité établi. Bilan au chemin demandé
docs/architecture/DIAG-BANC-15S-BILAN.md, figé à son commit de clôture.

### Archive vérifiée

`C:/wt/diag-banc/target/diag-banc-15s/archive/DIAG-BANC-15S-20261104.zip` :
473 entrées, 2 870 690 octets source, 1 147 752 octets archivés.
Nombre, taille et SHA-256 de chaque entrée comparés aux sources : PASS.
SHA-256 ZIP : `47F8BB5FB6842D1E28985F56A04A5C22390894F7C4A7CB9587E8A4794A9A33F3`.
Contenu : reçus et logs N1/N2, paramètres d'exécution, traces et sorties utiles
des six Auto. Les caches, copies de runtime de 3,2 Go, modèles et temporaires
de compilation ne sont pas inclus. Les documents et sources restent dans Git.

### Nettoyage proposé à l'utilisateur, non exécuté

À 17:39 locale : target/n2-native = 23 026 540 358 octets apparents ;
target/diag-banc-15s = 4 113 655 octets avant création de l'archive. Deux jonctions
dans chacune de ces racines, toutes vers leurs propres fixtures/cache internes.
Ne pas supprimer les parents target/, le cache persistant target/cargo-home-shared,
le target Cargo partagé, l'archive, ni les ressources protégées du dépôt principal.

**Blocage actuel du nettoyage :** VCTIP 48408, créé à 17:25:05.739934 locale
pendant la compilation native, subsiste. Son usage des racines du lot n'est pas
exclu. Cette nouvelle instance n'a pas été arrêtée : l'accord précédent portait
sur 82512 seulement. Conserver les intermédiaires tant que cette instance et
toute campagne utilisant ces chemins ne sont pas terminées. Les commandes
ci-dessous refusent toute suppression si 48408 existe encore ; elles ne le tuent pas.

Après fermeture des processus concernés et vérification de l'absence de lot actif,
commande PowerShell utilisateur (la session principale ne l'exécute pas) :

```powershell
if (Get-Process -Id 48408 -ErrorAction SilentlyContinue) {
    throw 'Nettoyage suspendu : processus 48408 encore présent ; revérifier son identité et son usage.'
}
$diagCleanupPaths = @(
    'C:\wt\diag-banc\target\n2-native',
    'C:\wt\diag-banc\target\diag-banc-15s\build',
    'C:\wt\diag-banc\target\diag-banc-15s\clippy',
    'C:\wt\diag-banc\target\diag-banc-15s\fmt',
    'C:\wt\diag-banc\target\diag-banc-15s\fmt-reprise',
    'C:\wt\diag-banc\target\diag-banc-15s\incidents',
    'C:\wt\diag-banc\target\diag-banc-15s\n1-selftest',
    'C:\wt\diag-banc\target\diag-banc-15s\standard',
    'C:\wt\diag-banc\target\diag-banc-15s\standard-reprise'
)
foreach ($diagPath in $diagCleanupPaths) {
    if (-not (Test-Path -LiteralPath $diagPath)) { continue }
    $diagResolved = (Resolve-Path -LiteralPath $diagPath).Path.TrimEnd('\')
    if ($diagResolved -ine $diagPath -or
        -not $diagResolved.StartsWith('C:\wt\diag-banc\target\', [StringComparison]::OrdinalIgnoreCase)) {
        throw "Chemin de nettoyage invalide : $diagPath"
    }
    $diagAncestor = $diagResolved
    while ($diagAncestor) {
        if ((Get-Item -LiteralPath $diagAncestor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Parent ou racine redirigé : $diagAncestor"
        }
        $diagParent = Split-Path -Parent $diagAncestor
        if (-not $diagParent -or $diagParent -eq $diagAncestor) { break }
        $diagAncestor = $diagParent
    }
    foreach ($diagLink in Get-ChildItem -LiteralPath $diagResolved -Recurse -Force -Attributes ReparsePoint) {
        foreach ($diagTarget in @($diagLink.Target)) {
            if (-not [IO.Path]::IsPathRooted($diagTarget) -or
                -not ([IO.Path]::GetFullPath($diagTarget)).StartsWith($diagResolved + '\', [StringComparison]::OrdinalIgnoreCase)) {
                throw "Jonction hors racine du lot : $($diagLink.FullName)"
            }
        }
    }
}
$diagArchive = 'C:\wt\diag-banc\target\diag-banc-15s\archive\DIAG-BANC-15S-20261104.zip'
if ((Get-FileHash -LiteralPath $diagArchive -Algorithm SHA256).Hash -cne
    '47F8BB5FB6842D1E28985F56A04A5C22390894F7C4A7CB9587E8A4794A9A33F3') {
    throw 'Archive absente ou différente : aucune suppression.'
}
foreach ($diagPath in $diagCleanupPaths) {
    if (Test-Path -LiteralPath $diagPath) { Remove-Item -LiteralPath $diagPath -Recurse -Force }
}
Get-PSDrive C | Select-Object Free
```

Les petits logs de premier niveau et l'archive restent conservés. Après exécution
utilisateur, la vérification d'absence et l'espace libre seront un fait nouveau,
à consigner dans un nouveau document ; ne pas réécrire le bilan figé.

Clôture : revue indépendante des conclusions corrigées CLEAN, aucun constat
ouvert dans sa portée ; archivage/nettoyage hors de cette revue. Vérification
statique de syntaxe de la commande de nettoyage PASS, commande non exécutée.
Aucune suppression ; espace libre relevé avant clôture : 528 050 298 880 octets.
