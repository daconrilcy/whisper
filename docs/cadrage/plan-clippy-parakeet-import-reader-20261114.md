# Plan P39 — Isoler le lecteur d’événements du worker Parakeet

Avancée : après P38, Clippy signale `spawn_profile_with_log_root` à 112 lignes et 93 sites.
Blocage : aucun ; conserver le canal borné, le décodage et toutes les erreurs IPC.
Décision : déplacer seulement la boucle de lecture stdout dans un helper privé.

## Résumé et preuve

Le Clippy gardé après P38 signale `crates/whisper-adapters/src/parakeet_import.rs:400`, `spawn_profile_with_log_root`. Cette méthode valide les chemins, construit la commande et l’environnement du worker, prépare les flux, crée le canal borné des événements, puis valide le runtime avant de construire `ParakeetProcess`.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/parakeet_import.rs`.
- Extraire dans un helper privé la boucle détachée qui lit stdout, découpe les trames, appelle `decode_worker_frame` et transmet les erreurs via le canal existant.
- Conserver dans `spawn_profile_with_log_root` la configuration de commande/env, le choix du runtime, les pipes, la capacité du canal (2), la validation finale du runtime et la construction de l’état worker.
- Préserver les messages `IpcTruncated`, `IpcFrameTooLarge`, `IpcRead`, la fermeture à la déconnexion et le comportement du thread détaché.
- Aucun protocole, type public, feature, test ou autre fichier ne change.

## Tâche P39-T1

Introduire `spawn_worker_event_reader` (ou nom équivalent) et l’appeler avec le stdout enfant et le `SyncSender` créé dans la méthode.

**Acceptation :** `spawn_profile_with_log_root` passe sous 100 lignes; Clippy descend de 93 à 92 sites ou moins sans nouvelle alerte; N1 passe; revue indépendante CLEAN; diff limité au fichier autorisé.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation ou test; changement de canal, cadrage/décodage de trame, erreurs, fermeture ou durée de vie du lecteur; fichier hors périmètre nécessaire; nouvelle alerte Clippy. Préserver les journaux.

**Retour arrière :** réintégrer la boucle et retirer son helper si une observation ou une erreur IPC diverge.
