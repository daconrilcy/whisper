# Sources et règles — P-WHISPER-09

Base immuable P08 digest `d79bec1ab10a13e37adee551fe05cf834e43447617abccebdef6ca1460346e6f`. Parent D19 digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. P09 DRAFT ; revue/gate/préflight P09 exacts requis.

## Corpus

Le manifeste P09 inclut l’intégralité de P07 (DESIGN parent, sources/règles, demandes utilisateur, Q-07/DETAIL/Q05, contrats L01, matrice CPU), corrections P08 et snapshots/proofs ajoutés. Le champ base P08 n’emporte aucun héritage implicite. Manifest/proposal/receipt historiques ne remplacent pas métadonnées P09. P06/P07/P08 restent immuables.

Les références historiques dans les sources restent intactes ; seules les instructions normatives de P09 visent P09. `USER_AUTHORIZATION_L02.md` autorise les quinze chemins après le plan courant revu et gate PASS.

## Baseline et L01

`09_L02_SOURCE_BASELINE.md` inventorie les snapshots exacts, SHA, HEAD `5fee290b8e74205fca94d4d2b478d430680132d4`, neuf sources présentes, six futurs absents et supports hors allowlist. La preuve L01-CLOSURE-02 est copiée texte/JSONL exact sous `sources/progress/`; PNG gardés à l’origine avec hashes. Les neuf sources concordent avec outputs L01. Preuve liée historiquement à D19/P06/fingerprint L01 ; pas revue P09 ni campagne L02. Les snapshots initiaux ne sont jamais réécrits pour masquer progression ; outputs futurs vont au ledger séparé.

Les helpers/règles sont ceux manifestés ; comparer le helper exécuté à son freeze avant usage. Le coordinateur lie contributions brutes/receipts au checkpoint contrôlé ; reviewer vérifie le corpus P09 exact et ferme les findings. Campagnes L02 restent NOT RUN.
