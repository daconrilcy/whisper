Avancée : besoin, exigences, domaine et architecture proposés pour les deux workers Turbo.
Blocage : revue indépendante à effectuer ; aucune exécution autorisée.
Décision : soumettre quatre outils communs et préparer le plan après revue DESIGN.

# Whisper — outils communs des workers Turbo

Design **D-WCOMMON-01 v1**, 29 octobre 2026. Statut : CANDIDAT POUR REVUE DESIGN.

## 1. Autorité, sources et contenu

Source produit S1 : demande de créer `whisper-worker-common` pour les outils communs CPU/GPU, notamment Symphonia, MP3 et IPC, avec deux binaires concernés et aucun impact cœur/interface.

Source produit S2 : qualification validée dans cette session, puis instruction exacte : « Je valide cette qualification. Lance le cadrage de whisper-worker-common et prépare le plan d’implémentation, sans modifier le code. »

S2 autorise le cadrage et le plan, leur rédaction/versionnement documentaire selon AGENTS ; elle n'autorise ni code, ni manifeste, ni lockfile, ni campagne Cargo/native, ni push. La session principale est seule auteure des fichiers. Les spécialistes contribuent en lecture seule. Les choix techniques restent des propositions de conception ; la liste fermée et le plan seront soumis avant implémentation.

Base source : branche `evo-startup-perf`, commit `7a4d20b2a115fc6dd57e3b40faa75731b35c1832`. Éléments non suivis préexistants à préserver : `docs/architecture/`, `models/`, `sound example/`. Espace C relevé au début : 98 861 547 520 octets, environ 92,07 Gio ; il faudra le relever à nouveau avant toute campagne.

Sources normatives : AGENTS racine communiqué dans la session et fichier du dépôt ; AGENTS des workers CPU/GPU ; pack courant `C:/Users/cyril/.codex/agent-rules/` et skills `rust-predev-design`, `rust-predev-orchestration`. Aucune réactivation de G1.

Content map : présent document = brief, exigences/UC/AC, domaine, architecture, décisions, risques, acceptation/readiness ; `revue-whisper-worker-common.md` = revue DESIGN ; `plan-whisper-worker-common.md` = plan futur ; `revue-plan-whisper-worker-common.md` = revue PLANS. Aucun railguard supplémentaire : les contraintes produit utiles sont dans ce design et les règles existantes restent actives.

Attribution des contributions, propositions v1 sur la base source ci-dessus : `BRIEF-WCOMMON-01`, owner `rust_product_framer` ; `REQ-WCOMMON-01`, owner `rust_requirements_analyst`. Les contributions brutes sont accessibles dans les retours des agents de cette session ; ce document en intègre le contenu utile, sans revendiquer une copie verbatim. Les contributions domaine/architecture et les résultats des reviewers seront identifiés à leur remise.

## 2. Brief et frontière de version

Objectif : permettre au mainteneur de corriger une seule implémentation des outils partagés par `whisper-worker-cpu` et `whisper-worker-gpu`. Pour l'utilisateur Windows, import WAV/MP3, micro, résultats et erreurs applicables demeurent les mêmes ; aucune adaptation du parent IPC ni de l'interface.

| Capacité | Statut/périmètre | Critère de qualification et propriétaire |
| --- | --- | --- |
| CAP-01 source commune | Inclus, accepté par S2 ; liste exacte à proposer | CA-01, framer |
| CAP-02 décodage WAV/MP3/plages | Conservation acceptée | CA-02, framer |
| CAP-03 MP3 interne du micro | Conservation acceptée, aucune sortie MP3 utilisateur | CA-03, framer |
| CAP-04 stdin/stdout IPC | Compatibilité et refus conservés | CA-04, framer |
| CAP-05 isolation des deux binaires | Contrainte acceptée | CA-05, framer |
| CAP-06 preuve d'équivalence | Incluse, exécution future | CA-06, analyste |
| CAP-07 vérification SHA-256 | Candidat conditionnel à la liste fermée | D-01 de qualification, architecte |
| CAP-08 hooks mémoire existants | Conservation technique, inactifs par défaut | AGENTS et sources, analyste |

Exclus de toute modification : `whisper-core`, `whisper-desktop`, `whisper-adapters`, bootstrap, Parakeet, moteurs/FFI et politiques de backend/attestation/repli. Les types IPC restent dans core. Les changements indispensables aux manifestes/workspace/lockfile devront être explicitement décrits au plan, sans mise à jour de dépendance externe.

