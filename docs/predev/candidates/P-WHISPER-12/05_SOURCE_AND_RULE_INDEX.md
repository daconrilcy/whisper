# Sources et règles — P-WHISPER-10

Base immuable P08 digest `d79bec1ab10a13e37adee551fe05cf834e43447617abccebdef6ca1460346e6f`. Parent D19 digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. P10 DRAFT ; revue/gate/préflight P10 exacts requis.

## Corpus

Le manifeste P10 inclut l’intégralité de P07 (DESIGN parent, sources/règles, demandes utilisateur, Q-07/DETAIL/Q05, contrats L01, matrice CPU), corrections P08 et snapshots/proofs ajoutés. Le champ base P08 n’emporte aucun héritage implicite. Manifest/proposal/receipt historiques ne remplacent pas métadonnées P10. P06/P07/P08 restent immuables.

Les références historiques dans les sources restent intactes ; seules les instructions normatives de P10 visent P10. `USER_AUTHORIZATION_L02.md` autorise les quinze chemins après le plan courant revu et gate PASS.

## Baseline et L01

`09_L02_SOURCE_BASELINE.md` inventorie les snapshots exacts, SHA, HEAD `5fee290b8e74205fca94d4d2b478d430680132d4`, neuf sources présentes, six futurs absents et supports hors allowlist. La preuve L01-CLOSURE-02 est copiée texte/JSONL exact sous `sources/progress/`; PNG gardés à l’origine avec hashes. Les neuf sources concordent avec outputs L01. Preuve liée historiquement à D19/P06/fingerprint L01 ; pas revue P10 ni campagne L02. Les snapshots initiaux ne sont jamais réécrits pour masquer progression ; outputs futurs vont au ledger séparé.

Les helpers/règles sont ceux manifestés ; comparer le helper exécuté à son freeze avant usage. Le coordinateur lie contributions brutes/receipts au checkpoint contrôlé ; reviewer vérifie le corpus P10 exact et ferme les findings. L02 est completed avec la preuve CLOSURE-03 enregistrée dans state.json ; le modèle D19 est NOT RUN sur ce candidat faute de modèle local. Les campagnes live de L03 restent NOT RUN.


## Sources DETAIL-P02

La contribution attribuée à l’architecte et l’acceptation utilisateur exacte sont copiées sous `sources/DETAIL-P02-technical-contribution.md` et `sources/user-acceptance.md`; leur transport canonique et hashes figurent sous `transports/T-WHISPER-DETAIL-P02-ACCEPT-01/`. Le digest parent D19 reste inchangé.


La réponse auteur complète reconstruite R2, qui supersède l’ancienne synthèse, est dans `sources/ARCH-DETAIL-P02-v1-R2.md` et le transport `T-WHISPER-ARCH-DETAIL-P02-R2/` avec reçu/hash. L’acceptation utilisateur directe reste indépendante sous `T-WHISPER-DETAIL-P02-ACCEPT-01/`.


## Résolution Q-04 / Q-06 / Q-09 — P11

Apport architectural `/root/q04_q06_q09_arch`, `ARCH-Q04-Q06-Q09-v1`, transcription structurée attribuée sous `sources/architecture/ARCH-Q04-Q06-Q09-v1.md`. La demande utilisateur de résolution et l’acceptation « Rotation bornée (Recommended) » sont capturées sous `sources/authorization/user-Q04-Q06-Q09-authorization.md`. Le TRANSPORT canonique `P10-HOST-WORK/transports/T-WHISPER-Q04-Q06-Q09-ANSWER-01/` contient réponse, provenance et sources d’autorité ; manifeste digest `0d81ad463eb1bc26cd2138f616d706a584474da64fda6efa49a5b21cb33584e2`.

Q-06 s’appuie aussi sur D19 `01_PRODUCT_BRIEF.md`, `06_SOURCES_AND_DECISIONS.md`, `10_QUESTIONS_RISKS_AND_READINESS.md` (SHA-256 `6a0f5188178a942c6da82d5452e9dab0dfb32b733d8c66d6f58be692422227b6`) et sur la résolution DETAIL-P03 au transport `P10-L03-CLOSURE-WORK/transports/T-WHISPER-DETAIL-P03-RESOLUTION-01/` (SHA-256 du document `d80e2aa581926981f5ff138d9cec82940fcac626d46707c9b033e32aef766e98`). D19/P10 restent immuables. Les preuves d’implémentation, d’intégration, d’UI réelle et toutes campagnes restent NOT RUN.


## Continuité du corpus P12

P12 inclut les fichiers de payload complets du manifeste P10 et les sources de réponse/acceptation Q04/Q06/Q09 conservées par P11. La source architecturale et les captures utilisateur sont indexées dans les sections ci-dessus. Transport canonique : `P10-HOST-WORK/transports/T-WHISPER-Q04-Q06-Q09-ANSWER-01/transport-manifest.json`; manifeste de réponse digest `0d81ad463eb1bc26cd2138f616d706a584474da64fda6efa49a5b21cb33584e2`. P10, P11 et D19 restent immuables. Le candidat P11 était une étape partielle non promue ; P12 est le candidat complet soumis à sa propre revue indépendante exacte.
