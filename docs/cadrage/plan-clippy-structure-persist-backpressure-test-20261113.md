# Plan P10 — Réduire le test de contre-pression de persistance

Avancée : plan pour déplacer le faux port local au niveau du module de test.
Blocage : aucun ; conserver les invariants de saturation à huit et réessai du neuvième.
Décision : sortir uniquement le harness local (état, port et implémentation) de la fonction.

## En-tête

- Source audit : Clippy post-P9, 127 sites, journal `.../scratch/20261113T214250949-1845f0c0b5304bccb5b4b6e537aed0b9/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-core/tests/import_contract.rs`, worktree `C:/wt/cs`.
- Mode : plan borné.
- Confiance : high — le test et son faux port local inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

`persist_backpressure_retains_ninth_segment_until_queue_capacity_returns` fait 143 lignes. Il déclare localement `BoundedState`, `BoundedIo` et son implémentation `ImportIoPort`, puis le scénario vérifie la saturation du canal à huit persistences, la rétention/reprise du neuvième segment et l’attente des neuf reçus avant publication.

## 2. Fraîcheur

Le Clippy frais confirme l’alerte dans le fichier. Les P1–P9 sont déjà non committés et incluent une modification voisine P9; ne pas la réécrire ni la supprimer. Les consignes core exigent de garder les faux ports en mémoire et les tests dans `tests/`.

## 3. Hypothèses

- Safe : le harness local devient un type module-privé utilisé par ce seul test.
- Validation needed : capacité reste 8, compteur d’essais 9 au blocage puis 10 après reprise, événement End non consommé avant espace, 9 reçus requis.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| Test de backpressure 143 lignes | P10 | Planned | P10-T1 | Extraire uniquement les définitions du faux port |
| 126 autres sites | Hors lot | Deferred | Aucun | Suites ultérieures |

## 5. Séquence recommandée

Remonter les trois définitions locales au niveau du module permet de réduire le corps sans toucher au parcours des événements ou aux assertions.

### Phase P10 : rendre le harness de backpressure réutilisable dans le fichier

Objective : ramener le test sous 100 lignes.
Closed invariant : même canal borné de capacité 8 et même preuve de reprise du neuvième segment.
Before metrics : 143 lignes; 127 sites.
Target metrics : ≤100 lignes; au plus 126 sites, aucune nouvelle alerte.
Metric commands : Clippy structurel six crates; N1.
Stop conditions : arrêter si capacité, séquences, nombre d’essais, ordre des reçus, publication ou fichier hors périmètre changent.
Dependencies : aucune.
Expected outcome : types de harness privés au niveau module; test garde uniquement arrangement et assertions.
Tasks : P10-T1.

## 6. Tâches

#### P10-T1 — Extraire le faux port borné

- Priorité : Medium
- Source : alerte `too_many_lines` 143/100 du test nommé.
- Justification : séparer l’infrastructure de test du contrat observable.
- Fichiers : `crates/whisper-core/tests/import_contract.rs` uniquement.
- Bucket : tests/support dans le module d’intégration.
- Scope budget : un fichier existant, aucun fichier support/API supplémentaire.
- Closed invariant : scénario de saturation fidèle et test ≤100 lignes.
- Before metrics : 143; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : voir Phase P10.
- Dépendances : aucune.
- Outline : déplacer `BoundedState`, `BoundedIo` et `impl ImportIoPort` en privé au niveau module; garder leurs champs/branches identiques; remplacer les déclarations locales par la construction existante.
- Tests : test d’intégration existant seulement.
- Vérifications : `cargo fmt --check`, Clippy structurel six crates, N1 complet via `ci/guard-level1.ps1`.
- Provenance : Observed pour le site; Repo docs pour les gates.
- Acceptation : corps ≤100; canal toujours `sync_channel(8)`; saturation conserve 8 effets, neuvième retenu, reprise produit 1..=9; aucune publication avant 9 reçus; 10 tentatives; P9 helper intact; N1 PASS.
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

Exécute P10-T1 du plan `docs/cadrage/plan-clippy-structure-persist-backpressure-test-20261113.md`. Préserve les autres changements P1–P9, surtout le helper voisin P9. Modifie seulement `import_contract.rs`. Sors les types/impl du faux port sans modifier leur logique et garde toutes les assertions. Exécute fmt, Clippy structurel et N1 par la garde; arrête sur échec.
