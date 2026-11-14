# Plan P46 — Décomposer l’accumulation des mesures micro

Avancée : le Clippy gardé après P45 compte 86 sites et relève `ProbeAccumulator::push` à 130 lignes.
Blocage : aucun établi; préserver les fenêtres de mesure et la correspondance temporelle micro.
Décision : isoler les phases de cumul, calcul RMS/PSD et construction du snapshot dans le même module.

## Constat

L’alerte `too_many_lines` vise `crates/whisper-adapters/src/microphone_probe.rs`, méthode `ProbeAccumulator::push`. Elle vérifie epoch et dimensions, accumule les échantillons avec une fenêtre bornée, décide quand publier RMS/PSD, met à jour les caches et construit le snapshot de télémétrie.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/microphone_probe.rs`.
- Réduire `push` sous 100 lignes en isolant des étapes cohérentes de validation/cumul, mise à jour RMS/PSD et construction du snapshot dans des helpers privés locaux.
- Garder chaque helper sous 100 lignes.
- Préserver les resets d’epoch et de trame, le rejet d’échantillons non finis, le bornage de la deque, les fenêtres et cadence RMS/FFT, les gains et compteurs de limiteur, les séquences, timestamps et valeurs de fallback.
- Aucun changement d’API, format de mesure, test ou autre fichier source.

## Tâche P46-T1

Décomposer uniquement `ProbeAccumulator::push` sans changer les calculs, les conditions de publication ou les champs du snapshot.

**Acceptation :** `push` sous 100 lignes; Clippy passe seuil 0 et le ratchet descend à 85 sites ou moins sans nouvelle alerte; N1 et sélections worker passent; revue indépendante CLEAN; diff source limité au fichier autorisé.

**Vérifications :** SelfTest de la garde, fmt-check, mesure cognitive seuil 0 et Clippy standard six crates via la garde, N1 sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation, test ou ratchet; fichier hors périmètre nécessaire; divergence de fenêtre, séquence, mesure, timestamp ou valeur publiée.

**Retour arrière :** restaurer la méthode initiale tout en conservant les changements hors de ce fichier.