Pas de refonte des sessions, nouvelle fonction visible, campagne de performance ou nouvelle garantie de reprise après panne. Installation, tray, réglages et raccourcis ne sont pas concernés. Une extension à un autre worker ou à ces parcours nécessiterait une nouvelle décision produit. Aucun numéro de release ni seuil de performance nouveau.

## 3. Inventaire factuel et correction du relevé initial

Les sources actuelles prévalent sur `docs/architecture/ARCHITECTURE.md` et les fiches CPU/GPU : ces relevés non suivis décrivent encore des modules inline, contrairement aux fichiers actuels.

| Outil | Sources à la base inspectée | Observation |
| --- | --- | --- |
| Décodeur | CPU et GPU `src/decoder.rs:16,58` | Production identique ; tests CPU inline et GPU dans `src/decoder/tests.rs` |
| Encodeur | CPU et GPU `src/encoder.rs:3` | Fichiers identiques ; pas de tests propres déclarés dans ces fichiers |
| Lecture IPC | CPU et GPU `src/ipc.rs:6` | Trames bornées, LF inclus, EOF/incomplet contrôlés avant désérialisation |
| Écriture CPU active | `crates/whisper-worker-cpu/src/ipc_output.rs:25` ; `startup.rs:7`, `diagnostics.rs:1`, `session/mod.rs:10` | Writer déjà borné, sans trace mémoire de sérialisation |
| Écriture CPU historique | CPU `src/ipc.rs:52` | Sérialisation entière avant contrôle ; ne représente pas le writer produit actif |
| Écriture GPU active | GPU `src/ipc.rs:52,72` ; `startup.rs:4`, `diagnostics.rs:1` | Writer borné avec trace mémoire conditionnelle |
| Schéma IPC | `crates/whisper-core/src/ipc.rs:27,149` | Commandes et événements centralisés, hors déplacement |
| Intégrité | CPU/GPU `src/integrity.rs:4` | Vérification SHA-256 commune candidate ; politique d'appel locale |
| Hooks | CPU/GPU `src/decoder.rs:110` et `ipc.rs:38` | Appels conditionnels à `crate::memory_qualification::record` à découpler sans dépendance vers les binaires |

CHANGE-WC-01, EVIDENCE_ONLY : la qualification initiale comparait les helpers `ipc.rs`. L'inspection des appelants corrige la baseline : les deux writers de production sont déjà bornés. Aucun défaut produit ni nouveau comportement IPC n'est déduit du helper CPU historique. La différence d'instrumentation CPU/GPU reste à conserver.

## 4. Exigences, cas d'usage et acceptations

Chaque ligne REQ-WC-N / UC-WC-N / AC-WC-N correspond à CAP-N et, pour 01 à 06, à CA-N de la qualification acceptée. Owner : analyste, proposition v1 opérationnalisant S1/S2. WC-07 reste conditionnelle ; WC-08 précise une contrainte existante.

| IDs | Exigence et cas d'usage | Acceptation observable future |
| --- | --- | --- |
| WC-01 | Le mainteneur corrige une seule implémentation des outils retenus, utilisée par les deux binaires | Revue modules/imports/appels : chaque outil retenu n'a qu'une implémentation de production ; raccordement effectif et builds futurs des deux workers |
| WC-02 | Le worker décode import/plage avec les mêmes valeurs, positions, continuité, gapless et erreurs | WAV existant de 2 000 123 échantillons : 25 fenêtres de 80 000 puis 123 ; plage bornée de 80 000 ; comparer aussi les valeurs PCM non silencieuses avant/après, début/fin/EOF, requêtes invalides, source inaccessible/invalide, callback en erreur et MP3 gapless vérifié |
| WC-03 | Une instance encode les mêmes séquences PCM mono 16 kHz puis finalise | Comparer ordre et octets des paquets avant/après à PCM et découpage identiques ; vide, plusieurs push, finish, finish répété ; après finish, push refuse avec `InvalidRequest: live MP3 encoder is already finalized` |
| WC-04 | Le parent échange commandes/événements sans adaptation | Valide et trames successives ; limite exacte 1 048 576 octets LF inclus acceptée ; dépassement `InvalidData`, EOF vide `None`, incomplet `UnexpectedEof`, JSON invalide `InvalidData` ; erreur de sérialisation avant écriture de sortie, erreurs I/O propagées ; mêmes octets pour writers actifs ; aucun diagnostic libre sur stdout |
| WC-05 | Les sessions import/micro et leurs arrêt/annulation/fermeture demeurent pilotés localement | Diff conforme aux exclusions ; mêmes noms des exécutables, configurations natives, identités, enchaînements et features ; aucun GPU initialisé par le CPU ni repli ajouté |
| WC-06 | Le mainteneur dispose d'une matrice d'équivalence | Chaque test initial a une destination/scénario ; chaque AC indique données, commande réelle, observation et limites ; test conditionnel sans corpus marqué NOT RUN ; compilation, preuve native et fenêtre Windows distinctes |
| WC-07 | Si retenu, vérifier SHA-256 sans déplacer la politique d'appel | Fichier connu accepté avec bonne empreinte ; mismatch et source inaccessible refusés avec erreurs conservées ; mêmes points d'appel |
| WC-08 | Les hooks existants restent explicitement activables et inactifs par défaut | Matrice features ; mêmes stages/champs/distinctions CPU/GPU sur parcours déplacés ; absence d'activation/écriture diagnostic par défaut ; aucun common dépendant d'un worker |

