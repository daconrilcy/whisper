# Persistent, verified local vcpkg binary generations. No work on dot-sourcing.
# Provider syntax: https://learn.microsoft.com/en-us/vcpkg/reference/binarycaching
function Assert-NativeBinaryPath([string]$Path,[string]$Root){
    $full=[IO.Path]::GetFullPath($Path).TrimEnd('\');$base=[IO.Path]::GetFullPath($Root).TrimEnd('\')
    if($full -ine $base -and -not $full.StartsWith($base+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Binary cache path escaped its declared root.'}
    $cursor=$full
    while($cursor){if((Test-Path -LiteralPath $cursor) -and ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)){throw 'Binary cache path crosses a reparse point.'};$parent=Split-Path $cursor -Parent;if($parent -eq $cursor){break};$cursor=$parent}
    return $full
}
function Get-NativeBinaryKey([Collections.IDictionary]$Inputs){
    $ordered=[ordered]@{}
    foreach($name in @($Inputs.Keys|Sort-Object)){$ordered[$name]=[string]$Inputs[$name]}
    $text=ConvertTo-Json -InputObject $ordered -Compress
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($text))).ToLowerInvariant()
}
function Get-NativePatchFingerprint([object[]]$Historical,[object[]]$Local){
    $lines=@(foreach($group in @(@{kind='historical';files=$Historical},@{kind='local';files=$Local})){
        foreach($file in $group.files){
            if([string]::IsNullOrWhiteSpace($file.name) -or $file.name -match '[\r\n|]' -or $file.sha256 -notmatch '^[0-9A-Fa-f]{64}$'){throw 'Invalid patch fingerprint entry.'}
            $group.kind+'|'+$file.name+'|'+$file.sha256.ToUpperInvariant()
        }
    })
    return $lines -join "`n"
}
function Get-NativeBinaryInventory([string]$Archives){
    [void](Assert-NativeBinaryPath $Archives $Archives)
    $items=@(Get-NativePlainTree $Archives)
    if(@($items|Where-Object reparse).Count){throw 'Binary cache contains a linked path.'}
    $files=@($items|Where-Object {-not $_.directory}|Sort-Object relative)
    return @(foreach($file in $files){
        if($file.reparse){throw 'Binary cache contains a linked file.'}
        if([IO.Path]::GetExtension($file.path) -ine '.zip'){throw [IO.InvalidDataException]::new('Binary cache contains an unexpected non-archive file.')}
        [ordered]@{relative=$file.relative;bytes=(Get-Item -LiteralPath $file.path).Length;sha256=(Get-FileHash -LiteralPath $file.path -Algorithm SHA256).Hash}
    })
}
function Assert-NativeBinaryGeneration([string]$Root,[string]$Key,$Selector){
    if($Selector.generation -notmatch '^[0-9a-f]{32}$' -or $Selector.manifestSha256 -notmatch '^[0-9A-Fa-f]{64}$'){throw 'Binary cache selector/generation policy is invalid; no reconstruction.'}
    $generation=Assert-NativeBinaryPath (Join-Path $Root $Selector.generation) $Root
    $manifestPath=Assert-NativeBinaryPath (Join-Path $generation 'manifest.json') $generation
    if(-not(Test-Path -LiteralPath $manifestPath -PathType Leaf) -or (Get-FileHash -LiteralPath $manifestPath).Hash -cne $Selector.manifestSha256){throw [IO.InvalidDataException]::new('Binary cache manifest digest differs.')}
    try{$manifest=Get-Content -LiteralPath $manifestPath -Raw|ConvertFrom-Json -AsHashtable}catch{throw 'Binary cache manifest JSON policy is invalid; no reconstruction.'}
    if($manifest.schemaVersion -ne 1 -or $manifest.key -cne $Key -or $manifest.generation -cne $Selector.generation -or $manifest.status -cne 'SEALED' -or (Get-NativeBinaryKey $manifest.inputs) -cne $Key){throw 'Binary cache provenance policy differs; no reconstruction.'}
    $archives=Assert-NativeBinaryPath (Join-Path $generation 'archives') $generation
    if(-not(Test-Path -LiteralPath $archives -PathType Container)){throw [IO.InvalidDataException]::new('Binary cache archive folder missing.')}
    $actual=@(Get-NativeBinaryInventory $archives);$expected=@($manifest.files|Sort-Object relative)
    if($actual.Count -eq 0 -or $actual.Count -ne $expected.Count){throw [IO.InvalidDataException]::new('Binary cache file inventory differs.')}
    for($index=0;$index -lt $actual.Count;$index++){
        if($actual[$index].relative -cne $expected[$index].relative -or $actual[$index].bytes -ne $expected[$index].bytes -or $actual[$index].sha256 -cne $expected[$index].sha256){throw [IO.InvalidDataException]::new('Binary cache archive digest or size differs.')}
    }
    return @{mode='REUSE';key=$Key;root=$Root;generation=$Selector.generation;archives=$archives;manifestPath=$manifestPath;manifestSha256=$Selector.manifestSha256;files=$actual;rebuildReason=$null}
}
function Initialize-NativeBinaryCache([string]$Base,[string]$Key){
    if($Key -notmatch '^[0-9a-f]{64}$'){throw 'Binary cache requires a SHA-256 input key.'}
    [void](Assert-NativeBinaryPath $Base $Base)
    $root=Assert-NativeBinaryPath (Join-Path $Base $Key) $Base
    $selectorPath=Assert-NativeBinaryPath (Join-Path $root 'current.json') $root
    $reason='No sealed generation.'
    if(Test-Path -LiteralPath $selectorPath -PathType Leaf){
        try{
            try{$selector=Get-Content -LiteralPath $selectorPath -Raw|ConvertFrom-Json -AsHashtable}catch{throw 'Binary cache selector JSON policy is invalid; no reconstruction.'}
            return Assert-NativeBinaryGeneration $root $Key $selector
        }catch [IO.InvalidDataException]{$reason=$_.Exception.Message}
    }
    # Invalid archives are preserved; rebuilding never removes or merges a generation.
    $generation=[guid]::NewGuid().ToString('N')
    $directory=Assert-NativeBinaryPath (Join-Path $root $generation) $root
    $archives=Assert-NativeBinaryPath (Join-Path $directory 'archives') $directory
    if(Test-Path -LiteralPath $directory){throw 'Fresh binary cache generation already exists.'}
    New-Item -ItemType Directory -Path $archives | Out-Null
    return @{mode='POPULATE';key=$Key;root=$root;generation=$generation;archives=$archives;manifestPath=(Join-Path $directory 'manifest.json');manifestSha256=$null;files=@();rebuildReason=$reason}
}
function Complete-NativeBinaryCache([Collections.IDictionary]$State,[Collections.IDictionary]$Inputs){
    if($State.mode -ceq 'REUSE'){
        return Assert-NativeBinaryGeneration $State.root $State.key @{generation=$State.generation;manifestSha256=$State.manifestSha256}
    }
    if($State.mode -cne 'POPULATE'){throw 'Unknown binary cache state.'}
    $files=@(Get-NativeBinaryInventory $State.archives)
    if(-not $files.Count){throw 'No binary packages were produced; dependency cache cannot be sealed.'}
    $manifest=[ordered]@{schemaVersion=1;status='SEALED';key=$State.key;generation=$State.generation;createdUtc=[datetimeoffset]::UtcNow.ToString('o');inputs=$Inputs;files=$files}
    [void](Assert-NativeBinaryPath $State.manifestPath $State.root)
    if(Test-Path -LiteralPath $State.manifestPath){throw 'Binary generation manifest already exists.'}
    [IO.File]::WriteAllText($State.manifestPath,($manifest|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false))
    $selector=@{generation=$State.generation;manifestSha256=(Get-FileHash -LiteralPath $State.manifestPath).Hash}
    # Verify the complete snapshot before publishing the current pointer.
    $sealed=Assert-NativeBinaryGeneration $State.root $State.key $selector
    $path=Join-Path $State.root 'current.json';$pending=Join-Path $State.root ('current-'+[guid]::NewGuid().ToString('N')+'.pending')
    [void](Assert-NativeBinaryPath $path $State.root)
    [IO.File]::WriteAllText($pending,($selector|ConvertTo-Json),[Text.UTF8Encoding]::new($false))
    [IO.File]::Move($pending,$path,$true)
    return $sealed
}
