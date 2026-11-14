# Plan P20 — Extraire le faux inspecteur bloquant du test catalogue

Avancée : plan ciblé sur un helper de test de 117 lignes.
Blocage : aucun ; la synchronisation du thread et le comptage d’inspections doivent rester identiques.
Décision : extraire la closure `FixtureObserver` en helper privé de test.

## En-tête

- Source audit : Clippy frais P19, 117 sites, `.../scratch/20261114T110018853-349d2d35ca8b42d3ab6f4262d3e281e6/cargo-command.stdout.log`.
- Date : 14 novembre 2026.
- Espace : `whisper-desktop/src/root/model_catalog/tests.rs`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — helper, closure et deux tests appelants inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Clippy relève 117 lignes dans `check_removed_interest_stops_monitoring`. Le helper monte deux canaux de contrôle pour bloquer une première inspection, démarre le service, retire l’intérêt demandé pendant que l’inspection est en vol, libère l’inspection, puis vérifie que les suppressions n’entraînent pas de nouvelle observation ni d’effet obsolète. Une closure `FixtureObserver` contrôle le compteur, la synchronisation et injecte une qualification déterministe par-dessus la lecture réelle du catalogue.

## 2. Fraîcheur

Inspection du helper et des deux tests appelants (resume false/true, autre demande false/true), de `FixtureObserver`, et de l’AGENTS desktop. P1–P19 à préserver. Le lot ne change pas l’application/UI de production.

## 3. Hypothèses

- Safe : déplacer la closure dans `blocking_catalog_fixture_observer` sans changer ses captures ni la lecture `inspect_model`.
- Validation needed : compteur/ordre d’appel, barrier `entered_tx/release_rx`, validation modèle Ready et tous les asserts restent identiques.
- Validation needed : les quatre combinaisons testées par les appelants continuent de s’exécuter.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `check_removed_interest_stops_monitoring` 117 lignes | P20 | Planned | P20-T1 | Extraire la closure observer |
| 116 autres sites | Hors lot | Deferred | Aucun | Nouvelle tranche requise |

## 5. Séquence recommandée

Extraire uniquement le faux inspecteur; conserver dans le test l’orchestration des canaux, retrait d’intérêt, reprise de l’inspection et toutes les assertions.

### Phase P20 : séparer le fixture d’inspection du scénario concurrent

Objective : supprimer l’alerte `too_many_lines` ciblée.
Closed invariant : retrait d’intérêt pendant inspection empêche toute sollicitation obsolète après la fin.
Before metrics : 117 lignes; 117 sites.
Target metrics : test ≤100 lignes; au plus 116 sites; aucune alerte nouvelle.
Metric commands : Clippy six crates et N1.
Stop conditions : arrêter si la synchronisation/ordre, le compteur, le résultat Ready, la sélection ou les effets diffèrent; ou à tout échec gate.
Dependencies : aucune.
Expected outcome : une fonction privée de construction FixtureObserver.
Tasks : P20-T1.

## 6. Tâches

#### P20-T1 — Extraire la closure d’observation bloquante

- Priorité : Medium
- Source : Clippy 117/100 sur `check_removed_interest_stops_monitoring`.
- Justification : isoler un faux inspecteur réutilisable par ce test tout en gardant le scénario concurrent lisible.
- Fichiers : `crates/whisper-desktop/src/root/model_catalog/tests.rs` seulement.
- Bucket : tests/support local à root/model_catalog.
- Scope budget : un fichier de tests existant.
- Closed invariant : test principal ≤100 lignes, mêmes outcomes pour resume/another_need.
- Before metrics : 117; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : Phase P20.
- Dépendances : aucune.
- Outline : créer `blocking_catalog_fixture_observer(observed_calls, entered_tx, release_rx) -> FixtureObserver`; déplacer la closure en conservant l’assertion du ModelId, blocage uniquement au premier appel, timeout `WAIT`, appel réel à `inspect_model`, et surcouche Ready/sha/size/diagnostic; remplacer la déclaration locale par cet appel.
- Tests : conserver les deux tests appelants et leurs boucles `resume` avec `another_need` false/true; aucun test nouveau inline.
- Vérifications : fmt check, Clippy six crates, N1 via garde.
- Provenance : Observed pour diagnostic/test; Repo docs pour les gates.
- Acceptation : test ≤100 lignes; 4 combinaisons continuent d’être couvertes; premier appel attend le release, appels suivants ne bloquent pas; même observation Ready déterministe; tests/N1 PASS; seul fichier nommé touché.
- Retour arrière : annuler l’extraction si la synchro, le nombre d’appels ou le rendu de l’observation diffère.
- Notes : conserver le `Send` requis par le service et la lecture réelle d’`inspect_model`.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE, espace | Repo docs | PASS | P20-T1 | PE réel non exécuté |
| Format | fmt check via garde | Repo docs | PASS | P20-T1 | aucun fmt en écriture |
| Clippy | six crates, quatre lints structurels | Observed/Repo docs | ≤116 sites | P20-T1 | GPU N2 non requis |
| N1 | défaut et workers référence | Repo docs | PASS | P20-T1 | tests ignorés inchangés |
| Catalogue | deux tests d’orchestration et combinations | Source inferred | PASS | P20-T1 | aucun GUI Windows |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P20-G1 | Validation | clôture | Barrière seulement sur premier appel et compteur inchangés ? | Oui | N1 + revue helper |

## 9. Hors périmètre

Ne pas modifier CatalogService, stratégie de qualification, événements UI, canaux de production, snapshots, délais ou autres tests. Aucun nouveau test ni API produit.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | closure/helper/test nommés |
| Sections présentes | Pass | modèle suivi |
| Traçabilité | Pass | 1 prévu, reste différé |
| Tâche complète | Pass | étapes/gates/critères/rollback |
| Mesures/arrêts | Pass | 117→≤100 |
| Budget | Pass | un fichier |
| Tests dédiés | Pass | test module existant |
| Provenance | Pass | Clippy/source/AGENTS |
| Porte | Pass | synchro/count contrôlables |
| Chemins | Pass | constatés |
| Périmètre | Pass | fixture seul |

## 11. Prompt d’implémentation

Exécute P20-T1 du plan `docs/cadrage/plan-clippy-structure-model-catalog-observer-test-20261114.md` dans `C:/wt/cs`. Préserve P1–P19. Modifie uniquement `root/model_catalog/tests.rs`; extrais la closure en FixtureObserver sans changer barrières/counter/read/Ready fields. Passe SelfTests, fmt, Clippy et N1 via garde; arrête au premier échec.
