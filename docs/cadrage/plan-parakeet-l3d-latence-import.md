Avancée : L3d v2 a été revu CLEAN — PLANS READY sur les trois écarts laissés par L3c.
Blocage : le candidat L3c reste non commité et G-LOG demande un accord d'écriture ciblé avant l'exécution.
Décision : attendre l'acceptation du plan et l'autorisation écrite du lot ; aucun code L3d n'est autorisé ici.

# PLAN-PARA-L3d — latence du premier import GPU et vérifications manquantes

Version : v2, PLANS READY après revue CLEAN, 19/10/2026. Source : revue indépendante L3c `LIVRÉ AVEC RÉSERVES`, candidat non commité sur `evo-startup-perf` à partir de `088681edc345174b7845a07f8f5e08a3bd9046f3`, et [plan L3c](plan-parakeet-l3c-sha-cache.md). Workspace : `C:\dev\whisper`. Mode : **plan seulement, aucun code produit autorisé**. Confiance : **medium** ; les traces identifient le délai, mais l'effet du traitement groupé reste à mesurer. Contrôle qualité du plan : à faire après revue.

## 1. Audit Digest

Le test release `gpu_app_en5_twice_release` conserve une même instance d'`ImportApplication` et d'`AsyncImportIo`. Sur le dernier candidat, le premier import de `sound example/spkr0.wav` a pris **16,3005292 s** de `StartImport` aux TXT/SRT durables, le second **15,234 s**. Le seuil L3c du premier import, **≤ 15 s**, reste inchangé. Les dix artefacts GPU et le modèle ont été entièrement lus une fois par l'adaptateur et zéro fois au second import ; la vérification SHA propre au worker reste séparée.

Dans `target/parakeet-tests/l3/l3c/logs/r7-timing.jsonl`, pour le processus adaptateur `101804`, `worker_request_to_result` finit vers 10,083 s après 7,140 s ; 99 `archive_persist_segment` s'étalent ensuite d'environ 10,154 à 16,300 s, mais totalisent environ 0,290 s d'I/O chronométrée. `application.rs::poll_once()` ne lit qu'un événement par `Refresh`, et le harness EN5 attend 33 ms entre deux `Refresh` ; **inférence à confirmer** : les allers-retours `Segment`/`Persisted` expliquent l'essentiel des 6,2 s. Il faut mesurer la chaîne complète avant de changer sa cadence. `archive.rs::persist_segment()` fait `sync_all()` à chaque segment ; sa durée cumulée observée n'en fait pas la première cible d'optimisation.

La revue a fermé les constats de code L3C-R1/R3/R4/R5, mais deux contrôles restent NOT RUN : le test unitaire exact de confinement R5, faute d'un mode `--lib` de la garde L3c, et la recette GPU historique L3b sans `PARAKEET_L3C_EXEC_ROOT`. Cette dernière crée un journal `parakeet-asr-*.log` sous L3b et exige une autorisation d'écriture ciblée distincte. La suite workspace L3c a passé 52 suites, zéro échec et 34 tests ignorés ; A4 FR87/FR27/EN15, EN30 et les cinq R8 ont passé sous garde.

## 2. Freshness Check

Lecture ciblée le 19/10/2026 : `crates/whisper-core/src/application.rs::poll_once/accept_segment/maybe_publish`, `crates/whisper-adapters/src/archive.rs::persist_segment`, `crates/whisper-adapters/tests/parakeet_import.rs::gpu_app_en5_twice_release`, `crates/whisper-adapters/src/parakeet_import.rs::worker_log_prefix` et les journaux L3c cités. Tous existent dans l'arbre courant. Les sept fichiers du candidat L3c et son plan amendé sont modifiés et non commités ; `models/` et `sound example/` sont non suivis et restent intouchés. Aucun essai supplémentaire ni modification de code n'a été fait pour ce cadrage. La preuve temporelle est liée à ce candidat et doit être revalidée après toute modification.

## 3. Hypothèses et frontières

