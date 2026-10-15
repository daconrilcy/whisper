Avancée : design et plan revus CLEAN sur `3ef5d24c` ; cinq lots GPU décrits et aucune vérification du produit exécutée.
Blocage : le plan attend l'acceptation de l'utilisateur et l'accord précis de code pour chaque lot.
Décision : PLANS READY pour examen ; les lots ne sont pas encore autorisés à coder.

# Sélecteur GPU — plan proposé (15 octobre 2026)

**État : PLANS READY, acceptation utilisateur en attente.** Le reviewer indépendant a rendu **PLANS CLEAN** sur le contenu exact du commit `3ef5d24ce0cc0567d3959364090a4fba7e844afe`, blob `ce50fb17eff279a59fe3022addfb76c26030332d`, contre le design D08 du même commit ; R-PGS-001 et R-PGS-002 sont fermés. Le présent changement enregistre ces verdicts sans modifier les lots. Auteur Git : session Codex principale. Proposition initiale en lecture seule : `rust_plan_writer`, base `c42ad968cf0886399a66b515bb6792e89f475ec3`, version `PLAN-GPUSEL-v0.1` ; résolution de GS-ARCH-01 proposée en lecture seule par l'architecte `ARCH-GS-GAP-v0.3`. Ce plan n'est ni une acceptation utilisateur ni une autorisation de code, de téléchargement, d'installation, de changement système, de push ou de qualification de la fenêtre par l'agent.

## Périmètre, autorités et points à résoudre

Le livrable vise AC-GS01–08 : montrer les cartes et leur état fiable, choisir une carte en GPU manuel ou Auto, conserver la préférence, attester **la carte qui transcrit effectivement**, préserver le repli Auto borné, les parcours WAV/MP3 et micro, ainsi que TXT/SRT/WAV et la réactivité. Intel reste visible si Windows le voit ; aucun moteur Intel n'est ajouté. Aucun changement de GPU en séance, aucune supervision continue ni reprise après crash d'application. Les cibles `<2 s` interface et `<3 s` capture restent indicatives dans `evo-startup-perf.md` ; seul le premier bloc GPU affiché `<15 s` et le traitement GPU plus rapide que l'audio appartiennent aux critères à vérifier ici.

Le plan suit le design et les règles `AGENTS.md`, `RAILGUARD.md` et le contrat de passage `rust-predev-design/references/handoff-contract.md`. Les tâches techniques ne lèvent aucune règle. Le railguard actif contient des formulations V1 historiques sur MP3 et l'outillage ; comparer chaque clause au produit courant avant le lot affecté, sans réactiver G1 ni changer le railguard par ce plan.

| ID | État, owner, échéance et conséquence |
|---|---|
| GS-ARCH-01 | **STRUCTURING résolu pour le plan, architecte Rust, D08 DESIGN CLEAN.** L'utilisateur a autorisé le 15 octobre 2026 la révision documentaire selon l'option B : `windows = 0.62.2`, façades privées `gpu_inventory/windows_native.rs` et `native_ffi.rs`, adapters `forbid` hérité → `deny` local, worker `deny` conservé, exceptions `unsafe` bornées aux deux modules. API/features, ressources, threads, erreurs et chemins figurent dans D08, revu CLEAN sur `3ef5d24c`. Aucune écriture produit ne découle de cet accord. |
| GS-BUILD-01 | **Détail technique délégué, architecte Rust, avant L02.** Fixer dans Git la valeur explicite des architectures CUDA D07, commune aux flags du build natif et à la déclaration worker, sans réduire silencieusement la couverture promise. Construire ensuite une cible native neuve et vérifier les flags liés. La valeur n'est pas inventée dans ce plan. |
| Q-GS04 | **DETAIL, application/UI/stockage, avant L03.** Fixer le libellé accessible de l'indéterminé et le retour de sauvegarde par révision, sans faux vert/gris ni confirmation prématurée. |
| Q-AN-GS06 | **Constat documentaire confirmé par l'analyste/application, entrée L04.** `ui.rs` retient la fermeture pendant un import occupé ; après admission de Quit, l'UI le demande une fois, attend l'arrêt coopératif/stabilisation, puis ferme quand l'occupation cesse ; `Recoverable` efface cette intention. Stop/Quit admis avant commit annulent sans admission/repli tardif. Si la publication est déjà en commit, `Finalizing` attend son issue et l'arrêt n'est pas annoncé réussi. Conserver la source et les données pending selon le contrat existant. Aucun délai d'abandon nouveau. Produit NOT RUN. |

