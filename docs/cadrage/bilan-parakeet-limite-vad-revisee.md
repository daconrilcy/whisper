Avancée : les paliers synthétiques de 40 et 60 min réussissent sur CPU et CUDA0 ; la limite retenue avec 50 % de marge passe à 30 min pour chaque moteur.
Blocage : après normalisation, la VAD reste à 4,06–8,61 % d'écart à la passe unique, au-dessus du seuil révisé de 3 % ; les éditions lexicales sont hors des zones de coupure.
Décision : point 3A GO, point 3B NO-GO et parcours long global NO-GO ; les points 1, 2 et 4 restent GO acquis sans modification.

# Parakeet FR/EN — seuil de passe unique et ventilation VAD révisés, 15 octobre 2026

Ce bilan remplace les conclusions temporelles et VAD du [bilan antérieur](bilan-parakeet-limite-decoupe-longue.md). Les points 1, 2 et 4 restent acquis. Les fichiers synthétiques, scripts et journaux sont dans le banc Git ignoré `target/parakeet-feasibility/spike02/`. `spkr0.wav` est l'EN réel de 5 min ; les sources longues de 40 et 60 min sont ses répétitions `ffmpeg` ×8 et ×12, en PCM 16 kHz mono. Le FR de 319,3 s est un montage synthétique avec silences de jonction, déjà signalé. Aucun paramètre VAD ni code produit n'a changé.

| Point et critère | Résultat | Statut |
| --- | --- | --- |
| **1. Worker/DLL**, acquis | Résultat antérieur conservé ; aucune nouvelle intervention. | **GO acquis** |
| **2. Horodatages**, acquis | Résultat antérieur conservé. Aux nouveaux paliers de 40 et 60 min : zéro dépassement >80 ms, zéro frontière interne et zéro violation après borne sur CPU et CUDA0. | **GO acquis** |
| **3A. Passe unique** : pas de manque de mémoire ni crash, facteur ≥6,7× CPU ou ≥22× CUDA0, garde-fou 15 min | Les quatre paliers révisés réussissent. Dernier palier testé et réussi : **60 min** sur chaque moteur. La règle de 50 % donne une **limite retenue de 30 min CPU et 30 min CUDA0**. Aucun palier d'échec n'a été observé avec ce critère ; 30 min est une limite de routage prudente, pas une durée maximale physique démontrée. | **GO** |
| **3B. VAD longue** : ≤2 éditions normalisées par coupure à ±2 s, écart global normalisé ≤3 %, zéro doublon | **0 édition normalisée** dans les deux zones de coupure de chaque essai, zéro doublon relevé ; écart global **8,42 % EN CPU, 8,61 % EN CUDA0, 4,35 % FR CPU, 4,06 % FR CUDA0**. Le seuil global échoue dans les quatre cas. | **NO-GO** |
| **3C. Déterminisme EN** : deux nouvelles passes uniques identiques sur le même moteur | Deux sorties CPU identiques entre elles, texte et 966 mots horodatés ; deux sorties CUDA0 identiques entre elles, texte et 967 mots horodatés. Chaque première nouvelle sortie est aussi identique à la passe unique antérieure du même moteur. | **GO** |
| **4. Langue**, acquis | Résultat antérieur conservé ; aucune nouvelle intervention. | **GO acquis** |

## Passe unique : paliers révisés

Le plafond fixe de cinq fois le calcul de 5 min a été retiré. Aucun palier n'a atteint le garde-fou de 15 min de calcul. Le facteur temps réel est la durée audio divisée par le temps d'inférence mesuré dans le worker. La RAM est le pic `PeakWorkingSetSize` du worker Windows. Sous WDDM, `nvidia-smi` ne fournit pas de VRAM par processus : le pic et le delta suivants sont ceux de **toute la carte**, et peuvent inclure d'autres processus.

