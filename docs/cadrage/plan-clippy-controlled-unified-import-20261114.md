# Plan P48 — Décomposer l’admission d’un import unifié contrôlé

Avancée : le Clippy gardé après P47 compte 84 sites et relève `start_controlled_unified_import` à 132 lignes.
Blocage : `runtime/import.rs` contient des changements antérieurs distincts dans `publish_inner` et `stop_effect`.
Décision : extraire les étapes de préparation source/document de la méthode ciblée, sans toucher aux autres hunk.

## Constat

L’alerte `too_many_lines` vise `crates/whisper-adapters/src/worker_ipc/runtime/import.rs`, méthode `ImportRuntime::start_controlled_unified_import`. Les diff préexistants concernent `publish_inner` et `stop_effect`; ils sont disjoints de la méthode ciblée.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/worker_ipc/runtime/import.rs`, corps de `start_controlled_unified_import` et helpers privés locaux uniquement.
- Préserver sans modification les hunks préexistants de `publish_inner` et `stop_effect`.
- Réduire la méthode sous 100 lignes; garder chaque helper sous 100 lignes.
- Préserver l’ordre des contrôles d’admission et de route, les messages d’erreur, la création/décodage durable de la source, l’ouverture/reprise du document, le calcul de séquence, l’admission scheduler, l’activation des barrières, l’enregistrement de session/archive et le retrait final du checkpoint.
- Aucun changement d’API, protocole, test ou autre fichier source.

## Tâche P48-T1

Décomposer uniquement `start_controlled_unified_import` en phases privées cohérentes sans réordonner les effets ou erreurs observables.

**Acceptation :** méthode sous 100 lignes; Clippy passe seuil 0 et le ratchet descend à 83 sites ou moins sans nouvelle alerte; N1 et sélections worker passent; revue indépendante CLEAN; hunks préexistants intacts.

**Vérifications :** SelfTest de la garde, fmt-check, mesure cognitive seuil 0 et Clippy standard six crates via la garde, N1 sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation, test ou ratchet; chevauchement non prévu; fichier hors périmètre nécessaire; divergence de réservation, admission scheduler, état durable ou ordre d’erreur.

**Retour arrière :** restaurer uniquement le corps P48 de `start_controlled_unified_import`, en conservant les autres hunks du fichier.
