Avancée : workspace corrigé validé, 73 cibles atteintes ; bilan consolidé 773 réussis, 0 échec, 66 ignorés avec D2 réutilisé.
Blocage : aucun pour le périmètre exécuté ; scénarios NOT RUN et tests ignored restent non qualifiés.
Décision : PASS avec réutilisation D2 autorisée, sur 88850e8 + correction locale non commitée ; garde inchangée.

# Tests de niveau 1 — 88850e8 et correction locale autorisée

## Verdict courant — après correction locale

**PASS avec réutilisation D2 autorisée**, sur HEAD `88850e8afd45e3b4c827914fad4c55a796b479db` **plus les deux attendus corrigés non commités** dans `crates/whisper-core/tests/parakeet_import_contract.rs:224-239`. Run workspace unique `20261031T224038228-3c26d9fe`, code **0**, 279 secondes : **772 passed, 0 failed, 66 ignored, 1 filtered**. Le seul test filtré est D2, expressément remplacé par sa preuve native précédente : **1 test réussi couvrant 16 cas**. Bilan consolidé : **773 passed, 0 failed, 66 ignored**. Les autres tentatives et le test ciblé ne sont pas additionnés.

Les **73 cibles test/doctest attendues** sont atteintes sur les huit crates ; aucune manquante ni supplémentaire. [Résultats détaillés et limites](raw/niveau1-post-fix-results.json), [couverture](raw/niveau1-post-fix-coverage.json), [état après exécution](raw/niveau1-post-fix-after.json). Ce résultat ne transforme pas le commit 88850e8 sans correction en commit validé : ses échecs restent ci-dessous en historique.

## Verdict historique — 88850e8 sans correction

**FAILED sur 88850e8** : 772 résultats passed du harness, 1 failed et 66 ignored, consolidés sans doublonner les reprises. Les 73 cibles de tests/doctests attendues ont été atteintes sur les huit crates, et les 16 cas D2 ont réellement été exécutés. Des retours anticipés NOT RUN restent signalés dans les journaux ; les chiffres passed ne prouvent pas ces branches. L’unique échec observé est le contrat de clé de worker personnel dans whisper-core, `crates/whisper-core/tests/parakeet_import_contract.rs:224-227` ; `crates/whisper-core/src/model_registry.rs:52-53`.

[Bilan consolidé](raw/niveau1-remainder-results.json), [couverture des cibles](raw/niveau1-target-coverage.json), [preuves D2](raw/niveau1-retry-d2-checkpoint.jsonl). Les sections suivantes conservent les tentatives et leurs résultats historiques ; la dernière section détaille la poursuite finale.

## Cible et périmètre confirmé

SHA complet : `88850e8afd45e3b4c827914fad4c55a796b479db`. Le périmètre demandé et confirmé dans cette session est le workspace complet, tests non ignorés, features par défaut, sans filtre et sans ajout de `--ignored`.

Commande réellement lancée depuis `C:/dev/whisper`, par la garde Parakeet existante et inchangée :

```text
cargo test --offline --locked --workspace -- --test-threads=1 --nocapture
```

Sources et tests suivis identiques au commit avant et après la commande : diff HEAD vide sur crates/, scripts/, Cargo.toml, Cargo.lock, .cargo/ et rust-toolchain.toml ; aucune source non suivie dans ces arborescences. [Préflight](raw/niveau1-preflight.json), [état après exécution](raw/niveau1-after.json). Les modèles/corpus préexistants non suivis ne sont pas présentés comme des fichiers du commit.

Les racines d’écriture autorisées sont refacto-parakeet-live/ pour cache, temporaires, fixtures et journaux, cargo-shared/ pour la compilation et docs/architecture/ pour les sorties documentaires. `WHISPER_L1B_RUNTIME_ROOT` a été placé sous la racine d’exécution pour permettre les tests locaux de catalogue/préférences. Aucun code ni test ni garde modifié.

## Première tentative — résultat partiel historique

**INCOMPLETE — échecs de prérequis d’environnement, pas de confirmation que la suite passe.** La compilation a terminé en 43,79 s. Avant l’arrêt du premier binaire (tests unitaires whisper-adapters), 224 cas ont affiché ok, 8 FAILED et 4 ignored. Ces comptes sont partiels : les autres binaires/crates et doctests n’ont pas de verdict dans cette exécution. [Stdout brut](raw/niveau1-workspace.stdout.txt), [stderr brut](raw/niveau1-workspace.stderr.txt), [résumé structuré](raw/niveau1-results.json).

