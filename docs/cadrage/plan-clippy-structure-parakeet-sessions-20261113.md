Avancée : premier lot structurel borné à quatre diagnostics dans les tests privés Parakeet du cœur.
Blocage : aucune modification Rust avant acceptation de ce plan et confirmation du profil N1 de clôture.
Décision : traiter trois tests dans `parakeet_sessions.rs`, sans modifier le comportement ni la baseline.

# Plan — alertes structurelles Parakeet sessions

## Header

- **Source audit :** Clippy frais sur `af2e4062c283d4360670e75f4c5091c079e0cd08`, journal `C:/wt/clippy-corrections-suivantes/target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261113T164222485-275969d3dfd2487ca65ab045ea45a6e1/cargo-command.stdout.log`, SHA-256 `F5B3091EF3667847B608CBDC2C8628BC7FB35146D7E58414E103560D301D7440`.
- **Date de planification :** 13 novembre 2026.
- **Workspace cible :** crate `crates/whisper-core` dans le worktree dédié de ce lot.
- **Mode :** plan uniquement, aucune modification de code.
- **Confiance :** high — les quatre diagnostics et les trois fonctions sont présents dans le relevé frais et dans la source inspectée.
- **Contrôle qualité du plan :** passed.

## 1. Résumé de l’audit

La mesure fraîche a réussi sur six crates et comptabilise 141 sites structurels uniques : 62 en production et 79 dans les tests. La baseline reste à 168 sites et le cliquet passe. Les seuls lints observés sont `too_many_lines` et `cognitive_complexity`.

Ce lot commence par le groupe cohérent des scénarios privés de sessions Parakeet dans le runtime du cœur. Clippy signale trois tests trop longs aux lignes 81, 386 et 553 de `parakeet_sessions.rs`; le premier porte aussi un diagnostic `cognitive_complexity`. Cela fait quatre diagnostics sur trois fonctions. Les scénarios vérifient la séparation du texte confirmé et du résultat automatique non vérifié, l’isolation d’un échec tardif entre deux captures, et l’échec du démarrage direct avant `ParakeetLiveStarted`.

Les 137 autres sites de la mesure fraîche sont différés à des lots séparés : ils touchent d’autres responsabilités ou fichiers. Aucun constat Clippy n’est interprété comme un défaut fonctionnel.

## 2. Vérification de fraîcheur

- Inspectés : le journal JSON frais et son SHA-256, `crates/whisper-core/AGENTS.md`, `src/application/runtime/tests.rs`, `src/application/runtime/tests/parakeet_sessions.rs`, la garde N1 versionnée et le plan précédent.
- Les trois fonctions signalées existent toujours dans le candidat `af2e4062`; le fichier est inclus dans les tests privés du runtime via `tests.rs`.
- La commande observée était le profil Clippy gardé à six membres, avec `--all-targets --message-format=json` et les quatre lints structurels demandés. Sortie 0; cliquet PASS.
- Aucun test ni code n’a été modifié pendant la planification. Le worktree principal est sur une branche de lot, propre avant l’ajout de ce plan.
- La garde actuelle enregistre les profils Clippy, tests et format pour `target/parakeet-tests/clippy-corrections-suivantes-20261113`. Le même `LotRoot` précis est retenu pour ce sous-lot; aucun changement de garde n’est prévu. Le profil reste soumis à son SelfTest et aux autres préconditions N1.

## 3. Hypothèses de planification

- **Validation needed :** avant le premier Cargo, confirmer que le worktree dédié est à la révision contenant ce plan et que la garde autorise toujours les profils exacts, le `LotRoot` existant et une cible neuve dédiée.
- **Validation needed :** les helpers extraits restent des helpers privés de test dans l’arborescence actuelle `application/runtime/tests/`; aucune API de production, dépendance ou frontière de crate ne change.
- **Decision gate :** la validation de clôture Rust exige N1 complet selon `AGENTS.md`. La matrice peut compiler `whisper-worker-cpu`/`whisper-rs-sys`; le démarrage est bloqué jusqu’à confirmation de l’autorisation précise de cette N1 pour ce sous-lot, sauf si la voie gardée autorisée existante et l’accord utilisateur en cours la couvrent explicitement.

