Avancée : le Clippy gardé après P52 signale le test de pages audio périmées à 145 lignes.
Blocage : le scénario combine des réponses périmées, une identité de requête, une nouvelle révision et une erreur de stockage.
Décision : découper uniquement ce scénario en helpers locaux, en conservant son ordre et tous ses oracles.

# Plan P53 — Décomposer le scénario de pages audio périmées

## Constat et preuve

Le Clippy standard frais après P52 signale `clippy::too_many_lines (145/100)` pour `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs::stale_audio_pages_reset_revision_and_preserve_real_errors_and_request_identity` (ligne 1019 dans le candidat P52). La mesure provient du journal gardé `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261114T205004639-159e8ba702aa4baba1e2542c5713751f/cargo-command.stdout.log`. Le ratchet courant est à 78 sites.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs`.
- Extraire au besoin des phases contiguës du même scénario en fonctions privées locales appelées par le test existant. Le corps de test et chaque helper restent sous 100 lignes et sous 25 de complexité cognitive.
- Préserver exactement le nom du test, la boucle sur les deux erreurs périmées, les identités, révisions, valeurs, appels, événements, ordre et assertions. Aucun changement de comportement, de fixture ou d’API.
- Préserver les changements antérieurs à P53 dans ce fichier. Ne pas toucher aux autres tests, aux imports, à la production, aux baselines ou aux manifestes, sauf ajustement de format requis dans la zone P53.

## Critères d’acceptation P53-T1

Pour chacune des deux erreurs `StaleResponse`, la réponse au `request_id + 1` laisse la révision 41 intacte; la réponse correspondant au `request_id` invalide la révision et conserve la page précédente ainsi que l’erreur visible. Une lecture à révision 0 efface cette erreur, puis la réponse de révision 72 met à jour session et page. Une erreur `StorageCorrupt` reste visible après un événement de progression. Les identités et assertions existantes demeurent présentes sans affaiblissement ni suppression.

Le test conserve son identité et son ordre, toutes les fonctions du fichier restent sous 100 lignes, le fichier reste sous 1 500 lignes, le Clippy au seuil cognitif 0 ne crée aucune alerte structurelle nouvelle et le ratchet descend de 78 à 77 sites ou moins. Revue indépendante CLEAN. fmt-check, N1 fraîche, PE SelfTest, harnais et sélections CPU, Parakeet CPU et common réussissent via la garde applicable.

## Vérifications et limites

Utiliser `ci/guard-level1.ps1` avec le LotRoot admis `target/parakeet-tests/clippy-corrections-suivantes-20261113`, `fmt --check`, mesure cognitive seuil 0, Clippy standard six membres et une cible N1 fraîche dédiée à P53. Exécuter le SelfTest PE, les compilations `--no-run`, les vérifications PE sur chaque chemin exact de harnais, les inventaires `-- --list` et les suites des trois workers conformément aux profils gardés. Toutes les commandes Cargo utilisent `--offline --locked` et les contrôles N1 prescrits.

Les tests nécessitant modèle, runtime natif ou matériel restent ignorés et sont rapportés comme non exécutés; le témoin PE réel reste non exécuté. Aucun test GPU niveau 2, build natif, essai dans la fenêtre Windows, téléchargement ou push.

Retour arrière limité aux helpers P53 et à l’appel de ce test; préserver le candidat P52 et les changements préexistants. L’utilisateur a autorisé la tranche P51–P60 le 14 novembre 2026; P53 est la troisième tâche de cette tranche.
