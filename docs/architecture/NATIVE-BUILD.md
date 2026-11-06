DLL produites par ce script : NON QUALIFIÉES. Les DLL installées restent la référence. Ne pas supprimer target/parakeet-feasibility/.

# DLL natives — provenance et reconstruction

État du 6 novembre 2026 : voir la [note figée de perte du banc spike02](PERTE-SPIKE02-NOTE.md) ; la conservation historique du chemin absent est levée, les builds natifs restent suspendus et les DLL installées restent la référence.

Cette chaîne versionnée protège les entrées de compilation ggml/NeMo du produit Windows. Elle ne remplace pas les DLL installées et n'est pas un installeur. La qualification exige surfaces PE, sorties fonctionnelles et répétabilité ; elle reste incomplète.

## Sources épinglées

| Composant | Dépôt et commit complet |
| --- | --- |
| NeMo-Speech.cpp, ASR C/C++ | https://github.com/NVIDIA/NeMo-Speech.cpp.git — `6a3ca369370782acee0dd155e34394ba60b920c4` |
| llama.cpp / ggml | https://github.com/ggml-org/llama.cpp.git — `bd4f514db14d87fded667787a7a963bfbaa98e89` |
| vcpkg | https://github.com/microsoft/vcpkg.git — `9e593bb18ea69cc5095e012465dcd675a822ed0d` |

Les sources NeMo et leurs **19 correctifs ggml/llama** sont versionnés dans le dépôt NVIDIA au commit ci-dessus, téléchargés dans `native/cache/` ; ils ne sont pas vendored dans ce dépôt. Le correctif local `native/patches/ggml-pinned-build-commit.patch` est versionné ici et respecte le commit llama fourni par CMake, sans hériter du commit Git de Whisper. Les pins/options sont dans `native/build-config.json`.

Les DLL installées provenaient d'un ancien spike dont le chemin a disparu. Les pins retrouvés et surfaces compatibles ne prouvent pas à eux seuls l'identité de tous ses paramètres historiques. Aucune identité binaire avec les DLL installées n'est revendiquée.

## Outils et options

| Outil | Version contrôlée |
| --- | --- |
| MSVC / cl | toolset 14.44.35207 / 19.44.35229 |
| Windows SDK | 10.0.26100.0 |
| CMake | 4.4.4 |
| Ninja principal | 1.12.1 |
| CUDA / nvcc (GPU) | 12.9 / 12.9.86 |
| vcpkg-tool | release 2026-07-27, SHA-256 `13b8175e99a884c5ad34249218754b45541a1a63f216e92603aee57a285ac741` |

vcpkg utilise aussi ses outils internes épinglés par ses sources ; Ninja 1.13.2 a été observé dans la construction de Protobuf. Ne pas confondre ce générateur avec le Ninja principal. Le script refuse les écarts des versions qu'il contrôle ; aucune installation ou mise à jour automatique des outils système.

Configuration Release, runtime MSVC `MultiThreadedDLL` (`/MD`), triplet `x64-windows-static-md`, ASR seul, CPU ou CUDA selon la variante. `GGML_NATIVE=ON` conserve le choix existant : risque de dépendance au processeur de compilation, à traiter séparément. GPU : architectures CUDA `61`. Les autres options et valeurs CMake effectives sont consignées dans chaque manifeste.

`/Brepro` est transmis par les variables `*_FLAGS_INIT` pour conserver les défauts CMake, dont `/EHsc`. Les contradictions ou l'absence des flags requis sont refusées ; tout diagnostic **C4530** arrête la construction. Les commandes de compilation et de lien sont conservées. Ces réglages ne constituent pas une preuve de répétabilité.

## Compiler, une variante par commande

Depuis le worktree, PowerShell 7, après préflight et autorisation du lot applicable :

```powershell
pwsh -NoProfile -File ./ci/build-native.ps1 -Variant cpu -BuildId identifiant-cpu-neuf
pwsh -NoProfile -File ./ci/build-native.ps1 -Variant gpu -BuildId identifiant-gpu-neuf
```

Chaque ligne est une invocation indépendante. Ne pas lancer les deux simultanément. Le point d'entrée public appelle la garde par chemin absolu ; son enfant vérifie parent réel et SHA du script. Une invocation directe relative de la garde échoue à ce contrôle.

