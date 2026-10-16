Avancée : A4 passe sur le parcours applicatif final ; les clés historiques sont refusées et la recette B9 est documentée.
Blocage : B5 échoue sur un horodatage natif EN60 ; B6 échoue sur une frontière FR longue.
Décision : B8 reste inactive ; la qualification Test vaut pour import-single-pass court uniquement, la VAD longue reste non qualifiée.

# Fiche R5-2 — clé successeur Test

## Identité et périmètre

Branche `wip/l2b-candidate`, plan [PLAN-PARA-L2b-R5-2 v2](plan-parakeet-l2b-r5-2.md) revu CLEAN au commit `63ca0f0773d098697ab6c80b3e524c9968491fc7`. Worker CPU `e7672a520db87926b3da9b8e5b585a1e7e57ce53ff8b1d0fe95251046aa01f4d`, modèle `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`, runtime `d5a2cc8ef441dd5934972adae00f997241bea17a7bc5977a8c7f0122aaa958ff`.

DLL chargées et SHA-256 relevés : `ggml-base.dll` `398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a` ; `ggml-cpu.dll` `f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0` ; `ggml.dll` `2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794` ; `nemo_speech_asr_c.dll` `9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764` ; `nemo_speech_asr.dll` `923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019`.

Seul le profil **Test CPU** est concerné. Turbo ne change pas. Le worker court R4 `aa8df…` reste refusé. L'ancienne clé L2a n'est plus admise pour un nouvel import ; ses pending restent visibles et sont non reprenables avec une cause explicite. Aucun état `InRecipe` ne subsiste dans le code final. B8 reste inactive dans le produit : une importation VAD ordinaire est refusée avant pending ; seule une destination de recette canonique sous R5-2 peut employer cette clé.

## Préflight et exécution

La garde PowerShell a été testée **avant le premier Cargo** : cas OK admis, cas KO avec `CARGO_TARGET_DIR` extérieur rejeté avant le témoin Cargo. `PARAKEET_L2_RUNTIME_ROOT` est exactement `target/parakeet-tests/l2/l2b`, ancre de lecture ; les écritures Cargo, TEMP/TMP, audio généré, archive, pending, TXT/SRT et journaux sont confinées à `target/parakeet-tests/l2/l2b/r5-2`. Tous les Cargo ci-dessous sont passés par `guarded-cargo.ps1` dans cette dernière racine, en mode `--offline --locked` pour les tests. Aucun téléchargement ni nettoyage de l'incident R5-1 : 279 fichiers de build modifiés sous `target/debug`, 57 fichiers associés sous l'ancien TEMP et `.cargo/.global-cache` actualisé.

| Carte | Résultat observé | Statut |
| --- | --- | --- |
| A4 — import court | fr87, fr27, en15 : TXT/SRT produits par worker, adaptateur et archive réels ; archives Complete ; 0 `InvalidTimestamp`. fr87 contient `faire?` puis `Je` ; fr27 sans U+FFFD et dernier mot français valide. Le parcours a été rejoué après retrait de `InRecipe` et inscription de la fiche Qualified Test. | **PASS** |
| A5/B5 — EN60 | Test intégré ciblé, un scénario : référence passe unique arrêtée après 322,98 s : `InvalidTimestamp: word 2030 overlap cannot be normalized with positive durations`. VAD et import produit non atteints. | **FAIL** |
| A5/B6 — FR long synthétique | Test intégré ciblé, un scénario : audio de 1 915,8 s ; référence et VAD terminées, 22 fenêtres, 4 153 contre 4 137 tokens, 232 éditions globales informatives. Une frontière à 262 810 ms traverse un mot ; deux coupures précédentes conformes. Durée 280,35 s ; import produit et coupures suivantes non atteints. | **FAIL** |
| A5/B9 — bruit rose | `spkr0_x12.wav` de 60 min, 57 600 000 échantillons à −40, −30 et −20 dBFS : à chaque niveau, 0 silence admissible, 0 coupure, erreur `NoAdmissibleSilence`. Information de robustesse, sans décision d'activation B8. | **PASS informatif** |
| A6 — clés/pending | Nouvelle demande L2a et clé R4 refusées avant pending ; pending L2a conservé et visible, reprise refusée explicitement. | **PASS** |

Sorties A4 sous `target/parakeet-tests/l2/l2b/r5-2/archive/a4` : `fr87/transcriptions/000000000000000000000000000001a4/1/transcript.{txt,srt}`, `fr27/transcriptions/000000000000000000000000000001a5/1/transcript.{txt,srt}`, `en15/transcriptions/000000000000000000000000000001a6/1/transcript.{txt,srt}`. Les fichiers de recette restent hors Git.

## Vérifications exécutées

| Commande via la garde | Résultat |
| --- | --- |
| `fmt --all -- --check` | PASS |
| `test --offline --locked -p whisper-adapters --test parakeet_archive r5_2_a4_product_route_publishes_french_and_english_txt_srt -- --ignored --exact --nocapture` | PASS, 405,80 s sur le code final |
| `test --offline --locked -p whisper-adapters --test parakeet_archive pending_scan_and_resume_require_the_exact_qualified_test_key -- --exact --nocapture` | PASS |
| `test --offline --locked -p whisper-adapters --test parakeet_archive retired_l2a_pending_remains_visible_but_cannot_be_resumed -- --exact --nocapture` | PASS |
| `test --offline --locked -p whisper-adapters --test parakeet_archive r5_2_b9_reports_noise_floor_silences_and_cut_plans -- --ignored --exact --nocapture` | PASS |
| `test --offline --locked -p whisper-adapters --test parakeet_archive r5_2_long_tests::en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy -- --ignored --exact --nocapture` | FAIL B5, 322,98 s ; un test ciblé |
| `test --offline --locked -p whisper-adapters --test parakeet_archive r5_2_long_tests::fr_synthetic_long_reference_and_vad_use_same_candidate -- --ignored --exact --nocapture` | FAIL B6, 280,35 s ; un test ciblé |
| `git diff --check` | PASS |

La cible historique `tests/parakeet_long.rs` n'a pas été lancée : sa destination aurait été sous l'ancre L2b. Les scénarios longs sont intégrés comme tests ignorés ciblables dans `parakeet_archive.rs`, avec sorties sous R5-2. Leurs deux échecs reproduisent les premiers essais isolés. Ces résultats ne prouvent pas le parcours applicatif long. Le français long provient d'un montage synthétique ; la qualité absolue et les autres langues ne sont pas qualifiées par cette fiche.

**Verdict de revue du candidat : CLEAN**, après deux revues et une correction ; R1 (VAD produit refusée avant pending) et R2 (tests B5/B6 intégrés et ciblables) sont fermés. Ce verdict porte sur la conformité du candidat aux décisions R5-2 ; B5/B6 demeurent **FAIL** et demandent un lot distinct si leur correction touche le worker ou les règles VAD hors des chemins R5-2. Aucune activation B8 ne découle de cette fiche.
