Avancée : inventaire et commandes humaines préparés le 6 novembre 2026.
Blocage : déplacements P5 et suppressions éventuelles réservés à l'humain ; aucun n'est exécuté par l'agent.
Décision : conserver les preuves ; vérifier les contrôles ci-dessous avant activation des nouvelles racines.

# Annexe PRESERVATION — INSPECT-CIBLEE

## P5 — migration des archives

Inventaire initial apparent (liens physiques éventuels comptés par chemin) :

| Source | Fichiers | Octets | Destination |
|---|---:|---:|---|
| `C:/dev/whisper/target/poll-preserved` | 117586 | 89160572401 | `C:/wb/preserved/poll-preserved` |
| `C:/dev/whisper/target/pile-preserved` | 8485 | 11644240959 | `C:/wb/preserved/pile-preserved` |

Les deux destinations sont absentes au relevé initial. Les commandes suivantes sont proposées à l'humain pour **PowerShell 7** ; elles ne sont pas exécutées par l'agent. Ne les lancer qu'en l'absence de lot/processus utilisant ces archives. Elles contrôlent les racines exactes, refusent une racine traversant un lien/jonction, un inventaire modifié et une destination préexistante, puis vérifient les comptes/octets après chaque `/MOVE`. Aucun parent `target/` n'est déplacé. Un code robocopy >=8 ou un écart impose l'arrêt et la conservation de l'état, sans nettoyage automatique.

