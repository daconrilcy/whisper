# Plan P19 — Isoler la validation du passage live précédent

Avancée : plan ciblé sur `LiveArchive::prepare` (118 lignes).
Blocage : aucun ; reprise de génération, intégrité des artefacts et pending irrésolu doivent garder les mêmes résultats.
Décision : extraire la validation du précédent vers un helper privé.

## En-tête

- Source audit : Clippy frais post-P18, 118 sites, `.../scratch/20261114T104731043-3be67048084b4b238cf1f33634f7af87/cargo-command.stdout.log`.
- Date : 14 novembre 2026.
- Espace : `whisper-adapters/archive/live.rs`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — contrôles génération précédente et tests `tests/live_archive.rs` inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

`LiveArchive::prepare` valide l’archive, crée/locke le dossier du job, puis pour une génération supérieure à 1 vérifie le manifeste précédent, l’identité job/génération/hash, le pointeur CURRENT, les cinq artefacts, et met à jour l’offset groupal. Si le précédent n’est pas commis mais garde un pending, elle vérifie l’identité et bloque avec un diagnostic Recoverable jusqu’à reconciliation. Puis seulement elle crée le pending de la nouvelle génération et le MP3.

## 2. Fraîcheur

Inspection de `archive/live.rs`, `tests/live_archive.rs` (scénarios multi-génération et pending), AGENTS adapters. P1–P18 à préserver. Le verrou writer reste acquis avant les contrôles de génération.

## 3. Hypothèses

- Safe : le bloc `generation > 1` peut devenir `resolve_previous_live_passage(directory, &mut request) -> Result<Option<u64>, String>`.
- Validation needed : le retour Option correspond exactement à `previous_completed_at_ms` (None si aucun manifest antérieur commis).
- Validation needed : le helper est appelé après prise du verrou et avant création des fichiers de la nouvelle génération.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `LiveArchive::prepare` 118 lignes | P19 | Planned | P19-T1 | Extraire résolution précédente |
| 117 autres sites | Hors lot | Deferred | Aucun | Prochaines tranches |

## 5. Séquence recommandée

Extraire uniquement la branche de résolution du passage précédent; conserver dans prepare l’assertion archive, verrou, création du nouveau pending/MP3 et construction Self.

### Phase P19 : résoudre le précédent avant le nouveau pending

Objective : supprimer l’alerte de longueur sans altérer la reprise.
Closed invariant : aucun nouveau passage ne commence avant validation du tail précédent ou erreur Recoverable explicite.
Before metrics : 118 lignes; 118 sites.
Target metrics : prepare ≤100 lignes; au plus 117 sites, sans nouvelle alerte.
Metric commands : Clippy six crates; N1 inclut `tests/live_archive.rs`.
Stop conditions : arrêt si validation d’identité/artifacts/CURRENT, offset, retour completed_at, pending unresolved, lock order ou erreurs changent.
Dependencies : aucune.
Expected outcome : helper privé pur sauf lectures/vérifications déjà présentes; pas d’écriture nouvelle.
Tasks : P19-T1.

## 6. Tâches

#### P19-T1 — Extraire la résolution de la génération antérieure

- Priorité : Medium
- Source : `LiveArchive::prepare`, Clippy 118/100.
- Justification : séparer la reprise du tail antérieur de l’initialisation de la nouvelle capture.
- Fichiers : `crates/whisper-adapters/src/archive/live.rs` seulement.
- Bucket : adapters/infra — archive live.
- Scope budget : un fichier de production existant.
- Closed invariant : le dernier passage commis reste valide et un pending précédent reste explicitement bloquant.
- Before metrics : 118 lignes; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : celles de Phase P19.
- Dépendances : aucune.
- Outline : créer `resolve_previous_live_passage(directory, &mut request)`; déplacer toute la logique gen>1, vérifications et mise à jour offset; retourner `Some(completed_at_unix_ms)` uniquement après succès du précédent commis; conserver les mêmes `Err` pour identité corrompue/pending unresolved; appeler au point actuel après le writer lock.
- Tests : conserver les scénarios `LiveArchive::prepare` dans `tests/live_archive.rs`; aucun test inline nouveau.
- Vérifications : fmt, Clippy structurel six crates, N1 complet via garde.
- Provenance : Observed pour Clippy/test/source; Repo docs pour gates.
- Acceptation : même lock avant lecture; même validations, artefacts, messages et group offset; pending précédent demeure Recoverable; aucun fichier nouvelle génération créé avant résolution; N1 PASS; seul fichier nommé modifié.
- Retour arrière : restaurer uniquement l’extraction si l’ordre/erreur diffère.
- Notes : préserver le fait qu’une publication manifest + CURRENT est la preuve de tail commis; aucun contenu PCM seul ne vaut commit.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE et espace | Repo docs | PASS | P19-T1 | témoin réel non exécuté |
| Format | fmt check via garde | Repo docs | PASS | P19-T1 | aucun fmt écrivant |
| Clippy | six crates et lints structurels | Observed/Repo docs | ≤117 sites | P19-T1 | GPU N2 exclu |
| N1 | default et workers CPU | Repo docs | PASS | P19-T1 | tests ignorés inchangés |
| Archive live | scénarios multi-génération/pending | Source inferred | PASS | P19-T1 | aucun modèle/worker natif lancé |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P19-G1 | Validation | clôture | Le tail précédent reste-t-il requis avant création pending ? | Oui | tests et ordre source |

## 9. Hors périmètre

Ne pas changer schéma manifest/pending, nombres d’artefacts, CURRENT, locks, offset temporel, reconciliation ou publication. Aucun test manuel Windows affirmé.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | bloc/helper nommés |
| Sections présentes | Pass | modèle suivi |
| Traçabilité | Pass | 1 prévu, reste différé |
| Tâche complète | Pass | outline/gates/criteria/rollback |
| Mesures/arrêts | Pass | 118→≤100 |
| Budget | Pass | un fichier |
| Tests dédiés | Pass | integration tests existants |
| Provenance | Pass | rapport/source/docs |
| Porte claire | Pass | avant nouveau pending |
| Chemins | Pass | observés localement |
| Scope | Pass | précédent seulement |

## 11. Prompt d’implémentation

Exécute P19-T1 du plan `docs/cadrage/plan-clippy-structure-live-archive-prepare-20261114.md` dans `C:/wt/cs`. Préserve P1–P18. Modifie uniquement `archive/live.rs`; extrais gen>1 intact, conserve verrou avant helper et nouveau pending après. Passe SelfTests, fmt, Clippy et N1 par garde; stoppe au premier échec.
