# Baseline des sources L03 — P-WHISPER-10

HEAD observé lors de l’assemblage : f3a7f02db3867664016ceb5bcacef982de871c80 ; changements de travail L02 clôturés séparément. Les SHA ci-dessous seront générés au préflight sur la candidate exacte ; aucun SHA fictif n’est attribué. Fichiers futurs restent explicitement absents.

## Inventaire candidat L03

Hashes calculés sur l’arbre observé lors de l’assemblage P10 ; ceux des cinq sources modifiées reflètent le candidat L02 corrigé qui sera livré avec ce rapport. Les hashes absents ne sont pas inventés.

| Path | État | SHA-256 |
|---|---|---|
| `Cargo.lock` | present | `821b32dc3bc5a3c6f60b997772902e1af5de8029cdf8171c5292050ec4bf82aa` |
| `crates/whisper-adapters/Cargo.toml` | present | `917120ec0edf7725143a2d9d0c8405dbf82238ce87a3134b3df86c6af2f6011e` |
| `crates/whisper-adapters/src/archive.rs` | present | `f6bdff5a63c49796004164287834ca21c375ccceb592e7d479b1164cbdde3207` |
| `crates/whisper-adapters/src/capture.rs` | absent | — |
| `crates/whisper-adapters/src/journal.rs` | present | `824ea6734f4253cf6eb763036b1fbe8f164b7430fa692351868b3f0d6b867c5d` |
| `crates/whisper-adapters/src/lib.rs` | present | `7043670e6734355fac95cfea966275d89edc27a908cc77979204d01751f670af` |
| `crates/whisper-adapters/src/recovery.rs` | present | `94f5a4dc212f52a90211256b0c9681ee304e91aa9e4ecc2fc3f4fc0a650ef382` |
| `crates/whisper-adapters/src/staging.rs` | absent | — |
| `crates/whisper-adapters/src/vad.rs` | absent | — |
| `crates/whisper-adapters/src/worker_ipc.rs` | present | `58e3d22c598dd6c2ec7b276eb23e24224be1d8b7a83bea02069cd3ca69efe0c8` |
| `crates/whisper-adapters/tests/durability.rs` | present | `4b2f06db6630d4806de6842a62e365038280c24cb43cf5ceb064c39ae9a60fc9` |
| `crates/whisper-adapters/tests/live_archive.rs` | absent | — |
| `crates/whisper-adapters/tests/live_capture.rs` | absent | — |
| `crates/whisper-adapters/tests/vad_contract.rs` | absent | — |
| `crates/whisper-core/src/application.rs` | present | `50da488c497e0b026b0f283d98c4331d0a24e1723cbd44b25dac3eae55291229` |
| `crates/whisper-core/src/domain.rs` | present | `3b45f9852965311d58b4aa3b53ec6db8a5936b3a8933e27dd46ee76a754f7acd` |
| `crates/whisper-core/src/ipc.rs` | present | `535b5f453f3a613a0859dd85690317b8abedcef04c1f3c4ad45fdd551d956737` |
| `crates/whisper-core/src/lib.rs` | present | `c5c83ee62d7609a8a8f01bfc433b3a7a3af8aec642c7ff5306bd9973134827e6` |
| `crates/whisper-core/src/ports.rs` | present | `ba5abcba01689ba42f4cf1d761aa58aafbbfdfb1c097908b274e100780735bf8` |
| `crates/whisper-core/tests/live_contract.rs` | absent | — |
| `crates/whisper-core/tests/scheduler_contract.rs` | present | `133bd6a7a8a083f7896d5426aa9b967928b6c4d66309959a79d46cd8eb639845` |
| `crates/whisper-desktop/src/root.rs` | present | `9a3713112e3dfc3872e15e3cb581c7c6a53222e5f62864144ba4d111caa98599` |
| `crates/whisper-desktop/src/ui.rs` | present | `5ec778204dbbf7cc60a768dd7e701fd39a0fd3c92216479eb3d8f2d5d3ba45ba` |
| `crates/whisper-worker-cpu/Cargo.toml` | present | `c1181281797bf037e9ca35d6ee78927e6fc7d18af26b7445b76e54be0247fbe9` |
| `crates/whisper-worker-cpu/src/encoder.rs` | absent | — |
| `crates/whisper-worker-cpu/src/ipc.rs` | present | `1b06be095f4586f8e2cddc128fcc9e7307ab208d87323e18f97eddca97f06f97` |
| `crates/whisper-worker-cpu/src/main.rs` | present | `d04bd322d92501991d8be526b510a670c228161e6caa2c5e61940dd1894f2e9a` |
| `crates/whisper-worker-cpu/tests/live_worker.rs` | absent | — |


## Snapshots

Chaque source présente est figée octet pour octet dans `sources/L03-baseline/<chemin source>.txt`, avec SHA contrôlé par le manifeste P10. Les chemins absents restent documentés sans snapshot ni hash. La baseline L02 source historique reste séparée.
