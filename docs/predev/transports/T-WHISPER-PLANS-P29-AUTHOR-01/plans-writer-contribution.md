# PLANS writer contribution — P29

Actor: /root/plans_writer_l04; rôle: rust_plan_writer; base: P-WHISPER-28; parent: D-WHISPER-28; statut: DRAFT. Texte consolidé pour transport, non verbatim de la conversation; les deux fichiers qui suivent sont les octets UTF-8 soumis à la persistance.

## 00_PLANS_INDEX.md

# Plans Whisper — P-WHISPER-29

Statut à rédaction : DRAFT. Owner : AUTH-PLAN-L04. Date : 2026-10-08.

Successeur immuable de P-WHISPER-28. Parent DESIGN exact : D-WHISPER-28, digest `50e2ab1ac3afd17d9e4a1e51932aaa37f812e3162db0207626656108352a6911`.

Le statut courant se lit dans `C:/dev/whisper/docs/predev/state.json`, après contrôle canonique sur une racine documentaire unifiée et vérifiée. L'inscription DRAFT dans ce candidat décrit son état à rédaction. Le manifeste PLANS et la revue indépendante de P29 restent à produire. Aucun CLEAN ou READY PLANS n'est revendiqué ici.

## Entrée opérationnelle

`17_L04_D28_PLAN.md` est la spécification opératoire de L-WHISPER-04. Elle remplace pour L04 les instructions héritées des fichiers 00 à 16 concernant le parent D22, la portée de quinze chemins, la baseline dix présences/cinq absences, l'identité worker assimilée au passage, la cessation systématique de capture sur panne worker, le checkpoint calculé depuis le dernier mot et le lancement fondé sur la seule autorisation ancienne.

Les fichiers hérités sont conservés comme sources historiques. Leurs preuves L00–L03 gardent leur identité, leur candidat exécuté et leurs limites. Leurs campagnes non exécutées restent NOT RUN.

## Périmètre et graphe

L00 → L01 → L02 → L03 → L04 → L05 → L06 → L07. Les dépendances de lancement entre lots sont EXECUTION. Les contrats acceptés sont DOCUMENT ; les API et artefacts fournis sont CODE. Aucune dépendance DOCUMENT ne rend disponible un code absent.

L00–L03 sont historiques/completed selon leurs preuves canoniques. L04 demeure planned jusqu'à sa clôture attribuée. L05–L07 conservent leurs objectifs et couvertures de `01_LOTS.md` et consomment les contrats corrigés D28 lorsque pertinents. Les détails ouverts avant L05 ou L07 restent à leurs échéances. Les chemins partagés sont exécutés séquentiellement.

L04 contient cinq incréments internes séquentiels I04-A à I04-E. Ils ne constituent pas de nouveaux lots completed dans le ledger.

## Autorités et limites

La préparation et la revue des plans sont autorisées. `USER_AUTHORIZATION_L04.md` autorise l'implémentation et le préflight sur quinze chemins historiques. Six chemins supplémentaires du design accepté restent sans autorisation d'édition applicable à cette rédaction. `17_L04_D28_PLAN.md` les liste et impose le contrôle avant code.

Le railguard est déjà actif ; aucune nouvelle activation ou exception n'est proposée. Le préflight vérifie le fichier actif et son attestation d'équivalence.

PRODUCT_VALIDATION, DELIVERY_QUALIFICATION, probes d'environnement, préflight ciblé L04, builds et tests de ce candidat : NOT RUN.

## Sources et reprise

Parent D28 : `59_CHANGE_L04_DECISIONS_D24.md`, `60_CHANGE_L04_DOMAIN_D24.md`, `61_CHANGE_L04_ARCHITECTURE_D24.md`, `62_CHANGE_L04_REQUIREMENTS_D24.md`, `57_L04_CHANGE_BASELINE.json`, `52_L04_SOURCE_BASELINE_D20.md` et snapshots code utiles. Sources historiques : corpus P28, autorisation PLANS, réponses DETAIL-P03/Q-04/Q-06/Q-09, preuves L03 et railguard.

