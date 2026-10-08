# Sources courantes et préflight L04 — P-WHISPER-24

P24 DRAFT; P14 reste le candidat actif avant promotion; parent D19 exact. L’inventaire reproduit le checkout au HEAD `c75ae195b4431abd8f8c1dfd7abb45e81f0ad79b` le 2026-10-07. Ce n’est ni un préflight de lot, ni une baseline D0, ni un `code_state`.

## Allowlist et état observé

Allowlist exacte : 15 chemins, identique à l’index et `01_LOTS.md`.

| Chemin | Présence observée |
| --- | --- |
| `crates/whisper-adapters/src/supervisor.rs` | Absent — création future |
| `crates/whisper-adapters/tests/worker_control.rs` | Absent — création future |
| `crates/whisper-core/tests/compute_policy.rs` | Absent — création future |
| `crates/whisper-worker-cpu/src/native_engine.rs` | Présent — output L01 |
| `crates/whisper-worker-gpu/src/native_engine.rs` | Absent — création future |
| `crates/whisper-adapters/src/lib.rs` | Présent — consumer source |
| `crates/whisper-adapters/src/worker_ipc.rs` | Présent — consumer source |
| `crates/whisper-core/src/application.rs` | Présent — consumer source |
| `crates/whisper-core/src/compute_policy.rs` | Absent — création future |
| `crates/whisper-core/src/lib.rs` | Présent — consumer source |
| `crates/whisper-desktop/src/root.rs` | Présent — consumer source |
| `crates/whisper-desktop/src/ui.rs` | Présent — consumer source |
| `crates/whisper-worker-gpu/src/main.rs` | Présent — stub d’entrée GPU L00 |
| `crates/whisper-worker-gpu/Cargo.toml` | Présent — configuration/dependency manifest |
| `Cargo.lock` | Présent — configuration/dependency manifest |

Les huit sources existantes consumer/output sont copiées octet pour octet sous `sources/L04-consumer-baseline/`; hashes :

| Fichier | SHA-256 |
| --- | --- |
| `crates/whisper-worker-cpu/src/native_engine.rs` | `9d415e118185e0c97917098bf79902f649722a909f1486ef619c3626e88203d7` |
| `crates/whisper-adapters/src/lib.rs` | `9b71f5efe1d6669ca6e38e865915b6e4bfb6504558e076f520eaf86f20d09174` |
| `crates/whisper-adapters/src/worker_ipc.rs` | `4f1bfa7a9bdf0811046204831224a348901dfc36b64cec225ebab53ed284e2c2` |
| `crates/whisper-core/src/application.rs` | `47c32fcc0f9d2709670723ffea636a0324bdef51a62f9d0c9b35dbc4f2294c69` |
| `crates/whisper-core/src/lib.rs` | `5b047a792ee30b73fed1e3cab2c8bc1a47b8f41f781ceae750905cbd27607d3a` |
| `crates/whisper-desktop/src/root.rs` | `9a3713112e3dfc3872e15e3cb581c7c6a53222e5f62864144ba4d111caa98599` |
| `crates/whisper-desktop/src/ui.rs` | `148900588d475028b2c0c27c65ed05116dd93aecf62568c78cef6a554b913648` |
| `crates/whisper-worker-gpu/src/main.rs` | `047430149342e5fec4e77d8b9009ee377af8c0b3c50a9c96b3fb52536d96f953` |

Les deux manifests de configuration/dependencies existent et sont copiés sous `sources/L04-config-baseline/` :

| Fichier | SHA-256 |
| --- | --- |
| `crates/whisper-worker-gpu/Cargo.toml` | `5ef21c0437965d8ea28c01bdd33f5c5d3ec867e2498ab4329b3a24ad54553b89` |
| `Cargo.lock` | `9d714d4315a5dd63865faa6bc8eb9bcb84eb9abdfdd2182a1fafbb4c2b60b67f` |

Les cinq fichiers absents sont supervisor.rs, worker_control.rs, core compute_policy.rs, le test core compute_policy.rs et worker GPU native_engine.rs. `main.rs` GPU existe mais reste stub L00. Le moteur CPU `native_engine.rs` est output L01, distinct de main.rs CPU et des consumers. Le préflight devra recapturer HEAD et hashes; les snapshots ci-dessus ne remplacent pas les preuves de lots ou le manifest D19.

## Intégration et propriétés D19

`worker_ipc.rs` refuse aujourd’hui live non-CPU; `ui.rs` ne compose que CPU et ferme via Stop; `application.rs` rejette encore des commandes non traitées. `root.rs` assemble l’application. Ces chemins sont dans l’allowlist. `supervisor.rs` tient la supervision au parent/adapters; le core porte la policy pure; le desktop expose état, modes et commandes.

Le worker GPU enfant lit/écrit le protocole JSONL existant avec handshake version/identité, traite messages corrélés, infère via `whisper-rs` et ne confirme `Stopped` qu’après arrêt et état durable stabilisé. Stop est admis sous saturation; Quitter reste coopératif, sans fallback GPU Strict, sans kill ni faux Complete.

**Import** : l’enfant décode WAV/MP3 via `symphonia`, infère et retourne des segments/résultats corrélés. Le parent (scheduler/journal) publie TXT/SRT suivant D19; l’import ne crée ni n’archive de copie MP3 audio. **Live** : l’enfant consomme le PCM borné, infère et encode l’artefact MP3 selon D19/50; le parent reçoit événements/résultats corrélés, journalise puis publie TXT/SRT/artefact au bon ordre. Cette propriété parent/enfant suit les ports D19; aucun changement d’architecture DESIGN n’est revendiqué.

Le Cargo GPU possède déjà whisper-rs, serde_json et symphonia MP3/WAV. Il devra ajouter serde pour les structures protocole, sha2 pour les identités/empreintes contractuelles et rusty_mp3 pour encodage, aux versions déjà verrouillées côté CPU. GPU `Cargo.toml` et root `Cargo.lock` sont autorisés; le préflight vérifiera le graphe Cargo exact. Pas de dépendance non spécifiée.

## Racines, preuves et gate

Les candidats et transports de préparation sont conservés sous `docs/predev/L03-P14-CLOSURE`. Utiliser cette racine pour les manifests de candidats; le state canonique reste `docs/predev/state.json`. Le checkpoint pending `state.pending.P24.json` doit passer `check-state` sans `--lot` et conserver P14 actif avant la revue P24 et sa promotion. Il référence plan-writer P16, revue P16 brute, résumé P17 (non verbatim clairement qualifié), contribution coordinateur transportée et revue P19 lorsque son rapport est transporté.

Après CLEAN exact P24, mettre à jour un pending contrôlé qui lie P24 READY comme PLANS courant tout en gardant D19 READY et L00–L03, puis valider et promouvoir ce checkpoint selon l’autorité fournie. Alors seulement la vérification documentaire de lot peut être envisagée. Un mandat séparé est requis avant implémentation; le préflight de lot et `check-state --lot L-WHISPER-04` restent NOT RUN dans P24. Builds CPU/GPU, essais import/live, UI, Stop/Quitter et qualification restent NOT RUN.
