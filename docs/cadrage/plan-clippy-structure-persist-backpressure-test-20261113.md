# Plan P10 — Réduire le test de contre-pression de persistance

Avancée : plan pour déplacer le faux port et le setup des événements hors du test.
Blocage : aucun ; conserver les invariants de saturation à huit et réessai du neuvième.
Décision : extraire le harness local (état/port/implémentation) et le setup d’événements en helpers privés du même fichier.

## En-tête

- Source audit : Clippy post-P9, 127 sites, journal `.../scratch/20261113T214250949-1845f0c0b5304bccb5b4b6e537aed0b9/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-core/tests/import_contract.rs`, worktree `C:/wt/cs`.
- Mode : plan borné.
- Confiance : high — le test et son faux port local inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

`persist_backpressure_retains_ninth_segment_until_queue_capacity_returns` fait 143 lignes. Il déclare localement `BoundedState`, `BoundedIo` et son implémentation `ImportIoPort`, prépare les événements/segments, puis vérifie la saturation du canal à huit persistences, la rétention/reprise du neuvième segment et l’attente des neuf reçus avant publication. Le déplacement du seul harness laisse le corps à 115 lignes; l’extraction du setup en helper privé est nécessaire pour fermer le critère ≤100.

## 2. Fraîcheur

Le Clippy frais confirme l’alerte dans le fichier. Les P1–P9 sont déjà non committés et incluent une modification voisine P9; ne pas la réécrire ni la supprimer. Les consignes core exigent de garder les faux ports en mémoire et les tests dans `tests/`.

## 3. Hypothèses

- Safe : le harness local devient un type module-privé et la préparation des événements devient un helper privé du même fichier.
- Validation needed : capacité reste 8, compteur d’essais 9 au blocage puis 10 après reprise, événement End non consommé avant espace, 9 reçus requis.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| Test de backpressure 143 lignes | P10 | Planned | P10-T1 | Extraire les définitions du faux port et la préparation d’événements |
| 126 autres sites | Hors lot | Deferred | Aucun | Suites ultérieures |

## 5. Séquence recommandée

Remonter le harness et extraire l’enqueue des événements permet de réduire le corps sans toucher au parcours Refresh, aux effets acceptés ou aux assertions.

### Phase P10 : rendre le harness de backpressure réutilisable dans le fichier

Objective : ramener le test sous 100 lignes.
Closed invariant : même canal borné de capacité 8 et même preuve de reprise du neuvième segment.
Before metrics : 143 lignes; 127 sites.
Target metrics : ≤100 lignes; au plus 126 sites, aucune nouvelle alerte.
Metric commands : Clippy structurel six crates; N1.
Stop conditions : arrêter si capacité, séquences, nombre d’essais, ordre des reçus, publication ou fichier hors périmètre changent.
Dependencies : aucune.
Expected outcome : types et setup privés au niveau module; test garde le parcours Refresh et ses assertions.
Tasks : P10-T1.

## 6. Tâches

#### P10-T1 — Extraire le faux port et le setup de backpressure

- Priorité : Medium
- Source : alerte `too_many_lines` 143/100 du test nommé.
- Justification : séparer le harness et la mise en file du contrat observable pour respecter le seuil Clippy.
- Fichiers : `crates/whisper-core/tests/import_contract.rs` uniquement.
- Bucket : tests/support dans le module d’intégration.
- Scope budget : un fichier existant, aucun fichier support/API supplémentaire.
- Closed invariant : scénario de saturation fidèle et test ≤100 lignes.
- Before metrics : 143; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : voir Phase P10.
- Dépendances : aucune.
- Outline : déplacer `BoundedState`, `BoundedIo` et `impl ImportIoPort` au niveau module; extraire l’enqueue de Ready/Progress/neuf segments/End dans `enqueue_backpressure_events`; garder dans le test le dispatch Refresh, la consommation d’effets, les ACK et les assertions.
- Tests : test d’intégration existant seulement.
- Vérifications : `cargo fmt --check`, Clippy structurel six crates, N1 complet via `ci/guard-level1.ps1`.
- Provenance : Observed pour le site; Repo docs pour les gates.
- Acceptation : corps ≤100; canal toujours `sync_channel(8)`; saturation conserve 8 effets, neuvième retenu, reprise produit 1..=9; aucune publication avant 9 reçus; 10 tentatives; helper P9 byte-for-byte inchangé; N1 PASS.
- Retour arrière : annuler le déplacement des types si un résultat diffère.
- Notes : garder le faux port privé et limité à ce fichier d’intégration.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde et PE | Repo docs | PASS | P10-T1 | témoin réel non exécuté |
| Format | fmt check gardé | Repo docs | PASS | P10-T1 | format en écriture interdit |
| Clippy | six crates, quatre lints | Observed/Repo docs | ≤126 sites | P10-T1 | GPU N2 non requis |
| N1 | défaut + workers CPU | Repo docs | PASS | P10-T1 | tests ignorés gardés |
| Contrat | test saturation/reprise | Source inferred | toutes assertions existantes passent | P10-T1 | pas de nouveau test |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P10-G1 | Validation | clôture | La backpressure conserve-t-elle sa borne et son ordre ? | Oui | N1 et diff |

## 9. Hors périmètre

Ne pas modifier la politique de canal du produit, le handler, l’API, le scénario 99 segments voisin, le modèle de publication ou la baseline.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | Harness/test identifiés |
| Sections présentes | Pass | Modèle suivi |
| Constat tracé | Pass | un prévu, reste différé |
| Tâche complète | Pass | étapes/gates/acceptation/rollback |
| Métriques et arrêts | Pass | 143→≤100 |
| Budget | Pass | un fichier |
| Test sous tests/ | Pass | fichier existant |
| Provenance | Pass | journal et AGENTS |
| Porte | Pass | borne/ordre vérifiables |
| Chemins | Pass | confirmés |
| Périmètre | Pass | seulement harness local |

## 11. Prompt d’implémentation

Exécute P10-T1 du plan `docs/cadrage/plan-clippy-structure-persist-backpressure-test-20261113.md` révisé. Préserve P1–P9, surtout le helper P9 byte-for-byte. Modifie seulement `import_contract.rs`. Extrais le harness et l’enqueue d’événements en helpers privés sans changer leur logique; garde toutes les assertions. Exécute fmt, Clippy structurel et N1 par la garde.
