# Exigences L04 D26 — révisions sémantiques proposées

Pas de nouveau besoin produit. Les versions REQ/UC/AC ci-dessous sont révisions des IDs existants, proposées sous revue.

| REQ existante | v3 proposée | UC / AC |
| REQ-07 | Auto même passage, capture durant barrière sous capacités établies, watermark réglé puis CPU depuis T; saturation conservation ferme vers Recoverable. | UC-07 / AC-07 |
| REQ-08 | GPU forcé ferme/drain/scelle et attend; choix CPU explicite finit même passage depuis PCM sans réouverture/nouveaux samples. | UC-07 / AC-07 |
| REQ-11 | obligations supervisées séparément, aucun timeout destructif seul. | UC-09 / AC-09 |
| REQ-18 | T/A/C/E distincts, silence couvert, migration vérifiée sinon Recoverable. | UC-13 / AC-13 |

Les décisions Q-L04-AUTO-CAPTURE-01 et Q-L04-FORCED-GPU-FINISH-01 sont structural/answered et fondent REQ-07/08. Les détails sont dans 60/61. AC-07/09/13 sont les IDs existants en révision, pas de nouvelles capacités. REQ-26/UC-21/AC-21 de D24 sont retirés; Q-09 borne les diagnostics uniquement. Toute ancienne prose D24 qui les présente est supersédée. PRODUCT_VALIDATION NOT RUN.