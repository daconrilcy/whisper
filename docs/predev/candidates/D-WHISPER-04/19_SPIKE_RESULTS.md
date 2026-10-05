# Essais techniques autorisés — 2026-10-05

Mandat utilisateur : exécuter SPIKE-01 et SPIKE-02 en dehors du dépôt produit. Workspace d'essai : `%TEMP%\whisper-spikes`. Les sources, logs et hashes retenus sont dans `evidence/`. Il s'agit d'essais bornés et non de tests du futur produit.

## SPIKE-01 — résultat partiel

- Hôte : Windows x64, Rust/Cargo 1.98.1, MSVC Build Tools 2019, CMake/Ninja, NVIDIA GTX 1080 Ti capacité 6.1, pilote 581.29. `whisper-rs 0.16.0`; CUDA toolkit redistribuable 12.8.0 isolé en temp (nvcc 12.8.61), `libclang 18.1.1` dans un venv. Le build CUDA via générateur Visual Studio a échoué faute de CUDA toolset ; le build via Ninja et environnement `vcvars64` a réussi (`cuda-ninja-build.log.txt`).
- Modèle `ggml-large-v3-turbo.bin` : 1 624 555 275 octets, SHA-256 `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`; l'empreinte correspond au LFS officiel. Le SHA-1 `4af2b29d7ec73d781377bfd1758ca957a807e941` correspond au README upstream. Les assets complets sont dans `evidence/assets.json`.
- Même WAV local mono 16 kHz de 14,592 s : CPU release 14,5 s, GPU CUDA 1,99 s, CPU forcé dans binaire CUDA 13,4 s ; texte français identique sur cet échantillon. `gpu-run.log.txt` montre `CUDA0 backend`. Une exécution avec GPU masqué et mode demandé GPU a réussi sur CPU (`gpu-hidden.log.txt`) : `use_gpu=true` ne suffit donc pas à faire respecter le mode GPU forcé. Le contrat impose de vérifier le backend **effectif** avant de confirmer GPU, et d'arrêter/proposer CPU si GPU forcé est indisponible.
- Un paquet temporaire contenant EXE, modèle, `cublas64_12.dll`, `cublasLt64_12.dll`, `cudart64_12.dll` exécute CPU et GPU avec un PATH minimal sur le **même PC** (`package-cpu.log.txt`, `package-gpu.log.txt`). Sans ces DLL, l'EXE échoue au chargement (`cuda-binary-no-dll.log.txt`). Aucun installateur, aucune machine propre, aucune vérification de licence des binaires distribués ni premier lancement offline installé n'ont été testés.

**Statut : PARTIAL.** Le moteur Rust CPU/CUDA sur la cible est démontré pour un échantillon et un binaire portable. Auto→CPU, GPU forcé strict, installateur, licence/redistribution du paquet, ressources longues et modèle après installation restent à qualifier avant de clore SPIKE-01.

## SPIKE-02 — résultat partiel

- Protocole de fichiers versionnés avec marqueur `Complete` publié en dernier : 21 cas (18 sorties forcées de processus, 3 fins normales), couvrant live (3 artefacts), import (2 artefacts) et file, donnent `Recoverable` avant marqueur et `Complete` après ; altérer le texte force `Recoverable`. La source WAV importée est inchangée (`storage-report.json`, `storage-spike.py.txt`). Limite : arrêt de processus, pas coupure physique ; synchronisation du répertoire non démontrée sous Windows.
- Symphonia 0.5.5 lit un WAV mono 16 kHz et deux MP3 générés (CBR stéréo 44,1 kHz, VBR mono 16 kHz). Les durées de trames décodées MP3 dépassent la source de 36 et 96 ms, indiquant un besoin d'alignement du padding/horodatage (`formats-run.log.txt`, `formats-main.rs.txt`).
- Encodeur MP3 de sortie, récupération après panne électrique, saturation des canaux, worker/encodeur bloqué, mesure de perte du fragment ouvert, VAD, corpus qualité et délai live : **NOT RUN**. La décision produit est de mesurer sur corpus avant de fixer les seuils.

**Statut : PARTIAL.** Les résultats soutiennent le protocole envisagé mais ne démontrent pas la durabilité ni la qualité de bout en bout exigées par `AC-02/03/08/09/10/11/12/13`.

## Reprise des essais

Constituer un corpus local consentant avec parole FR/EN, silences, bruit, interruptions et longue durée ; mesurer VAD, perte du fragment non confirmé, délai live et qualité MP3 ; proposer les seuils à l'utilisateur. Compléter l'encodeur, l'installateur sur environnement propre et les scénarios de coupure/worker bloqué. Les chemins `%TEMP%` sont éphémères ; les logs et sources probantes sont copiés dans le corpus manifesté, mais les binaires et le modèle n'y sont pas inclus.
