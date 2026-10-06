# Détails de préflight — P-WHISPER-03

Le tableau décrit les questions et réponses attribuées ; leur statut courant et leurs preuves sont contrôlés dans `state.json`. Les questions encore ouvertes sont des `reversible_detail` bornées au lot. L'owner de suivi dans `state.json` est `AUTH-COORD` ; l'auteur de la réponse technique reste `rust_architect`, avec autorité produit lorsque la réponse touche l'effet visible ou autorise de nouvelles données. Une option ci-dessous est une proposition, jamais une réponse acceptée. L'échéance et les lots impactés sont répétés dans l'état contrôlé. Les décisions produit déjà acceptées dans D19 gardent leur force ; cette liste précise leur mise en œuvre sans les rouvrir.

| ID | Responsable de réponse ; options et conséquence | Échéance | Lots impactés | Besoin |
| --- | --- | --- | --- | --- |
| DETAIL-P01 | Architecte Rust : disposition crates/packages/features/binaires proposée ou regroupement équivalent ; fixer UI et décodeur. Toute option doit garder core/desktop sans natif moteur et builds CPU/GPU isolés. | avant L00 | L00,L01,L02 | REQ-03/24, TECH-D18-01/08 |
| DETAIL-P02 | Architecte Rust : capture/format d'entrée, capacités mesurables par étage. Une taille inférieure ou supérieure change les ressources/latences, jamais la cessation d'entrée explicite. | avant L03 | L03 | REQ-01/02/18 |
| DETAIL-P03 | Architecte Rust et produit si effet visible : attente/diagnostic ou geste manuel explicite lorsque Quitter bloque ; aucun kill automatique. | avant L04 | L04,L05 | REQ-15/16 |
| DETAIL-P04 | Utilisateur pour mandat de nouvelles captures, analyste pour protocole : corpus micro FR/EN calme/bruit représentatif, ou campagne restant NOT RUN et qualification bloquée. | avant L07 | L07 | REQ-02/09/18 |
| Q-03 | Analyste/UI : refus de raccourci en état incompatible par message inline ou notification visible ; jamais démarrage/mise en file cachés. | avant L05 | L05 | REQ-04/21 |
| Q-04 | Architecte Rust : signaux de progression par état et cadence/marge de diagnostic autour des 60 s indicatives ; aucune option ne permet l'arrêt sur le temps seul. | avant L04 | L04 | REQ-11 |
| Q-05 | Architecte Rust : snapshot du dossier par job, nouveaux jobs vers nouveau dossier, anciens dossiers explicitement accessibles ; aucune migration/suppression implicite. | avant L01 | L01,L02,L05 | REQ-20/22/23 |
| Q-06 | Architecte Rust/produit : état/message d'attente et geste manuel quand Quitter ne peut achever ; garder ouvert et récupérable conformément à D19. | avant L04 | L04,L05 | REQ-15/16 |
| Q-07 | Architecte Rust : inventaire des profils WAV/MP3 admis/refusés et profil/bitrate MP3 de sortie à qualifier ; pas de promesse sample-exact chez lecteur tiers. | avant L02 | L02,L03 | REQ-03/10/12 |
| Q-09 | Responsable diagnostic, produit si effet de conservation : rotation bornée ou rétention manuelle documentée ; logs techniques locaux sans audio/texte ni réseau. | avant L04 | L04,L05 | REQ-11/20 |

Une réponse doit citer son auteur, option, preuve et contrats/commandes touchés. Si elle modifie périmètre, stockage, garantie, frontières ou mode GPU, appliquer DESIGN_CHANGE_REQUIRED et suspendre les lots concernés. Le preflight exige `status=answered` avec preuve pour chaque `requires_details` du lot.


## Résolutions avant L01

- `DETAIL-P01` : answered v3, disposition privée au binaire / root, UI/lib pures, détails crates/features selon sa source exacte.
- `Q-05` : answered par l’utilisateur ; destination snapshotée à l’acceptation du job, sans migration/suppression.
- `Q-L01-BOUNDS-01` : reversible_detail, réponse technique architecte dans `ARCH-L01-BOUNDS-01`, impacts L01 (et IPC futur L04). Proposé answered sous réserve de vérification du transport et vérification du transport et de la provenance ; le gate de transfert exige la revue PLANS exacte de P06 ; bornes détaillées en `06_L01_INTEGRATION_CONTRACTS.md`. COMPAT v2 n’approuve pas les nombres BOUNDS.
