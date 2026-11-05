function Invoke-NativeMsvcTelemetryTests([string]$Root,[string]$Vcvars) {
    $ErrorActionPreference='Stop'
    $rootFull=[IO.Path]::GetFullPath($Root)
    $repoFull=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\')
    if (-not $rootFull.StartsWith($repoFull+'\target\native-build\',[StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $rootFull)) { throw 'Telemetry comparison requires a new native-build output directory.' }
    $cursor=$rootFull
    while ($cursor) {
        if ((Test-Path -LiteralPath $cursor) -and ((Get-Item -LiteralPath $cursor).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Telemetry comparison root crosses a reparse point.' }
        $parent=Split-Path -Parent $cursor; if ($parent -eq $cursor) { break }; $cursor=$parent
    }
    $receipts=@(foreach ($mode in @('without-switch','with-switch')) {
        $context=Join-Path $rootFull $mode
        New-Item -ItemType Directory -Path $context -Force | Out-Null
        $psi=[Diagnostics.ProcessStartInfo]::new($env:ComSpec)
        $psi.UseShellExecute=$false; $psi.CreateNoWindow=$true; $psi.WorkingDirectory=$context
        $psi.RedirectStandardOutput=$true; $psi.RedirectStandardError=$true
        $psi.Environment.Clear()
        foreach ($name in @('SystemRoot','WINDIR','SystemDrive','ComSpec','PATHEXT','OS','PROCESSOR_ARCHITECTURE','NUMBER_OF_PROCESSORS','ProgramFiles','ProgramFiles(x86)','ProgramW6432')) {
            $value=[Environment]::GetEnvironmentVariable($name,'Process')
            if ($value) { $psi.Environment[$name]=$value }
        }
        $psi.Environment['PATH']=@((Split-Path (Get-Command git.exe).Source),$PSHOME,(Join-Path $env:SystemRoot 'System32'),$env:SystemRoot) -join ';'
        foreach ($name in @('TEMP','TMP','LOCALAPPDATA','APPDATA','USERPROFILE','HOME','PROGRAMDATA','PUBLIC')) {
            $path=Join-Path $context $name.ToLowerInvariant()
            New-Item -ItemType Directory -Path $path | Out-Null
            $psi.Environment[$name]=$path
        }
        $psi.Environment.Remove('VSCMD_SKIP_SENDTELEMETRY') | Out-Null
        if ($mode -eq 'with-switch') { $psi.Environment['VSCMD_SKIP_SENDTELEMETRY']='1' }
        # Both contexts use Microsoft's diagnostic branch. It identifies and runs
        # the sender synchronously, avoiding a race with files already deleted.
        $psi.Environment['VSCMD_DEBUG']='1'
        $psi.Arguments='/d /s /c ""'+$Vcvars+'" x64 10.0.26100.0 -vcvars_ver=14.44 && set"'
        $process=[Diagnostics.Process]::new(); $process.StartInfo=$psi
        try {
            $null=$process.Start(); $stdout=$process.StandardOutput.ReadToEndAsync(); $stderr=$process.StandardError.ReadToEndAsync()
            if (-not $process.WaitForExit(30000)) { $process.Kill($true); throw 'MSVC telemetry comparison initialization timed out.' }
            $output=$stdout.GetAwaiter().GetResult(); $errors=$stderr.GetAwaiter().GetResult()
            if ($process.ExitCode -ne 0) { throw "MSVC preparation failed: $errors" }
            $toolVersions=@($output -split "`r?`n" | Where-Object { $_ -match '^(VCToolsVersion|WindowsSDKVersion)=' })
            if ($toolVersions.Count -ne 2) { throw 'MSVC comparison did not report the expected tools.' }
            $senderInvoked=$output -match '\[DEBUG:[^\]]+\] Sending telemetry'
            $telemetry=Join-Path $psi.Environment['LOCALAPPDATA'] 'Microsoft/VSApplicationInsights'
            $files=@(if (Test-Path -LiteralPath $telemetry) { Get-ChildItem -LiteralPath $telemetry -Recurse -File })
            [pscustomobject]@{mode=$mode;exitCode=$process.ExitCode;versions=$toolVersions;senderBranchExecuted=$senderInvoked;telemetryFiles=@($files.FullName);observedFiles=$files.Count;diagnosticMode='VSCMD_DEBUG=1 in both contexts, synchronous vendor sender branch'}
        } finally { $process.Dispose() }
    })
    $pass=$receipts[0].senderBranchExecuted -and -not $receipts[1].senderBranchExecuted -and (($receipts[0].versions -join '|') -ceq ($receipts[1].versions -join '|'))
    $result=[pscustomobject]@{status=$(if($pass){'PASS'}else{'FAIL'});contexts=$receipts;scope='two real MSVC preparations; no compilation; file observations do not attribute unrelated activity'}
    $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $rootFull 'result.json') -Encoding utf8NoBOM
    if (-not $pass) { throw "MSVC telemetry necessity witness failed; see $rootFull/result.json" }
    "PASS MSVC telemetry necessity witness; expected tools unchanged; evidence=$rootFull"
}

function Invoke-NativeJunctionTests([string]$Root) {
    $fixture=Join-Path $Root 'junction-tests'
    if (Test-Path -LiteralPath $fixture) { throw 'Junction fixture must be new.' }
    New-Item -ItemType Directory -Path $fixture -Force | Out-Null
    $checks=[Collections.Generic.List[object]]::new()
    $owner=[pscustomobject]@{product='Whisper NativeBuild';variant='fixture';buildId='test';guardPid=0}
    function New-CacheFixture([string]$Name) {
        $workspace=Join-Path $fixture "$Name/workspace"
        $output=Join-Path $fixture "$Name/output"
        $parent=Join-Path $workspace 'userprofile/AppData/Local/Microsoft/Windows/INetCache'
        $target=Join-Path $parent 'IE'; $link=Join-Path $parent 'Content.IE5'
        New-Item -ItemType Directory -Path $target,$output -Force | Out-Null
        $file=Join-Path $target 'keep.txt'; [IO.File]::WriteAllText($file,'preserve target')
        return [pscustomobject]@{workspace=$workspace;output=$output;parent=$parent;target=$target;link=$link;file=$file;hash=(Get-FileHash -LiteralPath $file).Hash}
    }
    $case=New-CacheFixture 'no-link'
    Remove-NativeCacheJunction $case.workspace $case.output $owner
    if (@(Get-ChildItem -LiteralPath $case.output -Force).Count) { throw 'Absent junction caused a mutation.' }
    $checks.Add(@{test='absent junction';result='PASS'})
    $case=New-CacheFixture 'positive'
    New-Item -ItemType Junction -Path $case.link -Target $case.target | Out-Null
    Remove-NativeCacheJunction $case.workspace $case.output $owner
    $receipt=Get-Content -LiteralPath (Join-Path $case.output 'junction-preparation.json') -Raw | ConvertFrom-Json
    if ((Test-Path -LiteralPath $case.link) -or (Get-FileHash -LiteralPath $case.file).Hash -cne $case.hash -or $receipt.status -cne 'REMOVED_TARGET_PRESERVED') { throw 'Junction removal did not preserve the target or receipt.' }
    $checks.Add(@{test='real exact junction, nonempty target';result='PASS'})
    foreach ($kind in @('wrong-internal-target','external-target','wrong-location','extra-link','ancestor-link','target-link')) {
        $case=New-CacheFixture $kind
        $other=Join-Path $fixture "$kind/other"
        New-Item -ItemType Directory -Path $other -Force | Out-Null
        if ($kind -eq 'ancestor-link') {
            New-Item -ItemType Junction -Path (Join-Path $case.workspace 'unexpected-parent') -Target $other | Out-Null
        } elseif ($kind -eq 'target-link') {
            # Target chain is represented by a sibling junction; all negative cases
            # must be rejected before mutating any link or any target content.
            New-Item -ItemType Junction -Path (Join-Path $case.parent 'other-target-link') -Target $other | Out-Null
            New-Item -ItemType Junction -Path $case.link -Target (Join-Path $case.parent 'other-target-link') | Out-Null
        } else {
            $destination=if ($kind -eq 'wrong-location') { Join-Path $case.parent 'wrong-name' } else { $case.link }
            $target=if ($kind -eq 'external-target') { $other } elseif ($kind -eq 'wrong-internal-target') { $case.parent } else { $case.target }
            New-Item -ItemType Junction -Path $destination -Target $target | Out-Null
            if ($kind -eq 'extra-link') { New-Item -ItemType Junction -Path (Join-Path $case.workspace 'extra') -Target $other | Out-Null }
        }
        $before=@(Get-NativePlainTree $case.workspace | Where-Object reparse | Select-Object -ExpandProperty path | Sort-Object)
        $caught=$false; try { Remove-NativeCacheJunction $case.workspace $case.output $owner } catch { $caught=$true }
        $after=@(Get-NativePlainTree $case.workspace | Where-Object reparse | Select-Object -ExpandProperty path | Sort-Object)
        if (-not $caught -or ($before -join '|') -cne ($after -join '|') -or (Get-FileHash -LiteralPath $case.file).Hash -cne $case.hash -or @(Get-ChildItem -LiteralPath $case.output -Force).Count) { throw "Junction negative control mutated the fixture: $kind" }
        $checks.Add(@{test=$kind;result='PASS'})
    }
    foreach ($kind in @('delete-failure','reappeared','post-remove-failure')) {
        $case=New-CacheFixture $kind
        New-Item -ItemType Junction -Path $case.link -Target $case.target | Out-Null
        $caught=& {
            function Remove-NativeDirectoryJunction([string]$Path) {
                if ($kind -eq 'delete-failure') { throw 'Synthetic removal failure.' }
                [IO.Directory]::Delete($Path,$false)
                if ($kind -eq 'reappeared') { New-Item -ItemType Junction -Path $Path -Target $case.target | Out-Null }
                else { throw 'Synthetic failure after unlink.' }
            }
            try { Remove-NativeCacheJunction $case.workspace $case.output $owner; $false } catch { $true }
        }
        $receipt=Get-Content -LiteralPath (Join-Path $case.output 'junction-preparation.json') -Raw | ConvertFrom-Json
        if (-not $caught -or $receipt.status -cne 'FAILED' -or (Get-FileHash -LiteralPath $case.file).Hash -cne $case.hash) { throw "Junction failure did not stop and retain the receipt: $kind" }
        if ($kind -in @('delete-failure','reappeared') -and -not (Test-Path -LiteralPath $case.link)) { throw 'Failure fixture unexpectedly lost its junction.' }
        $checks.Add(@{test=$kind;result='PASS'})
    }
    $checks.ToArray() | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $fixture 'result.json') -Encoding utf8NoBOM
    "PASS native junction tests: checks=$($checks.Count); fixture target content preserved."
}

function Invoke-NativeDirectoryMoveTests([string]$Root) {
    $fixture=Join-Path $Root 'directory-move-tests'
    if (Test-Path -LiteralPath $fixture) { throw 'Directory move fixture must be new.' }
    $source=Join-Path $fixture 'source'; $destination=Join-Path $fixture 'preserved'
    New-Item -ItemType Directory -Path $source -Force | Out-Null
    $file=Join-Path $source 'protected.txt'
    [IO.File]::WriteAllText($file,'preserve bytes and attributes')
    $attributes=[IO.FileAttributes]::Hidden -bor [IO.FileAttributes]::System -bor [IO.FileAttributes]::ReadOnly
    [IO.File]::SetAttributes($file,$attributes)
    $hash=(Get-FileHash -LiteralPath $file).Hash
    Move-NativeWorkspaceDirectory $source $destination
    $preserved=Join-Path $destination 'protected.txt'
    if ((Test-Path -LiteralPath $source) -or (Get-FileHash -LiteralPath $preserved).Hash -cne $hash -or (Get-Item -LiteralPath $preserved -Force).Attributes -ne $attributes) { throw 'Whole-directory preservation lost bytes or attributes.' }
    New-Item -ItemType Directory -Path $source | Out-Null
    [IO.File]::WriteAllText((Join-Path $source 'keep.txt'),'do not merge')
    $caught=$false; try { Move-NativeWorkspaceDirectory $source $destination } catch { $caught=$true }
    if (-not $caught -or -not (Test-Path -LiteralPath (Join-Path $source 'keep.txt')) -or (Test-Path -LiteralPath (Join-Path $destination 'keep.txt'))) { throw 'Existing destination was merged or overwritten.' }
    $caught=$false; try { Move-NativeWorkspaceDirectory $source '\\unreachable.invalid\share\preserved' } catch { $caught=$_.Exception.Message -match 'same volume' }
    if (-not $caught -or -not (Test-Path -LiteralPath $source)) { throw 'Cross-volume refusal did not precede mutation.' }
    $receipt=[pscustomobject]@{status='PASS';checks=3;preservedSha256=$hash;preservedAttributes=[string]$attributes;scope='whole-directory rename, existing-destination and cross-volume refusal; no real workspace change'}
    $receipt | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $fixture 'result.json') -Encoding utf8NoBOM
    'PASS native directory move tests: checks=3; hidden/system/read-only attributes and contents preserved.'
}

function Invoke-NativeWorkspaceTests([string]$Root) {
    $fixture=Join-Path $Root 'workspace-tests'
    New-Item -ItemType Directory -Path $fixture -Force | Out-Null
    $checks=0
    foreach ($variant in @('cpu','gpu')) {
        $null=Assert-NativeWorkspacePath ("C:/wb/native/$variant") $variant; $checks++
        foreach ($bad in @("C:/wb/native/$variant-other","C:/wb/native/$variant/child",'C:/outside')) {
            $rejected=$false; try { Assert-NativeWorkspacePath $bad $variant | Out-Null } catch { $rejected=$true }
            if (-not $rejected) { throw 'Workspace exact-path negative control failed.' }; $checks++
        }
    }
    # Minimal PE32+ fixture: exported RIP-relative constant getter, never loaded.
    $b=[byte[]]::new(2048)
    function Put16([int]$Offset,[uint16]$Value) { [BitConverter]::GetBytes($Value).CopyTo($b,$Offset) }
    function Put32([int]$Offset,[uint32]$Value) { [BitConverter]::GetBytes($Value).CopyTo($b,$Offset) }
    function PutText([int]$Offset,[string]$Value) { [Text.Encoding]::ASCII.GetBytes($Value).CopyTo($b,$Offset) }
    Put16 0 0x5a4d; Put32 60 128; Put32 128 0x4550; Put16 132 0x8664; Put16 134 1; Put16 148 240; Put16 152 0x20b
    Put32 264 4096
    Put32 404 4096; Put32 408 1536; Put32 412 512
    Put32 532 1; Put32 536 1; Put32 540 4160; Put32 544 4164; Put32 548 4168
    Put32 576 4224; Put32 580 4192; Put16 584 0; PutText 608 'ggml_commit'
    $b[640]=0x48; $b[641]=0x8d; $b[642]=0x05; Put32 643 57; $b[647]=0xc3
    PutText 704 $config.sources.llama.commit
    $pe=Join-Path $fixture 'constant.dll'; [IO.File]::WriteAllBytes($pe,$b)
    if ((Get-NativePeCommit $pe) -cne $config.sources.llama.commit) { throw 'PE constant getter positive fixture failed.' }; $checks++
    foreach ($change in @('opcode','rva','truncated')) {
        $bad=[byte[]]$b.Clone()
        if ($change -eq 'opcode') { $bad[640]=0x90 }
        elseif ($change -eq 'rva') { [BitConverter]::GetBytes([uint32]65535).CopyTo($bad,580) }
        else { $bad=$bad[0..150] }
        $path=Join-Path $fixture "$change.dll"; [IO.File]::WriteAllBytes($path,$bad)
        $rejected=$false; try { Get-NativePeCommit $path | Out-Null } catch { $rejected=$true }
        if (-not $rejected) { throw "PE getter negative fixture failed: $change" }; $checks++
    }
    [pscustomobject]@{checks=$checks;status='PASS';scope='exact roots and static PE decoding; no native build'} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $fixture 'result.json')
    "PASS native workspace tests: checks=$checks; no fixed-workspace write or native execution."
    Invoke-NativeJunctionTests $Root
    Invoke-NativeDirectoryMoveTests $Root
}
