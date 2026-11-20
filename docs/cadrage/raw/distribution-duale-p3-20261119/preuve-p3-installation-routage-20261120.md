Avancée : l’installation P3 sur profil vierge et le routage Parakeet CPU/GPU depuis le desktop sont démontrés sur le candidat épinglé.
Blocage : P4 (paquet de remise, manifeste complet et droits/notices), P5 (recette utilisateur du paquet exact sur les trois témoins) et P6 restent ouverts.
Décision : P3 est clos PASS pour la composition et le routage local ; le candidat ne devient pas pour autant prêt à distribuer.

# Preuve P3 — installation et routage dual — 20 novembre 2026

## Candidat et installation

- Candidat conservé sous target/parakeet-tests/distribution-duale-p3-20261119/candidate/.
- Installation bootstrapée dans le profil isolé target/parakeet-tests/distribution-duale-p3-20261119/scratch/install-p3-final-20261120/profile/WhisperTurbo/, avec un dossier de sortie dédié sous le même essai.
- Le profil neuf a affiché la confirmation « Prérequis satisfaits » ; l’utilisateur a confirmé l’installation. Le desktop installé s’est ouvert et a présenté les deux modèles.
- Candidat : 44 fichiers (43 ressources d’installation et le bootstrap). L’installation contient ces 43 ressources et des fichiers d’état créés par le produit au premier lancement.
- ci/build-dual-model-package.ps1 -Mode SelfTest : PASS (DUAL_PACKAGE_SELFTEST PASS).
- ci/build-dual-model-package.ps1 -Mode BuildBootstrapAndStage : code 0, puis CANDIDATE_ASSETS PASS. L’exécution Cargo associée a été faite par la garde.
- Aucune DLL native n’a été reconstruite pendant ce lot.

## Routage observé depuis l’interface

Les deux essais Parakeet ont été lancés par le desktop installé. Le dossier de sortie était vide au début du parcours. Le desktop a terminé chaque tâche et publié TXT/SRT non vides.

| Parcours | Job | Résultat d’exécution |
|---|---:|---|
| Parakeet CPU, WAV de référence 4 s | 1362404525521066102029896 | backend=cpu, 64000/64000 échantillons ; publication vérifiée |
| Parakeet CPU, WAV de référence 30 s | 1362404525521223755151196 | backend=cpu, 480000/480000 échantillons ; publication vérifiée |
| Parakeet GPU, même WAV 30 s | 1362404525521277405924096 | backend=cuda, 480000/480000 échantillons ; publication vérifiée |

Sources : docs/cadrage/feas-pm-01-corpus/fr_passage_28-32.wav (SHA-256 1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad) et docs/cadrage/feas-pm-01-corpus/fr_prefix_0-30.wav (SHA-256 dc86235c75c80c2a4243050e6b2f2b6cac60932ea5449eabc1b9419baa19703c).

Pendant l’exécution, les processus worker observés étaient les workers Parakeet CPU/GPU sous l’installation isolée, enfants du desktop. Le CPU a chargé ses DLL NeMo, ggml, CRT et OpenMP depuis WhisperTurbo/native/cpu/. Le GPU a chargé les composants NeMo, ggml, CRT, OpenMP, CUDA runtime/cuBLAS depuis WhisperTurbo/native/gpu/ ; nvcuda.dll provenait de C:\Windows\System32\, donc du pilote NVIDIA installé. Ces chemins ont été observés pendant les tâches actives ; aucun dump brut des modules de processus n’a été conservé.

Les exports ont les empreintes inscrites dans les manifestes de publication et la preuve compacte preuves-p3.zip. Les essais établissent le routage et la publication sur cette machine, pas une qualification de qualité comparative ni une recette des trois témoins.

## Limites et suites

P3 est PASS pour l’installation vierge et le routage privé CPU/GPU du candidat local sur la GTX 1080 Ti. P3 n’établit ni Auto pour les deux modèles, ni micro/MP3, ni compatibilité Dell/Snapdragon, ni droits de redistribution, ni préparation à la publication.

Étapes restantes selon le plan :

1. P4 — figer l’identité du candidat, établir le manifeste de remise (fichier, rôle, source/version, taille, SHA-256, licence/notice, chargement et destination), assembler l’archive, contrôler chaque entrée et fermer les conditions de redistribution VC Runtime/CUDA. Aucune reconstruction de DLL natives.
2. P5 — recette manuelle par l’utilisateur du seul paquet P4 sur Dell, GTX 1080 Ti et Snapdragon, dans la matrice et les limites du plan ; installer le pack Microsoft OpenGL ARM64 avant Snapdragon.
3. P6 — rattacher les résultats au SHA de l’archive, traiter les cellules non exécutées/échouées, consigner les limites puis prendre une décision distincte de distribution publique.

AMD, scénarios de panne Auto et anciens processeurs sans AVX restent exclus comme décidé. Les preuves historiques B7/tg7 ne qualifient pas cette édition duale.

## Fichiers de preuve

- preuves-p3.zip conserve les diagnostics worker, les manifestes de publication et les TXT/SRT issus des trois jobs.
- identites-p3.txt épingle les exécutables, les deux modèles et les DLL natives du candidat.
- Complément : [addendum des identités P3](addendum-identites-p3-20261120.md).
- Complément R6 : [tests automatisés du remplacement](addendum-remplacement-r6-20261121.md).
