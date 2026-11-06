# NativeBuild notification tests only; dot-sourced by the guard, never launches CMake/Cargo.
function Invoke-NativeQualificationRoutingTests {
    $tokens=$null;$errors=$null
    $ast=[Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'qualify-native.ps1'),[ref]$tokens,[ref]$errors)
    if($errors.Count){throw 'Qualification routing source does not parse.'}
    foreach($name in @('Get-QualificationRouting','Get-QualificationBuildId','Assert-QualificationStageRouting')){
        $definition=@($ast.EndBlock.Statements|Where-Object {$_ -is [Management.Automation.Language.FunctionDefinitionAst] -and $_.Name -ceq $name})
        if($definition.Count -ne 1){throw "Qualification routing helper missing: $name"}
        . ([scriptblock]::Create($definition[0].Extent.Text))
    }
    $normal=Get-QualificationRouting ''
    if($normal.root -cne 'C:\wb\native-qualif'){throw 'Original qualification root differs.'}
    foreach($id in @('r1','r2','r3','r4')){
        $reprise=Get-QualificationRouting $id
        if($reprise.root -cne ('C:\wb\native-qualif\reprise-'+$id.Substring(1)) -or $reprise.original -cne $normal.root){throw 'Qualification reprise root differs from its exact authorized route.'}
        $variants=if($id -cin @('r3','r4')){@('gpu')}else{@('cpu','gpu')}
        foreach($variant in $variants){
            foreach($number in @(1,2)){
                if((Get-QualificationBuildId $variant $number $id) -cne "qualif-$id-$variant$number" -or
                    (Get-QualificationBuildId $variant $number '') -cne "qualif-$variant$number"){throw 'Qualification reprise build ID routing differs.'}
            }
        }
    }
    foreach($invalid in @('r5','R1','R2','R3','R4','../reprise-1','C:/override')){
        $rejected=$false;try{$null=Get-QualificationRouting $invalid}catch{$rejected=$true}
        if(-not $rejected){throw "Unapproved qualification reprise accepted: $invalid"}
    }
    foreach($id in @('r3','r4')){
        foreach($stage in @('SelfTest','NativeSelfTest','SurfaceGpu','RepeatGpu','Gpu1','Gpu2','GpuReference1','GpuReference2','GpuRebuilt')){Assert-QualificationStageRouting $stage $id}
        foreach($stage in @('Prepare','Cpu1','Cpu2','SurfaceCpu','RepeatCpu','CpuReference1','CpuRebuilt','N2SelfTest')){
            $rejected=$false;try{Assert-QualificationStageRouting $stage $id}catch{$rejected=$true}
            if(-not $rejected){throw "Non-GPU stage accepted in ${id}: $stage"}
        }
        $rejected=$false;try{$null=Get-QualificationBuildId 'cpu' 1 $id}catch{$rejected=$true}
        if(-not $rejected){throw "CPU build ID accepted in GPU-only $id."}
    }
    [ordered]@{test='bounded r1/r2 roots and GPU-only r3/r4 stages/build IDs; arbitrary override refused';result='PASS'}
}
function Invoke-NativeDirectoryWatchTests([string]$Root, [object[]]$Specs, [switch]$RecordTestsOnly) {
    Initialize-NativeDirectoryWatch (Join-Path $Root 'collector-temp')
    $evidence=[Collections.Generic.List[object]]::new()
    function New-TestRecord([uint32]$Action, [string]$Name, [uint64]$Id=41, [long]$Sequence=1, [long]$Buffer=1) {
        $r=[Whisper.NativeWatch.Record]::new()
        $r.Root=$Root; $r.Session='test-session'; $r.Volume='test-volume'; $r.Name=$Name
        $r.Action=$Action; $r.FileId=$Id; $r.ParentFileId=7; $r.Sequence=$Sequence; $r.Buffer=$Buffer; $r.Index=[int]$Sequence
        $r.CreationTime=1; $r.At=[DateTimeOffset]::UtcNow.ToString('o')
        return $r
    }
    function Assert-RecordResult([string]$Label, [Whisper.NativeWatch.Record[]]$Records, [bool]$Fails) {
        $events=[Whisper.NativeWatch.Collector]::Assemble($Records)
        $failed=@($events | Where-Object Type -CEQ 'WatcherError').Count -gt 0
        if ($failed -ne $Fails) { throw "Extended collector ${Label}: failure=$failed expected=$Fails" }
        $evidence.Add([ordered]@{test=$Label;result='PASS';expectedFailure=$Fails})
        return ,$events
    }
    $old=New-TestRecord 4 'allowed/old.tmp' 41 1 1
    $new=New-TestRecord 5 'allowed/new.txt' 41 3 2
    $change=New-TestRecord 3 'noise.txt' 65 2 1
    $events=Assert-RecordResult 'interleaved across received buffers' @($old,$change,$new) $false
    $rename=@($events | Where-Object Type -CEQ 'Renamed')
    if ($rename.Count -ne 1 -or $rename[0].OldPath -ine (Join-Path $Root 'allowed/old.tmp') -or $rename[0].Path -ine (Join-Path $Root 'allowed/new.txt')) { throw 'Extended pair paths differ.' }
    $chain=@((New-TestRecord 4 'allowed/a.tmp' 41 80731 79320),(New-TestRecord 5 'allowed/a.trn' 41 80732 79320),
        (New-TestRecord 4 'allowed/a.trn' 41 80738 79326),(New-TestRecord 5 'allowed/b.trn' 41 80739 79326))
    $chain[0].Index=0;$chain[1].Index=1;$chain[2].Index=0;$chain[3].Index=1
    $chainEvents=Assert-RecordResult 'two sequential rename pairs of one generation in one drain' $chain $false
    if($chainEvents.Count -ne 2 -or $chainEvents[0].OldPath -ine (Join-Path $Root 'allowed/a.tmp') -or
        $chainEvents[0].Path -ine (Join-Path $Root 'allowed/a.trn') -or $chainEvents[1].OldPath -ine $chainEvents[0].Path -or
        $chainEvents[1].Path -ine (Join-Path $Root 'allowed/b.trn')){throw 'Sequential rename chain lost or collapsed a transition.'}
    foreach($mutation in @('generation','duplicate','sequence','buffer','index','broken-chain','parent-identity')){
        $broken=@($chain|ForEach-Object {$copy=[Whisper.NativeWatch.Record]::new();foreach($field in $_.GetType().GetFields()){$field.SetValue($copy,$field.GetValue($_))};$copy})
        switch($mutation){
            'generation' {$broken[2].CreationTime=2;$broken[3].CreationTime=2}
            'duplicate' {$broken+=($broken[3])}
            'sequence' {$broken[2].Sequence=$broken[1].Sequence}
            'buffer' {$broken[2].Buffer=$broken[0].Buffer-1}
            'index' {$broken[1].Index=$broken[0].Index}
            'broken-chain' {$broken[2].Name='allowed/unrelated.trn'}
            'parent-identity' {$broken[2].ParentFileId=8}
        }
        $null=Assert-RecordResult "rename chain rejects $mutation" $broken $true
    }
    foreach ($case in @(
        @{name='old orphan';records=@($old)}, @{name='new orphan';records=@($new)},
        @{name='different identity';records=@($old,(New-TestRecord 5 'allowed/new.txt' 42 3))},
        @{name='missing identity';records=@((New-TestRecord 4 'old' 0),$new)},
        @{name='unordered';records=@($new,$old)},
        @{name='repeated identity ambiguous';records=@($old,$new,(New-TestRecord 4 'another.tmp' 41 4),(New-TestRecord 5 'another.txt' 41 5))},
        @{name='missing name';records=@((New-TestRecord 4 ''),$new)},
        @{name='escape name';records=@((New-TestRecord 3 '../outside'))},
        @{name='absolute name';records=@((New-TestRecord 3 'C:\outside'))},
        @{name='unknown action';records=@((New-TestRecord 19 'file'))}
    )) { $null=Assert-RecordResult $case.name $case.records $true }
    foreach ($property in @('Volume','Session','CreationTime','Root','ParentFileId')) {
        $other=New-TestRecord 5 'allowed/new.txt' 41 3
        if ($property -eq 'CreationTime') { $other.$property=2 }
        elseif ($property -eq 'ParentFileId') { $other.$property=0 }
        else { $other.$property='different' }
        $null=Assert-RecordResult "different or invalid $property" @($old,$other) $true
    }
    $errorRecord=New-TestRecord 3 'error'; $errorRecord.Error='ERROR_NOTIFY_ENUM_DIR'
    $null=Assert-RecordResult 'native loss error' @($errorRecord) $true
    # A half delivered in a later drainage must never repair the previous failed drainage.
    $null=Assert-RecordResult 'first drain remains orphan' @($old) $true
    $null=Assert-RecordResult 'next drain remains orphan' @($new) $true

    # Construct a real extended packet, then exercise the parser's failure cases.
    $nameBytes=[Text.Encoding]::Unicode.GetBytes('allowed.txt')
    $packet=[byte[]]::new(84+$nameBytes.Length)
    [BitConverter]::GetBytes([uint32]1).CopyTo($packet,4)
    [BitConverter]::GetBytes([uint64]41).CopyTo($packet,64)
    [BitConverter]::GetBytes([uint64]7).CopyTo($packet,72)
    [BitConverter]::GetBytes([uint32]$nameBytes.Length).CopyTo($packet,80)
    $nameBytes.CopyTo($packet,84)
    $parsed=[Whisper.NativeWatch.Collector]::Parse($packet,$Root,'session','volume',12)
    if ($parsed.Count -ne 1 -or $parsed[0].Name -cne 'allowed.txt' -or $parsed[0].FileId -ne 41 -or $parsed[0].Buffer -ne 12) { throw 'Extended packet offsets differ.' }
    foreach ($bad in @([byte[]]@(),[byte[]]::new(83))) {
        Test-Rejects 'empty/truncated extended packet' { [Whisper.NativeWatch.Collector]::Parse($bad,$Root,'session','volume',1) }
    }
    foreach ($field in @('length','next','unicode')) {
        $bad=[byte[]]$packet.Clone()
        if ($field -eq 'length') { [BitConverter]::GetBytes([uint32]3).CopyTo($bad,80) }
        elseif ($field -eq 'next') { [BitConverter]::GetBytes([uint32]4).CopyTo($bad,0) }
        else { $bad[84]=0; $bad[85]=0xd8; $bad[86]=0; $bad[87]=0 }
        Test-Rejects "invalid extended packet $field" { [Whisper.NativeWatch.Collector]::Parse($bad,$Root,'session','volume',1) }
    }
    $evidence.Add([ordered]@{test='packet offsets, empty, truncation, offsets, lengths, UTF16';result='PASS'})

    # Classify exact pairs through the unmodified policy. No writes to real protected roots.
    foreach ($case in @(
        @{name='allowed pair';old='allowed/old';new='allowed/new';expected='ALLOWED'},
        @{name='protected destination';old='allowed/old';new='protected/new';expected='VIOLATION'},
        @{name='outside destination';old='allowed/old';new='outside/new';expected='VIOLATION'},
        @{name='Temp internal';old='user-temp/old';new='user-temp/new';expected='UNATTRIBUTED_TEMP'},
        @{name='Temp outgoing';old='user-temp/old';new='allowed/new';expected='VIOLATION'},
        @{name='Temp incoming';old='allowed/old';new='user-temp/new';expected='VIOLATION'}
    )) {
        $pair=[Whisper.NativeWatch.Collector]::Assemble(@((New-TestRecord 4 $case.old 41 1),(New-TestRecord 5 $case.new 41 2)))[0]
        $event=[pscustomobject]@{type=$pair.Type;path=$pair.Path;old_path=$pair.OldPath}
        $result=Get-EventDisposition $event @((Join-Path $Root 'allowed')) @((Join-Path $Root 'protected')) (Join-Path $Root 'user-temp')
        if ($result -cne $case.expected) { throw "Extended policy $($case.name): $result" }
        $evidence.Add([ordered]@{test=$case.name;result='PASS';disposition=$result})
    }
    $boundaryChain=@((New-TestRecord 4 'allowed/a.tmp' 41 1),(New-TestRecord 5 'outside/a.trn' 41 2),
        (New-TestRecord 4 'outside/a.trn' 41 3),(New-TestRecord 5 'allowed/b.trn' 41 4))
    $boundaryEvents=Assert-RecordResult 'complete chain preserves both boundary crossings' $boundaryChain $false
    if($boundaryEvents.Count -ne 2){throw 'Boundary chain must preserve two transitions.'}
    foreach($transition in $boundaryEvents){
        $event=[pscustomobject]@{type=$transition.Type;path=$transition.Path;old_path=$transition.OldPath}
        if((Get-EventDisposition $event @((Join-Path $Root 'allowed')) @() (Join-Path $Root 'user-temp')) -cne 'VIOLATION'){throw 'Boundary crossing within rename chain was authorized.'}
    }
    $evidence.Add([ordered]@{test='both intermediate boundary crossings remain refused';result='PASS'})
    $evidence.Add((Invoke-NativeQualificationRoutingTests))
    if($RecordTestsOnly){return ,$evidence.ToArray()}

    # Arm the exact complete set, without writing to those watched roots.
    $availability=Start-NativeBuildWatchState $Specs $Root
    try {
        $evidence.Add([ordered]@{test='all actual roots NTFS/API/open/armed';result='PASS';specifications=$Specs})
    } finally { Stop-WatchState $availability }
    if (@($availability.queue.ToArray() | Where-Object type -CEQ 'WatcherError').Count -gt 0) { throw 'Actual-root watcher failed during availability probe.' }

    $fixture=Join-Path $Root 'real-collector'
    New-Item -ItemType Directory -Path $fixture -Force | Out-Null
    $live=Start-NativeBuildWatchState @([pscustomobject]@{path=$fixture;recursive=$true}) $Root
    $writers=[Collections.Generic.List[Diagnostics.Process]]::new()
    $writerCaptures=[Collections.Generic.List[object]]::new()
    $seen=[Collections.Generic.List[object]]::new()
    try {
        foreach ($worker in 0..1) {
            $workerRoot=Join-Path $fixture "worker-$worker"
            New-Item -ItemType Directory -Path $workerRoot -Force | Out-Null
            $command='$ErrorActionPreference="Stop"; $p=$env:NATIVE_WATCH_FIXTURE; for($i=0;$i -lt 500;$i++){ $s=[IO.Path]::Combine($p,("rules.ninja.tmp"+$i.ToString("x8"))); [IO.File]::WriteAllText($s,"rule $i"); [IO.File]::AppendAllText([IO.Path]::Combine($p,"noise"),"x"); [IO.File]::Move($s,[IO.Path]::Combine($p,"rules.ninja"),$true); [IO.File]::AppendAllText([IO.Path]::Combine($p,"noise"),"y") }'
            $psi=[Diagnostics.ProcessStartInfo]::new((Join-Path $PSHOME 'pwsh.exe'))
            $psi.UseShellExecute=$false; $psi.CreateNoWindow=$true; $psi.WorkingDirectory=$workerRoot
            $psi.RedirectStandardOutput=$true; $psi.RedirectStandardError=$true
            foreach ($arg in @('-NoLogo','-NoProfile','-NonInteractive','-EncodedCommand',[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($command)))) { $psi.ArgumentList.Add($arg) }
            $psi.Environment['NATIVE_WATCH_FIXTURE']=$workerRoot
            foreach ($variable in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) {
                $isolated=Join-Path $Root ('writer-env/'+$worker+'/'+$variable.ToLowerInvariant())
                New-Item -ItemType Directory -Path $isolated -Force | Out-Null
                $psi.Environment[$variable]=$isolated
            }
            $process=[Diagnostics.Process]::new(); $process.StartInfo=$psi
            if (-not $process.Start()) { throw 'Could not start notification fixture writer.' }
            $writers.Add($process)
            $writerCaptures.Add([pscustomobject]@{process=$process;worker=$worker;stdout=$process.StandardOutput.ReadToEndAsync();stderr=$process.StandardError.ReadToEndAsync()})
        }
        $deadline=[DateTimeOffset]::UtcNow.AddSeconds(30)
        $quietSince=[DateTimeOffset]::UtcNow
        do {
            Receive-WatcherEvents $live
            $event=$null; $received=$false
            while ($live.queue.TryDequeue([ref]$event)) {
                $seen.Add($event); $received=$true
                if ($event.type -eq 'WatcherError') { throw "Real collector failed closed: $($event.detail)" }
            }
            if ($received) { $quietSince=[DateTimeOffset]::UtcNow }
            $running=@($writers | Where-Object { -not $_.HasExited }).Count -gt 0
            if ([DateTimeOffset]::UtcNow -ge $deadline) { throw 'Notification fixture exceeded 30 seconds.' }
            Start-Sleep -Milliseconds 100
        } while ($running -or ([DateTimeOffset]::UtcNow-$quietSince).TotalSeconds -lt 1)
        foreach ($writer in $writers) { if ($writer.ExitCode -ne 0) { throw "Fixture writer exit $($writer.ExitCode)" } }
        Stop-WatchState $live
        $event=$null; while ($live.queue.TryDequeue([ref]$event)) { $seen.Add($event) }
        if (@($seen | Where-Object type -CEQ 'WatcherError').Count -gt 0) { throw 'Final native collector drain failed.' }
        $renames=@($seen | Where-Object type -CEQ 'Renamed')
        if ($renames.Count -ne 1000) { throw "Real collector expected 1000 renames, observed $($renames.Count)." }
        $unique=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        foreach ($rename in $renames) {
            if (-not $unique.Add($rename.old_path) -or -not $rename.old_path.StartsWith($fixture+'\',[StringComparison]::OrdinalIgnoreCase) -or
                -not $rename.path.StartsWith($fixture+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Duplicate or escaping fixture rename.' }
        }
        $evidence.Add([ordered]@{test='real concurrent atomic replacements, 100ms drains, final shutdown drain';result='PASS';renames=$renames.Count;expected=1000;events=$seen.Count})
    } finally {
        try {
            foreach ($capture in $writerCaptures) {
                if (-not $capture.process.HasExited) { Stop-CargoProcessTree $capture.process }
                foreach ($streamName in @('stdout','stderr')) {
                    $task=$capture.$streamName
                    $text=if ($task.Wait(5000)) { $task.GetAwaiter().GetResult() } else { 'CAPTURE_INCOMPLETE: stream did not close within five seconds.' }
                    [IO.File]::WriteAllText((Join-Path $Root ("writer-$($capture.worker).$streamName.log")),$text,[Text.UTF8Encoding]::new($false))
                }
            }
        } finally {
            try {
                foreach ($writer in $writers) {
                    try { if (-not $writer.HasExited) { Stop-CargoProcessTree $writer } }
                    finally { $writer.Dispose() }
                }
            } finally { Stop-WatchState $live }
        }
        $seen | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $Root 'collector-live-events.json') -Encoding utf8NoBOM
        $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $Root 'collector-tests.json') -Encoding utf8NoBOM
    }

    # A completed real rename across an authorization boundary is still refused.
    foreach ($case in @(
        @{name='allowed';source='allowed';destination='allowed';expected='ALLOWED'},
        @{name='outgoing';source='allowed';destination='outside';expected='VIOLATION'},
        @{name='protected';source='allowed';destination='protected';expected='VIOLATION'},
        @{name='temp-internal';source='user-temp';destination='user-temp';expected='UNATTRIBUTED_TEMP'},
        @{name='temp-outgoing';source='user-temp';destination='allowed';expected='VIOLATION'},
        @{name='temp-incoming';source='allowed';destination='user-temp';expected='VIOLATION'}
    )) {
        $boundaryRoot=Join-Path $Root ('boundary-'+$case.name)
        New-Item -ItemType Directory -Path $boundaryRoot -Force | Out-Null
        $parent=if ($case.name -eq 'temp-internal') { Join-Path $boundaryRoot 'user-temp' } else { $boundaryRoot }
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
        # Same-parent directory rename produces OLD/NEW on NTFS. Its names cross the policy roots.
        $source=Join-Path $parent 'old-tree'
        $destination=Join-Path $parent 'new-tree'
        New-Item -ItemType Directory -Path $source -Force | Out-Null
        $allowed=if ($case.name -eq 'allowed') { @($source,$destination) } elseif ($case.name -eq 'temp-outgoing') { @($destination) } else { @($source) }
        $protected=if ($case.name -eq 'protected') { @($destination) } else { @() }
        $temp=if ($case.name -eq 'temp-internal') { $parent } elseif ($case.name -eq 'temp-outgoing') { $source } elseif ($case.name -eq 'temp-incoming') { $destination } else { Join-Path $boundaryRoot 'unrelated-temp' }
        $watch=Start-NativeBuildWatchState @([pscustomobject]@{path=$boundaryRoot;recursive=$true}) $Root
        try {
            [IO.Directory]::Move($source,$destination)
            $matched=$null; $deadline=[DateTimeOffset]::UtcNow.AddSeconds(3)
            do {
                Receive-WatcherEvents $watch
                $event=$null
                while ($watch.queue.TryDequeue([ref]$event)) {
                    if ($event.type -eq 'WatcherError') { throw "Boundary collector error: $($event.detail)" }
                    if ($event.type -eq 'Renamed' -and $event.old_path -ieq $source -and $event.path -ieq $destination) { $matched=$event }
                }
                if (-not $matched) { Start-Sleep -Milliseconds 100 }
            } while (-not $matched -and [DateTimeOffset]::UtcNow -lt $deadline)
            if (-not $matched) { throw "Real boundary rename missing: $($case.name)" }
            $actual=Get-EventDisposition $matched $allowed $protected $temp
            if ($actual -cne $case.expected) { throw "Real boundary policy differs: $($case.name)=$actual" }
            $watch.queue.Enqueue($matched)
            if ($actual -eq 'VIOLATION') {
                Test-Rejects "real boundary still stops $($case.name)" { Drain-WatcherEvents $watch $allowed $protected $temp $null 'NativeBuild' $Root }
            } else { Drain-WatcherEvents $watch $allowed $protected $temp $null 'NativeBuild' $Root }
            $evidence.Add([ordered]@{test='real boundary '+$case.name;result='PASS';expected=$case.expected;event=$matched})
        } finally { Stop-WatchState $watch }
    }

    # Retain the distinct cross-parent DEL/ADD observation: it is not a Renamed guarantee.
    $crossRoot=Join-Path $Root 'cross-parent-notification-form'
    $crossAllowed=Join-Path $crossRoot 'allowed'; $crossTemp=Join-Path $crossRoot 'user-temp'
    New-Item -ItemType Directory -Path $crossAllowed,$crossTemp -Force | Out-Null
    $crossSource=Join-Path $crossTemp 'old.txt'; $crossDestination=Join-Path $crossAllowed 'new.txt'
    [IO.File]::WriteAllText($crossSource,'cross-parent observation')
    $cross=Start-NativeBuildWatchState @([pscustomobject]@{path=$crossRoot;recursive=$true}) $Root
    $crossEvents=[Collections.Generic.List[object]]::new()
    try {
        [IO.File]::Move($crossSource,$crossDestination)
        $until=[DateTimeOffset]::UtcNow.AddSeconds(1)
        do {
            Receive-WatcherEvents $cross
            $event=$null; while ($cross.queue.TryDequeue([ref]$event)) { $crossEvents.Add($event) }
            Start-Sleep -Milliseconds 100
        } while ([DateTimeOffset]::UtcNow -lt $until)
        Stop-WatchState $cross
        $event=$null; while ($cross.queue.TryDequeue([ref]$event)) { $crossEvents.Add($event) }
        if (@($crossEvents | Where-Object type -CEQ 'WatcherError').Count -gt 0) { throw 'Cross-parent notification probe lost coverage.' }
        $classified=@(foreach ($event in $crossEvents) {
            [ordered]@{event=$event;disposition=(Get-EventDisposition $event @($crossAllowed) @() $crossTemp)}
        })
        $evidence.Add([ordered]@{test='cross-parent notification form and existing classification';result='OBSERVED';events=$classified
            limitation='NTFS may report DEL+ADD. These are not inferred as a Renamed pair; no guarantee of stopping every cross-parent move is established.'})
    } finally { Stop-WatchState $cross }

    # Exercise the actual bounded queue without changing a production limit or adding a test hook.
    $emptyRoot=Join-Path $Root 'queue-capacity'
    New-Item -ItemType Directory -Path $emptyRoot -Force | Out-Null
    $capacity=[Whisper.NativeWatch.Collector]::new([string[]]@($emptyRoot),[bool[]]@($true))
    try {
        $type=$capacity.GetType(); $flags=[Reflection.BindingFlags]'Instance,NonPublic'
        $pending=$type.GetField('pending',$flags).GetValue($capacity)
        $maximum=[int]$type.GetField('MaximumQueuedRecords',[Reflection.BindingFlags]'Static,NonPublic').GetRawConstantValue()
        $dummy=New-TestRecord 3 'dummy'
        $pending.AddRange([Linq.Enumerable]::Repeat[Whisper.NativeWatch.Record]($dummy,$maximum))
        $null=$type.GetMethod('Publish',$flags).Invoke($capacity,[object[]]@(,[Whisper.NativeWatch.Record[]]@($dummy)))
        # Drop injected placeholders, leaving only the collector's real sticky failure.
        $pending.Clear()
        foreach ($pass in 1..2) {
            $snapshot=$capacity.Drain()
            if (@($snapshot.Events | Where-Object Type -CEQ 'WatcherError').Count -ne 1) { throw 'Native queue capacity failure was lost or recovered silently.' }
        }
        $evidence.Add([ordered]@{test='queue capacity fails and remains terminal';result='PASS';maximum=$maximum})
    } finally { $capacity.Dispose() }

    # Startup ownership failure: exercise the production arm/start/cleanup helper without an unsafe native fault.
    $readerType=[Whisper.NativeWatch.Collector].Assembly.GetType('Whisper.NativeWatch.DirectoryReader',$true)
    $lifecycle=[Collections.Generic.List[string]]::new()
    $arm=[Action]({ $lifecycle.Add('armed') }.GetNewClosure())
    $start=[Action]({ $lifecycle.Add('start-failed'); throw 'synthetic thread start failure' }.GetNewClosure())
    $cancel=[Action]({ $lifecycle.Add('cancelled-and-completed') }.GetNewClosure())
    Test-Rejects 'exception after arm must cancel and complete' {
        $readerType.GetMethod('StartArmedThread',[Reflection.BindingFlags]'Static,NonPublic').Invoke($null,[object[]]@($arm,$start,$cancel))
    }
    if (($lifecycle -join ',') -cne 'armed,start-failed,cancelled-and-completed') { throw 'Armed startup cleanup was skipped.' }
    $evidence.Add([ordered]@{test='startup failure ownership cleanup';result='PASS';sequence=$lifecycle.ToArray()})

    # Inject a throwing managed publication delegate on a real armed reader. The callback is compiled
    # as a CLR expression (no PowerShell invocation on the background reader thread).
    $publishRoot=Join-Path $Root 'publication-failure'
    New-Item -ItemType Directory -Path $publishRoot -Force | Out-Null
    $publication=[Whisper.NativeWatch.Collector]::new([string[]]@($publishRoot),[bool[]]@($true))
    try {
        $flags=[Reflection.BindingFlags]'Instance,NonPublic'
        $reader=@($publication.GetType().GetField('readers',$flags).GetValue($publication))[0]
        $parameter=[Linq.Expressions.Expression]::Parameter([Whisper.NativeWatch.Record[]],'records')
        $exception=[Linq.Expressions.Expression]::Constant([InvalidOperationException]::new('synthetic publication failure'))
        $body=[Linq.Expressions.Expression]::Throw($exception)
        $throwing=[Linq.Expressions.Expression]::Lambda[Action[Whisper.NativeWatch.Record[]]]($body,[Linq.Expressions.ParameterExpression[]]@($parameter)).Compile()
        $readerType.GetField('publish',$flags).SetValue($reader,$throwing)
        [IO.File]::WriteAllText((Join-Path $publishRoot 'trigger.txt'),'callback failure witness')
        $limit=[DateTimeOffset]::UtcNow.AddSeconds(3); $failed=$false
        do {
            $failureSnapshot=$publication.Drain()
            $failed=@($failureSnapshot.Events | Where-Object Type -CEQ 'WatcherError').Count -gt 0
            if (-not $failed) { Start-Sleep -Milliseconds 20 }
        } while (-not $failed -and [DateTimeOffset]::UtcNow -lt $limit)
        if (-not $failed) { throw 'Publication failure was not exposed as a terminal watcher error.' }
        $publication.Dispose()
        foreach ($field in @('memory','overlapped')) {
            if ($readerType.GetField($field,$flags).GetValue($reader) -ne [IntPtr]::Zero) { throw 'Publication error cleanup did not complete before releasing resources.' }
        }
        if ($readerType.GetField('pending',$flags).GetValue($reader)) { throw 'Publication failure left an uncompleted I/O.' }
        if (@($publication.Drain().Events | Where-Object Type -CEQ 'WatcherError').Count -ne 1) { throw 'Publication error did not remain terminal.' }
        $evidence.Add([ordered]@{test='real publication failure cancels/completes before release and stays terminal';result='PASS'})
    } finally { $publication.Dispose() }
    $evidence | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $Root 'collector-tests.json') -Encoding utf8NoBOM
    "PASS extended native collector: identity/packet/policy controls, all actual roots armed, 1000 real replacements; evidence=$Root"
}
