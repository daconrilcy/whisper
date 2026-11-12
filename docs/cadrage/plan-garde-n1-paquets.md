Avancée : plan dédié préparé pour fermer l’écart de sélection Cargo décrit par l’incident.
Blocage : aucune implémentation ni campagne Cargo n’est autorisée par ce plan seul.
Décision : obtenir un accord précis en session dédiée avant de modifier ou valider le code.

# Plan d’implémentation — sélection exacte des paquets dans la garde N1

**État : EN COURS — D-02 et le refus global de `--workspace` ont été acceptés le 12 novembre 2026 ; la validation N1 complète reste conditionnée au préflight et aux builds natifs suspendus.**

## 1. Référence et périmètre

- Cadrage : [evo-garde-n1-paquets.md](evo-garde-n1-paquets.md).
- Source du défaut : [ci/guard-level1.ps1](../../ci/guard-level1.ps1), fonction `Test-CargoArguments` et `Invoke-SelfTests`.
- Incident : six paquets prévus, mais `whisper-worker-cpu` et `whisper-worker-gpu` ont été acceptés à la place de `whisper-desktop` et `whisper-bootstrap`, déclenchant un build `whisper-rs-sys`/CMake. Rapporté par l’utilisateur ; non reproduit durant le cadrage.
- Périmètre autorisable proposé : `ci/guard-level1.ps1`, uniquement.
- Aucun nouveau livrable, dépendance, changement manifeste ou code produit.

## 2. Contrat proposé

1. Enregistrer dans la garde une politique versionnée attachée au contexte canonique de lot résolu, indépendante des arguments Cargo. Recommandation : utiliser `LotRoot` canonique ; toute commande à sélection explicite avec une racine sans politique enregistrée échoue fermée, l’omission d’un identifiant facultatif ne peut donc pas désactiver le contrôle. Cette approche impose l’enregistrement des racines des lots ciblés futurs. Confirmer ce contrat avant code ; sinon arrêter pour arbitrage.
2. Normaliser les sélections Cargo reconnues (`-p name` et `-pname`) sans changer la commande transmise à Cargo.
3. Pour les commandes `-p` dont la racine de lot est protégée ou enregistrée, résoudre la politique versionnée et comparer les multisets triés : aucune omission, aucun surplus, aucun doublon. Refuser globalement `--workspace`, y compris hors d’une racine enregistrée, pour empêcher qu’un changement de `LotRoot` contourne le contrôle.
4. Produire un message de préflight qui nomme chaque paquet Cargo supplémentaire et distingue, si utile, les paquets attendus absents.
5. Déplacer l’identification du contexte protégé et la comparaison de sélection avant tout sous-processus Cargo, y compris `cargo metadata`. Sur rejet, écrire un diagnostic clair « Cargo non lancé ». Après sélection exacte seulement, appeler `metadata` et conserver la validation des membres du workspace.
6. Pour le contexte canonique protégé, appliquer la politique sans paramètre facultatif ; refuser les tentatives d’identifier un autre jeu ou une portée implicite. Refuser également toute commande `--workspace` globale, selon l’arbitrage utilisateur du 12 novembre 2026.

**Liste de scénario issue de la campagne précédente :** `whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common`, `whisper-worker-parakeet-cpu`. Le plan d’implémentation doit l’enregistrer sous un identifiant de sélection propre à ce lot dans la source versionnée retenue ; ne pas en faire une liste globale appliquée aux autres commandes.

## 3. Tâches

