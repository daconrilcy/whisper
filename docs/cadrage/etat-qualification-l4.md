Avancée : T42 validé, fermeture comprise. T45 a consolidé les preuves et figé le candidat B7/tg7.
Blocage : les preuves de transcription sur le paquet final exact restent limitées aux parcours rattachés ci-dessous.
Décision : figer le bilan dans la portée AVX, avec les limites de qualification par binaire.

**Décision du 18 novembre 2026 — T42 / AC-08 validé pour cette version.** L’utilisateur accepte l’essai automatisé lancé depuis le bureau du Snapdragon SC8480XP. Il lève l’exigence de déclenchement CI ARM64 pour cet essai. Le ZIP établit l’émulation x64 du desktop et du worker, la transcription et les exports. Le rapport brut reste INCOMPLET à la lecture du journal encore ouvert et la fermeture normale est confirmée par l’utilisateur lors de son essai manuel. [Preuves de l’essai](T42-2a857f69a9.zip).

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
| T42 | Validée sur le candidat testé par décision utilisateur | [Import automatisé sur le bureau Snapdragon](T42-2a857f69a9.zip). Windows ARM64, PE et processus AMD64, PID desktop 10488 et worker 1276, capacités v2 complètes, import CPU 64 000/64 000. TXT/SRT identiques à T33. Le pilote lancé par l’opérateur a consigné INCOMPLET à la lecture du journal encore ouvert ; corrélation vérifiée dans le ZIP. Fermeture normale confirmée manuellement. Mode de déclenchement accepté. |
| T43 | Partiellement validée | Intel avec NVIDIA, Intel sans NVIDIA et observation Auto documentés dans les [résultats du candidat](recette-second-pc-gpu-fix.md) et la [remise](remise-l4-candidat-turbo.md). Admission matérielle v2 exacte et refus réel sous v2 différés hors portée AVX ; AMD différé ; les deux pannes Auto provoquées sont hors périmètre. |
| T44 | Terminée pour le candidat de recette | Paquet, références, consignes et remises effectuées ; essais utilisateurs reçus. |
| T45 | Consolidation réalisée | Candidat B7/tg7 figé. Voir le verdict de portée ci-dessous. |

## Pièces ARM64 rattachées

- [Premier diagnostic OpenGL](rapport-opengl-Snapdragon-20261118-112523.zip) et [inférence CPU isolée](resultat-cpu-20261118-025646-44fad6.zip). Le worker x64 termine les 64 000 échantillons ; « Haute-Gon » remplace « Hong Kong ».
- [Archive de recette interactive récupérée](RETOUR-L4-20261118-053844-ee015.zip) : 48 entrées, dont vérification du candidat, rapport graphique, bilan, journaux et exports. Pack Microsoft.D3DMappingLayers ARM64 1.2608.3.0. CPU et Auto produisent deux segments SRT couvrant 0–2,020–4,000 s. Le journal de l’import Auto donne backend_ready CPU et 64 000/64 000 échantillons. Les deux essais micro ont leurs WAV/TXT/SRT. Les délais de premier texte (1 min 45 en CPU, 3 min en Auto) sont des observations opérateur. Ces éléments complètent la [fiche Snapdragon](recette-windows-arm64.md) et remplacent la mention historique du ZIP indisponible pour la recette désormais documentée.

L’installateur graphique sans terminal relève de la demande produit ultérieure. Le plan originel ne le spécifie pas comme condition de réussite des recettes déjà effectuées.

## Point après la recette du bootstrap B7

Sur le Snapdragon X2 Elite, le bootstrap B7 a signalé le pack graphique manquant, ouvert sa fiche dans le Microsoft Store, proposé la revérification et confirmé l’installation dans le profil utilisateur. La [fiche B7 et ses quatre captures](recette-snapdragon-store-arm64.md) clôturent cette recette. B7 désigne l’installateur dont le SHA-256 commence par 6560131d. Le desktop livré avec lui est tg7. Les preuves T33 et T42 conservent les identités de leurs propres exécutables.

| Étape et portée actualisée | Condition de clôture selon le plan |
|---|---|
| T41 et admission sur v2 exact de T43/AC-03b | Différés. Le témoin défini par T01 exclut AVX. |
| T42/AC-08 | Clos par acceptation de la preuve sur bureau Snapdragon. |
| T43/AC-03a | Essai physique de refus sur ancien PC sous v2 différé. Les tests logiciels T32 restent acquis. |
| T45 | Consolider AC-01 à AC-10 par candidat et par empreinte, identifier les preuves affectées par les changements de binaires, figer les configurations et limites effectivement qualifiées. Si AMD demeure indisponible, arbitrer la portée de distribution comme prévu au plan. |

T44 est terminé pour la remise de recette. Les pannes Auto provoquées restent hors périmètre. La distribution publique demeure en attente de T45 et d’un accord distinct.

## Portée AVX retenue pour les PC x64

Décision du 18 novembre 2026 applicable à la qualification en cours : sur PC x64, AVX utilisable par le processus Windows est désormais une condition de prise en charge, en complément du socle x64-v2 du candidat. AVX2 n’est pas exigé. Les témoins physiques anciens sans AVX (v2 exact et sous v2) sont différés et sortent des conditions de clôture de cette portée. Leur compatibilité reste non qualifiée. Cette décision prévaut sur les prescriptions historiques T01/T41 et les branches correspondantes de T43. AMD reste différé séparément.

