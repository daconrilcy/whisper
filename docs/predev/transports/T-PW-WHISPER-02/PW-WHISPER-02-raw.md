# PW-WHISPER-02 — corrections proposées v1

Auteur : `/root/plans_writer`, `rust_plan_writer`, lecture seule. Base : `P-WHISPER-01`, digest personnellement vérifié `fa5a1ee6fe37a5d28a41714d4a42dd2d5e7c77c8edb7865fc853836319ab5ad0`. Nouveau candidat proposé : `P-WHISPER-02`, `base_candidate_id=P-WHISPER-01`, parent `D-WHISPER-19`, digest personnellement revérifié `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

Entrée de corrections : les trois REQUIRED de `R-WHISPER-PLANS-01` transmis par le coordinateur. Leur résultat brut doit être transporté et référencé ; je ne prétends pas l’avoir lu dans un fichier absent. Aucun fichier modifié. Aucun finding fermé par cet auteur.

Dans les quatre documents, remplacer les mentions du candidat courant `P-WHISPER-01` par `P-WHISPER-02`, en conservant les références explicitement historiques à P1. Les autres contenus restent repris, avec les corrections suivantes.

## `00_PLANS_INDEX.md`

Remplacer la première ligne de provenance/statut par :

```markdown
Statut du candidat : DRAFT corrigé après R-WHISPER-PLANS-01, en attente de revue exacte. Auteur des corrections : `/root/plans_writer`, apport PW-WHISPER-02 v1 ; synthèse/persistance par le coordinateur et l’hôte autorisé. Base PLANS `P-WHISPER-01`, digest `fa5a1ee6fe37a5d28a41714d4a42dd2d5e7c77c8edb7865fc853836319ab5ad0` ; candidat courant `P-WHISPER-02` ; périmètre 2 ; parent `D-WHISPER-19`, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.
```

Remplacer la sortie L02 du tableau par :

```markdown
| L-WHISPER-02 | Import MP3 réel, journal durable, FIFO et reprise sans départ spontané | L01 EXECUTION |
```

Ajouter :

```markdown
## Détails contrôlés et clôture du préflight

Les questions ci-dessous sont des `reversible_detail`, pas des décisions produit nouvelles. Elles figurent dans `state.questions`, avec owner/échéance/refs, et dans `lot.requires_details`. Une question ouverte suspend le préflight des seuls lots qui la requièrent. Une réponse exige une preuve attribuée ; une option proposée ne vaut pas réponse.

| Lot | requires_details |
| --- | --- |
| L-WHISPER-00 | DETAIL-P01 |
| L-WHISPER-01 | DETAIL-P01, Q-05 |
| L-WHISPER-02 | DETAIL-P01, Q-05, Q-07 |
| L-WHISPER-03 | DETAIL-P02, Q-07 |
| L-WHISPER-04 | Q-04, Q-06, Q-09, DETAIL-P03 |
| L-WHISPER-05 | Q-03, Q-05, Q-06, Q-09, DETAIL-P03 |
| L-WHISPER-06 | Aucun détail supplémentaire |
| L-WHISPER-07 | DETAIL-P04 |

Q-03 précise seulement la présentation du refus déjà accepté ; Q-05 précise le chemin/configuration snapshot et la gestion des anciennes archives, sans modifier la suppression déjà acceptée ; Q-06 et DETAIL-P03 n’autorisent pas un kill automatique. Toute réponse changeant une garantie, frontière ou capacité déclenche DESIGN_CHANGE_REQUIRED.

## Sources manifestées et provenance

Le manifeste PLANS doit couvrir le corpus courant, le manifeste DESIGN parent, les sources REQ/UC/AC/décisions effectivement utilisées, l’autorisation utilisateur de préparer/revoir les PLANS et les snapshots prospectifs du pack central réellement employé.

La copie historique D19/15 reste une source de comparaison, et ne remplace pas le freeze prospectif. Celui-ci conserve les sources originales, hashes réels, identité et date du relevé ; il inclut les profils/skills utilisés et les helpers effectifs.

Les contributions de plan writer, avis indépendant d’impact et revues PLANS sont conservés hors du corpus examiné en TRANSPORT. Le checkpoint référence leurs transport-manifests vérifiés. L’égalité source exportée/proposition/TRANSPORT est vérifiée ; si l’export manque, la limite de provenance est explicite et examinée par le reviewer. Aucun reçu ou hash n’est inventé par ces plans.
```

## `01_LOTS.md`

Dans L02, remplacer le titre et ajouter le paragraphe suivant après les informations de couverture :

```markdown
## L-WHISPER-02 — import MP3, journal, FIFO et récupération

