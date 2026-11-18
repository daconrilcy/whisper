Avancée : les deux workers, le modèle et les chemins de la chaîne CUDA ont été vérifiés sur la station GTX 1080 Ti.
Blocage : la cible Cargo canonique du worktree manque. La garde exige son existence dès le préflight.
Décision proposée : autoriser sa création et un essai gardé du contrat de réutilisation sur le candidat T33, sous les conditions ci-dessous.

## Entrées vérifiées le 18 novembre 2026

Worktree `C:\wt\distcpu-l2-reprise`, branche `codex/cpu-distribution-l2-reprise`, base examinée `30362795`. Windows rapporte une NVIDIA GeForce GTX 1080 Ti et le pilote `32.0.15.8129`. Volume C avec environ 145 Gio disponibles. La comparaison porte sur le worker GPU release de référence utilisé pour le contrat historique et le worker GPU x64 livré avec le candidat T33. L’exemplaire retenu pour le candidat est dans un dossier de remise Snapdragon du worktree et son SHA-256 correspond à la [fiche du candidat](mandat-t42-windows-arm64.md).

Contenu exact du futur fichier `target/parakeet-tests/main-facades/values-n2-t33-gtx1080ti.json`. Les tailles et SHA-256 des trois fichiers ont été relus sur disque. La référence concorde avec le manifeste historique `snapshots/reusable-preparation-v8/TurboGpuReusableImports.json` des [preuves N2](../architecture/raw/n2-native/native-runs.zip).

```json
{
  "schemaVersion": 1,
  "profile": "TurboGpuReusableImports",
  "inputs": [
    {"key":"referenceExe","kind":"file","path":"C:/dev/whisper/target/parakeet-tests/main-facades/reference/gpu/release/whisper-worker-gpu.exe","bytes":20877824,"sha256":"e7927e833ac9e0ab97d9bd74a1c7aa90a4753ed2327710994a02e97da473d427"},
    {"key":"candidateExe","kind":"file","path":"C:/wt/distcpu-l2-reprise/deliverables/recette-arm64-diagnostic-20261118/paquet/app/whisper-worker-gpu.exe","bytes":20939264,"sha256":"d874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9"},
    {"key":"model","kind":"file","path":"C:/dev/whisper/models/ggml-large-v3-turbo.bin","bytes":1624555275,"sha256":"1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69"}
  ],
  "values": {
    "cudaToolkitPath":"C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.9",
    "cudaToolkitVersionedPath":"C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.9",
    "libclangPath":"C:/Users/cyril/AppData/Local/Programs/WhisperBuildDeps/libclang-22.1.8/runtimes/win-x64/native"
  }
}
```

Les deux variables CUDA de l’hôte désignent le dossier déclaré. Son `version.json` indique CUDA SDK 12.9.2. `bin/nvcc.exe`, `cudart64_12.dll`, `cublas64_12.dll` et `cublasLt64_12.dll` sont présents sous ce SDK. `LIBCLANG_PATH` désigne le dossier contenant `libclang.dll`, SHA-256 `c3c6799add679682580c9add26f73950463396b0997110f07eaf5578a25a09b8`. Les chemins de ces trois entrées et les chaînes de répertoires CUDA/libclang examinées sont exempts de points de réanalyse.

## Opérations soumises à accord

1. Depuis ce worktree, revérifier chemins, identités, état Git, absence de concurrence pertinente et espace disponible (minimum 30 Gio). Créer la cible dédiée `target/parakeet-tests/cargo-shared` après contrôle de son parent, puis écrire le JSON ci-dessus dans le lot, sans écraser de fichier divergent. Ce répertoire cible vide implique une compilation à froid du harnais Cargo et potentiellement de ses dépendances C/C++/CUDA. L’autorisation demandée couvre uniquement cette compilation induite par le profil N2. Les builds de production de DLL BUILD-NATIF demeurent suspendus.
2. Lancer le préflight ci-dessous. Sur code zéro, lancer une fois le même profil en mode Run. La garde initialise le cache hors ligne absent `main-facades/cargo-home` à partir du cache utilisateur lu comme source, utilise la cible canonique et isole sorties et temporaires. Les deux workers et le modèle sont des entrées en lecture seule. Aucun téléchargement, privilège supplémentaire, changement de garde ou remplacement du candidat n’est prévu.

```powershell
Set-Location -LiteralPath 'C:\wt\distcpu-l2-reprise'
$lot = 'target/parakeet-tests/main-facades'
$values = (Resolve-Path -LiteralPath "$lot/values-n2-t33-gtx1080ti.json").Path
$argsN2 = @('-NoProfile','-File','.\ci\guard-native.ps1','-Profile','TurboGpuReusableImports','-LotRoot',$lot,'-ValuesJson',$values)
powershell.exe @argsN2 -Mode Preflight
if ($LASTEXITCODE -ne 0) { throw 'Préflight N2 en échec.' }
powershell.exe @argsN2 -Mode Run
if ($LASTEXITCODE -ne 0) { throw 'Essai N2 en échec.' }
```

3. Accepter uniquement un `result.json` du run indiquant PASS pour l’unique cas `worker_contract::turbo_reusable_import_contract` réellement exécuté, avec ses journaux et les identités ci-dessus. Le harnais traite deux imports silencieux de 16 000 échantillons dans un même processus par exécutable, exige Ready CUDA puis End corrélés, une sortie normale, les traces de chargement et de réutilisation du modèle, et compare les événements des deux workers. Conserver le résultat et ses journaux selon la rétention du projet. Au premier écart de préflight, de compilation, de surveillance ou d’exécution, arrêter, préserver les preuves et rendre la cause avant toute reprise. Le verdict concerne la réutilisation CUDA du candidat, indépendamment des recettes de qualité et de vitesse déjà réalisées.
