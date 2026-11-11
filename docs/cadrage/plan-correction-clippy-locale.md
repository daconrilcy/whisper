Avancée : le périmètre est regroupé en un lot local, hors alertes de structure.
Blocage : l’implémentation et le cliquet de garde attendent chacun leur autorisation propre.
Décision : traiter les 96 diagnostics hors structure ensemble ; reporter les 248 alertes structurelles.

# Plan de correction Clippy locale — un lot, hors structure

## Header

- **Source audit :** [cadrage Clippy et baseline fraîche](lot-clippy-priorites.md), commit `ee26c767` ; JSONL complet archivé sous `docs/cadrage/raw/clippy-frais-20261111/`.
- **Planning date :** 2026-11-11.
- **Target workspace :** `C:\dev\whisper`, source `2972f11e`; seuls des commits documentaires ont suivi cette révision.
- **Planning mode :** plan only, no code changes.
- **Confidence :** medium — la baseline JSONL est disponible et les catégories sont dénombrées ; chaque site devra être revalidé si les sources changent avant l’implémentation.
- **Plan quality check :** passed après auto-vérification de la matrice de traçabilité, des tâches et des commandes.

## 1. Audit Digest

L’utilisateur accepte **T1 et T2 en un seul lot** : les alertes locales qui n’exigent pas une refonte de structure. La baseline fraîche compte 344 diagnostics distincts (438 émissions), dont **96 diagnostics hors structure** dans 46 fichiers. Dans cette convention, les 96 se composent de 69 autres lints Clippy et 27 avertissements Rust (`dead_code`, `unused_imports`, `unused_variables`). Ces éléments sont des diagnostics distincts selon la clé de déduplication du relevé, pas nécessairement 96 fonctions ou fichiers.

Le premier cas est `normalize_pci` : deux imports inutilisés dans le worker GPU, trois émissions. Les deux imports peuvent être retirés sans toucher à la fonction de normalisation dans `native_ffi/pure.rs` ni au contrôle d’identité PCI.

Les quatre catégories explicitement structurelles sont hors objectif : `too_many_lines` 192 sites, `cognitive_complexity` 40, `too_many_arguments` 8 et `type_complexity` 8, soit 248 diagnostics distincts cumulés. Les 192 fonctions longues ne deviennent pas un objectif de refactor ; une fonction de production ne sera scindée que lorsqu’une évolution produit la modifie et que le découpage sert cette évolution.

Parmi les 96 diagnostics hors structure, 21 `dead_code` nécessitent de vérifier `cfg`, les consommateurs et les harness avant retrait ; trois alertes (`large_enum_variant` ×2 et `result_large_err` ×1) portent sur deux contrôles privés du polling et peuvent conduire à une indirection/allocation. Elles restent dans le lot, avec un arrêt si la correction change les transitions ou leurs résultats. Les tests et leurs assertions ne sont pas simplifiés pour réduire leur longueur ou complexité.

Le cliquet de garde est une **évolution ultérieure distincte**, exclue du présent lot. Il modifierait `ci/guard-level1.ps1` et exige une autorisation explicite avant cadrage détaillé ou implémentation. Le présent plan ne modifie ni n’autorise la garde.

## 2. Freshness Check

- Clippy a été exécuté sous la garde N1 sur le workspace `--all-targets`, sans exécuter le corps des tests. Le code source mesuré était `2972f11e`; les commits documentaires suivants n’ont pas modifié les crates, manifestes, lockfile ou garde.
- Commande observée : `cargo clippy --offline --locked --workspace --all-targets --message-format=json -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity`.
- SelfTest de la garde observé : PASS, 64 cas positifs / 99 négatifs. Les journaux et les résultats metadata sont versionnés sous `docs/cadrage/raw/clippy-frais-20261111/`.
- Le plan a été borné aux 96 diagnostics qui ne sont pas dans les quatre lints structurels ci-dessus. Leurs émissions, chemins et spans sont dans le JSONL ; les principaux groupes de fichiers sont détaillés aux tâches P1-T1 à P1-T6.
- Aucun contrôle Clippy frais n’est exécuté pendant la rédaction de ce plan. L’agent d’implémentation devra refaire le relevé au début de son lot et confirmer que le diff source n’a pas décalé les sites.
- Les plans Clippy antérieurs du core et adapters/desktop ne sont pas repris comme base d’exécution : ils portaient sur des diagnostics et révisions antérieurs. Le relevé `88850e8` et le lot historique « 338 » ne sont pas comparables directement à la baseline actuelle.
- Les suites GPU de niveau 2 ne sont pas exécutées dans N1. Clippy compile les cibles GPU `--all-targets`; toute exécution du corps GPU reste hors de ce plan.

## 3. Planning Assumptions

