Avancée : l’étude statique MSIX est terminée ; la séparation CPU/NVIDIA en deux fiches est conservée.
Blocage : les actifs en lecture seule et le prérequis OpenGL Snapdragon ne sont pas raccordés au modèle d’exécution MSIX.
Décision : MSIX paraît faisable sous conditions ; autoriser ensuite un prototype d’intégration ciblé avant tout lot de production.

# Étude de compatibilité MSIX — WhisperTurbo — 22 novembre 2026

## Portée

Cette étude confronte les sources et les deux candidats P7 aux exigences Microsoft documentées. Elle est en lecture seule : aucun manifeste, binaire, package ou pipeline MSIX n’a été créé ni modifié, et aucun essai n’a été exécuté. Les deux fiches Store CPU et NVIDIA restent le choix accepté ; AMD reste différé. La note de préparation Store et la clôture P7/P6 demeurent les références de décision et de qualification.

**Limite de preuve :** l’étude de code a été faite sur le worktree `codex/cpu-distribution-l2-reprise`, dont de nombreux fichiers produit sont modifiés ou non suivis par rapport à HEAD `b4769e85`. Elle décrit les sources présentes, sans les attribuer au build P7 et sans établir l’équivalence entre ces sources et les ZIP P7.

## Constats techniques

### Actifs et données à séparer

Le bootstrap actuel installe les ressources depuis le dossier de l’exécutable vers `%LOCALAPPDATA%\WhisperTurbo`. Le desktop dual reconstruit depuis ce même chemin les chemins des modèles, workers, paramètres et journaux (`crates/whisper-desktop/src/root/distribution_edition.rs` et `root.rs`). Les préférences et chemins sont écrits dans `settings/settings.json`; les journaux GPU et de réactivité vont sous `logs/`. Le runtime Parakeet valide ses modèles, workers et DLL à l’intérieur de sa racine installée.

Les imports et captures produisent leurs archives sous le dossier de transcription choisi par l’utilisateur (`transcriptions/`); TXT/SRT et WAV restent dans cet emplacement utilisateur. Ces écritures hors du paquet sont cohérentes avec l’emplacement de données MSIX, sous réserve de préserver l’accès aux fichiers choisis dans les dialogues Windows.

Dans un MSIX, les fichiers de l’application sont installés dans un emplacement protégé et ne peuvent pas être modifiés par l’application. Les répertoires de données propres à l’application sont distincts. Microsoft décrit `ApplicationData.LocalFolder` / `%LOCALAPPDATA%\Packages\<PFN>\LocalState` pour les données qui doivent persister. Le modèle courant ne peut donc pas simplement continuer à chercher les modèles, workers et runtimes sous `%LOCALAPPDATA%\WhisperTurbo` : cette racine n’est pas l’emplacement des fichiers MSIX.

**Conséquence d’architecture :** les binaires, modèles, DLL et notices doivent être résolus depuis un emplacement de package en lecture seule; paramètres, journaux et archives gérées par l’application doivent rester dans des emplacements inscriptibles. Les sorties audio choisies par l’utilisateur doivent conserver leur destination explicite. Aucun changement de ces chemins n’est inclus dans cette étude.

### Fichiers natifs, deux éditions et prérequis Windows

Le worker Parakeet charge `nemo_speech_asr_c.dll` par chemin absolu dans `native/cpu` ou `native/gpu`, avec les drapeaux de recherche Windows `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS`. Les DLL CUDA sont privées à l’édition NVIDIA et doivent rester résolues depuis son package. Microsoft documente un ordre de recherche propre aux applications empaquetées et le rôle du graphe de dépendances du package. Cela rend le scénario plausible, mais ne prouve pas que la hiérarchie exacte des DLL et dépendances transitives P7 sera chargée correctement dans le package; un essai MSIX doit le démontrer pour CPU et NVIDIA.

Les éditions P7 sont x64 et séparées. Deux packages MSIX, chacun pour une fiche Store, respecteraient cette séparation. La taille maximale publiée par Microsoft est de 25 Go par package MSIX; les ZIP P7 sont sous cette limite, mais leur taille MSIX finale et la taille installée restent à mesurer pendant l’assemblage.

Le bootstrap actuel vérifie le rendu Glow avant d’installer et guide l’utilisateur vers le Microsoft Store lorsque le pack de compatibilité OpenGL ARM64 manque. Dans le flux MSIX, l’installation du package précède le premier lancement; le démarrage graphique pourrait échouer avant que cette interface de prérequis soit disponible. La documentation consultée ne permet pas d’établir que la dépendance `Microsoft.D3DMappingLayers` peut être déclarée, installée ou réparée automatiquement comme dépendance Store de ces deux fiches. **C’est un point bloquant à trancher avant d’affirmer la compatibilité MSIX sur Snapdragon.** La recette déjà réussie après installation manuelle du pack ne prouve pas le parcours MSIX sans ce pack.