Les tests MP3 gapless sont conditionnés par `WHISPER_L02_GAPLESS_MP3` : CPU `decoder.rs:370`, GPU `decoder/tests.rs:63`. Un retour sans corpus n'est pas une preuve audio. Les fichiers encodeur identiques ne prouvent pas une couverture suffisante : des vérifications dédiées sont nécessaires.

États visibles utilisateur : attente, progression, succès, erreur, annulation et fermeture restent ceux du parent existant ; common ne crée pas de nouvel état visible. Les garanties d'arrêt du callback et de finalisation MP3 restent applicables. Aucune nouvelle migration, file ou reprise durable.

## 5. Questions, décisions et risques fonctionnels

| ID / classification | Options, recommandation et autorité | Échéance/statut |
| --- | --- | --- |
| Q-WC-01 STRUCTURING périmètre | Trois familles seules ou utilitaires réellement communs ; proposer une liste fermée sans sessions/moteurs. Owner architecte/coordinateur ; utilisateur valide avant code | Proposition à présenter avec le design/plan ; ne bloque pas leur préparation |
| Q-WC-02 technique | Préserver writers effectifs bornés et distinctions hooks ; architecte recommande le raccordement. Utilisateur seulement si contrat observable changé | Résolution requise dans le design avant plan |

| Risque | Conséquence et fermeture attendue |
| --- | --- |
| RISK-F-01 | Supprimer une différence de test utile : inventaire test → destination → AC |
| RISK-F-02 | Prendre le helper legacy comme baseline : inventaire des appels actifs et comparaison de leurs sorties |
| RISK-F-03 | Fausse preuve gapless : corpus vérifié ou scénario NOT RUN explicite |
| RISK-F-04 | Perte de hooks ou activation en produit : contrat et matrice features AC-WC-08 |

## 6. Domaine et invariants

Contribution `DOM-WCOMMON-01 v1`, owner `rust_domain_architect`, même base source et entrées brief/exigences v1. Proposition intégrée par la session principale.

| Concept | Owner et responsabilité |
| --- | --- |
| Demande de décodage | Parent fournit identité/plage/borne ; session contrôle identité/corrélation ; outil ne choisit ni moteur ni politique |
| Fenêtre PCM | Outil possède le tampon jusqu'à remise du Vec au callback avec son début sur chronologie 16 kHz |
| Instance MP3 | Session possède instance/ordre des appels ; outil possède état d'encodage et finalisation |
| Trame IPC | Outil transforme valeur/octets ; core conserve schéma/version/codes ; worker interprète |
| Empreinte conditionnelle | Outil vérifie égalité ; session décide quand et comment traduire l'erreur |
| Observation mémoire | Appelant conserve activation/destination/effets ; observation ne confirme aucun traitement métier |

### 6.1 Décodage

Demande invalide (borne nulle ou > 80 000, taux autre que 16 kHz, plage vide/inversée) → refus existant. Source ouverte → lecture/conversion → fenêtres contiguës non vides de taille ≤ 80 000 → queue finale → compteurs de fin. Source/probe/paramètres/décodage conservent leurs erreurs. La conversion mono/interpolation/saturation et i16 ne change pas.

