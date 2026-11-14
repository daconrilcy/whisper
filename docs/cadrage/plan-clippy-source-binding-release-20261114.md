# Plan P43 — Isoler la libération d’une identité de capture

Avancée : le Clippy gardé après P42 compte 89 sites et relève `release_source_binding_inner` à 122 lignes.
Blocage : aucun établi; préserver les contrôles d’intégrité et d’orphelins des instantanés.
Décision : extraire la branche de libération d’une capture unifiée dans des helpers privés locaux.

## Constat

L’alerte `too_many_lines` vise `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs`, méthode `release_source_binding_inner`. Elle retire d’abord une réservation active; pour une identité de capture unifiée sans réservation en mémoire, elle inspecte les artefacts durables et le registre avant de décider si la libération est sûre.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs`.
- Extraire la branche d’identité de capture unifiée et, au besoin, l’inspection des artefacts de snapshot dans des helpers privés du même module.
- Réduire `release_source_binding_inner` sous 100 lignes; garder chaque helper sous 100 lignes.
- Préserver les mêmes opérations filesystem, l’ordre des vérifications et des suppressions, les erreurs et leurs textes, les règles d’absence du registre et l’inspection finale des artefacts après retrait.
- Aucun changement d’API, protocole, test ou autre fichier source.

## Tâche P43-T1

Décomposer uniquement la branche « capture unifiée sans réservation active » en helpers privés; laisser le chemin de réservation active et le fallback non unifié inchangés.

**Acceptation :** méthode sous 100 lignes; Clippy passe seuil 0 et le ratchet descend à 88 sites ou moins sans nouvelle alerte; N1 et sélections worker passent; revue indépendante CLEAN; diff source limité au fichier autorisé.

**Vérifications :** SelfTest de la garde, fmt-check, mesure cognitive seuil 0 et Clippy standard six crates via la garde, N1 sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation, test ou ratchet; fichier hors périmètre nécessaire; altération de l’ordre des vérifications, de la décision de retrait ou du texte d’erreur.

**Retour arrière :** restaurer le corps initial de `release_source_binding_inner` tout en conservant les changements hors de ce fichier.
