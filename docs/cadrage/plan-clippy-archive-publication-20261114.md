# Plan P25 — Découper la caractérisation de publication interne

Avancée : Clippy standard après P24 passe le ratchet à 109 sites; un test autonome de publication interne fait 207 lignes.
Blocage : aucun; la caractérisation doit continuer à prouver l’absence d’export externe et l’intégrité des artefacts unifiés.
Décision : extraire dans le même fichier les phases cohérentes de préparation et d’assertion.

## Source et périmètre

Le Clippy frais des six crates signale `create_only_publication_returns_verified_internal_unified_artifacts_without_external_exports` dans `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs:30` (207/100). Le test utilise une archive et des artefacts synthétiques et N1 le couvre.

Périmètre code unique : `crates/whisper-adapters/src/archive/tests/archive_and_contracts.rs`. Extraire des helpers privés dans ce fichier sans nouveau fichier, réexport, dépendance ou API de production. Préserver les valeurs, écritures disque, ordre des opérations, contrôles d’intégrité et assertions; ne pas modifier les autres tests, l’implémentation d’archive, la baseline ni la garde.

## P25-T1 — Isoler préparation et preuves

Garder une orchestration lisible dans le test et déplacer la préparation des artefacts/publication ainsi que les groupes d’assertions dans de petits helpers privés nommés par responsabilité. Conserver le `PcmPartWriter`, les métadonnées `PendingRecord`/`CompleteRecord`, la réservation, les sentinelles TXT/SRT préexistantes, la vérification des artefacts internes, et la preuve qu’aucune exportation externe n’a eu lieu. Les helpers restent dans le même module et le fichier ne doit pas croître de plus de 30 lignes nettes. Le score cognitif au seuil 0 de chaque helper doit être inférieur à 25; le Clippy standard avec ratchet reste la gate finale.

## Acceptation

- Le diagnostic `too_many_lines` ciblé disparaît; le Clippy standard six crates passe sans nouvelle clé, au plus 108 sites.
- Le test conserve sa valeur de preuve : artefacts unifiés vérifiés, fichiers externes sentinelles inchangés, et index/publication cohérents; aucune assertion supprimée ou affaiblie.
- SelfTest, `fmt --check`, mesure seuil 0 gardée, Clippy standard ratcheté, N1 complet, sélections workers, fixtures PE et `git diff --check` passent.
- Seul le fichier cible est modifié pour le code; les changements préexistants du worktree sont préservés.

## Gates et arrêt

Worktree `C:/wt/cs`; `LotRoot` `target/parakeet-tests/clippy-corrections-suivantes-20261113`; cible Clippy existante `p21-fresh-20261114`, puis cible N1 neuve `n1-p25-20261114`. Toutes les commandes Cargo passent exclusivement par `ci/guard-level1.ps1`. La mesure seuil 0 et le Clippy standard utilisent le profil exact six crates; N1 utilise `-WithoutCargoTargetDir`, une cible fraîche et le chemin de modèle validé. Arrêter et préserver les logs à tout échec de porte. P25 est la cinquième tâche de la tranche P21–P30; demander un nouvel arbitrage après P30.

## Retour arrière

Si une extraction change l’ordre des écritures, la représentation des artefacts, les sentinelles ou le résultat d’une vérification, retirer uniquement l’extraction fautive et signaler la preuve affectée; ne pas assouplir l’oracle.
