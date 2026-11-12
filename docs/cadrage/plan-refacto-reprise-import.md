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
| **T0 — Préflight et autorisation** | Vérifier HEAD, état Git, design/plan acceptés, crate `whisper-adapters` et consignes applicables, worktree dédié, espace libre ≥30 Gio, cache hors ligne, modèle et témoins PE requis. Confirmer une racine de lot approuvée et une politique de garde couvrant la matrice de clôture complète. Sélectionner une cible Cargo unique : cible partagée canonique existante ou cible fraîche unique créée sous la racine approuvée puis réutilisée. Obtenir un accord écrit précis d’implémentation avant le premier changement Rust. | SelfTest de garde et précontrôles requis PASS avant Cargo. **Arrêt actuel :** la politique enregistrée pour le Clippy précédent n’autorise que son chemin exact et son jeu de six membres ; elle n’autorise pas tacitement ce nouveau lot. Les sélections workers individuelles exigées par la clôture sont également hors de cette portée. Le plan ne permet ni de réutiliser l’ancienne racine, ni de contourner la liste, ni de modifier la garde. Tant que la voie n’est pas résolue séparément, statut `NON EXÉCUTÉ`, aucun code/Cargo. |
| **T1 — Baseline et contrats témoins** | Sur `recovery.rs`, cartographier validation FIFO/modèle/capacité, reconfirmation, transfert de source, écritures archive/file, inscription de route, mise à jour mémoire et événements. Rejouer les témoins existants qui sont admissibles, sans élargir les tests ignorés. | Baseline Clippy JSON gardée et résultats ciblés capturés avant modification ; chaque témoin attribué à la portion qu’il couvre. Vérifier aussi dans le cœur `Queued → Preparing` et la réservation de trois événements pour `ResumeInterrupted`. |
| **T2 — Extraire les contrôles et inspections** | Dans `recovery.rs`, extraire si utile l’ouverture/validation archive-file, la comparaison par stratégie et l’inspection catalogue, puis l’identité durable/modèle exécutable et le calcul de capacité/route. Garder l’ouverture/réparation de file au même point d’ordre ; `QueueStore::open` peut créer le dossier ou réparer l’état. Laisser la reconfirmation à sa place et ne pas déplacer les autres effets de liaison ou d’archive. | Mêmes refus précoces `Failed/Return`, identifiants, causes, snapshots et chemins de modèle. Rejouer les témoins AC-01 et vérifier par revue l’ordre d’ouverture/réparation et les frontières. |
| **T3 — Extraire la séquence à effets** | Toujours dans `recovery.rs`, découper avec des helpers privés au plus près des blocs contigus. Préserver strictement : transfert de liaison → archive → file durable `Running` → route facultative → état mémoire/checkpoint → `Queue → ResumeCheckpoint → Prepared`. Garder les voies `Continue(Err)` archive/file et `Failed/Return` après refus route distinctes ; restauration uniquement après transfert `Ok(true)`, y compris une liaison réinsérée par une tentative antérieure. | Revue statique de l’ordre et des trois branches d’erreur ; les tests cités ci-dessous ne prouvent que leurs stimuli locaux. Aucun effet ou compensation déplacé, aucun changement d’ordre, aucun trio d’événements ajouté à la voie de refus route. Si l’équivalence d’une extraction n’est pas démontrable, conserver cette portion et signaler le résiduel au lieu de forcer la baisse Clippy. |
| **T4 — Vérifications finales** | Après succès des prérequis, rejouer format, témoins ciblés, matrice N1 autorisée et Clippy gardé sur exactement le même périmètre que la baseline. Tous les appels Cargo réutilisent la cible unique choisie à T0, y compris le N1 sans variable `CARGO_TARGET_DIR` via le mécanisme gardé. | Aucun échec ; aucun nouveau site structurel ; preuves partielles attribuées selon la matrice ci-dessous. Cible `too_many_lines ≤100` si compatible avec les invariants ; sinon diagnostic restant mesuré et justifié. Tests intégrés ignorés, GPU niveau 2 et validation Windows rapportés `NON EXÉCUTÉS`. |
| **T5 — Revue et clôture** | Faire relire le diff Rust par le `rust_review_coordinator` selon `AGENTS.md`, avec au plus deux cycles revue/correction. Archiver les preuves hors `target/` en préservant leur arborescence ; vérifier nombre et SHA-256. Après cela, identifier chaque cible/cargo-home isolé du lot et le supprimer seulement après vérification de chemin absolu dans le worktree, appartenance au lot, absence de lien/reparse point et absence de processus/lot actif. Conserver les cibles et caches partagés/cargo-home persistant, preuves et ressources produit. Si l’admissibilité manque, conserver et signaler les chemins. | Verdict CLEAN ou réserves explicites ; preuves archivées et contrôlées selon les règles du lot ; nettoyage exact autorisé par les règles permanentes exécuté ou documenté comme bloqué. N1, Clippy, revue et validation produit restent des résultats distincts. |

