# Validation L03

**Statut: CHECKS_PASSED**

- Base: `7a9198e8b1d71687f55e491fe02597b130afc2d9`
- Candidate fingerprint: `3d442cfae5edcd5cb0fb15c3d8d4990820595ae146cc84640356a139bc9a5966`
- 10 sorties, fingerprint identique avant/après.
- CWD: `C:\dev\whisper`; Windows x64, Developer PowerShell / VS2019 BuildTools 16.11.51, MSVC 14.29.30133.
- Rust/Cargo 1.98.1; cible `x86_64-pc-windows-msvc`; features par défaut.
- Variables processus: RUSTUP_TOOLCHAIN=1.98.1, RUSTUP_AUTO_INSTALL=0, LIBCLANG_PATH libclang 22.1.8; CUDA_PATH absent sans affecter les checks prescrits.
- target-dir réutilisé sans nettoyage: `target/l03-closure-validation`.
- Logs complets: `target/l03-closure-validation-logs/20261007T093912-447f24a8/`.

| ID | argv | résultat | SHA-256 du log |
|---|---|---|---|
| FMT | `cargo fmt --all -- --check` | PASS, exit 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| CHECK | `cargo check --locked -p whisper-core -p whisper-adapters -p whisper-worker-cpu -p whisper-desktop --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation` | PASS, exit 0 | `050830b9078ce793abfbb99d6b2b04647b6769f99202aa7c8b71cae95192c229` |
| TEST-LIVE-CAPTURE | `cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation --test live_capture -- --nocapture` | PASS, exit 0 | `41d8fcc4f9c06b5a2cf8de26bb12025fccfe9080e7b5d8b1ad9dd75e812c8c0e` |
| TEST-VAD | `cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation --test vad_contract -- --nocapture` | PASS, exit 0 | `63414aab8e4855206ab35f1bec27074bfde3aa557d490717be18e3256cc36c0b` |
| TEST-ARCHIVE | `cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation --test live_archive -- --nocapture` | PASS, exit 0 | `3840a56a7773500663a98eb6857d2f9178dc21b5a644e0305fb23dc75898f5df` |
| TEST-CORE-LIVE | `cargo test --locked -p whisper-core --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation --test live_contract -- --nocapture` | PASS, exit 0 | `5c1ec209e20a879af149647c17abb8a7cf20e91802b0bca3afa8a770b6a71d6a` |
| TEST-WORKER-LIVE | `cargo test --locked -p whisper-worker-cpu --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation --test live_worker -- --nocapture` | PASS, exit 0 | `08c956baa1ffd788c6140cde560c29337f17cac7c6a83d7ef12a09d7ff201539` |
| TEST-DURABILITY | `cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation --test durability -- --nocapture` | PASS, exit 0 | `9ae35826d2e9b4f1ca8c191d6a86646e30c7aaf841c2b6908a74ac0a2c5beba1` |
| CLIPPY | `cargo clippy --locked -p whisper-core -p whisper-adapters -p whisper-worker-cpu -p whisper-desktop --all-targets --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation -- -D warnings` | PASS, exit 0 | `d81908fcf3ad942f083d853b01c2a5855ae51ea5ebf3abe1cf5219e7fcc9e803` |
| RELEASE | `cargo build --locked --release -p whisper-worker-cpu -p whisper-desktop --target x86_64-pc-windows-msvc --target-dir target/l03-closure-validation` | PASS, exit 0 | `835d0b552bf581682813ddaf4c11e56e44076d4fbab906967f34eb2017c70f9e` |
| DIFF | `git diff --check` | PASS, exit 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

Artefacts release:
- `target/l03-closure-validation/x86_64-pc-windows-msvc/release/whisper-desktop.exe`, 9,911,296 bytes, SHA-256 `7292b00c50c6a173e2eaf3b0fb299a4b9c9d6312e7a526aef4697435d48234cd`.
- `target/l03-closure-validation/x86_64-pc-windows-msvc/release/whisper-worker-cpu.exe`, 2,299,392 bytes, SHA-256 `6b4944571af1fee0320c2e3cb0f30a7044fb9cb77b940887222fc35e7afc62bc`.

Qualification micro réel / affichage natif / validation produit: NOT RUN.
