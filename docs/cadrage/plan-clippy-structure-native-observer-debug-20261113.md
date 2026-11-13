# Plan P6 — Réduire le formatage Debug de l’observateur natif

Avancée : cadrage d’une implémentation `Debug` de test au-dessus du seuil.
Blocage : aucun ; changements limités au support de tests.
Décision : répartir le formatage par familles de variantes, en gardant le texte Debug identique.

## En-tête

- Source audit : Clippy structurel frais après P5, `.../scratch/20261113T205628790-4ad5e66442a54a508b5b721e6bb8ec14/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-adapters`, worktree `C:/wt/cs`.
- Mode : plan uniquement.
- Confiance : high — diagnostic 108/100 et implémentation vérifiés dans les sources.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Le rapport post-P5 compte 131 sites. Un seul site proche du seuil dans le support concerné est `NativeQualificationEventKind::fmt` (`src/worker_ipc/tests/runtime_support.rs:64`, 108 lignes), un `Debug` manuel exhaustif sur les variantes d’un enum privé de test. Le test harness ne doit pas modifier les traces ni l’architecture de production.

## 2. Fraîcheur

Inspection ciblée de `runtime_support.rs`, de son inclusion dans les tests IPC et de `crates/whisper-adapters/AGENTS.md`. P1–P5 restent des changements locaux non commités à préserver. Les quatre crates N1 worker et le Clippy six crates restent les gates.

## 3. Hypothèses

- Safe : les bras du match peuvent être regroupés en helpers privés de formatage par responsabilité.
- Validation needed : chaînes Debug, noms de variantes, champs et ordre demeurent strictement identiques.
- Validation needed : le nouveau nombre de sites diminue de un et aucune alerte n’est introduite.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `Debug::fmt` 108/100 | P6 | Planned | P6-T1 | Extraire helpers privés de formatage |
| 130 autres sites | Hors lot | Deferred | Aucun | Lots ultérieurs |

## 5. Séquence recommandée

Une tâche garde le type privé et les sorties Debug. Aucun changement produit ou nouvelle assertion n’est prévu.

### Phase P6 : découper le formatage des événements natifs

Objective : supprimer l’alerte `too_many_lines`.
Closed invariant : `fmt` est ≤100 lignes et le rendu de chaque variante demeure identique.
Before metrics : 108 lignes; 131 sites structurels.
Target metrics : `fmt` ≤100; au plus 130 sites, sans nouvelle alerte.
Metric commands : Clippy structurel six crates sous la garde et revue des bras/motifs.
Stop conditions : arrêt si un champ, ordre, nom de variante ou rendu est modifié; si le périmètre dépasse runtime_support.rs; ou en cas d’échec de gate.
Dependencies : aucune.
Expected outcome : helpers privés de formatage pour familles de variantes.
Tasks : P6-T1.

## 6. Tâches

#### P6-T1 — Extraire le formatage Debug des variantes

- Priorité : Medium
- Source : `NativeQualificationEventKind::fmt`, Clippy 108/100.
- Justification : réduire la fonction tout en gardant une sortie de diagnostic stable.
- Fichiers : `crates/whisper-adapters/src/worker_ipc/tests/runtime_support.rs` seulement.
- Bucket : tests/support.
- Scope budget : un fichier de support existant; aucun fichier production.
- Closed invariant : fonction `fmt` ≤100 lignes, format inchangé.
- Before metrics : 108 lignes; target ≤100.
- Metric commands : Clippy structurel via garde.
- Stop conditions : voir Phase P6.
- Dépendances : aucune.
- Outline : répartir les bras exhaustifs par familles; créer les `DebugStruct` au nom actuel de chaque variante; préserver l’ordre des champs et le `.finish()`.
- Tests : aucun cas nouveau requis; garder les scénarios IPC existants inchangés.
- Vérifications : `cargo fmt --check`, Clippy structurel six crates, N1 complet via `ci/guard-level1.ps1`.
- Provenance : Repo docs pour garde/N1; Observed pour l’alerte.
- Acceptation : fmt ≤100; sortie Debug identique pour toutes les variantes; Clippy ne signale pas de nouveau site; N1 PASS; seul le fichier ciblé change.
- Retour arrière : retirer seulement l’extraction si elle change la sortie ou la visibilité.
- Notes : enum et implémentation demeurent `cfg(test)`; n’ajouter aucun API produit.

## 7. Matrice de vérification

| Domaine | Commande ou contrôle | Provenance | Attendu | Tâche | Limite |
|---|---|---|---|---|---|
| Préflight | SelfTests garde et contrôles PE | Repo docs | PASS | P6-T1 | témoin réel non exécuté |
| Format | `cargo fmt --check` via garde | Repo docs | PASS | P6-T1 | aucun formatage en écriture |
| Clippy | six crates, quatre lints structurels via garde | Observed/Repo docs | ≤130 sites et ratchet | P6-T1 | GPU N2 exclu |
| N1 | défaut et sélection workers CPU via garde | Repo docs | PASS | P6-T1 | ignorés restent ignorés |
| Sortie | revue de chaque variante/field | Source inferred | rendu byte-for-byte équivalent à l’expression actuelle | P6-T1 | pas d’assertion runtime dédiée |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut recommandé | Validation |
|---|---|---|---|---|---|
| P6-G1 | Validation | clôture | Noms/champs/ordre identiques ? | Oui | Revue différentielle des bras |

## 9. Hors périmètre

Ne pas modifier l’enum, l’observateur, la production, événements, journalisation, tests voisins, API, manifeste ou baseline. Pas de nouveau test métier.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | Fichier/fonction identifiés |
| Sections dans l’ordre | Pass | Modèle suivi |
| Constats tracés | Pass | 1 prévu, reste différé |
| Tâche complète | Pass | outline, vérifs, acceptation, rollback |
| Mesures et arrêts | Pass | 108→≤100 |
| Budget | Pass | un fichier |
| Tests séparés | Pass | support existant, aucun test inline nouveau |
| Provenance | Pass | docs et diagnostic observé |
| Gates explicites | Pass | revue de rendu |
| Chemins vérifiés | Pass | inspection ciblée |
| Portée petite | Pass | un support de test |

## 11. Prompt d’implémentation

Exécute P6-T1 de `docs/cadrage/plan-clippy-structure-native-observer-debug-20261113.md` dans `C:/wt/cs`. Préserve tous les autres changements. Modifie uniquement `src/worker_ipc/tests/runtime_support.rs`; découpe `Debug::fmt` sans changer le rendu ni élargir l’API. Exécute SelfTests, fmt, Clippy structurel et N1 via `ci/guard-level1.ps1`; arrête sur échec.
