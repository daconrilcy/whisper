# Préflight et vérifications — P-WHISPER-08

P07 est la base immuable. Les revues, contrôles et préflights P06/P07 restent historiques ; appliquer seulement le manifeste, la revue exacte et l’état contrôlé P08.

P07 est la base immuable. Les revues, contrôles et préflights P06/P07 sont historiques et ne valent pas preuve pour P08. Seuls manifeste, revue exacte et état contrôlé P08 s’appliquent.

Toutes les campagnes produit sont PROPOSED / NOT RUN. Environnement proposé : PowerShell, cwd `C:\dev\whisper`, cible `x86_64-pc-windows-msvc`, Rust/Cargo 1.98.1, édition 2024. L00 a ses propres preuves ; elles ne prouvent pas un import réel L01.

## Candidat et contrôle documentaire

Contrôleur effectif : `C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py`. Avant usage, vérifier son SHA contre la copie gelée dans P06 et le pack manifest courant. Vérifier D19 et P06 avec `verify-manifest`; le verdict PLANS attendu doit cibler P06 et son digest exact. P05 est uniquement la base non promue. Le manifeste seul ne donne pas READY.

Les chemins de preuves/manifests sont relatifs à la racine documentaire ; code_state et sorties CODE sont relatifs à la racine code. Le helper courant offre `--code-root`, mais il valide encore globalement certains `dependency_kinds.CODE.outputs` sous la racine documentaire (fonction `check_state`/`proofs`, ligne 357), puis les revalide sous `code_root` au contrôle du lot (ligne 469). Essai séparé observé : `check-state --root docs/predev --code-root C:\dev\whisper --state docs/predev/state.json --lot L-WHISPER-01` échoue `file missing: Cargo.lock`. Ne pas présenter cette commande comme PASS.

Tant que le contrôleur n’est pas corrigé par son propriétaire, reprendre la méthode historique documentée de vue unifiée temporaire : inclure uniquement les documents référencés et sorties/code nécessaires, comparer chaque copie/hardlink à la source par SHA avant/après, vérifier chemins absolus, absence de collision et absence de mutation. Préserver le dépôt original. Exécuter `check-state` avec la vue vérifiée comme `--root` et `--code-root`, puis conserver manifest de vue, inventaire, hashes, commande, cwd, sortie/code. Cette campagne révisée reste NOT RUN jusqu’à son exécution après revue CLEAN exacte et promotion de P06, puis le mandat applicable. Le PASS historique P02/7 chemins ne vaut pas P06/21.

## Préconditions et inventaire avant L01

Vérifier D19/P06/revues exactes, DETAIL-P01 v3, Q-05, Q-L01-BOUNDS-01 answered avec preuve attribuée, transports INTEGRATION/BOUNDS/COMPAT et checkpoint. Vérifier railguard actif, attestation, équivalence ; autorisation dont les 21 paths égalent exactement 01_LOTS.md. Inventorier `git rev-parse --show-toplevel`, `git rev-parse HEAD`, `git status --short`, `git diff --stat`. Les hashes d’anciennes observations sont historiques : les recalculer à l’exécution.

Comparer les sorties L00 à son ledger; classer les 21 entrées en présentes/hashées ou absentes/futures. Aucun hash fictif pour fichiers futurs et aucun hash de sortie future dans code_state.current. Préserver changements ordinaires. Vérifier les fixtures, modèle hashé/taille, source en lecture seule, dossiers autorisés, ressources et matrice d’environnement. Ouvrir ledger L01 seulement après gate.

Commande de contrôle qui sera utilisée sur vue unifiée vérifiée : `python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-state --root $L01GateRoot --code-root $L01GateRoot --state $L01GateState --lot L-WHISPER-01`. Attendu : `ok=true`, lot exact, D19/P06 digests revus, phase/préconditions applicables. Pas encore exécutée.

## Commandes techniques après code

Sous PowerShell/cwd ci-dessus, proposer séquentiellement : `cargo fmt --all -- --check` ; `cargo test --locked -p whisper-core` ; `cargo clippy --locked -p whisper-core -p whisper-adapters -p whisper-desktop -p whisper-bootstrap --all-targets -- -D warnings` ; `git diff --check`. Pour les frontières, conserver `cargo metadata --locked --format-version 1 --no-deps`, `cargo tree --locked -p whisper-desktop -e features`, `cargo tree --locked -p whisper-bootstrap -e features`, `cargo check --locked -p whisper-core --target x86_64-pc-windows-msvc`, puis inspection des imports et consumers. Attendu : domaines/application isolés, UI via façade, desktop/bootstrap sans Whisper/CUDA. Inspecter en plus `crates/whisper-desktop/src/main.rs`, `root.rs`, `lib.rs` et `ui.rs` : le binaire appelle l’API de lancement, seul `root.rs` assemble les dépendances, `lib.rs` expose le point d’entrée, et l’UI n’importe aucun adaptateur concret. La présence seule des fichiers ne ferme pas cette vérification.

