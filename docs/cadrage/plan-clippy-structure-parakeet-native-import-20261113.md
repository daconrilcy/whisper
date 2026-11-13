# Plan P5 — Réduire le test de normalisation native Parakeet

Avancée : cadrage d’un test natif ignoré de 105 lignes.
Blocage : aucun ; l’exécution des tests ignorés et des DLL natives reste hors N1.
Décision : découper le scénario en helpers privés, sans exécuter le moteur natif.

## En-tête

- Source audit : Clippy structurel frais post-P4, six crates, 13 novembre 2026, journal `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261113T203040211-e02926f4185b41df94c999be6f772f73/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-worker-parakeet-cpu`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — l’alerte et le test sont présents dans le journal et les sources.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Clippy relève 132 sites structurels après P4. Le test `cpu_worker_normalizes_fr87_fr27_and_en15_short_fixtures` fait 105 lignes (`tests/native_import.rs:483`). Il est conditionnellement ignoré sans `native-tests` et demande des WAV, DLL et un worker qualifiés. Le découpage doit seulement clarifier le scénario ; aucune preuve native ne sera revendiquée.

## 2. Fraîcheur

Sources ciblées inspectées : `tests/native_import.rs`, `tests/support/worker.rs`, `crates/whisper-worker-parakeet-cpu/AGENTS.md` et rapport Clippy post-P4. Le fichier de test contient d’autres cas natifs ; ils sont hors périmètre. Le worktree porte des modifications P1–P4 non commitées à préserver.

## 3. Hypothèses

- Safe : le montage du worker, les trois requêtes FR/EN et les vérifications de mots peuvent être extraits en helpers privés dans le même test d’intégration.
- Validation needed : préserver l’ordre des trois langues, les identités et contrôles de timestamps.
- Décision gate : aucune activation de `native-tests`; la recette réelle reste différée.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| Test natif 105 lignes à `native_import.rs:483` | P5 | Planned | P5-T1 | Helpers privés, comportement préservé |
| Autres sites du rapport (131) | Hors lot | Deferred | Aucun | Lots ultérieurs, hors plan P5 |

## 5. Séquence recommandée

Une tâche ferme l’invariant du test sans toucher au produit ou aux tests voisins.

### Phase P5 : isoler les phases du test de normalisation

Objective : supprimer l’alerte Clippy sur le test ciblé.
Closed invariant : le test principal fait au plus 100 lignes avec les mêmes trois fixtures et assertions.
Before metrics : 105 lignes ; 132 sites structurels post-P4.
Target metrics : test ≤100 lignes ; au plus 131 sites, aucune nouvelle alerte.
Metric commands : Clippy structurel six crates et lecture du diff.
Stop conditions : arrêter si changement de protocole/attentes, besoin de DLL/audio, échec N1/Clippy, ou dépassement du fichier prévu.
Dependencies : aucune.
Expected outcome : extraction de helpers privés de test.
Tasks : P5-T1.

## 6. Tâches

#### P5-T1 — Extraire la préparation, les appels et les assertions de normalisation

- Priorité : Medium
- Source : diagnostic `too_many_lines` à `tests/native_import.rs:483`, 105 lignes.
- Justification : supprimer une alerte structurelle en gardant intacte la preuve envisagée pour la recette native.
- Fichiers : `crates/whisper-worker-parakeet-cpu/tests/native_import.rs` seulement.
- Bucket : tests/support local au fichier d’intégration.
- Scope budget : un fichier existant ; aucun nouveau fichier ni fichier production.
- Closed invariant : le test orchestrateur fait ≤100 lignes.
- Before metrics : 105 lignes ; target ≤100.
- Metric commands : Clippy structurel via la garde de niveau 1.
- Stop conditions : celles de Phase P5 ; ne pas lancer les tests ignorés.
- Dépendances : aucune.
- Outline : extraire le chargement/contrôle du worker; extraire l’itération des trois requêtes et assertions de résultat; extraire les assertions spécifiques FR87 existantes sans les affaiblir.
- Tests : conserver le test ignoré et son attribut; aucun test ajouté.
- Vérifications : format, Clippy six crates, N1 complet via `ci/guard-level1.ps1`.
- Provenance : Repo docs pour la garde et les interdictions natives; Observed pour l’alerte.
- Acceptation : test ≤100 lignes; aucune nouvelle alerte; N1 et Clippy passent; seul le fichier nommé modifié; le test demeure ignoré sans feature native.
- Retour arrière : annuler uniquement l’extraction si elle change le protocole ou les assertions.
- Notes : conserver les fixtures `fr87`, `fr27`, `en15`, l’ordre, les contrôles d’identité, le texte non vide/sans caractère de remplacement, et la monotonie/non-chevauchement des timestamps.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Résultat attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Garde | SelfTest + contrôles workers PE | Repo docs | PASS | P5-T1 | pas de témoin PE réel requis |
| Format | `cargo fmt --check` via garde | Repo docs | PASS | P5-T1 | aucune écriture hors racines |
| Clippy | six crates et quatre lints structurels via garde | Observed/Repo docs | ≤131 sites, ratchet passe | P5-T1 | aucun `--fix` |
| N1 | `cargo test --no-fail-fast`, puis sélections worker de référence via garde | Repo docs | PASS | P5-T1 | test natif ignoré, CUDA N2 exclu |
| Revue | diff/assertions | Source inferred | preuve potentielle préservée | P5-T1 | ne prouve pas une transcription native |

## 8. Portes de décision

| Gate | Statut | Bloque | Question | Valeur par défaut | Validation |
|---|---|---|---|---|---|
| P5-G1 | Validation | clôture | Les assertions natives restent-elles non exécutées ? | Oui | Inspecter les commandes réelles |

## 9. Hors périmètre

Ne pas modifier les DLL, le worker, protocoles, fixtures audio, feature `native-tests`, autres tests natifs ou contrôles GPU. Ne pas lancer un test ignoré, build natif, GPU N2 ou modifier baseline/manifeste.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan réel et exécutable | Pass | Un test ciblé |
| Sections du modèle présentes | Pass | Dans l’ordre |
| Constat tracé ou différé | Pass | 1 planifié, 131 différés |
| Fichiers, tâches, vérifications, critères, rollback | Pass | Inclus |
| Invariant, métriques, arrêts | Pass | 105→≤100 |
| Budget explicite | Pass | Un fichier |
| Tests dédiés et non ajoutés en inline | Pass | Test existant préservé |
| Provenance des contrôles | Pass | Garde/règles et diagnostic observé |
| Portes et limites | Pass | Exécution native exclue |
| Chemins valides | Pass | Inspectés dans la crate |
| Périmètre borné | Pass | Aucune source production |

## 11. Prompt d’implémentation

Exécute P5-T1 de `docs/cadrage/plan-clippy-structure-parakeet-native-import-20261113.md` dans `C:/wt/cs`. Préserve les modifications des autres lots. Modifie seulement `tests/native_import.rs`, garde le test et ses attributs ignorés, et conserve toutes les attentes FR87/FR27/EN15. Exécute les gates et la matrice N1/Clippy par `ci/guard-level1.ps1`. N’active pas `native-tests` et arrête sur tout échec.
