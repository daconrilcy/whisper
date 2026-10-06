# Préflight et vérifications — P-WHISPER-09

Toutes commandes/campagnes : PROPOSED / NOT RUN. Shell Developer PowerShell MSVC, cwd `C:\dev\whisper`, cible `x86_64-pc-windows-msvc`, Rust/Cargo 1.98.1, édition 2024. Base immuable P08 digest `d79bec1ab10a13e37adee551fe05cf834e43447617abccebdef6ca1460346e6f`; parent D19 digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

## Contrôle documentaire courant

Le manifeste P09 doit contenir le corpus hérité complet P07, corrections P08/P09 et snapshots. Vérifier D19 et P09 avec `verify-manifest`. Le verdict PLANS requis cible exactement P-WHISPER-09, D-WHISPER-19 et le digest courant ; aucune revue P06/P07/P08 ne le remplace. Ces antériorités sont historiques.

## Préflight L02

Vérifier revue indépendante P09 et promotion applicable ; autorisation HUMAN-USER sur les quinze chemins de `01_LOTS.md` ; railguard actif/attestation/équivalence D19 ; DETAIL-P01 v3, Q-05 v1 et Q-07 v1 answered ; L01 completed et outputs hashés ; aucun finding REQUIRED ouvert applicable. Inventorier `git rev-parse --show-toplevel`, `git rev-parse HEAD`, `git status --short`, `git diff --stat`. Comparer le code courant à `09_L02_SOURCE_BASELINE.md` puis aux outputs/ledger completed. Divergence inexpliquée : suspendre et retourner au coordinateur. Six fichiers futurs restent absents sans hash fictif. Préserver les changements ordinaires.

## Environnement CPU

Utiliser `build-environment-L01.json` manifesté sous P09 ; nom historique, aucune preuve PASS L02. Les méthodes de `07_BUILD_ENVIRONMENT_L01.md` s’appliquent.

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.1'
$env:RUSTUP_AUTO_INSTALL = '0'
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-build-environment --root C:\dev\whisper --matrix C:\dev\whisper\docs\predev\candidates\P-WHISPER-09\build-environment-L01.json
rustup target list --installed --toolchain 1.98.1
```

Attendu PASS, dépendances READY, target MSVC installée, SDK/linkage/libclang vérifiés. MISSING/NOT CHECKED bloque avant édition. Pas d’installation implicite.

## Gate L02

Le helper peut vérifier certaines sorties CODE sous docs puis sous code-root. Si le défaut demeure après vérification du helper, préparer une vue unifiée temporaire des références/sorties, comparer les hashes avant/après et préserver le dépôt original. Renseigner `$L02GateRoot` et `$L02GateState` depuis la vue et l’état P09 promu, quinze chemins/préconditions courants.

```powershell
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-state --root $L02GateRoot --code-root $L02GateRoot --state $L02GateState --lot L-WHISPER-02
```

Attendu `ok=true`, D19/P09 exacts, phase IMPLEMENTATION et préconditions valides. L’ancien gate 7 chemins et les états/revues P06/P07/P08 ne valent pas celui-ci. Le PASS documentaire n’est pas validation produit.

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
