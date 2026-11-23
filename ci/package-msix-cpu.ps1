[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet('SelfTest', 'BuildAndValidate')]
    [string] $Mode,

    [Parameter(Mandatory)]
    [string] $LotRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:ExpectedLot = 'target/parakeet-tests/msix-cpu-prototype-20261122'
$script:PackagePublisher = 'CN=WhisperTurbo MSIX CPU Prototype'
$script:P7Zip = 'C:\wt\distcpu-l2-reprise\deliverables\distribution-duale-p7-20261122\cpu\WhisperTurbo-Dual-CPU.zip'
$script:P7Manifest = 'C:\wt\distcpu-l2-reprise\deliverables\distribution-duale-p7-20261122\cpu\WhisperTurbo-Dual-CPU.manifest.json'
$script:P7ZipSha256 = 'b3231dbc6e05fa10495a0ef23e96e2e3cf18c4be96e8c8add3d4f0ff7a1b72db'
$script:P7ManifestSha256 = '81692e3080775e0b10636aded3a6951d67723f03362d70139625f44306d1a305'
$script:BootstrapSha256 = 'f3319958f1a4e0760ec353c4faacc06086db55fd94a09a236b807b22299a02da'
$script:DesktopSha256 = '6efb8c631d9e9def5f0b476ebb5ce3f4abb3a00ca5fcd132c477c0d91d22935a'
$script:TurboWorkerSha256 = 'a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b'
$script:ParakeetWorkerSha256 = 'aa0abbd61ab2cf05bdc1291b5c5d5d37a9a481713be08e43cf96d884ed3bb0f1'

