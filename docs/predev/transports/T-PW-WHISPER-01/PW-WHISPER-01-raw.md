# PW-WHISPER-01 — apport brut condensé v1

Actor/auteur : `/root/plans_writer`, rôle `rust_plan_writer`. Date : 2026-10-05. Activité en lecture seule ; aucun fichier modifié.

Base intellectuelle : `D-WHISPER-19`, périmètre 2, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. Candidat proposé : `P-WHISPER-01`, première lignée PLANS, `base_candidate_id=null`, `design_candidate_id=D-WHISPER-19`. Statut de l’apport : DRAFT, non persisté.

Sources : `docs/predev/state.json` ; D19 documents 02/07/08/09/10/11/15/32/47/49/50/51 ; `transports/T-WHISPER-ARCH-D18-FINAL/arch-D18-final-raw.md` ; `transports/T-WHISPER-REVIEW-19/review-D19.json` ; skill rust-predev-design, cinq règles centrales, contrats handoff/engineering/workflow/deliverable-quality et runtime-qualification.

Contrôles personnels : verify-manifest D19 PASS avec digest ci-dessus ; check-state PASS, phase DESIGN ; état D19 READY et R19 CLEAN liés au digest. Git status vide lors du contrôle ; aucun Cargo/AGENTS/RAILGUARD produit trouvé.

Ownership : propositions d’index, lots, campagnes et impact. Le coordinateur possède fusion/état ; l’hôte autorisé possède persistance ; le reviewer indépendant possède findings/verdict. Aucune décision produit acceptée par cet auteur.

## Index et couverture proposés

DAG séquentiel sans parallélisation : L00→L01→L02→L03→L04→L05→L06→L07. L01 dépend de L00 CODE ; les autres arêtes sont EXECUTION, preuves futures NOT RUN. Tous héritent des entrées DOCUMENT exactes et du preflight.

- L00 bootstrap/contrats : TECH-D18-01/04/08 ; aucun REQ/AC produit déclaré réalisé.
- L01 WAV→worker CPU→TXT/SRT→historique visible : REQ-03/06/12/13/24 ; AC-03/06/10/19.
- L02 journal/file FIFO/recovery : REQ-03/04/12/13/16/17/18 ; AC-03/04/10/11/12/13.
- L03 live/VAD/MP3/Stop+Reprise : REQ-01/02/03/04/09/10/12/18/25 ; AC-01/02/03/04/08/10/13/20.
- L04 GPU/Auto/supervision : REQ-07/08/11/14/15/16/18/24 ; AC-07/09/11/13/19.
- L05 réglages/OS/archives : REQ-06/14/15/16/19/20/21/22/23 ; AC-06/11/14/15/16/17/18.
- L06 installation/package/offline : REQ-05/07/08/24 ; AC-05/07/19.
- L07 qualification intégrée : REQ-01..25 ; AC-01..20.

Contrats partagés : ports/application, scheduler, archive/journal, racines, manifests/lockfile. Exécution séquentielle proposée pour prévenir collisions.

## Impact et limites

Comparaison réelle : neuf différences sur 21 références du freeze D19, concernant trois profils et six sources skill/contrats. Les ajouts renforcent provenance exacte, séparation de l’exécutant SPIKE, preuve stimulus/branche/effet et lecture du statut courant. Aucun changement TECH-D18-01..08 identifié : conserver D19 proposé.

Les bancs historiques attribués au coordinateur précèdent la règle renforcée de séparation des exécutants. Ne pas réécrire cette attribution ni revendiquer conformité rétroactive ; soumettre explicitement cette limite au reviewer PLANS. R19 CLEAN reste la revue historique exacte.

## Questions et suite

Détails avant preflight : disposition/packages/UI/décodeur ; capture et capacités ; Q-07 codecs/bitrate ; Q-04 supervision ; Q-09 rétention logs ; Q-06 action manuelle ; mandat futur corpus micro représentatif. Aucun nouveau besoin inventé.

Restant : persister corpus et TRANSPORT, vérifier reçus/hash/checkpoint ; matérialiser REQ/UC/AC acceptés et tâches/refs depuis sources produit ; vérifier manifeste PLANS et parent ; revue indépendante puis corrections/revue exacte. Autorisation coding absente ; railguard proposé inactif ; toutes campagnes produit NOT RUN. L00 serait le premier lot après autorisations et preflight. Aucun CLEAN/READY autoattribué ; aucun DESIGN_CHANGE_REQUIRED identifié.
