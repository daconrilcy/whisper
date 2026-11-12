Avancée : plan borné à la reprise d’import et relié au cadrage DESIGN CLEAN.
Blocage : le préflight N1/Clippy du futur lot ne peut pas être déclaré satisfaisable avec la politique de garde actuelle.
Décision : ne commencer ni code ni Cargo tant qu’une voie de vérification N1 conforme et une autorisation d’implémentation précise ne sont pas établies.

# Plan — refactorisation de `resume_interrupted`

**Référence design :** [cadrage-refacto-reprise-import.md](cadrage-refacto-reprise-import.md), DESIGN CLEAN au commit `8373e8faef3f685f4adc1c66a0a251756a6e284f`.
**Base de code observée :** `main@6193fbdf`; sources Rust identiques au worktree Clippy `5f59d920` utilisé pour le relevé frais. Le lot d’implémentation devra refaire son préflight depuis son HEAD réel.
**Constat initial :** Clippy frais gardé le 12 novembre 2026, six crates N1, `resume_interrupted` à 272 lignes contre le seuil de 100 ; cliquet structural PASS, 153 sites contre 168. Les builds/workers Turbo restent non mesurés.
**État :** candidat à la revue indépendante du plan. Ce document ne donne pas l’autorisation de coder ou de lancer Cargo.

## Périmètre et chemins

- **Production modifiable prévue :** `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs` seulement.
- **Tests existants à réutiliser, en lecture :**
  - `crates/whisper-adapters/tests/durability.rs` : `resume_validation_rejects_later_fifo_row_without_advancing_any_generation`.
  - `crates/whisper-adapters/src/worker_ipc/tests/queue_inspection_characterization.rs` : les trois tests `interrupted_*`.
  - `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs` : `injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume`.
  - `crates/whisper-adapters/src/archive/tests/recovery.rs` : `r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision` et `resumed_prefix_copy_retries_after_pending_creation_without_duplication` (contrats d’archive seulement).
  - `crates/whisper-adapters/src/worker_ipc/tests/unified_poll/equivalence.rs` : `unified_poll_resumed_prefix_emits_only_new_segments` (checkpoint injecté).
  - `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs` : annulation du dialogue doublon et refus de réponse périmée (contrats génériques seulement).
- **Tests d’intégration ignorés sans `native-tests` :** reconfirmation R7, succès unifié et scénarios de restauration/transfert dans `unified_import.rs`, ainsi que les cas de reprise catalogue dans `resume_and_admission.rs`. Ils ne sont pas des preuves N1 exécutées.
- Aucun autre fichier de production, test nouveau, manifeste, verrou, baseline ou garde n’est inclus. Tout besoin supplémentaire suspend le lot et exige une décision de portée distincte.

## Tâches

| Tâche | Travail et chemins | Vérification / critère de sortie |
|---|---|---|
| **T0 — Préflight et autorisation** | Vérifier HEAD, état Git, design/plan acceptés, crate `whisper-adapters` et consignes applicables, worktree dédié, espace libre ≥30 Gio, cache hors ligne, modèle et témoins PE requis. Confirmer une racine N1 approuvée et une politique de garde qui couvre la matrice de clôture complète. Obtenir un accord écrit précis d’implémentation avant le premier changement Rust. | SelfTest de garde et précontrôles requis PASS avant Cargo. **Arrêt actuel :** la garde N1 enregistrée accepte un jeu précis de six membres, tandis que les sélections workers individuelles exigées par la clôture ne sont pas permises par cette politique. Le plan ne permet pas de contourner la liste ni de modifier la garde. Tant que la voie n’est pas résolue séparément, statut `NON EXÉCUTÉ`, aucun code/Cargo. |
| **T1 — Baseline et contrats témoins** | Sur `recovery.rs`, cartographier validation FIFO/modèle/capacité, reconfirmation, transfert de source, écritures archive/file, inscription de route, mise à jour mémoire et événements. Rejouer les témoins existants qui sont admissibles, sans élargir les tests ignorés. | Baseline Clippy JSON gardée et résultats ciblés capturés avant modification ; chaque témoin attribué à la portion qu’il couvre. Vérifier aussi dans le cœur `Queued → Preparing` et la réservation de trois événements pour `ResumeInterrupted`. |
| **T2 — Extraire les contrôles sans effet** | Dans `recovery.rs`, extraire si utile la validation archive/file, la comparaison par stratégie et l’inspection catalogue, puis l’identité durable/modèle exécutable et le calcul de capacité/route. Laisser la reconfirmation à sa place et ne pas déplacer les effets de liaison ou d’archive. | Mêmes refus précoces `Failed/Return`, identifiants, causes, snapshots et chemins de modèle. Rejouer les témoins AC-01 et vérifier par revue les frontières sans changement de contrat. |
| **T3 — Extraire la séquence à effets** | Toujours dans `recovery.rs`, découper avec des helpers privés au plus près des blocs contigus. Préserver strictement : transfert de liaison → archive → file durable `Running` → route facultative → état mémoire/checkpoint → `Queue → ResumeCheckpoint → Prepared`. Garder les voies `Continue(Err)` archive/file et `Failed/Return` après refus route distinctes ; restauration uniquement après transfert `Ok(true)`, y compris une liaison réinsérée par une tentative antérieure. | AC-02 à AC-04 satisfaits ; aucun effet ou compensation déplacé, aucun changement d’ordre, aucun trio d’événements ajouté à la voie de refus route. Si l’équivalence d’une extraction n’est pas démontrable, conserver cette portion et signaler le résiduel au lieu de forcer la baisse Clippy. |
| **T4 — Vérifications finales** | Après succès des prérequis, rejouer format, témoins ciblés, matrice N1 autorisée et Clippy gardé sur exactement le même périmètre que la baseline. Les commandes restent futures et doivent être matérialisées avec les racines approuvées par le futur lot. | Aucun échec ; aucun nouveau site structurel ; tous les critères AC-01 à AC-04 couverts. Cible `too_many_lines ≤100` si compatible avec ces invariants ; sinon diagnostic restant mesuré et justifié. Tests intégrés ignorés, GPU niveau 2 et validation Windows rapportés `NON EXÉCUTÉS`. |
| **T5 — Revue et clôture** | Faire relire le diff Rust par le `rust_review_coordinator` selon `AGENTS.md`, avec au plus deux cycles revue/correction. Conserver l’état Git et les preuves du lot ; ne pas pousser, fusionner ni nettoyer dans le cadre de ce plan. | Verdict CLEAN ou réserves explicites ; preuves archivées et contrôlées selon les règles du lot. N1, Clippy, revue et validation produit restent des résultats distincts. |

