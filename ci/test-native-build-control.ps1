# Offline policy/journal parity and load gate; no compiler, process campaign, or watcher started.
[CmdletBinding()]
param([string]$EvidenceRoot, [string]$HistoricalRecords)
$ErrorActionPreference='Stop'
$repo=Split-Path $PSScriptRoot
if (-not $EvidenceRoot) { $EvidenceRoot=Join-Path $repo ('target/native-build/control-tests-'+[guid]::NewGuid().ToString('N')) }
$EvidenceRoot=[IO.Path]::GetFullPath($EvidenceRoot)
$lot=[IO.Path]::GetFullPath((Join-Path $repo 'target/native-build')).TrimEnd('\')
if (-not $EvidenceRoot.StartsWith($lot+'\',[StringComparison]::OrdinalIgnoreCase) -or (Test-Path $EvidenceRoot)) { throw 'Tests require a fresh directory under target/native-build.' }
$cursor=$EvidenceRoot
while ($cursor) {
    if ((Test-Path -LiteralPath $cursor) -and ((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Reparse point: $cursor" }
    $cursor=Split-Path $cursor -Parent
}
if ((Get-PSDrive C).Free -lt 30GB) { throw 'Less than 30 GiB free.' }
New-Item -ItemType Directory $EvidenceRoot | Out-Null
$env:TEMP=Join-Path $EvidenceRoot 'temp'; $env:TMP=$env:TEMP
New-Item -ItemType Directory $env:TEMP | Out-Null
Add-Type -Path @((Join-Path $PSScriptRoot 'native-directory-watch.cs'),(Join-Path $PSScriptRoot 'native-build-control.cs'))
$ast=[Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'guard-native.ps1'),[ref]$null,[ref]$null)
foreach ($name in @('Get-FullPath','Test-InRoot','Get-EventDisposition','Write-JsonLine','Receive-NativeBuildWatchEvents','Complete-NativeBuildCleanup','Stop-WatchState','Drain-WatcherEvents','Receive-WatcherEvents','Stop-CargoProcessTree')) {
    $f=$ast.Find({param($a) $a -is [Management.Automation.Language.FunctionDefinitionAst] -and $a.Name -ceq $name}.GetNewClosure(),$true)
    Invoke-Expression $f.Extent.Text
}
$script:Utf8NoBom=[Text.UTF8Encoding]::new($false)
$allowed=[string[]]@((Join-Path $EvidenceRoot 'allowed'))
$protected=[string[]]@((Join-Path $allowed[0] 'protected'))
$tempRoot=Join-Path $EvidenceRoot 'user-temp'
$results=[Collections.Generic.List[object]]::new()
function Assert([bool]$Condition,[string]$Label) { if (-not $Condition) { throw $Label }; $results.Add(@{test=$Label;result='PASS'}) }
function Observation([string]$Type,[string]$Path,[string]$Old='') {
    $r=[Whisper.NativeWatch.Record]::new(); $r.Root=$EvidenceRoot; $r.Name='é&test'; $r.At='fixed'; $r.Session='s'; $r.Volume='v'; $r.FileId=41; $r.ParentFileId=7; $r.Sequence=1; $r.Action=3
    $e=[Whisper.NativeWatch.Observation]::new(); $e.Type=$Type; $e.Path=$Path; $e.OldPath=$Old; $e.At='fixed'; $e.Detail='detail'; $e.Records=@($r)
    if ($Type -ceq 'Renamed') { $r.Action=4; $n=[Whisper.NativeWatch.Record]::new(); foreach ($field in $r.GetType().GetFields()) { $field.SetValue($n,$field.GetValue($r)) }; $n.Action=5; $n.Sequence=2; $e.Records=@($r,$n) }
    return $e
}
function Snapshot([Whisper.NativeWatch.Observation[]]$Events) {
    $s=[Whisper.NativeWatch.Snapshot]::new(); $s.Drain=1; $s.Events=$Events; $s.Records=@($Events | ForEach-Object { $_.Records }); return $s
}
function JsonEqual([string]$Left,[string]$Right) {
    $l=[Text.Json.Nodes.JsonNode]::Parse($Left); $r=[Text.Json.Nodes.JsonNode]::Parse($Right)
    if ($l -is [Text.Json.Nodes.JsonObject]) { [void]$l.Remove('detected_at'); [void]$r.Remove('detected_at') }
    return [Text.Json.Nodes.JsonNode]::DeepEquals($l,$r)
}
$memory=[IO.StringWriter]::new()
$classifier=[Whisper.NativeBuild.Control]::new($allowed,$protected,$tempRoot,$memory,$memory,$memory,$memory)
foreach ($type in @('Created','Changed','Deleted','Renamed','WatcherError')) {
    foreach ($path in @($allowed[0],(Join-Path $allowed[0] 'a/../b'),$protected[0],$tempRoot,(Join-Path $tempRoot 'a'),'C:\outside',($allowed[0]+'-other'))) {
        foreach ($old in @('', $allowed[0],$protected[0],$tempRoot,'C:\outside')) {
            $e=Observation $type $path $old
            $reference=$null; $actual=$null; $referenceFailed=$false; $actualFailed=$false
            try { $reference=Get-EventDisposition ([pscustomobject]@{type=$type;path=$path;old_path=$old}) $allowed $protected $tempRoot } catch { $referenceFailed=$true }
            try { $actual=$classifier.Disposition($e) } catch { $actualFailed=$true }
            Assert ($actualFailed -eq $referenceFailed -and $actual -ceq $reference) "policy $type $path $old"
        }
    }
}
$classifier.Dispose()

# Complete legacy envelopes, not the simplified diagnostic prototype.
$journalEvents=@((Observation Created (Join-Path $allowed[0] 'a')),(Observation Renamed (Join-Path $tempRoot 'new') (Join-Path $tempRoot 'old')),(Observation WatcherError ''))
$snapshot=Snapshot $journalEvents
$legacy=Join-Path $EvidenceRoot 'legacy'; $fast=Join-Path $EvidenceRoot 'fast'
New-Item -ItemType Directory $legacy,$fast | Out-Null
$fake=[pscustomobject]@{snapshot=$snapshot}; $fake | Add-Member ScriptMethod Drain { return $this.snapshot }
$state=[pscustomobject]@{nativeCollector=$fake;nativeLogRoot=$legacy;queue=[Collections.Concurrent.ConcurrentQueue[object]]::new()}
Receive-NativeBuildWatchEvents $state
$event=$null
while ($state.queue.TryDequeue([ref]$event)) {
    $disposition=Get-EventDisposition $event $allowed $protected $tempRoot
    Write-JsonLine (Join-Path $legacy 'watcher-events.jsonl') ([ordered]@{mode='NativeBuild';path=$event.path;old_path=$event.old_path;event=$event.type;event_at=$event.at;disposition=$disposition;raw_event=$event.raw_event})
    if ($disposition -eq 'UNATTRIBUTED_TEMP') { Write-JsonLine (Join-Path $legacy 'unattributed-temp.jsonl') ([ordered]@{mode='NativeBuild';path=$event.path;old_path=$event.old_path;event=$event.type;event_at=$event.at;detected_at='clock';attribution='UNATTRIBUTED';detail=$event.detail;raw_event=$event.raw_event}) }
    if ($disposition -eq 'VIOLATION') { Write-JsonLine (Join-Path $legacy 'guard-events.jsonl') ([ordered]@{mode='NativeBuild';status='FAIL';path=$event.path;old_path=$event.old_path;event=$event.type;event_at=$event.at;detected_at='clock';action='campaign-process-tree-terminated';detail=$event.detail;raw_event=$event.raw_event}); break }
}
$control=[Whisper.NativeBuild.Control]::new($allowed,$protected,$tempRoot,$fast)
Assert ($control.Process($snapshot).Type -ceq 'WatcherError') 'first violation returned'
$control.Dispose()
foreach ($name in @('watcher-native-records','watcher-events','unattributed-temp','guard-events')) {
    $left=[IO.File]::ReadAllLines((Join-Path $legacy ($name+'.jsonl'))); $right=[IO.File]::ReadAllLines((Join-Path $fast ($name+'.jsonl')))
    Assert ($left.Count -eq $right.Count) "journal count $name"
    for ($i=0;$i -lt $left.Count;$i++) { Assert (JsonEqual $left[$i] $right[$i]) "journal complete semantic parity $name $i" }
}

# Journal write/flush/close failures must escape; all writers still receive cleanup.
Add-Type -TypeDefinition @'
using System; using System.IO;
public class FaultJournal : StringWriter {
 public string Stage; public bool Closed;
 public override void WriteLine(string s) { if(Stage=="write") throw new IOException("write witness"); base.WriteLine(s); }
 public override void Flush() { if(Stage=="flush") throw new IOException("flush witness"); base.Flush(); }
 protected override void Dispose(bool b) { Closed=true; if(Stage=="dispose") throw new IOException("dispose witness"); base.Dispose(b); }
}
'@
foreach ($stage in @('write','flush','dispose')) {
    $writers=@(0..3 | ForEach-Object { [FaultJournal]::new() }); $writers[0].Stage=$stage
    $c=[Whisper.NativeBuild.Control]::new($allowed,$protected,$tempRoot,$writers[0],$writers[1],$writers[2],$writers[3])
    $failed=$false
    try { if ($stage -eq 'dispose') { $c.Dispose() } else { $null=$c.Process($snapshot) } } catch { $failed=$true }
    Assert $failed "journal $stage fatal"
    if ($stage -ne 'dispose') { $c.Dispose() }
    Assert (@($writers | Where-Object { -not $_.Closed }).Count -eq 0) "journal $stage cleanup complete"
}

# Exercise production cleanup orchestration, including the final snapshot on the failure path.
$cleanupRoot=Join-Path $EvidenceRoot 'cleanup'; New-Item -ItemType Directory $cleanupRoot | Out-Null
$terminalLog=Join-Path $cleanupRoot 'terminal'; New-Item -ItemType Directory $terminalLog | Out-Null
$terminalCollector=[pscustomobject]@{snapshot=$snapshot;stopped=$false;drained=$false}
$terminalCollector | Add-Member ScriptMethod Dispose { $this.stopped=$true }
$terminalCollector | Add-Member ScriptMethod Drain { if (-not $this.stopped) { throw 'terminal drain before shutdown' }; $this.drained=$true; return $this.snapshot }
$terminalControl=[Whisper.NativeBuild.Control]::new($allowed,$protected,$tempRoot,$terminalLog)
$terminalState=[pscustomobject]@{nativeCollector=$terminalCollector;nativeControl=$terminalControl;nativeStopped=$false}
$cleanupSteps=[Collections.Generic.List[string]]::new()
$cleanupRecord=[ordered]@{status='FAIL';failure='original failure'}
Complete-NativeBuildCleanup @(
    { Stop-WatchState $terminalState },
    { Drain-WatcherEvents $terminalState $allowed $protected $tempRoot $null 'NativeBuild' $terminalLog },
    { $terminalControl.Dispose(); throw 'injected close failure' },
    { $cleanupSteps.Add('handlers removed') },
    { $cleanupSteps.Add('stdout closed') },
    { $cleanupSteps.Add('stderr closed') },
    { $cleanupSteps.Add('process disposed') }
) $cleanupRecord $cleanupRoot
Assert ($terminalCollector.stopped -and $terminalCollector.drained) 'failure cleanup drains after collector shutdown'
Assert ($cleanupSteps.Count -eq 4) 'cleanup continues after terminal drain and dispose errors'
$receipt=Get-Content (Join-Path $cleanupRoot 'guard-result.json') -Raw | ConvertFrom-Json
Assert ($receipt.status -ceq 'FAIL' -and $receipt.failure -ceq 'original failure' -and $receipt.cleanupFailures.Count -eq 2) 'failure receipt preserves original and cleanup failures'
Assert ([IO.File]::ReadAllLines((Join-Path $terminalLog 'watcher-native-records.jsonl')).Count -eq 1) 'terminal snapshot journal preserved on failure'
$newFailureRoot=Join-Path $cleanupRoot 'new-failure'; New-Item -ItemType Directory $newFailureRoot | Out-Null
$threw=$false
try { Complete-NativeBuildCleanup @({ throw 'cleanup-only failure' },{ $cleanupSteps.Add('late cleanup') }) ([ordered]@{status='PASS'}) $newFailureRoot } catch { $threw=$true }
Assert ($threw -and (Get-Content (Join-Path $newFailureRoot 'guard-result.json') -Raw | ConvertFrom-Json).status -ceq 'FAIL') 'cleanup-only failure throws after FAIL receipt'

# Permanent ambiguous/orphan fixtures; no retention across boundaries.
$pair=(Observation Renamed (Join-Path $allowed[0] 'new') (Join-Path $allowed[0] 'old')).Records
$pair[0].Name='old'; $pair[1].Name='new'
foreach ($records in @((,$pair[0]),(,$pair[1]),@($pair[0],$pair[1],$pair[0],$pair[1]))) {
    Assert (@([Whisper.NativeWatch.Collector]::Assemble([Whisper.NativeWatch.Record[]]$records) | Where-Object Type -CEQ WatcherError).Count -gt 0) 'orphan/ambiguous pair still refused'
}

if ($HistoricalRecords) {
    foreach ($line in [IO.File]::ReadLines($HistoricalRecords)) {
        $json=[Text.Json.JsonDocument]::Parse($line)
        $drain=$json.RootElement.GetProperty('Drain').GetInt64(); $json.Dispose()
        if ($drain -ne 83) { continue }
        $options=[Text.Json.JsonSerializerOptions]::new(); $options.IncludeFields=$true
        $historical=[Text.Json.JsonSerializer]::Deserialize($line,[Whisper.NativeWatch.Snapshot],$options)
        Assert (@([Whisper.NativeWatch.Collector]::Assemble($historical.Records) | Where-Object Type -CEQ WatcherError).Count -gt 0) 'historical snapshot 83 remains FAIL'
    }
}

# Full assembly + classification + all journals + flush. Comparable burst, repeated drains.
$records=[Whisper.NativeWatch.Record[]]::new(55414)
for ($i=0;$i -lt $records.Length;$i++) {
    $r=[Whisper.NativeWatch.Record]::new(); $r.Root=$allowed[0]; $r.Name="file-$i"; $r.Session='load'; $r.Volume='v'; $r.Action=3; $r.FileId=[uint64]($i+1); $r.Sequence=$i+1; $r.At='fixed'; $records[$i]=$r
}
$load=Join-Path $EvidenceRoot 'load'; New-Item -ItemType Directory $load | Out-Null
$c=[Whisper.NativeBuild.Control]::new($allowed,$protected,$tempRoot,$load)
$timings=[Collections.Generic.List[double]]::new()
try {
    foreach ($iteration in 1..3) {
        $sw=[Diagnostics.Stopwatch]::StartNew()
        $s=[Whisper.NativeWatch.Snapshot]::new(); $s.Drain=$iteration; $s.Records=$records; $s.Events=[Whisper.NativeWatch.Collector]::Assemble($records)
        $failure=$c.Process($s); $sw.Stop(); $timings.Add($sw.Elapsed.TotalMilliseconds)
        Assert ($null -eq $failure -and $s.Events.Length -eq 55414) "burst $iteration no loss"
        Assert ($sw.Elapsed.TotalMilliseconds -le 1000) "burst $iteration assembly+policy+journals <=1s"
        Assert (($sw.Elapsed.TotalMilliseconds+100) -le 1000) "burst $iteration drain interval including 100ms pause <=1s"
        Start-Sleep -Milliseconds 100
    }
} finally { $c.Dispose() }
Assert ([Linq.Enumerable]::Count[string]([IO.File]::ReadLines((Join-Path $load 'watcher-events.jsonl'))) -eq (3*55414)) 'load event count exact'
@{status='PASS';tests=$results;burstMs=$timings;nativeBuildStarted=$false;watcherStarted=$false;historicalWitness=if ($HistoricalRecords) {'83 FAIL'} else {'NON EXÉCUTÉ'}} | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $EvidenceRoot 'result.json') -Encoding utf8NoBOM
"PASS NativeBuild control: evidence=$EvidenceRoot burstMs=$($timings -join ',')"