## 4. Traçabilité des constats

| Constat du relevé frais | Sévérité | Disposition | Tâche / gate | Notes |
|---|---|---|---|---|
| `whisper-core`, `too_many_lines`, `src/application/runtime/tests/parakeet_sessions.rs:81` | Structurel | Planned | P1-T1 | Scénario résultat automatique non vérifié; aussi concerné par `cognitive_complexity`. |
| `whisper-core`, `cognitive_complexity`, même fonction à la ligne 81 | Structurel | Planned | P1-T1 | Réduire la complexité en séparant les phases du scénario, sans retirer d’oracle. |
| `whisper-core`, `too_many_lines`, `src/application/runtime/tests/parakeet_sessions.rs:386` | Structurel | Planned | P1-T1 | Échec automatique tardif isolé à sa capture. |
| `whisper-core`, `too_many_lines`, `src/application/runtime/tests/parakeet_sessions.rs:553` | Structurel | Planned | P1-T1 | Échec du démarrage avant l’événement de démarrage live. |
| Tous les autres sites du relevé frais (137 sites uniques) | Structurel | Deferred | Lots ultérieurs | Répartition et lignes exactes dans le journal source; chaque lot ultérieur doit avoir un plan accepté et sa propre attribution. |

## 5. Séquence recommandée

Une tâche unique traite le groupe homogène des trois scénarios privés Parakeet. Les extractions suivent les étapes métier déjà visibles dans chaque test et sont placées dans un fichier de support privé inclus par le module de tests existant, afin d’alléger le fichier déjà long sans transformer les tests en tests d’intégration.

### Phase 1 : rendre les scénarios Parakeet lisibles sans changer leurs oracles

**Objectif :** supprimer les quatre diagnostics structurels ciblés.

**Invariant fermé :** les événements, valeurs, identités et ordres d’effets observés par chacun des trois scénarios sont identiques avant et après extraction.

**Métriques avant :** 3 sites `too_many_lines` (fonctions aux lignes de départ 81, 386 et 553); 1 site `cognitive_complexity` (fonction à la ligne 81).

**Métriques cible :** 0 de ces diagnostics dans `parakeet_sessions.rs`; aucun nouveau site structurel dans le crate ni dans le cliquet.

**Commandes de métrique :** Clippy JSON gardé à six crates et comparaison du lint, crate, chemin, fonction et opérande avec le relevé frais.

**Conditions d’arrêt :** arrêter si une extraction modifie un événement, une assertion, l’identité d’une capture/import, l’ordre des événements/effets, ou requiert une API publique; arrêter si un prérequis échoue, si un diagnostic structurel nouveau apparaît, si la garde refuse le profil ou si l’arbre contient une modification concurrente pertinente.

**Dépendances :** aucune.

**Résultat attendu :** trois tests plus courts et lisibles, avec les mêmes scénarios et assertions.

**Tâches :** P1-T1.

## 6. Tâches

#### P1-T1 : extraire les phases des tests de sessions Parakeet

