Avancée : le lot 1 relie le GPU au micro et à l'import ; l'import de 60 s passe en 15,62 s sur GPU.
Blocage : le délai d'affichage et la qualité du micro restent à vérifier par l'utilisateur dans la fenêtre.
Décision : Q1–Q3 acceptées ; les tâches 7 et 8 restent hors du lot autorisé.

# Évolution GPU — cadrage et lot 1 (13 octobre 2026)

## Besoin, périmètre et critères

Whisper Windows propose **CPU ou GPU** avant import WAV/MP3 ou séance micro, avec le même **large-v3-turbo** local. Le CPU reste utilisable ; le micro conserve ses blocs confirmés et sorties TXT/SRT/WAV. Exclus : Auto, bascule automatique, supervision, reprise, autre modèle, tray, sortie MP3. D-WHISPER-30 et P-WHISPER-31 ne gouvernent pas ce lot.

| ID | Critère d'acceptation proposé, à vérifier dans la fenêtre Windows en release |
|---|---|
| AC1 | Choisir CPU puis GPU ; moteur demandé effectif, CPU encore sélectionnable après erreur, sans bascule implicite. |
| AC2 | Micro GPU avec parole, silence et phrase finale : blocs confirmés ordonnés avant Arrêter ; TXT/SRT/WAV de même nom, dernière phrase et SRT chronologique après arrêt ou fermeture. |
| AC3 | Imports WAV/MP3 GPU : TXT/SRT lisibles, source intacte ; parcours CPU préservé. |
| AC4 | Même fichier large-v3-turbo, langue et décodage CPU/GPU ; textes comparés selon Q2. |
| AC5 | Sur ce PC, import et micro de **60 s** : traitement GPU / durée audio **< 1**, résultat complet. Pour le micro, sommer le calcul des fenêtres ; relever préparation et export séparément. |
| AC6 | Premier bloc micro confirmé **affiché** en **< 15 s** selon Q3 ; l'événement worker ne suffit pas. |
| AC7 | GPU absent ou défaillant : cause et action visibles, aucun faux succès ; CPU disponible manuellement, sort de l'audio capté selon Q1. |

## Faisabilité locale et réemploi

PC : **GTX 1080 Ti** (11 264 MiB), pilote 581.29 ; CUDA **12.9**, CMake 4.4.4, Rust 1.98.1, VS2022 et modèle local présents. Le worker `whisper-rs 0.16.0`/CUDA compile en release **hors ligne**. La trace native confirme `using CUDA0 backend` ; `Ready Cuda` seul ne suffit pas.

Sur le WAV de référence de **30 s** (extrait CPU historique + silence), même modèle/langue : **GPU 9,349 s (0,312×)** ; **CPU 69,411 s (2,314×)**, dont 67,706 s après `Ready` (**2,257×**, proche des 2,25× antérieurs). Textes concaténés identiques ; affichage micro non mesuré.

Réemploi : GPU `StartImport`/`StartLiveWorker`, `ComputeChoice` et commandes masquées de `ui.rs`. Avant le lot, le micro direct imposait CPU et le GPU ignorait `StartDirectLiveWorker` ; ses fenêtres plafonnaient à 5 s contre 8 s au micro.

Lot 1, mesure technique hors fenêtre sur release : bloc micro GPU de 5 s traité en **1,136 s** après démarrage du worker (2,071 s) ; import du même WAV de 60 s en **15,623 s GPU (0,260×)** et **137,919 s CPU (2,299×)**. Textes d'import identiques. Le WAV de 60 s répète deux fois l'extrait de 30 s ; ces chronos ne prouvent ni le délai d'affichage depuis « Démarrer » ni le comportement d'une séance micro réelle.

Architecture : conserver capture 16 kHz, WAV et export ; figer le choix par requête, attester CUDA avant capture, préserver l'ordre des fenêtres et calculer hors UI. En erreur, arrêter, signaler l'incomplet et laisser CPU sélectionnable.

## Plan — 8 tâches ; lot 1 limité aux tâches 1 à 6

1. Décider Q1–Q3 et fixer les scénarios de mesure/texte — AC4–AC7.
2. Rendre le choix CPU/GPU visible sans Auto, puis le porter réellement aux requêtes import et micro — AC1, AC3.
3. Vérifier au démarrage le backend réellement utilisé et traduire l'indisponibilité en message/action — AC1, AC7.
4. Raccorder le worker GPU au micro direct et aux fenêtres de 5 à 8 s — AC2, AC5–AC6.
5. Préserver ordre, confirmation, arrêt, fermeture et exports du micro avec le moteur choisi — AC2, AC7.
6. Traiter panne GPU et résultat partiel selon Q1, sans bascule automatique — AC1, AC7.
7. Tester choix, erreurs, identité des blocs, sorties et non-régression CPU/import — AC1–AC4, AC7.
8. Qualifier dans la fenêtre 60 s import/micro, délai du premier affichage, qualité CPU/GPU et indisponibilité — AC1–AC7.

## Décisions produit attendues

Q1–Q3 sont **acceptées le 13 octobre 2026** pour le lot 1. Les tâches 1 à 6 sont autorisées en commits locaux, sans push ; l'essai dans la fenêtre reste à l'utilisateur.

- **Q1, panne GPU en micro (AC2, AC7)** : arrêter la session, garder le WAV et les blocs confirmés ; nommer TXT/SRT avec `-incomplet`, afficher une proposition de réimport CPU du WAV. Aucun passage automatique au CPU. Vérifier ce cas par un test simulé.
- **Q2, qualité (AC4)** : l'utilisateur compare côte à côte le texte d'une séance micro GPU et celui obtenu en important son WAV en CPU.
- **Q3, premier texte (AC6)** : chronométrer depuis le clic « Démarrer », en parlant immédiatement ; seuil de 15 s pour le micro.

Sources : demande, `AGENTS.md`, bilan micro, crates ; apports `PRODUCT-GPU-01 v0.2`, exigences v0.2, `ARCH-GPU-v1`. Tests du workspace réussis ; critères dans la fenêtre à qualifier par l'utilisateur.
