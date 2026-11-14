Avancée : le Clippy gardé après P56 signale le test de gain micro avec worker différé à 171 lignes.
Blocage : un scénario relie la révision du gain à l’acquisition, au rejet d’une autre source, à l’arrêt et aux échantillons WAV.
Décision : extraire ses phases en helpers privés locaux en conservant chaque événement et oracle.

# Plan P57 — Décomposer le scénario de gain avec worker différé

## Constat et preuve

Le Clippy standard frais après P56 signale `clippy::too_many_lines (171/100)` pour `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs::microphone_gain_command_reaches_acquisition_and_finalized_wav_while_worker_is_delayed` (ligne 490 dans le candidat P56). La mesure est dans `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261114T220234610-5fa018d33a6a4528863f85f52a609515/cargo-command.stdout.log`. Le seuil cognitif 0 mesure le test à 19; le ratchet courant est à 74 sites.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs`.
- Extraire uniquement les phases de ce test en helpers privés locaux; déplacer son `SimulatedPort` local vers un support privé de ce même fichier si nécessaire pour le partage avec les helpers. Ne pas éditer le fichier agrégateur `tests.rs`.
- Le corps de test et chaque helper restent sous 100 lignes et sous 25 de complexité cognitive; aucun helper ne dépasse 7 arguments. Le delta net du fichier reste limité à +30 lignes maximum.
- Préserver le nom du test, le worker différé, la source, identités et génération, les blocs audio, les commandes et événements, révisions, valeurs, ordre, assertions WAV, messages et nettoyage. Aucun changement de production, autre test ou comportement.

## Critères d’acceptation P57-T1

Le worker reste non prêt pendant le scénario. La commande de gain 6 dB ne produit pas d’accusé immédiat; le backlog antérieur confirme seulement le gain 0/révision 0/frame 0 puis la capture active. Le bloc suivant produit le gain 6/révision 1/frame 16 000. La source `test:other` est rejetée et le propriétaire de capture reste en place. Stop produit `LiveFinalizing`; le WAV final contient 64 000 octets de données, 32 000 échantillons, les régions à 8 192 et les échantillons de la seconde région dans l’intervalle existant; le fichier et le répertoire temporaires sont supprimés. Toutes les assertions et leur ordre sont conservés.

Le test et helpers satisfont les plafonds de taille/complexité, le fichier augmente d’au plus 30 lignes, Clippy au seuil 0 révèle aucune nouvelle alerte structurelle, et Clippy standard passe avec ratchet à 73 sites ou moins. Revue indépendante CLEAN. fmt-check, N1 fraîche, PE SelfTest, harnais exacts, inventaires et suites CPU, Parakeet CPU et common réussissent via la garde.

## Vérifications et limites

Utiliser `ci/guard-level1.ps1` et le LotRoot `target/parakeet-tests/clippy-corrections-suivantes-20261113`: fmt-check, mesure seuil cognitif 0, Clippy standard six membres, N1 sur une cible fraîche P57, SelfTest PE, compilations `--no-run`, contrôles PE des chemins exacts, inventaires et suites des trois workers. Le cache Cargo partagé étant absent, utiliser des cibles fraîches gardées sous le lot. Commandes Cargo offline et locked.

Les tests nécessitant modèle ou runtime natif restent ignorés/non exécutés; témoin PE réel non exécuté. Ces tests synthétiques ne qualifient pas un microphone matériel. Pas de test GPU niveau 2, build natif, essai dans la fenêtre Windows, téléchargement ou push.

Retour arrière limité aux helpers et au support `SimulatedPort` de P57. L’utilisateur a autorisé P51–P60 le 14 novembre 2026; P57 est la septième tâche de cette tranche.
