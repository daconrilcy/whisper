# Matrice de vérification L04 — P-WHISPER-28

Cette matrice reprend les corrections de vérification proposées dans les prédécesseurs P26/P27; la revue exacte de P28 reste à effectuer. Tout est PROPOSED / NOT RUN : aucun test, build, probe, essai natif ou parcours UI n’a été exécuté pour produire P28. Shell Developer PowerShell MSVC; cwd `C:\dev\whisper`; target `x86_64-pc-windows-msvc`; sélection `RUSTUP_TOOLCHAIN=1.98.1`, `RUSTUP_AUTO_INSTALL=0`. Exécuter les commandes Cargo séquentiellement et arrêter au premier échec. Les cinq nouveaux chemins/test targets ne sont pas encore présents tant que l’implémentation n’a pas commencé.

## Gates après implémentation — ordre et commandes

1. Format/workspace :

```powershell
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets --target x86_64-pc-windows-msvc
```

2. Tests unitaires/integration ciblés (après création des targets autorisés) :

```powershell
cargo test --locked -p whisper-core --test compute_policy --target x86_64-pc-windows-msvc -- --nocapture
cargo test --locked -p whisper-adapters --test worker_control --target x86_64-pc-windows-msvc -- --nocapture
cargo test --locked -p whisper-core --lib --target x86_64-pc-windows-msvc -- --nocapture
cargo test --locked -p whisper-adapters --lib --target x86_64-pc-windows-msvc -- --nocapture
```

3. Clippy sur tous les targets Rust applicables, sans builds concurrents :

```powershell
cargo clippy --locked --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings
```

4. Builds distincts, dans cet ordre, en conservant dossiers et logs séparés :

```powershell
cargo build --locked --release -p whisper-worker-cpu --target x86_64-pc-windows-msvc --target-dir target/l04/cpu
cargo build --locked --release -p whisper-worker-gpu --target x86_64-pc-windows-msvc --target-dir target/l04/gpu
cargo build --locked --release -p whisper-desktop --target x86_64-pc-windows-msvc --target-dir target/l04/desktop
```

5. Contrôle du diff :

```powershell
git diff --check
```

`cargo check`/tests/clippy/build attestent uniquement leurs commandes, targets et résultats observés. Build CUDA ne prouve pas GPU opérationnel, inférence, fallback ou comportement produit. Les `--locked` et target explicites garantissent la reproductibilité du graphe ciblé; aucun feature supplémentaire n’est à inventer (le manifest GPU fixe déjà `whisper-rs` CUDA).

## Scénarios et preuves de clôture

| AC-07 | Mode Auto et mode GPU forcé : GPU absent/échec avant et pendant tâche, puis dernier point confirmé | Backend effectif/motif visibles; Auto reprend selon contrat sans doublon; GPU forcé suspend et ne bascule pas CPU sans accord | Device, mode, versions, job/generation/offset, événement/erreur, sortie redacted et fingerprint | Test policy/build ne qualifie pas le GPU natif |
| AC-09 | Silence attendu, inférence longue, disque plein et worker figé; observer fenêtre de progrès ~60 s | Avertissement/diagnostic distingue attente, lenteur et panne; arrêt uniquement sur preuve de panne; silence ne stoppe pas | Horodatages, progress/reçus, erreurs injectées, profondeur/capacité, diagnostic fermé, état Recoverable | Horloge simulée ne qualifie pas un worker/stockage réel |
| AC-11 | Masquer la fenêtre pendant live/import; déclencher Quitter; enfant bloque avant `Stopped`, puis répond/retourne ancien événement | Masquer ne stoppe rien; Quitter live finalise ou préserve Recoverable; import s’annule sans modifier source; si enfant bloqué, vraie fenêtre reste visible/réactive, attend ack corrélé, aucun faux Complete | Capture UI/tray horodatée, hash source, IPC/job/generation, attente/ack, événements anciens refusés, état final | Tests purs ne prouvent pas le vrai consumer, UI, child ou codec |
| AC-13 | Injecter coupures avant/après confirmations de fragments live puis redémarrage/récupération | Tous fragments confirmés récupérés; partie/file ouverte éventuellement absente, perte mesurée sans borne seconde inventée | IDs/offsets/acks, audio confirmé hashé, état journal, artefact récupéré, portion perdue mesurée, fingerprint | Pas de preuve de panne électrique/OS si seule interruption simulée |
| AC-19 | Sur machine avec GPU compatible, choisir CPU forcé avant tâche | Backend CPU attesté; aucun moteur/contexte GPU initialisé pour ce job | Host/device inventory, mode choisi, trace init, backend effectif, code/logs et job ID | Machine/driver réel requis; test pur ne montre pas l’absence d’initialisation GPU native |

### Vérifications transversales (pas d’attribution à un autre AC)

- Saturer la file data, soumettre Stop, Injecter événement de génération ancienne : Stop admis malgré saturation, état final corrélé, ancien résultat rejeté (contrats TASK-P04, exigences couvertes par AC-09/11 selon comportement mesuré).
- Exercer `Recoverable`, événement explicite de panne, durée longue et DurableAck figé pour régressions live/import; source importée demeure inchangée, état durable annoncé avec exactitude.
- Émettre données diagnostiques autorisées/interdites; quatre fichiers de 2 MiB total 8 MiB et âge max 7 jours suivant Q-09; purge refusée, coupure pendant rotation et erreurs filesystem. Aucun audio/transcription/chemin/message libre; dégradation de log ne perd ni Stop ni fragments confirmés. Ces scénarios vérifient DETAIL/Q-09, pas AC-19.
- Pour chaque scénario, séparer fixture pure, IPC, child natif CPU/GPU, audio codec/live/import et vraie UI. Retenir IDs/stimuli/données hashes, backend/device, résultat/fingerprint, log/capture, erreurs et limites. Toutes campagnes futures restent PROPOSED / NOT RUN.

## Environnement et attribution

Avant toute commande de build, suivre `build-environment-L04.json` : Rust/Cargo 1.98.1, MSVC, CMake, Libclang, NVCC CUDA 12.8, Ninja/CUDA_PATH selon matrice, target installé, toolchain/lockfile. `check-build-environment` non réussi suspend les builds; ne pas installer implicitement.

Pour chaque ligne : preuve `PROPOSED` jusqu’à exécution, puis commande exacte, cwd, env, target/features/options, inputs et hashes, branche/stimuli, output/log/code retour, fingerprint, résultat attendu/observé, échecs et limites. Séparer les tests purs, tests IPC, child CPU/GPU natifs, audio live/import et vraie UI. Les essais natifs conservent l’identifiant/hash du modèle/audio et device/backend effectif. Toute scène de fenêtre utilise une capture et mesure réelles; aucun test pur ne se substitue à celle-ci. La livraison/package reste une qualification distincte.
