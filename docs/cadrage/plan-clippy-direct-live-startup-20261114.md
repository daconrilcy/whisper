# Plan P34 — Isoler le démarrage du worker direct

Avancée : cible de production à 145 lignes, couverte par des scénarios Auto synthétiques.
Blocage : aucun ; le worker démarre avant l’ouverture du microphone.
Décision : isoler sélection, repli autorisé et démarrage initial du worker.

## Résumé et preuve

Le Clippy structurel post-P33 compte 98 sites. `DirectLive::start_with_capture`, dans `crates/whisper-adapters/src/direct_live.rs:113`, déclenche `too_many_lines` à 145 lignes. Des tests N1 contrôlés couvrent Auto sans worker GPU, échec d’attestation GPU puis repli CPU, mode GPU manuel sans repli, et arrêt avant READY avec audio déjà enregistrée.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/direct_live.rs`.
- Extraire dans un helper privé la sélection initiale et le démarrage du worker, avec un résultat interne nommé si nécessaire; ne pas modifier les tests.
- Garder la sélection Auto et la préparation GPU avant le spawn, puis le spawn avant l’ouverture micro. Préserver `fallback_reason`, identité GPU, choix CPU/GPU, worker réutilisé sur le premier essai et `None` au retry CPU.
- Une seule tentative CPU si le spawn GPU échoue en Auto, avant traitement audio; aucun repli du GPU manuel; aucun autre comportement, champ ou type public modifié.
- Préserver l’abandon du worker et l’arrêt de la capture aux erreurs de setup, ainsi que toutes les valeurs dans `DirectLive`.

## Tâche P34-T1

Extraire l’initialisation du backend/worker hors de `start_with_capture`; garder la validation, l’ouverture micro et la composition de l’objet dans la fonction principale. Aucun nouveau test nécessaire : réutiliser les tests synthétiques existants de démarrage et de repli.

**Acceptation :** `start_with_capture` passe sous 100 lignes; le Clippy structural passe de 98 à 97 sites ou moins sans alerte nouvelle; tests Auto/CPU/GPU concernés et N1 passent; revue indépendante CLEAN; diff limité au fichier prévu.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE. Les tests natifs ignorés et validations manuelles restent explicitement non exécutés.

**Arrêt :** échec de garde, compilation ou test; changement du moment de sélection/repli/démarrage, du chemin microphone, de l’identité GPU ou du traitement audio; fichier hors périmètre nécessaire; nouvelle alerte structurelle. Préserver les journaux.

**Retour arrière :** retirer uniquement le helper P34 et sa délégation si un scénario de sélection, d’échec ou d’arrêt diffère.
