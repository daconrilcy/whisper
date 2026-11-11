Avancée : plan d’implémentation proposé pour le cliquet structurel Clippy.
Blocage : aucun préalable de décision ; arrêt si les données ne respectent pas les règles STOP convenues.
Décision : GO utilisateur du 11/11/2026 ; implémentation PowerShell limitée aux fichiers acceptés.

# Plan d’implémentation — Cliquet Clippy structurel

## Header

- **Source audit :** [cadrage du cliquet](evo-cliquet-clippy-structure.md) et [bilan du premier lot](../architecture/CLIPPY-LOCAL-BILAN.md).
- **Planning date :** 2026-11-11.
- **Target workspace :** `C:\wt\cliquet-clippy`, branche `cliquet-clippy` issue de `main`.
- **Planning mode :** plan uniquement, aucun code modifié.
- **Confidence :** medium — la clé choisie est testée sur le flux frais et la revue est CLEAN ; sa limite de renommage reste intentionnelle.
- **Plan quality check :** mis à jour selon le GO utilisateur du 11 novembre 2026 ; G1, G2 et G5 sont closes.

## 1. Audit Digest

Le premier lot a fait passer les alertes hors structure de 96 à 25. Les quatre lints structurels comptaient 248 diagnostics avant et après ; le bilan rapporte zéro nouveau site. La priorité suivante est de préserver ce stock et de prévenir son augmentation, sans refactorer les fonctions existantes pour atteindre les seuils.

La garde d’exécution se trouve dans `ci/guard-level1.ps1`. Elle accepte Clippy et `--message-format=json`, capture les sorties et retourne un échec sur un code Cargo non nul. Son mode `Structure` ne traite que le registre de classification des tests (`ci/check-structure.rs`, `ci/tests-baseline.txt`). Le GO autorise le comparateur PowerShell dans cette garde ; aucun code Rust ni nouveau binaire n’est prévu. `AGENTS.md` sera corrigé pour refléter l’existant et formaliser N1 + Clippy.

La référence Clippy est produite depuis le commit source du lot. La clé est décidée par l’utilisateur : lint, package, chemin relatif normalisé et signature issue du premier texte du span primaire. Après normalisation des espaces, une ligne contenant `fn <ident>` devient `fn <ident>` ; sinon un `struct|enum|type|trait|impl <ident>` devient `<mot-clé> <ident>` ; sinon le préfixe `pub(...)` est retiré et la ligne est tronquée après le premier `(`, `<` ou `{`. La comparaison est en multi-ensemble. Aucune mise à jour automatique n’est permise.

## 2. Freshness Check

- Inspectés : `ci/guard-level1.ps1` (mode Run/Structure, validation Clippy, capture), `ci/check-structure.rs`, `ci/tests-baseline.txt`, `docs/architecture/CLIPPY-LOCAL-BILAN.md`, `docs/architecture/raw/clippy-diagnostics.json` et `.tsv`.
- Observé : le JSON d’analyse archivé expose package, cible, lint, message, chemin, lignes et une identité de fonction dérivée par l’outil d’analyse. Cette projection n’est pas le format brut Clippy et ne prouve pas que la garde peut retrouver ce champ sans parser le code source.
- Référence initiale : créée dans ce lot depuis le commit de départ, format `ci/clippy-structure-baseline.tsv` ; les comptes doivent correspondre aux 248 sites du bilan ou l’écart doit être expliqué.
- Validations autorisées : SelfTest N1, Clippy workspace JSON gardé et un essai négatif temporaire ; N1 complet non requis.
- État de départ : worktree dédié `C:/wt/cliquet-clippy`, branche `cliquet-clippy` créée depuis `main`.

## 3. Planning Assumptions

- **Décidée — identité :** comparer les comptes par clé spécifiée en multi-ensemble ; un span sans texte est un échec.
- **Décidée — référence :** créer un TSV trié avec commit source, date et commande ; jamais modifier à l’exécution.
- **Décidée — organisation :** le dépôt fait foi ; actualiser `AGENTS.md` dans le périmètre autorisé.
- **Validation needed :** démontrer que les duplications multi-cibles sont dédupliquées, le multiset total correspond à 248 et aucune alerte n’est dépourvue de ligne source.
- **Safe — évolution monotone :** une alerte de référence peut disparaître sans échec ; le contrôle n’exige jamais le maintien du stock.
- **Safe — erreur de compilation :** une erreur Clippy reste un échec par le code de sortie non nul ; le cliquet ne transforme pas un échec en succès.