- **Safe :** le périmètre approuvé par l’utilisateur est un seul lot couvrant les diagnostics hors structure ; les quatre lints de structure restent différés.
- **Safe :** les corrections doivent préserver valeurs, branches, ordre des effets, identités de session, protocole, publication, capture, assertions et résultats.
- **Validation needed :** au début du lot, les sources touchées et les 96 diagnostics de la baseline sont toujours applicables. Si le HEAD a changé, établir une baseline fraîche avant édition et réviser la liste des sites.
- **Validation needed :** chaque `dead_code` observé est réellement inutilisé dans les configurations et harness retenus ; aucun élément n’est supprimé sur la seule base de son nom ou de son diagnostic.
- **Validation needed :** le boxing possible des contrôles de polling ne modifie ni les sorties, ni l’ordre de priorité, ni le filtrage d’identités. Si une allocation nouvelle a un effet observable ou exige un changement de transition, arrêter et soumettre ce seul sous-périmètre.
- **Decision gate — Blocking :** le présent plan n’est pas un GO d’implémentation. Un accord précis dans la session d’implémentation est nécessaire avant toute modification Rust ou campagne Cargo.
- **Decision gate — Blocking, plus tard :** la garde de non-régression structurelle est exclue ; elle requiert une autorisation spécifique pour toucher la garde et son référentiel de seuils.

## 4. Finding Traceability

| Diagnostic / finding du relevé | Sévérité | Disposition | Tâches / gate | Notes |
|---|---|---|---|---|
| `unused_imports` : 5 diagnostics / 6 émissions, dont `normalize_pci` aux deux sites du worker GPU | IMPORTANT | Planned | P1-T1 | Retrait de l’import uniquement ; ne pas toucher à `native_ffi/pure.rs`. |
| 66 autres lints Clippy hors les quatre lints structurels et hors les 3 alertes de représentation | IMPORTANT | Planned | P1-T2 à P1-T4 | Transformations locales uniquement après vérification de l’équivalence ; chemins dans le JSONL. |
| `unused_variables` : 1 diagnostic | MINEUR | Planned | P1-T2/P1-T4 | Renommer/supprimer uniquement si la valeur n’a aucun usage requis dans la configuration. |
| `dead_code` : 21 diagnostics | IMPORTANT | Planned | P1-T5 | Analyse de `cfg`/harness/consommateurs avant retrait ; aucun `allow` pour masquer. |
| `large_enum_variant` ×2 et `result_large_err` ×1, dans deux modules privés de polling | IMPORTANT | Planned, sous arrêt de risque | P1-T6 | Représentation mémoire différente possible ; préserver séquences d’événements et sorties. |
| `too_many_lines` (192), `cognitive_complexity` (40), `too_many_arguments` (8), `type_complexity` (8) | IMPORTANT | Deferred | Hors de P1 ; décision produit | Aucun objectif de réduction générale. Une extraction future doit servir une évolution produit touchant la fonction. |
| Refus des nouvelles alertes structurelles dans la garde | IMPORTANT | Decision gate | Gate G2, lot futur séparé | Exige une autorisation spécifique ; aucune édition de garde prévue ici. |

## 5. Recommended Sequence

Un seul lot L1 regroupe T1 et T2, avec des tâches séquentielles pour garder les petits changements lisibles. Les imports `normalize_pci` ouvrent le lot ; suivent les lints locaux de production et de tests, puis l’analyse `dead_code` et les trois alertes de représentation. Les alertes structurelles existantes restent en place et ne sont ni neutralisées ni réécrites. La validation compare le candidat à une baseline fraîche avec la même commande et les mêmes options.

### Phase L1: Corriger les avertissements locaux hors structure

**Objective :** fermer les 96 diagnostics hors structure de la baseline, sans altérer le comportement.

**Closed invariant :** aucune des alertes hors structure ciblées ne subsiste ; aucun nouveau diagnostic n’apparaît sur le workspace avec la même commande. Les alertes structurelles existantes sont conservées et ne font pas l’objet de refactor.

**Before metrics :** 96 diagnostics hors structure / 116 émissions, 46 fichiers ; 248 diagnostics structurels différés. Méthode : lint + chemin primaire normalisé + lignes + message.

**Target metrics :** 0 diagnostic hors structure dans le périmètre accepté ; 0 nouveau diagnostic sur le workspace ; les 248 diagnostics structurels ne sont pas un critère d’échec de ce lot.

**Metric commands :** Clippy workspace frais, JSON, mêmes quatre options `-W`, via `ci/guard-level1.ps1`. Une déduplication avant/après avec la même clé est requise.

**Stop conditions :** échec des vérifications de préflight ou d’un test pertinent ; changement de comportement, sortie, ordre d’effet, identité, protocole, publication ou capture ; lint nécessitant un refactor d’une fonction longue pour son seul seuil ; nouveau fichier hors scope ; modification du manifeste, lockfile ou garde ; nouveau diagnostic non expliqué ; dépassement du scope L1 ; défaut de racine isolée ou du disque.

**Dependencies :** accord précis d’implémentation, état Git et baseline Clippy frais confirmés, SelfTest N1 PASS.

**Expected outcome :** lot local de corrections syntaxiques et d’usage, sans réduction générale de dette structurelle ; rapports de test et diagnostics résiduels attribués.

**Tasks :** P1-T0 à P1-T7, dans l’ordre ci-dessous.

### 6. Task Backlog

#### P1-T0: Établir le préflight et figer la liste du lot

