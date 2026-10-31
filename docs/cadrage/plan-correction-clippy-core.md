Avancée : P0 préparé et auto-tests stricts de garde réussis ; aucune correction Rust appliquée.
Blocage : baseline interrompue après un événement Temp hors racine d'origine indéterminée ; P0 non clos.
Décision : campagne arrêtée et preuves conservées ; arbitrage utilisateur requis avant reprise.

# Plan de correction Clippy du cœur

## Header

- Source audit : analyse de cette session du 31 octobre 2026 ; journal `target/parakeet-tests/refacto-ui/logs/Run-20261031T045723893-798bc559.stderr.log` et entrée correspondante de `guard.jsonl`.
- Planning date : 2026-10-31.
- Target workspace : `C:\dev\whisper`, référence `c5bd149fccd0e13dfc63ab25ee2b963d13dab4bc`, branche `evo-startup-perf`.
- Planning mode : plan only, no code changes.
- Confidence : high — dix diagnostics retrouvés dans les sources et le journal T10 antérieur, avec constructions et consommations directes inspectées ; compilation future nécessaire pour fermer les hypothèses de représentation.
- Plan quality check : passed, auto-contrôle documentaire uniquement ; aucune revue indépendante ni qualification technique revendiquée.

## 1. Audit Digest

Objectif produit : retrouver un contrôle strict utile du cœur tout en préservant admission CPU/GPU, file durable, confirmation des doublons et échanges Parakeet. Il s'agit de corriger une dette de validation, pas d'ajouter une fonctionnalité.

La commande UI observée était `clippy --offline --locked -p whisper-desktop --all-targets -- -D warnings`. Elle échoue dans `whisper-core (lib)` avec dix erreurs avant de qualifier desktop. Le contrôle T10 du cœur utilisait des `-W` structurels, sans `-D warnings` : son exit 0 était compatible avec des avertissements. Les dix diagnostics sont déjà présents dans `target/parakeet-tests/refacto-application-core/t10/logs/Run-20261030T192752023-db4a34ba.stderr.log`. Aucun changement de core, du manifeste racine ou de `clippy.toml` depuis `2829ce45` jusqu'à la référence de ce plan.

Les dix constats sont IMPORTANT pour l'objectif de validation ; aucune panne fonctionnelle démontrée. Les sept corrections simples ne changent ni API ni politique. Les trois `large_enum_variant` touchent des types publics : correction coordonnée des constructeurs et consommateurs, en conservant les charges utiles existantes.

### Carte des responsabilités conservée

| Responsabilité | Fichiers actuels et cibles inchangées | Règle |
| --- | --- | --- |
| Domaine/politiques | core : `gpu_inventory_app`, `gpu_selection`, `model_registry`, `model_selection_app`, `parakeet_scheduler`, `parakeet_session` | Aucune dépendance nouvelle vers desktop/adapters, aucune I/O ajoutée |
| Application et ports | core : `application/contracts.rs`, `application/runtime/queue.rs`, `ports.rs` | Décide et émet les effets, garde identités et refus |
| Adaptateurs | adapters : `worker_ipc/runtime/effects.rs`, `parakeet_import.rs` | Exécute les effets et sérialise les requêtes |
| Interface | desktop : `ui/model/view/dialogs.rs` | Construit les commandes, ne redéfinit pas la politique de doublons |
| Protocole et exécution | core : `parakeet_ipc.rs` ; worker Parakeet : `session.rs` | Protocole v1 inchangé ; validation des identités inchangée |
| Tests | `crates/whisper-core/tests/`, `crates/whisper-adapters/tests/`, `crates/whisper-worker-parakeet-cpu/tests/` | Réutiliser les scénarios existants ; nouveau test de compatibilité dans adapters qui dépend déjà de serde_json |

Aucun déplacement de module ou ajout de couche. La façade `application.rs` reste une façade ; aucun `#[path]`, élargissement de visibilité ou réexport nouveau.

## 2. Freshness Check

Lecture ciblée des dix emplacements, manifestes, consignes racine/core, carte d'architecture et fiche core ; celles-ci sont non suivies et décrivent un état plus ancien, donc les consommateurs sont établis par les sources actuelles. Aucun plan Clippy préexistant trouvé dans `docs/cadrage/`. État initial : seuls `docs/architecture/`, `models/` et `sound example/` non suivis, à préserver. Espace observé : 66 931 634 176 octets libres, sans valeur d'autorisation pour une campagne future.

### Analyse des consommateurs des enums

**E1 — `AppCommand::ResolveDuplicateImport`** (`application/contracts.rs:13,21`). Diagnostic : enum au moins 608 octets, variante principale au moins 600, deuxième 224. Proposition : seul `prompt` devient `Box<DuplicateImportPrompt>`. Le prompt métier et le champ de vue restent des valeurs ordinaires.

- Constructeur de production : `crates/whisper-desktop/src/ui/model/view/dialogs.rs`, six boutons (reprise remplacer/créer, dernier/tous/chemin sélectionné/créer). Encapsuler la copie déjà effectuée ; ne pas ajouter une seconde copie.
- Consommation : `crates/whisper-core/src/application/runtime/queue.rs:376`. Comparaison avec `view.duplicate_import_prompt` : comparer les valeurs via `prompt.as_ref()`, puis transférer la même Box à l'effet E2. Garder le refus si prompt périmé, le refus lorsqu'un traitement est actif, `resume_from`, la copie de la requête confirmée et l'ordre des mutations après submit réussi.
- Lectures sans modification attendue : `contracts.rs::identity`, `runtime/commands.rs` et les matches par variante de `runtime/queue.rs` ; l'accès aux champs empruntés reste possible par déréférencement.
- Constructeurs de tests à adapter : core `src/application/runtime/tests/{closing_and_recovery,selection_and_microphone}.rs` ; adapters `tests/{import_cpu,reprocessing_create,reprocessing_cleanup,reprocessing_baseline}.rs` et `src/worker_ipc/tests/{resume_and_admission,unified_import}.rs`. Vérifier les constructions abrégées `{ prompt, choice }`, pas seulement `prompt:`.

