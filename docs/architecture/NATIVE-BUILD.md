Avancée : cpu6 reste le dernier build terminé ; stabilisations de clôture codées et relues.
Blocage : cpu7 interrompu par la garde ; cpu7b refusé avant lancement pour une jonction Windows dans l'espace fixe.
Décision : qualification de clôture non démontrée ; aucune installation ni fusion, voir le nouveau bilan.

# DLL natives — provenance et reconstruction BUILD-NATIF

## État de la procédure

**BUILD-NATIF-CLOTURE : arrêt sur blocage.** Le [bilan de clôture](BUILD-NATIF-CLOTURE-BILAN.md) remplace le critère d'identité avec l'installation par surfaces PE, égalité des sorties N2 et répétabilité entre deux constructions neuves. Aucun de ces trois critères n'est établi pour le candidat de clôture. Les résultats cpu6 ci-dessous restent historiques.

Le pilote matérialise désormais sources, dépendances, build, profils et temporaires au chemin physique constant `C:/wb/native/<cpu|gpu>`. Les téléchargements restent dans `native/cache/` ; les caches binaires vcpkg anciens ne sont pas réutilisés. Un patch local versionné respecte `GGML_BUILD_COMMIT`, fourni avec le commit llama complet ; le getter PE `ggml_commit` est décodé sans charger la DLL et doit correspondre au pin après compilation. Les 19 patches amont et les options ISA restent inchangés. Le contrôle du getter reconstruit n'a pas été atteint.

Une invocation publique par variante : `pwsh -File ci/build-native.ps1 -Variant cpu -BuildId <identifiant-neuf>` ou `pwsh -File ci/build-native.ps1 -Variant gpu -BuildId <identifiant-neuf>`. **Ne pas relancer tant que le blocage de préservation n'est pas arbitré.** Le précédent espace doit être préservé et remplacé par un espace neuf ; le script refuse actuellement toute jonction interne. Windows a créé `userprofile/AppData/Local/Microsoft/Windows/INetCache/Content.IE5` vers `INetCache/IE`, ce qui empêche cette préparation. Aucun déplacement ni nettoyage effectué.

`VSCMD_SKIP_SENDTELEMETRY=1` désactive seulement le lanceur de télémétrie de VsDevCmd avant initialisation ; un témoin réel comparatif confirme la branche supprimée et les mêmes versions MSVC/SDK. Cela ne change aucun refus du collecteur et ne garantit pas l'absence d'autres renommages répétés. cpu7 a été arrêté sur quatre notifications de renommage d'une même identité dans un drainage ; cpu7b n'a pas démarré de compilateur.

Les quatre profils N2 de clôture CPU/GPU référence/reconstruit sont bornés aux sélections du plan. Le pin persistant `native/runtime-pins/reference.json` décrit 21 fichiers ; seuls cinq fichiers CPU ou six fichiers ggml/NeMo GPU pourront différer dans les pins reconstruits futurs, contrôlés par noms, tailles et SHA. Les DLL CUDA restent des copies vérifiées du toolkit. Aucun pin reconstruit, runtime alternatif, résultat N2 ou comparaison N1 n'a été produit dans cette clôture. L'installation est inchangée.

`ci/build-native.ps1` et `native/build-config.json` constituent un candidat dont la voie surveillée est implémentée. `sources2` a récupéré les trois sources et contrôlé leurs pins ; les profils `NativeDllCpu/Gpu` se limitent au script et aux racines prévues. Après cpu1 et cpu2, l'utilisateur a autorisé une observation plus précise et un unique cpu3 : cet essai s'est arrêté avec un nouveau nom effectivement null. Le snapshot du worker, écrit avant CMake, a survécu à l'arrêt. Ce document ne constitue pas encore une recette de compilation validée. Aucune DLL ggml/NeMo reconstruite, compilation Rust ou campagne Cargo/N2. Les 14 empreintes installées ont été revérifiées sans changement. Bilans : [préparation](../cadrage/build-natif-script-phase2.md), [cpu1](../cadrage/build-natif-essai-surveille.md), [cpu2](../cadrage/build-natif-arret-cpu2.md), [diagnostic cpu3](../cadrage/build-natif-diagnostic-cpu3.md).

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

