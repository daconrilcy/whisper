Avancée : installation canonique publiée ; SHA de 21 fichiers concordants et contrôle PE des cinq EXE installés réussi ; recette isolée préparée.
Blocage : Windows Sandbox activé avec autorisation, redémarrage Windows nécessaire ; qualification CPU sans CUDA NOT RUN.
Décision : aucun redémarrage automatique ni worker exécuté sur l'hôte ; attendre le redémarrage utilisateur avant la recette.

# Livraison et qualification R8 — six tâches

1. Préflight en lecture seule : Git, espace libre, SHA des deux modèles et quatorze DLL, quatre workers et identités Parakeet admises par le registre. Préserver les preuves R8-FIX et toutes les sources historiques.
2. Construire le desktop release `personal-install` avec `ci/guard-level1.ps1`, hors ligne/verrouillé, racines isolées sous `target/r8-install/`, cible fraîche distincte. Vérifier PE et enregistrer l'attestation exacte requise par l'installeur. Aucun worker/modèle lancé sur l'hôte.
3. Préparer les sources du script canonique sous le dépôt. Accord utilisateur reçu : « Oui, conserver Parakeet qualifié ». Adapter uniquement `scripts/install-personal.ps1` pour sélectionner explicitement les deux workers Parakeet déjà qualifiés, vérifier leurs SHA contre les constantes actuelles du registre, sans modifier celui-ci. Sources retrouvées : `C:/dev/whisper/target/parakeet-tests/refacto-parakeet-live/p3-workers/localappdata/Whisper/worker/` ; CPU `a965c9503f909124f8ddedad09e83e5d187e70edd1869e880e9f1ad5ae2fb99d`, GPU `7ffb0b4a022ec26909a8939b37787de6df6c9bdb2903edf14d3ffb32579eefc1`. Les binaires et preuves R8-FIX restent intacts. Implémentation de cette adaptation déléguée au même rust_implementer, revue indépendante ensuite, budget propre de deux cycles pour ce livrable. Aucun worker/modèle exécuté sur l'hôte.
4. Exécuter `scripts/install-personal.ps1` pour staging puis publication d'un candidat neuf sous le véritable `%LOCALAPPDATA%/Whisper`. Préserver sa récupération/sauvegarde. Relever et comparer les SHA, puis exécuter `ci/check-worker-imports.ps1` sur les cinq EXE effectivement installés. L'autorisation précise de préparation puis installation a été donnée dans la session présente.
5. Recette Windows Sandbox : autorisation précise reçue pour création et lancement, GPU virtuel et réseau désactivés, entrées en lecture seule, résultats sous le dépôt. Vérifier dans l'invité l'absence réelle de DLL CUDA avant essai d'une courte transcription CPU avec les copies des fichiers installés. Toute activation du composant Windows, élévation ou redémarrage requiert un accord supplémentaire. Ne modifier ni pilote ni DLL CUDA de l'hôte. La fenêtre produit reste réservée à l'utilisateur.
6. Bilan dans le présent document et preuves compactes sous `docs/architecture/raw/r8-install/` : distinguer installé/SHA/PE, environnement invité et inférence, tests non exécutés, coûts et limites. Git local seulement, aucun push.

Autorisations reçues : « Oui, préparer puis installer » (desktop gardé, ressources locales, attestations de livraison, installation et contrôle des fichiers installés) ; « Oui, autoriser la recette isolée » (sans activation système implicite). Arrêt sur échec ou prérequis divergent avant installation ; aucune acquisition ni nouvelle dépendance, aucun changement de code produit, manifeste, Cargo.lock ou garde historique.

## Préparation exécutée

Les quatorze DLL et deux modèles ont passé les comparaisons SHA et ont été copiés sous les sources attendues dans ce worktree. Desktop personnel construit sous garde (Cargo code 0, 65 s), PE sans CUDA, SHA `aa19791bac66c4c31511bcaf209baa16d38168d27e7b737a8f8ddf925302328d`. Le contrôle de retour de l'appelant a signalé à tort un échec après la réussite ; aucune recompilation, résultat attribué au journal `N1_GUARD exit_code=0` et au contrôle PE.

