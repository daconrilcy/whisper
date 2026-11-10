Avancée : T8 est terminé et sa revue indépendante est CLEAN.
Blocage : poll_parakeet_worker reste à 20 ; T9 n’est pas engagé.
Décision : arrêter après T8, sans merge ni push.

# Suivi T8 — REFACTO-POLL-PARAKEET

Base T8 `27ee7330`, candidat `6537d2f8`, branche `refacto-poll-parakeet`. Seul `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs` a été modifié.

| Extraction | Lignes | Args | Complexité | Commit |
|---|---:|---:|---:|---|
| `handle_single_pass_parakeet_result` | 97 | 2 | 5 | `c418e978` |
| `handle_parakeet_result` | 11 | 2 | 2 | `6537d2f8` |

À chaque commit : `cargo test -p whisper-adapters` et corpus Q/D sous garde, Clippy JSON avec `recette-hooks`, N1 complet. Les deux N1 passent 944/0/129. Clippy du second candidat mesure `poll_parakeet_worker` à 20. Les références et assertions restent inchangées.

Revue indépendante T8 **CLEAN**, aucune finding ni gap. Le reviewer confirme Progress avant validation, ordre de corrélation/erreurs, gestion de `end`/shutdown/file et `pop_front`, record `ParakeetVadResult`, `gpu_evidence` séparée, voie VAD inchangée. Aucun changement timestamp, T9 ou hors périmètre.

T9 et T10 restent à autoriser. La recette Windows reste non exécutée. Aucun merge ni push.

Suivi T9 : [résultats et revue](REFACTO-POLL-PARAKEET-SUIVI-T9.md).
