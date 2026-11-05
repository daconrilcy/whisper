# Shared by the native driver and its guard. No work is performed by dot-sourcing.
function Get-NativeWorkspacePath([string]$Variant) {
    if ($Variant -cnotin @('cpu','gpu')) { throw 'Native workspace requires cpu or gpu.' }
    return [IO.Path]::GetFullPath("C:/wb/native/$Variant").TrimEnd('\')
}

function Assert-NativeWorkspacePath([string]$Path, [string]$Variant) {
    $expected=Get-NativeWorkspacePath $Variant
    if ([IO.Path]::GetFullPath($Path).TrimEnd('\') -ine $expected) { throw "Native workspace must be exactly $expected" }
    $cursor=$expected
    while ($cursor) {
        if ((Test-Path -LiteralPath $cursor) -and ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Native workspace crosses a reparse point: $cursor" }
        $parent=Split-Path -Parent $cursor
        if ($parent -eq $cursor) { break }; $cursor=$parent
    }
    return $expected
}

function Assert-NativePlainDirectory([string]$Path,[string]$Root) {
    $full=[IO.Path]::GetFullPath($Path).TrimEnd('\'); $base=[IO.Path]::GetFullPath($Root).TrimEnd('\')
    if ($full -ine $base -and -not $full.StartsWith($base+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Native preparation directory escaped its root.' }
    $cursor=$full
    while ($cursor) {
        $item=Get-Item -LiteralPath $cursor -Force -ErrorAction Stop
        if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Native preparation requires an ordinary directory: $cursor" }
        $parent=Split-Path -Parent $cursor; if ($parent -eq $cursor) { break }; $cursor=$parent
    }
}

function Get-NativePlainTree([string]$Root) {
    Assert-NativePlainDirectory $Root $Root
    $pending=[Collections.Generic.Stack[string]]::new(); $pending.Push($Root)
    while ($pending.Count) {
        $directory=$pending.Pop()
        Assert-NativePlainDirectory $directory $Root
        foreach ($item in Get-ChildItem -LiteralPath $directory -Force -ErrorAction Stop) {
            $relative=[IO.Path]::GetRelativePath($Root,$item.FullName).Replace('\','/')
            $link=[bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)
            [pscustomobject]@{path=$item.FullName;relative=$relative;reparse=$link;directory=$item.PSIsContainer}
            if ($item.PSIsContainer -and -not $link) { $pending.Push($item.FullName) }
        }
    }
}

function Get-NativeCacheJunction([string]$Workspace) {
    $links=@(Get-NativePlainTree $Workspace | Where-Object reparse)
    if (-not $links.Count) { return $null }
    $relative='userprofile/AppData/Local/Microsoft/Windows/INetCache/Content.IE5'
    $link=[IO.Path]::GetFullPath((Join-Path $Workspace $relative))
    if ($links.Count -ne 1 -or $links[0].path -ine $link) { throw 'Workspace contains an unexpected or additional reparse point; nothing removed.' }
    Assert-NativePlainDirectory (Split-Path -Parent $link) $Workspace
    $target=[IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $link) 'IE'))
    Assert-NativePlainDirectory $target $Workspace
    $item=Get-Item -LiteralPath $link -Force -ErrorAction Stop
    $targets=@($item.Target)
    if ($item.LinkType -cne 'Junction' -or $targets.Count -ne 1 -or [string]::IsNullOrWhiteSpace($targets[0]) -or -not [IO.Path]::IsPathFullyQualified($targets[0]) -or [IO.Path]::GetFullPath($targets[0]).TrimEnd('\') -ine $target) { throw 'INetCache junction type or exact target differs; nothing removed.' }
    $raw=@(& (Join-Path $env:SystemRoot 'System32/fsutil.exe') reparsepoint query $link 2>&1)
    if ($LASTEXITCODE -ne 0 -or @($raw | Where-Object { [string]$_ -match '0xa0000003\b' }).Count -ne 1) { throw 'INetCache reparse tag could not be verified; nothing removed.' }
    return [pscustomobject]@{path=$link;relative=$relative;target=$target;targetRelative=[IO.Path]::GetRelativePath($Workspace,$target);tag='0xa0000003';type='Junction';raw=($raw -join "`n")}
}

function Get-NativeTargetSnapshot([string]$Target) {
    @(Get-NativePlainTree $Target | Sort-Object relative | ForEach-Object {
        if ($_.reparse) { throw 'Cache target contains a reparse point.' }
        if ($_.directory) { [pscustomobject]@{path=$_.relative;kind='directory'} }
        else { [pscustomobject]@{path=$_.relative;kind='file';bytes=(Get-Item -LiteralPath $_.path -Force).Length;sha256=(Get-FileHash -LiteralPath $_.path -Algorithm SHA256).Hash} }
    })
}

function Remove-NativeDirectoryJunction([string]$Path) {
    [IO.Directory]::Delete($Path,$false)
}

function Remove-NativeCacheJunction([string]$Workspace,[string]$OutputRoot,[object]$Owner) {
    # The production caller has already verified the fixed root and inactive owner.
    $junction=Get-NativeCacheJunction $Workspace
    if ($null -eq $junction) { return }
    $repoRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\')
    Assert-NativePlainDirectory $OutputRoot (Join-Path $repoRoot 'target/native-build')
    $before=@(Get-NativeTargetSnapshot $junction.target)
    $receipt=[ordered]@{schemaVersion=1;status='PREPARED';workspace=$Workspace;owner=$Owner;junction=$junction;targetSnapshot=$before;operation='Directory.Delete(junction,false)';preparedUtc=[DateTime]::UtcNow.ToString('o')}
    $path=Join-Path $OutputRoot 'junction-preparation.json'
    $stream=[IO.File]::Open($path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try { $writer=[IO.StreamWriter]::new($stream,[Text.UTF8Encoding]::new($false)); try { $writer.Write(($receipt|ConvertTo-Json -Depth 12)) } finally { $writer.Dispose() } } finally { $stream.Dispose() }
    try {
        $again=Get-NativeCacheJunction $Workspace
        if ($null -eq $again -or ($again|ConvertTo-Json -Compress) -cne ($junction|ConvertTo-Json -Compress) -or (ConvertTo-Json -InputObject @(Get-NativeTargetSnapshot $junction.target) -Depth 8 -Compress) -cne (ConvertTo-Json -InputObject $before -Depth 8 -Compress)) { throw 'Junction or target changed before removal.' }
        # Deliberately nonrecursive: removes the directory junction, never its target.
        Remove-NativeDirectoryJunction $junction.path
        if (@(Get-ChildItem -LiteralPath (Split-Path -Parent $junction.path) -Force | Where-Object Name -IEQ 'Content.IE5').Count) { throw 'Cache junction still exists or reappeared.' }
        $after=@(Get-NativeTargetSnapshot $junction.target)
        if ((ConvertTo-Json -InputObject $before -Depth 8 -Compress) -cne (ConvertTo-Json -InputObject $after -Depth 8 -Compress)) { throw 'Cache target changed after junction removal.' }
        if (@(Get-NativePlainTree $Workspace | Where-Object reparse).Count) { throw 'Workspace contains a reparse point after preparation.' }
        $receipt.status='REMOVED_TARGET_PRESERVED'
    } catch { $receipt.status='FAILED'; $receipt.failure=$_.Exception.Message; throw }
    finally { $receipt.finishedUtc=[DateTime]::UtcNow.ToString('o'); [IO.File]::WriteAllText($path,($receipt|ConvertTo-Json -Depth 12),[Text.UTF8Encoding]::new($false)) }
}

function Move-NativeWorkspaceDirectory([string]$Source,[string]$Destination) {
    $sourceFull=[IO.Path]::GetFullPath($Source).TrimEnd('\')
    $destinationFull=[IO.Path]::GetFullPath($Destination).TrimEnd('\')
    if ([IO.Path]::GetPathRoot($sourceFull) -ine [IO.Path]::GetPathRoot($destinationFull)) { throw 'Native workspace preservation requires the same volume.' }
    Assert-NativePlainDirectory $sourceFull $sourceFull
    Assert-NativePlainDirectory (Split-Path -Parent $destinationFull) (Split-Path -Parent $destinationFull)
    if (Test-Path -LiteralPath $destinationFull) { throw 'Native preservation destination already exists; no merge or overwrite.' }
    # The caller has checked ownership, inactive users and absence of child links.
    # Rename the whole directory on this volume, without enumerating/moving its
    # children individually or modifying hidden/system/read-only attributes.
    [IO.Directory]::Move($sourceFull,$destinationFull)
}

function Initialize-NativeWorkspace([string]$Variant, [string]$BuildId, [string]$OutputRoot) {
    $workspace=Assert-NativeWorkspacePath (Get-NativeWorkspacePath $Variant) $Variant
    if ($BuildId -cnotmatch '^[a-zA-Z0-9][a-zA-Z0-9_-]{0,31}$') { throw 'Invalid native workspace owner id.' }
    $repoRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\')
    $output=[IO.Path]::GetFullPath($OutputRoot).TrimEnd('\')
    if (-not $output.StartsWith($repoRoot+'\target\native-build\guard\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Workspace preservation destination must be under the current native guard output.' }
    Assert-NoReparseChain $output 'native workspace preservation destination'
    $processes=@(Get-CimInstance Win32_Process -Property ProcessId,Name,CommandLine,CreationDate -ErrorAction Stop)
    foreach ($p in $processes) {
        if ($p.CommandLine -and ([string]$p.CommandLine).Replace('/','\').Contains($workspace,[StringComparison]::OrdinalIgnoreCase)) { throw "Native workspace is still referenced by process $($p.ProcessId)." }
        # Windows process inventory does not expose cwd: relative arguments cannot
        # establish that a native process is unrelated to this workspace.
        if ($p.Name -in @('cmake.exe','ninja.exe','cl.exe','link.exe','lib.exe','vcpkg.exe','nvcc.exe','cicc.exe','ptxas.exe')) { throw "Native tool is active before workspace preservation: $($p.Name), PID $($p.ProcessId)." }
    }
    if (Test-Path -LiteralPath $workspace) {
        $marker=Join-Path $workspace '.whisper-native-workspace.json'
        $contents=@(Get-ChildItem -LiteralPath $workspace -Force)
        if ($contents.Count) {
            if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) { throw 'Existing nonempty workspace has no ownership marker; nothing moved.' }
            $owner=Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
            if ($owner.schemaVersion -ne 1 -or $owner.product -cne 'Whisper NativeBuild' -or $owner.variant -cne $Variant) { throw 'Workspace ownership differs; nothing moved.' }
            if (@($processes | Where-Object ProcessId -EQ $owner.guardPid).Count) { throw 'Previous workspace owner PID is still active; nothing moved.' }
            Remove-NativeCacheJunction $workspace $output $owner
            if (@(Get-ChildItem -LiteralPath $workspace -Recurse -Force -Attributes ReparsePoint).Count) { throw 'Workspace contains reparse points; nothing moved.' }
            $destination=Join-Path $output 'previous-workspace'
            if (Test-Path -LiteralPath $destination) { throw 'Workspace preservation destination already exists.' }
            # Both absolute endpoints were checked; preservation is a move, never a recursive deletion.
            Move-NativeWorkspaceDirectory $workspace $destination
        }
    }
    New-Item -ItemType Directory -Path $workspace -Force | Out-Null
    $self=@($processes | Where-Object ProcessId -EQ $PID)
    if ($self.Count -ne 1) { throw 'Current workspace owner identity unavailable.' }
    [ordered]@{schemaVersion=1;product='Whisper NativeBuild';variant=$Variant;buildId=$BuildId;guardPid=$PID;guardCreatedUtc=$self[0].CreationDate.ToUniversalTime().ToString('o')} |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $workspace '.whisper-native-workspace.json') -Encoding utf8NoBOM
    return $workspace
}

function Export-NativeSource([string]$Source, [string]$Destination, [string]$Name) {
    if (Test-Path -LiteralPath $Destination) { throw "Materialized source already exists: $Destination" }
    if (@(Get-ChildItem -LiteralPath (Join-Path $Source '.git') -Recurse -Force -Attributes ReparsePoint).Count) { throw 'Source Git metadata contains a reparse point.' }
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    $tar=Join-Path $workspace "temp/$Name.tar"
    Invoke-Logged $git @('--no-optional-locks','-C',$Source,'archive','--format=tar','-o',$tar,'HEAD') "$Name-export"
    Invoke-Logged $tools.cmake @('-E','chdir',$Destination,$tools.cmake,'-E','tar','xf',$tar) "$Name-extract"
    Copy-Item -LiteralPath (Join-Path $Source '.git') -Destination (Join-Path $Destination '.git') -Recurse
}

function Get-NativePeCommit([string]$Path) {
    $bytes=[IO.File]::ReadAllBytes($Path)
    function U16([int]$Offset) { if ($Offset -lt 0 -or $Offset+2 -gt $bytes.Length) { throw 'PE field out of bounds.' }; [BitConverter]::ToUInt16($bytes,$Offset) }
    function U32([int]$Offset) { if ($Offset -lt 0 -or $Offset+4 -gt $bytes.Length) { throw 'PE field out of bounds.' }; [BitConverter]::ToUInt32($bytes,$Offset) }
    if ((U16 0) -ne 0x5a4d) { throw 'Missing MZ signature.' }
    $pe=[int](U32 60)
    if ((U32 $pe) -ne 0x4550 -or (U16 ($pe+4)) -ne 0x8664 -or (U16 ($pe+24)) -ne 0x20b) { throw 'Expected x64 PE32+ image.' }
    $table=$pe+24+(U16 ($pe+20)); $count=U16 ($pe+6)
    function Map-Rva([long]$Value) {
        for ($i=0;$i -lt $count;$i++) {
            $section=$table+40*$i; $rva=U32 ($section+12); $size=U32 ($section+16)
            if ($Value -ge $rva -and $Value -lt $rva+$size) {
                $offset=[long](U32 ($section+20))+$Value-$rva
                if ($offset -lt 0 -or $offset -ge $bytes.Length) { throw 'PE RVA outside file.' }; return [int]$offset
            }
        }; throw 'PE RVA outside raw sections.'
    }
    function Read-Ascii([int]$Offset) {
        $end=$Offset
        while ($end -lt $bytes.Length -and $end-$Offset -lt 256 -and $bytes[$end] -ne 0) { $end++ }
        if ($end -ge $bytes.Length -or $end-$Offset -ge 256) { throw 'Unterminated PE string.' }
        [Text.Encoding]::ASCII.GetString($bytes,$Offset,$end-$Offset)
    }
    $export=Map-Rva (U32 ($pe+24+112)); $names=[int](U32 ($export+24))
    if ($names -gt 65536) { throw 'Unbounded PE export count.' }
    $functions=Map-Rva (U32 ($export+28)); $nameTable=Map-Rva (U32 ($export+32)); $ordinals=Map-Rva (U32 ($export+36))
    for ($i=0;$i -lt $names;$i++) {
        if ((Read-Ascii (Map-Rva (U32 ($nameTable+4*$i)))) -cne 'ggml_commit') { continue }
        $ordinal=U16 ($ordinals+2*$i)
        if ($ordinal -ge (U32 ($export+20))) { throw 'PE export ordinal outside function table.' }
        $rva=U32 ($functions+4*$ordinal); $offset=Map-Rva $rva
        if ($offset+8 -gt $bytes.Length -or $bytes[$offset] -ne 0x48 -or $bytes[$offset+1] -ne 0x8d -or $bytes[$offset+2] -ne 0x05 -or $bytes[$offset+7] -ne 0xc3) { throw 'ggml_commit is not the expected read-only x64 constant getter.' }
        return Read-Ascii (Map-Rva ($rva+7+[BitConverter]::ToInt32($bytes,$offset+3)))
    }
    throw 'ggml_commit export missing.'
}