Un écart de design prouvé suspend seulement les lots touchés. L01 peut être détaillé sur les contrats antérieurs ; L03 peut préparer des essais simulés, mais sa preuve réelle attend L02. D08 a reçu la revue DESIGN CLEAN ; L02/L04 attendent encore l'acceptation du plan et l'accord précis de code pour leurs chemins. Tout changement de contrat, chemin ou dépendance après revue relève de `PLAN_CHANGE`; tout changement d'exigence ou d'architecture relève de `DESIGN_CHANGE`.

## Préflight commun et graphe

Avant chaque lot : comparer `HEAD`, branche, état Git, design et plan versionnés, verdicts sur leurs octets, autorisation écrite de la session pour **ce lot et ses chemins**, décisions nécessaires, stack Rust, dépendances installées, modèles/audio locaux, prérequis documentaires, code et exécution. Préserver tout travail préexistant. Les checks commandés ci-dessous sont proposés et **NOT RUN**. Un code retour nul doit être accompagné des résultats comportementaux attendus. Les fichiers de test, binaires, journaux et données de campagne restent sous les chemins du dépôt déclarés au lot ; les réglages utilisateur ordinaires ne servent pas de fixture.

| Lot | Sortie vérifiable | Prérequis DOCUMENT | Prérequis CODE / EXECUTION |
|---|---|---|---|
| L01 | Catalogue, qualification et résolution purs prouvés sur données contrôlées | Design courant, GS01–04 | Base Git propre ou écarts attribués / toolchain offline |
| L02 | Cartes Windows réellement jointes et qualifiées, CPU utilisable pendant collecte | D08 DESIGN CLEAN ; GS-BUILD-01 | L01 / API et build natif disponibles, accord précis de lot et des chemins natifs |
| L03 | Sélecteur, préférence durable et substitution visible | Q-GS04 fixé ; contrat de stockage partagé | L01, L02 pour démonstration réelle / réglages isolés |
| L04 | Même état GPU sélectionné, attesté et utilisé pour import et micro | D08 DESIGN CLEAN, Q-AN-GS06 ; contrats GS03/05/06/08 | L01–L03 / workers, modèle et audio locaux, accord précis de lot |
| L05 | Validation produit et qualification du paquet candidat | Corpus et matériel de campagne identifiés | L04 / build réel et essais utilisateur de la fenêtre |

Graphe principal `L01 → L02 → L03 → L04 → L05`, sans édition parallèle annoncée : les lots partagent inventaire, IPC, UI et worker. Les lots contiennent respectivement 6, 7, 6, 8 et 5 tâches, sous la limite de dix. L05 n'est pas un permis de publier ; commit de code, push et release restent distincts selon l'autorité applicable.

## L01 — contrats purs, six tâches

**Objectif.** Séparer présence, support, disponibilité et rang Windows, puis résoudre un candidat fixe sans faux positif ni faux premier. **Exclusions :** appels OS/CUDA, UI et persistance. **Owner d'exécution futur :** `rust_implementer` après autorisation du lot. Contrats : INV-GS01–07/09, PORT-GS01/02 (types seulement), CONTRACT-GS03/04 ; AC-GS01/02/04/05/06/07.

1. Étendre l'observation par identité Windows opaque et rang sourcé optionnel ; préserver les homonymes et signaler rang absent.
2. Séparer jointure, support, disponibilité, couverture et causes ; vert/gris exigent les preuves du design.
3. Étendre le catalogue applicatif par génération, une collecte en vol, publication terminale, conservation du partiel et invalidation à la fermeture.
4. Résoudre mode, préférence et catalogue : P compatible prioritaire, sinon premier compatible selon rang Windows démontré, sinon CPU/attente/refus motivé ; ne jamais écrire la préférence en cas de substitution.
5. Distinguer candidat de séance et moteur effectif attesté dans les types et événements purs.
6. Tester homonymes, ordre Windows/moteur inversé, rang absent, résultat tardif/doublon et matrice mode/préférence/catalogue.