Sources, dépendances, profils, temporaires et build sont matérialisés au chemin physique fixe `C:/wb/native/<cpu|gpu>`. Le précédent espace appartenant à un propriétaire inactif est préservé sous les preuves de garde, puis recréé neuf. Les liens sont refusés ; l'unique normalisation INetCache Windows admise est contrôlée par type/cible/absence d'autres liens, reçus avant/après et empreintes de la cible, avec retrait non récursif du seul lien. Aucun contenu n'est supprimé par cette préparation.

Téléchargements/sources persistants : `native/cache/`, hors de target. Les anciens caches binaires à chemins inconnus ne sont pas réutilisés ; le candidat actuel reconstruit ses dépendances. DLL copiées sous `target/native-build/<variante>/<id>/dll/`, manifeste avec pins/versions/options/flags/SHA ; journaux de garde sous `target/native-build/guard/`. Le getter PE `ggml_commit` est décodé sans charger la DLL et doit égaler le pin llama complet.

La garde surveille les racines prévues, capture sorties/codes et arrête sur échec/violation. Le collecteur `ReadDirectoryChangesExW` assemble un seul drainage, par session/volume/FileId, sans retenir une moitié de renommage pour une échéance ultérieure. Orphelins, ambiguïtés, pertes et erreurs restent bloquants. La politique particulière des événements Temp non attribués est celle d'AGENTS.md. `VSCMD_SKIP_SENDTELEMETRY=1` supprime la branche VsDevCmd concernée ; il ne garantit pas l'absence de télémétrie interne du compilateur.

## Vérifier avant de qualifier

1. Deux constructions successives neuves, même code/options/chemin fixe : SHA des cinq DLL CPU, ou six ggml/NeMo GPU, identiques.
2. Exports nom+ordinal, imports DLL+symboles directs/différés identiques aux DLL installées ; contrôle PE CPU sans CUDA. Expliquer les écarts de taille >2 % et comparer les options effectives.
3. Runtime alternatif épinglé, seules DLL ggml/NeMo remplacées, désigné par `WHISPER_N2_PERSONAL_RUNTIME_ROOT` ; l'installation reste intacte. Comparer TXT/SRT/WAV par SHA et assertions N2, puis nombres/résultats N1 à main.
4. GPU : les trois DLL CUDA redistribuées sont copiées du toolkit attendu, non recompilées ; le script vérifie leurs SHA contre l'inventaire versionné `docs/architecture/raw/native-build/installed-inventory.json`.

Tests préparatoires sans compilation de DLL : `ci/build-native.ps1 -SelfTest`, puis garde `NativeDllCpu` en mode `SelfTest`. Les tests de notifications/contrôle UTF-8 sont versionnés sous `ci/test-native-*`. Les profils et pins de référence N2 sont des dépendances des contrôles de garde ; aucun résultat fonctionnel n'en découle.

Dernier état connu : cpu7c4 PASS, cinq surfaces PE conformes sans CUDA, commit llama décodé correct. cpu8 a échoué dans Protobuf hôte Release, crash interne MSVC C1001 dans c2.dll ; répétabilité, N1/N2 et GPU restent à qualifier. Une erreur de compilation n'autorise aucune relance automatique.

## Licences et contrainte de chargement

NeMo-Speech.cpp : [Apache-2.0](https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/6a3ca369370782acee0dd155e34394ba60b920c4/LICENSE). llama.cpp/ggml : [MIT](https://raw.githubusercontent.com/ggml-org/llama.cpp/bd4f514db14d87fded667787a7a963bfbaa98e89/LICENSE). Préserver les notices des correctifs et dépendances tierces. CUDA : licence NVIDIA et [EULA 12.9.1](https://docs.nvidia.com/cuda/archive/12.9.1/eula/index.html), dont les conditions de redistribution ; leur identification n'est pas une validation juridique du paquet.

Contrainte connue : crash GGML_ASSERT rapporté lorsqu'un module ggml est chargé puis rechargé dans un même processus après FreeLibrary. Les bancs conservaient les DLL jusqu'à la sortie du worker. La destruction d'un recognizer ne prouve pas qu'un déchargement/rechargement soit sûr ; ce défaut n'est pas corrigé par la chaîne de compilation.

La fusion de protection éventuelle conserve uniquement la chaîne et ses contrôles. Remplacement des DLL, suppression du spike, mise à jour de versions et changement d'ISA restent des décisions distinctes de l'utilisateur.