Contrôle complémentaire : **181 liens/jonctions dans poll et 97 dans pile**. Il s'agit notamment d'anciens profils INetCache et de fixtures de garde. La première proposition les refusait sans déplacement ; la version ci-dessous utilise `/SJ /SL` (options confirmées par l'aide Robocopy locale) pour copier les liens eux-mêmes, jamais leurs cibles. Elle conserve les cibles textuelles historiques, dont certaines peuvent déjà être absentes, et ne les répare pas. L'inventaire PowerShell 7 n'utilise pas `-FollowSymlink`. Tout autre type de reparse point est refusé. Si les droits existants ne permettent pas la copie d'un lien, arrêter sans élévation automatique.

```powershell
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7) { throw 'Ouvrir PowerShell 7 pour cette recette' }
$migrations = @(
    @{ Source='C:\dev\whisper\target\poll-preserved'; Destination='C:\wb\preserved\poll-preserved'; Files=117586L; Bytes=89160572401L; Links=181 },
    @{ Source='C:\dev\whisper\target\pile-preserved'; Destination='C:\wb\preserved\pile-preserved'; Files=8485L; Bytes=11644240959L; Links=97 }
)
foreach ($migration in $migrations) {
    $source = [IO.Path]::GetFullPath($migration.Source)
    $destination = [IO.Path]::GetFullPath($migration.Destination)
    if ($source -notin @('C:\dev\whisper\target\poll-preserved','C:\dev\whisper\target\pile-preserved')) { throw 'Source inattendue' }
    if ($destination -notin @('C:\wb\preserved\poll-preserved','C:\wb\preserved\pile-preserved')) { throw 'Destination inattendue' }
    foreach ($path in @($source,$destination)) {
        $cursor = $path
        while ($cursor) {
            if ((Test-Path -LiteralPath $cursor) -and ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Lien/jonction : $cursor" }
            $cursor = Split-Path -Parent $cursor
        }
    }
    if (-not (Test-Path -LiteralPath $source -PathType Container)) { throw "Source absente : $source" }
    if (Test-Path -LiteralPath $destination) { throw "Destination déjà présente : $destination" }
    $entries = @(Get-ChildItem -LiteralPath $source -Recurse -Force)
    $links = @($entries | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint })
    if ($links.Count -ne $migration.Links) { throw 'Inventaire des liens modifié' }
    if (@($links | Where-Object { $_.LinkType -notin @('Junction','SymbolicLink') }).Count) { throw 'Type de reparse point non pris en charge' }
    $beforeLinks = @($links | ForEach-Object { [pscustomobject]@{Relative=$_.FullName.Substring($source.Length);Type=$_.LinkType;Target=$_.LinkTarget} } | Sort-Object Relative) | ConvertTo-Json -Depth 5 -Compress
    $before = $entries | Where-Object { -not $_.PSIsContainer } | Measure-Object Length -Sum
    if ($before.Count -ne $migration.Files -or $before.Sum -ne $migration.Bytes) { throw 'Inventaire initial modifié : relever à nouveau avant déplacement' }
    robocopy $source $destination /E /MOVE /COPY:DAT /DCOPY:DAT /SJ /SL /R:1 /W:1 /NP
    $copyExit = $LASTEXITCODE
    if ($copyExit -ge 8) { throw "Robocopy a échoué : $copyExit" }
    $afterEntries = @(Get-ChildItem -LiteralPath $destination -Recurse -Force)
    $after = $afterEntries | Where-Object { -not $_.PSIsContainer } | Measure-Object Length -Sum
    $afterLinkEntries = @($afterEntries | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint })
    $afterLinks = @($afterLinkEntries | ForEach-Object { [pscustomobject]@{Relative=$_.FullName.Substring($destination.Length);Type=$_.LinkType;Target=$_.LinkTarget} } | Sort-Object Relative) | ConvertTo-Json -Depth 5 -Compress
    $sourceEntries = if (Test-Path -LiteralPath $source) { @(Get-ChildItem -LiteralPath $source -Recurse -Force) } else { @() }
    $remaining = @($sourceEntries | Where-Object { -not $_.PSIsContainer }).Count
    $remainingLinks = @($sourceEntries | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count
    [pscustomobject]@{Source=$source;Destination=$destination;BeforeFiles=$before.Count;AfterFiles=$after.Count;BeforeBytes=$before.Sum;AfterBytes=$after.Sum;BeforeLinks=$links.Count;AfterLinks=$afterLinkEntries.Count;LinksIdentical=($beforeLinks -ceq $afterLinks);SourceFiles=$remaining;SourceLinks=$remainingLinks;RobocopyExit=$copyExit}
    if ($after.Count -ne $before.Count -or $after.Sum -ne $before.Sum -or $remaining -ne 0 -or $remainingLinks -ne 0 -or $beforeLinks -cne $afterLinks) { throw 'Migration incomplète : conserver les deux emplacements et signaler' }
}
```

Ces contrôles de taille/nombre ne constituent pas une nouvelle qualification SHA. Ils ne changent pas les archives internes. Après retour humain, la session vérifiera les deux destinations et l'absence de fichiers aux anciennes racines avant de déclarer P5 accompli.

## P7 — worktrees

Inventaire Git comparé à `main=34975d43` ; aucune modification suivie ni non suivie non ignorée sur les cinq worktrees. Les tailles sont apparentes, pas l'espace libérable (liens physiques non dédupliqués). Les fichiers ignorés sont aussi des fichiers non versionnés : ils sont inclus ci-dessous.

| Worktree | Branche / HEAD | Ancêtre de main | Commits hors main | Fichiers / octets apparents | Non versionnés observés | Proposition |
|---|---|---|---:|---:|---:|---|
| `target/agent-worktrees/agents-tests-guidelines` | `codex/agents-tests-guidelines` / `bd72ebeb` | non | 1 | 6711 / 202060320 | 0 | suppression du worktree possible après examen du commit ; garder la branche |
| `target/agent-worktrees/diag-r8-cuda-cpu` | `diag/r8-cuda-cpu` / `982592ba` | non | 3 | 6728 / 205249293 | 0 | suppression possible après archivage/examen du diagnostic ; garder la branche |
| `target/agent-worktrees/tests-autonomes-r8-spike` | `refacto/tests-autonomes-r8-spike` / `db93eca1` | non | 14 | 37517 / 1002593290 | 30785, ignorés | conserver tant que le contenu ignoré n'a pas été trié |
| `C:/wt/garde-deps` | `codex/garde-dependances` / `9cd15ca6` | oui | 0 | 29737 / 868670214 | 22959, ignorés | suppression possible après examen des preuves/caches ignorés |
| `C:/wt/build-natif` | `build-natif` / `1089d8dd` | non | 52 | 367867 / 36810701742 | au moins 218318, ignorés | **CONSERVER** |

Les trois chemins abrégés du tableau sont sous `C:/dev/whisper/`. Liste observée des fichiers non versionnés et des commits hors main : `C:/wb/inspect-ciblee/worktrees-inventory.txt` (28094853 octets). Limite de lecture : Git a signalé des chemins trop longs dans le spike et des refus d'accès/jonctions INetCache dans build-natif ; aucun droit ni lien n'a été modifié pour les traverser. Les nombres de ces deux worktrees ne certifient donc pas un inventaire exhaustif. Cette limite justifie leur conservation.

Commits uniques : guidelines `bd72ebeb` ; diagnostic `6c3954d4`, `0e753893`, `982592ba` ; spike `8ea99ffd`, `61b0a21d`, `43c62542`, `42f9afe6`, `a7344a9e`, `1eec4e2d`, `c749e8fa`, `206b1d39`, `9df93659`, `1601cb79`, `f2186cb7`, `53b5f608`, `8b1f58db`, `db93eca1`. Pour build-natif, le commit de tête `1089d8dd` et 51 autres sont absents de main ; `git log main..build-natif` fournit la liste, et `git cherry main build-natif` distingue 51 patches nouveaux et un patch équivalent déjà présent. Une absence d'ascendance n'établit pas que tous les changements sont inédits.

**C1001 : ne pas fusionner le bilan seul en l'état.** `1089d8dd` contient le bilan, `NATIVE-BUILD.md`, le plan C1001 et `raw/build-natif-c1001/evidence-20261105T194944Z.zip` (23939167 octets). Le bilan pointe aussi vers `docs/cadrage/build-natif-protection-proposition.md`. Proposition : reprise documentaire ciblée de la clôture et de ses références/preuves après décision humaine, sans intégrer les changements de code/garde natifs de la branche. Cette reprise n'est pas exécutée dans INSPECT-CIBLEE.

Commandes proposées après examen, à exécuter **par l'humain**, depuis le dépôt principal ; sans `--force`, sans suppression de branche. Les trois worktrees non fusionnés restent récupérables par leurs branches si leur worktree propre est retiré. Pour les worktrees contenant des ignorés, ne pas forcer le refus Git ; conserver et préparer un périmètre séparé après tri.

```powershell
git -C C:\dev\whisper worktree remove C:\dev\whisper\target\agent-worktrees\agents-tests-guidelines
git -C C:\dev\whisper worktree remove C:\dev\whisper\target\agent-worktrees\diag-r8-cuda-cpu
# Seulement après examen de ses 22959 fichiers ignorés :
git -C C:\dev\whisper worktree remove C:\wt\garde-deps
```

## Cartographie initiale des inspections et SHA de modèles

Sources recherchées dans toutes les crates : `inspect_requested_model`, `LocalModelCatalog`, `.inspect(`, `.snapshot(`, `cached_model_sha256`, `cached_artifact_sha256`, `verify_sha256`, `sha256_file`, `Sha256`. Les appels en modules de tests/integration/cfg(test) sont séparés de la production ; les SHA d'audio, exports, binaires et DLL ne sont pas comptés comme inspections de modèles. Le croisement des primitives de hash et des appelants est le contrôle de complétude ; les lecteurs natifs internes aux bibliothèques ne sont pas ré-audités dans ce lot.

| Appelant production (lignes de base indicatives) | Déclencheur | Modèles inspectés sur 34975d43 | Coût |
|---|---|---|---|
| desktop `root.rs:61`, `compose` | démarrage, thread catalogue dédié | tous les modèles du registre | somme des lectures SHA froides ; publication UI différée |
| adapters `backends/mod.rs:126`, `bind_import` | admission import | ciblé par `inspect_requested_model` | déjà corrigé d7e6445a ; 25→8,6 s historique import debug, pas mesure de ce lot |
| adapters `runtime/queue.rs:200`, `process_queued` | traitement de l'élément FIFO | tous via `snapshot()` | lecture étrangère évitable, coût dépendant du cache |
| adapters `runtime/recovery.rs:19`, `resume_interrupted` | reprise interrompue | tous via `snapshot()` | même dette |
| adapters `backends/mod.rs:229`, `bind_live` | départ micro Turbo depuis `runtime/live.rs:365` | tous | hors correction ; lecture froide potentielle |
| adapters `backends/mod.rs:239`, `bind_native_unified_live` | admission live unifiée depuis `runtime/live.rs:1184` | tous | hors correction |
| adapters `backends/mod.rs:273`, `bind_parakeet_live` | admission live Parakeet depuis `runtime/live.rs:1120` | tous, puis bind_import ciblé | hors correction |
| adapters `backends/mod.rs:337`, `is_parakeet_live_request` | choix du routage live | tous | hors correction |
| adapters `archive/import.rs:325`, `prepare_inner_impl` | préparation/revalidation archive | uniquement `model_path` de la requête | SHA partagé/caché, maintien de l'intégrité |
| adapters `parakeet_import.rs:114/156/190/214` | vérification qualification, clés CPU/GPU/personnelle | uniquement `self.model` | cache partagé ; clés réutilisées au démarrage/admission |
| workers Turbo CPU `session/mod.rs:95`, `session/import.rs:46/257` ; GPU `session/mod.rs:148/216`, `session/import.rs:45/335`, `gpu_commands.rs:37` | initialisation/admission et revalidation import/live/commandes GPU | modèle de la commande via worker-common `integrity::verify_sha256` | lecture intégrale indépendante du cache parent ; non modifiée |
| worker Parakeet `startup.rs:47` | démarrage du worker | modèle fourni via `integrity::sha256_file` | lecture intégrale indépendante du cache parent ; non modifiée |

Primitives parentes : `inspect` énumère MODELS → `inspect_entry` → `inspect_file` → `cached_model_sha256` → `cached_file_sha256` → `ModelFileHashCache::sha256`. `inspect_requested_model` utilise les mêmes inspection et hash avec une seule entrée. Les SHA de DLL/worker dans `parakeet_import` empruntent `cached_artifact_sha256_with_read` et le même mutex ; ils sont pertinents pour le diagnostic de concurrence, sans être des modèles supplémentaires. Dans les workers, `verify_sha256` vérifie aussi les sources audio ; ces appels audio sont exclus du décompte modèles.

## Verrou — diagnostic uniquement

`ARTIFACT_HASH_CACHE` est un `OnceLock<Mutex<ModelFileHashCache>>` global au processus parent. `cached_file_sha256:234–252` prend le verrou, appelle `cache.sha256(path)` puis le relâche. Sous le verrou : ouverture, identité/métadonnées, recherche de cache, lecture complète/SHA si cache froid, réouverture/revalidation et insertion. Un hit garde une section courte d'ouverture/identité ; un miss garde le mutex pendant toute la lecture. Les autres modèles, DLL et binaires utilisant ce cache attendent le même verrou. Le temps n'est pas mesuré ici ; [DIAG-CORR-PREPARE](DIAG-CORR-PREPARE-BILAN.md) relevait 16,4424–16,5009 s pour Turbo ~1,62 Go et 7,1499–7,2500 s pour Parakeet en debug. L'attente du mutex y était ≤0,0029 ms : aucune attente significative du verrou n'y était démontrée. Ces durées historiques ne sont pas des mesures ni des bornes de cette session.

Le catalogue de `root.rs:61` s'exécute sur un thread dédié : son SHA ne bloque pas directement la boucle d'événements UI. En revanche l'UI attend son snapshot/disponibilité, et les admissions qui utilisent le même cache peuvent attendre son SHA. Sans mesure de chemin UI, ne pas assimiler cette attente au gel de la fenêtre. Proposition hors lot : calculer les misses hors mutex puis revérifier/insérer sous verrou, avec coordination par identité pour éviter les doubles lectures et conserver la revalidation des fichiers. Cette modification mérite ses propres tests de concurrence ; elle n'est pas appliquée ici.

## Documents figés et liens historiques

Les bilans `bilan-parakeet-faisabilite-cloture.md`, `bilan-parakeet-fr-en-criteres-revises.md`, `bilan-parakeet-fr-en-go.md`, `bilan-parakeet-limite-decoupe-longue.md`, `bilan-parakeet-limite-vad-revisee.md` conservent leurs références au banc perdu. Ils ne sont pas modifiés. Les documents figés `BILAN-FUSION-PILE.md` et `POLL-INTEGRATION-NETTOYAGE.md` gardent les anciens chemins `target/pile-preserved` et `target/poll-preserved` après P5 ; la correspondance de migration figure dans le tableau P5. Les anciens plans/fiches/recettes citant feasibility restent historiques ; seules les trois références vivantes demandées reçoivent le renvoi à la note de perte.

## Rétention et limites

Le worktree `C:/wt/inspect-ciblee` et les commits de la branche sont conservés. Aucun bilan figé antérieur n'est réécrit. Inventaire de clôture du 6 novembre, 09:21 Paris :

| Chemin créé par le lot | Fichiers | Octets | Proposition |
|---|---:|---:|---|
| `C:/wb/inspect-ciblee` | 2755 | 4674216689 | suppression humaine possible après archivage ; cible Cargo et copies de preuves |
| `C:/wt/inspect-ciblee/target/inspect-ciblee` | 436 | 8546423 | suppression humaine après vérification des six jonctions internes |
| `C:/wt/inspect-ciblee/target/native-build/control-test-r1r3` | 1 | 360 | suppression humaine |
| `C:/wt/inspect-ciblee/target/native-build/control-test-r1r3-final` | 1 | 841 | suppression humaine |
| `C:/wt/inspect-ciblee/target/native-build/control-test-r1r3-pass1` | 33 | 164932994 | suppression humaine ; journaux synthétiques régénérables |
| `C:/wt/inspect-ciblee/target/native-build/control-test-r1r3-retry` | 1 | 620 | suppression humaine |
| `C:/wt/inspect-ciblee/target/native-build/control-test-r1r3-retry2` | 1 | 843 | suppression humaine |
| `C:/wt/inspect-ciblee/target/native-build/control-tests-f410db65f7164646847f0b61461f1064` | 34 | 270316206 | suppression humaine ; journaux synthétiques régénérables |
| `C:/wt/inspect-ciblee/target/native-build/protected-fixtures-final` | 1 | 820 | suppression humaine |
| `C:/wt/inspect-ciblee/target/native-build/protected-fixtures-final2` | 1 | 822 | suppression humaine |
| `C:/wt/inspect-ciblee/target/cargo-home-shared` | 22417 | 651825324 | **conserver**, cache persistant déclaré ; copie locale sans réseau |

Les six jonctions de la campagne pointent toutes vers leur propre `scratch/<run>/junction-target`, sous le même lot : trois `junction-fixture` et trois `fresh-fixture/targets` dans les runs `20261106T080659452-9513ee9840154498be4d02a8eacd48d4`, `20261106T080709317-5aff08fef59b4dcc8f1236fc6d946a5c`, `20261106T083043112-7a2965bdaffc48ae8153eb7338f82f85`. Aucune autre racine du tableau ne contient de reparse point au relevé. Aucun cargo/rustc/robocopy observé à la clôture. Les tailles sont apparentes, pas une promesse d'espace libéré.

Les archives `C:/wb/preserved`, les autres worktrees, modèles, audios et l'installation personnelle sont exclus du nettoyage proposé. L'humain exécute les commandes suivantes **après fermeture du lot**, sans processus utilisant ces chemins. Le script compare les inventaires avant toute suppression et refuse les jonctions externes. Il retire les seules jonctions internes comme liens, sans récursion, avant leurs racines. En cas de refus d'un outil, arrêter ; aucun autre mécanisme n'est proposé pour le contourner. Aucun de ces retraits n'est exécuté par l'agent.

```powershell
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7) { throw 'Utiliser PowerShell 7' }
$archive = 'C:\wt\inspect-ciblee\docs\architecture\raw\INSPECT-CIBLEE-20261106T092341.zip'
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne 'A86957C3921A8F760A236BEA81CBB75818119DF6412D17340800D05302E0F2D0') { throw 'Archive absente ou modifiée' }
$cleanup = @(
    @{Path='C:\wb\inspect-ciblee';Files=2755;Bytes=4674216689L},
    @{Path='C:\wt\inspect-ciblee\target\inspect-ciblee';Files=436;Bytes=8546423L},
    @{Path='C:\wt\inspect-ciblee\target\native-build\control-test-r1r3';Files=1;Bytes=360L},
    @{Path='C:\wt\inspect-ciblee\target\native-build\control-test-r1r3-final';Files=1;Bytes=841L},
    @{Path='C:\wt\inspect-ciblee\target\native-build\control-test-r1r3-pass1';Files=33;Bytes=164932994L},
    @{Path='C:\wt\inspect-ciblee\target\native-build\control-test-r1r3-retry';Files=1;Bytes=620L},
    @{Path='C:\wt\inspect-ciblee\target\native-build\control-test-r1r3-retry2';Files=1;Bytes=843L},
    @{Path='C:\wt\inspect-ciblee\target\native-build\control-tests-f410db65f7164646847f0b61461f1064';Files=34;Bytes=270316206L},
    @{Path='C:\wt\inspect-ciblee\target\native-build\protected-fixtures-final';Files=1;Bytes=820L},
    @{Path='C:\wt\inspect-ciblee\target\native-build\protected-fixtures-final2';Files=1;Bytes=822L}
)
foreach ($item in $cleanup) {
    $root = [IO.Path]::GetFullPath($item.Path).TrimEnd('\')
    if ($root -ine $item.Path) { throw 'Chemin de nettoyage non canonique' }
    if (-not (Test-Path -LiteralPath $root)) { continue }
    $cursor = $root
    while ($cursor) {
        if ((Test-Path -LiteralPath $cursor) -and ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Ancêtre lié : $cursor" }
        $cursor = Split-Path -Parent $cursor
    }
    $entries = @(Get-ChildItem -LiteralPath $root -Recurse -Force)
    $stats = $entries | Where-Object { -not $_.PSIsContainer } | Measure-Object Length -Sum
    if ($stats.Count -ne $item.Files -or $stats.Sum -ne $item.Bytes) { throw "Inventaire changé : $root" }
    $links = @($entries | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint })
    foreach ($link in $links) {
        if ($link.LinkType -notin @('Junction','SymbolicLink')) { throw 'Type de lien inconnu' }
        foreach ($target in $link.LinkTarget) {
            $fullTarget = if ([IO.Path]::IsPathRooted($target)) { [IO.Path]::GetFullPath($target) } else { [IO.Path]::GetFullPath((Join-Path $link.DirectoryName $target)) }
            if (-not $fullTarget.StartsWith($root+'\',[StringComparison]::OrdinalIgnoreCase)) { throw "Lien externe : $($link.FullName)" }
        }
    }
    $item.Links = $links
}
foreach ($item in $cleanup) {
    if (-not (Test-Path -LiteralPath $item.Path)) { continue }
    foreach ($link in $item.Links) { Remove-Item -LiteralPath $link.FullName -Force }
    if (@(Get-ChildItem -LiteralPath $item.Path -Recurse -Force -Attributes ReparsePoint).Count) { throw 'Lien restant : arrêter' }
    Remove-Item -LiteralPath $item.Path -Recurse -Force
}
Get-PSDrive C | Select-Object Free
```

Après exécution humaine, vérifier l'absence des dix chemins et consigner le résultat dans un nouveau complément, sans réécrire le bilan figé. Les références durables sont le Git du lot et l'archive ci-dessous ; les journaux synthétiques volumineux non archivés sont régénérables par le script versionné.

## Vérifications PRESERVATION avant clôture

- P1 : note figée de 13 lignes ; mentions Git des 21/10 et 04/11 vérifiées, DLL installées 5 CPU + 9 GPU et les deux fichiers de référence présents. Enquête disque/Corbeille/VSS attribuée au constat utilisateur.
- P2 : renvoi ajouté aux trois documents vivants demandés ; liste des références historiques ci-dessus. Aucun bilan figé antérieur modifié.
- P3 : deux anciennes protections feasibility retirées, règle de présence et conservation hors target inscrite dans AGENTS ; suspension native maintenue.
- P4 : contrôle `test-native-build-control.ps1` rapporté exit 0, racine présente acceptée et racine absente refusée sans création ; `nativeBuildStarted=false`, `watcherStarted=false`. Validation des racines réelles nouvelles en attente de P5.
- P5 : aucun déplacement par l'agent ; premier script non lancé selon retour utilisateur, version révisée `/SJ /SL` proposée ; résultat humain attendu.
- P6 : test ciblé runtime personnel vert ; zéro résidu dans TEMP isolé et TEMP utilisateur. Les trois N1 finaux restent à consigner avant de conclure P6.
- P7 : inventaire ci-dessus ; build-natif conservé, reprise du bilan seul déconseillée ; aucun retrait de worktree exécuté.

## État de validation intermédiaire — avant arbitrage

La caractérisation avant correctif exécute deux tests sur le comportement produit de `34975d43` : chacun échoue avec compteur Turbo=1, attendu=0, alors que Parakeet seul est demandé. Les six fichiers de preuve ont été copiés avant correction dans `C:/wb/inspect-ciblee/evidence-before` (9788 octets).

Le premier N1 après correctif, commande `test --no-fail-fast` sans variable CARGO_TARGET_DIR via garde, s'arrête avec exit 101 : **718 réussites / 1 échec / 128 ignorés**, agrégation des 59 lignes `test result`. Les quatre nouveaux tests queue/recovery sont verts. Échec unique du contrôle des sources de tests : le nouveau fichier combine des chemins fictifs en `.exe` et une fixture worker écrite hors constructeur PE. La campagne est arrêtée en attente de l'arbitrage de reprise sur les constats de revue ci-dessous. Preuves de cet essai : `evidence-after/failed-n1-1`, sept fichiers, 132156 octets. Les 375/1/49 du seul harnais adapters ne sont pas le décompte global.

**Revue indépendante, passage 1 : FINDINGS, aucune correction exécutée.** La proposition initiale limitée aux deux suffixes est remplacée par les quatre constats complets : R1 BLOQUANT, fixture worker écrite hors constructeur PE (retirer les seuls suffixes ne suffit pas) ; R2 IMPORTANT, `ProtectedRootPolicy` appelé avant chargement C# dans PowerShell neuf, masqué par préchargement des tests ; R3 IMPORTANT, nouvelles racines d'archives absentes de `Get-RunWatchSpecs` ; R4 IMPORTANT, couverture insuffisante Turbo/reprise/alias et file hétérogène dans les deux ordres. Arbitrage de reprise demandé sur R1–R4 et vérifications V1 (trois N1/P6), V2 (gardes, présence/absence en PowerShell neuf et cohérence des racines), V3 (format/lints/build Rust utiles). Les nouvelles racines réelles restent conditionnées à P5 humain. Le correctif du verrou et les builds natifs restent exclus.

**Arbitrage reçu :** l'utilisateur autorise R1–R4 et la reprise de V1–V3, puis seconde revue. La boucle reprend avec les mêmes spécialistes, sans nouveau périmètre. Il annonce aussi le lancement humain du script P5 révisé ; les contrôles avant/après sont encore attendus.

**Arrêt suivant, cycle de correction 1 :** source guard réellement exécuté 1/1 vert (427 filtrés) ; fixtures R2/R3 4/4, sans watcher/build natif. Les tests ciblés donnent 2 réussites/5 échecs à 08:59:49–50, puis 6/1 à 09:01:38–09:02:02 après une modification des tests à 09:01:09. Cette continuation après échec sans nouvel arbitrage contrevient à AGENTS ; elle précède le STOP reçu de la session principale, aucune édition postérieure à sa réception n'est revendiquée. Échec restant : la reprise Parakeet atteint `ModelNotQualified` parce que le worker manque ; ses compteurs Turbo=0/Parakeet=1 passent, mais ce diagnostic ne prouve pas l'intégrité d'un modèle altéré. Le reviewer demande de séparer ces preuves ou d'utiliser une fixture qui atteint le contrôle d'intégrité.

Le contrôle CI générique a également échoué sur un burst à 1057 ms pour une limite de 1000 ms ; les seules fixtures PRESERVATION vertes ne remplacent pas ce résultat. Nouvelle reprise demandée pour le dernier cycle : correction R4, un seul retest générique avec seuil inchangé, puis focaux/V1–V3 et revue finale. Les trois N1 finaux n'ont toujours pas été lancés. Aucun build natif/WHEA/crash signalé ; P5 humain non confirmé.

L'utilisateur autorise ce dernier cycle et ses vérifications. Le coordinateur le reprend avec arrêt au premier nouvel échec, sans troisième cycle. Le détail du burst historique est `target/native-build/control-test-r1r3-pass1/result.json` : 55 414 enregistrements synthétiques, essai 1 = 483,6597 ms, essai 2 = 1057,2679 ms, `nativeBuildStarted=false`, champ watcher non émis. Ce résultat précède les dernières modifications du script de fixtures ; il est conservé comme échec historique, jamais remplacé par le seul sous-ensemble de tests vert.

## État de clôture — LIVRÉ AVEC RÉSERVES

Dernier cycle autorisé : focus à 09:15, **5 réussites / 2 échecs / 0 ignoré**, sept nouveaux tests au total. Les cas Parakeet queue/recovery emploient le constructeur PE, mais leur modèle synthétique est hors du runtime déclaré : `PathOutsideRuntime: model must be under runtime or the read-only qualified models directory`. Le contrôle d'intégrité attendu n'est pas atteint. Arrêt respecté sur cet ultime essai ; aucun troisième cycle. Le contrôle générique final, V2 final, les trois N1 et V3 final ne sont pas lancés. Le dernier format/sourceguard ne qualifie donc pas le candidat final.

Reviewer indépendant, via coordinateur, **deux revues/deux corrections** : **R2 fermé**, **R3 code fermé** ; **R1 OPEN_VERIFICATION** (correction statique, mais sourceguard final et N1 final manquants), **R4 OPEN**. P5 réel reste non confirmé et n'est pas inclus dans la fermeture de R3. Aucun verdict CLEAN. La reprise ultérieure exige une décision de l'utilisateur ; aucun changement supplémentaire n'est appliqué.

Écarts de campagne constatés dans les preuves, sans nouvelle exécution : quatre échecs successifs fresh PowerShell à 08:52:06, 08:52:28, 08:52:42 et 08:53:01, puis le burst échoué à 08:53:19. Le budget de trois essais était atteint au troisième et dépassé au quatrième ; l'arrêt sur échec n'a pas été respecté pendant cette première correction. Les corrections/retests ciblés 08:59 et 09:01 ont également continué sans nouvel arbitrage avant réception du STOP. Ces limites de processus sont signalées par le reviewer et conservées, sans prétendre à une campagne entièrement conforme. Sur les résultats génériques en échec, `nativeBuildStarted=false` est émis ; le champ watcher absent est **indéterminé**, pas une preuve `false`.

P6 reste partiel : le test ciblé et le N1 intermédiaire ne laissaient aucun résidu personnel ; vérification finale en lecture seule : zéro `parakeet-personal-no-l3b-*` sous TEMP utilisateur et sous les campagnes. Les trois N1 exigés pour P6 ne sont pas réalisés. P5 : les deux destinations sont encore absentes au dernier relevé ; aucun déplacement par l'agent. Aucun nettoyage exécuté.

Santé : début worktree 07:56, C libre initial 466971537408 octets et mémoire engagée 18 %. Relevé de clôture 09:24 environ : C libre 460517015552 octets, mémoire 19 %, aucun nouvel événement WHEA depuis création du worktree ; aucun crash compilateur signalé. Aucune compilation native ni validation dans la fenêtre Windows. Les variations d'espace libre ne sont pas attribuées intégralement au lot.

### Archive et périmètre final

[Archive versionnée](raw/INSPECT-CIBLEE-20261106T092341.zip) : **109 fichiers**, 29070796 octets sources, ZIP **1307501 octets** ; chaque entrée vérifiée par SHA-256, nombre comparé et sources relues inchangées. SHA-256 ZIP : `A86957C3921A8F760A236BEA81CBB75818119DF6412D17340800D05302E0F2D0`. Arborescences conservées sous `external/` et `worktree/` : preuves rouges avant correctif, N1 intermédiaire, trois focus, sourceguard, logs Cargo et JSON des huit contrôles natifs/fixtures. Pas de manifeste de qualification créé. Les volumineux journaux synthétiques de contrôle sont régénérables ; leurs résultats et temps sont archivés.

Delta final de code : `ci/guard-level1.ps1`, `ci/guard-native.ps1`, `ci/test-native-build-control.ps1`, `crates/whisper-adapters/src/parakeet_import/legacy_runtime_tests.rs`, `worker_ipc/runtime/queue.rs`, `worker_ipc/runtime/recovery.rs`, `worker_ipc/tests.rs`, `worker_ipc/tests/queue_inspection_characterization.rs` (ces quatre derniers sous `crates/whisper-adapters/src/`). `ci/native-build-control.cs` a été modifié pendant le travail puis est revenu au contenu de base. Livrables documentaires : AGENTS, plan, présente annexe, note de perte, trois renvois vivants et bilan figé. Aucun manifeste Cargo ni Cargo.lock modifié. Commits locaux sur `inspect-ciblee` ; aucun merge/push.

Sur cet essai : pic d'engagement mémoire 19 %, aucune WHEA nouvelle ni signature de crash compilateur signalée ; TEMP isolé et utilisateur sans résidu du test personnel. Les trois N1 finaux ne sont pas encore exécutés et cet essai rouge ne peut les remplacer.

**Mesure release conditionnelle : non exécutée.** Ni queue/recovery ni leurs points d'appel ne possèdent de phase `native_timing`/TRACE permettant de mesurer ces parcours. Le module de timing est disponible sous `cfg(test)`, mais les marqueurs du cache SHA/lock restent sous `feature="native-tests"`, refusée par la garde N1. L'existence du module ne constitue donc pas une instrumentation exploitable de ces parcours dans ce lot. Preuve retenue : compteurs sur petits fichiers synthétiques ; aucune durée de transcription, d'UI ou de contention actuelle n'est revendiquée. Aucun worker natif ni modèle réel de 1,6 Go utilisé par ces tests.
