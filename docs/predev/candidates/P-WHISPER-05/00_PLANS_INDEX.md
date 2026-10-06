# Plans Whisper — P-WHISPER-05

Statut : DRAFT soumis à revue indépendante. Base PLANS : P-WHISPER-03, digest `2af7eaf55c20184bfdeb1035dab7e64593799c94e0bbad5ccfafea65c04e8ead`. Parent DESIGN : D-WHISPER-19, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. Auteur des corrections : `/root/correct_p03`, rôle rust_plan_writer, lecture seule. L’auteur ne ferme aucun finding.

Cette version traite WHISPER-PLANS-004..007 et intègre DETAIL-P01 v3, ARCH-L01-INTEGRATION v2, ARCH-L01-BOUNDS v1 et R-WHISPER-D19-COMPAT-01 v2. La revue COMPAT est bornée à INTEGRATION et ne valide pas les nombres BOUNDS ni ce candidat PLANS. P03 reste immuable avec son verdict FINDINGS ; P02 reste le dernier PLANS READY du checkpoint tant que P04 n’est pas revu et promu.

Le statut courant vient de `../../state.json` contrôlé. L00 est completed selon son ledger ; L01–L07 planned, validations produit NOT RUN. Les mentions historiques de D19/P03 ne remplacent pas l’état. Le plan conserve REQ-01..25, UC/AC-01..20 et TECH-D18-01..08. D19/51 prévaut sur les revendications E2 retirées de D18.

## Lots

| Lot | Sortie | Dépendance |
|---|---|---|
| L-WHISPER-00 | Socle, contrats et builds isolés livrés selon son ledger | Aucune |
| L-WHISPER-01 | WAV réel → application core → worker CPU → TXT/SRT → historique visible | L00 CODE |
| L-WHISPER-02 | Journal durable, FIFO, import MP3 et reprise sans départ spontané | L01 EXECUTION |
| L-WHISPER-03 | Live, VAD, MP3 par passage, Stop + Reprise | L02 EXECUTION |
| L-WHISPER-04 | GPU strict/Auto, générations et supervision, IPC v2 | L03 EXECUTION |
| L-WHISPER-05 | Réglages, tray, raccourci, autostart et archives | L04 EXECUTION |
| L-WHISPER-06 | Installation, payload et premier usage offline | L05 EXECUTION |
| L-WHISPER-07 | Qualification intégrée V1 | L06 EXECUTION |

DAG séquentiel ; DOCUMENT exige les candidats/revues exacts et contrats applicables, CODE les API/sorties hashées, EXECUTION le résultat prouvé du fournisseur. Les chemins partagés interdisent une parallélisation présumée. Les campagnes L01–L07 restent NOT RUN.

## Composition desktop

| Fichier | Responsabilité | Limite |
|---|---|---|
| `desktop/src/main.rs` | Déclare `mod root`, construit la composition privée et lance `whisper_desktop::run(application)` | Point d’entrée sans état métier parallèle |
| `desktop/src/root.rs` | Assemble application, ports/adaptateurs, service IO/canaux/worker et injecte l’application au consumer | Composition privée au binaire ; aucune règle métier ou second scheduler |
| `desktop/src/lib.rs` | Bibliothèque UI et API générique `run(application)` | N’exporte pas root ; sans adapter/fs/process/native ; hors allowlist L01 |
| `desktop/src/ui.rs` | `DesktopApp<A>`, rendu et traduction des interactions vers façade/vues | UI pure, sans assemblage ni IO bloquante ; modifiable dans L01 |

Chemin : `main.rs` → composition privée `root.rs` → `whisper_desktop::run(application)` → `ui::DesktopApp<A>`. Le contrat/run existant de lib est réutilisé.

## Détails avant transfert

| Lot | Détails |
|---|---|
| L00 | DETAIL-P01 answered v3 |
| L01 | DETAIL-P01, Q-05, Q-L01-BOUNDS-01 |
| L02 | DETAIL-P01, Q-05, Q-07 |
| L03 | DETAIL-P02, Q-07 |
| L04 | DETAIL-P03, Q-04, Q-06, Q-09 |
| L05 | DETAIL-P03, Q-03, Q-05, Q-06, Q-09 |
| L06 | Aucun supplémentaire |
| L07 | DETAIL-P04 |

Q-05 est répondu par l’utilisateur : snapshot de destination au démarrage du job. BOUNDS v1 apporte la réponse technique attribuée à Q-L01-BOUNDS-01 ; son enregistrement et sa revue dans le checkpoint sont préalables à L01. Les détails non L01 gardent leurs statuts de 04 et state.json.

## Autorité et préflight

L00 dispose d’autorisation et railguard actif attesté. L’ancien mandat L01 couvre sept anciennes entrées et ne s’étend pas implicitement à P04. La demande utilisateur du 2026-10-06 autorise l’enregistrement de l’allowlist exacte révisée ; le coordinateur la capture en source d’autorisation et met à jour l’état après revue CLEAN. L’ancien PASS P02 ne vaut pas pour les 21 chemins. Refaire inventaire/hashes, préflight et `check-state --lot L-WHISPER-01` après revue et promotion P04.

Le railguard actif `RAILGUARD-active.md` SHA256 `9fed5e976f9f49a994718637e3489acfe7d341911645969d0f5e4fc8a9107c34`, attestation `railguard-activation-L00.md` et équivalence avec D19 sont à recontrôler au préflight.

## Carte et reprise

`01_LOTS.md` définit ownership/AC ; `06_L01_INTEGRATION_CONTRACTS.md` définit application, effets, IPC et bornes ; `02_VERIFICATION_AND_PREFLIGHT.md` définit commandes/preuves ; `05_SOURCE_AND_RULE_INDEX.md` lie les sources exactes. D18/D19 restent DESIGN_FEASIBILITY, pas des tests produit. L02 garde l’import MP3, L01 reste WAV. P04 a fermé 004–006 ; 007 reste REQUIRED jusqu’à verdict indépendant sur P05.
