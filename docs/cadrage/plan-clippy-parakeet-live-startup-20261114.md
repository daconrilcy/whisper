# Plan P36 — Isoler le démarrage du worker Parakeet live

Avancée : Clippy relève `start_with_capture` à 124 lignes et 96 sites après P35.
Blocage : aucun ; l’ordre capture/WAV puis démarrage du worker doit rester identique.
Décision : extraire uniquement la préparation des canaux et le spawn du worker.

## Résumé et preuve

Le Clippy gardé après P35 signale `crates/whisper-adapters/src/parakeet_live/lifecycle.rs:23`, `ParakeetLive::start_with_capture`, au-dessus du seuil de 100 lignes. Cette méthode valide la demande et les chemins, démarre la capture et son WAV durable avant le worker, puis assemble l’état de session.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/parakeet_live/lifecycle.rs`.
- Extraire la création des canaux d’inférence, du signal d’annulation, de l’événement initial et le spawn du thread dans un helper privé et un petit résultat de démarrage si nécessaire.
- Conserver dans `start_with_capture` la validation des entrées, la préparation et l’ordre capture, convertisseur, probe, WAV et répertoire de fenêtres, puis l’assemblage de `ParakeetLive`.
- Préserver les capacités des canaux, le nom du thread, le contexte worker, les diagnostics D2, le backend annoncé et le message d’erreur de spawn.
- Aucun test, protocole, module externe, feature ou comportement public ne change.

## Tâche P36-T1

Introduire un helper privé de démarrage du worker, appelé après la préparation du WAV et du répertoire, puis réutiliser le résultat pour construire `ParakeetLive`.

**Acceptation :** `start_with_capture` passe sous 100 lignes; Clippy descend de 96 à 95 sites ou moins sans nouvelle alerte; N1 passe; revue indépendante CLEAN; diff limité au fichier autorisé.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE. Les scénarios dépendant d’un worker/modèle réel demeurent ignorés et sont signalés.

**Arrêt :** échec de garde, compilation ou test; altération de l’ordre capture/WAV/worker, des canaux bornés ou de l’événement initial; fichier hors périmètre nécessaire; nouvelle alerte Clippy. Préserver les journaux.

**Retour arrière :** retirer uniquement le helper et son appel si l’ordre, les ressources ou les erreurs divergent.