| Moteur | Audio synthétique | Calcul | Facteur | Pic RAM worker | VRAM globale initiale → pic (delta) | Horodatages | Résultat |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| CPU | 40 min | 194,029 s | 12,369× | 5 128,9 Mio | — | 0 violation | Succès |
| CPU | 60 min | 300,175 s | 11,993× | 5 407,4 Mio | — | 0 violation | Succès |
| CUDA0 / GTX 1080 Ti | 40 min | 67,692 s | 35,454× | 719,9 Mio | 2 715 → 6 845 Mio (+4 130) | 0 violation | Succès |
| CUDA0 / GTX 1080 Ti | 60 min | 103,466 s | 34,794× | 868,2 Mio | 2 674 → 6 936 Mio (+4 262) | 0 violation | Succès |

## Ventilation des écarts VAD existants

La réanalyse relit uniquement les journaux de la passe unique et des segments VAD existants. Elle aligne par distance d'édition les tokens Unicode en minuscules, sans ponctuation. Elle retire ensuite les tokens isolés `um`, `uh`, `euh`, `hum`, `ben`, `bah` et les paires consécutives `you know`, `i mean`, sans jugement sémantique de chaque occurrence. Une édition est en **zone de coupure** si au moins un de ses tokens alignés porte un horodatage à ±2 s du milieu du silence choisi ; sinon elle est **hors zone**. Le contrôle des doublons du bilan antérieur demeure zéro. Les coupures restent 107,48 et 202,20 s en EN ; 88,13 et 203,17 s en FR synthétique, avec segments 60–120 s, silence ≥400 ms et marge 200 ms.

| Source / moteur | Tokens référence → VAD après retrait | Remplissages retirés réf. / VAD | Éditions brutes en zone : remplissage / lexical | Éditions brutes hors zone : remplissage / lexical | Éditions normalisées zone / hors zone | Écart normalisé |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| EN réel / CPU | 986 → 996 | 39 / 34 | 2 / 0 | 3 / 81 | 0 / 83 | **8,418 %** |
| EN réel / CUDA0 | 987 → 996 | 39 / 34 | 2 / 0 | 3 / 83 | 0 / 85 | **8,612 %** |
| FR synthétique / CPU | 690 → 695 | 0 / 0 | 0 / 0 | 0 / 30 | 0 / 30 | **4,348 %** |
| FR synthétique / CUDA0 | 689 → 697 | 0 / 0 | 0 / 28 | 0 / 28 | **4,064 %** |

Les colonnes d'éditions **brutes** classent les modifications qui impliquent un remplissage ou un token lexical. Après retrait des remplissages, l'alignement est recalculé ; ses nombres ne sont donc pas la simple soustraction des colonnes brutes. Dans chaque essai, chacune des deux coupures comporte **0 édition normalisée à ±2 s**, sous le seuil de deux. Les écarts restants sont dispersés ailleurs dans l'audio. La référence est la passe unique du même moteur, pas une transcription humaine annotée : ces pourcentages mesurent la stabilité du découpage, pas la précision linguistique absolue.

## Déterminisme et proposition de design

Deux nouvelles passes uniques de `spkr0.wav` ont été exécutées par moteur dans le même banc, en workers distincts. Les transcriptions et chaque mot avec ses horodatages sont **identiques** entre les deux répétitions CPU, et identiques entre les deux répétitions CUDA0. Aucune variation à quantifier n'a été observée dans ce contrôle.

**Valeurs de design proposées : passe unique jusqu'à 30 min sur CPU et CUDA0 ; au-delà, VAD 60–120 s, silence ≥400 ms, marge 200 ms.** La première branche est étayée par les paliers réussis et la marge demandée. La seconde reste **non qualifiée** : elle échoue au critère global normalisé de 3 % même sur des fichiers d'environ 5 min, sans que les coupures elles-mêmes expliquent les éditions lexicales. Le statut du **parcours complet reste NO-GO**. Le FR reste synthétique ; les 23 autres langues et le paquet final sont hors périmètre.
