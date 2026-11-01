param(
    [ValidateSet('Cpu','ParakeetCpu','Gpu','Desktop')]
    [string]$Kind,
    [string]$Path,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
$cudaPattern = '^(cublas(?:lt)?|cudart|nvcuda|cudnn|nvrtc|cufft|curand|cusparse).*\.dll$'

function Assert-Range([byte[]]$Bytes, [long]$Offset, [long]$Count) {
    if ($Offset -lt 0 -or $Count -lt 0 -or $Offset + $Count -gt $Bytes.LongLength) {
        throw "PE structure exceeds file bounds (offset=$Offset, count=$Count, size=$($Bytes.LongLength))."
    }
}

function Read-U16([byte[]]$Bytes, [long]$Offset) {
    Assert-Range $Bytes $Offset 2
    return [BitConverter]::ToUInt16($Bytes, [int]$Offset)
}

function Read-U32([byte[]]$Bytes, [long]$Offset) {
    Assert-Range $Bytes $Offset 4
    return [BitConverter]::ToUInt32($Bytes, [int]$Offset)
}

function Read-U64([byte[]]$Bytes, [long]$Offset) {
    Assert-Range $Bytes $Offset 8
    return [BitConverter]::ToUInt64($Bytes, [int]$Offset)
}

function Convert-RvaToOffset([byte[]]$Bytes, [object[]]$Sections, [long]$Rva) {
    foreach ($section in $Sections) {
        $span = [Math]::Max($section.VirtualSize, $section.RawSize)
        if ($Rva -ge $section.VirtualAddress -and $Rva -lt ($section.VirtualAddress + $span)) {
            $rawOffset = $section.RawPointer + ($Rva - $section.VirtualAddress)
            Assert-Range $Bytes $rawOffset 1
            return $rawOffset
        }
    }
    if ($Rva -lt $Bytes.LongLength) { return $Rva }
    throw "PE RVA 0x$('{0:x}' -f $Rva) does not map to file data."
}

function Get-PeImports([byte[]]$Bytes) {
    Assert-Range $Bytes 0 64
    if ((Read-U16 $Bytes 0) -ne 0x5a4d) { throw 'Not a PE file: missing MZ signature.' }
    $peOffset = [long](Read-U32 $Bytes 0x3c)
    Assert-Range $Bytes $peOffset 24
    if ((Read-U32 $Bytes $peOffset) -ne 0x00004550) { throw 'Not a PE file: missing PE signature.' }
    $sectionCount = Read-U16 $Bytes ($peOffset + 6)
    $optionalSize = Read-U16 $Bytes ($peOffset + 20)
    $optional = $peOffset + 24
    Assert-Range $Bytes $optional $optionalSize
    $magic = Read-U16 $Bytes $optional
    if ($magic -eq 0x20b) { $directoryOffset = 112; $countOffset = 108; $imageBase = [long](Read-U64 $Bytes ($optional + 24)) }
    elseif ($magic -eq 0x10b) { $directoryOffset = 96; $countOffset = 92; $imageBase = [long](Read-U32 $Bytes ($optional + 28)) }
    else { throw "Unsupported PE optional-header magic: 0x$('{0:x}' -f $magic)." }
    if ($optionalSize -lt ($directoryOffset + 14 * 8)) { throw 'PE optional header does not contain import directories.' }
    $directoryCount = Read-U32 $Bytes ($optional + $countOffset)
    $sectionTable = $optional + $optionalSize
    Assert-Range $Bytes $sectionTable ([long]$sectionCount * 40)
    $sections = @()
    for ($index = 0; $index -lt $sectionCount; $index++) {
        $offset = $sectionTable + ($index * 40)
        $sections += [pscustomobject]@{
            VirtualSize = [long](Read-U32 $Bytes ($offset + 8))
            VirtualAddress = [long](Read-U32 $Bytes ($offset + 12))
            RawSize = [long](Read-U32 $Bytes ($offset + 16))
            RawPointer = [long](Read-U32 $Bytes ($offset + 20))
        }
    }
    $result = [Collections.Generic.List[string]]::new()
    foreach ($directoryIndex in @(1,13)) {
        if ($directoryCount -le $directoryIndex) { continue }
        $directory = $optional + $directoryOffset + ($directoryIndex * 8)
        $tableRva = [long](Read-U32 $Bytes $directory)
        $tableSize = [long](Read-U32 $Bytes ($directory + 4))
        if ($tableRva -eq 0) { continue }
        $descriptorSize = if ($directoryIndex -eq 1) { 20 } else { 32 }
        if ($tableSize -le 0) { throw 'PE import directory has an RVA but no declared descriptor size.' }
        $maxDescriptors = [Math]::Min(4096, [int][Math]::Floor($tableSize / $descriptorSize))
        if ($maxDescriptors -le 0) { throw 'PE import directory is smaller than one descriptor.' }
        $terminated = $false
        for ($descriptorIndex = 0; $descriptorIndex -lt $maxDescriptors; $descriptorIndex++) {
            $descriptorRva = $tableRva + ($descriptorIndex * $descriptorSize)
            $descriptorOffset = Convert-RvaToOffset $Bytes $sections $descriptorRva
            Assert-Range $Bytes $descriptorOffset $descriptorSize
            if ($directoryIndex -eq 1) {
                $nameValue = [long](Read-U32 $Bytes ($descriptorOffset + 12))
                if ($nameValue -eq 0 -and (Read-U32 $Bytes $descriptorOffset) -eq 0 -and (Read-U32 $Bytes ($descriptorOffset + 16)) -eq 0) { $terminated = $true; break }
                $nameRva = $nameValue
            } else {
                $attributes = Read-U32 $Bytes $descriptorOffset
                $nameValue = [long](Read-U32 $Bytes ($descriptorOffset + 4))
                if ($nameValue -eq 0 -and (Read-U32 $Bytes ($descriptorOffset + 8)) -eq 0) { $terminated = $true; break }
                $nameRva = if (($attributes -band 1) -ne 0) { $nameValue } else { $nameValue - $imageBase }
            }
            if ($nameRva -le 0) { throw 'PE import descriptor has no valid DLL name RVA.' }
            $nameOffset = Convert-RvaToOffset $Bytes $sections $nameRva
            $end = $nameOffset
            while ($end -lt $Bytes.LongLength -and $Bytes[[int]$end] -ne 0) { $end++ }
            if ($end -ge $Bytes.LongLength) { throw 'Unterminated PE import DLL name.' }
            $name = [Text.Encoding]::ASCII.GetString($Bytes, [int]$nameOffset, [int]($end - $nameOffset))
            if ([string]::IsNullOrWhiteSpace($name)) { throw 'Empty PE import DLL name.' }
            $result.Add($name)
        }
        if (-not $terminated) {
            $tableName = if ($directoryIndex -eq 1) { 'direct' } else { 'delay' }
            throw "Unterminated $tableName PE import descriptor table within its declared size or safety cap."
        }
    }
    return @($result.ToArray())
}

function Test-Imports([string]$ArtifactKind, [string[]]$Imports) {
    $cuda = @($Imports | Where-Object { $_ -match $cudaPattern } | Sort-Object -Unique)
    if ($ArtifactKind -eq 'Gpu') {
        $missing = @()
        if (-not (@($Imports | Where-Object { $_ -match '^cudart.*\.dll$' }).Count)) { $missing += 'cudart*' }
        if (-not (@($Imports | Where-Object { $_ -match '^nvcuda.*\.dll$' }).Count)) { $missing += 'nvcuda*' }
        return [pscustomobject]@{ Passed = ($missing.Count -eq 0); Reason = if ($missing.Count) { 'Missing required CUDA imports: ' + ($missing -join ', ') } else { 'Required CUDA imports present.' }; Cuda = $cuda }
    }
    return [pscustomobject]@{ Passed = ($cuda.Count -eq 0); Reason = if ($cuda.Count) { 'Forbidden CUDA imports present.' } else { 'No forbidden CUDA imports.' }; Cuda = $cuda }
}

function Invoke-Check([string]$ArtifactKind, [string]$ArtifactPath, [string[]]$KnownImports = $null, [byte[]]$KnownBytes = $null) {
    $parseFailure = $null
    if ($null -eq $KnownImports) {
        $bytes = if ($null -ne $KnownBytes) { $KnownBytes } else { [IO.File]::ReadAllBytes($ArtifactPath) }
        try { $imports = @(Get-PeImports $bytes) }
        catch { $parseFailure = $_.Exception.Message; $imports = @() }
    } else { $imports = $KnownImports }
    $result = if ($parseFailure) { [pscustomobject]@{ Passed = $false; Reason = "PE parse failed: $parseFailure"; Cuda = @() } } else { Test-Imports $ArtifactKind $imports }
    $sha = if ($null -eq $KnownImports -and $null -eq $KnownBytes) { (Get-FileHash -LiteralPath $ArtifactPath -Algorithm SHA256).Hash.ToLowerInvariant() } else { 'synthetic' }
    $verdict = if ($result.Passed) { 'PASS' } else { 'FAIL' }
    [pscustomobject]@{ Kind = $ArtifactKind; Path = $ArtifactPath; Sha256 = $sha; Verdict = $verdict; Imports = @($imports | Sort-Object -Unique); CudaImports = $result.Cuda; Reason = $result.Reason }
}

function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { throw "Self-test failed: $Message" } }

