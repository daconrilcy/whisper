Avancée : bootstrap B7 validé sur Snapdragon, T33 Dell et N2 CUDA GTX 1080 Ti clos pour leurs candidats. Les recettes matérielles manuelles restent acquises.
Blocage : déclenchement CI de T42 à solder, puis consolidation T45 dans le périmètre AVX.
Décision : retenir les PC x64 avec AVX utilisable. Différer les anciens PC sans AVX et conserver le report AMD.

## Relecture du plan et état des tâches

Le plan initial (Git ca8fb16e) distingue construction, installation, témoins CPU/ARM et remise. La [version validée et amendée](plan-implementation-distribution-cpu-x64.md) précise les gates, T00 et T45. Les verdicts ci-dessous suivent cette version et les décisions ultérieures. Ce point est une revue des preuves existantes.

| Tâches | Verdict | Preuve et périmètre |
|---|---|---|
| T00 à T04 | Terminées au titre de L0 | Garde et SelfTest, protocoles de mesure préparés dans le plan. Les exécutions matérielles sont reportées en L4. |
| T11 à T13 | Terminées au titre de L1 | Profil portable, contrôle des capacités et composition CPU acceptés au passage vers L2. [Entrées du candidat](entrees-rust-candidats-turbo.md) et [correction du worker portable en L3](preuves-correction-cpu-l3.zip). |
| T21 à T24 | Terminées au titre de L2 | Édition, dépendances, notices et contrat dans le [bilan L2](bilan-l2-distribution-turbo.md). N2 CUDA de réutilisation des imports validé sur GTX 1080 Ti pour le candidat T33 : un test exécuté, un succès, aucun ignoré. [Résultat et journaux](preuves-n2-gtx1080ti-59c7a61f.zip). |
| T31, T32, T34 | Terminées au titre de L3 | [Bootstrap et installation](preuves-recette-l3-bootstrap.zip), [preuves T32](preuves-t32-message-fix.zip), [configuration et remise](remise-l4-candidat-turbo.md). |
| T33 | Terminée pour le candidat testé | [Archive Dell du 18 novembre 2026, 18:08 UTC](T33-dcbf572ce7.zip). Le contrôle d’environnement ne détecte aucun indice de dépôt, SDK ou outil de build. Installation en profil vide, identités vérifiées, import du WAV par le desktop installé, backend CPU et 64 000/64 000 échantillons confirmés. Les empreintes TXT/SRT concordent avec le manifeste du job. Le worker encore actif après publication se termine pendant la fermeture normale du desktop. Verdict du rapport : PASS. |
| T41 | Différé hors portée AVX ; non exécuté | Runner x64 au prédicat T01 exact, import automatisé et exports. |
| T42 | Preuves matérielles vérifiées ; déclenchement CI non exécuté | [Import automatisé sur le bureau Snapdragon](T42-2a857f69a9.zip). Windows ARM64, PE et processus AMD64, PID desktop 10488 et worker 1276, capacités v2 complètes, import CPU 64 000/64 000. TXT/SRT identiques à T33. Le pilote lancé par l’opérateur a consigné INCOMPLET à la lecture du journal encore ouvert ; corrélation vérifiée dans le ZIP. Fermeture normale non attestée. Clôture sur ce mode de déclenchement soumise à décision. |
| T43 | Partiellement validée | Intel avec NVIDIA, Intel sans NVIDIA et observation Auto documentés dans les [résultats du candidat](recette-second-pc-gpu-fix.md) et la [remise](remise-l4-candidat-turbo.md). Admission matérielle v2 exacte et refus réel sous v2 différés hors portée AVX ; AMD différé ; les deux pannes Auto provoquées sont hors périmètre. |
| T44 | Terminée pour le candidat de recette | Paquet, références, consignes et remises effectuées ; essais utilisateurs reçus. |
| T45 | En attente | Consolider les critères applicables et figer les configurations qualifiées après résolution des éléments ouverts. |

