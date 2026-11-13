# Plan P16 — Isoler la mise à jour de propriété d’un export interrompu

Avancée : plan borné pour `reconfirm_interrupted_publication` (107 lignes).
Blocage : aucun ; contrôle d’ownership, empreintes et ordre d’écriture à préserver.
Décision : déplacer le bloc d’ownership en helper privé du même module.

## En-tête

- Source audit : Clippy frais P15, 121 sites, `.../scratch/20261113T232614755-4c875e5aef464fdb8a4a14655372f54d/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-adapters/archive/publication/catalog.rs`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — bloc conditionnel d’ownership et test de reprise inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Clippy relève 107 lignes sur `ArchiveStore::reconfirm_interrupted_publication`. Après confirmation du choix/nouveau basename et revalidation de l’intention, la fonction vérifie si le record d’ownership existe; si oui, elle contrôle job/génération/source/name, met à jour version et identités temporaires, vérifie les SHA-256 txt/srt, puis remplace le record avant d’écrire l’intention mise à jour.

## 2. Fraîcheur

Inspection de `catalog.rs`, test `archive/tests/recovery.rs` autour de la reconfirmation et `crates/whisper-adapters/AGENTS.md`. P1–P15 restent à préserver. Ce lot ne change ni protocole ni format durable.

## 3. Hypothèses

- Safe : le bloc conditionnel entier peut être déplacé vers un helper privé de mise à jour d’ownership.
- Validation needed : même validation des quatre champs, même version=3, identités txt/srt temporaires, SHA, erreur et ordre d’écriture.
- Validation needed : l’intention durable est encore écrite après la mise à jour d’ownership, comme avant.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| Reconfirmation 107 lignes | P16 | Planned | P16-T1 | Extraire ownership conditionnel |
| 120 autres sites | Hors lot | Deferred | Aucun | Tranches ultérieures |

## 5. Séquence recommandée

Un helper privé regroupe la lecture, validation et réécriture conditionnelles de l’ownership. Les contrôles d’intention, choix de nom, choix utilisateur et écriture finale demeurent dans la fonction publique.

### Phase P16 : extraire l’ownership durable

Objective : supprimer l’alerte de longueur.
Closed invariant : record ownership modifié uniquement après validation complète et avec les mêmes identités/hashes.
Before metrics : 107 lignes; 121 sites structurels.
Target metrics : fonction ≤100 lignes; au plus 120 sites; pas de nouvelle alerte.
Metric commands : Clippy six crates, N1 et tests recovery.
Stop conditions : arrêt si validation, SHA, version, identité, ordre d’écriture ou erreurs changent; ou échec de gate.
Dependencies : aucune.
Expected outcome : helper privé `reconfirm_installed_export_ownership`.
Tasks : P16-T1.

## 6. Tâches

#### P16-T1 — Extraire la reconfirmation de propriété installée

- Priorité : Medium
- Source : `reconfirm_interrupted_publication`, Clippy 107/100.
- Justification : isoler la vérification conditionnelle et rendre l’orchestration de reprise lisible.
- Fichiers : `crates/whisper-adapters/src/archive/publication/catalog.rs` seulement.
- Bucket : adapters/infra — persistance de publication.
- Scope budget : un fichier production existant.
- Closed invariant : les fichiers visibles restent ambigus jusqu’à confirmation, et leurs métadonnées restent vérifiées avant réécriture.
- Before metrics : 107; target ≤100.
- Metric commands : Clippy via la garde.
- Stop conditions : Phase P16.
- Dépendances : aucune.
- Outline : déplacer intact le `if ownership_path.is_file()` en helper privé; passer les valeurs utiles observées; préserver la mise à jour de l’intention après retour de helper.
- Tests : conserver le scénario de recovery/reconfirmation; aucun nouveau test inline.
- Vérifications : fmt check, Clippy structurel six crates, N1 par `ci/guard-level1.ps1`.
- Provenance : Observed pour diagnostic/test; Repo docs pour la garde.
- Acceptation : toutes les erreurs et contrôles d’ownership/SHA inchangés; version 3 et identities temporaires inchangées; ordre ownership puis intention identique; test existant/N1 PASS; seul fichier cible modifié.
- Retour arrière : annuler seulement l’extraction si un champ ou ordre diffère.
- Notes : ne pas utiliser l’égalité de contenu comme preuve d’appartenance; garder le contrôle des deux fichiers staging.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE et espace | Repo docs | PASS | P16-T1 | témoin réel non exécuté |
| Format | fmt check via garde | Repo docs | PASS | P16-T1 | pas de `cargo fmt` écrivant |
| Clippy | six crates, quatre lints structurels | Observed/Repo docs | ≤120 sites | P16-T1 | GPU N2 exclu |
| N1 | default et sélections workers | Repo docs | PASS | P16-T1 | natifs ignorés |
| Recovery | test de reconfirmation existant | Source inferred | PASS | P16-T1 | aucun export manuel |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P16-G1 | Validation | clôture | Ownership revalidé avant write intent ? | Oui | ordre du diff et test recovery |

## 9. Hors périmètre

Ne pas changer le schéma ownership, noms de fichier, SHA, politique utilisateur, atomic writes, format durable, tests nouveaux, dépendances ou baseline.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | fonction/helper/contrôles nommés |
| Sections présentes | Pass | modèle suivi |
| Traçabilité | Pass | 1 planifié, reste différé |
| Tâche complète | Pass | étapes, tests, vérifs, rollback |
| Mesure/stop | Pass | 107→≤100 |
| Budget | Pass | un fichier |
| Tests dédiés | Pass | recovery test existant |
| Provenance | Pass | Clippy/source/docs |
| Porte | Pass | ordre d’écriture |
| Chemins | Pass | observés |
| Scope | Pass | ownership conditionnel seul |

## 11. Prompt d’implémentation

Exécute P16-T1 de `docs/cadrage/plan-clippy-structure-reconfirm-ownership-20261113.md` dans `C:/wt/cs`. Préserve P1–P15. Modifie uniquement `archive/publication/catalog.rs`; déplace le bloc conditionnel ownership sans changer validation/champs/SHA/ordre d’écriture. Passe SelfTests, fmt, Clippy et N1 via la garde; arrête au premier échec.
