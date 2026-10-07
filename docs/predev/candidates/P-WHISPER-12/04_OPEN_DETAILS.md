# Détails de préflight — P-WHISPER-10

Le tableau décrit les questions et réponses attribuées ; leur statut courant et leurs preuves sont contrôlés dans `state.json`. Les questions encore ouvertes sont des `reversible_detail` bornées au lot. L'owner de suivi dans `state.json` est `AUTH-COORD` ; l'auteur de la réponse technique reste `rust_architect`, avec autorité produit lorsque la réponse touche l'effet visible ou autorise de nouvelles données. Une option sous une entrée ouverte est une proposition, jamais une réponse acceptée. L'échéance et les lots impactés sont répétés dans l'état contrôlé. Les décisions produit déjà acceptées dans D19 gardent leur force ; cette liste précise leur mise en œuvre sans les rouvrir.

| ID | Responsable de réponse ; options et conséquence | Échéance | Lots impactés | Besoin |
| --- | --- | --- | --- | --- |
| DETAIL-P01 | Architecte Rust : disposition crates/packages/features/binaires proposée ou regroupement équivalent ; fixer UI et décodeur. Toute option doit garder core/desktop sans natif moteur et builds CPU/GPU isolés. | avant L00 | L00,L01,L02 | REQ-03/24, TECH-D18-01/08 |
| DETAIL-P02 | Répondu v1 : CPAL/WASAPI, format PCM16 mono 16 kHz, capacités par étage, staging durable ; réponse et acceptation attribuées dans les sources P10. Validation bytes locales P02-GAP-BYTES-01 reste NOT RUN sous L03. | avant L03 | L03 | REQ-01/02/18 |
| DETAIL-P03 | Architecte Rust et produit si effet visible : attente/diagnostic ou geste manuel explicite lorsque Quitter bloque ; aucun kill automatique. | avant L04 | L04,L05 | REQ-15/16 |
| DETAIL-P04 | Utilisateur pour mandat de nouvelles captures, analyste pour protocole : corpus micro FR/EN calme/bruit représentatif, ou campagne restant NOT RUN et qualification bloquée. | avant L07 | L07 | REQ-02/09/18 |
| Q-03 | Analyste/UI : refus de raccourci en état incompatible par message inline ou notification visible ; jamais démarrage/mise en file cachés. | avant L05 | L05 | REQ-04/21 |
| Q-04 | Répondu v1 : progrès monotone par obligation/étape ; avertissement à 60 s, sondage ≤1 s, diagnostic inchangé rafraîchi ≤30 s ; minuteur sans pouvoir d’arrêt/fallback/publication. Réponse attribuée et preuve dans `sources/architecture/ARCH-Q04-Q06-Q09-v1.md` et le transport Q04/Q06/Q09. | avant L04 | L04 | REQ-11 |
| Q-05 | Architecte Rust : snapshot du dossier par job, nouveaux jobs vers nouveau dossier, anciens dossiers explicitement accessibles ; aucune migration/suppression implicite. | avant L01 | L01,L02,L05 | REQ-20/22/23 |
| Q-06 | Répondu v1 : réutiliser la décision acceptée D19/DETAIL-P03 ; quitter latched/cooperatif, application réactive avec attente/diagnostic tant que `Stopped` n’est pas confirmé, pas de faux `Complete` ni terminaison forcée, données confirmées récupérables. | avant L04 | L04,L05 | REQ-15/16 |
| Q-07 | Architecte Rust : inventaire des profils WAV/MP3 admis/refusés et profil/bitrate MP3 de sortie à qualifier ; pas de promesse sample-exact chez lecteur tiers. | avant L02 | L02,L03 | REQ-03/10/12 |
| Q-09 | Répondu v1, option « Rotation bornée » acceptée : max 4×2 MiB, 7 jours, oldest-first, sans minimum garanti ; logs diagnostiques locaux filtrés seulement. L’acceptation complète et la preuve sont dans `sources/authorization/user-Q04-Q06-Q09-authorization.md` et `sources/architecture/ARCH-Q04-Q06-Q09-v1.md`. | avant L04 | L04,L05 | REQ-11/20 |