Le callback `Ok` confirme seulement l'acceptation de sa fenêtre, pas sa persistance ni sa transcription. Son `Err` arrête l'appel immédiatement et est propagé ; aucun rollback des callbacks antérieurs. La plage débutant après la fin effective retourne `None`, celle chevauchant la fin retourne les échantillons disponibles. Aucun nouveau délai d'annulation ni préemption ; la session conserve l'annulation et le callback peut interrompre le parcours. Références : CPU `decoder.rs:16,58,133,232,275,298`, code production GPU identique.

### 6.2 Encodeur

Création → ouvert ; `push_pcm` reste ouvert (vide ne finalise pas) ; premier `finish` → finalisé + drainage ; `finish` suivant draine seulement, sans finaliser à nouveau ni rejouer les paquets rendus. Après finalisation `push_pcm` refuse avec l'erreur existante. Les erreurs encodeur/drainage ne créent ni transaction ni reprise. Les paquets rendus restent en mémoire et n'attestent pas d'une écriture durable. Abandon avant finish peut perdre les données retenues ; l'arrêt normal de session demeure responsable de finish. Source : deux `encoder.rs:3-49`.

### 6.3 IPC et confirmations

Lecture : accumulation locale → LF sous limite → retrait LF → désérialisation. EOF vide `None`, EOF non vide `UnexpectedEof`, dépassement/JSON invalide `InvalidData`. Aucune promesse de resynchronisation après erreur.

Écriture active : sérialisation complète bornée → payload → LF → flush. Un refus de sérialisation n'écrit rien à la destination. Une erreur I/O après début de `write_all` peut laisser une trame partielle : aucun rollback du pipe ni atomicité de livraison promis. Succès après flush ne prouve ni réception ni traitement par le parent. Sources : CPU `ipc.rs:6-48`, `ipc_output.rs:29-38` ; GPU `ipc.rs:6-94`.

### 6.4 Invariants et applicabilité

| Invariant | Contrat et traçabilité |
| --- | --- |
| INV-WC-01 | Une implémentation par outil retenu, aucune politique session/moteur/parent déplacée — WC-01/05 |
| INV-WC-02 | Même chronologie/valeurs/bornes PCM, aucun trou/chevauchement/double retrait gapless — WC-02 |
| INV-WC-03 | Finalisation interne unique, refus post-finalisation — WC-03 |
| INV-WC-04 | Limite LF inclus, refus incomplet/excessif avant désérialisation, stdout réservé aux trames — WC-04 |
| INV-WC-05 | Types/version/codes/corrélations restent au même owner — WC-04/05 |
| INV-WC-06 | Si SHA retenu, mêmes contrôles avant/après decode et traduction SourceChanged — WC-07 |
| INV-WC-07 | Stages/champs/distinctions hooks conservés, trace serialize GPU et non CPU actif — WC-08 |
| INV-WC-08 | Observation/compilation ≠ preuve audio/native/durable/Windows — WC-06 |

Persistance : aucune nouvelle donnée durable/migration/réconciliation ; états et tampons volatils, reprise après panne exclue. Les lectures sources et effets de diagnostics existants restent soumis à leurs contrats. Concurrence : appels synchrones, threads/canaux/ordre possédés par workers inchangés ; aucune file, supervision, nouvel état global métier ni nouveau parallélisme.

## 7. Architecture et décisions techniques

Contribution `ARCH-WCOMMON-01 v1.1`, owner `rust_architect`, entrées BRIEF/REQ/DOM v1 sur base source. La correction v1.1 conserve sha2 production pour recette-hooks (qualification.rs:2,189), après vérification ciblée des consommateurs. Choix techniques délégués par S2 ; liste fermée recommandée à accepter avec le plan avant code. Aucun nouvel algorithme produit ni technologie.

