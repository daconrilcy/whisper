# Safety witnesses for the isolated compiler driver; no cl.exe/CMake/vcpkg invocation.
[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'repro-native-compiler.ps1') -LibraryOnly
$evidence=Join-Path $script:Lot ('repro-selftest-'+[guid]::NewGuid().ToString('N'))
Assert-NoReparseChain $evidence 'self-test root'
if((Get-PSDrive C).Free -lt 30GB){throw 'Less than 30 GiB free.'}
New-Item -ItemType Directory -Path $evidence | Out-Null
$script:Journal=Join-Path $evidence 'journal.jsonl'
$checks=[Collections.Generic.List[string]]::new()
function Reject([string]$Name,[scriptblock]$Action,[string]$Reason){
    $errorText=$null;try{& $Action}catch{$errorText=$_.Exception.Message}
    if(-not $errorText -or $errorText -notlike $Reason){throw "Negative witness failed: $Name : $errorText"}
    $checks.Add($Name)
}
Reject 'root itself' {Assert-ReproOutput 'C:\wb\c1001'} '*strict descendant*'
Reject 'sibling prefix' {Assert-ReproOutput 'C:\wb\c1001-other\a'} '*strict descendant*'
Reject 'traversal' {Assert-ReproOutput 'C:\wb\c1001\..\other\a'} '*strict descendant*'
Reject 'unknown stage' {Get-ReproSchedule 'C'} '*Unknown reproduction*'
if(-not(Test-ReproDumpName 'cl.exe.123.dmp' 123) -or -not(Test-ReproDumpName 'cl.exe.123.extra.dmp' 123) -or (Test-ReproDumpName 'cl.exe.1234.dmp' 123) -or (Test-ReproDumpName 'cl.exe.12.dmp' 123)){throw 'Dump PID delimitation failed.'};$checks.Add('dump PID exact')
Reject 'quota override' {& (Join-Path $PSScriptRoot 'repro-native-compiler.ps1') -LibraryOnly -Iterations 1} '*parameter*Iterations*'
$a=Get-ReproSchedule A;$b=Get-ReproSchedule B1;$c=Get-ReproSchedule B2
if($a.count -ne 100 -or $a.parallel -ne 1 -or $b.count -ne 200 -or $b.parallel -ne 4 -or $c.count -ne 100 -or $c.parallel -ne 2){throw 'Quotas differ.'};$checks.Add('quotas fixed')
$target=Join-Path $evidence 'link-target';New-Item -ItemType Directory -Path $target | Out-Null
$link=Join-Path $evidence 'junction';New-Item -ItemType Junction -Path $link -Target $target | Out-Null
Reject 'junction refused' {Assert-NoReparseChain (Join-Path $link 'child') 'fixture'} '*reparse point*'
$command=Get-ReproCommand;$checks.Add('exact archived command and binaries')
$specs=@(Get-RunWatchSpecs $script:Lot)
if(@($specs | Where-Object {$_.recursive -and (Test-InRoot $evidence $_.path)}).Count){throw 'Evidence logs are recursively watched.'};$checks.Add('guard evidence excluded from recursive acquisition')
$oldLot=$script:Lot
try{
    $script:Lot=$evidence
    $command.exactCommand+=' /Od'
    $command | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $evidence 'isolated-command.json') -Encoding utf8NoBOM
    Reject 'optimization tampering' {Get-ReproCommand} '*unique archived command*'
}finally{$script:Lot=$oldLot}
# Fake only the level source and WHEA reader, using this live process as monitor identity.
# These fixtures exercise refusal before launch; they are never used by the production stage.
function Get-WinEvent {return @()}
$script:Monitor=[Diagnostics.Process]::GetCurrentProcess();$script:MonitorCreated=$script:Monitor.StartTime.ToUniversalTime()
$script:MonitorHash=(Get-FileHash -LiteralPath C:/memwatch/MemWatch.ps1).Hash
$script:LevelFile=Join-Path $evidence 'level.txt';$script:MonitorCsv=Join-Path $evidence 'sample.csv'
[IO.File]::WriteAllText($script:MonitorCsv,'fixture',$script:Utf8NoBom)
foreach($level in @('OK','ALERTE','STOP-LANCEMENTS')){
    [IO.File]::WriteAllText($script:LevelFile,$level,$script:Utf8NoBom)
    if((Get-ReproLevel) -cne $level){throw 'Level changed.'}
    $checks.Add("level $level")
}
[IO.File]::WriteAllText($script:LevelFile,'UNKNOWN',$script:Utf8NoBom)
Reject 'unknown level' {Get-ReproLevel} '*Invalid MemWatch level*'
[IO.File]::WriteAllText($script:LevelFile,'OK',$script:Utf8NoBom)
(Get-Item -LiteralPath $script:LevelFile).LastWriteTimeUtc=[datetime]::UtcNow.AddMinutes(-1)
Reject 'stale level' {Get-ReproLevel} '*missing or stale*'
[IO.File]::WriteAllText($script:LevelFile,'OK',$script:Utf8NoBom)
$originalEnd=$script:End;$script:End=[datetimeoffset]::UtcNow.AddSeconds(-1)
Reject 'deadline' {Get-ReproLevel} '*deadline*';$script:End=$originalEnd
$script:MonitorCreated=$script:MonitorCreated.AddSeconds(-1)
Reject 'monitor identity' {Get-ReproLevel} '*identity lost*';$script:MonitorCreated=$script:Monitor.StartTime.ToUniversalTime()
function Get-WinEvent {return @([pscustomobject]@{Id=19;Message='fixture WHEA'})}
Reject 'WHEA' {Get-ReproLevel} '*MATERIEL A REVERIFIER*'
function Get-WinEvent {return @()}
# A stopped launch must not create its output directory or start the supplied executable.
$script:OutputRoot=Join-Path $evidence 'outputs';New-Item -ItemType Directory -Path $script:OutputRoot | Out-Null
$savedLot=$script:Lot;$script:Lot=$evidence
try{
    Reject 'preprocessing required' {Assert-ReproPreprocessed} '*receipt missing*'
    New-Item -ItemType Directory -Path (Join-Path $evidence 'isolated-Preprocess') | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $script:OutputRoot 'preprocessed') | Out-Null
    $preprocessed=Join-Path $script:OutputRoot 'preprocessed/any_lite.i'
    [IO.File]::WriteAllText($preprocessed,'fixture source',$script:Utf8NoBom)
    @{pass=$true;preprocessedSha256=(Get-FileHash -LiteralPath $preprocessed).Hash} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'isolated-Preprocess/result.json') -Encoding utf8NoBOM
    Assert-ReproPreprocessed;$checks.Add('qualified preprocessing accepted')
    [IO.File]::AppendAllText($preprocessed,' changed',$script:Utf8NoBom)
    Reject 'preprocessed alteration' {Assert-ReproPreprocessed} '*artifact altered*'
}finally{$script:Lot=$savedLot}
[IO.File]::WriteAllText($script:LevelFile,'STOP-LANCEMENTS',$script:Utf8NoBom)
$never=Join-Path $script:OutputRoot 'never'
$notStarted=Start-ReproProcess 'does-not-exist.exe' @() $never (New-ReproEnvironment) 1
if($null -ne $notStarted -or (Test-Path -LiteralPath $never)){throw 'STOP-LANCEMENTS created/launched an output.'};$checks.Add('STOP prevents launch before output creation')
# Exercise actual process ownership and stdout/stderr closure with a harmless PowerShell child.
[IO.File]::WriteAllText($script:LevelFile,'OK',$script:Utf8NoBom)
$script:Stage='Environment'
$entry=$null
try{
    $entry=Start-ReproProcess (Join-Path $env:SystemRoot 'System32/cmd.exe') @('/d','/s','/c','"echo stdout&echo stderr 1>&2"') (Join-Path $script:OutputRoot 'child') (New-ReproEnvironment) 1
    while(-not $entry.process.HasExited){Update-VoxPopuliProcessFamily $entry.family;Start-Sleep -Milliseconds 50}
    Complete-ReproProcess $entry
    if(-not $entry.finished -or $entry.exit -ne 0 -or -not $entry.family.cleanupClosed -or [IO.File]::ReadAllText((Join-Path $entry.directory 'stdout.log')).Trim() -cne 'stdout' -or [IO.File]::ReadAllText((Join-Path $entry.directory 'stderr.log')).Trim() -cne 'stderr'){throw 'Real capture/cleanup witness differs.'}
    $checks.Add('real owned process and both closed streams')
}finally{if($null -ne $entry){Stop-VoxPopuliProcessFamily $entry.family;Close-VoxPopuliProcessFamily $entry.family;$entry.process.Dispose()}}
$entry=$null
try{
    $entry=Start-ReproProcess (Join-Path $env:SystemRoot 'System32/cmd.exe') @('/d','/s','/c','"echo started&ping -n 30 127.0.0.1 >nul"') (Join-Path $script:OutputRoot 'interrupted-child') (New-ReproEnvironment) 2
    Start-Sleep -Milliseconds 200
    $script:Stage='A' # Forced interruption must bypass the ordinary crash/dump path.
    Complete-ReproProcess $entry -Interrupted
    if(-not $entry.finished -or -not $entry.process.HasExited -or -not $entry.family.cleanupClosed -or -not(Test-Path -LiteralPath (Join-Path $entry.directory 'stdout.log')) -or -not(Test-Path -LiteralPath (Join-Path $entry.directory 'stderr.log'))){throw 'Interrupted process/captures not closed.'}
    $checks.Add('forced interruption closes family and preserves both captures without WER wait')
}finally{if($null -ne $entry){Stop-VoxPopuliProcessFamily $entry.family;Close-VoxPopuliProcessFamily $entry.family;$entry.process.Dispose()}}
@{pass=$true;checks=$checks.ToArray();nativeCompilerStarted=$false;fixtures=$evidence;retainedJunction=$link;junctionTarget=$target} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $evidence 'result.json') -Encoding utf8NoBOM
"REPRO_SELFTEST PASS checks=$($checks.Count) evidence=$evidence compilerStarted=false"
