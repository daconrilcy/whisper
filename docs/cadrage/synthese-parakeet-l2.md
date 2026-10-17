Avancée : la branche `wip/l2b-candidate` contient le candidat Parakeet Test CPU, R7/R8 et R9 ; ce document prépare sa fusion sans la réaliser.
Blocage : B8 reste inactive ; les longues découpes, le français long réel, la qualité absolue, le GPU Parakeet et le micro Parakeet ne sont pas qualifiés.
Décision : conserver le périmètre Test CPU et demander une décision distincte avant fusion, push ou nouveau lot.

# Synthèse Parakeet L2 — 17 octobre 2026

## Portée de la comparaison

Comparaison Git `evo-startup-perf...wip/l2b-candidate` au 17 octobre 2026, avant le présent commit documentaire. Base commune `8a4385e9` ; `wip/l2b-candidate` a 31 commits propres, `evo-startup-perf` en a deux. Les travaux L2 antérieurs à cette base sont communs aux deux branches et n'apparaissent donc pas dans le diff ci-dessous. Les binaires, modèles et WAV sous `target/`, `models/` et `sound example/` restent hors Git et hors de cette liste.

## Commits propres à `wip/l2b-candidate`

| Hash | Titre | Lot |
| --- | --- | --- |
| `4c835531` | docs(parakeet): plan L2b R4 admission correction | R4 plan |
| `56dd0dc9` | docs(parakeet): record R4 plan review CLEAN | R4 plan |
| `f5c8394f` | WIP L2b livré avec réserves, R4 CLEAN, B8 inactive | L2b/R4 |
| `595466fc` | docs(parakeet): draft L2b R5 correction plan | R5 plan |
| `0b41d984` | docs(parakeet): split R5 correction and admission gates | R5 plan |
| `76e00f44` | feat(parakeet): normalize R5-1 words and validate IPC | R5-1 |
| `2c110d82` | docs(parakeet): draft R5-2 admission plan and root gate | R5-2 plan |
| `63ca0f07` | Plan R5-2 v2: ancre de lecture et admission en recette | R5-2 plan |
| `467f3bce` | Qualifie Parakeet Test court et confine la recette VAD R5-2 | R5-2 |
| `2470d297` | Documenter diagnostics B5 B6 et proposition VAD adaptative | B5/B6 diagnostic |
| `4cdc6944` | Préparer le plan R6 pour revue indépendante | R6 plan |
| `40920b49` | Consigner le verdict NOT READY du plan R6 | R6 plan |
| `922c6592` | Réviser le plan R6 selon arbitrages et recettes | R6 plan |
| `8a5b49b0` | Acter le verdict CLEAN du plan R6 | R6 plan |
| `c8c38021` | R6 livré avec réserves : R4, B9 et contrat de clé L2a-2 ; B8 inactive | R6 |
| `3ed45dd1` | Plan R7 : amendement B9 et contrat de clé revu CLEAN | R7 plan |
| `319a7737` | Plan R7 : racine de test archive R7 autorisée | R7 plan |
| `f5313734` | Archiver le bilan documentaire R7 | R7 bilan |
| `554f493e` | Documenter la regeneration des WAV synthetiques | Nettoyage |
| `ea4c2dd0` | Definir la regle permanente des campagnes Cargo | Nettoyage |
| `6899c011` | Documenter le nettoyage de target et les controles apres nettoyage | Nettoyage |
| `4fbef019` | Proposer la reprise du R8 historique sur la cle Test courante | R8 plan |
| `9061e3a5` | Documenter le bilan R8 et son amendement de gate | R8 bilan |
| `a28a8690` | Clore le bilan documentaire R8 apres revue CLEAN | R8 bilan |
| `30cdd25b` | R7 (livré avec réserves) + R8 reprise robustesse (CLEAN) | R7/R8 |
| `ff7ac796` | Préparer le plan R9 et consigner les gates de périmètre | R9 plan |
| `939603b6` | Intégrer les arbitrages R9 au plan v3 | R9 plan |
| `3aa6fdc5` | Préciser les recettes A4 et R8 du plan R9 | R9 plan |
| `77b9830a` | Documenter la réserve finale du plan R9 et corriger les recettes | R9 plan |
| `d64260b9` | Acter PLANS READY pour R9 | R9 plan |
| `d9b07caf` | R9 cache SHA adaptateur qualifié Test | R9 |

## Fichiers de code modifiés par crate

Les chiffres sont les lignes ajoutées/supprimées du diff Git à trois points, pas la taille totale des fichiers. Hors crates : `AGENTS.md` +7/−0 et `Cargo.lock` +1/−0. Les documents sous `docs/cadrage/` sont listés dans les commits, sans les confondre avec le code produit.

