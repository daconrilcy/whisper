# Plans Whisper — P-WHISPER-14

Statut : DRAFT, revue indépendante exacte de P14 en attente. Base PLANS immuable : P-WHISPER-13, digest `fbd7b0d2033cbdda8f362921539fdf3349e1a6c6a976ad1ebf82e4840cc203ad`. Parent DESIGN immuable : D-WHISPER-19, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

Le registre actif reste `docs/predev/state.json` jusqu'à promotion canonique distincte : P-WHISPER-10 et PLANS-10-v3 CLEAN ; L03 completed avec son fingerprint exact ; L04 planned. P13 a reçu FINDINGS et n'est ni CLEAN ni promu. P14 est le successor complet qui corrige le finding P11-01. Les revues antérieures P09/P10/P12/P13 sont historiques et ne valent pas revue de P14.

## Corpus, provenance et statut

P14 transporte l'intégralité du corpus hérité et les documents nécessaires au finding P13. Le rapport exact P13 est reproduit sous `sources/reviews/R-WHISPER-PLANS-13-v1.md`, SHA `c19e3909f46e9143cf2e04c68b9b7f290828225ba8815617fda571adebda1d33`, à partir du transport `T-WHISPER-PLANS-REVIEW-13` (manifest digest `512a5cd18893fdfde5e38c6770421a752170e6893614bfa6f102129a8849d251`, manifest SHA `b49a4f94a443098032f03af8aef26ba750e2b25347852b5ed2ab2838b71483c9`). P10, P11, P12, P13 et D19 restent immuables.

La contribution `ARCH-Q04-Q06-Q09-v2` est une nouvelle contribution rédigée par son auteur, conservée dans `T-WHISPER-Q04-Q06-Q09-ANSWER-02`; elle n'est pas présentée comme export de sa réponse antérieure. La capture utilisateur Q09 ne contient que le choix réel « Rotation bornée (Recommended) ». Les limites chiffrées de l'option sont attribuées au paquet décrit par l'architecte; les paramètres techniques complémentaires restent attribués à l'architecte, pas à des choix utilisateur séparés.

## Lots et progression contrôlés

L00/L01/L02 et L03 sont completed selon le registre actif et les preuves qui y sont liées ; L04–L07 restent planned. P14 ne modifie pas l'état actif. Tous les essais runtime et campagnes énumérés restent PROPOSED / NOT RUN. Les réponses Q-04/Q-06/Q-09 sont documentées dans P14, mais le registre actif ne changera qu'après revue CLEAN, décision/acceptation dans le cadre délégué et promotion contrôlée. Aucun code L04/L05 n'est autorisé par ce document.

## Carte documentaire

`01_LOTS.md` décrit couverture, ownership et l'allowlist ; `02_VERIFICATION_AND_PREFLIGHT.md` distingue les gates P14 du matériel historique ; `04_OPEN_DETAILS.md` est le registre des détails, aligné sur v2 et la revue P13 ; `05_SOURCE_AND_RULE_INDEX.md` décrit sources, transports et provenance ; les documents `06` à `12` conservent les contrats/snapshots/preuves historiques L01–L03 ; `13_L04_DETAIL_CONTRACTS.md` consigne les réponses proposées et leurs limites.

## Allowlist L04 issue de l'état actif

L'unique liste de chemins autorisés à réconcilier pour L04 demeure exactement les cinq chemins du registre actif. P14 ne l'élargit pas. Tout fichier supplémentaire nécessaire — notamment UI/root, IPC, ports, manifeste ou configuration — constitue un `CHANGE` et bloque le préflight jusqu'à révision documentaire et autorisation explicite. Une revue CLEAN de P14 ne vaut pas autorisation de coder.

## Lignées antérieures

P10 a révisé L03 ; P11 a été une étape documentaire partielle ; P12 a restauré le corpus complet ; P13 a reçu FINDINGS sur son registre des détails. Ces mentions expliquent la provenance seulement et ne déterminent pas le statut courant.