**E2 — `ImportEffect::ConfirmDuplicateImport`** (`ports.rs:486,532`). Diagnostic : enum au moins 608 octets, principale au moins 600, deuxième 384 (`StartAutomaticFinalTranscription`). Proposition : seul `prompt` devient `Box<DuplicateImportPrompt>`, comme E1 ; ne pas encapsuler toute la commande ni changer `ImportRequest`.

- Producteur principal : le transfert depuis `runtime/queue.rs` ci-dessus, sans réallouer.
- Consommateur propriétaire : `crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:92` ; extraire `*prompt` pour l'appel existant `confirm_duplicate_import(DuplicateImportPrompt, ...)` dans `runtime/duplicates.rs:214`. La signature de ce helper et ses politiques restent inchangées.
- Lecteur emprunté : `crates/whisper-adapters/src/worker_ipc/service/mod.rs:402`, identité et génération via les champs ; inspecter sans changer si l'autodéréférencement suffit.
- Constructeurs de tests : `crates/whisper-adapters/src/worker_ipc/tests/duplicate_characterization.rs`, deux effets. Préserver le test `duplicate_confirmation_short_refusals_preserve_runtime_identity` et ses assertions.
- E1 et E2 ne portent pas de derive Serialize/Deserialize ; leur changement est une rupture d'API source Rust interne au workspace, pas un changement du format d'archive. Aucun consommateur externe démontré par cette inspection : l'absence de consommateurs hors dépôt reste une hypothèse de périmètre.

**E3 — `parakeet_ipc::Command::Transcribe`** (`parakeet_ipc.rs:56`). Diagnostic : enum au moins 240 octets contre 0 pour Shutdown. Proposition minimale : `identity: Box<WorkerIdentity>` seulement dans cette variante. Les identités de Ready/Result et les structures partagées restent inchangées. Réduction attendue d'environ 88 octets sur la cible Windows 64 bits ; estimation, pas mesure compilée ni engagement ABI.

