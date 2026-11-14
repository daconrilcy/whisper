# Plan P33 — Découper l’ingestion des hypothèses Parakeet

Avancée : cible de production pure à 143 lignes avec caractérisations synthétiques existantes.
Blocage : aucun ; l’ordre validation→état→concordance→récupération→publication est contractuel.
Décision : extraire des helpers privés sans changer l’algorithme de confirmation.

## Résumé et preuve

Le Clippy structurel post-P32 compte 99 sites. `Confirmation::ingest_observed`, dans `crates/whisper-adapters/src/parakeet_confirmation.rs`, déclenche `too_many_lines` à 143 lignes. Le module contient déjà des scénarios synthétiques pour les fenêtres distinctes, l’ancrage de contexte, les gaps non résolus, la récupération ciblée et la publication finale.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/parakeet_confirmation.rs`.
- Extraire uniquement des fonctions privées au même module; aucun type ou comportement public ne change.
- Préserver les refus avant mutation (séquence, taux et bornes; mots non blancs, taux, inclusion, ordre), puis l’enregistrement de la preuve, l’avancement monotone des frontières, la concordance par mot, la révision avec contexte, le marquage de conflits, l’expiration/bornage des pending, la résolution des gaps et la publication.
- Préserver tous les tests/assertions existants et leurs seuils. Ne pas modifier les tests inline, les contrats, les constantes, la baseline, l’audio ou les artefacts.

## Tâche P33-T1

Déplacer les groupes cohérents de travail de `ingest_observed` vers des helpers privés, en transmettant les mêmes données et en gardant le même ordre d’appel. Toute mutation doit rester après les validations correspondantes.

**Acceptation :** `ingest_observed` passe sous 100 lignes; Clippy passe de 99 à 98 sites ou moins sans alerte nouvelle; les caractérisations N1 de confirmation et la suite N1 passent; revue indépendante CLEAN; diff limité au fichier prévu.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE. Témoin PE réel facultatif, à signaler non exécuté.

**Arrêt :** échec de garde, compilation ou test; toute différence de refus, frontière monotone, fenêtre témoin, gap, conflit ou publication; nouveau fichier nécessaire; nouvelle alerte Clippy. Préserver les journaux et l’état.

**Retour arrière :** retirer uniquement les helpers et délégations P33 si une caractérisation de confirmation diffère.
