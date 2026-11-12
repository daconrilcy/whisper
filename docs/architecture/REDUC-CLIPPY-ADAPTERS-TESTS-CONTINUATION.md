Avancée : 19 des 21 clés candidates ont été retirées; baseline cumulative 240 → 168.
Blocage : deux sites dépassent 150 lignes modifiées et restent dans la baseline.
Décision : continuation livrée avec réserves; aucune fusion ni publication.

# REDUC-CLIPPY-ADAPTERS-TESTS — continuation des sites test

| Fichier / sites | Clés | Correction / état | Diff du site |
|---|---:|---|---:|
| `archive/tests/recovery.rs`; `worker_ipc/tests/reprocessing_and_transport.rs` | 2 | Helpers de fixtures et d’observation; assertions conservées. | 58 / 95 |
| `worker_ipc/tests/unified_capture.rs` | 7 | Extraction d’attente, validation d’artefacts, refus de route et assertions. | chaque site <150 |
| `worker_ipc/tests/unified_import.rs` | 5 | Helpers de publication, reprise et import en file d’attente. | chaque site ≤146 |
| `worker_ipc/tests/resume_and_admission.rs`; `worker_ipc/tests/native_execution.rs` | 4 | Helpers d’admission, d’attente et de couverture. | chaque site <150 |
| `worker_ipc/tests/microphone_and_finalization.rs` | 1 | Extraction d’observations de finalisation. | 24 |
| `unified_import.rs::import_application_uses_real_archive_path...`; `microphone_and_finalization.rs::microphone_gain_command...` | 2 | Écartées : le premier site reste à 128/100 après une extraction de 120 lignes; au moins 56 lignes supplémentaires porteraient le diff à ≥176. Le second a exigé 157 lignes. | >150 |

La décision utilisateur portait sur les 21 clés candidates avec une limite stricte de 150 lignes par site. **19/21** ont été traitées; les deux autres restent au baseline. Les 46 autres clés de test `src/#[cfg(test)]` de l’inventaire initial dépassaient déjà la borne estimée; la clé `runtime_support.rs::fmt` et les 13 clés d’intégration trop longues restent également exclues. Au total, **62 clés de test** et 56 clés de production demeurent dans le baseline après cette continuation.

Baseline depuis le candidat `8bc085a7` : 187 → 168 clés Clippy (19 retraits, 0 ajout); `whisper-adapters` : 137 → 118. Clippy gardé final : PASS, 122 diagnostics / 168 clés baseline. `cargo fmt --check` et `git diff --check` PASS. Liste des tests : 776, sortie brute byte-identique au candidat précédent; SHA-256 canonique des identités triées `4B6519229B32C4832C653289CCD60C376FCF62745C6978FA89827BCC88CFDBA3`.

N1 gardé : PASS, 944 réussis / 0 échec / 129 ignorés; `CARGO_TARGET_DIR_child=False` et cible partagée rapportés par la garde. SelfTest : PASS, 72 positifs / 106 négatifs. Modèle vérifié par SHA-256 `E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E`.

Candidat code : `7eac2b74dc99504d6294083a93212eef833b5650`, sept commits locaux depuis le candidat précédent, sans merge/push. Les changements et clés exacts sont indexés sous `continuation-final/` dans le ZIP. L’aller-retour de la clé `unified_archive_pages_cap_text_and_audio_entries_at_128_with_real_cursors` a été retiré de l’historique : elle reste absente sans commit baseline-only. Le tableau des 19 retraits référence les commits réécrits. Revue indépendante de cette correction en cours. Preuves : `evidence/REDUC-CLIPPY-ADAPTERS-TESTS-validation.zip`, 105 entrées, 620 872 octets, SHA-256 `36436CB9DE7CA10F9357561CF7456BD06E8594DCEDE9843D47A5A273E6382D58`.

Suivi : aucun
