# Plan P7 — Découper les demandes de pages de session unifiée

Avancée : plan ciblé sur un helper UI de 103 lignes.
Blocage : aucun ; comportement de requête et de retry à préserver.
Décision : séparer demande de page texte et gestion/demande des pages audio.

## En-tête

- Source audit : Clippy frais après P6, journal `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261113T211614809-98f4eeb18d4849caa62bcf00f0b5cb9e/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-desktop`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — fonction, appels et état UI vérifiés dans la source.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Le Clippy structurel post-P6 compte 130 sites. `DesktopApp::request_unified_pages` fait 103 lignes (Clippy, `ui/model/view/unified.rs:145`). Il regroupe la validation/demande de page de document, le retry temporisé des pages audio après stale, et la demande normale de page audio.

## 2. Fraîcheur

Inspection de la fonction et des méthodes voisines dans `ui/model/view/unified.rs`, du `AGENTS.md` desktop et des tests UI existants. Les modifications P1–P6 restent non commitées et doivent rester intactes. Aucun routage ou comportement produit nouveau n’est visé.

## 3. Hypothèses

- Safe : les blocs document et audio peuvent être extraits en méthodes privées de `DesktopApp`.
- Validation needed : identité, révision, offset, phase, cache des requêtes, message stale, temporisation 250 ms et demande de repaint restent inchangés.
- Validation needed : les tests UI de stale et de sélection continuent de passer dans N1.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `request_unified_pages` 103 lignes | P7 | Planned | P7-T1 | Séparer document et audio |
| 129 autres sites | Hors lot | Deferred | Aucun | Lots ultérieurs |

## 5. Séquence recommandée

Une tâche extrait les deux responsabilités de lecture de pages tout en gardant la signature et résultat combiné de la façade si cela limite les appels. Aucun nouveau test ou changement de modèle n’est planifié.

### Phase P7 : isoler la demande de page document et audio

Objective : supprimer l’alerte `too_many_lines` du helper de rendu.
Closed invariant : helper `request_unified_pages` ≤100 lignes et comportement de pagination identique.
Before metrics : 103 lignes; 130 sites structurels.
Target metrics : ≤100 lignes; au plus 129 sites, aucune nouvelle alerte.
Metric commands : Clippy structurel six crates; tests UI par la matrice N1.
Stop conditions : arrêt si révision, offset, ordre des commandes, délai, repaint ou réessai change; échec d’une gate; besoin de nouveaux fichiers hors budget.
Dependencies : aucune.
Expected outcome : deux helpers privés pour page document et pages audio.
Tasks : P7-T1.

## 6. Tâches

#### P7-T1 — Extraire les requêtes de pages unifiées

- Priorité : Medium
- Source : Clippy 103/100 sur `request_unified_pages`.
- Justification : clarifier deux flux UI qui ont des règles distinctes sans changer leur temporisation.
- Fichiers : `crates/whisper-desktop/src/ui/model/view/unified.rs` seulement.
- Bucket : interface/view.
- Scope budget : un fichier de production UI; aucun autre fichier attendu.
- Closed invariant : fonction principale ≤100 lignes et mêmes commandes/résultats.
- Before metrics : 103 lignes; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : voir Phase P7.
- Dépendances : aucune.
- Outline : extraire le calcul et dispatch du document; extraire l’état stale/retry et la demande audio; conserver une façade claire qui renvoie `(document_current, parts_current)`.
- Tests : réutiliser les scénarios existants de `unified_sessions.rs`/`ui/tests`; n’ajouter aucun test inline.
- Vérifications : format, Clippy structurel six crates, N1 complet via `ci/guard-level1.ps1`.
- Provenance : Observed pour site; Repo docs pour garde/N1.
- Acceptation : alerte ciblée supprimée; requête document une fois par cache clé; audio stale re-tente après 250 ms et repaint; requête normale respecte révision/offset; N1 PASS; un seul fichier changé.
- Retour arrière : annuler uniquement les extractions si un appel, délai ou clé de requête diffère.
- Notes : ne pas modifier les politiques métier ni les commandes du cœur.

## 7. Matrice de vérification

| Domaine | Commande/contrôle | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE et ≥30 Gio | Repo docs | PASS | P7-T1 | témoin réel PE non exécuté |
| Format | `cargo fmt --check` via garde | Repo docs | PASS | P7-T1 | pas d’écriture rustfmt |
| Clippy | six crates, quatre lints structurels via garde | Observed/Repo docs | ≤129 sites | P7-T1 | GPU N2 non requis |
| N1 | défaut et workers CPU via garde | Repo docs | PASS | P7-T1 | tests ignorés inchangés |
| UI | tests existants de pages stale/unifiées | Source inferred | PASS | P7-T1 | rendu Windows manuel réservé à l’utilisateur |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P7-G1 | Validation | clôture | Les 250 ms et clés de retry sont-ils inchangés ? | Oui | revue diff et tests |

## 9. Hors périmètre

Ne pas modifier commands/types/politiques du cœur, le rendu textuel, sélection micro, les interfaces publiques, les autres lots ou la baseline. Pas de validation manuelle Windows prétendue.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | Fonction et invariant nommés |
| Sections dans l’ordre | Pass | Modèle suivi |
| Constat et différés tracés | Pass | 1 planifié, reste différé |
| Tâche avec cibles/vérifs/rollback | Pass | Inclus |
| Métriques et arrêts | Pass | 103→≤100 |
| Budget | Pass | un fichier |
| Tests dédiés | Pass | tests existants réutilisés |
| Provenance | Pass | source et règle du dépôt |
| Porte explicite | Pass | délai vérifiable |
| Chemins | Pass | constatés dans le dépôt |
| Périmètre borné | Pass | une méthode UI |

## 11. Prompt d’implémentation

Exécute P7-T1 du plan `docs/cadrage/plan-clippy-structure-unified-page-requests-20261113.md` dans `C:/wt/cs`. Préserve P1–P6. Modifie seulement `ui/model/view/unified.rs`; garde mêmes clés, commandes, retours, retry 250 ms et repaint. Passe fmt, Clippy structurel et N1 par `ci/guard-level1.ps1`; stoppe à tout échec.
