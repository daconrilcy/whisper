Paquet original `PW-WHISPER-01 v1`, auteur `/root/plans_writer`, rôle `rust_plan_writer`, lecture seule. Base intellectuelle et parent : `D-WHISPER-19`, périmètre 2. Première lignée PLANS : `base_candidate_id=null`. Identifiant proposé : `P-WHISPER-01`. Aucun fichier écrit.

Contrôles personnellement exécutés :

- `verify-manifest` D19 : PASS, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.
- `check-state` : PASS, phase DESIGN ; D19 READY et R19 CLEAN liés au digest.
- Revue `transports/T-WHISPER-REVIEW-19/review-D19.json` lue.
- Comparaison des 21 références de `15_PACK_MANIFEST.json` aux fichiers centraux actuels : neuf fichiers différents, listés ci-dessous.
- Aucun manifeste Cargo, code produit, AGENTS ou railguard actif trouvé ; `git status --short` vide au moment du contrôle.

Sources consultées : skill et cinq règles centrales ; contrats canonique, engineering, workflow, deliverable-quality et qualification ; D19 documents 00/02/03/06/07/08/09/10/11/15/32/47/49/50/51 ; TRANSPORT `T-WHISPER-ARCH-D18-FINAL/arch-D18-final-raw.md` ; state et revue R19. Les anciens libellés DRAFT restent historiques : l’état contrôlé porte le statut courant.

Pas de `DESIGN_CHANGE_REQUIRED` identifié. Proposition PLANS DRAFT ; persistance, TRANSPORT et revue indépendante restent à effectuer par le coordinateur/hôte.

Les quatre contenus suivants sont prêts à persister.

### `00_PLANS_INDEX.md`

```markdown
# Plans Whisper — P-WHISPER-01

Version 1 ; statut proposé DRAFT ; périmètre 2 ; parent D-WHISPER-19.
Auteur : /root/plans_writer, rust_plan_writer.
Première version PLANS : base_candidate_id=null.

Le parent est D-WHISPER-19, digest
903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529.
La source du statut courant est ../../state.json, après vérification
canonique du manifeste et de R-WHISPER-DESIGN-19. Les mentions DRAFT du
corpus DESIGN immuable décrivent son état avant promotion.

Le périmètre conserve REQ-01..25 et UC/AC-01..20.
TECH-D18-01..08 sont repris sans changement ; les précisions D19 de
51_CLOSURE_INSTALL_MODES_D19.md prévalent sur les preuves E2 retirées de D18.

## Documents

- 01_LOTS.md : lots, entrées/sorties, contrats, couverture et dépendances.
- 02_VERIFICATION_AND_PREFLIGHT.md : commandes proposées, données,
  résultats comportementaux, preuves et récupération.
- 03_CENTRAL_PACK_IMPACT.md : différence des règles centrales et portée.

Sources produit et techniques : ../D-WHISPER-19/02_REQUIREMENTS.md,
07_COVERAGE_AND_ACCEPTANCE.md, 08_STATE_AND_PORT_CONTRACTS.md,
11_RAILGUARD_PROPOSAL.md, 32_USER_DECISIONS.md,
47_USER_VALIDATION_D17.md, 50_ARCH_DECISIONS_D18.md et
51_CLOSURE_INSTALL_MODES_D19.md.
Source originale des choix techniques :
../../transports/T-WHISPER-ARCH-D18-FINAL/arch-D18-final-raw.md.
Revue parent :
../../transports/T-WHISPER-REVIEW-19/review-D19.json.

## Ordre

| Lot | Résultat observable | Dépendance de lot |
| --- | --- | --- |
| L-WHISPER-00 | Bootstrap et contrats compilables sans natif dans le core | Aucune |
| L-WHISPER-01 | WAV -> worker CPU -> TXT/SRT -> historique minimal visible | L00 CODE |
| L-WHISPER-02 | Journal, file FIFO et restauration sans départ spontané | L01 EXECUTION |
| L-WHISPER-03 | Live, VAD, MP3 par passage et Stop + Reprise | L02 EXECUTION |
| L-WHISPER-04 | GPU strict, Auto, génération et contrôle indépendant | L03 EXECUTION |
| L-WHISPER-05 | Réglages, tray, autostart, hotkey et gestion archives | L04 EXECUTION |
| L-WHISPER-06 | Installation, payload CPU/GPU et premier usage offline | L05 EXECUTION |
| L-WHISPER-07 | Qualification intégrée du périmètre V1 | L06 EXECUTION |

DAG : L00 -> L01 -> L02 -> L03 -> L04 -> L05 -> L06 -> L07.
Toutes les dépendances EXECUTION désignent une campagne future NOT RUN :
elles n'ont actuellement aucune preuve de completion.
La dépendance CODE L01<-L00 exige les API/artefacts réels de L00 ; le
document PLANS seul ne fournit pas ces API.

Chaque lot a en outre des entrées DOCUMENT : parent DESIGN, PLANS exacts,
revues liées, contrats REQ/UC/AC et décisions applicables. Ce sont des
sources du lot, et non des lots fictifs ajoutés au DAG.

Aucune parallélisation annoncée : Cargo.toml/Cargo.lock, ports application,
scheduler, archive et composition roots sont partagés. Le propriétaire du
lot courant possède ces changements ; une extraction de tâche parallèle
exige une nouvelle répartition explicite des chemins et contrats.

## Autorisations et railguard

La préparation et la revue des plans ont été autorisées par le message
utilisateur de reprise D19. L'autorisation d'implémenter est absente.
Le railguard D19/11 reste proposé et inactif.

Avant L00, l'autorité utilisateur/gouvernance doit autoriser l'implémentation
sur les lots et chemins retenus, et l'activation du railguard applicable.
L'attestation doit porter chemin cible, hash de proposition revue, hash
actif, date et équivalence d'effet normatif. Aucune activation n'est faite
par ces plans.

L00 est le premier lot à préflighter après ces autorisations ; L01 est le
premier parcours produit démontrable. Aucun lot n'est actuellement exécuté.

## Détails à résoudre avant les lots concernés

Les choix ci-dessous sont réversibles dans TECH-D18-08 et ne changent pas
les garanties acceptées. Ils doivent être attribués à rust_architect avant
leur preflight ; le plan writer ne les accepte pas à sa place.

- DETAIL-P01, avant L00/L01 : disposition physique finale, noms des
  packages/features/binaires, versions de toolkit UI et décodeur.
  Les chemins de 01 sont une proposition adaptable, pas une nouvelle
  architecture acceptée.
- DETAIL-P02, avant L03 : version capture, appareils/formats gérés,
  capacités et tailles maximales de tous les étages ; Q-07 profils
  codecs/bitrate et rejets.
- Q-04, avant L04 : signaux de progrès par état et marge de diagnostic
  autour des 60 s indicatives ; aucun arrêt fondé sur le temps seul.
- Q-09, avant diagnostics L04 : rétention des logs techniques locaux,
  sans audio ni texte.
- DETAIL-P03/Q-06, avant L04/L05 : geste manuel concret quand Quitter
  reste bloqué ; aucun kill automatique repris du nettoyage des bancs.
- DETAIL-P04, avant L07 : protocole et mandat du corpus représentatif
  micro FR/EN calme/bruit courant. DEC-28 autorise seulement les WAV
  existants pour le cadrage actuel ; de nouveaux enregistrements ne
  sont pas implicitement autorisés.

Si une réponse impose un nouveau stockage, une autre garantie, une
réduction GPU V1, de la télémétrie ou une modification des frontières,
retourner DESIGN_CHANGE_REQUIRED et suspendre les lots impactés.
```

