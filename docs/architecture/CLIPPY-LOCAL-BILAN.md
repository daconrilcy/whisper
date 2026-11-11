Avancée : correction locale Clippy clôturée le 11 novembre 2026; revue indépendante CLEAN.
Blocage : aucun.
Décision : conserver les 25 résiduels hors structure attribués ci-dessous; zéro nouveau site.

# CLIPPY-LOCAL — bilan

## Mesures et validation

| Diagnostics distincts | Avant | Après |
|---|---:|---:|
| Hors structure | 96 (116 émissions) | 25 résiduels acceptés |
| Structure (`too_many_lines`, `cognitive_complexity`, `too_many_arguments`, `type_complexity`) | 248 | 248, inchangés |
| Total | 344 (438 émissions) | 273 (351 émissions) |

Clippy JSON, même clé de déduplication : **0 nouveau site**. Format, Clippy workspace/all-targets, Clippy GPU ciblé et `git diff --check` : PASS. Revue indépendante finale : **CLEAN**.

N1 gardé : **944 réussis / 0 échec / 129 ignorés**. Les 1 073 identités correspondent au run de référence; différence : **0**. CPU 12/0/9; Parakeet CPU 26/0/11; common 11/0/1. Contrôles PE : PASS. GPU `--list` seulement; corps GPU non exécuté.

## 20 diagnostics `dead_code` conservés

| Site signalé | Recommandation |
|---|---|
| `native_timing::TRACE` | Garder : trace conditionnelle. |
| `native_timing::TraceWriter` | Garder : trace conditionnelle. |
| `native_timing::Phase` | Garder : trace conditionnelle. |
| `native_timing::phase` | Garder : trace conditionnelle. |
| `native_timing::record` | Garder : appelé par catalogue/imports. |
| `native_timing::enabled` | Garder : trace conditionnelle. |
| `native_timing::trace_lock` | Garder : verrou de trace. |
| `native_timing::open_trace` | Garder : ouverture de trace. |
| `compile_failing_worker` (PE fixture adapters) | Garder : utilisé par tests unitaires; inclus sans appel dans deux cibles. |
| `decode_requested_block` (CPU) | Garder : session produit; harness `#[path]` factice. |
| `cached_cpu_engine` | Garder : consommé par la session CPU. |
| `CpuEngine::load` (harness CPU) | Garder : stub de frontière native. |
| `decode_requested_block` (GPU) | Garder : session produit; harness `#[path]` factice. |
| champs `model_path`, `model_sha256`, `candidate`, `engine` | Garder : lus par le cache GPU. |
| `cached_gpu_engine` | Garder : consommé par la session GPU. |
| `qualify` | Garder : qualification GPU produit. |
| `GpuEngine::{load, attest, matches_device}` (harness GPU) | Garder : stubs sans effet natif. |
| `cuda_devices` (harness GPU) | Garder : stub sans accès CUDA. |
| `ggml_devices` (harness GPU) | Garder : stub de frontière native. |
| `whisper_gpu_device_index` (harness GPU) | Garder : stub de frontière native. |

**Écart accepté à « dead_code : analyse seulement » :** retrait du bouchon `verify_windows_identity` dans `crates/whisper-worker-gpu/tests/support/autonomous_harness.rs`. Les fonctions de production `native_ffi.rs:333/378` et leurs appels restent inchangés.

Deux imports (`MAX_WINDOW_SAMPLES`, `OUTPUT_RATE`) restent requis par les sessions CPU/GPU; les harness privés seuls les signalent. Trois lints polling (`large_enum_variant` ×2, `result_large_err` ×1) restent pour éviter des allocations sur le parcours fréquent.

Journaux gardés dans `docs/architecture/raw/CLIPPY-LOCAL-20261111.zip` : 143 fichiers, 689 999 octets; arborescence, nombre et SHA-256 vérifiés. Validation Windows manuelle : NON EXÉCUTÉE.
