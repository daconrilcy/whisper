Avancée : les paliers synthétiques fixent une limite retenue de 10 min en passe unique sur CPU et CUDA0 ; les points 1, 2 et 4 restent GO acquis.
Blocage : la VAD 60–120 s dépasse le seuil d'écart de 2 % sur le fichier EN réel et sur le montage FR synthétique ; la voie longue n'est pas qualifiée.
Décision : GO pour la passe unique FR/EN dans la limite mesurée ; NO-GO pour la découpe longue et donc pour le parcours complet au-delà de 10 min.

# Parakeet FR/EN — limite de passe unique et découpe longue, 15 octobre 2026

Ce bilan applique le nouveau critère de corpus : un long fichier réel par langue, ou un long fichier synthétique explicitement signalé. Le fichier EN réel est `sound example/spkr0.wav` (300 s, PCM 16 kHz mono). Aucun fichier FR réel d'au moins 5 min n'existe dans `sound example/` ; le montage FR de 319,3 s concatène `fr87.wav`, `fr60.wav`, `fr27.wav`, `fr25.wav`, puis répète `fr87.wav` et `fr27.wav`, avec 1 s de silence numérique à chaque jonction. Ses frontières sont **artificielles** ; il ne démontre pas la qualité sur un enregistrement FR long réel.

Tous les fichiers générés, les scripts et les journaux sont dans le banc Git ignoré `target/parakeet-feasibility/spike02/`, notamment `fixtures/synthetic/`, `synthetic_limit_summary.json` et `long_vad_summary.json`. Le WAV EN original reste hors Git. Aucun code produit ni point 1, 2 ou 4 n'a été modifié. La variante à fenêtres chevauchantes a été abandonnée et n'a pas été retestée.

| Critère | Résultat | Statut |
| --- | --- | --- |
| **1. Worker/DLL** | Acquis dans le [bilan précédent](bilan-parakeet-fr-en-criteres-revises.md) : 20 inférences FR/EN sur CPU et CUDA0 sans crash du processus hôte. Aucun nouvel essai. | **GO acquis** |
| **2. Horodatages** | Acquis : dépassement brut d'au plus une trame de 80 ms sur les extraits vérifiés et zéro violation après borne. Aux nouveaux paliers réussis 10 et 20 min, zéro dépassement >80 ms, zéro frontière interne et zéro violation après borne. Les essais de 40 min ont été arrêtés au plafond de temps avant production de mots ; les horodatages y sont donc non mesurés. | **GO acquis** |
| **3A. Limite en passe unique** | 10 et 20 min réussissent sur CPU et CUDA0. Le palier 40 min atteint le plafond de calcul de cinq fois le temps de 5 min sur les deux moteurs ; le worker isolé est arrêté. Dernier palier réussi : 20 min ; marge de 50 % : **10 min retenues sur CPU et CUDA0**. | **GO pour fixer la limite** |
| **3B. VAD longue EN** | `spkr0.wav` réel, trois segments de 94,72 à 107,48 s, silence ≥400 ms, marge 200 ms : écart à la passe unique **8,39 % CPU** et **8,58 % CUDA0**, au lieu de ≤2 %. Aucun doublon détecté, aucun mot horodaté ne traverse une coupure ; des écarts lexicaux existent près des coupures, dont « you know » et « um » dans la référence. | **NO-GO** |
| **3C. VAD longue FR** | Montage synthétique de 319,3 s, trois segments de 88,13 à 116,13 s, mêmes règles : écart **4,35 % CPU** et **4,06 % CUDA0**. Aucun doublon détecté, aucun mot horodaté ne traverse une coupure ; corpus à frontières artificielles. Le statut « GO provisoire FR » n'est pas atteint car l'écart dépasse 2 %. | **NO-GO** |
| **3D. Corpus révisé** | Un EN long réel et un FR long synthétique explicitement identifié sont présents. Le corpus répond à la nouvelle condition d'entrée, sans rendre la VAD conforme. | **GO corpus** |
| **4. Langue** | Acquis : Parakeet v3 ne renvoie pas d'identifiant de langue ; ce n'est pas un critère de réussite. Aucun nouvel essai. | **GO acquis** |

## Paliers de passe unique

Les 10, 20 et 40 min sont produits par `ffmpeg -f concat` avec respectivement deux, quatre et huit répétitions de `spkr0.wav`, puis normalisés en PCM 16 kHz mono. La base de 5 min est de **22,339 s CPU** et **6,842 s CUDA0** ; plafonds de calcul : **111,695 s CPU** et **34,210 s CUDA0**. Le temps de calcul est chronométré dans le worker autour de l'inférence. Le pic RAM est le `PeakWorkingSetSize` du worker Windows. Sous WDDM, `nvidia-smi` ne donne pas la VRAM par processus ; le tableau donne le pic **global** de la carte et son delta par rapport à l'instant avant essai, donc une attribution exacte à Parakeet n'est pas possible.

