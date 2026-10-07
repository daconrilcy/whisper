# Sources et règles — P-WHISPER-14

Base immuable PLANS P13 digest `fbd7b0d2033cbdda8f362921539fdf3349e1a6c6a976ad1ebf82e4840cc203ad`. Parent D19 digest exact `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. P14 DRAFT ; revue indépendante P14 requise. P13 a reçu FINDINGS dans `transports/T-WHISPER-PLANS-REVIEW-13/transport-manifest.json` (digest `512a5cd18893fdfde5e38c6770421a752170e6893614bfa6f102129a8849d251`, manifest SHA `b49a4f94a443098032f03af8aef26ba750e2b25347852b5ed2ab2838b71483c9`). Le rapport exact `R-WHISPER-PLANS-13-v1.md` SHA `c19e3909f46e9143cf2e04c68b9b7f290828225ba8815617fda571adebda1d33` est reproduit sous `sources/reviews/`. Le contenu d'index qui suit est hérité des sources P10 et décrit leur provenance historique, non le statut actif.

## Corpus

Le manifeste P10 inclut l’intégralité de P07 (DESIGN parent, sources/règles, demandes utilisateur, Q-07/DETAIL/Q05, contrats L01, matrice CPU), corrections P08 et snapshots/proofs ajoutés. Le champ base P08 n’emporte aucun héritage implicite. Manifest/proposal/receipt historiques ne remplacent pas métadonnées P10. P06/P07/P08 restent immuables.

Les références historiques dans les sources restent intactes ; seules les instructions normatives de P10 visent P10. `USER_AUTHORIZATION_L02.md` autorise les quinze chemins après le plan courant revu et gate PASS.

## Baseline et L01

`09_L02_SOURCE_BASELINE.md` inventorie les snapshots exacts, SHA, HEAD `5fee290b8e74205fca94d4d2b478d430680132d4`, neuf sources présentes, six futurs absents et supports hors allowlist. La preuve L01-CLOSURE-02 est copiée texte/JSONL exact sous `sources/progress/`; PNG gardés à l’origine avec hashes. Les neuf sources concordent avec outputs L01. Preuve liée historiquement à D19/P06/fingerprint L01 ; pas revue P10 ni campagne L02. Les snapshots initiaux ne sont jamais réécrits pour masquer progression ; outputs futurs vont au ledger séparé.

Les helpers/règles sont ceux manifestés ; comparer le helper exécuté à son freeze avant usage. Le coordinateur lie contributions brutes/receipts au checkpoint contrôlé ; reviewer vérifie le corpus P10 exact et ferme les findings. L02 est completed avec la preuve CLOSURE-03 enregistrée dans state.json ; le modèle D19 est NOT RUN sur ce candidat faute de modèle local. Les campagnes live de L03 restent NOT RUN.


## Sources DETAIL-P02

La contribution attribuée à l’architecte et l’acceptation utilisateur exacte sont copiées sous `sources/DETAIL-P02-technical-contribution.md` et `sources/user-acceptance.md`; leur transport canonique et hashes figurent sous `transports/T-WHISPER-DETAIL-P02-ACCEPT-01/`. Le digest parent D19 reste inchangé.


La réponse auteur complète reconstruite R2, qui supersède l’ancienne synthèse, est dans `sources/ARCH-DETAIL-P02-v1-R2.md` et le transport `T-WHISPER-ARCH-DETAIL-P02-R2/` avec reçu/hash. L’acceptation utilisateur directe reste indépendante sous `T-WHISPER-DETAIL-P02-ACCEPT-01/`.


## Réponses Q-04 / Q-06 / Q-09 — lignée P14

La nouvelle contribution rédigée par l'architecte `ARCH-Q04-Q06-Q09-v2` est conservée sous `sources/architecture/ARCH-Q04-Q06-Q09-v2.md` et transportée dans `T-WHISPER-Q04-Q06-Q09-ANSWER-02`. Ce document est attribué à son auteur; il indique qu'il ne s'agit pas d'un export de sa contribution précédente. La source utilisateur de Q09 contient uniquement le choix exact « Rotation bornée (Recommended) »; les limites de l'option sont distinguées des paramètres techniques proposés par l'architecte. L'autorité de fermeture et l'état actif restent ceux du registre tant que le checkpoint P14 n'est pas validé et promu.

Q-06 conserve l'invariant D19/DETAIL-P03 : attente visible/réactive si l'enfant ne confirme pas son arrêt, aucune terminaison forcée. Les références et SHA des sources D19/P03 sont celles consignées dans le contrat `13_L04_DETAIL_CONTRACTS.md`; D19 et P10 restent immuables. Toutes validations d'implémentation/runtime, UI, rétention et campagnes restent NOT RUN.

## Continuité du corpus P14

P14 est un successeur complet de P13, lequel restaurait les payloads hérités P10. P10, P11, P12 et D19 ne sont pas modifiés. P13 a reçu FINDINGS; P14 doit être revu exactement et ne reçoit aucun verdict anticipé. Les références P10/P11/P12 ailleurs dans le corpus sont historiques sauf lorsqu'elles sont explicitement citées comme base immuable.


