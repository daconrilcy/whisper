Avancée : inventaire et commandes préparés le 7 novembre 2026 ; manifeste des copies approuvé dans la session courante ; aucune suppression exécutée.
Blocage : aucun pour la préparation ; l'exécution du nettoyage n'est pas autorisée par la demande « Ne supprime rien ».
Décision : conserver toutes les données en place ; proposer uniquement les chemins recensés, avec simulation par défaut.

# Refacto-poll — proposition de nettoyage des intermédiaires

Complément aux bilans figés de [phase 1](../architecture/REFACTO-POLL-UNIFIED-PHASE1-BILAN.md) et de [phase 2](../architecture/REFACTO-POLL-UNIFIED-BILAN.md), selon la rétention du [plan](plan-refacto-poll-unified.md). Base locale observée : `main` et `refacto-poll-unified` à `ee3a4ccb`. La demande courante autorise la préparation, **aucune suppression**. Aucun Cargo, arrêt de processus, nettoyage ou push effectué.

**Approbation du 7 novembre 2026.** Après remise de la proposition `ba276844`, l'utilisateur répond « ok approuvé ». L'approbation du manifeste des 18 303 copies et audios est consignée ; aucun chemin ni volume n'est ajouté. La consigne précédente « Ne supprime rien » reste appliquée. Les indicateurs d'exécution et d'inclusion des copies restent désactivés dans les commandes ci-dessous.

## Liste exacte et volume

La [liste des 457 dossiers](nettoyage-refacto-poll-dossiers.csv) donne pour chaque chemin absolu son identifiant, ses octets, son nombre de fichiers et sa dernière écriture. Les chemins sont disjoints : aucun parent sélectionné ne contient un autre dossier sélectionné.

| Ensemble proposé, sous réserve des contrôles avant exécution | Dossiers | Octets | Gio logiques |
|---|---:|---:|---:|
| Cache Cargo isolé N2 : `C:\wt\refacto-poll\target\n2-native\cargo-home` | 1 | 651 937 640 | 0,607 |
| Autres intermédiaires sans copies de modèles/audio, preuves archivées | 410 | 224 564 671 | 0,209 |
| Dossiers contenant des copies ou audios temporaires : manifeste approuvé | 46 | 395 037 210 574 | 367,907 |
| **Total** | **457** | **395 913 712 885** | **368,723** |

Les 411 premiers dossiers représentent 876 502 311 octets, soit 0,816 Gio. Les deux plus volumineux du troisième ensemble sont `C:\wt\refacto-poll\target\refacto-poll-unified\temp` (R457, 193,143 Gio) et `C:\wt\refacto-poll\target\refacto-poll-unified\phase2\temp` (R136, 37,599 Gio). Les 422 dossiers N1 représentent 347,279 Gio ; les 35 dossiers N2, 21,445 Gio. **Ne pas remplacer les lignes du CSV par la suppression des racines N1/N2.** Des fichiers et des jonctions y restent volontairement en place.

Le [manifeste approuvé des copies et audios](nettoyage-refacto-poll-copies.csv) énumère **18 303 fichiers** : 529 copies de modèles de campagne, 897 fixtures binaires/modèles synthétiques et 16 877 audios temporaires. Le statut est `APPROUVE_SESSION_2026-11-07`. Leur origine est celle des temporaires de tests et du staging N2 ; notamment le corpus `auto_fallback` copie le modèle dans son répertoire temporaire. Les copies n'ont pas toutes été rehachées. Le manifeste ne contient ni modèle installé ni audio source canonique. Les fixtures du cache Cargo isolé sont traitées comme contenu régénérable du cache.

## Conservation et contrôles observés

Tous les chemins ci-dessous ont été vérifiés présents ; ils sont hors sélection :

- les trois archives de preuves ci-dessous, les documents et fixtures suivis par Git ; les deux archives historiques existent aussi dans `C:\wt\refacto-poll\docs\architecture\raw` ;
- `C:\wt\refacto-poll\deliverables\refacto-poll-unified` : les trois EXE de remise, dont le desktop validé ;
- `C:\wt\refacto-poll\target\parakeet-tests\cargo-shared` et `C:\wt\refacto-poll\target\cargo-home-shared` ;
- `C:\dev\whisper\target\parakeet-tests\cargo-shared`, `C:\dev\whisper\target\cargo-home-shared` et `C:\dev\whisper\native\cache` ;
- `C:\Users\cyril\AppData\Local\Whisper\models`, `worker` et `native` ; les SHA-256 des deux modèles installés correspondent encore à ceux de la remise ;
- l'audio source `C:\wb\native-qualif\reprise-5\steps\GpuReference2\n2-outputs\NativeClosureGpuReference-7abb1bbf7e06446aa132b4498f638e45\l04\fixtures\en-short.mp3` ;
- le worktree `C:\wt\refacto-poll`, sa branche, ses métadonnées Git et tous les autres dossiers non listés. `C:\wb\refacto-poll` est absent et n'est pas proposé.

