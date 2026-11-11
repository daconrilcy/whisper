Avancée : cliquet Clippy structurel intégré et vérifié dans `cliquet-clippy`.
Blocage : aucun ; N1 complet n’était pas requis et n’a pas été exécuté.
Décision : 248 alertes préexistantes acceptées ; nouvelles alertes structurelles bloquées.

# CLIQUET-CLIPPY — bilan

**Date :** 11 novembre 2026. **Worktree/branche :** `C:/wt/cliquet-clippy` / `cliquet-clippy`.
**Commit de référence :** `1e043f08cf2bb9eb7a9b35a0fea9798654e9d5d3`.
**Référence :** `ci/clippy-structure-baseline.tsv`, sortie Clippy N1 fraîche, jamais régénérée automatiquement.

## Référence et rapprochement

| Lint | Clés distinctes | Compte multi-ensemble |
|---|---:|---:|
| `too_many_lines` | 179 | 192 |
| `cognitive_complexity` | 40 | 40 |
| `too_many_arguments` | 8 | 8 |
| `type_complexity` | 7 | 8 |
| **Total** | **234** | **248** |

La sortie avait 322 émissions. La déduplication multi-cibles par lint, package, chemin, offsets du span primaire et message laisse **248 sites**, identiques au bilan `CLIPPY-LOCAL`. Le multi-ensemble comporte 9 clés répétées, avec 14 occurrences au-delà de la première ; leur compte exact permet de bloquer une occurrence supplémentaire. **0 alerte** sans ligne ou texte de span source. La clé persistée suit la décision utilisateur : lint, package, chemin relatif normalisé `/`, premier texte du span primaire avec espaces normalisés et conservé jusqu’au premier `(`, `<` ou `{`. Renommer un site forme une nouvelle clé et bloque.

## Implémentation et vérification

- Comparateur PowerShell ajouté à `ci/guard-level1.ps1`, seulement pour Clippy JSON incluant les quatre `-W`. Référence absente/invalide, span incomplet, alerte nouvelle et échec Clippy échouent ; aucune mise à jour de référence automatique.
- SelfTest N1 : **70 positifs / 106 négatifs — PASS**. Fixtures : inchangé, nouveau, retiré, lignes déplacées, doublon multi-cible, occurrences même clé sur une même ligne à offsets différents, lint hors périmètre, référence absente/invalide, span sans texte et code Clippy non nul.
- Clippy workspace `--all-targets --message-format=json` via la garde : **code 0**, `keys=234 sites=248 baseline_keys=234 baseline_sites=248`.
- Probe négatif temporaire de test : la garde a échoué sur le nouveau `clippy::too_many_lines` (`current=1, baseline=0`) ; fichier retiré et absent du diff.
- Revue générique ciblée : premier cycle a trouvé une omission des offsets dans la déduplication ; correction faite et couverte par SelfTest. Deuxième cycle : **CLEAN**, aucun autre défaut sur la question « nouvelle alerte ou échec Clippy masqué ? ».
- `AGENTS.md` corrigé : distinction du contrôle de tests existant et règle N1 + Clippy pour clore un lot Rust. `ci/check-structure.rs` est inchangé.

## Limites et livraison

Le remplacement d’une alerte par une autre portant exactement la même clé peut rester sous le plafond multi-ensemble ; les renommages, eux, bloquent. Cette limite suit l’identité et la comparaison choisies par l’utilisateur. N1 complet, tests Rust complets, tests GPU, validation Windows, merge et push : **NON EXÉCUTÉS**.

Les changements sont conservés localement sur `cliquet-clippy`. Aucun merge/push autorisé. Le worktree et ses caches sont conservés. Commandes de nettoyage proposées, non exécutées : après vérification des processus, des chemins exacts et de la rétention, `git worktree remove C:/wt/cliquet-clippy` puis `git branch -d cliquet-clippy`. Ne pas supprimer les caches persistants avant leur disposition explicite.