Après observation de FAILED, le processus Cargo et ses descendants ont été arrêtés explicitement. La garde a enregistré un code de sortie -1, durée 75 s ; **ce -1 ne représente pas une terminaison normale de Cargo avec un bilan complet**. Plusieurs échecs étaient déjà intervenus dans le même binaire avant leur détection dans les sorties. Aucun `test result:` final n’a été produit. [Exécution gardée](raw/niveau1-guard-run.json).

## Causes des huit échecs

| Groupe | Cas échoués observés | Précondition absente et source |
| --- | --- | --- |
| D2 | d2_corpus_real_cpu_turbo_and_parakeet ; test_status_writer_preserves_diagnostics | WHISPER_MIC_LEVEL_D2_ROOT exigée par expect : `crates/whisper-adapters/src/direct_live/d2_corpus.rs:143-147` ; `crates/whisper-adapters/src/direct_live/d2_corpus.rs:541-543` |
| Sources | recovery_refuses_reserved_operation_while_acquire_holds_the_cross_process_lock ; retirement_intent_replays_after_each_disposition_boundary ; snapshot_v2_preserves_strategy_and_rejects_request_mismatch | PARAKEET_L2_ARCHIVE_ROOT exigée par le helper : `crates/whisper-adapters/src/source_binding.rs:1524-1533` |
| RTC-02 | browse_accepts_only_exact_verified_treatment_in_current_destination ; emptied_transcriptions_keeps_history_but_offers_no_duplicate_targets | PARAKEET_RTC02_EXEC_ROOT exigée par le helper : `crates/whisper-adapters/src/worker_ipc/tests/reprocessing_and_transport.rs:19-29` |
| MP3 | import_application_decodes_real_mp3_and_publishes_full_unified_coverage | WHISPER_PU_L1_MP3_FIXTURE exigée par expect ; taille et SHA attendus imposés : `crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:1055-1070` |

Les messages de panic sont conservés dans le stderr brut. [Déduction] Le classement ENVIRONMENT vient de ces expect sur variables absentes, avant les assertions fonctionnelles concernées ; il ne prouve pas que les mêmes scénarios réussiront une fois configurés.

## Limites et prérequis d’une reprise

Le test D2 non ignoré exige aussi `WHISPER_MIC_LEVEL_D2_ENGINE=parakeet-cpu`, l’installation personnelle et quatre corpus/références dont il vérifie les empreintes. Il parcourt quatre corpus × quatre conditions : **16 cas natifs**, au-delà d’une suite purement synthétique. `crates/whisper-adapters/src/direct_live/d2_corpus.rs:19-52` ; `crates/whisper-adapters/src/direct_live/d2_corpus.rs:148-205`. [Non vérifié] Ce scénario natif n’a pas été exécuté : il a échoué sur sa première variable.

Pour une reprise sans toucher les tests : configurer les racines D2/archive/RTC-02 sous refacto-parakeet-live/niveau1-88850e8/, configurer le moteur D2 prescrit, vérifier le corpus et l’installation en lecture, puis pointer la fixture MP3 approuvée existante sous refacto-parakeet-live/retry-eight/fixtures/pu-l1-input.mp3 après vérification de ses octets attendus. À la fin de cette première tentative, ces réglages n’avaient pas encore été appliqués ; les reprises ultérieures et leurs autorisations sont décrites plus bas.

Un premier auto-test de la garde a été interrompu sur Changed de docs/architecture/raw ; la relance séparée avec garde inchangée a réussi. L’exécution des tests rapportée ici a ensuite démarré normalement. [Sortie initiale](raw/niveau1-selftest-initial.txt), [événement](raw/niveau1-selftest-event.json), [auto-tests réussis](raw/niveau1-selftest-passed.txt).

La cartographie et le succès Clippy antérieurs ne remplacent pas ce résultat de tests. La preuve complète demandée reste à obtenir ; aucune exclusion, modification d’assertion, activation de --ignored ou transformation d’un échec en PASS n’a été faite.

