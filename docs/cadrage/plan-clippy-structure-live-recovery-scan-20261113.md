# Plan P11 — Isoler le scan d’une récupération live

Avancée : plan ciblé sur `scan_live_recoveries` (105 lignes).
Blocage : aucun ; la sélection de génération et la preuve de couverture sont sensibles.
Décision : extraire le traitement d’un seul dossier de job dans un helper privé.

## En-tête

- Source audit : Clippy frais après P10, 126 sites, journal `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261113T215329974-2c464af7badd42cd93cdbbf0ef37e6ae/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-adapters/archive/recovery/live.rs`, worktree `C:/wt/cs`.
- Mode : plan borné.
- Confiance : high — diagnostic et sélection par job inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

`scan_live_recoveries` fait 105 lignes et parcourt les jobs puis les générations live : elle sélectionne le plus récent pending, vérifie sa cohérence avec le manifeste et la génération courante, inspecte le checkpoint PCM, scanne le préfixe confirmé et exige que le manifeste soit entièrement confirmé avant d’annoncer `publication_ready`.

## 2. Fraîcheur

Inspection ciblée de `recovery/live.rs`, tests de recovery existants et AGENTS adapters. Les lots P1–P10 sont des changements locaux à préserver. La refactorisation ne doit pas modifier la récupération durable ni les hooks de qualification.

## 3. Hypothèses

- Safe : la boucle extérieure peut agréger `Option<LiveRecoveryInfo>` d’un helper pour chaque job.
- Validation needed : les `continue` internes deviennent uniquement `Ok(None)` au bon niveau; toute erreur I/O/corruption/campaign gate est toujours propagée.
- Validation needed : l’ordre des campaign gates avant/après scan reste identique.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `scan_live_recoveries` 105 lignes | P11 | Planned | P11-T1 | Extraire la sélection par dossier de job |
| 125 autres sites | Hors lot | Deferred | Aucun | Lots ultérieurs |

## 5. Séquence recommandée

Conserver la façade qui parcourt les `job_entry` et recueille les résultats; mettre la sélection de reprise pour un dossier dans un helper privé. Garder erreurs, filtres et ordre strictement inchangés.

### Phase P11 : traiter un dossier de récupération par helper

Objective : supprimer l’alerte sans changer la politique de reprise.
Closed invariant : exactement les mêmes générations/récupérations sont retournées, mêmes erreurs propagées.
Before metrics : 105 lignes; 126 sites.
Target metrics : ≤100 lignes; au plus 125 sites, pas de nouvelle alerte.
Metric commands : Clippy six crates, N1 incluant tests recovery.
Stop conditions : arrêt si sélection pending/courante, filtre manifest, contrôle checkpoint/couverture, gates ou publication_ready changent; tout échec de campagne arrête le lot.
Dependencies : aucune.
Expected outcome : `scan_live_recovery_job` privé avec résultat Option.
Tasks : P11-T1.

## 6. Tâches

#### P11-T1 — Extraire le scan d’un job live

- Priorité : Medium
- Source : `scan_live_recoveries`, Clippy 105/100.
- Justification : isoler une unité de récupération et garder l’agrégation simple.
- Fichiers : `crates/whisper-adapters/src/archive/recovery/live.rs` seulement.
- Bucket : adapters/infra — archive recovery.
- Scope budget : un fichier production existant.
- Closed invariant : même sortie/errors pour chaque entrée de dossier.
- Before metrics : 105 lignes; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : celles de Phase P11.
- Dépendances : aucune.
- Outline : helper reçoit le root/job entry ou chemins dérivés; déplace la logique de génération/pending/checkpoint/manifest/coverage; retourne `Ok(Some(info))` ou `Ok(None)` pour les mêmes continuations; garde la boucle extérieure et l’accumulation ordonnée.
- Tests : conserver les tests de recovery existants, aucun test inline nouveau.
- Vérifications : `cargo fmt --check`, Clippy structurel six crates, N1 complet via garde.
- Provenance : Observed pour le diagnostic; Repo docs pour la garde.
- Acceptation : fonction ≤100; même choix latest pending et vérification next generation; gates restent autour du scan; résultats en même ordre; tests/N1 PASS; seul fichier ciblé modifié.
- Retour arrière : annuler l’extraction si un `continue`/`?` change la sémantique.
- Notes : préserver les deux noms campaign gate et les mêmes arguments `job_id/generation/0`.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE, disque | Repo docs | PASS | P11-T1 | témoin PE réel non exécuté |
| Format | fmt check gardé | Repo docs | PASS | P11-T1 | pas d’écriture rustfmt |
| Clippy | six crates, quatre lints | Observed/Repo docs | ≤125 sites | P11-T1 | GPU N2 hors lot |
| N1 | défaut et workers de référence | Repo docs | PASS | P11-T1 | tests ignorés préservés |
| Recovery | tests existants | Source inferred | génération/couverture/error identiques | P11-T1 | aucun test natif manuel |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P11-G1 | Validation | clôture | Gates et couverture gardent-ils mêmes bornes ? | Oui | diff et tests recovery |

## 9. Hors périmètre

Ne pas modifier le format durable, manifestes, hooks de campagne, stockage, règles de génération, API, ou tests voisins. Ne pas ajouter de journal ou de comportement de recovery.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | Fonction et unité par job nommées |
| Sections présentes | Pass | Modèle suivi |
| Traçabilité | Pass | un prévu, reste différé |
| Tâche complète | Pass | fichiers, outline, tests, acceptance, rollback |
| Mesures/arrêts | Pass | 105→≤100 |
| Budget | Pass | un fichier |
| Tests dédiés | Pass | tests recovery existants |
| Provenance | Pass | diagnostic et AGENTS |
| Porte | Pass | gates/couverture |
| Chemins | Pass | source vérifiée |
| Périmètre | Pass | un helper local |

## 11. Prompt d’implémentation

Exécute P11-T1 du plan `docs/cadrage/plan-clippy-structure-live-recovery-scan-20261113.md` dans `C:/wt/cs`. Préserve P1–P10. Modifie uniquement `archive/recovery/live.rs`; garde mêmes filtres, generations, erreurs, hooks et ordre. Exécute SelfTests, fmt, Clippy et N1 via la garde; arrête au premier échec.
