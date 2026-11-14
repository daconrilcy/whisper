# Plan P45 — Décomposer le démarrage de l’inventaire GPU

Avancée : le Clippy gardé après P44 compte 87 sites et relève `WindowsGpuInventoryPort::start` à 129 lignes.
Blocage : aucun établi; préserver les résultats et diagnostics propres aux plateformes.
Décision : extraire le traitement de collecte/qualification GPU en helpers privés dans le même fichier.

## Constat

L’alerte `too_many_lines` vise `crates/whisper-adapters/src/gpu_inventory.rs`, méthode `WindowsGpuInventoryPort::start`. Elle installe les canaux et la génération, lance un thread, publie la collecte brute, rapproche les observations avec DXGI sous Windows, qualifie les cibles via le worker GPU puis publie les résultats.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/gpu_inventory.rs`.
- Réduire `start` sous 100 lignes en isolant les étapes cohérentes de traitement de collecte, rapprochement DXGI et qualification dans des helpers privés locaux.
- Garder chaque helper sous 100 lignes et conserver les branches `cfg(windows)` / `cfg(not(windows))` applicables.
- Préserver l’ordre des messages sur les trois canaux, les générations, les journaux, le repli non qualifié sur erreur DXGI, l’état atomique `collecting` et son reset en cas d’échec de spawn.
- Aucun changement d’API publique, protocole, manifeste, test ou autre fichier source.

## Tâche P45-T1

Décomposer uniquement le traitement de thread de `start` sans changer l’ordre d’émission ni le comportement de collecte/qualification.

**Acceptation :** `start` sous 100 lignes; Clippy passe seuil 0 et le ratchet descend à 86 sites ou moins sans nouvelle alerte; N1 et sélections worker passent; revue indépendante CLEAN; diff source limité au fichier autorisé.

**Vérifications :** SelfTest de la garde, fmt-check, mesure cognitive seuil 0 et Clippy standard six crates via la garde, N1 sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation, test ou ratchet; fichier hors périmètre nécessaire; divergence d’ordre des messages, d’identité de génération, de journaux, de repli DXGI ou de fermeture du thread.

**Retour arrière :** restaurer le corps initial de `start` tout en conservant les changements hors du fichier.