Le coordinateur fait persister le corpus complet et le paquet brut auteur en TRANSPORT, vérifie reçus/hashes, puis conserve le checkpoint de reprise. La revue PLANS doit porter P29 exact, son manifeste et D28 exact. Toute correction d'octets demande un successeur et une revue exacte.


## 17_L04_D28_PLAN.md

# L-WHISPER-04 — plan opératoire D28 / P29

Owner proposé : AUTH-PLAN-L04. Statut : DRAFT. Date : 2026-10-08. Parent DESIGN : D-WHISPER-28, digest `50e2ab1ac3afd17d9e4a1e51932aaa37f812e3162db0207626656108352a6911`.

Ce fichier remplace les instructions L04 héritées de P28. Leurs sources restent conservées avec leurs octets et limites. Aucun résultat produit, CLEAN PLANS ou READY PLANS n'est établi ici.

## 1. Objectif, couverture et exclusions

Terminer calcul et supervision L04 selon D28 : CPU forcé attesté, GPU attesté, secours Auto sur le même passage ou import, fin CPU explicitement choisie après panne GPU forcé, confirmations durables incluant silence, arrêt coopératif et diagnostics bornés.

Couverture : REQ-07/08/11/14/15/16/18/24, UC-07/09/11/13/19, AC-07/09/11/13/19 ; obligations connexes REQ-10/12/17/25, UC-08/10/12/20 et AC-08/10/12/20 pour archive, reprise et publication. REQ-07/08/11/18 et AC-07/09/13 suivent les versions corrigées D28. TASK-P04 demeure la tâche du lot. DEC-L04-CAPTURE-AUTO-01 et DEC-L04-STRICT-FINISH-01 sont les choix acceptés. DETAIL-P03, Q-04, Q-06, Q-09 et Q-L04-AUDIO-CAPACITY-01 sont les entrées documentaires requises avec leurs sources attribuées. REQ-26/UC-21/AC-21 proposés autrefois sont retirés par D28 ; Q-09 est un contrat de détail.

Exclusions : nouveau crate ou DLL, autre modèle ou pile native, timeout destructif, kill forcé, plafond RSS global, quota spool, durée de continuité garantie, nouvelle politique produit et publication. L05–L07 restent dans leurs lots.

## 2. Portée physique et responsabilités

Portée prévue : exactement les 21 chemins suivants.

```text
Cargo.lock
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/supervisor.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/tests/live_archive.rs
crates/whisper-adapters/tests/worker_control.rs
crates/whisper-core/src/application.rs
crates/whisper-core/src/compute_policy.rs
crates/whisper-core/src/ipc.rs
crates/whisper-core/src/lib.rs
crates/whisper-core/src/ports.rs
crates/whisper-core/tests/compute_policy.rs
crates/whisper-core/tests/live_contract.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-worker-cpu/src/main.rs
crates/whisper-worker-cpu/src/native_engine.rs
crates/whisper-worker-gpu/Cargo.toml
crates/whisper-worker-gpu/src/main.rs
crates/whisper-worker-gpu/src/native_engine.rs
```

Application/core : intentions, identités, policy, admission des résultats, barrière et orchestration ; core possède ports/types IPC. Archive : exclusivité, journaux, checkpoint, confirmation et publication. Worker IPC : effets hors UI, capture conservée, transport et observation des enfants, sans policy métier parallèle. Workers : codecs, inférence et attestation, sans décision de fallback. Supervisor : obligations, progrès et diagnostics fermés. Root assemble ; UI présente l'état et transmet les commandes application. Tests : contrats dans les targets existantes. Cargo.lock et manifest GPU ne changent que si nécessaires aux contrats et pins acceptés, avec justification et revue.

Six chemins ajoutés à l'autorisation historique : `crates/whisper-adapters/src/archive.rs`, `crates/whisper-adapters/tests/live_archive.rs`, `crates/whisper-core/src/ipc.rs`, `crates/whisper-core/src/ports.rs`, `crates/whisper-core/tests/live_contract.rs`, `crates/whisper-worker-cpu/src/main.rs`. Ils restent NON AUTORISÉS pour édition tant qu'une source applicable n'accorde pas leur portée exacte. `capture.rs`, `staging.rs`, `worker-cpu/src/encoder.rs` et `worker-cpu/src/ipc.rs` sont des sources de validation héritées hors portée. Tout besoin de les modifier retourne au coordinateur avant édition.