- Constructeur de production : `crates/whisper-adapters/src/parakeet_import.rs:793` (`Command as WorkerCommand`) ; Box de l'identité possédée existante.
- Récepteur : `crates/whisper-worker-parakeet-cpu/src/session.rs:59` ; lecture des champs de `request_identity` pour les quatre vérifications modèle/runtime/build/profil, a priori sans adaptation grâce à l'autodéréférencement. Vérifier la compilation plutôt que modifier préventivement.
- Tests : `crates/whisper-core/tests/parakeet_ipc.rs` (construction et comparaison de l'identité déstructurée), `crates/whisper-worker-parakeet-cpu/tests/native_import.rs` (trois constructeurs WorkerCommand::Transcribe et fixture de désérialisation existante). Les workers Turbo utilisent un autre protocole : pas de correction attendue.
- Sérialisation : enum Serde tagué `type`, Envelope version 1. L'équivalence JSON attendue du champ Box doit être démontrée dans les deux sens avec une fixture v1 littérale ; un simple aller-retour du nouveau type ne suffit pas. Couvrir expected_gpu absent, null et présent ; Shutdown et la version ne changent pas.

### Limites établies

La recherche des variantes et alias porte sur les sources Rust du dépôt hors `target/`. Elle n'établit pas une liste de consommateurs binaires externes. Aucun Cargo exécuté ici. Le Clippy structurel final UI contient aussi des avertissements dans adapters/desktop et les tests core : résoudre ces dix erreurs ne garantit pas un strict workspace ou core `--all-targets` vert.

## 3. Planning Assumptions

- Safe : aucune politique, donnée persistée, sortie TXT/SRT/WAV, dépendance, feature ou version de protocole à modifier.
- Safe : préférer Box aux exceptions de lint ; une allocation par confirmation de doublon et par commande Transcribe, hors boucle d'échantillons audio. Aucun gain de performance mesuré revendiqué.
- Validation needed : les Box choisies suffisent à faire disparaître les trois diagnostics sans révéler un autre déséquilibre ; Clippy tranche, pas un calcul manuel de tailles.
- Validation needed : tous les constructeurs, y compris les tests ignorés, compilent et les JSON v1 restent identiques sémantiquement.
- Decision gate : accord écrit futur pour le lot A puis le lot B, leurs chemins, adaptation de la garde et campagnes. Le présent plan et son commit n'autorisent pas l'implémentation.
- Safe : migration des anciens tests inline explicitement différée car étrangère à ces dix corrections ; adapter leurs constructions seulement, aucun scénario nouveau ajouté dans les sources de production.

## 4. Finding Traceability

Les chemins ci-dessous sont relatifs à `crates/whisper-core/src/`.

| Audit finding | Severity | Disposition | Task IDs / gate | Notes |
| --- | --- | --- | --- | --- |
| S1 `parakeet_session.rs:6` unused_imports | IMPORTANT | Planned | A1 | SAMPLE_RATE n'a aucune utilisation dans le fichier, tests compris |
| S2 `gpu_inventory_app.rs:186` manual_saturating_arithmetic | IMPORTANT | Planned | A1 | saturating_add(1), maintien de la saturation |
| S3 `gpu_selection.rs:82` collapsible_if | IMPORTANT | Planned | A1 | Chaîne let, ordre de sélection inchangé |
| S4 `model_registry.rs:235` if_same_then_else | IMPORTANT | Planned | A2 | Fusionner seulement les deux branches Parakeet CPU/GPU identiques |
| S5 `model_selection_app.rs:249` unnecessary_lazy_evaluations | IMPORTANT | Planned | A2 | ok_or, erreur ConfigurationUnknown inchangée |
| S6 `model_selection_app.rs:304` unnecessary_lazy_evaluations | IMPORTANT | Planned | A2 | ok_or, erreur LanguageUnavailable inchangée |
| S7 `parakeet_scheduler.rs:261` collapsible_if | IMPORTANT | Planned | A2 | Même effacement conditionnel de preview_ready |
| E1 `application/contracts.rs:13` large_enum_variant | IMPORTANT | Planned | B1 | Box du prompt et consommateurs coordonnés |
| E2 `ports.rs:486` large_enum_variant | IMPORTANT | Planned | B1 | Transfert du même prompt alloué |
| E3 `parakeet_ipc.rs:56` large_enum_variant | IMPORTANT | Planned | B2 | Box identity et preuve JSON v1 |
| Autres avertissements structurels/tests/adapters/desktop | IMPORTANT | Deferred | V1 | Rapport dans ce même document, aucune correction implicite |

## 5. Recommended Sequence

Sept tâches au total : P0, A1, A2, B1, B2, V1 et R1. Le lot A ferme les sept corrections locales avant le lot B. B1 ferme ensemble les deux enums de doublons ; B2 ferme ensuite la commande IPC. Chaque état intermédiaire doit compiler et conserver les tests concernés.

### Phase A : sept corrections locales

- Objective : éliminer S1–S7 sans modifier les contrats publics.
- Closed invariant : aucun des sept diagnostics simples dans la bibliothèque core.
- Before metrics : 7 diagnostics simples + 3 large_enum_variant dans la bibliothèque.
- Target metrics : 0 simple, 3 enums héritées ; aucune aggravation ailleurs.
- Metric commands : V-CL et comparaison des emplacements avec la table §4.
- Stop conditions : nouveau diagnostic, test en échec, modification de politique, état Git concurrent en conflit, préflight/garde non conforme.
- Dependencies : P0 et GO précis du lot A.
- Expected outcome : lot local vérifiable ; ne pas annoncer le strict vert avec les trois enums restantes.
- Tasks : A1 puis A2 ; revue R1 applicable au livrable du lot.

### Phase B : trois enums et consommateurs

- Objective : supprimer E1–E3 en préservant les valeurs métier et le protocole.
- Closed invariant : zéro large_enum_variant sur les trois types ciblés, consommateurs cohérents et trames v1 inchangées.
- Before metrics : 3 diagnostics d'enums après A.
- Target metrics : 0 diagnostic ciblé ; V-CS exit 0 pour core lib.
- Metric commands : V-CL, V-CS, V-WIRE et V-CHECK ; lecture manuelle des transferts/identités.
- Stop conditions : critère A partiel, incompatibilité JSON, besoin de nouvelle dépendance/API ou d'autres champs boxés non prévus, nouvel avertissement bibliothèque, dépassement du budget non justifié.
- Dependencies : A fermé et GO précis du lot B.
- Expected outcome : bibliothèque core strictement propre, portée `--lib` explicitée ; qualification élargie rapportée séparément.
- Tasks : B1 puis B2 puis V1, revue R1.

## 6. Task Backlog

Les identifiants V-* renvoient aux commandes exactes du §7, exécutées uniquement après autorisation et sous garde. Critères communs : respecter le budget, préserver les changements tiers, aucun changement de comportement ni de direction des dépendances. En cas d'échec inattendu, arrêter et conserver les sorties ; pas de neutralisation de lint.

### P0 — Préparer une campagne bornée et relever la baseline

- Priority : High. Source finding : tous. Rationale : attribuer les résultats sans contourner la garde permanente.
- Target files/modules : future racine proposée `target/parakeet-tests/clippy-core/`, garde dérivée de `target/parakeet-tests/refacto-ui/guard.ps1` ; bilan dans le présent document.
- Ownership bucket : validation existante. Scope budget : 0 fichier de production ; garde adaptée et ce seul rapport, aucun nouvel outil autonome.
- Closed invariant : N/A (préflight). Before/Target metrics : campagne non autorisée → environnement borné, auto-tests réussis et baseline conservée. Metric commands : V-P, V-T, V-CL.
- Dependencies : accord écrit précis campagne/adaptation ; lire AGENTS des crates avant toute modification de leur sous-arborescence.
- Implementation outline : relever Git/espace et responsabilités concurrentes ; fixer et vérifier canoniquement CARGO_HOME, TEMP, TMP sous la racine du lot, CARGO_TARGET_DIR sous cargo-shared ; copier le cache nécessaire sans écriture dans le profil ; adapter les racines de la garde existante et supprimer sa tolérance héritée aux événements Temp non attribués, avec les auto-tests correspondants désormais exigeant leur refus. Analyser sa syntaxe et passer SelfTest avant tout Cargo. Fixer CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only. Relever tests core et diagnostics en mode avertissements avant édition.
- Tests to add or update : aucun produit ; conserver les auto-tests de refus de chemins de la garde et leur surveillance, sans ajouter de dispositif Windows.
- Verification commands / provenance : V-P (Repo docs/Source inferred), V-T et V-CL (Source inferred depuis les commandes du plan core).
- Acceptance criteria : au moins 30 Gio libres, aucune jonction/variable hors racine, baseline tests réussie, dix diagnostics retrouvés dans core lib. Pas de baseline volontairement en échec strict.
- Stop conditions : garde invalide, écriture attribuée hors racine, échec inattendu, références différentes inexpliquées ; aucune ancienne dérogation UI/FEAS reprise automatiquement.
- Rollback/fallback : aucune suppression de caches, journaux ou modifications tiers ; arbitrage si garde inutilisable.
- Notes for implementation agent : ce plan ne prépare ni ne lance cette campagne lui-même.

### A1 — Corriger import, saturation et sélection GPU

- Priority : High. Source finding : S1–S3. Rationale : corrections locales sans rupture de contrat.
- Target files/modules : core `src/parakeet_session.rs`, `src/gpu_inventory_app.rs::close`, `src/gpu_selection.rs`.
- Ownership bucket : domaine/politiques. Scope budget : 3 fichiers production, aucun test nouveau prévu.
- Closed invariant : N/A, correction locale. Before/Target metrics : 3 diagnostics → 0 à ces emplacements. Metric commands : V-CL.
- Dependencies : P0. Implementation outline : retirer uniquement SAMPLE_RATE de l'import ; remplacer checked_add(1).unwrap_or(u64::MAX) par saturating_add(1) ; réunir les deux if let sans modifier le prédicat is_compatible, le retour prioritaire ou le parcours suivant.
- Tests to add or update : aucun test miroir de ces réécritures ; réutiliser `tests/gpu_inventory.rs`, `tests/gpu_selection.rs` et les tests core existants. Saturation à MAX préservée par équivalence de l'opération, sans exposer un champ privé pour la tester.
- Verification commands / provenance : V-T, V-CL, V-F, V-D (voir §7).
- Acceptance criteria : mêmes identités, ordre GPU et invalidation de close ; aucun changement de signatures ; zéro nouveau diagnostic.
- Stop conditions : critères communs ; aucun rattrapage d'une dette structurelle rencontrée.
- Rollback/fallback : revenir uniquement sur les trois corrections attribuables si régression ; conserver les preuves.
- Notes for implementation agent : laisser les anciens tests inline en place, ne pas élargir la visibilité.

### A2 — Simplifier catalogue, refus et ordonnanceur

- Priority : High. Source finding : S4–S7. Rationale : fermer les quatre autres diagnostics simples.
- Target files/modules : core `src/model_registry.rs`, `src/model_selection_app.rs`, `src/parakeet_scheduler.rs`.
- Ownership bucket : domaine/politiques. Scope budget : 3 fichiers production, aucun test nouveau prévu.
- Closed invariant : N/A, correction locale. Before/Target metrics : 4 simples restants → 0 ; total bibliothèque attendu : 3 enums. Metric commands : V-CL.
- Dependencies : A1. Implementation outline : garder la branche Whisper Turbo et fusionner les deux branches non-Turbo identiques, sans supprimer la dimension backend de configuration_key ; remplacer seulement les deux ok_or_else constants par ok_or ; réunir le if let et l'égalité preview_ready sans changer la remise à None ni les étapes de terminaison d'appel.
- Tests to add or update : réutiliser `tests/model_catalog.rs`, `tests/model_selection_app.rs` (matrice Turbo 1206 et sélections Parakeet), `tests/gpu_selection.rs`, tests existants parakeet_scheduler. Aucun scénario inline ajouté.
- Verification commands / provenance : V-T, V-CL, V-F, V-D.
- Acceptance criteria : mêmes qualifications CPU/GPU, mêmes causes de refus, mêmes mises à jour de preview ; sept simples fermés, trois enums seuls diagnostics de bibliothèque attendus.
- Stop conditions : critères communs ou perte de distinction de clé CPU/GPU.
- Rollback/fallback : corrections locales seulement, pas de réécriture du catalogue.
- Notes for implementation agent : ne pas fusionner les politiques Turbo/Parakeet parce que certaines valeurs se ressemblent.

### B1 — Réduire ensemble les deux enums de confirmation de doublons

- Priority : High. Source finding : E1/E2. Rationale : un seul transfert de propriété du prompt entre interface, application et adaptateur.
- Target files/modules : core `src/application/contracts.rs`, `src/ports.rs`, `src/application/runtime/queue.rs` ; adapters `src/worker_ipc/runtime/effects.rs` ; desktop `src/ui/model/view/dialogs.rs`. Tests constructeurs listés §2. `service/mod.rs` et `runtime/duplicates.rs` sont à inspecter, sans modification attendue.
- Ownership bucket : application/ports → adaptateur, interface productrice.
- Scope budget : exception atomique justifiée de 5 fichiers production et 9 fichiers de tests existants listés §2 (2 core, 4 intégration adapters, 3 privés adapters). Les deux signatures publiques imposent l'adaptation coordonnée de leurs producteurs et de leur consommateur ; les scinder laisserait des états artificiels sans valeur de validation. Aucun autre fichier de production sans arbitrage.
- Closed invariant : mêmes prompts et choix circulent entre les deux enums à taille réduite, sans copie/allocation supplémentaire au transfert.
- Before metrics : 2 diagnostics E1/E2, tailles journalisées 608/608 ; comparaison de valeurs entre prompt et vue.
- Target metrics : 0 E1/E2 ; même comparaison de valeurs et mêmes refus, une allocation initiale transférée de commande vers effet.
- Metric commands : V-CL, V-CHECK ; inspection manuelle de Box::new/as_ref/déréférencement dans les cinq fichiers.
- Dependencies : A fermé, GO B. Implementation outline : box des deux champs prompt uniquement ; envelopper les constructeurs UI/tests ; comparer le prompt à la vue par référence de valeur dans queue.rs ; déplacer la Box vers l'effet ; déballer avant le helper confirm_duplicate_import existant. Rechercher de nouveau les deux variantes et les constructions abrégées.
- Tests to add or update : adapter les constructions existantes, assertions conservées ; V-T pour core et V-DUP pour les refus courts. Ne pas lancer les tests d'intégration natifs pour une simple vérification de compilation ; V-CHECK compile leurs cibles. Une couverture de prompt périmé doit être repérée dans les tests core ; si absente, ajouter un scénario public dans `crates/whisper-core/tests/import_contract.rs` (budget : un fichier test supplémentaire), avec faux port vérifiant qu'aucun effet n'est soumis.
- Verification commands / provenance : V-T, V-DUP, V-CL, V-CHECK, V-F, V-D.
- Acceptance criteria : pas de changement du prompt de vue/persistance, identité et génération inchangées, six choix UI conservés, refus courts et prompt périmé préservés ; pas de clone supplémentaire au transfert ; seul E3 reste dans core lib.
- Stop conditions : critères communs, besoin de changer un helper métier ou d'élargir une API au-delà des deux champs, couverture critique manquante non refermée.
- Rollback/fallback : retour cohérent des deux signatures et de tous leurs constructeurs, jamais d'état moitié migré ; aucun allow de lint en secours.
- Notes for implementation agent : clones de Box conservent une copie profonde, pas un partage Arc ; ne pas modifier les choix Replace/ReplaceLatest/ReplaceAll/ReplaceSelectedPath/CreateNew ni annulation/reprise.

### B2 — Réduire la commande Parakeet et prouver sa compatibilité v1

- Priority : High. Source finding : E3. Rationale : réduire la représentation Rust sans changer les échanges avec les workers installés.
- Target files/modules : core `src/parakeet_ipc.rs` ; adapters `src/parakeet_import.rs` ; worker `src/session.rs` seulement si nécessaire ; tests core `tests/parakeet_ipc.rs`, worker `tests/native_import.rs`, nouveau adapters `tests/parakeet_command_wire.rs`.
- Ownership bucket : protocole, transport adaptateur et worker.
- Scope budget : au plus 3 fichiers production et 3 fichiers tests ; session.rs peut rester inchangé. Pas de manifeste.
- Closed invariant : commande Transcribe à taille réduite et JSON v1 équivalent dans les deux sens.
- Before metrics : E3 présent, enum au moins 240 octets, aucune preuve de compatibilité après Box.
- Target metrics : E3 absent, V-CS vert, fixtures v1 lues et valeurs sérialisées identiques au JSON attendu ; version/tags/champs inchangés.
- Metric commands : V-CL, V-CS, V-WIRE, V-LEGACY, V-CHECK ; pas d'assertion de taille ABI exacte.
- Dependencies : B1 fermé. Implementation outline : ajouter d'abord le test de compatibilité v1 sur le type existant et le passer ; box identity dans Transcribe uniquement ; adapter producteurs et comparaison de test (`*decoded_identity` ou références de valeurs) ; conserver les validations du worker ; repasser les mêmes fixtures sur le nouveau type.
- Tests to add or update : `parakeet_command_wire.rs` compare un JSON littéral indépendant avec serde_json::Value, puis désérialise la fixture en Envelope<Command> et vérifie chaque identité/request/generation/attempt ; variants expected_gpu absent/null/présent, Shutdown et version 1. Pas seulement un aller-retour du nouveau type. Utiliser serde_json déjà présent dans adapters ; core reste dépendant de serde seul. Adapter aussi les constructeurs des tests natifs ignorés, sans les exécuter.
- Verification commands / provenance : V-WIRE avant/après, V-LEGACY, V-T, V-CS, V-CHECK, V-F, V-D.
- Acceptance criteria : formats JSON compatibles, tags et ordre logique de validation du worker conservés, types Ready/Result/WorkerIdentity non modifiés, aucun bump de protocole, E3 fermé sans nouveau large_enum_variant.
- Stop conditions : JSON différent, allocation supplémentaire ailleurs, besoin de box d'autres champs, changement de protocole ou autre type public non prévu.
- Rollback/fallback : revenir sur ce seul ensemble coordonné ; garder la fixture de caractérisation si son ajout reste utile et accepté. Ne pas modifier le format pour satisfaire le lint.
- Notes for implementation agent : compatibilité de représentation JSON n'est pas une qualification native ni une validation CPU/GPU mesurée.

### V1 — Consolider et attribuer les limites

- Priority : High. Source finding : tous. Rationale : distinguer fermeture des dix erreurs et dette hors périmètre.
- Target files/modules : ce document pour bilan ; sources en lecture seule. Ownership bucket : validation. Scope budget : 0 production, un rapport existant.
- Closed invariant : N/A. Before/Target metrics : dix erreurs historiques → zéro pour core lib strict ; résultats tests/consommateurs attribués au candidat.
- Dependencies : B2. Implementation outline : terminer V-F/V-T/V-CS/V-CHECK et tests de contrats prévus ; comparer baseline/candidat ; distinguer code de sortie, warnings, tests ignorés et non exécutés. Relever espace final et SHA candidat ; pas de manifeste de qualification nouveau.
- Tests to add or update : aucun. Verification commands / provenance : §7, toutes les commandes requises affectées une fois sur candidat final ; répéter uniquement si changement ultérieur pertinent.
- Acceptance criteria : les dix constats sont fermés, core lib strict PASS, cibles consommatrices compilées dans les profils nommés ; bilan honnête des avertissements tests/adapters/UI. Un strict core all-targets ou desktop global ne peut pas être revendiqué à partir de --lib.
- Stop conditions : test/build/strict requis en échec ; ne pas corriger d'autres constats par extension automatique.
- Rollback/fallback : livrer les écarts précis au décideur ; ne pas présenter une suppression de lint comme fermeture.
- Notes for implementation agent : aucun lancement d'interface ni recette native requis pour une preuve JSON ; ces validations restent NOT RUN.

### R1 — Faire relire le candidat et livrer localement

- Priority : High. Source finding : tous. Rationale : vérifier indépendamment le respect des invariants.
- Target files/modules : diff borné du lot A puis du lot B ; ce document pour bilan. Ownership bucket : revue. Scope budget : un rapport existant ; reviewer lecture seule.
- Closed invariant : N/A. Before/Target metrics : candidat vérifié → verdict indépendant et réserves attribuées. Metric commands : lecture diff et V-D.
- Dependencies : contrôles du lot concerné réussis ; A avant GO B, V1 pour B.
- Implementation outline : déléguer l'implémentation à rust_implementer après GO, puis confier le candidat à rust_review_coordinator ; corrections par l'implémenteur et vérification par le même reviewer. Deux cycles revue/correction maximum par livrable, puis LIVRÉ AVEC RÉSERVES et décision utilisateur. Commits locaux limités aux changements attribuables ; aucun push sans accord précis.
- Tests to add or update : selon constats précis, sous les mêmes budgets ; sinon arbitrage. Verification commands / provenance : V-D et contrôles affectés de §7.
- Acceptance criteria : verdict indépendant distinct des tests ; chaque réserve cite exigence, critère ou risque produit concret ; pas de revendication Windows/native.
- Stop conditions : conflit avec des changements tiers, dépassement de deux cycles ou de périmètre ; rendre les réserves sans troisième boucle.
- Rollback/fallback : pas de reset global ; retour sur les seuls changements attribuables si demandé.
- Notes for implementation agent : cette tâche de revue future ne signifie pas que le présent plan est déjà relu ou approuvé.

## 7. Verification Matrix

Toutes les commandes Cargo ci-dessous sont **proposées, NOT RUN**. Depuis `C:\dev\whisper`, définir `$g = '.\target\parakeet-tests\clippy-core\guard.ps1'` après adaptation autorisée. Il s'agit de la future copie de la garde existante, pas d'un outil disponible aujourd'hui. Exécuter chaque commande via `& $g -Mode Run -CargoArguments @(...)`, jamais Cargo directement. V-P est préalable obligatoire ; les variables doivent être définies et vérifiées par cette garde conformément à AGENTS, sans réutiliser les racines d'une ancienne campagne.

| ID / Area | Command or check | Provenance | Expected result | Covers tasks | Notes |
| --- | --- | --- | --- | --- | --- |
| V-P préflight | `git status --short` ; `git rev-parse HEAD` ; `Get-PSDrive C` ; `& $g -Mode SelfTest` après vérification syntaxe/racines/variables décrite P0 | Repo docs | Git connu, ≥30 Gio, auto-tests et garde conformes | P0, tous | La syntaxe SelfTest vient de la garde UI inspectée ; adaptation future indispensable |
| V-D statique | `git diff --check` ; examen de `git diff --stat` et du diff complet | Observed | Aucun défaut d'espace ni fichier hors périmètre | tous | Exécuté sur le document lors de la planification ; à refaire sur candidat |
| V-F format | `& $g -Mode Run -CargoArguments @('fmt','--all','--','--check')` | Repo docs | Exit 0 | A1–V1 | Aucun --fix |
| V-T core | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-core','--all-targets')` | Source inferred | Tests baseline/candidat réussis ; compte actuel relevé, pas un ancien total supposé | P0–V1 | Vérifie aussi les scénarios privés existants, sans les déplacer |
| V-CL diagnostics | `& $g -Mode Run -CargoArguments @('clippy','--offline','--locked','-p','whisper-core','--lib')` | Source inferred | Exit 0 ; 10 → 3 → 1 → 0 diagnostics bibliothèque selon phase | P0–B2 | Ne pas ajouter -W structurels à cette mesure des dix diagnostics usuels |
| V-CS strict final | `& $g -Mode Run -CargoArguments @('clippy','--offline','--locked','-p','whisper-core','--lib','--','-D','warnings')` | Source inferred | Exit 0, zéro avertissement bibliothèque | B2,V1 | Critère de fermeture des dix erreurs ; pas équivalent à --all-targets |
| V-DUP refus courts | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--lib','duplicate_confirmation_short_refusals_preserve_runtime_identity')` | Source inferred | Un test exécuté et réussi, aucun filtrage vide | B1,V1 | Faux runtime existant ; pas de moteur réel |
| V-WIRE JSON | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-adapters','--test','parakeet_command_wire')` | Source inferred | Fixtures indépendantes v1 réussies avant/après Box | B2,V1 | Nouvelle cible test à créer, pas présente à la planification |
| V-LEGACY fixture existante | `& $g -Mode Run -CargoArguments @('test','--offline','--locked','-p','whisper-worker-parakeet-cpu','--test','native_import','additive_gpu_fields_default_when_reading_existing_cpu_ipc_frames','--','--exact')` | Source inferred | Un test JSON non natif réussi | B2,V1 | Ne pas passer --ignored |
| V-CHECK consommateurs | `& $g -Mode Run -CargoArguments @('check','--offline','--locked','--workspace','--all-targets')` | Source inferred | Compilation de toutes les cibles, y compris tests ignorés, réussie | B1,B2,V1 | Compile sans lancer les tests dépendant de corpus/installations |
| V-CHECK personnel desktop | `& $g -Mode Run -CargoArguments @('check','--offline','--locked','-p','whisper-desktop','--all-targets','--features','personal-install')` | Source inferred | Exit 0 | B1,V1 | Défaut déjà couvert par workspace |
| V-CHECK worker CUDA/personnel | `& $g -Mode Run -CargoArguments @('check','--offline','--locked','-p','whisper-worker-parakeet-cpu','--all-targets','--features','parakeet-cuda,parakeet-personal')` | Source inferred | Exit 0 | B2,V1 | CPU défaut couvert par workspace ; aucune inférence |
| Relevé élargi non strict | `& $g -Mode Run -CargoArguments @('clippy','--offline','--locked','-p','whisper-desktop','--all-targets')` | Source inferred | Exit 0 avec dette hors périmètre attribuée | V1 | Révéler les avertissements aval sans promettre leur correction |
| Strict tests/workspace/UI | Non exécuté dans ce plan borné ; autre autorisation si fermeture globale demandée | Repo docs | Réserve explicite, aucun PASS inféré | V1 | Les journaux existants montrent de la dette hors des dix erreurs |
| Native/Windows | Recettes d'import réel, reprocessing natif et fenêtre UI : NOT RUN | Repo docs | Pas de qualification revendiquée | V1 | import_cpu et reprocessing_* ont des chemins/corpus/racines spécifiques ; compilation seulement ici |

Une erreur inattendue d'une commande impose l'arrêt prévu par AGENTS ; le mode avertissements n'autorise pas à continuer après un échec. Aucun --all-features (recette-hooks/qualifications distinctes). Aucun changement du manifeste ou Cargo.lock. Les tests historiques de reprocessing ne sont pas lancés aveuglément : `reprocessing_baseline` est ignoré et exige E0 ; create/cleanup exigent d'autres racines explicites. Le nouveau scénario public de B1 doit rester en mémoire via un faux port core.

## 8. Decision Gates

| Gate | Status | Blocks | Question | Recommended default | How to validate |
| --- | --- | --- | --- | --- | --- |
| G-A | Blocking | P0/A1/A2 | Autorisation précise future du lot A et de sa campagne/garde ? | Autoriser A seul en premier, puis examiner son résultat | Accord écrit courant et plan Git accepté |
| G-B | Blocking | B1/B2 | Autorisation précise des trois champs publics et adaptations consommateurs du lot B ? | Box prompt dans les deux enums, identity dans Transcribe | Accord futur sur le périmètre, A fermé |
| G-JSON | Validation | Fermeture B2 | Les trames v1 sont-elles équivalentes ? | Garder protocole 1, sans changement JSON | V-WIRE avant/après + V-LEGACY ; arrêter si différence |
| G-CONS | Validation | Fermeture B | Inventaire complet sur le candidat et profils observés ? | Refaire la recherche et compiler les consommateurs | V-CHECK et examen des alias/constructions abrégées |
| G-RES | Defaultable | Aucun travail du lot courant | Étendre à tous les avertissements tests/adapters/desktop ? | Différer, consigner la dette ; ce plan corrige les dix erreurs bibliothèque | V1 ne corrige rien hors périmètre ; nouvelle demande si objectif élargi |

G-JSON/G-CONS sont des vérifications d'implémentation, pas des décisions produit soumises à l'utilisateur. Aucun accord technique supplémentaire demandé pour chaque expression locale couverte par un GO précis.

## 9. Out of Scope

- Ne pas faire : ajout de allow/expect, baisse de niveaux de lint, modification des seuils, Cargo --fix, dépendance ou crate nouvelle, modification de Cargo.lock, changement de features ou de CI distante.
- Ne pas faire : déplacer les tests inline existants, refondre la file/archives/worker IPC/UI, modifier d'autres enums, généraliser les Box à des modèles de domaine partagés, introduire Arc/traits ou un nouveau payload public.
- Ne pas faire : changer les formats persistés ou JSON, la version IPC, les identités/hachages, les règles CPU/GPU/Auto, les refus ou l'ordre de publication.
- Ne pas faire : créer un service/ACL/tâche planifiée, écrire hors dépôt, nettoyer des caches ou données tiers, lancer une recette native/UI ou pousser sans accord précis.
- Aucun rapport supplémentaire prévu : un document de plan et son bilan, sept tâches ; respecter l'arrêt/arbitrage au-delà de dix tâches ou dix fichiers de rapport par lot.

## 10. Final Self-Check

| Check | Result | Notes |
| --- | --- | --- |
| Real plan content, not placeholder output | Pass | Deux lots et sept tâches avec clôtures mesurables |
| All template sections present and in order | Pass | Header, sections 1–11 |
| Every audit finding mapped | Pass | S1–S7, E1–E3 ; dette aval explicitement différée |
| Each task has targets, dependencies, outline, tests, verification, acceptance, rollback | Pass | §6, références exactes §7 |
| Structural phases/tasks define invariant, metrics and stop conditions | Pass | B/B1/B2 ; corrections locales déclarées non structurelles |
| Scope budgets explicit and justified | Pass | B1 exception atomique 5 production ; autres tâches ≤3 |
| Dedicated test paths or justified existing private tests | Pass | Nouveau contrat JSON dans adapters/tests ; anciens tests seulement adaptés |
| Verification provenance and limits | Pass | Observed/Repo docs/Source inferred, aucun Cargo exécuté ici |
| Gates identify blocked work and defaults | Pass | GO A/B distincts, hypothèses techniques vérifiables |
| Paths real or explicitly proposed | Pass | Nouvelle racine et nouvelle cible test signalées comme futures |
| Narrow implementation scope | Pass | Dix diagnostics bibliothèque ; strict global non promis |

Contrôles de planification réellement exécutés : lectures ciblées et recherches des consommateurs/alias, état Git, comparaison `2829ce45..HEAD` sur core/configuration et `git diff --check` du document. Aucun changement de code, test, garde, cache, configuration ; aucun Cargo ni recette exécuté. Le commit de ce plan reste documentaire et n'ouvre aucun GO.

Exécution autorisée ensuite dans cette même session par l'utilisateur : « ok implemente le plan ». Cet accord porte sur les deux lots dans l'ordre prévu, leurs vérifications et adaptations de garde, revues et commits locaux ; aucun push ni recette native/UI. Avant toute écriture de campagne, l'implémenteur a relevé que la garde UI contient une tolérance Temp non attribué et des auto-tests qui l'exigent. La rédaction P0 « adapter seulement les racines » est donc corrigée ci-dessus pour satisfaire l'exclusion explicite de cette dérogation ; aucun assouplissement, nouvelle racine ou dispositif de surveillance n'est ajouté.

### Arrêt de P0 — 31 octobre 2026

Implémenteur délégué `/root/implementation_clippy`, baseline Git `979dbe5834b518d643337034e6b6e2b39a030c1f`. Garde adaptée sous `target/parakeet-tests/clippy-core/`, syntaxe PowerShell validée, cache copié dans cette racine. V-P/SelfTest strict : PASS, exit 0, notamment refus des quatre types d'événements Temp non attribués simulés. Ces simulations d'auto-test ne constituent pas des écritures externes réelles.

Première commande V-T sous garde : `test --offline --locked -p whisper-core --all-targets`, run `20261031T052121951-c14e409d`, exit 1 après six secondes. La garde a interrompu l'arbre de processus pendant la compilation de dépendances après un événement réel `Deleted` à `2026-10-31T05:21:27.2393182+01:00`, détecté à `05:21:27.2908140+01:00`, sur `C:\Users\cyril\AppData\Local\Temp\65a12127-6417-4175-9888-1b63925b5b99.tmp`. FileSystemWatcher ne fournit pas de PID : origine, attribution à Cargo et contenu du fichier indéterminés. Cet événement n'est donc pas présenté comme une fuite Cargo démontrée, mais il déclenche bien l'arrêt strict prévu.

Preuves conservées sous `target/parakeet-tests/clippy-core/logs/` : `guard-events.jsonl`, `guard.jsonl`, `Run-20261031T052121951-c14e409d.stderr.log` et `.stdout.log`. Aucun résultat de tests baseline PASS ; V-CL non lancé, A1/A2/B1/B2 non commencés. Aucun fichier Rust, manifeste, lock ou configuration suivie modifié. Aucune revue indépendante d'implémentation demandée faute de candidat ; aucune recette native/UI ni push. Aucun nettoyage ou restauration après l'arrêt. Les trois répertoires non suivis préexistants sont conservés. Espace final relevé par l'implémenteur : 66 043 084 800 octets.

Arbitrage produit/périmètre à soumettre avant toute reprise : soit maintenir l'arrêt avec la règle stricte actuelle, soit autoriser pour cette seule campagne `clippy-core` une tolérance bornée des événements purement locaux sous `%LOCALAPPDATA%\Temp` sans attribution démontrée, consignés UNATTRIBUTED sans qualification de leur origine. Dans cette seconde option, les écritures applicatives volontaires restent limitées aux deux racines prévues ; une écriture externe attribuée à la campagne, un événement sensible, un défaut de garde ou un échec test/build imposent toujours l'arrêt. Recommandation : cette tolérance limitée pour pouvoir poursuivre les corrections, sous réserve d'accord explicite de l'utilisateur sur le risque résiduel ; aucune reprise ni modification de garde permissive effectuée à ce stade.

## 11. Implementation Agent Prompt

À utiliser seulement après GO précis, en complétant le lot effectivement autorisé dans le message de mission :

> Lire `docs/cadrage/plan-correction-clippy-core.md`, ses références et les AGENTS applicables. Mode IMPLEMENT, sous responsabilité rust_implementer. Commencer par P0 puis A1/A2 si le GO autorise le lot A ; ne pas commencer B avant sa propre autorisation et la fermeture de A. Préserver tous les changements tiers, y compris docs/architecture, models et sound example. Respecter les budgets et l'exception atomique B1 ; aucun élargissement d'API au-delà des trois champs prévus. Appliquer les commandes §7 sous la garde adaptée et autorisée uniquement ; ne pas utiliser Cargo directement ni --fix. Sept diagnostics simples puis trois enums, aucun allow et aucun changement métier/JSON. Arrêter aux conditions du plan ; rapporter diff, résultats réellement exécutés, réserves et espace. La session principale demandera ensuite la revue à rust_review_coordinator, au plus deux cycles. Aucun push, recette native ou fenêtre Windows autorisé par ce prompt générique.
