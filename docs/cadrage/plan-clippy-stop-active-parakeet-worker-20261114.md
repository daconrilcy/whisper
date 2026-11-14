# Plan P38 — Isoler l’arrêt d’un worker Parakeet actif

Avancée : après P37, Clippy relève `stop_effect` à 111 lignes et 94 sites.
Blocage : aucun ; terminer le processus et libérer l’identité avant l’I/O du journal de file.
Décision : extraire le sous-parcours d’arrêt du worker Parakeet historique actif.

## Résumé et preuve

Le Clippy gardé après P37 signale `crates/whisper-adapters/src/worker_ipc/runtime/import.rs:922`, `ImportRuntime::stop_effect`, à 111 lignes. Cette méthode route plusieurs parcours : route unifiée préparée, microphone Parakeet live, worker Parakeet historique, micro direct et worker générique.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/worker_ipc/runtime/import.rs`.
- Extraire uniquement le sous-parcours où une requête active correspondante et un worker Parakeet historique sont présents : terminaison, libération de l’état mémoire, nettoyage du fichier VAD et progression, puis mise à jour éventuelle de la file et événements de sortie.
- Conserver dans `stop_effect` la sélection des routes et les parcours unifiés, live, direct et générique.
- Préserver strictement l’ordre de terminaison avant l’état mémoire, puis de journalisation; conserver les événements `Queue`/`Stopped`, les erreurs et le statut `Interrupted`.
- Aucun type public, protocole, test, autre module ou comportement ne change.

## Tâche P38-T1

Créer un helper privé dans le même `impl ImportRuntime`, appelé seulement lorsque le worker historique actif est pris en charge; réduire `stop_effect` sous 100 lignes sans déplacer ses autres branches.

**Acceptation :** `stop_effect` passe sous 100 lignes; Clippy descend de 94 à 93 sites ou moins sans nouvelle alerte; N1 passe; revue indépendante CLEAN; diff limité au fichier autorisé.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation ou test; ordre terminaison/nettoyage/journalisation modifié; différence d’événements, erreur ou mise à jour de statut; fichier hors périmètre nécessaire; nouvelle alerte Clippy. Préserver les journaux.

**Retour arrière :** retirer uniquement le helper et son appel si un résultat d’arrêt diffère.