| Crate | Fichier | + | − |
| --- | --- | ---: | ---: |
| whisper-adapters | `Cargo.toml` | 1 | 0 |
| whisper-adapters | `src/archive.rs` | 483 | 15 |
| whisper-adapters | `src/model_catalog.rs` | 150 | 13 |
| whisper-adapters | `src/parakeet_import.rs` | 201 | 11 |
| whisper-adapters | `src/worker_ipc.rs` | 989 | 99 |
| whisper-adapters | `tests/model_catalog.rs` | 81 | 0 |
| whisper-adapters | `tests/parakeet_archive.rs` | 1 319 | 107 |
| whisper-adapters | `tests/parakeet_control.rs` | 202 | 5 |
| whisper-adapters | `tests/parakeet_import.rs` | 800 | 34 |
| whisper-adapters | `tests/parakeet_long.rs` | 667 | 0 |
| **whisper-adapters** | **10 fichiers** | **4 893** | **284** |
| whisper-core | `src/lib.rs` | 1 | 0 |
| whisper-core | `src/model_registry.rs` | 5 | 2 |
| whisper-core | `src/model_selection_app.rs` | 4 | 11 |
| whisper-core | `src/parakeet_ipc.rs` | 10 | 0 |
| whisper-core | `src/parakeet_windows.rs` | 333 | 0 |
| whisper-core | `tests/parakeet_import_contract.rs` | 10 | 6 |
| whisper-core | `tests/parakeet_windows.rs` | 194 | 0 |
| **whisper-core** | **7 fichiers** | **557** | **19** |
| whisper-worker-parakeet-cpu | `Cargo.toml` | 4 | 0 |
| whisper-worker-parakeet-cpu | `src/main.rs` | 338 | 16 |
| whisper-worker-parakeet-cpu | `src/native_engine.rs` | 476 | 34 |
| whisper-worker-parakeet-cpu | `src/native_ffi.rs` | 66 | 16 |
| whisper-worker-parakeet-cpu | `tests/native_import.rs` | 343 | 2 |
| **whisper-worker-parakeet-cpu** | **5 fichiers** | **1 227** | **68** |

`whisper-desktop` n'a aucun diff propre à cette branche : son interface de sélection Parakeet était déjà dans la base commune. Ce diff ne décrit donc pas à lui seul tout L2.

## Décisions produit et arbitrages depuis L2

Les décisions numérotées sont dans `evo-parakeet.md` : DEC-PARA-20 (faisabilité GO avec réserves FR/EN CPU/GTX 1080 Ti), 21 (passe unique ≤30 min, VAD 60–120 s au-delà dans le design de faisabilité ; format 20 min écarté), 22 (écart de texte sans vérité terrain informatif), 23 (recette intégration 60 min encore requise), 24 (matrice Turbo actuelle reconnue sans mesure supplémentaire), 25 (modèles Test locaux hors Git et sans acquisition en L1b), 26 (scission L1b-1/L1b-2), 27 (sauvegarde v1 avant écriture v2), 28 (rattachement des chemins Turbo historiques connus). DEC-PARA-19 reste la règle de langue : Auto Parakeet seulement dans la portée qualifiée, manuel non pris en charge, aucun identifiant de langue promis.

Les arbitrages utilisateur ultérieurs, consignés dans les plans et bilans L2, limitent le produit Test à **CPU, import en passe unique ≤30 min**, sur la clé R6 `1d8f57b9…`. Les anciennes clés sont refusées pour un nouvel import et leurs pending sont conservés sans reprise. Au-delà de 30 min, refus avant pending avec « Fichier trop long pour Parakeet, utilisez Turbo », sans bascule silencieuse. B8/import-vad reste inactive. La VAD à seuil RMS fixe est rejetée ; le principe d'une VAD adaptative est accepté pour un lot futur distinct. Turbo demeure le moteur du micro ; Parakeet live est reporté. R9 conserve la vérification SHA du worker et limite les lectures complètes du modèle par l'adaptateur à une par identité et session. Ces arbitrages ne qualifient ni la distribution finale ni le GPU Parakeet.

## Réserves ouvertes

- **B8 inactive** : aucun import long par VAD n'est offert dans le produit ; la VAD adaptative n'est qu'une note de conception.
- La VAD 60–120 s n'a été éprouvée que sur des fichiers d'environ 5 min avec deux coupures ; les 30–60 coupures d'un fichier de 60 min n'ont pas de test d'acceptation d'intégration concluant. Le français long du banc est un montage synthétique, sans enregistrement long réel.
- Aucune vérité terrain ne mesure la qualité absolue FR/EN ; les différences à la passe unique indiquent une sensibilité au contexte.
- GPU Parakeet, live/micro Parakeet, 23 autres langues et paquet final restent hors du périmètre qualifié. Le GPU et le micro existants de Turbo ne sont pas remis en cause.
- Le gate workspace R9 est vert avec **31 tests ignorés/NOT RUN** ; notamment B9 et des recettes natives longues. Deux nouveaux ignorés R9 sont décrits ci-dessous. Les recettes ciblées R9/A4/R8 ont été exécutées séparément et sont détaillées dans `bilan-parakeet-r9.md`.
- Le cache SHA adaptateur ne détecte pas une altération qui préserverait identifiant physique, taille et date ; le worker gelé revérifie son SHA à chaque démarrage avant inférence. Cela ne démontre pas un refus avant pending pour ce cas particulier.