Les [16 jonctions et leurs cibles](nettoyage-refacto-poll-liens.csv) sont exclues, ainsi que les parents dont la suppression les engloberait. Le parcours des 122 432 fichiers sélectionnés n'a trouvé aucun fichier en lecture seule. Les [3 770 noms de liens physiques](nettoyage-refacto-poll-liens-physiques.csv), regroupés en 1 281 groupes, pointent tous dans la sélection ; aucun alias externe n'a été trouvé. Leur double comptage logique représente 88 284 octets. L'espace réellement récupéré dépend aussi de l'allocation du volume : 368,723 Gio est une somme de tailles logiques.

Au contrôle du 7 novembre vers 21 h 37, aucun processus de compilation, VCTIP, garde ou Whisper pertinent n'était actif ; les autres sessions consultées étaient inactives. Cette observation ne garantit pas une absence d'utilisation future. Les commandes refusent les activités pertinentes détectées et revérifient les chemins ; exécuter ultérieurement dans une session sans campagne concurrente. Elles ne verrouillent pas préventivement tout le volume. Espace libre observé avant tout nettoyage : **89,161 Gio**.

## Preuves archivées

Les deux archives historiques sont inchangées. Le complément conserve l'arborescence relative au worktree et a été rouvert : ses **72 563 membres ont chacun été comparés par SHA-256 à leur source**, sans supprimer celle-ci. Sur les deux racines de campagne, 77 365 petits fichiers de preuves/données sont couverts : 4 802 déjà identiques dans les archives historiques et 72 563 dans le complément. Les binaires régénérables, modèles/audio et le cache Cargo isolé sont exclus de cette collecte ; les fichiers en attente de publication, logs et autres petites données sont inclus. Aucun manifeste autonome d'empreintes n'est créé.

| Archive sous `C:\dev\whisper\docs\architecture\raw` | Membres | Octets | SHA-256 |
|---|---:|---:|---|
| [REFACTO-POLL-UNIFIED-PHASE1-PREUVES.zip](../architecture/raw/REFACTO-POLL-UNIFIED-PHASE1-PREUVES.zip) | 1 239 | 2 839 439 | `6DC647709B97F992603A11D01C110177C3335E55E29EDCCFD445306DC0B3ABEB` |
| [refacto-poll-unified-preuves.zip](../architecture/raw/refacto-poll-unified-preuves.zip) | 3 613 | 4 087 265 | `308C0E63A3B103859DF8ECED5AA07FD6D65F9888777803351823C38175677570` |
| [REFACTO-POLL-UNIFIED-NETTOYAGE-COMPLEMENT.zip](../architecture/raw/REFACTO-POLL-UNIFIED-NETTOYAGE-COMPLEMENT.zip) | 72 563 | 47 981 843 | `211C49601E03A47298454737A5F20F5BA016143097981AE6DD658BC16B3ADA76` |

## Commandes proposées, non exécutées

Dans **PowerShell 7.2 ou ultérieur**, le bloc ci-dessous contrôle tous les chemins avant le premier appel de suppression. Par défaut, il ne sélectionne que les 411 dossiers sans copies et utilise `-WhatIf` : simulation. Après une autorisation ultérieure de suppression, `$Executer = $true` permet l'exécution ; `$InclureCopies = $true` permet d'inclure les copies du manifeste désormais approuvé. L'accord sur ce manifeste est acquis ; aucune autorisation d'exécution n'est déduite de la demande de préparation.

Une différence d'inventaire, de preuve, un lien externe ou une activité pertinente provoque un arrêt, sans suppression ni arrêt de processus automatique. Si une suppression future échoue après avoir commencé, conserver l'état et relever les chemins effectivement supprimés avant de proposer une suite ; ne pas relancer aveuglément. Le résultat et l'espace libéré appartiendront à un nouveau compte rendu, pas aux bilans figés.