## 4. Finding Traceability

| Finding | Priority | Disposition | Task / gate | Notes |
|---|---|---|---|---|
| 248 alertes structurelles existantes ne sont pas un objectif de refactor | High | Planned | P1-T1, P1-T2 | Stock de référence accepté ; pas de réduction exigée. |
| Toute nouvelle alerte parmi quatre lints doit être bloquée | High | Planned | P1-T2 à P1-T4 | Échec explicite et site signalé. |
| Déplacement de ligne ne doit pas compter comme nouveau site | High | Planned | P1-T2 | Les lignes ne sont pas dans la clé persistée. |
| Contrôle `Structure` existant concerne la classification des tests | Medium | Already resolved | P1-T2 | Ne pas le présenter comme cliquet Clippy existant. |
| `AGENTS.md` fourni et dépôt divergent sur les fichiers de structure | High | Planned | P1-T4 | Corriger la documentation versionnée selon le dépôt et le GO. |
| Référence historique non garantie à jour | High | Planned | P1-T1 | Mesure fraîche obligatoire sur le commit autorisé. |
| 25 alertes hors structure | Low | Deferred | Hors périmètre | Aucun traitement dans le cliquet. |

## 5. Recommended Sequence

Les décisions utilisateur ont fermé l’identité, les consignes et le GO. La séquence est : établir la référence fraîche au commit de départ, intégrer le comparateur PowerShell et ses fixtures dans SelfTest, exécuter N1 SelfTest et Clippy gardé, puis un probe négatif temporaire. La référence ne se régénère jamais en réaction automatique à une alerte nouvelle.

### Phase 1 : Verrouiller l’invariant de non-régression

Objective: empêcher l’augmentation du stock des quatre lints sans forcer sa réduction.
Closed invariant: après Clippy réussi, chaque diagnostic de ces lints correspond à un site accepté par la référence ; tout site nouveau fait échouer le contrôle.
Before metrics: 248 sites structurels dans le bilan du 2026-11-11 ; 25 alertes non structurelles hors périmètre.
Target metrics: 0 nouveaux sites tolérés ; 0 alerte existante supprimée exigée ; 4 lints couverts.
Metric commands: commande Clippy JSON gardée de P1-T1, puis contrôles du comparateur sur fixtures.
Stop conditions: identité instable/ambiguë ; écart non expliqué avec 248 ; changement requis aux commandes ou garanties N1 ; auto-test négatif qui ne détecte pas une alerte nouvelle.
Dependencies: GO reçu ; aucun gate produit restant.
Expected outcome: référence fraîche rapprochée des 248 sites.
Tasks: P1-T1.

### Phase 2 : Intégrer le cliquet à N1

Objective: exécuter la comparaison dans le chemin Clippy de la garde niveau 1.
Closed invariant: toute invocation gardée de Clippy workspace avec JSON termine en échec si son résultat contient un site inédit parmi les quatre lints.
Before metrics: aucune comparaison Clippy de référence dans la garde ; 248 sites acceptés.
Target metrics: cas inchangé, ajout, retrait, déplacement, référence invalide et erreur Clippy tous distingués correctement ; pas de nouvelle surveillance.
Metric commands: SelfTest N1 et `cargo clippy --workspace --all-targets --message-format=json` via la garde.
Stop conditions: chemin Clippy hors garde nécessaire ; commande N1 à assouplir ; modification automatique de référence ; perte de sortie ou masquage d’erreur.
Dependencies: P1-T1.
Expected outcome: cliquet actif sous N1 et vérifié par cas positifs/négatifs.
Tasks: P1-T2 à P1-T4.

## 6. Task Backlog

#### P1-T1 : Établir la référence fraîche du lot

