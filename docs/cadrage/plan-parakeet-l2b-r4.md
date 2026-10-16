Avancée : le candidat L2b a été relu après deux cycles ; les règles VAD et single-pass sont séparées, mais B8 reste désactivée.
Blocage : R4 HIGH/BLOQUANT — l'archive admet une clé L2b `import-single-pass` dont l'import FR produit échoue.
Décision : retirer cette seule admission dans un lot correctif distinct, puis vérifier le refus avant pending sans altérer L2a, Turbo ou la clé VAD.

# PLAN-PARA-L2b-R4 v1 — retrait de l'admission courte L2b non qualifiée

**Statut : CLEAN — PLANS READY-R4.** Revue indépendante du commit `e61c865f` : aucun REQUIRED, les deux chemins jugés suffisants et l'ordre Prepare → archive → Prepared → StartWorker vérifié dans le code. Auteur : session Codex principale. Base Git `8a4385e9` sur `evo-startup-perf`, plan parent `PLAN-PARA-L2 v6` CLEAN — PLANS READY-L2b, candidat L2b non commité ; patch final du cycle 2 SHA-256 `fc9161721c3f1de9c93595497707af98dff9d496ee8ff616f41d1f9eb6349b65`. Le reviewer L2b a rendu **LIVRÉ AVEC RÉSERVES** après deux cycles, R4 ouvert. L'utilisateur a autorisé dans cette session la préparation, la revue puis l'implémentation conditionnelle de ce lot **sur les deux chemins ci-dessous seulement**. Ce lot ne rouvre pas le cycle L2b précédent et ne qualifie pas B4–B8. La revue du plan ne ferme pas R4 : la correction et ses tests attendent une revue de code distincte.

## 1. Cause, résultat et limites

Le worker L2b figé `aa8dfdf317649eabbfe670c5109df46c70d3f1908064bee61d26fe7aa059bf31` a réussi vingt inférences natives FR/EN et la parité WAV/MP3, mais l'import court FR par l'application échoue avec `InvalidTimestamp` lorsque le comportement single-pass L2a est préservé. Dans le candidat courant, `ArchiveQualificationKey::is_parakeet_import_test` accepte pourtant la paire runtime/worker L2b pour `import-single-pass`. `ArchiveStore::prepare_inner` peut alors créer un pending version 2 et `validate_pending` le traite comme qualifié au scan. Le routeur produit obtient cette clé de ses chemins Parakeet et appelle la préparation avant le démarrage du worker. Cette admission est donc incompatible avec l'échec FR observé ; aucune réussite du harness natif ne la remplace.

**Résultat attendu :** `import-single-pass` accepte uniquement la clé Test L2a historique déjà inscrite dans le registre. La paire L2b courte non qualifiée est refusée avant création de destination/pending et ne peut être présentée comme pending sain ou reprise exécutable. Les pending L2a historiques, la validation Turbo et la clé L2b `import-vad` restent régis par leurs règles actuelles ; cette dernière demeure **inactive côté produit** tant que B8 n'est pas qualifiée. Le refus conserve la source et les données préexistantes avec une cause explicite. Aucun autre profil, seuil VAD, horodatage, binaire ou identité native n'est changé.

## 2. Gates et propriété

| Gate | Vérification avant action | Échec |
| --- | --- | --- |
| R4-P0 | Branche/HEAD, candidat dirty et patch SHA ci-dessus relevés ; plan parent et verdicts L2b relus ; les deux fichiers ciblés sauvegardés en comparaison sous le runtime L2b. | Aucune écriture source. |
| R4-P1 | Ce plan versionné dans Git, revu CLEAN ; accord utilisateur conditionnel sur les deux chemins toujours applicable. | Aucune écriture source. |
| R4-P2 | Modèle, worker, cinq DLL, fixtures et racines de test présents avec leurs SHA attendus ; Cargo offline/locked disponible. Aucun nouvel essai natif n'est requis pour retirer une admission. | Tests dépendants `NOT RUN`, sans téléchargement ni autre chemin. |

`rust_implementer` possède la correction et ses tests. `rust_review_coordinator` mandate un `rust_reviewer` indépendant ; lui seul ferme R4. Deux cycles revue/correction au plus pour **ce lot distinct**. La session principale conserve le candidat non commité et la rédaction documentaire. Les agents ne modifient pas les fichiers hors périmètre.

