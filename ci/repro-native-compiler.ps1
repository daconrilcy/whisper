# Bounded single-translation-unit reproduction; never changes the native guard.
[CmdletBinding()]
param(
    [ValidateSet('Environment','Essai0','Preprocess','A','B1','B2')][string]$Stage,
    [switch]$LibraryOnly
)
$ErrorActionPreference='Stop'
$PSNativeCommandUseErrorActionPreference=$false
$script:Repo=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\')
$script:Lot=Join-Path $script:Repo 'target/build-natif-c1001'
$script:Utf8NoBom=[Text.UTF8Encoding]::new($false)
$script:End=[datetimeoffset]::Parse('2026-11-05T21:54:46Z')
$script:Begin=[datetimeoffset]::Parse('2026-11-05T16:54:46Z')
$script:OutputRoot='C:\wb\c1001'
$script:SourceRoot='C:\wb\native\cpu'
$script:GuardCi=$PSScriptRoot
$guardAst=[Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'guard-native.ps1'),[ref]$null,[ref]$null)
foreach($name in @('Get-FullPath','Test-InRoot','Assert-NoReparseChain','Register-Watch','Add-SplitTreeWatches','Get-RunWatchSpecs','Initialize-NativeDirectoryWatch','Start-NativeBuildWatchState','New-VoxPopuliProcessFamily','Assert-VoxPopuliCleanupDeadline','Update-VoxPopuliProcessFamily','Stop-VoxPopuliKnownProcesses','Stop-VoxPopuliProcessFamily','Close-VoxPopuliProcessFamily')){
    $definition=@($guardAst.EndBlock.Statements | Where-Object {$_ -is [Management.Automation.Language.FunctionDefinitionAst] -and $_.Name -ceq $name})
    if($definition.Count -ne 1){throw "Existing guard helper missing: $name"}
    . ([scriptblock]::Create($definition[0].Extent.Text.Replace("function $name(","function script:${name}(").Replace('$PSScriptRoot','$script:GuardCi')))
}
function Write-ReproRecord([string]$Event,$Data){
    [IO.File]::AppendAllText($script:Journal,((@{utc=[datetimeoffset]::UtcNow.ToString('o');event=$Event;data=$Data}|ConvertTo-Json -Depth 12 -Compress)+[Environment]::NewLine),$script:Utf8NoBom)
}
function Assert-ReproOutput([string]$Path){
    if(-not(Test-InRoot $Path $script:OutputRoot) -or (Get-FullPath $Path) -ieq $script:OutputRoot){throw 'Output must be a strict descendant of the authorized reproduction root.'}
    Assert-NoReparseChain $Path 'isolated output'
}
function Get-ReproSchedule([string]$Name){
    switch -CaseSensitive ($Name){
        'Environment' {return @{count=1;parallel=1;folder='environment'}}
        'Essai0' {return @{count=1;parallel=1;folder='essai-0'}}
        'Preprocess' {return @{count=1;parallel=1;folder='preprocessed'}}
        'A' {return @{count=100;parallel=1;folder='A'}}
        'B1' {return @{count=200;parallel=4;folder='B1'}}
        'B2' {return @{count=100;parallel=2;folder='B2'}}
        default {throw 'Unknown reproduction stage.'}
    }
}
function Get-ReproCommand {
    $doc=Get-Content -LiteralPath (Join-Path $script:Lot 'isolated-command.json') -Raw | ConvertFrom-Json
    $archive=Join-Path $script:Repo 'docs/architecture/raw/build-natif-cpu7c4-cpu8/cpu8-stop.zip'
    if($doc.archive -ine $archive -or (Get-FileHash -LiteralPath $archive).Hash -cne '78653F3F126463E322BF33874EC94826966FC61B7229F91D9672CDA693CBCF15'){throw 'CPU8 archive differs.'}
    $zip=[IO.Compression.ZipFile]::OpenRead($archive)
    try{
        $entry=$zip.GetEntry('protobuf/install-x64-windows-rel-out.log')
        if($null -eq $entry){throw 'Archived compilation log missing.'}
        $reader=[IO.StreamReader]::new($entry.Open())
        try{$text=$reader.ReadToEnd()}finally{$reader.Dispose()}
        $commands=@($text -split '\r?\n' | Where-Object {$_ -match '^"[^"\r\n]+cl\.exe"' -and $_ -match ' /FoCMakeFiles\\libprotobuf\.dir\\src\\google\\protobuf\\any_lite\.cc\.obj '})
        if($commands.Count -ne 1 -or $commands[0].Trim() -cne $doc.exactCommand){throw 'Command is not the unique archived command.'}
        $tokens=@([regex]::Matches($commands[0].Trim(),'"[^"]*"|\S+') | ForEach-Object {$_.Value.Trim('"')})
        if($tokens[0] -cne $doc.compiler -or (ConvertTo-Json -InputObject @($tokens|Select-Object -Skip 1) -Compress) -cne (ConvertTo-Json -InputObject @($doc.arguments) -Compress)){throw 'Parsed command arguments differ.'}
    }finally{$zip.Dispose()}
    foreach($p in @($doc.compiler,(Join-Path (Split-Path $doc.compiler) 'c2.dll'))){Assert-NoReparseChain $p 'pinned compiler'}
    if((Get-FileHash -LiteralPath $doc.compiler).Hash -cne 'FE251EF50A1545B1B0835EE17B1E785459712B38D79E45B5C1D3D28970A36619' -or (Get-FileHash -LiteralPath (Join-Path (Split-Path $doc.compiler) 'c2.dll')).Hash -cne 'B057A18570F712FFB535E227825A291706620F3337E5B13D896D6586FB6B3CEC'){throw 'Pinned MSVC binaries differ.'}
    foreach($source in $doc.sources){Assert-NoReparseChain $source.path 'compiler source';if((Get-FileHash -LiteralPath $source.path).Hash -cne $source.sha256){throw 'Compiler source differs.'}}
    return $doc
}
function Assert-ReproWer {
    $key=Get-Item -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\cl.exe'
    if($key.GetValue('DumpFolder') -cne 'C:\wb\c1001\dumps' -or $key.GetValue('DumpType') -ne 2 -or $key.GetValue('DumpCount') -ne 512){throw 'WER full dump configuration differs.'}
    if($key.GetValueKind('DumpFolder') -ne 'ExpandString' -or $key.GetValueKind('DumpType') -ne 'DWord' -or $key.GetValueKind('DumpCount') -ne 'DWord'){throw 'WER registry types differ.'}
    Assert-NoReparseChain 'C:\wb\c1001\dumps' 'WER dump root'
    if(-not(Test-Path -LiteralPath (Join-Path $script:Lot 'wer-verified.json'))){throw 'Non-admin WER verification absent.'}
    foreach($path in @('HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\AeDebug','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows NT\CurrentVersion\AeDebug')){
        if(Test-Path -LiteralPath $path){$key=Get-Item -LiteralPath $path;if($key.GetValue('Auto') -eq '1' -and $key.GetValue('Debugger')){throw 'Automatic debugger conflicts with WER capture.'}}
    }
    $ifeo='HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\cl.exe'
    if(Test-Path -LiteralPath $ifeo){if((Get-Item -LiteralPath $ifeo).GetValue('Debugger')){throw 'cl.exe IFEO debugger configured.'}}
}
function Get-ReproMemory {
    $m=Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory
    return @{committed=[uint64]$m.CommittedBytes;limit=[uint64]$m.CommitLimit;pct=(100.0*$m.CommittedBytes/$m.CommitLimit);paged=[uint64]$m.PoolPagedBytes;nonpaged=[uint64]$m.PoolNonpagedBytes}
}
function Assert-ReproPrecondition {
    if((Get-PSDrive C).Free -lt 30GB){throw 'Less than 30 GiB free.'}
    $memory=Get-ReproMemory
    $processes=@(Get-CimInstance Win32_Process | Where-Object {$_.Name -match '^(cargo|rustc|cl|link|ninja|cmake|vctip)\.exe$|^(whisper-|parakeet-|l04-)' } | Select-Object Name,ProcessId,ParentProcessId,CommandLine)
    $task=Get-ScheduledTask -TaskName MemWatch
    $info=Get-ScheduledTaskInfo -TaskName MemWatch
    $csv=Get-Item -LiteralPath C:/memwatch/memwatch.csv
    Write-ReproRecord 'PRECONDITION' @{memory=$memory;processes=$processes;taskState=[string]$task.State;taskResult=$info.LastTaskResult;lastSampleUtc=$csv.LastWriteTimeUtc.ToString('o');freeBytes=(Get-PSDrive C).Free}
    if($memory.pct -gt 50 -or $processes.Count -or $task.State -eq 'Disabled' -or $info.LastTaskResult -ne 0 -or ([datetime]::UtcNow-$csv.LastWriteTimeUtc).TotalMinutes -gt 15){throw 'Reproduction precondition failed; no compiler launched.'}
}
function Get-ReproLevel {
    if([datetimeoffset]::UtcNow -ge $script:End){throw 'Lot deadline reached.'}
    try{$w=@(Get-WinEvent -FilterHashtable @{LogName='System';ProviderName='Microsoft-Windows-WHEA-Logger';StartTime=$script:Begin.LocalDateTime} -ErrorAction Stop)}catch{if($_.FullyQualifiedErrorId -like 'NoMatchingEventsFound*'){$w=@()}else{throw}}
    if($w.Count){Write-ReproRecord 'WHEA' $w;throw 'MATERIEL A REVERIFIER'}
    if($script:Monitor.HasExited -or $script:Monitor.StartTime.ToUniversalTime() -ne $script:MonitorCreated){throw 'MemWatch process identity lost.'}
    if((Get-FileHash -LiteralPath C:/memwatch/MemWatch.ps1).Hash -cne $script:MonitorHash){throw 'MemWatch changed.'}
    foreach($file in @($script:LevelFile,$script:MonitorCsv)){if(-not(Test-Path -LiteralPath $file) -or ([datetime]::UtcNow-(Get-Item -LiteralPath $file).LastWriteTimeUtc).TotalSeconds -gt 20){throw 'MemWatch sample missing or stale.'}}
    $stream=[IO.File]::Open($script:LevelFile,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::ReadWrite)
    try{$reader=[IO.StreamReader]::new($stream);try{$level=$reader.ReadToEnd().Trim().Trim([char]0xfeff)}finally{$reader.Dispose()}}finally{$stream.Dispose()}
    if($level -notin @('OK','ALERTE','STOP-LANCEMENTS','INTERROMPRE')){throw 'Invalid MemWatch level.'}
    if($level -eq 'ALERTE'){Write-ReproRecord 'ALERTE' @{csv=$script:MonitorCsv}}
    if($level -eq 'INTERROMPRE'){
        Write-ReproRecord 'INTERROMPRE' @{largest=@(Get-CimInstance Win32_Process | Sort-Object PrivatePageCount -Descending | Select-Object -First 15 Name,ProcessId,PrivatePageCount);most=@(Get-Process | Group-Object Name | Sort-Object Count -Descending | Select-Object -First 15 Name,Count)}
        throw 'INTERROMPRE: commit >=90%'
    }
    return $level
}
function Drain-ReproWatch {
    $violation=$script:Watch.nativeControl.Process($script:Watch.nativeCollector.Drain())
    if($null -ne $violation){throw "Existing NativeBuild policy refused $($violation.Type) at $($violation.Path)"}
}
function Get-ReproGitState {
    $states=@()
    foreach($repo in @($script:Repo,'C:\dev\whisper')){
        $head=& git --no-optional-locks -C $repo rev-parse HEAD;if($LASTEXITCODE){throw 'Git HEAD read failed.'}
        $status=& git --no-optional-locks -C $repo status --porcelain=v1 --untracked-files=all;if($LASTEXITCODE){throw 'Git status read failed.'}
        $states+=@{repo=$repo;head=$head;status=@($status)}
    }
    return ConvertTo-Json -InputObject $states -Compress -Depth 4
}
function New-ReproEnvironment {
    $environment=[Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach($name in @('SystemRoot','WINDIR','SystemDrive','ComSpec','PATHEXT','OS','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS','ProgramFiles','ProgramFiles(x86)','ProgramW6432')){
        $value=[Environment]::GetEnvironmentVariable($name);if($value){$environment[$name]=$value}
    }
    $environment['PATH']="$env:SystemRoot\System32;$env:SystemRoot"
    $environment['VSCMD_SKIP_SENDTELEMETRY']='1'
    return ,$environment
}
function Start-ReproProcess([string]$Executable,[string[]]$Arguments,[string]$Directory,$Environment,[int]$Index){
    $level=Get-ReproLevel
    if($level -eq 'STOP-LANCEMENTS'){return $null}
    if((Get-PSDrive C).Free -lt 30GB){throw 'Less than 30 GiB free.'}
    Assert-ReproOutput $Directory
    if(Test-Path -LiteralPath $Directory){throw "Fresh output already exists: $Directory"}
    New-Item -ItemType Directory -Path (Join-Path $Directory 'CMakeFiles/libprotobuf.dir/src/google/protobuf') | Out-Null
    $psi=[Diagnostics.ProcessStartInfo]::new($Executable);$psi.UseShellExecute=$false;$psi.CreateNoWindow=$true;$psi.RedirectStandardOutput=$true;$psi.RedirectStandardError=$true;$psi.WorkingDirectory=$Directory
    $psi.Environment.Clear();foreach($entry in $Environment.GetEnumerator()){$psi.Environment[$entry.Key]=[string]$entry.Value}
    foreach($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','HOME','USERPROFILE')){$p=Join-Path $Directory $name;New-Item -ItemType Directory -Path $p | Out-Null;$psi.Environment[$name]=$p}
    if($Stage -eq 'Environment'){$psi.Arguments=$Arguments -join ' '}
    else{foreach($arg in $Arguments){$psi.ArgumentList.Add($arg)}}
    $p=[Diagnostics.Process]::new();$p.StartInfo=$psi
    $entry=[pscustomobject]@{index=$Index;directory=$Directory;process=$p;family=$null;stdout=$null;stderr=$null;startedUtc=[datetimeoffset]::UtcNow;finished=$false;exit=$null}
    $started=$false
    try{
        $journalEnvironment=[ordered]@{}
        foreach($name in $psi.Environment.Keys){$journalEnvironment[$name]=[string]$psi.Environment[$name]}
        Write-ReproRecord 'LAUNCH' @{index=$Index;directory=$Directory;executable=$Executable;arguments=$Arguments;level=$level;environment=$journalEnvironment}
        if(-not $p.Start()){throw 'Process start failed.'}
        $started=$true
        $entry.family=New-VoxPopuliProcessFamily $p
        $entry.stdout=$p.StandardOutput.ReadToEndAsync();$entry.stderr=$p.StandardError.ReadToEndAsync()
        return $entry
    }catch{if($started -and -not $p.HasExited){$p.Kill($true);$p.WaitForExit(10000)|Out-Null};$p.Dispose();throw}
}
function Complete-ReproProcess($Entry,[switch]$Interrupted){
    if($Entry.finished){return}
    $errors=[Collections.Generic.List[string]]::new()
    # Interrupted work is not a compiler crash. Never wait for its WER dump.
    if(-not $Interrupted){
        try{
            if(-not $Entry.process.HasExited){throw 'Normal completion requires an exited process.'}
            $Entry.process.WaitForExit();$Entry.exit=$Entry.process.ExitCode
            if($Entry.exit -ne 0 -and $Stage -ne 'Environment'){Wait-ReproDump $Entry}
        }catch{$errors.Add($_.Exception.Message)}
    }
    try{Stop-VoxPopuliProcessFamily $Entry.family}catch{$errors.Add('Owned stop: '+$_.Exception.Message)}
    try{if($Entry.process.HasExited){$Entry.exit=$Entry.process.ExitCode}else{$errors.Add('Owned process survived closure.')}}catch{$errors.Add($_.Exception.Message)}
    foreach($stream in @(@{task=$Entry.stdout;name='stdout.log'},@{task=$Entry.stderr;name='stderr.log'})){
        try{
            $text=if($null -ne $stream.task -and $stream.task.Wait(10000)){$stream.task.GetAwaiter().GetResult()}else{'CAPTURE_INCOMPLETE: stream did not close.'}
            [IO.File]::WriteAllText((Join-Path $Entry.directory $stream.name),$text,$script:Utf8NoBom)
            if($text -like 'CAPTURE_INCOMPLETE:*'){$errors.Add($text)}
        }catch{$errors.Add('Capture '+$stream.name+': '+$_.Exception.Message)}
    }
    $object=Join-Path $Entry.directory 'CMakeFiles/libprotobuf.dir/src/google/protobuf/any_lite.cc.obj'
    try{Write-ReproRecord 'FINISH' @{index=$Entry.index;directory=$Entry.directory;pid=$Entry.process.Id;startedUtc=$Entry.startedUtc.ToString('o');exit=$Entry.exit;interrupted=[bool]$Interrupted;errors=$errors.ToArray();objectPresent=(Test-Path -LiteralPath $object);objectSha256=if(Test-Path -LiteralPath $object){(Get-FileHash -LiteralPath $object).Hash}else{$null}}}finally{$Entry.finished=$true}
    if($errors.Count){throw ($errors -join '; ')}
}
function Test-ReproDumpName([string]$Name,[int]$CompilerPid){
    return $Name -match ('^cl\.exe\.'+[regex]::Escape([string]$CompilerPid)+'(?:\..+)?\.dmp$')
}
function Assert-ReproPreprocessed {
    $receipt=Join-Path $script:Lot 'isolated-Preprocess/result.json'
    $artifact=Join-Path $script:OutputRoot 'preprocessed/any_lite.i'
    if(-not(Test-Path -LiteralPath $receipt -PathType Leaf)){throw 'Preprocessing receipt missing.'}
    $doc=Get-Content -LiteralPath $receipt -Raw | ConvertFrom-Json
    Assert-NoReparseChain $artifact 'preprocessed source'
    if(-not $doc.pass -or -not(Test-Path -LiteralPath $artifact -PathType Leaf) -or (Get-Item -LiteralPath $artifact).Length -eq 0 -or $doc.preprocessedSha256 -cne (Get-FileHash -LiteralPath $artifact).Hash){throw 'Preprocessing not completed or artifact altered.'}
}
function Wait-ReproDump($Entry){
    # Do not kill an observed WerFault child while it is writing the compiler dump.
    $until=[datetimeoffset]::UtcNow.AddSeconds(120)
    while([datetimeoffset]::UtcNow -lt $until){
        Drain-ReproWatch;[void](Get-ReproLevel);Update-VoxPopuliProcessFamily $Entry.family
        $dumps=@(Get-ChildItem -LiteralPath 'C:\wb\c1001\dumps' -File -Filter 'cl.exe.*.dmp' | Where-Object {(Test-ReproDumpName $_.Name $Entry.process.Id) -and $_.LastWriteTimeUtc -ge $Entry.startedUtc.UtcDateTime})
        foreach($dump in $dumps){
            $handle=$null
            try{$handle=[IO.File]::Open($dump.FullName,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::None)}catch [IO.IOException]{continue}
            try{
                $reader=[IO.BinaryReader]::new($handle,[Text.Encoding]::UTF8,$true)
                if($handle.Length -lt 32 -or $reader.ReadUInt32() -ne 0x504d444d){throw 'Invalid compiler dump header.'}
                $handle.Position=24;$flags=$reader.ReadUInt64()
                if(($flags -band 2) -eq 0){throw 'Compiler dump does not include full memory.'}
                $handle.Position=0;$sha=[Security.Cryptography.SHA256]::HashData($handle)
                Write-ReproRecord 'CRASH_DUMP' @{pid=$Entry.process.Id;path=$dump.FullName;bytes=$handle.Length;flags=$flags;sha256=[Convert]::ToHexString($sha);exit=$Entry.exit;csv=$script:MonitorCsv}
                return
            }finally{if($null -ne $handle){$handle.Dispose()}}
        }
        Start-Sleep -Milliseconds 250
    }
    throw 'Compiler failure has no complete full-memory WER dump after 120 seconds; stop.'
}
function Invoke-ReproStage {
    $schedule=Get-ReproSchedule $Stage
    $script:StageRoot=Join-Path $script:Lot ('isolated-'+$Stage)
    Assert-NoReparseChain $script:StageRoot 'stage evidence'
    if(Test-Path -LiteralPath $script:StageRoot){throw 'Stage already attempted; no implicit retry.'}
    New-Item -ItemType Directory -Path $script:StageRoot | Out-Null
    $script:Journal=Join-Path $script:StageRoot 'journal.jsonl'
    $script:Monitor=$null;$script:Watch=$null;$entries=[Collections.Generic.List[object]]::new();$success=$false
    $monitorOut=$null;$monitorErr=$null;$monitorStarted=$false;$primaryFailure=$null
    $tag='C1001-'+$Stage;$script:LevelFile="C:\memwatch\niveau-$tag.txt";$stopFile="C:\memwatch\stop-$tag"
    try{
        $command=Get-ReproCommand;Assert-ReproWer;Assert-ReproPrecondition
        $beforeGit=Get-ReproGitState
        Write-ReproRecord 'GIT_BEFORE' $beforeGit
        foreach($path in @($script:OutputRoot,$script:SourceRoot)){Assert-NoReparseChain $path 'reproduction root'}
        if($Stage -ne 'Environment'){
            $envFile=Join-Path $script:Lot 'isolated-Environment/environment.json'
            $envDoc=Get-Content -LiteralPath $envFile -Raw | ConvertFrom-Json
            $receipt=Get-Content -LiteralPath (Join-Path $script:Lot 'isolated-Environment/result.json') -Raw | ConvertFrom-Json
            if(-not $receipt.pass -or $receipt.environmentSha256 -cne (Get-FileHash -LiteralPath $envFile).Hash){throw 'Reconstructed environment not qualified or altered.'}
            $environment=[Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
            foreach($property in $envDoc.values.PSObject.Properties){$environment[$property.Name]=[string]$property.Value}
        }else{$environment=New-ReproEnvironment}
        if($Stage -in @('Preprocess','A','B1','B2')){
            $prior=if($Stage -eq 'B1'){'A'}elseif($Stage -eq 'B2'){'B1'}else{'Essai0'}
            $receipt=Get-Content -LiteralPath (Join-Path $script:Lot "isolated-$prior/result.json") -Raw | ConvertFrom-Json
            if(-not $receipt.pass){throw 'Prior stage did not pass.'}
            if($Stage -eq 'B2' -and $receipt.failed -lt 1){throw 'B2 allowed only following B1 crashes.'}
        }
        if($Stage -in @('A','B1','B2')){Assert-ReproPreprocessed}
        foreach($path in @($script:LevelFile,$stopFile)){if(Test-Path -LiteralPath $path){throw 'MemWatch tag already used.'}}
        if(@(Get-ChildItem C:/memwatch -File -Filter "intensive-$tag-*.csv").Count){throw 'MemWatch tag CSV exists.'}
        $script:MonitorHash=(Get-FileHash -LiteralPath C:/memwatch/MemWatch.ps1).Hash
        $psi=[Diagnostics.ProcessStartInfo]::new((Get-Command pwsh.exe).Source);$psi.UseShellExecute=$false;$psi.CreateNoWindow=$true;$psi.RedirectStandardOutput=$true;$psi.RedirectStandardError=$true
        foreach($arg in @('-NoProfile','-NonInteractive','-File','C:\memwatch\MemWatch.ps1','-Mode','Intensive','-Tag',$tag,'-IntervalSeconds','5')){$psi.ArgumentList.Add($arg)}
        foreach($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','HOME','USERPROFILE')){$p=Join-Path $script:StageRoot "monitor-$name";New-Item -ItemType Directory -Path $p | Out-Null;$psi.Environment[$name]=$p}
        $script:Monitor=[Diagnostics.Process]::new();$script:Monitor.StartInfo=$psi
        Write-ReproRecord 'MEMWATCH_LAUNCH' @{tag=$tag;sha256=$script:MonitorHash}
        if(-not $script:Monitor.Start()){throw 'MemWatch launch failed.'};$monitorStarted=$true;$script:MonitorCreated=$script:Monitor.StartTime.ToUniversalTime()
        $monitorOut=$script:Monitor.StandardOutput.ReadToEndAsync();$monitorErr=$script:Monitor.StandardError.ReadToEndAsync()
        $wait=[datetimeoffset]::UtcNow.AddSeconds(20)
        while(-not(Test-Path -LiteralPath $script:LevelFile)){if($script:Monitor.HasExited -or [datetimeoffset]::UtcNow -ge $wait){throw 'MemWatch startup failed.'};Start-Sleep -Milliseconds 200}
        $csv=@(Get-ChildItem C:/memwatch -File -Filter "intensive-$tag-*.csv");if($csv.Count -ne 1){throw 'MemWatch CSV identity ambiguous.'};$script:MonitorCsv=$csv[0].FullName
        [void](Get-ReproLevel)
        $specs=[Collections.Generic.List[object]]::new();foreach($spec in @(Get-RunWatchSpecs $script:Lot)){$specs.Add($spec)}
        # Keep the guard's exclusion of its own evidence/journals: no recursive lot watch.
        Register-Watch $specs $script:OutputRoot $true;Register-Watch $specs (Split-Path $script:OutputRoot) $false;Register-Watch $specs $script:SourceRoot $true
        $script:Watch=Start-NativeBuildWatchState $specs $script:StageRoot
        $script:Watch | Add-Member -NotePropertyName nativeControl -NotePropertyValue ([Whisper.NativeBuild.Control]::new([string[]]@($script:Lot,$script:OutputRoot),[string[]]@((Join-Path $script:Repo '.git'),$script:SourceRoot,(Join-Path $env:LOCALAPPDATA 'Whisper'),'C:\dev\whisper\target\parakeet-feasibility','C:\dev\whisper\target\poll-preserved'),(Join-Path $env:LOCALAPPDATA 'Temp'),$script:StageRoot))
        $next=1
        while($next -le $schedule.count -or @($entries|Where-Object {-not $_.finished}).Count){
            Drain-ReproWatch;$level=Get-ReproLevel
            foreach($entry in @($entries | Where-Object {-not $_.finished})){
                Update-VoxPopuliProcessFamily $entry.family
                if(([datetimeoffset]::UtcNow-$entry.startedUtc).TotalMinutes -gt 10){throw 'Single compiler exceeded ten-minute bound.'}
                if($entry.process.HasExited){Complete-ReproProcess $entry;if($Stage -in @('Environment','Essai0','Preprocess') -and $entry.exit -ne 0){throw 'Preparation compiler/environment failed; no retry.'}}
            }
            if($next -le $schedule.count -and @($entries|Where-Object {-not $_.finished}).Count -lt $schedule.parallel -and $level -ne 'STOP-LANCEMENTS'){
                Assert-ReproWer
                $directory=Join-Path $script:OutputRoot $schedule.folder;if($schedule.count -gt 1){$directory=Join-Path $directory ([string]$next)}
                $exe=$command.compiler;$arguments=[string[]]$command.arguments
                if($Stage -eq 'Environment'){
                    $exe=Join-Path $env:SystemRoot 'System32/cmd.exe'
                    $vcvars='C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat'
                    Assert-NoReparseChain $vcvars 'pinned MSVC environment script'
                    $arguments=@('/d','/s','/c',('""'+$vcvars+'" x64 10.0.26100.0 -vcvars_ver=14.44 >nul && set"'))
                }elseif($Stage -eq 'Preprocess'){$arguments+=@('/P','/Fiany_lite.i')}
                $entry=Start-ReproProcess $exe $arguments $directory $environment $next
                if($null -ne $entry){$entries.Add($entry);$next++}
            }
            Start-Sleep -Milliseconds 100
        }
        $failed=@($entries | Where-Object exit -NE 0).Count
        if($Stage -eq 'Environment'){
            $values=[ordered]@{};foreach($line in ([IO.File]::ReadAllLines((Join-Path $entries[0].directory 'stdout.log')))){if($line -match '^([^=]+)=(.*)$'){$values[$matches[1]]=$matches[2]}}
            if($values.VCToolsVersion.TrimEnd('\') -cne '14.44.35207' -or $values.WindowsSDKVersion.TrimEnd('\') -cne '10.0.26100.0' -or $values.VSCMD_SKIP_SENDTELEMETRY -cne '1' -or -not $values.INCLUDE -or -not $values.LIB -or -not $values.PATH){throw 'Reconstructed MSVC environment differs.'}
            $envFile=Join-Path $script:StageRoot 'environment.json'
            @{status='RECONSTRUIT, NON ARCHIVE DE CPU8';provenance='Pinned vcvarsall x64 10.0.26100.0 -vcvars_ver=14.44, initially minimal Windows environment; not original vcpkg port environment';values=$values} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $envFile -Encoding utf8NoBOM
        }elseif($Stage -eq 'Preprocess'){if((Get-Item -LiteralPath (Join-Path $entries[0].directory 'any_lite.i')).Length -eq 0){throw 'Preprocessed output empty.'}}
        elseif($Stage -eq 'Essai0'){if((Get-Item -LiteralPath (Join-Path $entries[0].directory 'CMakeFiles/libprotobuf.dir/src/google/protobuf/any_lite.cc.obj')).Length -eq 0){throw 'Object empty.'}}
        $result=@{stage=$Stage;pass=$true;launched=$entries.Count;failed=$failed;successful=$entries.Count-$failed;parallel=$schedule.parallel;finishedUtc=[datetimeoffset]::UtcNow.ToString('o');monitorCsv=$script:MonitorCsv;preprocessedSha256=if($Stage -eq 'Preprocess'){(Get-FileHash -LiteralPath (Join-Path $entries[0].directory 'any_lite.i')).Hash}else{$null};environmentSha256=if($Stage -eq 'Environment'){(Get-FileHash -LiteralPath $envFile).Hash}else{(Get-FileHash -LiteralPath (Join-Path $script:Lot 'isolated-Environment/environment.json')).Hash}}
        $success=$true
    }catch{$primaryFailure=$_;Write-ReproRecord 'STOP' @{failure=$_.Exception.Message};throw}
    finally{
        $errors=[Collections.Generic.List[string]]::new()
        foreach($entry in $entries){try{if(-not $entry.finished){Complete-ReproProcess $entry -Interrupted}}catch{$errors.Add($_.Exception.Message)};try{Close-VoxPopuliProcessFamily $entry.family}catch{$errors.Add($_.Exception.Message)};try{$entry.process.Dispose()}catch{$errors.Add($_.Exception.Message)}}
        if($null -ne $script:Watch){try{$script:Watch.nativeCollector.Dispose()}catch{$errors.Add($_.Exception.Message)};try{Drain-ReproWatch}catch{$errors.Add($_.Exception.Message)};try{$script:Watch.nativeControl.Dispose()}catch{$errors.Add($_.Exception.Message)}}
        if($monitorStarted){try{if(-not $script:Monitor.HasExited){[IO.File]::WriteAllText($stopFile,'stop',$script:Utf8NoBom);if(-not $script:Monitor.WaitForExit(20000)){throw 'MemWatch did not stop.'}};foreach($stream in @(@{task=$monitorOut;name='memwatch.stdout.log'},@{task=$monitorErr;name='memwatch.stderr.log'})){if(-not $stream.task.Wait(10000)){throw 'MemWatch stream open.'};[IO.File]::WriteAllText((Join-Path $script:StageRoot $stream.name),$stream.task.GetAwaiter().GetResult(),$script:Utf8NoBom)};$script:Monitor.Dispose()}catch{$errors.Add($_.Exception.Message)}}
        Write-ReproRecord 'CLOSED' @{success=$success;cleanupErrors=$errors.ToArray()}
        if($errors.Count -and $null -eq $primaryFailure){throw "Cleanup failed: $($errors -join '; ')"}
    }
    $afterGit=Get-ReproGitState
    Write-ReproRecord 'GIT_AFTER' $afterGit
    if($beforeGit -cne $afterGit){throw 'Tracked/untracked candidate or main Git state changed during the stage.'}
    $result | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $script:StageRoot 'result.json') -Encoding utf8NoBOM
}
if($LibraryOnly){return}
if(-not $Stage){throw 'Explicit stage required.'}
Invoke-ReproStage
