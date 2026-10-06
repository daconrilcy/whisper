# Plans Whisper — P-WHISPER-08

Statut : DRAFT soumis à une revue indépendante propre à P07. Base immuable : P-WHISPER-08, digest `d929d3b13efb4a213dfbfac3e0ae23415fb4433a8692a0fd37d968465a2f070d`. Parent DESIGN : D-WHISPER-19, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

P07 corrige la portée d’exécution de L02 : quinze chemins explicites couvrent application/ports, journal/FIFO/récupération, préparation d’archive, worker CPU réel et consumer desktop. L01 completed et ses preuves restent les prérequis de progression ; les campagnes L02 sont PROPOSED / NOT RUN. Q-07 v1 est répondu et doit être appliqué au parcours MP3 réel, pas seulement au proxy ou à la prévalidation. Le préflight antérieur limité à sept chemins ne vaut pas pour ce périmètre.

P06 reste immuable avec son verdict historique. P07 demeure DRAFT jusqu’à la revue indépendante du corpus exact et au contrôle/promotion correspondants. D19 conserve son identité et son périmètre accepté. `08_L02_INTEGRATION_CONTRACTS.md` précise les frontières, entrées/sorties et étapes L02. La source du mandat étendu, la réponse Q-07 et les preuves L01 sont indexées dans `05_SOURCE_AND_RULE_INDEX.md`.

Cette révision conserve les exigences, critères d’acceptation, décisions et tâches existants. Elle n’élargit ni les plafonds mémoire ni le périmètre GPU. Les vérifications produit restent NOT RUN.

## Lots

| Lot | Sortie | Dépendance |
|---|---|---|
| L-WHISPER-00 | Socle, contrats et builds isolés livrés selon son ledger | Aucune |
| L-WHISPER-01 | WAV réel → application core → worker CPU → TXT/SRT → historique visible | L00 CODE |
| L-WHISPER-02 | Journal durable, FIFO, import MP3 réel et reprise sans départ spontané | L01 EXECUTION |
| L-WHISPER-03 | Live, VAD, MP3 par passage, Stop + Reprise | L02 EXECUTION |
| L-WHISPER-04 | GPU strict/Auto, générations et supervision, IPC v2 | L03 EXECUTION |
| L-WHISPER-05 | Réglages, tray, raccourci, autostart et archives | L04 EXECUTION |
| L-WHISPER-06 | Installation, payload et premier usage offline | L05 EXECUTION |
| L-WHISPER-07 | Qualification intégrée V1 | L06 EXECUTION |

DAG séquentiel ; DOCUMENT exige les candidats/revues exacts et contrats applicables, CODE les API/sorties hashées, EXECUTION le résultat prouvé du fournisseur. Les chemins partagés interdisent une parallélisation présumée. Les campagnes L01–L07 restent NOT RUN, sauf preuves de clôture L01 enregistrées hors de ce plan.

## Prérequis L02

L01 est completed selon son ledger et la preuve d’exécution courante dans le checkpoint. D19/P07 et leurs revues doivent être exacts ; DETAIL-P01 v3, Q-05 v1 et Q-07 v1 doivent être answered/acceptés selon leur autorité. Le préflight doit vérifier le railguard actif, l’identité du dépôt, les sorties L01 et les quinze chemins. L’ancien préflight sur sept chemins est historique et ne qualifie pas cette allowlist.

## Autorité et état

HUMAN-USER a autorisé l’implémentation L02 sur les quinze chemins listés dans `sources/progress/USER_AUTHORIZATION_L02.md`, après PLANS P07 revu et préflight PASS. Cette autorisation de coder ne confère aucun PASS aux tests/campagnes produits. Le statut courant contrôlé reste porté par `docs/predev/state.json`.

## Carte documentaire

`01_LOTS.md` définit ownership/AC ; `02_VERIFICATION_AND_PREFLIGHT.md` les commandes et preuves ; `04_OPEN_DETAILS.md` les réponses ; `05_SOURCE_AND_RULE_INDEX.md` les sources exactes ; `08_L02_INTEGRATION_CONTRACTS.md` le contrat d’intégration L02. Les sources et règles héritées P06 restent attribuées et identifiées.


`09_L02_SOURCE_BASELINE.md` fixe les sources initiales, leur provenance, les six chemins futurs absents et les preuves L01 incorporées. Ces snapshots ne sont pas des hashes de sortie future.

`09_L02_SOURCE_BASELINE.md` fixe les sources initiales, provenance, six chemins futurs absents et preuves L01. Aucun contrôle P06/P07 ne vaut revue ou préflight P08.