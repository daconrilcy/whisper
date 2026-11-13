# Plan P9 — Réduire le contrat de drainage et d’ACK d’import

Avancée : plan ciblé sur un test de contrat de 118 lignes.
Blocage : aucun ; le seuil de drainage et l’attente des 99 ACK doivent rester vérifiés.
Décision : extraire l’installation de la séquence d’événements en helper privé.

## En-tête

- Source audit : Clippy frais post-P8, `.../scratch/20261113T213352591-805dde64395e460d8b6b66f9e33a3107/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-core/tests/import_contract.rs`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — test et assertions inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Après P8, Clippy compte 128 sites. `refresh_drains_at_most_four_events_and_waits_for_every_persisted_ack` fait 118 lignes. Il configure un import avec 99 segments, vérifie le plafond de quatre effets persistés par Refresh, interdit la publication tant que manquent des reçus, puis autorise la finalisation après le 99e ACK.

## 2. Fraîcheur

Inspection ciblée du test et des helpers dans `crates/whisper-core/tests/import_contract.rs`, avec les règles root/core. P1–P8 restent dirty et doivent être préservés. Ce test d’intégration public du cœur ne nécessite aucun adapter.

## 3. Hypothèses

- Safe : l’initialisation des événements préparés et des segments peut être extraite dans une fonction privée.
- Validation needed : 99 segments, cadence de quatre, ordre 1..99, absence de Publish à 98/99 reçus, finalizing après le 99e doivent rester asserts du scénario.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| Test 118 lignes à `import_contract.rs:171` | P9 | Planned | P9-T1 | Extraire le setup du flux de segments |
| 127 autres sites | Hors lot | Deferred | Aucun | Lots ultérieurs |

## 5. Séquence recommandée

Une tâche réduit le corps du test en extrayant seulement la mise en place des événements et segments; elle ne change ni Refresh ni les assertions de backpressure/publication.

### Phase P9 : séparer le setup du contrat d’ACK

Objective : supprimer l’alerte `too_many_lines`.
Closed invariant : le test principal fait ≤100 lignes et maintient tous les seuils 99/4/98.
Before metrics : 118 lignes; 128 sites structurels.
Target metrics : ≤100 lignes; au plus 127 sites, sans nouvelle alerte.
Metric commands : Clippy six crates, N1.
Stop conditions : arrêt si nombre/ordre de segments, cadence, reçus, publication ou état final change; si nouveau fichier est nécessaire; ou en cas d’échec d’un gate.
Dependencies : aucune.
Expected outcome : helper privé de setup strictement test-only.
Tasks : P9-T1.

## 6. Tâches

#### P9-T1 — Extraire le setup des 99 segments

- Priorité : Medium
- Source : Clippy 118/100 du test nommé.
- Justification : garder le contrat de drainage lisible sans retirer aucune preuve.
- Fichiers : `crates/whisper-core/tests/import_contract.rs` seulement.
- Bucket : tests/support local à l’intégration.
- Scope budget : un fichier de test existant.
- Closed invariant : corps du test ≤100 lignes avec assertions inchangées.
- Before metrics : 118; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : ceux de Phase P9.
- Dépendances : aucune.
- Outline : extraire la création/enqueue des Prepared, Ready, Progress, 99 segments et End; garder dans le test les boucles Refresh, mesures, ACK manquants, envoi du dernier ACK et assertions finales.
- Tests : conserver le test existant; aucun cas nouveau.
- Vérifications : `cargo fmt --check`, Clippy structurel six crates, N1 complet via garde.
- Provenance : Observed pour taille/scénario; Repo docs pour les gates.
- Acceptation : test ≤100 lignes; 99 IDs séquentiels et batch ≤4 vérifiés; aucune publication avec 98 reçus; finalizing et Publish après le 99e; N1 PASS; seul fichier ciblé modifié.
- Retour arrière : annuler l’extraction si assertions ou volumes changent.
- Notes : ne pas réduire la boucle réelle de Refresh ni remplacer les assertions par une assertion agrégée moins précise.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE et espace | Repo docs | PASS | P9-T1 | témoin PE réel non exécuté |
| Format | fmt check via garde | Repo docs | PASS | P9-T1 | pas de formatage en écriture |
| Clippy | six crates, quatre lints | Observed/Repo docs | ≤127 sites | P9-T1 | GPU N2 exclu |
| N1 | `test --no-fail-fast` + sélections CPU | Repo docs | PASS | P9-T1 | ignorés préservés |
| Contrat | scénario et assertions existants | Source inferred | ACK complet requis | P9-T1 | modèle de campagne inchangé |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P9-G1 | Validation | clôture | Le seuil par Refresh reste-t-il 4 ? | Oui | assertion préservée et test N1 |

## 9. Hors périmètre

Ne pas modifier l’API du cœur, runtime, règle de publication, volumes du scénario, dépendances ou baseline. Ne pas ajouter de campagne documentaire/test.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | test et invariant nommés |
| Sections conformes | Pass | modèle suivi |
| Traçabilité | Pass | un constat prévu, reste différé |
| Tâche complète | Pass | fichiers, outline, tests, gates, rollback |
| Métriques/arrêts | Pass | 118→≤100 |
| Budget | Pass | un fichier |
| Test dédié | Pass | fichier tests/ existant |
| Provenance des commandes | Pass | docs/Clippy observé |
| Porte explicite | Pass | batch=4 |
| Chemins valides | Pass | inspectés localement |
| Portée limitée | Pass | setup de ce test seulement |

## 11. Prompt d’implémentation

Exécute P9-T1 de `docs/cadrage/plan-clippy-structure-import-ack-test-20261113.md` dans `C:/wt/cs`. Préserve P1–P8. Modifie uniquement `crates/whisper-core/tests/import_contract.rs`; extrais le setup et garde exactement 99 segments, cadence 4, les 98 ACK insuffisants et la publication après ACK 99. Passe fmt, Clippy structurel et N1 via la garde; stoppe sur échec.