**Réalisation MP3 obligatoire** : compléter `crates/whisper-adapters/src/decoder.rs` et les contrats DecoderPort pour WAV/MP3 selon le profil Q-07 ; créer `crates/whisper-adapters/tests/import_mp3.rs`. Le décodeur concret/version et ses features sont attribués par DETAIL-P01 avant ce lot. Exécuter le décodage bloquant hors UI, avec buffers bornés, source en lecture seule, correspondance source→PCM16 mono 16 kHz→offsets/SRT et propagation d’erreur. Utiliser le vrai worker CPU et le publisher de L01 pour produire TXT/SRT et référence source, sans copier ni archiver un MP3 d’import.

**Acceptation MP3** : les profils MP3 courants définis par Q-07 sont transcrits réellement, avec langue manuelle prioritaire, backend CPU attesté, timestamps sur l’axe de la source et archive reconnue ; hash source inchangé avant/après. MP3 invalide/tronqué/profil non pris en charge, source modifiée après mise en file, annulation et crash sont exercés : erreur visible, aucun faux Complete, aucune suppression source et reprise seulement après choix. Vérifier V-IMPORT-MP3, V-DURABLE, V-SCHEDULER et V-UI. Les imports WAV/MP3 restent mutuellement exclusifs du live.
```

Dans L02, compléter l’ownership et la couverture UC/AC existants :

```markdown
Ownership ajouté : `crates/whisper-adapters/src/decoder.rs`, `crates/whisper-adapters/tests/import_mp3.rs` et manifests/features du décodeur. REQ-03/06/24 et UC/AC-03/06/19 sont également couverts et vérifiés par ce lot pour MP3.
```

Donc L02 devient :

- `covers` : REQ-03/04/06/12/13/16/17/18/24.
- `verifies` : AC-03/04/06/10/11/12/13/19.
- Ajouter tâche `TASK-P02-MP3`, owner auteur d’implémentation du lot, `refs=["REQ-03","REQ-06","REQ-24"]`, justification : réalisation MP3 actuellement absente ; aucun nouveau besoin.

Ajouter dans chaque lot la ligne `requires_details` correspondant au tableau de l’index. Ajouter dans L07 :

```markdown
La campagne intégrée reprend explicitement les imports MP3 implémentés et validés en L02 ; elle ne remplace pas leur réalisation par une mesure finale.
```

## `02_VERIFICATION_AND_PREFLIGHT.md`

Le chemin de manifeste courant devient `P-WHISPER-02/manifest.json`.

Ajouter au préflight :

```markdown
6. Pour chaque ID de `lot.requires_details`, vérifier que la question existe dans l’état, que le lot appartient à `impacted_lots`, que la réponse est `answered` avec preuve attribuée, et que les noms/versions/features/options adoptés correspondent au contrat exécuté. Toute absence ou question encore ouverte suspend ce préflight. Les réponses déjà acceptées de D19 sont reprises avec leur source, sans inventer la clôture d’un détail restant ouvert.
7. Vérifier le manifeste prospectif des sources/règles et l’autorisation documentaire PLANS, ainsi que les TRANSPORT/checkpoint des contributions/revues applicables. Une source fondatrice lisible mais omise du manifeste est un échec de provenance, pas une dispense.
```

Ajouter aux campagnes :

```markdown
**V-IMPORT-MP3 — PROPOSED / NOT RUN** :
shell PowerShell ; cwd `C:\dev\whisper` ; cible `x86_64-pc-windows-msvc` ; toolchain 1.98.1 ; packages/features du décodeur fixés par DETAIL-P01/Q-07.
Commande proposée :
`cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test import_mp3 -- --nocapture`.

Données : MP3 existants manifestés ou fixtures générées depuis les WAV autorisés, avec protocole et mandat applicables, paramètres/profils Q-07 et hash source ; variantes invalides/tronquées/corrompues et source changée après enqueue. L’implémentation du harness doit conserver les chemins et hashes exacts. Aucun nouveau corpus micro n’est implicitement autorisé.

Comportements attendus : vrai décodage et vrai moteur CPU jusqu’à TXT/SRT, langue effective prioritaire, offsets source conservés, backend attesté, source intacte ; échec de format/cancellation/crash sans faux Complete ni destruction source ; choix explicite avant reprise. V-UI observe ces résultats dans le vrai consumer. Un test de prévalidation ou mock moteur seul ne satisfait pas AC-03 pour MP3.
```

## `03_CENTRAL_PACK_IMPACT.md`

Remplacer l’identité de proposition par P2 et ajouter :

```markdown
## Corrections de provenance après R-WHISPER-PLANS-01

La conservation de D19 ne dispense pas de manifester les sources utilisées par les nouveaux PLANS. P2 doit inclure dans son manifeste :