## Matrice de traçabilité preuves

| Critère | Tâches | Stimulus / témoin futur et résultat attendu | Limite de preuve |
|---|---|---|---|
| **AC-01 Validation et identité** | T1, T2, T4 | `resume_validation_rejects_later_fifo_row_without_advancing_any_generation` : refus de la ligne FIFO ultérieure sans avancer de génération. `interrupted_recovery_inspects_only_its_required_model`, `interrupted_turbo_recovery_accepts_historical_alias_and_preserves_durable_identity`, `interrupted_turbo_recovery_rejects_different_durable_model` : comportements modèle historiques/ciblés. `injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume` : les routes d’effets testées refusent une attestation catalogue invalide, checkpoint inclus. | Les tests vérifient ces refus/configurations précis, pas chaque branche de `resume_interrupted`; la capacité est complétée par comparaison statique aux validations du cœur et par revue. |
| **AC-02 Reconfirmation** | T1, T2, T4 | `cancelling_duplicate_import_releases_only_the_new_source_snapshot` et `stale_duplicate_dialog_response_cannot_submit_a_replacement` (cœur) : annulation/réponse périmée n’admet pas le remplacement. `r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision` : récupération d’archive/collision seulement. | Ces témoins n’exécutent pas le parcours intégré R7 de reprise ; celui-ci est ignoré sans `native-tests` et reste NON EXÉCUTÉ. Le maintien du prompt/verrou avant transfert est aussi vérifié statiquement. |
| **AC-03 Ordre durable, checkpoint et événements** | T1, T3, T4 | `resumed_prefix_copy_retries_after_pending_creation_without_duplication` : archive seule et absence de duplication après retry. `unified_poll_resumed_prefix_emits_only_new_segments` : poll avec checkpoint injecté. Vérifier statiquement archive → file durable `Running` → route éventuelle → mémoire → trio réservé `Queue → ResumeCheckpoint → Prepared`. | Les deux premiers témoins ne vérifient pas l’intégration archive/file/route. `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` et `import_application_resumes_a_verified_interrupted_unified_import` sont ignorés sans `native-tests`; succès intégré NON EXÉCUTÉ. |
| **AC-04 Erreurs et compensation** | T1, T3, T4 | Revue statique de chaque retour dans `recovery.rs` et de l’aiguillage dans `runtime/effects.rs`; distinguer refus précoces `Failed/Return`, erreur archive/file `Continue(Err)` avec restauration seulement après `Ok(true)`, et échec route après écritures `Failed/Return` sans compensation/trio. Les tests `failed_unified_resume_preserves_interrupted_source_binding_and_queue_identity` et `failed_unified_resume_after_binding_transfer_restores_registry_and_queue_identity` sont des témoins d’intégration partiels. | Ces deux tests sont ignorés sans `native-tests`; rollback intégré NON EXÉCUTÉ. La revue statique prouve l’ordre du candidat, pas le comportement d’exécution. |
| **AC-05 Découpage** | T2, T3, T4 | Clippy JSON après refactor : cible ≤100 lignes ; si une extraction expose un risque sur AC-01–04, garder le résiduel et justifier sa taille/signature. | Le nombre de lignes seul ne démontre pas une meilleure architecture ni l’équivalence produit. |
| **AC-06 Validation et limites** | T0, T4, T5 | Résultats de garde, tests ciblés et matrice N1 attribués par code et compte réussis/échoués/ignorés ; archive de preuves vérifiée par nombre/SHA ; verdict reviewer séparé. | Toute commande non admissible ou tous scénarios ignorés restent explicitement NON EXÉCUTÉS ; aucune réussite produit n’en est déduite. |

## Commandes candidates — futures, jamais exécutées par ce cadrage

Chaque appel passera par `ci/guard-level1.ps1` dans le worktree du futur lot, PowerShell 7, `--offline --locked` injectés par la garde quand permis. Variables, racines, cible unique, cache, modèle et espace sont préflightés par le lot. Les noms `$Lot`, `$MembreWorker` et `$ModeleVerifie` ci-dessous sont des paramètres à approuver et vérifier, pas des chemins déjà autorisés. L’ancienne racine Clippy n’est pas implicitement approuvée pour ce lot.

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -SelfTest -LotRoot $Lot
pwsh -NoProfile -File ci/check-worker-imports.ps1 -SelfTest
```

Si aucune cible canonique partagée n’est retenue, la première commande de compilation est le Clippy de référence ci-dessous avec `-FreshTarget refacto-reprise`. Après elle, tous les appels Cargo, y compris fmt et les filtres de test, utilisent `-ExistingTarget refacto-reprise`. Si la cible partagée est retenue, tous utilisent cette cible par défaut sans les commutateurs `Fresh/ExistingTarget`.

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $Lot -ExistingTarget refacto-reprise -CargoArgsJson '["fmt","--check"]'
```

