# Plan — Profil de mesure cognitive Clippy isolé

Avancée : le workflow P22 exige un score cognitif exact des helpers, et le profil Clippy courant applique le ratchet au seuil projet 25.
Blocage : sans profil de mesure dédié, le seuil 0 émet des diagnostics historiques et échoue au ratchet avant de fournir une mesure exploitable.
Décision : ajouter un mode gardé, isolé et non ratcheté pour cette seule mesure; conserver inchangés le profil et le ratchet standards.

## Mandat et autorisation

L’utilisateur autorise dans la session du 14 novembre 2026 un lot dédié qui ajoute à `ci/guard-level1.ps1` et ses SelfTests un profil Clippy de mesure seule au seuil cognitif 0. Le lot ne change ni `ci/clippy-structure-baseline.tsv`, ni `clippy.toml`, ni le comportement de la commande Clippy standard. Il sert à mesurer les refactors structurels P22–P30 sous la garde existante.

## Constat vérifié

La garde versionne pour `target/parakeet-tests/clippy-corrections-suivantes-20261113` un profil exact Clippy à six crates et quatre lints, et applique `Assert-StructuralClippyPassed` ainsi que le ratchet structurel. `CLIPPY_CONF_DIR` est contrôlé canoniquement sous le LotRoot. La configuration Clippy du dépôt fixe `cognitive-complexity-threshold = 25`. Une commande à seuil 0 sous la voie standard générerait les diagnostics des fonctions historiques et ferait échouer le ratchet avant mesure des helpers.

Clippy lit `clippy.toml` ou `.clippy.toml` dans le dossier désigné par `CLIPPY_CONF_DIR`; la clé de configuration est `cognitive-complexity-threshold` ([documentation de configuration Clippy](https://doc.rust-lang.org/nightly/clippy/lint_configuration.html), [implémentation du chargeur de configuration](https://github.com/rust-lang/rust-clippy/blob/master/clippy_config/src/conf.rs)).

## Portée

- Un fichier de code : `ci/guard-level1.ps1`, avec ses auto-tests internes.
- Un nouveau mode explicitement opt-in de mesure; n’élargir aucune autre catégorie Cargo.
- La sélection reste le profil exact Clippy à six membres, `--all-targets --message-format=json` et quatre lints déjà admis, pour le LotRoot existant seulement.
- La garde fixe `cognitive-complexity-threshold = 0` dans un `clippy.toml` temporaire sous le run root gardé et passe son dossier par `CLIPPY_CONF_DIR` au processus cargo.
- Le profil ne met pas à jour ni ne compare la baseline structurelle; il doit quand même vérifier l’exit code Cargo, capturer stdout/stderr/JSON et appliquer tous les préflights, variables isolées, racines, cible Cargo, surveillance et arrêt sur erreur existants.
- La commande Clippy standard reste inchangée et continue d’appliquer le ratchet versionné. Le mode mesure ne peut pas exécuter `test`, télécharger, sélectionner un autre membre, utiliser `--workspace`, changer la cible hors des règles ou changer le LotRoot.
- Aucun nouveau binaire, dépendance, seuil permanent, fichier de configuration versionné, cible compilation, téléchargement, build natif, merge ou push.

## Tâche GCM-T1 — Ajouter le profil mesure seule et ses SelfTests

Introduire une option/mode gardé distinct de `Run` qui :

1. refuse avant Cargo tout LotRoot différent, toute commande différente du profil Clippy exact ou tout argument inattendu;
2. crée sous le run root un dossier de configuration neuf, sans lien/jonction; lit le `clippy.toml` versionné; remplace uniquement la ligne unique `cognitive-complexity-threshold = 25` par `... = 0`; refuse ligne absente, doublée, mal formée ou toute autre dérive;
3. fixe `CLIPPY_CONF_DIR` sur ce dossier après le nettoyage environnemental de la garde et avant préflight/lancement; confirme canoniquement qu’il reste sous le LotRoot;
4. capture la même commande cargo exacte via `Invoke-CargoCaptured`, exige exit code 0, conserve les logs sous `scratch/<run-id>/`, et signale explicitement `measurement-only`, seuil 0 et `ratchet=not-applied`;
5. n’appelle pas le comparateur baseline dans ce seul mode; tous les appels standards restent soumis au comparateur strict;
6. ajoute des SelfTests sans Cargo qui prouvent l’acceptation du seul profil prévu, le refus de profils/membres/racines inattendus, le rendu/validation TOML, la fixation d’environnement sous le LotRoot, le skip du ratchet uniquement pour le mode mesure, et le maintien du ratchet pour le mode standard.

## Critères d’acceptation

- `ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/clippy-corrections-suivantes-20261113` passe sans lancer Cargo et les cas positifs/négatifs couvrent chaque nouvelle branche.
- Le mode standard est identique avant/après : même profil exact, même ratchet Clippy, aucun changement baseline.
- La mesure ne lance qu’un Clippy gardé à six crates; toutes les écritures restent sous LotRoot/target déclaré; CLIPPY_CONF_DIR pointe sur le seul fichier temporaire du run.
- Exit non-zéro de Cargo reste bloquant; JSON invalide ou absence de build-finished réussi ne peut pas être rapportée comme mesure réussie.
- `git diff --check` passe; le diff est limité au plan et à `ci/guard-level1.ps1`.
- Aucun Cargo de compilation ne sert à valider ce changement de garde; les SelfTests suffisent. Les vérifications Clippy/N1 de P22 reprennent ensuite via les deux modes prévus.

## Vérification, arrêt et retour arrière

Exécuter les SelfTests avec la garde et inspecter le diff/les chemins statiquement. Au premier échec d’auto-test, arrêt; ne pas lancer de campagne Clippy de mesure tant que la garde n’est pas corrigée et relue. Ne modifier ni la baseline ni les seuils de production. Le retour arrière retire uniquement le nouveau mode et ses tests après preuve que les protections standard restent intactes; garder les logs/preuves selon les règles du lot.

## Clôture et limites

Ce lot est un préalable de mesure pour P22–P30, pas une réduction des alertes et pas une validation de produit. Après SelfTests réussis, le profil mesure sert au score exact des helpers; le Clippy six crates au seuil 25 avec ratchet, N1 complet, sélections worker/PE et revue indépendante restent les gates de chaque tâche. Aucun push ni commit de code n’est inclus.
