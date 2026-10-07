# Clôture L03 — sorties et preuves

Statut : **L-WHISPER-03 completed** dans `state.json`. Cette clôture consolidée est rattachée à P-WHISPER-14 actif par les digests du checkpoint contrôlé et par la revue exacte de P14. Le plan P14 reste immuable.

## Identités et candidat

- Plan actif : `P-WHISPER-14`, digest `2724e325c5377d725b4e4f73541b6079a7bf08a9ff8e4328b10768e6823ca55b`, manifeste SHA-256 `6140b7a32a28acef0acf370a7a74c504cefa9071cc72fc97527438d3f0cfba90`, revue `R-WHISPER-PLANS-14-v1` verdict `CLEAN`, rapport conservé sous `plan-review/R-WHISPER-PLANS-14-v1.md` et manifeste de revue sous `plan-review/manifest.json`.
- Design parent : `D-WHISPER-19`, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`, verdict `CLEAN`.
- Fingerprint des 10 sorties source L03 : `3d442cfae5edcd5cb0fb15c3d8d4990820595ae146cc84640356a139bc9a5966`. Les 10 SHA-256 ont été recontrôlés dans le dépôt avant assemblage et concordent tous avec le paquet d’exécution.
- La revue d’implémentation indépendante et les vérifications techniques portent le même fingerprint : revue `CLEAN`, validation `CHECKS_PASSED`.
- Autorisation code L03 : `USER_AUTHORIZATION_L03.md`, 28 chemins bornés. La clôture documentaire n’élargit pas cette allowlist.

## Sorties source et vérifications

Les dix fichiers source modifiés (archive, capture, staging, IPC worker, tests live/archive, application, ports, contrat live et UI) sont énumérés avec SHA-256 dans `execution-evidence.json`. Ils concordent avec l’exécution L03 conservée. Les onze commandes de vérification CPU/desktop sont toutes **PASS** : format, check, cinq tests live/VAD/worker, durability, Clippy, build release CPU/desktop, diff-check. Les journaux bruts sont sous `commands/`; `validation.md` consigne argv, environnement, codes retour, hashes des logs et binaires.

## Compilation native GPU complémentaire

La compilation GPU est conservée comme preuve complémentaire au lot L03 : CUDA 12.9.86, VS 2022 17.14, MSVC v143 et architecture `sm_61`. Les journaux d’environnement et de build sont sous `cuda-logs/`. Deux essais antérieurs en échec sont conservés avec leurs causes (résolution CUDA sous MSVC 2019, puis conflit `-Thost=x64`/toolset explicite). Le build release final sous VS2022, sans variable de toolset contradictoire, est **PASS**. Binaire : `target/l03-closure-cuda-vs2022-auto/x86_64-pc-windows-msvc/release/whisper-worker-gpu.exe`, 123 904 octets, SHA-256 `2f4c522d10cdde111d05d0e71917f2bca4e6f805cc9139a2932b5f8e9faced41`. Cette compilation ne prouve pas le démarrage du worker ni une inférence GPU.

## Non exécuté / limites de portée

Microphone réel représentatif, sessions longues, saturation/retard CPU, panne disque ou périphérique, inférence et panne pilote GPU, UI native et qualification produit intégrée : **NOT RUN**. `CHECKS_PASSED` et la compilation CUDA attestent les commandes consignées pour le fingerprint L03, pas une qualification du produit.

## Références d’origine et lien d’état

Les preuves ont été consolidées depuis `P10-L03-CLOSURE-WORK/transports/T-WHISPER-L03-CLOSURE-03` (11 commandes, fingerprint, revue et validation) et `P10-L03-CLOSURE-WORK/transports/T-WHISPER-L03-CUDA-02` (compilation GPU), puis reliées au digest P14 ci-dessus. `active-state-snapshot.json` conserve les octets de `docs/predev/state.json` observés à la clôture : L03 est `completed` et le checkpoint actif est P14 ; son champ `completion_evidence` historique continue de pointer vers le paquet source P10. La présente référence est un complément immuable de clôture et ne réécrit ni le checkpoint déjà promu ni les documents READY.
