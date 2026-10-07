# Préflight et vérifications courantes — P-WHISPER-15

P15 est DRAFT en attente de revue indépendante exacte. Base PLANS P14 digest `2724e325c5377d725b4e4f73541b6079a7bf08a9ff8e4328b10768e6823ca55b`; parent DESIGN D19 digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

Le registre courant indique D19 READY, P14 READY/CLEAN et L03 completed. La préparation de P15 autorise les corrections documentaires et leur revue indépendante. L’autorisation d’implémentation L04 demeure NOT_REQUESTED. Les campagnes produit/runtime L04 restent PROPOSED / NOT RUN.

## Gate documentaire P15

Vérifier les manifests exacts D19/P15, le parent DESIGN, la lignée P14→P15, les sources et le nouveau gel effectif du pack. Obtenir une revue indépendante sur le corpus complet P15, les changements CHANGE-P15-01..04, la représentation L04 et l’impact KEEP_D19 proposé. Aucun CLEAN P14 ne remplace ce verdict.

Après verdict admissible seulement, préparer le pending documentaire conservant D19, L00–L03 completed, L04 planned et les preuves historiques. Actualiser le lien de dépendance L04 vers la clôture canonique L03 actuelle. Ne pas créer d’autorisation de code L04 dans cet état.

Commandes proposées, NOT RUN par ce document : shell PowerShell, cwd `C:\dev\whisper`, Python -B; aucune cible Cargo ni donnée produit.

```powershell
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py verify-manifest --root C:\dev\whisper\docs\predev --manifest C:\dev\whisper\docs\predev\candidates\D-WHISPER-19\manifest.json
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py verify-manifest --root C:\dev\whisper\docs\predev --manifest C:\dev\whisper\docs\predev\candidates\P-WHISPER-15\manifest.json
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-state --root C:\dev\whisper\docs\predev --code-root C:\dev\whisper --state <pending-documentaire-P15>
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py promote-checkpoint --root C:\dev\whisper\docs\predev --code-root C:\dev\whisper --pending <pending-documentaire-P15> --current C:\dev\whisper\docs\predev\state.json
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-state --root C:\dev\whisper\docs\predev --code-root C:\dev\whisper --state C:\dev\whisper\docs\predev\state.json
```

L’hôte renseigne le chemin exact du pending réellement persisté. Attendu : manifests valides, état cohérent avec verdict exact, promotion seulement après contrôle, puis état actif vérifié. Ce contrôle documentaire utilise volontairement check-state sans `--lot` : la permission de coder L04 n’a pas encore été accordée.

## Préparation du préflight L04

Consigner l’inventaire réel des cinq chemins, HEAD/branche/status, les changements ordinaires préexistants, l’output CPU L01 et les quatre absences. Appliquer `14_L04_SOURCE_BASELINE_AND_PREFLIGHT.md` : baseline vide, ledger CPU L01 prouvé et current CPU correspondant; preuve des quatre absences dans repository_evidence. Aucun hash de fichier absent ni fichier vide de substitution.

Préparer la matrice d’environnement L04 consommable par check-build-environment et les preuves applicables CPU/GPU. Ne pas supposer que le succès CUDA L03 qualifie l’inférence GPU L04.

## Gate d’implémentation L04

Après autorisation explicite applicable aux cinq chemins : refaire manifests et preuves, vérifier P15 READY et sa revue exacte, réponses requises, railguard actif/équivalence, environnement réel, dépendance L03 et inventaire courant. Renseigner les préconditions canoniques à partir des preuves réelles puis exécuter :

```powershell
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-state --root C:\dev\whisper\docs\predev --code-root C:\dev\whisper --state <pending-preflight-L04> --lot L-WHISPER-04
```

Shell PowerShell, cwd `C:\dev\whisper`, Python -B; attendu ok=true sur état exact et permission ciblée. Un refus suspend l’édition. Toute intégration nécessaire hors allowlist exige CHANGE. Préserver état courant, pending, preuves, données et changements ordinaires; pas de reset global.

## Annexes historiques

Les procédures héritées ci-dessous conservent leur provenance et leur version. Elles ne déterminent ni le statut actif P15, ni son verdict, ni l’autorisation L04.


Le contenu hérité ci-dessous reproduit des procédures et commandes des candidats antérieurs P10/L01–L03. Ces sections sont conservées pour la traçabilité du corpus; elles ne constituent pas les commandes/gates courants P14/L04 et ne valent ni autorisation ni preuve d'exécution.
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

Elles restent PROPOSED / NOT RUN. Elles ne remplacent pas l’observation du widget dans la fenêtre réelle, l’injection d’un worker qui n’acquitte pas Stop, ni les essais de rotation/saturation. Le préflight courant exigera la revue exacte P14, Q-04/Q-06/Q-09 answered dans l’état promu, L03 completed, l’autorisation d’implémentation L04 pour l’allowlist exacte et le railguard actif. Ce document ne déclare aucun de ces gates franchis ni n’autorise le code.




