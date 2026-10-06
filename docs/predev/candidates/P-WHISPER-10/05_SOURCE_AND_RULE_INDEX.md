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
