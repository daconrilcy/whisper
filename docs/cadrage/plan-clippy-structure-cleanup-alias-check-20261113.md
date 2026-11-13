# Plan P8 — Extraire le contrôle d’alias du nettoyage

Avancée : plan ciblé sur une fonction de cleanup à 102 lignes.
Blocage : aucun ; le contrôle doit toujours précéder la suppression.
Décision : isoler la comparaison d’identités successeur/cibles dans un helper privé.

## En-tête

- Source audit : Clippy frais après P7, six crates, `.../scratch/20261113T211614809-98f4eeb18d4849caa62bcf00f0b5cb9e/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-adapters`, sous-module archive cleanup, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — code et garde d’alias existante inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Le compteur post-P7 est 129 sites. `run_cleanup_plan_locked` fait 102 lignes. Son bloc d’alias construit l’ensemble des identités fichier successeur/index et vérifie que les objets et conteneurs à supprimer n’en contiennent aucune.

## 2. Fraîcheur

Inspection de `archive/cleanup/execution.rs`, des tests `archive/tests/cleanup_equivalence.rs` et de `crates/whisper-adapters/AGENTS.md`. Les lots P1–P7 sont présents dans le worktree et doivent rester inchangés. Aucun état Windows natif ou GPU n’est requis pour cette extraction.

## 3. Hypothèses

- Safe : déplacer la construction d’identité et le parcours de cibles en fonction privée synchrone ne change pas les ressources ni l’ordre.
- Validation needed : même tuple `(volume_serial, file_id, is_directory)`, mêmes objets/containers inclus, même message `CleanupUnsafePath`.
- Validation needed : aucun retrait ne peut précéder ce contrôle.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `run_cleanup_plan_locked` 102 lignes | P8 | Planned | P8-T1 | Extraire le contrôle d’alias |
| 128 autres sites | Hors lot | Deferred | Aucun | Lots suivants |

## 5. Séquence recommandée

Une méthode auxiliaire privée portera uniquement le contrôle successeur vs cibles. Elle sera invoquée au même point, après validation/open du successeur et avant le préflight/retrait des cibles.

### Phase P8 : isoler le garde-fou d’alias du cleanup

Objective : supprimer l’alerte de longueur sans affaiblir le contrôle.
Closed invariant : l’ensemble exact des identités est contrôlé avant toute suppression.
Before metrics : fonction 102 lignes; 129 sites.
Target metrics : fonction ≤100 lignes; au plus 128 sites; pas de nouveau diagnostic.
Metric commands : Clippy structurel six crates, N1, tests de cleanup existants.
Stop conditions : arrêt si l’ensemble d’identités, l’ordre d’appel, le message d’erreur ou la position avant retrait change; ou gate en échec.
Dependencies : aucune.
Expected outcome : un helper privé dédié à l’absence d’alias.
Tasks : P8-T1.

## 6. Tâches

#### P8-T1 — Extraire la vérification d’alias successeur/cibles

- Priorité : Medium
- Source : Clippy 102/100 sur `run_cleanup_plan_locked`.
- Justification : rendre explicite le garde-fou de non-aliasing avant retrait et ramener la fonction sous le seuil.
- Fichiers : `crates/whisper-adapters/src/archive/cleanup/execution.rs` seulement.
- Bucket : adapters/infra.
- Scope budget : un fichier production existant.
- Closed invariant : aucun target object/container ne partage une identité avec successor directory/files/index proof.
- Before metrics : 102 lignes; target ≤100.
- Metric commands : Clippy via la garde.
- Stop conditions : ceux de Phase P8.
- Dépendances : aucune.
- Outline : créer `verify_cleanup_targets_do_not_alias_successor`; construire le même HashSet et parcourir les mêmes cibles/containers; l’appeler à la position actuelle immédiatement avant de maintenir les handles et de préparer le retrait.
- Tests : conserver les cas d’équivalence/alias existants; ne pas ajouter un comportement simulé.
- Vérifications : `cargo fmt --check`, Clippy structurel six crates et N1 via `ci/guard-level1.ps1`.
- Provenance : Observed pour diagnostic et test existant; Repo docs pour la garde.
- Acceptation : alert ciblée supprimée, aucune nouvelle alerte; erreur et contrôle identiques; appels de retrait toujours après le contrôle; N1 PASS; un seul fichier modifié.
- Retour arrière : restaurer uniquement l’extraction si ordre ou identité diffère.
- Notes : préserver le tuple identité incluant `is_directory`; ne pas relâcher le maintien des handles `VerifiedFile`.

## 7. Matrice de vérification

| Domaine | Contrôle | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE et espace disque | Repo docs | PASS | P8-T1 | témoin réel PE non exécuté |
| Format | fmt check gardé | Repo docs | PASS | P8-T1 | aucun format en écriture |
| Clippy | six crates, quatre lints via garde | Observed/Repo docs | ≤128 sites | P8-T1 | GPU N2 hors lot |
| N1 | défaut et workers de référence | Repo docs | PASS | P8-T1 | tests ignorés restent ignorés |
| Contrat cleanup | tests existants + revue d’ordre | Source inferred | erreurs/identités préservées | P8-T1 | pas de test Windows manuel ajouté |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P8-G1 | Validation | clôture | Vérification toujours avant retrait ? | Oui | appels ordonnés dans diff + tests |

## 9. Hors périmètre

Ne pas modifier l’algorithme d’identité, les chemins, preuves persistées, autres fonctions cleanup, suppression native ou baseline. Aucun autre fichier.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | Un bloc identifiable |
| Sections présentes | Pass | Modèle suivi |
| Constat tracé | Pass | 1 prévu, reste différé |
| Tâche complète | Pass | cible, tests, critères et rollback |
| Métriques et arrêts | Pass | 102→≤100 |
| Budget | Pass | un fichier |
| Tests séparés | Pass | tests existants de cleanup |
| Provenance | Pass | rapport/source et règle dépôt |
| Porte explicite | Pass | ordre avant retrait |
| Chemins valides | Pass | consultés localement |
| Périmètre borné | Pass | contrôle unique |

## 11. Prompt d’implémentation

Exécute P8-T1 de `docs/cadrage/plan-clippy-structure-cleanup-alias-check-20261113.md` dans `C:/wt/cs`. Préserve les lots précédents. Modifie seulement `archive/cleanup/execution.rs`; garde exactement les tuples, cibles, conteneurs, message d’erreur et position avant tout retrait. Passe format, Clippy structurel et N1 via la garde. Arrête sur tout échec.
