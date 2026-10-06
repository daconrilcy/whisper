# Plans Whisper — P-WHISPER-09

Statut : DRAFT soumis à une revue indépendante propre à P09. Base de correction immuable : P-WHISPER-08, digest `d79bec1ab10a13e37adee551fe05cf834e43447617abccebdef6ca1460346e6f`. Parent DESIGN : D-WHISPER-19, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

P09 corrige l’identité des contrôles, la continuité des sources et le corpus manifesté. Son manifeste comprend le corpus complet hérité de P07, les corrections documentaires P08 et les snapshots/preuves incorporés. La base P08 ne dispense pas d’inclure les sources héritées utiles.

P06, P07 et P08 restent immuables. Leurs revues, contrôles et préflights sont historiques ; aucun ne vaut revue, promotion ou préflight P09. P09 exige son propre verdict indépendant sur son corpus exact.

## Lots et progression

L00/L01 sont completed selon l’état contrôlé et leurs preuves ; L02–L07 restent planned. L02 livre journal/FIFO/récupération et MP3 réel jusqu’à TXT/SRT. Les campagnes futures restent PROPOSED / NOT RUN. Le DAG reste séquentiel : DOCUMENT exige contrats/candidats/revues exacts, CODE exige API/sorties hashées, EXECUTION exige preuve de completion.

## Allowlist exacte L02

```text
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/decoder.rs
crates/whisper-adapters/src/journal.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/queue_store.rs
crates/whisper-adapters/src/recovery.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/tests/durability.rs
crates/whisper-adapters/tests/import_mp3.rs
crates/whisper-core/src/application.rs
crates/whisper-core/src/ports.rs
crates/whisper-core/tests/scheduler_contract.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-worker-cpu/src/decoder.rs
```

Application possède admission/scheduler/choix/identité ; ports définit les contrats ; adaptateurs exécutent persistance/scan/IO ; worker CPU possède codec/PCM ; root compose, UI consomme. Aucun scheduler parallèle. Les features codec sont déjà présentes. Aucun changement Cargo/lockfile prévu.

## Autorisation et transfert

HUMAN-USER a autorisé ces quinze chemins selon `sources/progress/USER_AUTHORIZATION_L02.md`, après revue du candidat PLANS applicable et préflight PASS. Avant édition : D19/P09 et revues exacts, railguard actif/équivalence, L01 completed/sorties vérifiées, DETAIL-P01 v3, Q-05 v1, Q-07 v1 answered, dépôt et environnement vérifiés, gate L02 PASS. Limites mémoire conservées ; GPU hors L02 ; MP3 live L03.

## Carte

`01_LOTS.md` lots/ownership/AC ; `02_VERIFICATION_AND_PREFLIGHT.md` contrôles P09 ; `04_OPEN_DETAILS.md` réponses ; `05_SOURCE_AND_RULE_INDEX.md` sources/règles/provenance ; `06_L01_INTEGRATION_CONTRACTS.md` contrats hérités ; `07_BUILD_ENVIRONMENT_L01.md` et `build-environment-L01.json` environnement CPU historique ; `08_L02_INTEGRATION_CONTRACTS.md` frontières ; `09_L02_SOURCE_BASELINE.md` sources/SHA/provenance/futurs absents/preuve L01.
