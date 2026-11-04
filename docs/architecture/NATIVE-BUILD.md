Avancée : inventaire et script préparatoire versionnés le 4 novembre 2026 ; provenance historique retrouvée.
Blocage : voie CMake absente de la garde ; acquisition llama refusée sur un dossier de sous-module vide.
Décision : LIVRÉ AVEC RÉSERVES pour la préparation ; aucune équivalence ni compilation reproductible revendiquée.

# DLL natives — provenance et reconstruction BUILD-NATIF

## État de la procédure

`ci/build-native.ps1` et `native/build-config.json` constituent un candidat préparatoire. Le mode construction refuse explicitement toute exécution tant que la garde existante n'a pas de voie CMake autorisée ; aucun paramètre de contournement n'est fourni. Le téléchargement préparatoire a récupéré NeMo mais s'est arrêté sur le dossier vide du sous-module llama. Ce document ne constitue pas encore une recette de compilation validée. Aucun chargement des DLL, compilation, test Cargo ou N2 n'a été lancé. L'installation `%LOCALAPPDATA%/Whisper` est restée en lecture seule. Voir le [bilan de phase 2](../cadrage/build-natif-script-phase2.md).

La recherche dans le profil n'avait trouvé aucune copie de la chaîne. La lecture ultérieure des chats compressés a retrouvé les **commandes et sorties historiques**, notamment deux lanceurs et des extraits CMake ; elle n'a pas restauré le dossier disparu. Les extraits utiles sont conservés dans [history-extracts.txt](raw/native-build/history-extracts.txt), avec nom de session, numéro de ligne JSONL décompressée, horodatage, commande et sortie. Ils ne remplacent pas l'archive de phase 0.

Les relevés texte versionnés normalisent les fins de ligne en LF et retirent les espaces de fin de ligne ; les symboles et empreintes ne sont pas modifiés. Ils ne sont pas des copies octet pour octet des sorties de terminal.

## Provenance fixée

| Élément | Origine / révision | Preuve |
| --- | --- | --- |
| NeMo-Speech.cpp, ABI ASR C/C++ | NVIDIA/NeMo-Speech.cpp, v0.2.0, `6a3ca369370782acee0dd155e34394ba60b920c4` | history-extracts.txt:15 et :479 ; evo-parakeet.md:294 |
| ggml fourni par llama.cpp | ggml-org/llama.cpp, `bd4f514db14d87fded667787a7a963bfbaa98e89` | history-extracts.txt:15, :458, :481 ; `.gitmodules` historique dans le même fichier |
| vcpkg | microsoft/vcpkg, `9e593bb18ea69cc5095e012465dcd675a822ed0d` | history-extracts.txt:15 et :482 ; pin du pilote Windows NeMo |
| Correctifs ggml/llama | Série de **19 patches fournis par NeMo**, appliqués à une copie `_deps/llama.cpp` | history-extracts.txt:496 ; [série versionnée](https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/6a3ca369370782acee0dd155e34394ba60b920c4/patches/series) |

Les sources NeMo sont donc identifiées comme provenant du dépôt NVIDIA, et non comme un moteur ASR écrit dans le spike. Les commandes Git historiques montrent les révisions complètes et aucune ligne de modification pour les contrôles `status --short` conservés. Cela ne prouve pas l'absence de toute modification ultérieure ou directe de `_deps` avant copie des DLL. Aucun correctif local supplémentaire retrouvé.

La [résolution CMake épinglée](https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/6a3ca369370782acee0dd155e34394ba60b920c4/cmake/llama_cpp.cmake), lignes 160–189, utilise le sous-module puis la copie corrigée. Ce mécanisme explique le chemin d'assertion sous `build-cpu/_deps/llama.cpp`. Il ne faut pas reconstruire ggml depuis llama.cpp seul sans les patches NeMo. La [documentation des patches](https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/6a3ca369370782acee0dd155e34394ba60b920c4/patches/README.md) explique leur application dans l'ordre `series`.

## Outils et options

Les versions historiques sont distinguées des outils simplement présents aujourd'hui. Les commandes actuelles et sorties PE sont dans [installed-pe.txt](raw/native-build/installed-pe.txt).