## 3. Entrées, sorties et dépendances

DOCUMENT : D28 READY/CLEAN exact, décisions acceptées, contrats 59–62, détails répondus, railguard et règles effectives. CODE : sorties L03, protocole/types, capture, staging, encodeur, décodeur et consumers existants, hashes confrontés aux preuves. EXECUTION : L03 completed avec preuve canonique et limites natives conservées ; cette clôture ne prouve aucune campagne L04.

Sorties L04 : API/types corrélés, journaux/checkpoint versionnés, workers homogènes, fallback et fin explicite intégrés, vraie UI, diagnostics bornés ; sources/artefacts hashés, revue indépendante, validation attribuée et clôture du candidat réel. L05 consomme L04 par EXECUTION : du code partiel ou une revue seule ne satisfait pas cette dépendance.

Les cinq incréments partagent des chemins : exécution séquentielle, un propriétaire d'édition par chemin à un instant donné, handoff des contrats modifiés. Aucune parallélisation de ces incréments.

## 4. Contrats à réaliser

Identité durable : groupe/passage ou import, storage_generation, configuration/source/axe stables. Capture : flux/handle du passage indépendant de la tentative. Tentative moteur : namespace/generation/instance corrélés. Aucun événement ancien n'agit sur confirmation, publication, capture ou arrêt du successeur.

T = couverture transcription confirmée, silence inclus ; A = PCM contigu durable ; C = capture connue ; E = encodage pending. Invariant `0 <= T <= A <= C`. E ne confirme ni T ni Complete. RecoveryCheckpoint v2, CoverageConfirmation v2 et AttemptJournal v1 suivent D28/60–61. Records JSON UTF-8 newline, checksum SHA256 canonique, lecture et sérialisation plafonnées à 1 MiB, scans progressifs. AttemptJournal : Reserved/Started/Retired. Reserved durable avant spawn consomme le numéro même sans Started ; overflow interdit spawn ; Reserved sans Started signifie lancement possible et état inconnu, sans démarrage automatique ni recyclage.

Writer archive unique : verrou exclusif non bloquant sur fichier stable, hors UI, conservé pendant mutation. Un ancien writer non coopérant exige preuve de cessation ; le nouveau verrou ne la remplace pas. Ne supprimer ni recréer le verrou, ne pas transmettre son handle aux enfants.

WindowFinished associe identité, plage complète et zéro ou plusieurs segments. Payload nécessaire synchronisé, record de couverture synchronisé en dernier, puis DurableAck. T avance seulement par préfixe contigu. Commit valide/ACK perdu : scan adopte ; ACK sans record valide : aucun T.

Auto, panne GPU éligible établie et corrélée : fermer l'admission des résultats d'inférence anciens, fixer M des transactions complètes déjà admises, régler cet ensemble fini et attendre arrêt corrélé ou sortie effective de l'ancien enfant. PCM Auto continue sous capacités ; issue inconnue reste attente ; les nouveaux writes PCM ne déplacent pas M. Après T stabilisé/A vérifié, intentions contrôlées, réservation durable, intentions revérifiées, spawn/handshake CPU attesté puis replay.

GPU forcé : fermer/drainer capture, régler confirmations/arrêt, sceller PCM, attendre choix humain. Choix CPU lance FinishPreservedPassage sur le même passage, sans micro rouvert ni nouveaux samples. Nouveau pending propre à la tentative : [0,T) encodage seul, [T,A) encodage/inférence, puis nouveaux préfixes Auto dans l'ordre. E repart de zéro après interruption ; T vient des records vérifiés. Ne pas concaténer un encodeur défaillant ni doubler audio/texte. Publication seulement après capture fermée, audio scellé couvert, encodeur terminé et artefacts vérifiés ; manifeste/pointeur dernier. Erreur préserve PCM, commits et publication précédente sans faux Complete.