Adaptation du script canonique déléguée puis revue indépendante CLEAN, sans correction, sur le delta depuis `f1a92cbf` : option explicite `-QualifiedParakeetSourceDirectory`, préflight avant mutation, SHA égaux au registre, gates PE et récupération conservés. Sandbox : composant initialement désactivé (InstallState 2) ; accord explicite utilisateur pour `Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM -All -NoRestart` avec élévation. Activation exécutée avec succès ; `RestartNeeded=true`. Aucun redémarrage ni lancement de Sandbox.

## Installation exécutée et contrôle des copies installées

Script canonique depuis `C:/dev/whisper/.wt/r8`, candidat `r8-install-20261101-a` : `-StageOnly -QualifiedParakeetSourceDirectory C:/dev/whisper/target/parakeet-tests/refacto-parakeet-live/p3-workers/localappdata/Whisper/worker`, puis `-Publish`, codes 0. Installation réelle `C:/Users/cyril/AppData/Local/Whisper` ; sauvegarde antérieure conservée sous `backups/before-r8-install-20261101-a`. Aucun worker ou desktop lancé.

| EXE installé | SHA-256 | Contrôle PE sur le fichier installé |
| --- | --- | --- |
| `app/whisper-desktop.exe` | `aa19791bac66c4c31511bcaf209baa16d38168d27e7b737a8f8ddf925302328d` | PASS, sans CUDA |
| `app/whisper-worker-cpu.exe` | `eb33d96b57ee47e5dcbe3238467df4f5656d055757374dbff5d58a5f751bbfef` | PASS, sans CUDA |
| `app/whisper-worker-gpu.exe` | `5ff4c494b86754d563c5d41cc7cdcd1b2748aed9c9ffa5f929be29973cf6001b` | PASS, cudart et nvcuda présents |
| `worker/whisper-worker-parakeet-cpu.exe` | `a965c9503f909124f8ddedad09e83e5d187e70edd1869e880e9f1ad5ae2fb99d` | PASS, sans import statique CUDA |
| `worker/whisper-worker-parakeet-gpu.exe` | `7ffb0b4a022ec26909a8939b37787de6df6c9bdb2903edf14d3ffb32579eefc1` | PASS, sans import statique CUDA |

Les cinq EXE, quatorze DLL et deux modèles installés ont tous été comparés aux sources vérifiées : **21/21 SHA concordants**, **5/5 contrôles PE réussis** (imports directs et différés). Aucun registre, source Rust, manifeste ou dépendance modifié. Les Parakeet R8-FIX reconstruits restent conservés dans leurs cibles ; les Parakeet effectivement installés sont les copies déjà qualifiées correspondant au registre.

Preuves : [résultats installés, chemins, SHA et listes complètes d'imports](../architecture/raw/r8-install/installed-results.json), [journaux et recette préparée](../architecture/raw/r8-install/installation-logs.zip). Archive de **19 entrées**, relues et comparées aux sources par taille et SHA, SHA-256 `9dcc8346e702107fbb4018860bcb2f2f3fdf751d39030f1335df47d56fe3565e`. Ressources locales conservées ; aucun nettoyage ni push.

## Recette après redémarrage — non exécutée

Configuration préparée : `C:/dev/whisper/.wt/r8/target/r8-install/cpu-no-cuda.wsb`. GPU virtuel, réseau et redirections désactivés ; 12 Gio mémoire ; entrées uniquement en lecture seule, sorties dans `target/r8-install/sandbox-output/`. Entrées : copie du CPU **installé**, modèle installé, trois DLL Visual C++ contrôlées sans CUDA, extrait PCM16 mono 16 kHz de 30 s (480 000 échantillons), issu de `sound example/spkr0.wav` sans modifier l'original.

Le script invité vérifie ses entrées/SHA, l'absence de DLL CUDA dans les racines de chargement contrôlées et l'absence de pilote NVIDIA avant une commande IPC CPU. Il exige Ready CPU, texte non vide, End à 480 000 échantillons et sortie 0 ; il consigne les modules chargés et refuse CUDA. Analyse syntaxique Windows PowerShell 5.1 et contraintes XML : PASS, **aucune exécution** de la recette. Aucun résultat d'inférence, absence CUDA réelle dans l'invité, repli Auto, export applicatif ou fenêtre Windows qualifié. Le redémarrage reste à l'initiative de l'utilisateur ; après celui-ci, reprendre ce plan à la tâche 5.
