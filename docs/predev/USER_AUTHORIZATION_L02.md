# Autorisation utilisateur — implémentation L-WHISPER-02

Cette capture consolide les autorisations directes de HUMAN-USER dans la conversation Codex active le 2026-10-06 :

1. « ok je t'autorise » — implémentation de L02 après préflight PASS, initialement sur les sept chemins de L02 alors inscrits dans l'état.
2. « je t'accorde l'extension de portée » — autorisation des cinq chemins complémentaires explicitement listés pour corriger la portée PLANS.
3. « je t'autorise » — autorisation des trois chemins complémentaires explicitement listés pour permettre l'admission FIFO, la reprise explicite et la préparation MP3.

La portée totale ci-dessous s'applique à l'implémentation L02 seulement. Elle ne permet le démarrage du code qu'après création d'un nouveau candidat PLANS couvrant ces chemins, revue indépendante CLEAN de ce candidat, promotion/contrôle du nouvel état, puis préflight `check-state --lot L-WHISPER-02` PASS. Aucun autre lot n'est autorisé par cette capture.

## Chemins autorisés (15)

- `crates/whisper-adapters/src/archive.rs`
- `crates/whisper-adapters/src/decoder.rs`
- `crates/whisper-adapters/src/journal.rs`
- `crates/whisper-adapters/src/lib.rs`
- `crates/whisper-adapters/src/queue_store.rs`
- `crates/whisper-adapters/src/recovery.rs`
- `crates/whisper-adapters/src/worker_ipc.rs`
- `crates/whisper-adapters/tests/durability.rs`
- `crates/whisper-adapters/tests/import_mp3.rs`
- `crates/whisper-core/src/application.rs`
- `crates/whisper-core/src/ports.rs`
- `crates/whisper-core/tests/scheduler_contract.rs`
- `crates/whisper-desktop/src/root.rs`
- `crates/whisper-desktop/src/ui.rs`
- `crates/whisper-worker-cpu/src/decoder.rs`

Les plafonds mémoire existants restent inchangés. Le GPU reste hors périmètre de L02. Les validations produit et la qualification MP3 live de L03 ne sont pas réputées exécutées par cette autorisation.