T45 doit documenter ce prérequis et rattacher aux témoins Intel retenus les capacités effectivement disponibles. Le bootstrap B7 contrôle le niveau v2 ; le refus automatique d’un CPU v2 sans AVX ne fait pas partie du comportement livré. La recette Store sur Snapdragon est acquise ; le lancement sur bureau est accepté pour T42.

## Préparation de T45 dans la portée AVX

| Critère | Résultat attribuable et limite |
|---|---|
| AC-01 / AC-02 | Installation et fonctionnement sur les deux PC Intel de recette. Sur la station i9-14900K, un contrôle complémentaire dans PowerShell x64 rapporte Avx.IsSupported=True. Cette mesure porte sur ce processus. Les essais physiques au plancher exact sont différés, de même que le témoin AMD. |
| AC-03a / AC-03b | Refus matériel sur ancien CPU différé. Les contrôles de refus avant mutation, de compatibilité indéterminée et de ressource altérée restent rattachés à T32. Le parcours graphique de B7 sur Snapdragon est validé séparément. |
| AC-04 | Fermeture de paquet et installation en profil vierge documentées sur le candidat T33 Dell. Pour la configuration finale, annoncer x64-v2 avec AVX utilisable sur PC x64, et distinguer les éditions Windows testées ainsi que le pack graphique requis sur le Snapdragon de recette. |
| AC-05 / AC-06 | Imports CPU et capture micro documentés dans les recettes Intel et Snapdragon. Les exports et le contenu du WAV de référence sont vérifiés sur le Dell par T33 ; la capture micro relève des recettes utilisateur. |
| AC-07 | CI sur témoin v2 exact sans AVX différée hors de la portée actuelle. |
| AC-08 | Preuve matérielle obtenue sur SC8480XP, Windows 11 Enterprise ARM64 build 28000, avec desktop et worker x64 émulés. Le script d’import a été démarré depuis le bureau. Son journal final indique INCOMPLET ; import et exports sont établis par les pièces du ZIP, fermeture normale confirmée manuellement. La dispense de CI est actée pour cette version. |
| AC-09 / AC-10 | GTX 1080 Ti : CPU, GPU manuel et Auto testés ; moteur GPU affiché en Auto. Import GPU de 5 minutes en 1 min 10 s et premier bloc confirmé au micro en 4 s. Sur le Dell sans NVIDIA : Auto utilise le CPU, et le GPU manuel est refusé. Les deux pannes provoquées sont hors périmètre. |

**Identités à conserver.** Les archives T33 et T42 concernent le desktop 01e2ea8b… ; l’installateur B7 (6560131d…) embarque le desktop tg7 (6602055e…). La recette B7 prouve le guidage Store et l’installation. Les transcriptions historiques gardent l’identité de leur candidat. Les identités du candidat retenu ont été vérifiées à la consolidation T45.

[Recette Intel et NVIDIA](recette-second-pc-gpu-fix.md) · [Pièces T33](T33-dcbf572ce7.zip) · [Pièces T42](T42-2a857f69a9.zip) · [Mandat T42 et empreintes](mandat-t42-windows-arm64.md) · [Recette B7](recette-snapdragon-store-arm64.md)

## Verdict T45 — candidat figé le 18 novembre 2026

Le kit de recette retenu est WhisperTurbo-Snapdragon-Store-ARM64.zip (SHA-256 64969a4105e33abc865576c3d5fa3993c3a6a246cf1761fe617f614d5408b794). Empreintes SHA-256 relues directement depuis le ZIP :

| Entrée | SHA-256 |
|---|---|
| Bootstrap B7 | 6560131dc0bbc6e6bdd45a2728f719db7a6079324dc3fd06b816cd6ca22f21f6 |
| Desktop tg7 | 6602055e1e57092f22c32e3966ea36d723a8e79ff0e67f6a1ab9a7cc82df6ad4 |
| Worker CPU | a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b |
| Worker NVIDIA | d874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9 |
| Modèle large-v3-turbo | 1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69 |

T42/AC-08 est pleinement validé sur le témoin accepté. L’utilisateur a testé la fermeture normale. Le journal brut documente la limite de lecture du banc. Ces deux éléments sont conservés avec leur provenance.

Configuration de référence pour les PC : Windows 11 Famille/Pro x64, socle v2 avec AVX utilisable, modèle local vérifié. Témoins Intel : Dell i7-1185G7 sans NVIDIA et station i9-14900K avec GTX 1080 Ti (CUDA 61-real). Sur Snapdragon SC8480XP, Windows 11 Enterprise ARM64 build 28000, les exécutables x64 fonctionnent sous émulation ; le guidage graphique B7 a été validé avec le pack Microsoft installé depuis le Store. AMD et anciens PC sans AVX sont différés.

T45 documentaire terminé. Installation B7 vérifiée sur Snapdragon ; workers CPU/GPU et modèle identiques à ceux des recettes antérieures. Le desktop tg7 ajoute le parcours de sonde Glow avant la composition normale. Les mesures de débit, les imports et les essais micro restent attribués au desktop antérieur effectivement exécuté. Le dossier ne fournit pas de recette complète des transcriptions sur le SHA final tg7. La qualification intégrale du même paquet au sens strict du plan reste ouverte sur cet écart. La distribution publique requiert une décision distincte.