| Paramètre | CPU | GPU | Preuve / limite |
| --- | --- | --- | --- |
| CMake | 4.4.4 | 4.4.4 | Bilan historique extrait :17 ; `cmake --version` actuel identique |
| Toolset MSVC | 14.44.35207 | 14.44.35207 | Caches historiques extraits :295 et :306 ; PE linker 14.44 |
| Compilateur | MSVC x64 | MSVC x64 19.44.35229.0 | Trace CUDA historique :465 ; `cl` 2022 actuel : 19.44.35229 |
| SDK Windows | UCRT 10.0.26100.0 | UCRT 10.0.26100.0 | Traces de compilation historiques :517, :549 ; version des outils link/rc SDK non prouvée |
| Générateur | Ninja | Ninja | Commande CMake historique :459 ; version historique non retrouvée, binaire VS2022 actuel 1.12.1 |
| MSBuild | Non utilisé par la commande Ninja | Idem | Version actuelle 17.14.60.43110, sans preuve historique ni nécessité démontrée pour les DLL |
| Configuration | Release | Release | Cache CPU :294 ; GPU :304 |
| Runtime | Dynamique, compatible /MD | Idem | Triplet `x64-windows-static-md` :302/:313 et imports MSVCP140/VCRUNTIME140 ; flags de chaque compilation non récupérés |
| CUDA | GGML_CUDA=OFF | GGML_CUDA=ON, nvcc 12.9.86 | Caches :297/:308 ; trace :467 ; nvcc actuel identique |
| Architectures CUDA | Sans objet | `61` | Cache :305 ; `cuobjdump --list-elf` actuel : 146 images sm_61 ; présence éventuelle de PTX non inventoriée par cette commande |
| GGML_NATIVE | Valeur effective non récupérée | Valeur effective non récupérée | Risque ON : défaut amont ON hors cross-compilation/SOURCE_DATE_EPOCH ; aucune déduction certaine depuis ce défaut |
| ISA CPU ggml | Non récupérée directement | `/arch:AVX2`, GGML_AVX2, GGML_FMA, GGML_F16C | Trace :508 issue de la configuration CUDA ; ne pas l'étendre sans preuve à la variante CPU |
| GGML_AVX*, autres flags | Cache complet manquant | Cache complet manquant | La ligne de backend ne remplace pas toutes les valeurs CMake |
| ASR / CLI / DIAR / TTS | ON / OFF / OFF / OFF | Identique | Cache CPU :298–301 ; GPU :309–312 |

**Risque prioritaire :** AVX2/FMA/F16C sont attestés pour le backend CPU embarqué dans la variante CUDA. La portabilité vers un processeur plus ancien n'est pas établie. Le [CMake ggml épinglé](https://raw.githubusercontent.com/ggml-org/llama.cpp/bd4f514db14d87fded667787a7a963bfbaa98e89/ggml/CMakeLists.txt), lignes 95–143, définit GGML_NATIVE et les défauts ISA ; l'état du cache historique reste inconnu. Ne pas corriger les options dans BUILD-NATIF. Le toolset 2019 présent dans PATH aujourd'hui (19.29.30159) diffère du toolset historique 2022 : le futur script devra choisir et vérifier explicitement ce dernier.

### Commandes historiques retrouvées (ne pas relancer dans le spike)

Les deux lanceurs complets sont archivés comme texte dans history-extracts.txt:140 et :169. Ils invoquaient `NeMo-Speech.cpp/scripts/windows/build.ps1` avec `-Backend cpu` ou `-Backend cuda`, `-Profile asr`, `-Jobs 4`, `-VcpkgRoot <spike02>/vcpkg`, une racine build distincte, `-CMakeArgs @('-DNEMO_SPEECH_BUILD_DIAR=OFF', '-DNEMO_SPEECH_BUILD_CLI=OFF')`, et `-CudaArch '61'` pour CUDA. TEMP/TMP, cache XDG et caches/téléchargements vcpkg étaient redirigés sous le spike. Ce sont des éléments d'inventaire, pas une procédure autonome nouvelle.

La commande CMake CUDA détaillée conservée :459 désactive aussi NMT, HTTP, TLS, GRPC, FLASHLIGHT, NORM, TTS_JA/ZH, tests, exemples, outils, Vulkan et le shim cuBLAS ; elle sélectionne le toolchain vcpkg et la feature asr. Ses deux définitions DIAR successives finissent à OFF dans le cache. Le [pilote amont fixé](https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/6a3ca369370782acee0dd155e34394ba60b920c4/scripts/windows/build.ps1) rafraîchit PATH et choisit les outils : il devra être encadré, sans laisser sa sélection automatique invalider les versions épinglées.

