Avancée : recettes manuelles Intel/NVIDIA, Intel sans NVIDIA et Snapdragon après installation du pack graphique validées sur les parcours documentés.
Blocage : T33 conserve un écart de traçabilité ; T41, T42, le refus matériel sous v2 et N2 CUDA restent ouverts. AMD est différé.
Décision : conserver les résultats matériels acquis et réserver les prochaines exécutions aux critères encore ouverts.

## Relecture du plan et état des tâches

Le plan initial (Git ca8fb16e) distingue construction, installation, témoins CPU/ARM et remise. La [version validée et amendée](plan-implementation-distribution-cpu-x64.md) précise les gates, T00 et T45. Les verdicts ci-dessous suivent cette version et les décisions ultérieures. Ce point est une revue des preuves existantes.

| Tâches | Verdict | Preuve et périmètre |
|---|---|---|
| T00 à T04 | Terminées au titre de L0 | Garde et SelfTest, protocoles de mesure préparés dans le plan. Les exécutions matérielles sont reportées en L4. |
| T11 à T13 | Terminées au titre de L1 | Profil portable, contrôle des capacités et composition CPU acceptés au passage vers L2. [Entrées du candidat](entrees-rust-candidats-turbo.md) et [correction du worker portable en L3](preuves-correction-cpu-l3.zip). |
| T21 à T24 | Terminées au titre de L2 | Édition, dépendances, notices et contrat dans le [bilan L2](bilan-l2-distribution-turbo.md). N2 CUDA reste une validation prédistribution distincte. |
| T31, T32, T34 | Terminées au titre de L3 | [Bootstrap et installation](preuves-recette-l3-bootstrap.zip), [preuves T32](preuves-t32-message-fix.zip), [configuration et remise](remise-l4-candidat-turbo.md). |
| T33 | Assemblage, démarrage et inférence constatés ; traçabilité partielle | [Premier lancement](preuves-t33-premier-lancement.zip), [recette CPU](preuves-recette-cpu-l3.zip), [fiche Intel avec empreintes et résultats par candidat](recette-second-pc-gpu-fix.md). L’utilisateur déclare le PC sans SDK CUDA ni outils de développement. La procédure contrôle les empreintes et utilise un profil neuf ; son listing seul ne certifie pas l’exécution des contrôles sur ce PC. Il manque le rattachement explicite du contrôle d’identité et de l’absence d’accès au dépôt à ce premier lancement. Aucun nouvel import manuel demandé. |
| T41 | NON EXÉCUTÉ | Runner x64 au prédicat T01 exact, import automatisé et exports. |
| T42 | NON EXÉCUTÉ | [Mandat ARM64](mandat-t42-windows-arm64.md). Le retour Snapdragon récupéré documente le parcours manuel. Aucun résultat de job CI réunissant pilotage de l’import, preuves OS/PE et capacités des deux processus. |
| T43 | Partiellement validée | Intel avec NVIDIA, Intel sans NVIDIA et observation Auto documentés dans les [résultats du candidat](recette-second-pc-gpu-fix.md) et la [remise](remise-l4-candidat-turbo.md). Admission matérielle v2 exacte et refus réel sous v2 ouverts ; AMD différé ; les deux pannes Auto provoquées sont hors périmètre. |
| T44 | Terminée pour le candidat de recette | Paquet, références, consignes et remises effectuées ; essais utilisateurs reçus. |
| T45 | En attente | Consolider les critères applicables et figer les configurations qualifiées après résolution des éléments ouverts. |

## Pièces ARM64 rattachées

- [Premier diagnostic OpenGL](rapport-opengl-Snapdragon-20261118-112523.zip) et [inférence CPU isolée](resultat-cpu-20261118-025646-44fad6.zip). Le worker x64 termine les 64 000 échantillons ; « Haute-Gon » remplace « Hong Kong ».
- [Archive de recette interactive récupérée](RETOUR-L4-20261118-053844-ee015.zip) : 48 entrées, dont vérification du candidat, rapport graphique, bilan, journaux et exports. Pack Microsoft.D3DMappingLayers ARM64 1.2608.3.0. CPU et Auto produisent deux segments SRT couvrant 0–2,020–4,000 s. Le journal de l’import Auto donne backend_ready CPU et 64 000/64 000 échantillons. Les deux essais micro ont leurs WAV/TXT/SRT. Les délais de premier texte (1 min 45 en CPU, 3 min en Auto) sont des observations opérateur. Ces éléments complètent la [fiche Snapdragon](recette-windows-arm64.md) et remplacent la mention historique du ZIP indisponible pour la recette désormais documentée.

L’installateur graphique sans terminal relève de la demande produit ultérieure. Le plan originel ne le spécifie pas comme condition de réussite des recettes déjà effectuées.