- **Priorité :** High.
- **Constats source :** les quatre lignes de traçabilité Planned ci-dessus.
- **Raison :** réduire les trois tests qui dépassent le seuil structurel, dont un dont la complexité cognitive est aussi élevée, sans changer leurs contrats.
- **Fichiers/modules cibles :** `crates/whisper-core/src/application/runtime/tests.rs`, `crates/whisper-core/src/application/runtime/tests/parakeet_sessions.rs`, nouveau `crates/whisper-core/src/application/runtime/tests/parakeet_sessions_support.rs`.
- **Carte d’architecture :** `application/runtime` garde l’orchestration et l’état privés; `application/runtime/tests/` garde les tests privés qui ont besoin de ces détails; le nouveau fichier contient uniquement des helpers privés de support de tests. Aucune dépendance du cœur vers l’adaptateur, l’UI ou un support externe.
- **Convention de modules :** préserver la convention locale `include!("tests/<fichier>.rs")` depuis `tests.rs`; ajouter le fichier de support dans cette même liste. Ne pas introduire `#[path]`, module public ou nouvelle crate.
- **Budget de portée :** deux fichiers de test existants et un nouveau fichier de support de test; aucun fichier de production, manifeste, baseline ou garde.
- **Invariant fermé :** mêmes événements, `dispatch`, effets, identités, valeurs d’état et assertions dans les trois scénarios.
- **Métriques avant :** 3 sites `too_many_lines` + 1 site `cognitive_complexity` dans le groupe ciblé.
- **Métriques cible :** 0 diagnostic ciblé; les helpers restent chacun sous 100 lignes et le fichier de support ne contient pas de scénario autonome.
- **Commandes métriques :** Clippy gardé de la matrice et inspection des diagnostics JSON par fonction/chemin.
- **Conditions d’arrêt :** arrêt si l’extraction requiert une nouvelle visibilité publique, réduit/retire une assertion, change l’ordre d’exécution, crée un test qui n’est pas atteint dans la matrice, ou si le profil gardé n’est pas admis.
- **Dépendances :** aucune.
- **Étapes d’implémentation :**
  1. Séparer le scénario de ligne 81 en phases nommées correspondant à l’état initial avec texte confirmé, capture suivante, progression de l’import automatique et publication finale non vérifiée; garder le test principal comme séquence explicite de ces phases.
  2. Séparer le scénario de ligne 386 en préparation des deux captures et vérification que l’échec tardif de A ne modifie pas la ligne ni le message de B.
  3. Réduire le scénario de ligne 553 par une extraction locale au support de préparation/vérification, en gardant les assertions sur la capture A finalisée et la capture B échouée avant le démarrage live.
  4. Comparer soigneusement les assertions et la séquence d’événements au diff de base; conserver les noms et attributs des tests.
- **Tests à ajouter/modifier :** aucun test nouveau; conserver et exécuter les trois tests existants par la matrice N1.
- **Vérifications :** `cargo fmt --check` via `ci/guard-level1.ps1`; Clippy complet à six membres via la garde; `cargo test --no-fail-fast` N1 via la garde et avec les paramètres/personnel modèle requis par `AGENTS.md`.
- **Provenance :** format, Clippy et commande N1 sont définis dans `AGENTS.md` et le plan précédent; profils exacts d’abord confirmés par le préflight de la garde. Les sélections filtrées ne remplacent pas la N1 de clôture.
- **Critères d’acceptation :**
  - Aucun diagnostic ciblé ne reste; le cliquet passe et la baseline est inchangée.
  - Les trois tests existants passent et gardent leurs noms, attributs et oracles.
  - Aucun nouveau code de production, test inline ajouté ailleurs, test natif `native-tests`, build DLL, GPU niveau 2 ou validation Windows n’est introduit/annoncé.
  - L’implémentation reste dans les trois chemins de test listés et respecte le budget de portée.
- **Retour arrière :** annuler uniquement les changements de ces trois fichiers de test; préserver les autres changements du worktree et le plan accepté.
- **Notes à l’implémenteur :** la ligne 81 traite un import automatique produit mais non vérifié et une nouvelle capture simultanée; maintenir la séparation du texte confirmé, du texte provisoire et de la ligne de capture. Le cas ligne 386 attribue l’échec tardif à l’import A tandis que B est terminé. Le cas ligne 553 distingue le démarrage direct échoué avant `ParakeetLiveStarted` de la capture A déjà finalisée.

## 7. Matrice de vérification

| Domaine | Commande ou contrôle | Provenance | Résultat attendu | Tâches | Limites |
|---|---|---|---|---|---|
| Préflight / garde | SelfTest N1, contrôles PE et préflight prévus dans `AGENTS.md`, sans Cargo hors garde | Repo docs | PASS avant les commandes Cargo | P1-T1 | Témoin PE réel `NON EXÉCUTÉ` sans sélection explicite et SHA-256. |
| Format | `fmt --check` via `ci/guard-level1.ps1`, racine `target/parakeet-tests/clippy-corrections-suivantes-20261113` | Repo docs | PASS | P1-T1 | Ne pas lancer `fmt` automatique. |
| Clippy | Profil JSON à six membres et quatre lints structurels déjà établi dans le plan précédent, même `LotRoot`, cible neuve dédiée | Observed / Repo docs | Sortie 0; 0 des quatre sites ciblés; cliquet PASS; aucun site nouveau | P1-T1 | Workers Turbo non inclus dans ce profil. |
| N1 | `test --no-fail-fast` via la garde, en shell dédié, paramètres canoniques et chemin de modèle personnel avec SHA requis | Repo docs | PASS; attribution des tests et ignorés | P1-T1 | Risque `whisper-rs-sys`/CMake selon la matrice; ne pas démarrer avant l’accord précis confirmé au gate. |
| Oracles du scénario | Revue du diff et résultats des trois tests nommés | Source inferred | Événements et assertions conservés | P1-T1 | Ne prouve pas CUDA réel ni les corps `ignore`/`native-tests`. |