Une réponse doit citer son auteur, option, preuve et contrats/commandes touchés. Si elle modifie périmètre, stockage, garantie, frontières ou mode GPU, appliquer DESIGN_CHANGE_REQUIRED et suspendre les lots concernés. Le preflight exige `status=answered` avec preuve pour chaque `requires_details` du lot.


## Résolutions avant L01

- `DETAIL-P01` : answered v3, disposition privée au binaire / root, UI/lib pures, détails crates/features selon sa source exacte.
- `Q-05` : answered par l’utilisateur ; destination snapshotée à l’acceptation du job, sans migration/suppression.
- `Q-L01-BOUNDS-01` : reversible_detail, réponse technique architecte dans `ARCH-L01-BOUNDS-01`, impacts L01 (et IPC futur L04). Proposé answered sous réserve de vérification du transport et vérification du transport et de la provenance ; le gate de transfert exige la revue PLANS exacte de P06 ; bornes détaillées en `06_L01_INTEGRATION_CONTRACTS.md`. COMPAT v2 n’approuve pas les nombres BOUNDS.


## Réponse Q-07 v1 intégrée pour L02/L03

Statut contrôlé : answered, reversible_detail, `ANSWERED_DETAIL`, acceptée par l’autorité technique déléguée `AUTH-ARCH-Q07`, sans DESIGN_CHANGE_REQUIRED. La source attribuée est `transports/T-WHISPER-Q07-ANSWER-01/Q-07-architecture-answer.md`, avec son manifeste TRANSPORT. La décision fixe les profils techniques d’admission et un profil de sortie live à qualifier ; elle ne constitue pas une validation produit.

WAV : RIFF/WAVE little-endian PCM U8/S16/S24/S32, float F32/F64, A-law/μ-law et sous-types WAVEFORMATEXTENSIBLE reconnus. MP3 : MPEG-1 Layer III 32/44,1/48 kHz, mono/stéréo/joint/dual, CBR/VBR débits standards ; MPEG-2/2.5 Layer III 16/22,05/24 kHz et 8/11,025/12 kHz, mêmes modes/canaux et débits standards. Dépendre du contenu réellement reconnu et décodable, pas de l’extension. Refuser free-format/Layer I/II avec features présentes, codecs non activés, paramètres invalides/réservés, conteneurs non supportés, flux absents, corrupt/truncated lorsqu’ils sont détectés. Ne pas ajouter de plafond durée/taille/fréquence.

Sortie MP3 live choisie pour qualification future L03 : `rusty_mp3 = 0.8.0`, MPEG-2 Layer III, mono 16 kHz, CBR cible 64 kbps, un fichier immuable par passage. La qualif vérifie les en-têtes effectifs ; qualité/écoute et compatibilité universelle non promises. Axes temporels source conservés, traitement gapless si métadonnées disponibles sans double retrait. Les plafonds mémoire restent inchangés. `02_VERIFICATION_AND_PREFLIGHT.md` conserve V-IMPORT-MP3 et la qualification L03 en NOT RUN.


## DETAIL-P02 v1 — répondu pour L03

Statut proposé dans le checkpoint P10 : `answered`, `ANSWERED_DETAIL`, `approval=accepted`, `design_change_required=false`, `answer_version=1`; la source d’architecture et l’acceptation utilisateur sont attribuées dans `sources/` et le transport correspondant. La conception retenue et les limites wire/RSS sont détaillées dans `10_L03_INTEGRATION_CONTRACTS.md`. Q-07 reste answered tel quel. P02-GAP-BYTES-01 est une validation ultérieure L03 / NOT RUN, pas une question structurelle ouverte.


## Réponses proposées par P11 — Q-04 / Q-06 / Q-09

Les réponses techniques attribuées, leur preuve, les limites et les sources utilisateur sont reprises dans `13_L04_DETAIL_CONTRACTS.md`, `sources/architecture/ARCH-Q04-Q06-Q09-v1.md` et `sources/authorization/user-Q04-Q06-Q09-authorization.md`. Le reçu du transport est `P10-HOST-WORK/transports/T-WHISPER-Q04-Q06-Q09-ANSWER-01/transport-manifest.json`.

P11 propose `status=answered` pour ces trois questions après son examen indépendant et sa promotion. Tant que l’état actif n’a pas été promu, `docs/predev/state.json` reste la source de statut et conserve les questions ouvertes. Aucune campagne d’exécution n’est réputée passée.
