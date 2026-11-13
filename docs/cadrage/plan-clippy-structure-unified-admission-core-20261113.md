# Plan P14 — Isoler le refus d’admission d’une capture unifiée

Avancée : plan ciblé sur `start_unified_live_command` (107 lignes).
Blocage : aucun ; préserver les critères d’admission avant toute soumission d’effet.
Décision : extraire le prédicat de capacité/conflit dans un helper privé.

## En-tête

- Source audit : Clippy frais post-P13, 123 sites, `.../scratch/20261113T225254188-dcc6b0bb2c9c40ae8a37fe79eeee54cd/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-core/application/runtime/unified.rs`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — condition et suites de contrats existantes inspectées.
- Qualité du plan : passée.

## 1. Résumé de l’audit

`start_unified_live_command` fait 107 lignes. Pour une requête `StartUnifiedLiveConfigured`, elle calcule si un import peut partager le moteur, s’il existe déjà une capture ouverte, si le plafond de deux sessions est atteint, s’il existe une carte terminale évictable, et si l’identité est dupliquée/active; sinon elle soumet l’effet puis initialise la vue microphone, l’admission et la carte de session.

## 2. Fraîcheur

Inspection de la fonction, des tests du runtime unifié et `crates/whisper-core/AGENTS.md`. P1–P13 à préserver; aucune API publique ne doit être ajoutée. L’ordre validation→submit→mutations de vue est un invariant.

## 3. Hypothèses

- Safe : extraire uniquement les conditions et retourner `Err(InvalidCommand)` dans une méthode privée `validate_unified_live_admission`.
- Validation needed : conserver strictement le partage d’import bounded-window, maximum 2, capture ouverte, cartes terminales évictables, doublon identité et active identity.
- Validation needed : le helper ne mute aucun état et s’exécute avant `io.submit`.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| Handler unifié 107 lignes | P14 | Planned | P14-T1 | Extraire validation d’admission |
| 122 autres sites | Hors lot | Deferred | Aucun | Prochaines tranches |

## 5. Séquence recommandée

Conserver l’ordre des calculs/politiques dans le helper privé et laisser le handler appliquer l’effet et construire l’état de session. Ne pas modifier le seuil de sessions ou l’éviction.

### Phase P14 : séparer validation et application de l’admission

Objective : supprimer l’alerte de longueur.
Closed invariant : toute requête interdite est rejetée avant submit ou mutation de vue; toute requête autorisée garde le même effet et la même admission.
Before metrics : 107 lignes; 123 sites.
Target metrics : ≤100 lignes; au plus 122 sites; aucun nouveau diagnostic.
Metric commands : Clippy structurel six crates et N1.
Stop conditions : arrêter si un calcul/admission change, si erreur déplacée après submit, si mutation arrive avant validation, ou gate en échec.
Dependencies : aucune.
Expected outcome : helper privé de validation; handler allégé.
Tasks : P14-T1.

## 6. Tâches

#### P14-T1 — Extraire la validation d’admission unifiée

- Priorité : Medium
- Source : `start_unified_live_command`, 107 lignes.
- Justification : séparer le garde-fou des effets et initialisations sans déplacer les politiques.
- Fichiers : `crates/whisper-core/src/application/runtime/unified.rs` seulement.
- Bucket : application/domain coordination.
- Scope budget : un fichier production.
- Closed invariant : refus avant tout effet et mêmes admissibilités.
- Before metrics : 107 lignes; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : Phase P14.
- Dépendances : aucune.
- Outline : déplacer intact le calcul de `import_can_share`, `open_capture_exists`, `admitted_sessions`, `no_terminal_card_to_evict` et toute la condition dans une méthode privée prenant `&crate::ports::LiveRequest` et retournant `Result<(), ApplicationError>`; remplacer le bloc par l’appel immédiatement avant la création de l’identité/submit.
- Tests : garder les scénarios de refus/admission des tests runtime existants; pas de test inline supplémentaire.
- Vérifications : fmt, Clippy six crates, N1 via garde.
- Provenance : Observed pour le handler et l’alerte; Repo docs pour les gates.
- Acceptation : même ensemble de conditions et d’ordre; aucune mutation/submit avant Ok; mêmes scénarios de partage/max2/éviction/doublon; diagnostic Clippy supprimé; N1 PASS; seul fichier ciblé modifié.
- Retour arrière : annuler extraction si un cas d’admission ou ordre change.
- Notes : ne pas rendre visible le helper au-delà du module; ne pas élargir les types publics.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE et disque | Repo docs | PASS | P14-T1 | témoin PE réel non exécuté |
| Format | fmt check via garde | Repo docs | PASS | P14-T1 | pas d’écriture rustfmt |
| Clippy | six crates et lints structurels | Observed/Repo docs | ≤122 sites | P14-T1 | N2 GPU exclu |
| N1 | tests default et worker | Repo docs | PASS | P14-T1 | ignored préservés |
| Admission | scénarios runtime existants | Source inferred | résultats identiques | P14-T1 | pas de GUI Windows |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P14-G1 | Validation | clôture | L’admission est-elle vérifiée avant effet ? | Oui | lecture ordre + tests |

## 9. Hors périmètre

Ne pas modifier politique de plafond, types de session, stratégie d’import, eviction, commandes IPC ou vues; aucune API/dépendance nouvelle.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | méthode/conditions nommées |
| Sections présentes | Pass | modèle suivi |
| Traçabilité | Pass | prévu/différé |
| Tâche complète | Pass | fichiers, vérifs, critères, rollback |
| Invariant/mesures/stop | Pass | 107→≤100 |
| Scope budget | Pass | un fichier |
| Tests dédiés | Pass | runtime tests existants |
| Provenance | Pass | sources/AGENTS |
| Porte explicite | Pass | avant effet |
| Chemins | Pass | consultés localement |
| Périmètre | Pass | validation seulement |

## 11. Prompt d’implémentation

Exécute P14-T1 de `docs/cadrage/plan-clippy-structure-unified-admission-core-20261113.md` dans `C:/wt/cs`. Préserve P1–P13. Modifie uniquement `application/runtime/unified.rs`; extrais la validation sans changer les prédicats ni l’ordre avant submit. Passe SelfTests, fmt, Clippy et N1 par la garde; arrête à tout échec.
