Avancée : worker Parakeet CUDA0 L3a qualifié sur les recettes natives FR/EN et revu CLEAN.
Blocage : aucun pour L3a ; un échec intermittent historique de live_archive reste signalé.
Décision : cette fiche atteste le worker GPU L3a, sans admission dans le produit ; L3b requiert un accord distinct.

# Fiche L3a — worker Parakeet GPU

Référence : [plan L3 v5](plan-parakeet-l3-gpu-import.md), activation écrite du 18/10/2026 sur `d1fa9249`. Profil de qualification : import, français et anglais, GTX 1080 Ti unique, CUDA0. Les recettes emploient la copie figée `target/parakeet-tests/l3/l3a/worker/whisper-worker-parakeet-gpu.exe`. Le worker CPU R6, sa clé Test et Turbo ne sont pas modifiés. Cette fiche ne rend pas le GPU sélectionnable dans l'application : cette admission appartient à L3b.

## Identité figée

| Ressource | SHA-256 |
| --- | --- |
| Worker GPU release `whisper-worker-parakeet-gpu.exe` | `5054612a05fbfd1039e108def3aa4e598268d2b4274b8c1dc96cd6b8e3f2cd6d` |
| Modèle `models/parakeet-tdt-0.6b-v3.q8_0.gguf` | `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e` |
| `cublas64_12.dll` | `52ce5ba0ec5327d39be6021f0f9362d2ed4d64d7206f9b5afc06398d827b8e82` |
| `cublasLt64_12.dll` | `71705a7cf0923f4ab034d0d2620bbfc989ff669ac3d79f868b7a7c597ca559c5` |
| `cudart64_12.dll` | `760c38928bbe5759f7b31ed6692599eb7ec83cedd5702e84c2b72028a89837e1` |
| `ggml-base.dll` | `24b7342f10bc14e17dcb55ebafdbaa1a0fd0187cf5ae1a8ecf03bb9f8a17ef93` |
| `ggml-cpu.dll` | `23166143accf20055ccaafb8b720ad3dac33bf16ac027cc77225e0bb2a99e699` |
| `ggml-cuda.dll` | `7e244413387852825ed420fceef5e3b318f3a264f97f59fe0233789dfbdb6822` |
| `ggml.dll` | `9148ba2ed0eaeee385d09eba37e1a297d5e2f0649a02ffd4e325fea562c29ea6` |
| `nemo_speech_asr.dll` | `4fac537a8d99a8cee4cd1dba2391379cb895fafb7d21beb9d6f4fe670be05449` |
| `nemo_speech_asr_c.dll` | `6643c1c657f05bd545655044ec59fc052a789042ce9c17496b9c77a240c7bbb9` |

Les neuf DLL proviennent de `target/parakeet-feasibility/spike02/runtime space/cuda/bin/`. Nom, taille, SHA et imports ont été contrôlés avant copie, puis les copies ont été revérifiées. Journaux : `target/parakeet-tests/l3/l3a/logs/cuda-source-preflight.json` et `cuda-copy-verified.json`. Le worker CPU R6 conserve le SHA `1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0`.

## A2 — quatre preuves obligatoires

| Preuve | Résultat |
| --- | --- |
| Journal natif dans le worker | `[asr] ... backend=CUDA0`, PID `94404`, empreinte du journal `635fe8725e59d0d1bb392a80d8f7016d852223e2ae1226661ca29a52b6264f5b` pour EN5. |
| Carte physique unique | Inventaire Windows : `NVIDIA GeForce GTX 1080 Ti`, bus `00000000:01:00.0`, `PCIROOT(0)#PCI(0100)#PCI(0000)` ; `nvidia-smi --query-gpu=count,name,pci.bus_id,uuid --format=csv` : une carte, même bus, UUID `GPU-99e636f6-45ac-0068-a378-7d7a8bca645e`. Préflight avant démarrage du worker. |
| Temps natif EN5 | `6,396 s`, sous le plafond `11,1 s` (50 % de la référence CPU R6 `22,134 s`). |
| Activité pendant l'appel | 45 échantillons `nvidia-smi` dans la fenêtre native, utilisation maximale `100 %`, VRAM globale maximale `6 758 MiB` contre `3 725 MiB` avant l'appel. |

**A2 PASS.** La VRAM est globale sous WDDM ; son pic ne peut pas être attribué précisément au worker. Le journal complet est sous `target/parakeet-tests/l3/l3a/recipes/gpu-single-pass/gpu-single-pass.json`.

## A3 — recettes sur la même copie figée

| Audio | Échantillons à 16 kHz | Mots | Inférence native | Requête worker | Résultat |
| --- | ---: | ---: | ---: | ---: | --- |
| EN5 réel (`spkr0.wav`) | 4 800 000 | 967 | 6,396 s | 6,578 s | PASS |
| FR87 | 1 402 400 | 176 | 0,628 s | 0,713 s | PASS |
| FR27 | 432 000 | 58 | 0,145 s | 0,194 s | PASS |
| EN15 | 242 416 | 38 | 0,090 s | 0,134 s | PASS |
| EN30 synthétique | 28 800 000 | 5 908 | 48,893 s | 49,720 s | PASS |

Chaque transcription a du texte non vide, des mots et horodatages valides, sans `InvalidTimestamp`. Vingt appels FR/EN consécutifs dans un seul worker : **PASS**, sortie du processus `0` ; mauvais périphérique : **refus avant ouverture audio et avant `NativeCallStarted`**, PASS. Journaux sous `target/parakeet-tests/l3/l3a/recipes/gpu-twenty-calls/` et `gpu-wrong-device/`. Le démarrage du worker jusqu'à `Ready` prend `1,157 s` ; démarrage + requête EN5 prennent `7,735 s`, contre `38,6 s` de référence totale CPU R6. Sur 224 prélèvements, le pic Windows `WorkingSet64` est `679 591 936` octets et le pic `PrivateMemorySize64` est `11 037 626 368` octets. Ces compteurs mémoire ont des sens différents et ne doivent pas être additionnés. **A3 PASS.**

## Limites de la fiche

La qualification porte sur le worker et le harness natif, pas sur l'import GPU dans l'application. L3b doit encore qualifier admission de clé, Auto/manuel, archive, TXT/SRT et interface. Le corpus FR long réel et la vérité terrain restent absents ; EN30 est synthétique. L4 micro, VAD adaptative et B8 restent en backlog non planifié. La validation dans la fenêtre Windows reste réservée à l'utilisateur.
