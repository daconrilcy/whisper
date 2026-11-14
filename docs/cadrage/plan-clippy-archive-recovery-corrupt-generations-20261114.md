Avancée : le Clippy gardé après P53 signale le test de scan d’archives corrompues à 179 lignes.
Blocage : le scénario prépare plusieurs générations saines, corrompues, transplantées et leur journal de file.
Décision : extraire ses phases de préparation et d’assertion en helpers locaux, sans changer les fixtures ni les attentes.

# Plan P54 — Décomposer la caractérisation du scan de récupération

## Constat et preuve

Le Clippy standard frais après P53 signale `clippy::too_many_lines (179/100)` pour `crates/whisper-adapters/src/archive/tests/recovery.rs::scan_preserves_healthy_entries_and_reports_each_corrupt_generation` (ligne 35 dans le candidat P53). La mesure provient du journal gardé `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261114T210411677-f35017f8f337410894b2b10906bc8bd1/cargo-command.stdout.log`. Le ratchet courant est à 77 sites.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-adapters/src/archive/tests/recovery.rs`.
- Extraire les phases de préparation des fixtures et de vérification des résultats en helpers privés locaux appelés par le test existant. Le corps du test et chaque helper restent sous 100 lignes et sous 25 de complexité cognitive.
- Préserver le nom du test, la structure exacte des entrées disque, les identités, générations, contenus et ordre des opérations/assertions. Aucun comportement produit, API, fixture observée ou résultat attendu n’est modifié.
- Ne modifier aucun autre test ou fichier, production, manifeste ou baseline. Préserver l’état Git sale préexistant hors de la seule zone explicitement autorisée.

## Critères d’acceptation P54-T1

Le test continue à créer la génération saine (JobId 7), les entrées corrompues 8, 9 et 10, le pointeur CURRENT traversant pour 11, la génération transplantée 12, et le journal `queue.jsonl` non répertorié comme tâche. Deux scans successifs retournent les mêmes diagnostics. L’entrée saine demeure complète; chaque tâche corrompue reçoit un diagnostic; les cas 11 et 12 ne sont pas marqués complets. Les assertions et l’ordre d’exécution existants restent intégralement présents.

Le nom du test est inchangé, son corps et chaque helper restent sous 100 lignes, le fichier reste sous 1 500 lignes, le Clippy au seuil cognitif 0 ne signale aucune nouvelle alerte, et le Clippy standard passe avec ratchet à 76 sites ou moins. Revue indépendante CLEAN. fmt-check, N1 frais, PE SelfTest, harnais exacts et inventaires/suites CPU, Parakeet CPU et common réussissent via la garde.

## Vérifications et limites

Utiliser `ci/guard-level1.ps1` avec le LotRoot `target/parakeet-tests/clippy-corrections-suivantes-20261113`. Le cache Cargo partagé de ce worktree étant absent au préflight P53, utiliser une cible fraîche autorisée distincte sous ce LotRoot, sans recréer le cache partagé. Exécuter fmt-check, mesure cognitive seuil 0, Clippy standard exact six membres, N1 sur une cible fraîche dédiée à P54, SelfTest PE, compilations `--no-run`, contrôles PE par chemin de harnais exact, inventaires `-- --list` et suites des trois workers. Commandes Cargo offline et locked selon la garde.

Les tests qui exigent un modèle, runtime natif ou matériel demeurent ignorés/non exécutés; témoin PE réel non exécuté. Pas de test GPU niveau 2, build natif, essai dans la fenêtre Windows, téléchargement ou push.

Retour arrière limité aux helpers P54 et à leur usage dans le test. L’utilisateur a autorisé la tranche P51–P60 le 14 novembre 2026; P54 est la quatrième tâche de cette tranche.
