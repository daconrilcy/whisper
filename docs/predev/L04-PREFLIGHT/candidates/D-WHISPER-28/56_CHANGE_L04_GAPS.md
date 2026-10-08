# GAP register — CHANGE L04 — DRAFT

| ID | GAP / owner | Preuve de fermeture attendue |
|---|---|---|
| GAP-L04-AUTO-01 | Domaine puis architecture : identité passage/archive versus génération worker, dernier point durable, replay live/import et rejet d'anciens résultats. | Transitions, contrats ports/IPC/archive et tests correspondants revus sur successor DESIGN. |
| GAP-L04-FORCED-01 | Architecture : GPU forcé attend choix explicite et ne bascule pas automatiquement. | Oracle de transitions et scénarios AC-07/D. |
| GAP-L04-SUPERVISION-01 | Architecture : obligations indépendantes, attente/suspicion/panne, horloge et événements UI sous Q-04. | Contrat de supervision et mesures futures, timer sans action destructive. |
| GAP-L04-LIFECYCLE-01 | Domaine/architecture : Stop/Quit prioritaires pendant barrière/reprise, fermeture de fenêtre sans arrêt implicite. | Transitions et scénarios d'annulation/rejeu tardif. |
| GAP-L04-DURABILITY-01 | Domaine/architecture : ACK durable, confirmations, checkpoint de reprise et Recoverable. | Invariants archive/PCM/segments/MP3 et migration metadata. |
| GAP-L04-DIAGNOSTICS-01 | Requirements/architecture : bornes Q-09, suspension visible, pertes et confidentialité. | Schéma fermé et tests de saturation/rotation sentinelles. |
| GAP-L04-BASELINE-01 | Coordinateur/hôte : baseline successor 21 paths versus D0, preuve de status scoped et candidate dirty. | Hashes recapturés, snapshots D23, comparaison D0; aucun dirty global n'est nié. |
| GAP-L04-PLAN-01 | Plan writer après DESIGN READY : P29 et matrice de preuves. | PLANS exact, parent du DESIGN réellement READY, revue indépendante CLEAN. |
| GAP-L04-PRODUCT-PROOF-01 | Validation : vrais workers GPU/CPU, UI, archives, import/live, interruption. | Campagnes exécutées ou NOT RUN avec limites explicites. |

Tous ces GAP sont ouverts. Les commandes précédemment rapportées pour la candidate partielle ne les ferment pas.
