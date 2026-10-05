# Preflight documentaire — L-WHISPER-00

Date d’observation : 2026-10-05, Europe/Paris.

## Corpus et revues liés

- DESIGN : D-WHISPER-19, `candidates/D-WHISPER-19/manifest.json`, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`, manifest SHA-256 `7f6465f9254611d432da50ab64034c7ae92b088351396f2c3eb3249c71866cb7` ; revue exacte `R-WHISPER-DESIGN-19`, verdict CLEAN, reviewer `AUTH-REVIEWER-D19`, preuve `transports/T-WHISPER-REVIEW-19/transport-manifest.json`.
- PLANS : P-WHISPER-02, `candidates/P-WHISPER-02/manifest.json`, digest `8113649a38976e0c14e1c82a10ed1a04dff7c1022aaff5587962a0d94b8b2bad`, manifest SHA-256 `7ae618d2b1149d5856ae9e080296af5470a509327d732ecec276eda77d3e96e9` ; parent D19 exact ; revue exacte `R-WHISPER-PLANS-02`, verdict CLEAN, reviewer `AUTH-REVIEWER-PLANS-01`, preuve `transports/T-WHISPER-PLANS-REVIEW-02/transport-manifest.json`.
- Détail : DETAIL-P01 répondu par ARCH-P01 v3 et persisté en `transports/T-WHISPER-ARCH-P01-DETAIL-02/transport-manifest.json` (digest `9360b37c37a4a7ad9155af7087c474da253f41df87e72b2879e0ac497baa9cfe`). Le premier essai de sérialisation `T-WHISPER-ARCH-P01-DETAIL` n’est pas la preuve retenue ; il est remplacé par le v3 et exclu du checkpoint courant.

## Gouvernance, autorisation et disposition

- Railguard actif : `RAILGUARD.md` ; activation et équivalence : `docs/predev/railguard-activation-L00.md`.
- Mandat humain de préflight et d’implémentation limitée à L00 : `docs/predev/USER_AUTHORIZATION_L00.md`.
- Scope code L00 du plan : Cargo.toml, Cargo.lock, rust-toolchain.toml et les chemins `crates/whisper-core/**`, `crates/whisper-adapters/**`, `crates/whisper-worker-cpu/**`, `crates/whisper-worker-gpu/**`, `crates/whisper-desktop/**`, `crates/whisper-bootstrap/**`. L’autorisation vise seulement ces racines et L-WHISPER-00.
- DETAIL-P01 : répondu avant L00 ; le choix fixe six packages, des workers CPU/GPU séparés, UI eframe/Glow et décodeur Symphonia exécuté dans l’enfant. L’architecte conclut aucun changement de garantie/frontière/capacité GPU (`ANSWERED_DETAIL`).

## HEAD, dépôt et dérive

- HEAD observé : `dd3495b27fc72fbbca64ee6d865eb16d2244b9f4`.
- `git status --short` attendu et vérifié après promotion de ce checkpoint :

```text
 M docs/predev/state.json
?? RAILGUARD.md
?? docs/predev/L00_IMPLEMENTATION_PREFLIGHT.md
?? docs/predev/RAILGUARD-active.md
?? docs/predev/USER_AUTHORIZATION_L00.md
?? docs/predev/railguard-activation-L00.md
?? docs/predev/transports/T-WHISPER-ARCH-P01-DETAIL-02/
?? docs/predev/transports/T-WHISPER-ARCH-P01-DETAIL/
```

Ces entrées sont les seuls changements locaux : préflight/autorisation/attestation/railguard et transports documentaires ; aucun chemin de code produit modifié. `git ls-tree -r --name-only HEAD` ne contient aucun `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` ou `crates/**`. Le code baseline et courant de L00 sont donc vides ; aucun lot précédent n’existe (`drift=none`).

L’identité d’entrée est HEAD `dd3495b27fc72fbbca64ee6d865eb16d2244b9f4` et les deux manifests/revues ci-dessus. Les résultats bruts de D19/P02 sont revérifiés via leurs transports référencés dans `state.json`. Aucune preuve de build ou de comportement produit n’est inférée de ce préflight documentaire.