**Chemins d'écriture proposés :** `crates/whisper-core/src/gpu_inventory.rs`, `src/gpu_inventory_app.rs`, `src/gpu_selection.rs` (nouveau), `src/lib.rs`, `tests/gpu_inventory.rs`, `tests/gpu_selection.rs` (nouveau), sous `crates/whisper-core/`. Lecture nécessaire : `src/compute_policy.rs`, `src/domain.rs`, `src/ports.rs`, `src/application.rs`, `src/ipc.rs`. Tout appelant à changer doit être ajouté au plan avant exécution. Vérification proposée depuis `C:\dev\whisper` dans PowerShell : `cargo test --offline --locked -p whisper-core --test gpu_inventory --test gpu_selection`. Attendre les assertions C01, pas seulement exit 0.

## L02 — jointure Windows et qualification du build, sept tâches

**Objectif.** Montrer une liste réelle dont chaque état découle du build moteur livré et de la carte logique Windows. **Exclusions :** moteur Intel, inférence multicarte finale. **Owner futur :** `rust_implementer`. Contrats : D03/D05/D07/D08, PORT-GS01/02 ; AC-GS01/02/04/08. **Statut : description complète proposée, revue D08 et accord de code du lot encore requis.**

1. Joindre PnP, rang DXGI et ID logique DisplayConfig/SetupAPI par la façade privée `gpu_inventory/windows_native.rs` de D08 ; ne jamais identifier par nom ou matériel KMT sous-jacent seul.
2. Collecter hors UI avec génération, un résultat terminal et fermeture sans attendre un appel Windows bloqué.
3. Versionner la valeur D07 puis reconstruire le natif en cible neuve ; rapprocher déclaration worker, flags CMake et binaire lié.
4. Qualifier chaque carte sans modèle complet par jointure univoque Windows↔CUDA↔GGML dans le worker du build visé, avec la façade privée `native_ffi.rs` de D08 ; ajouter la commande, la réponse typée et leur corrélation dans `ipc.rs`, puis acheminer la réponse par le transport parent `worker_ipc.rs` jusqu'au catalogue applicatif. Une réponse `CudaAvailable` globale ne vaut pas qualification par carte.
5. Classer support et disponibilité séparément ; une allocation seule, pilote absent ou endpoint manquant ne produit pas un gris/vert indu.
6. Raccorder le consumer desktop aux libellés et couleurs accessibles ; préserver la voie CPU pendant une réponse retenue.
7. Vérifier inventaire réel, partiel, ambigu, absence, erreurs contrôlées et absence de faux « aucun GPU » ; compiler les deux façades sous leurs lints, inspecter chaque bloc `unsafe`, la destruction COM/SetupAPI/CUDA/GGML et le refus si `nvcuda.dll` ou un symbole manque. MultiCUDA reste NOT RUN sans matériel.

**Chemins d'écriture proposés :** `.cargo/config.toml`, `Cargo.lock`, `crates/whisper-adapters/Cargo.toml`, `src/gpu_inventory.rs`, `src/gpu_inventory/windows_native.rs` (nouveau), `src/worker_ipc.rs`, `src/lib.rs`, `tests/gpu_inventory.rs`, `tests/gpu_qualification_transport.rs` (nouveau) sous adapters ; `crates/whisper-core/src/ipc.rs`, `src/gpu_inventory_app.rs` ; `crates/whisper-worker-gpu/Cargo.toml`, `src/main.rs`, `src/native_ffi.rs` (nouveau), `src/gpu_qualification.rs` (nouveau) ; `crates/whisper-desktop/src/root.rs`, `src/ui.rs`, `tests/gpu_inventory_ui.rs`. Lecture indirecte : `native_engine.rs`, build script et features natifs. Vérifications proposées : `cargo test --offline --locked -p whisper-adapters --test gpu_inventory --test gpu_qualification_transport` avec identité par carte, génération et refus de réponse périmée ; `cargo test --offline --locked -p whisper-desktop --test gpu_inventory_ui` ; `cargo build --offline --locked --release -p whisper-worker-gpu --target-dir target/gpu-selection-build` ; contrôler les flags liés, les lints et C02/C03. Ces commandes et le build seul ne prouvent ni la sûreté FFI ni le matériel multicarte.