| Décision | Choix, alternative et justification | Statut / preuve |
| --- | --- | --- |
| DEC-WC-01 périmètre | Liste fermée : **decoder, encoder, transport IPC, integrity SHA-256** ; exclure diagnostics, logger mémoire, sessions, moteurs et qualification GPU. Alternative trois familles : laisse le helper intégrité identique dupliqué | Recommandation soumise utilisateur avant code ; deux integrity.rs et appelants §3 |
| DEC-WC-02 organisation | Bibliothèque common et petites façades locales gardant noms/signatures ; alternative changement de tous les imports : diff sessions inutilement étendu | Choix de conception ; main/startup/session/diagnostics actuels |
| DEC-WC-03 IPC | Lecture/écriture bornées uniques. Writer CPU actif toujours sans trace ; GPU conserve trace conditionnelle. Helper CPU legacy devient façade bornée instrumentée, sans maintenir son ancien algorithme | Choix de conception ; sources §3 ; aucun changement du writer CPU actif |
| DEC-WC-04 hooks | Callback facultatif par appel, aucun logger/état global ni common→worker | Choix de conception ; signature memory_qualification::record dans les deux workers |
| DEC-WC-05 dépendances | Versions/features inchangées ; aucune dépendance whisper-rs/Windows/CUDA dans common ; forbid unsafe hérité | Choix de conception ; manifestes/toolchain/config courants |

Q-WC-02 est résolue techniquement par DEC-WC-03/04 : bornage actif conservé, différences d'observation conservées. Q-WC-01 est remplacée par la liste fermée DEC-WC-01 recommandée, non par une variante à choisir par le plan writer. Son acceptation produit reste à l'utilisateur avant exécution. Toute nouvelle différence observable retourne au cadrage.

### 7.1 Dépendances et stack

| Consommateur | Dépendances autorisées | Interdits / contrôle futur |
| --- | --- | --- |
| common | core (DecodeRequest), std, serde, serde_json, symphonia, rusty_mp3, sha2 | workers/adapters/desktop/bootstrap/Parakeet, whisper-rs, Windows/CUDA/FFI ; revue manifeste/imports |
| worker-cpu / worker-gpu | common + core + dépendances locales encore utilisées | Aucun moteur, politique ou attestation déplacé dans common |
| core et autres crates | Graphe actuel inchangé | Aucun nouveau lien vers common ni aucune modification de leur manifeste |

Common est une bibliothèque d'outils d'infrastructure au service des workers ; elle ne crée pas un nouveau domaine. Transport générique Serde ; alias Incoming et interprétation du protocole restent locaux. Direction worker→common→core, avec liens directs worker→core conservés. Les autres frontières architecturales existantes ne changent pas ; aucune refonte en couches n'est imposée à cette extraction.

Manifeste cible common : package `whisper-worker-common`, version `0.1.0`, édition/rust-version/publish/lints hérités du workspace ; `default = []`, `l01-memory-qualification = []`. Dépendances workspace : core, serde, serde_json, symphonia, sha2 ; `rusty_mp3 = "=0.8.0"`. Pas de build script ni recette-hooks dans common.

Versions lues le 29 octobre 2026 dans les sources primaires locales : Rust 1.98.1, édition 2024, cible x86_64-pc-windows-msvc (`rust-toolchain.toml`, Cargo racine) ; Serde 1.0.228 derive, serde_json 1.0.145, Symphonia 0.5.5 default-features=false + wav/pcm/mp3, sha2 0.10.9. Whisper-rs 0.16.0 reste CPU sans GPU, GPU avec cuda/raw-api ; Windows 0.62.2, CUDA `61-real`, options MSVC release restent inchangés (`.cargo/config.toml`, manifestes workers). Aucun nouveau besoin natif, d'installation, de redistribution ou de packaging : toujours deux binaires, bibliothèque liée statiquement.

Cargo racine : ajouter common dans members et workspace.dependencies, sans changer default-members. Chaque worker : ajouter common.workspace ; forwarder `l01-memory-qualification = ["whisper-worker-common/l01-memory-qualification"]` ; retirer Symphonia/rusty_mp3 directs après vérification de leurs usages. Conserver Serde/serde_json/core et les autres dépendances encore nécessaires. **Conserver sha2 en dépendance de production dans les deux workers** : utilisé dans qualification.rs sous recette-hooks, ainsi que par des tests. Aucun changement des hooks locaux.

Cargo.lock : future modification explicitement nommée au plan, limitée au package local common et aux arêtes des deux workers ; versions/checksums/packages externes inchangés. Le mandat d'implémentation doit couvrir la mise en cohérence locale hors `--locked` si nécessaire ; tous les contrôles suivants utiliseront `--offline --locked`. Arrêt si dérive externe ; pas de téléchargement.

### 7.2 Modules et API

