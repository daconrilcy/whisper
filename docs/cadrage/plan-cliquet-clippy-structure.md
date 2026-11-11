Avancée : plan d’implémentation proposé pour le cliquet structurel Clippy.
Blocage : D-01 à D-03 du cadrage restent bloquantes avant tout code ou génération de référence.
Décision : ne réduire aucune alerte existante ; ne bloquer que les nouvelles alertes des quatre lints retenus.

# Plan d’implémentation — Cliquet Clippy structurel

## Header

- **Source audit :** [cadrage du cliquet](evo-cliquet-clippy-structure.md) et [bilan du premier lot](../architecture/CLIPPY-LOCAL-BILAN.md).
- **Planning date :** 2026-11-11.
- **Target workspace :** `C:\dev\whisper`, état `main` propre observé pendant le cadrage.
- **Planning mode :** plan uniquement, aucun code modifié.
- **Confidence :** medium — les points d’intégration sont inspectés, mais l’identité fiable des sites et la fraîcheur de la référence doivent être validées sur une exécution autorisée.
- **Plan quality check :** passed après vérification de la traçabilité, des arrêts et des commandes. Ce plan ne vaut pas accord d’implémentation.

## 1. Audit Digest

Le premier lot a fait passer les alertes hors structure de 96 à 25. Les quatre lints structurels comptaient 248 diagnostics avant et après ; le bilan rapporte zéro nouveau site. La priorité suivante est de préserver ce stock et de prévenir son augmentation, sans refactorer les fonctions existantes pour atteindre les seuils.

La garde d’exécution se trouve dans `ci/guard-level1.ps1`. Elle accepte Clippy et `--message-format=json`, capture les sorties et retourne un échec sur un code Cargo non nul. Son mode `Structure` ne traite que le registre de classification des tests (`ci/check-structure.rs`, `ci/tests-baseline.txt`). Ces fichiers ne sont pas une implémentation de cliquet Clippy. L’`AGENTS.md` fourni indique qu’ils n’existent pas : contradiction avec le dépôt inspecté, à résoudre avant l’implémentation.

La référence Clippy devra être nouvellement produite sur le commit du futur lot. L’export historique n’est pas réputé valide par défaut. La clé d’identité est une décision bloquante, car un numéro de ligne seul est instable et une clé trop large pourrait masquer des alertes inédites.

## 2. Freshness Check

- Inspectés : `ci/guard-level1.ps1` (mode Run/Structure, validation Clippy, capture), `ci/check-structure.rs`, `ci/tests-baseline.txt`, `docs/architecture/CLIPPY-LOCAL-BILAN.md`, `docs/architecture/raw/clippy-diagnostics.json` et `.tsv`.
- Observé : le JSON d’analyse archivé expose package, cible, lint, message, chemin, lignes et une identité de fonction dérivée par l’outil d’analyse. Cette projection n’est pas le format brut Clippy et ne prouve pas que la garde peut retrouver ce champ sans parser le code source.
- Absent : aucune référence dédiée aux alertes Clippy structurelles n’a été observée. `ci/tests-baseline.txt` est un registre de tests, pas un baseline Clippy.
- Non exécuté dans ce cadrage : Cargo, SelfTest, compilation ou tests. Les commandes proposées ci-dessous sont des commandes du futur lot, soumises à son autorisation et au préflight N1.
- État local : `main` propre et alignée avec `origin/main` lors de l’inspection ; les commits du premier lot sont déjà intégrés.

## 3. Planning Assumptions

- **Validation needed — branche de référence :** refaire le Clippy frais sur le commit exact autorisé pour le lot ; expliquer tout écart par rapport aux 248 sites du bilan.
- **Decision gate — identité :** résoudre D-01 du cadrage avant d’inscrire une clé au format de référence.
- **Decision gate — consignes :** vérifier les règles versionnées applicables à la garde et résoudre leur contradiction avec l’AGENTS fourni.
- **Safe — évolution monotone :** une alerte de référence peut disparaître sans échec ; le contrôle n’exige jamais le maintien du stock.
- **Safe — erreur de compilation :** une erreur Clippy reste un échec par le code de sortie non nul ; le cliquet ne transforme pas un échec en succès.

