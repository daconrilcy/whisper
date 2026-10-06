# Préflight d’implémentation — L-WHISPER-01

**Date :** 2026-10-05 (Europe/Paris). **Résultat :** PASS.

## Entrées contrôlées

- DESIGN D-WHISPER-19 et PLANS P-WHISPER-02 sont READY ; manifests/digests et revues exactes CLEAN liés au checkpoint courant. Aucun octet des candidats READY n’a été modifié.
- Q-05 est répondu et accepté par HUMAN-USER : snapshot du dossier au démarrage de chaque job ; le changement s’applique aux jobs suivants ; aucun déplacement ni suppression automatique. Réponse brute dans `transports/T-WHISPER-Q05-ANSWER-01/`.
- L-WHISPER-00 est terminé. Ledger `execution-ledgers/L-WHISPER-00-2026-10-05.md`, SHA-256 `03ac66707e082e8bb84079f000af3e8fc4bcb55b916a8dab91b3f3d4ff18bf1c`. Les 23 sorties CODE ont été comparées aux fichiers présents. CPU EXE SHA-256 `39db7beddfaa99cbbf846fd9bd745500032f53bfdbb262d7a809323bef00a107`; GPU EXE SHA-256 `bd784896a920fd3adf0e23d5024604ca17e5fd9852f419256e4fc31dcf1e5b73`; log GPU SHA-256 `60827331bc06af3254d654ed39a0dc82966914e112cc455322d110616a545ee2`.
- Autorisation utilisateur L01 `USER_AUTHORIZATION_L01.md`, SHA-256 `b6088b12a1e8aa221db1f1b4ca9e29a5e867fce35590f217ff82164548cc3151`, limitée aux chemins exacts de `L-WHISPER-01` dans `state.json`, alignés sur P02 : `crates/whisper-core/tests/import_contract.rs` et `crates/whisper-worker-cpu/**` inclus.
- Railguard actif confirmé par la copie probatoire `RAILGUARD-active.md`, son attestation `railguard-activation-L00.md` et la proposition D19.

## Dépôt et identité des sources

- Branche `main`, HEAD `e0f055e649bf4c4905130a244290d458ae3a76c1`. `git status --short` montre uniquement `?? target/`, artefacts locaux des builds L00 ; aucun changement de code non commis n’est observé.
- Les chemins L01 déjà présents (`crates/whisper-adapters/src/decoder.rs`, `crates/whisper-desktop/src/ui.rs`) concordent avec les sorties L00 hashées. Les autres chemins L01 sont encore absents et restent des sorties futures ; aucun n’a été créé pendant le préflight.
- Le contrôleur accepte un seul `--root`, mais le corpus documentaire est sous `docs/predev` et les chemins Cargo sont relatifs à la racine du dépôt. L’appel direct avec `--root docs/predev` a échoué sur `file missing: Cargo.lock`. Le contrôle normatif a donc été relancé dans une vue temporaire unifiée créée par hardlinks/copies des documents contrôlés et des fichiers du dépôt. Les 23 hashes sources de la vue ont été comparés octet par octet aux fichiers courants du workspace avant exécution.

## Gate

Commande exécutée : `predev_control.py check-state --root C:\Users\cyril\AppData\Local\Temp\whisper-predev-L01-gate-20261005\root --state C:\Users\cyril\AppData\Local\Temp\whisper-predev-L01-gate-20261005\root\state.json --lot L-WHISPER-01`. Sortie exacte :

```json
{"candidate_digests":{"DESIGN":"903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529","PLANS":"8113649a38976e0c14e1c82a10ed1a04dff7c1022aaff5587962a0d94b8b2bad"},"lot":"L-WHISPER-01","observed_drift":{},"ok":true,"phase":"IMPLEMENTATION","schema":"rust-predev/2"}
```

Le check-state L01 est PASS. Aucun ledger L01 n’est créé avant ce gate. V-IMPORT et V-UI restent NOT RUN et sont à produire après le gate. L00 n’atteste aucun parcours réel d’import/transcription ; campagnes PE, smoke natif et qualification produit restent NOT RUN.
