Avancée : les cinq tests R8 existent déjà et la suite workspace passe après nettoyage.
Blocage : le plan L2a-2-ACCEPT fige le worker `545a…`, refusé pour un nouvel import ; ses fixtures écrivent hors d'une racine de lot dédiée.
Décision : accepter ou refuser le remplacement de cette clé par la clé Test R6 `1d8f…` pour la seule reprise R8.

# Proposition de reprise du R8 historique — 17 octobre 2026

**Statut : v2 accepté et exécuté ; candidat R8 revu CLEAN après deux cycles, non commité.** Elle amende seulement F3/R8 de `plan-parakeet-l2a2-acceptance.md`. R6 et R7 de ce plan, la fiche de qualification Test, le produit et le worker restent inchangés.

## Motif et périmètre

Le plan historique (lignes 7, 11 et 19) associe R8 au worker `545a4830…` et refuse explicitement de conclure sur une autre clé. `model_registry.rs` associe aujourd'hui le profil Test au worker R6 `1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0`. Le test `parakeet_import.rs` contient déjà les cinq cas de DLL incompatible, mort du worker, trame incomplète et leurs deux traversées du parcours applicatif (lignes 403, 556, 583, 614 et 620). Les cas de DLL incompatible, de trame incomplète et de route produit dérivent actuellement leurs sorties de `PARAKEET_L2_RUNTIME_ROOT` (lignes 408–410, 585–586, 628–631), qui doit rester exactement la racine `…/l2b` pour l'admission du modèle partagé (`parakeet_import.rs`, lignes 28–68). Leurs écritures doivent aller sous `…/l2b/r8`.

La qualification déjà acquise de `1d8f…` n'est pas transposée rétroactivement à `545a…` ; R8 teste seulement les refus du produit sur la **clé actuellement admise**. Si l'utilisateur exige la clé historique exacte pour R8, cette proposition est refusée et R8 reste `NOT RUN` dans le produit actuel.

## Lot proposé

1. Préflight en lecture seule : branche/HEAD/diff, SHA du worker R6, du modèle et des cinq DLL, fixtures EN, racine canonique `…/l2b`, espace libre supérieur ou égal à 30 Gio. Tester la garde en cas OK et KO avant Cargo.
2. Modifier uniquement `crates/whisper-adapters/tests/parakeet_import.rs` : introduire `PARAKEET_R8_EXEC_ROOT=…/l2b/r8` et y diriger les copies DLL incompatibles, la source et l'exécutable du faux worker, les archives, pending, TXT/SRT et marqueurs d'injection. Garder `PARAKEET_L2_RUNTIME_ROOT=…/l2b` comme ancre de lecture et conserver les cinq assertions de refus existantes. Ne pas réutiliser ni modifier les dossiers `l2/l2a-2/native/r8-incompatible-*`.
3. Exécuter les cinq tests ignorés individuellement, avec `PARAKEET_L2_ACCEPTANCE_TEST_BUILD=1` au moment de la compilation des tests, puis les régressions pertinentes et `cargo test --offline --locked --workspace --no-fail-fast`. Vérifier pour chaque cas la cause et la phase, l'absence de repli Turbo et de faux `Complete`, ainsi que la conservation de la source et du pending lorsqu'il existe.
4. Soumettre le diff exact à `rust_review_coordinator` ; au plus deux cycles revue/correction. Rapporter séparément chaque cas `PASS/FAIL/NOT RUN` et les autres tests ignorés.

**Chemin source ouvert dans la première autorisation :** `crates/whisper-adapters/tests/parakeet_import.rs` uniquement. **Écriture d'exécution :** `target/parakeet-tests/l2/l2b/r8/` uniquement pour sorties, temporaires, journaux et fixtures privées ; `target/parakeet-tests/cargo-shared/` uniquement pour Cargo. `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. Modèle, worker R6, DLL de référence, `sound example/`, `spike02`, Cargo.lock et code produit restent en lecture. Réseau, téléchargement, push et changement de fiche interdits. Un besoin hors de ces chemins rend `PLAN_CHANGE_REQUIRED` avant édition.

**Critère de clôture :** cinq cas exécutés sur la clé R6 exacte, cinq `PASS`, zéro faux `Complete`, zéro substitution Turbo et zéro régression de la suite workspace. Un test ignoré, une autre clé ou une écriture hors R8 rend le cas `NOT RUN` ou `FAIL`, sans le convertir en preuve. Cette clôture porte sur le R8 repris et ne change pas les preuves historiques attachées à `545a…`.

## Delta nécessaire après la première revue indépendante

Les cinq cas R8 individuels sont `PASS`. La suite workspace sous la garde R8 a été exécutée deux fois (avec et sans `PARAKEET_L2_ACCEPTANCE_TEST_BUILD`) : **343 PASS, 22 FAIL, 29 ignorés** dans les deux exécutions. Les échecs se répartissent entre deux tests unitaires `worker_ipc`, trois `legacy_model_requests`, un `model_catalog`, six `model_preferences` et neuf `parakeet_archive`. Les premières familles manquent `WHISPER_L1B_RUNTIME_ROOT` ; `parakeet_archive.rs::archive_root()` (lignes 28–49) accepte seulement R6 ou une racine nommée R7, et le cas enfant R4 manque `PARAKEET_L2_EN_LONG_WAV`. Le reviewer rend `BLOCKED` sur le gate, sans défaut R8 attribuable.

**Autorisation complémentaire proposée, limitée à un second chemin de test :** `crates/whisper-adapters/tests/parakeet_archive.rs`, fonction `archive_root()` seulement. Ajouter l'acceptation de `PARAKEET_R8_EXEC_ROOT` lorsque sa valeur canonique est exactement `target/parakeet-tests/l2/l2b/r8` et que l'archive est sous cette racine. Préserver les acceptations R6/R7 et tous les refus hors racine existants. Ne changer aucun test ni aucune règle produit dans ce fichier.

**Variables d'exécution à ajouter à la garde R8, sans code produit :** `WHISPER_L1B_RUNTIME_ROOT=…/l2b/r8/l1b-runtime` et `PARAKEET_L2_EN_LONG_WAV=…/spike02/fixtures/synthetic/spkr0_x12.wav` en lecture seule. Conserver `PARAKEET_R7_EXEC_ROOT=…/l2b/r8` pour le cas enfant R4 existant, avec la vérification de confinement déjà appliquée à cette variable. La garde journalise ses résultats OK/KO dans `…/l2b/r8/logs/guard-ok.log` et `guard-ko.log`. Le modèle, le worker, les DLL et l'audio restent en lecture. Réexécuter les tests du helper et la suite workspace complète sous la garde ; rapporter tous les `FAIL` et `NOT RUN`.

La correction et la revue de ce delta constituent au plus le **second cycle** du candidat R8. Si le gate reste rouge, rendre `LIVRÉ AVEC RÉSERVES` et demander une décision utilisateur, sans élargir encore les chemins.
