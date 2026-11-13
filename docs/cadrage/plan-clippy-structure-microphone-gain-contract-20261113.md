# Plan P12 — Réduire le test de contrat du gain micro

Avancée : plan pour un test privé de 118 lignes.
Blocage : aucun ; le gain demandé/appliqué, les révisions et la capture active sont à préserver.
Décision : extraire seulement le démarrage de la capture directe en helper de setup.

## En-tête

- Source audit : Clippy frais post-P11, 125 sites, journal `.../scratch/20261113T222052906-b9b77c6fb6394b519ce78ce32f642e84/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-core` runtime tests, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — scénario et test support inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Clippy relève 118 lignes pour `microphone_gain_command_is_scoped_to_the_active_direct_capture` dans `application/runtime/tests/selection_and_microphone.rs`. Son premier bloc démarre la capture directe avec un `LiveRequest`; le reste prouve que les demandes de gain sont séquencées, les refus n’arrêtent pas la capture, les événements Applied/Rejected affectent la vue, et un gain ne s’applique pas à une autre génération ni après Stop.

## 2. Fraîcheur

Inspection du fichier inclus via `tests.rs`, du faux port `ScanTestIo` et de l’AGENTS core. P1–P11 sont à préserver; le fichier P12 n’a pas été modifié précédemment. Tests du cœur restent dans `application/runtime/tests/`.

## 3. Hypothèses

- Safe : démarrer la capture avec l’identité donnée constitue un helper privé test-only réutilisable par ce scénario.
- Validation needed : `job_id=901`, génération first, source mic:A, CPU/Auto, modèle/archive et gain initial 0 restent identiques.
- Validation needed : le corps passe sous 100 lignes sans déplacer ou supprimer les assertions de révision/refus/arrêt.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| test gain micro 118 lignes | P12 | Planned | P12-T1 | Extraire le démarrage direct |
| 124 autres sites | Hors lot | Deferred | Aucun | Tranches ultérieures |

## 5. Séquence recommandée

Extraire le montage initial de la capture dans une fonction privée du même fichier. Le test garde le protocole de gain et ses oracles.

### Phase P12 : réduire le setup du test gain micro

Objective : supprimer `too_many_lines` sur le test ciblé.
Closed invariant : même capture active mic:A et même protocole de gains/refus/stop.
Before metrics : 118 lignes; 125 sites.
Target metrics : ≤100 lignes; au plus 124 sites; aucune nouvelle alerte.
Metric commands : Clippy six crates et N1 complet.
Stop conditions : arrêt si request, effet, révisions, événements, effet sur capture ou assertions changent; ou échec d’un gate.
Dependencies : aucune.
Expected outcome : helper privé qui prépare la capture active.
Tasks : P12-T1.

## 6. Tâches

#### P12-T1 — Extraire le setup de la capture active

- Priorité : Medium
- Source : Clippy 118/100 sur `microphone_gain_command_is_scoped_to_the_active_direct_capture`.
- Justification : séparer le setup d’archive/microphone du contrat de gains.
- Fichiers : `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs` seulement.
- Bucket : tests/support privé runtime.
- Scope budget : un fichier de test existant.
- Closed invariant : test ≤100 lignes, même comportement observé.
- Before metrics : 118; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : voir Phase P12.
- Dépendances : aucune.
- Outline : créer un helper privé prenant app, job_id et génération; déplacer exactement le `StartDirectLiveConfigured` initial; remplacer ces lignes par l’appel; laisser toutes demandes et assertions de gain dans le test.
- Tests : test existant, aucun nouveau scénario inline.
- Vérifications : `cargo fmt --check`, Clippy structurel six crates, N1 via `ci/guard-level1.ps1`.
- Provenance : Observed pour site; Repo docs pour garde et tests core.
- Acceptation : corps ≤100; effet de démarrage identique; expected revisions 0→1→2 inchangées; refus stale et autre génération; Applied/Rejected ne terminent pas la capture; Set après Stop refusé; N1 PASS; aucun autre fichier touché.
- Retour arrière : annuler le seul déplacement de setup si une valeur diffère.
- Notes : garder source explicite `mic:A`, gain 0 initial, CPU, destination/modèle de test actuels.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE et espace | Repo docs | PASS | P12-T1 | PE réel non sélectionné |
| Format | fmt check via garde | Repo docs | PASS | P12-T1 | pas de cargo fmt en écriture |
| Clippy | six crates, quatre lints | Observed/Repo docs | ≤124 sites | P12-T1 | GPU N2 hors lot |
| N1 | défaut et sélections worker | Repo docs | PASS | P12-T1 | ignorés restent ignorés |
| Contrat | scénario gain micro existant | Source inferred | toutes attentes préservées | P12-T1 | aucune validation GUI Windows |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P12-G1 | Validation | clôture | Le helper ne change-t-il que le setup ? | Oui | diff et test N1 |

## 9. Hors périmètre

Ne pas modifier les règles de gain du cœur, ports, capture Windows, modèle ou archive; aucun test de fenêtre manuelle prétendu; aucune nouvelle dépendance ni baseline.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | test/helper nommés |
| Sections présentes | Pass | modèle suivi |
| Traçabilité | Pass | 1 planifié, reste différé |
| Tâche complète | Pass | outline/gates/acceptation/rollback |
| Mesures et stop | Pass | 118→≤100 |
| Budget | Pass | un fichier |
| Tests dédiés | Pass | fichier runtime/tests existant |
| Provenance | Pass | rapport et AGENTS |
| Porte claire | Pass | setup-only contrôlé |
| Chemins | Pass | vérifiés |
| Périmètre borné | Pass | un test |

## 11. Prompt d’implémentation

Exécute P12-T1 de `docs/cadrage/plan-clippy-structure-microphone-gain-contract-20261113.md` dans `C:/wt/cs`. Préserve P1–P11. Modifie uniquement `tests/selection_and_microphone.rs`; extrais seulement le démarrage direct et garde tous les événements, révisions et assertions du gain. Exécute fmt, Clippy structurel et N1 par la garde; arrête sur tout échec.