## Les deux nouveaux NOT RUN du gate workspace

| Test ignoré | Pourquoi le gate ne le lance pas | Exécution ciblée |
| --- | --- | --- |
| `parakeet_archive::modified_parakeet_model_is_refused_before_pending_after_cache_revalidation` | `#[ignore]` : copie puis altère un GGUF réel sous la racine R9 ; dépend du modèle, de l'audio EN et du profil de recette. | `cargo test --offline --locked -p whisper-adapters --test parakeet_archive modified_parakeet_model_is_refused_before_pending_after_cache_revalidation -- --ignored --exact --nocapture` |
| `parakeet_import::r9_application_recipe` | `#[ignore]` : lance le worker natif réel ; nécessite un cas et un WAV sélectionnés, les DLL, le modèle et l'environnement R9. | Pour EN5 : `$env:PARAKEET_R9_CASE='en5'; $env:PARAKEET_R9_RECIPE_WAV='C:\dev\whisper\sound example\spkr0.wav'; $env:PARAKEET_R6_CASE='r9-en5'; cargo test --offline --locked -p whisper-adapters --test parakeet_import r9_application_recipe -- --ignored --exact --nocapture` |

Ces commandes sont à lancer **après** le préflight et la garde à échec fermé du [plan R9](plan-parakeet-r9-sha-cache.md) : Cargo partagé, racines R9, modèle/worker/DLL qualifiés, `PARAKEET_L2_EN_WAV`, `PARAKEET_L2_ACCEPTANCE_TEST_BUILD=1`, `TEMP/TMP` sous R9 et journal natif R6 exact. Elles ont été exécutées en recettes ciblées et ont passé pendant R9 ; leur statut reste NOT RUN dans la commande workspace ordinaire.

## Recette manuelle Windows — profil Test

1. Depuis `C:\dev\whisper`, vérifier les deux modèles locaux sous `models/`, le worker R6 gelé `target/parakeet-tests/l2/l2b/r6/worker/whisper-worker-parakeet-cpu.exe` et les cinq DLL sous `target/parakeet-tests/l2/l2b/native/`. Le modèle, le worker et les DLL doivent correspondre aux SHA de `fiche-parakeet-r6.md`. Une ressource absente ou différente rend Parakeet indisponible ; ne pas télécharger pour cette recette.
2. Dans la session PowerShell qui lancera l'application Test, définir `$env:PARAKEET_L2_RUNTIME_ROOT='C:\dev\whisper\target\parakeet-tests\l2\l2b'`, `$env:PARAKEET_L2_WORKER='C:\dev\whisper\target\parakeet-tests\l2\l2b\r6\worker\whisper-worker-parakeet-cpu.exe'`, `$env:PARAKEET_L2_NATIVE_DIR='C:\dev\whisper\target\parakeet-tests\l2\l2b\native'` et `$env:PARAKEET_L2_MODEL='C:\dev\whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'`. Lancer l'exécutable Test construit pour cette branche, par exemple `target/parakeet-tests/cargo-shared/debug/whisper-desktop.exe` si ce build est à jour. Définir le dossier des transcriptions dans un emplacement de test accessible.
3. Attendre la fin du chargement des modèles. Dans « Modèle local », sélectionner **Parakeet TDT 0.6B v3**. Vérifier qu'il est sélectionnable, que « Langue » offre **Automatique** et que le calcul affiché est **CPU (requis)**. Le micro ne doit pas être proposé pour Parakeet.
4. Importer `C:\dev\whisper\sound example\spkr0.wav` avec « Importer et transcrire ». Vérifier que l'opération se termine sans `InvalidTimestamp`, que le TXT et le SRT de même base sont créés, non vides, avec des horodatages SRT positifs et ordonnés. Le fichier fait 5 min ; la recette automatisée R9 a passé en 38,598 s de la demande aux sorties, mais ce temps n'est pas une cible de validation de l'interface.
5. Importer ensuite `C:\dev\whisper\target\parakeet-feasibility\spike02\fixtures\fr87.wav`. Vérifier TXT/SRT non vides, aucun caractère `�`, et la séquence « faire? Je » sans mot de ponctuation séparé ; vérifier que le mot suivant possède une durée positive. Un import d'un fichier de plus de 30 min doit afficher « Fichier trop long pour Parakeet, utilisez Turbo » avant toute création de pending ; ne pas attendre de repli automatique vers Turbo.

Réussite manuelle : Parakeet sélectionnable uniquement avec la clé Test attendue ; import FR/EN réussi en CPU, sorties TXT/SRT correctes ; Turbo toujours disponible et micro réservé à Turbo. Cette recette dans la fenêtre appartient à l'utilisateur et n'est pas couverte par les tests automatisés.