function New-FixturePe([bool]$Delay, [string]$DllName, [string]$SecondDllName = $null, [bool]$Truncated = $false) {
    $bytes = [byte[]]::new(0x600)
    $writer = [IO.BinaryWriter]::new([IO.MemoryStream]::new($bytes, $true))
    try {
        $writer.BaseStream.Position = 0; $writer.Write([uint16]0x5a4d)
        $writer.BaseStream.Position = 0x3c; $writer.Write([uint32]0x80)
        $writer.BaseStream.Position = 0x80; $writer.Write([uint32]0x00004550); $writer.Write([uint16]0x8664); $writer.Write([uint16]1)
        $writer.BaseStream.Position = 0x94; $writer.Write([uint16]0xf0)
        $writer.BaseStream.Position = 0x98; $writer.Write([uint16]0x20b)
        $writer.BaseStream.Position = 0x98 + 108; $writer.Write([uint32]16)
        $directoryIndex = if ($Delay) { 13 } else { 1 }
        $descriptorSize = if ($Delay) { 32 } else { 20 }
        $importCount = if ($SecondDllName) { 2 } else { 1 }
        $tableSize = if ($Truncated) { $descriptorSize } else { ($importCount + 1) * $descriptorSize }
        $writer.BaseStream.Position = 0x98 + 112 + $directoryIndex * 8
        $writer.Write([uint32]0x1000); $writer.Write([uint32]$tableSize)
        $writer.BaseStream.Position = 0x190; $writer.Write([uint32]0x200); $writer.Write([uint32]0x1000); $writer.Write([uint32]0x200); $writer.Write([uint32]0x400)
        for ($descriptorIndex = 0; $descriptorIndex -lt $importCount; $descriptorIndex++) {
            $writer.BaseStream.Position = 0x400 + ($descriptorIndex * $descriptorSize)
            $nameRva = [uint32](0x1040 + ($descriptorIndex * 0x20))
            if ($Delay) { $writer.Write([uint32]1); $writer.Write($nameRva); $writer.Write([uint32]0); $writer.Write([uint32]0); $writer.Write([uint32]0); $writer.Write([uint32]0); $writer.Write([uint32]0); $writer.Write([uint32]0) }
            else { $writer.Write([uint32]0); $writer.Write([uint32]0); $writer.Write([uint32]0); $writer.Write($nameRva); $writer.Write([uint32](0x1050 + ($descriptorIndex * 0x20))) }
            $dll = if ($descriptorIndex -eq 0) { $DllName } else { $SecondDllName }
            $writer.BaseStream.Position = 0x440 + ($descriptorIndex * 0x20)
            $writer.Write([Text.Encoding]::ASCII.GetBytes($dll + "`0"))
        }
    } finally { $writer.Dispose() }
    return ,$bytes
}