| Hypothèse | Statut | Vérification |
| --- | --- | --- |
| Le délai post-inférence provient surtout du nombre de rafraîchissements applicatifs | Validation needed | Corréler `Segment`, `Persisted`, `Refresh` et horloges du worker et de l'archive avant correction. |
| Un drainage borné de plusieurs événements par `Refresh` garde l'ordre et la réactivité UI | Validation needed | Tests de transitions, arrêt, reprise, live et chronométrage EN5 ; ne pas modifier l'archive durable sans preuve contraire. |
| L3c devient une base Git stable avant L3d | Decision gate | L'utilisateur doit décider du sort du candidat L3c livré avec réserves ; aucun lot d'implémentation ne démarre sur un diff ambigu. |
| La recette historique L3b peut créer son journal `parakeet-asr-*.log` sous `target/parakeet-tests/l3/l3b/logs/` | Decision gate | Accord écrit précis pour cette exception d'exécution et contrôle de garde avant de lancer la recette. |

### Racines exactes proposées pour L3d

`PARAKEET_L3D_EXEC_ROOT=C:\dev\whisper\target\parakeet-tests\l3\l3d` est la racine canonique de toutes les archives, fixtures, journaux ordinaires et TEMP/TMP ; `PARAKEET_L2_ARCHIVE_ROOT=...\l3d\archive` et `PARAKEET_R8_EXEC_ROOT=...\l3d\r8`. `PARAKEET_L3D_RECIPE_CASE` nomme exactement le sélecteur ciblé. `CARGO_TARGET_DIR=...\target\parakeet-tests\cargo-shared`. `PARAKEET_L2_RUNTIME_ROOT=...\l3\l3b` reste l'ancre de lecture du worker GPU figé et `PARAKEET_L2_NATIVE_DIR=...\l3b\native` reste en lecture seule. Le worker GPU exige un journal ASR sous sa racine : G-LOG doit autoriser **seulement** `l3b/logs/l3d-parakeet-asr-*.log` pendant les recettes L3d, et `l3b/logs/parakeet-asr-*.log` pendant la recette historique sans variable L3d. Aucune autre écriture L3b. Le worker CPU R6, ses DLL, les modèles, `sound example/` et spike02 restent en lecture seule.

Le code de production `src/parakeet_import.rs` doit reconnaître la variable L3d uniquement si sa valeur canonique est exactement cette racine, choisir le préfixe de journal L3d et ne pas changer les parcours sans variable. Son helper privé `is_canonical_l3c_test_artifact()` accepte aussi L3d **seulement sous la feature et le drapeau de compilation d'acceptation existants**, pour les faux workers/DLL R8 ; le nom peut rester historique, mais le commentaire doit préciser les deux racines. `src/worker_ipc.rs` n'ajoute l'admission de L3d que sous cette même frontière privée. Les helpers de tests `tests/parakeet_import.rs::selected_recipe/r8_exec_root`, le préfixe ASR du cas R8 incompatible, `tests/parakeet_archive.rs::archive_root` et `whisper-desktop/src/root.rs::fixture_root`, **avec le test directement lié `fixture_root_accepts_only_the_canonical_l3c_campaign_root`**, acceptent cette seule racine L3d sans retirer l'assertion L3c ni les racines historiques. La garde refuse un chemin voisin, une racine non canonique ou un sélecteur donnant zéro test. Cette ouverture demande un accord d'implémentation distinct ; elle n'est pas accordée par le présent cadrage.

## 4. Traçabilité des constats

| Constat | Gravité | Disposition | Tâche / gate | Preuve attendue |
| --- | --- | --- | --- | --- |
| L3C-R2 : premier EN5 >15 s | Bloquant produit | Planned | D1, D2, D4 | Premier import ≤15 s sur trois passages release du même parcours, avec TXT/SRT. |
| R5 : test unitaire exact NOT RUN | Important | Planned | D3 | Test ciblé exécuté et PASS sous garde, sans compter zéro test comme PASS. |
| Recette historique L3b sans variable L3c NOT RUN | Important | Decision gate | G-LOG, D3 | Recette réellement exécutée sous garde après accord d'écriture ciblé. |
| Cache SHA L3c et cinq R8 déjà prouvés | Régression à prévenir | Planned | D4 | Compteurs et scénarios restent PASS sur le binaire corrigé. |