- **Priority:** High
- **Source finding:** CA-01, CA-03, CA-06 ; décision D-01/D-02.
- **Rationale:** le comparateur n’est sûr que si les sites sont distingués sans être attachés à des numéros de lignes fragiles.
- **Target files/modules:** `ci/clippy-structure-baseline.tsv` ; sortie JSON Clippy capturée par la garde sous la racine de campagne autorisée.
- **Ownership bucket:** garde locale et données de référence versionnées.
- **Scope budget:** 0 fichier Rust produit ; 1 référence créée depuis le JSON frais du commit de départ.
- **Closed invariant:** la référence représente le stock du commit choisi et la clé distingue les fonctions/site sans dépendre seulement de la ligne.
- **Before metrics:** 248 sites selon le bilan ; aucune référence Clippy dédiée observée.
- **Target metrics:** référence fraîche couvrant exactement les diagnostics des 4 lints ; différence expliquée et approuvée avant versionnement.
- **Metric commands:** Clippy JSON workspace gardé et outil local de comparaison/inspection ; aucun outil précis d’empreinte externe n’est présumé.
- **Stop conditions:** résultat ne permet pas de construire une clé stable ; compte différent de 248 sans explication ; consignes versionnées interdisent ce fichier ou son usage.
- **Dependencies:** aucune ; choix de clé autorisé par le GO.
- **Implementation outline:**
  1. lancer la commande Clippy gardée au commit retenu et extraire seulement les quatre codes lint.
  2. extraire les alertes et dédupliquer les émissions de plusieurs cibles avant comptage.
  3. former la clé et le nombre par clé prescrits, trier ordinalement et vérifier le rapprochement avec 248.
  4. arrêter si collision non résolue par le multi-ensemble ou plus de cinq alertes sans ligne citée.
  5. versionner une fois avec commit, date et commande ; aucune réécriture automatique.
- **Tests to add or update:** scénarios de fixtures du comparateur prévus en P1-T2 ; aucun test produit.
- **Verification commands:** commande Clippy gardée décrite en matrice ; inspection que 4 codes seuls sont présents dans la référence.
- **Command provenance:** Clippy de mesure observé au bilan ; commande de reprise prescrite par AGENTS, à exécuter seulement après autorisation.
- **Acceptance criteria:** le total du multi-ensemble est 248 ou tout écart est expliqué dans le bilan ; nombre d’alertes sans ligne citées ≤5 ; les quatre comptes sont rapportés ; le TSV contient commit, date, commande et est trié.
- **Rollback/fallback notes:** si l’identité ne passe pas les fixtures, ne pas versionner de référence ; garder les sorties de campagne conformément au plan autorisé et demander décision.
- **Notes for implementation agent:** ne pas reprendre directement `clippy-diagnostics.json` comme source baseline ; c’est une projection historique et la fraîcheur n’est pas garantie.

#### P1-T2 : Implémenter le comparateur PowerShell et les fixtures SelfTest

- **Priority:** High
- **Source finding:** CA-01 à CA-05.
- **Rationale:** séparer l’algorithme de comparaison de l’orchestration PowerShell rend les règles testables avant branchement à Cargo.
- **Target files/modules:** `ci/guard-level1.ps1` uniquement ; fixtures JSON synthétiques du SelfTest ; référence de P1-T1.
- **Ownership bucket:** outil local de garde et tests de garde.
- **Scope budget:** 1 fichier PowerShell ; aucun Rust, nouveau binaire, dépendance ou fichier de fixtures.
- **Closed invariant:** comparaison d’ensembles de sites ne dépendant pas du numéro de ligne seul.
- **Before metrics:** aucun comparateur Clippy ; 0 cas SelfTest cliquet.
- **Target metrics:** tous les cas du GO : inchangé, nouveau, retiré, déplacement, doublon multi-cible, deux occurrences d’une clé, deux fonctions `pub(crate)` distinctes, visibilité `pub(crate)`/`pub` équivalente pour même nom, lint hors périmètre, référence absente/invalide, span sans texte, Clippy en échec.
- **Metric commands:** SelfTest N1 par `ci/guard-level1.ps1`; pas de compilation Rust du comparateur.
- **Stop conditions:** comparaison nécessite d’accepter une clé ambiguë ; références malformées sont tolérées ; parser ou fixture nécessite un accès réseau ou un crate nouveau.
- **Dependencies:** P1-T1.
- **Implementation outline:**
  1. implémenter lecture/validation stricte du TSV et des messages Clippy JSON avec `ConvertFrom-Json`.
  2. comparer les quatre lints par clé établie, en tolérant les entrées retirées.
  3. faire échouer sur entrée invalide et inclure lint, package/cible et site dans le diagnostic d’échec.
  4. exposer une fonction SelfTest déterministe avec fixtures sous le répertoire d’exécution autorisé.