### `01_LOTS.md`

```markdown
# Lots proposés — P-WHISPER-01

Tous les lots sont planned ; toutes les campagnes produit sont NOT RUN.
Chaque lot hérite du preflight et de la récupération de 02.
Chaque tâche ci-dessous renvoie aux REQ/UC/AC ou TECH explicités dans
son lot ; aucune tâche produit hors de ce périmètre.

Les chemins proposés sont à créer. Ils sont précis pour l'ownership et
pour les commandes proposées ; DETAIL-P01 peut les adapter avant exécution
avec concordance documentaire revue. Leur regroupement ne peut modifier
les frontières ni l'isolation CPU/GPU.

## L-WHISPER-00 — bootstrap et contrats

Objectif : rendre les responsabilités et le protocole compilables.
Préalable technique : TECH-D18-01/04/08, RG-01/07/09/10.
Entrées DOCUMENT : DESIGN/PLANS/revues exacts, activation attestée et
autorisation d'implémenter ; DETAIL-P01 résolu.
Prérequis de lot : aucun.

Chemins proposés :
Cargo.toml, Cargo.lock, rust-toolchain.toml ;
crates/whisper-core/Cargo.toml ;
crates/whisper-core/src/lib.rs ;
crates/whisper-core/src/domain.rs ;
crates/whisper-core/src/application.rs ;
crates/whisper-core/src/ports.rs ;
crates/whisper-core/tests/boundaries.rs ;
crates/whisper-adapters/Cargo.toml ;
crates/whisper-adapters/src/lib.rs ;
crates/whisper-worker-cpu/Cargo.toml ;
crates/whisper-worker-cpu/src/main.rs ;
crates/whisper-worker-gpu/Cargo.toml ;
crates/whisper-worker-gpu/src/main.rs ;
crates/whisper-desktop/Cargo.toml ;
crates/whisper-desktop/src/main.rs ;
crates/whisper-desktop/src/ui.rs ;
crates/whisper-bootstrap/Cargo.toml ;
crates/whisper-bootstrap/src/main.rs.

Étapes : fixer dans les manifests la pile D18 autorisée et les détails
techniques attribués ; définir identités, JobConfig, transitions pures,
façade/vues et ports application ; définir messages versionnés et erreurs ;
assembler les racines sans logique métier parallèle.

Sorties CODE : core sans adaptateurs natifs, protocole job/generation/
segment/range, façades et squelettes de racines ; lockfile et inventaire.
CPU et GPU sont deux packages/binaires distincts ; les builds doivent
éviter l'unification de features CUDA.

Acceptation : core compile seul ; domain n'importe pas application/OS/UI ;
UI n'accède pas directement aux adaptateurs ; desktop/bootstrap sans
Whisper/CUDA ; visibilité des ports et ownership conformes à TECH08.
Vérifications : V-BASE/V-BOUNDARY, NOT RUN.
Exclusion : aucune transcription ni garantie produit déclarée réalisée.

## L-WHISPER-01 — premier parcours WAV CPU observable

Covers REQ-03/06/12/13/24 ; verifies AC-03/06/10/19.
Needs UC-03/06/10/19, TECH-D18-01/03/06/08.
Prérequis L00 CODE : core/ports/racines/lockfile disponibles, avec hashes
et preuves ; les seuls contrats documentaires ne suffisent pas.

Chemins supplémentaires :
crates/whisper-adapters/src/decoder.rs ;
crates/whisper-adapters/src/archive.rs ;
crates/whisper-adapters/src/worker_ipc.rs ;
crates/whisper-core/tests/import_contract.rs ;
crates/whisper-adapters/tests/import_cpu.rs.
Modifications : core application/ports, worker-cpu main, desktop UI/root.

Entrées : WAV existant manifesté, modèle local vérifié D18, CPU forcé et
langue ; dossier fixture explicitement choisi. Le modèle est un prérequis
du démonstrateur local ; ce chemin ne remplace pas l'installateur L06.

Étapes : sélectionner/prévalider WAV ; produire PCM/offsets dans adaptateur ;
lancer le vrai worker CPU ; attester backend avant résultat ; afficher
progression dans le consumer desktop ; publier TXT/SRT/manifeste/pointeur
selon TECH06 ; scanner et ouvrir la transcription depuis l'historique
minimal. Préserver source ; erreur entrée/modèle/dossier visible.

Sorties : premier import réel complet, TXT/SRT cohérents et référence source,
sans copie MP3 ; scan minimal reconnaissant Complete ou Recoverable.

Acceptation : un WAV réel produit texte/SRT visible et fichiers vérifiés ;
source hashée identique ; CPU effectif attesté même sur PC GPU ; langue
manuelle prioritaire ; erreur/corruption n'affiche jamais Complete.
Un résultat moteur mocké ne valide pas ce parcours.
Vérifications V-IMPORT et V-UI, NOT RUN.
Exclusions : MP3 import, file restaurée, live, GPU et fonctionnalités OS.

## L-WHISPER-02 — journal, file et restauration

Covers REQ-03/04/12/13/16/17/18 ;
verifies AC-03/04/10/11/12/13.
Needs UC-03/04/10/11/12/13, TECH06, RG-02/03/04/05.
Prérequis L01 EXECUTION : parcours CPU réel validé, evidence de completion.

Chemins supplémentaires :
crates/whisper-adapters/src/journal.rs ;
crates/whisper-adapters/src/queue_store.rs ;
crates/whisper-adapters/src/recovery.rs ;
crates/whisper-core/tests/scheduler_contract.rs ;
crates/whisper-adapters/tests/durability.rs.
Modifications : application/ports/domain, archive/desktop UI.

Entrées : jobs/config/source identity ; dossier fixture de session/queue.
Étapes : persister fragments audio+texte avant DurableAck ; implémenter
FIFO versionné, ACK après sync, identité source revalidée au traitement ;
queued restauré AwaitingChoice, running restauré Interrupted ; Traiter/
Retirer explicites, confirmation avant reprise ; scan des générations,
pending et corruption idempotent ; annulation import et sortie source intacte.

Sorties : journal/session et file durable distincts de l'historique ;
diagnostic plage acquise/non confirmée ; restauration sans travail spontané.

Acceptation : injections à chaque frontière sync/record/pointeur ; aucun
ACK précoce ; préfixe confirmé récupéré ; deux scans identiques ; aucun
partiel Complete ; FIFO multi-ID ; source modifiée/absente visible ; retrait
et annulation ne touchent pas la source ; Start refusé pendant import.
Vérifications V-DURABLE/V-SCHEDULER, NOT RUN.
Limite : preuve arrêt de processus, sans prétention universelle powerloss.

## L-WHISPER-03 — live, VAD et passages

Covers REQ-01/02/03/04/09/10/12/18/25 ;
verifies AC-01/02/03/04/08/10/13/20.
Needs UC-01/02/03/04/08/10/13/20, TECH04/05/06/07 et DEC26/27/34/35.
Prérequis L02 EXECUTION : journal, queue et recovery validés.
Détails nécessaires : DETAIL-P02/Q-07 ; acquisition représentative P04
non requise pour le premier smoke autorisé sur WAV existants.

Chemins supplémentaires :
crates/whisper-adapters/src/capture.rs ;
crates/whisper-adapters/src/vad.rs ;
crates/whisper-worker-cpu/src/encoder.rs ;
crates/whisper-core/tests/live_contract.rs ;
crates/whisper-adapters/tests/live_archive.rs.
Modifications : worker CPU, application/ports/domain, archive/journal,
desktop UI/root et manifests concernés.

Étapes : pending durable avant capture ; callback sans IO/send bloquant ;
spool et canaux bornés ; compteur parole VAD séparé de temps total ;
texte provisoire/confirmé et horodatages capture/disponibilité/premier rendu ;
Stop ferme entrée puis draine/finalise ; AutoStop seulement si activé ;
nouveau Start explicite après Stop + Reprise, IDs et pause persistée ;
un MP3 immuable par passage, TXT/SRT cumulés selon horloge PCM originale.

Sorties : première chaîne live complète et groupe multipassage.
Acceptation : Start répété n'ouvre qu'un handle ; live/import exclusifs ;
imports en attente ne commencent qu'après finalisation stabilisée ; silence
conservé dans audio, exclu du seul compteur parlé ; un passage Recoverable
ne produit pas groupe Complete ; aucun Start automatique ; CPU retard
visible sans plafond temps réel ; SRT n'utilise jamais durée MP3 décodée.
VAD Rust rejoué sur les 26 WAV et annotations D17 : seuils DEC35 dans leur
seule portée ; aucun PASS représentatif micro FR/EN/bruit déduit.
Vérifications V-LIVE/V-VAD/V-DURABLE/V-UI, NOT RUN.
Exclusion : alignement sample-exact dans lecteur MP3 tiers.

## L-WHISPER-04 — modes, saturation et supervision

Covers REQ-07/08/11/14/15/16/18/24 ;
verifies AC-07/09/11/13/19.
Needs UC-07/09/11/13/19, TECH01/03/04 et Q04/Q06/Q09 résolus.
Prérequis L03 EXECUTION : live/confirmation/passage validés.

Chemins supplémentaires :
crates/whisper-adapters/src/supervisor.rs ;
crates/whisper-worker-gpu/src/native_engine.rs ;
crates/whisper-worker-cpu/src/native_engine.rs ;
crates/whisper-core/tests/compute_policy.rs ;
crates/whisper-adapters/tests/worker_control.rs.
Modifications : worker_ipc, application/ports, worker roots, desktop UI.

Étapes : backend effectif attesté après contexte/state et avant inférence ;
GPU strict s'arrête sans CPU automatique ; Auto invalide génération et
relance EXE CPU au dernier offset durable ; résultat GPU périmé rejeté
avant journal/publication ; contrôle indépendant de l'IPC audio ; toutes
capacités instrumentées ; plein cesse entrée avec diagnostic/plage perdable ;
différencier silence/inférence lente/panne ; Quitter bloqué reste ouvert.

Sorties : modes et supervision réellement exercés ; modes/motifs visibles.
Acceptation : GPU absent et panne worker avant/après inférence ; strict
sans publication ni CPU ; Auto exactement une fois par ID/range ; CPU
forcé sans initialisation CUDA ; vieux résultat réel rejeté ; Stop accessible
pendant saturation ; pas d'arrêt au temps seul ; pas de kill automatique.
Vérifications V-MODES/V-CONTROL/V-UI, NOT RUN.
Limites : injection typée distincte de panne physique pilote.

## L-WHISPER-05 — réglages et intégration Windows

Covers REQ-06/14/15/16/19/20/21/22/23 ;
verifies AC-06/11/14/15/16/17/18.
Needs UC-06/11/14..18, Q03/05/06/09, RG04/08.
Prérequis L04 EXECUTION : lifecycle et supervision validés.

Chemins supplémentaires :
crates/whisper-adapters/src/settings.rs ;
crates/whisper-adapters/src/os.rs ;
crates/whisper-core/tests/settings_lifecycle.rs ;
crates/whisper-adapters/tests/windows_integration.rs.
Modifications : archive/recovery, desktop UI/root, application/ports.

Étapes : paramètres versionnés/validation/commit ou ancienne valeur ;
snapshot par job incluant dossier ; inventaire suppression avec confirmation ;
historique depuis fichiers et ouverture TXT/SRT/dossier ; tray/notifications ;
fermeture masque sans interruption ; autostart opt-in/désactivable sans
capture ni fenêtre ; instance unique ; hotkey configurable et conflits.

Sorties : consumer desktop complet et réglages/OS fonctionnels.
Acceptation : chemin effectif explicite et aucun déplacement implicite ;
dossier inaccessible/plein signalé ; suppression bornée archives, source
jamais touchée ; paramètres invalides conservent précédent ; job actif
inchangé ; login/double lancement une instance inactive ; raccourci refusé
durant états incompatibles ; fenêtre/tray montrent cause/Recoverable.
Vérifications V-OS/V-HISTORY/V-UI, NOT RUN.

## L-WHISPER-06 — installation et package

Covers REQ-05/07/08/24 ; verifies AC-05/07/19.
Needs UC05/07/19, TECH01/02/03, RG08/09 et CHANGE001.
Prérequis L05 EXECUTION : application/lifecycle validés.

Chemins supplémentaires :
crates/whisper-bootstrap/src/acquisition.rs ;
crates/whisper-bootstrap/src/prerequisites.rs ;
crates/whisper-bootstrap/tests/install_contract.rs ;
docs/release/whisper-payload.json ;
docs/release/whisper-notices.md.
Modifications : bootstrap root/manifests ; pas de téléchargement dans worker.

Étapes : deux builds natifs isolés ; inventory versions/hash/imports PE ;
DLL CUDA privées, nvcuda pilote non copiée ; VC Redist compatible explicite ;
WinHTTP TLS/redirect/200/206/416 et reprise contrôlée ; modèle fixe
commit6034871ec87c84e342efab769d4c5c06cd126db3,
taille1624555275,
SHA2561fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69 ;
staging/hash/prérequis avant Ready ; notices et provenance de redistribution.

Sorties : payload installable, inventaire et premier usage offline.
Acceptation : interruption/hash faux/troncature/prérequis absent ou refusé
ne font pas Ready ; 200 non concaténé ; 206/Content-Range et identité
validés ; 416 géré ; TLS non désactivé ; runtime absent et redémarrage
requis visibles ; workers chargent modèle local sans téléchargement caché.
Vérifications V-INSTALL/V-PE/V-OFFLINE, NOT RUN.
PC propre demeure différé par CHANGE001 ; publication non autorisée.

## L-WHISPER-07 — qualification intégrée

Covers REQ01..25 ; verifies AC01..20.
Needs UC01..20, décisions acceptées et preuves des lots précédents.
Prérequis L06 EXECUTION : payload et campagnes install validés.
Détail/mandat nécessaire : DETAIL-P04 pour nouveaux micros/enregistrements.

Chemins nouveaux limités aux rapports de campagnes attribuées sous
docs/qualification/ ; aucune modification DESIGN/PLANS pour enregistrer
une progression de code prévue. Les findings produit retournent au worker
d'implémentation, puis à une revue indépendante du candidat corrigé.

Étapes : exécuter matrice complète dans consumer final et payload exact ;
live/import, restaurations, trois modes, langues, réglages/OS, codecs,
erreurs disque/micro/worker ; corpus micro FR/EN calme/bruit et sessions
longues selon mandat ; mesurer latence capture->rendu, retard, buffers,
perte et VAD sans seuil global inventé ; distinguer tests, UI native,
qualification livraison et publication.

Sorties : rapport exact candidat/version/données/environnement/limites.
Acceptation : AC01..20 renseignés avec preuve/échec/limite ; aucun succès
de banc D18/D19 converti en test produit ; chaque refus pertinent exercé.
PC propre et panne électrique ne sont pas requis artificiellement par
ces plans ; leur statut reste explicitement différé/non prouvé.
Vérifications : toutes campagnes V, NOT RUN.
```