Migration : Complete historique lu sans réécriture ; pending connu vérifié. Couverture contiguë démontrable, silence inclus : checkpoint v2/namespace neuf, frontière tentative zéro, première réservation un, originaux gardés. Couverture ambiguë : Recoverable/fragments accessibles et nouveau replay refusé ; jamais T depuis fins de mots. Version inconnue, corruption interne, identité/séquence incohérente : préservation et refus. Seul suffixe final incomplet est ignorable. Import revalide source/hash/sample rate/axe ; source modifiée ou offset ambigu refuse sans toucher la source.

Stop/Quit/annulation avant spawn empêche nouvelle tentative automatique. Après CPU lancé, fermer capture/drainer/finaliser courant ; échec CPU préserve Recoverable sans boucle. Quitter GPU forcé sans choix ne vaut pas choix CPU. Quitter reste latched ; enfant courant confirmé/stabilisé avant sortie ; UI ouverte et réactive en attente ; répétition sans Stop concurrent. Masquer ne change aucun état métier. Aucun timeout ne force une transition.

## 5. Admission, supervision et diagnostics

Appliquer l'inventaire des quatorze étages D28/61 : pool capture 100 slots partagés ; writer PCM 50 commandes ; fenêtre replay <=80 000 samples ; frame <=1 048 576 octets newline incluse ; effets 8 queued ; événement global 64, résidences et transfert comptés. Ces bornes wire ne sont pas un plafond RSS.

Réserver slots et budget du burst avant insertion. Cas 20 occupés/burst 3 dans pending capacité 21 : aucune insertion partielle ; résidence active comptée ou construction différée, aucune file cachée/troncature. Sérialisation par writer plafonné avant dépassement, pas de `to_vec` illimité suivi d'un contrôle. Stop garde une admission indépendante. Saturation réelle capture/writer ou erreur disque ferme l'entrée, préserve confirmations et expose perte connue/inconnue. Backpressure IPC seule peut laisser progresser le spool PCM. Aucun quota ni délai de panne n'est ajouté.

Q-04 : obligations indépendantes capture, audio durable, inférence, commit, réservation, encodeur, publication et arrêt. Progrès réel monotone corrélé ; heartbeat, événement obsolète ou progrès voisin ne rafraîchit pas l'obligation. Polling au plus 1/s ; avertissement après 60 s, observation nominale <=61 s sans garantie OS ; rafraîchissement diagnostic au plus toutes les 30 s si inchangé, immédiat sur erreur/transition ; un avertissement par épisode/obligation. Idle, Queued, AwaitingChoice, terminal stable et silence VAD normal ne déclenchent pas de fausse panne. Timer = diagnostic uniquement ; panne exige événement explicite.

Q-09 : quatre fichiers de 2 MiB, actif inclus, plafond 8 MiB, âge maximum 7 jours. Expiration au démarrage et avant écriture/rotation ; fichiers connus du seul répertoire diagnostics ; aucun nettoyage archive/source/pending. Writer non bloquant : <=128 records de <=4 KiB, payload <=512 KiB, overhead mesuré séparément ; pertes visibles, snapshots remplaçables. Schéma fermé : versions, UTC, IDs opaques, états/backend, codes, durées, compteurs/capacités. Audio, texte, chemins/noms libres, commande/environnement, messages natifs libres et réseau interdits. Purge impossible ou erreur IO suspend diagnostics et expose dégradation sans dépasser budgets ni perdre Stop/fragment produit. Ces logs ne prouvent pas la récupération durable.

## 6. Incréments séquentiels

I04-A, premier parcours minimal démontrable. Entrées : préflight valide et CODE L03. Implémenter identités/types/ports, policy CPU, archive couverture/journal, confirmation silence, réservation/exclusivité, worker CPU/root/UI. Sortie : live CPU court avec silence, Stop coopératif, MP3/TXT/SRT cohérents, T incluant silence, backend CPU attesté et même identité durable. Chemins : core application/compute_policy/ipc/ports/lib/tests, archive/lib/tests live_archive, worker_ipc/worker_control, worker CPU main/native_engine et desktop root/ui. Oracles V01/V02/V03/V12 ; un succès simulé ne qualifie pas le CPU natif.

