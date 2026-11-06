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

Les deux destinations sont absentes au relevé initial. Les commandes suivantes sont proposées à l'humain pour PowerShell ; elles ne sont pas exécutées par l'agent. Ne les lancer qu'en l'absence de lot/processus utilisant ces archives. Elles contrôlent les racines exactes, refusent les liens/jonctions, un inventaire modifié et une destination préexistante, puis vérifient les comptes/octets après chaque `/MOVE`. Aucun parent `target/` n'est déplacé. Un code robocopy >=8 ou un écart impose l'arrêt et la conservation de l'état, sans nettoyage automatique.

```powershell
$ErrorActionPreference = 'Stop'
$migrations = @(
    @{ Source='C:\dev\whisper\target\poll-preserved'; Destination='C:\wb\preserved\poll-preserved'; Files=117586L; Bytes=89160572401L },
    @{ Source='C:\dev\whisper\target\pile-preserved'; Destination='C:\wb\preserved\pile-preserved'; Files=8485L; Bytes=11644240959L }
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
    if (@($entries | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }).Count) { throw 'Lien/jonction dans les archives' }
    $before = $entries | Where-Object { -not $_.PSIsContainer } | Measure-Object Length -Sum
    if ($before.Count -ne $migration.Files -or $before.Sum -ne $migration.Bytes) { throw 'Inventaire initial modifié : relever à nouveau avant déplacement' }
    robocopy $source $destination /E /MOVE /COPY:DAT /DCOPY:DAT /XJ /R:1 /W:1 /NP
    $copyExit = $LASTEXITCODE
    if ($copyExit -ge 8) { throw "Robocopy a échoué : $copyExit" }
    $after = Get-ChildItem -LiteralPath $destination -Recurse -File -Force | Measure-Object Length -Sum
    $remaining = if (Test-Path -LiteralPath $source) { @(Get-ChildItem -LiteralPath $source -Recurse -File -Force).Count } else { 0 }
    [pscustomobject]@{Source=$source;Destination=$destination;BeforeFiles=$before.Count;AfterFiles=$after.Count;BeforeBytes=$before.Sum;AfterBytes=$after.Sum;SourceFiles=$remaining;RobocopyExit=$copyExit}
    if ($after.Count -ne $before.Count -or $after.Sum -ne $before.Sum -or $remaining -ne 0) { throw 'Migration incomplète : conserver les deux emplacements et signaler' }
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

Les trois chemins abrégés du tableau sont sous `C:/dev/whisper/`. Liste observée des fichiers non versionnés : `C:/wb/inspect-ciblee/worktrees-inventory.txt` (28085255 octets). Limite de lecture : Git a signalé des chemins trop longs dans le spike et des refus d'accès/jonctions INetCache dans build-natif ; aucun droit ni lien n'a été modifié pour les traverser. Les nombres de ces deux worktrees ne certifient donc pas un inventaire exhaustif. Cette limite justifie leur conservation.

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

`ARTIFACT_HASH_CACHE` est un `OnceLock<Mutex<ModelFileHashCache>>` global au processus parent. `cached_file_sha256:234–252` prend le verrou, appelle `cache.sha256(path)` puis le relâche. Sous le verrou : ouverture, identité/métadonnées, recherche de cache, lecture complète/SHA si cache froid, réouverture/revalidation et insertion. Un hit garde une section courte d'ouverture/identité ; un miss garde le mutex pendant toute la lecture. Les autres modèles, DLL et binaires utilisant ce cache attendent le même verrou. Le temps n'est pas mesuré ici ; le diagnostic antérieur relevait environ 16,5 s pour Turbo ~1,62 Go en debug et n'est pas une borne de cette machine/session.

Le catalogue de `root.rs:61` s'exécute sur un thread dédié : son SHA ne bloque pas directement la boucle d'événements UI. En revanche l'UI attend son snapshot/disponibilité, et les admissions qui utilisent le même cache peuvent attendre son SHA. Sans mesure de chemin UI, ne pas assimiler cette attente au gel de la fenêtre. Proposition hors lot : calculer les misses hors mutex puis revérifier/inserer sous verrou, avec coordination par identité pour éviter les doubles lectures et conserver la revalidation des fichiers. Cette modification mérite ses propres tests de concurrence ; elle n'est pas appliquée ici.

## Documents figés et liens historiques

Les bilans `bilan-parakeet-faisabilite-cloture.md`, `bilan-parakeet-fr-en-criteres-revises.md`, `bilan-parakeet-fr-en-go.md`, `bilan-parakeet-limite-decoupe-longue.md`, `bilan-parakeet-limite-vad-revisee.md` conservent leurs références au banc perdu. Ils ne sont pas modifiés. Les documents figés `BILAN-FUSION-PILE.md` et `POLL-INTEGRATION-NETTOYAGE.md` gardent les anciens chemins `target/pile-preserved` et `target/poll-preserved` après P5 ; la correspondance de migration figure dans le tableau P5. Les anciens plans/fiches/recettes citant feasibility restent historiques ; seules les trois références vivantes demandées reçoivent le renvoi à la note de perte.

## Rétention et limites

Le worktree `C:/wt/inspect-ciblee` et les commits de la branche sont conservés. Les tailles finales des journaux, temporaires et de `C:/wb/inspect-ciblee` ainsi que les commandes humaines de suppression seront relevées à la clôture. Aucun bilan figé antérieur n'est réécrit.
