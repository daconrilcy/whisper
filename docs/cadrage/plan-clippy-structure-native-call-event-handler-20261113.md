# Plan P18 — Isoler le traitement de NativeCallStarted

Avancée : plan ciblé sur le poll Parakeet (114 lignes).
Blocage : aucun ; liaison requête/worker/PID et attestation GPU à préserver.
Décision : extraire la validation de `NativeCallStarted` dans un helper privé.

## En-tête

- Source audit : Clippy frais P17, 119 sites, `.../scratch/20261113T235308291-8ca18977885d42fca6670f61b6471b06/cargo-command.stdout.log`.
- Date : 14 novembre 2026.
- Espace : `whisper-adapters/worker_ipc/runtime/poll/parakeet.rs`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — branche complète et tests de corrélation observés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

`poll_parakeet_worker` traite d’abord hooks de recette, file d’événements, ACK Stopped et lecture du process, puis route Ready/Result/Failed/NativeCallStarted/Stopped. La branche NativeCallStarted vérifie request id, identité job/génération/attempt, existence de la requête courante, attestation GPU et PID du worker avant de marquer le début de l’appel natif.

## 2. Fraîcheur

Inspection du handler, des helpers événementiels, du test `poll_parakeet_native_call_started_binds_request_and_worker_pid` et AGENTS adapters. P1–P17 à préserver. Aucun nouveau protocole ou état.

## 3. Hypothèses

- Safe : le bloc de corrélation devient un helper privé du même `impl ImportRuntime`.
- Validation needed : invalide retourne le même `ProtocolMismatch`, valide met le booléen à true et retourne None.
- Validation needed : `request_id==1`, job, génération, tentative, attestation GPU et PID restent tous nécessaires.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| poll Parakeet 114 lignes | P18 | Planned | P18-T1 | Extraire branche NativeCallStarted |
| 118 autres sites | Hors lot | Deferred | Aucun | Tranches ultérieures |

## 5. Séquence recommandée

Déplacer la validation d’événement corrélé dans un helper, en gardant la lecture du process et le routing principal dans `poll_parakeet_worker`.

### Phase P18 : isoler la corrélation de l’appel natif

Objective : supprimer l’alerte de longueur.
Closed invariant : `parakeet_native_call_started` ne s’active que pour l’événement attaché au worker/requête courants.
Before metrics : 114 lignes; 119 sites.
Target metrics : méthode ≤100 lignes; au plus 118 sites; aucune nouvelle alerte.
Metric commands : Clippy six crates et N1.
Stop conditions : arrêt si l’une des six conditions corrélation/attestation est retirée, si erreur/None diffère, ou gate échoue.
Dependencies : aucune.
Expected outcome : helper privé de gestion NativeCallStarted.
Tasks : P18-T1.

## 6. Tâches

#### P18-T1 — Extraire la validation NativeCallStarted

- Priorité : Medium
- Source : Clippy 114/100 sur `poll_parakeet_worker`.
- Justification : clarifier la vérification d’identité d’un événement de worker avant de changer l’état.
- Fichiers : `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs` seulement.
- Bucket : adapters/infra — protocole worker.
- Scope budget : un fichier production existant.
- Closed invariant : aucun worker/request ancien ou non attesté ne marque l’appel natif actif.
- Before metrics : 114; target ≤100.
- Metric commands : Clippy via la garde.
- Stop conditions : Phase P18.
- Dépendances : aucune.
- Outline : créer helper prenant request_id, job_id, generation, attempt et pid; déplacer le test d’active.request, id exact, generation, attempt, GPU attested si CUDA et processus PID; erreur et mutation booléenne inchangées; remplacer uniquement cette branche par le helper.
- Tests : garder le test de liaison et tests stale/inactive voisins; aucun nouveau test inline.
- Vérifications : fmt, Clippy six crates, N1 via garde.
- Provenance : Observed pour l’alerte/test; Repo docs pour les gates.
- Acceptation : helper empêche chaque mismatch; texte d’erreur identique; événement valide positionne booléen et n’émet aucun WorkerEvent; test existant/N1 PASS; aucune autre modification.
- Retour arrière : restaurer la branche inline si le résultat diffère.
- Notes : conserver `active.backend != Cuda || active.gpu_attested` et la vérification du pid enfant.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE et disque | Repo docs | PASS | P18-T1 | PE réel non exécuté |
| Format | fmt check gardé | Repo docs | PASS | P18-T1 | aucune écriture rustfmt |
| Clippy | six crates, lints structurels | Observed/Repo docs | ≤118 sites | P18-T1 | N2 GPU non demandé |
| N1 | défaut et workers référence | Repo docs | PASS | P18-T1 | ignorés restent ignorés |
| Protocole | tests corrélation existants | Source inferred | PASS | P18-T1 | aucune fixture native |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P18-G1 | Validation | clôture | mismatch job/gen/attempt/GPU/PID toujours fatal ? | Oui | test et lecture helper |

## 9. Hors périmètre

Ne pas modifier la configuration de worker, protocole, types IPC, recette-hooks, process polling général ou comportement Stopped/Ready/Result.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | branche et invariant nommés |
| Sections présentes | Pass | modèle suivi |
| Traçabilité | Pass | 1 prévu, autres différés |
| Tâche complète | Pass | outline, gates, acceptation, rollback |
| Métriques/arrêt | Pass | 114→≤100 |
| Budget | Pass | un fichier |
| Tests dédiés | Pass | poll tests existants |
| Provenance | Pass | source/report/docs |
| Porte | Pass | correlations nommées |
| Chemins | Pass | vérifiés |
| Scope | Pass | une branche |

## 11. Prompt d’implémentation

Exécute P18-T1 du plan `docs/cadrage/plan-clippy-structure-native-call-event-handler-20261113.md` dans `C:/wt/cs`. Préserve P1–P17. Modifie uniquement `poll/parakeet.rs`; extrais la branche NativeCallStarted sans changer les contrôles, erreur ou état. Passe SelfTests, fmt, Clippy et N1 via garde; arrête sur tout échec.