I04-B, workers homogènes et admission. Dépendance CODE I04-A ; DOCUMENT D28/61. Aligner protocole CPU/GPU, WindowFinished silence, attestation, framing plafonné, budgets/résidences/bursts et Stop indépendant. Chemins : core ipc/ports, worker_ipc, CPU/GPU main/native_engine, manifest GPU/lockfile si nécessaire et tests de la portée. Sortie : handshake incompatible refusé, anciens messages drainés/rejetés, bursts refusés avant insertion, Stop conservé. Oracles V04/V05/V06.

I04-C, secours et reprise sans doublon. Dépendance CODE I04-B. Intégrer barrière M, preuves d'arrêt, réservation, Auto capture continue, strict fin préservée, reconstruction et import/source immuable. Chemins : application/policy/ports, archive/worker_ipc, workers main, root/ui et tests de la portée. Sortie : Auto/strict/reprise/crash conformes à la section 4. Oracles V07/V08/V09/V10/V11.

I04-D, supervision et consumers. Dépendance CODE I04-C. Réaliser Q-04/Q-06/Q-09, messages corrélés et commandes latched, diagnostics bornés, vraie fenêtre interactive et masquer sans arrêt. Chemins : supervisor/lib/worker_ipc, application, root/ui, worker_control. Sortie : avertissement diagnostique, Quitter coopératif et logs fermés. Oracles V13/V14/V15.

I04-E, revue, validation et clôture. Dépendance CODE I04-D ; EXECUTION campagnes requises. Revue indépendante du delta complet, correction des findings et nouvelle revue exacte ; validation attribuée, preuves de comportement et paquet execution-evidence du candidat réel. Aucun incrément n'est completed dans state avant clôture L04. GPU natif/UI non exercés restent NOT RUN et ne ferment pas leurs oracles.

## 7. Matrice comportementale — toutes lignes PROPOSED / NOT RUN

Chaque campagne garde candidat/fingerprint, inputs hashes, environnement, stimulus exact, branche parcourue, résultat attendu/observé, logs/captures, codes retour et limites.

| ID | Contrat | Stimulus et oracle |
| --- | --- | --- |
| V01 | AC-19 | Machine GPU compatible, choisir CPU avant job ; attestation CPU et aucune initialisation moteur/contexte GPU de ce job. Native nécessaire. |
| V02 | AC-13/08 | Fenêtre silencieuse puis parlée ; T avance sur plage silencieuse avec zéro segment ; audio conserve silence, texte sans ajout fictif. |
| V03 | AC-13/10/20 | Coupure avant payload sync, avant/après record, ACK perdu ; seuls commits valides contigus adoptés, absence ACK ne perd pas commit, ACK sans commit ne confirme rien, aucun Complete prématuré. |
| V04 | AC-07 | Handshake CPU/GPU incompatible ou namespace ancien ; refus avant traitement/publication, ancien résultat sans effet. |
| V05 | AC-09/11 | Files data saturées et Stop soumis ; Stop admis indépendamment ; 20 slots/burst3 ne fait pas 23 ; pas de perte d'effet accepté. |
| V06 | AC-09 | Payload à limite puis limite+1 ; reader/writer refusent avant dépassement ; occupation/capacités/transfert mesurés, pas seulement wire. |
| V07 | AC-07/13 | Panne Auto GPU établie après commits et ticket retardé ; PCM C/A progresse pendant barrière M fixe, issue inconnue attend, ancien enfant arrêté/sorti avant CPU ; même passage, T non régressif, CPU attesté, aucun double audio/texte. Fixture puis pipeline natif. |
| V08 | AC-07 | Panne GPU forcé ; micro fermé/drainé, PCM scellé, attente ; choix CPU explicite termine même passage sans réouverture. Quitter sans choix ne lance pas CPU. |
| V09 | AC-11/13 | Stop/Quit avant réservation, après réservation avant spawn, après spawn ; pas de lancement automatique si intention préalable ; après spawn courant drain/finalise, échec CPU sans retry. |
| V10 | AC-13/10/20 | Reserved sans Started, crash encodage [0,T), crash replay, suffixe incomplet/corruption interne/overflow/verrou indisponible ; numéro consommé, E neuf zéro, T vérifié, exclusivité/préservation, reprise explicite seulement, refus sans faux Complete. |
| V11 | AC-13/10 | Legacy migrable avec silence prouvé, legacy ambigu, version inconnue, source import changée ; migration admissible préserve originaux, autres cas Recoverable/refus, source hash inchangé après refus. |
| V12 | AC-08/20 | PCM scellé encodé entièrement, publication interrompue à chaque phase ; un MP3 par passage, TXT/SRT cohérents, pointeur dernier, publication précédente intacte et double scan stable. |
| V13 | AC-09 | Horloge 59/60/61 s ; capture active avec DurableAck figé, worker vivant mais inférence figée, silence normal, événement ancien ; obligations indépendantes, aucune action métier par timer. L'horloge simulée ne prouve pas le scheduling natif. |
| V14 | AC-11 | Vraie fenêtre live/import, masquer puis Quitter ; enfant retarde Stopped, commande répétée, ancien puis bon Stopped ; fenêtre réactive et attente visible, source intacte, arrêt corrélé, aucun faux Complete ni sortie anticipée. Capture réelle du consumer. |
| V15 | Q-09 | Records autorisés, sentinelles audio/texte/path/message libre, queue pleine, plafonds fichier/total/âge, purge refusée, coupure rotation ; refus fermé, aucune fuite, fichiers inconnus/archive intacts, dégradation visible, Stop/confirmation produit préservés. |

