Avancée : T9 est terminé et sa revue indépendante est CLEAN.
Blocage : T10 et sa validation finale complète ne sont pas engagés.
Décision : arrêter après T9, sans merge ni push.

# Suivi T9 — REFACTO-POLL-PARAKEET

Base T9 `6537d2f8`, candidat `cc273e9d`, branche `refacto-poll-parakeet`. Seul `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs` a été modifié.

| Extraction | Lignes | Arguments | Complexité | Commit |
|---|---:|---:|---:|---|
| `handle_parakeet_failure` | 77 | 1 | 4 | `4054aa00` |
| `poll_stopped_parakeet_exit` | 38 | 0 | 6 | `ca20914c` |
| `poll_idle_parakeet_exit` | 21 | 0 | 4 | `cc273e9d` |

`poll_parakeet_worker` mesure 9. Chaque commit passe adapters avec Q/D, Clippy JSON et N1 : 944/0/129. Les références et assertions restent intactes. Les trois nettoyages demeurent distincts, `NativeCallStarted` inline, le repli Auto et la récursion `Stopped` préservés.

Revue indépendante **CLEAN**, base `6537d2f8`, candidat `cc273e9d`, aucun finding. Les avertissements `too_many_lines` résiduels mesurés dans le fichier sont `handle_single_pass_parakeet_result` à 102/100 et `poll_parakeet_worker` à 117/100 ; aucun nouvel avertissement T9 dans ses helpers.

T10, validation finale, nouvelle N2 et recette Windows ne sont pas exécutés dans cette phase. Aucun merge ni push.