function Get-RepoRoot {
    $path = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
    return [IO.Path]::GetFullPath($path).TrimEnd('\')
}

function Assert-LotRoot([string] $Path) {
    $repo = Get-RepoRoot
    $expected = [IO.Path]::GetFullPath((Join-Path $repo $script:ExpectedLot))
    $candidate = [IO.Path]::GetFullPath((Join-Path $repo $Path))
    if (-not $candidate.Equals($expected, [StringComparison]::OrdinalIgnoreCase)) {
        throw "LotRoot must be exactly $($script:ExpectedLot)."
    }
    if (-not $candidate.StartsWith(($repo + '\'), [StringComparison]::OrdinalIgnoreCase)) {
        throw 'LotRoot escapes the repository.'
    }
    return $candidate
}

function Test-CpuInventory([string[]] $Paths) {
    foreach ($path in $Paths) {
        if ($path -match '(^|[\\/])native[\\/]gpu([\\/]|$)' -or
            $path -match 'whisper-worker-(gpu|parakeet-gpu)\.exe$' -or
            $path -match 'ggml-cuda\.dll$' -or
            $path -match '\b(cublas|cudart|nvrtc)') {
            return $false
        }
    }
    return $true
}

function Assert-ManifestContract([xml] $Document, [string] $ExpectedPublisher) {
    $namespaces = [Xml.XmlNamespaceManager]::new($Document.NameTable)
    $namespaces.AddNamespace('f', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10')
    $namespaces.AddNamespace('uap', 'http://schemas.microsoft.com/appx/manifest/uap/windows10')
    $namespaces.AddNamespace('rescap', 'http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities')
    $identity = $Document.SelectSingleNode('/f:Package/f:Identity', $namespaces)
    $application = $Document.SelectSingleNode('/f:Package/f:Applications/f:Application', $namespaces)
    $runFullTrust = $Document.SelectSingleNode('/f:Package/f:Capabilities/rescap:Capability[@Name="runFullTrust"]', $namespaces)
    $microphone = $Document.SelectSingleNode('/f:Package/f:Capabilities/f:DeviceCapability[@Name="microphone"]', $namespaces)
    if ($null -eq $identity -or $identity.GetAttribute('Name') -cne 'WhisperTurbo.CpuPrototype' -or
        $identity.GetAttribute('Publisher') -cne $ExpectedPublisher -or
        $identity.GetAttribute('ProcessorArchitecture') -cne 'x64') {
        throw 'MSIX identity/publisher/architecture does not match the approved CPU prototype.'
    }
    if ($null -eq $application -or $application.GetAttribute('Executable') -cne 'whisper-bootstrap-cpu.exe' -or
        $application.GetAttribute('EntryPoint') -cne 'Windows.FullTrustApplication') {
        throw 'MSIX entry point does not match the approved full-trust bootstrap.'
    }
    if ($null -eq $runFullTrust -or $null -eq $microphone) {
        throw 'MSIX package must declare runFullTrust and microphone capabilities.'
    }
}

function Assert-True([bool] $Condition, [string] $Message) {
    if (-not $Condition) { throw $Message }
}

function Assert-PayloadInventoriesMatch([object[]] $Expected, [object[]] $Actual) {
    $expectedByPath = @{}
    foreach ($entry in $Expected) { $expectedByPath[[string]$entry.Path] = $entry }
    $actualByPath = @{}
    foreach ($entry in $Actual) { $actualByPath[[string]$entry.Path] = $entry }

    $allowedFootprint = @(
        'AppxBlockMap.xml',
        '[Content_Types].xml',
        'AppxSignature.p7x',
        'AppxMetadata/CodeIntegrity.cat'
    )
    $missing = @($expectedByPath.Keys | Where-Object { -not $actualByPath.ContainsKey($_) })
    $unexpected = @($actualByPath.Keys | Where-Object {
        -not $expectedByPath.ContainsKey($_) -and $_ -notin $allowedFootprint
    })
    Assert-True ($missing.Count -eq 0) "Unpacked MSIX is missing payload paths: $($missing -join ', ')"
    Assert-True ($unexpected.Count -eq 0) "Unpacked MSIX has unexpected paths: $($unexpected -join ', ')"

    foreach ($path in $expectedByPath.Keys) {
        $expected = $expectedByPath[$path]
        $actual = $actualByPath[$path]
        Assert-True ([long]$actual.Bytes -eq [long]$expected.Bytes) "Unpacked MSIX payload size differs: $path"
        Assert-True ([string]$actual.Sha256 -ceq [string]$expected.Sha256) "Unpacked MSIX payload SHA-256 differs: $path"
    }
}

function Invoke-SelfTest {
    $repo = Get-RepoRoot
    $exact = Assert-LotRoot $script:ExpectedLot
    Assert-True ($exact.StartsWith(($repo + '\'), [StringComparison]::OrdinalIgnoreCase)) 'positive lot-root test failed'
    $escaped = $false
    try { [void](Assert-LotRoot 'target/parakeet-tests/../outside') } catch { $escaped = $true }
    Assert-True $escaped 'negative lot-root test accepted an escaped path'
    Assert-True (Test-CpuInventory @('app/whisper-worker-cpu.exe', 'native/cpu/ggml-cpu.dll')) 'positive CPU inventory test failed'
    Assert-True (-not (Test-CpuInventory @('native/gpu/ggml-cuda.dll'))) 'negative GPU inventory test failed'
    Assert-True (-not (Test-CpuInventory @('worker/whisper-worker-parakeet-gpu.exe'))) 'negative GPU worker test failed'
    [xml]$manifest = Get-Content -LiteralPath (Join-Path (Get-RepoRoot) 'packaging/msix/cpu/Package.appxmanifest') -Raw
    Assert-ManifestContract $manifest $script:PackagePublisher
    $badArchitectureRejected = $false
    try {
        [xml]$bad = $manifest.OuterXml.Replace('ProcessorArchitecture="x64"', 'ProcessorArchitecture="arm64"')
        Assert-ManifestContract $bad $script:PackagePublisher
    } catch { $badArchitectureRejected = $true }
    Assert-True $badArchitectureRejected 'negative manifest architecture test failed'
    $badPublisherRejected = $false
    try { Assert-ManifestContract $manifest 'CN=unexpected publisher' } catch { $badPublisherRejected = $true }
    Assert-True $badPublisherRejected 'negative manifest publisher test failed'

    $inventory = @(
        [pscustomobject]@{ Path = 'AppxManifest.xml'; Bytes = 10L; Sha256 = 'manifest' },
        [pscustomobject]@{ Path = 'app/whisper.exe'; Bytes = 20L; Sha256 = 'desktop' }
    )
    $withFootprint = @($inventory) + @(
        [pscustomobject]@{ Path = 'AppxBlockMap.xml'; Bytes = 30L; Sha256 = 'blockmap' },
        [pscustomobject]@{ Path = '[Content_Types].xml'; Bytes = 40L; Sha256 = 'types' },
        [pscustomobject]@{ Path = 'AppxSignature.p7x'; Bytes = 50L; Sha256 = 'signature' },
        [pscustomobject]@{ Path = 'AppxMetadata/CodeIntegrity.cat'; Bytes = 60L; Sha256 = 'catalog' }
    )
    Assert-PayloadInventoriesMatch $inventory $withFootprint
    $missingRejected = $false
    try { Assert-PayloadInventoriesMatch $inventory @($inventory[0]) } catch { $missingRejected = $true }
    Assert-True $missingRejected 'negative missing-payload inventory test failed'
    $unexpectedRejected = $false
    try {
        Assert-PayloadInventoriesMatch $inventory (@($inventory) + @(
            [pscustomobject]@{ Path = 'unexpected.txt'; Bytes = 1L; Sha256 = 'unexpected' }
        ))
    } catch { $unexpectedRejected = $true }
    Assert-True $unexpectedRejected 'negative unexpected-payload inventory test failed'
    $digestRejected = $false
    try {
        Assert-PayloadInventoriesMatch $inventory @(
            [pscustomobject]@{ Path = 'AppxManifest.xml'; Bytes = 10L; Sha256 = 'changed' },
            $inventory[1]
        )
    } catch { $digestRejected = $true }
    Assert-True $digestRejected 'negative payload digest test failed'
    Write-Output 'MSIX_CPU_PACKAGE_SELFTEST positive=4 negative=8; Cargo=NOT RUN; system changes=NONE'
}

function Get-Sha256([string] $Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
}

function Get-FileInventory([string] $Root) {
    return @(
        Get-ChildItem -LiteralPath $Root -File -Recurse | ForEach-Object {
            [pscustomobject]@{
                Path = [IO.Path]::GetRelativePath($Root, $_.FullName).Replace('\','/')
                Bytes = [long]$_.Length
                Sha256 = Get-Sha256 $_.FullName
            }
        }
    )
}

function Assert-MsixRoundTrip([string] $MakeAppx, [string] $Msix, [string] $Staging, [string] $RoundTrip, [string] $Phase) {
    if (Test-Path -LiteralPath $RoundTrip) { throw "Refusing to replace existing MSIX validation output: $RoundTrip" }
    New-Item -ItemType Directory -Path $RoundTrip -ErrorAction Stop | Out-Null
    & $MakeAppx unpack /p $Msix /d $RoundTrip
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx unpack $Phase failed: $LASTEXITCODE" }

    $expected = @(Get-FileInventory $Staging)
    $actual = @(Get-FileInventory $RoundTrip)
    Assert-PayloadInventoriesMatch $expected $actual

    Remove-Item -LiteralPath $RoundTrip -Recurse -Force -ErrorAction Stop
    if (Test-Path -LiteralPath $RoundTrip) { throw "MSIX validation output remains after cleanup: $RoundTrip" }
}

function Assert-ExactFile([string] $Path, [string] $Sha256, [string] $Label) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "$Label is missing: $Path" }
    $actual = Get-Sha256 $Path
    if ($actual -ne $Sha256.ToLowerInvariant()) { throw "$Label SHA-256 mismatch: $actual" }
}

function Assert-NoReparseAncestors([string] $Path, [string] $Boundary) {
    $cursor = [IO.Path]::GetFullPath($Path)
    $root = [IO.Path]::GetFullPath($Boundary).TrimEnd('\')
    while ($cursor.StartsWith(($root + '\'), [StringComparison]::OrdinalIgnoreCase)) {
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -LiteralPath $cursor -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Reparse point in output path: $cursor" }
        }
        $parent = Split-Path -Parent $cursor
        if ($parent -eq $cursor) { break }
        $cursor = $parent
    }
}

function Assert-P7Archive {
    Assert-ExactFile $script:P7Zip $script:P7ZipSha256 'P7 CPU archive'
    Assert-ExactFile $script:P7Manifest $script:P7ManifestSha256 'P7 CPU manifest'
    $manifest = Get-Content -LiteralPath $script:P7Manifest -Raw | ConvertFrom-Json
    Assert-True ($manifest.variant -ceq 'cpu') 'P7 manifest is not CPU-only'
    Assert-True ($manifest.archive.sha256 -ceq $script:P7ZipSha256) 'P7 manifest archive digest does not match the approved digest'
    Assert-True ((Get-Item -LiteralPath $script:P7Zip).Length -eq [long]$manifest.archive.bytes) 'P7 archive size differs from its manifest'
    Assert-True (Test-CpuInventory @($manifest.files.path)) 'P7 manifest contains a GPU resource'
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($script:P7Zip)
    try {
        $entries = @($archive.Entries | Where-Object { -not [string]::IsNullOrEmpty($_.Name) })
        $expected = @($manifest.files.path)
        Assert-True ($entries.Count -eq $expected.Count) 'P7 archive entry count differs from its manifest'
        $actualNames = @($entries.FullName | Sort-Object -Unique)
        $expectedNames = @($expected | Sort-Object -Unique)
        $pathDifferences = @(Compare-Object $actualNames $expectedNames)
        Assert-True ($pathDifferences.Count -eq 0) 'P7 archive paths differ from its manifest'
        $records = foreach ($file in $manifest.files) {
            [pscustomobject]@{ Path = [string]$file.path; Bytes = [long]$file.bytes; Sha256 = [string]$file.sha256 }
        }
        $records += [pscustomobject]@{
            Path = [string]$manifest.bootstrap.file
            Bytes = [long]$manifest.bootstrap.bytes
            Sha256 = [string]$manifest.bootstrap.sha256
        }
        foreach ($record in $records) {
            $entry = $archive.GetEntry($record.Path)
            Assert-True ($null -ne $entry) "P7 entry missing: $($record.Path)"
            Assert-True ($entry.Length -eq $record.Bytes) "P7 entry size mismatch: $($entry.FullName)"
            $stream = $entry.Open()
            $sha = [Security.Cryptography.SHA256]::Create()
            try { $digest = [Convert]::ToHexString($sha.ComputeHash($stream)).ToLowerInvariant() }
            finally { $stream.Dispose(); $sha.Dispose() }
            Assert-True ($digest -ceq $record.Sha256) "P7 entry SHA-256 mismatch: $($entry.FullName)"
        }
    } finally { $archive.Dispose() }
    return $manifest
}

function Write-LogoSet([string] $DesktopExe, [string] $OutputDirectory) {
    Add-Type -AssemblyName System.Drawing
    $source = [System.Drawing.Icon]::ExtractAssociatedIcon($DesktopExe)
    if ($null -eq $source) { throw 'The desktop executable has no usable embedded icon.' }
    $sourceBitmap = $source.ToBitmap()
    try {
        $specs = @(
            @{ Name = 'StoreLogo.png'; Width = 50; Height = 50; Icon = 44 },
            @{ Name = 'Square44x44Logo.png'; Width = 44; Height = 44; Icon = 40 },
            @{ Name = 'Square150x150Logo.png'; Width = 150; Height = 150; Icon = 128 },
            @{ Name = 'Wide310x150Logo.png'; Width = 310; Height = 150; Icon = 128 }
        )
        foreach ($spec in $specs) {
            $image = New-Object System.Drawing.Bitmap([int]$spec.Width, [int]$spec.Height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
            $graphics = [System.Drawing.Graphics]::FromImage($image)
            try {
                $graphics.Clear([System.Drawing.Color]::Transparent)
                $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
                $inner = [int]$spec.Icon
                $graphics.DrawImage($sourceBitmap, [System.Drawing.Rectangle]::new([int](($spec.Width - $inner) / 2), [int](($spec.Height - $inner) / 2), $inner, $inner))
                $image.Save((Join-Path $OutputDirectory $spec.Name), [System.Drawing.Imaging.ImageFormat]::Png)
            } finally { $graphics.Dispose(); $image.Dispose() }
        }
    } finally { $sourceBitmap.Dispose(); $source.Dispose() }
}

function New-DevelopmentCertificate([string] $SigningDirectory) {
    $rsa = [Security.Cryptography.RSA]::Create(2048)
    $request = [Security.Cryptography.X509Certificates.CertificateRequest]::new(
        $script:PackagePublisher,
        $rsa,
        [Security.Cryptography.HashAlgorithmName]::SHA256,
        [Security.Cryptography.RSASignaturePadding]::Pkcs1
    )
    $request.CertificateExtensions.Add([Security.Cryptography.X509Certificates.X509BasicConstraintsExtension]::new($false, $false, 0, $true))
    $request.CertificateExtensions.Add([Security.Cryptography.X509Certificates.X509KeyUsageExtension]::new([Security.Cryptography.X509Certificates.X509KeyUsageFlags]::DigitalSignature, $true))
    $eku = [Security.Cryptography.OidCollection]::new()
    [void]$eku.Add([Security.Cryptography.Oid]::new('1.3.6.1.5.5.7.3.3'))
    $request.CertificateExtensions.Add([Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new($eku, $false))
    $certificate = $request.CreateSelfSigned([DateTimeOffset]::UtcNow.AddMinutes(-5), [DateTimeOffset]::UtcNow.AddYears(1))
    $password = [Convert]::ToBase64String([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
    $pfxPath = Join-Path $SigningDirectory 'development-signing.pfx'
    $cerPath = Join-Path $SigningDirectory 'development-signing.cer'
    [IO.File]::WriteAllBytes($pfxPath, $certificate.Export([Security.Cryptography.X509Certificates.X509ContentType]::Pfx, $password))
    [IO.File]::WriteAllBytes($cerPath, $certificate.Export([Security.Cryptography.X509Certificates.X509ContentType]::Cert))
    $subject = $certificate.Subject
    $thumbprint = $certificate.Thumbprint
    $certificate.Dispose()
    $rsa.Dispose()
    return [pscustomobject]@{
        Pfx = $pfxPath
        Cer = $cerPath
        Password = $password
        Subject = $subject
        Thumbprint = $thumbprint
    }
}

function Verify-MsixSignature([string] $Msix, [string] $Thumbprint) {
    Add-Type -AssemblyName System.Security.Cryptography.Pkcs
    $archive = [IO.Compression.ZipFile]::OpenRead($Msix)
    try {
        $entry = $archive.GetEntry('AppxSignature.p7x')
        if ($null -eq $entry) { throw 'MSIX signature payload is missing.' }
        $stream = $entry.Open()
        try {
            $bytes = [byte[]]::new([int]$entry.Length)
            $offset = 0
            while ($offset -lt $bytes.Length) {
                $count = $stream.Read($bytes, $offset, $bytes.Length - $offset)
                if ($count -le 0) { throw 'Truncated MSIX signature payload.' }
                $offset += $count
            }
        } finally { $stream.Dispose() }
        if ($bytes.Length -lt 8 -or [Text.Encoding]::ASCII.GetString($bytes, 0, 4) -ne 'PKCX') {
            throw 'MSIX signature header is invalid.'
        }
        $cms = [Security.Cryptography.Pkcs.SignedCms]::new()
        $cms.Decode($bytes[4..($bytes.Length - 1)])
        $cms.CheckSignature($true)
        $signer = $cms.SignerInfos[0].Certificate
        if ($null -eq $signer -or $signer.Thumbprint -ne $Thumbprint) { throw 'MSIX signature signer does not match the local certificate.' }
        return $signer.Subject
    } finally { $archive.Dispose() }
}

function Invoke-BuildAndValidate {
    $repo = Get-RepoRoot
    $lot = Assert-LotRoot $LotRoot
    Assert-NoReparseAncestors $lot $repo
    $drive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($lot))
    if ($drive.AvailableFreeSpace -lt 30GB) { throw 'P8 requires at least 30 GiB free before package staging.' }

    $p7 = Assert-P7Archive
    $release = Join-Path $lot 'targets/msix-cpu/release'
    $bootstrap = Join-Path $release 'whisper-bootstrap.exe'
    $desktop = Join-Path $release 'whisper-desktop.exe'
    $turboWorker = Join-Path $release 'whisper-worker-cpu.exe'
    $parakeetWorker = Join-Path $release 'whisper-worker-parakeet-cpu.exe'
    foreach ($path in @($bootstrap,$desktop,$turboWorker,$parakeetWorker)) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Guarded release artifact missing: $path" }
    }
    Assert-ExactFile $bootstrap $script:BootstrapSha256 'P8 bootstrap'
    Assert-ExactFile $desktop $script:DesktopSha256 'P8 desktop'
    Assert-ExactFile $turboWorker $script:TurboWorkerSha256 'P7 Turbo CPU worker'
    Assert-ExactFile $parakeetWorker $script:ParakeetWorkerSha256 'P8 Parakeet CPU worker'

    $packageRoot = Join-Path $lot 'package'
    $staging = Join-Path $packageRoot 'staging'
    $signing = Join-Path $packageRoot 'signing'
    $roundTrip = Join-Path $packageRoot 'unpack-validation'
    $msix = Join-Path $packageRoot 'WhisperTurbo-CPU-prototype.msix'
    $reportPath = Join-Path $packageRoot 'package-report.json'
    foreach ($path in @($staging,$signing,$roundTrip,$msix,$reportPath)) {
        if (Test-Path -LiteralPath $path) { throw "Refusing to replace existing package output: $path" }
    }
    New-Item -ItemType Directory -Path $staging,$signing -ErrorAction Stop | Out-Null
    Assert-NoReparseAncestors $staging $repo
    [IO.Compression.ZipFile]::ExtractToDirectory($script:P7Zip, $staging)

    $assetsSource = Join-Path $repo 'packaging/msix/cpu/Assets'
    $manifestSource = Join-Path $repo 'packaging/msix/cpu/Package.appxmanifest'
    $assetsTarget = Join-Path $staging 'Assets'
    New-Item -ItemType Directory -Path $assetsTarget -ErrorAction Stop | Out-Null
    Write-LogoSet $desktop $assetsTarget
    foreach ($name in @('StoreLogo.png','Square44x44Logo.png','Square150x150Logo.png','Wide310x150Logo.png')) {
        $sourceLogo = Join-Path $assetsSource $name
        Assert-True ((Get-Sha256 $sourceLogo) -ceq (Get-Sha256 (Join-Path $assetsTarget $name))) "Generated logo differs from approved source: $name"
    }
    Copy-Item -LiteralPath $manifestSource -Destination (Join-Path $staging 'AppxManifest.xml') -ErrorAction Stop
    [xml]$stagedManifest = Get-Content -LiteralPath (Join-Path $staging 'AppxManifest.xml') -Raw
    Assert-ManifestContract $stagedManifest $script:PackagePublisher
    Copy-Item -LiteralPath $bootstrap -Destination (Join-Path $staging 'whisper-bootstrap-cpu.exe') -Force -ErrorAction Stop
    Copy-Item -LiteralPath $desktop -Destination (Join-Path $staging 'app/whisper-desktop.exe') -Force -ErrorAction Stop
    Copy-Item -LiteralPath $turboWorker -Destination (Join-Path $staging 'app/whisper-worker-cpu.exe') -Force -ErrorAction Stop
    Copy-Item -LiteralPath $parakeetWorker -Destination (Join-Path $staging 'worker/whisper-worker-parakeet-cpu.exe') -Force -ErrorAction Stop

    $paths = @(Get-ChildItem -LiteralPath $staging -File -Recurse | ForEach-Object { [IO.Path]::GetRelativePath($staging, $_.FullName).Replace('\','/') })
    if (-not (Test-CpuInventory $paths)) { throw 'CPU package staging contains a GPU resource.' }
    $makeAppx = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\makeappx.exe'
    $signTool = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe'
    foreach ($tool in @($makeAppx,$signTool)) { if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) { throw "Windows SDK tool missing: $tool" } }
    & $makeAppx pack /d $staging /p $msix
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx pack failed: $LASTEXITCODE" }
    Assert-MsixRoundTrip $makeAppx $msix $staging $roundTrip 'before signing'

    $certificate = New-DevelopmentCertificate $signing
    Assert-ManifestContract $stagedManifest $certificate.Subject
    & $signTool sign /fd SHA256 /f $certificate.Pfx /p $certificate.Password $msix
    if ($LASTEXITCODE -ne 0) { throw "SignTool signing failed: $LASTEXITCODE" }
    Assert-MsixRoundTrip $makeAppx $msix $staging $roundTrip 'after signing'
    $signer = Verify-MsixSignature $msix $certificate.Thumbprint

    $inventory = @(Get-ChildItem -LiteralPath $staging -File -Recurse | ForEach-Object {
        [pscustomobject]@{
            path = [IO.Path]::GetRelativePath($staging, $_.FullName).Replace('\','/')
            bytes = $_.Length
            sha256 = Get-Sha256 $_.FullName
        }
    })
    $report = [ordered]@{
        status = 'ASSEMBLED_SIGNED_NOT_INSTALLED'
        timestampUtc = [DateTime]::UtcNow.ToString('o')
        sourceP7Archive = @{ path = $script:P7Zip; sha256 = $script:P7ZipSha256; manifestSha256 = $script:P7ManifestSha256 }
        releaseArtifacts = @(
            @{ path = 'whisper-bootstrap-cpu.exe'; sha256 = Get-Sha256 $bootstrap },
            @{ path = 'app/whisper-desktop.exe'; sha256 = Get-Sha256 $desktop },
            @{ path = 'app/whisper-worker-cpu.exe'; sha256 = Get-Sha256 $turboWorker },
            @{ path = 'worker/whisper-worker-parakeet-cpu.exe'; sha256 = Get-Sha256 $parakeetWorker }
        )
        package = @{ path = $msix; bytes = (Get-Item -LiteralPath $msix).Length; sha256 = Get-Sha256 $msix }
        signature = @{ cryptographicCheck = 'PASS'; signerSubject = $signer; thumbprint = $certificate.Thumbprint; trustStoreModified = $false }
        installation = 'NOT RUN; separate user authorization required'
        stagedInventory = $inventory
    }
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding utf8
    Write-Output "MSIX assembled and cryptographically signed: $msix"
    Write-Output "SHA256=$($report.package.sha256) CERT=$($certificate.Thumbprint)"
    Write-Output "Trust store and AppX registration unchanged. Report: $reportPath"
}

switch ($Mode) {
    'SelfTest' { Invoke-SelfTest }
    'BuildAndValidate' { Invoke-BuildAndValidate }
}
