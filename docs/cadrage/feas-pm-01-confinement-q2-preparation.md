Avancée : le lanceur autonome Q2 emploie `DETACHED_PROCESS` et les outils préparatoires compilent hors ligne.
Blocage : la surveillance et l'arrêt automatique ne satisfont pas encore les préconditions de l'unique tentative Q2.
Décision : NO-GO pour Q2 ; qualification livrée avec réserves après deux cycles de revue/correction.

# FEAS-PM-01 — préparation Q2 et décision avant essai

## Périmètre et état

Le candidat autonome se trouve sous `target/parakeet-tests/micro-feasibility/confinement-q2/`. La source Q1 a été conservée. Aucune modification du code produit, installation, élévation, session ETW, modification d'ACL, exécution de sonde, essai CPU/CUDA, stratégie A/B ou FEAS-PM-01 n'a été réalisée pour Q2. Les fichiers `models/` et `sound example/` n'ont pas été touchés.

La condition fixée par l'utilisateur était de ne lancer l'unique tentative Q2 **que si** la surveillance en direct, l'attribution, l'arrêt automatique et la sauvegarde/restauration des ACL étaient prêts. Cette condition n'est pas satisfaite. L'essai Q2 n'a donc pas été consommé.

## Ce qui est établi

- `launcher.rs` définit `CREATE_SUSPENDED=0x00000004`, `DETACHED_PROCESS=0x00000008` et leur combinaison `0x0000000C` pour le processus racine. L'appel à `CreateProcessAsUserW` passe `bInheritHandles=FALSE`, un `STARTUPINFO` initialisé sans `STARTF_USESTDHANDLES`, et attache le processus encore suspendu au Job avant `ResumeThread`. Le Job demande `KILL_ON_JOB_CLOSE` sans option de breakaway. Le processus enfant candidat utilise `DETACHED_PROCESS` et `bInheritHandles=FALSE`. Cette cohérence est **statique** ; le démarrage réel reste non vérifié.
- Le consommateur ETW autonome est compilé avec les dépendances Windows déjà en cache. `cargo fmt --check`, `cargo check --offline --locked`, `cargo build --offline --locked` et la compilation `rustc` du lanceur ont réussi avec `CARGO_HOME`, `TEMP` et `TMP` sous `micro-feasibility/confinement-q2/`, et `CARGO_TARGET_DIR` sous `cargo-shared/`. L'espace libre relevé avant compilation était d'environ 166,9 Gio.
- La revue indépendante a vérifié deux cycles. Le nom `ParentProcessID` de la trace Q1 est désormais utilisé ; les racines attendues sont fixées et contrôlées dans le consommateur. Ces points ferment R1 et R7. La compilation n'établit pas la réception d'événements ETW ni l'efficacité du jeton restreint.

## Réserves de la seconde revue

| Constat | Niveau | Effet sur Q2 |
| --- | --- | --- |
| R2 — attribution d'un PID court ou déjà terminé incomplète | Bloquant | Des événements d'un descendant peuvent échapper à une attribution certaine. |
| R3 — seul `File` ID 16 est traité comme écriture ; une création réussie de fichier vide peut se terminer sans ID 16 | Bloquant | Une création hors racine pourrait ne pas déclencher l'arrêt automatique. |
| R4 — `ready` est signalé avant preuve de réception et de décodage ETW en direct | Bloquant | La sonde pourrait être reprise sans surveillance effective. |
| R5 — les événements nommés `ready` et `heartbeat` ne sont pas liés à un consommateur authentifié | Bloquant | Un signal étranger pourrait autoriser ou maintenir la sonde sans le moniteur attendu. |
| R6 — `IsProcessInJob(..., NULL)` atteste seulement un Job quelconque | Réserve | L'attestation demandée du Job Q2 précis manque pour la racine et l'enfant. |

Les constats R2–R5 concernent la traçabilité et l'arrêt exigés **avant** l'essai négatif. Aucun résultat PASS ou FAIL de prévention NTFS, d'enfant, d'attribution ETW ou de compatibilité CUDA ne peut être déduit de cette préparation.

## État de la qualification demandée

| Domaine | Verdict | Preuve ou limite |
| --- | --- | --- |
| Correction statique des flags de démarrage | PARTIAL | Valeurs et appels vérifiés dans la source ; aucun démarrage sous jeton restreint. |
| Prévention des écritures hors racines | NON EXÉCUTÉ | Ni ACL Q2 ni test `ACCESS_DENIED` ; aucune conclusion sur la prévention. |
| Processus enfants | NON EXÉCUTÉ | Aucune sonde enfant Q2 lancée. |
| Surveillance, attribution et arrêt automatique | FAIL en précondition | Constats R2–R5 ouverts ; pas de preuve de chaîne opérationnelle avant `ResumeThread`. |
| Restauration des ACL et arrêt ETW | SANS OBJET | Aucune ACL Q2 modifiée et aucune session ETW Q2 ouverte. |
| Q2 global | PARTIAL — NO-GO | Outils préparés, essai non lancé ; les conditions préalables restent ouvertes. |

Le répertoire Q2 est sous `target/` et ignoré par Git ; ce document consigne l'état du candidat, sans le qualifier comme artefact reproductible versionné. Les répertoires `models/` et `sound example/` restent des entrées non suivies préexistantes dans l'état Git.

## Décision à prendre avant une nouvelle tentative

Le prochain périmètre éventuel devrait traiter, dans cet ordre, l'attribution des descendants, la distinction entre ouverture en lecture et création effective sans écriture de contenu, la preuve ETW **avant** `ready`, puis un canal de signalement lié au consommateur et l'attestation du Job Q2 exact. Il faudra ensuite revoir un candidat concret et préparer une procédure complète de sauvegarde/restauration des ACL et d'arrêt ETW. La limite de deux cycles pour le présent livrable est atteinte ; ces travaux nécessitent un nouvel arbitrage de périmètre, sans lancer Q2 sur le candidat actuel.
