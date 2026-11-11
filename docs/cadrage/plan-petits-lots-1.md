Avancée : plan conditionnel PETITS-LOTS-1 dérivé du cadrage revu CLEAN.
Blocage : le N1 complet est bloqué par l’absence du cache Cargo hors ligne.
Décision : appliquer les arrêts par point; aucun contournement de garde n’est prévu.

# Plan — PETITS-LOTS-1

**État :** plan proposé, limité aux conditions et arrêts du besoin. Le présent document n’autorise pas de dépassement de périmètre.

**Base :** branche `petits-lots-1`, worktree `C:/wt/petits-lots`, base technique `267e820f`. Cadrage [evo-petits-lots-1.md](evo-petits-lots-1.md), version `68855e68`, revue indépendante `CLEAN` / `DESIGN READY` pour la planification.

## État des critères à la base observée

| Critère | État constaté | Preuve / suite |
|---|---|---|
| CA-01 | Déjà conforme; aucun commit vide | `.gitignore` couvre `/deliverables/`; `git ls-files -- deliverables/` vide; mentions restantes documentaires. Reconfirmer au préflight et arrêter si un script/test dépend du dossier. |
| CA-02 | Abandon avant édition et auto-tests | `GGML_NATIVE=ON` dans `native/build-config.json` et valeurs effectives de quatre manifestes CPU r2/GPU r5. |
| CA-03 | Abandon avant formatage | `guard-level1.ps1` accepte seulement `fmt --check`, et refuse les deux commandes `--all`. Changer la garde est interdit. |
| CA-04 | SelfTest et N1 à distinguer; N1 complet non prêt | Les sources `target/cargo-home-shared/registry/src` sont absentes. La cible canonique `target/parakeet-tests/cargo-shared` est absente. Un `fetch --locked` seul ne passe pas la garde de la cible canonique. |
| CA-05 | À établir à la clôture | Bilan d’une page à `docs/architecture/PETITS-LOTS-1-BILAN.md`; nettoyage et déplacement seulement proposés. |

## Ordre et règles de branche

Ordre : **T0 → T1 → T2 → T3 → T4 → T5 → T6 → T7**. Les constats de cette base indiquent que T1 est déjà satisfait, T2 et T3 s’arrêtent, et T4 reste non applicable. Les validations finales T6 sont évaluées séparément.

Si une base ultérieure rend T2 ou T3 admissible, préserver les dépendances : les auto-tests Cargo de T2 viennent après le N1 du point 3; aucun commit du point 3 avant revue, build et N1 de son candidat. Si la condition « point 3 en dernier » empêche alors l’ordre demandé, arrêter les points concernés et demander un arbitrage. Ne pas déplacer les changements d’un point dans le commit d’un autre.

Les lectures Git, sources et preuves statiques se font en PowerShell dans `C:\wt\petits-lots` sans précondition de cache Cargo. Chaque tâche de test ou compilation a son propre préflight de garde, ressources, racines autorisées, liens/jonctions et au moins 30 Gio libres. L’absence de cache bloque seulement la campagne Cargo qui le requiert; elle ne bloque pas un SelfTest séparé qui ne l’utilise pas. Résultats Cargo, garde, revue et preuves de build sont des faits distincts. Budget diagnostic maximal : 3 essais ou 30 minutes.

## Tâches

### T0 — Préflight et portée commune

**Responsable :** session principale. **Dépendances :** cadrage et présent plan. **Vérifications en lecture seule :** relever `git --no-optional-locks status --short --branch`, `git --no-optional-locks rev-parse --show-toplevel`, `git --no-optional-locks log -1 --oneline`; lire les `AGENTS.md` et gardes applicables; confirmer worktree `C:/wt/petits-lots`, branche et absence de changements concurrents pertinents. **Arrêt :** état ou racine inattendus, ou changement concurrent invalidant. Ne pas faire dépendre les lectures Git/documentaires des prérequis de test Cargo. Avant chaque campagne ultérieure, son propre préflight vérifie espace libre, racines et reparse points. Aucun changement de garde, test de garde, `native-profiles.json` ou configuration rustfmt; aucune construction native, requalification, N2, merge ou push.

### T1 — `deliverables/` dans l’index et les usages

