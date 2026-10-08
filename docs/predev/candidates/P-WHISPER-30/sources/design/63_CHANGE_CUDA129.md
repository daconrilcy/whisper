# CHANGE-CUDA129-01 — référence native L04

Contribution technique attribuée à AUTH-ARCH-CUDA129, transport T-WHISPER-ARCH-CUDA129-01. Date : 2026-10-08. Base : D-WHISPER-28, digest 50e2ab1ac3afd17d9e4a1e51932aaa37f812e3162db0207626656108352a6911. Cette proposition D29 reste DRAFT jusqu'à sa revue indépendante.

## Décision et précédence

La réponse utilisateur conservée dans USER_AUTHORIZATION_L04_EXTENSION_CUDA129.md autorise CUDA 12.9 pour L04. CHANGE-CUDA129-01 v1, nature decision, approval accepted par l'utilisateur, remplace la valeur CUDA 12.8 de TECH-D18-01. TECH-D18-01 v2 retient CUDA 12.9 Update 1 et NVCC 12.9.86 pour le worker GPU. Le worker CPU reste séparé et sans CUDA ; le parent, l'UI et l'installateur restent sans moteur natif. La référence EULA de TECH-D18-02 v2 devient CUDA 12.9 Update 1. Les références 12.8 de D28 et des essais antérieurs demeurent des preuves historiques.

Ce delta a priorité sur les deux lignes TECH-D18-01/02 de 50_ARCH_DECISIONS_D18.md pour L04. Il ne change ni les frontières, ports, IPC, formats persistés, publication, modèle et hash, ni la migration des données. Le payload futur doit inventorier et hasher ses DLL réellement livrées, conserver les notices applicables et laisser nvcuda.dll au pilote.

## Détail technique et preuve

DETAIL-CUDA129-ENV-01 v1 retient la voie observée Visual Studio 2022 x64, CMake 4.4.4, MSVC 14.44.35207, CUDA_PATH et CudaToolkitDir vers CUDA 12.9. CMAKE_GENERATOR_TOOLSET doit être absent : la valeur cuda=12.9 a causé un conflit avec -Thost=x64. Rust/Cargo 1.98.1, RUSTUP_TOOLCHAIN explicite, auto-install désactivé, LIBCLANG_PATH valide et cible installée restent requis. Ninja est facultatif pour cette voie Visual Studio, requis seulement si une voie Ninja est retenue explicitement.

Une campagne L03 distincte a réussi le build GPU release sur la même pile whisper-rs 0.16.0 / whisper-rs-sys 0.15.0 avec NVCC 12.9.86 et cible sm_61. Source : docs/predev/L03-P14-CLOSURE/transports/T-WHISPER-L03-CLOSURE-04/cuda-build-evidence.md, SHA-256 a8035bed5ea41c390e83ec30aaffe4a15c4091abc8f3fca34c7f36d932a917d8. Son EXE SHA-256 est 2f4c522d10cdde111d05d0e71917f2bca4e6f805cc9139a2932b5f8e9faced41. Cette campagne n'a pas exécuté d'inférence GPU.

Sources primaires consultées par l'architecte le 2026-10-08 : NVIDIA CUDA 12.9 Update 1 release notes (https://docs.nvidia.com/cuda/archive/12.9.1/cuda-toolkit-release-notes/index.html), installation Windows (https://docs.nvidia.com/cuda/archive/12.9.1/cuda-installation-guide-microsoft-windows/index.html) et EULA (https://docs.nvidia.com/cuda/archive/12.9.1/eula/index.html). La preuve locale du build prime pour la configuration MSVC exacte.

## Risque et fermeture

RISK-T-CUDA129-01 : la compilation démontrée ne qualifie pas le runtime ni le package 12.9. DESIGN_FEASIBILITY : build même pile PASS historique et ciblage Pascal 12.x documenté. Préflight du successeur L04 : NOT RUN. PRODUCT_VALIDATION : NOT RUN pour inférence GPU, attestation, GPU strict absent, Auto, générations et Stop. DELIVERY_QUALIFICATION : NOT RUN pour imports PE, DLL privées et modules chargés, notices, VC runtime et package offline.

Le plan successeur P30 parent D30 doit porter la matrice CUDA 12.9, le chemin VS2022 démontré, les commandes et les limites de ces preuves. RG-09/RG-10 demeurent actifs ; leurs hashes et attestation seront revérifiés au préflight. Aucune activation nouvelle du railguard n'est proposée.