Créer dans `crates/whisper-worker-common/` : `Cargo.toml`, `src/lib.rs`, `src/decoder.rs`, `src/encoder.rs`, `src/ipc.rs`, `src/integrity.rs`, `src/observation.rs` ; tests séparés `src/{decoder,encoder,ipc,integrity,observation}/tests.rs`. La notation à accolades désigne cinq chemins, pas un mécanisme de génération.

API cible (proposition, non code produit) :

```rust
pub type MemoryObserver = fn(&str, serde_json::Value);
// decoder
pub fn decode_requested_block(request: &DecodeRequest, observer: Option<MemoryObserver>)
    -> Result<Option<Vec<i16>>, String>;
pub fn decode_wav_windows(path: &Path,
    on_window: impl FnMut(u64, Vec<f32>) -> Result<(), String>,
    observer: Option<MemoryObserver>) -> Result<(u64, u64), String>;
// ipc
pub fn read_frame<T: DeserializeOwned>(input: &mut impl BufRead,
    observer: Option<MemoryObserver>) -> io::Result<Option<T>>;
pub fn write_frame<T: Serialize>(output: &mut impl Write, value: &T,
    observer: Option<MemoryObserver>) -> io::Result<()>;
// integrity
pub fn verify_sha256(path: &Path, expected: &[u8; 32]) -> Result<(), String>;
```

Encoder garde type LiveMp3Encoder/new/default/push_pcm/finish, paramètres, ordre et erreurs. Constants decoder/IPC publiques inchangées. Les signatures ne varient pas selon la feature : l'unification de features Cargo ne change pas l'API d'un autre consommateur. Collecte et construction JSON sont conditionnées à la feature common **et** à observer Some ; avec None, aucune collecte/JSON/écriture d'observation. Aucun état global nouveau.

Les deux façades decoder transmettent le recorder local sous feature locale, sinon None. Encoder réexporte le type commun, integrity réexporte la fonction à visibilité locale. Les façades ipc gardent alias Incoming/constants et wrappers ; transmettent recorder sous feature locale sinon None. CPU ipc_output transmet **toujours None**, même si common reçoit une feature activée par ailleurs. Les recorders/process memory/destinations restent dans les binaires. Aucune copie d'algorithme dans une façade.

### 7.3 Portée physique proposée

| Action future | Chemins fermés |
| --- | --- |
| Manifestes/lock | `Cargo.toml`, `Cargo.lock`, `crates/whisper-worker-cpu/Cargo.toml`, `crates/whisper-worker-gpu/Cargo.toml` |
| Production common | Les sept fichiers de package/module listés §7.2 |
| Façades workers | Dans CPU/GPU : `src/decoder.rs`, `src/encoder.rs`, `src/ipc.rs`, `src/integrity.rs` ; en plus CPU `src/ipc_output.rs` |
| Tests common | Les cinq fichiers séparés listés §7.2, selon scénarios associés |
| Tests locaux | CPU `src/ipc_output/tests.rs`, et nouveaux `src/decoder/tests.rs`, `src/ipc/tests.rs` ; GPU `src/decoder/tests.rs`, `src/ipc/tests.rs` ; inventaire avant suppression de redondances |

Appelants indirects contrôlés sans modification prévue : CPU/GPU startup.rs, diagnostics.rs, session/mod.rs, session/decode.rs, session/import.rs, session/live.rs ; GPU gpu_commands.rs et diagnostics/tests.rs. Main conserve ses déclarations ; memory_qualification.rs, qualification.rs, native_engine.rs/native_ffi.rs/attestation ne changent pas. Si un chemin supplémentaire devient nécessaire, exposer le PLAN_CHANGE plutôt qu'élargir automatiquement.

Tests : migrer l'union utile des tests CPU inline decoder/ipc, CPU ipc_output/tests et GPU decoder/tests/ipc/tests ; conserver séparément des tests de raccordement et de hooks dans les façades. Un fichier de tests devenu redondant n'est retiré qu'après table test→destination→AC. Pas d'ajout aux gros blocs inline existants.

## 8. Preuves, risques techniques et exécution future