## Reprise configurée — 31 octobre 2026, 21:58–22:16

Après autorisation explicite, les corpus/références D2 et la fixture MP3 ont été vérifiés. Les cinq DLL CPU et le worker Parakeet correspondent aux empreintes du registre courant ; les modèles correspondent aux tailles et SHA attendus. L’installation personnelle réelle était absente : des copies de ressources ont été préparées exclusivement sous refacto-parakeet-live/niveau1-88850e8/retry-20261031T215723/localappdata/Whisper. Cargo a fourni ce LOCALAPPDATA aux processus de test via deux options `--config` (valeur et force=true), sans modifier celui de la garde. Les racines D2/archive/RTC-02/L1B ont été configurées sous la même racine autorisée. [Préparation et empreintes](raw/niveau1-retry-preflight.json).

La garde est inchangée (SHA avant/après identique), ses auto-tests ont réussi. La commande workspace non filtrée a compilé en 0,68 s puis terminé naturellement avec **code 101**, durée gardée **1 079 s**. Le premier binaire, whisper-adapters, a rendu : **306 passed, 1 failed, 16 ignored, 0 filtered out**. Cargo n’a pas exécuté les autres binaires après cet échec. [Commande et garde](raw/niveau1-retry-guard-run.json), [stdout](raw/niveau1-retry-workspace.stdout.txt), [stderr](raw/niveau1-retry-workspace.stderr.txt), [résultats](raw/niveau1-retry-results.json).

### D2 : preuve conservée

Le test `direct_live::d2_corpus::d2_corpus_real_cpu_turbo_and_parakeet` est marqué **ok**. Les 16 événements case-complete, numérotés de 1 à 16, et campaign-complete sont présents. Pour chacun, input_frames = output_frames et les assertions sur révisions/gains et exports ont été franchies. [Checkpoint brut](raw/niveau1-retry-d2-checkpoint.jsonl), [récapitulatif des cas](raw/niveau1-retry-d2-summary.json). Le code mesuré est celui de `crates/whisper-adapters/src/direct_live/d2_corpus.rs:143-282`.

Limites : le moteur natif D2 est la ressource CPU figée a965c950… exigée par le registre, pas une reconstruction du worker courant ; les adaptateurs/tests sont ceux de 88850e8. Les 16 statuts moteur enregistrent complete=false et les exports portent le suffixe incomplet. Le test n’exige pas la complétude de la transcription ni un seuil WER ; son succès ne vaut pas recette produit complète. `crates/whisper-adapters/src/direct_live/d2_corpus.rs:237-273` ; [statuts conservés](raw/niveau1-retry-d2-summary.json).

### Échec restant : modèle L1B absent

`worker_ipc::tests::resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` appelle `fs::hard_link(&model, &legacy_model).unwrap()` avec model = WHISPER_L1B_RUNTIME_ROOT/models/ggml-large-v3-turbo.bin. `crates/whisper-adapters/src/worker_ipc/tests/resume_and_admission.rs:1156-1180`. Cette seconde copie n’avait pas été préparée : NotFound, code Windows 3, à la ligne 1180, avant les assertions de reprise. L’audio spkr0.wav est présent. [Contrôle après exécution](raw/niveau1-retry-after.json). [Déduction] Échec de préparation ENVIRONMENT ; la réussite après correction n’est pas déduite.

Les sources, tests, manifestes et Cargo.lock restent identiques au commit 88850e8 ; aucune source non suivie dans crates/scripts/. Les deux retours NOT RUN liés aux variables historiques L2 sont conservés dans le stderr et le résumé, distincts du failed et des ignored. Les résultats ne permettent toujours pas de confirmer le workspace entier.

Préparation proposée : copier le modèle Turbo déjà vérifié dans le dossier L1B/models sous la même racine, exécuter le test échoué, puis poursuivre selon l’arbitrage utilisateur. Réutiliser la preuve D2 ou rejouer les 16 cas constitue la décision de périmètre soumise dans cette session. À ce stade, aucun test ni garde n’était à modifier et aucune reprise n’avait été effectuée après ce premier code 101 ; la décision ultérieure figure ci-dessous.

## Poursuite historique autorisée avec réutilisation de D2

