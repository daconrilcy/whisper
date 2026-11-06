#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('SelfTest','SurfaceCpu','SurfaceGpu','RepeatCpu','RepeatGpu','Prepare','NativeSelfTest','N2SelfTest','Cpu1','Cpu2','Gpu1','Gpu2','CpuReference1','CpuReference2','CpuRebuilt','GpuReference1','GpuReference2','GpuRebuilt')][string]$Stage,
    [ValidateSet('r1')][string]$RepriseId
)
$ErrorActionPreference='Stop'
$PSNativeCommandUseErrorActionPreference=$false
$repo=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\')
function Get-QualificationRouting([string]$Reprise) {
    if($Reprise -and $Reprise -cne 'r1'){throw 'Only the explicitly authorized r1 qualification reprise is supported.'}
    $original='C:\wb\native-qualif'
    return [pscustomobject]@{root=$(if($Reprise){Join-Path $original 'reprise-1'}else{$original});original=$original;buildPrefix=$(if($Reprise){'qualif-r1'}else{'qualif'})}
}
function Get-QualificationBuildId([string]$Variant,[int]$Number,[string]$Reprise) {
    if($Variant -cnotin @('cpu','gpu') -or $Number -notin @(1,2)){throw 'Qualification build ID requires cpu/gpu and build 1/2.'}
    return (Get-QualificationRouting $Reprise).buildPrefix+'-'+$Variant+$Number
}
$routing=Get-QualificationRouting $RepriseId
$root=$routing.root
function Assert-QualificationMemory([double]$Percent,[bool]$Launching) {
    if ([double]::IsNaN($Percent) -or $Percent -lt 0 -or $Percent -gt 100) { throw 'Invalid committed memory measurement.' }
    if ($Percent -ge 90 -or ($Launching -and $Percent -ge 85)) { throw "Committed memory threshold reached: $Percent % (launch=$Launching)." }
}
function Get-QualificationWhea {
    $errors=@()
    $events=@(Get-WinEvent -FilterHashtable @{LogName='System';ProviderName='Microsoft-Windows-WHEA-Logger'} -ErrorAction SilentlyContinue -ErrorVariable errors)
    foreach($errorItem in $errors) { if ($errorItem.FullyQualifiedErrorId -notlike 'NoMatchingEventsFound*') { throw $errorItem } }
    return $events
}
function Get-QualificationMemory {
    $memory=Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory -OperationTimeoutSec 2 -ErrorAction Stop
    if ($memory.CommitLimit -le 0) { throw 'Missing committed-memory limit.' }
    return (100.0*$memory.CommittedBytes/$memory.CommitLimit)
}
function Get-NewQualificationWhea([object[]]$Events,[long]$LastId) { @($Events | Where-Object RecordId -GT $LastId) }
function Assert-QualificationPlainPath([string]$Path) {
    $cursor=[IO.Path]::GetFullPath($Path)
    while($cursor) {
        if ((Test-Path -LiteralPath $cursor) -and ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Qualification path crosses reparse point: $cursor" }
        $next=Split-Path -Parent $cursor; if($next -eq $cursor){break};$cursor=$next
    }
}
function Import-QualificationProcessControls {
    $tokens=$null;$errors=$null
    $ast=[Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'guard-native.ps1'),[ref]$tokens,[ref]$errors)
    if($errors.Count){throw 'Guard process-control source cannot be parsed.'}
    foreach($name in @('New-VoxPopuliProcessFamily','Assert-VoxPopuliCleanupDeadline','Update-VoxPopuliProcessFamily','Stop-VoxPopuliKnownProcesses','Stop-VoxPopuliProcessFamily','Close-VoxPopuliProcessFamily')) {
        $definitions=@($ast.EndBlock.Statements | Where-Object {$_ -is [Management.Automation.Language.FunctionDefinitionAst] -and $_.Name -ceq $name})
        if($definitions.Count -ne 1){throw "Process helper missing: $name"}
        . ([scriptblock]::Create($definitions[0].Extent.Text.Replace("function $name(","function script:${name}(")))
    }
}
function Add-QualificationFailure([object]$Context,[string]$Message) {
    if(-not $Context.failures.Contains($Message)){$Context.failures.Add($Message)}
    $Context.record.status='FAIL';$Context.record.failure=($Context.failures -join '; ')
    [ordered]@{stage=$Context.stage;at=[DateTimeOffset]::UtcNow.ToString('o');failures=$Context.failures.ToArray()}|
        ConvertTo-Json -Depth 6|Set-Content -LiteralPath (Join-Path $Context.root 'STOP.json') -Encoding utf8
}
function Assert-QualificationCanStart([string]$Root) {
    if(Test-Path -LiteralPath (Join-Path $Root 'STOP.json')){throw 'Qualification previously stopped; no further attempt without user decision.'}
}
function Assert-QualificationWhea([object]$Context,[object[]]$Events) {
    $new=@(Get-NewQualificationWhea $Events $Context.baselineId)
    if(-not $new.Count){return}
    foreach($event in $new){
        $event.ToXml()|Set-Content -LiteralPath (Join-Path $Context.evidence "whea-$($event.RecordId).xml") -Encoding utf8
        [ordered]@{recordId=$event.RecordId;id=$event.Id;timeCreated=$event.TimeCreated;message=$event.Message}|
            ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $Context.evidence "whea-$($event.RecordId).json") -Encoding utf8
    }
    throw 'New WHEA event of any type since the persistent lot baseline; qualification stopped.'
}
function Invoke-QualificationHealth([object]$Context,[switch]$Launching,[switch]$Force,[switch]$CollectOnly) {
    if(-not $Force -and -not $Launching -and [DateTimeOffset]::UtcNow -lt $Context.nextSample){return}
    try {
        $events=@(& $Context.readWhea);$Context.record.wheaAfter=$events.Count
        Assert-QualificationWhea $Context $events
        $percent=[double](& $Context.readMemory);$Context.peak=[Math]::Max($Context.peak,$percent)
        Assert-QualificationMemory $percent ([bool]$Launching)
        if(([DateTimeOffset]::UtcNow-$Context.budgetStart).TotalHours -ge 6){throw 'Global qualification budget exhausted.'}
        $level='NOT_STARTED'
        if($Context.monitorStarted){
            if($Context.monitor.HasExited -or $Context.monitor.StartTime.ToUniversalTime() -ne $Context.monitorIdentity){throw 'MemWatch identity or liveness lost.'}
            if((Get-FileHash -LiteralPath 'C:/memwatch/MemWatch.ps1').Hash -cne $Context.record.memwatchSha256){throw 'MemWatch source changed.'}
            $sample=Get-Item -LiteralPath $Context.levelPath
            if(([datetime]::UtcNow-$sample.LastWriteTimeUtc).TotalSeconds -gt 20){throw 'MemWatch sample stale.'}
            $level=(Get-Content -LiteralPath $Context.levelPath -Raw).Trim()
            $emptyDeadline=[DateTimeOffset]::UtcNow.AddMilliseconds(500)
            while(-not $level -and [DateTimeOffset]::UtcNow -lt $emptyDeadline){Start-Sleep -Milliseconds 20;$level=(Get-Content -LiteralPath $Context.levelPath -Raw).Trim()}
            if($level -eq 'INTERROMPRE'){throw 'MemWatch requests interruption.'}
            $allowed=if($Launching){@('OK','ALERTE')}else{@('OK','ALERTE','STOP-LANCEMENTS')}
            if($level -notin $allowed){throw "MemWatch does not permit this operation: $level"}
        }
        [ordered]@{at=[DateTimeOffset]::UtcNow.ToString('o');committedPct=$percent;whea=$events.Count;baselineId=$Context.baselineId;memwatch=$level}|
            ConvertTo-Json -Compress|Add-Content -LiteralPath (Join-Path $Context.evidence 'samples.jsonl') -Encoding utf8
        $Context.nextSample=[DateTimeOffset]::UtcNow.AddSeconds(1)
    } catch {
        Add-QualificationFailure $Context $_.Exception.Message
        if(-not $CollectOnly){throw}
    }
}
function Get-QualificationHash([string]$Path,[scriptblock]$Check) {
    $stream=[IO.File]::OpenRead($Path);$hash=[Security.Cryptography.IncrementalHash]::CreateHash([Security.Cryptography.HashAlgorithmName]::SHA256)
    try {
        $buffer=[byte[]]::new(1MB)
        while(($count=$stream.Read($buffer,0,$buffer.Length)) -gt 0){if($Check){& $Check};$hash.AppendData($buffer,0,$count)}
        if($Check){& $Check}
        return [Convert]::ToHexString($hash.GetHashAndReset())
    } finally {$hash.Dispose();$stream.Dispose()}
}
function Copy-QualificationFile([string]$Source,[string]$Destination,[scriptblock]$Check) {
    $inputStream=[IO.File]::OpenRead($Source);$outputStream=$null
    try {
        $outputStream=[IO.File]::Open($Destination,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
        $buffer=[byte[]]::new(1MB)
        while(($count=$inputStream.Read($buffer,0,$buffer.Length)) -gt 0){if($Check){& $Check};$outputStream.Write($buffer,0,$count)}
        if($Check){& $Check}
    } finally {if($outputStream){$outputStream.Dispose()};$inputStream.Dispose()}
}
function Save-QualificationTree([string]$Source,[string]$Destination,[scriptblock]$Check=$null) {
    if($Check){& $Check}
    Assert-QualificationPlainPath $Source;Assert-QualificationPlainPath $Destination
    if(Test-Path -LiteralPath $Destination){throw "Archive already exists: $Destination"}
    $sourceFull=[IO.Path]::GetFullPath($Source).TrimEnd('\')
    if (-not $sourceFull.StartsWith($repo+'\target\',[StringComparison]::OrdinalIgnoreCase)){throw 'Archive source must be worktree target.'}
    if (-not [IO.Path]::GetFullPath($Destination).StartsWith($root+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Archive destination escaped qualification root.'}
    $files=@(Get-ChildItem -LiteralPath $Source -File -Recurse -Force | Where-Object {
        [IO.Path]::GetRelativePath($Source,$_.FullName) -notmatch '^previous-workspace[\\/]'
    })
    if(@(Get-ChildItem -LiteralPath $Source -Recurse -Force -Attributes ReparsePoint).Count){throw 'Archive source contains reparse point.'}
    New-Item -ItemType Directory -Path $Destination -Force|Out-Null
    $receipt=@(foreach($file in $files){
        $relative=[IO.Path]::GetRelativePath($Source,$file.FullName);$copy=Join-Path $Destination $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $copy) -Force|Out-Null
        $hash=Get-QualificationHash $file.FullName $Check
        Copy-QualificationFile $file.FullName $copy $Check
        if((Get-QualificationHash $copy $Check) -cne $hash){throw "Archive SHA mismatch: $relative"}
        [ordered]@{path=$relative;bytes=$file.Length;sha256=$hash}
    })
    if(@(Get-ChildItem -LiteralPath $Destination -Recurse -File -Force).Count -ne $files.Count){throw 'Archive file count differs.'}
    $receipt|ConvertTo-Json -Depth 5|Set-Content ($Destination+'.receipt.json') -Encoding utf8
    if($Check){& $Check}
}
function Read-QualificationBytes([string]$Path,[scriptblock]$Check=$null) {
    $stream=[IO.File]::OpenRead($Path)
    try {
        if($stream.Length -gt [int]::MaxValue){throw 'PE image is too large to inspect.'}
        $bytes=[byte[]]::new([int]$stream.Length);$offset=0
        while($offset -lt $bytes.Length){if($Check){& $Check};$read=$stream.Read($bytes,$offset,[Math]::Min(1MB,$bytes.Length-$offset));if(-not $read){throw 'PE image shortened during read.'};$offset+=$read}
        if($Check){& $Check};return ,$bytes
    } finally {$stream.Dispose()}
}
function Read-QualificationPe([string]$Path,[scriptblock]$Check=$null) {
    Assert-QualificationPlainPath $Path
    $bytes=Read-QualificationBytes $Path $Check
    function Bounds([long]$Offset,[long]$Length) {if($Offset -lt 0 -or $Length -lt 0 -or $Offset+$Length -gt $bytes.LongLength){throw 'PE field out of bounds.'}}
    function U16([long]$Offset){Bounds $Offset 2;[BitConverter]::ToUInt16($bytes,$Offset)}
    function U32([long]$Offset){Bounds $Offset 4;[BitConverter]::ToUInt32($bytes,$Offset)}
    function U64([long]$Offset){Bounds $Offset 8;[BitConverter]::ToUInt64($bytes,$Offset)}
    if((U16 0) -ne 0x5a4d){throw 'PE missing MZ.'}
    $pe=[long](U32 60);$opt=$pe+24
    if((U32 $pe) -ne 0x4550 -or (U16 ($pe+4)) -ne 0x8664 -or (U16 $opt) -ne 0x20b){throw 'Expected x64 PE32+.'}
    $sectionCount=U16 ($pe+6);$sectionTable=$opt+(U16 ($pe+20));$imageBase=U64 ($opt+24)
    if($sectionCount -gt 96){throw 'Unbounded PE sections.'}
    $sections=@(for($i=0;$i -lt $sectionCount;$i++){
        $offset=$sectionTable+40*$i;Bounds $offset 40
        [pscustomobject]@{name=[Text.Encoding]::ASCII.GetString($bytes,$offset,8).TrimEnd([char]0);rva=[long](U32 ($offset+12));size=[long](U32 ($offset+16));raw=[long](U32 ($offset+20))}
    })
    function Rva([long]$Value) {
        if($Value -lt (U32 ($opt+60))){Bounds $Value 1;return $Value}
        foreach($section in $sections){if($Value -ge $section.rva -and $Value -lt $section.rva+$section.size){$mapped=$section.raw+$Value-$section.rva;Bounds $mapped 1;return $mapped}}
        throw "PE RVA unmapped: $Value"
    }
    function Str([long]$Offset) {
        Bounds $Offset 1;$end=$Offset
        while($end -lt $bytes.LongLength -and $end-$Offset -lt 65536 -and $bytes[$end] -ne 0){$end++}
        if($end -eq $bytes.LongLength -or $end-$Offset -ge 65536){throw 'Unterminated PE string.'}
        [Text.Encoding]::ASCII.GetString($bytes,$Offset,$end-$Offset)
    }
    function Thunks([long]$Value,[string]$Dll,[string]$Kind) {
        $offset=Rva $Value
        for($i=0;$i -lt 1000000;$i++){
            if($i%1024 -eq 0 -and $Check){& $Check}
            $thunk=U64 ($offset+8*$i);if($thunk -eq 0){return}
            if(($thunk -band [uint64]::Parse('8000000000000000',[Globalization.NumberStyles]::HexNumber)) -ne 0){$symbol='#'+($thunk -band 65535)}
            else{$symbol=Str ((Rva ([long]$thunk))+2)}
            "$Kind|$($Dll.ToLowerInvariant())|$symbol"
        };throw 'PE thunk count unbounded.'
    }
    $imports=@(
        $importRva=U32 ($opt+112+8)
        if($importRva){
            $offset=Rva $importRva;$directorySize=U32 ($opt+112+8+4)
            if($directorySize -lt 20 -or $directorySize -gt 200000){throw 'Invalid PE import directory size.'}
            Bounds $offset $directorySize;$terminated=$false
            for($i=0;20*($i+1) -le $directorySize;$i++){
                $entry=$offset+20*$i;$lookup=U32 $entry;$name=U32 ($entry+12);$address=U32 ($entry+16)
                if(-not $lookup -and -not $name -and -not $address){$terminated=$true;break}
                if(-not $name){throw 'PE import DLL name missing.'}
                $dll=Str (Rva $name);$thunk=if($lookup){$lookup}else{$address}
                Thunks $thunk $dll 'direct'
                if($i -eq 9999){throw 'PE import descriptors unbounded.'}
            }
            if(-not $terminated){throw 'PE import descriptors lack bounded terminator.'}
        }
        $delayRva=U32 ($opt+112+13*8)
        if($delayRva){
            $offset=Rva $delayRva;$directorySize=U32 ($opt+112+13*8+4)
            if($directorySize -lt 32 -or $directorySize -gt 320000){throw 'Invalid PE delay directory size.'}
            Bounds $offset $directorySize;$terminated=$false
            for($i=0;32*($i+1) -le $directorySize;$i++){
                $entry=$offset+32*$i;$attributes=U32 $entry;$name=[long](U32 ($entry+4));$lookup=[long](U32 ($entry+16))
                if(-not $attributes -and -not $name -and -not $lookup){$terminated=$true;break}
                if(($attributes -band 1) -eq 0){$name-=$imageBase;$lookup-=$imageBase}
                Thunks $lookup (Str (Rva $name)) 'delay'
                if($i -eq 9999){throw 'PE delay descriptors unbounded.'}
            }
            if(-not $terminated){throw 'PE delay descriptors lack bounded terminator.'}
        }
    )
    $exports=@(
        $exportRva=U32 ($opt+112);$exportSize=U32 ($opt+116)
        if($exportRva){
            $offset=Rva $exportRva;$base=U32 ($offset+16);$count=U32 ($offset+20);$names=U32 ($offset+24)
            if($count -gt 1000000 -or $names -gt $count){throw 'PE exports unbounded.'}
            $functions=Rva (U32 ($offset+28));$nameMap=@{}
            if($names){$nameTable=Rva (U32 ($offset+32));$ordinalTable=Rva (U32 ($offset+36))
                for($i=0;$i -lt $names;$i++){$ordinal=U16 ($ordinalTable+2*$i);if($ordinal -ge $count){throw 'PE export ordinal invalid.'};if(-not $nameMap.ContainsKey([int]$ordinal)){$nameMap[[int]$ordinal]=@()};$nameMap[[int]$ordinal]+=(Str (Rva (U32 ($nameTable+4*$i))))}
            }
            for($i=0;$i -lt $count;$i++){
                if($i%1024 -eq 0 -and $Check){& $Check}
                $target=U32 ($functions+4*$i);if(-not $target){continue}
                $forward=if($target -ge $exportRva -and $target -lt $exportRva+$exportSize){Str (Rva $target)}else{''}
                $entryNames=if($nameMap.ContainsKey($i)){$nameMap[$i]}else{@('[NONAME]')}
                foreach($name in $entryNames){"$($base+$i)|$name|$forward"}
            }
        }
    )
    return [ordered]@{name=[IO.Path]::GetFileName($Path);machine=(U16 ($pe+4));subsystem=(U16 ($opt+68));exports=@($exports|Sort-Object -CaseSensitive);imports=@($imports|Sort-Object -CaseSensitive);dynamicCrt=[bool]@($imports|Where-Object {$_ -match '\|(vcruntime|msvcp|ucrtbase|api-ms-win-crt)'}).Count}
}
function Get-QualificationPeRegions([byte[]]$Bytes) {
    if($Bytes.Length -lt 64 -or [BitConverter]::ToUInt16($Bytes,0) -ne 0x5a4d){throw 'Invalid PE layout for byte differences.'}
    $pe=[long][BitConverter]::ToUInt32($Bytes,60);$opt=$pe+24
    if($pe+24 -gt $Bytes.LongLength -or [BitConverter]::ToUInt32($Bytes,$pe) -ne 0x4550){throw 'Invalid PE header for byte differences.'}
    $count=[BitConverter]::ToUInt16($Bytes,$pe+6);$optionalSize=[BitConverter]::ToUInt16($Bytes,$pe+20)
    $table=$opt+$optionalSize
    if($count -gt 96 -or $optionalSize -lt 64 -or $table+40*$count -gt $Bytes.LongLength){throw 'Unbounded PE layout for byte differences.'}
    $headerSize=[long][BitConverter]::ToUInt32($Bytes,$opt+60)
    if($headerSize -lt $table+40*$count -or $headerSize -gt $Bytes.LongLength){throw 'Invalid PE header size for byte differences.'}
    [pscustomobject]@{name='[headers]';start=0L;end=$headerSize}
    for($i=0;$i -lt $count;$i++){
        $entry=$table+40*$i;$size=[long][BitConverter]::ToUInt32($Bytes,$entry+16);$raw=[long][BitConverter]::ToUInt32($Bytes,$entry+20)
        if($raw+$size -gt $Bytes.LongLength){throw 'PE raw section exceeds file in byte comparison.'}
        if($size){[pscustomobject]@{name=("section[$i]:"+[Text.Encoding]::ASCII.GetString($Bytes,$entry,8).TrimEnd([char]0));start=$raw;end=($raw+$size)}}
    }
}
function Get-QualificationByteRegion([object[]]$Regions,[long]$Offset,[long]$Length) {
    if($Offset -ge $Length){return '[missing]'}
    foreach($region in $Regions){if($Offset -ge $region.start -and $Offset -lt $region.end){return $region.name}}
    return '[outside-sections]'
}
function Compare-QualificationBytes([byte[]]$First,[byte[]]$Other,[scriptblock]$Check=$null) {
    $firstRegions=@(Get-QualificationPeRegions $First);$otherRegions=@(Get-QualificationPeRegions $Other)
    $regions=@{};$total=0L;$maximum=[Math]::Max($First.LongLength,$Other.LongLength)
    for($offset=0L;$offset -lt $maximum;$offset++){
        if($offset%65536 -eq 0 -and $Check){& $Check}
        if($offset -lt $First.LongLength -and $offset -lt $Other.LongLength -and $First[$offset] -eq $Other[$offset]){continue}
        $a=Get-QualificationByteRegion $firstRegions $offset $First.LongLength
        $b=Get-QualificationByteRegion $otherRegions $offset $Other.LongLength
        $key=$a+'|'+$b
        if(-not $regions.ContainsKey($key)){$regions[$key]=[ordered]@{build1Region=$a;otherRegion=$b;differentBytes=0L;firstOffset=$offset;lastOffset=$offset}}
        $regions[$key].differentBytes++;$regions[$key].lastOffset=$offset;$total++
    }
    if($Check){& $Check}
    return [ordered]@{differentBytes=$total;regions=@($regions.GetEnumerator()|Sort-Object Name|ForEach-Object Value)}
}
function Invoke-QualificationDumpbin([string]$Executable,[string]$Path,[string]$Mode,[string]$Destination,[scriptblock]$Check) {
    $info=[Diagnostics.ProcessStartInfo]::new($Executable);$info.UseShellExecute=$false;$info.CreateNoWindow=$true
    $info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
    foreach($arg in @('/nologo',"/$Mode",$Path)){$info.ArgumentList.Add($arg)}
    $process=[Diagnostics.Process]::new();$process.StartInfo=$info;$started=$false
    try {
        if($Check){& $Check}
        if(-not $process.Start()){throw 'dumpbin did not start.'};$started=$true
        $stdout=$process.StandardOutput.ReadToEndAsync();$stderr=$process.StandardError.ReadToEndAsync()
        $streamDeadline=$null
        while(-not ($process.HasExited -and $stdout.IsCompleted -and $stderr.IsCompleted)){
            if($Check){& $Check}
            if($process.HasExited){
                if($null -eq $streamDeadline){$streamDeadline=[DateTimeOffset]::UtcNow.AddSeconds(10)}
                if([DateTimeOffset]::UtcNow -ge $streamDeadline){throw 'dumpbin output streams did not close.'}
            }
            Start-Sleep -Milliseconds 50
        }
        $output=$stdout.GetAwaiter().GetResult()+$stderr.GetAwaiter().GetResult()
        $output|Set-Content -LiteralPath $Destination -Encoding utf8
        if($process.ExitCode -ne 0){throw "dumpbin failed: $Path $Mode"}
        if($Check){& $Check}
    } finally {if($started -and -not $process.HasExited){$process.Kill($true);if(-not $process.WaitForExit(10000)){throw 'dumpbin did not stop.'}};$process.Dispose()}
}
function Compare-QualificationDlls([string]$Variant,[bool]$Repeat,[scriptblock]$Check=$null) {
    if($Check){& $Check}
    $firstId=Get-QualificationBuildId $Variant 1 $RepriseId;$secondId=Get-QualificationBuildId $Variant 2 $RepriseId
    $first=Join-Path $root "$Variant/$firstId/dll"
    $other=if($Repeat){Join-Path $root "$Variant/$secondId/dll"}else{Join-Path $env:LOCALAPPDATA "Whisper/native/$Variant"}
    Assert-QualificationPlainPath $first;Assert-QualificationPlainPath $other
    $names=@(Get-ChildItem -LiteralPath $first -File|ForEach-Object Name|Sort-Object)
    $otherNames=@(Get-ChildItem -LiteralPath $other -File|ForEach-Object Name|Sort-Object)
    $expected=if($Variant -eq 'cpu'){5}else{9}
    if($names.Count -ne $expected -or ($names -join '|') -cne ($otherNames -join '|')){throw 'DLL filename sets differ.'}
    $report=@(foreach($name in $names){
        $a=Join-Path $first $name;$b=Join-Path $other $name
        $tools=Get-Content (Join-Path $root "preflight/tools-$Variant.json") -Raw|ConvertFrom-Json
        foreach($entry in @(@{label='build1';path=$a},@{label='other';path=$b})){
            foreach($mode in @('headers','exports','imports')){
                Invoke-QualificationDumpbin $tools.tools.dumpbin $entry.path $mode (Join-Path $root "$Variant/$($entry.label)-$name-$mode.txt") $Check
            }
        }
        $surfaceA=Read-QualificationPe $a $Check;$surfaceB=Read-QualificationPe $b $Check
        $sameSurface=($surfaceA|ConvertTo-Json -Depth 8 -Compress) -ceq ($surfaceB|ConvertTo-Json -Depth 8 -Compress)
        $shaA=Get-QualificationHash $a $Check;$shaB=Get-QualificationHash $b $Check
        $different=$null
        if($Repeat -and $shaA -cne $shaB){
            $x=Read-QualificationBytes $a $Check;$y=Read-QualificationBytes $b $Check
            $different=Compare-QualificationBytes $x $y $Check
        }
        [ordered]@{name=$name;sha256Build1=$shaA;sha256Other=$shaB;surfaceEqual=$sameSurface;bytesEqual=($shaA -ceq $shaB);byteDifferences=$different;build1Surface=$surfaceA;otherSurface=$surfaceB}
    })
    $reportPath=Join-Path $root "$Variant/$(if($Repeat){'repeat'}else{'surface'}).json"
    $report|ConvertTo-Json -Depth 12|Set-Content $reportPath -Encoding utf8
    if($Check){& $Check}
    if(@($report|Where-Object {if($Repeat){-not $_.bytesEqual}else{-not $_.surfaceEqual}}).Count){
        [ordered]@{at=[DateTimeOffset]::UtcNow.ToString('o');failure='DLL comparison differs; characterize before further execution.';report=$reportPath}|ConvertTo-Json|Set-Content (Join-Path $root 'STOP.json')
        throw "DLL comparison differs; see $reportPath"
    }
    "PASS DLL comparison $Variant repeat=$Repeat report=$reportPath"
}
function New-QualificationContext([string]$Root,[string]$Evidence,[string]$Stage,[long]$BaselineId,[DateTimeOffset]$BudgetStart,[object]$Record) {
    [pscustomobject]@{root=$Root;evidence=$Evidence;stage=$Stage;baselineId=$BaselineId;budgetStart=$BudgetStart;record=$Record
        failures=[Collections.Generic.List[string]]::new();peak=0.0;nextSample=[DateTimeOffset]::MinValue
        readWhea={Get-QualificationWhea};readMemory={Get-QualificationMemory};monitor=$null;monitorStarted=$false;monitorWasStarted=$false;monitorIdentity=$null;levelPath=$null}
}
function Stop-QualificationChild([object]$Context,[Diagnostics.Process]$Child,[bool]$Started,[object]$Family,[scriptblock]$StopFamily={param($Owned) Stop-VoxPopuliProcessFamily $Owned}) {
    if(-not $Started){return}
    try {if($Family){& $StopFamily $Family}}
    catch {
        Add-QualificationFailure $Context ('Process-family cleanup failed: '+$_.Exception.Message)
        # Reuse the owned, identity-bound handles even if descendant observation failed.
        foreach($entry in @($Family.entries.Values)){
            try {if(-not $entry.process.HasExited){$entry.process.Kill($true);if(-not $entry.process.WaitForExit(10000)){throw 'Owned descendant did not exit.'}}}
            catch {Add-QualificationFailure $Context ('Owned descendant closure failed: '+$_.Exception.Message)}
        }
    }
    finally {
        try {if(-not $Child.HasExited){$Child.Kill($true);if(-not $Child.WaitForExit(10000)){throw 'Owned guard process did not exit.'}}}
        catch {Add-QualificationFailure $Context ('Owned guard closure failed: '+$_.Exception.Message)}
        try {if($Family){Close-VoxPopuliProcessFamily $Family}}
        catch {Add-QualificationFailure $Context ('Process-family handle closure failed: '+$_.Exception.Message)}
    }
}
function Stop-QualificationMonitor([object]$Context,[string]$MonitorRoot) {
    if(-not $Context.monitorWasStarted){return}
    # Proof copies are complete. Continue direct WHEA/memory checks while MemWatch closes.
    $Context.monitorStarted=$false
    try {
        if($Context.monitor.HasExited){throw 'MemWatch exited before requested shutdown.'}
        if(-not $Context.monitor.HasExited){
            'stop'|Set-Content -LiteralPath (Join-Path $MonitorRoot 'stop-BUILD-NATIF-QUALIF') -Encoding utf8
            $deadline=[DateTimeOffset]::UtcNow.AddSeconds(10)
            while(-not $Context.monitor.WaitForExit(100)){
                Invoke-QualificationHealth $Context -CollectOnly
                if([DateTimeOffset]::UtcNow -ge $deadline){throw 'MemWatch graceful stop exceeded ten seconds.'}
            }
        }
    } catch {Add-QualificationFailure $Context ('MemWatch closure failed: '+$_.Exception.Message)}
    finally {
        if(-not $Context.monitor.HasExited){$Context.monitor.Kill($true);if(-not $Context.monitor.WaitForExit(10000)){throw 'Owned MemWatch process did not stop.'}}
    }
}
function Complete-QualificationStep([object]$Context,[scriptblock[]]$Actions) {
    foreach($action in $Actions){
        try {& $action}
        catch {Add-QualificationFailure $Context ('Stage finalization failed: '+$_.Exception.Message)}
    }
    $Context.record.finishedUtc=[DateTimeOffset]::UtcNow.ToString('o')
    $Context.record.peakMemoryPct=$Context.peak
    if(-not $Context.failures.Count){$Context.record.status='PASS'}
    $Context.record|ConvertTo-Json -Depth 8|Set-Content -LiteralPath (Join-Path $Context.evidence 'step.json') -Encoding utf8
}
if($Stage -eq 'SelfTest') {
    Assert-QualificationMemory 84.9 $true;Assert-QualificationMemory 89.9 $false
    $rejects=0
    foreach($case in @(@{pct=85;launch=$true},@{pct=90;launch=$false},@{pct=-1;launch=$false},@{pct=[double]::NaN;launch=$false})) {
        try {Assert-QualificationMemory $case.pct $case.launch}catch{$rejects++;continue};throw 'Memory negative control failed.'
    }
    $events=@([pscustomobject]@{RecordId=11;Id=19},[pscustomobject]@{RecordId=12;Id=18})
    if(@(Get-NewQualificationWhea $events 10).Count -ne 2 -or @(Get-NewQualificationWhea $events 12).Count -ne 0){throw 'WHEA all-types negative control failed.'}
    $fixture=Read-QualificationPe (Join-Path $PSScriptRoot 'fixtures/pe/no-cuda.bin')
    if($fixture.machine -ne 0x8664 -or @($fixture.imports|Where-Object {$_ -match 'cuda'}).Count){throw 'PE fixture control failed.'}
    $rejected=$false
    try{$null=Read-QualificationPe (Join-Path $PSScriptRoot 'fixtures/pe/truncated-direct.bin')}catch{$rejected=$true}
    if(-not $rejected){throw 'Truncated PE negative control failed.'}
    $testRoot=Join-Path $repo ('target/native-qualif-selftests/'+[guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $testRoot -Force|Out-Null
    function New-TestContext([string]$Name){
        $caseRoot=Join-Path $testRoot $Name;$caseEvidence=Join-Path $caseRoot 'step'
        New-Item -ItemType Directory -Path $caseEvidence -Force|Out-Null
        $state=New-QualificationContext $caseRoot $caseEvidence $Name 10 ([DateTimeOffset]::UtcNow) ([ordered]@{status='STARTED'})
        $state.readWhea={@()};$state.readMemory={20.0};return $state
    }
    $event=[pscustomobject]@{RecordId=11L;Id=18;TimeCreated=[datetime]::UtcNow;Message='synthetic WHEA witness'}
    $event|Add-Member -MemberType ScriptMethod -Name ToXml -Value {'<Event><System><EventID>18</EventID><EventRecordID>11</EventRecordID></System><EventData>synthetic-witness</EventData></Event>'}
    $context=New-TestContext 'whea-during-copy';$root=$context.root
    $source=Join-Path $testRoot 'source';New-Item -ItemType Directory -Path $source -Force|Out-Null
    [IO.File]::WriteAllBytes((Join-Path $source 'data.bin'),[byte[]]::new(2MB))
    $destination=Join-Path $root 'preserved';$copiedFile=Join-Path $destination 'data.bin'
    $context.readWhea={if((Test-Path -LiteralPath $copiedFile) -and (Get-Item -LiteralPath $copiedFile).Length -gt 0){$event}}
    Save-QualificationTree $source $destination {Invoke-QualificationHealth $context -Force -CollectOnly}
    Complete-QualificationStep $context @({Invoke-QualificationHealth $context -Force -CollectOnly})
    if($context.record.status -ne 'FAIL' -or -not (Test-Path (Join-Path $root 'STOP.json')) -or -not (Test-Path (Join-Path $context.evidence 'whea-11.xml')) -or -not (Test-Path ($destination+'.receipt.json'))){throw 'WHEA copy witness did not fail and preserve complete evidence.'}
    $rejected=$false;try{Assert-QualificationCanStart $root}catch{$rejected=$true};if(-not $rejected){throw 'Next stage accepted after WHEA during copy.'}
    $context=New-TestContext 'whea-between-stages';$root=$context.root
    $context.readWhea={$event};$rejected=$false
    try{Invoke-QualificationHealth $context -Launching -Force}catch{$rejected=$true}
    Complete-QualificationStep $context @()
    if(-not $rejected -or $context.record.status -ne 'FAIL' -or -not (Test-Path (Join-Path $context.evidence 'whea-11.xml'))){throw 'Persistent-baseline WHEA stage-entry witness failed.'}
    $rejected=$false;try{Assert-QualificationCanStart $root}catch{$rejected=$true};if(-not $rejected){throw 'Next stage accepted after inter-stage WHEA.'}
    Import-QualificationProcessControls
    $context=New-TestContext 'failed-start';$root=$context.root
    $failedProcess=[Diagnostics.Process]::new();$failedProcess.StartInfo=[Diagnostics.ProcessStartInfo]::new((Join-Path $testRoot 'does-not-exist.exe'))
    $failedProcess.StartInfo.UseShellExecute=$false;$ownedStarted=$false
    try{if($failedProcess.Start()){$ownedStarted=$true;throw 'Invalid start witness unexpectedly launched.'}}
    catch {Add-QualificationFailure $context ('Expected start failure: '+$_.Exception.Message)}
    Complete-QualificationStep $context @(
        {Stop-QualificationChild $context $failedProcess $ownedStarted $null},
        {'startup proof'|Set-Content -LiteralPath (Join-Path $context.evidence 'proof.txt')},
        {$failedProcess.Dispose()}
    )
    if($ownedStarted -or $context.record.status -ne 'FAIL' -or -not (Test-Path (Join-Path $root 'STOP.json')) -or -not (Test-Path (Join-Path $context.evidence 'proof.txt'))){throw 'Failed-start evidence or no-start witness failed.'}
    $context=New-TestContext 'failed-cleanup';$root=$context.root
    $ownedProcess=[Diagnostics.Process]::new();$ownedProcess.StartInfo=[Diagnostics.ProcessStartInfo]::new((Join-Path $PSHOME 'pwsh.exe'))
    $ownedProcess.StartInfo.UseShellExecute=$false;$ownedProcess.StartInfo.CreateNoWindow=$true
    foreach($arg in @('-NoProfile','-NonInteractive','-Command','Start-Sleep -Seconds 60')){$ownedProcess.StartInfo.ArgumentList.Add($arg)}
    $ownedStarted=$false
    try {
        if(-not $ownedProcess.Start()){throw 'Cleanup witness could not start.'};$ownedStarted=$true
        $ownedFamily=New-VoxPopuliProcessFamily $ownedProcess
        Complete-QualificationStep $context @(
            {Stop-QualificationChild $context $ownedProcess $ownedStarted $ownedFamily {param($Owned) throw 'Injected cleanup failure.'}},
            {'cleanup proof'|Set-Content -LiteralPath (Join-Path $context.evidence 'proof.txt')}
        )
        if(-not $ownedProcess.HasExited -or $context.record.status -ne 'FAIL' -or -not (Test-Path (Join-Path $root 'STOP.json')) -or -not (Test-Path (Join-Path $context.evidence 'proof.txt'))){throw 'Failed-cleanup did not stop owned process and preserve failure evidence.'}
        $rejected=$false;try{Assert-QualificationCanStart $root}catch{$rejected=$true};if(-not $rejected){throw 'Next stage accepted after cleanup failure.'}
    } finally {if($ownedStarted -and -not $ownedProcess.HasExited){$ownedProcess.Kill($true);[void]$ownedProcess.WaitForExit(10000)};$ownedProcess.Dispose()}
    $first=[IO.File]::ReadAllBytes((Join-Path $PSScriptRoot 'fixtures/pe/no-cuda.bin'))
    # The import-only fixture deliberately omits SizeOfHeaders; complete that field
    # in this in-memory layout witness without modifying the versioned fixture.
    $optionalOffset=[BitConverter]::ToUInt32($first,60)+24
    [BitConverter]::GetBytes([uint32]512).CopyTo($first,$optionalOffset+60)
    $other=[byte[]]$first.Clone()
    $regions=@(Get-QualificationPeRegions $first);$section=@($regions|Where-Object name -like 'section*')[0]
    if(-not $section){throw 'PE witness lacks section.'}
    $headerOffset=8;$sectionOffset=[int]$section.start
    $other[$headerOffset]=$other[$headerOffset] -bxor 1;$other[$sectionOffset]=$other[$sectionOffset] -bxor 1
    $difference=Compare-QualificationBytes $first $other
    if($difference.differentBytes -ne 2 -or @($difference.regions|Where-Object {$_.build1Region -eq '[headers]' -and $_.differentBytes -eq 1 -and $_.firstOffset -eq $headerOffset}).Count -ne 1 -or @($difference.regions|Where-Object {$_.build1Region -eq $section.name -and $_.differentBytes -eq 1 -and $_.firstOffset -eq $sectionOffset}).Count -ne 1){throw 'PE section/header difference witness failed.'}
    $extended=[byte[]]::new($first.Length+1);[Array]::Copy($first,$extended,$first.Length);$extended[-1]=1
    $overlay=Compare-QualificationBytes $first $extended
    if($overlay.differentBytes -ne 1 -or $overlay.regions[0].build1Region -ne '[missing]' -or $overlay.regions[0].otherRegion -ne '[outside-sections]'){throw 'PE outside-section difference witness failed.'}
    "PASS qualification thresholds, WHEA copy/inter-stage STOP, failed-start/cleanup closure and PE section/header/overlay differences; no compilation. selftestArtifacts=$testRoot";return
}
if($repo -ine 'C:\wt\build-natif-qualif' -or [IO.Path]::GetFullPath((Get-Location).Path).TrimEnd('\') -ine $repo){throw 'Qualification requires its exact dedicated worktree and cwd.'}
Assert-QualificationPlainPath $repo;Assert-QualificationPlainPath $root
Assert-QualificationCanStart $root
if($RepriseId){
    foreach($name in @('initial.json','tools-cpu.json','tools-gpu.json')){
        $original=Join-Path $routing.original "preflight/$name";$copy=Join-Path $root "preflight/$name"
        Assert-QualificationPlainPath $original;Assert-QualificationPlainPath $copy
        if((Get-FileHash -LiteralPath $original).Hash -cne (Get-FileHash -LiteralPath $copy).Hash){throw "Reprise preflight must preserve the original baseline and tool versions: $name"}
    }
}
$preflight=Get-Content -LiteralPath (Join-Path $root 'preflight/initial.json') -Raw|ConvertFrom-Json
if($null -eq $preflight.lastRecordId -or [long]$preflight.lastRecordId -lt 0){throw 'Persistent lot WHEA baseline missing or invalid.'}
$budgetStart=[DateTimeOffset]$preflight.at
$evidence=Join-Path $root ('steps/'+$Stage)
if(Test-Path -LiteralPath $evidence){throw 'Qualification stage already attempted; no automatic retry.'}
New-Item -ItemType Directory -Path $evidence -Force|Out-Null
$arguments=@('-NoLogo','-NoProfile','-NonInteractive','-File')
$buildVariant=$null;$buildId=$null;$comparison=$null
switch -Regex ($Stage) {
    '^(Surface|Repeat)(Cpu|Gpu)$' {$comparison=@{variant=$Matches[2].ToLowerInvariant();repeat=($Matches[1] -eq 'Repeat')}}
    '^Prepare$' {$arguments+=@((Join-Path $PSScriptRoot 'build-native.ps1'),'-Variant','cpu','-BuildId',($routing.buildPrefix+'-prepare'),'-PrepareOnly')}
    '^NativeSelfTest$' {$arguments+=@((Join-Path $PSScriptRoot 'guard-native.ps1'),'-Mode','SelfTest','-Profile','NativeDllCpu','-LotRoot','target/native-build')}
    '^N2SelfTest$' {$arguments+=@((Join-Path $PSScriptRoot 'guard-native.ps1'),'-Mode','SelfTest','-LotRoot','target/n2-native')}
    '^(Cpu|Gpu)(1|2)$' {
        $buildVariant=$Matches[1].ToLowerInvariant();$buildId=Get-QualificationBuildId $buildVariant ([int]$Matches[2]) $RepriseId
        $arguments+=@((Join-Path $PSScriptRoot 'build-native.ps1'),'-Variant',$buildVariant,'-BuildId',$buildId)
    }
    '^(Cpu|Gpu)(Reference[12]|Rebuilt)$' {
        $selectedId='NativeClosure'+$Matches[1]+($Matches[2] -replace '[12]$','')
        $arguments+=@((Join-Path $PSScriptRoot 'guard-native.ps1'),'-Mode','Run','-Profile',$selectedId,'-ValuesJson',(Join-Path $repo "target/n2-native/requests/$selectedId.json"),'-LotRoot','target/n2-native')
    }
    default {throw 'Unknown qualification stage.'}
}
$started=[DateTimeOffset]::UtcNow
$record=[ordered]@{stage=$Stage;startedUtc=$started.ToString('o');wheaBaselineCount=$preflight.wheaCount;baselineRecordId=[long]$preflight.lastRecordId
    status='STARTED';command=$arguments;memwatchSha256=(Get-FileHash -LiteralPath C:/memwatch/MemWatch.ps1).Hash}
$context=New-QualificationContext $root $evidence $Stage ([long]$preflight.lastRecordId) $budgetStart $record
$monitorRoot=Join-Path $evidence 'memwatch';$child=$null;$childStarted=$false;$family=$null;$writer=$null
$check={Invoke-QualificationHealth $context}
$preserveCheck={Invoke-QualificationHealth $context -CollectOnly}
try {
    if((Get-PSDrive C).Free -lt 60GB){throw 'Qualification requires at least 60 GiB free.'}
    Invoke-QualificationHealth $context -Launching -Force
    $record.wheaBefore=$record.wheaAfter;$record.memoryBeforePct=$context.peak
    Import-QualificationProcessControls
    New-Item -ItemType Directory -Path $monitorRoot -Force|Out-Null
    $monitorInfo=[Diagnostics.ProcessStartInfo]::new((Join-Path $PSHOME 'pwsh.exe'))
    $monitorInfo.UseShellExecute=$false;$monitorInfo.CreateNoWindow=$true;$monitorInfo.WorkingDirectory=$repo
    foreach($a in @('-NoProfile','-NonInteractive','-File','C:/memwatch/MemWatch.ps1','-Mode','Intensive','-Tag','BUILD-NATIF-QUALIF','-IntervalSeconds','5','-MaxMinutes','360','-OutDir',$monitorRoot)){$monitorInfo.ArgumentList.Add($a)}
    $context.monitor=[Diagnostics.Process]::new();$context.monitor.StartInfo=$monitorInfo
    if(-not $context.monitor.Start()){throw 'MemWatch did not start.'}
    $context.monitorWasStarted=$true;$context.monitorIdentity=$context.monitor.StartTime.ToUniversalTime()
    $context.levelPath=Join-Path $monitorRoot 'niveau-BUILD-NATIF-QUALIF.txt'
    $readyDeadline=[DateTimeOffset]::UtcNow.AddSeconds(20)
    while(-not (Test-Path -LiteralPath $context.levelPath)){
        Invoke-QualificationHealth $context
        if($context.monitor.HasExited -or [DateTimeOffset]::UtcNow -ge $readyDeadline){throw 'MemWatch not ready.'}
        Start-Sleep -Milliseconds 100
    }
    $context.monitorStarted=$true
    Invoke-QualificationHealth $context -Launching -Force
    if($comparison){Compare-QualificationDlls $comparison.variant $comparison.repeat $check}
    else {
        $childInfo=[Diagnostics.ProcessStartInfo]::new((Join-Path $PSHOME 'pwsh.exe'))
        $childInfo.UseShellExecute=$false;$childInfo.CreateNoWindow=$true;$childInfo.WorkingDirectory=$repo
        $childInfo.RedirectStandardOutput=$true;$childInfo.RedirectStandardError=$true
        foreach($a in $arguments){$childInfo.ArgumentList.Add($a)}
        $child=[Diagnostics.Process]::new();$child.StartInfo=$childInfo
        if(-not $child.Start()){throw 'Guard command did not start.'};$childStarted=$true
        $family=New-VoxPopuliProcessFamily $child
        $writer=[IO.StreamWriter]::new((Join-Path $evidence 'command.log'),$false,[Text.UTF8Encoding]::new($false));$writer.AutoFlush=$true
        $streams=@(@{reader=$child.StandardOutput;task=$child.StandardOutput.ReadLineAsync();done=$false},@{reader=$child.StandardError;task=$child.StandardError.ReadLineAsync();done=$false})
        $heartbeat=[DateTimeOffset]::UtcNow;$streamDeadline=$null
        while(-not ($child.HasExited -and $streams[0].done -and $streams[1].done)){
            Update-VoxPopuliProcessFamily $family
            foreach($stream in $streams){
                for($batch=0;$batch -lt 64 -and -not $stream.done -and $stream.task.IsCompleted;$batch++){
                    $line=$stream.task.GetAwaiter().GetResult()
                    if($null -eq $line){$stream.done=$true;break}
                    $writer.WriteLine($line)
                    if($line -match '\bC1001\b|internal compiler error|erreur interne du compilateur'){throw "Compiler ICE: $line"}
                    $stream.task=$stream.reader.ReadLineAsync()
                }
            }
            Invoke-QualificationHealth $context
            if($buildId){
                $buildLogs=Join-Path $repo "target/native-build/$buildVariant/$buildId"
                foreach($log in @(Get-ChildItem -LiteralPath $buildLogs -File -Filter '*.log' -ErrorAction SilentlyContinue)){
                    if(@(Get-Content -LiteralPath $log.FullName -Tail 80|Select-String '\bC1001\b|internal compiler error|erreur interne du compilateur').Count){throw "Compiler ICE in $($log.FullName)"}
                }
            }
            if(([DateTimeOffset]::UtcNow-$heartbeat).TotalSeconds -ge 30){Write-Host "QUALIF $Stage running, memory=$([Math]::Round($context.peak,1))%, WHEA=$($record.wheaAfter)";$heartbeat=[DateTimeOffset]::UtcNow}
            if($child.HasExited){
                if($null -eq $streamDeadline){$streamDeadline=[DateTimeOffset]::UtcNow.AddSeconds(10)}
                if([DateTimeOffset]::UtcNow -ge $streamDeadline){throw 'Guard streams did not close.'}
            }
            Start-Sleep -Milliseconds 50
        }
        $child.WaitForExit();$record.exitCode=$child.ExitCode
        if($child.ExitCode -ne 0){throw "Guard command failed with exit $($child.ExitCode); no automatic retry."}
    }
    Invoke-QualificationHealth $context -Force
} catch {Add-QualificationFailure $context $_.Exception.Message}
finally {
    Complete-QualificationStep $context @(
        {Stop-QualificationChild $context $child $childStarted $family},
        {if($writer){$writer.Dispose()}},
        {
            if($buildId -and (Test-Path -LiteralPath (Join-Path $repo "target/native-build/$buildVariant/$buildId"))){
                Save-QualificationTree (Join-Path $repo "target/native-build/$buildVariant/$buildId") (Join-Path $root "$buildVariant/$buildId") $preserveCheck
            }
        },
        {
            $guardLogs=Join-Path $repo 'target/native-build/guard'
            if(Test-Path -LiteralPath $guardLogs){
                foreach($directory in @(Get-ChildItem -LiteralPath $guardLogs -Directory|Where-Object {$_.LastWriteTimeUtc -ge $started.UtcDateTime})){
                    Save-QualificationTree $directory.FullName (Join-Path $evidence ('guard/'+$directory.Name)) $preserveCheck
                }
            }
        },
        {
            $n2Logs=Join-Path $repo 'target/n2-native/logs'
            if(Test-Path -LiteralPath $n2Logs){
                foreach($directory in @(Get-ChildItem -LiteralPath $n2Logs -Directory|Where-Object {$_.LastWriteTimeUtc -ge $started.UtcDateTime})){
                    Save-QualificationTree $directory.FullName (Join-Path $evidence ('n2-logs/'+$directory.Name)) $preserveCheck
                }
            }
        },
        {
            $n2Runs=Join-Path $repo 'target/n2-native/runs'
            if(Test-Path -LiteralPath $n2Runs){
                foreach($directory in @(Get-ChildItem -LiteralPath $n2Runs -Directory|Where-Object {$_.LastWriteTimeUtc -ge $started.UtcDateTime})){
                    $writes=Join-Path $directory.FullName 'writes'
                    if(Test-Path -LiteralPath $writes){Save-QualificationTree $writes (Join-Path $evidence ('n2-outputs/'+$directory.Name)) $preserveCheck}
                }
            }
        },
        {
            Invoke-QualificationHealth $context -Force -CollectOnly
            $werErrors=@()
            @(Get-WinEvent -FilterHashtable @{LogName='Application';Id=1000,1001;StartTime=$started.LocalDateTime} -ErrorAction SilentlyContinue -ErrorVariable werErrors)|
                ForEach-Object {$_.ToXml()}|Set-Content -LiteralPath (Join-Path $evidence 'application-events.xml') -Encoding utf8
        },
        {Invoke-QualificationHealth $context -Force -CollectOnly},
        {Stop-QualificationMonitor $context $monitorRoot},
        {if($child){$child.Dispose()};if($context.monitor){$context.monitor.Dispose()}},
        {Invoke-QualificationHealth $context -Force -CollectOnly}
    )
}
if($context.failures.Count){throw "Qualification stopped: $($record.failure); evidence=$evidence"}
Write-Host "QUALIF $Stage PASS; peak committed=$([Math]::Round($context.peak,1))%; evidence=$evidence"
