# Bilan de l'évolution GPU — 14 octobre 2026

**Livré.** L'application Windows transcrit les imports WAV/MP3 et le micro avec le même modèle large-v3-turbo, au choix CPU ou GPU. CUDA confirmé au démarrage présélectionne GPU ; sinon CPU est présélectionné avec un message. Le choix reste modifiable avant une session, sans bascule pendant celle-ci. Le micro garde ses blocs confirmés et produit TXT, SRT et WAV ; si le GPU échoue, le WAV et les textes confirmés restent disponibles avec TXT/SRT `-incomplet` et une invitation à réimporter le WAV en CPU. Les tests automatiques couvrent choix, erreur simulée, sorties, parcours CPU et filtre de fin de fichier.

**Mesures et validation.** Sur l'import de référence de 60 s : GPU **0,26×** la durée audio, CPU **2,30×**. L'utilisateur a chronométré le premier texte micro à **1 s** depuis « Démarrer » en parlant immédiatement. Il a validé une séance micro complète de 60 s, la qualité comparable CPU/GPU et les réimports WAV/MP3 sur les deux moteurs. Ces mesures valent pour ce PC et ces fichiers.

**Correctif de fin de fichier.** Le correctif `b716758a` empêche l'ajout « Thank you » sur les fenêtres finales presque silencieuses. Sa portée est **la fin de fichier uniquement** ; il ne prétend pas supprimer toute hallucination dans d'autres passages. Un court passage audible après un silence reste admis pour transcription.

**Vigilance matérielle.** La GTX 1080 Ti (Pascal) dépend de versions de CUDA et de pilotes dont NVIDIA retire progressivement le support pour cette génération. Vérifier le fonctionnement GPU après toute mise à jour CUDA ou pilote ; le CPU reste sélectionnable. [NVIDIA, évolution du support CUDA de Pascal](https://developer.nvidia.com/blog/cuda-toolkit-12-8-delivers-nvidia-blackwell-support/).

Clôture locale après la tâche 7 ; aucune installation, dépendance nouvelle ni publication distante.