**`lot.paths.write` source, exclusifs (2) :**

```text
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/tests/parakeet_archive.rs
```

**`lot.paths.runtime_write` :** `target/parakeet-tests/l2/l2b/` exclusivement pour Cargo, temporaires, comparaison avant/après et résultats. `models/`, `sound example/`, le spike, les autres sources et la racine L2a-2 restent en lecture. Aucun réseau, téléchargement, changement C++/FFI, `Cargo.toml`, `Cargo.lock`, commit du code ou push dans ce lot. Si une correction requiert un autre chemin ou une règle produit nouvelle : `PLAN_CHANGE_REQUIRED` avant édition.

## 3. Tâches et cartes

| Tâche | Écriture | Preuve attendue |
| --- | --- | --- |
| R4.1 | `archive.rs` | Retirer uniquement la branche L2b du profil `import-single-pass` dans `is_parakeet_import_test` et aligner la validation des pending version 2 sur la clé L2a exacte. Conserver les contrôles de profil, modèle, runtime, worker et backend, et la clé VAD distincte. |
| R4.2 | `parakeet_archive.rs` | Remplacer l'attente d'admission L2b courte par un refus attribué avant destination/pending ; modèle/source inchangés, aucun CURRENT/TXT/SRT. Tester une clé L2a historique acceptée, les clés divergentes refusées et la clé VAD inchangée. |
| R4.3 | `parakeet_archive.rs` | À partir d'un pending existant portant la clé L2b courte, vérifier qu'un scan le signale non qualifié et qu'une reprise ne le traite pas comme exécutable ; préserver physiquement source et pending. Ne pas fabriquer une nouvelle réussite L2b par un chemin de test. |
| R4.4 | Aucun autre fichier | Lire la route `ImportEffect::Prepare`/`StartWorker` dans `worker_ipc.rs` pour vérifier que le refus d'archive précède le lancement natif ; vérifier format, diff, tests d'archive, tests adapters/core pertinents et build workspace offline/locked ; rendre le statut R4 et les limites B4–B8 sans les requalifier. Si cette lecture ne prouve pas l'ordre des effets, demander un changement de plan pour un test de route produit avant de déclarer R4 fermé. |

**Critères d'acceptation R4 :** zéro nouveau pending/destination après la demande L2b courte non qualifiée ; aucune sortie et aucun lancement natif selon l'ordre vérifié de la route produit ; cause `ModelNotQualified` ou équivalent explicite ; un ancien pending L2b court est conservé mais non repris ; clé L2a historique toujours admissible ; Turbo inchangé ; clé VAD exacte conservée sans activation B8. Une réussite de tests purs ne constitue pas une qualification VAD longue. Le test peut utiliser un chemin de destination inédit sous le runtime pour constater l'absence d'effet ; il ne supprime pas des données utilisateur.

## 4. Exécution et sortie

Avant chaque commande Cargo, résoudre `CARGO_TARGET_DIR`, `TEMP`, `TMP`, `WHISPER_L1B_RUNTIME_ROOT` et les racines `PARAKEET_L2_*` sous `target/parakeet-tests/l2/l2b/`, puis restaurer les anciennes valeurs. Les entrées modèle/audio/worker/DLL sont explicites et en lecture ; aucune fixture de secours. Les tests marqués `ignored` restent dénombrés séparément et ne sont lancés que si nécessaires au risque R4. Commandes minimales après correction :

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-adapters --test parakeet_archive
cargo test --offline --locked -p whisper-adapters --test model_catalog --test model_preferences --test legacy_model_requests
cargo test --offline --locked -p whisper-core --test parakeet_import_contract
cargo build --offline --locked --workspace --target-dir target/parakeet-tests/l2/l2b/cargo-r4-regression/
git diff --check
```

Le handoff inclut le diff isolé par rapport aux deux fichiers pré-correction, les sorties et nombres de tests réellement exécutés, le SHA du worker/modèle/DLL lus, le statut de R4 et tout constat ouvert. Après deux cycles au maximum, le verdict est CLEAN ou LIVRÉ AVEC RÉSERVES. Le candidat L2b complet reste non commité ; aucune activation B8, qualification Final, GPU ou live ne découle de R4.