Témoins par filtre, si la garde accepte la racine approuvée sans sélection `-p` :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $Lot -ExistingTarget refacto-reprise -CargoArgsJson '["test","resume_validation_rejects_later_fifo_row_without_advancing_any_generation"]'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $Lot -ExistingTarget refacto-reprise -CargoArgsJson '["test","interrupted_"]'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $Lot -ExistingTarget refacto-reprise -CargoArgsJson '["test","injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume"]'
```

Tous les appels Cargo du lot partagent la même cible. Si le préflight valide la cible canonique existante `target/parakeet-tests/cargo-shared/`, l’utiliser pour Clippy, tests et format. Sinon, créer une seule cible fraîche sous l’unique racine approuvée avec `-FreshTarget <nom>` à la première compilation ; la réutiliser ensuite par `-ExistingTarget <nom>`, y compris avec `-WithoutCargoTargetDir` pour N1. Ne pas créer de cible séparée pour Clippy ou N1.

La suite de référence N1 sans variable `CARGO_TARGET_DIR` reste requise. Elle dépend de la politique de modèle Parakeet et de la résolution des sélections worker refusées :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $Lot -WithoutCargoTargetDir -ExistingTarget refacto-reprise -PersonalParakeetModelPath $ModeleVerifie -CargoArgsJson '["test","--no-fail-fast"]'
```

Clippy de référence avant changement : initialiser une seule cible fraîche `refacto-reprise` sous la racine approuvée et conserver le JSON :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $Lot -FreshTarget refacto-reprise -CargoArgsJson '["clippy","-p","whisper-core","-p","whisper-adapters","-p","whisper-desktop","-p","whisper-bootstrap","-p","whisper-worker-common","-p","whisper-worker-parakeet-cpu","--all-targets","--message-format=json","--","-W","clippy::too_many_lines","-W","clippy::cognitive_complexity","-W","clippy::too_many_arguments","-W","clippy::type_complexity"]'
```

Clippy final sur les six membres autorisés, toutes cibles, avec les quatre lints structurels ; réutiliser la cible précédente :

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot $Lot -ExistingTarget refacto-reprise -CargoArgsJson '["clippy","-p","whisper-core","-p","whisper-adapters","-p","whisper-desktop","-p","whisper-bootstrap","-p","whisper-worker-common","-p","whisper-worker-parakeet-cpu","--all-targets","--message-format=json","--","-W","clippy::too_many_lines","-W","clippy::cognitive_complexity","-W","clippy::too_many_arguments","-W","clippy::type_complexity"]'
```

Les commandes par sélection worker et leurs contrôles PE ne sont admissibles qu’après approbation de la politique exacte de membre/racine et de la clôture N1. Un `SelfTest` réussi ne lève pas cette restriction. Si les prérequis de garde demeurent non résolus, ces commandes restent `NON EXÉCUTÉES` et T0 arrête le lot.

## Risques, reprises et limites

- **R1 — ordre/compensation :** restauration de liaison ou erreur traitée dans le mauvais dispatcher. Réduire par T3, témoins de source binding disponibles et revue indépendante.
- **R2 — file/checkpoint :** divergence entre file durable `Running`, états visibles, checkpoint confirmé et triplet d’événements. Réduire par contrôles T1/T3 et tests partiels correspondants.
- **R3 — couverture incomplète :** reconfirmation, succès unifié et rollback réel ignorés sans `native-tests`. Ne pas remplacer par un faux modèle non approuvé ni annoncer les AC correspondants comme intégralement validés.
- **R4 — garde :** matrice N1 bloquée par les politiques actuelles de sélection/racine. Aucun changement de garde n’est autorisé par ce plan ; escalader avant implémentation.
- **Retour arrière :** si le lot échoue, examiner son diff et retirer seulement les changements propres à `recovery.rs` après vérification des dépendances et effets. Ne pas faire de reset global, ne pas restaurer les changements d’autrui.

**Hors périmètre :** modifications des autres alertes Clippy, réduction générale du cliquet, modification de garde/baseline, nouvelles fonctionnalités de reprise, builds natifs, GPU niveau 2, campagne de performance, recette Windows, merge et push. Le nettoyage des intermédiaires propres au lot est requis à la clôture lorsque ses conditions de rétention sont établies ; tout nettoyage non admissible est différé et signalé.

**Autorisation :** seule la préparation de ce plan est autorisée actuellement. Toute implémentation Rust exige une demande et un accord écrit précis ultérieurs, les prérequis T0 satisfaits et la délégation à `rust_implementer`. Aucun commit de code ou push n’est couvert.
