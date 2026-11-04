Avancée : Auto conforme par décision utilisateur ; quatre scénarios PASS et durées conformes.
Blocage : aucun à la préparation ; tag, fusion et N1 main non encore exécutés.
Décision : mesures sur 28b96d9a, fusion conditionnelle sans push, aucune correction de code/test.

# POLL-INTEGRATION — décision par mode et suite, 4 novembre 2026

L'utilisateur remplace le critère de médiane : Auto CONFORME par analyse par
mode (rapide candidat 25,19 s / base 25,23 s, lent 40,23 / 40,57 s, modes
3/2 identiques). Valeurs retenues par sa décision ; essai candidat 28,34 s
conservé en observation, sans valeur décisionnelle. Les dix mesures et leurs
relevés restent dans le bilan précédent, sans réécriture ni attribution causale.

| Corps sur candidat exact 28b96d9a | B | Plafond B×1,10 | Mesures / verdict |
| --- | ---: | ---: | --- |
| Reprise interrompue | 13,35 s | 14,685 s | 9,00 s / PASS |
| Direct Turbo CPU | 42,54 s | 46,794 s | 31,71 s / PASS |
| Import long FR | 168,57 s | 185,427 s | 143,63 s / PASS |
| Import long EN | 187,17 s | 205,887 s | 161,19 s / PASS |

Durées : corps Rust hors compilation, banc/profils B inchangés. PASS
fonctionnel requis ; dépassement de durée : deux essais supplémentaires
du scénario, trois mesures présentées sans décision automatique. Aucun
diagnostic causal ou correctif autorisé. Cache isolé conservé, pas de
tentative de suppression ; ressources et autres travaux non suivis préservés.

SelfTest natif et trois préflights réussis sur le candidat exact, puis quatre
corps PASS (trois commandes gardées, codes 0). Aucun dépassement : pas de
mesure supplémentaire. Preuves target/n2-native/logs/ :
PollInterruptedResume-092910a649e64f45a47eddea35b364cf,
PollTurboCpuDirect-46c58780cd124cad94b33c10ac591888,
PersonalLongCpu-206605ee2eef4df5944b4c0bc865ed39, reçus et stdout associés.
Retour sur refacto/poll f8e1eb90 après fin des processus : seuls des documents
diffèrent de 28b96d9a, sources et banc testés identiques.

Si quatre PASS sans écart inexpliqué : revue des preuves et préconditions
FUSION-PILE, fetch origin/main serveur, main local égal à origin/main et
ancêtre, suivis propres, aucun autre agent actif. Classification hors poll
antérieure CLEAN, sources inchangées depuis 28b96d9a. Tag avant-fusion-poll
sur main puis merge --no-ff refacto/poll. N1 main : 754/149/0 attendue,
Structure, format, Clippy. Aucun push. Compte rendu non encore figé.

Fetch origin main réussi : main=origin/main=FETCH_HEAD=9cd15ca6, ancêtre
de refacto/poll. Suivis propres, ressources et deux documents non suivis de
main préservés, tag absent ; aucun processus de campagne actif. Revue
préf fusion indépendante CLEAN sur les quatre reçus, durées et classification.
Archive target/n2-native/archive/POLL-INTEGRATION-DECISION-20261104.zip :
52 entrées, 677 048 octets source / 204 369 archivés ; nombre, tailles et
SHA-256 de chaque entrée vérifiés. SHA-256 archive :
BB68138409B28B254B83A092FC3A3EDA76EBC8E39F5DC51D32013AEA36833414.