L’utilisateur a autorisé la copie supplémentaire du modèle sous le dossier L1B existant, la vérification du test échoué puis la poursuite du workspace en réutilisant la preuve D2. `ggml-large-v3-turbo.bin` a été copié depuis models/ vers WHISPER_L1B_RUNTIME_ROOT/models/, avec SHA-256 vérifié `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`. Aucune source, assertion, feature ou garde modifiée. [Préflight de poursuite](raw/niveau1-completion-preflight.json).

Les auto-tests de la garde inchangée ont réussi. Le contrôle exact de `worker_ipc::tests::resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` a terminé avec **code 0 : 1 passed, 0 failed, 0 ignored, 322 filtered out**, en 50,70 s de test, 78 s gardées avec compilation. L’ancien défaut de préparation est donc fermé par une exécution réelle, pas par une modification du test. [Stdout ciblé](raw/niveau1-focused-fixed.stdout.txt), [stderr ciblé](raw/niveau1-focused-fixed.stderr.txt), [enregistrements gardés](raw/niveau1-completion-guard-runs.json).

### Commande et couverture de poursuite

```text
cargo test --offline --locked --workspace
  --config env.LOCALAPPDATA.value='<LOCALAPPDATA de test>'
  --config env.LOCALAPPDATA.force=true
  -- --test-threads=1 --nocapture
  --skip direct_live::d2_corpus::d2_corpus_real_cpu_turbo_and_parakeet
```

La commande exacte et les chemins sont conservés dans [niveau1-completion-guard-runs.json](raw/niveau1-completion-guard-runs.json). Le seul test filtré est le D2 natif expressément autorisé pour réutilisation ; ses 16 cas réussis proviennent du même commit et des mêmes ressources natives vérifiées. Le test ciblé de reprise est réexécuté dans cette poursuite et n’est pas ajouté une deuxième fois aux comptes.

La poursuite a terminé naturellement avec **code 101** après **245 s** : 46 binaires de tests ont rendu un bilan, **583 passed, 1 failed, 49 ignored, 1 filtered out**. Les champs running/summary sont rapprochés dans l’ordre du flux Cargo ; les 46 libellés correspondent aux 46 bilans. [Résultat structuré](raw/niveau1-completion-results.json), [stdout complet](raw/niveau1-completion-workspace.stdout.txt), [stderr complet](raw/niveau1-completion-workspace.stderr.txt).

Les tests des adaptateurs ont passé dans cette poursuite ; Cargo a ensuite exécuté bootstrap et une partie du cœur. L’arrêt intervient dans l’intégration core parakeet_import_contract, qui rend **5 passed, 1 failed**. Les cibles suivantes, notamment desktop, workers et doctests, ne sont pas validées par cette poursuite. Les 22 mentions NOT RUN du stderr restent des retours de scénarios optionnels, distincts des ignored et du filtered D2 ; les passed du harness ne sont pas une preuve d’exécution de ces branches anticipées.

### Échec de contrat constaté

Test : `personal_release_keys_match_the_qualified_candidate_b_workers_and_dll_sets`.

| Valeur | Attendu par le test | Fourni par le registre | Preuve |
| --- | --- | --- | --- |
| Worker CPU | 84c371d4df55043ada4a1d96b80180ed0c03b208410c599262c8ea3762304e60 | a965c9503f909124f8ddedad09e83e5d187e70edd1869e880e9f1ad5ae2fb99d | Échec exécuté : `crates/whisper-core/tests/parakeet_import_contract.rs:224-227` ; déclaration : `crates/whisper-core/src/model_registry.rs:52-53` |
| Worker GPU | bf58eb9d7300c444864347496b5a27e8584da3d30d634a759be99a598eda5ce4 | 7ffb0b4a022ec26909a8939b37787de6df6c9bdb2903edf14d3ffb32579eefc1 | Différence statique : `crates/whisper-core/tests/parakeet_import_contract.rs:236-239` ; `crates/whisper-core/src/model_registry.rs:56-57` ; assertion GPU non atteinte après le panic CPU |

Ce test construit les clés depuis le registre et compare des valeurs littérales ; il ne charge pas de modèle et ne dépend pas des variables de préparation de cette campagne. `crates/whisper-core/tests/parakeet_import_contract.rs:216-245`. L’échec actuel est donc une incohérence entre le contrat attendu et le code courant, distincte des prérequis manquants des premières tentatives. Aucune conclusion n’est prise ici sur la valeur qui devrait être la référence.