- **Tests to add or update:** fixtures synthétiques pour tous les cas explicitement autorisés dans le SelfTest ; aucun scénario produit.
- **Verification commands:** SelfTest N1 après ajout du comparateur.
- **Command provenance:** règles N1 et mode SelfTest observés dans `ci/guard-level1.ps1`.
- **Acceptance criteria:** tous les cas du GO passent ; aucune alerte hors périmètre n’est prise en compte ; aucun accès réseau ni changement Cargo manifests/lockfile.
- **Rollback/fallback notes:** retirer le comparateur et ses fixtures si une règle ne peut être rendue stricte ; ne jamais neutraliser l’échec pour faire passer le stock.
- **Notes for implementation agent:** ne pas modifier `ci/check-structure.rs`; la comparaison compte les occurrences de chaque clé en multi-ensemble.

#### P1-T3 : Brancher la comparaison au chemin Clippy gardé

- **Priority:** High
- **Source finding:** CA-02, CA-04, CA-05.
- **Rationale:** le cliquet doit s’appliquer à la sortie Clippy réelle dans la garde qui contrôle déjà les commandes et leurs racines.
- **Target files/modules:** `ci/guard-level1.ps1`, `ci/clippy-structure-baseline.tsv`.
- **Ownership bucket:** orchestration de garde locale.
- **Scope budget:** 1 fichier de garde et 1 référence ; fixtures dans le SelfTest de la garde.
- **Closed invariant:** une commande Clippy N1 réussie est ensuite soumise au comparateur ; un Clippy non nul ou un diagnostic inédit mène à un échec global.
- **Before metrics:** garde capture stdout/stderr et échoue sur code Clippy non nul ; aucun contrôle de baseline Clippy.
- **Target metrics:** les quatre lints sont comparés une fois par résultat JSON ; 0 changement aux commandes Cargo permises, isolation et surveillance.
- **Metric commands:** SelfTest N1 ; commande Clippy workspace/all-targets/JSON sous la garde.
- **Stop conditions:** besoin d’une exécution Cargo directe ; sortie JSON perdue/tronquée sans échec ; extension des commandes ou changement de sécurité N1.
- **Dependencies:** P1-T1, P1-T2.
- **Implementation outline:**
  1. intégrer la lecture de la sortie capturée uniquement pour les invocations Clippy avec sortie JSON.
  2. conserver le contrôle de code de sortie et faire échouer en fermeture stricte sur erreur du comparateur ou référence absente.
  3. présenter les sites nouveaux dans les journaux gardés sans convertir une erreur Clippy en réussite.
  4. s’assurer que sorties et fichiers temporaires restent dans les racines N1 prévues.
- **Tests to add or update:** SelfTest garde pour référence absente/invalide, diagnostics non structurels, erreur Clippy et nouvel avertissement.
- **Verification commands:** SelfTest N1 ; Clippy workspace ci-dessous.
- **Command provenance:** `guard-level1.ps1` et règle Cargo N1 observées dans le dépôt/AGENTS.
- **Acceptance criteria:** nouvel avertissement fait échouer la garde ; stock inchangé et alertes retirées passent ; erreur Clippy conserve son échec ; pas de nouveau mode de surveillance.
- **Rollback/fallback notes:** revert atomique des fichiers de garde et référence si les invariants échouent ; garder les preuves du lot selon sa rétention autorisée.
- **Notes for implementation agent:** aucun « allow », seuil élargi, baseline auto-généré à l’échec, argument inconnu ou option Cargo hors liste.

#### P1-T4 : Valider le cliquet et sa fermeture N1

