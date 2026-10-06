# Validation attribuée — L-WHISPER-02

Validator : `/root/validate_l02`, validation indépendante, 2026-10-06. Candidat P09 exact : fingerprint `6b84a31709416ba57a5f5c52671039da89cad3fb12a351c56f11c9bcb06fa767`. Le hash des dix sorties est resté stable avant et après les commandes. Cible `x86_64-pc-windows-msvc`, features Cargo par défaut; les commandes Cargo ont été exécutées séquentiellement. `LIBCLANG_PATH` et CMake ont été ajoutés au seul environnement de processus des commandes qui en avaient besoin.

Verdict de validation : **CHECKS_PASSED** pour les commandes L02 prescrites ci-dessous.

| ID | Commande | Résultat | Log |
|---|---|---|---|
| FMT | `cargo fmt --all -- --check` | PASS | `logs/01-cargo-fmt.log` |
| CHECK | `cargo check --locked -p whisper-core -p whisper-adapters -p whisper-worker-cpu -p whisper-desktop --target x86_64-pc-windows-msvc` | PASS | `logs/02-cargo-check-windows.log` |
| SCHEDULER | `cargo test --locked -p whisper-core --target x86_64-pc-windows-msvc --test scheduler_contract -- --nocapture` | PASS, 4/4 | `logs/03-test-scheduler-contract.log` |
| DURABILITY | `cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test durability -- --nocapture` | PASS, 2/2 | `logs/04-test-durability.log` |
| IMPORT-MP3-FIXTURE | `cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test import_mp3 -- --nocapture` avec `WHISPER_L02_MP3_FIXTURE` process-local | PASS, 3/3 | `logs/05-test-import-mp3-fixture.log` |
| CLIPPY | `cargo clippy --locked -p whisper-core -p whisper-adapters -p whisper-worker-cpu -p whisper-desktop -p whisper-bootstrap --all-targets -- -D warnings` | PASS | `logs/06-cargo-clippy-all-targets.log` |
| BUILD-CPU | `cargo build --locked --release -p whisper-worker-cpu --target x86_64-pc-windows-msvc --target-dir target/cpu` | PASS | `logs/07-cargo-build-cpu-release.log` |
| BUILD-DESKTOP | `cargo build --locked --release -p whisper-desktop -p whisper-bootstrap --target x86_64-pc-windows-msvc --target-dir target/desktop` | PASS | `logs/08-cargo-build-desktop-bootstrap-release.log` |
| DIFF-CHECK | `git diff --check` | PASS | `logs/09-git-diff-check.log` |

La fixture MP3 vaut 7 017 octets, SHA-256 `98fe334dbcde589b35dce8ce88396960cdcf58165524c576f29f368e27ab3e38`.

**Limite explicite :** la partie du test MP3 qui exige le modèle D19 a renvoyé `V-IMPORT-MP3 NOT RUN`, car `WHISPER_L01_MODEL` n’était pas défini et le modèle n’est plus présent dans le profil local. Aucun résultat n’est revendiqué pour cette branche; la campagne de modèle D19 et qualification produit restent NOT RUN. Les tests avec fixture, contrôles compilation/lint/build ci-dessus sont PASS.

Les journaux sont les sorties brutes attribuées au validateur, copiées de son répertoire Temp et vérifiées par SHA-256. Aucun code produit, état ou configuration globale n’a été modifié par le validateur.