## DLL réellement installées

Racine inspectée : `C:/Users/cyril/AppData/Local/Whisper/native`. SHA-256 recalculés avant et après l'inspection de chaque DLL, identiques. **14/14 empreintes figurent dans `scripts/install-personal.ps1`** (tables à partir de :628). Données structurées : [installed-inventory.json](raw/native-build/installed-inventory.json). Exports/imports complets, y compris symboles : installed-pe.txt ; chaque section porte sa commande et son SHA.

| Variante / DLL | Taille | SHA-256 | Exports nommés relevés |
| --- | ---: | --- | ---: |
| CPU ggml-base.dll | 680960 | `398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a` | 691 |
| CPU ggml-cpu.dll | 947200 | `f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0` | 64 |
| CPU ggml.dll | 61952 | `2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794` | 16 |
| CPU nemo_speech_asr_c.dll | 56320 | `9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764` | 40 |
| CPU nemo_speech_asr.dll | 4192256 | `923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019` | 10992 |
| GPU ggml-base.dll | 680960 | `24b7342f10bc14e17dcb55ebafdbaa1a0fd0187cf5ae1a8ecf03bb9f8a17ef93` | 691 |
| GPU ggml-cpu.dll | 947200 | `23166143accf20055ccaafb8b720ad3dac33bf16ac027cc77225e0bb2a99e699` | 64 |
| GPU ggml-cuda.dll | 33219072 | `7e244413387852825ed420fceef5e3b318f3a264f97f59fe0233789dfbdb6822` | 14 |
| GPU ggml.dll | 61952 | `9148ba2ed0eaeee385d09eba37e1a297d5e2f0649a02ffd4e325fea562c29ea6` | 16 |
| GPU nemo_speech_asr_c.dll | 56320 | `6643c1c657f05bd545655044ec59fc052a789042ce9c17496b9c77a240c7bbb9` | 40 |
| GPU nemo_speech_asr.dll | 4209152 | `4fac537a8d99a8cee4cd1dba2391379cb895fafb7d21beb9d6f4fe670be05449` | 11064 |
| GPU cublas64_12.dll | 102518272 | `52ce5ba0ec5327d39be6021f0f9362d2ed4d64d7206f9b5afc06398d827b8e82` | 747 |
| GPU cublasLt64_12.dll | 668673536 | `71705a7cf0923f4ab034d0d2620bbfc989ff669ac3d79f868b7a7c597ca559c5` | 2492 |
| GPU cudart64_12.dll | 583680 | `760c38928bbe5759f7b31ed6692599eb7ec83cedd5702e84c2b72028a89837e1` | 461 |

Les exports ont été relevés par `dumpbin /exports`, imports par `/imports` et `/dependents`, caractéristiques par `/headers`. Les nombres seuls ne suffiront pas à l'équivalence : comparer les noms et ordinaux, ainsi que les imports complets. Le contrôle `ci/check-worker-imports.ps1 -SelfTest` donne **4 positifs et 15 négatifs PASS**, témoin réel facultatif NON EXÉCUTÉ ; `-Kind ParakeetCpu -Path <DLL>` donne **5/5 PASS** sur les DLL CPU installées (imports directs et différés). Aucun résultat N1 global n'est revendiqué.

ggml-cuda importe notamment ggml-base, cudart, cublas et nvcuda. La DLL pilote `nvcuda.dll` n'appartient pas au payload. Les imports VCRUNTIME/MSVCP, UCRT et VCOMP montrent aussi des dépendances système/runtime à préserver, au-delà des seules DLL du produit. L'inspection statique ne prouve pas les chargements dynamiques en exécution.

### Redistribuables CUDA

Les trois DLL NVIDIA installées sont **identiques par SHA-256** à `C:/Program Files/NVIDIA GPU Computing Toolkit/CUDA/v12.9/bin/<nom>`. Le toolkit actuel annonce SDK 12.9.2, nvcc 12.9.86, cudart 12.9.79 et cuBLAS 12.9.2.10 dans `version.json`. Ne pas confondre SDK, compilateur et versions de composants. Leurs FileVersion Win32 sont conservées dans le JSON ; elles ne remplacent pas les versions composants. Ce sont des copies binaires du toolkit, sans compilation locale de ces trois DLL.

## Licences et contrainte connue