- **Priority:** High
- **Source finding:** CA-01 à CA-06.
- **Rationale:** vérifier ensemble les auto-tests négatifs, la commande Clippy réelle, les comptes et la fermeture sous la garde.
- **Target files/modules:** fichiers du lot, garde et référence ; aucun code produit.
- **Ownership bucket:** validation de garde.
- **Scope budget:** 0 fichier produit persistant ; seul le fichier de test temporaire du probe peut être modifié puis retiré sans commit.
- **Closed invariant:** le candidat accepte la référence, bloque les nouveaux sites et respecte le préflight N1.
- **Before metrics:** 248 alertes structurelles actuelles ; résultat de garde sans cliquet.
- **Target metrics:** SelfTest PASS ; comparaison Clippy complète PASS ; toute fixture négative est refusée ; 0 nouvelle alerte au départ du lot.
- **Metric commands:** `-Mode SelfTest`, puis Clippy workspace `--all-targets --message-format=json` par `-Mode Run` ; détails en matrice.
- **Stop conditions:** moins de 30 Gio libres ; préflight/selftest échoue ; site ajouté non expliqué ; différence de baseline ; échec compilation ou écriture hors racines ; activité Git pertinente concurrente.
- **Dependencies:** P1-T1 à T3.
- **Implementation outline:**
  1. vérifier l’état Git et protections N1, l’espace libre, les racines et l’absence de conflit pertinent.
  2. lancer le SelfTest par la garde.
  3. lancer Clippy workspace gardé, puis ajouter temporairement une fonction dépassant le seuil dans un fichier de test et relancer la garde ; vérifier l’échec, puis retirer cette modification sans commit.
  4. vérifier le rapprochement, les résultats positifs/négatifs, le diff et les limites ; arrêter si un critère manque.
- **Tests to add or update:** aucun scénario produit ; tests de garde de P1-T2/T3 seulement.
- **Verification commands:** matrice ci-dessous.
- **Command provenance:** Repo docs et source observée ; la cible exacte est décidée au préflight du lot.
- **Acceptance criteria:** N1 SelfTest réussi ; Clippy candidat code 0 avec zéro site nouveau ; total multiset rapproché de 248 ; le probe temporaire fait échouer la garde et est retiré ; aucune autre modification Rust persistante.
- **Rollback/fallback notes:** en échec, interrompre la fermeture, garder sorties et ne pas régénérer le baseline. Ne pas nettoyer sans appliquer les règles du lot autorisé.
- **Notes for implementation agent:** validation N1 uniquement ; pas de test GPU exécuté, validation Windows, campagne native, push ou livraison produit implicites.

## 7. Verification Matrix

| Area | Command or check | Provenance | Expected result | Covers tasks | Notes |
|---|---|---|---|---|---|
| Garde SelfTest | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/clippy-ratchet -FreshTarget clippy-ratchet-selftest` | Repo docs/source observed | PASS, y compris fixtures Clippy positives/négatives | P1-T2–T4 | Cible neuve à chaque SelfTest. |
| Clippy workspace | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/clippy-ratchet -FreshTarget cliquet-clippy -CargoArgsJson '["clippy","--workspace","--all-targets","--message-format=json","--","-W","clippy::too_many_lines","-W","clippy::cognitive_complexity","-W","clippy::too_many_arguments","-W","clippy::type_complexity"]'` | Repo docs/source observed | Code 0 ; 4 lints comparés ; zéro nouveau site ; multiset total 248 | P1-T1, P1-T3, P1-T4 | Garde ajoute `--offline --locked`. |
| Probe négatif | Rejouer la même commande gardée avec une fonction temporaire de test dépassant un seuil ; retirer ensuite le delta sans commit | GO utilisateur | Garde non nulle avec lint/site identifiés ; fichier restauré | P1-T4 | Une fois seulement ; aucun test N1 complet. |
| Comparateur SelfTest | inchangé, nouveau, retiré, lignes différentes/même clé, multi-cible, occurrence supplémentaire, lint hors périmètre, baseline absente/invalide, span sans texte, code Clippy non nul | GO utilisateur | PASS/FAIL attendus par fixture | P1-T2, P1-T3 | JSON synthétique, aucune compilation Rust. |
| Revue manuelle | inspection clé, référence, messages d’erreur, `git diff --check`, état Git | Repo docs | Aucune clé ambiguë ni fichier hors plan | Tous | Git séparé de Cargo. |
| Limites | aucune validation Windows/native/GPU niveau 2 | Repo docs | explicitement NON EXÉCUTÉE | P1-T4 | Le lot porte sur la garde locale. |

