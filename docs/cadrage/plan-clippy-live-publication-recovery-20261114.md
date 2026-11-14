# Plan P28 — Découper le scénario de reprise d’une publication live

Avancée : le Clippy standard après P27 passe le ratchet à 105 sites; `passage_publishes_durable_pcm_mp3_and_confirmed_text_as_one_generation` a 225 lignes et une complexité cognitive de 35.
Blocage : aucun; le test prouve la publication immuable, la découverte de l’interruption et la résolution explicite.
Décision : séparer ses phases dans un fichier de support dédié inclus par le test.

## Source et périmètre

Clippy frais six crates signale `passage_publishes_durable_pcm_mp3_and_confirmed_text_as_one_generation` dans `crates/whisper-adapters/tests/live_archive.rs:334` pour `too_many_lines` et `cognitive_complexity`. `ci/tests-baseline.txt` le classe `AUTONOME / AUTONOME`, avec racines temporaires et ports contrôlés, sans modèle, DLL, corpus ou worker réel.

Périmètre code exact : `crates/whisper-adapters/tests/live_archive.rs` et nouveau `crates/whisper-adapters/tests/live_archive/passage_publication_support.rs`. Inclure le support dans le même module avec `include!("live_archive/passage_publication_support.rs");`. Préserver les autres tests et imports nécessaires; aucun changement d’archive produit, protocole, dépendance, baseline, garde ou fixture.

## P28-T1 — Extraire la publication, l’interruption et la résolution

Réduire le test à une orchestration en phases nommées. Extraire dans le fichier de support privé des helpers cohérents pour (1) préparer/publier le premier passage PCM/MP3/segment et coverage, (2) publier les artefacts du second passage puis simuler le crash avant `CURRENT`, (3) démontrer la stabilité et l’absence de mutation de `ArchiveStore::scan` et `scan_live_recoveries`, (4) vérifier que la troisième génération est bloquée tant que la récupération reste en attente, (5) résoudre explicitement la publication et vérifier l’alias/historique final. Préserver toutes les valeurs, opérations disque, ranges, checksums, ordres, contrôles, assertions et messages. Préserver explicitement le `drop` des deux `PcmStaging` avant la suppression de la racine temporaire. Helpers <100 lignes et scores cognitifs au seuil 0 <25.

## Acceptation

- Les deux diagnostics ciblés disparaissent; Clippy standard six crates n’ajoute aucune clé et passe à 103 sites ou moins.
- Le test garde ses preuves : 8 000 échantillons et MP3 durables, segment nettoyé, couverture silencieuse, CURRENT bloqué sur manifeste 1 malgré les artefacts de génération 2, scans répétables et sans écriture, génération 3 refusée avant création, résolution explicite seulement, reconstruction de l’alias, deux générations complètes.
- Le test reste exécuté dans N1; aucun assert, ordre, message, taille audio ou précondition n’est affaibli.
- SelfTest, `fmt --check`, mesure seuil 0, Clippy standard ratcheté, N1 complet, sélections workers, fixtures PE et `git diff --check` passent.
- Seuls les deux chemins autorisés changent; les modifications antérieures sont préservées.

## Gates et arrêt

Worktree `C:/wt/cs`; `LotRoot` `target/parakeet-tests/clippy-corrections-suivantes-20261113`; cible Clippy existante `p21-fresh-20261114`, puis cible N1 fraîche `n1-p28-20261114`. Toute commande Cargo passe via `ci/guard-level1.ps1` avec les profils exacts. Arrêter et préserver les journaux au premier échec selon l’arbitrage de reprise. P28 est la huitième tâche de la tranche P21–P30; nouvel arbitrage après P30.

## Retour arrière

Si le découpage change un fichier/manifest immuable, l’ordre d’écriture, `CURRENT`, l’éligibilité de recovery, le blocage de la génération suivante ou la reconstruction des aliases, retirer uniquement le découpage fautif et signaler l’oracle affecté; ne pas affaiblir la caractérisation.