- NeMo-Speech.cpp : [Apache-2.0 au commit fixé](https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/6a3ca369370782acee0dd155e34394ba60b920c4/LICENSE), avec notices tierces à inventorier depuis l'ensemble des sources et dépendances lors de la préparation du script.
- llama.cpp/ggml : [MIT au commit fixé](https://raw.githubusercontent.com/ggml-org/llama.cpp/bd4f514db14d87fded667787a7a963bfbaa98e89/LICENSE). Les patches sont livrés par NeMo ; préserver leurs en-têtes et notices.
- CUDA : licence NVIDIA propriétaire, EULA locale du toolkit, section 2.6 Attachment A, lignes 585–647 : listes runtime et cuBLAS, noms à compléter avec les suffixes de version ; [EULA CUDA 12.9.1 publiée](https://docs.nvidia.com/cuda/archive/12.9.1/eula/index.html). Cette identification documentaire ne constitue pas une validation de conformité du paquet livré.

**Contrainte de chargement :** les bilans historiques `evo-parakeet.md:312` et `bilan-parakeet-fr-en-go.md:11` rapportent un crash après inférence et `FreeLibrary` ; le banc conservait les DLL jusqu'à la sortie du worker. L'utilisateur rapporte également GGML_ASSERT lors d'un chargement puis rechargement dans un même processus. Ne pas déduire que destruction d'un recognizer autorise un déchargement/rechargement sûr. Aucun de ces défauts n'a été rejoué dans BUILD-NATIF.

## Script préparatoire de phase 2 et vérification future

Préserver les CMake **du NeMo exact**, sans réécrire les sources ASR. L'utilisateur a autorisé la préparation avec les révisions retrouvées et les paramètres manquants explicités comme hypothèses à éprouver. Les copies Git épinglées dans `native/cache/` persistant ignoré conservent la sémantique Git du mécanisme des patches. Les commandes CMake historiques sont reproduites par le script ; le pilote Windows amont n'est pas lancé, afin de ne pas lui laisser choisir automatiquement les versions et rafraîchir PATH depuis le registre. Aucun CMakeLists enveloppe n'est ajouté : il modifierait CMAKE_SOURCE_DIR et le chemin des sous-modules.

Le fichier de configuration fixe les trois commits, les versions et les options. Hypothèses acceptées pour vérification : Ninja 1.12.1, GGML_NATIVE=ON, autres valeurs amont non retrouvées, /MD explicite ; /Brepro est ajouté conformément au mandat. Chaque manifeste conserve ces limites. Le vcpkg.exe de la release 2026-07-27 est épinglé par SHA-256 et comparé aux métadonnées du bootstrap avant réutilisation ; le digest d'asset provient de l'[API officielle GitHub](https://api.github.com/repos/microsoft/vcpkg-tool/releases/tags/2026-07-27). Les variables vcpkg héritées sont neutralisées, puis ses caches explicitement fixés sous `native/cache/`. Ces mesures ne remplacent pas la surveillance native exigée. Aucune nouvelle règle AGENTS imposant une chaîne non fonctionnelle n'est ajoutée ; la conservation existante demeure.

Commandes préparatoires depuis le worktree, PowerShell 7 :

```powershell
./ci/build-native.ps1 -SelfTest
./ci/build-native.ps1 -Variant cpu -CheckOnly
./ci/build-native.ps1 -Variant gpu -CheckOnly
```

Elles ne téléchargent ni ne compilent. L'essai `-Variant cpu -BuildId sources1 -PrepareOnly` a échoué avant récupération de llama/vcpkg : le répertoire vide de gitlink llama créé par le checkout NeMo est considéré comme cache existant incomplet. Ne pas relancer avant correction autorisée de ce cas. Les sorties de l'essai sont conservées et archivées. La commande prévue pour une construction par variante, `-Variant cpu|gpu -BuildId <id-neuf>`, est actuellement **refusée avant écriture**, pour absence de voie CMake dans la garde. Les branches bootstrap/CMake du candidat restent donc non validées en exécution.

Après résolution de ce point : une commande par variante, deux compilations indépendantes, comparaison binaire puis exports/imports/tailles/options ; ensuite seulement runtime N2 alternatif sans toucher à l'installation et au plus huit essais, puis N1 comparé à main selon le plan. **Statut actuel : NON REPRODUIT**, équivalences statique et fonctionnelle NON EXÉCUTÉES. Conservation intégrale des ressources protégées ; aucune commande de nettoyage exécutée ou nécessaire à cette phase documentaire.
