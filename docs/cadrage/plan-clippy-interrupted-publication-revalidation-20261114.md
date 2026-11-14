# Plan P40 — Isoler la revalidation d’une publication interrompue

Avancée : après P39, Clippy relève `confirm_interrupted_publication` à 112 lignes et 92 sites.
Blocage : aucun ; préserver l’identité de source, les contrôles d’archive et la reconfirmation.
Décision : extraire uniquement le calcul de revalidation et ses réponses de prompt.

## Résumé et preuve

Le Clippy gardé après P39 signale `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:96`. La méthode vérifie le prompt de reprise, rebinde et identifie la source, compare les publications courantes, revalide le nom réservé, reconfirme le choix de remplacement puis reprend ou réémet un prompt actualisé.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs`.
- Extraire la closure de revalidation (binding, identification de source, campagne archive, comparaisons, publication active, basename et reconfirmation) vers un helper privé dans le même `impl ImportRuntime`.
- Laisser dans `confirm_interrupted_publication` les gardes sur `resume_from`/nom réservé et le dispatch final vers `ResumeInterrupted` ou `Failed`.
- Préserver les mutations du prompt, les messages de revalidation, les erreurs et le moment où les événements sont ajoutés.
- Aucun changement de protocole, type public, autre fichier, feature ou comportement.

## Tâche P40-T1

Introduire un helper privé qui retourne le même `Result<Option<ImportRequest>, String>` et reçoit le prompt mutable, la requête précédente, le basename réservé, le choix et la file d’événements.

**Acceptation :** `confirm_interrupted_publication` passe sous 100 lignes; Clippy descend de 92 à 91 sites ou moins sans nouvelle alerte; N1 passe; revue indépendante CLEAN; diff limité au fichier autorisé.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation ou test; divergence d’identité/source, archive, basename, revalidation de choix, messages ou ordre d’événements; fichier hors périmètre nécessaire; nouvelle alerte Clippy. Préserver les journaux.

**Retour arrière :** remettre la closure dans la méthode et retirer le helper si une confirmation ou une erreur diverge.
