# Plan P15 — Qualifier un groupe d’observations GPU

Avancée : plan ciblé sur `gpu_inventory::qualify` (113 lignes).
Blocage : aucun ; ordre, consolidation et couverture du catalogue doivent rester identiques.
Décision : extraire la qualification d’une identité dans un helper privé.

## En-tête

- Source audit : Clippy frais P14, 122 sites, `.../scratch/20261113T231148612-61ac4fffd8af4bc78819e6b3c8b49cde/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-core/gpu_inventory.rs`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — groupement d’observations et couvertures inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

`qualify` regroupe les observations par identité tout en retenant l’ordre de première apparition. Pour chaque groupe, elle ignore l’absence complète, ajoute une cause de collection à la présence inconnue/contradictoire, ou consolide noms/état/rang/identité/support/disponibilité et construit un appareil avec ses causes. Enfin elle calcule Complete/Partial/Failed et ajoute l’échec global éventuel.

## 2. Fraîcheur

Inspection de `gpu_inventory.rs`, appels `qualify` et tests `tests/gpu_inventory.rs`/`tests/gpu_selection.rs`, AGENTS core lu. P1–P14 à préserver. Les règles de couverture de l’inventaire sont un contrat produit; seul le découpage privé est visé.

## 3. Hypothèses

- Safe : un groupe d’identité peut retourner `Ok(None)` pour absent, `Err(GpuCause::Collection(_))` pour présence incohérente, ou `Ok(Some(device))` pour un appareil consolidé.
- Validation needed : l’ordre d’insertion des appareils et causes reste celui d’origine.
- Validation needed : l’échec global conserve la règle had_observations et son ajout final de cause.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `qualify` 113 lignes | P15 | Planned | P15-T1 | Extraire une identité observée |
| 121 autres sites | Hors lot | Deferred | Aucun | Tranches ultérieures |

## 5. Séquence recommandée

Le helper consolide seulement une identité; la fonction publique garde le regroupement ordonné, agrégation des causes, calcul de couverture et résultat final.

### Phase P15 : extraire la consolidation d’une identité GPU

Objective : enlever l’alerte `too_many_lines` ciblée.
Closed invariant : mêmes appareils, causes et état de couverture pour les mêmes observations.
Before metrics : 113 lignes; 122 sites.
Target metrics : `qualify` ≤100 lignes; au plus 121 sites, sans nouveaux diagnostics.
Metric commands : Clippy structurel six crates, N1.
Stop conditions : arrêter si ordre première observation, champs consolidés, causes, couverture, règle failed/had_observations ou erreur de compilation change.
Dependencies : aucune.
Expected outcome : fonction privée `qualify_identity_records`.
Tasks : P15-T1.

## 6. Tâches

#### P15-T1 — Consolider un groupe d’observations qualifiées

- Priorité : Medium
- Source : Clippy 113/100 sur `qualify`.
- Justification : distinguer la qualification par identité du calcul final de couverture.
- Fichiers : `crates/whisper-core/src/gpu_inventory.rs` seulement.
- Bucket : domain/policy.
- Scope budget : un fichier production.
- Closed invariant : consolidation identique d’un groupe d’identité.
- Before metrics : 113; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : Phase P15.
- Dépendances : aucune.
- Outline : extraire le bloc d’une identité dans un helper privé retournant `Result<Option<GpuQualifiedDevice>, GpuCause>`; conserver le `BTreeMap` et le vecteur `order`; l’appelant ajoute device/cause dans les mêmes branches et positions.
- Tests : conserver les tests `gpu_inventory` et `gpu_selection`; pas de nouveau test inline.
- Vérifications : fmt, Clippy structurel six crates, N1 via `ci/guard-level1.ps1`.
- Provenance : Observed pour le diagnostic; Repo docs pour gates.
- Acceptation : absent→None; présence incohérente→même cause Collection; appareils en même ordre avec mêmes états/rang/support/disponibilité/causes; couverture et failed cause identiques; diagnostic supprimé; N1 PASS; seul fichier cible modifié.
- Retour arrière : restaurer le bloc initial si un résultat diffère.
- Notes : préserver les textes GPU inconnus/causes exactes et fallback values.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE, disque | Repo docs | PASS | P15-T1 | témoin réel PE non exécuté |
| Format | fmt check via garde | Repo docs | PASS | P15-T1 | pas d’écriture rustfmt |
| Clippy | six crates et lints structurels | Observed/Repo docs | ≤121 sites | P15-T1 | N2 GPU exclu |
| N1 | défaut + sélection workers CPU | Repo docs | PASS | P15-T1 | GPU corps tests ignorés |
| GPU policy | tests inventory/selection existants | Source inferred | PASS | P15-T1 | pas de GPU matériel requis |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P15-G1 | Validation | clôture | Résultats exacts en ordre d’identité ? | Oui | tests et comparaison du diff |

## 9. Hors périmètre

Ne pas modifier observation Windows, collecte GPU, critères de support, exposition API, manifestes ou baseline; aucune requête à matériel GPU.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | helper et résultat nommés |
| Sections présentes | Pass | modèle suivi |
| Traçabilité | Pass | 1 planifié, autres différés |
| Tâche complète | Pass | outline/tests/gates/rollback |
| Métriques/arrêt | Pass | 113→≤100 |
| Budget | Pass | un fichier |
| Tests dédiés | Pass | tests GPU existants |
| Provenance | Pass | rapport/source/AGENTS |
| Porte | Pass | ordre et couverture |
| Chemins | Pass | constatés |
| Scope | Pass | qualification par identité seulement |

## 11. Prompt d’implémentation

Exécute P15-T1 du plan `docs/cadrage/plan-clippy-structure-gpu-qualified-records-20261113.md` dans `C:/wt/cs`. Préserve P1–P14. Modifie seulement `gpu_inventory.rs`; extrais la qualification d’un groupe, garde ordre/champs/causes/couverture. Passe SelfTests, fmt, Clippy et N1 via garde; arrête sur tout échec.
