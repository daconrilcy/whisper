Avancée : 53 clés structurelles de tests retirées; baseline 240 → 187.
Blocage : 81 clés de tests restent dans le périmètre; ce lot est livré avec réserves.
Décision : ne pas fusionner ni pousser; décider d’un lot de continuation pour les sites restants.

# REDUC-CLIPPY-ADAPTERS-TESTS — suivi final

| Clé / site | État et motif | Lignes modifiées |
|---|---|---:|
| 53 clés traitées, 26 fichiers Rust | Extraction de helpers de tests; assertions, fixtures et identités conservées. Détail exact dans `followup-final/removed-keys-final.tsv`; hash/objet des commits dans `followup-final/commit-path-history.txt` du ZIP. | Hunk maximal : 129 lignes; chaque site reste sous 150. |
| 13 clés `tests/**` restantes | Écartées : chaque site dépasse 150 lignes; bornes inférieures par site dans `followup/remaining-site-metrics.tsv`. | >150/site |
| `src/worker_ipc/tests/runtime_support.rs::fmt` | Écartée après extraction limitée : le helper reste à 117 lignes; la suite exige au moins 17 lignes de plus. | 136 déjà modifiées; minimum final ≥153 |
| 67 clés `src/#[cfg(test)]` restantes | Non traitées avant le gel des validations; métriques finales par site dans `followup-final/remaining-site-metrics-final.tsv` du ZIP. | Pas de modification |
| 56 clés de production | Écartées, hors périmètre autorisé. | 0 |

Baseline : 240 → 187, retrait exact de 53 clés. Clippy gardé `whisper-adapters` : 194 → 141 diagnostics, ratchet PASS (141 sites / 187 clés de baseline), sans nouvel avertissement non structurel. `cargo fmt --check` gardé et `git diff --check` PASS. Les 776 identités de tests avant/après sont identiques (SHA-256 trié `4B6519229B32C4832C653289CCD60C376FCF62745C6978FA89827BCC88CFDBA3`).

N1 gardé final : PASS, 944 réussis / 0 échec / 129 ignorés. L’enveloppe de garde consignée en session rapporte `CARGO_TARGET_DIR_child=False`; les sorties Cargo brutes sont dans le ZIP. Modèle vérifié : `C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf`, SHA-256 `E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E`. SelfTest PASS, 72 positifs / 106 négatifs.

Commits : 27 locaux depuis `9711784e`; les 26 commits précédant ce suivi sont indexés avec hash/objet dans `followup-final/commit-path-history.txt`. Candidat code vérifié `3b2e0934530bef3e8b11c906fa8a1dda6fff7389`; le suivi est un commit documentaire séparé. Aucun merge/push. Preuves brutes : `evidence/REDUC-CLIPPY-ADAPTERS-TESTS-validation.zip`, 52 entrées, 399 630 octets, SHA-256 `555EB5BD75051A85EBE312404AD98CEAD8A2DDF9C281834E00EBD4D6FA2206F6`.

Suivi : aucun
