# Plan P47 — Décomposer la publication d’un import

Avancée : le Clippy gardé après P46 compte 85 sites et relève `publish_inner` à 127 lignes.
Blocage : le fichier a un diff antérieur dans `stop_effect`, hors de la méthode ciblée.
Décision : isoler les phases de vérification et d’export final tout en gardant la transaction de publication ordonnée.

## Constat

L’alerte `too_many_lines` vise `crates/whisper-adapters/src/worker_ipc/runtime/import.rs`, méthode `ImportRuntime::publish_inner`. Le diff préexistant de ce fichier ne concerne que `stop_active_parakeet_worker` et `stop_effect` (lignes proches de 919–1084), sans chevauchement avec `publish_inner`.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/worker_ipc/runtime/import.rs`, corps de `publish_inner` et helpers privés nécessaires dans le même `impl`.
- Ne pas toucher au diff préexistant de `stop_effect` ni aux autres méthodes du fichier.
- Réduire `publish_inner` sous 100 lignes; garder chaque helper sous 100 lignes.
- Préserver les barrières avant/après commit et `PublicationGate`, les événements et leur ordre, le double scan stable, la libération de source, la publication TXT/SRT, les manifestes, le cycle de vie du worker idle et les erreurs.
- Aucun changement d’API, protocole, test ou autre fichier source.

## Tâche P47-T1

Décomposer uniquement `publish_inner` en phases privées cohérentes en gardant l’ordre de toutes les opérations et effets observables.

**Acceptation :** méthode sous 100 lignes; Clippy passe seuil 0 et le ratchet descend à 84 sites ou moins sans nouvelle alerte; N1 et sélections worker passent; revue indépendante CLEAN; aucun chevauchement avec le diff `stop_effect` préexistant.

**Vérifications :** SelfTest de la garde, fmt-check, mesure cognitive seuil 0 et Clippy standard six crates via la garde, N1 sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation, test ou ratchet; chevauchement non prévu; fichier hors périmètre nécessaire; divergence d’ordre transactionnel, de publication ou d’événements.

**Retour arrière :** restaurer uniquement le corps P47 de `publish_inner` sans toucher au diff préexistant de `stop_effect` ni aux autres changements.
