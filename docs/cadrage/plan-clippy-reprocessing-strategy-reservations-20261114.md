Avancée : le Clippy gardé après P54 signale le test de réservation par stratégie à 212 lignes.
Blocage : ce scénario caractérise les réservations de publications, le filtrage de stratégie et le rejet d’un index modifié.
Décision : extraire les phases de fixture, de réservation et de mutation contrôlée en helpers locaux sans affaiblir les assertions.

# Plan P55 — Décomposer le scénario de réservation par stratégie

## Constat et preuve

Le Clippy standard frais après P54 signale `clippy::too_many_lines (212/100)` pour `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs::reprocessing_reservation_tracks_only_current_strategy_publications_for_resume` (ligne 279 dans le candidat P54). La mesure est dans `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261114T212115133-49b4b151d1eb42f9b0bfcf8cfed919c3/cargo-command.stdout.log`. Le ratchet courant est à 76 sites; la mesure cognitive seuil 0 du test est 9.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs`.
- Décomposer uniquement le test nommé en helpers privés locaux; si une donnée de scénario doit être regroupée, n’ajouter qu’un support privé limité à cette caractérisation dans ce même fichier.
- Conserver le test sous 100 lignes, chaque helper sous 100 lignes et sous 25 de complexité cognitive, les appels sous 8 arguments et le fichier sous 1 500 lignes.
- Préserver l’identité et l’ordre des opérations, fixtures de fichiers, valeurs/identités, réservations, résultat attendu et nettoyage du dossier temporaire. N’ajouter ni nouvelle assertion ni modification de production ou d’autres tests.

## Critères d’acceptation P55-T1

Le test Windows conserve les publications complètes pour `BoundedWindowsV1` et `LegacyV1`, les IDs `0x5301` et `0x5302`, la source partagée et son empreinte, puis les deux réservations qui ne capturent que la publication de leur stratégie respective. Après désactivation de la publication bornée dans l’index, la réservation de remplacement échoue toujours avec `DuplicateStateChanged`. Les manifestes, segments, exports, versions, ordinals, générations et assertions restent intacts, dans le même ordre; le dossier temporaire est supprimé comme avant.

Le Clippy seuil cognitif 0 ne révèle pas d’alerte structurelle nouvelle; le Clippy standard passe avec ratchet à 75 sites ou moins. La revue indépendante est CLEAN. fmt-check, N1 fraîche, PE SelfTest, harnais exacts et inventaires/suites CPU, Parakeet CPU et common réussissent via la garde.

## Vérifications et limites

Via `ci/guard-level1.ps1` et le LotRoot `target/parakeet-tests/clippy-corrections-suivantes-20261113`: fmt-check, seuil cognitif 0, Clippy standard six membres, N1 sur une cible fraîche P55, contrôles PE, compilations `--no-run`, inventaires et suites workers. Le cache Cargo partagé du worktree étant absent, sélectionner une cible fraîche gardée sous le lot; ne pas rétablir le cache manquant. Toutes les commandes Cargo restent offline et locked.

Les scénarios nécessitant modèle, runtime natif ou matériel restent ignorés/non exécutés; témoin PE réel non exécuté. Pas de test GPU niveau 2, build natif, essai dans la fenêtre Windows, téléchargement ni push.

Retour arrière limité au test et aux helpers P55. L’utilisateur a autorisé P51–P60 le 14 novembre 2026; P55 est la cinquième tâche de cette tranche.