**Responsable :** session principale; relecture générique pour tout changement de configuration. **Dépendance :** T0. Exécuter la lecture `git ls-files -- deliverables/`, vérifier l’ignore et rechercher les références dans `ci/`, `native/`, `crates/`, `docs/`, `AGENTS.md` et `*.ps1`. Classer les références opérationnelles et documentaires. Si script ou test dépend de `deliverables/`, arrêter ce point. Si déjà ignoré et non suivi, consigner **déjà conforme**, sans commit vide. Si un diff admissible s’avère nécessaire, modifier `.gitignore` et retirer de l’index seulement les fichiers suivis avec `git rm --cached -r -- deliverables/`; vérifier que les fichiers locaux restent présents, faire la revue générique, puis créer `chore: ignore deliverables`. Ne pas déplacer le dossier. **Preuve :** sorties Git/recherche et diff limité.

### T2 — Valeur effective native

**Responsable :** session principale; relecture générique si changement admissible. **Dépendance :** T0. Lire les caches/journaux et manifestes CPU/GPU sous `C:\wb\native-qualif`, `native/build-config.json`, `ci/` et les garde/hash/listes concernées. Attribuer les valeurs effectives aux sources/configurations par empreintes. Sur la base actuelle, CPU et GPU sont ON : arrêter avant édition et auto-tests; ne pas créer le commit. Une autre base ne poursuit que si CPU **et** GPU sont effectivement OFF et si aucun garde/hash/liste figée/requalification ne doit changer. Dans cette seule branche, faire le N1 du point 3 avant les auto-tests conditionnels `ci/guard-native.ps1 -Mode SelfTest` et `ci/test-native-build-control.ps1` si la couverture de `build-config` est prouvée. Aucun build natif. **Preuve :** chemins, valeurs, empreintes et décision.

### T3 — Candidat rustfmt

**Responsable :** session principale coordonne; déléguer le candidat à `rust_implementer` seulement si une voie conforme est établie; `rust_review_coordinator` examine exclusivement si le diff ne change que la mise en forme. **Dépendance :** T0. Lire la présence de `rustfmt.toml` / `.rustfmt.toml` sans les créer ni les modifier, et vérifier l’allowlist. La garde actuelle refuse `cargo fmt --all` et `cargo fmt --all --check`; appliquer l’arrêt et marquer formatage, revue, build et N1 **NON EXÉCUTÉS**. Toute reprise nécessite une voie autorisée pour les deux commandes, le build Rust et le N1, sans changer une garde dans ce lot. Si admissible : formater, vérifier l’idempotence, relire le diff de forme, exécuter `cargo build` et le N1 précommit; arrêter si code non nul, logique modifiée ou construction native impliquée. Aucun commit avant ces preuves.

### T4 — Auto-tests conditionnels de `GGML_NATIVE`

**Responsable :** session principale; relecteur générique. **Dépendances :** T2 devient admissible et N1 précommit T3 réussit. Sinon **NON EXÉCUTÉ**. Vérifier que la cible canonique `target/parakeet-tests/cargo-shared` du worktree existe avant `guard-native` : un N1 avec `-FreshTarget` utilise une cible distincte. Dans la seule branche permise, modifier l’option existante, puis appeler `ci/guard-native.ps1 -Mode SelfTest -LotRoot 'target/n2-native'`; ce chemin est la racine de travail de l’auto-test et ne lance ni campagne ni niveau N2. Exécuter `ci/test-native-build-control.ps1` seulement si sa couverture effective de `build-config` est établie. Toute exigence d’édition d’une garde, liste ou empreinte, tout besoin de requalification ou échec arrête le point. Revue générique, puis commit `build: pin GGML_NATIVE=OFF` seulement pour un diff admissible et validé. Ne pas inclure les fichiers Rust dans son index.

### T5 — Commit de formatage puis blame

**Responsable :** session principale. **Dépendances :** T3 admissible, revue, build et N1 réussis; toute preuve invalidée par T4 doit être refaite avant commit. Créer `style: apply cargo fmt` seulement pour le diff Rust exclusivement formaté. Relever son hash intégral, ajouter ce hash à `.git-blame-ignore-revs` en préservant le contenu, puis créer séparément `chore: ignore fmt commit in blame`. Sans commit de formatage, ne pas créer ou modifier le fichier blame.

### T6 — SelfTest N1 et validation N1 finale