## L03 — choix et préférence durable, six tâches

**Objectif.** Choix par ID, sauvegarde révisionnée et substitution temporaire annoncée. **Exclusions :** passage à un autre GPU pendant séance. **Owner futur :** `rust_implementer`. Contrat PORT-GS07, INV-GS03/04 ; AC-GS01/02/03/04/05/07.

1. Fixer Q-GS04 : état indéterminé neutre et retour de sauvegarde attribué à une révision.
2. Ajouter un champ GPU optionnel au propriétaire unique des réglages existants, avec migration additive.
3. Accuser la révision réellement écrite et rendre l'échec visible ; ne pas écraser modèle, sortie ou micro par snapshot ancien.
4. Afficher chaque carte par identité, y compris homonymes ; distinguer choix de calcul et préférence de carte.
5. Appliquer P ou Q temporaire seulement à la prochaine résolution, conserver P absente/incompatible et la rétablir quand elle revient.
6. Tester ancien fichier, redémarrage, P→Q→P, panne d'écriture et modifications entremêlées des quatre réglages.

**Chemins d'écriture proposés :** `crates/whisper-core/src/path_settings.rs`, `src/gpu_inventory_app.rs`, `src/gpu_selection.rs` ; `crates/whisper-adapters/src/path_settings.rs`, `tests/gpu_settings.rs` (nouveau), `tests/microphone_settings.rs` ; `crates/whisper-desktop/src/ui.rs`, `src/root.rs`, `tests/gpu_selection_ui.rs` (nouveau). Les essais lisent/écrivent seulement des réglages isolés sous `target/gpu-selection-tests/`, jamais `%LOCALAPPDATA%` ordinaire. Vérifications proposées : `cargo test --offline --locked -p whisper-adapters --test gpu_settings --test microphone_settings` et `cargo test --offline --locked -p whisper-desktop --test gpu_selection_ui` ; observer C04 et le consumer réel. Tout constructeur/test existant à changer hors chemins est un PLAN_CHANGE préalable.

## L04 — séance, attestation du même état et cache, huit tâches

**Objectif.** En GPU manuel et Auto, la carte candidate est celle qui exécute réellement import et micro. **Exclusions :** recherche silencieuse d'un deuxième GPU après échec et changement en cours de séance. **Owner futur :** `rust_implementer`. Contrats D08, PORT-GS05, CONTRACT-GS06/08, INV-GS05–08/10 ; AC-GS03/04/06/07/08. **Statut : revue D08 et accord de code du lot encore requis.**

1. Transmettre séance, candidat, ordinal résolu et configuration par IPC ; rejeter résultats obsolètes.
2. Revalider identité logique, LUID/PCI et ordinal dans **le worker lancé** avant chargement du modèle.
3. Sérialiser tous les appels natifs du worker derrière `native_ffi.rs`, installer le callback versionné une fois et borner sa collecte ; aucun pointeur ou handle ne sort de la façade.
4. Ouvrir une phase exclusive à chaque `create_state`, y compris cache hit ; exiger `using CUDA{ordinal}`, aucun `no GPU found`/échec contradictoire, succès de création, identité LUID/PCI fraîche ; employer **ce même état** pour `full`.
5. Indexer le seul contexte en cache par worker, carte et configuration ; remplacer le worker incompatible et réattester tout nouvel état.
6. Raccorder WAV/MP3 et micro au candidat fixe ; n'afficher l'effectif qu'après attestation corrélée à la séance.
7. Sur échec du candidat : GPU manuel échoue, Auto tente au plus un CPU avant traitement ; jamais un autre GPU arbitraire ni repli après début audio.
8. Tester annulation, fermeture import et micro, résultat tardif, panne worker avec application vivante et conservation des plages WAV admises ; distinguer Stop/Quit admis avant commit, publication déjà engagée (`Finalizing` sans fausse annulation), Quit envoyé une fois par l'UI et `Recoverable` qui efface l'intention de fermeture.

