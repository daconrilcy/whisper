# Plans Whisper — P-WHISPER-24

Statut : DRAFT, candidat soumis à revue indépendante exacte. PLANS actif conservé avant promotion : P-WHISPER-14, digest `2724e325c5377d725b4e4f73541b6079a7bf08a9ff8e4328b10768e6823ca55b`. Parent DESIGN : D-WHISPER-19, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. P15 à P23 restent immuables; P19 à P23 ont reçu FINDINGS. P24 corrige la dernière commande courante qui ciblait P21.

Le registre actif `docs/predev/state.json` conserve D19 READY et P14 READY/CLEAN, L00–L03 completed, L04–L07 planned. P24 est sous `docs/predev/L03-P14-CLOSURE/candidates/P-WHISPER-24/`. `state.pending.P24.json` conserve P14 actif et passe le contrôle documentaire sans `--lot`; aucune promotion, préflight de lot ou implémentation n’est effectuée ici.

## Autorité, corrections et design

L’utilisateur demande d’atteindre l’étape où le préflight L04 sera possible et autorise une proposition élargissant les chemins et le DESIGN si nécessaire. Cette autorité couvre préparation et revues. L’autorisation d’implémentation reste NOT_REQUESTED. P22 conserve D19 et corrige l’incohérence de transfert vers le gate L04.

P19 a reçu P16-REQ-002/003 et P19-REQ-001; P20 puis P21 ont gardé P16-REQ-002 ouvert sur des instructions discordantes; P22 a corrigé ces mentions mais sa commande `verify-manifest` ciblait P21. P24 synchronise titres, chemin du candidat, manifeste, checkpoint et condition CLEAN→P24 READY dans les procédures courantes. Les responsabilités import/live continuent de suivre D19.

L’allowlist de quinze chemins, le protocole enfant GPU et les dépendances serde/sha2/rusty_mp3 ont été revus dans P19. Le correctif du contrôleur utilise la racine code uniquement pour `code_state.current`; les preuves `baseline.snapshot`/`ledger.evidence` restent sous docs. Le test utilise les documents uniques `snapshot-only.md` et `ledger-only.md`; 30 tests passent. La fidélité du pack et les snapshots exacts sont manifestés séparément du code produit.

## Allowlist exacte L04

L04 dépend de L03 EXECUTION; L05 dépend de L04 EXECUTION. Pas de parallélisation. Ces quinze chemins sont identiques à ceux de `01_LOTS.md` et `14_L04_SOURCE_BASELINE_AND_PREFLIGHT.md` :

- `crates/whisper-adapters/src/supervisor.rs`
- `crates/whisper-adapters/tests/worker_control.rs`
- `crates/whisper-core/tests/compute_policy.rs`
- `crates/whisper-worker-cpu/src/native_engine.rs`
- `crates/whisper-worker-gpu/src/native_engine.rs`
- `crates/whisper-adapters/src/lib.rs`
- `crates/whisper-adapters/src/worker_ipc.rs`
- `crates/whisper-core/src/application.rs`
- `crates/whisper-core/src/compute_policy.rs`
- `crates/whisper-core/src/lib.rs`
- `crates/whisper-desktop/src/root.rs`
- `crates/whisper-desktop/src/ui.rs`
- `crates/whisper-worker-gpu/src/main.rs`
- `crates/whisper-worker-gpu/Cargo.toml`
- `Cargo.lock`

L’extension Cargo GPU/root lockfile autorise uniquement l’ajout des dépendances nécessaires dans le worker GPU et leur graphe verrouillé. Aucun autre chemin n’est inféré; un besoin découvert en préflight exige CHANGE avant toute édition.

## Readiness et suite

Les commandes documentaires utilisent `C:\dev\whisper\docs\predev\L03-P14-CLOSURE`; le state canonique reste `docs/predev/state.json`. `state.pending.P24.json` conserve D19/P14 avant revue et référence les transports plan-writer P16, revue P16, résumé P17 non verbatim, contributions P19–P24 et revues P19–P22.

Le préflight L04 ne peut être envisagé qu’après revue exacte CLEAN de P24 puis validation et promotion d’un pending qui lie P24 READY comme PLANS actif, tout en conservant D19, L00–L03 et leurs preuves. Les procédures P14/P19–P22 sont historiques et ne satisfont pas ce gate. Toute autorisation d’implémentation distincte reste requise avant code. Aucun `check-state --lot L-WHISPER-04` ni campagne produit n’est exécuté ici.