## 8. Décisions et gates

| Gate | Statut | Bloque | Question | Défaut recommandé | Validation |
|---|---|---|---|---|---|
| G1 — acceptation du plan | Blocking | P1-T1 | Acceptes-tu ce lot exact et ses trois chemins de test ? | Aucun code avant acceptation explicite | Plan versionné dans Git et accord écrit dans la session. |
| G2 — N1 susceptible de compiler `whisper-rs-sys` | Blocking | N1 de clôture P1-T1 | L’accord antérieur sur le worker CPU et la voie N1 s’applique-t-il à ce lot, avec la même garde et les mêmes préconditions? | Ne pas lancer Cargo si la couverture de l’accord n’est pas explicite | Confirmer l’autorisation; préflight et arrêt sur échec restent obligatoires. |
| G3 — profil gardé exact | Validation | Toute commande Cargo | La garde accepte-t-elle toujours la racine et les profils enregistrés sans élargissement ? | Profils exacts uniquement sous l’ancien `LotRoot` | SelfTest puis commande autorisée via `ci/guard-level1.ps1`. |

## 9. Hors périmètre

- Les 137 autres sites du relevé frais; chacun sera regroupé dans un lot ultérieur, au plus dix tâches, avec plan accepté et chemins explicites.
- Toute modification de production, des autres tests, des manifests, de `Cargo.lock`, de la baseline, des seuils, de la garde, du comportement ou des contrats IPC.
- Toute commande Cargo non admise, téléchargement, `native-tests`, tests `--ignored`, build DLL, GPU niveau 2 ou validation dans la fenêtre Windows.
- Toute extension d’API ou migration générale des tests privés du runtime.

## 10. Auto-contrôle final

| Contrôle | Résultat | Notes |
|---|---|---|
| Plan concret, non placeholder | Pass | Quatre diagnostics et trois fonctions attribués. |
| Sections du template présentes dans l’ordre | Pass | Sections 1–11. |
| Constats du relevé attribués | Pass | Quatre planifiés; 137 explicitement différés au futur lot. |
| Tâche complète et bornée | Pass | Fichiers, dépendances, outline, tests, commandes, critères et rollback présents. |
| Invariant et métriques par phase/tâche structurelle | Pass | Avant/cible, commandes, conditions d’arrêt. |
| Budget de portée par tâche | Pass | 2 fichiers existants + 1 support; aucun dépassement. |
| Tests privés correctement justifiés | Pass | Emplacement existant nécessaire à l’accès à l’état privé; pas de nouveau scénario. |
| Provenance de vérification | Pass | Repo docs ou observée; préflight reste obligatoire. |
| Gates et conséquences explicites | Pass | Acceptation du plan et N1 bloquants. |
| Chemins valides | Pass | Sources inspectées; nouveau chemin de support sous le répertoire existant. |
| Lot exécutable progressivement | Pass | Un fichier de scénarios, une tâche, un invariant. |

## 11. Prompt pour l’implémenteur

Après acceptation du plan et résolution des gates, déléguer P1-T1 à `rust_implementer` en mode `IMPLEMENT`. Limiter les écritures à `crates/whisper-core/src/application/runtime/tests.rs`, `tests/parakeet_sessions.rs` et `tests/parakeet_sessions_support.rs`. Préserver les changements sans rapport et ne modifier ni la production ni la garde. Exécuter les vérifications exactes de P1-T1 via `ci/guard-level1.ps1`; arrêter sur tout échec de préflight, écart d’oracle ou diagnostic structurel nouveau.
