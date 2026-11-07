Avancée : CPU et GPU QUALIFIÉS sur ce banc Windows, en anglais et en français.
Blocage : aucun critère ouvert.
Décision : conserver les 14 DLL installées. Leur remplacement reste une décision humaine distincte.

# BUILD-NATIF-QUALIF — clôture GPU du 6 novembre 2026

| Critère | CPU r2 | GPU r5 |
|---|---:|---:|
| a. Surfaces PE identiques aux DLL installées | 5/5 | 9/9 |
| c. Deux builds propres au même chemin, SHA-256 identiques | 5/5 | 9/9 |
| Déterminisme préalable de la référence, TXT/SRT/WAV EN/FR | 6/6 | 6/6 |
| b. Référence contre runtime reconstruit, mêmes sorties | 6/6 | 6/6 |
| **Verdict** | **QUALIFIÉ** | **QUALIFIÉ** |

**Origine de l’écart r4.** Sur 146 objets CUDA, trois instances d’attention (64, 128 et 256) portaient des noms internes variables dans le PTX embarqué. Les cubins étaient identiques. La substitution des seuls préfixes de symboles rend les trois PTX identiques (7, 14 et 14 occurrences). Trois autres objets contenaient des symboles lambda MSVC variables. La simulation nvcc montre que le nom du fichier C++ transmis à MSVC dépendait du PID. [Comparaison des objets](C:/wb/native-qualif/reprise-5/diagnostic/comparison.json) et [commandes nvcc simulées](C:/wb/native-qualif/reprise-5/diagnostic/nvcc-dryrun.json).

**Correction et preuve.** Le patch CMake GPU, le pilote et ses auto-tests fixent une graine nvcc par source et placent les temporaires auprès de chaque objet. Correctif et routage r5 relus indépendamment, commit `8337ba64`. Les deux builds à `C:/wb/native/gpu` ont duré 13 min 35 s et 13 min 34 s. Build 1 archivé avant le second. Les 146 objets CUDA sont identiques octet par octet. SHA-256 commun de `ggml-cuda.dll` : `F1BBF581B43FC334A2E6A11A85DC0F9DD22A0FB0A38AF7FD6E4977C731748AD6`. Les deux séries Reference complètes ont précédé Rebuilt, avec CUDA0 attesté pour les deux langues et comparaison des octets bruts. [Surfaces](C:/wb/native-qualif/reprise-5/gpu/surface.json), [SHA des neuf DLL](C:/wb/native-qualif/reprise-5/gpu/repeat.json), [Reference ×2](C:/wb/native-qualif/reprise-5/gpu/reference-determinism.json), [équivalence](C:/wb/native-qualif/reprise-5/gpu/equivalence.json), [CPU r2](C:/wb/native-qualif/reprise-2/cpu/verdict.json).

| Surveillance r5, dans l’ordre indiqué | WHEA avant → après, chaque étape | Pic engagé par étape |
|---|---|---|
| Auto-test / GPU1 / surfaces / GPU2 / répétabilité | 1→1 chacune | 20,5 / 21,1 / 20,7 / 21,0 / 20,7 % |
| Reference interrompue / Reference1 / Reference2 / Rebuilt | 1→1 chacune | 20,2 / 20,2 / 20,1 / 20,2 % |

Le premier essai Reference a été interrompu sur deux notifications de répertoire pendant l’archivage parallèle des auto-tests, sans PID attribuable. L’anglais avait réussi. L’essai et son STOP ont été conservés. Reprise autorisée, archivage séquencé, garde active. Aucun nouvel événement WHEA ni ICE. Dernière WHEA historique 527992. Les SHA-256 des 14 DLL installées correspondent toujours à l’inventaire. `GGML_NATIVE=ON`, backend CPU AVX2/FMA/F16C, CUDA compute_61/sm_61. Portabilité à qualifier séparément. [Mesures et reçus](C:/wb/native-qualif/reprise-5/gpu/verdict.json) · [Tailles et commandes de suppression humaine](C:/wb/native-qualif/reprise-5/RETENTION.md) · [Plan](../cadrage/plan-build-natif-qualif-ptx.md).

[Fusion et suivi du nettoyage](BUILD-NATIF-QUALIF-FUSION-NETTOYAGE.md).