**Chemins d'écriture proposés :** `crates/whisper-core/src/domain.rs`, `src/ports.rs`, `src/application.rs`, `src/ipc.rs`, `src/compute_policy.rs`, `tests/compute_policy.rs`, `tests/import_contract.rs`, `tests/live_contract.rs` ; `crates/whisper-adapters/src/worker_ipc.rs`, `src/supervisor.rs`, `src/direct_live.rs`, `tests/worker_control.rs`, `tests/gpu_selection_worker.rs` (nouveau) ; `crates/whisper-worker-gpu/src/main.rs`, `src/native_engine.rs`, `src/native_ffi.rs`, `src/gpu_qualification.rs`, `src/gpu_attestation.rs` (nouveau), `tests/gpu_selection_native.rs` (nouveau) ; `crates/whisper-desktop/src/ui.rs`, `src/root.rs`, `src/reactivity_log.rs`, `tests/gpu_selection_ui.rs`. Lire `crates/whisper-worker-cpu/src/main.rs` : si l'IPC partagé impose une modification, ajouter son chemin en écriture par PLAN_CHANGE avant ce lot. `native_engine.rs` crée actuellement dans `load/attest` des états jetés puis un autre dans `transcribe` ; tous sont concernés par CONTRACT-GS06. Vérifications proposées : tests core `compute_policy`, `import_contract`, `live_contract` ; adapters `worker_control`, `gpu_selection_worker` ; worker `gpu_selection_native` ; C05–C08 sur modèle/audio locaux, y compris callback manquant/contradictoire et cache. Aucun test n'est compté PASS avant exécution.

## L05 — validation et qualification de livraison, cinq tâches

**Objectif.** Vérifier le candidat réel, ses sorties et le paquet réellement construit. **Owner :** `rust_validator` pour checks assignés, utilisateur pour la fenêtre Windows ; reviewer indépendant pour le verdict de code. Aucun chemin source en écriture par défaut : un correctif retourne au lot concerné et à sa revue.

1. Exécuter format, build, tests offline/locked et contrôler imports/frontières et variantes CPU/GPU.
2. Tester callback manquant/contradictoire, échec après `using`, ancien contexte, cache hit et candidat disparu sur le produit.
3. Tester import et micro CPU/GPU/Auto, arrêt/fermeture, panne worker, intégrité TXT/SRT/WAV et source import intacte.
4. Avec deux CUDA réels : A/B choisis, rang Windows contre ordre moteur, identité homonyme, performances et qualité ; marquer explicitement NOT RUN si matériel absent.
5. Vérifier dans le paquet candidat architectures déclarées/liées, workers et DLL, puis smoke offline de la fenêtre par l'utilisateur.

Commandes proposées depuis `C:\dev\whisper`, PowerShell : `cargo fmt --all -- --check`, `cargo build --offline --locked --workspace`, `cargo test --offline --locked --workspace` ; Clippy strict et builds/package des variantes conformément au railguard et à la configuration effective, à préciser avant L05. Les fixtures, logs et outputs d'exécution restent sous `target/gpu-selection-tests/` ou `target/gpu-selection-build/` ; aucune suppression globale. Publication/push demandent leur décision propre.

## Carte d'exécutabilité des critères — toutes les lignes NOT RUN

Les durées sont des budgets de campagne proposés, jamais des seuils produit. L'implémenteur observe les tests contrôlés, l'utilisateur la fenêtre réelle ; le reviewer indépendant juge le code. Les chemins directs sont ceux des lots ci-dessus ; les chemins indirects incluent `root.rs`, `worker_ipc.rs`, workers, stockage et consumer UI selon chaque ligne.

