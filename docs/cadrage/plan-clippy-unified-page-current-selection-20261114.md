Avancée : le Clippy gardé après P50 signale `unified_page_responses_and_errors_follow_latest_selection_request` à 208 lignes.
Blocage : le test porte plusieurs générations de réponses périmées et leur priorité ; conserver exactement son identité et ses assertions.
Décision : extraire des helpers privés de scénario pour descendre le corps du test sous 100 lignes.

# Plan P51 — Réduire le test des réponses de pages unifiées

## Constat et preuve

Le Clippy frais gardé après P50 signale `too_many_lines (208/100)` sur `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs::unified_page_responses_and_errors_follow_latest_selection_request` (ligne 2). Ce test vérifie que les résultats et erreurs anciens des pages document/audio ne remplacent pas les réponses de la session et requête actuellement sélectionnées.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs`.
- Extraire au besoin la construction des requêtes, pages synthétiques et événements de réponse dans des helpers privés; conserver le test nommé, sa fixture, son ordre d’événements et ses assertions finales.
- Réduire le test principal sous 100 lignes; maintenir chaque helper sous 100 lignes et le fichier sous 1 500 lignes.
- Ne modifier aucun comportement de production, identité de test, donnée/assertion, fixture, feature, baseline ou manifeste. Aucun autre fichier n’est autorisé.

## Critères d’acceptation P51-T1

1. Le même test est toujours directement exécuté sous le nom existant et draine tous les événements.
2. Les réponses périmées A (succès et erreur document/audio), les réponses B, les dernières réponses A, les limites 128, le nombre de rafraîchissements et les assertions sur les pages et erreurs restent identiques.
3. Le test fait moins de 100 lignes, tous ses helpers font moins de 100 lignes, le fichier reste sous 1 500 lignes et le diff ne concerne que le fichier autorisé.
4. Le Clippy structurel au seuil cognitif 0 passe; le ratchet descend de 81 à 80 sites au plus, sans nouvelle alerte.
5. Le format, SelfTests, N1 gardée, PE et sélections des trois workers passent; la revue indépendante déclare CLEAN.

## Vérifications

Via `ci/guard-level1.ps1`, au `LotRoot` déjà admis `target/parakeet-tests/clippy-corrections-suivantes-20261113` : `fmt --check`; mesure cognitive au seuil 0 puis Clippy standard exact à six membres; N1 `test --no-fail-fast` sur cible fraîche propre à P51; SelfTest PE; compilation `--no-run`, contrôles PE exacts, inventaires `-- --list` et suites de `whisper-worker-cpu`, `whisper-worker-parakeet-cpu`, `whisper-worker-common`. La cible neuve sera nommée `n1-p51-20261114`. Respecter le préflight du dépôt, les limites mémoire/cache, `--offline --locked` et les conditions d’arrêt de la garde.

Les tests dépendant d’un modèle, d’un runtime natif ou d’un corpus réel restent ignorés/non exécutés et ne sont pas présentés comme validés. Le témoin PE réel reste `NON EXÉCUTÉ` sans sélection et SHA autorisés. Aucun test GPU niveau 2, build natif, validation Windows, téléchargement ou push.

## Retour arrière et autorisation

Retour arrière limité au rétablissement du corps original du test dans ce fichier, en conservant toutes les autres modifications du worktree. La campagne de structure Clippy P51–P60 a été explicitement autorisée par l’utilisateur le 14 novembre 2026; P51 est la première tâche de cette tranche. Cette autorisation ne couvre aucun chemin supplémentaire.