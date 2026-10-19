Avancée : les workers Parakeet release CPU et CUDA0 sont admis pour l'import personnel et les imports installés EN5 passent.
Blocage : la restauration entre générations distinctes et la recette dans la fenêtre Windows restent NOT RUN.
Décision : conserver ces réserves ; aucune capacité live, VAD ou langue supplémentaire n'est admise par cette fiche.

# Fiche L6-perso — installation personnelle Parakeet CPU et CUDA0

Date : 19/10/2026. Référence : [PLAN-PARA-L6-PERSO v4](plan-parakeet-l6-perso.md), autorisation écrite de l'utilisateur dans la session, revue Rust indépendante **CLEAN** après deux cycles de correction. Profil `Perso`, import WAV/MP3 FR/EN en passe unique jusqu'à 30 min. Racine publiée : `%LOCALAPPDATA%\Whisper\`. Le profil Test et ses clés R6/L3a restent distincts.

## Identités exactes publiées

| Ressource | SHA-256 |
| --- | --- |
| Desktop release `app/whisper-desktop.exe`, feature `personal-install` sans injection de qualification | `1bc14d3b0fcefba547fb89e9efbe735d73afce42a3cb47f47364d6347e43cae0` |
| Worker Turbo CPU `app/whisper-worker-cpu.exe` | `adb23fbb2f6d8c6476144aa4de811a36d2cf2753ffebb40381c6eb6eaceb45da` |
| Worker Turbo GPU `app/whisper-worker-gpu.exe` | `5ea3c673f3a19fff019c7fcca30509b57a1d835aafece7780d67d650a06dcf61` |
| Worker Parakeet CPU `worker/whisper-worker-parakeet-cpu.exe` | `84c371d4df55043ada4a1d96b80180ed0c03b208410c599262c8ea3762304e60` |
| Worker Parakeet CUDA0 `worker/whisper-worker-parakeet-gpu.exe` | `bf58eb9d7300c444864347496b5a27e8584da3d30d634a759be99a598eda5ce4` |
| Modèle Turbo `models/ggml-large-v3-turbo.bin` | `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` |
| Modèle Parakeet `models/parakeet-tdt-0.6b-v3.q8_0.gguf` | `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e` |

| DLL CPU dans `native/cpu/` | SHA-256 |
| --- | --- |
| `ggml-base.dll` | `398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a` |
| `ggml-cpu.dll` | `f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0` |
| `ggml.dll` | `2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794` |
| `nemo_speech_asr.dll` | `923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019` |
| `nemo_speech_asr_c.dll` | `9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764` |

| DLL CUDA0 dans `native/gpu/` | SHA-256 |
| --- | --- |
| `cublas64_12.dll` | `52ce5ba0ec5327d39be6021f0f9362d2ed4d64d7206f9b5afc06398d827b8e82` |
| `cublasLt64_12.dll` | `71705a7cf0923f4ab034d0d2620bbfc989ff669ac3d79f868b7a7c597ca559c5` |
| `cudart64_12.dll` | `760c38928bbe5759f7b31ed6692599eb7ec83cedd5702e84c2b72028a89837e1` |
| `ggml-base.dll` | `24b7342f10bc14e17dcb55ebafdbaa1a0fd0187cf5ae1a8ecf03bb9f8a17ef93` |
| `ggml-cpu.dll` | `23166143accf20055ccaafb8b720ad3dac33bf16ac027cc77225e0bb2a99e699` |
| `ggml-cuda.dll` | `7e244413387852825ed420fceef5e3b318f3a264f97f59fe0233789dfbdb6822` |
| `ggml.dll` | `9148ba2ed0eaeee385d09eba37e1a297d5e2f0649a02ffd4e325fea562c29ea6` |
| `nemo_speech_asr.dll` | `4fac537a8d99a8cee4cd1dba2391379cb895fafb7d21beb9d6f4fe670be05449` |
| `nemo_speech_asr_c.dll` | `6643c1c657f05bd545655044ec59fc052a789042ce9c17496b9c77a240c7bbb9` |

La clé agrégée de runtime native CPU vaut `d5a2cc8ef441dd5934972adae00f997241bea17a7bc5977a8c7f0122aaa958ff` ; la clé CUDA0 vaut `cbf7543a711a341df4387fb9047fcd55d3d5c572465cbe9bbb7d14b5963e6b10`. Les clés du registre associent chacune son worker exact, le modèle, ces DLL, le backend, le profil et, pour CUDA0, le périphérique qualifié. Une divergence refuse l'import avant pending ; la reprise d'une ancienne clé conserve les données mais est refusée.

## Recettes attribuées aux mêmes copies figées

| Carte | CPU | CUDA0 |
| --- | --- | --- |
| A4 FR87, FR27, EN15, TXT/SRT et temps valides | PASS | PASS |
| EN5 et EN30 applicatifs | PASS ; EN30 173,451 s | PASS ; EN30 79,568 s |
| Cinq scénarios R8 (DLL incompatible, worker tué, trame incomplète et deux pannes par route produit) | 5/5 PASS | 5/5 PASS |
| Refus >30 min avant pending ; aucun repli Turbo | PASS | PASS |
| Import EN5 depuis la racine publiée, TXT/SRT hors de celle-ci | PASS ; harness 80,97 s | PASS ; harness 44,38 s |

CUDA0 satisfait les quatre preuves A2 : journal natif `backend=CUDA0` avec PID, une seule GTX 1080 Ti cohérente entre Windows et `nvidia-smi` (bus `00000000:01:00.0`, UUID `GPU-99e636f6-45ac-0068-a378-7d7a8bca645e`), inférence native EN5 **6,348 s** contre le plafond 11,1 s, et activité GPU 22–100 % avec VRAM globale environ 3 563→6 551 Mio. Trois sessions release neuves ont donné **11,777 / 11,848 / 11,890 s** au premier import EN5 et **11,216 / 11,179 / 11,247 s** au second, sans seconde lecture complète adaptateur du modèle/worker/DLL inchangés. Le worker redémarre à chaque import. GPU manuel sans repli et Auto GPU puis CPU Parakeet pré-audio seulement ont passé ; le micro reste Turbo.

Le gate workspace complet avec variables et celui sans variables ont chacun couvert 52 cibles, exit 0 et zéro FAIL sous garde. Sans variables : 34 tests explicitement ignorés et 39 lignes `NOT RUN` par entrée de campagne absente dans le dernier journal ; libtest peut néanmoins afficher `ok` pour ces retours anticipés. Le détail nominatif est dans `target/parakeet-tests/l6-perso/logs/WorkspaceNoCampaign.stdout.log`. La garde a passé 25/25 auto-tests après son adaptation aux opérations publiées. Les SHA protégés 27/27 sont restés identiques lors du rollback final.

## Installation et limites

La publication locale a réussi ; les imports installés CPU et CUDA0 lisent les ressources sous `%LOCALAPPDATA%\Whisper`, sans worker ni modèle du dépôt. `settings/settings.json` et `settings/settings.json.bak` ont le même SHA initial `9d7ab67a7ca49aa8ac3d9075898f01ba3fd9841fe74aa77571e991e56d047bb2`; les réglages historiques à la racine ont été laissés intacts. Le backup de la génération B est complet (26/26 fichiers). Un rollback réel C→C a rendu exit 0, garde exit 0 et snapshot actif conforme (23/23). **La restauration inter-génération B→C est NOT RUN** : ses deux tentatives ont été arrêtées par la garde avant modification active. Un dossier transactionnel `.backup-…pending` incomplet est conservé pour diagnostic. Aucune recette visuelle de cette installation n'a été exécutée par l'assistant.

Inventaire PE relu avec `dumpbin /dependents` sur les cinq EXE et les 14 DLL installés. La chaîne Parakeet CPU/GPU résout les DLL privées dans `native/cpu` ou `native/gpu`, avec runtime VC et pilote NVIDIA du système. Les deux workers Turbo importent directement `cublas64_12.dll` et `cudart64_12.dll` depuis le CUDA Toolkit installé sur ce PC ; ces deux DLL ne sont pas copiées dans `app/`. Cette dépendance externe est **hors du dépôt**, mais la portabilité de Turbo sur un PC sans CUDA Toolkit n'est pas démontrée par L6-perso. L'installation personnelle reste bornée à cet ordinateur.

Les preuves d'exécution brutes restent sous `target/parakeet-tests/l6-perso/logs/` et les sorties de smoke sous `target/parakeet-tests/l6-perso/outputs/final-smoke/`. La [recette personnelle](recette-parakeet-l6-perso.md) décrit l'ouverture de l'application et le retour arrière. Cette fiche ne constitue ni un installateur distribué, ni une qualification du micro Parakeet, de la VAD, des 23 autres langues ou d'une qualité absolue sans vérité terrain.
