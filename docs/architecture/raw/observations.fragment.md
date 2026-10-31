### Tests, recette et conditions de compilation

- Des champs d’injection `controlled_*` se trouvent dans la structure ImportRuntime, sous `#[cfg(test)]` ; leurs constructeurs/injecteurs sont aussi conditionnels. Ce sont des éléments de tests dans un fichier mixte, pas des champs de production livrés. `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:110-137` ; `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:311-413`.
- Les barrières d’acceptation se trouvent dans les sources applicatives sous `recette-hooks` et vérifient aussi un marqueur de compilation ; une variante sans cette feature retourne immédiatement. `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:51-102`.
- Archive et workers contiennent également des barrières et marqueurs de campagne conditionnels. `crates/whisper-adapters/src/archive/campaign.rs:1-53` ; `crates/whisper-worker-cpu/src/qualification.rs:1-85` ; `crates/whisper-worker-gpu/src/qualification.rs:1-85`.
- Le script de build partagé refuse `recette-hooks` quand `PROFILE` vaut `release` ; il est déclaré pour adapters et les deux workers Turbo. `scripts/recette-build.rs:1-7` ; `crates/whisper-adapters/Cargo.toml:7` ; `crates/whisper-worker-cpu/Cargo.toml:7` ; `crates/whisper-worker-gpu/Cargo.toml:7`. [Non vérifié] Son exécution n’a pas été testée ici.

### Identifiants historiques

- `l04_campaign_gate` et `l04_campaign_worker_result` restent des noms de fonctions des workers. `crates/whisper-worker-cpu/src/qualification.rs:8-85` ; `crates/whisper-worker-gpu/src/qualification.rs:8-85`.
- `is_canonical_l3c_test_artifact` existe dans les validations de chemins, avec variantes selon `recette-hooks`. `crates/whisper-adapters/src/parakeet_import/historical_runtime.rs:141-157`.
- Le répertoire `.l2a2-acceptance` et la variable `PARAKEET_L2_ACCEPTANCE_TEST_BUILD` sont des valeurs de contrat des barrières. `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:59-87`.

### Sens des dépendances

- Le manifeste du cœur ne déclare que serde ; aucune dépendance Cargo du cœur vers adapters/desktop n’y figure. `crates/whisper-core/Cargo.toml:1-12`. Les arêtes déclarées sont reproduites en section 2. [Non vérifié] Cela ne constitue pas une analyse des dépendances après expansion des macros, des DLL ou de tous les effets indirects.

### Responsabilités parallèles

- CPU et GPU possèdent chacun une boucle `run_live_service` : vérification des identités/séquences, encodage MP3 conditionnel, transcription et émission de résultats. Les types de moteurs et certains contrôles diffèrent ; aucune identité sémantique complète n’est affirmée. `crates/whisper-worker-cpu/src/session/live.rs:4-254` ; `crates/whisper-worker-gpu/src/session/live.rs:3-254`.
- Les fonctions de barrières L04 sont présentes dans les deux workers, alors que décodage/MP3/trames/intégrité passent par worker-common. `crates/whisper-worker-cpu/src/qualification.rs:1-307` ; `crates/whisper-worker-gpu/src/qualification.rs:1-307` ; `crates/whisper-worker-common/src/lib.rs:1-5`.

### Concentration et mélange constaté

- `source_binding.rs` rassemble acquisition/copie audio, registre transactionnel, transfert d’identité, récupération et suppression de snapshots. `crates/whisper-adapters/src/source_binding.rs:101-338` ; `crates/whisper-adapters/src/source_binding.rs:450-536` ; `crates/whisper-adapters/src/source_binding.rs:627-672` ; `crates/whisper-adapters/src/source_binding.rs:539-569` ; `crates/whisper-adapters/src/source_binding.rs:1217-1262`. [Déduction] Ces opérations couvrent plusieurs étapes du cycle de vie d’une source ; leur présence dans un même module est constatée, sans conclusion sur une frontière cible.
- La boucle `poll` du runtime traite les événements worker, les segments, les reprises, les modes live et les terminaisons ; `poll_unified_sessions` gère capture, disponibilité, résultats, ordonnanceur et recyclage d’exécuteur. `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs:134-1231` ; `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:382-1373`. [Déduction] Ces fonctions concentrent plusieurs responsabilités liées au cycle d’exécution ; « sans rapport » n’est pas établi.

### Code mort apparent

- [Déduction] Les ports Clock, JobRepository et WorkerPort semblent être des contrats réservés : la recherche textuelle relève leurs déclarations et réexportations mais pas d’implémentation explicite. Ce n’est pas une preuve de code mort, notamment pour une API publique. `crates/whisper-core/src/ports.rs:16-23` ; `crates/whisper-core/src/ports.rs:80-84` ; `crates/whisper-core/src/lib.rs:40-45`. Recherche brute : [rg-unused-ports.txt](raw/rg-unused-ports.txt). [Non vérifié] Aucun diagnostic compilateur n’établit leur inutilité.

### Diagnostics de code inutilisé effectivement émis

Clippy/rustc a émis `dead_code` sur `kind_name` (`crates/whisper-core/src/application/event_metadata.rs:5`), `transcribe` (`crates/whisper-worker-parakeet-cpu/src/native_engine.rs:42`) et `allowed_runtime_roots_for` (`crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs:133`) dans le groupe de diagnostics rattaché à une compilation non-test [Déduction pour l’attribution de cible]. Ce sont des constats pour les configurations compilées, pas une preuve d’inutilité dans toutes les features. [Diagnostics bruts](raw/cargo-clippy.stdout.jsonl), [index](raw/clippy-diagnostics.tsv).