### Intégrité, conservation et verdict

SHA final : `88850e8afd45e3b4c827914fad4c55a796b479db`. Diff HEAD vide sur sources, tests, manifestes, lockfile, configuration et scripts ; aucune source non suivie dans les chemins contrôlés. La garde conserve l’empreinte du préflight. [État final](raw/niveau1-completion-after.json). Espace libre final : **183,93 Gio**. Les copies de modèles et sorties sont conservées, sans nettoyage.

**Verdict : FAILED — la suite de niveau 1 ne passe pas sur 88850e8 dans le périmètre testé.** Les 16 cas D2 et le test ciblé de reprise ont réussi, mais ils ne remplacent pas l’échec de contrat ni les cibles non exécutées. Aucun changement du code ou des tests, aucune exclusion supplémentaire et aucune relance après ce nouvel échec.

## Cibles restantes — poursuite finale autorisée

Après la demande « continue », seules les cibles qui n’avaient pas encore été exécutées ont été lancées, avec les mêmes ressources et variables préparées, la garde inchangée et le SHA 88850e8 confirmé. L’échec de contrat précédent n’a été ni relancé jusqu’à réussite, ni exclu du verdict global. Les auto-tests de la garde ont réussi après une première interruption sur Changed de docs/architecture/raw ; les deux sorties sont conservées.

| Périmètre restant | Commande de sélection (options communes offline/locked/config conservées) | Résultat |
| --- | --- | --- |
| Core, trois intégrations restantes | `cargo test -p whisper-core --test parakeet_ipc --test parakeet_windows --test scheduler_contract -- --test-threads=1 --nocapture` | code 0 ; 15 passed, 0 failed, 0 ignored ; 3 cibles |
| Desktop et quatre packages workers | `cargo test --workspace --exclude whisper-adapters --exclude whisper-bootstrap --exclude whisper-core -- --test-threads=1 --nocapture` | code 0 ; 173 passed, 0 failed, 17 ignored ; 22 cibles, doctests desktop/common compris |
| Doctests core/adapters | `cargo test -p whisper-core -p whisper-adapters --doc -- --test-threads=1 --nocapture` | code 0 ; deux cibles, aucun exemple de test présent (0 exécuté) |

Les tableaux de commandes ci-dessus sont abrégés pour lecture ; arguments exacts, racines LOCALAPPDATA et codes figurent dans [les exécutions gardées](raw/niveau1-remainder-guard-runs.json). Sorties : [core stdout](raw/niveau1-remainder-core.stdout.txt), [core stderr](raw/niveau1-remainder-core.stderr.txt), [desktop/workers stdout](raw/niveau1-remainder-desktop-workers.stdout.txt), [desktop/workers stderr](raw/niveau1-remainder-desktop-workers.stderr.txt), [doctests stdout](raw/niveau1-remainder-doc.stdout.txt), [doctests stderr](raw/niveau1-remainder-doc.stderr.txt).

### Consolidation sans double comptage

- Poursuite workspace après correction L1B : 583 passed, 1 failed, 49 ignored ; un test D2 filtré avec accord explicite.
- Cibles restantes : 188 passed, 0 failed, 17 ignored.
- Test D2 réutilisé : 1 passed, comportant les 16 cas natifs ; il n’est ajouté qu’une fois.
- Test ciblé de reprise réussi : non additionné car il a également passé dans les 583 résultats de la poursuite.
- Anciennes tentatives échouées/interrompues : conservées en historique, non additionnées au bilan consolidé.

**Total consolidé : 772 passed, 1 failed, 66 ignored.** [Calcul détaillé](raw/niveau1-remainder-results.json). Ce bilan provient de plusieurs commandes sur le même SHA ; il n’est pas présenté comme un unique `cargo test --workspace` vert.

La comparaison multiensemble des cibles Cargo test/doctest attendues avec les libellés d’exécution retrouve **73 attendues, 73 observées, aucune manquante ni supplémentaire**. Les homonymes sont comptés avec leur multiplicité ; les groupes de commandes documentent les packages sélectionnés. [Contrôle de couverture](raw/niveau1-target-coverage.json). Ce contrôle porte sur les cibles ; il ne prouve pas l’exécution des branches qui retournent NOT RUN.