## 5. Séquence recommandée

Le routage de campagne D3 est posé d'abord pour rendre la racine L3d et les recettes exécutables, sans toucher la cadence. D1 prouve ensuite le chemin critique ; D2 corrige seulement la cadence si cette cause est confirmée. D4 ferme la recette release sur le même candidat.

### Phase 0 : confiner et exécuter la campagne L3d

Objectif : fermer D3 avant toute recette EN5 L3d. Invariant fermé : tous les chemins d'écriture de campagne sont sous L3d/Cargo partagé ou correspondent exactement aux journaux ASR G-LOG, et le test unitaire R5 exécute un cas. Avant : helpers L3c seulement, deux NOT RUN. Cible : racine L3d canonique acceptée seulement dans les branches prévues, 12/12 auto-tests de garde, test R5 et recette historique exécutés PASS. Mesure : garde et compteurs de tests. Arrêt : G-BASE ou G-LOG non accordé, zéro test, ou écriture hors motif.

### Phase 1 : attribuer la latence

Objectif : relier chaque événement EN5 à la publication. Invariant fermé : aucun temps >100 ms du parcours `StartImport` → TXT/SRT ne reste sans étape attribuée ou intervalle d'attente identifié. Avant : environ 6,2 s entre premier segment persisté et dernier, 0,290 s d'I/O cumulée. Cible : tableau temporel avec horodatages `Segment`/`Persisted`/`Refresh`/publication, nombre d'événements et temps d'attente. Mesure : recette EN5 release gardée, journaux sous racine L3d, après D3. Arrêt : si la cadence des `Refresh` n'explique pas le délai, ne pas passer à D2 ; produire `DESIGN_CHANGE_REQUIRED` avec la cause mesurée.

### Phase 2 : corriger une seule cause

Objectif : réduire l'attente entre résultat natif et publication sans modifier la durabilité des segments. Invariant fermé : les événements d'un job sont appliqués **dans l'ordre**, au plus 32 par `Refresh` ; un événement de fin n'est publié qu'après tous les accusés de persistance, et Stop/reprise/micro conservent leurs transitions. Avant : un événement par appel. Cible : drainage borné, zéro perte/doublon, ≤15 s au premier EN5 release. Mesure : tests de transitions + traces EN5. Arrêt : si 32 événements par appel altèrent une transition ou ne suffisent pas pour le seuil, ne pas élargir à l'archive ni assouplir 15 s sans nouveau plan.

### Phase 3 : prouver et clôturer

Objectif : obtenir le test R5, la recette L3b historique et les régressions sur le candidat final. Invariant fermé : toutes les recettes annoncées PASS ont réellement exécuté leur sélecteur, sous garde, sans écriture hors chemins autorisés. Avant : deux NOT RUN. Cible : zéro NOT RUN pour ces deux contrôles, suite workspace verte, trois premiers imports EN5 consécutifs ≤15 s ; les autres ignorés sont listés séparément. Mesure : journaux de garde, compteur de tests, SHA avant/après. Arrêt : une violation de garde, un SHA divergent ou un échec de recette stoppe la campagne et interdit la qualification.

## 6. Tâches (quatre, au plus sept fichiers sources/tests)

### D1 — établir le chemin critique