### `02_VERIFICATION_AND_PREFLIGHT.md`

```markdown
# Préflight, vérifications et récupération — P-WHISPER-01

## Préflight de chaque lot

1. Exécuter le contrôleur central actuel en Python -B, cwd C:/dev/whisper,
   shell PowerShell. Vérifier manifests DESIGN et PLANS puis check-state.
   Commandes proposées :
   python -B C:/Users/cyril/.codex/skills/rust-predev-design/scripts/predev_control.py verify-manifest --root C:/dev/whisper/docs/predev --manifest C:/dev/whisper/docs/predev/candidates/D-WHISPER-19/manifest.json
   python -B C:/Users/cyril/.codex/skills/rust-predev-design/scripts/predev_control.py verify-manifest --root C:/dev/whisper/docs/predev --manifest C:/dev/whisper/docs/predev/candidates/P-WHISPER-01/manifest.json
   python -B C:/Users/cyril/.codex/skills/rust-predev-design/scripts/predev_control.py check-state --root C:/dev/whisper/docs/predev --state C:/dev/whisper/docs/predev/state.json
   À l'exécution, utiliser --lot L-WHISPER-NN uniquement sur checkpoint
   IMPLEMENTATION muni des preuves d'autorisation/railguard/prérequis.
   Résultat attendu : octets parent/plans exacts, verdicts indépendants
   liés à leurs IDs/digests, aucune exigence de gate manquante.

2. Vérifier source/portée autorisation de coder et activation railguard :
   chemin/hashes/date/équivalence. Elles sont absentes actuellement.

3. Vérifier branche/HEAD et modifications ordinaires :
   git rev-parse --show-toplevel
   git rev-parse HEAD
   git status --short
   git diff --stat
   Attendu : checkout C:/dev/whisper identifié ; tous changements
   inventoriés, chemins du lot attribués ; aucune divergence inexpliquée.

4. Vérifier entrées DOCUMENT ; sorties CODE réelles et hashes ; completion
   des fournisseurs EXECUTION avec résultats comportementaux conservés.
   Vérifier décisions/détails nécessaires et disponibilité du modèle,
   fixtures, micro/OS/GPU/runtime selon lot. Preflight FAIL suspend le lot.

5. Ledger d'exécution distinct : base HEAD et hashes, lot/candidat,
   sorties/hash/commits, campagnes et preuves. Progression prévue des lots
   précédents est consignée ; une divergence inattendue revient au
   coordinateur sans invalider mécaniquement tout le corpus.

## Commandes proposées après code

Toutes les commandes et campagnes ci-dessous sont NOT RUN.
Cwd général C:/dev/whisper ; shell PowerShell ; cible
x86_64-pc-windows-msvc ; toolchain Rust/Cargo1.98.1 édition2024.
Noms packages/tests proposés par 01, à confirmer DETAIL-P01.

V-BASE :
cargo fmt --all -- --check
cargo test --locked -p whisper-core
cargo clippy --locked -p whisper-core -p whisper-adapters -p whisper-desktop -p whisper-bootstrap --all-targets -- -D warnings
git diff --check
Attendu : format, erreurs contractuelles et absence avertissements ;
la revue de frontières demeure nécessaire même si exit0.

V-BOUNDARY :
cargo metadata --locked --format-version 1 --no-deps
cargo tree --locked -p whisper-desktop -e features
cargo tree --locked -p whisper-bootstrap -e features
cargo check --locked -p whisper-core --target x86_64-pc-windows-msvc
Données : manifests/visibilité/imports du candidat.
Attendu : core sans OS/IO/native/UI ; application sans adaptateurs ; UI
via façade ; desktop/bootstrap sans Whisper/CUDA ; inspection consumers
et unsafe conservée avec la sortie metadata/tree.

Builds séparés, séquentiels :
cargo build --locked --release -p whisper-worker-cpu --target x86_64-pc-windows-msvc --target-dir target/cpu
cargo build --locked --release -p whisper-worker-gpu --target x86_64-pc-windows-msvc --target-dir target/gpu
cargo build --locked --release -p whisper-desktop -p whisper-bootstrap --target x86_64-pc-windows-msvc --target-dir target/desktop
Ne pas lancer cargo build --workspace --all-features pour qualifier
séparation CPU/GPU. Chaque package fixe ses features natives selon TECH01.

V-IMPORT : cargo test --locked -p whisper-adapters --test import_cpu -- --nocapture
V-DURABLE : cargo test --locked -p whisper-adapters --test durability -- --nocapture
V-SCHEDULER : cargo test --locked -p whisper-core --test scheduler_contract
V-LIVE : cargo test --locked -p whisper-adapters --test live_archive -- --nocapture
V-MODES : cargo test --locked -p whisper-core --test compute_policy
V-CONTROL : cargo test --locked -p whisper-adapters --test worker_control -- --nocapture
V-OS : cargo test --locked -p whisper-adapters --test windows_integration -- --nocapture
V-HISTORY : campagne UI/history de V-UI plus recovery/durability.
V-INSTALL : cargo test --locked -p whisper-bootstrap --test install_contract -- --nocapture

Chaque test/harness attendu dans 01 doit exercer le vrai port/adaptateur
applicable ; les tests purs de compute_policy sont complétés par le
worker natif dans V-UI/V-OFFLINE. Un mock ou exit0 n'est pas preuve CPU/GPU.

Fixtures : copies autorisées des WAV et annotations exacts D19, source
hashée avant/après ; dossiers isolés dédiés sous la destination d'essai
autorisée ; modèle réel approuvé vérifié par taille/SHA ; fichiers invalides,
corrompus/tronqués, fichiers absents, refus d'accès et saturation.
Ne jamais modifier les corpus DESIGN pour injecter une panne.

V-VAD : harness L03 sur les 26 WAV/human-annotations de D17. Commande
proposée portée par test live_archive et options de fixture à documenter
dans son protocole avant exécution. Attendu : TP/FP/FN/TN, agrégés et quatre
groupes, comparaison DEC35 ; portée corpus DESIGN explicitement maintenue.

V-PE : depuis shell Developer PowerShell MSVC qualifié :
dumpbin /DEPENDENTS target/cpu/x86_64-pc-windows-msvc/release/whisper-worker-cpu.exe
dumpbin /DEPENDENTS target/gpu/x86_64-pc-windows-msvc/release/whisper-worker-gpu.exe
dumpbin /DEPENDENTS target/desktop/x86_64-pc-windows-msvc/release/whisper-desktop.exe
Attendu : CPU/desktop sans imports CUDA ; GPU DLL privées attendues ;
driver nvcuda et VC runtime identifiés. Conserver aussi inventory complet
des payloads/runtime/notices et SHA. Les imports PE seuls ne prouvent
pas les chemins exacts des modules chargés.

V-UI : lancer le binaire desktop exact puis agir dans ses vrais consumers
fenêtre/tray/réglages/historique :
& ./target/desktop/x86_64-pc-windows-msvc/release/whisper-desktop.exe
Données et scénario choisis dans le protocole de lot : WAV modèle validés,
micro autorisé et dossier dédié. Aucun flag caché inventé.
Attendu : états et effets décrits par les AC du lot ; captures/logs techniques
et horodatages liés au candidat. Réelles capture/disponibilité/premier rendu
mesurées ; une vue simulée ou réponse du shell n'est pas qualification UI.

V-OFFLINE : lancer le payload installé dans environnement sans réseau
effectivement attesté, CPU puis GPU selon choix UI. Le protocole doit
identifier moyen d'isolation, témoin connexion normale/refus isolé,
hashes payload/modèle, identité environnement et résultats.
Ne pas reprendre aveuglément les outils du banc D19 comme outils produit.
Attendu : modèle local ; aucune acquisition cachée ; CPU/GPU attestés.

Campagnes installation : serveur fixture contrôlé pour 200/206/416,
Range/ETag/Content-Range, troncature/hash faux ; HTTPS/redirection externe
et modèle réel depuis source fixe avec mandat d'exécution applicable ;
branches runtime absent/refus/redémarrage en environnement dédié.
Attendu : Ready seulement après validation et prérequis.
Commande UI du bootstrap :
& ./target/desktop/x86_64-pc-windows-msvc/release/whisper-bootstrap.exe

## Preuves et récupération bornée

Conserver commande exacte/cwd/shell/cible/features, données/hash,
environnement, stimulus injecté, branche réellement parcourue,
effet observé, stdout/stderr/code, captures et limites. Classer
PRODUCT_VALIDATION ou DELIVERY_QUALIFICATION ; les bancs historiques
restent DESIGN_FEASIBILITY.

En échec : arrêter l'entrée du scénario, conserver journaux/pending et
preuves ; identifier lot/contrat/path impacté. Le worker corrige dans son
périmètre, puis revue indépendante et vérification ciblée sur nouveau
candidat. Deux essais ciblés maximum sur une cause inchangée avant retour
au coordinateur ; ne pas boucler en élargissant les permissions.

La récupération du produit suit ses règles durables : aucune suppression
de source, aucun Complete non prouvé, aucune exécution restaurée sans choix.
L'application peut rester ouverte quand Quitter est bloqué ; la borne de
campagne ne crée pas une promesse de shutdown FFI borné.

Ne pas reset global/stash forcé/clean récursif. Préserver modifications
ordinaires ; ne supprimer que les artefacts de fixture expressément
autorisés, avec chemins absolus vérifiés et journaux déjà conservés.
Ne pas supprimer un lock ou répertoire récupérable sans qualification.
```