- **Priority :** High.
- **Source finding :** baseline fraîche, tous les diagnostics ciblés.
- **Rationale :** les nombres sont valides pour le code de `2972f11e`; le lot futur doit confirmer la correspondance avant édition.
- **Target files/modules :** aucun changement produit ; sources des diagnostics dans l’archive Clippy.
- **Ownership bucket :** validation / workspace.
- **Scope budget :** 0 fichier produit ; 0 fichier de garde.
- **Closed invariant :** N/A (préflight).
- **Before metrics :** 96 diagnostics non structurels dans 46 fichiers.
- **Target metrics :** liste du lot attachée au HEAD candidat ; 0 écart inexpliqué avec la baseline.
- **Metric commands :** SelfTest N1 ; Clippy workspace frais JSON par P1-T7.
- **Stop conditions :** état Git conflictuel, fichiers touchés modifiés depuis baseline, espace <30 Gio, garde SelfTest non PASS, racines non canoniques, échec Clippy ou diagnostic non expliqué.
- **Dependencies :** accord précis d’implémentation ; aucun.
- **Implementation outline :** vérifier status Git et processus pertinents ; lire les `AGENTS.md` de chaque crate touchée ; vérifier les chemins `CARGO_HOME`, temporaires et cible ; exécuter SelfTest via la garde ; extraire des journaux JSON les seuls diagnostics hors structure et revérifier les spans.
- **Tests to add or update :** aucun.
- **Verification commands :** `ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/clippy-local -FreshTarget clippy-local` ; préflight de disque, Git et racines suivant `AGENTS.md`.
- **Command provenance :** SelfTest observé dans le dépôt ; autres contrôles provenant de `AGENTS.md`.
- **Acceptance criteria :** SelfTest PASS ; aucune écriture Cargo avant préflight ; inventaire des diagnostics revérifié et limité aux 96 entrées hors structure.
- **Rollback/fallback notes :** aucune édition ; arrêter si le HEAD est divergent et demander une baseline fraîche.
- **Notes for implementation agent :** campagne uniquement dans worktree dédié, pas depuis `C:\dev\whisper`; préserver cache Cargo persistant déclaré du worktree.

#### P1-T1: Retirer les imports inutilisés, en commençant par `normalize_pci`

- **Priority :** High.
- **Source finding :** `unused_imports`, dont `normalize_pci` (2 sites / 3 émissions).
- **Rationale :** correction locale avec l’équivalence la plus directe.
- **Target files/modules :** `crates/whisper-worker-gpu/src/native_ffi.rs`, `crates/whisper-worker-gpu/tests/support/autonomous_harness.rs`, `crates/whisper-worker-gpu/src/decoder.rs`, `crates/whisper-worker-cpu/src/decoder.rs`, `crates/whisper-adapters/src/worker_ipc/tests/unified_poll/temp_root.rs`.
- **Ownership bucket :** workers GPU/CPU, adaptateurs et tests/support.
- **Scope budget :** 3 fichiers de production + 2 modules/fichiers de tests ; dépasse le budget par défaut d’un fichier de test parce que les cinq imports `unused_imports` couvrent ces sites dans le lot unique et que l’édition ne change aucun appel ni contrat. Aucun autre lint dans ces fichiers ne peut être élargi à l’occasion.
- **Closed invariant :** aucun import listé n’est supprimé si le binding est utilisé dans une configuration/target retenu.
- **Before metrics :** 5 diagnostics `unused_imports`, 6 émissions.
- **Target metrics :** 0 diagnostic `unused_imports` ciblé ; 0 site nouveau.
- **Metric commands :** recherche `rg` des usages puis Clippy workspace frais.
- **Stop conditions :** usage conditionnel, macro ou configuration non couverte ; toute modification de la fonction PCI ou de la validation ; erreur de compilation.
- **Dependencies :** P1-T0.
- **Implementation outline :** vérifier les usages dans chaque target ; retirer seulement les noms signalés ; conserver `normalize_pci` dans `native_ffi/pure.rs` et ses appels de comparaison ; ne pas modifier les assertions du harness.
- **Tests to add or update :** aucun scénario nouveau ; les harness concernés doivent compiler. La cible GPU est compilée par `clippy --all-targets`.
- **Verification commands :** Clippy `-p whisper-worker-gpu --all-targets` ciblé, puis workspace complet par P1-T7.
- **Command provenance :** commande workspace observée ; commande ciblée conforme aux options allowlistées de `ci/guard-level1.ps1`.
- **Acceptance criteria :** uniquement les cinq sites listés sont touchés ; zéro `unused_imports` à ces sites ; `normalize_pci` reste utilisé par son implémentation de production ; aucune autre différence dans le contrôle PCI ; aucun diagnostic nouveau.
- **Rollback/fallback notes :** restaurer uniquement les imports concernés si une configuration autorisée révèle leur usage ; arrêter plutôt que supprimer une API requise.
- **Notes for implementation agent :** retirer l’import `normalize_pci`, pas la fonction. Ne pas introduire `#[allow]`.

#### P1-T2: Corriger les lints locaux dans la production adapters