## 8. Decision Gates

| Gate | Status | Blocks | Question | Recommended default | How to validate |
|---|---|---|---|---|---|
| G1 — Identité | Closed by user | — | Clé et multi-ensemble | Valeurs exactes fournies le 11/11/2026 | Encodées et couvertes par fixtures SelfTest. |
| G2 — Consignes | Closed by user | — | Le dépôt fait foi ; corriger `AGENTS.md` | Règle fournie dans le GO | Vérifier le diff de `AGENTS.md`. |
| G3 — Référence fraîche | Validation | Clôture P1-T1/T4 | Le total s’aligne-t-il sur 248 ? | Exactement 248 ; sinon expliquer dans le bilan ou STOP | Sortie gardée, comptes par lint, multi-ensemble. |
| G4 — Nouvelle alerte acceptée | Blocking au moment concerné | Toute exception future | Une alerte inédite peut-elle être intégrée à la référence ? | Seulement ajout manuel, justifié dans le bilan du lot | Revue du diff de référence et justification. |
| G5 — GO | Closed by user | — | Lot autorisé dans le périmètre cité | Décision du 11/11/2026 | Aucun fichier ou commande hors autorisation. |

## 9. Out of Scope

- Réduire les fonctions existantes ou les 248 alertes structurelles pour leur métrique seule.
- Modifier les 25 résiduels hors structure ou toucher la logique Whisper.
- Changer les seuils, lints demandés, membres workspace, règles Cargo ou isolation N1.
- Ajouter CI distante, dépendance, surveillance, campagne documentaire supplémentaire ou nouveau binaire ; le comparateur reste PowerShell dans `ci/guard-level1.ps1`.
- Régénérer automatiquement la référence au constat d’un nouvel avertissement.
- Push, installation, tests GPU exécutés, build natif, recette Windows ou nettoyage hors procédure autorisée.

## 10. Final Self-Check

| Check | Result | Notes |
|---|---|---|
| Plan concret, non-placeholder | Pass | 4 tâches et 2 phases fermées. |
| Sections template présentes et ordonnées | Pass | Header, audit, freshness, assumptions, traceability, sequence, tasks, verification, gates, out-of-scope, self-check, prompt. |
| Chaque constat mappé | Pass | Stock, nouveautés, stabilité, garde existante, divergence AGENTS et baseline historisée. |
| Chaque tâche a chemins, dépendances, outline, tests, vérification, critères, rollback | Pass | Détails par tâche. |
| Invariant/métriques/arrêts définis | Pass | Phases et tâches donnent avant/cible/mesure/arrêt. |
| Scope budget par tâche | Pass | Budgets explicites ; choix des fichiers de comparateur sous gate. |
| Commandes avec provenance | Pass | SelfTest N1 et Clippy gardé observés dans le lot ; N1 complet explicitement exclu. |
| Gates distinguent permission et validation | Pass | Identité, AGENTS, baseline et GO séparés. |
| Scope borné | Pass | Un fichier PowerShell, référence, AGENTS et documents autorisés ; aucune dette produit à résorber. |

## 11. Implementation Agent Prompt

Le GO utilisateur autorise P1-T1 à P1-T4 dans le worktree `C:/wt/cliquet-clippy`. Limiter les changements aux fichiers nommés dans le cadrage et à la fonction de probe temporaire retirée sans commit. Implémenter uniquement le comparateur PowerShell et ses fixtures dans `ci/guard-level1.ps1`; ne pas modifier `ci/check-structure.rs`. Garder la référence manuelle et immuable à l’échec. Exécuter SelfTest N1, Clippy workspace JSON par la garde, puis un essai négatif et restaurer le fichier de test. Faire relire par un relecteur générique avec la question prescrite ; maximum deux cycles. Arrêter si le multiset n’est pas rapproché des 248 ou si plus de cinq alertes n’ont pas de ligne source. Aucun push, merge, N1 complet, GPU/native ou nettoyage effectif.