| Preuve | Classe et état actuel | Suite attendue |
| --- | --- | --- |
| Extraction sans inversion des dépendances | DESIGN_FEASIBILITY, démonstration statique proposée | Modules/appelants/API existants, revue DESIGN ; aucun mécanisme nouveau demandant un SPIKE |
| Stack/features sans nouveau FFI | DESIGN_FEASIBILITY, démonstration statique proposée | Sources primaires locales lues ; revue manifestes et frontières |
| PCM/paquets/IPC identiques | PRODUCT_VALIDATION, NOT RUN | Baseline avant modification puis comparaison ciblée après, sous garde autorisée |
| Hooks/raccordement/features | PRODUCT_VALIDATION, NOT RUN | Common défaut/feature et chaque worker séparément défaut/feature/recette-hooks, sans activer campagnes historiques |
| Builds CPU/GPU | PRODUCT_VALIDATION, NOT RUN | Invocations distinctes ; pas de preuve native déduite du build |
| Native/fenêtre | DELIVERY_QUALIFICATION, NOT RUN | Mandat correspondant ; fenêtre Windows réservée utilisateur, aucune livraison de paquet dans cette mission |

| Risque / owner architecte | Fermeture attendue |
| --- | --- |
| RISK-T-01 perte/couplage hooks | Stages/champs exacts et tests activation/refus ; signatures stables, observation par appel |
| RISK-T-02 unification CUDA | Invocations CPU/GPU distinctes ; ne pas qualifier le CPU en regroupant les deux workers dans une seule commande |
| RISK-T-03 baseline silencieuse/corpus absent | Valeurs PCM non silencieuses et corpus gapless vérifié ; scénarios manquants NOT RUN |
| RISK-T-04 dérive lockfile | Diff local limité ; aucun package/version/checksum externe modifié |
| RISK-T-05 garde d'un autre lot | Autorisation compatible précise ; aucune dérogation ancienne héritée |

Avant toute commande Cargo, baseline comprise : accord d'implémentation/campagne, racines autorisées, ≥30 Gio libres, garde existante compatible, CARGO_HOME/TEMP/TMP canoniques sous racine de lot, cache privé, CARGO_TARGET_DIR canonique `target/parakeet-tests/cargo-shared/`, auto-tests de refus et surveillance exigés. CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only. Aucun Cargo direct ni cache utilisateur en écriture.

L'existence de gardes sous target pour d'autres lots n'établit aucune compatibilité/autorisation ici. Pas de création/adaptation de garde, nouvelle surveillance ou racine de campagne autorisée par cette mission. Ce préalable conditionne l'exécution future, pas la rédaction du plan. Une dérogation ou une adaptation éventuelle devra être concrètement soumise ; jamais inventée par le plan writer. Arrêt et conservation des preuves sur violation/échec selon AGENTS.

## 9. Matrice de couverture et readiness

| Source | Exigence/UC/AC | Domaine | Architecture/risque | Acceptation |
| --- | --- | --- | --- | --- |
| S1/S2 CAP01 CA01 | WC-01 | INV01 | DEC01/02, façades | Imports/appels uniques |
| CAP02 CA02 | WC-02 | INV02, §6.1 | decoder + observation, T01/T03 | PCM/plages/gapless/erreurs |
| CAP03 CA03 | WC-03 | INV03, §6.2 | encoder | Paquets/cycle finish |
| CAP04 CA04 | WC-04 | INV04/05, §6.3 | DEC03/04, T01 | LF/limites/EOF/erreurs I/O |
| CAP05 CA05 | WC-05 | INV01/05 | Matrice deps, T02/T04 | Diff/exclusions/natifs inchangés |
| CAP06 CA06 | WC-06 | INV08 | Matrice preuves, T03/T05 | Inventaire/résultats/NOT RUN |
| CAP07 D01 qualification | WC-07, recommandé avec DEC01 | INV06 | integrity | Digest bon/mauvais/inaccessible |
| CAP08 AGENTS/sources | WC-08 | INV07 | DEC04, T01 | Matrice features/stages distincts |

Rubriques scope, use_cases, errors, domain, dependencies, platform, acceptance et risks : couvertes pour revue. Persistence : conservation des effets existants, aucune migration nouvelle ; concurrency : conservation appels synchrones et ownership workers, aucune orchestration nouvelle ; railguard supplémentaire N/A, les règles existantes suffisent et aucun nouveau mécanisme n'est demandé. Sources et limites explicites dans §§1/3/6/7/8.

État avant revue : candidat documentaire complet ; DESIGN READY ne sera déclaré que sur verdict indépendant et aucun constat requis ouvert. L'acceptation de la liste/design/plan par l'utilisateur, l'accord de coder et les prérequis d'exécution restent distincts. Tous les tests/build/native/fenêtre sont NOT RUN ; aucune preuve produit n'est revendiquée.
