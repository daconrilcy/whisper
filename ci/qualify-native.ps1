#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('SelfTest','SurfaceCpu','SurfaceGpu','RepeatCpu','RepeatGpu','Prepare','NativeSelfTest','N2SelfTest','Cpu1','Cpu2','Gpu1','Gpu2','CpuReference1','CpuReference2','CpuRebuilt','GpuReference1','GpuReference2','GpuRebuilt')][string]$Stage
)
$ErrorActionPreference='Stop'
$PSNativeCommandUseErrorActionPreference=$false
$repo=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\')
$root='C:\wb\native-qualif'
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
function Save-QualificationTree([string]$Source,[string]$Destination) {
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
        $hash=(Get-FileHash -LiteralPath $file.FullName).Hash
        Copy-Item -LiteralPath $file.FullName -Destination $copy
        if((Get-FileHash -LiteralPath $copy).Hash -cne $hash){throw "Archive SHA mismatch: $relative"}
        [ordered]@{path=$relative;bytes=$file.Length;sha256=$hash}
    })
    if(@(Get-ChildItem -LiteralPath $Destination -Recurse -File -Force).Count -ne $files.Count){throw 'Archive file count differs.'}
    $receipt|ConvertTo-Json -Depth 5|Set-Content ($Destination+'.receipt.json') -Encoding utf8
}
function Read-QualificationPe([string]$Path) {
    Assert-QualificationPlainPath $Path
    $bytes=[IO.File]::ReadAllBytes($Path)
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
                $target=U32 ($functions+4*$i);if(-not $target){continue}
                $forward=if($target -ge $exportRva -and $target -lt $exportRva+$exportSize){Str (Rva $target)}else{''}
                $entryNames=if($nameMap.ContainsKey($i)){$nameMap[$i]}else{@('[NONAME]')}
                foreach($name in $entryNames){"$($base+$i)|$name|$forward"}
            }
        }
    )
    return [ordered]@{name=[IO.Path]::GetFileName($Path);machine=(U16 ($pe+4));subsystem=(U16 ($opt+68));exports=@($exports|Sort-Object -CaseSensitive);imports=@($imports|Sort-Object -CaseSensitive);dynamicCrt=[bool]@($imports|Where-Object {$_ -match '\|(vcruntime|msvcp|ucrtbase|api-ms-win-crt)'}).Count}
}
function Compare-QualificationDlls([string]$Variant,[bool]$Repeat) {
    $first=Join-Path $root "$Variant/qualif-${Variant}1/dll"
    $other=if($Repeat){Join-Path $root "$Variant/qualif-${Variant}2/dll"}else{Join-Path $env:LOCALAPPDATA "Whisper/native/$Variant"}
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
                $raw=@(& $tools.tools.dumpbin /nologo "/$mode" $entry.path 2>&1)
                if($LASTEXITCODE -ne 0){throw "dumpbin failed: $name $mode"}
                $raw|Set-Content (Join-Path $root "$Variant/$($entry.label)-$name-$mode.txt") -Encoding utf8
            }
        }
        $surfaceA=Read-QualificationPe $a;$surfaceB=Read-QualificationPe $b
        $sameSurface=($surfaceA|ConvertTo-Json -Depth 8 -Compress) -ceq ($surfaceB|ConvertTo-Json -Depth 8 -Compress)
        $shaA=(Get-FileHash -LiteralPath $a).Hash;$shaB=(Get-FileHash -LiteralPath $b).Hash
        $different=$null
        if($Repeat -and $shaA -cne $shaB){
            $x=[IO.File]::ReadAllBytes($a);$y=[IO.File]::ReadAllBytes($b);$different=[long][Math]::Abs($x.LongLength-$y.LongLength)
            for($i=0;$i -lt [Math]::Min($x.LongLength,$y.LongLength);$i++){if($x[$i] -ne $y[$i]){$different++}}
        }
        [ordered]@{name=$name;sha256Build1=$shaA;sha256Other=$shaB;surfaceEqual=$sameSurface;bytesEqual=($shaA -ceq $shaB);differentBytes=$different;build1Surface=$surfaceA;otherSurface=$surfaceB}
    })
    $reportPath=Join-Path $root "$Variant/$(if($Repeat){'repeat'}else{'surface'}).json"
    $report|ConvertTo-Json -Depth 12|Set-Content $reportPath -Encoding utf8
    if(@($report|Where-Object {if($Repeat){-not $_.bytesEqual}else{-not $_.surfaceEqual}}).Count){
        [ordered]@{at=[DateTimeOffset]::UtcNow.ToString('o');failure='DLL comparison differs; characterize before further execution.';report=$reportPath}|ConvertTo-Json|Set-Content (Join-Path $root 'STOP.json')
        throw "DLL comparison differs; see $reportPath"
    }
    "PASS DLL comparison $Variant repeat=$Repeat report=$reportPath"
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
    'PASS qualification thresholds and all-type WHEA selection; no compilation.';return
}
if($repo -ine 'C:\wt\build-natif-qualif' -or [IO.Path]::GetFullPath((Get-Location).Path).TrimEnd('\') -ine $repo){throw 'Qualification requires its exact dedicated worktree and cwd.'}
Assert-QualificationPlainPath $repo;Assert-QualificationPlainPath $root
if($Stage -match '^(Surface|Repeat)(Cpu|Gpu)$'){Compare-QualificationDlls $Matches[2].ToLowerInvariant() ($Matches[1] -eq 'Repeat');return}
if(Test-Path (Join-Path $root 'STOP.json')){throw 'Qualification previously stopped; no further attempt without user decision.'}
if((Get-PSDrive C).Free -lt 60GB){throw 'Qualification requires at least 60 GiB free.'}
$percent=Get-QualificationMemory;Assert-QualificationMemory $percent $true
$initial=@(Get-QualificationWhea);$lastId=[long](($initial|Measure-Object RecordId -Maximum).Maximum)
$budgetStart=[DateTimeOffset](Get-Content (Join-Path $root 'preflight/initial.json') -Raw|ConvertFrom-Json).at
if(([DateTimeOffset]::UtcNow-$budgetStart).TotalHours -ge 6){throw 'Global qualification budget exhausted.'}
$evidence=Join-Path $root ('steps/'+$Stage)
if(Test-Path -LiteralPath $evidence){throw 'Qualification stage already attempted; no automatic retry.'}
New-Item -ItemType Directory -Path $evidence -Force|Out-Null
$arguments=@('-NoLogo','-NoProfile','-NonInteractive','-File')
$buildVariant=$null;$buildId=$null
switch -Regex ($Stage) {
    '^Prepare$' {$arguments+=@((Join-Path $PSScriptRoot 'build-native.ps1'),'-Variant','cpu','-BuildId','qualif-prepare','-PrepareOnly')}
    '^NativeSelfTest$' {$arguments+=@((Join-Path $PSScriptRoot 'guard-native.ps1'),'-Mode','SelfTest','-Profile','NativeDllCpu','-LotRoot','target/native-build')}
    '^N2SelfTest$' {$arguments+=@((Join-Path $PSScriptRoot 'guard-native.ps1'),'-Mode','SelfTest','-LotRoot','target/n2-native')}
    '^(Cpu|Gpu)(1|2)$' {
        $buildVariant=$Matches[1].ToLowerInvariant();$buildId='qualif-'+$buildVariant+$Matches[2]
        $arguments+=@((Join-Path $PSScriptRoot 'build-native.ps1'),'-Variant',$buildVariant,'-BuildId',$buildId)
    }
    '^(Cpu|Gpu)(Reference[12]|Rebuilt)$' {
        $profile='NativeClosure'+$Matches[1]+($Matches[2] -replace '[12]$','')
        $arguments+=@((Join-Path $PSScriptRoot 'guard-native.ps1'),'-Mode','Run','-Profile',$profile,'-ValuesJson',(Join-Path $repo "target/n2-native/requests/$profile.json"),'-LotRoot','target/n2-native')
    }
    default {throw 'Unknown qualification stage.'}
}
Import-QualificationProcessControls
$monitorRoot=Join-Path $evidence 'memwatch'
New-Item -ItemType Directory -Path $monitorRoot -Force|Out-Null
$monitorInfo=[Diagnostics.ProcessStartInfo]::new((Join-Path $PSHOME 'pwsh.exe'))
$monitorInfo.UseShellExecute=$false;$monitorInfo.CreateNoWindow=$true;$monitorInfo.WorkingDirectory=$repo
foreach($a in @('-NoProfile','-NonInteractive','-File','C:/memwatch/MemWatch.ps1','-Mode','Intensive','-Tag','BUILD-NATIF-QUALIF','-IntervalSeconds','5','-MaxMinutes','360','-OutDir',$monitorRoot)){$monitorInfo.ArgumentList.Add($a)}
$monitor=[Diagnostics.Process]::new();$monitor.StartInfo=$monitorInfo
$childInfo=[Diagnostics.ProcessStartInfo]::new((Join-Path $PSHOME 'pwsh.exe'))
$childInfo.UseShellExecute=$false;$childInfo.CreateNoWindow=$true;$childInfo.WorkingDirectory=$repo
$childInfo.RedirectStandardOutput=$true;$childInfo.RedirectStandardError=$true
foreach($a in $arguments){$childInfo.ArgumentList.Add($a)}
$child=[Diagnostics.Process]::new();$child.StartInfo=$childInfo
$family=$null;$writer=$null;$failure=$null;$peak=$percent;$started=[DateTimeOffset]::UtcNow
$record=[ordered]@{stage=$Stage;startedUtc=$started.ToString('o');wheaBefore=$initial.Count;lastRecordBefore=$lastId;memoryBeforePct=$percent;status='STARTED';command=$arguments;memwatchSha256=(Get-FileHash C:/memwatch/MemWatch.ps1).Hash}
$record|ConvertTo-Json -Depth 6|Set-Content (Join-Path $evidence 'step.json')
try {
    if(-not $monitor.Start()){throw 'MemWatch did not start.'}
    $monitorIdentity=$monitor.StartTime.ToUniversalTime()
    $levelPath=Join-Path $monitorRoot 'niveau-BUILD-NATIF-QUALIF.txt'
    $readyDeadline=[DateTimeOffset]::UtcNow.AddSeconds(20)
    while(-not (Test-Path $levelPath)) {
        if($monitor.HasExited -or [DateTimeOffset]::UtcNow -ge $readyDeadline){throw 'MemWatch not ready.'}
        Start-Sleep -Milliseconds 100
    }
    $percent=Get-QualificationMemory;Assert-QualificationMemory $percent $true
    $beforeLaunch=@(Get-QualificationWhea)
    $newBefore=@(Get-NewQualificationWhea $beforeLaunch $lastId)
    if($newBefore.Count){foreach($event in $newBefore){$event.ToXml()|Set-Content (Join-Path $evidence "whea-$($event.RecordId).xml")};throw 'New WHEA before guarded launch.'}
    $readyLevel=(Get-Content -LiteralPath $levelPath -Raw).Trim()
    if($readyLevel -notin @('OK','ALERTE')){throw 'MemWatch does not permit a new launch.'}
    if(-not $child.Start()){throw 'Guard command did not start.'}
    $family=New-VoxPopuliProcessFamily $child
    $writer=[IO.StreamWriter]::new((Join-Path $evidence 'command.log'),$false,[Text.UTF8Encoding]::new($false));$writer.AutoFlush=$true
    $streams=@(@{reader=$child.StandardOutput;task=$child.StandardOutput.ReadLineAsync();done=$false},@{reader=$child.StandardError;task=$child.StandardError.ReadLineAsync();done=$false})
    $sampleAt=[DateTimeOffset]::MinValue;$heartbeat=[DateTimeOffset]::UtcNow;$streamDeadline=$null
    while(-not ($child.HasExited -and $streams[0].done -and $streams[1].done)) {
        Update-VoxPopuliProcessFamily $family
        foreach($stream in $streams) {
            for($batch=0;$batch -lt 64 -and -not $stream.done -and $stream.task.IsCompleted;$batch++) {
                $line=$stream.task.GetAwaiter().GetResult()
                if($null -eq $line){$stream.done=$true;break}
                $writer.WriteLine($line)
                if($line -match '\bC1001\b|internal compiler error|erreur interne du compilateur'){throw "Compiler ICE: $line"}
                $stream.task=$stream.reader.ReadLineAsync()
            }
        }
        if([DateTimeOffset]::UtcNow -ge $sampleAt) {
            $events=@(Get-QualificationWhea);$new=@(Get-NewQualificationWhea $events $lastId)
            if($new.Count){foreach($event in $new){$event.ToXml()|Set-Content (Join-Path $evidence "whea-$($event.RecordId).xml")};throw 'New WHEA event of any type; qualification stopped.'}
            $percent=Get-QualificationMemory;$peak=[Math]::Max($peak,$percent);Assert-QualificationMemory $percent $false
            if($monitor.HasExited -or $monitor.StartTime.ToUniversalTime() -ne $monitorIdentity){throw 'MemWatch identity or liveness lost.'}
            $level=Get-Item -LiteralPath $levelPath
            if(([datetime]::UtcNow-$level.LastWriteTimeUtc).TotalSeconds -gt 20){throw 'MemWatch sample stale.'}
            $raw=(Get-Content -LiteralPath $levelPath -Raw).Trim()
            $emptyDeadline=[DateTimeOffset]::UtcNow.AddMilliseconds(500)
            while(-not $raw -and [DateTimeOffset]::UtcNow -lt $emptyDeadline){Start-Sleep -Milliseconds 20;$raw=(Get-Content -LiteralPath $levelPath -Raw).Trim()}
            if($raw -eq 'INTERROMPRE'){throw 'MemWatch requests interruption.'}
            if($raw -notin @('OK','ALERTE','STOP-LANCEMENTS')){throw "Invalid MemWatch level: $raw"}
            if($buildId) {
                $buildLogs=Join-Path $repo "target/native-build/$buildVariant/$buildId"
                foreach($log in @(Get-ChildItem -LiteralPath $buildLogs -File -Filter '*.log' -ErrorAction SilentlyContinue)) {
                    if(@(Get-Content -LiteralPath $log.FullName -Tail 80 | Select-String '\bC1001\b|internal compiler error|erreur interne du compilateur').Count){throw "Compiler ICE in $($log.FullName)"}
                }
            }
            [ordered]@{at=[DateTimeOffset]::UtcNow.ToString('o');committedPct=$percent;whea=$events.Count;memwatch=$raw}|ConvertTo-Json -Compress|Add-Content (Join-Path $evidence 'samples.jsonl')
            $sampleAt=[DateTimeOffset]::UtcNow.AddSeconds(1)
        }
        if(([DateTimeOffset]::UtcNow-$heartbeat).TotalSeconds -ge 30){Write-Host "QUALIF $Stage running, memory=$([Math]::Round($percent,1))%, WHEA=$($initial.Count)";$heartbeat=[DateTimeOffset]::UtcNow}
        if($child.HasExited) {
            if($null -eq $streamDeadline){$streamDeadline=[DateTimeOffset]::UtcNow.AddSeconds(10)}
            if([DateTimeOffset]::UtcNow -ge $streamDeadline){throw 'Guard streams did not close.'}
        }
        if(([DateTimeOffset]::UtcNow-$budgetStart).TotalHours -ge 6){throw 'Qualification stage exceeded global budget.'}
        Start-Sleep -Milliseconds 50
    }
    $child.WaitForExit();$record.exitCode=$child.ExitCode
    if($child.ExitCode -ne 0){throw "Guard command failed with exit $($child.ExitCode); no automatic retry."}
    $final=@(Get-QualificationWhea)
    if(@(Get-NewQualificationWhea $final $lastId).Count){foreach($event in @(Get-NewQualificationWhea $final $lastId)){$event.ToXml()|Set-Content (Join-Path $evidence "whea-$($event.RecordId).xml")};throw 'New WHEA at final check.'}
    $record.status='PASS'
} catch {
    $failure=$_.Exception.Message;$record.status='FAIL';$record.failure=$failure
    [ordered]@{stage=$Stage;at=[DateTimeOffset]::UtcNow.ToString('o');failure=$failure}|ConvertTo-Json|Set-Content (Join-Path $root 'STOP.json')
    throw
} finally {
    try { if($family){Stop-VoxPopuliProcessFamily $family} } finally {
        if($family){Close-VoxPopuliProcessFamily $family}
        if($writer){$writer.Dispose()}
        if(-not $monitor.HasExited){'stop'|Set-Content (Join-Path $monitorRoot 'stop-BUILD-NATIF-QUALIF');if(-not $monitor.WaitForExit(10000)){$monitor.Kill($true);$monitor.WaitForExit()}}
        $record.finishedUtc=[DateTimeOffset]::UtcNow.ToString('o');$record.peakMemoryPct=$peak
        $record.wheaAfter=@(Get-QualificationWhea).Count
        $record|ConvertTo-Json -Depth 8|Set-Content (Join-Path $evidence 'step.json')
        $werErrors=@()
        @(Get-WinEvent -FilterHashtable @{LogName='Application';Id=1000,1001;StartTime=$started.LocalDateTime} -ErrorAction SilentlyContinue -ErrorVariable werErrors)|ForEach-Object {$_.ToXml()}|Set-Content (Join-Path $evidence 'application-events.xml')
        $child.Dispose();$monitor.Dispose()
        try {
            if($buildId -and (Test-Path (Join-Path $repo "target/native-build/$buildVariant/$buildId"))){
                Save-QualificationTree (Join-Path $repo "target/native-build/$buildVariant/$buildId") (Join-Path $root "$buildVariant/$buildId")
            }
            $guardLogs=Join-Path $repo 'target/native-build/guard'
            if(Test-Path $guardLogs){
                foreach($directory in @(Get-ChildItem -LiteralPath $guardLogs -Directory | Where-Object {$_.LastWriteTimeUtc -ge $started.UtcDateTime})){
                    Save-QualificationTree $directory.FullName (Join-Path $evidence ('guard/'+$directory.Name))
                }
            }
            $n2Logs=Join-Path $repo 'target/n2-native/logs'
            if(Test-Path $n2Logs){
                foreach($directory in @(Get-ChildItem -LiteralPath $n2Logs -Directory | Where-Object {$_.LastWriteTimeUtc -ge $started.UtcDateTime})){
                    Save-QualificationTree $directory.FullName (Join-Path $evidence ('n2-logs/'+$directory.Name))
                }
            }
            $n2Runs=Join-Path $repo 'target/n2-native/runs'
            if(Test-Path $n2Runs){
                foreach($directory in @(Get-ChildItem -LiteralPath $n2Runs -Directory | Where-Object {$_.LastWriteTimeUtc -ge $started.UtcDateTime})){
                    $writes=Join-Path $directory.FullName 'writes'
                    if(Test-Path $writes){Save-QualificationTree $writes (Join-Path $evidence ('n2-outputs/'+$directory.Name))}
                }
            }
        } catch {
            [ordered]@{stage=$Stage;at=[DateTimeOffset]::UtcNow.ToString('o');failure=('Evidence preservation failed: '+$_.Exception.Message)}|ConvertTo-Json|Set-Content (Join-Path $root 'STOP.json')
            throw
        }
    }
}
Write-Host "QUALIF $Stage PASS; peak committed=$([Math]::Round($peak,1))%; evidence=$evidence"