| Moteur | Durée audio | Résultat | Calcul | Facteur temps réel | Pic RAM worker | VRAM globale initiale → pic (delta) | Horodatage |
| --- | ---: | --- | ---: | ---: | ---: | ---: | --- |
| CPU | 5 min réel | Succès, base | 22,339 s | 13,43× | 3 106,1 Mio | — | 0 violation |
| CPU | 10 min synthétique | Succès | 47,018 s | 12,76× | 4 363,6 Mio | — | 0 violation |
| CPU | 20 min synthétique | Succès | 95,992 s | 12,50× | 4 478,3 Mio | — | 0 violation |
| CPU | 40 min synthétique | Échec, plafond de temps ; worker arrêté à 113,829 s | Non terminé | — | 4 840,8 Mio observés | — | Non mesuré |
| CUDA0 / GTX 1080 Ti | 5 min réel | Succès, base | 6,842 s | 43,85× | 388,9 Mio | 2 896 → 6 101 Mio (+3 205) | 0 violation |
| CUDA0 / GTX 1080 Ti | 10 min synthétique | Succès | 14,054 s | 42,69× | 448,0 Mio | 2 660 → 6 456 Mio (+3 796) | 0 violation |
| CUDA0 / GTX 1080 Ti | 20 min synthétique | Succès | 32,330 s | 37,12× | 562,6 Mio | 2 704 → 6 545 Mio (+3 841) | 0 violation |
| CUDA0 / GTX 1080 Ti | 40 min synthétique | Échec, plafond de temps ; worker arrêté à 36,333 s | Non terminé | — | 713,3 Mio observés | 2 661 → 6 697 Mio (+4 036) | Non mesuré |

L'échec à 40 min est **temporel** selon le critère demandé, sans preuve de manque de RAM ou de VRAM. La limite retenue de **10 min** vaut pour ce matériel et ce protocole ; le palier de 20 min réussit mais n'est pas la limite de routage, conformément à la marge de 50 %.

## Découpe VAD et comparaison

La VAD utilise des trames RMS de 20 ms. Elle choisit des milieux de silences d'au moins 400 ms de façon à produire trois segments de 60 à 120 s ; chaque bord interne reçoit 200 ms de contexte. Les mots sont ramenés aux horodatages globaux et rattachés au segment dont l'intervalle central contient leur centre. Il s'agit des marges VAD demandées, sans nouvelle variante de fusion par fenêtres chevauchantes. L'écart est la distance d'édition des tokens normalisés divisée par le nombre de tokens de la passe unique **sur le même moteur** ; cette référence mesure la stabilité de découpe, pas une vérité terrain humaine.

| Source et moteur | Coupures au milieu d'un silence | Durées centrales | Référence / VAD | Éditions ; écart | Doublons ; mots horodatés traversant les coupures | Contrôle lexical à ±1 s |
| --- | --- | --- | ---: | ---: | --- | --- |
| EN réel, CPU | 107,48 s ; 202,20 s | 107,48 ; 94,72 ; 97,80 s | 1 025 / 1 030 tokens | 86 ; **8,39 %** | 0 ; 0 | 2 frontières différentes |
| EN réel, CUDA0 | idem | idem | 1 026 / 1 030 | 88 ; **8,58 %** | 0 ; 0 | 2 frontières différentes |
| FR synthétique, CPU | 88,13 s ; 203,17 s | 88,13 ; 115,04 ; 116,13 s | 690 / 695 | 30 ; **4,35 %** | 0 ; 0 | 0 frontière différente |
| FR synthétique, CUDA0 | idem | idem | 689 / 697 | 28 ; **4,06 %** | 0 ; 0 | 0 frontière différente |

Les silences EN retenus durent respectivement 640 ms et 640 ms. Les silences FR sont des jonctions artificielles de 1 100 ms et 1 020 ms. Le contrôle « zéro mot coupé » ici vérifie qu'aucun mot horodaté par la référence ni par la VAD ne traverse le milieu d'une coupure ; cela ne prouve pas l'absence de mot omis, comme le montrent les différences EN aux frontières.

## Proposition de design et statut

**Routage chiffré : passe unique jusqu'à 10 min sur CPU ou CUDA0 ; au-delà, VAD en segments de 60 à 120 s, sur silences d'au moins 400 ms avec 200 ms de marge.** La première branche est étayée par les paliers réussis. La seconde est une **proposition non qualifiée** : les mesures présentes échouent au seuil de 2 % en EN et en FR, même sur un fichier de 5 min. Elle ne doit pas être présentée comme prête pour l'implémentation du parcours long. Le statut global du **parcours complet demandé est NO-GO**, avec GO limité à la passe unique ≤10 min dans la portée FR/EN, CPU/GTX 1080 Ti. Les 23 autres langues et le paquet final restent hors périmètre.
