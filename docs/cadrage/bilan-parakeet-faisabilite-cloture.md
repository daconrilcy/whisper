Avancée : faisabilité FR/EN acquise sur CPU et GTX 1080 Ti ; passe unique retenue jusqu'à 30 min et mécanique VAD 60–120 s requalifiée GO.
Blocage : l'essai unique VAD visant 20 min sur un audio synthétique de 60 min présente 3 puis 4 éditions à ±2 s des coupures sur chaque moteur ; ce format n'est pas retenu.
Décision : clôture de l'essai de faisabilité en GO avec réserves ; au-delà de 30 min, design retenu avec VAD 60–120 s, silence ≥400 ms et marge 200 ms.

# Clôture de faisabilité Parakeet TDT 0.6B v3 — 16 octobre 2026

Le présent bilan clôt l'essai isolé `target/parakeet-feasibility/spike02/` dans le périmètre **français et anglais, CPU et NVIDIA GTX 1080 Ti (CUDA0)**. Il remplace le statut du [bilan précédent](bilan-parakeet-limite-vad-revisee.md) pour le point 3B : le seuil d'écart global à la passe unique est retiré des critères bloquants, car la passe unique n'est pas une vérité terrain. Les points 1, 2 et 4 restent GO acquis, sans nouvelle intervention. Les scripts, WAV synthétiques et journaux restent dans le banc hors Git ; `sound example/spkr0.wav` reste hors Git. Aucun code produit n'a été modifié, aucune revue supplémentaire n'a été demandée ou exécutée.

| Critère | Résultat | Statut |
| --- | --- | --- |
| **1. Worker/DLL** | Résultat antérieur acquis : worker hors processus, 20 inférences FR/EN sur CPU et CUDA0 sans crash hôte. Aucun nouvel essai. | **GO acquis** |
| **2. Horodatages** | Résultat antérieur acquis. Pour les six appels de l'essai VAD 60 min, zéro dépassement >80 ms, zéro frontière interne et zéro violation après borne. | **GO acquis** |
| **3A. Passe unique** | Les paliers 40 et 60 min réussissent sur CPU et CUDA0 ; limite de routage retenue à **30 min** par la marge de 50 % sur le dernier palier testé et réussi. Voir le [détail des temps et mémoires](bilan-parakeet-limite-vad-revisee.md). | **GO** |
| **3B. Mécanique VAD 60–120 s** | Sur l'EN réel de 5 min et le montage FR synthétique de 319,3 s : **0 édition normalisée à ±2 s des deux coupures par essai, 0 doublon détecté** sur CPU et CUDA0. L'écart global normalisé de **4,06 à 8,61 %** est une **sensibilité au contexte, à arbitrer avec une vérité terrain** ; il ne décide plus du passage. | **GO mécanique** |
| **3C. VAD visant 20 min, maximum 25 min** | Essai unique sur l'EN synthétique de 60 min, trois segments par moteur. **3 puis 4 éditions normalisées à ±2 s**, sur CPU comme CUDA0 ; **0 doublon détecté**, **0 violation d'horodatage**. Le critère « zéro édition aux coupures » échoue. | **NO-GO pour ce format ; repli 60–120 s** |
| **3D. Déterminisme** | Deux nouvelles passes uniques de `spkr0.wav` identiques entre elles sur chaque moteur, texte et mots horodatés. Aucun nouvel essai dans cette clôture. | **GO acquis** |
| **4. Langue** | Résultat antérieur conservé : pas d'identifiant de langue retourné, comportement attendu. Aucun nouvel essai. | **GO acquis** |

## Essai unique du parcours long de 60 min

Le fichier `spkr0_x12.wav` est constitué de douze répétitions du fichier EN réel de 5 min, en PCM mono 16 kHz. La VAD RMS en trames de 20 ms a retenu le silence admissible le plus proche de chaque cible de 20 min : **1 194,87 s** et **2 394,87 s**. Chacun de ces silences mesure **860 ms**, au-dessus des 400 ms requis. Les durées centrales sont **19,9145 ; 20,0000 ; 20,0855 min** ; avec 200 ms de marge aux bords internes, les entrées restent à **19,9178 ; 20,0067 ; 20,0888 min**, toutes sous le maximum de 25 min. Chaque moteur a chargé la DLL dans un worker unique et traité ses trois segments.

| Moteur | Calcul par segment | Éditions normalisées aux coupures 1 / 2, à ±2 s | Doublons détectés | Violations d'horodatage | Écart global normalisé à la passe unique de 60 min |
| --- | --- | ---: | ---: | ---: | ---: |
| CPU | 96,293 ; 97,366 ; 99,619 s | **3 / 4** | 0 | 0 | 1 403 / 11 895 tokens, **11,795 %** |
| CUDA0 | 32,497 ; 32,587 ; 34,840 s | **3 / 4** | 0 | 0 | 1 437 / 11 861 tokens, **12,115 %** |

Aux coupures, la passe unique et la VAD rendent notamment `140` contre `hundred forty` ; des tokens `they` et `were` diffèrent aussi dans la fenêtre temporelle. Ces chiffres documentent **l'échec du critère local de zéro édition** pour les segments de 20 min. L'écart global à la passe unique est indiqué pour montrer la sensibilité au contexte ; il n'est pas un score de qualité absolue et n'est pas bloquant. Aucune seconde configuration ou campagne n'a été essayée.

## Design retenu et réserves

**Passe unique jusqu'à 30 min sur CPU ou CUDA0 ; au-delà, découpe VAD en segments de 60 à 120 s**, sur des silences d'au moins **400 ms**, avec **200 ms de marge**. Le format visant 20 min, maximum 25 min, n'est pas retenu. Le GO de la VAD 60–120 s porte sur la mécanique observée aux coupures et l'absence de doublons ; il ne constitue pas une mesure de justesse de transcription. Le test de 60 min de ce bilan a porté seulement sur le format 20 min ; il n'étend pas directement la preuve expérimentale du format 60–120 s à 60 min.

**Statut global de l'essai de faisabilité : GO avec réserves**, dans le périmètre FR/EN, CPU/GTX 1080 Ti. Réserves explicites :

1. Le français long n'a été testé que sur un montage synthétique à silences de jonction ; la mécanique et le rendu restent à confirmer sur un enregistrement FR réel long.
2. Aucune vérité terrain humaine n'est disponible ; la qualité absolue de transcription et l'effet du contexte ne sont pas mesurés.
3. Les 23 autres langues et le paquet final de l'application restent hors périmètre.
4. Sous WDDM, la VRAM a été observée globalement par `nvidia-smi`, sans attribution exacte au worker Parakeet.

Cette clôture qualifie la faisabilité isolée. Le design et le plan d'intégration produit demeurent régis par leurs propres prérequis et autorisations.