## 4. Finding Traceability

| Finding | Priority | Disposition | Task / gate | Notes |
|---|---|---|---|---|
| 248 alertes structurelles existantes ne sont pas un objectif de refactor | High | Planned | P1-T1, P1-T2 | Stock de référence accepté ; pas de réduction exigée. |
| Toute nouvelle alerte parmi quatre lints doit être bloquée | High | Planned | P1-T2 à P1-T4 | Échec explicite et site signalé. |
| Déplacement de ligne ne doit pas compter comme nouveau site | High | Decision gate | G1, P1-T1 | Choix après essais sur les diagnostics observés. |
| Contrôle `Structure` existant concerne la classification des tests | Medium | Already resolved | P1-T2 | Ne pas le présenter comme cliquet Clippy existant. |
| `AGENTS.md` fourni et dépôt divergent sur les fichiers de structure | High | Decision gate | G2 | Vérifier le texte versionné et arbitrer avant code. |
| Référence historique non garantie à jour | High | Planned | P1-T1 | Mesure fraîche obligatoire sur le commit autorisé. |
| 25 alertes hors structure | Low | Deferred | Hors périmètre | Aucun traitement dans le cliquet. |

## 5. Recommended Sequence

Résoudre d’abord les décisions sur l’identité et les consignes, puis établir une référence fraîche, réaliser la comparaison isolée et enfin la brancher sur le chemin Clippy gardé. La référence ne doit jamais être régénérée en réaction automatique à une alerte nouvelle.

### Phase 1 : Verrouiller l’invariant de non-régression

Objective: empêcher l’augmentation du stock des quatre lints sans forcer sa réduction.
Closed invariant: après Clippy réussi, chaque diagnostic de ces lints correspond à un site accepté par la référence ; tout site nouveau fait échouer le contrôle.
Before metrics: 248 sites structurels dans le bilan du 2026-11-11 ; 25 alertes non structurelles hors périmètre.
Target metrics: 0 nouveaux sites tolérés ; 0 alerte existante supprimée exigée ; 4 lints couverts.
Metric commands: commande Clippy JSON gardée de P1-T1, puis contrôles du comparateur sur fixtures.
Stop conditions: identité instable/ambiguë ; écart non expliqué avec 248 ; changement requis aux commandes ou garanties N1 ; auto-test négatif qui ne détecte pas une alerte nouvelle.
Dependencies: G1 et G2.
Expected outcome: choix d’identité et référence versionnée fidèle à la baseline fraîche.
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

#### P1-T1 : Arrêter l’identité des diagnostics et établir la référence fraîche

- **Priority:** High
- **Source finding:** CA-01, CA-03, CA-06 ; décision D-01/D-02.
- **Rationale:** le comparateur n’est sûr que si les sites sont distingués sans être attachés à des numéros de lignes fragiles.
- **Target files/modules:** `ci/guard-level1.ps1` pour la capture à examiner ; sorties Clippy JSON temporaires sous les racines autorisées ; futur fichier de référence dédié sous `ci/` (nom à fixer après résolution G1).
- **Ownership bucket:** garde locale et données de référence versionnées.
- **Scope budget:** 0 fichier de production Rust ; au plus 1 fichier de référence, après validation de la clé. Mesure sans édition de garde.
- **Closed invariant:** la référence représente le stock du commit choisi et la clé distingue les fonctions/site sans dépendre seulement de la ligne.
- **Before metrics:** 248 sites selon le bilan ; aucune référence Clippy dédiée observée.
- **Target metrics:** référence fraîche couvrant exactement les diagnostics des 4 lints ; différence expliquée et approuvée avant versionnement.
- **Metric commands:** Clippy JSON workspace gardé et outil local de comparaison/inspection ; aucun outil précis d’empreinte externe n’est présumé.
- **Stop conditions:** résultat ne permet pas de construire une clé stable ; compte différent de 248 sans explication ; consignes versionnées interdisent ce fichier ou son usage.
- **Dependencies:** G1 identité, G2 consignes.
- **Implementation outline:**
  1. lancer la commande Clippy gardée au commit retenu et extraire seulement les quatre codes lint.
  2. comparer les candidates de clé sur les alertes existantes, doublons multi-cibles et fixtures de lignes déplacées.
  3. retenir une clé qui ne fusionne pas deux sites distincts ; documenter faux positifs/faux négatifs connus.
  4. versionner la référence résultante avec provenance de révision et comptage, sans écrire dans le bilan clos.