## Commandes candidates — futures, jamais exécutées par ce cadrage

Chaque appel passera par `ci/guard-level1.ps1` dans le worktree du futur lot, PowerShell 7, `--offline --locked` injectés par la garde quand permis. Variables, racines, cible, cache, modèle et espace sont préflightés par le lot. Les noms `$LotN1`, `$LotClippy`, `$LotWorker` et `$ModeleVerifie` ci-dessous sont des paramètres à approuver et vérifier, pas des chemins déjà autorisés.

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -SelfTest -LotRoot $LotN1
pwsh -NoProfile -File ci/check-worker-imports.ps1 -SelfTest
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $LotN1 -CargoArgsJson '["fmt","--check"]'
```

Témoins par filtre, si la garde accepte une racine N1 approuvée sans sélection `-p` :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $LotN1 -CargoArgsJson '["test","resume_validation_rejects_later_fifo_row_without_advancing_any_generation"]'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $LotN1 -CargoArgsJson '["test","interrupted_"]'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $LotN1 -CargoArgsJson '["test","injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume"]'
```

La suite de référence N1 sans variable `CARGO_TARGET_DIR` reste requise. Elle dépend de la politique de modèle Parakeet et de la résolution des sélections worker refusées :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $LotN1 -WithoutCargoTargetDir -PersonalParakeetModelPath $ModeleVerifie -CargoArgsJson '["test","--no-fail-fast"]'
```

Clippy frais sur les six membres autorisés, toutes cibles, avec les quatre lints structurels :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $LotClippy -FreshTarget clippy-reprise -CargoArgsJson '["clippy","-p","whisper-core","-p","whisper-adapters","-p","whisper-desktop","-p","whisper-bootstrap","-p","whisper-worker-common","-p","whisper-worker-parakeet-cpu","--all-targets","--message-format=json","--","-W","clippy::too_many_lines","-W","clippy::cognitive_complexity","-W","clippy::too_many_arguments","-W","clippy::type_complexity"]'
```

Les commandes par sélection worker et leurs contrôles PE ne sont admissibles qu’après approbation de la politique exacte de membre/racine et de la clôture N1. Un `SelfTest` réussi ne lève pas cette restriction. Si les prérequis de garde demeurent non résolus, ces commandes restent `NON EXÉCUTÉES` et T0 arrête le lot.

## Risques, reprises et limites

- **R1 — ordre/compensation :** restauration de liaison ou erreur traitée dans le mauvais dispatcher. Réduire par T3, témoins de source binding disponibles et revue indépendante.
- **R2 — file/checkpoint :** divergence entre file durable `Running`, états visibles, checkpoint confirmé et triplet d’événements. Réduire par contrôles T1/T3 et tests partiels correspondants.
- **R3 — couverture incomplète :** reconfirmation, succès unifié et rollback réel ignorés sans `native-tests`. Ne pas remplacer par un faux modèle non approuvé ni annoncer les AC correspondants comme intégralement validés.
- **R4 — garde :** matrice N1 bloquée par les politiques actuelles de sélection/racine. Aucun changement de garde n’est autorisé par ce plan ; escalader avant implémentation.
- **Retour arrière :** si le lot échoue, examiner son diff et retirer seulement les changements propres à `recovery.rs` après vérification des dépendances et effets. Ne pas faire de reset global, ne pas restaurer les changements d’autrui.

**Hors périmètre :** modifications des autres alertes Clippy, réduction générale du cliquet, modification de garde/baseline, nouvelles fonctionnalités de reprise, builds natifs, GPU niveau 2, campagne de performance, recette Windows, merge, push et nettoyage non prévu par le lot.

**Autorisation :** seule la préparation de ce plan est autorisée actuellement. Toute implémentation Rust exige une demande et un accord écrit précis ultérieurs, les prérequis T0 satisfaits et la délégation à `rust_implementer`. Aucun commit de code ou push n’est couvert.
