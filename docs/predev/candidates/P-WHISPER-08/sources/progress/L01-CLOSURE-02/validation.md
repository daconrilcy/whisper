# Validation indépendante — candidat L01

**Verdict : CHECKS_PASSED** sur le fingerprint `ca361b9890ece948b804d2692625e38d0e2138493633d87014972d8d9343c5f5`. Les 21 hashes canoniques sont identiques avant/après campagne.

## Gates PASS

- `cargo test --locked --workspace --exclude whisper-worker-gpu -- --test-threads=1 --nocapture` avec fixtures D19 : suite CPU complète réussie; import WAV réel CPU attesté; Stop durant inférence; test source longue de 2 000 123 échantillons en fenêtres contiguës.
- Test memory feature-enabled sur candidat final : WAV généré de 640 123 échantillons, 9 inférences (8×80 000 + queue 123), exit 0, 164 484 ms d’import. PrivateBytes par étage : environ 1,639–1,640 GB avant/après et 2,491–2,584 GB pendant l’inférence. Le parcours est une caractérisation sans borne universelle. Résultat first-last +884 736 octets, cinq hausses/trois baisses dans cette exécution validator.
- `cargo fmt --all -- --check` PASS.
- Clippy strict complet PASS après suppression de la conversion inutile dans `main.rs:55`.
- Build feature-off worker/adapters PASS; arbres desktop/bootstrap sans `whisper-rs`/CUDA PASS.
- Builds release worker CPU, DesktopApp et bootstrap PASS. Le DesktopApp a utilisé `--target-dir target/validator-desktop` car un processus gardait le binaire standard ouvert.
- `cargo metadata --locked`, check `whisper-core` MSVC, frontières de features et `git diff --check` PASS.

## Limites

La requête de capacité pipe stdin du processus validator indépendant a renvoyé Access denied (Win32 5); stdout est mesuré à 65 536 octets par direction. La trace mémoire principale jointe, exécutée sur le même code à instrumentation inchangée par la correction Clippy, contient la mesure stdin worker-side à 65 536 octets par direction. Les allocations internes native/modèle/contexte et buffers OS demeurent non attribuées; aucune limite RSS universelle n’est revendiquée.

Le placement checker/policy n’existe pas dans le dépôt et n’a pas pu être exécuté. La variante GPU est hors périmètre L01. Aucun fichier source ou état du dépôt n’a été modifié par le validateur; ses traces supplémentaires sont sous TEMP et ses builds sous `target/`.

Les rapports d’échec antérieurs sur `8e8d…ac1b` sont historiques : le Clippy `useless_conversion` y a été corrigé, puis la présente campagne complète a passé sur `ca361…c5f5`.

## Commandes de validation finales

Toutes exécutées depuis `C:\dev\whisper` sous PowerShell, toolchain Rust 1.98.1 et cible Windows MSVC, sauf mention contraire. Les sorties détaillées restent dans l’historique d’exécution validator; la présente table est le relevé conservé.

| Gate | Commande exacte | Résultat |
|---|---|---|
| Workspace CPU | `cargo test --locked --workspace --exclude whisper-worker-gpu -- --test-threads=1 --nocapture` | exit 0 |
| Build feature-off | `cargo build --locked --no-default-features -p whisper-worker-cpu -p whisper-adapters --target x86_64-pc-windows-msvc` | exit 0 |
| Build worker instrumenté | `cargo build --locked --no-default-features --features l01-memory-qualification -p whisper-worker-cpu` | exit 0 |
| Test mémoire long | `cargo test --locked --no-default-features --features l01-memory-qualification -p whisper-adapters --test import_cpu generated_long_wav_runs_eight_full_native_windows_with_stage_memory_samples -- --exact --nocapture` | exit 0 |
| Format | `cargo fmt --all -- --check` | exit 0 |
| Clippy strict | `cargo clippy --locked -p whisper-core -p whisper-adapters -p whisper-worker-cpu -p whisper-desktop -p whisper-bootstrap --all-targets --all-features -- -D warnings` | exit 0 |
| Release worker | `cargo build --locked --release --no-default-features -p whisper-worker-cpu --target x86_64-pc-windows-msvc` | exit 0 |
| Release DesktopApp | `cargo build --locked --release --no-default-features -p whisper-desktop --target x86_64-pc-windows-msvc --target-dir target/validator-desktop` | exit 0 |
| Release bootstrap | `cargo build --locked --release --no-default-features -p whisper-bootstrap --target x86_64-pc-windows-msvc` | exit 0 |
| Metadata | `cargo metadata --locked --format-version 1 --no-deps` | exit 0 |
| Features desktop/bootstrap | `cargo tree --locked -p whisper-desktop -e features`; `cargo tree --locked -p whisper-bootstrap -e features` | exit 0; sans whisper-rs/CUDA |
| Features worker/adapters | `cargo tree --locked -p whisper-worker-cpu -e features`; `cargo tree --locked -p whisper-adapters -e features` | exit 0; feature qualification absente par défaut |
| Core MSVC | `cargo check --locked -p whisper-core --target x86_64-pc-windows-msvc` | exit 0 |
| Diff whitespace | `git diff --check` | exit 0 |

Le validator confirme que ces résultats sont liés au fingerprint final avant/après et n’a modifié ni sources ni état.
