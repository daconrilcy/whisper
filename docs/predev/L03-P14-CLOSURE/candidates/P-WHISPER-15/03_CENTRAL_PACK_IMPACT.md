# Impact courant du pack central — P-WHISPER-15

P15 DRAFT, base P14 exacte, parent D19 conservé. À la préparation de P15, une seule entrée du pack figé P14 diffère de la source centrale effective :

- `skills/rust-predev-design/scripts/predev_control.py`
- SHA P14 : `2fe0d71e151fb35d86551808a6b9db491fa1f6f4b59f4e67f491db5828d3b485`
- SHA effectif : `43191671e4bca55d5e23419e1125c2b2d524cd2e6b0530294cf86b664a49d3a4`

CHANGE-P15-01 actualise uniquement ce snapshot dans le nouveau corpus et régénère le pack-manifest effectif. P14 et D19 restent immuables.

Le correctif concerne la protection des références lors de `promote_checkpoint` : les références de code courant dans `code_state` sont résolues sous `code-root`, tandis que les références documentaires conservent leur racine documentaire. Son objectif est de permettre une promotion validée lorsque ces deux racines sont distinctes. La portée exacte du code et de ses preuves reste à examiner dans la revue indépendante P15.

Un passage antérieur de 42 tests du contrôleur/hôte a été observé dans la sortie de la session précédente. Son journal brut n’est pas conservé dans le transport P15; la ligne est donc une trace de contexte, pas une preuve de validation reproductible liée au candidat. Aucun nouveau test n’est revendiqué ici et ce résultat ne valide pas le produit L04.

KEEP_D19 est proposé : le delta concerne la procédure de promotion, la provenance et la représentation des sorties attendues; il ne modifie aucun REQ/AC, TECH-D18, DEC ou invariant produit. Le reviewer indépendant P15 doit vérifier cette conclusion et l’absence de changement normatif caché. Une preuve d’incompatibilité de design déclenche DESIGN_CHANGE_REQUIRED et suspend les lots touchés.

Les sections suivantes sont historiques. Elles décrivent l’impact des packs de versions antérieures et ne remplacent pas le gel effectif P15.

# Impact du pack central — continuité P04–P06

Proposition P-WHISPER-04, base D-WHISPER-19. Le manifeste DESIGN (221 fichiers, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`) et `state.json` passent les contrôles actuels. L'état courant est DESIGN READY avec R-WHISPER-DESIGN-19 CLEAN. Les mentions DRAFT du candidat immuable sont historiques.

Neuf des 21 règles figées dans D19 diffèrent des sources centrales actuelles : `agents/rust_architect.toml`, `rust_design_reviewer.toml`, `rust_predev_orchestrator.toml`, `skills/rust-predev-design/SKILL.md`, ses références `deliverable-quality.md`, `engineering-contract.md`, `handoff-contract.md`, `workflow-schema.md`, et `skills/rust-predev-orchestration/SKILL.md`. Les cinq `agent-rules`, le profil plan writer, les profils framer/analyst/domain, `runtime-qualification.md` et les deux helpers figés sont identiques.

Les ajouts exigent une comparaison source/proposition/TRANSPORT pour tout apport dit exact, un exécutant SPIKE distinct et séparément autorisé, la vérification stimulus/branche/effet, la séparation DESIGN_FEASIBILITY/PRODUCT_VALIDATION/DELIVERY_QUALIFICATION, et la lecture du statut courant dans `state.json`. Ils renforcent la provenance et les prochains transferts. Ils ne modifient ni REQ-01..25 ni TECH-D18-01..08, les contrats, les preuves observées ou les limites établies par R19. L'avis indépendant d'impact conclut KEEP_D19 ; aucune nouvelle identité DESIGN n'est nécessaire.

Limite historique : les bancs D18/D19 ont été attribués au coordinateur avant la nouvelle règle de séparation. Cette attribution reste visible dans D19/49 et D19/51 ; elle n'est pas réécrite en conformité rétroactive. Le reviewer R19 a examiné code et logs des scénarios effectivement exercés ; ses conclusions portent seulement sur la faisabilité bornée. Les futurs bancs requièrent mandat et exécutant distincts de la coordination, de l'architecture, de la revue et de l'hôte documentaire. Toute vérification du produit ou de sa livraison demeure NOT RUN.

Si une preuve nouvelle révèle une erreur technique de D19, appliquer CHANGE et suspendre les lots concernés ; un simple changement de procédure du pack ne remplace pas le CLEAN historique par silence. Railguard proposé inactif et autorisation d'implémenter absente.

## Provenance corrigée pour P2

P2 fige séparément les sources D19 02/07/08/10/11/32/47/50/51 effectivement utilisées, le message utilisateur autorisant préparation/revue PLANS, le manifeste DESIGN parent, le résultat indépendant KEEP_D19 et les règles centrales actuelles, y compris `verify_raw_handoff.py`. `05_SOURCE_AND_RULE_INDEX.md` donne leurs origines et les empreintes. Le gel D19/15 reste historique et immuable. Les TRANSPORT exacts PW/IMPACT/REVIEW sont hors corpus mais dans le checkpoint. Aucun choix TECH-D18-01..08 ne change. Les anciens bancs restent attribués au coordinateur, sans conformité rétroactive au nouveau processus. Seul le reviewer sur P2 exact peut fermer les findings P1.


## Réconciliation historique P04–P05 et gel P06

Constats de différences de règles datant de P03 sont historiques. P04 a actualisé six copies. P05 a inclus execution-evidence et restauré les 23 snapshots byte-identiques. P06 gèle ces sources après comparaison ; son propre manifest/freeze sont ceux à contrôler. Le contrôleur effectif conserve une limite roots séparées sur CODE.outputs, décrite avec preuve et méthode vue unifiée en `02_VERIFICATION_AND_PREFLIGHT.md`. Le railguard D19 est actif et attesté pour L00 ; réattester au préflight L01. L’ancien mandat L01 couvre sept entrées seulement ; la capture de la demande humaine du 2026-10-06 permet au coordinateur de consigner séparément le nouveau périmètre de 21 chemins. P04 n’active pas de mandat à la place du state.
# P15 pack refresh — 2026-10-07

P15 snapshots all 23 files from the current central pack. Its refreshed manifest digest is `b5cd6730e16a4b1cd9c2fa38275d78502b94d7969f0b7602820d194ecc87881f`. The previous P14 pack digest was `2460bd0dbf0fee3cf7445f4e65ad4dbba29f8bc8765df705c88cba44b0191ef3`; the controller file in that baseline had SHA-256 `2fe0d71e151fb35d86551808a6b9db491fa1f6f4b59f4e67f491db5828d3b485`, while the current controller is `43191671e4bca55d5e23419e1125c2b2d524cd2e6b0530294cf86b664a49d3a4`. This documents pack drift and refresh; it does not claim that the central helper's prior test output is a persisted P15 validation artifact.