- **Priority :** High.
- **Source finding :** lints non structurels sur code produit adapters.
- **Rationale :** traiter des substitutions locales sans créer d’extraction de fonctions pour les seuils structurels.
- **Target files/modules :** `crates/whisper-adapters/src/archive/import.rs`, `archive/recovery.rs`, `archive/source.rs`, `direct_live/d2_corpus.rs`, `gpu_inventory.rs`, `model_catalog.rs`, `parakeet_audio.rs`, `parakeet_confirmation.rs`, `parakeet_live/trace.rs`, `parakeet_pipeline.rs`, `worker_ipc/runtime/poll_unified.rs`.
- **Ownership bucket :** adapters (archives, capture/direct, catalogue, Parakeet et polling).
- **Scope budget :** jusqu’à 11 fichiers de production pour les diagnostics locaux identifiés ; dépassement justifié par 68 des 96 diagnostics hors structure appartenant à adapters et l’objectif utilisateur de les fermer en un lot. Aucun déplacement de module ou changement d’API.
- **Closed invariant :** chaque substitution conserve les conditions, valeur renvoyée, ordre d’effets, intégrité, publication et filtrage d’identité du chemin concerné.
- **Before metrics :** sites exacts dans le JSONL; dans `direct_live/d2_corpus.rs` 5 diagnostics locaux, ailleurs regroupés par message dans l’archive.
- **Target metrics :** aucun diagnostic hors structure ciblé dans ces fichiers ; aucun nouveau diagnostic à l’échelle du workspace.
- **Metric commands :** comparaison même clé de diagnostics avant/après ; lecture diff des expressions touchées.
- **Stop conditions :** le fix Clippy modifie le résultat, l’évaluation paresseuse, une durée de vie/fermeture, la politique d’identité ou le moment d’un effet ; correction impliquant une extraction pour le seuil de longueur/complexité.
- **Dependencies :** P1-T0.
- **Implementation outline :** appliquer le changement minimal suggéré par le lint ; dans `d2_corpus.rs`, valider les restes et frontières des chunks ; pour `drop_non_drop`, vérifier les portées d’emprunt ; préserver l’ordre des contrôles de polling et du pipeline.
- **Tests to add or update :** réutiliser les tests adapters concernés par archives, direct_live et polling ; nouveaux tests seulement si l’équivalence n’est pas déjà caractérisée, dans `tests/` ou les emplacements séparés existants.
- **Verification commands :** `cargo test` membres par défaut sous la garde ; Clippy ciblé et workspace par P1-T7.
- **Command provenance :** suite de test observée dans `AGENTS.md`; sélection Clippy conforme à `ci/guard-level1.ps1`.
- **Acceptance criteria :** rester dans les 11 fichiers listés, sans déplacement de module ; aucune altération d’octets publiés, d’ordre des effets, d’identité job/génération/instance ou de branches métier ; diagnostics ciblés disparaissent ; tous les tests concernés passent.
- **Rollback/fallback notes :** annuler seulement le changement local dont l’équivalence n’est pas démontrée ; le noter en résiduel, ne pas compenser par un `allow`.
- **Notes for implementation agent :** ne pas toucher aux fonctions longues pour abaisser leurs seuils ; si un autre lint dans une fonction impose une extraction, limiter l’édition au fix local ou arrêter.

#### P1-T3: Corriger les lints locaux dans les tests et supports adapters

- **Priority :** High.
- **Source finding :** lints non structurels dans tests/support adapters.
- **Rationale :** garder le signal Clippy des tests sans relâcher les scénarios ni les assertions.
- **Target files/modules :** `crates/whisper-adapters/src/model_catalog/concurrency_tests.rs`, `src/parakeet_import/legacy_runtime_tests.rs`, `src/worker_ipc/runtime/poll/tests.rs`, `src/worker_ipc/tests/microphone_and_finalization.rs`, `native_execution.rs`, `queue_inspection_characterization.rs`, `unified_capture.rs`, `unified_poll/temp_root.rs`, `unified_support.rs`, `tests/parakeet_archive.rs`, `parakeet_import.rs`, `parakeet_long.rs`, `parakeet_unified_native.rs`, `reprocessing_cleanup.rs`, `reprocessing_create.rs`, `tests/support/pe_fixture.rs`.
- **Ownership bucket :** tests/support adapters.
- **Scope budget :** jusqu’à 16 fichiers de tests/support, 0 fichier de production. Regroupement accepté car les diagnostics sont des transformations locales de test et le lot utilisateur regroupe T1+T2 ; les assertions, fixtures PE et scénarios restent inchangés.
- **Closed invariant :** mêmes entrées, assertions effectives, scénarios exécutés, sorties attendues et identité de test.
- **Before metrics :** les diagnostics exacts sont 96-entry baseline JSONL; les chemins ci-dessus sont les sites adapters de test/support.
- **Target metrics :** aucun diagnostic hors structure visé dans ces fichiers ; aucun test supprimé, ignoré ou réduit.
- **Metric commands :** décompte des tests et identités via les sorties Cargo sous garde ; même Clippy workspace.
- **Stop conditions :** `assertions_on_constants` ne peut être converti en assertion réellement reliée à une valeur sans modifier le scénario ; un remplacement affaiblit l’assertion ; un `dead_code` est supprimé sans vérification de harness.
- **Dependencies :** P1-T0.
- **Implementation outline :** conserver la structure des cas ; corriger seulement emprunts/itérations/conditions/closures/style explicitement signalés ; pour `assertions_on_constants`, établir ce que le test cherche à protéger avant d’éditer.
- **Tests to add or update :** aucun nouveau scénario dans `src`; exécuter les tests adapters de la suite N1 et vérifier les identités de tests avant/après.
- **Verification commands :** `cargo test --no-fail-fast` par P1-T7 ; les cibles de test sont également compilées par Clippy `--all-targets`.
- **Command provenance :** `cargo test --no-fail-fast` et la suite de référence viennent de `AGENTS.md`; Clippy observé.
- **Acceptance criteria :** rester dans les 16 fichiers listés ; mêmes cas/identités et assertions ; tests concernés réussis ; aucun diagnostic hors structure attribué à ces sites.
- **Rollback/fallback notes :** laisser le diagnostic ciblé résiduel et justifié si sa suppression affaiblit un test ; demander arbitrage pour tout changement de scénario.
- **Notes for implementation agent :** ne pas fractionner les fonctions pour satisfaire `too_many_lines` ou `cognitive_complexity`.

