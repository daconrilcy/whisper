# PW-WHISPER-P04-CORRECTIONS-01 — contribution brute compacte

Acteur `/root/correct_p03`, rôle rust_plan_writer, lecture seule, 2026-10-06 Europe/Paris. Base P03 digest `2af7eaf55c20184bfdeb1035dab7e64593799c94e0bbad5ccfafea65c04e8ead`; parent D19 `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. Cible P04 DRAFT. Aucun fichier modifié, finding fermé ou promotion effectuée par l’auteur.

Findings couverts : WHISPER-PLANS-004..007. Candidat immuable : créer P04 depuis P03; conserver P03 et sa revue FINDINGS. Le reviewer P04 doit considérer INTEGRATION v2 et BOUNDS v1 ensemble; COMPAT v2 ne valide que la compatibilité INTEGRATION/D19.

Fichiers proposés : remplacer `00_PLANS_INDEX.md`, `01_LOTS.md`, `02_VERIFICATION_AND_PREFLIGHT.md`, `03_CENTRAL_PACK_IMPACT.md`, `04_OPEN_DETAILS.md`, `05_SOURCE_AND_RULE_INDEX.md`; ajouter `06_L01_INTEGRATION_CONTRACTS.md`. Ajouter les copies exactes architecturales de DETAIL-P01 v3, INTEGRATION v2, BOUNDS v1, COMPAT v2, Q05; copies de progression L00/L01 preflight, ledger L00, railguard active/activation. Actualiser six copies de règles et reconstruire `rules/pack-manifest.json`.

004 : main déclare `mod root`; root privé au binaire compose et appelle `whisper_desktop::run(application)`; lib réutilise API run sans exporter root; lib/ui sans adapters/fs/process/native; aucun état métier/scheduler parallèle. UI reste modifiable et pure.

005 : L00 reste completed; ledger prouve qu’il ne livre ni root.rs, ni application d’import, écrivain durable ou vrai parcours. Affecter ces travaux à L01; ne pas transférer l’ancien PASS P02/sept chemins.

006 : intégrer ImportApplication/ImportIoPort, propriété des états/effets/reçus, étapes Prepared/pending, Ready/Progress/Segment/End, sync avant reçu, publication vérifiée pointeur en dernier, Complete après ACK, scan Recoverable idempotent; Q05 snapshot destination. IPC v2 corrélé (job/génération/instance/requête/séquence/plage/source hashée/backend), End explicite, erreurs/Stopped et contrôle indépendant; refuser V1, GPU en L04, DTO IPC distincts du durable. Bornes: 8 effets/8 MiB; 64 événements/64 MiB; frame entière 1 MiB; un décodage/bloc, une inférence, contrôle indépendant Stop/Shutdown, UI provisoire remplaçable. PCM mono 16 kHz/5 s/80k samples max, valider 1..80k avant allocation, tailles PCM16/f32, offsets contigus source, aucune borne RSS globale. Saturation visible/rétropression, génération invalidée avant ancien résultat, aucun délai FFI garanti/kill auto. Ajouter Q-L01-BOUNDS-01, preuve attribuée BOUNDS v1 à enregistrer après contrôle; COMPAT ne valide pas les nombres. Les vérifications ciblées restent PRODUCT_VALIDATION / NOT RUN.

007 : P03 comportait six divergences de copies; P04 rafraîchit les six snapshots et le pack-manifest. Le helper actuel `check-state` vérifie dependency CODE outputs sous doc root dans `proofs(root,...)` (ligne 357) puis sous `code_root` (ligne 469); appel à deux roots échoue `file missing: Cargo.lock`. Décrire la vue unifiée temporaire, ses hashes/absence collision/comparaison avant-après, ou attendre correction du helper. Ne jamais présenter le check comme PASS.

Allowlist complète exacte de L01 (21 chemins; `ui.rs` inclus, `lib.rs` exclu):

```text
Cargo.lock
crates/whisper-adapters/Cargo.toml
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/decoder.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/tests/import_cpu.rs
crates/whisper-core/src/application.rs
crates/whisper-core/src/ipc.rs
crates/whisper-core/src/lib.rs
crates/whisper-core/src/ports.rs
crates/whisper-core/tests/import_contract.rs
crates/whisper-desktop/Cargo.toml
crates/whisper-desktop/src/main.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-worker-cpu/Cargo.toml
crates/whisper-worker-cpu/src/decoder.rs
crates/whisper-worker-cpu/src/ipc.rs
crates/whisper-worker-cpu/src/main.rs
crates/whisper-worker-cpu/src/native_engine.rs
```

Sources exact contrôlées: DETAIL-P01 `c554c748d888eb91aef8cff024ba4eb6b3b7d0317f333f0395111c084cae8086`; INTEGRATION `1c8675f6d1e0f18e05556e8e3118d824e1c95591aca37e8d4e6823733787ea27`; BOUNDS `fee3f0dbcb5ecdbc9f70082aba311bca3daa34bf5efaa12d6b90b71a10f2efce`; COMPAT `34c8ce954c352eb42da575c2164f19959b144f39889d846d5d3b816eb02947ac`; Q05 `89538c0beebfe221883bb9e260d415818a992e58273bb870ba5dbaddea0f45c6`. Six rule hashes are in the author turn; coordinator recalculates all actual copies.

L00/P02 baselines were verified before authoring. Remaining work: persist P04/this raw contribution, verify receipts and provenance, independent review and further corrections, then checkpoint, user authorization, inventory and new gate. No product code/test executed.
