Avancée : modèle, deux workers et jeux de DLL Parakeet historiques retrouvés et identifiés.
Blocage : compatibilité sur les trois témoins, provenance exacte des DLL et fermeture de leurs licences restent à établir.
Décision : étudier d’abord la réutilisation des ressources existantes ; si une reconstruction portable est nécessaire, soumettre un mandat spécifique avant reprise native.

## Ressources retrouvées le 19 novembre 2026

Le modèle `C:/dev/whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf` mesure 713975456 octets et son SHA-256 est `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`. Il correspond au GGUF officiel NVIDIA de [révision 541d1f99](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3/blob/541d1f99c6b0c3cd0b11a95167540bb8edefd82b/parakeet-tdt-0.6b-v3.q8_0.gguf). NVIDIA déclare [CC BY 4.0](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) ; la remise doit inclure attribution, source, licence et indication des modifications applicables selon [la licence](https://creativecommons.org/licenses/by/4.0/legalcode.en).

Une copie de travail des exécutables et DLL existe sous `C:/dev/whisper/target/n2-native/qualification-gpu/inputs/personal-runtime/`. Elle est historique et n’est pas encore retenue comme paquet public. L’inventaire associé `C:/dev/whisper/target/n2-native/qualification-gpu/runtime-inventory.json` indique un profil Codex distinct du profil utilisateur courant, expliquant l’absence observée dans ce dernier.

| Ressource | SHA-256 mesuré |
|---|---|
| worker Parakeet CPU | `a965c9503f909124f8ddedad09e83e5d187e70edd1869e880e9f1ad5ae2fb99d` |
| worker Parakeet GPU | `7ffb0b4a022ec26909a8939b37787de6df6c9bdb2903edf14d3ffb32579eefc1` |
| CPU ggml-base.dll | `398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a` |
| CPU ggml-cpu.dll | `f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0` |
| CPU ggml.dll | `2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794` |
| CPU nemo_speech_asr_c.dll | `9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764` |
| CPU nemo_speech_asr.dll | `923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019` |
| GPU ggml-base.dll | `24b7342f10bc14e17dcb55ebafdbaa1a0fd0187cf5ae1a8ecf03bb9f8a17ef93` |
| GPU ggml-cpu.dll | `23166143accf20055ccaafb8b720ad3dac33bf16ac027cc77225e0bb2a99e699` |
| GPU ggml-cuda.dll | `7e244413387852825ed420fceef5e3b318f3a264f97f59fe0233789dfbdb6822` |
| GPU ggml.dll | `9148ba2ed0eaeee385d09eba37e1a297d5e2f0649a02ffd4e325fea562c29ea6` |
| GPU nemo_speech_asr_c.dll | `6643c1c657f05bd545655044ec59fc052a789042ce9c17496b9c77a240c7bbb9` |
| GPU nemo_speech_asr.dll | `4fac537a8d99a8cee4cd1dba2391379cb895fafb7d21beb9d6f4fe670be05449` |
| GPU cublas64_12.dll | `52ce5ba0ec5327d39be6021f0f9362d2ed4d64d7206f9b5afc06398d827b8e82` |
| GPU cublasLt64_12.dll | `71705a7cf0923f4ab034d0d2620bbfc989ff669ac3d79f868b7a7c597ca559c5` |
| GPU cudart64_12.dll | `760c38928bbe5759f7b31ed6692599eb7ec83cedd5702e84c2b72028a89837e1` |

Le contrat du worker `crates/whisper-worker-parakeet-cpu/src/integrity.rs` calcule une identité du runtime à partir des cinq DLL CPU et, pour le profil CUDA, de quatre DLL supplémentaires. Le modèle et ces identités ne prouvent pas une inférence ni le routage autonome. Les DLL correspondent aux valeurs de `C:/dev/whisper/docs/cadrage/fiche-parakeet-l6-perso.md` ; les workers mesurés sont d’autres révisions que ceux de cette fiche. `C:/dev/whisper/docs/architecture/NIVEAU1.md` rattache ces workers à un inventaire ultérieur ; les essais D2 mentionnés restent incomplets.

## Licences et prérequis

Les sources historiques indiquent NeMo-Speech.cpp au commit `6a3ca369370782acee0dd155e34394ba60b920c4`, [licence Apache-2.0](https://github.com/NVIDIA/NeMo-Speech.cpp/blob/6a3ca369370782acee0dd155e34394ba60b920c4/LICENSE), et llama.cpp au commit `bd4f514db14d87fded667787a7a963bfbaa98e89`. Leur lien exact aux DLL mesurées, les modifications, avis NOTICE et dépendances effectivement liées restent à établir. CUDA 12.9 inclut les familles cuBLAS et runtime sous les [conditions NVIDIA](https://docs.nvidia.com/cuda/archive/12.9.2/eula/index.html). Vérifier également SentencePiece, protobuf, tokenizer et dépendances transitives effectivement incorporées. Les droits du modèle ne ferment pas les licences des binaires.

Le bilan `C:/dev/whisper/docs/architecture/BUILD-NATIF-QUALIF-PTX.md` rapporte un build historique `GGML_NATIVE=ON`, AVX2/FMA/F16C et CUDA `sm_61`. Ce constat est attribué à ce build ; les instructions effectivement exécutées et la compatibilité de chaque DLL sur chaque machine restent à démontrer. Microsoft documente l’émulation x64 par Prism et ses réglages sur Windows ARM ; cela ne qualifie pas ce runtime sur Snapdragon. [Microsoft](https://learn.microsoft.com/en-us/windows/arm/apps-on-arm-program-compat-troubleshooter).

## Solution proposée

1. Rattacher les seize fichiers mesurés aux journaux, sources, importations PE et notices ; contrôler les prérequis CPU et pilote CUDA, sans modifier ces copies historiques.
2. Préparer un essai borné sur les ressources identifiées, comprenant les capacités vues par le processus x64, le chargement et l’inférence CPU sur Dell et Snapdragon, puis CUDA sur GTX 1080 Ti. Chaque essai doit identifier modèle, worker, DLL et résultat. Les autorisations, commandes et critères exacts seront définis avant exécution. Si les ressources passent et que leurs droits sont établis, étudier leur intégration au nouveau paquet.
3. Si une reconstruction est nécessaire, proposer un build CPU portable avec `GGML_NATIVE=OFF` et options ISA explicites après audit des sources, accompagné d’un build GPU CUDA `sm_61` pour la GTX. Fixer sources, patches, outils, configuration, dépendances, profils de garde et tests de codegen avant tout lancement. Le script et la configuration historiques ne démontrent pas déjà ce profil portable.

La reconstruction native reste suspendue après WHEA non qualifiée. Sa reprise exige une décision précise et `ci/build-native.ps1` par sa voie surveillée selon AGENTS.md. La preuve d’une configuration future ne rétablit pas la chaîne historique perdue. Aucune construction, inférence ou qualification matérielle n’a été exécutée durant cet inventaire.
