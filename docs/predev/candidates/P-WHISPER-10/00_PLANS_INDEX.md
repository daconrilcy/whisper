# Plans Whisper — P-WHISPER-10

Statut : DRAFT soumis à une revue indépendante propre à P10. Base immuable : P-WHISPER-09, digest `92c049ae7e6e407d5f2f87d7526747fc2898bd3817a94068b7bbac6c00b42594`. Parent DESIGN : D-WHISPER-19, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

P10 révise le périmètre L03 après la réponse architecturale DETAIL-P02 acceptée par l’utilisateur. Il comprend le corpus hérité de P09, la source de la décision, les contrats L03, la baseline et l’environnement. P09 reste immuable et sa revue ne vaut pas revue de P10.

P06, P07 et P08 restent immuables. Leurs revues, contrôles et préflights sont historiques ; aucun ne vaut revue, promotion ou préflight P10. P10 exige son propre verdict indépendant sur son corpus exact.

## Lots et progression

L00/L01/L02 sont completed selon l’état contrôlé et leurs preuves ; L03–L07 restent planned. L02 livre journal/FIFO/récupération et MP3 jusqu’à TXT/SRT ; son exécution est enregistrée dans l’état canonique. Les validations futures L03 restent PROPOSED / NOT RUN. Le DAG reste séquentiel : DOCUMENT exige contrats/candidats/revues exacts, CODE exige API/sorties hashées, EXECUTION exige preuve de completion.

P10 élargit le plan L03 à CPAL/WASAPI, conversion 16 kHz, staging durable, contrôles de capacité et P02-GAP-BYTES-01. Cela n’autorise pas l’implémentation de L03. Toute vérification live reste NOT RUN jusqu’à une candidate exécutée.

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

HUMAN-USER a autorisé ces quinze chemins selon `sources/progress/USER_AUTHORIZATION_L02.md`, après revue du candidat PLANS applicable et préflight PASS. Avant édition : D19/P10 et revues exacts, railguard actif/équivalence, L01 completed/sorties vérifiées, DETAIL-P01 v3, Q-05 v1, Q-07 v1 et DETAIL-P02 v1 answered ; dépôt/environnement et gate applicable vérifiés. HUMAN-USER autorise uniquement les quinze chemins L02 établis ; P10 est une révision documentaire L03 et ne constitue pas l’autorisation d’implémenter ses 28 chemins. Limites mémoire conservées ; GPU hors L02 ; MP3 live L03.

## Carte

`01_LOTS.md` lots/ownership/AC ; `02_VERIFICATION_AND_PREFLIGHT.md` contrôles P10 ; `04_OPEN_DETAILS.md` réponses ; `05_SOURCE_AND_RULE_INDEX.md` sources/règles/provenance ; `06_L01_INTEGRATION_CONTRACTS.md` contrats hérités ; `07_BUILD_ENVIRONMENT_L01.md` et `build-environment-L01.json` environnement CPU historique ; `08_L02_INTEGRATION_CONTRACTS.md` frontières ; `09_L02_SOURCE_BASELINE.md` sources/SHA/provenance/futurs absents/preuve L01 ; `10_L03_INTEGRATION_CONTRACTS.md`, `11_L03_SOURCE_BASELINE.md`, `12_BUILD_ENVIRONMENT_L03.md` et `build-environment-L03.json` définissent le lot proposé et ses contrôles.


P10 élargit uniquement le lot L03 après acceptation utilisateur de DETAIL-P02. Toutes ses validations produit restent PROPOSED / NOT RUN. P09 demeure immuable et son verdict ne vaut pas revue de P10. Aucun code L03 n’est autorisé par ce plan.
