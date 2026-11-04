Avancée : inventaire de phase 1 effectué le 4 novembre 2026 ; pins, lanceurs historiques et 14 DLL identifiés.
Blocage : sauvegarde spike02 impossible ; version Ninja historique et options/flags CMake complets encore inconnus.
Décision : NON REPRODUIT ; préparer la phase 2 à partir de la provenance retrouvée, après arbitrage explicite des inconnues.

# BUILD-NATIF — clôture de l'inventaire, complément au bilan phase 0

L'utilisateur a demandé de passer à l'étape suivante après la recherche infructueuse dans le profil. L'inventaire a repris sans sauvegarde préalable possible ; aucune reconstruction autorisée par déduction de cette reprise. [Inventaire détaillé et preuves](../architecture/NATIVE-BUILD.md).

| Composant / paramètre | Résultat | Preuve |
| --- | --- | --- |
| NeMo-Speech.cpp v0.2.0 | `6a3ca369370782acee0dd155e34394ba60b920c4` | history-extracts.txt:15, :479 |
| llama.cpp | `bd4f514db14d87fded667787a7a963bfbaa98e89` | history-extracts.txt:458, :481 |
| vcpkg | `9e593bb18ea69cc5095e012465dcd675a822ed0d` | history-extracts.txt:15, :482 |
| Patches NeMo | 19, copie ggml sous `_deps` | history-extracts.txt:496 et série amont fixée |
| Construction | Release, Ninja, MSVC toolset 14.44.35207, compilateur 19.44.35229 ; UCRT SDK 10.0.26100.0 | Cache/logs historiques conservés ; version Ninja ancienne inconnue |
| CUDA / runtime MSVC | nvcc 12.9.86, arch 61 ; dynamique compatible /MD | Cache et imports PE ; flags complets absents |
| Portabilité CPU | AVX2/FMA/F16C attestés dans la variante CUDA ; GGML_NATIVE exact inconnu | history-extracts.txt:508 ; ne pas extrapoler la trace à la variante CPU |
| DLL CUDA | Trois copies exactes du toolkit 12.9 local | SHA-256 comparés, installed-pe.txt |
| Licences | NeMo Apache-2.0, llama/ggml MIT, CUDA EULA NVIDIA | Fichiers amont fixés et EULA locale ; conformité de redistribution non qualifiée |

**Vérifications :** 14 DLL inventoriées (SHA-256, taille, version Win32 si présente, en-têtes, exports et imports complets). Les 14 empreintes correspondent au script d'installation versionné. Absence de modification des DLL pendant inspection contrôlée par SHA avant/après. Contrôle PE CPU : 5/5 PASS ; auto-tests permanents PE : 4 positifs/15 négatifs PASS, témoin réel facultatif NON EXÉCUTÉ. Outils exécutés : lectures, décompression en mémoire des chats historiques, dumpbin, cuobjdump en lecture, requêtes de version, contrôle PE et Git. Aucune compilation, aucun N1 global ou N2 ; aucune DLL chargée pour inférence.

**Limites :** les chats compressés contenaient des extraits des lanceurs et logs, pas une sauvegarde des sources ni les caches complets. L'absence de modifications aux contrôles Git historiques ne prouve pas l'absence de tout changement ultérieur dans `_deps`. Archive phase 0 toujours NON CRÉÉE ; équivalence statique/fonctionnelle NON EXÉCUTÉE. Aucun nettoyage, fusion, push ou modification Rust/installation.

Revue indépendante en lecture seule : **CLEAN pour l'inventaire de phase 1**, sans validation du script ou de l'équivalence. Cohérence des preuves contrôlée : 14 entrées JSON, 14 sections PE et 14 SHA concordants ; `git diff --check` réussi.

**Décision de périmètre à prendre :** (A, recommandé) poursuivre la préparation du script sur les trois révisions retrouvées et les 19 patches amont, accepter que Ninja 1.12.1 et les valeurs amont non retrouvées soient des hypothèses explicites à éprouver par équivalence ; (B) suspendre jusqu'à récupération des caches/logs complets historiques. Ne pas confondre A avec une preuve préalable d'options identiques. Architecture proposée : `ci/build-native.ps1`, pins sous `native/`, copies Git épinglées dans `native/cache/` persistant ignoré, builds dans `target/native-build/{cpu,gpu}/`. Le mécanisme Git des patches motive la révision du choix provisoire d'archive. Conservation du spike et interdiction de remplacer les DLL installées maintenues.
