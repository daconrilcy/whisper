Avancée : proposition de protection préparée ; N1 et structure passent sur main et build-natif.
Blocage : DLL non qualifiées ; aucune fusion exécutée.
Décision : recommander la fusion de protection seule, après décision utilisateur ; poursuivre C1001 sans attendre.

# Proposition de protection BUILD-NATIF-C1001

Le commit proposé est `1d9e7d60d7babff944e15f6b33d3d6a303755b24`, branche `codex/build-natif-protection-c1001-v2`, parent main `5d3f7629435ab5661accd36eb2e4e958b6dee6c3`. Il protège la chaîne, sans modifier les DLL installées ni qualifier celles reconstruites. Aucun changement Rust, Cargo ou version d'outil. NeMo et les 19 correctifs sont conservés dans le dépôt NVIDIA épinglé, pas vendus comme sources locales. Le premier paragraphe de NATIVE-BUILD interdit de supprimer le spike et annonce les DLL NON QUALIFIÉES.

## Vérifications exécutées le 5 novembre 2026

| Sélection | PASS main / candidat | Ignorés chaque côté | Échecs | Identités listées |
|---|---:|---:|---:|---:|
| Défaut | 712 / 712 | 128 | 0 | 840 identiques |
| worker-cpu | 12 / 12 | 9 | 0 | 21 identiques |
| worker-parakeet-cpu | 26 / 26 | 11 | 0 | 37 identiques |
| worker-common | 11 / 11 | 1 | 0 | 12 identiques |
| Total des sélections | 761 / 761 | 149 | 0 | 910 identiques |

Les tests ignorés ne sont pas qualifiés. Avant les corps worker : vingt contrôles PE, tous code 0. Le journal conserve chacun des appels ; les captures stdout utilisent un nom par sélection et ne conservent donc que le dernier contrôle de chaque sélection. Garde N1 SelfTest : 35 positifs / 79 négatifs PASS sur chaque branche. Structure : 958 tests enregistrés / 958 présents, dont 166 natifs ; 7 helpers, PASS. Aucune relance de corps de test nécessaire. MemWatch rapproché : 58 mesures, maximum 20,6 %, niveau OK, WHEA 0.

La première tentative Structure avait arrêté la recette : sept tests autonomes existants manquaient au registre (951 contre 958). Après accord précis, seul `ci/tests-baseline.txt` a changé sur main, commit `5d3f7629`, puis même patch reporté par `5b6e464e` sur build-natif. Cette correction de données n'est pas une fusion de protection et ne démontre pas une instabilité des tests. La reprise complète ci-dessus utilise le registre corrigé ; les premières preuves sont conservées.

## Fichiers inclus (18)

- `AGENTS.md`
- `ci/build-native.ps1`
- `ci/guard-native.ps1`
- `ci/native-build-control.cs`
- `ci/native-directory-watch.cs`
- `ci/native-profiles.json`
- `ci/native-workspace.ps1`
- `ci/test-native-build-control.cs`
- `ci/test-native-build-control.ps1`
- `ci/test-native-build-flags.ps1`
- `ci/test-native-directory-watch.ps1`
- `ci/test-native-workspace.ps1`
- `docs/architecture/NATIVE-BUILD.md`
- `docs/architecture/raw/native-build/installed-inventory.json`
- `native/.gitignore`
- `native/build-config.json`
- `native/patches/ggml-pinned-build-commit.patch`
- `native/runtime-pins/reference.json`

Le diff apporte pilote, pins, contrôles de flags/commit, collecteur FileId et tests, conservation AGENTS et procédure minimale. Les profils/pin de référence sont inclus car la garde les exige. Le présent plan C1001 et ses preuves de campagne sont exclus du commit proposé.

## Fichiers exclus

Liste exhaustive des différences historiques exclues : champ `excluded` de `target/build-natif-c1001/protection-proposal-v2.json`, à conserver dans l'archive finale. Elle comprend les cinq anciens bilans architecture, les archives et extractions sous `docs/architecture/raw/` autres que l'inventaire installé, les vingt documents historiques de cadrage BUILD-NATIF et les plans. Aucun de ces fichiers n'est supprimé de build-natif. L'arbre proposé permet de contrôler directement la liste des dix-huit inclus indépendamment de cette liste historique.

Preuves détaillées : `protection-n1-parity.json`, `protection-pe-gates.json`, `protection-memory.json`, journaux `protection-run2/` et sorties brutes référencées, sous `target/build-natif-c1001/`. La recette run2 ne diffère de la recette relue que par son tag MemWatch et sa racine de captures.

Commande proposée, **non exécutée** :

```powershell
git -C C:/dev/whisper merge --no-ff codex/build-natif-protection-c1001-v2
```

Les résultats N1 concernent main et le candidat build-natif ; l'arbre de protection n'a pas été checkouté pour une troisième campagne. Les sources Rust/Cargo sont identiques et le registre de tests est le même. Aucune preuve de répétabilité native, de N2 ni de GPU n'est déduite de ces contrôles.
