Avancée : la clé GPU Parakeet exacte est qualifiée dans le profil Test pour l'import FR/EN jusqu'à 30 min.
Blocage : aucun pour l'admission Test ; la validation dans la fenêtre Windows reste à faire par l'utilisateur.
Décision : conserver le worker GPU L3a figé et réserver cette admission à la clé, au périphérique et aux DLL contrôlés.

# Fiche Parakeet GPU Test — L3b

Références : [plan L3 v5](plan-parakeet-l3-gpu-import.md), [fiche du worker L3a](fiche-parakeet-l3a-gpu.md), [bilan L3b](bilan-parakeet-l3-gpu-import.md). Qualification du 18/10/2026 sur `evo-startup-perf`, profil `import-single-pass`, CPU et GPU manuels et mode Auto. Cette fiche ne qualifie ni le microphone Parakeet ni la découpe VAD.

## Clé et environnement admis

| Élément | Valeur qualifiée |
| --- | --- |
| Modèle | `parakeet-tdt-0.6b-v3.q8_0.gguf`, SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e` |
| Worker release GPU | `target/parakeet-tests/l3/l3a/worker/whisper-worker-parakeet-gpu.exe`, SHA-256 `5054612a05fbfd1039e108def3aa4e598268d2b4274b8c1dc96cd6b8e3f2cd6d` |
| Runtime NeMo | `nemo-speech-asr 0.2.0`, build GPU `cbf7543a711a341df4387fb9047fcd55d3d5c572465cbe9bbb7d14b5963e6b10`, protocole worker `2` |
| Backend et profil | `CUDA0`, `import-single-pass`, FR/EN, langue manuelle ou Auto, durée ≤ 30 min |
| Carte | NVIDIA GeForce GTX 1080 Ti unique, PCI bus `00000000:01:00.0`, PNP `PCIROOT(0)#PCI(0100)#PCI(0000)`, UUID `GPU-99e636f6-45ac-0068-a378-7d7a8bca645e` |

Les neuf DLL CUDA chargées depuis la copie sous `target/parakeet-tests/l3/l3b/native/` ont exactement les empreintes consignées dans la [fiche L3a](fiche-parakeet-l3a-gpu.md#identité-figée). La garde a recontrôlé la copie figée du worker et les DLL avant les recettes. Le modèle, le worker CPU R6 et ses cinq DLL ont gardé les mêmes SHA avant et après chaque campagne finale.

L'admission requiert la concordance de l'identité du modèle, du worker, des neuf DLL et de la carte physique. Une clé, une DLL, un worker ou un périphérique divergent interdit GPU manuel avant `pending` avec cause visible. En Auto, la préférence reste Auto : GPU qualifié d'abord ; au plus une tentative CPU **du même modèle** avant `NativeCallStarted` si le GPU est indisponible. Après ce marqueur, ou si le worker meurt sans marqueur, aucun repli n'est permis. Il n'y a jamais de repli vers Turbo. Une reprise dont la clé backend diffère est refusée et ses données sont conservées.

## Recettes d'admission

| Carte | Preuve | Statut |
| --- | --- | --- |
| B1 | Clé exacte inscrite dans `model_registry.rs` après les recettes sur la clé candidate ; injection réservée au test retirée du build final. | PASS |
| B2 | GPU manuel sans repli, Auto GPU puis CPU Parakeet pré-natif au plus une fois, refus après appel natif, refus de reprise avec une autre clé. | PASS |
| B3 | GPU grisé lorsque la clé physique ou les fichiers diffèrent ; Turbo, CPU Parakeet et microphone Turbo préservés. | PASS |
| B4 | FR87, FR27, EN15, EN5 réel et EN30 synthétique sur le worker figé ; TXT/SRT publiés, zéro `InvalidTimestamp` ; >30 min refusé avant `pending`. Cinq scénarios de robustesse R8 rejoués sur CPU et GPU. | PASS |
| B5 | Formatage, build CPU et suite workspace complète sous garde : 377 résultats `ok` dont deux recettes sans sélecteur non exécutées, zéro échec, 34 tests ignorés. Revue indépendante CLEAN après deux passes. | PASS |

Les recettes applicatives finales ont duré `185,29 s` pour le groupe FR87/FR27/EN15, `132,34 s` pour EN5 et `220,80 s` pour EN30, selon les temps des tests Cargo qui incluent leur préparation. Ce ne sont pas les temps d'inférence pure du worker : ceux-ci figurent dans la [fiche L3a](fiche-parakeet-l3a-gpu.md#a3--recettes-sur-la-même-copie-figée). EN5 respecte le délai de recette de `195 s`. L'objectif de `≤ 15 s` de bout en bout n'est pas démontré par cette campagne. Les sorties et journaux sont sous `target/parakeet-tests/l3/l3b/archive/recipes/` et `target/parakeet-tests/l3/l3b/logs/`.

## Limites

La qualification couvre le profil Test sur cet hôte et ce jeu exact d'artefacts. Le FR long réel et la vérité terrain restent absents ; EN30 est synthétique. La VRAM mesurée sous WDDM reste globale et non attribuée exclusivement au worker. L'installation personnelle L6-perso, le microphone Parakeet, la VAD adaptative et B8 ne sont pas qualifiés ici. L'essai dans la fenêtre Windows relève de l'utilisateur.
