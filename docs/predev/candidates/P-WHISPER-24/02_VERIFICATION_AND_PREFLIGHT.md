# Préflight et vérifications — P-WHISPER-24

Statut DRAFT. Base active P14 READY/CLEAN et parent D19 READY, exactement contrôlés. Le candidat canonique se trouve sous `docs/predev/L03-P14-CLOSURE/candidates/P-WHISPER-24/`; la racine documentaire hôte est `C:\dev\whisper\docs\predev\L03-P14-CLOSURE`. Les chemins de manifestes sont relatifs à cette racine. P17 FINDINGS et P18 non revu ne transfèrent aucun verdict. L’autorisation porte sur la préparation/revue, pas sur le code L04.

## Contrôle documentaire courant P24

```powershell
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py verify-manifest --root C:\dev\whisper\docs\predev\L03-P14-CLOSURE --manifest C:\dev\whisper\docs\predev\L03-P14-CLOSURE\candidates\P-WHISPER-24\manifest.json
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-state --root C:\dev\whisper\docs\predev\L03-P14-CLOSURE --code-root C:\dev\whisper --state C:\dev\whisper\docs\predev\L03-P14-CLOSURE\state.pending.P24.json
```

Le second contrôle est documentaire (`--lot` absent), garde D19/P14 actifs et exige les références raw contributions vérifiées. Il ne constitue pas le préflight L04. Vérifier également le checkpoint pending et les hashes des transports. En cas d’échec ou de chemin supplémentaire requis, ne pas promouvoir ni coder.

## Futur préflight L04

Après CLEAN exact de P24 et promotion contrôlée d’un checkpoint liant P24 READY comme PLANS actif, avec D19 et L00–L03 conservés, un mandat code L04 distinct couvrant les quinze chemins sera requis. Le préflight recapture HEAD/status, railguard, environnement, L03, les manifests Cargo et l’allowlist, puis exécute `check-state --lot L-WHISPER-04`. Une divergence impose arrêt et retour au cadrage. Aucun préflight de lot, lot check-state, build ou test produit n’est exécuté dans ce candidat.

Les campagnes live/import CPU/GPU, protocole enfant, saturation, Stop/Quitter et UI native restent PROPOSED / NOT RUN.

## Procédures historiques héritées — P10

Toutes commandes/campagnes : PROPOSED / NOT RUN. Shell Developer PowerShell MSVC, cwd `C:\dev\whisper`, cible `x86_64-pc-windows-msvc`, Rust/Cargo 1.98.1, édition 2024. Base immuable P08 digest `d79bec1ab10a13e37adee551fe05cf834e43447617abccebdef6ca1460346e6f`; parent D19 digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

## Contrôle documentaire courant

Le manifeste P10 doit contenir le corpus hérité complet P07, corrections P08/P10 et snapshots. Vérifier D19 et P10 avec `verify-manifest`. Le verdict PLANS requis cible exactement P-WHISPER-10, D-WHISPER-19 et le digest courant ; aucune revue P06/P07/P08 ne le remplace. Ces antériorités sont historiques.

## Préflight L02

Vérifier revue indépendante P10 et promotion applicable ; autorisation HUMAN-USER sur les quinze chemins de `01_LOTS.md` ; railguard actif/attestation/équivalence D19 ; DETAIL-P01 v3, Q-05 v1 et Q-07 v1 answered ; L01 completed et outputs hashés ; aucun finding REQUIRED ouvert applicable. Inventorier `git rev-parse --show-toplevel`, `git rev-parse HEAD`, `git status --short`, `git diff --stat`. Comparer le code courant à `09_L02_SOURCE_BASELINE.md` puis aux outputs/ledger completed. Divergence inexpliquée : suspendre et retourner au coordinateur. Six fichiers futurs restent absents sans hash fictif. Préserver les changements ordinaires.

## Environnement CPU