Builds natifs séparés : `cargo build --locked --release -p whisper-worker-cpu --target x86_64-pc-windows-msvc --target-dir target/cpu` ; puis équivalent `whisper-worker-gpu` avec `target/gpu` ; puis `cargo build --locked --release -p whisper-desktop -p whisper-bootstrap --target x86_64-pc-windows-msvc --target-dir target/desktop`. Ne pas qualifier l'isolation CPU/GPU par un build workspace `--all-features`.

Campagnes ciblées proposées, avec leurs fixtures et assertions consignées dans les tests du lot : `cargo test --locked -p whisper-adapters --test import_cpu -- --nocapture` (V-IMPORT) ; `--test durability` (V-DURABLE) ; `cargo test --locked -p whisper-core --test scheduler_contract` (V-SCHEDULER) ; adaptateurs `--test live_archive` (V-LIVE/V-VAD) ; core `--test compute_policy` (V-MODES) ; adaptateurs `--test worker_control` (V-CONTROL) et `--test windows_integration` (V-OS) ; bootstrap `--test install_contract` (V-INSTALL). Chaque harness doit exercer le port/adaptateur concerné, et les tests purs de politique doivent être complétés par le vrai worker. Tous NOT RUN.

**Données** : copies explicitement autorisées des 26 WAV et annotations D17, source hashée avant/après, modèle fixe vérifié par SHA/taille, dossiers de fixture isolés, fichiers absents/corrompus/tronqués, accès refusé et saturation. V-VAD rapporte TP/FP/FN/TN agrégés et par quatre groupes contre DEC-35, limité à ce corpus. Les données nouvelles micro FR/EN/bruit exigent le mandat de L07.

**PE et installation** : dans Developer PowerShell MSVC, `dumpbin /DEPENDENTS` sur les trois EXE CPU/GPU/desktop exacts après build ; CPU/desktop sans import CUDA, GPU avec DLL privées prévues, `nvcuda.dll` du pilote et VC runtime identifiés. Conserver inventaire de payload, modules réellement chargés, hash/signatures/notices. Installer : scénarios 200/206/416, Range/ETag/Content-Range, TLS/redirections, interruption, hash faux, runtime absent/refusé/redémarrage ; `Ready` uniquement après modèle et prérequis validés. PC propre reste différé.

**UI et offline** : lancer `& .\target\desktop\x86_64-pc-windows-msvc\release\whisper-desktop.exe` sur le candidat exact, puis agir dans les vrais consumers fenêtre/tray/réglages/historique avec WAV, micro et dossier autorisés. Observer backend effectif, erreurs, Recoverable, résultats et horodatages capture→rendu. Une réponse shell ou un mock ne vaut pas preuve UI. Pour offline, lancer le payload installé dans un environnement réellement isolé, avec témoin connexion normale/refus isolé, hash modèle/payload et transcript CPU/GPU ; ne pas recycler automatiquement le banc D19 en qualification produit.

## Preuves et récupération

Conserver commande exacte, cwd, shell, cible/features, fixture/hash, environnement, stimulus injecté, branche atteinte, effet observé, stdout/stderr/code, captures et limites. Classer PRODUCT_VALIDATION ou DELIVERY_QUALIFICATION ; D18/D19 restent DESIGN_FEASIBILITY. Sur échec, cesser l'entrée du scénario, préserver pending/journaux, identifier le lot/contrat touché, corriger puis faire revoir le candidat exact et relancer les vérifications ciblées. Ne pas réinitialiser globalement le dépôt, supprimer récursivement des répertoires calculés ni effacer des sources importées. Un Quitter bloqué reste ouvert selon D19 ; la borne du banc ne devient pas une promesse d'arrêt FFI.

## Contrôle des détails et sources P2

Pour chaque `lot.requires_details`, vérifier que la question existe dans `state.json`, que le lot figure dans `impacted_lots`, que `status=answered` possède une preuve attribuée, et que la version/option choisie correspond aux commandes et contrats exécutés. Question absente ou ouverte : préflight du seul lot FAIL. Vérifier également le manifeste prospectif, le mandat documentaire PLANS et les TRANSPORT du checkpoint ; une source fondatrice omise n'est pas dispensée par sa simple lisibilité.

**V-IMPORT-MP3 — PROPOSED / NOT RUN.** PowerShell, cwd `C:\dev\whisper`, cible `x86_64-pc-windows-msvc`, toolchain 1.98.1, versions/features fixées par DETAIL-P01/Q-07 : `cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test import_mp3 -- --nocapture`. Fixtures MP3 existantes manifestées ou générées depuis les WAV autorisés avec protocole/mandat applicable, paramètres et SHA conservés ; variantes invalides/tronquées, source changée après enqueue, annulation/crash. Attendu : vrai décodage et vrai worker CPU jusqu'à TXT/SRT dans le consumer, langue/backend/offsets corrects, source intacte ; aucun faux Complete ni reprise spontanée. Un mock ne suffit pas.


