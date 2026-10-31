Avancée : relevé statique actualisé au 31 octobre 2026.
Blocage : exécution native et outils absents [Non vérifié].
Décision : description de l’existant, sans proposition de transformation.

# whisper-bootstrap

## Rôle et type

Construit une vue par défaut et référence la version IPC et le type du transport. `crates/whisper-bootstrap/src/main.rs:1-10`

Son point d’entrée réserve les opérations d’installation dans un commentaire. `crates/whisper-bootstrap/src/main.rs:1-10`

Type : **Binaire**. `crates/whisper-bootstrap/src/main.rs:1-10`

Mesures : **10 production / 0 tests** ; lignes physiques, vides/commentaires inclus. Sources par fichier ci-dessous ; [méthode](../raw/METHOD.md).

## Dépendances déclarées

| Dépendance | Nature | Groupe/condition | Déclaration | Source |
| --- | --- | --- | --- | --- |
| whisper-core | interne | dependencies | {"workspace": true} | `crates/whisper-bootstrap/Cargo.toml:9` |
| whisper-adapters | interne | dependencies | {"workspace": true} | `crates/whisper-bootstrap/Cargo.toml:10` |

Metadata et Clippy workspace/all-targets ont terminé avec code 0 ; [résultats](../raw/cargo-results.json). [Non vérifié] Transitives (metadata sans dépendances), autres combinaisons de features et exécution des tests. Features déclarées : `crates/whisper-bootstrap/Cargo.toml:1-13`.

## Interfaces publiques et usages intercrates

[Non vérifié] Aucun import consommateur d’une autre crate relevé ; les autres usages ne sont pas exclus.

Définitions publiques détaillées par module ci-dessous ; réexportations de racine : `crates/whisper-bootstrap/src/main.rs:1-10`. Imports textuels, pas un graphe de symboles résolu.

## Frontières constatées

| Frontière (repérage textuel) | Sites hors tests, sous leurs cfg |
| --- | --- |
| Disque | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Processus | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Threads/canaux | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Entrées/sorties standard | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| GPU/natif | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |
| Réseau explicite | [Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus. |

Les sites désignent du code présent, pas une exécution ; [lignes brutes](../raw/rg-threads-processes-io.txt).

## Carte des fichiers et modules

| Module/cible | Responsabilité et source | Prod | Tests |
| --- | --- | --- | --- |
| crate | Construit une vue et référence le protocole/transport. `crates/whisper-bootstrap/src/main.rs:1-10` | 10 | 0 |

## Déclarations de modules

| Déclaration | Forme | Contexte | Cible/plage | Source |
| --- | --- | --- | --- | --- |

## Détail des modules

[Non vérifié] Relations `use crate::` textuelles : alias, réexportations, glob imports, chemins `self`/`super`, références hors `use`, macros et cibles distinctes ne forment pas ici un graphe complet.

### crate — `crates/whisper-bootstrap/src/main.rs`

Construit une vue et référence le protocole/transport. `crates/whisper-bootstrap/src/main.rs:1-10`

Mesure : 10 production / 0 tests / 10 physiques. `crates/whisper-bootstrap/src/main.rs:1-10`

Types : aucun struct/enum/trait/alias relevé dans `crates/whisper-bootstrap/src/main.rs:1-10`.

Fonctions avec corps : `main` `crates/whisper-bootstrap/src/main.rs:5-10`.

Déclarations `pub` (visibilité sous conditions) : aucune relevée dans `crates/whisper-bootstrap/src/main.rs:1-10`.

Dépend de, via `use crate::` : [Non vérifié] Aucun import de cette forme relevé. `crates/whisper-bootstrap/src/main.rs:1-10`.

Utilisé par, via ces imports : [Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée..

## Comptages textuels

| Motif | Production | Tests |
| --- | --- | --- |
| builder_spawn | 0 | 0 |
| cfg_feature | 0 | 0 |
| cfg_test | 0 | 0 |
| command_new | 0 | 0 |
| expect | 0 | 0 |
| thread_spawn | 0 | 0 |
| unsafe | 0 | 0 |
| unwrap | 0 | 0 |

Chaque occurrence sourcée : [occurrences.tsv](../raw/occurrences.tsv). Chaînes/commentaires inclus ; appels effectifs non résolus.

## Diagnostics Clippy de cette crate

Commande workspace/all-targets, code 0, aucun test exécuté. 0 messages warning concernant les sources de cette crate, répétitions entre cibles incluses : 0 sur plages hors tests et 0 sur plages tests. [Sortie brute](../raw/cargo-clippy.stdout.jsonl), [détail sourcé](../raw/clippy-diagnostics.tsv).
| Diagnostic | Plages hors tests | Plages tests |
| --- | --- | --- |
