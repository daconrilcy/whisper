# Plans Whisper — P-WHISPER-01

Statut du candidat : DRAFT avant revue. Auteur du contenu : `/root/plans_writer` (proposition PW-WHISPER-01 v1), synthèse documentaire par le coordinateur. Première lignée PLANS (`base_candidate_id=null`), périmètre 2, parent `D-WHISPER-19`, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

Lire le statut courant dans `../../state.json` après `check-state`, et la revue parent `../../transports/T-WHISPER-REVIEW-19/review-D19.json`. Les mentions DRAFT/OPEN de D19 décrivent son gel avant promotion. Le contenu reprend REQ-01..25, UC/AC-01..20 et TECH-D18-01..08 ; D19/51 prévaut sur les revendications E2 retirées de D18. `03_CENTRAL_PACK_IMPACT.md` motive la conservation de D19 malgré les nouvelles règles centrales.

## Ordre de transfert

| Lot | Sortie démontrable | Prérequis de lot |
| --- | --- | --- |
| L-WHISPER-00 | Bootstrap, frontières et contrats compilables | Aucun |
| L-WHISPER-01 | WAV réel → worker CPU → TXT/SRT → historique visible | L00 CODE |
| L-WHISPER-02 | Journal durable, FIFO et reprise sans départ spontané | L01 EXECUTION |
| L-WHISPER-03 | Live, VAD, MP3 par passage, Stop + Reprise | L02 EXECUTION |
| L-WHISPER-04 | GPU strict/Auto, générations et supervision | L03 EXECUTION |
| L-WHISPER-05 | Réglages, tray, raccourci, autostart et archives | L04 EXECUTION |
| L-WHISPER-06 | Installation, payload et premier usage offline | L05 EXECUTION |
| L-WHISPER-07 | Qualification intégrée du périmètre V1 | L06 EXECUTION |

Le graphe est acyclique et séquentiel. L01 exige les API et artefacts réels de L00 ; les liens EXECUTION exigent la réussite prouvée du fournisseur. Toutes ces preuves futures sont NOT RUN. Chaque lot exige en outre les entrées DOCUMENT DESIGN/PLANS/revues exactes et les contrats applicables. Les chemins partagés (Cargo, ports, scheduler, journal, archives, racines) empêchent de présumer une exécution parallèle.

## Autorités et conditions

L'utilisateur a autorisé préparation et revue des plans. L'implémentation n'est pas autorisée. Le railguard `D-WHISPER-19/11_RAILGUARD_PROPOSAL.md` est proposé, non actif. Avant L00 : autorisation de coder portant sur chemins/lots, activation attestée du railguard par l'autorité compétente (chemin, hash de proposition et fichier actif, date, équivalence normative), état Git inventorié et preflight de `02_VERIFICATION_AND_PREFLIGHT.md`. L00 est le premier lot qui pourrait alors commencer ; L01 est le premier parcours produit de bout en bout.

Les choix physiques de crates/packages/features, UI/décodeur, capture et tailles de files, profils codec, cadence du diagnostic Q-04, rétention des logs Q-09 et geste manuel Q-06 restent des détails réversibles à trancher par le responsable technique avant leurs lots. Le protocole de nouveaux micros représentatifs nécessite un mandat avant L07 ; DEC-28 ne couvre que les WAV existants pour le cadrage. Aucun détail ne peut changer silencieusement le stockage, les frontières, le périmètre GPU ou les garanties : une découverte structurante revient en DESIGN_CHANGE_REQUIRED.

## Carte

`01_LOTS.md` fixe ownership, couverture et acceptation ; `02_VERIFICATION_AND_PREFLIGHT.md` fixe commandes proposées et preuves futures ; `03_CENTRAL_PACK_IMPACT.md` documente le delta de règles. Sources fondatrices : D19/02, 07, 08, 11, 32, 47, 50, 51 ; apport original `T-WHISPER-ARCH-D18-FINAL`, revue R19 et état contrôlé. Les campagnes du produit et de livraison restent NOT RUN.
