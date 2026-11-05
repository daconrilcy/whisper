# Synthetic cache integrity checks only; no compiler, vcpkg, network or deletion.
[CmdletBinding()]
param([string]$EvidenceRoot)
$ErrorActionPreference='Stop'
$repo=Split-Path $PSScriptRoot
. (Join-Path $PSScriptRoot 'native-workspace.ps1')
. (Join-Path $PSScriptRoot 'native-cache.ps1')
if(-not $EvidenceRoot){$EvidenceRoot=Join-Path $repo ('target/native-build/cachetest-'+[guid]::NewGuid().ToString('N').Substring(0,8))}
$base=[IO.Path]::GetFullPath((Join-Path $repo 'target/native-build')).TrimEnd('\')
$EvidenceRoot=[IO.Path]::GetFullPath($EvidenceRoot).TrimEnd('\')
if(-not $EvidenceRoot.StartsWith($base+'\',[StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $EvidenceRoot)){throw 'Cache tests require a fresh native-build evidence root.'}
[void](Assert-NativeBinaryPath $EvidenceRoot $base)
if((Get-PSDrive C).Free -lt 30GB){throw 'Less than 30 GiB free.'}
New-Item -ItemType Directory -Path $EvidenceRoot | Out-Null
$checks=[Collections.Generic.List[string]]::new()
function Reject([string]$Name,[scriptblock]$Action,[string]$Reason){$errorText=$null;try{& $Action}catch{$errorText=$_.Exception.Message};if(-not $errorText -or $errorText -notlike $Reason){throw "Negative cache witness failed $Name : $errorText"};$checks.Add($Name)}
function Write-FixtureArchive([string]$Folder){
    $path=Join-Path $Folder 'a.zip'
    $zip=[IO.Compression.ZipFile]::Open($path,[IO.Compression.ZipArchiveMode]::Create)
    try{$entry=$zip.CreateEntry('include/package.h');$writer=[IO.StreamWriter]::new($entry.Open());try{$writer.Write('fixture headers')}finally{$writer.Dispose()}}finally{$zip.Dispose()}
    return $path
}
$inputs=[ordered]@{source='pinned';tool='1';triplet='static-md'};$key=Get-NativeBinaryKey $inputs
if($key -cne (Get-NativeBinaryKey @{triplet='static-md';tool='1';source='pinned'})){throw 'Key depends on input ordering.'};$checks.Add('canonical input key')
if($key -ceq (Get-NativeBinaryKey @{triplet='static-md';tool='2';source='pinned'})){throw 'Tool input did not change cache key.'};$checks.Add('tool change invalidates key')
$patchFile=Join-Path $EvidenceRoot 'local.patch';[IO.File]::WriteAllText($patchFile,'first')
$patchInputs=@{};foreach($name in $inputs.Keys){$patchInputs[$name]=$inputs[$name]};$patchInputs.patches=Get-NativePatchFingerprint @() @(@{name='local.patch';sha256=(Get-FileHash -LiteralPath $patchFile).Hash})
$patchKey=Get-NativeBinaryKey $patchInputs
[IO.File]::WriteAllText($patchFile,'second')
$patchInputs.patches=Get-NativePatchFingerprint @() @(@{name='local.patch';sha256=(Get-FileHash -LiteralPath $patchFile).Hash})
if($patchKey -ceq (Get-NativeBinaryKey $patchInputs)){throw 'Patch change did not invalidate cache.'};$checks.Add('patch byte change invalidates key')
Reject 'path escape' {Assert-NativeBinaryPath (Join-Path $EvidenceRoot '../outside') $EvidenceRoot} '*escaped*'
Reject 'invalid key' {Initialize-NativeBinaryCache $EvidenceRoot '../outside'} '*SHA-256 input key*'
$state=Initialize-NativeBinaryCache $EvidenceRoot $key
if($state.mode -cne 'POPULATE'){throw 'Fresh cache did not populate.'};$checks.Add('fresh generation')
Reject 'empty cache not sealed' {Complete-NativeBinaryCache $state $inputs} '*No binary packages*'
if(Test-Path -LiteralPath (Join-Path $state.root 'current.json')){throw 'Empty cache pointer published.'};$checks.Add('no pointer before verification')
$archive=Write-FixtureArchive $state.archives
$sealed=Complete-NativeBinaryCache $state $inputs
$reused=Initialize-NativeBinaryCache $EvidenceRoot $key
if($sealed.mode -cne 'REUSE' -or $reused.generation -cne $state.generation -or $reused.files.Count -ne 1 -or $reused.files[0].sha256 -cne (Get-FileHash -LiteralPath $archive).Hash){throw 'Verified cache not reused.'};$checks.Add('verified archives reused')
# Corruption must cause a fresh generation while keeping every old byte.
[IO.File]::AppendAllText($archive,'corrupt',[Text.UTF8Encoding]::new($false))
$corruptHash=(Get-FileHash -LiteralPath $archive).Hash
$rebuilt=Initialize-NativeBinaryCache $EvidenceRoot $key
if($rebuilt.mode -cne 'POPULATE' -or $rebuilt.generation -ceq $state.generation -or $rebuilt.rebuildReason -notlike '*digest or size differs*' -or (Get-FileHash -LiteralPath $archive).Hash -cne $corruptHash){throw 'Corrupt cache not preserved and rebuilt separately.'};$checks.Add('archive corruption rebuilds without deletion')
[void](Write-FixtureArchive $rebuilt.archives)
$sealed=Complete-NativeBinaryCache $rebuilt $inputs
if((Initialize-NativeBinaryCache $EvidenceRoot $key).generation -cne $rebuilt.generation){throw 'Fresh valid generation not selected.'};$checks.Add('replacement pointer atomic and reusable')
[IO.File]::AppendAllText($sealed.manifestPath,' ',[Text.UTF8Encoding]::new($false))
$new=Initialize-NativeBinaryCache $EvidenceRoot $key
if($new.mode -cne 'POPULATE' -or $new.rebuildReason -notlike '*manifest digest differs*'){throw 'Manifest corruption not detected.'};$checks.Add('manifest digest checked before usage')
[void](Write-FixtureArchive $new.archives);$sealed=Complete-NativeBinaryCache $new $inputs
$selectorPath=Join-Path $sealed.root 'current.json';$savedSelector=[IO.File]::ReadAllText($selectorPath)
$generationsBefore=@(Get-ChildItem -LiteralPath $sealed.root -Directory).Count
@{generation='../escape';manifestSha256=('A'*64)}|ConvertTo-Json|Set-Content -LiteralPath $selectorPath -Encoding utf8NoBOM
Reject 'selector path policy stops' {Initialize-NativeBinaryCache $EvidenceRoot $key} '*selector/generation policy*'
if(@(Get-ChildItem -LiteralPath $sealed.root -Directory).Count -ne $generationsBefore){throw 'Policy failure created a new generation.'};$checks.Add('policy failure does not rebuild')
[IO.File]::WriteAllText($selectorPath,$savedSelector,[Text.UTF8Encoding]::new($false))
$savedManifest=[IO.File]::ReadAllText($sealed.manifestPath)
$bad=Get-Content -LiteralPath $sealed.manifestPath -Raw|ConvertFrom-Json -AsHashtable;$bad.key='B'*64
$bad|ConvertTo-Json -Depth 8|Set-Content -LiteralPath $sealed.manifestPath -Encoding utf8NoBOM
@{generation=$sealed.generation;manifestSha256=(Get-FileHash -LiteralPath $sealed.manifestPath).Hash}|ConvertTo-Json|Set-Content -LiteralPath $selectorPath -Encoding utf8NoBOM
Reject 'provenance policy stops' {Initialize-NativeBinaryCache $EvidenceRoot $key} '*provenance policy*'
[IO.File]::WriteAllText($sealed.manifestPath,$savedManifest,[Text.UTF8Encoding]::new($false));[IO.File]::WriteAllText($selectorPath,$savedSelector,[Text.UTF8Encoding]::new($false))
$target=Join-Path $EvidenceRoot 'link-target';New-Item -ItemType Directory -Path $target | Out-Null
$link=Join-Path $sealed.archives 'linked';New-Item -ItemType Junction -Path $link -Target $target | Out-Null
Reject 'linked generation refused' {Initialize-NativeBinaryCache $EvidenceRoot $key} '*linked path*'
@{pass=$true;checks=$checks.ToArray();evidence=$EvidenceRoot;nativeCompilerStarted=$false;retainedJunction=$link;junctionTarget=$target} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $EvidenceRoot 'result.json') -Encoding utf8NoBOM
"NATIVE_CACHE_SELFTEST PASS checks=$($checks.Count) evidence=$EvidenceRoot compilerStarted=false"