- Priorité : haute. Source : L3C-R2. Propriétaire : tests/support ; aucun changement fonctionnel.
- Chemins : `crates/whisper-adapters/tests/parakeet_import.rs` pour un sélecteur diagnostique distinct, éventuellement `crates/whisper-core/src/application.rs` pour une instrumentation minimale de `Refresh` sous variable de campagne ; preuves sous `target/parakeet-tests/l3/l3d/`.
- Budget : au plus deux fichiers dont un de production, instrumentation retirée avant livraison si elle n'est pas utile au diagnostic futur.
- Dépendances : D3, décision sur base L3c ; préflight des SHA, espace libre ≥30 Gio et garde. Outline : horodater demande, événements, accusés et publication ; corréler `JobId` et PID ; comparer la durée de l'I/O et les intervalles sans travail. Test : ajouter `gpu_app_en5_timing_diagnostic`, même parcours et mêmes TXT/SRT que le test strict, mais qui **rapporte** le dépassement de 15 s sans l'asserter ; un code 0 ne vaut que PASS du diagnostic, jamais du critère C4. Commande : mode `ReleaseRecipe` de la garde L3d avec `cargo test --release --offline --locked -p whisper-adapters --test parakeet_import gpu_app_en5_timing_diagnostic -- --exact --nocapture` (source : plan L3c et sélecteur D1 à ajouter).
- Acceptation : tableau de temps sans double comptage, cause dominante établie ; zéro écriture protégée. Repli : diagnostic seulement, aucune optimisation spéculative.

### D2 — drainer les événements applicatifs avec une borne

- Priorité : haute. Source : L3C-R2. Propriétaire : application/core. Chemins : `crates/whisper-core/src/application.rs`, `crates/whisper-core/tests/import_contract.rs` ; `crates/whisper-adapters/tests/parakeet_import.rs` pour la recette.
- Budget : un fichier produit et deux fichiers de test. Dépendance : D1 confirme la cause. Outline : garder la validation et les transitions existantes, traiter les événements déjà disponibles dans leur ordre jusqu'à 32 par `Refresh`, sortir immédiatement si la file est vide, respecter Stop et les générations ; conserver les `PersistSegment` et leurs accusés avant `Publish` sans changer `archive.rs`. La borne 32 s'applique à chaque appel, sans boucle d'attente active quand la file est vide.
- Tests : rafale d'au moins 99 segments avec End et accusés retardés ; arrêt et reprise entre deux segments ; événement périmé ; micro direct et Turbo inchangés. Commandes : `cargo test --offline --locked -p whisper-core --test import_contract`, puis suite workspace complète sous garde L3d (provenance : source/AGENTS.md).
- Acceptation : ordre, compte, état et sortie durable identiques aux tests historiques ; pas de boucle sans borne ni de blocage UI ; premier EN5 release ≤15 s. Repli : revenir au drainage d'un seul événement si une transition n'est pas préservée, conserver la preuve de D1 et demander un nouveau design.

### D3 — rendre exécutables les deux contrôles manquants