### Clôture

La poursuite des cibles restantes n’a révélé **aucun nouvel échec**. Le contrat de clé personnelle reste l’échec unique et le verdict global reste **FAILED**. Toutes les sources, tous les tests, les manifestes et Cargo.lock sont identiques à 88850e8, et la garde conserve son empreinte du préflight. [État final](raw/niveau1-remainder-after.json). Espace libre final : **183,82 Gio**. Copies, sorties et preuves conservées ; aucun nettoyage des modèles, WAV ou du cargo-shared.

[Non vérifié] Les autres combinaisons de features, les tests ignored, les branches NOT RUN, une recette graphique Windows et une qualification générale de qualité/vitesse restent hors de cette preuve. Le résultat D2 demeure limité aux assertions réellement présentes, avec les limites de complétude/WER documentées plus haut.

## Correction locale ultérieure du contrat Parakeet — 31 octobre 2026

Avancée : les deux attendus worker CPU/GPU du test ont été actualisés avec autorisation explicite ; test d’intégration complet réussi.
Blocage : aucun sur ce correctif ; le bilan historique de 88850e8 ci-dessus reste FAILED.
Décision : correction locale non commitée, revue indépendante CLEAN ; pas de nouvelle qualification du workspace complet.

Seul fichier suivi modifié : `crates/whisper-core/tests/parakeet_import_contract.rs:224-239`, exactement deux littéraux SHA-256. CPU = `a965c9503f909124f8ddedad09e83e5d187e70edd1869e880e9f1ad5ae2fb99d`, GPU = `7ffb0b4a022ec26909a8939b37787de6df6c9bdb2903edf14d3ffb32579eefc1`. Ces attendus correspondent au registre, `crates/whisper-core/src/model_registry.rs:49-57`, au bilan de qualification `docs/cadrage/plan-nettoyage-environnements-historiques.md:257-275` et aux empreintes des copies figées recalculées avant correction. Le commit 778ef0bc avait changé le registre sans actualiser les attendus introduits par 0e610c95. Le registre de production et les identités historiques restent inchangés.

Validation sur HEAD 88850e8 **plus le diff de deux lignes** : `cargo test --offline --locked -p whisper-core --test parakeet_import_contract -- --test-threads=1 --nocapture`, via la garde existante inchangée. Run `20261031T223806530-b29780f0` : code 0, **6 passed, 0 failed, 0 ignored, 0 filtered**. Avertissement existant `kind_name` inutilisé. Auto-tests de garde réussis avec PowerShell 7 ; une invocation préalable avec Windows PowerShell a échoué faute de pwsh.exe sous son PSHOME, sans Cargo lancé, puis une interruption sur Deleted `.git/objects/pack/tmp_pack_BuKqL0` est conservée dans le journal de garde. Reprise conforme à l’autorisation de retenter avec la garde inchangée malgré les événements extérieurs.

L’implémenteur dédié a effectué la correction ; le reviewer indépendant, via rust_review_coordinator, a rendu **CLEAN**, zéro constat, un passage. `git diff --check` réussi ; aucun commit ni push. La garde conserve SHA-256 `0f8f692456aba2c9884f080cd38f1a5e0be1c5cb0ac2f85d7f4f82685f009323`. Espace libre final 183,81 Gio. Les deux journaux du run sont archivés dans `target/parakeet-tests/refacto-parakeet-live/niveau1-88850e8/parakeet-contract-fix-evidence.zip`, arborescence préservée, deux entrées relues et SHA-256 comparés aux originaux ; SHA-256 de l’archive `11722a8c25b50d4e4e2f30d1a5fe638667817553e8bcd46e4b15f2256c26730e`. Aucun nettoyage.

[Non vérifié] Le workspace complet n’a pas été relancé sur ce diff ; les résultats précédents sur 88850e8 ne sont pas transformés en un PASS du candidat corrigé.

## Relance complète après correction — 31 octobre 2026

