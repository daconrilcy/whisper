Avancée : les imports PE statiques des DLL Parakeet et leurs prérequis Visual C++ ont été inspectés en lecture seule.
Blocage : le runtime personnel historique ne contient pas les DLL CRT/OpenMP nécessaires dans son inventaire ; leur résolution sur poste propre n’est pas démontrée.
Décision : inclure cette fermeture dans P1/P4 ; aucune copie, installation ni reconstruction n’a été réalisée.

# Constat P1 — fermeture du runtime Parakeet Windows

Date : 19 novembre 2026. Sources : les 21 chemins du `runtime-inventory.json` personnel, vérifiés présents et conformes en taille/SHA ; `dumpbin /dependents` en lecture seule ; fichiers Redistributable x64 locaux Visual Studio 2022. Ce relevé décrit ces fichiers historiques, pas le candidat public.

## Imports directs observés

Les DLL `native/cpu/ggml-base.dll` et `ggml-cpu.dll`, ainsi que leurs équivalents GPU, importent `VCOMP140.DLL`, `MSVCP140.dll`, `VCRUNTIME140.dll` et `VCRUNTIME140_1.dll`. `nemo_speech_asr.dll`, `nemo_speech_asr_c.dll`, `ggml.dll` et `ggml-cuda.dll` importent aussi des fichiers de cette famille CRT. Les imports Windows comme `KERNEL32.dll`, les API sets et `dbghelp.dll` sont distincts de ces redistribuables. Le pilote `nvcuda.dll` reste externe. `dumpbin /dependents` montre les imports statiques inspectés ; imports delay-load, appels `LoadLibrary` et modules chargés à l’exécution ne sont pas fermés par ce seul relevé.

Les 21 fichiers du runtime personnel ne comprennent pas les quatre DLL CRT/OpenMP. Le profil de développement de cette machine dispose d’un Visual Studio Build Tools ; les essais historiques sur ce banc ne démontrent donc pas que ces dépendances sont résolues sur un Windows propre. La racine durable `%LOCALAPPDATA%\WhisperTurbo` contient actuellement trois DLL CRT dans `app/`, mais aucun worker ou runtime Parakeet ; le répertoire de l’EXE Parakeet et les règles `LoadLibrary` doivent être pris en compte, car le chargeur ne recherche pas automatiquement un dossier parent arbitraire.

## Redistribuables Microsoft x64 relevés

| Fichier | Version | Octets | SHA-256 | Source x64 locale | Authenticode |
|---|---:|---:|---|---|---|
| `msvcp140.dll` | 14.44.35211.0 | 557 728 | `0f885b509a685d2bbfa652fed26b5fb31d88fbdab0a978c641d1c7b8aa460aa9` | `VC/Redist/MSVC/14.44.35112/x64/Microsoft.VC143.CRT/` | Valide, Microsoft |
| `vcruntime140.dll` | 14.44.35211.0 | 124 544 | `d5e4d9a3e835fa679450145d6a7d94e36573a509317111904d9b3712c30d9066` | même dossier CRT | Valide, Microsoft |
| `vcruntime140_1.dll` | 14.44.35211.0 | 49 792 | `1f2d41c4aa5db0bc33ebf7b66d72943a817d7ce6cbe880502a9403823633093f` | même dossier CRT | Valide, Microsoft |
| `vcomp140.dll` | 14.44.35211.0 | 193 152 | `55aba23cdcd6484fbb06f4155b8ca75adfce7a881f10afd0c49457165e677164` | `VC/Redist/MSVC/14.44.35112/x64/Microsoft.VC143.OpenMP/` | Valide, Microsoft |

Ces quatre copies sont candidates seulement. Microsoft limite la redistribution aux fichiers permis par les termes d’une licence Visual Studio applicable, sans modification et hors répertoires exclus ; la licence Community déclarée par l’utilisateur ne remplace pas l’examen des termes pour le distributeur et son mode de remise. Sources : [Redistribution Visual Studio 2022](https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution), [redistribution des fichiers Visual C++](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files?view=msvc-170), [liste des DLL requises](https://learn.microsoft.com/en-gb/cpp/windows/determining-which-dlls-to-redistribute?view=msvc-160).

**Contrat proposé pour P3/P4 :** rendre les quatre fichiers disponibles à chaque processus Parakeet sans dépendre d’une installation VC préexistante. Une option compatible avec la topologie `worker/` + `native/cpu|gpu/` est de placer des copies locales des quatre fichiers à côté des EXE Parakeet, en complément des DLL moteur sous leurs dossiers natifs privés ; une autre est d’établir et tester explicitement un chemin de recherche privé approprié. Avec les indicateurs observés dans `LoadLibraryEx`, la recherche inclut le dossier du DLL chargé et le dossier de l’exécutable, pas un parent arbitraire ; voir l’ordre documenté par [Microsoft](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order). Aucun changement de code, API Windows ni manifeste n’est décidé ici. La recette d’installation propre doit confirmer l’absence des messages DLL, l’image effectivement chargée, les versions et les empreintes.

## CUDA : versions à distinguer

Les ressources GPU du runtime personnel sont `cublas64_12.dll` et `cublasLt64_12.dll`, version de fichier `6.14.11.1292`, ainsi que `cudart64_12.dll`, version `6.14.11.12090`. Leurs SHA concordent avec l’inventaire personnel. Les trois fichiers sont `NotSigned` Authenticode ; leurs ressources de version et leurs noms ne prouvent pas à eux seuls leur provenance. La référence EULA doit suivre les fichiers exacts : ne pas faire passer une page CUDA 12.9.2 pour une vérification du runtime `cudart` 12.9.0. Examiner et associer les conditions et notices des versions pertinentes avant la remise. Références : [EULA CUDA 12.9.0](https://docs.nvidia.com/cuda/archive/12.9.0/eula/index.html) et [EULA CUDA 12.9.2](https://docs.nvidia.com/cuda/archive/12.9.2/eula/index.html).

## Conséquence sur P1/P2

1. Le manifeste final devra inclure `VCOMP140.dll` en plus des trois DLL CRT déjà retenues pour Turbo, si ce profil de build Parakeet est conservé.
2. La redistribution VC doit être vérifiée pour les quatre fichiers et leurs emplacements exacts ; ne pas conclure à l’autonomie à partir de l’inventaire de 21 fichiers.
3. Les notices NeMo/ggml, CUDA et Microsoft restent à relier à chaque binaire final. Les imports PE ne déterminent pas quels projets ou avis de droit d’auteur sont incorporés dans les DLL.
4. Le build Parakeet reste suspendu au titre de la décision WHEA. Ce constat ne lance ni build ni test matériel et ne rend pas nécessaire une reconstruction tant que les artefacts existants peuvent être identifiés et qualifiés autrement.