- **Tests to add or update:** scénarios de fixtures du comparateur prévus en P1-T2 ; aucun test produit.
- **Verification commands:** commande Clippy gardée décrite en matrice ; inspection que 4 codes seuls sont présents dans la référence.
- **Command provenance:** Clippy de mesure observé au bilan ; commande de reprise prescrite par AGENTS, à exécuter seulement après autorisation.
- **Acceptance criteria:** la référence est générée à partir du commit de lot ; chaque entrée correspond à un site Clippy réel ; les quatre comptes sont rapportés et leur somme dédupliquée est rapprochée des 248 diagnostics.
- **Rollback/fallback notes:** si l’identité ne passe pas les fixtures, ne pas versionner de référence ; garder les sorties de campagne conformément au plan autorisé et demander décision.
- **Notes for implementation agent:** ne pas reprendre directement `clippy-diagnostics.json` comme source baseline ; c’est une projection historique et la fraîcheur n’est pas garantie.

#### P1-T2 : Implémenter le comparateur avec tests déterministes

- **Priority:** High
- **Source finding:** CA-01 à CA-05.
- **Rationale:** séparer l’algorithme de comparaison de l’orchestration PowerShell rend les règles testables avant branchement à Cargo.
- **Target files/modules:** cible principale à confirmer après G1 : `ci/check-structure.rs` ou nouveau module dédié ; fixtures internes au SelfTest ; référence produite en P1-T1.
- **Ownership bucket:** outil local de garde et tests de garde.
- **Scope budget:** maximum 2 fichiers de garde et 1 référence, fichiers de test/fixtures inclus dans le plafond uniquement s’ils restent légers et déterministes ; tout dépassement requiert justification de l’atomicité dans le diff.
- **Closed invariant:** comparaison d’ensembles de sites ne dépendant pas du numéro de ligne seul.
- **Before metrics:** aucun comparateur Clippy ; 0 cas SelfTest cliquet.
- **Target metrics:** 6 cas minimum : inchangé passe, nouveau échoue, retiré passe, ligne déplacée passe, référence malformée échoue, lint hors périmètre ignoré.
- **Metric commands:** `rustc --edition=2024 ci/check-structure.rs -o <sortie>` n’est pas retenue tant que l’outil existant n’a pas été choisi ; SelfTest par la garde après implémentation.
- **Stop conditions:** comparaison nécessite d’accepter une clé ambiguë ; références malformées sont tolérées ; parser ou fixture nécessite un accès réseau ou un crate nouveau.
- **Dependencies:** P1-T1.
- **Implementation outline:**
  1. implémenter lecture/validation stricte de la référence et des messages JSON Clippy capturés.
  2. comparer les quatre lints par clé établie, en tolérant les entrées retirées.
  3. faire échouer sur entrée invalide et inclure lint, package/cible et site dans le diagnostic d’échec.
  4. exposer une fonction SelfTest déterministe avec fixtures sous le répertoire d’exécution autorisé.
- **Tests to add or update:** les six cas listés dans les cibles ; ne pas ajouter des scénarios produit.
- **Verification commands:** SelfTest N1 après ajout du comparateur.
- **Command provenance:** règles N1 et mode SelfTest observés dans `ci/guard-level1.ps1`; compilation exacte à choisir selon l’outil retenu.
- **Acceptance criteria:** les six cas produisent les résultats attendus ; aucun accès réseau ni changement Cargo manifests/lockfile ; erreurs et alertes inédites restent distinguées.
- **Rollback/fallback notes:** retirer le comparateur et ses fixtures si une règle ne peut être rendue stricte ; ne jamais neutraliser l’échec pour faire passer le stock.
- **Notes for implementation agent:** `ci/check-structure.rs` existe mais son rôle est la classification des tests ; ne pas y fusionner le parser Clippy sans justifier l’ownership et le point d’intégration.