## Environnement avant première modification L01

Matrice attribuée et manifestée : `build-environment-L01.json`, schema `rust-predev-build-env/1`. Méthode et limites : `07_BUILD_ENVIRONMENT_L01.md`. Le contrat `execution-evidence.md` est copié exactement et couvert par le manifest PLANS et le freeze du pack.

Commande PROPOSED / NOT RUN, Developer PowerShell MSVC, cwd `C:\dev\whisper`, cible `x86_64-pc-windows-msvc` :

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.1'
$env:RUSTUP_AUTO_INSTALL = '0'
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-build-environment --root C:\dev\whisper --matrix C:\dev\whisper\docs\predev\candidates\P-WHISPER-08\build-environment-L01.json
rustup target list --installed --toolchain 1.98.1
```

`--root` est la racine du code ; la matrice est référencée par chemin absolu. Ce contrôle est indépendant de `check-state` et de sa limite legacy `CODE.outputs`. Avant toute modification produit, vérifier le hash du helper et du freeze, exécuter dans le processus des commandes Cargo, exiger `ok=true`, `status=PASS` et `READY` pour chaque dépendance obligatoire. Vérifier aussi target Rust, SDK et DLL libclang chargeable. Conserver commande, cwd, cible, hash matrice, résultat JSON, stderr/code retour et identité de l’exécutant. Tout `MISSING`/`NOT CHECKED` bloque L01 avant modification. Aucun résultat n’est produit : NOT RUN.

## Vérification de target avant première modification

Après les probes de la matrice et dans le même Developer PowerShell MSVC, cwd `C:\dev\whisper`, exécuter `rustup target list --installed --toolchain 1.98.1`. Attendu : `x86_64-pc-windows-msvc` présent. Cette commande est PROPOSED / NOT RUN ; `rustup` n’est pas un executable probe autorisé par la matrice. Le helper exécute les probes depuis un cwd temporaire ; définir `RUSTUP_TOOLCHAIN=1.98.1` dans son environnement hérité, ainsi que `RUSTUP_AUTO_INSTALL=0`, pour que les probes et les futures commandes Cargo sélectionnent explicitement la même toolchain sans modifier la toolchain globale ni installer implicitement. Conserver la commande, sortie, code retour, cwd, toolchain et identité de l’exécutant dans les preuves attribuées. Une target absente bloque L01 avant modification produit. Le SDK/linkage et la chargeabilité libclang restent des preuves séparées, non présumées par cette commande.


## Préflight révisé L02 — P07

Le gate L02 doit être refait après la revue exacte de P07 et sa promotion. Le contrôle antérieur sur sept chemins n’autorise pas le périmètre P07 à quinze chemins. Vérifier D19/P07, manifests et revues exactes ; l’autorisation HUMAN-USER couvrant les quinze chemins ; Q-07 answered ; L01 completed et les hashes courants de ses sorties ; railguard actif ; inventaire Git et environnement CPU applicable. Exécuter `check-state --lot L-WHISPER-02` selon la méthode de vue temporaire unifiée décrite plus haut, tant que la validation globale des sorties CODE sous `--root` impose cette vue. Comparer les sources/copies par hash avant et après.

Commandes produit proposées — PowerShell, cwd `C:\dev\whisper`, cible `x86_64-pc-windows-msvc`, toutes PRODUCT_VALIDATION / NOT RUN :

```powershell
cargo test --locked -p whisper-core --target x86_64-pc-windows-msvc --test scheduler_contract -- --nocapture
cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test durability -- --nocapture
cargo test --locked -p whisper-adapters --target x86_64-pc-windows-msvc --test import_mp3 -- --nocapture
```

V-SCHEDULER couvre multi-import, états restaurés, live/import exclusifs, FIFO, choix explicites, aucun départ au scan/redémarrage et ACK après sync. V-DURABLE injecte les coupures autour de sync/publication/retrait, vérifie préfixe confirmé, double scan et absence de faux Complete/source touchée. V-IMPORT-MP3 exerce les profils Q-07 acceptés/refusés, corruption, source changée, annulation/crash et le vrai worker CPU/modèle jusqu’à TXT/SRT avec chronologie et SHA cohérents. V-UI utilise le consumer desktop réel pour Traiter/Retirer, reprise confirmée, FIFO/erreurs/états visibles. Chaque résultat reste lié au candidat réellement exécuté ; un mock seul ne qualifie pas le parcours.

Récupération : arrêter seulement le job/processus ciblé, conserver pending/journal/source, relancer le scan et attendre le choix explicite ; ne pas réinitialiser le dépôt ni supprimer les sources. Les plafonds mémoire documentés restent applicables et inchangés ; GPU reste hors périmètre L02.
