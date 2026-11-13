Avancée : Clippy frais terminé avec code de sortie 0 sur six crates.
Blocage : les workers Turbo `whisper-worker-cpu` et `whisper-worker-gpu` ne font pas partie de cette mesure.
Décision : 151 sites structurels restent dans le périmètre mesuré ; aucun nouveau site par rapport au cliquet.

# Rapport — alertes Clippy restantes

**Base :** `main` au commit `4889e916486bb13cb9998751fe0c47edfe351f2e`, worktree dédié `C:/wt/clippy-frais-20261113`, sources inchangées pendant la campagne.

**Commande gardée :** `cargo clippy` sur `whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common` et `whisper-worker-parakeet-cpu`, avec `--all-targets --message-format=json` et `-W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity`. Cible fraîche sous `target/parakeet-tests/refacto-reprise-import/targets/clippy-frais-20261113`. Exécution hors ligne/locked par `ci/guard-level1.ps1` ; code de sortie **0**.

## Résultat

Le journal JSON contient **210 émissions d’avertissement**. Après déduplication par crate, lint, fichier et ligne primaire, elles correspondent à **151 sites** : **61 en production** et **90 dans les tests**. Les émissions répétées entre cibles expliquent l’écart entre 210 et 151.

| Crate | `too_many_lines` production | `too_many_lines` tests | `cognitive_complexity` tests | Sites uniques |
|---|---:|---:|---:|---:|
| `whisper-adapters` | 51 | 37 | 25 | 113 |
| `whisper-core` | 4 | 10 | 2 | 16 |
| `whisper-desktop` | 3 | 8 | 1 | 12 |
| `whisper-worker-common` | 0 | 1 | 0 | 1 |
| `whisper-worker-parakeet-cpu` | 3 | 4 | 2 | 9 |
| `whisper-bootstrap` | 0 | 0 | 0 | 0 |
| **Total** | **61** | **60** | **30** | **151** |

Aucun avertissement autre que ces deux lints structurels n’est présent dans les diagnostics JSON Clippy. Aucun site `too_many_arguments` ni `type_complexity` n’a été émis. Le cliquet `CLIPPY_STRUCTURE_RATCHET` passe : **151 clés/sites contre 168 dans la baseline**, sans nouveau site détecté.

## Sites de production restants

Les valeurs sont les lignes Clippy observées sur cette base ; elles ne démontrent pas un défaut fonctionnel.

| Crate | Lint | Fichier:ligne | Mesure |
|---|---|---|---:|
| `whisper-adapters` | `too_many_lines` | `src/archive/cleanup/execution.rs:27` | 102/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/cleanup/proofs.rs:19` | 169/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/cleanup/proofs.rs:378` | 133/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/identity.rs:358` | 139/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/import.rs:249` | 256/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/live.rs:12` | 118/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/live.rs:463` | 169/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/publication.rs:135` | 144/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/publication/catalog.rs:428` | 107/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/publication/exports.rs:50` | 172/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/recovery.rs:39` | 266/100 |
| `whisper-adapters` | `too_many_lines` | `src/archive/recovery/live.rs:185` | 105/100 |
| `whisper-adapters` | `too_many_lines` | `src/capture.rs:451` | 137/100 |
| `whisper-adapters` | `too_many_lines` | `src/direct_live.rs:113` | 145/100 |
| `whisper-adapters` | `too_many_lines` | `src/direct_live/d2_corpus.rs:147` | 138/100 |
| `whisper-adapters` | `too_many_lines` | `src/direct_live/worker.rs:4` | 205/100 |
| `whisper-adapters` | `too_many_lines` | `src/gpu_inventory.rs:99` | 129/100 |
| `whisper-adapters` | `too_many_lines` | `src/microphone_probe.rs:583` | 130/100 |
| `whisper-adapters` | `too_many_lines` | `src/parakeet_confirmation.rs:176` | 143/100 |
| `whisper-adapters` | `too_many_lines` | `src/parakeet_import.rs:400` | 112/100 |
| `whisper-adapters` | `too_many_lines` | `src/parakeet_live/events.rs:4` | 232/100 |
| `whisper-adapters` | `too_many_lines` | `src/parakeet_live/inference.rs:17` | 342/100 |
| `whisper-adapters` | `too_many_lines` | `src/parakeet_live/inference.rs:376` | 114/100 |
| `whisper-adapters` | `too_many_lines` | `src/parakeet_live/lifecycle.rs:23` | 124/100 |
| `whisper-adapters` | `too_many_lines` | `src/source_binding.rs:101` | 223/100 |
| `whisper-adapters` | `too_many_lines` | `src/source_binding.rs:675` | 129/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/parakeet_unified/executor.rs:134` | 218/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/parakeet_unified/native.rs:283` | 174/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/duplicates.rs:96` | 112/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/duplicates.rs:216` | 199/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/effects.rs:4` | 227/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/import.rs:8` | 163/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/import.rs:329` | 127/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/import.rs:471` | 155/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/import.rs:922` | 111/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/import.rs:1175` | 132/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/import/worker_start.rs:20` | 162/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/import/worker_start.rs:190` | 254/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/import/worker_start.rs:494` | 163/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/live.rs:644` | 103/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/live.rs:822` | 120/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/poll_unified.rs:49` | 173/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/poll/capture.rs:108` | 125/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/poll/parakeet.rs:201` | 102/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/poll/parakeet.rs:975` | 114/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/queue.rs:4` | 156/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/queue.rs:192` | 220/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/source_binding.rs:97` | 136/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/runtime/source_binding.rs:322` | 122/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/service/mod.rs:412` | 184/100 |
| `whisper-adapters` | `too_many_lines` | `src/worker_ipc/service/mod.rs:664` | 143/100 |
| `whisper-core` | `too_many_lines` | `src/application/event_metadata.rs:5` | 108/100 |
| `whisper-core` | `too_many_lines` | `src/application/runtime/unified.rs:979` | 107/100 |
| `whisper-core` | `too_many_lines` | `src/engine_capabilities.rs:185` | 121/100 |
| `whisper-core` | `too_many_lines` | `src/gpu_inventory.rs:140` | 113/100 |
| `whisper-desktop` | `too_many_lines` | `src/root.rs:45` | 208/100 |
| `whisper-desktop` | `too_many_lines` | `src/root.rs:1458` | 117/100 |
| `whisper-desktop` | `too_many_lines` | `src/ui/model/view/unified.rs:145` | 103/100 |
| `whisper-worker-parakeet-cpu` | `too_many_lines` | `src/decoder.rs:25` | 118/100 |
| `whisper-worker-parakeet-cpu` | `too_many_lines` | `src/native_engine.rs:100` | 133/100 |
| `whisper-worker-parakeet-cpu` | `too_many_lines` | `src/session.rs:13` | 211/100 |

## Limites et références

Les alertes des tests sont comptées ci-dessus, mais ne sont pas proposées comme refactorisations. La mesure ne couvre pas `whisper-worker-cpu` ni `whisper-worker-gpu` : leurs alertes restent inconnues, la voie native demeurant suspendue. Aucun changement de code, de baseline ou de garde n’a été effectué ; aucun test, build natif ou essai dans la fenêtre Windows n’a été lancé.

Journal JSON brut : `C:/wt/clippy-frais-20261113/target/parakeet-tests/refacto-reprise-import/scratch/20261113T080726035-2731406849e147e4af2746e4c914c2ac/cargo-command.stdout.log` — SHA-256 `F79E9DCEC3D73084251F8719D5D5A6BAE01116F55F77D3F24A310A3D9343970C`.