if ($SelfTest) {
    $direct = @(Get-PeImports (New-FixturePe $false 'cudart64_12.dll'))
    $delayed = @(Get-PeImports (New-FixturePe $true 'nvcuda.dll'))
    Assert-True ($direct.Count -eq 1 -and $direct[0] -eq 'cudart64_12.dll') 'direct import fixture parsed.'
    Assert-True ($delayed.Count -eq 1 -and $delayed[0] -eq 'nvcuda.dll') 'delay import fixture parsed.'
    $truncatedDirect = Invoke-Check 'Cpu' 'synthetic truncated direct table' -KnownBytes (New-FixturePe $false 'KERNEL32.dll' 'cudart64_12.dll' $true)
    $truncatedDelay = Invoke-Check 'Cpu' 'synthetic truncated delay table' -KnownBytes (New-FixturePe $true 'KERNEL32.dll' 'cudart64_12.dll' $true)
    Assert-True ($truncatedDirect.Verdict -eq 'FAIL' -and $truncatedDirect.Reason -match 'Unterminated direct') 'truncated direct table must fail closed even when CUDA lies beyond its declared size.'
    Assert-True ($truncatedDelay.Verdict -eq 'FAIL' -and $truncatedDelay.Reason -match 'Unterminated delay') 'truncated delay table must fail closed.'
    Assert-True ((Test-Imports 'Gpu' @('cudart64_12.dll','nvcuda.dll')).Passed) 'GPU must require both CUDA imports.'
    Assert-True (-not (Test-Imports 'Gpu' @('cudart64_12.dll')).Passed) 'GPU must fail when nvcuda is missing.'
    Assert-True (-not (Test-Imports 'Cpu' $delayed).Passed) 'delay import must fail CPU.'
    $nonPeRejected = $false
    try { [void](Get-PeImports ([byte[]](1,2,3))) } catch { $nonPeRejected = $true }
    Assert-True $nonPeRejected 'non-PE fixture refused.'
    $nonPeReport = Invoke-Check 'Cpu' 'synthetic non-PE fixture' -KnownBytes ([byte[]](1,2,3))
    Assert-True ($nonPeReport.Verdict -eq 'FAIL' -and $nonPeReport.Sha256 -eq 'synthetic') 'non-PE report contains failure verdict.'
    $negative = '../../../target/parakeet-tests/cargo-shared/release/whisper-worker-cpu.exe'
    $negativePath = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot $negative))
    if (-not (Test-Path -LiteralPath $negativePath -PathType Leaf)) { throw "Required real negative witness is missing: $negativePath" }
    $negativeSha = (Get-FileHash -LiteralPath $negativePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($negativeSha -ne '67e668ff75c0c59522c1e3cdb3529518938209d962e3c66aa5662bbad7001cf4') { throw "Negative witness SHA mismatch: $negativeSha" }
    $witness = Invoke-Check 'Cpu' $negativePath
    Assert-True ($witness.Verdict -eq 'FAIL' -and $witness.CudaImports.Count -gt 0) 'real CPU release witness must be rejected.'
    'PASS: direct/delay import fixtures, non-PE rejection, and SHA-pinned CPU negative witness.'
    exit 0
}

if (-not $Kind -or -not $Path) { throw 'Specify -Kind and -Path, or use -SelfTest.' }
$fullPath = (Resolve-Path -LiteralPath $Path -ErrorAction Stop).Path
$report = Invoke-Check $Kind $fullPath
$report | ConvertTo-Json -Depth 4
if ($report.Verdict -ne 'PASS') { exit 1 }
exit 0
