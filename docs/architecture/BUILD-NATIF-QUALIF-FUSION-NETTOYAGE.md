Avancée : fusion 04be2ee6 publiée, N1 validé et nettoyage humain vérifié.
Blocage : aucun pour cette clôture.
Décision : conserver le cache natif, les preuves et l’ancien worktree build-natif.

# Fusion et nettoyage BUILD-NATIF-QUALIF — 7 novembre 2026

Bilan figé après vérification du nettoyage le 7 novembre 2026. Les bilans de qualification précédents sont conservés.

## Fusion vérifiée

`git pull --ff-only` à jour sur `946765cf`. La branche qualifiée contenait 26 commits et 19 fichiers modifiés. L’unique chemin hors ci/native/docs est le test N2 GPU français, autorisé dans la session puis explicitement accepté pour la fusion. Les gardes niveau 1/native refusent la racine principale, avec 37 témoins positifs/négatifs PASS. Fusion `--no-ff` publiée sous `04be2ee6`, branche distante revérifiée. Le delta Rust est limité au test N2 GPU français.

Le commit fusionné a été testé depuis le worktree dédié, en HEAD détachée, via la garde N1, `--offline --locked`, sans variable CARGO_TARGET_DIR et avec cible effective vérifiée. Auto-tests N1 58 positifs / 85 négatifs. Fixtures PE 4 positives / 15 négatives, témoin réel facultatif non exécuté. Les dix harnais des workers ont passé le contrôle PE avant exécution.

| Sélection | Réussis | Échecs | Ignorés |
|---|---:|---:|---:|
| cargo test --no-fail-fast, membres par défaut | 774 | 0 | 129 |
| whisper-worker-cpu | 12 | 0 | 9 |
| whisper-worker-parakeet-cpu | 26 | 0 | 11 |
| whisper-worker-common | 11 | 0 | 1 |

Le filtre proposé rencontre le nom du test réussi `panicked_reader_poison_is_explicit_and_another_key_still_progresses ... ok`. Push autorisé explicitement après présentation de ce résultat. Les codes de sortie des quatre sélections sont zéro. Le backend CPU natif enregistré dans la qualification utilise `/arch:AVX2`, `GGML_AVX2;GGML_FMA;GGML_F16C`, avec `GGML_NATIVE=ON`. `GGML_AVX_VNNI=OFF`, absent des options effectives.

## Préservation et nettoyage exécuté

Les preuves CPU r2 et GPU r5 ont été revérifiées, 26 reçus et 599 fichiers. Les 14 DLL installées correspondent aux empreintes initiales. Les 36 journaux N1 de cette fusion sont archivés et leurs copies vérifiées.

Le cache natif a été déplacé vers `C:/dev/whisper/native/cache`, avec 18 664 fichiers, 293 593 263 octets et les trois commits source épinglés vérifiés. Le contrôle automatique a ensuite refusé toute la commande de suppression, avec le seul motif « blocked by policy ». L’utilisateur a exécuté le script remis. Son journal atteste le retrait non récursif de 63 jonctions et la suppression des trois racines autorisées. Leur absence a été vérifiée, puis la référence worktree a été élaguée et la branche fusionnée build-natif-qualif supprimée.

| Racine autorisée au nettoyage | Taille apparente avant préparation | État |
|---|---:|---|
| C:/wb/native/cpu | 9,51 Gio | Supprimée, absence vérifiée |
| C:/wb/native/gpu | 9,65 Gio | Supprimée, absence vérifiée |
| C:/wt/build-natif-qualif | 115,38 Gio, cache de 0,27 Gio déplacé depuis | Supprimée, absence vérifiée |
| C:/wb/native-qualif | 7,50 Gio | Preuves conservées |

L’espace libre a augmenté de **121,47 Gio**, de 324 245 925 888 à 454 673 461 248 octets, soit **423,45 Gio libres** au terme de la suppression. Les tailles apparentes avant nettoyage totalisaient environ 134 Gio après déduction du cache déplacé. L’ancien worktree `C:/wt/build-natif` et sa branche sont conservés, avec 52 commits absents de main, dont le bilan C1001 et son archive ZIP. Le dossier préexistant non suivi `C:/dev/whisper/deliverables` est préservé.

[Résultats N1](C:/wb/native-qualif/fusion-nettoyage/n1/results.json) · [Archives vérifiées](C:/wb/native-qualif/fusion-nettoyage/qualification-archives-check.json) · [Inventaire](C:/wb/native-qualif/fusion-nettoyage/cleanup-before.json) · [Cache préservé](C:/wb/native-qualif/fusion-nettoyage/cache-preserved.json) · [Suppression humaine](C:/wb/native-qualif/fusion-nettoyage/cleanup-result.json) · [Vérification finale](C:/wb/native-qualif/fusion-nettoyage/final-check.json). Les 599 fichiers de qualification, les 36 journaux N1, les 14 DLL installées et les 18 664 fichiers du cache ont été revérifiés après nettoyage.