| Tâche | Travail | Critères | Preuve / arrêt |
|---|---|---|---|
| T0 | Préflight : confirmer accord précis, branche/worktree dédié, état Git, espace libre ≥30 Gio, consignes racine et garde ; relire le diff final de cadrage. | CA-07 | Pas de modification si base inattendue ou accord absent. |
| T1 | Appliquer la décision acceptée : lier politique versionnée et racine canonique ; refuser les racines non enregistrées pour toute sélection `-p`, ainsi que tout `--workspace`. | CA-01, CA-03, CA-04 | Tester racine protégée, racine non enregistrée, sélection différente et `--workspace` hors comme dans la racine protégée. |
| T2 | Implémenter le préflight de sélection avant tout processus Cargo, puis résoudre la politique et normaliser les sélections `-p` pour comparaison exacte, y compris multiplicité. | CA-01–CA-04 | Messages clairs pour surplus, absence et doublon ; sur rejet, Cargo non lancé. Après succès seulement, `metadata` conserve le contrôle d’appartenance workspace. |
| T3 | Étendre le SelfTest positif : liste de six attendue et six sélectionnée exactement ; inclure les formes séparées `-p name` et accolée `-pname` si supportées actuellement. | CA-01, CA-05 | Validation pure, aucun Cargo compilant. |
| T4 | Étendre les SelfTests négatifs : paquet de workspace supplémentaire, en particulier CPU/GPU, paquet attendu manquant, répétition et `--workspace` avec liste attendue. | CA-02–CA-05 | Chaque échec indique le paquet ou la différence pertinente. |
| T5 | Prouver via une frontière d’invocation observable que les refus de portée, paquet, racine/identifiant et `--workspace` précèdent tout processus Cargo. | CA-06 | Compteur total des sous-processus Cargo reste à zéro sur chaque refus ; diagnostics indiquent « Cargo non lancé ». Si impossible sans Cargo réel, arrêter et corriger le dispositif avant validation. |
| T6 | Résoudre l’applicabilité de la matrice N1 et les prérequis, puis exécuter les vérifications autorisées, relire diff et établir les codes de sortie. | CA-05–CA-07 | Détails en section 4 ; aucun `--workspace` dans les commandes ciblées ; arrêt si validation imposée requiert un build natif suspendu. |
| T7 | Revue indépendante du candidat final et fermeture des constats dans la limite de deux cycles. | CA-01–CA-07 | Verdict CLEAN ou LIVRÉ AVEC RÉSERVES ; la validation produit/native n’est pas revendiquée. |

## 4. Vérifications proposées après autorisation

Toutes les commandes restent conditionnées au préflight permanent de `AGENTS.md`, à un worktree de lot et à l’accord précis de la session d’implémentation. Le plan n’autorise pas leur exécution. Les contrôles ci-dessous sont une validation ciblée de la garde ; ils ne seront pas rapportés comme un N1 complet si la commande obligatoire `cargo test --no-fail-fast` n’a pas été exécutée.

1. SelfTest dédié de la règle : positifs/négatifs des formes arguments et JSON ; instruments la frontière réelle de processus Cargo et vérifie zéro appel total pour un paquet supplémentaire (CPU et GPU), une omission, une mauvaise racine/politique et `--workspace`. Le SelfTest complet existant peut lui-même appeler `cargo metadata` ailleurs ; ces appels non compilants doivent être distingués des sous-cas négatifs instrumentés.
2. Cas négatif de garde avec le contexte canonique protégé : arguments JSON sélectionnent `whisper-worker-cpu` ou `whisper-worker-gpu` en surplus ; code non nul, paquet signalé, diagnostic « Cargo non lancé », compteur total Cargo à zéro. Refaire avec `-CargoArgs`.
3. Cas positif avec le contexte protégé et six paquets sélectionnés exactement : `-DryRun` accepté pour les deux formes. Cet appel positif peut lancer `cargo metadata` pour maintenir la validation des membres ; il ne lance pas la commande compilante.
4. Cas `--workspace` dans le contexte protégé et sous une autre racine : rejet avant tout Cargo, code non nul, message clair et compteur total à zéro.
5. `git diff --check` séparé ; revue du diff pour s’assurer que seuls la garde et ses SelfTests changent.

La validation ciblée ne lance pas de commande Clippy, build natif, activation CUDA, test GPU de niveau 2, `--workspace` ciblé ou test d’intégration de dépendances natives. Cependant, toute validation N1 est soumise à la règle racine incluant `cargo test --no-fail-fast` sans variable de cible. Avant exécution, vérifier les prérequis, la disponibilité du cache et si cette commande déclenche un build natif suspendu. Si elle le ferait, ou si la demande interdit la commande, arrêter et demander l’arbitrage ; ne pas contourner la règle ni appeler le résultat un N1 complet.

## 5. Gates, revue et clôture

- L’implémentation a été demandée explicitement le 12 novembre 2026 ; l’utilisateur a choisi le refus global recommandé pour `--workspace`. Aucun commit sur `main` ni push n’est autorisé.
- Aucun changement hors de `ci/guard-level1.ps1` sans arbitrage de portée.
- Le SelfTest négatif doit prouver zéro sous-processus Cargo dans le chemin de rejet ; un simple contrôle de code d’erreur n’est pas suffisant. Il vérifie aussi que la garde résout la politique à partir du contexte canonique versionné et refuse les racines non enregistrées ou les sélections différentes.
- Revue indépendante requise sur le candidat final selon `AGENTS.md`, au plus deux cycles de revue/correction.
- Aucun commit sur `main`, aucun push. Le commit local éventuel est limité à la branche du lot et dépend de l’accord prévu par les règles applicables.
- Le bilan final commence par les trois lignes simples prescrites par `AGENTS.md` et distingue résultats SelfTest, `-DryRun`, diff-check et revue.