Le fichier de configuration fixe les trois commits, les versions et les options. Hypothèses acceptées pour vérification : Ninja 1.12.1, GGML_NATIVE=ON, autres valeurs amont non retrouvées, /MD explicite ; /Brepro est ajouté conformément au mandat. Chaque manifeste conserve ces limites. Le vcpkg.exe de la release 2026-07-27 est épinglé par SHA-256 et comparé aux métadonnées du bootstrap avant réutilisation ; le digest d'asset provient de l'[API officielle GitHub](https://api.github.com/repos/microsoft/vcpkg-tool/releases/tags/2026-07-27). Les variables vcpkg héritées sont neutralisées, puis ses caches explicitement fixés sous `native/cache/`. Ces mesures complètent la surveillance native existante réutilisée par les nouveaux profils. La règle de production uniquement par ce script figure dans AGENTS.md ; elle n'annonce pas la validation de la chaîne.

Commandes préparatoires depuis le worktree, PowerShell 7 :

```powershell
./ci/build-native.ps1 -SelfTest
./ci/build-native.ps1 -Variant cpu -CheckOnly
./ci/build-native.ps1 -Variant gpu -CheckOnly
```

Elles ne téléchargent ni ne compilent. Auto-tests supplémentaires de la voie surveillée, sans bootstrap/CMake :

```powershell
./ci/guard-native.ps1 -Mode SelfTest -Profile NativeDllCpu -LotRoot target/native-build
```

Les commandes de campagne prévues sont `./ci/build-native.ps1 -Variant cpu -BuildId <id-neuf>` et la même commande avec `-Variant gpu`, une variante par appel. `-PrepareOnly` sélectionne la seule acquisition. Le script public appelle la garde, dont le processus enfant vérifie le parent réel et l'identité du script. Les racines sont canoniques et sans liens ; sorties, variables temporaires et dossier courant restent dans l'exécution, caches dans `native/cache/`. La garde refuse les profils modifiés, les identifiants traversants et les dossiers de sortie déjà présents ; elle capture les sorties/codes et s'arrête sur échec ou violation. La surveillance n'est pas un confinement préventif universel.

Historique d'exécution : `sources1` a échoué sur le gitlink vide, cas corrigé après accord utilisateur. `sources2` a ensuite réussi sous garde. `cpu1` a échoué avant CMake : le BAT de bootstrap cherchait `powershell.exe` dans un PATH filtré où il n'était pas présent ; la garde a aussi observé la création fugace de `C:/wt/build-natif/$null`, hors racines autorisées. Son origine n'est pas attribuée et son absence après arrêt ne résulte d'aucun nettoyage de l'agent. Le correctif appelle Windows PowerShell par son chemin OS explicite et le même script fournisseur ; la garde fixe le dossier courant sous sa racine d'exécution. Ces deux changements ont été relus, avec tests de refus et témoin du dossier courant, avant la relance cpu2 autorisée ultérieurement. Les sorties cpu1 sont conservées.

La relance cpu2 a été autorisée et son bootstrap a réussi : le vcpkg.exe téléchargé correspond au digest épinglé. CMake a démarré, mais un événement Renamed a interrompu la configuration d'Abseil : ancien nom sous le cache, nouveau FullPath égal à la racine `native/`. Dans cpu2, la garde ne conservait pas les noms bruts permettant d'établir la destination réelle ; un nom vide peut produire ce chemin, comme vérifié par un test .NET sans écriture. Cette hypothèse ne permettait pas de poursuivre ou d'ignorer l'événement. Le manifeste du worker cpu2 n'a pas survécu à l'interruption (écrit seulement en finally) : persistance préalable et meilleure observation étaient proposées pour arbitrage dans le bilan cpu2, avant l'autorisation cpu3 décrite ci-dessous.

Après autorisation spécifique, cpu3 a ajouté l'observation des noms bruts, leurs états null/vide et la racine de l'observateur. Les événements NativeBuild sont enregistrés dans `watcher-events.jsonl`, en plus des traces d'arrêt existantes ; leur classement reste inchangé. Le manifeste est maintenant écrit atomiquement avant acquisition, bootstrap, configuration et build, puis à la clôture si le worker peut l'atteindre. Après l'arrêt forcé de cpu3, il est resté lisible à STARTED/CONFIGURE avec les options demandées : ce snapshot ne constitue pas un PASS, le résultat de garde est FAIL. Sur 56 renommages enregistrés, un nouveau nom est réellement null ; la destination n'a donc pas été fournie dans cet événement. Sa cause et sa destination restent inconnues. L'autorisation de cpu3 était unique : aucune suite CPU/GPU/N2 ni répétition n'est engagée.

Suite du plan, non engagée par cpu4 : deux compilations indépendantes par variante, comparaison binaire puis exports/imports/tailles/options ; ensuite seulement runtime N2 alternatif sans toucher à l'installation et au plus huit essais, puis N1 comparé à main selon le plan. **Statut actuel : NON REPRODUIT**, équivalence statique partiellement évaluée, non établie ; équivalence fonctionnelle NON EXÉCUTÉE. Conservation intégrale des ressources protégées ; aucune commande de nettoyage exécutée.

Diagnostic puis correction autorisés : [limite d'assemblage FileSystemWatcher, correction et preuves](../cadrage/build-natif-cause-renommage.md). NativeBuild utilise maintenant `ci/native-directory-watch.cs`, chargé en mémoire par la garde, avec temporaires de compilation gérés sous le lot. Windows 10 1709+ et NTFS sur chaque racine surveillée sont obligatoires ; échec avant enfant si indisponibles, sans repli. Les enregistrements étendus sont journalisés dans `watcher-native-records.jsonl`, par instantané de drainage ; les décisions restent dans `watcher-events.jsonl`. Moitiés non réunies au contrôle courant, ambiguïtés, débordements et erreurs restent bloquants, sans attente supplémentaire. Les autres profils conservent leur collecte antérieure. La commande SelfTest ci-dessus inclut les contrôles du collecteur, 1 000 renommages concurrents réels, frontières, capacité et durée de vie native.

La revue indépendante du correctif est CLEAN. Le déclencheur Win32 exact et la destination historique cpu3 restent inconnus. Limite héritée mesurée : certains mouvements interparents sont notifiés Deleted+Created, pas Renamed. La politique existante peut alors classer séparément une suppression Temp UNATTRIBUTED_TEMP et une création autorisée ALLOWED ; aucune garantie générale d'arrêt de tous ces mouvements ni absence universelle de fuite n'est établie.

Une relance CPU unique a ensuite été autorisée et exécutée : **cpu4 PASS**, 512,061395 s, cinq DLL sous `target/native-build/cpu/cpu4/build/bin/`, manifeste BUILT_NOT_QUALIFIED. GGML_NATIVE=ON, GGML_CUDA=OFF, Release, /MD sont effectifs pour cette construction ; la valeur historique de GGML_NATIVE et la portabilité CPU restent non prouvées. Les 14 empreintes installées sont inchangées. Les observations PE en lecture seule montrent des écarts non expliqués : exports identiques 4/5, imports DLL+symboles identiques 1/5, tailles ggml.dll −4,132 % et nemo_speech_asr_c.dll −3,636 %. Le contrôle PE existant sans CUDA passe pour les cinq DLL, imports directs et différés. Cela ne démontre pas la résolution récursive de toutes les dépendances à l'exécution. [Tableaux, empreintes et archive vérifiée](../cadrage/build-natif-cause-renommage.md#compilation-cpu4-autorisée). Aucune deuxième compilation, GPU, N1 ou N2 exécutée ; aucune fusion recommandée en l'état.

Après une nouvelle autorisation, **cpu5 PASS** en 53,9897883 s, mêmes pins/options, nouveau dossier. Comparaison : NeMo et son API C identiques octet pour octet ; ggml.dll et ggml-cpu.dll différents uniquement par les chemins intégrés et champs PE/REPRO identifiés (normalisation analytique en mémoire, aucun binaire modifié) ; ggml-base.dll conserve des différences de code et de déroulement non entièrement expliquées. Les imports et exports nom+ordinal sont identiques entre cpu4/cpu5 pour les cinq DLL ; les écarts avec l'installation persistent.

**Défaut confirmé sur cpu4/cpu5 :** `native/build-config.json` à `cb3ed03e` fixait CMAKE_C_FLAGS et CMAKE_CXX_FLAGS à /Brepro, remplaçant les défauts CMake. /EHsc manquait dans le Ninja ; chacun émettait 30 C4530. La gestion du déroulement des exceptions C++ est affectée, donc aucune équivalence fonctionnelle n'en est déduite. La valeur historique exacte des options reste introuvable et l'attribution de tous les écarts installés à ce défaut n'est pas démontrée. [Détail et limites de l'analyse](../cadrage/build-natif-cause-renommage.md#cpu5-et-analyse-des-écarts).

Après autorisation de correction, la configuration transmet /Brepro par CMAKE_C_FLAGS_INIT, CMAKE_CXX_FLAGS_INIT, CMAKE_SHARED_LINKER_FLAGS_INIT et CMAKE_EXE_LINKER_FLAGS_INIT ; le script utilise CMAKE_CUDA_FLAGS_INIT pour l'hôte CUDA. Les défauts de plateforme sont ainsi laissés à CMake. /GR et /W3 ne sont pas imposés : les politiques CMP0117/CMP0092 peuvent les omettre avec le NeMo épinglé. Avant BUILD, le cache global et Release doivent conserver /EHsc, /Brepro, les définitions Windows et la machine x64 ; contradictions refusées, résultat consigné dans `flagValidation` du manifeste. Ce contrôle ne prétend pas analyser toutes les options Ninja par cible.

`Invoke-Logged` capture stdout/stderr et conserve C4530 avant d'échouer ; le nettoyage s'applique même si le parent direct est déjà sorti. Il réutilise six fonctions versionnées de familles de processus de `ci/guard-native.ps1`, importées seules via AST sans exécuter son entrée ni modifier la garde. Identités/dates de création, fermeture et empreinte du code de contrôle sont consignées dans `process-control.jsonl` et le manifeste. Les garanties et limites d'observation restent celles des helpers existants, sans nouvelle isolation Windows ni surveillance de fichiers. SelfTest inclut des émetteurs possédés sur les deux flux, un descendant et un parent déjà sorti ; 41 contrôles PASS, huit négatifs existants PASS, garde complète et CheckOnly CPU/GPU PASS. Revue indépendante CLEAN, BN-F01 fermé. [Archive des tests préparatoires](../cadrage/build-natif-cause-renommage.md#préservation-des-options-et-arrêt-c4530). Ces auto-tests ne compilaient pas le candidat ; la construction réelle cpu6 est décrite ci-dessous.

**Candidat corrigé éprouvé une fois : cpu6 PASS**, 64,7947984 s. Les 63 commandes C++ ont /EHsc et les 69 commandes C/C++ /Brepro et /MD ; aucun C4530. Les cinq DLL ont les mêmes exports nom+ordinal et imports DLL+symboles que l'installation, sans import CUDA direct/différé. Quatre tailles sont identiques ; NeMo est plus petit de 1 024 octets (−0,024 %, dans .rdata). Les SHA-256 diffèrent 5/5, donc aucune identité binaire avec l'installation ; aucun verdict fonctionnel déduit de cette concordance statique. Les 14 DLL installées sont inchangées. [Comparaison, empreintes et preuve](../cadrage/build-natif-cause-renommage.md#cpu6-effet-réel-du-correctif-et-concordance-statique). Une seule construction du nouveau candidat : répétabilité non éprouvée, N2/N1/GPU non exécutés.

**Métadonnée ggml_commit :** le CMake de la copie llama patchée appelle Git depuis son dossier sans métadonnées indépendantes ; Git trouve le dépôt Whisper parent. Le header et le getter compilé portent ainsi `338c7a9e` dans cpu6, `77f4413b` dans cpu4 et `d27a5d9d` dans cpu5 ; la DLL installée porte `6cf94e8b`, qui résout aussi un commit Whisper. Le vrai pin source llama reste `bd4f514db14d87fded667787a7a963bfbaa98e89`, vérifié dans le cache et le manifeste. Ne pas lire ce getter comme une preuve de version des sources. Cette entrée dépend donc de HEAD, en plus des chemins de build ; sa stabilisation reste à traiter avant une revendication de reproductibilité indépendante des commits documentaires. Aucune correction de cette métadonnée ni explication complète des anciennes différences .text de ggml-base n'est annoncée. Pas de fusion recommandée avant preuve complète.
