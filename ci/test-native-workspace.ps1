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
}