Utiliser `build-environment-L01.json` manifesté sous P10 ; nom historique, aucune preuve PASS L02. Les méthodes de `07_BUILD_ENVIRONMENT_L01.md` s’appliquent.

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.1'
$env:RUSTUP_AUTO_INSTALL = '0'
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-build-environment --root C:\dev\whisper --matrix C:\dev\whisper\docs\predev\candidates\P-WHISPER-10\build-environment-L01.json
rustup target list --installed --toolchain 1.98.1
```

Attendu PASS, dépendances READY, target MSVC installée, SDK/linkage/libclang vérifiés. MISSING/NOT CHECKED bloque avant édition. Pas d’installation implicite.

## Gate L02

Le helper peut vérifier certaines sorties CODE sous docs puis sous code-root. Si le défaut demeure après vérification du helper, préparer une vue unifiée temporaire des références/sorties, comparer les hashes avant/après et préserver le dépôt original. Renseigner `$L02GateRoot` et `$L02GateState` depuis la vue et l’état P10 promu, quinze chemins/préconditions courants.

```powershell
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-state --root $L02GateRoot --code-root $L02GateRoot --state $L02GateState --lot L-WHISPER-02
```

Attendu `ok=true`, D19/P10 exacts, phase IMPLEMENTATION et préconditions valides. L’ancien gate 7 chemins et les états/revues P06/P07/P08 ne valent pas celui-ci. Le PASS documentaire n’est pas validation produit.

## Campagnes après code

```powershell
cargo fmt --all -- --check
cargo test --locked -p whisper-core --target x86_64-pc-windows-msvc --test scheduler_contract -- --nocapture
cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test durability -- --nocapture
cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test import_mp3 -- --nocapture
cargo clippy --locked -p whisper-core -p whisper-adapters -p whisper-worker-cpu -p whisper-desktop -p whisper-bootstrap --all-targets -- -D warnings
cargo build --locked --release -p whisper-worker-cpu --target x86_64-pc-windows-msvc --target-dir target/cpu
cargo build --locked --release -p whisper-desktop -p whisper-bootstrap --target x86_64-pc-windows-msvc --target-dir target/desktop
git diff --check
```

Cargo séquentiel. V-SCHEDULER couvre FIFO/ACK/choix/absence de départ et exclusion live/import ; V-DURABLE interruptions, source et préfixe confirmé ; V-IMPORT-MP3 vrai worker CPU/modèle jusqu’à TXT/SRT, Q-07, langue, SHA/chronologie, annulation/crash ; V-UI vraie fenêtre. Toutes PRODUCT_VALIDATION / NOT RUN. GPU hors L02.

## Preuves/reprise

Conserver candidat exécuté, commandes/cwd/cible/features, fixtures/hash, environnement, stimuli, effets, stdout/stderr/code et captures attribuées. Checks, revue et qualification restent distincts. Sur échec préserver pending/journal/source, scanner puis attendre choix explicite ; pas de reset global ni suppression de source. Arrêt de processus ne prouve pas toute panne électrique. Limites mémoire/FFI existantes conservées.


## DETAIL-P02 / P02-GAP-BYTES-01 — L03

La réponse DETAIL-P02 est acceptée : CPAL 0.18.2/WASAPI, PCM16 mono 16 kHz, rubato 0.16.2, webrtc-vad 0.4.0 mode 1 à 320 échantillons/20 ms, fenêtre d’inférence maximale de 80 000 échantillons, callback borné à 100 slots, staging PCM durable et cessation explicite à saturation. Les campagnes restent NOT RUN.

P02-GAP-BYTES-01 (PRODUCT_VALIDATION / NOT RUN) exige d’inventorier/mesurer les événements persistants globaux, les buffers wire et allocations locales Queue/History, y compris files dépassant capacité et rétropression. Si nécessaire, admission bornée ou pagination par curseurs sans éviction ni troncature silencieuse des entrées durables. Mesurer la mémoire résidente séparément : les budgets wire ne bornent pas le RSS. Vérification indépendante requise après exécution.


## Environnement / préflight L03

Avant édition, utiliser `build-environment-L03.json` avec `check-build-environment`, vérifier target MSVC, WASAPI/CPAL, rust-toolchain/Cargo.lock, CMake, MSVC et LIBCLANG_PATH. Enregistrer versions, chemins, résultats exacts et hashes. Toute dépendance absente bloque sans installation implicite. Vérifier manifestes D19/P10 et `check-state --lot L-WHISPER-03`, Q-07/DETAIL-P02 acceptés, L02 completed et les 28 chemins stricts. Capturer `git status`, HEAD et hashes baseline `11_L03_SOURCE_BASELINE.md`; une divergence arrête le préflight.

## Campagnes L03 proposées (toutes NOT RUN)

```powershell
cargo fmt --all -- --check
cargo check --locked -p whisper-core -p whisper-adapters -p whisper-worker-cpu -p whisper-desktop --target x86_64-pc-windows-msvc
cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test live_capture -- --nocapture
cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test vad_contract -- --nocapture
cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test live_archive -- --nocapture
cargo test --locked -p whisper-core --target x86_64-pc-windows-msvc --test live_contract -- --nocapture
cargo test --locked -p whisper-worker-cpu --target x86_64-pc-windows-msvc --test live_worker -- --nocapture
cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test durability -- --nocapture
cargo clippy --locked -p whisper-core -p whisper-adapters -p whisper-worker-cpu -p whisper-desktop --all-targets -- -D warnings
cargo build --locked --release -p whisper-worker-cpu -p whisper-desktop --target x86_64-pc-windows-msvc
git diff --check
```

L03-AC/live UI, real microphone, long-running capture, CPU backlog, disk-full/device-loss and P02-GAP-BYTES-01 require explicit fixtures/stimuli and observable evidence. These proposed commands do not alone close product validation. Preserve all input/audio/job data on failure; no destructive cleanup.


## Détails et vérifications attendues L04 — proposition héritée

Q-04/Q-06/Q-09 sont résolus comme contrats dans `13_L04_DETAIL_CONTRACTS.md`. Après revue PLANS indépendante, promotion du checkpoint et autorisation séparée, les contrôles L04 ciblent les chemins existants de `01_LOTS.md`. Vérifications de contrat proposées :

```powershell
cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test worker_control -- --nocapture
cargo test --locked -p whisper-core --target x86_64-pc-windows-msvc --test compute_policy -- --nocapture
git diff --check
```

Elles restent PROPOSED / NOT RUN. Elles ne remplacent pas l’observation du widget dans la fenêtre réelle, l’injection d’un worker qui n’acquitte pas Stop, ni les essais de rotation/saturation. Le préflight L04 exigera la revue exacte CLEAN de P24 et son checkpoint promu; P14 est la base actuelle jusqu’à promotion; les étapes P20/P21/P22/P23 sont historiques et FINDINGS, Q-04/Q-06/Q-09 answered dans l’état promu, L03 completed, l’autorisation d’implémentation L04 pour l’allowlist exacte et le railguard actif. Ce document ne déclare aucun de ces gates franchis ni n’autorise le code.