**Responsable :** validateur désigné/session principale. **Dépendances :** T0 et dispositions finales des points. Exécuter séparément `ci/guard-level1.ps1 -SelfTest` après son propre préflight (espace/root de lot, garde et sondes metadata); consigner résultat et code de sortie. Son exécution ne dépend pas de la présence de `registry/src` vérifiée pour le N1 complet et ne le prouve pas. Si le SelfTest échoue, arrêter les campagnes Cargo dépendantes. Pour chaque `cargo test --no-fail-fast` précommit ou final, ouvrir un shell PowerShell dédié conforme à `AGENTS.md`, vérifier les sources `target/cargo-home-shared/registry/src`, le modèle Parakeet personnel et son empreinte, sélectionner une cible admise, et confirmer que la compilation ne requiert aucun mécanisme natif interdit. Le modèle persistant est fourni avec `-PersonalParakeetModelPath <chemin-vérifié-hors-target>`; la garde valide le SHA-256 attendu et fixe `N1_TEST_PERSONAL_PARAKEET_MODEL`. Cette exigence s’applique aussi au N1 précommit T3 si cette branche devenait admissible; chaque N1 s’exécute dans son propre shell dédié. L’absence de `CARGO_TARGET_DIR` dans le processus Cargo s’obtient avec `-WithoutCargoTargetDir`; la garde valide soit la cible canonique existante, soit une `-FreshTarget` distincte. Un N1 de cible neuve ne remplit pas le prérequis de `guard-native`. Au préflight observé, le cache hors ligne manque; inscrire **N1 NON EXÉCUTÉ** si cela demeure vrai. Aucun `fetch --locked` sans mandat et sans cible canonique préexistante. Si admissible, relever code, compte réel et présence `CARGO_TARGET_DIR_child=False`; `944/0/129` est l’attente, non le résultat présumé.

Exemple de commande N1 final si la cible fraîche et les autres prérequis sont admissibles; remplacer le chemin de modèle par la sélection persistante vérifiée avant exécution :

```powershell
.\ci\guard-level1.ps1 -LotRoot 'target/parakeet-tests/petits-lots-1' `
  -WithoutCargoTargetDir -FreshTarget 'n1-final' `
  -PersonalParakeetModelPath '<chemin-vérifié-hors-target>' `
  -CargoArgsJson '["test","--no-fail-fast"]'
```

### T7 — Bilan et propositions de clôture

**Responsable :** session principale. **Dépendance :** dispositions et preuves T1–T6. Rédiger le bilan figé d’une page à `docs/architecture/PETITS-LOTS-1-BILAN.md`, en commençant par `Avancée :`, `Blocage :`, `Décision :`. Pour chaque point, rapporter fait/abandonné, raison, sortie clé et commit réellement créé; distinguer les deux résultats de validation N1. Mentionner la proposition `cargo fmt --check` pour les gardes sans la mettre en œuvre. Inclure les commandes demandées uniquement comme propositions.

Commande de déplacement proposée, à soumettre sans exécution et après vérification de source/destination :

```powershell
if (-not (Test-Path -LiteralPath 'C:\dev\whisper\deliverables' -PathType Container)) { throw 'Source absente.' }
if (-not (Test-Path -LiteralPath 'C:\wb' -PathType Container)) { throw 'Parent destination absent.' }
if (Test-Path -LiteralPath 'C:\wb\deliverables') { throw 'Destination déjà présente.' }
Move-Item -LiteralPath 'C:\dev\whisper\deliverables' -Destination 'C:\wb\deliverables' -ErrorAction Stop
```

Proposer le nettoyage seulement après inventaire des ressources et des commits à préserver, avec le worktree propre et inutilisé. Avant retrait, archiver les preuves de campagne éventuelles en préservant l’arborescence et en vérifiant nombre et SHA-256; vérifier et conserver caches persistants, preuves, livrables, modèles, audios, références de contrôles permanents et ressources de lot encore actives. Si une ressource protégée doit rester sur place, conserver le worktree et signaler le blocage. Sinon, proposer `git -C C:/dev/whisper worktree remove C:/wt/petits-lots`, sans `--force`. Garder la branche locale tant que ses commits ne sont pas conservés ailleurs. Ne supprimer aucun cache ou autre chemin par extension.

## Chemins et reprise

Écritures conditionnelles : `.gitignore` et index de `deliverables/` en T1; déclaration CMake existante en T4; fichiers Rust produits par rustfmt en T3/T5; `.git-blame-ignore-revs` en T5; bilan en T7. Les racines de campagne sont celles établies par la garde, sous `target/`; caches persistants, preuves et ressources protégées ne sont pas supprimés. Aucun script, mécanisme, dépendance, modèle, ou changement de garde n’est créé.

À la reprise, relire `HEAD`, l’index, le diff et les non suivis. Préserver les changements et preuves déjà présents. Si les préconditions diffèrent de la base, refaire les preuves du point concerné et appliquer son arrêt sans élargir le plan. Les validations et commits restent non exécutés dans ce document.