- Priorité : moyenne. Source : R5 NOT RUN et recette L3b NOT RUN. Propriétaire : tests/support. Chemins : script privé `target/parakeet-tests/l3/l3d/guard-campaign.ps1`, `crates/whisper-adapters/src/worker_ipc.rs` (seuls l'embranchement de qualification R8 et son test), `crates/whisper-adapters/src/parakeet_import.rs` (routage canonique du journal ASR **et seul helper privé `is_canonical_l3c_test_artifact()`**), `crates/whisper-adapters/tests/parakeet_import.rs` (sélecteurs, racine R8 et préfixe ASR du cas incompatible), `crates/whisper-adapters/tests/parakeet_archive.rs`, et `crates/whisper-desktop/src/root.rs::fixture_root()` **et son test directement lié `fixture_root_accepts_only_the_canonical_l3c_campaign_root`**. Aucune suppression des assertions historiques.
- Budget : trois fichiers sous `src/`, dont `root.rs` limité à son helper de test, et deux fichiers sous `tests/` ; leur ouverture est nécessaire car les gardes L3c actuelles refusent L3d. Dépendance : garde L3d auto-testée ; G-LOG avant toute recette GPU. Outline : accepter uniquement la racine L3d canonique dans ces helpers, sans affaiblir L3b/L3c ; ajouter un mode `--lib` ciblé et fail-closed qui exige **un test exécuté** ; adapter le test R5 existant pour exercer L3d sous la feature privée tout en gardant ses assertions L3c ; pour la recette historique, enlever `PARAKEET_L3D_EXEC_ROOT` et `PARAKEET_L3C_EXEC_ROOT`, admettre seulement la création/écriture de `parakeet-asr-*.log` sous `l3b/logs/` après accord G-LOG, toute autre écriture L3b restant FAIL.
- Commandes : `cargo test --offline --locked -p whisper-adapters --features l3-gpu-qualification --lib worker_ipc::tests::l3c_acceptance_fault_is_confined_to_its_canonical_execution_root -- --exact --nocapture`, avec `PARAKEET_L2_ACCEPTANCE_TEST_BUILD=1` posé par la garde avant Cargo ; recette L3b `cargo test --offline --locked -p whisper-adapters --test parakeet_import gpu_app_fr_en_single_pass -- --exact --nocapture`, avec le sélecteur historique `PARAKEET_L3B_RECIPE_CASE=gpu_app_fr_en_single_pass`. Provenance : code `worker_ipc.rs`, `tests/parakeet_import.rs` et plan L3c ; aucun zéro test ne vaut PASS.
- Acceptation : test R5 exécuté exactement une fois et PASS, avec témoin négatif L3a existant en lecture seule ; recette L3b réellement exécutée PASS, journaux dans le motif autorisé. Les 9 auto-tests hérités et 3 témoins de l'exception (nom L3d autorisé, autre écriture L3b refusée, préfixe historique autorisé seulement en mode historique) sont conformes après chaque changement du script, sans toucher aux vrais dossiers protégés. Repli : laisser NOT RUN et arrêter la qualification si G-LOG n'est pas accordé ; ne pas déplacer le journal ni modifier le worker figé.

### D4 — recette et revue finales

- Priorité : haute. Source : L3C-R2 et prévention des régressions. Propriétaire : validation. Chemins : tests déjà cités, `docs/cadrage/fiche-parakeet-l3d-latence.md` et `docs/cadrage/bilan-parakeet-l3d-latence.md` **uniquement après PASS et CLEAN** ; sorties sous L3d et Cargo partagé.
- Budget : aucune nouvelle source. Dépendances : D1–D3. Outline : exécuter trois **processus/sessions neufs** de la recette EN5 release, chacun avec qualification SHA du GPU par le catalogue en arrière-plan **sans inférence préalable**, puis premier et second import dans la même session ; chronomètre `StartImport` → TXT/SRT durables ; second import sans relecture ; rejouer A4, EN30, cinq R8, tests core et suite workspace ; revue indépendante du code, deux cycles de correction maximum.
- Commandes : `cargo fmt --all -- --check`, `cargo build --offline --locked --workspace`, `cargo test --offline --locked --workspace --no-fail-fast`, recettes ciblées L3c du plan v3 transposées sous garde L3d ; provenance : plan L3c/AGENTS.md. Aucun `cargo fmt` écrivant ne s'exécute dans une campagne gardée ; `rustfmt` direct sur fichiers autorisés peut précéder le check.
- Acceptation : les trois premiers imports ≤15 s chacun, TXT/SRT, worker/DLL/modèles SHA inchangés, zéro nouveau SHA adaptateur au deuxième, aucun SHA sur fil UI, 0 FAIL workspace, NOT RUN listés et justifiés ; reviewer CLEAN. Repli : `LIVRÉ AVEC RÉSERVES`, pas de nouvelle fiche qualifiée ni de commit produit si une carte bloque.

## 7. Matrice de vérification

| Zone | Commande ou contrôle | Provenance | Attendu |
| --- | --- | --- | --- |
| Garde | 9 auto-tests hérités + 3 témoins de l'exception L3d, contrôles SHA avant/après, watcher et racines canoniques | Plan L3c et garde privée observée ; 3 témoins proposés | 12/12, zéro déviation |
| Core | `cargo test --offline --locked -p whisper-core --test import_contract` | Source inférée du test existant | PASS, rafales et transitions couvertes |
| Adaptateur R5 | `cargo test --offline --locked -p whisper-adapters --features l3-gpu-qualification --lib worker_ipc::tests::l3c_acceptance_fault_is_confined_to_its_canonical_execution_root -- --exact --nocapture`, avec `PARAKEET_L2_ACCEPTANCE_TEST_BUILD=1` | Source `worker_ipc.rs:6290,8371` | 1 test exécuté PASS |
| Produit release | `cargo test --release --offline --locked -p whisper-adapters --test parakeet_import gpu_app_en5_twice_release -- --exact --nocapture` | Plan L3c | 3 premiers imports ≤15 s, TXT/SRT, zéro relecture au second |
| Régressions | A4, EN30 et cinq R8 selon les sélecteurs du plan L3c | Plan L3c | Tous exécutés PASS |
| Workspace | `cargo fmt --all -- --check` ; `cargo build --offline --locked --workspace` ; `cargo test --offline --locked --workspace --no-fail-fast` | AGENTS.md / plan L3c | Code 0, tous les ignorés listés |
| Interface | Recette manuelle Windows de la première transcription GPU | Validation produit utilisateur | À effectuer par l'utilisateur après livraison CLEAN |

Toutes les commandes Cargo, builds et recettes passent par la garde L3d ; `CARGO_TARGET_DIR=target/parakeet-tests/cargo-shared/`, TEMP/TMP et sorties sous L3d. Les workers CPU/GPU, DLL, modèles, `sound example/`, spike02 et les fixtures R8 existantes sont en lecture seule. Une écriture observée hors tolérance ou un SHA divergent arrête la campagne. Aucun nettoyage ni push.

## 8. Gates de décision

| Gate | Statut | Bloque | Décision recommandée | Preuve |
| --- | --- | --- | --- | --- |
| G-BASE | Blocking | Tout code L3d | Fixer par décision utilisateur le sort Git du candidat L3c non commité ; ne pas mêler les diffs | `git status`, hash/diff L3c figé |
| G-LOG | Blocking pour les recettes GPU D1–D4 | Journal ASR exigé sous la racine du worker figé | Autoriser uniquement `target/parakeet-tests/l3/l3b/logs/l3d-parakeet-asr-*.log` pour L3d et `.../l3b/logs/parakeet-asr-*.log` pour la recette historique, en création/écriture ; autre L3b interdit | garde + SHA avant/après |
| G-CAUSE | Validation | D2 | Confirmer la part de latence des `Refresh` avant d'éditer la cadence | tableau D1 |

Ces gates seront soumis avec le plan. L'accord sur ce **cadrage** ne vaut pas accord d'implémentation, d'écriture L3b ou de qualification du candidat L3c.

## 9. Hors périmètre

Worker natif/ABI, DLL, modèle, code CUDA, VAD, micro Parakeet, Turbo, seuil EN5 différent de 15 s, cache SHA persistant, modification de `Cargo.lock`, réseau, push et installation L6-perso. Aucune réécriture de la durabilité d'archive sans nouvelle preuve et nouveau plan.

## 10. Contrôle de qualité avant transmission

| Contrôle | État |
| --- | --- |
| Sections 1–11, tâches bornées et traces des constats | PASS — revue indépendante cycle 2 CLEAN |
| Chemins existants, commandes observées ou indiquées comme inférées | PASS — revue indépendante cycle 2 CLEAN |
| Budget : quatre tâches, moins de dix fichiers de rapport | PASS |
| Aucune édition de code dans ce cadrage | PASS |

## 11. Handoff d'implémentation proposé

Après acceptation du plan versionné et accord écrit distinct : confier D1 à `rust_implementer` en mode IMPLEMENT avec le hash de base L3c stabilisé, les seuls chemins autorisés, les règles de garde et les seuils ci-dessus. Préserver tout changement non lié. Faire relire le candidat par `rust_review_coordinator` et `rust_reviewer` indépendant, deux cycles revue/correction maximum. Aucun lot suivant n'est autorisé par ce handoff documentaire.