| Carte, AC / lots | Données et stimulus | Observation attendue, preuve et limite | Budget |
|---|---|---|---|
| C01, GS01/02/04 ; L01 | A utilisable, B exclue, C sans jointure, D désactivée, E panne ; homonymes et rangs inversés dans tests core | A seule sélectionnable ; C/D/E neutres ou indisponibles selon cause, jamais gris par défaut ; premier compatible selon Windows, pas moteur. Contrôlé seulement. | 5 min |
| C02, GS01/02/08 ; L02 | Inventaire réel puis réponse retenue/partielle dans desktop → root/adapters/worker | UI et CPU utilisables, observations fiables conservées, pas d'annonce « aucun compatible » pendant attente. Fenêtre par utilisateur. | 15 min |
| C03, GS01/02 ; L02 | Build D07 neuf, flags natifs, carte couverte/non couverte/inconnue → qualification worker | Vert seulement identité+architecture+vérification individuelle ; incohérence neutre. Autre matériel peut rester NOT RUN. | 30–60 min |
| C04, GS05 ; L03 | Ancien JSON isolé, P→Q temporaire, retour P, write en échec → stockage/UI | P conservée, autres champs intacts, erreur visible et révision confirmée exacte. | 10 min |
| C05, GS03/04 ; L04 | Candidats A/B, ordinal ou PCI discordant → IPC/native/cache | Refus avant audio si discordance ; B effectif corrélé au même état qui transcrit. Simulation ne prouve pas multiCUDA. | 20 min |
| C06, GS03/07 ; L04 | `using` suivi d'échec, message absent/contradictoire, contexte ancien/cache → callback/state/`full` | Aucun `full` sur état non attesté ; phases exclusives, cause visible. Observer l'absence de traitement, pas seulement une erreur. | 20 min |
| C07, GS06/07 ; L04 | Import WAV/MP3 et micro contrôlés : échec GPU avant/après admission, Stop/Quit avant commit, `Prepared`/`Ready`/segment tardifs → import/live/policy/UI | Auto un CPU maximum avant traitement ; zéro autre GPU ; zéro admission, segment ou repli après annulation ; manuel erreur. Source import intacte, pending conservé ; trace de séance et vue. | 20 min |
| C08, GS07/08 ; L04/L05 | WAV à plages repérables, arrêt/fermeture, panne worker avec application vivante ; publication import tenue après entrée en commit, Quit répété, puis `Recoverable` → capture/archive/consumer UI | Plages admises une fois et en ordre, sorties lisibles, source import intacte. Fermeture retenue jusqu'à stabilisation, un Quit admis ; `Finalizing` puis issue réelle sans fausse annulation ; `Recoverable` garde la fenêtre. Crash application exclu. | 20 min |
| C09, GS03/04 ; L05 | Deux CUDA réels A/B, ordre moteur inversé → UI/IPC/native | B choisi calcule sur B ; sans préférence premier compatible Windows attesté. Matériel absent ici : NOT RUN. | 30 min |
| C10, GS08 ; L05 | large-v3-turbo, parole immédiate, audio non silencieux connu → bouton/capture/moteur/UI | Premier bloc GPU **affiché** `<15 s` depuis Démarrer ; GPU plus rapide que durée audio ; qualité comparée au CPU. Jalons distincts, `<2/<3 s` non acquis. | 30 min |
| C11, GS01/03/08 ; L05 | Paquet candidat → workers/DLL/flags/modèle | Paquet contient le build qualifié ; smoke offline et sorties correctes. Méthode de packaging à relever avant campagne. | 30 min |

## Revue, arrêt et récupération

La revue indépendante a rendu **DESIGN CLEAN**, puis **PLANS CLEAN**, sur `3ef5d24c` ; R-PGS-001/002 sont fermés. Elle a conservé le conseil A-PGS-001 (Low) : préciser avant L05 les commandes finales de qualification et la méthode de packaging selon le candidat effectivement construit. Les AC, tâches, chemins, données, observations et limites ci-dessus ont été rapprochés. Deux cycles revue/correction maximum par livrable ; seul le reviewer déclare CLEAN. Le présent changement d'état n'ajoute aucun contrat de code. L'acceptation du plan et l'autorisation écrite de chaque lot restent nécessaires avant toute délégation à l'implémenteur.

En cas d'échec d'un essai futur, préserver sources et sorties, isoler le delta fautif, corriger seulement les chemins autorisés et rejouer les cartes invalidées. Arrêter uniquement les processus de la campagne ; aucun `git reset`, nettoyage Cargo global, suppression récursive ou remplacement de réglages utilisateur. L'essai final dans la fenêtre Windows reste à l'utilisateur.