Périmètre : workspace entier, features par défaut, tests non ignorés, garde et racines d’écriture inchangées. Seule différence de sources par rapport à 88850e8 : les deux littéraux attendus du contrat, `crates/whisper-core/tests/parakeet_import_contract.rs:224-239`. Le registre de production et le code D2 sont inchangés ; la réutilisation de la preuve D2 est explicitement autorisée. Le hash du test, le diff des sources, HEAD et la garde restent identiques entre le préflight et la clôture. [Préflight](raw/niveau1-post-fix-preflight.json), [état final](raw/niveau1-post-fix-after.json).

Commande exacte (variables de tests réutilisées depuis le [préflight de configuration](raw/niveau1-completion-preflight.json)) :

```text
cargo test --offline --locked --workspace --config "env.LOCALAPPDATA.value='C:/dev/whisper/target/parakeet-tests/refacto-parakeet-live/niveau1-88850e8/retry-20261031T215723/localappdata'" --config env.LOCALAPPDATA.force=true -- --test-threads=1 --nocapture --skip direct_live::d2_corpus::d2_corpus_real_cpu_turbo_and_parakeet
```

La commande a été exécutée uniquement par la garde existante. Auto-tests réussis avant chaque tentative. La première tentative `20261031T223949813-77486dc7` a été interrompue sur Changed de `.git/objects/pack/tmp_pack_eZ8TZo`, événement horodaté `2026-10-31T22:40:03.5692101+01:00`, origine non établie. Elle est conservée et n’est pas comptée comme succès. La reprise, autorisée avec garde inchangée malgré les événements extérieurs, a terminé normalement. [Deux enregistrements gardés](raw/niveau1-post-fix-guard-runs.json), [auto-tests initiaux](raw/niveau1-post-fix-selftest.txt), [auto-tests avant reprise](raw/niveau1-post-fix-retry-selftest.txt), [stdout interrompu](raw/niveau1-post-fix-interrupted.stdout.txt), [stderr interrompu](raw/niveau1-post-fix-interrupted.stderr.txt).

Run final : **code 0**, **772 passed, 0 failed, 66 ignored, 1 filtered**, 279 secondes ; le contrat Parakeet corrigé est réussi. **73/73 cibles test/doctest**, dont quatre cibles doctest sans exemple exécuté ; ce contrôle compte les homonymes avec leur multiplicité et ne prouve pas les branches internes. [Stdout complet](raw/niveau1-post-fix-workspace.stdout.txt), [stderr complet](raw/niveau1-post-fix-workspace.stderr.txt), [résultats par cible](raw/niveau1-post-fix-results.json).

D2 réutilisé : run `20261031T215801649-2f4a5577`, **16 cas natifs réussis** dans un test. Ajouter ce seul résultat au run workspace donne **773 réussis, aucun échec, 66 ignorés**. Les limites D2 exposées plus haut, dont les statuts complete=false et l’absence de seuil WER, sont conservées. [Résumé natif](raw/niveau1-retry-d2-summary.json).

[Non vérifié] **25 mentions NOT RUN** apparaissent dans les sorties du run final : ce sont des messages, pas un décompte de tests supplémentaires ni de scénarios distincts. Les tests retournant ainsi peuvent être classés passed par le harness sans avoir exercé leur branche native. Les 66 tests ignored ne sont pas forcés, les autres features et la recette dans la fenêtre Windows ne sont pas qualifiées. Les panics attendus des scénarios de délai restent visibles avec nocapture ; tous les bilans de harness et le code final sont réussis. [Messages exacts](raw/niveau1-post-fix-results.json).

La garde garde son SHA-256 `0f8f692456aba2c9884f080cd38f1a5e0be1c5cb0ac2f85d7f4f82685f009323`. Espace libre au préflight : 183,68 Gio ; après exécution : 183,77 Gio. La variation n’est pas attribuée à cette seule campagne. Aucune modification supplémentaire de code, des tests ou de la garde ; aucune installation, suppression, commit ni push. Les documents de bilan et les sorties brutes sont actualisés sous docs/architecture/.

Petites preuves archivées sous `target\parakeet-tests\refacto-parakeet-live\niveau1-88850e8\post-fix-workspace\evidence.zip`, 13 fichiers, arborescence préservée et contenu relu puis comparé par SHA-256 aux originaux. [Contrôle de l’archive](raw/niveau1-post-fix-archive-check.json). Aucun nettoyage des ressources ou du cargo-shared.