#### P1-T4: Traiter les diagnostics locaux des autres crates

- **Priority :** Medium.
- **Source finding :** lints hors structure core, desktop, worker CPU, GPU et Parakeet CPU.
- **Rationale :** fermer les diagnostics locaux en dehors du plus gros groupe adapters.
- **Target files/modules :** `crates/whisper-core/src/application/runtime/tests/closing_and_recovery.rs`, `crates/whisper-core/tests/parakeet_windows.rs`, `crates/whisper-desktop/src/ui/model/controller.rs`, `crates/whisper-worker-cpu/tests/support/worker.rs`, `crates/whisper-worker-gpu/tests/support/worker.rs`, `crates/whisper-worker-gpu/src/native_ffi.rs`, `crates/whisper-worker-parakeet-cpu/src/runtime_paths.rs`, `crates/whisper-worker-parakeet-cpu/src/session.rs`, `crates/whisper-worker-parakeet-cpu/tests/support/worker.rs`.
- **Ownership bucket :** cœur/tests, desktop/interface, workers/tests/support.
- **Scope budget :** jusqu’à 4 fichiers de production et 5 fichiers de tests/support ; dépassement d’un fichier de production parce que le dernier site est un lint local dans le même module GPU que P1-T1. Les catégories et règles métier demeurent inchangées.
- **Closed invariant :** mêmes politiques de sélection/erreur, mêmes assertions et mêmes flux IPC.
- **Before metrics :** diagnostics hors structure par chemins cités dans le JSONL.
- **Target metrics :** zéro diagnostic ciblé, aucun nouveau diagnostic.
- **Metric commands :** Clippy même workspace et sélection tests par P1-T7.
- **Stop conditions :** changement du résultat, erreur retournée, choix de GPU/CPU, politique d’admission, protocole ou UI ; retirer `assertions_on_constants` sans préserver l’intention du scénario ; fixer un lint exigerait un refactor structurel.
- **Dependencies :** P1-T0.
- **Implementation outline :** appliquer uniquement les remplacements locaux ; examiner les options de boucle/itération pour préserver zéro/une/plusieurs valeurs ; conserver les tests de sélection et admission.
- **Tests to add or update :** aucun scénario nouveau par défaut ; suites CPU/common/Parakeet CPU existantes et tests workspace concernés.
- **Verification commands :** PE des harnais worker avant les sélections CPU/Parakeet CPU ; tests N1 par P1-T7.
- **Command provenance :** `ci/check-worker-imports.ps1` et sélections sont définis par `AGENTS.md` et `docs/architecture/GARDE-N1.md`.
- **Acceptance criteria :** rester dans les neuf chemins listés ; mêmes décisions visibles et erreurs, tests concernés passants, zéro lint ciblé restant.
- **Rollback/fallback notes :** isoler et reporter le site si la modification change une politique ou si la couverture ne permet pas de l’établir.
- **Notes for implementation agent :** respecter les `AGENTS.md` de chaque crate avant modification ; les bodies GPU restent hors niveau 1.

#### P1-T5: Auditer les 21 diagnostics `dead_code`