#### P1-T3 : Brancher la comparaison au chemin Clippy gardé

- **Priority:** High
- **Source finding:** CA-02, CA-04, CA-05.
- **Rationale:** le cliquet doit s’appliquer à la sortie Clippy réelle dans la garde qui contrôle déjà les commandes et leurs racines.
- **Target files/modules:** `ci/guard-level1.ps1`, comparateur choisi en P1-T2, référence de P1-T1.
- **Ownership bucket:** orchestration de garde locale.
- **Scope budget:** maximum 2 fichiers de garde plus 1 référence ; toute fixture supplémentaire doit rester dans la cible du SelfTest et justifier le dépassement.
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
- **Scope budget:** 0 fichier production ; correction éventuelle seulement dans les fichiers autorisés P1-T1 à T3.
- **Closed invariant:** le candidat accepte la référence, bloque les nouveaux sites et respecte le préflight N1.
- **Before metrics:** 248 alertes structurelles actuelles ; résultat de garde sans cliquet.
- **Target metrics:** SelfTest PASS ; comparaison Clippy complète PASS ; toute fixture négative est refusée ; 0 nouvelle alerte au départ du lot.
- **Metric commands:** `-Mode SelfTest`, puis Clippy workspace `--all-targets --message-format=json` par `-Mode Run` ; détails en matrice.
- **Stop conditions:** moins de 30 Gio libres ; préflight/selftest échoue ; site ajouté non expliqué ; différence de baseline ; échec compilation ou écriture hors racines ; activité Git pertinente concurrente.
- **Dependencies:** P1-T1 à T3.
- **Implementation outline:**
  1. vérifier l’état Git et protections N1, l’espace libre, les racines et l’absence de conflit pertinent.
  2. lancer le SelfTest par la garde.
  3. lancer le Clippy workspace gardé sur le candidat avec cible autorisée neuve ou partagée selon le lot approuvé.
  4. vérifier décompte, résultats positifs/négatifs, diff, et consigner les limites ; arrêter si un critère manque.
- **Tests to add or update:** aucun scénario produit ; tests de garde de P1-T2/T3 seulement.
- **Verification commands:** matrice ci-dessous.
- **Command provenance:** Repo docs et source observée ; la cible exacte est décidée au préflight du lot.
- **Acceptance criteria:** N1 SelfTest réussi ; résultat Clippy code 0 ; 248 alertes initiales reconnues ou écart expliqué ; fixtures négatives échouent comme prévu ; aucun effet produit ni changement hors scope.
- **Rollback/fallback notes:** en échec, interrompre la fermeture, garder sorties et ne pas régénérer le baseline. Ne pas nettoyer sans appliquer les règles du lot autorisé.
- **Notes for implementation agent:** validation N1 uniquement ; pas de test GPU exécuté, validation Windows, campagne native, push ou livraison produit implicites.

## 7. Verification Matrix

