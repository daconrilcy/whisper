#requires -Version 7.4
[CmdletBinding()]
param(
    [ValidateSet('Git','P7R1','Modeles','Assemblage','Cibles','Interface','PaquetP5')]
    [string[]]$Groupe = @('Git','P7R1','Modeles','Assemblage','Cibles','PaquetP5'),
    [switch]$Appliquer
)

$ErrorActionPreference = 'Stop'
$repository = 'C:\dev\whisper'
$modelReferences = @{
    '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' = 'C:\dev\whisper\models\ggml-large-v3-turbo.bin'
    'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' = 'C:\dev\whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'
}
$modelCopies = @(
    @{ Groupe = 'Modeles'; Chemin = 'C:\Tests\ParakeetFaisabilite\2d5a0f5a9f5a4511be07728bf825747c\profil\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\Tests\ParakeetFaisabilite\a8cc3311bd83449f90a6cdcce721e596\profil\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpu-0beace7bb0474d4982730cc777c7ecf1\localappdata\Whisper\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpu-0beace7bb0474d4982730cc777c7ecf1\localappdata\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpu-a3af00c9b7014cc78e35d50c3aea63a5\localappdata\Whisper\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpu-a3af00c9b7014cc78e35d50c3aea63a5\localappdata\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpu-d1db985834ea444f91428dbd51fb5ba2\localappdata\Whisper\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpu-d1db985834ea444f91428dbd51fb5ba2\localappdata\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuEnglishAutoLong-715526e598bc4738b897f167a8bcb000\localappdata\Whisper\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuEnglishAutoLong-715526e598bc4738b897f167a8bcb000\localappdata\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuEnglishLong-5ad2d319845244538f898ff38ff82902\localappdata\Whisper\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuEnglishLong-5ad2d319845244538f898ff38ff82902\localappdata\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuFrenchAutoLong-79fa928beb4045c3bcf1aef349bf68cb\localappdata\Whisper\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuFrenchAutoLong-79fa928beb4045c3bcf1aef349bf68cb\localappdata\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuFrenchLong-674842d94ada4101bf733545d578ae70\localappdata\Whisper\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuFrenchLong-674842d94ada4101bf733545d578ae70\localappdata\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuFrenchShort-585f680bf93044419d4c216f322f02cb\localappdata\Whisper\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\dev\whisper\target\n2-native\runs\ParakeetGpuFrenchShort-585f680bf93044419d4c216f322f02cb\localappdata\Whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\p5gtx-20261121\package\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\p5gtx-20261121\package\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\p5gtx-20261121\profile\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\p5gtx-20261121\profile\WhisperTurbo\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\T33-verif-bb3fd3\profil-essai\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\arm64-remise-preflight-20261118\local\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\boot-dell-249fd0\profil-essai\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\bootstrap-recette-7ab533a9d1904a46924ea1580135b866\profil\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\diagnostic-arm64-fixture-20261118\modele\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\obs-auto-l4\profil-complet\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\recette-cpu-fix\LocalAppData\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\recette-l3-20261117\t33-profile\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-final-desktop\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-final-desktop\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-identity-fix\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-identity-fix\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-runtime-fix\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-runtime-fix\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-worker-admission-fix-20261120\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-worker-admission-fix-20261120\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-worker-sha-realignment\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate-pre-worker-sha-realignment\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\candidate\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\p4-candidate\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\p4-candidate\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\scratch\fresh-ui-f33d3ba927534f8c99d852acdcd9a0fc\LocalAppData\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\scratch\fresh-ui-f33d3ba927534f8c99d852acdcd9a0fc\LocalAppData\WhisperTurbo\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\scratch\fresh-ui-runtime-p3-final-20261120\LocalAppData\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\scratch\fresh-ui-runtime-p3-final-20261120\LocalAppData\WhisperTurbo\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\scratch\install-p3-final-20261120\profile\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\scratch\install-p3-final-20261120\profile\WhisperTurbo\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\scratch\p3-fresh-6e2b94535a14483dbfe7bab20d7e95f5\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'Modeles'; Chemin = 'C:\wt\distcpu-l2-reprise\target\parakeet-tests\distribution-duale-p3-20261119\scratch\p3-fresh-6e2b94535a14483dbfe7bab20d7e95f5\WhisperTurbo\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'P7R1'; Chemin = 'C:\Users\cyril\AppData\Local\Temp\WhisperTurbo-P7-R1-2d3436150e284263b8e73b4968d1be67\package\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'P7R1'; Chemin = 'C:\Users\cyril\AppData\Local\Temp\WhisperTurbo-P7-R1-2d3436150e284263b8e73b4968d1be67\package\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'P7R1'; Chemin = 'C:\Users\cyril\AppData\Local\Temp\WhisperTurbo-P7-R1-2d3436150e284263b8e73b4968d1be67\profile\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'P7R1'; Chemin = 'C:\Users\cyril\AppData\Local\Temp\WhisperTurbo-P7-R1-2d3436150e284263b8e73b4968d1be67\profile\WhisperTurbo\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'P7R1'; Chemin = 'C:\Users\cyril\AppData\Local\Temp\WhisperTurbo-P7-R1-b7be1bf952754dbfa2dc12bc5565c5db\package\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'P7R1'; Chemin = 'C:\Users\cyril\AppData\Local\Temp\WhisperTurbo-P7-R1-b7be1bf952754dbfa2dc12bc5565c5db\package\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
    @{ Groupe = 'P7R1'; Chemin = 'C:\Users\cyril\AppData\Local\Temp\WhisperTurbo-P7-R1-b7be1bf952754dbfa2dc12bc5565c5db\profile\WhisperTurbo\models\ggml-large-v3-turbo.bin'; SHA256 = '1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69' }
    @{ Groupe = 'P7R1'; Chemin = 'C:\Users\cyril\AppData\Local\Temp\WhisperTurbo-P7-R1-b7be1bf952754dbfa2dc12bc5565c5db\profile\WhisperTurbo\models\parakeet-tdt-0.6b-v3.q8_0.gguf'; SHA256 = 'E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E' }
)
$gitCopies = @(
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_6mJdib'; Taille = 764542988; Modification = 639292223714643352L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_74LjbW'; Taille = 625737740; Modification = 639292219393143614L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_egoVGw'; Taille = 782368780; Modification = 639292210474983919L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_gbRL4j'; Taille = 915013644; Modification = 639292235861128068L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_Gwg9vc'; Taille = 580648972; Modification = 639292215626942648L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_I2FKlv'; Taille = 385220620; Modification = 639292191925268757L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_KwfuHd'; Taille = 483262476; Modification = 639292414146822722L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_KyVkyN'; Taille = 633470988; Modification = 639292227274931551L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_mOnSUV'; Taille = 786956300; Modification = 639292196931611082L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_OJX5Yx'; Taille = 740818956; Modification = 639292240039515368L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_PCJOct'; Taille = 229769228; Modification = 639292248926287512L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_RrNO79'; Taille = 646709260; Modification = 639292230142113044L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_sGHCWV'; Taille = 417595404; Modification = 639292195275924406L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_SqdTL2'; Taille = 138805260; Modification = 639292910140660747L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_U5BSLp'; Taille = 638320652; Modification = 639292201693977172L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_wwTwUN'; Taille = 272367628; Modification = 639292398414192486L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_x9d7ri'; Taille = 947257356; Modification = 639292222070141851L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_yrCEUK'; Taille = 266862604; Modification = 639292195837897362L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_Z6kqb1'; Taille = 197394444; Modification = 639292193142789414L }
    @{ Chemin = 'C:\dev\whisper\.git\objects\pack\tmp_pack_zLEvwT'; Taille = 952762380; Modification = 639292411855097647L }
)
$targetRoots = @(
    'C:\wt\clippy-corrections-suivantes\target\parakeet-tests\clippy-corrections-suivantes-20261113\targets'
    'C:\wt\clippy-frais-20261113\target\parakeet-tests\refacto-reprise-import\targets'
    'C:\wt\clippy-priorites\target\parakeet-tests\garde-n1-paquets-20261112\targets'
    'C:\wt\clippy-structure-parakeet-sessions\target\parakeet-tests\clippy-corrections-suivantes-20261113\targets'
    'C:\wt\cpu-distribution-design-plan\target\parakeet-tests\cpu-distribution-x64-20261115\targets'
    'C:\wt\distcpu-l2-reprise\target\parakeet-tests\cpu-distribution-x64-20261115\targets'
    'C:\wt\distcpu-l2-reprise\target\parakeet-tests\main-facades\targets'
    'C:\wt\refacto-reprise-import\target\parakeet-tests\refacto-reprise-import\targets'
    'C:\wt\ui-transcriber-impl\target\parakeet-tests\ui-transcriber-impl\targets'
    'C:\wt\whisper-msix-cpu-spike\target\parakeet-tests\msix-cpu-prototype-20261122\targets'
    'C:\wt\whisper-msix-cpu-spike\target\parakeet-tests\msix-nvidia-prototype-20261123\targets'
)

function Get-CheckedItem([string]$Path, [bool]$Directory) {
    if ([IO.Path]::GetFullPath($Path) -ine $Path) { throw "Chemin non canonique : $Path" }
    $leaf = Get-Item -LiteralPath $Path -Force
    if ($leaf.PSIsContainer -ne $Directory) { throw "Type inattendu : $Path" }
    for ($part = $Path; $part; $part = [IO.Path]::GetDirectoryName($part)) {
        if ((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Lien ou jonction dans le chemin : $part"
        }
    }
    $leaf
}

function Get-TreeFiles([string]$Root, [bool]$SkipLinks) {
    $pending = [Collections.Generic.Stack[string]]::new()
    $pending.Push($Root)
    while ($pending.Count -gt 0) {
        $directory = [IO.DirectoryInfo]::new($pending.Pop())
        foreach ($entry in $directory.EnumerateFileSystemInfos()) {
            if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                if (-not $SkipLinks) { throw "Lien ou jonction : $($entry.FullName)" }
                continue
            }
            if ($entry -is [IO.DirectoryInfo]) { $pending.Push($entry.FullName) }
            else { $entry }
        }
    }
}

function Get-StreamHash([IO.Stream]$Stream) {
    try { [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($Stream)) }
    finally { $Stream.Dispose() }
}

function Assert-FileHash([string]$Path, [string]$SHA256) {
    if ((Get-StreamHash ([IO.File]::OpenRead($Path))) -cne $SHA256) {
        throw "SHA-256 inattendu : $Path"
    }
}

function Assert-ZipEntries($Zip, [string]$Root, [object[]]$Files, [string]$Prefix) {
    foreach ($file in $Files) {
        $name = [IO.Path]::GetRelativePath($Root, $file.FullName).Replace('\', '/')
        $entry = $Zip.GetEntry($Prefix + $name)
        if ($null -eq $entry -or $entry.Length -ne $file.Length) { throw "Entrée manquante ou taille différente : $name" }
        if ((Get-StreamHash ($entry.Open())) -cne (Get-StreamHash ([IO.File]::OpenRead($file.FullName)))) {
            throw "Contenu différent : $($file.FullName)"
        }
    }
}

function Assert-LotsIdle([bool]$CheckGit, [bool]$CheckBuild) {
    if ($CheckBuild) {
        $busy = @(Get-Process -Name 'cargo','rustc','rust-analyzer','cl','cmake','ninja','msbuild','nvcc','whisper*','parakeet*' -ErrorAction SilentlyContinue)
        if ($busy.Count) { throw "Ferme les processus suivants : $($busy.ProcessName -join ', ')" }
    }
    if ($CheckGit) {
        if (Get-Process -Name 'git-lfs','git-index-pack','git-pack-objects','git-remote-*' -ErrorAction SilentlyContinue) {
            throw 'Une opération Git de transfert ou de pack est en cours.'
        }
        $writes = @(Get-CimInstance Win32_Process -Filter "Name = 'git.exe'" |
            Where-Object { $_.CommandLine -match '(?i)\s(fetch|pull|push|clone|gc|maintenance|repack|index-pack|pack-objects|receive-pack)\b' })
        if ($writes.Count) { throw 'Une opération Git susceptible de manipuler des packs est en cours.' }
        if (Get-ChildItem -LiteralPath (Join-Path $repository '.git\objects\pack') -Filter '*.lock' -Force) {
            throw 'Verrou détecté dans le dossier des packs Git.'
        }
    }
}

function Get-StageJob([string]$Root, [string]$Folder, [string]$Package, [string]$Prefix, [int]$Count) {
    $path = Join-Path $Root $Folder
    if (-not (Test-Path -LiteralPath $path)) { return }
    $null = Get-CheckedItem $path $true
    $zipPath = Join-Path $Root $Package
    $null = Get-CheckedItem $zipPath $false
    $files = @(Get-TreeFiles $path $false)
    if ($files.Count -ne $Count) { throw "Nombre de fichiers inattendu : $path" }
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try { Assert-ZipEntries $zip $path $files $Prefix }
    finally { $zip.Dispose() }
    [pscustomobject]@{
        Groupe = 'Assemblage'
        Chemin = $path
        Dossier = $true
        Octets = ($files | Measure-Object Length -Sum).Sum
        Copies = $files
        ZipPath = $zipPath
        Prefix = $Prefix
    }
}

function Save-TargetEvidence($Job) {
    $null = Get-CheckedItem $Job.Source $true
    $files = @(Get-TreeFiles $Job.Source $true)
    if ($files.Count -eq 0) { throw "Journaux absents : $($Job.Source)" }

    $worktree = $Job.Chemin.Substring(0, $Job.Chemin.IndexOf('\target\', [StringComparison]::OrdinalIgnoreCase))
    $archiveDir = Join-Path $worktree 'evidence\retention-compilation'
    $archiveParent = Split-Path -Parent $archiveDir
    if (Test-Path -LiteralPath $archiveParent) { $null = Get-CheckedItem $archiveParent $true }
    if (-not (Test-Path -LiteralPath $archiveDir)) { $null = [IO.Directory]::CreateDirectory($archiveDir) }
    $null = Get-CheckedItem $archiveDir $true
    $name = Split-Path -Leaf (Split-Path -Parent $Job.Chemin)
    $archive = Join-Path $archiveDir "$name-$([guid]::NewGuid().ToString('N')).zip"
    $partial = "$archive.partial"

    $zip = [IO.Compression.ZipFile]::Open($partial, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in $files) {
            $entry = [IO.Path]::GetRelativePath($Job.Source, $file.FullName).Replace('\', '/')
            [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $file.FullName, $entry, [IO.Compression.CompressionLevel]::Fastest)
        }
    }
    finally { $zip.Dispose() }

    $zip = [IO.Compression.ZipFile]::OpenRead($partial)
    try {
        $currentFiles = @(Get-TreeFiles $Job.Source $true)
        if ($zip.Entries.Count -ne $currentFiles.Count) { throw "Nombre d'entrées différent : $partial" }
        Assert-ZipEntries $zip $Job.Source $currentFiles ''
    }
    finally { $zip.Dispose() }
    [IO.File]::Move($partial, $archive)
    Write-Host "Archive vérifiée : $archive"
}

# Exécution
$before = (Get-PSDrive -Name C).Free
$jobs = @(
    if ('Git' -in $Groupe) {
        foreach ($copy in $gitCopies) {
            if (-not (Test-Path -LiteralPath $copy.Chemin)) { continue }
            $file = Get-CheckedItem $copy.Chemin $false
            if ($file.Length -ne $copy.Taille -or $file.LastWriteTimeUtc.Ticks -ne $copy.Modification) {
                throw "Pack temporaire modifié depuis l'audit : $($copy.Chemin)"
            }
            [pscustomobject]@{ Groupe = 'Git'; Chemin = $copy.Chemin; Dossier = $false; Octets = $file.Length; Pack = $copy }
        }
    }

    if ('P7R1' -in $Groupe) {
        $r1Zip = 'C:\wt\whisper-dual-cpu-r1\deliverables\distribution-duale-p7-r1-20261122\cpu\WhisperTurbo-Dual-CPU-P7-R1-retry-01-20261122.zip'
        $null = Get-CheckedItem $r1Zip $false
        Assert-FileHash $r1Zip '7247D7E04CE051AF1D1E08830CA701FC3EFA9062D1894C57FBFE42EF517E7437'
    }
    if ('P7R1' -in $Groupe -or 'Modeles' -in $Groupe) {
        foreach ($hash in $modelReferences.Keys) {
            $source = $modelReferences[$hash]
            $null = Get-CheckedItem $source $false
            Assert-FileHash $source $hash
        }
        foreach ($copy in $modelCopies) {
            if ($copy.Groupe -notin $Groupe -or -not (Test-Path -LiteralPath $copy.Chemin)) { continue }
            $file = Get-CheckedItem $copy.Chemin $false
            if ($file.LinkType) { throw "Lien physique ou symbolique inattendu : $($copy.Chemin)" }
            Assert-FileHash $copy.Chemin $copy.SHA256
            [pscustomobject]@{ Groupe = $copy.Groupe; Chemin = $copy.Chemin; Dossier = $false; Octets = $file.Length; SHA256 = $copy.SHA256 }
        }
    }

    if ('Assemblage' -in $Groupe) {
        Get-StageJob 'C:\dev\whisper\target\remise-cpu-actualisee-20261102' 'Whisper-CPU' 'Whisper-CPU-Windows-x64.zip' 'Whisper-CPU/' 28
        Get-StageJob 'C:\dev\whisper\target\remise-cpu-sans-cuda-20261102' 'Whisper-CPU' 'Whisper-CPU-Windows-x64.zip' 'Whisper-CPU/' 28
        Get-StageJob 'C:\wt\whisper-msix-cpu-spike\target\parakeet-tests\msix-cpu-prototype-20261122\package' 'staging' 'WhisperTurbo-CPU-prototype.msix' '' 24
        Get-StageJob 'C:\wt\whisper-msix-cpu-spike\target\parakeet-tests\msix-nvidia-prototype-20261123\package' 'staging' 'WhisperTurbo-NVIDIA-prototype.msix' '' 42
    }

    if ('PaquetP5' -in $Groupe) {
        $deliverables = 'C:\wt\distcpu-l2-reprise\deliverables'
        $name = 'WhisperTurbo-Dual-x64-P4-20261121-c08ba39d.zip'
        $source = Join-Path $deliverables "distribution-duale-p4-20261121\$name"
        $copyPath = Join-Path $deliverables "distribution-duale-p5-dell-20261121\01-Candidat-P4\$name"
        if (Test-Path -LiteralPath $copyPath) {
            $sha = 'B7580B7067D421DC88C665C61262261A6CF723EE28A7239671DA5F0847FBA848'
            $null = Get-CheckedItem $source $false
            $file = Get-CheckedItem $copyPath $false
            Assert-FileHash $source $sha
            Assert-FileHash $copyPath $sha
            [pscustomobject]@{ Groupe = 'PaquetP5'; Chemin = $copyPath; Dossier = $false; Octets = $file.Length; SHA256 = $sha }
        }
    }

    foreach ($path in $targetRoots) {
        $targetGroup = if ($path.StartsWith('C:\wt\ui-transcriber-impl\', [StringComparison]::OrdinalIgnoreCase)) { 'Interface' } else { 'Cibles' }
        if ($targetGroup -notin $Groupe -or -not (Test-Path -LiteralPath $path)) { continue }
        $null = Get-CheckedItem $path $true
        $scratch = Join-Path (Split-Path -Parent $path) 'scratch'
        $null = Get-CheckedItem $scratch $true
        if ($null -eq (Get-TreeFiles $scratch $true | Select-Object -First 1)) { throw "Aucune preuve : $scratch" }
        $measure = Get-TreeFiles $path $false | Measure-Object Length -Sum
        [pscustomobject]@{ Groupe = $targetGroup; Chemin = $path; Dossier = $true; Octets = $measure.Sum; Source = $scratch }
    }
)

if ($jobs.Count -eq 0) { Write-Host 'Aucun élément présent pour les groupes choisis.'; return }
foreach ($job in $jobs) {
    Write-Host ('[{0}] {1} ({2:N2} Gio logiques)' -f $job.Groupe, $job.Chemin, ($job.Octets / 1GB))
    foreach ($file in $job.Copies) { Write-Host "    $($file.FullName)" }
}
if (-not $Appliquer) { Write-Host 'Aperçu terminé. Aucune écriture ni suppression.'; return }
if ((Read-Host 'Si les lots sélectionnés sont clos et les chemins approuvés, saisis SUPPRIMER') -cne 'SUPPRIMER') {
    Write-Host 'Annulé.'
    return
}

$hasGit = 'Git' -in $jobs.Groupe
$hasBuild = @($jobs | Where-Object Groupe -ne 'Git').Count -gt 0
Assert-LotsIdle $hasGit $hasBuild
if ($hasGit) {
    & git --no-optional-locks -C $repository fsck --connectivity-only --no-dangling
    if ($LASTEXITCODE -ne 0) { throw 'Le contrôle de connectivité Git a échoué.' }
}
foreach ($job in $jobs) {
    if ($job.Source) { Save-TargetEvidence $job }
}
foreach ($job in $jobs) {
    Assert-LotsIdle ($job.Groupe -eq 'Git') ($job.Groupe -ne 'Git')
    $item = Get-CheckedItem $job.Chemin $job.Dossier
    if ($job.Pack -and ($item.Length -ne $job.Pack.Taille -or $item.LastWriteTimeUtc.Ticks -ne $job.Pack.Modification)) {
        throw "Pack modifié après l'aperçu : $($job.Chemin)"
    }
    if ($job.SHA256) {
        if ($item.LinkType) { throw "Lien physique ou symbolique inattendu : $($job.Chemin)" }
        Assert-FileHash $job.Chemin $job.SHA256
    }
    if ($job.Dossier) {
        if ($job.ZipPath) {
            $files = @(Get-TreeFiles $job.Chemin $false)
            if (Compare-Object ($job.Copies.FullName | Sort-Object) ($files.FullName | Sort-Object)) {
                throw "Liste des copies modifiée : $($job.Chemin)"
            }
            $null = Get-CheckedItem $job.ZipPath $false
            $zip = [IO.Compression.ZipFile]::OpenRead($job.ZipPath)
            try { Assert-ZipEntries $zip $job.Chemin $files $job.Prefix }
            finally { $zip.Dispose() }
        }
        else { Get-TreeFiles $job.Chemin $false | Out-Null }
        Remove-Item -LiteralPath $job.Chemin -Force -Recurse
    }
    else { Remove-Item -LiteralPath $job.Chemin -Force }
    if (Test-Path -LiteralPath $job.Chemin) { throw "Chemin encore présent : $($job.Chemin)" }
    Write-Host "Supprimé : $($job.Chemin)"
}
$after = (Get-PSDrive -Name C).Free
'C: {0:N2} Gio libres. Variation nette : {1:N2} Gio.' -f ($after / 1GB), (($after - $before) / 1GB)
