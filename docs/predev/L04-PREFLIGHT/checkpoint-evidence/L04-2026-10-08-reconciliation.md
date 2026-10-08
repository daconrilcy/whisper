# L04 — observation de lancement et environnement CUDA 12.9

Date : 2026-10-08. Observation en lecture seule avant implémentation. Source d'autorisation : `docs/predev/USER_AUTHORIZATION_L04_EXTENSION_CUDA129.md`.

## Code existant

- Branche `main`, HEAD `c0e3abc2c09bcf910ab829520336097c0d53f297`.
- Les 21 chemins de `candidates/D-WHISPER-28/57_L04_CHANGE_BASELINE.json` sont présents et leur SHA-256 courant correspond chacun au hash `current_files` D28 : 21 égaux, zéro divergent, zéro absent.
- Le candidat D28 porte le statut `DRAFT_CAPTURE_NOT_LOT_GATE` pour cette capture de code partiel ; elle ne prouve aucune clôture L04.
- L'état courant `docs/predev/state.json` est en phase `PLANS`, lie D28/P29, conserve L04 `planned`, et son `code_state` L04 contient une baseline/current de 10 chemins et un ledger vide. Une transition doit remplacer cette baseline historique par 21 références vérifiées et conserver cette provenance.
- Le worktree contient des modifications et des fichiers non suivis, y compris `Cargo.lock`. Aucune remise à zéro ou attribution de ces changements à L04 n'a été faite.

## Environnement

Le contrôle canonique de la matrice P29 inchangée a retourné `BLOCKED` : `RUSTUP_TOOLCHAIN` absent, `ninja` hors `PATH`, et `nvcc` 12.9 non conforme au motif 12.8.

Un contrôle ponctuel a utilisé une copie temporaire de cette matrice dont seul le motif `NVCC.version_pattern` est devenu `release 12\.9`. Dans le même processus, `RUSTUP_TOOLCHAIN=1.98.1` a été défini et le dossier Ninja de Visual Studio 2019 a été ajouté au `PATH`. `check-build-environment --root C:/dev/whisper --matrix <copie temporaire>` a retourné `ok: true`, `status: PASS`, exit 0 ; les onze probes sont `READY`, dont `nvcc` 12.9 et Ninja 1.10.2. Ces réglages de session ne sont pas persistés et ce PASS ne qualifie ni build ni runtime GPU.

## Transition restant à effectuer

Intégrer le passage à CUDA 12.9 dans un successeur DESIGN/PLANS avec revue indépendante sur les octets exacts ; persister la matrice acceptée et sa source. Capturer les 21 hashes et le status Git dans un nouvel objet d'evidence, étendre l'autorité L04 à partir des deux sources utilisateur, préparer un pending `IMPLEMENTATION`, contrôler puis promouvoir atomiquement. Tant que ces contrôles et `check-state --lot L-WHISPER-04` n'ont pas réussi, L04 reste `planned` sans lancement de code.
