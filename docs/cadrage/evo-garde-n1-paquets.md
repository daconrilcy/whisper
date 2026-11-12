Avancée : incident reformulé en exigences observables pour le préflight N1.
Blocage : la correction et sa campagne de validation ne sont pas autorisées par ce cadrage.
Décision : isoler un contrat de sélection de paquets pour chaque commande ciblée.

# Cadrage — contrôle exact des paquets Cargo par la garde N1

**Nature :** cadrage d’un correctif de garde ; ce document ne déclenche aucune implémentation.
**État :** À VALIDER — la source de liaison entre le lot protégé et sa politique doit être arbitrée avant que le plan soit exécutable.

## BESOIN

Empêcher `ci/guard-level1.ps1` de lancer une commande Cargo ciblée sur des paquets différents de ceux prévus pour le lot. Le contrôle doit refuser tout paquet ajouté, même s’il est membre valide du workspace, avant l’appel compilant à Cargo.

**Incident rapporté par l’utilisateur :** une commande Clippy prévue pour six crates a sélectionné `whisper-worker-cpu` et `whisper-worker-gpu` à la place de `whisper-desktop` et `whisper-bootstrap`. La garde a laissé passer ces paquets parce qu’ils appartenaient au workspace ; l’appel a lancé `whisper-rs-sys` et configuré CMake. Les builds natifs sont suspendus par `AGENTS.md`.

**État observé dans le dépôt :** le flux de `ci/guard-level1.ps1` appelle actuellement `cargo metadata` avant `Test-CargoArguments` pour les commandes compilantes. Cette séquence doit changer pour un lot protégé : la portée et la sélection exacte doivent être contrôlées avant tout sous-processus Cargo ; après succès, `metadata` continue de valider les membres du workspace. Les SelfTests vérifient des membres connus et invalides, mais pas une autorisation propre au lot. Les formes `-CargoArgs` et `-CargoArgsJson` sont déjà prises en charge. Références : `docs/cadrage/cadrage-clippy-priorites-20261112.md` et `docs/cadrage/plan-correction-clippy-priorites-20261112.md`.

**Résultat attendu :** la garde compare les sélections Cargo normalisées (y compris sélections répétées) à une liste d’autorisation stable et approuvée pour le lot, indépendante des arguments Cargo à valider. Un surplus ou un manque fait échouer le préflight avant toute commande compilante. `--workspace` est refusé lorsqu’une liste explicite est attendue. Les erreurs identifient au minimum chaque paquet inattendu. Les mêmes règles s’appliquent quel que soit le format d’entrée des arguments Cargo.

**Critères d’acceptation :**

- **CA-01 — sélection exacte :** avec une liste attendue et une commande comportant exactement les mêmes paquets, la validation d’arguments réussit.
- **CA-02 — paquet supplémentaire :** avec une liste attendue et un paquet Cargo valide du workspace ajouté, la validation échoue avant l’appel compilant à Cargo et nomme le paquet inattendu.
- **CA-03 — sélection manquante ou dupliquée :** toute différence de cardinalité ou de contenu entre liste attendue et sélection réelle est refusée avant l’appel compilant.
- **CA-04 — portée workspace :** `--workspace` combiné à une liste attendue est refusé avant l’appel compilant.
- **CA-05 — deux formes d’entrée :** les cas positifs et négatifs sont couverts pour `-CargoArgs` et `-CargoArgsJson` ; le cas négatif inclut `whisper-worker-cpu` et/ou `whisper-worker-gpu` comme paquet en surplus.
- **CA-06 — preuve sans appel Cargo sur rejet :** le chemin de rejet pour le lot protégé s’exécute avant tout processus Cargo, y compris `metadata`. Le diagnostic indique « Cargo non lancé » et le SelfTest, via une frontière d’invocation observable, vérifie zéro appel Cargo au total. Après sélection exacte, la garde conserve `metadata` et son contrôle d’appartenance au workspace.
- **CA-07 — maintien des protections :** les validations existantes de catégories, options, appartenance workspace, environnement et racines restent actives ; aucune dépendance ni modification de `Cargo.lock` n’est introduite.

**Périmètre proposé :** `ci/guard-level1.ps1` uniquement, plus les auto-tests intégrés nécessaires dans ce même fichier. La liste de référence de l’incident est : `whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common`, `whisper-worker-parakeet-cpu`. Elle sert au scénario de preuve. La source de l’autorisation pour un lot doit être versionnée et distincte des arguments Cargo ; un simple paramètre libre répété par l’appelant ne constitue pas une source d’autorité suffisante. Le choix du support versionné reste à trancher avant implémentation.

