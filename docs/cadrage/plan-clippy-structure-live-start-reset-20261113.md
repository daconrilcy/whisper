# Plan P13 — Isoler la remise à zéro de démarrage live

Avancée : plan ciblé sur `ImportRuntime::start_live` (103 lignes).
Blocage : aucun ; tout état réinitialisé au démarrage doit rester identique.
Décision : extraire son bloc contigu d’initialisation en helper privé.

## En-tête

- Source audit : Clippy frais P12, 124 sites, `.../scratch/20261113T224317531-53c3bc3a06cb41939a0f9eb318b8dced/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-adapters/worker_ipc/runtime/live.rs`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — bloc contigu de remise à zéro inspecté.
- Qualité du plan : passée.

## 1. Résumé de l’audit

`ImportRuntime::start_live` fait 103 lignes. Après validation du choix CPU/GPU, elle démarre les diagnostics et remet à zéro le rapport dégradé, le superviseur de progression, compteurs de persistance, état de reset et résultats différés avant de construire le chemin et les ressources d’archive live.

## 2. Fraîcheur

Inspection du bloc lines 688–695, méthodes de `ImportRuntime` et tests de démarrage dans `worker_ipc/runtime/poll/tests` et `worker_ipc/tests`. P1–P12 sont à préserver; aucun changement de dépendance/API n’est requis.

## 3. Hypothèses

- Safe : `start_diagnostics(destination)` et les affectations contiguës peuvent être déplacés ensemble dans `reset_live_start_state`.
- Validation needed : l’appel reste après validation backend/identité GPU et avant l’accès au dossier/LiveArchive.
- Validation needed : tous les champs remis à zéro et l’ordre restent identiques.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `start_live` 103 lignes | P13 | Planned | P13-T1 | Extraire le bloc initialisation |
| 123 autres sites | Hors lot | Deferred | Aucun | Lots suivants |

## 5. Séquence recommandée

Déplacer sans modification le bloc contigu de remise à zéro dans un helper privé du même `impl`. Aucun autre découpage de sélection, stockage ou lancement worker.

### Phase P13 : nommer l’initialisation du runtime live

Objective : supprimer l’alerte de longueur de `start_live`.
Closed invariant : état runtime fresh avant préparation des fichiers/worker, sur le même chemin de succès et au même ordre.
Before metrics : 103 lignes; 124 sites.
Target metrics : ≤100 lignes; au plus 123 sites; aucune nouvelle alerte.
Metric commands : Clippy six crates, N1.
Stop conditions : arrêter si l’appel bouge avant sélection backend/GPU, après création de ressources, si un champ/ordre change, ou à tout échec campagne.
Dependencies : aucune.
Expected outcome : `reset_live_start_state(destination)` privé.
Tasks : P13-T1.

## 6. Tâches

#### P13-T1 — Extraire la remise à zéro d’état live

- Priorité : Medium
- Source : diagnostic 103/100 dans `runtime/live.rs:644`.
- Justification : nommer une responsabilité cohérente tout en ramenant la méthode sous le seuil.
- Fichiers : `crates/whisper-adapters/src/worker_ipc/runtime/live.rs` seulement.
- Bucket : adapters/infra — orchestration IPC.
- Scope budget : un fichier production.
- Closed invariant : les champs de démarrage sont remis à zéro avant préparation du dossier live.
- Before metrics : 103 lignes; target ≤100.
- Metric commands : Clippy via la garde.
- Stop conditions : celles de Phase P13.
- Dépendances : aucune.
- Outline : ajouter une méthode privée qui appelle `start_diagnostics` puis remet à zéro exactement `diagnostic_degraded_reported`, `progress`, `live_pending_persistence`, `pending_attempt_reset`, `pending_recovery_counters`, `deferred_live_stopped` et `deferred_live_window_finished`; remplacer le bloc actuel par cet appel au même endroit.
- Tests : garder les scénarios start_live/recovery existants; aucun nouveau test inline.
- Vérifications : `cargo fmt --check`, Clippy structurel six crates, N1 complet via garde.
- Provenance : Observed pour l’alerte/bloc; Repo docs pour les gates.
- Acceptation : aucun état omis; ordre d’appel identique; même sélection CPU/GPU et erreurs; site Clippy supprimé; N1 et scénarios passent; seul fichier ciblé modifié.
- Retour arrière : annuler seulement l’extraction si l’ordre ou un champ diffère.
- Notes : l’identité GPU doit être validée avant cette remise à zéro, comme aujourd’hui.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE, espace disque | Repo docs | PASS | P13-T1 | témoin PE réel non exécuté |
| Format | fmt check via garde | Repo docs | PASS | P13-T1 | pas d’écriture rustfmt |
| Clippy | six crates, lints structurels | Observed/Repo docs | ≤123 sites | P13-T1 | N2 GPU exclu |
| N1 | défaut + workers référence | Repo docs | PASS | P13-T1 | native ignored inchangé |
| Runtime | tests start/recovery existants | Source inferred | PASS | P13-T1 | aucun build DLL natif |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P13-G1 | Validation | clôture | Bloc identique et situé au même point ? | Oui | revue diff/tests |

## 9. Hors périmètre

Ne pas modifier le choix de backend, les règles Auto/GPU, vérifications d’identité, création archive, spawn worker, diagnostics ou nouveaux tests. Pas de feature/manifest.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | bloc/fonction nommés |
| Sections présentes | Pass | modèle suivi |
| Traçabilité | Pass | 1 prévu, reste différé |
| Tâche complète | Pass | fichiers/gates/acceptation/rollback |
| Métriques et arrêts | Pass | 103→≤100 |
| Budget | Pass | un fichier |
| Tests séparés | Pass | tests existants réutilisés |
| Provenance | Pass | rapport/source et AGENTS |
| Porte | Pass | ordre/champs vérifiables |
| Chemins | Pass | constatés |
| Scope | Pass | bloc d’initialisation seul |

## 11. Prompt d’implémentation

Exécute P13-T1 de `docs/cadrage/plan-clippy-structure-live-start-reset-20261113.md` dans `C:/wt/cs`. Préserve P1–P12. Modifie seulement `worker_ipc/runtime/live.rs`; déplace le bloc d’initialisation sans changer champs, ordre ni point d’appel. Passe SelfTests, fmt, Clippy et N1 via garde; arrête au premier échec.
