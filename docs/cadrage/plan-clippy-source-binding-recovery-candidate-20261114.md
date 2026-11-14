Avancée : Clippy frais après P57 relève `recovery_candidate` à 129 lignes.
Blocage : l’évaluation séquentielle protège réservation, destination, file, archive, publication, snapshots et identité.
Décision : extraire des phases privées dans le même fichier sans modifier l’ordre ni les résultats observables.

# Plan P58 — Décomposer la qualification des candidats de récupération

## Constat et preuve

La Clippy standard fraîche exécutée après P57 relève `clippy::too_many_lines (129/100)` pour `crates/whisper-adapters/src/source_binding.rs::recovery_candidate` (ligne 675), dans `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261114T221743836-38e07c66afdf40e1bbf88d92e406bd2c/cargo-command.stdout.log`. Le fichier source est propre avant P58 et compte 1 719 lignes.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-adapters/src/source_binding.rs`.
- Extraire uniquement des phases privées de `recovery_candidate`, sans nouvelle abstraction publique, dépendance, changement de stockage ni nouveau fichier.
- Le delta net du fichier est limité à +30 lignes ; la fonction et les helpers concernés respectent les seuils de 100 lignes, 25 de complexité cognitive, 7 arguments et 250 de complexité de type.
- Conserver l’ordre des gardes et leurs raisons exactes : session active, opération occupée, réservation non protégée, canonicalisation, file durable, archive/pending, réservation de publication, copie partielle, reprise Retiring/intention/pending, absence ou paire incomplète, statut Bound et validation de la paire. Préserver les mêmes erreurs propagées et le même résultat pour chaque branche.
- Aucun changement de comportement de récupération, suppression, identité de fichier, publication, autre fonction ou autre test.

## Critères d’acceptation P58-T1

Les cas de caractérisation déjà présents autour de `inspect_recovery_candidates` et `recover_abandoned` dans le module `source_binding.rs` réussissent ; les raisons exposées, recoverable, identités job/generation/destination et ordre des premières protections restent identiques. Aucun test n’est déplacé ni affaibli.

La mesure Clippy seuil 0 n’ajoute aucune alerte structurelle ; Clippy standard passe avec ratchet à 72 sites ou moins. Revue indépendante CLEAN. fmt-check, SelfTest de garde, N1 fraîche, PE SelfTest, les dix harnais exacts, inventaires et suites CPU, Parakeet CPU et common passent via `ci/guard-level1.ps1`.

## Vérifications et limites

Utiliser `ci/guard-level1.ps1` avec le LotRoot `target/parakeet-tests/clippy-corrections-suivantes-20261113`, toutes commandes Cargo offline et locked, cibles fraîches dédiées au lot. La cible N1 reçoit `-PersonalParakeetModelPath C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf`. Contrôler l’espace libre avant compilation et ne pas modifier la baseline structurelle.

Les tests ignorés restent NON EXÉCUTÉS ; témoin PE réel non exécuté. Pas de test GPU niveau 2, build natif, essai dans la fenêtre Windows, téléchargement, nettoyage ou push.

Retour arrière limité aux helpers extraits de P58. La tranche P51–P60 est autorisée par l’utilisateur le 14 novembre 2026 ; P58 est la huitième tâche.
