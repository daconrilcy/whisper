# Plan P44 — Décomposer l’envoi des fenêtres live

Avancée : le Clippy gardé après P43 relève `pump_live_windows` à 125 lignes.
Blocage : aucun établi; préserver l’audio durable, le replay et la contre-pression bornée.
Décision : séparer la préparation, l’envoi de fenêtre et l’envoi final Stop dans le module de polling.

## Constat

L’alerte `too_many_lines` vise `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs`, méthode `pump_live_windows`. Elle prépare une fenêtre à partir des échantillons durables, gère le replay et la file worker bornée, puis envoie Stop après drainage complet.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/worker_ipc/runtime/poll/capture.rs`.
- Extraire uniquement les étapes cohérentes de préparation/lecture de fenêtre, soumission de fenêtre et soumission Stop dans des helpers privés.
- Réduire `pump_live_windows` sous 100 lignes; garder chaque helper sous 100 lignes.
- Préserver la taille et les bornes de fenêtre, l’ordre replay/live, les curseurs et séquences, la remise en pending à saturation, les erreurs de stockage/worker, et l’envoi unique de Stop uniquement après drainage durable.
- Aucun changement de protocole, API, test ou autre fichier source.

## Tâche P44-T1

Décomposer le corps de `pump_live_windows` en helpers privés du même module sans changer les transitions d’état ni l’ordre des effets.

**Acceptation :** méthode sous 100 lignes; Clippy passe seuil 0 et le ratchet descend d’au moins un site sans nouvelle alerte; N1 et sélections worker passent; revue indépendante CLEAN; diff source limité au fichier autorisé.

**Vérifications :** SelfTest de la garde, fmt-check, mesure cognitive seuil 0 et Clippy standard six crates via la garde, N1 sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation, test ou ratchet; fichier hors périmètre nécessaire; changement de fenêtre, perte/double envoi d’échantillons, ordre replay/live, comportement en saturation ou arrêt anticipé.

**Retour arrière :** restaurer le corps monolithique initial tout en conservant les changements hors du fichier.
