# Plan P32 — Découper le test du spectre Parakeet

Avancée : cible autonome de 200+ lignes, avec oracle fréquentiel et WAV exact.
Blocage : aucun ; garder capture, progression événementielle et preuve WAV inchangées.
Décision : séparer préparation du runtime, contrôle des événements et comparaison WAV.

## Résumé et preuve

Le Clippy structurel post-P31 compte 100 sites. `parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav`, dans `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs:11`, dépasse le seuil de 100 lignes. Le test n’utilise que des blocs synthétiques et des ports contrôlés (classification `AUTONOME` dans `ci/tests-baseline.txt`). Il vérifie les événements de contrôle et de mesure, le pic à 500 Hz ±10 Hz, les 4 800 échantillons capturés et l’identité binaire du WAV avec ou sans lecteur de spectre.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/parakeet_live/microphone_spectrum_tests.rs`.
- Extraire des helpers privés de test pour l’arrangement du runtime, les assertions de progression/spectre et la construction/comparaison WAV.
- Préserver cinq blocs sinusoïdaux de 960 frames, les pauses de 20 ms, la deadline de 2 s et le polling; garder les événements injectés et contrôlés dans leur ordre.
- Préserver les seuils, messages d’assertion, échantillon count, chemins et comparaison d’octets WAV; ne pas modifier le runtime produit, le baseline ou la feature native.

## Tâche P32-T1

Extraire les étapes cohérentes pour que le test principal devienne un scénario d’orchestration lisible et que chaque helper reste sous 100 lignes. Les helpers restent dans le même fichier et la visibilité privée.

**Acceptation :** le test ciblé passe sous 100 lignes; ratchet 100→99 sites ou moins sans nouvelle alerte; N1 et la revue indépendante passent; aucune modification hors du fichier autorisé.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE. Témoin PE réel facultatif, à signaler non exécuté.

**Arrêt :** échec de garde, compilation ou test; différence de seuil, deadline, événement, valeur spectrale ou octet WAV; fichier hors périmètre nécessaire; nouvelle alerte structurelle. Préserver logs et état.

**Retour arrière :** retirer uniquement les extractions si l’oracle ou le comportement du scénario diffère.
