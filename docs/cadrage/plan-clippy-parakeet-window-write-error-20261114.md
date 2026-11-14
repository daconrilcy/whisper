# Plan P27 — Découper le scénario d’erreur d’écriture Parakeet

Avancée : Clippy standard après P26 passe à 107 sites; le scénario de poll vérifiant l’erreur de fenêtre fait environ 232 lignes et a une complexité de 45.
Blocage : aucun; l’erreur doit arrêter la capture tout en préservant exactement l’audio final annoncé comme capturé.
Décision : isoler les phases du helper enfant en helpers privés dans le même fichier.

## Source et périmètre

Le Clippy frais des six crates signale `run_parakeet_window_write_error_child` dans `crates/whisper-adapters/src/worker_ipc/runtime/poll/tests/parakeet_poll.rs` pour `too_many_lines` et `cognitive_complexity`. Le test enveloppe `poll_parakeet_window_write_error_returns_stopped_event_with_automatic_final_import` est classé `A_VERIFIER / AUTONOME` dans `ci/tests-baseline.txt`; il utilise un worker factice, une capture synthétique et un obstacle d’écriture dans son root enfant gardé.

Périmètre code unique : `crates/whisper-adapters/src/worker_ipc/runtime/poll/tests/parakeet_poll.rs`, intact au baseline. Conserver les helpers dans ce fichier. Ne changer ni le test parent, ni variables/contrôles du child root, ni production, fixtures, baselines, garde ou autre test.

## P27-T1 — Séparer les phases du scénario enfant

Garder la séquence lisible et extraire des helpers privés distincts pour la préparation worker/capture/runtime, l’observation de la progression durable avant READY, l’attente du READY, la détection de l’arrêt sur obstacle et la vérification complète du WAV. Préserver les 48 000 échantillons mono, le seuil de fenêtre de 32 000, la queue de conversion, toutes les deadlines 5 s et pauses 2 ms, l’identité job/génération/session, le comportement de fichier sentinelle, les paths TXT/SRT/WAV, et chaque assertion d’en-tête/données WAV, d’arrêt et de nettoyage. Helpers sous 100 lignes et score cognitif au seuil 0 sous 25; fichier final sous 600 lignes.

## Acceptation

- Les diagnostics `too_many_lines` et `cognitive_complexity` ciblés disparaissent; le Clippy standard ratcheté ne crée aucune nouvelle clé et passe à 105 sites ou moins.
- L’oracle WAV vérifie toujours RIFF, PCM mono 16 kHz/16 bits, 48 000 échantillons exacts, absence d’octets manquants/ajoutés, TXT/SRT présents, arrêt incomplet et absence d’état Parakeet actif/en file.
- Le test enfant reste découvert et exécuté dans N1; aucune attente, condition, assertion ou sortie d’erreur n’est affaiblie.
- SelfTest, `fmt --check`, mesure seuil 0, Clippy standard des six crates avec ratchet, N1 complet, sélections workers, fixtures PE et `git diff --check` passent.
- Seul le fichier cible est modifié; les changements précédents restent intacts.

## Gates et arrêt

Worktree `C:/wt/cs`; `LotRoot` `target/parakeet-tests/clippy-corrections-suivantes-20261113`; cible Clippy existante `p21-fresh-20261114`, puis cible N1 neuve `n1-p27-20261114`. Toutes commandes Cargo via `ci/guard-level1.ps1`, profils exacts enregistrés. Au premier échec, préserver les journaux et arrêter selon l’arbitrage de reprise applicable. P27 est la septième tâche de la tranche P21–P30; nouvel arbitrage après P30.

## Retour arrière

Si l’extraction change l’ordre capture → progression durable → READY → obstacle → arrêt, les octets exportés, le traitement du tail ou le cleanup, retirer la seule extraction fautive et signaler la preuve affectée; ne pas réduire l’oracle WAV.
