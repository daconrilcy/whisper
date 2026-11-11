Avancée : N1 complet PASS (944 réussis, 0 échec, 129 ignorés) ; revue indépendante CLEAN.
Blocage : aucun ; la mesure N2 est fondée sur les tailles épinglées du profil, aucun run N2 n’a été lancé.
Décision : conserver les runs échoués et le dernier run réussi par profil ; commandes de nettoyage proposées, non exécutées.

# EMPREINTE-DISQUE-TESTS v2 — bilan figé

## N1 : mesures et résidus

Commande gardée : `cargo test --no-fail-fast`, sans variable `CARGO_TARGET_DIR`, cible fraîche interne au worktree. **944/0/129** sur 59 cibles ; `CARGO_TARGET_DIR_child=False`. `guard-level1.ps1 -SelfTest` : **64 positifs / 99 négatifs**. Le premier essai N1, sans `-PersonalParakeetModelPath`, a échoué sur le prérequis modèle ; relancé avec le modèle vérifié, il a passé.

Tailles immédiates et de deuxième niveau sous `target`, `debug`/`release` exclus ; Go arrondi à l’unité (détail décimal entre parenthèses) :

| Mesure | Avant → après N1 initial | Après correction |
|---|---:|---:|
| Libre sur C: | 612 610 445 312 → 612 341 235 712 o | 612 331 905 024 → 612 047 876 096 o |
| `target/cargo-home-shared` | ≈1 Go (0,65) | ≈1 Go (0,65) |
| `target/parakeet-tests` | ≈3 Go (3,30) | ≈3 Go (3,34) |
| 2e niveau : `cargo-shared` / `garde-n1-categories` | ≈0 / 3 Go | ≈0 / 3 Go |

Résidus Rust après N1 initial : **72 528 725 o (0,073 Go)** sous `target/parakeet-tests/garde-n1-categories/temp/tmp`, dont 46 495 669 o sous `whisper-adapter-tests` ; sous le seuil d’1 Go. Après le N1 de validation, le cumul est 109 011 153 o. Les six copies du modèle faites par `prepare_fake_personal_worker` valent 713 975 456 o chacune (≈4,28 Go copiés au total), mais aucun fichier de cette taille ne reste après succès ; douze fixtures de modèle synthétiques de 13–15 o subsistent. Le chemin heureux nettoyait déjà ces copies. La correction protège maintenant les diagnostics pendant un unwind ; aucun gain disque additionnel mesurable sur N1 réussi.

Restes de garde : `target/parakeet-tests/garde-n1-categories/scratch`, **9 runs / 682 774 o** au total : deux préflights refusés (0 o chacun), fetch (14 589 o), N1 incomplet sans modèle (190 029 o), N1 réussis (173 694 et 174 144 o), SelfTest (129 804, 514 et 0 o). Le run `20261111T113906309-81446c8915fc4da7a662841e9d773b97` contient `junction-fixture` ; garde et dossier restent inchangés. Règle proposée : garder les échecs/inconnus et le dernier succès par profil/opération.

Espace pendant le SelfTest de la garde : 612 051 668 992 → 612 051 595 264 o.

Caches laissés en place : Cargo home 651 892 849 o ; cible N1 3 229 888 552 o ; cible partagée 1 391 o.

## N2 : mesure du profil et rétention proposée

Le profil `PersonalUnified` épingle 3 196 035 499 o de runtime (21 fichiers) et deux MP3 de 24 237 o chacun : **3 196 083 973 o par cas** ; quatre cas représentent jusqu’à **12 784 335 892 o copiés**. Il s’agit de la mesure des tailles épinglées, pas d’un run N2 exécuté ; aucun `target/n2-native` n’existe dans ce worktree. Proposition : conserver tous les runs en échec/inconnus et le dernier réussi par profil ; ne retirer les succès antérieurs qu’après vérification des exports et journaux requis. `Copy-ProfileStaging` et les gardes restent inchangés.

## Revue et commandes proposées

Revue/correction Rust : **CLEAN**, 2 lectures, 1 correction ; R1 fermé après le N1 gardé. `git diff --check` passe ; `rustfmt --check` signale des différences préexistantes sur des lignes non modifiées. Les commandes ci-dessous sont proposées seulement ; aucune suppression n’a été faite. Les chemins furent vérifiés sans reparse point ; conserver les caches partagés.

```powershell
Remove-Item -LiteralPath 'C:\wt\empreinte-v2\target\parakeet-tests\garde-n1-categories\temp\tmp' -Recurse -Force
Remove-Item -LiteralPath 'C:\wt\empreinte-v2\target\parakeet-tests\garde-n1-categories\scratch\20261111T112418495-28fdf1e7a2ea4061aa5c90d25929ff59' -Recurse -Force
Remove-Item -LiteralPath 'C:\wt\empreinte-v2\target\parakeet-tests\garde-n1-categories\targets\mesure-v2-n1' -Recurse -Force
```