- **Priority :** Medium.
- **Source finding :** `dead_code`, 21 diagnostics distincts / 22 émissions, 9 fichiers.
- **Rationale :** distinguer code réellement inutilisé des modules compilés selon `cfg` et des harness synthétiques.
- **Target files/modules :** `crates/whisper-adapters/src/native_timing.rs`, `crates/whisper-adapters/tests/support/pe_fixture.rs`, `crates/whisper-worker-cpu/src/decoder.rs`, `crates/whisper-worker-cpu/src/engine_cache.rs`, `crates/whisper-worker-cpu/tests/support/autonomous_harness.rs`, `crates/whisper-worker-gpu/src/decoder.rs`, `crates/whisper-worker-gpu/src/engine_cache.rs`, `crates/whisper-worker-gpu/src/gpu_qualification.rs`, `crates/whisper-worker-gpu/tests/support/autonomous_harness.rs`.
- **Ownership bucket :** adapters, workers CPU/GPU et tests/support.
- **Scope budget :** jusqu’à 6 fichiers de production + 3 fichiers de tests/support. C’est un dépassement du budget de production limité aux sites `dead_code` de la baseline ; toute suppression qui affecte une autre configuration/harness arrête ce sous-lot.
- **Closed invariant :** aucun appel/usage requis sous une configuration produit ou test n’est retiré ; aucun `allow(dead_code)` ajouté.
- **Before metrics :** 21 diagnostics, 9 fichiers.
- **Target metrics :** aucun `dead_code` ciblé non justifié ; si un élément est requis dans une configuration hors baseline, le conserver et soumettre le diagnostic résiduel, sans neutralisation.
- **Metric commands :** recherche des usages `rg`, inspection de `cfg`/manifestes, Clippy all-targets ; aucune compilation de feature supplémentaire sans scope autorisé.
- **Stop conditions :** usage via macro/réexport/feature, appel indirect non établi, modification de configuration, ajout d’un `allow` ou besoin de nouveaux tests/hook.
- **Dependencies :** P1-T0 et P1-T1 à T4.
- **Implementation outline :** identifier chaque symbole et toutes ses configurations ; retirer seulement ceux démontrés inaccessibles dans le produit et les cibles autorisées ; conserver les helpers liés aux frontières PE/worker si le harness les requiert.
- **Tests to add or update :** pas de nouveau test qui élargit les configurations ; réutiliser les harness existants.
- **Verification commands :** Clippy workspace ; N1 suites autorisées ; pas de GPU body.
- **Command provenance :** Clippy observé ; tests par `AGENTS.md`.
- **Acceptance criteria :** rester dans les neuf fichiers listés ; zéro suppression d’un code utilisé par un produit/harness ; diagnostics fermés ou justifiés site par site ; aucun `allow`.
- **Rollback/fallback notes :** rétablir le symbole au premier usage révélé ; conserver un résiduel explicitement attribué plutôt que changer `cfg`.
- **Notes for implementation agent :** traiter le `dead_code` des modules `#[path]` de harness comme un possible artefact du harness, pas comme une preuve suffisante de code mort produit.

#### P1-T6: Corriger les alertes de représentation des contrôles privés de polling

- **Priority :** Medium.
- **Source finding :** `large_enum_variant` ×2 et `result_large_err` ×1.
- **Rationale :** ces trois diagnostics ne sont pas dans les catégories structurelles écartées par l’utilisateur, mais leurs corrections possibles affectent la représentation d’événements du polling.
- **Target files/modules :** `crates/whisper-adapters/src/worker_ipc/runtime/poll/collection.rs`, `crates/whisper-adapters/src/worker_ipc/runtime/poll/terminal_routing.rs`.
- **Ownership bucket :** adapters / orchestration polling.
- **Scope budget :** 2 fichiers de production et leurs tests directs existants ; aucune API publique ni protocole.
- **Closed invariant :** les mêmes variantes, résultats `PollOutput`, priorité des événements, job/generation/instance, ACK et traitement d’erreur sont observés avant/après.
- **Before metrics :** deux sites `large_enum_variant`, un site `result_large_err` ; enum contrôles à environ 592 octets selon Clippy.
- **Target metrics :** zéro de ces trois diagnostics si la solution reste une indirection privée sans changement de transition ; sinon sites conservés pour décision, sans suppression de lint.
- **Metric commands :** Clippy workspace et tests existants de polling ; diff manuel des producteurs/consommateurs de chaque enum.
- **Stop conditions :** allocation nouvelle sur un chemin chaud avec impact mesurable, changement d’ordre ou de sémantique, modification d’un type de protocole/public, ou nécessité de refactor poll au-delà de ces deux fichiers.
- **Dependencies :** P1-T0 à T4.
- **Implementation outline :** inspecter chaque producteur/consommateur ; évaluer le boxing proposé par Clippy ; effectuer le changement interne minimal seulement si toutes les transitions et valeurs restent identiques ; relire les fonctions poll sans abaisser leurs seuils structurels.
- **Tests to add or update :** réutiliser les tests de polling/terminal routing existants ; ajouter un petit test privé uniquement si une invariance de représentation ne peut pas être exprimée par les tests existants.
- **Verification commands :** tests adapters de polling de la suite N1, puis Clippy workspace par P1-T7.
- **Command provenance :** Clippy observé ; tests gardés selon `AGENTS.md`.
- **Acceptance criteria :** rester dans les deux fichiers listés et leurs tests existants ; mêmes événements retournés dans le même ordre, même filtrage d’identités, aucun protocole changé, diagnostics fermés ou décision résiduelle documentée.
- **Rollback/fallback notes :** si la solution augmente un coût de polling ou modifie une transition, revert du seul site et report.
- **Notes for implementation agent :** ne pas changer les contrats IPC, publication ou sessions unifiées pour éteindre ces lints.

#### P1-T7: Valider le lot combiné et rapporter les résiduels