**Hors périmètre :** code produit, manifestes et `Cargo.lock`, dépendances, builds natifs, lancement GPU niveau 2, campagne Clippy de fond, changement des cibles du lot, écriture hors des chemins ci-dessus, commit sur `main`, push et validation Windows.

## ACCORD PRÉCIS POUR CETTE SESSION

**Objectif :** enregistrer ce cadrage et son plan d’implémentation futur dans `docs/cadrage/`.
**Accord explicite :** lecture des fichiers et de Git, création de ces deux documents dans `docs/cadrage/`, branche dédiée et commit local conformément aux règles racine. Aucun code de garde ni commande Cargo n’est autorisé par la rédaction.
**Environnement / base :** dépôt `C:/dev/whisper`, branche de cadrage dédiée issue du `main` observé au début de cette session ; état initial propre.
**Interdictions :** aucune implémentation, commande Cargo, build natif, test GPU niveau 2 ou push dans cette session documentaire.
**Pour le futur lot :** le plan fixe les chemins et les vérifications proposées, mais n’autorise pas leur exécution. Il faut confirmer l’état Git et obtenir un accord écrit précis dans la session d’implémentation avant toute modification de garde ou campagne.
**Règle d’arrêt :** si le lot protégé n’est pas identifié par un contexte versionné indépendant de CargoArgs, si le SelfTest ne peut pas prouver l’absence de tout appel Cargo sur rejet, ou si une validation requise déclenche un build natif suspendu, arrêter et demander l’arbitrage correspondant.

## À FAIRE

1. **[CA-01–CA-05]** Décrire le contrat de normalisation et de comparaison, incluant répétitions, sélection absente, sélection supplémentaire, liste vide et `--workspace`.
2. **[CA-02, CA-05, CA-06]** Définir les cas SelfTest JSON et arguments, avec témoin d’absence de tout sous-processus Cargo dans les rejets (metadata inclus).
3. **[CA-07]** Définir la revue de non-régression des catégories et autres gardes existantes sans campagne Cargo de construction.
4. **[CA-01–CA-07]** Produire le plan futur ci-joint, borné à la garde, ses SelfTests et aux validations autorisées après accord.

## DÉCISIONS À VALIDER

- **D-01 — Source de l’autorisation du lot :** choisir un registre versionné de jeux de paquets nommés dans la garde ou un autre support versionné vérifiable. Un paramètre libre fourni en parallèle de CargoArgs est plus simple, mais permet qu’une erreur recopie la sélection erronée dans les deux valeurs ; il ne satisfait pas seul l’objectif. Recommandation : un identifiant de sélection versionné résolu par la garde en liste de paquets, avec modification explicite de cette entrée quand un lot change.
- **D-02 — Contexte du lot protégé :** lier l’autorisation à un contexte de lot versionné et canonique indépendant de CargoArgs, de sorte que l’omission ou le remplacement d’un identifiant ne désactive pas le contrôle pour le contexte protégé. Recommandation : une politique enregistrée dans la garde, sélectionnée par la racine canonique de lot résolue ; pour toute commande à sélection explicite par `-p`, une racine sans politique enregistrée échoue fermée. Cela impose d’enregistrer les futures racines de lots ciblés. Dans la racine protégée, `--workspace` est refusé car il ne fournit pas la sélection exacte attendue ; hors de cette racine, le comportement général existant est conservé. Tester racine protégée, racine non enregistrée, politique différente et `--workspace` dans la racine protégée.

- **D-03 — Matrice N1 :** décider avant exécution si la validation de cette garde doit inclure la commande N1 complète `cargo test --no-fail-fast` prescrite par `AGENTS.md`. Recommandation : respecter la règle si les préconditions sont satisfaites ; si cette commande nécessite un build natif suspendu ou une commande interdite par la demande, arrêter et demander une décision, sans la remplacer par un N1 partiel présenté comme complet.

## RAPPORT

L’incident provient d’un contrôle d’appartenance au workspace qui ne garantit pas que la commande respecte la sélection du lot. Le préflight doit maintenant refuser avant tout processus Cargo et prouver zéro appel sur les chemins négatifs ; les vérifications de membres continuent après une sélection exacte. Le contexte d’autorisation versionné et l’application de la matrice N1 complète devront être résolus avant exécution.
