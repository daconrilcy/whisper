Avancée : le Clippy gardé après P55 signale le test UI de capture Turbo terminée à 141 lignes.
Blocage : le test relie démarrage Turbo, état actif, arrêt/drain, libération moteur et admission Parakeet.
Décision : extraire les phases UI en helpers privés du test en préservant les actions et tous les oracles.

# Plan P56 — Décomposer la transition UI Turbo vers Parakeet

## Constat et preuve

Le Clippy standard frais après P55 signale `clippy::too_many_lines (141/100)` pour `crates/whisper-desktop/src/ui/tests/unified_sessions.rs::turbo_completed_capture_allows_unified_parakeet_button_and_dispatch` (ligne 368 dans le candidat P55). La mesure est dans `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261114T214412531-44d0900553ba45a9a5c7746e8748ac14/cargo-command.stdout.log`. Le seuil cognitif 0 donne 13. Le ratchet courant est à 75 sites.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-desktop/src/ui/tests/unified_sessions.rs`.
- Extraire uniquement les phases de ce test en helpers privés locaux appelés par le test. Le test et chaque helper restent sous 100 lignes et sous 25 de complexité cognitive; aucun helper ne dépasse 7 arguments.
- Préserver l’identité du test, le faux port local et ses événements, valeurs/identités/générations, choix Turbo/Parakeet/langue, actions de clic, ordre, assertions et messages. Pas de changement du code produit, du rendu, d’autre test ou du support.
- Maintenir le fichier sous 1 500 lignes et préserver les changements antérieurs qui s’y trouvent.

## Critères d’acceptation P56-T1

Le test démarre le direct Turbo CPU depuis l’interface, confirme le démarrage puis la phase Active et la commande Stop avec le même `JobId`/`Generation`. En drain, l’admission unifiée reste désactivée. Après `Ready` et `DirectPublished`, le job est Complete, la capture est Stopped et le moteur Released. Parakeet reste désactivé sous langue française manuelle puis devient disponible sous langue automatique; le clic envoie `StartUnifiedLiveConfigured`. Les assertions et leur ordre restent identiques.

Le test et chaque helper ont moins de 100 lignes; le seuil cognitif 0 ne produit pas de nouvelle alerte; le ratchet standard descend de 75 à 74 sites ou moins. Revue indépendante CLEAN. fmt-check, N1 frais, PE SelfTest, harnais exacts et inventaires/suites workers réussissent via la garde.

## Vérifications et limites

Via `ci/guard-level1.ps1` et le LotRoot `target/parakeet-tests/clippy-corrections-suivantes-20261113`: fmt-check, mesure cognitive seuil 0, Clippy standard six membres et N1 sur une cible P56 fraîche. Le cache Cargo partagé du worktree étant absent, utiliser une cible fraîche gardée sous le lot. Effectuer aussi SelfTest PE, compilations `--no-run`, contrôles PE sur les chemins exacts, inventaires et suites CPU, Parakeet CPU et common. Cargo reste offline et locked.

Ce test automatisé ne qualifie pas le rendu dans la fenêtre Windows. Les scénarios exigeant modèle, runtime natif ou matériel demeurent ignorés/non exécutés; témoin PE réel non exécuté. Pas de test GPU niveau 2, build natif, essai Windows manuel, téléchargement ou push.

Retour arrière limité aux helpers et à leur appel dans ce test. L’utilisateur a autorisé P51–P60 le 14 novembre 2026; P56 est la sixième tâche de cette tranche.
