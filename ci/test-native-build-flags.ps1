# Versioned contract fixtures; no dependency on disposable previous build outputs.
function Invoke-NativeBuildFlagTests([string]$Root,[switch]$InertOnly) {
    $run=Assert-Within (Join-Path $Root 'flag-tests') $Root
    New-Item -ItemType Directory -Path $run -Force | Out-Null
    $checks=[Collections.Generic.List[object]]::new()
    function Assert-FlagRejects([string]$Name, [scriptblock]$Action, [string]$Reason) {
        $caught=$null
        try { & $Action | Out-Null } catch { $caught=$_.Exception.Message }
        if (-not $caught -or $caught -notmatch $Reason) { throw "Flag negative control failed: $Name, received=$caught" }
        $checks.Add([pscustomobject]@{test=$Name;result='PASS';rejected=$caught})
    }
    function New-EffectiveFlagFixture {
        return @{
            CMAKE_C_FLAGS='/Brepro /DWIN32 /D_WINDOWS'; CMAKE_C_FLAGS_RELEASE='/O2 /Ob2 /DNDEBUG'
            CMAKE_CXX_FLAGS='/Brepro /DWIN32 /D_WINDOWS /EHsc'; CMAKE_CXX_FLAGS_RELEASE='/O2 /Ob2 /DNDEBUG'
            CMAKE_SHARED_LINKER_FLAGS='/Brepro /machine:x64'; CMAKE_SHARED_LINKER_FLAGS_RELEASE='/INCREMENTAL:NO'
            CMAKE_EXE_LINKER_FLAGS='/Brepro /machine:x64'; CMAKE_EXE_LINKER_FLAGS_RELEASE='/INCREMENTAL:NO'
            CMAKE_CUDA_FLAGS='-Xcompiler=/Brepro -D_WINDOWS -Xcompiler=" /EHsc"'
            CMAKE_CUDA_FLAGS_RELEASE='-Xcompiler="-O2 -Ob2" -DNDEBUG'
        }
    }
    foreach ($variant in @('cpu','gpu')) {
        $options=New-NativeBuildOptions $config.options $variant
        foreach ($name in @('C','CXX','SHARED_LINKER','EXE_LINKER')) {
            if ($options.ContainsKey("CMAKE_${name}_FLAGS") -or $options["CMAKE_${name}_FLAGS_INIT"] -cne '/Brepro') { throw 'Additive flag option witness differs.' }
        }
        if ($variant -eq 'gpu' -and ($options.ContainsKey('CMAKE_CUDA_FLAGS') -or $options.CMAKE_CUDA_FLAGS_INIT -cne '-Xcompiler=/Brepro')) { throw 'CUDA initializer replaces host defaults.' }
        $receipt=Assert-NativeEffectiveFlags (New-EffectiveFlagFixture) $variant
        if ($receipt.status -cne 'PASS' -or $receipt.variant -cne $variant) { throw 'Effective flag receipt differs.' }
        $checks.Add([pscustomobject]@{test="additive options and effective $variant fixture";result='PASS';receipt=$receipt})
    }
    foreach ($name in @('C','CXX','CUDA','SHARED_LINKER','EXE_LINKER')) {
        $bad=@{}+$config.options; $bad["CMAKE_${name}_FLAGS"]='/Brepro'
        Assert-FlagRejects "full $name override" { New-NativeBuildOptions $bad 'cpu' } 'Full flag override refused'
    }
    $bad=@{}+$config.options; $bad.CMAKE_CXX_FLAGS_RELEASE='/EHsc-'
    Assert-FlagRejects 'release override in requested config' { New-NativeBuildOptions $bad 'cpu' } 'Full flag override refused'
    $bad=@{}+$config.options; $bad.Remove('CMAKE_CXX_FLAGS_INIT')
    Assert-FlagRejects 'missing initializer' { New-NativeBuildOptions $bad 'cpu' } 'additive /Brepro initializer'
    foreach ($key in @('CMAKE_C_FLAGS','CMAKE_CXX_FLAGS','CMAKE_SHARED_LINKER_FLAGS','CMAKE_EXE_LINKER_FLAGS','CMAKE_CUDA_FLAGS')) {
        $variant=if ($key -eq 'CMAKE_CUDA_FLAGS') { 'gpu' } else { 'cpu' }
        $bad=New-EffectiveFlagFixture; $bad.Remove($key)
        Assert-FlagRejects "effective $key absent" { Assert-NativeEffectiveFlags $bad $variant } 'Effective flags missing'
        $bad=New-EffectiveFlagFixture; $bad[$key]=$bad[$key].Replace('/Brepro','/BreproExtra')
        Assert-FlagRejects "partial Brepro token $key" { Assert-NativeEffectiveFlags $bad $variant } 'Required effective native flag missing'
    }
    # The configuration observed in cpu4/cpu5 is an explicit versioned negative witness.
    $bad=New-EffectiveFlagFixture; $bad.CMAKE_CXX_FLAGS='/Brepro'
    Assert-FlagRejects 'old flags replacing platform defaults' { Assert-NativeEffectiveFlags $bad 'cpu' } 'Required effective native flag missing'
    foreach ($extra in @('/EHsc-','/EHa','/EHs-','/EHc-','/EHscExtra','/Brepro-','/U_WINDOWS')) {
        $bad=New-EffectiveFlagFixture; $bad.CMAKE_CXX_FLAGS_RELEASE+=' '+$extra
        Assert-FlagRejects "contradicting release flag $extra" { Assert-NativeEffectiveFlags $bad 'cpu' } 'Conflicting effective native flag'
    }
    $bad=New-EffectiveFlagFixture; $bad.CMAKE_CXX_FLAGS=$bad.CMAKE_CXX_FLAGS.Replace('/EHsc','')
    Assert-FlagRejects 'missing C++ exception model' { Assert-NativeEffectiveFlags $bad 'cpu' } 'Required effective native flag missing'
    $bad=New-EffectiveFlagFixture; $bad.CMAKE_CUDA_FLAGS=$bad.CMAKE_CUDA_FLAGS.Replace('/EHsc','')
    Assert-FlagRejects 'missing CUDA host exception model' { Assert-NativeEffectiveFlags $bad 'gpu' } 'Required effective native flag missing'
    foreach($required in @('/Brepro','-D_WINDOWS')){
        $bad=New-EffectiveFlagFixture;$bad.CMAKE_CUDA_FLAGS=$bad.CMAKE_CUDA_FLAGS.Replace($required,'')
        Assert-FlagRejects "missing CUDA host flag $required" {Assert-NativeEffectiveFlags $bad 'gpu'} 'Required effective native flag missing'
    }
    foreach($extra in @('/EHsc-','/EHa','/Brepro-','/U_WINDOWS')){
        $bad=New-EffectiveFlagFixture;$bad.CMAKE_CUDA_FLAGS_RELEASE+=' '+$extra
        Assert-FlagRejects "contradicting CUDA host flag $extra" {Assert-NativeEffectiveFlags $bad 'gpu'} 'Conflicting effective native flag'
    }
    $bad=New-EffectiveFlagFixture; $bad.CMAKE_SHARED_LINKER_FLAGS_RELEASE+=' /machine:ARM64'
    Assert-FlagRejects 'linker architecture override' { Assert-NativeEffectiveFlags $bad 'cpu' } 'Conflicting effective native flag'
    Assert-NativeDiagnostic 'file.cpp: warning C4244: ordinary unrelated diagnostic'
    Assert-NativeDiagnostic 'the number C45300 is not the diagnostic'
    Assert-FlagRejects 'localized C4530' { Assert-NativeDiagnostic 'fichier.cpp: avertissement C4530 : test' } 'C4530 detected'

    $metadata=@('set(CMAKE_CUDA_PLATFORM_ID "Windows")','set(CMAKE_CUDA_COMPILER_ARCHITECTURE_ID "x64")','set(CMAKE_CUDA_SIZEOF_DATA_PTR "8")')
    $metadataPath=Join-Path $run 'CMakeCUDACompiler.cmake'
    $metadata|Set-Content -LiteralPath $metadataPath -Encoding utf8
    $metadataReceipt=Read-NativeCudaPlatformMetadata $metadataPath
    if($metadataReceipt.values.CMAKE_CUDA_PLATFORM_ID -cne 'Windows' -or $metadataReceipt.values.CMAKE_CUDA_COMPILER_ARCHITECTURE_ID -cne 'x64' -or $metadataReceipt.values.CMAKE_CUDA_SIZEOF_DATA_PTR -cne '8'){throw 'CUDA metadata positive fixture differs.'}
    $checks.Add([pscustomobject]@{test='pinned generated CUDA Windows/x64/pointer8 metadata';result='PASS';receipt=$metadataReceipt})
    for($index=0;$index -lt $metadata.Count;$index++){
        foreach($mutation in @('missing','duplicate','wrong','malformed')){
            $changed=[string[]]$metadata.Clone()
            if($mutation -eq 'missing'){$changed=@($changed|Where-Object {$_ -cne $metadata[$index]})}
            elseif($mutation -eq 'duplicate'){$changed+=($metadata[$index])}
            elseif($mutation -eq 'wrong'){$changed[$index]=$changed[$index].Replace('Windows','Linux').Replace('x64','ARM64').Replace('"8"','"4"')}
            else{$changed[$index]=$changed[$index].TrimEnd(')')}
            $changed|Set-Content -LiteralPath $metadataPath -Encoding utf8
            Assert-FlagRejects "CUDA metadata $mutation field $index" {Read-NativeCudaPlatformMetadata $metadataPath} 'CUDA compiler metadata (missing or duplicate|differs|malformed)'
        }
    }
    Assert-FlagRejects 'CUDA metadata absent file' {Read-NativeCudaPlatformMetadata (Join-Path $run 'absent.cmake')} 'CUDA compiler metadata file is missing'
    $metadata|Set-Content -LiteralPath $metadataPath -Encoding utf8
    if($InertOnly){
        $checks.ToArray()|ConvertTo-Json -Depth 12|Set-Content -LiteralPath (Join-Path $run 'flag-inert-tests.json') -Encoding utf8NoBOM
        "PASS inert native flag/metadata tests: checks=$($checks.Count); no child process or native compiler. evidence=$run"
        return
    }

    # Exercise the actual streaming pipeline with owned PowerShell emitters, no native compiler.
    $saved=@{}
    foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME','NATIVE_BUILD_FLAGS_TEST_ROOT')) { $saved[$name]=[Environment]::GetEnvironmentVariable($name,'Process') }
    try {
        foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME')) {
            $path=Join-Path $run ('env/'+$name.ToLowerInvariant())
            New-Item -ItemType Directory -Path $path -Force | Out-Null
            [Environment]::SetEnvironmentVariable($name,$path,'Process')
        }
        $shell=Join-Path $PSHOME 'pwsh.exe'
        Invoke-Logged $shell @('-NoLogo','-NoProfile','-NonInteractive','-Command','[Console]::WriteLine("ordinary C4244"); exit 0') 'flag-positive'
        Assert-FlagRejects 'ordinary failed command' { Invoke-Logged $shell @('-NoLogo','-NoProfile','-NonInteractive','-Command','exit 7') 'flag-exit' } 'failed \(exit 7\)'
        foreach ($stream in @('stdout','stderr','stderr-child','parent-exits')) {
            $fixture=Join-Path $run $stream
            New-Item -ItemType Directory -Path $fixture -Force | Out-Null
            $env:NATIVE_BUILD_FLAGS_TEST_ROOT=$fixture
            $command='$ErrorActionPreference="Stop"; [IO.File]::WriteAllText([IO.Path]::Combine($env:NATIVE_BUILD_FLAGS_TEST_ROOT,"pid.txt"),[string]$PID); '
            if ($stream -in @('stderr-child','parent-exits')) {
                $grandchild='Start-Sleep -Seconds 10; [IO.File]::WriteAllText([IO.Path]::Combine($env:NATIVE_BUILD_FLAGS_TEST_ROOT,"child-must-not-finish.txt"),"not interrupted")'
                $grandchildEncoded=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($grandchild))
                $command+='$psi=[Diagnostics.ProcessStartInfo]::new([IO.Path]::Combine($PSHOME,"pwsh.exe")); $psi.UseShellExecute=$false; $psi.CreateNoWindow=$true; foreach($a in @("-NoLogo","-NoProfile","-NonInteractive","-EncodedCommand","'+$grandchildEncoded+'")){ $psi.ArgumentList.Add($a) }; $p=[Diagnostics.Process]::Start($psi); [IO.File]::WriteAllText([IO.Path]::Combine($env:NATIVE_BUILD_FLAGS_TEST_ROOT,"child-pid.txt"),[string]$p.Id); $p.Dispose(); '
            }
            if ($stream -eq 'parent-exits') { $command+='[Console]::WriteLine("fixture parent leaving"); exit 0' }
            else {
                $command+=if ($stream.StartsWith('stderr')) { '[Console]::Error.WriteLine("fixture: warning C4530: missing unwinding"); ' } else { '[Console]::WriteLine("fixture: warning C4530: missing unwinding"); ' }
                $command+='Start-Sleep -Seconds 10; [IO.File]::WriteAllText([IO.Path]::Combine($env:NATIVE_BUILD_FLAGS_TEST_ROOT,"must-not-finish.txt"),"not interrupted")'
            }
            $encoded=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($command))
            $watch=[Diagnostics.Stopwatch]::StartNew(); $emitterPid=0
            try {
                $reason=if ($stream -eq 'parent-exits') { 'observed descendant remained alive' } else { 'C4530 detected' }
                Assert-FlagRejects "owned process stop $stream" { Invoke-Logged $shell @('-NoLogo','-NoProfile','-NonInteractive','-EncodedCommand',$encoded) "flag-$stream" } $reason
                $emitterPid=[int](Get-Content -LiteralPath (Join-Path $fixture 'pid.txt') -Raw)
                $deadline=[DateTimeOffset]::UtcNow.AddSeconds(3)
                while ((Get-Process -Id $emitterPid -ErrorAction SilentlyContinue) -and [DateTimeOffset]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 20 }
                if ((Get-Process -Id $emitterPid -ErrorAction SilentlyContinue) -or $watch.Elapsed.TotalSeconds -ge 8 -or (Test-Path (Join-Path $fixture 'must-not-finish.txt'))) { throw 'C4530 did not promptly stop its owned emitter.' }
                if ($stream -in @('stderr-child','parent-exits')) {
                    $childPid=[int](Get-Content -LiteralPath (Join-Path $fixture 'child-pid.txt') -Raw)
                    if ((Get-Process -Id $childPid -ErrorAction SilentlyContinue) -or (Test-Path (Join-Path $fixture 'child-must-not-finish.txt'))) { throw 'C4530 did not stop the owned emitter descendant.' }
                }
                $expected=if ($stream -eq 'parent-exits') { 'fixture parent leaving' } else { 'C4530' }
                if ((Get-Content -LiteralPath (Join-Path $run "flag-$stream.log") -Raw) -notmatch $expected) { throw 'Blocking command output was not retained in its log.' }
                $checks.Add([pscustomobject]@{test="streaming stops owned $stream emitter";result='PASS';milliseconds=$watch.ElapsedMilliseconds})
            } finally {
                if ($emitterPid -gt 0) {
                    if (Test-Path -LiteralPath (Join-Path $fixture 'child-pid.txt')) {
                        $childPid=[int](Get-Content -LiteralPath (Join-Path $fixture 'child-pid.txt') -Raw)
                        $leftChild=Get-CimInstance Win32_Process -Filter "ProcessId=$childPid" -ErrorAction SilentlyContinue
                        if ($leftChild -and $leftChild.ParentProcessId -eq $emitterPid -and $leftChild.ExecutablePath -ieq $shell) { Stop-Process -Id $childPid -Force }
                    }
                    $left=Get-CimInstance Win32_Process -Filter "ProcessId=$emitterPid" -ErrorAction SilentlyContinue
                    if ($left -and $left.ParentProcessId -eq $PID -and $left.ExecutablePath -ieq $shell) { Stop-Process -Id $emitterPid -Force }
                }
            }
        }
        $checks.Add([pscustomobject]@{test='unrelated warning and positive command';result='PASS'})
    } finally {
        foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name,$saved[$name],'Process') }
        $checks.ToArray() | ConvertTo-Json -Depth 9 | Set-Content -LiteralPath (Join-Path $run 'flag-tests.json') -Encoding utf8NoBOM
    }
    "PASS native flag tests: checks=$($checks.Count); synthetic CPU/GPU cache, conflicting flags, streamed C4530; no native build. evidence=$run"
}