## 8. Vérifications après code — PROPOSED / NOT RUN

Developer PowerShell MSVC, cwd `C:/dev/whisper`, cible `x86_64-pc-windows-msvc`, toolchain `rust-toolchain.toml` 1.98.1, auto-install Rustup désactivé. Aucune feature nouvelle ; manifest GPU fixe déjà CUDA. Cargo séquentiel, arrêt au premier échec, logs séparés.

```text
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets --target x86_64-pc-windows-msvc
cargo test --locked -p whisper-core --test compute_policy --target x86_64-pc-windows-msvc -- --nocapture
cargo test --locked -p whisper-core --test live_contract --target x86_64-pc-windows-msvc -- --nocapture
cargo test --locked -p whisper-adapters --test worker_control --target x86_64-pc-windows-msvc -- --nocapture
cargo test --locked -p whisper-adapters --test live_archive --target x86_64-pc-windows-msvc -- --nocapture
cargo test --locked -p whisper-core --lib --target x86_64-pc-windows-msvc -- --nocapture
cargo test --locked -p whisper-adapters --lib --target x86_64-pc-windows-msvc -- --nocapture
cargo test --locked -p whisper-worker-cpu --bin whisper-worker-cpu --target x86_64-pc-windows-msvc -- --nocapture
cargo clippy --locked --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo build --locked --release -p whisper-worker-cpu --target x86_64-pc-windows-msvc --target-dir target/l04/cpu
cargo build --locked --release -p whisper-worker-gpu --target x86_64-pc-windows-msvc --target-dir target/l04/gpu
cargo build --locked --release -p whisper-desktop --target x86_64-pc-windows-msvc --target-dir target/l04/desktop
git diff --check
```

Vérifier le nom réel de target `whisper-worker-cpu` dans le manifest avant gel. Fixtures ciblées V02–V06/V09–V13/V15 ; preuves natives/UI V01/V07/V08/V14 avec exécutables séparés, modèle, WAV parlé/silencieux et import WAV/MP3 hashés, device/backend consignés. Datasets/injections dans répertoires dédiés avec sentinelles hors campagne. Distinguer panne simulée et physique. Aucun harnais inexistant n'est déclaré disponible ; ses commandes exactes seront consignées seulement après création autorisée dans la portée.

`check-build-environment` précède toute modification selon la matrice `build-environment-L04.json` : Rust/Cargo 1.98.1, MSVC, CMake, Libclang, Ninja, NVCC/CUDA_PATH et cible. La matrice héritée exige CUDA 12.8 ; une version différente n'est pas acceptée silencieusement et revient à l'architecte/coordinateur. Build CUDA ne qualifie ni runtime GPU, ni produit, ni livraison.