- **Priority :** High.
- **Source finding :** CA-01 à CA-04 et tous les diagnostics planifiés.
- **Rationale :** une clôture est fondée sur la même commande complète, avec les écarts par cible séparés.
- **Target files/modules :** aucun fichier produit supplémentaire ; sorties sous les racines autorisées et rapport du lot prévu.
- **Ownership bucket :** validation workspace.
- **Scope budget :** 0 fichier production ; 0 test source ajouté sauf si une tâche précédente le justifie.
- **Closed invariant :** même commande de lint passe, diagnostics ciblés 96→0 (ou résiduels avec gate utilisateur), pas de nouveau site ; tests requis passent.
- **Before metrics :** 96 diagnostics hors structure, 116 émissions ; 248 structurels hors scope.
- **Target metrics :** 0 diagnostic hors structure dans le scope clôturé ; structure existante toujours rapportée, sans objectif de réduction.
- **Metric commands :** `cargo fmt --check`, Clippy workspace JSON, `cargo test --no-fail-fast` sans `CARGO_TARGET_DIR` via la garde, tests worker N1 requis.
- **Stop conditions :** tout échec, diagnostic nouveau non expliqué, test silencieusement ignoré, échec du PE gate, écart de sortie, écriture hors racine ou état Git concurrent pertinent.
- **Dependencies :** P1-T1 à T6.
- **Implementation outline :** exécuter les vérifications prévues par garde ; comparer le JSON final au baseline avec la même clé ; attribuer les messages aux crates/cibles ; rapporter tests, ignorés, limites et toute alerte structurelle laissée intacte.
- **Tests to add or update :** aucun dans cette tâche.
- **Verification commands :** détails dans Verification Matrix.
- **Command provenance :** Clippy observé ; N1, fmt et PE tirés de `AGENTS.md`/`docs/architecture/GARDE-N1.md`.
- **Acceptance criteria :** tous les codes de sortie et comptes rapportés ; commandes uniquement via garde ; aucun test GPU N1 présenté comme exécuté.
- **Rollback/fallback notes :** en échec, arrêter, préserver sorties ; ne pas restaurer ou nettoyer des données sans état vérifié.
- **Notes for implementation agent :** la livraison locale et le push restent séparés ; aucun push sans accord précis.

## 7. Verification Matrix

| Area | Command or check | Provenance | Expected result | Covers tasks | Notes |
|---|---|---|---|---|---|
| Guard | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/clippy-local -FreshTarget clippy-local` | Repo docs | PASS avant Cargo | P1-T0, P1-T7 | Dans worktree dédié ; préflight racines/disque et activité Git. |
| Format | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/clippy-local -CargoArgsJson '["fmt","--check"]'` | Repo docs | Aucun changement de format | P1-T1 à T7 | Pas d’appel Cargo direct. |
| Baseline/closure Clippy | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/clippy-local -FreshTarget clippy-local -CargoArgsJson '["clippy","--workspace","--all-targets","--message-format=json","--","-W","clippy::too_many_lines","-W","clippy::cognitive_complexity","-W","clippy::too_many_arguments","-W","clippy::type_complexity"]'` | Observed | 96→0 diagnostics ciblés ; aucun nouveau site ; les 248 alertes structurelles sont seulement rapportées | Tous | La garde ajoute `--offline --locked`. Dédupliquer avec la même clé ; archiver les petites preuves utiles au lot. |
| Clippy GPU ciblé | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/clippy-local -ExistingTarget clippy-local -CargoArgsJson '["clippy","-p","whisper-worker-gpu","--all-targets","--message-format=json","--","-W","clippy::too_many_lines","-W","clippy::cognitive_complexity","-W","clippy::too_many_arguments","-W","clippy::type_complexity"]'` | Source inferred | Aucun `normalize_pci` ; cibles GPU compilent | P1-T1 | N’est qu’un contrôle intermédiaire ; la fermeture utilise le workspace complet. |
| Workspace tests | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -WithoutCargoTargetDir -ExistingTarget clippy-local -LotRoot target/parakeet-tests/clippy-local -CargoArgsJson '["test","--no-fail-fast"]'` | Repo docs | 0 échec ; nombre d’ignorés déclaré | P1-T2, T3, T4, T6, T7 | Shell dédié, cible fixée par `--config`; N1, aucune écriture Cargo hors racines. |
| Worker CPU | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/clippy-local -ExistingTarget clippy-local -CargoArgsJson '["test","-p","whisper-worker-cpu","--no-run"]'`, PE exact `pwsh -NoProfile -File ci/check-worker-imports.ps1 -Kind Cpu -Path <harnais exact>`, puis même garde avec `CargoArgsJson '["test","-p","whisper-worker-cpu"]'` | Repo docs | PE PASS avant test ; test vert | P1-T4/T5/T7 | Chemin exact lu dans la sortie gardée ; aucun glob approximatif. |
| Worker Parakeet CPU | Même garde et cible pour `CargoArgsJson '["test","-p","whisper-worker-parakeet-cpu","--no-run"]'`, contrôle de chaque harnais par `pwsh -NoProfile -File ci/check-worker-imports.ps1 -Kind ParakeetCpu -Path <harnais exact>`, puis garde avec `CargoArgsJson '["test","-p","whisper-worker-parakeet-cpu"]'` | Repo docs | PE PASS avant test ; test vert | P1-T4/T5/T7 | Chemin exact issu de la sortie ; aucun glob approximatif. |
| Worker common | Même garde et cible pour `CargoArgsJson '["test","-p","whisper-worker-common","--no-run"]'`, PE de chaque harnais avec `pwsh -NoProfile -File ci/check-worker-imports.ps1 -Kind Cpu -Path <harnais exact>`, puis garde avec `CargoArgsJson '["test","-p","whisper-worker-common"]'` | Repo docs | PE PASS avant test ; test vert | P1-T4/T5/T7 | Chemin exact issu de la sortie ; aucun glob approximatif. |
| Worker GPU | Clippy workspace `--all-targets` ; inventaire optionnel `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/clippy-local -ExistingTarget clippy-local -CargoArgsJson '["test","-p","whisper-worker-gpu","--","--list"]'` | Repo docs | Harnais compilé ; identité seulement si `--list` exécuté | P1-T1/T4/T5/T7 | Corps GPU non exécuté en N1. |
| Usage/cfg audit | `rg -n '<symbole relevé dans le JSONL>' crates/<crate>` puis lecture des `cfg`, réexports et harness autour des usages | Source inferred | Chaque `dead_code` prouvé ou gardé comme résiduel | P1-T5 | Placeholder résolu par le nom du diagnostic ; aucune suppression par seul lint. |
| Diff / Git | `git diff --check`, revue diff et `git status --short --branch` | Repo docs | Aucun whitespace issue ni modification hors scope | Tous | Git séparé de la campagne Cargo. |

