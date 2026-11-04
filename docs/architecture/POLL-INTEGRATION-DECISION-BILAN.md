Avancée : qualification native complémentaire PASS ; fusion locale et N1 main réussies.
Blocage : aucun pour l'intégration ; bimodalité et avertissements conservés comme limites.
Décision : lot clos, bilan figé le 4 novembre 2026 ; aucun push, aucune correction de code/test.

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

Corps Rust hors compilation, banc/profils B inchangés ; tous codes 0,
aucun dépassement, aucun essai supplémentaire nécessaire. SelfTest natif et
trois préflights réussis. Reprise contrôlée, direct à capture simulée avec
vrai moteur CPU, imports natifs CPU ; aucune nouvelle qualification Windows.

Preuves dans C:/wt/poll/target/n2-native/logs/ :
PollInterruptedResume-092910a649e64f45a47eddea35b364cf,
PollTurboCpuDirect-46c58780cd124cad94b33c10ac591888,
PersonalLongCpu-206605ee2eef4df5944b4c0bc865ed39, reçus et stdout associés.
Depuis 28b96d9a, seuls des documents ont changé. Classification hors poll
CLEAN acquise : aucune ligne produit modifiée hors poll, registre ajouté
seulement, garde/profils/tests et documents dans le périmètre arbitré.

**Fusion :** fetch serveur réussi, main=origin/main=FETCH_HEAD=9cd15ca6,
ancêtre de la branche. Revue préfusion indépendante CLEAN ; suivis propres,
aucun autre agent actif, non-suivis préservés. Tag **avant-fusion-poll** sur
9cd15ca6 ; merge **3ffc99ca**, parents 9cd15ca6/65a903bc, arbre identique à
la tête fusionnée. Aucun push.

**N1 sur main 3ffc99ca :** SelfTest35/79, Structure951/951 (166 native-gated
recensés), format et Clippy codes 0 ; avertissements présents, sans prétention
de Clippy strict. Défaut705/128/0, common11/1/0, CPU12/9/0, Parakeet26/11/0 :
**754 réussites /149 ignorés /0 échec**, preuves sous
C:/dev/whisper/target/parakeet-tests/poll-integration-main/. Cache persistant
main préparé depuis celui du worktree, sans réseau ni changement de dépendance.

**Archives vérifiées** (nombre, tailles et SHA-256 de chaque entrée) :
- N2 worktree, target/n2-native/archive/POLL-INTEGRATION-DECISION-20261104.zip :
  52 entrées, 677 048/204 369 octets source/ZIP, SHA-256
  BB68138409B28B254B83A092FC3A3EDA76EBC8E39F5DC51D32013AEA36833414.
- N1 main, poll-integration-main/archive/POLL-INTEGRATION-MAIN-N1-20261104.zip :
  175 entrées, 408 375/132 994 octets, SHA-256
  A4E99CCFBB0F5AD53E3BE8F7835AEE2E289A9484CD4B34550215FDAEDE5D4260.

Cache isolé worktree conservé : 651 827 048 octets ; aucune suppression
tentée. Commande utilisateur après vérification du chemin et absence de campagne :
`Remove-Item -LiteralPath 'C:\wt\poll\target\n2-native\cargo-home' -Recurse -Force`.
Caches persistants/partagés, ressources et preuves préservés ; 559 824 293 888
octets libres. Bimodalité consignée hors périmètre pour diagnostic séparé.
Dernière relecture indépendante du compte rendu et des preuves N1 : CLEAN.

[Préservation des preuves et retrait du worktree après push](POLL-INTEGRATION-NETTOYAGE.md).