| Area | Command or check | Provenance | Expected result | Covers tasks | Notes |
|---|---|---|---|---|---|
| Garde SelfTest | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/clippy-ratchet` | Repo docs/source observed | SelfTest PASS avant Clippy | P1-T2–T4 | Future lot seulement ; préflight N1 complet. |
| Baseline fraîche et intégration | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/clippy-ratchet -FreshTarget clippy-ratchet -CargoArgsJson '["clippy","--workspace","--all-targets","--message-format=json","--","-W","clippy::too_many_lines","-W","clippy::cognitive_complexity","-W","clippy::too_many_arguments","-W","clippy::type_complexity"]'` | Repo docs/source observed | Code 0 ; 4 lints comparés ; zéro site nouveau ; stock initial rapproché de 248 | P1-T1, P1-T3, P1-T4 | La garde ajoute les options hors-ligne/verrouillées prévues ; exécution soumise à accord futur et à ses prérequis. |
| Comparateur | SelfTest interne : inchangé, nouveau, retiré, ligne déplacée, hors périmètre, référence invalide | Source inferred | Résultats conformes aux 6 critères déterministes | P1-T2, P1-T3 | Fixtures synthétiques, aucun Cargo requis pour les cas unitaires. |
| Revue manuelle | inspection clé, référence, messages d’erreur, `git diff --check`, état Git | Repo docs | Aucune clé ambiguë ni fichier hors plan | Tous | Git séparé de Cargo. |
| Limites | aucune validation Windows/native/GPU niveau 2 | Repo docs | explicitement NON EXÉCUTÉE | P1-T4 | Le lot porte sur la garde locale. |

## 8. Decision Gates

| Gate | Status | Blocks | Question | Recommended default | How to validate |
|---|---|---|---|---|---|
| G1 — Clé d’identité | Blocking | P1-T1–T3 | Quels champs identifient un site sans dépendre de ses lignes ni fusionner deux alertes ? | Clé composite prouvée par fixtures ; si l’incertitude demeure, ne pas activer le cliquet | Tester doublons, déplacement, changement de métrique et fonctions voisines sur JSON brut. |
| G2 — AGENTS applicable | Blocking | Tout code | Le texte d’AGENTS versionné autorise-t-il la modification et comment concilier l’écart local ? | Suivre la version en dépôt ; arrêter si contradiction de permission subsiste | Relire `AGENTS.md` depuis le commit futur et demander arbitrage si nécessaire. |
| G3 — Référence initiale | Validation | P1-T1/T4 | Le Clippy frais du commit de lot produit-il les 248 sites décrits ? | Établir une nouvelle référence et expliquer tout écart avant validation | Sortie JSON Clippy gardée et rapprochement avec le bilan. |
| G4 — Élargir la référence | Blocking au moment concerné | Toute alerte volontairement nouvelle | Peut-on accepter cette nouvelle alerte ? | Pas de mise à jour implicite ; décision explicite dans le changement qui l’introduit | Diff de référence, cause, expiration/réexamen si pertinent et revue. |
| G5 — GO d’implémentation | Blocking | P1-T1–T4 | L’utilisateur autorise-t-il ce lot de garde, ses fichiers et ses commandes ? | Aucun défaut ne vaut autorisation | Accord précis dans la session d’implémentation, après fermeture de G1/G2. |

## 9. Out of Scope

- Réduire les fonctions existantes ou les 248 alertes structurelles pour leur métrique seule.
- Modifier les 25 résiduels hors structure ou toucher la logique Whisper.
- Changer les seuils, lints demandés, membres workspace, règles Cargo ou isolation N1.
- Ajouter CI distante, dépendance, manifeste d’empreintes générique, surveillance, campagne documentaire supplémentaire ou binaire produit.
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
| Commandes avec provenance | Pass | N1 vient des consignes et du script observés ; Cargo non exécuté ici. |
| Gates distinguent permission et validation | Pass | Identité, AGENTS, baseline et GO séparés. |
| Scope borné | Pass | Garde seulement ; aucune dette produit à résorber. |

## 11. Implementation Agent Prompt

Après accord explicite distinct et fermeture de G1/G2, déléguer l’implémentation de P1-T1 à P1-T4 à `rust_implementer` en mode `IMPLEMENT`, en commençant par la mesure fraîche et la clé, avant toute intégration. Respecter les fichiers et budgets de ce plan, préserver les changements tiers, ne jamais mettre à jour la référence automatiquement, et exécuter uniquement SelfTest N1 puis le Clippy workspace gardé défini dans la matrice. Arrêter sur tout écart non expliqué, échec de préflight ou besoin de modifier les règles N1. La session principale confiera ensuite la revue au coordinateur Rust prévu par l’AGENTS ; aucun push ni validation GPU/native/Windows n’est inclus.