```powershell
#requires -Version 7.2
$ErrorActionPreference = 'Stop'
$Executer = $false
$InclureCopies = $false # Manifeste approuvé ; inclusion désactivée par défaut.
$depot = 'C:\dev\whisper'
$worktree = 'C:\wt\refacto-poll'
$docs = Join-Path $depot 'docs\cadrage'
$rows = @(Import-Csv -LiteralPath "$docs\nettoyage-refacto-poll-dossiers.csv")
$copies = @(Import-Csv -LiteralPath "$docs\nettoyage-refacto-poll-copies.csv")
$liens = @(Import-Csv -LiteralPath "$docs\nettoyage-refacto-poll-liens.csv")
if ($rows.Count -ne 457 -or $copies.Count -ne 18303 -or $liens.Count -ne 16) {
    throw 'Inventaires différents de la proposition : réexaminer avant de continuer.'
}
$selection = @($rows | Where-Object { $InclureCopies -or [int]$_.MediaFiles -eq 0 })
$racines = @("$worktree\target\refacto-poll-unified", "$worktree\target\n2-native")
$exclus = @($liens.Path) + @($liens.Target)
function EstSous([string]$chemin, [string]$parent) {
    $chemin.StartsWith($parent.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)
}
function VerifierAncetres([string]$chemin) {
    for ($p = Get-Item -LiteralPath $chemin -Force; $null -ne $p; $p = $p.Parent) {
        if ($p.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Lien ou jonction interdits : $($p.FullName)"
        }
    }
}
function VerifierActivite {
    $actifs = @(Get-CimInstance Win32_Process | Where-Object {
        $_.ProcessId -ne $PID -and (
            $_.Name -match '^(cargo|rustc|rustdoc|cl|link|cmake|ninja|mspdbsrv|vctip|whisper[^.]*)\.exe$' -or
            ($_.CommandLine -and (
                $_.CommandLine -match [regex]::Escape($worktree) -or
                $_.CommandLine -match 'guard-(level1|native)\.ps1'
            ))
        )
    })
    if ($actifs.Count) {
        $actifs | Select-Object Name, ProcessId, ExecutablePath | Format-Table
        throw 'Activité à examiner avant nettoyage ; aucun processus arrêté.'
    }
}
VerifierActivite
$proteges = @(
    "$worktree\deliverables\refacto-poll-unified",
    "$worktree\target\parakeet-tests\cargo-shared", "$worktree\target\cargo-home-shared",
    "$depot\target\parakeet-tests\cargo-shared", "$depot\target\cargo-home-shared",
    "$depot\native\cache", 'C:\Users\cyril\AppData\Local\Whisper\models',
    'C:\Users\cyril\AppData\Local\Whisper\worker', 'C:\Users\cyril\AppData\Local\Whisper\native',
    'C:\wb\native-qualif\reprise-5\steps\GpuReference2\n2-outputs\NativeClosureGpuReference-7abb1bbf7e06446aa132b4498f638e45\l04\fixtures\en-short.mp3'
)
foreach ($p in $proteges) { if (-not (Test-Path -LiteralPath $p)) { throw "Absent : $p" } }
foreach ($r in $selection) {
    $p = [IO.Path]::GetFullPath($r.Path).TrimEnd('\')
    if ($p -cne $r.Path -or -not (Test-Path -LiteralPath $p -PathType Container)) {
        throw "Chemin non conforme ou absent : $($r.Path)"
    }
    if (-not @($racines | Where-Object { EstSous $p $_ }).Count) { throw "Hors lot : $p" }
    foreach ($e in $exclus + $proteges) {
        if ($p -ieq $e -or (EstSous $p $e) -or (EstSous $e $p)) { throw "Conservation : $p / $e" }
    }
    if (@($selection | Where-Object { $_.Path -ine $p -and (EstSous $_.Path $p) }).Count) {
        throw "Chevauchement des dossiers : $p"
    }
    VerifierAncetres $p
}

# Relire les archives et construire en mémoire les empreintes des preuves.
$archives = @(
    @('REFACTO-POLL-UNIFIED-PHASE1-PREUVES.zip', 1239, '6DC647709B97F992603A11D01C110177C3335E55E29EDCCFD445306DC0B3ABEB'),
    @('refacto-poll-unified-preuves.zip', 3613, '308C0E63A3B103859DF8ECED5AA07FD6D65F9888777803351823C38175677570'),
    @('REFACTO-POLL-UNIFIED-NETTOYAGE-COMPLEMENT.zip', 72563, '211C49601E03A47298454737A5F20F5BA016143097981AE6DD658BC16B3ADA76')
)
$preuves = @{}
foreach ($a in $archives) {
    $p = Join-Path "$depot\docs\architecture\raw" $a[0]
    if ((Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash -ne $a[2]) { throw "Archive différente : $p" }
    if ($a[0] -ne 'REFACTO-POLL-UNIFIED-NETTOYAGE-COMPLEMENT.zip') {
        $double = Join-Path "$worktree\docs\architecture\raw" $a[0]
        if ((Get-FileHash -LiteralPath $double -Algorithm SHA256).Hash -ne $a[2]) { throw "Archive worktree différente : $double" }
    }
    $zip = [IO.Compression.ZipFile]::OpenRead($p)
    try {
        if ($zip.Entries.Count -ne $a[1]) { throw "Nombre de membres différent : $p" }
        foreach ($e in $zip.Entries) {
            $stream = $e.Open()
            try { $h = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream)) }
            finally { $stream.Dispose() }
            $preuves[$e.FullName] = @($preuves[$e.FullName]) + $h
        }
    } finally { $zip.Dispose() }
}
$media = @{}
foreach ($m in $copies) {
    if ($media.ContainsKey($m.Path)) { throw "Doublon du manifeste : $($m.Path)" }
    $media[$m.Path] = $m
}
$extensionsMedia = @('.gguf','.ggml','.onnx','.bin','.wav','.mp3','.pcm')
$regenerables = @('.exe','.dll','.pdb','.lib','.obj','.exp','.a','.rlib','.rmeta','.crate') + $extensionsMedia
$fichiers = [Collections.Generic.List[IO.FileInfo]]::new()
$noms = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($r in $selection) {
    [long]$octets = 0; $nombre = 0; $nbMedia = 0; $dernier = [datetime]::MinValue
    $pile = [Collections.Generic.Stack[IO.DirectoryInfo]]::new()
    $pile.Push([IO.DirectoryInfo]::new($r.Path))
    while ($pile.Count) {
        foreach ($f in $pile.Pop().GetFileSystemInfos()) {
            if ($f.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Lien rencontré : $($f.FullName)" }
            if ($f -is [IO.DirectoryInfo]) { $pile.Push($f); continue }
            if ($f.IsReadOnly) { throw "Lecture seule : $($f.FullName)" }
            $octets += $f.Length; $nombre++; $fichiers.Add($f)
            if (-not $noms.Add($f.FullName)) { throw "Fichier sélectionné deux fois : $($f.FullName)" }
            if ($f.LastWriteTimeUtc -gt $dernier) { $dernier = $f.LastWriteTimeUtc }
            if ($r.Id -eq 'R001') { continue } # Cache Cargo isolé, régénérable.
            if ($f.Extension -in $extensionsMedia) {
                $m = $media[$f.FullName]
                if (-not $InclureCopies -or -not $m -or $m.CandidateId -ne $r.Id -or [long]$m.Bytes -ne $f.Length) {
                    throw "Copie non couverte par la sélection approuvée : $($f.FullName)"
                }
                $nbMedia++
            } elseif ($f.Extension -notin $regenerables) {
                $rel = [IO.Path]::GetRelativePath($worktree, $f.FullName).Replace('\','/')
                $h = (Get-FileHash -LiteralPath $f.FullName -Algorithm SHA256).Hash
                if ($h -notin @($preuves[$rel])) { throw "Preuve non archivée à l'identique : $($f.FullName)" }
            }
        }
    }
    $date = [datetime]::Parse($r.LatestWriteUtc, [Globalization.CultureInfo]::InvariantCulture, [Globalization.DateTimeStyles]::RoundtripKind)
    if ($octets -ne [long]$r.Bytes -or $nombre -ne [int]$r.Files -or $nbMedia -ne [int]$r.MediaFiles -or $dernier.Ticks -ne $date.Ticks) {
        throw "Inventaire modifié : $($r.Path)"
    }
}
# Refuser tout alias physique extérieur à la sélection courante.
$groupesVus = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
foreach ($f in $fichiers) {
    if ($f.LinkType -ne 'HardLink' -or $groupesVus.Contains($f.FullName)) { continue }
    $alias = @(& fsutil.exe hardlink list $f.FullName)
    if ($LASTEXITCODE -ne 0 -or $alias.Count -lt 2) { throw "Liens physiques non vérifiés : $($f.FullName)" }
    foreach ($nom in $alias) {
        $nom = $nom.Trim()
        if (-not $nom.StartsWith('\')) { throw "Alias non reconnu : $nom" }
        $p = [IO.Path]::GetPathRoot($f.FullName).TrimEnd('\') + $nom
        if (-not $noms.Contains($p)) { throw "Lien physique hors sélection : $p" }
        [void]$groupesVus.Add($p)
    }
}
VerifierActivite
$selection | Select-Object Id, Path, GiB | Format-Table -AutoSize
foreach ($r in $selection) {
    VerifierAncetres $r.Path
    Remove-Item -LiteralPath $r.Path -Recurse -Force -WhatIf:(-not $Executer)
}
Get-PSDrive -Name C | Select-Object Name, Used, Free
```

Vérification de préparation : syntaxe PowerShell valide ; partie préalable du bloc exécutée en lecture seule sur les 457 dossiers, avec contrôle des preuves et des liens physiques : **PASS**. L'instruction `Remove-Item` était exclue du code exécuté. Aucun groupe de liens physiques ne traverse la séparation entre les 411 dossiers sans copies et les 46 soumis au manifeste. Inventaires et périmètre contrôlés, `git diff --check` réussi. Les commandes de suppression, y compris en simulation, n'ont pas été lancées. Les CSV sont des inventaires de chemins et tailles, pas des autorisations ni des certificats d'absence d'utilisation.
