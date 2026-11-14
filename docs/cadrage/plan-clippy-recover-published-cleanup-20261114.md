# Plan P49 — Décomposer la reprise d’un nettoyage publié

Avancée : le Clippy gardé après P48 compte 83 sites et relève `recover_published_cleanup_successor` à 133 lignes.
Blocage : aucun diff source antérieur dans `archive/cleanup/proofs.rs`; préserver l’ordre de récupération Windows.
Décision : isoler la validation des preuves et la reprise de l’index sans déplacer le marqueur CURRENT.

## Constat

L’alerte `too_many_lines` vise `crates/whisper-adapters/src/archive/cleanup/proofs.rs`, la variante `#[cfg(windows)]` de `recover_published_cleanup_successor`. Elle vérifie l’identité du successeur et ses preuves, contrôle l’intention de publication et utilise `CURRENT` comme marqueur de commit avant de restaurer l’index.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/archive/cleanup/proofs.rs`.
- Réduire la variante Windows de `recover_published_cleanup_successor` sous 100 lignes avec des helpers privés locaux, sous le même `cfg(windows)` si nécessaire.
- Garder chaque helper sous 100 lignes; ne pas modifier la variante `cfg(not(windows))`.
- Préserver l’ordre des validations canoniques, identités de fichiers, SHA-256, manifeste/pending/segments, intention de publication, `CURRENT`, écriture d’index et mise à jour du plan de nettoyage.
- Garder les handles `VerifiedFile` vivants pendant toute l’opération de récupération, comme aujourd’hui.
- Aucun changement de protocole, API publique, test ou autre fichier source.

## Tâche P49-T1

Décomposer uniquement la récupération Windows en phases privées sans changer le point de commit, les erreurs ni la durée de vie des preuves ouvertes.

**Acceptation :** fonction sous 100 lignes; Clippy passe seuil 0 et le ratchet descend à 82 sites ou moins sans nouvelle alerte; N1 et sélections worker passent; revue indépendante CLEAN; diff source limité au fichier autorisé.

**Vérifications :** SelfTest de la garde, fmt-check, mesure cognitive seuil 0 et Clippy standard six crates via la garde, N1 sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation, test ou ratchet; fichier hors périmètre nécessaire; divergence du contrôle d’identité, de l’intention, du commit `CURRENT`, de l’index restauré ou des handles maintenus.

**Retour arrière :** restaurer uniquement la variante Windows initiale sans toucher à la variante non-Windows.