## Pièces ARM64 rattachées

- [Premier diagnostic OpenGL](rapport-opengl-Snapdragon-20261118-112523.zip) et [inférence CPU isolée](resultat-cpu-20261118-025646-44fad6.zip). Le worker x64 termine les 64 000 échantillons ; « Haute-Gon » remplace « Hong Kong ».
- [Archive de recette interactive récupérée](RETOUR-L4-20261118-053844-ee015.zip) : 48 entrées, dont vérification du candidat, rapport graphique, bilan, journaux et exports. Pack Microsoft.D3DMappingLayers ARM64 1.2608.3.0. CPU et Auto produisent deux segments SRT couvrant 0–2,020–4,000 s. Le journal de l’import Auto donne backend_ready CPU et 64 000/64 000 échantillons. Les deux essais micro ont leurs WAV/TXT/SRT. Les délais de premier texte (1 min 45 en CPU, 3 min en Auto) sont des observations opérateur. Ces éléments complètent la [fiche Snapdragon](recette-windows-arm64.md) et remplacent la mention historique du ZIP indisponible pour la recette désormais documentée.

L’installateur graphique sans terminal relève de la demande produit ultérieure. Le plan originel ne le spécifie pas comme condition de réussite des recettes déjà effectuées.

## Point après la recette du bootstrap B7

Sur le Snapdragon X2 Elite, le bootstrap B7 a signalé le pack graphique manquant, ouvert sa fiche dans le Microsoft Store, proposé la revérification et confirmé l’installation dans le profil utilisateur. La [fiche B7 et ses quatre captures](recette-snapdragon-store-arm64.md) clôturent cette recette. B7 désigne l’installateur dont le SHA-256 commence par 6560131d. Le desktop livré avec lui est tg7. Les preuves T33 et T42 conservent les identités de leurs propres exécutables.

| Étape et portée actualisée | Condition de clôture selon le plan |
|---|---|
| T41 et admission sur v2 exact de T43/AC-03b | Différés. Le témoin défini par T01 exclut AVX. |
| T42/AC-08 | Le ZIP du Snapdragon établit l’émulation x64 du desktop et du worker et l’import CPU. Le lancement depuis le bureau était manuel. Pour appliquer le plan en vigueur, exécuter le job sur un runner CI ARM64 interactif autorisé. Accepter le déclenchement déjà réalisé demanderait une décision explicite modifiant ce critère. |
| T43/AC-03a | Essai physique de refus sur ancien PC sous v2 différé. Les tests logiciels T32 restent acquis. |
| T45 | Consolider AC-01 à AC-10 par candidat et par empreinte, identifier les preuves affectées par les changements de binaires, figer les configurations et limites effectivement qualifiées. Si AMD demeure indisponible, arbitrer la portée de distribution comme prévu au plan. |

T44 est terminé pour la remise de recette. Les pannes Auto provoquées restent hors périmètre. La distribution publique demeure en attente de T45 et d’un accord distinct.

## Portée AVX retenue pour les PC x64

Décision du 18 novembre 2026 applicable à la qualification en cours : sur PC x64, AVX utilisable par le processus Windows est désormais une condition de prise en charge, en complément du socle x64-v2 du candidat. AVX2 n’est pas exigé. Les témoins physiques anciens sans AVX (v2 exact et sous v2) sont différés et sortent des conditions de clôture de cette portée. Leur compatibilité reste non qualifiée. Cette décision prévaut sur les prescriptions historiques T01/T41 et les branches correspondantes de T43. AMD reste différé séparément.

T45 doit documenter ce prérequis et rattacher aux témoins Intel retenus les capacités effectivement disponibles. Le bootstrap B7 contrôle le niveau v2 ; le refus automatique d’un CPU v2 sans AVX ne fait pas partie du comportement livré. La recette Store sur Snapdragon est acquise ; la question du déclenchement CI de T42 reste distincte.