Le logiciel capture le microphone par WASAPI. Le package doit préserver ce parcours et le comportement de consentement/confidentialité de Windows; le manifeste et le niveau de confiance du package ne sont pas encore définis. Les documents Microsoft indiquent qu’un manifeste peut déclarer la capacité microphone et que les contrôles diffèrent selon l’exécution en conteneur AppContainer ou en confiance complète. Le prototype doit fixer explicitement ce choix, vérifier l’accès au microphone et éviter de demander une capacité non utilisée.

## Évaluation

| Domaine | État | Preuve ou condition restante |
|---|---|---|
| Deux fiches Store CPU/NVIDIA | Compatible en principe | Deux packages x64 séparés, sans sélection Store par GPU. |
| Actifs dans un emplacement MSIX protégé | Adaptation requise | Remplacer la résolution actuelle `%LOCALAPPDATA%\WhisperTurbo` par une racine package pour modèles, workers, notices et DLL. |
| Données utilisateur | Adaptation requise | Séparer paramètres/journaux du package et garantir leur persistance au fil des mises à jour. |
| CUDA et DLL natives privées | Non démontré | Smoke test de chargement, dépendances et routage sur le package NVIDIA. |
| Microphone et accès aux fichiers choisis | À confirmer au manifeste | Définir le niveau de confiance et vérifier le consentement microphone ainsi que les dialogues de fichiers. |
| Snapdragon sans pack OpenGL | Bloquant non résolu | Démontrer un parcours de prérequis avant lancement graphique, ou établir le minimum matériel/logiciel pris en charge par la fiche. |
| Taille et format Store | Aucun plafond identifié à ce stade | Mesurer les deux MSIX finaux; maximum documenté 25 Go par package. |
| Package / pipeline existant | Aucun trouvé dans les chemins examinés | Aucun manifeste `Package.appxmanifest`, MSIX, ni étape d’assemblage associée trouvé sous `crates/`, `ci/` et `docs/` hors archives historiques. |

**Conclusion :** l’empaquetage MSIX n’est pas compatible « tel quel », car l’application suppose que ses ressources distribuées résident dans son arbre `%LOCALAPPDATA%\WhisperTurbo`. Une adaptation d’accès aux ressources et aux données semble techniquement possible, mais deux questions nécessitent une preuve réelle : le chargement privé des DLL natives/CUDA depuis le package et le prérequis OpenGL avant le premier rendu sur Snapdragon. Les documents Microsoft ne suffisent pas à clore ces points.

## Suite recommandée — prototype de faisabilité à autoriser séparément

Préparer un prototype minimal pour l’édition CPU d’abord, puis un contrôle ciblé de l’édition NVIDIA si le chemin package fonctionne. Le prototype devrait :

1. lire le modèle et le worker CPU depuis l’emplacement package en lecture seule;
2. écrire et relire les préférences et journaux dans l’emplacement de données MSIX;
3. ouvrir un WAV choisi par l’utilisateur, exécuter un import CPU, puis vérifier les exports à leur destination choisie;
4. démarrer le bureau et obtenir le consentement microphone, sans refaire de qualification de qualité ou de vitesse;
5. pour NVIDIA, vérifier la résolution effective des DLL privées et l’identité du worker, sans essai de transcription supplémentaire si le chargement et l’admission sont démontrés;
6. sur Snapdragon sans pack, vérifier si l’utilisateur reçoit une explication et un chemin réparable avant l’échec du moteur graphique; puis vérifier le lancement après installation du pack. Si le prérequis ne peut être contrôlé avant activation MSIX, arrêter ce prototype avec un constat `non compatible pour ce scénario` et proposer une alternative produit.

Le lot devrait être isolé dans un worktree court, sans toucher aux ZIP P7 ni aux bilans figés, et préciser avant exécution les fichiers autorisés, le manifeste/capacités proposés, les commandes, les preuves attendues et la limite de l’essai. Aucun prototype, build, téléversement Partner Center ou publication n’est autorisé par la présente étude.

## Références Microsoft

- [MSIX containerization overview](https://learn.microsoft.com/en-us/windows/msix/msix-containerization-overview) — emplacement protégé du package, distinction entre fichiers installés et données.
- [MSIX troubleshooting guide](https://learn.microsoft.com/en-us/windows/msix/msix-troubleshooting-guide) — `LocalState` et conséquences des écritures virtualisées.
- [App capability declarations](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations) et [Enable device capabilities](https://learn.microsoft.com/en-us/windows/apps/develop/devices-sensors/enable-device-capabilities) — capacités, microphone et distinctions AppContainer/desktop empaqueté.
- [Dynamic-link library search order](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order) — résolution des dépendances DLL des applications empaquetées.
- [App package requirements](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/package-version-numbering?pivots=store-installer-msix) — taille MSIX maximale de 25 Go.
- [Prepare to package a desktop application](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-prepare) — compatibilité et limitations des applications Win32 empaquetées.