### `03_CENTRAL_PACK_IMPACT.md`

```markdown
# Impact du pack central sur D19 et les plans

Auteur : /root/plans_writer, analyse en lecture seule, 2026-10-05.
Parent conservé proposé : D-WHISPER-19.
Proposition à soumettre à la revue indépendante des PLANS.

## Contrôles

Le contrôleur actuel verify-manifest a vérifié personnellement D19 :
digest903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529.
check-state actuel : PASS, phase DESIGN.
L'état porte D19 READY et R-WHISPER-DESIGN-19 CLEAN lié au digest.
Le rapport R19, ses limites et son indépendance ont été lus.

Comparaison réelle du freeze D19/15_PACK_MANIFEST.json aux sources centrales :
9 différences sur21 :
agents/rust_architect.toml
agents/rust_design_reviewer.toml
agents/rust_predev_orchestrator.toml
skills/rust-predev-design/SKILL.md
skills/rust-predev-design/references/deliverable-quality.md
skills/rust-predev-design/references/engineering-contract.md
skills/rust-predev-design/references/handoff-contract.md
skills/rust-predev-design/references/workflow-schema.md
skills/rust-predev-orchestration/SKILL.md

Les fichiers du corpus D19 restent inchangés et manifest-valides.
Cette comparaison établit le changement de références ; la qualification
native effective des nouveaux profils demeure une preuve distincte.

## Effets

| Évolution | Effet sur conception D19 | Application future |
| --- | --- | --- |
| Provenance brute exacte, comparaison export/proposal/TRANSPORT sans normalisation | Ne change pas TECH01..08 ; contrôle documentaire renforcé | Vérifier chaque nouveau apport ; déclarer limite si source conversationnelle non exportée |
| Exécutant SPIKE distinct et séparément autorisé, hôte documentaire limité aux docs | Ne change pas modèle/worker/archive/VAD ; séparation d'autorités renforcée | Aucun futur banc depuis mandat docs ; protocole/mandat/exécutant dédiés |
| Stimulus/branche/effet requis pour qualifier preuve | D19 possède précisément attestation avant state.full, erreur73 réelle, validation commune, rejet stale et témoin offline | Refaire ces contrôles sur produit ; aucune preuve adjacente utilisée |
| État courant contrôlé dans state.json | Évite interpréter le DRAFT prépromotion comme statut actuel | PLANS lisent statut exact et revues ; corpus parent immuable conservé |
| Classement DESIGN_FEASIBILITY/PRODUCT_VALIDATION/DELIVERY_QUALIFICATION | Aucune nouvelle exigence produit ; clarifie portée existante | Campagnes des lots séparées, toutes NOT RUN avant exécution |

## Limite historique

Les bancs D18/D19 sont attribués au coordinateur dans49/51 et dans la
contribution originale ARCH-D18. Ils ont précédé le renforcement imposant
un exécutant séparé du coordinateur/hôte documentaire.
Leur attribution n'est ni réécrite ni présentée rétroactivement comme
conforme à cette nouvelle séparation.

Leurs sources/logs/locks/rapports sont néanmoins conservés et examinés par
R19 indépendamment ; les résultats du banc restent des faisabilités
bornées. La séparation insuffisante selon la règle nouvelle limite la
réutilisation du workflow d'exécution historique ; elle ne démontre pas
une erreur du backend, du protocole durable ou des choix TECH01..08.

R19 CLEAN reste la revue historique exacte de D19. Le présent auteur
ne le ferme ni ne le réémet. L'application future du pack actuel et
l'impact de cette limite sont explicitement à faire confirmer par la
revue indépendante des PLANS. Si elle identifie une preuve indispensable
invalidée, suspendre les lots touchés et retourner au coordinateur ;
ne pas fabriquer une conformité rétroactive.

## Proposition

Conserver D19 et son digest. Aucun ajout de périmètre ou remplacement
TECH-D18-01..08 n'est nécessaire pour appliquer les règles nouvelles.
Documenter la gouvernance actuelle dans PLANS et conserver les sources
centrales effectives/freeze avec provenance, sans modifier les snapshots
immuables de D19.

Aucun DESIGN_CHANGE_REQUIRED identifié par ce plan writer.
Le statut PLANS reste DRAFT avant persistance complète, TRANSPORT
attribué, manifeste exact et revue indépendante. Aucun CLEAN/READY
autoattribué. Railguard inactif ; autorisation coding absente.
```

Travail restant du coordinateur : persister corpus et contribution originale, vérifier reçus et référence checkpoint, inventorier les sources/règles effectives dans le manifeste PLANS, matérialiser `elements`/lots/tasks selon le schéma, puis faire revoir l’exact candidat. Les détails ouverts bloquent leur préflight, sans constituer des décisions produit acceptées.

<oai-mem-citation>
<citation_entries>
MEMORY.md:134-139|note=[continuite provenance TRANSPORT et checkpoint]
MEMORY.md:146-147|note=[ne pas reconstruire les contributions historiques manquantes]
</citation_entries>
<rollout_ids>
01a10b44-3030-7da3-89f3-0d664e3bbf8d
</rollout_ids>
</oai-mem-citation>