## 9. Préflight, état courant et récupération

1. Vérifier DESIGN D28 exact et revue indépendante CLEAN, onze rubriques, aucun REQUIRED ouvert ; puis PLANS P29 exact, parent D28, revue CLEAN et sept rubriques.
2. Vérifier règles/freeze exécutés, autorisation de préparation PLANS, autorisation de code couvrant exactement les 21 chemins et autorité scoped. L'ancienne autorisation quinze chemins est insuffisante.
3. Vérifier `RAILGUARD.md` courant, copie active, hashes et attestation d'équivalence.
4. Vérifier détails requis et prérequis DOCUMENT/CODE/EXECUTION, sortie L03, contrats sources et absence de travail concurrent sur chemins partagés.
5. Capturer HEAD/branche/status/diff scoped et hors scope, 21 hashes, données/contrats affectés, comparaison aux snapshots D28. La baseline D23/57 décrit le candidat partiel existant, pas un lot terminé. Les 21 fichiers y sont présents. Ne pas réutiliser les dix présences/cinq absences D22 comme état courant.
6. Réconcilier explicitement le candidat partiel : nouvelle baseline de lancement sourcée comparée aux snapshots D28, ou progression attribuée par preuves réelles. Aucun ledger fictif ne transforme le code partiel en completed. Écart inexpliqué suspend le lot sans reset.
7. Exécuter `check-build-environment` sur matrice acceptée avant édition.
8. Préparer pending distinct, phase IMPLEMENTATION, L04 planned, 21 chemins, autorisation valide, railguard, détails, evidence et code_state. Snapshots/repository_evidence sous racine documentaire ; current/outputs CODE sous `C:/dev/whisper`. Vue unifiée hash équivalente, documentée et contrôlée, sans réécriture des sources immuables ni remplacement opportuniste d'un doublon divergent.
9. Contrôler manifests/pending sans `--lot` puis avec `--lot L-WHISPER-04`, promouvoir atomiquement après succès et revérifier le lot.

Contrôleurs : cwd `C:/dev/whisper`, PowerShell, `python -B`, helper `C:/Users/cyril/.codex/skills/rust-predev-design/scripts/predev_control.py`. `verify-manifest` précise DESIGN puis PLANS ; `check-build-environment` précise root code et matrice ; `promote-checkpoint` précise pending/current exacts. Résultat attendu JSON ok true, identités/racines exactes, exit 0. Un refus préserve current et suspend le lancement ; aucun Cargo n'est lancé par les contrôles documentaires.

Pendant exécution, sauvegarder patches ordinaires et données. Arrêt : fermer admission concernée, intentions latched, attendre enfant corrélé et stabilisation durable hors UI, préserver Recoverable. Reprise : revérifier candidat/checkpoint/hashes/intention/exclusivité, T autoritatif et sources ; choix explicite nécessaire après crash. Nettoyage ciblé des seuls artefacts temporaires connus après inventaire et conservation des preuves. Aucun reset global ni effacement archive/source. Une campagne échouée garde preuve/fingerprint et résultat FAIL/PARTIAL.

## 10. Gate PLANS et handoff

Sept rubriques à évaluer par coordinateur/reviewer : coverage (§1/4/6/7 et autres lots hérités), dependencies (§3/6 et graphe index), preflight (§9), verification (§7/8), authorization (§2/9), railguard (§9), handoff (§2/3/6/9). Aucun PASS/CLEAN/READY n'est autodéclaré par l'auteur.

Le coordinateur conserve versions/IDs sources, transports bruts, manifests, reçus et résultats indépendants avant promotion. PLANS READY peut coexister avec le lancement code bloqué par les six chemins, la baseline de lancement ou l'environnement, avec motifs explicites. Le prochain acteur reçoit scope, contrats/types, état du dépôt, source d'autorisation, preuves L03, données de campagne, `check-state --lot` réussi et consignes de récupération. Toute décision structurante nouvelle retourne DESIGN_CHANGE_REQUIRED.
