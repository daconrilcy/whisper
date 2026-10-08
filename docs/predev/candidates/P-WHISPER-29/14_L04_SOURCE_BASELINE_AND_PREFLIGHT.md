# Baseline et disponibilité du préflight L04 — P-WHISPER-28

La baseline D0 L04 est définie par le parent D22 dans `52_L04_SOURCE_BASELINE_D20.md`, HEAD `ffd93125604be5dc6439587b7edad4f420f5cccc` après L00–L03. Elle comprend dix snapshots manifestés DESIGN et cinq créations futures absentes. Les quinze chemins sont exactement ceux de `01_LOTS.md` et de l’autorisation humaine. Les absences ne figurent pas dans `code_state.baseline/current`, sans hash fictif.

L’observation P24 au HEAD `c75ae195…` reste historique. D22 documente les comparaisons et égalités observées; l’identité du dépôt doit être recapturée au lancement et enregistrée séparément. Les preuves repository_evidence doivent décrire HEAD, branche, worktree/diff scoped, dix hashes et cinq absences, et séparer les modifications documentaires hors périmètre. La stratégie de hashes et de réconciliation figure dans `15_L04_CHECKPOINT_CONTRACT.md`.

Les probes, `check-state --lot L-WHISPER-04`, builds, tests produit/UI et livraison restent NOT RUN dans le présent candidat. Une divergence de contenu inexpliquée suspend le lot et préserve toutes les données.
