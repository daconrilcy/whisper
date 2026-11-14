Avancée : Clippy standard après P58 relève `published_unified_files` à 139 lignes.
Blocage : ce parcours vérifie deux formes de publication, dont les pages audio internes et leur intégrité.
Décision : extraire les phases privées dans le même fichier en conservant les mêmes contrôles et erreurs.

# Plan P59 — Décomposer la vérification des fichiers de publication unifiée

## Constat et preuve

Clippy standard après P58 relève `clippy::too_many_lines (139/100)` pour `crates/whisper-adapters/src/archive/identity.rs::ArchiveStore::published_unified_files` (ligne 358) dans `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261114T224242159-607ff3c1cc3a4aa59c8c1faefb38b32f/cargo-command.stdout.log`. Le fichier source est propre avant P59 et compte 877 lignes.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-adapters/src/archive/identity.rs`.
- Extraire uniquement des phases privées locales de `published_unified_files`; conserver sa signature publique et son contrat. Aucun autre fichier, test, stockage, export, chemin, protocole ou comportement.
- Le delta net est limité à +30 lignes. La fonction et helpers respectent les plafonds de 100 lignes, 25 de complexité cognitive, 7 arguments et 250 de complexité de type.
- Préserver l’ordre et le contenu des validations `CURRENT`, manifeste/identité, intention, publication interne/externe, vérification des artefacts et assemblage des chemins. Pour l’interne, préserver canonicalisation, vérification du WAV/index, total d’échantillons non nul et concordant, lecture paginée, continuité des numéros/offsets, bornes des parties, identité du premier WAV, pagination complète et empreinte d’index inchangée. Garder tous messages/erreurs, retours et propagations à l’identique.

## Critères d’acceptation P59-T1

Le test d’intégration existant `create_only_publication_returns_verified_internal_unified_artifacts_without_external_exports` dans `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs` réussit dans la N1 et conserve ses rejets de manifeste, intention, artefact et index corrompus ou manquants. Les branches externes et les autres cas de publication restent inchangés.

La mesure Clippy seuil 0 ne révèle aucun dépassement des plafonds sur la fonction et les helpers. Clippy standard passe avec le ratchet réduit à 71 sites ou moins. Revue indépendante CLEAN. fmt-check, SelfTest de garde, N1 fraîche, PE SelfTest, dix harnais exacts, inventaires et suites CPU, Parakeet CPU et common passent via `ci/guard-level1.ps1`.

## Vérifications et limites

Utiliser `ci/guard-level1.ps1`, LotRoot `target/parakeet-tests/clippy-corrections-suivantes-20261113`, commandes Cargo offline et locked et cibles fraîches dédiées. La N1 reçoit `-PersonalParakeetModelPath C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf`. Relever l’espace libre avant compilation. Ne pas modifier la baseline structurelle.

Tests ignorés : NON EXÉCUTÉS ; témoin PE réel : NON EXÉCUTÉ. Pas de GPU niveau 2, build natif, essai dans la fenêtre Windows, téléchargement, nettoyage ou push.

Retour arrière limité à l’extraction P59. La tranche P51–P60 est autorisée par l’utilisateur le 14 novembre 2026 ; P59 est la neuvième tâche.