## 8. Decision Gates

| Gate | Status | Blocks | Question | Recommended default | How to validate |
|---|---|---|---|---|---|
| G1 — GO code/campagne | Blocking | Tout P1-T1 à T7 | L’utilisateur autorise-t-il le lot combiné dans la session d’implémentation ? | Aucun défaut ne vaut autorisation ; attendre un GO précis | Plan accepté + fichiers/crates, worktree/campagne et contrôles explicitement cités. |
| G2 — représentation polling | Validation | Fermeture P1-T6 seulement | L’indirection des deux enums privées ajoute-t-elle un coût ou change-t-elle un résultat/ordre ? | Garder les contrôles actuels si équivalence stricte non établie | Tests polling, diff consommateurs et comportement; arrêter si allocation/latence observable. |
| G3 — cliquet de garde | Blocking, lot futur | Toute modification de `ci/guard-level1.ps1` | Quel périmètre exact de lints structurels, quelle règle de reconnaissance des alertes déjà présentes et quelle fixture de non-régression ? | Hors du présent lot ; demander une autorisation explicite avant son cadrage détaillé et toute édition | Nouvelle mission avec critères et accord citant la garde, le baseline et la politique pour les alertes existantes.

## 9. Out of Scope

- Réduire les 192 `too_many_lines` ou poursuivre les seuils de complexité dans la production ou les tests comme objectif autonome.
- Extraire des modules, changer façades/visibilités, renommer APIs, ajouter des couches, dépendances, features, hooks ou nouveaux scénarios produit.
- Changer la logique de capture, polling, admission GPU/CPU, protocoles, sérialisation, identités, archives, publication ou UI.
- Ajouter des `allow`/`expect`/baseline pour masquer les alertes ciblées.
- Modifier ou étendre la garde Clippy dans ce lot ; le cliquet structurel est un projet ultérieur soumis à son accord précis.
- Lancer des builds DLL natifs, tests niveau 2 GPU, validation fenêtre Windows, installation, push ou nettoyage de worktree.

## 10. Final Self-Check

| Check | Result | Notes |
|---|---|---|
| Real plan content, not placeholder output | Pass | Lot L1, 8 tâches et portée future du cliquet séparée. |
| All template sections present and in order | Pass | Sections Header, Audit, Freshness, Assumptions, Traceability, Sequence, Tasks, Verification, Gates, Out-of-scope, Self-check, Prompt. |
| Every audit finding mapped | Pass | 96 hors structure planifiés ; 248 structurels différés ; garde future sous gate. |
| Each task has targets, dependencies, tests, verification, criteria, fallback | Pass | Budgets et arrêts spécifiés pour chaque tâche. |
| Structural phase metrics and stop conditions | Pass | Aucune phase structurelle planifiée ; metrics structurels inclus comme hors scope. |
| Scope budget per task | Pass | Dépassements justifiés par le lot unique et les 96 sites répartis. |
| Rust test placement | Pass | Tests existants privilégiés ; aucun test de scénario ajouté en production. |
| Verification command provenance | Pass | Chaque entrée est `Observed` ou `Repo docs`. |
| Decision gates state blocking/defaultable | Pass | GO, polling et garde distingués. |
| Paths valid | Pass | Chemins relatifs au dépôt ; aucun chemin de code inventé.
| Scope bounded | Pass | Un lot local, 8 tâches, aucun objectif de refactor structurel.

## 11. Implementation Agent Prompt

Après GO explicite seulement, déléguer le lot L1 de ce plan à `rust_implementer` en mode `IMPLEMENT`. Commencer par P1-T0 ; ne pas modifier le code avant le SelfTest N1 et la baseline Clippy fraîche. Préserver tous les changements tiers et limiter le diff aux sites hors structure planifiés. Ne pas réduire les alertes de longueur/complexité, ne pas modifier la garde, Cargo manifests/lockfile, ni contourner les lints avec `allow`. Exécuter uniquement les vérifications gardées de P1-T7 et rapporter diagnostics ciblés, structurels laissés, tests exécutés/ignorés, fichiers et limites. La session principale commandera ensuite une revue indépendante ; aucun push ni test GPU niveau 2 sans accord précis distinct.