- les fichiers DESIGN 02/07/08/10/11/32/47/50/51 effectivement utilisés, avec empreintes réelles, et le manifeste parent ;
- la capture fidèle du message utilisateur autorisant préparation/revue des PLANS, distincte de toute autorisation d’implémenter ;
- le freeze prospectif et les snapshots des profils/skills/contrats/helpers centraux employés pour P2 ;
- l’inventaire des sources originales et la comparaison historique D19→freeze prospectif, avec hashes réellement recalculés.

Ces ajouts sont documentaires et ne modifient pas TECH-D18-01..08. Les apports bruts/revues restent hors corpus en TRANSPORT, avec références checkpoint vérifiées ; les sources citées par le corpus doivent être accessibles et manifestées dans leur portée exacte. Le coordinateur doit conserver l’avis indépendant KEEP_D19 réellement reçu, sans le reconstruire depuis une phrase de synthèse.

Aucun finding n’est fermé par cet auteur. Seule une revue indépendante du manifeste P2 exact peut juger les corrections suffisantes.
```

## Mapping des questions pour `state.questions`

Toutes ci-dessous : `classification="reversible_detail"`, `status="open"` pour leur partie de détail restante. Conserver les réponses produit déjà acceptées de D19 dans les sources ; les questions n’en annulent pas l’accord. Owner de suivi `AUTH-COORD` existant ; le responsable de réponse est indiqué séparément dans le contenu documentaire. Cela évite d’inventer une autorité d’architecte enregistrée.

| ID | Responsable de réponse ; options bornées proposées | Échéance | impacted_lots | refs |
|---|---|---|---|---|
| DETAIL-P01 | rust_architect : disposition proposée ou regroupement équivalent ; choisir versions/features UI/décodeur et noms, sans native dans desktop/core | before:L-WHISPER-00 | L00,L01,L02 | REQ-03,REQ-24 |
| DETAIL-P02 | rust_architect : adapter capture/format et capacités par étage mesurables ; capacités finies et cessation d’entrée conservées | before:L-WHISPER-03 | L03 | REQ-01,REQ-02,REQ-18 |
| DETAIL-P03 | rust_architect et autorité produit si le geste change l’effet : attendre/diagnostiquer ou geste manuel explicite documenté ; aucun arrêt forcé automatique | before:L-WHISPER-04 | L04,L05 | REQ-15,REQ-16 |
| DETAIL-P04 | autorité produit pour mandat ; analyste pour protocole : corpus micro représentatif autorisé, ou campagne restant NOT RUN avec blocage de qualification explicite | before:L-WHISPER-07 | L07 | REQ-02,REQ-09,REQ-18 |
| Q-03 | analyste/UI : message inline ou notification visible pour refus déjà accepté ; aucune mise en file live | before:L-WHISPER-05 | L05 | REQ-04,REQ-21 |
| Q-04 | rust_architect : signaux par état et cadence/marge autour de 60 s ; panne fondée sur preuve, jamais temps seul | before:L-WHISPER-04 | L04 | REQ-11 |
| Q-05 | rust_architect : snapshot du dossier par job, nouveau dossier pour jobs suivants ; historique anciens dossiers explicitement accessible ; aucune migration/suppression implicite | before:L-WHISPER-01 | L01,L02,L05 | REQ-20,REQ-22,REQ-23 |
| Q-06 | rust_architect/produit : état/message d’attente et geste manuel ; garder ouvert et récupérable conformément au choix accepté | before:L-WHISPER-04 | L04,L05 | REQ-15,REQ-16 |
| Q-07 | rust_architect : inventaire profils WAV/MP3 pris en charge/rejetés ; profil sortie/bitrate à qualifier sans promesse lecteurs tiers | before:L-WHISPER-02 | L02,L03 | REQ-03,REQ-10,REQ-12 |
| Q-09 | responsable diagnostics et produit si effet conservation : durée/rotation bornées ou conservation manuelle documentée ; logs sans audio/texte/réseau | before:L-WHISPER-04 | L04,L05 | REQ-11,REQ-20 |

Sources des Q héritées : `D-WHISPER-19/10_QUESTIONS_RISKS_AND_READINESS.md`, avec précisions `08`, `50` et source produit `06`. Sources DETAIL : correction P2 et contrats D19 applicables, sans preuve de réponse inventée.

Les tableaux emploient L00…L07 comme abréviation d’affichage ; utiliser les IDs complets `L-WHISPER-00`…`L-WHISPER-07` dans le JSON.

Travail restant : hôte persiste P2 et sources/freeze/mandat réels ; TRANSPORT de PW2/RP1/avis d’impact, comparaison et checkpoint ; contrôle des lots/questions/paths/autorités ; nouvelle revue indépendante P2 exact. Aucune exécution produit, activation railguard ou autorisation coding ajoutée.