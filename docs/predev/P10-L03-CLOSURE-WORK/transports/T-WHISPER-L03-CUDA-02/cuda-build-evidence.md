# Addendum L03 — compilation native CUDA

Candidat strict inchangé : `candidate_fingerprint=3d442cfae5edcd5cb0fb15c3d8d4990820595ae146cc84640356a139bc9a5966`; HEAD/base `7a9198e8b1d71687f55e491fe02597b130afc2d9`. Empreinte avant/après la compilation VS2022 identique sur les 10 sorties L03.

## Référence de build et provenance

Configuration communiquée par l’utilisateur : Windows, GTX 1080 Ti, Pascal, sm_61, driver 581.29, CUDA Toolkit 12.9 / NVCC 12.9.86, CMake 4.4.4, VS Build Tools 2022 17.14, MSVC v143 / 19.44.35229.0, SDK 10.0.26100.0. Les logs de l’essai observent VS 17.14.41, toolset MSVC 14.44.35207, SDK 10.0.26100.0, CMake 4.4.4 et NVCC 12.9.86. `CUDA_PATH` et `CudaToolkitDir` étaient limités au processus de build. Aucun réglage système n’a été écrit.

Aucune configuration CUDA d’architecture centralisée n’a été trouvée dans les sources du dépôt. L’essai final n’a pas défini `CMAKE_CUDA_ARCHITECTURES`. Le `ggml-cuda.vcxproj` généré sélectionne `PlatformToolset=v143` et `--generate-code=arch=compute_61,code=[sm_61]`, soit sm_61. Le cache CMake indique le générateur `Visual Studio 17 2022`, plateforme x64, `GGML_CUDA=ON`, et NVCC du toolkit 12.9.

## Commandes et résultats

1. `nvcc --version` (processus configuré pour CUDA 12.9) : code 0.
2. `cargo check --locked -p whisper-worker-gpu --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation` avec le premier environnement MSVC 2019 : code 101. MSBuild/CMake ne résolvait pas le répertoire CUDA dans cette configuration. Essai historique, remplacé par la configuration de référence utilisateur.
3. `cargo build --locked --release -p whisper-worker-gpu --target x86_64-pc-windows-msvc --target-dir target/l03-closure-cuda-vs2022` avec VS2022 et `CMAKE_GENERATOR_TOOLSET=cuda=12.9` : code 101. L’invocation de `whisper-rs-sys` imposait `-Thost=x64`, en conflit avec la variable outil/cache `cuda=12.9`; CMake échouait avant l’identification du compilateur. Cause de configuration propre à l’intégration du crate, pas à l’hôte CUDA.
4. Même commande dans un répertoire de build neuf `target/l03-closure-cuda-vs2022-auto`, avec `CMAKE_GENERATOR_TOOLSET` retiré, `CUDA_PATH`/`CudaToolkitDir` sur 12.9 et l’intégration VS2022 installée : code 0. CMake a détecté le toolkit 12.9 et le build CUDA a réussi avec le toolset hôte x64/v143.

Commande finale exacte :

```powershell
cargo build --locked --release -p whisper-worker-gpu --target x86_64-pc-windows-msvc --target-dir target/l03-closure-cuda-vs2022-auto
```

Artefact : `target/l03-closure-cuda-vs2022-auto/x86_64-pc-windows-msvc/release/whisper-worker-gpu.exe`, 123904 octets, SHA-256 `2f4c522d10cdde111d05d0e71917f2bca4e6f805cc9139a2932b5f8e9faced41`.

Cache CMake (preuve locale) SHA-256 `96c8dd4fe5daed97192c6fd3264d12e919be6d0131fd94677c04cd4ff2b5f840`; projet `ggml-cuda.vcxproj` SHA-256 `824b4c439125b7cd9e554f166ae9a2c02c47d8806c3aca78c7d94ea61c199b45`.

Les sorties brutes des trois campagnes sont sous `logs/`. Les deux échecs précédents sont conservés pour expliquer la correction de configuration; seul le build release GPU VS2022 sans variable outil contradictoire est PASS. Aucune exécution du worker, inférence, smoke test GPU produit ou qualification de panne pilote n’a été réalisée : runtime GPU reste NOT RUN.
