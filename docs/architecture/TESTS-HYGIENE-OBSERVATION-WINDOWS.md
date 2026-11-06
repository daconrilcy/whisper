Avancée : N1 supplémentaire PASS ; absence de fenêtre système attestée par l’opérateur.
Blocage : essai depuis main après fusion NON EXÉCUTÉ ; réserve mineure TH-R4 inchangée.
Décision : observation visuelle validée pour cette relance ; fusion toujours laissée à l’humain.

# TESTS-HYGIENE — complément du 6 novembre 2026

À la demande de l’utilisateur présent devant la machine, une suite complète des membres par défaut a été relancée sur le commit `0c5e1888`, branche `tests-hygiene`, sans modification du code.

- Exécution : **07:24:12–07:24:24**, heure de Paris ; **11,69 s** au total.
- Commande : `cargo test --no-fail-fast` par `ci/guard-level1.ps1 -Mode Run -WithoutCargoTargetDir -LotRoot target/tests-hygiene`.
- Résultat : **715 passés, 0 échec, 128 ignorés**, soit **843 tests dans 59 cibles**.
- Environnement Cargo : `CARGO_TARGET_DIR` absent ; cible effective `C:/wb/tests-hygiene/cargo-target`, fixée par l’argument `--config` de la garde.
- Mémoire validée après exécution : **17,5 %** ; aucune nouvelle WHEA relevée.

Pour l’intervalle d’observation annoncé **07:24:11–07:24:24**, l’utilisateur a répondu : « Aucune fenêtre, intervalle entièrement observé ». L’attestation porte sur cette suite supplémentaire ; les trois N1 et cinquante répétitions antérieurs restent conservés avec leur périmètre de preuve initial.

Le [bilan initial](TESTS-HYGIENE-BILAN.md) demeure figé. La réserve visuelle est désormais couverte par cette nouvelle observation. L’essai sans garde depuis `C:/dev/whisper`, prévu humain après fusion, reste à faire ; TH-R4 (`fmt --check` du mode sans variable) reste MINEUR ouvert et la cause historique exacte de C demeure non prouvée. Aucune fusion, aucun push et aucun nettoyage réalisés.

Preuves : [nouvelle archive](raw/TESTS-HYGIENE-OBSERVATION-20261106T072411.zip), six fichiers et un manifeste ; nombres, tailles et SHA-256 vérifiés. Les archives antérieures sont inchangées.

SHA-256 : `D1FAD5E4AEF79FD8B32A34655FD3B8DC7E06BDEE939F84AA89FA9B7E2C59B5F6`.
