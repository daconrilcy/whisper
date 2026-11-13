# Plan P4 — Clippy du contrat de capacités moteur

Avancée : plan prêt pour un seul test d’intégration structurel.
Blocage : aucun ; le code est hors de ce document.
Décision : extraction locale sans changement de comportement, N1 et Clippy gardés.

## En-tête

- Source audit : Clippy structurel frais, six crates, exécution du 13 novembre 2026 à 20:14 UTC, journal `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261113T201435205-3ab809c4e941406eaa9cb4b51f58d60f/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : crate `whisper-core`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — le diagnostic et la fonction sont présents dans le journal et le fichier courant.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Le Clippy frais mesure 133 sites structurels uniques avant P4. Le contrat public du cœur expose `project` et `validate`; le test d’intégration vérifie leur cohérence pour les langues automatique et manuelle. Un test atteint 107 lignes dans `crates/whisper-core/tests/engine_capabilities.rs:115`. Le reste de la dette Clippy est hors de ce lot.

## 2. Fraîcheur

Inspection ciblée de `engine_capabilities.rs`, de `crates/whisper-core/AGENTS.md` et du journal frais. Les alertes répétées pour plusieurs cibles compilées ne représentent qu’un site source. Le worktree contient déjà les modifications des lots P1–P3 ; elles ne doivent pas être touchées, sauf le fichier partagé explicitement autorisé si l’inclusion du nouveau support l’exige.

## 3. Hypothèses

- Safe : le scénario peut être séparé en helpers privés de test sans modifier les fonctions produit ni les assertions.
- Validation needed : le résultat doit garder les mêmes cas, erreurs attendues et préférences projetées.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `auto_and_manual_language_are_independent_and_projection_matches_admission`, 107 lignes | P4 | Planned | P4-T1 | Extraire le montage/assertions en helpers de test privés |
| 132 autres sites structurels | Hors lot | Deferred | Aucun | Reprendre par lots dédiés après clôture P4 |

## 5. Séquence recommandée

Une seule tâche réduit l’unique invariant mesurable : aucun test de ce fichier ne dépasse 100 lignes. Pas de migration d’architecture ni de changement de production.

### Phase P4 : raccourcir le scénario de langue

Objective : supprimer l’alerte `too_many_lines` ciblée.
Closed invariant : le test ciblé fait au plus 100 lignes et conserve ses assertions comportementales.
Before metrics : 107 lignes pour le test ; 133 sites structurels frais.
Target metrics : test ciblé ≤100 lignes ; 132 sites au plus, sans nouvelle alerte.
Metric commands : Clippy structurel six crates et comptage source par le diagnostic Clippy.
Stop conditions : arrêt si la séparation change les données ou assertions, si un échec préexistant apparaît, ou si des fichiers hors budget sont nécessaires.
Dependencies : aucune.
Expected outcome : extraction de test privée, visibilité limitée au fichier de test.
Tasks : P4-T1.

## 6. Tâches

#### P4-T1 — Extraire le scénario de langue automatique/manuelle

- Priorité : Medium
- Source : Clippy frais, `engine_capabilities.rs:115`.
- Justification : fermer une alerte structurelle du test de contrat sans toucher au comportement produit.
- Fichiers : `crates/whisper-core/tests/engine_capabilities.rs` et, seulement si nécessaire, son support de test adjacent.
- Bucket : tests/support.
- Scope budget : un fichier de test principal plus un support directement associé, aucun fichier production.
- Invariant fermé : test ciblé ≤100 lignes, contenu sémantique préservé.
- Mesure avant : 107 lignes ; cible ≤100.
- Commandes métriques : Clippy structurel via la garde N1.
- Conditions d’arrêt : celles de Phase P4 ; ne pas élargir la visibilité produit.
- Dépendances : aucune.
- Plan : extraire les arrangements/assertions cohérents dans des fonctions d’assistance privées; garder le scénario et ses attentes lisibles; vérifier le diff pour confirmer que les deux branches (mode manuel accepté/mode automatique refusé puis inverse) sont toujours couvertes.
- Tests : conserver le test de contrat existant et ses cas; aucun test nouveau nécessaire.
- Vérifications : `cargo fmt --check` et N1 complet via `ci/guard-level1.ps1`, avec Clippy structurel six crates selon la matrice déjà autorisée.
- Provenance : Repo docs pour garde/N1 ; Observed pour le site et le seuil Clippy.
- Acceptation : test ≤100 lignes; Clippy frais ne signale plus ce site ni n’ajoute d’alerte; N1 passe; aucun fichier non prévu modifié.
- Retour arrière : rétablir seulement l’extraction P4 si une assertion ou un résultat change.
- Notes : préserver les valeurs de génération 7/8, la distinction unsupported/qualified et les causes exactes de refus.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Résultat attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Format | `cargo fmt --check` sous garde | Repo docs | succès | P4-T1 | garde obligatoire |
| N1 | matrice CPU décrite dans AGENTS.md via `ci/guard-level1.ps1` | Repo docs | réussite complète | P4-T1 | GPU N2 non demandé |
| Clippy structurel | commande de la matrice Clippy six crates via garde | Observed/Repo docs | ratchet sans nouvelle alerte | P4-T1 | compile/test natif ignoré inchangé |
| Contrat | revue des assertions existantes et du diff | Source inferred | scénarios identiques | P4-T1 | aucun essai dans fenêtre Windows |

## 8. Portes de décision

| Porte | État | Bloque | Question | Valeur recommandée | Validation |
|---|---|---|---|---|---|
| P4-G1 | Validation | clôture | Helpers privés suffisants ? | Oui, sans API publique | Revue diff et Clippy |

## 9. Hors périmètre

Ne pas modifier les fonctions de production `project`/`validate`, les fixtures partagées, les autres tests, les manifestes ou la baseline Clippy. Ne pas ajouter d’exception au ratchet.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan exécutable, non placeholder | Pass | Une fonction et un invariant |
| Sections présentes | Pass | Structure conforme au modèle |
| Constat relié à une tâche ou différé | Pass | 1 prévu, reste différé explicitement |
| Tâche a fichiers, vérification, critères et retour arrière | Pass | Inclus |
| Métriques et arrêt mesurables | Pass | 107→≤100 |
| Budget explicite | Pass | 1 fichier + support optionnel |
| Tests dédiés | Pass | Test d’intégration existant préservé |
| Provenance des commandes | Pass | Garde et exécution observée |
| Portes et limites claires | Pass | Validation seulement |
| Chemins valides | Pass | Vérifiés dans le worktree |
| Périmètre borné | Pass | Aucun code production |

## 11. Prompt d’implémentation

Exécute P4-T1 de `docs/cadrage/plan-clippy-structure-engine-capabilities-test-20261113.md`. Préserve les modifications des autres lots. Modifie uniquement le test visé et son support direct si nécessaire. Préserve chaque scénario et ses valeurs de refus. Après l’édition, effectue format, Clippy structurel et la matrice N1 uniquement par `ci/guard-level1.ps1`; arrête-toi à tout échec. Aucun changement de production ou de baseline.
