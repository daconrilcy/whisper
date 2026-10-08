# Détails de préflight — P-WHISPER-28

Le registre courant est `docs/predev/state.json`. DETAIL-P03, Q-04, Q-06 et Q-09 sont `answered`, avec preuves et autorités attribuées; P25 ne les rouvre pas. Le checkpoint futur doit conserver ces statuts et preuves. Les tableaux hérités restent informatifs/historiques et ne modifient pas ces réponses. Toute nouvelle réponse changeant un invariant revient au DESIGN.

# Détails de préflight — P-WHISPER-24

Ce registre conserve des détails hérités de P14; son titre ne définit pas le statut courant. Dans `docs/predev/state.json`, Q-04/Q-06/Q-09 et DETAIL-P03 sont déjà answered avec leurs preuves. P24 les reprend sans réouverture et les vérifie au checkpoint courant. Les autres détails ouverts demeurent attribués à leurs lots futurs comme indiqué ci-dessous.

| ID | Responsable de réponse ; options et conséquence | Échéance | Lots impactés | Besoin |
| --- | --- | --- | --- | --- |
| DETAIL-P01 | Architecte Rust : disposition crates/packages/features/binaires proposée ou regroupement équivalent ; fixer UI et décodeur. Toute option doit garder core/desktop sans natif moteur et builds CPU/GPU isolés. | avant L00 | L00,L01,L02 | REQ-03/24, TECH-D18-01/08 |
| DETAIL-P02 | Répondu v1 : CPAL/WASAPI, format PCM16 mono 16 kHz, capacités par étage, staging durable ; réponse et acceptation attribuées dans les sources P10. Validation bytes locales P02-GAP-BYTES-01 reste NOT RUN sous L03. | avant L03 | L03 | REQ-01/02/18 |
| DETAIL-P03 | Architecte Rust et produit si effet visible : attente/diagnostic ou geste manuel explicite lorsque Quitter bloque ; aucun kill automatique. | avant L04 | L04,L05 | REQ-15/16 |
| DETAIL-P04 | Utilisateur pour mandat de nouvelles captures, analyste pour protocole : corpus micro FR/EN calme/bruit représentatif, ou campagne restant NOT RUN et qualification bloquée. | avant L07 | L07 | REQ-02/09/18 |
| Q-03 | Analyste/UI : refus de raccourci en état incompatible par message inline ou notification visible ; jamais démarrage/mise en file cachés. | avant L05 | L05 | REQ-04/21 |
| Q-04 | Réponse technique v2 proposée : progression réelle par obligation/étage, polling ≤1 s, avertissement 60 s, diagnostic ≤30 s, aucun effet d’arrêt/fallback/publication piloté par timer. Source attribuée ARCH-Q04-Q06-Q09-v2 et transport ANSWER-02. | avant L04 | L04 | REQ-11 |
| Q-05 | Architecte Rust : snapshot du dossier par job, nouveaux jobs vers nouveau dossier, anciens dossiers explicitement accessibles ; aucune migration/suppression implicite. | avant L01 | L01,L02,L05 | REQ-20/22/23 |
| Q-06 | Réponse technique v2 proposée : réutiliser D19/DETAIL-P03, attente visible/réactive tant que l’enfant courant n’a pas confirmé Stopped, pas de faux Complete ni terminaison forcée. Source ARCH-Q04-Q06-Q09-v2 et transport ANSWER-02. | avant L04 | L04,L05 | REQ-15/16 |
| Q-07 | Architecte Rust : inventaire des profils WAV/MP3 admis/refusés et profil/bitrate MP3 de sortie à qualifier ; pas de promesse sample-exact chez lecteur tiers. | avant L02 | L02,L03 | REQ-03/10/12 |
| Q-09 | Option de rotation bornée choisie par l’utilisateur : « Rotation bornée (Recommended) ». La capture humaine séparée est limitée à ces mots; les limites proposées et détails techniques sont attribués à ARCH-Q04-Q06-Q09-v2 et au transport ANSWER-02. | avant L04 | L04,L05 | REQ-11/20 |

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


## Q-04 / Q-06 / Q-09 — réponse v2 et statut du registre

La revue exacte de P13 a relevé un finding requis dans le présent registre; son rapport complet est conservé sous `sources/reviews/R-WHISPER-PLANS-13-v1.md`, transport `T-WHISPER-PLANS-REVIEW-13`, manifest digest `512a5cd18893fdfde5e38c6770421a752170e6893614bfa6f102129a8849d251`, manifest SHA256 `b49a4f94a443098032f03af8aef26ba750e2b25347852b5ed2ab2838b71483c9`. P14 corrige les références v1/P11/ANSWER-01 et la capture Q09 trop large. La contribution technique attribuée `ARCH-Q04-Q06-Q09-v2.md` est conservée dans `T-WHISPER-Q04-Q06-Q09-ANSWER-02`; le fichier d'acceptation utilisateur contient seulement « Rotation bornée (Recommended) ». Les réponses sont présentées pour acceptation/enregistrement comme `ANSWERED_DETAIL`, sans prétendre que le registre actif est déjà modifié. Le statut courant est celui de `docs/predev/state.json`; les réponses Q-04/Q-06/Q-09 sont answered. P24 devra recevoir CLEAN et un checkpoint P24 promu avant que l’allowlist élargie puisse servir au gate L04. Tous les essais produit/runtime restent NOT RUN.

### Historique supersédé

Les anciennes descriptions Q-04/Q-06/Q-09 v1 et les références à P11/`T-WHISPER-Q04-Q06-Q09-ANSWER-01` sont conservées uniquement dans les sources immuables P11/P12/P13. Elles sont supersédées pour le registre courant par la contribution v2 et ANSWER-02; elles ne constituent ni le statut actuel ni la provenance Q09 retenue.

