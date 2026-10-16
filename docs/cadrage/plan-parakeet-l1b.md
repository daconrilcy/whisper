Avancée : L1a est livré CLEAN ; la revue utilisateur borne L1b aux modèles déjà présents, à la matrice Turbo relevée dans le code et aux ajouts non régressifs de L1a.
Blocage : DRAFT v3 attend la revue indépendante et l'accord écrit précis de chaque sous-lot ; Parakeet produit reste non qualifié.
Décision : `PLAN-PARA-L1b DRAFT v3` fixe chemins, gates et cartes ; aucun code L1b n'est autorisé par ce document.

# Plan Parakeet L1b v3 — raccordement du catalogue à l'application

**Base Git :** L1a `6ccb1dfe` sur `evo-startup-perf` ; design `docs/cadrage/evo-parakeet.md` et `docs/cadrage/evo-engine-capabilities.md`, révisés selon `DEC-PARA-24–28` et `DEC-CAP-07`. Ce document remplace DRAFT v2 au commit `e948e7af` selon la revue utilisateur du 16/10/2026. Contributions `rust_plan_writer` et `rust_architect` en lecture seule ; la session principale rédige et versionne. **Périmètre :** L1b-1 puis L1b-2, sans backend Parakeet, téléchargement/acquisition, paquet final ou changement du micro.

## 1. Résultat, limites et gates

L1b branche les règles pures de L1a dans l'application Windows. Le catalogue fermé montre Turbo et Parakeet ; un modèle absent, corrompu ou non admissible reste visible avec cause, grisé et non lançable. La langue manuelle devient une liste et Auto langue suit les capacités admissibles. L'application refuse la même demande inadmissible lorsqu'elle contourne le widget, notamment à l'ajout, au traitement et à la reprise d'un import, et aux deux voies live. Le choix du modèle, le choix Auto calcul et les paramètres v1 sont restaurés sans substitution silencieuse. **Toutes les options Turbo aujourd'hui exposées** (modèle, CPU/GPU, Auto calcul, import, live et langues utilisables) ont le statut distinct `Existant reconnu` et la provenance « comportement de production actuel, décision utilisateur du 16/10/2026 ». Cette décision de périmètre n'est pas une qualification mesurée. Aucune de ces options ne devient grisée ou refusée par L1b ; une capacité Turbo nouvelle exige une preuve.

**Parakeet reste grisé dans L1b.** Les inférences FR/EN du banc isolé ne qualifient pas le runtime, les workers, les profils et le paquet du produit. Le manuel GGUF/NeMo v0.2.0 est `Unsupported` ; Auto langue du candidat produit exige `AC-PARA-16F` avant activation. Un test peut injecter une fiche synthétique qualifiée pour éprouver la règle, jamais l'embarquer comme qualification réelle.

| Gate | Entrée nécessaire avant implémentation ou conclusion | Si absente |
| --- | --- | --- |
| G-DESIGN | Design révisé selon `DEC-CAP-07` et `DEC-PARA-24–28`, L1a `6ccb1dfe`, puis revue indépendante du plan v3 exact. La règle v3 de modèles Test précise la portée L1b de `DEC-PARA-25` : acquisition réservée à L5 et RG-08. | `PLAN_CHANGE_REQUIRED` ou `DESIGN_CHANGE_REQUIRED`. |
| G-TURBO | La **matrice exhaustive de §1 bis**, relevée dans le code à `6ccb1dfe`, reçoit `Existant reconnu` avec provenance exacte ci-dessus ; `Qualified` reste distinct. Toute capacité nouvelle exige une preuve. | Préserver chaque entrée actuelle et réviser le plan si cette matrice ne peut être tenue. |
| G-TEST-MODELS | Profil Test fermé sous `C:\dev\whisper\models\` hors Git, **en lecture seule pour L1b-1 et L1b-2**. Les deux fichiers exacts existent au contrôle du 16/10/2026. Présence, taille et SHA revérifiés au préflight et pour chaque recette réelle. | Si absent ou SHA différent : recette réelle correspondante `NOT RUN`, sans téléchargement, acquisition ni écriture sous `models/`. Poursuivre les tâches indépendantes ; acquisition réservée à L5 (RG-08). |
| G-EXECUTION-1 | Plan revu et accepté, accord écrit précis pour L1b-1 et ses chemins, préflight complet. | Aucune écriture de code L1b-1. |
| G-EXECUTION-2 | Verdict **CLEAN** de L1b-1, puis accord écrit précis pour L1b-2 et ses chemins, préflight propre. | Ne pas commencer L1b-2. |

Au contrôle du 16/10/2026, Turbo mesure `1 624 555 275` octets, SHA-256 `1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69` ; Parakeet `713 975 456` octets, SHA-256 `E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E`. Ce constat ne vaut pas recette d'inférence. Une absence ou une divergence au préflight laisse la recette réelle du modèle concerné `NOT RUN`, sans téléchargement ni écriture sous `models/`. Les tâches de contrat indépendantes continuent. Les chiffres RAM/VRAM/disque restent `Unknown` sans mesure comparable, pas `Insufficient` inventé.

## 1 bis. Matrice Turbo de référence à `6ccb1dfe`

Cette matrice est **l'inventaire du comportement exposé par le code**, pas une mesure de qualité de transcription. Son unique modèle prédéfini est `ggml-large-v3-turbo.bin` : `crates/whisper-desktop/src/ui.rs:17–18` fixe son SHA-256 et `:1292–1295` son chemin par défaut ; `:337–338` et `:370–371` transmettent ce chemin et le SHA pour import/live. Le champ de chemin est aujourd'hui éditable ; cela n'établit **aucun autre modèle** admissible dans le catalogue fermé.

| Dimension | Entrées exhaustives aujourd'hui exposées | Source au commit `6ccb1dfe` |
| --- | --- | --- |
| Modèle prédéfini | `ggml-large-v3-turbo.bin` (une identité Turbo) | `crates/whisper-desktop/src/ui.rs:17–18, 64, 1292–1295` ; `crates/whisper-core/src/model_catalog.rs:4–11` |
| Calcul | `Cpu`, `Gpu`, `Auto` ; Auto calcul est distinct d'Auto langue | `crates/whisper-core/src/domain.rs:59–65` ; `crates/whisper-desktop/src/ui.rs:550–560` |
| Parcours | import WAV/MP3 via `EnqueueImport` ; live micro via `StartDirectLiveConfigured` | `crates/whisper-desktop/src/ui.rs:317–344, 350–395` |
| Langue | Auto langue (`LanguageChoice::Automatic`) et les **100 identités linguistiques** ci-dessous en manuel. Pour chaque identité, le code court **et** le nom anglais exact sont saisissables, validés et transmis ; soit 200 graphies manuelles, sans autre normalisation que `trim()` dans l'UI. | `crates/whisper-desktop/src/ui.rs:323–326, 381–384, 544–546, 664, 675` ; `crates/whisper-core/src/domain.rs:53–57` ; workers CPU/GPU `native_engine.rs:64–67, 118` / `:77–80, 95` |

**Source exhaustive des graphies :** `Cargo.lock` épingle `whisper-rs 0.16.0` et `whisper-rs-sys 0.15.0`. Les workers appellent `whisper_rs::get_lang_id` avant `FullParams::set_language` ; `whisper-rs/src/standalone.rs:18–26` délègue à `whisper_lang_id`. La table `g_lang` de la source native épinglée `whisper-rs-sys-0.15.0/whisper.cpp/src/whisper.cpp:280–380` contient exactement les paires suivantes ; `:3985–3996` accepte soit la clé code, soit le nom anglais **à l'identique**, et rejette les autres chaînes. Les noms anglais sont donc des alias d'entrée historiques de la même identité, pas 100 langues supplémentaires. Le champ manuel libre est remplacé en L1b-2 par 100 options de liste ; le parcours des anciennes valeurs et requêtes manuelles doit rester compatible avec **les deux graphies**. Casse non modifiée, aucun nom français ni alias non listé inféré.

| Identifiants | Code = nom anglais exact (10 entrées par ligne) |
| --- | --- |
| 0–9 | `en=english`, `zh=chinese`, `de=german`, `es=spanish`, `ru=russian`, `ko=korean`, `fr=french`, `ja=japanese`, `pt=portuguese`, `tr=turkish` |
| 10–19 | `pl=polish`, `ca=catalan`, `nl=dutch`, `ar=arabic`, `sv=swedish`, `it=italian`, `id=indonesian`, `hi=hindi`, `fi=finnish`, `vi=vietnamese` |
| 20–29 | `he=hebrew`, `uk=ukrainian`, `el=greek`, `ms=malay`, `cs=czech`, `ro=romanian`, `da=danish`, `hu=hungarian`, `ta=tamil`, `no=norwegian` |
| 30–39 | `th=thai`, `ur=urdu`, `hr=croatian`, `bg=bulgarian`, `lt=lithuanian`, `la=latin`, `mi=maori`, `ml=malayalam`, `cy=welsh`, `sk=slovak` |
| 40–49 | `te=telugu`, `fa=persian`, `lv=latvian`, `bn=bengali`, `sr=serbian`, `az=azerbaijani`, `sl=slovenian`, `kn=kannada`, `et=estonian`, `mk=macedonian` |
| 50–59 | `br=breton`, `eu=basque`, `is=icelandic`, `hy=armenian`, `ne=nepali`, `mn=mongolian`, `bs=bosnian`, `kk=kazakh`, `sq=albanian`, `sw=swahili` |
| 60–69 | `gl=galician`, `mr=marathi`, `pa=punjabi`, `si=sinhala`, `km=khmer`, `sn=shona`, `yo=yoruba`, `so=somali`, `af=afrikaans`, `oc=occitan` |
| 70–79 | `ka=georgian`, `be=belarusian`, `tg=tajik`, `sd=sindhi`, `gu=gujarati`, `am=amharic`, `yi=yiddish`, `lo=lao`, `uz=uzbek`, `fo=faroese` |
| 80–89 | `ht=haitian creole`, `ps=pashto`, `tk=turkmen`, `nn=nynorsk`, `mt=maltese`, `sa=sanskrit`, `lb=luxembourgish`, `my=myanmar`, `bo=tibetan`, `tl=tagalog` |
| 90–99 | `mg=malagasy`, `as=assamese`, `tt=tatar`, `haw=hawaiian`, `ln=lingala`, `ha=hausa`, `ba=bashkir`, `jw=javanese`, `su=sundanese`, `yue=cantonese` |

**Couverture B-A8 :** test paramétré sur l'identité Turbo, les 3 calculs × les 2 parcours × (Auto langue + les 100 codes + les 100 noms anglais), soit **1 206 combinaisons** d'admission/commande sans inférence native, avec compte de chaque entrée et absence de nouveau grisement/refus. Le widget liste les 100 identités et Auto ; choisir une identité transmet un code accepté. Tester aussi la restauration et l'admission des noms anglais historiques, avec comparaison à leur code. Les états GPU réellement indisponible ou modèle absent conservent leurs refus habituels : ce critère interdit les **nouveaux refus dus à L1b**.

## 2. Contrats et tâches par sous-lot

**Architecture retenue pour le plan :** un module applicatif `model_selection_app.rs` possède la projection et l'admission ; `model_registry.rs` porte les fiches attribuées. Un port de catalogue et l'adaptateur `model_catalog.rs` constatent les artefacts et l'hôte hors du thread UI. La composition injecte leurs résultats et ne reconstruit aucune liste de langues ou de modèles admissibles dans `ui.rs`. Le protocole Turbo `ImportRequest`/`LiveRequest` et les workers restent inchangés : après admission, l'application construit la requête avec le chemin du catalogue, sans accepter un chemin fourni par une commande UI. Le contrôle d'empreinte est refait immédiatement avant délégation ; il ne constitue pas à lui seul une attestation de l'artefact finalement ouvert par le worker, qui reste un contrôle des lots moteurs.

| Tâche | Travail et preuve attendue | Traçabilité |
| --- | --- | --- |
| B1 / L1b-1 | Déclarer les deux modèles et fiches exactes ; `Existant reconnu` Turbo distinct de `Qualified`, sans extension à une capacité nouvelle ; Parakeet produit non qualifié. Adapter `engine_capabilities.rs` et ses tests **par ajouts uniquement**, sans supprimer ni affaiblir une assertion L1a existante. | DEC-CAP-07, DEC-PARA-24, B-A1/8/9. |
| B2 / L1b-1 | Ajouter observation locale injectée : fichier présent, taille/empreinte, runtime et ressource disponible/inconnue, erreurs, génération ; aucune recherche libre ni IO sur le thread UI. | PORT-CAT/HOST, CAP-32, AC-PARA-05/07A. |
| B3 / L1b-1 | Raccorder la projection L1a à la sélection : préférence distincte de sélection exécutable, refus typé, résultat tardif écarté. | INV-PARA-01/15, CAP-32/33. |
| B4 / L1b-1 | Garde d'admission avant start/enqueue/process/reprise import et start live, y compris commandes directes ; contrôle de l'artefact avant effet. Chemin Turbo historique **explicitement connu** dans file/checkpoint rattaché en mémoire à Turbo, requête exécutable avec chemin du catalogue vérifié ; chemin inconnu refusé avec message. Préserver donnée durable et contrôles source/SHA/job/génération/FIFO/préfixe confirmé. | AC-PARA-05/08C/16A–E, B-A2/3/10. |
| B5 / L1b-1 | Migrer v1→v2 : avant première écriture v2, copier les octets v1 dans `settings.json.v1.bak`, création exclusive, jamais écrasée ; conflit/échec bloque v2 et préserve v1. Résoudre Turbo historique connu, inconnu gardé en diagnostic ; conserver audio, destination, micro, GPU, modèle préféré et Auto/FollowProbe avec ACK révisionnés. | PORT-PREF, AC-PARA-08/08B, B-A4/5. |
| B6 / L1b-2 | Composer le profil Test depuis la racine du dépôt, injecter catalogue/préférences, garantir qu'Auto sauvegardé précède la sonde tardive et qu'un choix plus récent n'est pas écrasé. | DEC-PARA-06/09, PORT-CAT/PREF. |
| B7 / L1b-2 | Remplacer le chemin « Modèle local » et la saisie libre de langue par des listes consommant la projection ; griser avec cause, conserver audio et destination et toute la matrice Turbo actuelle. | DEC-PARA-02/10/16/24, CAP-33, B-A7/8. |
| B8-1 / L1b-1 | Tests core/adapters des états, admission, migration/sauvegarde, files/checkpoints et **1 206 combinaisons** de la matrice Turbo de §1 bis ; fixture synthétique sans qualification de production. Les assertions L1a dans `tests/engine_capabilities.rs` sont conservées intégralement, avec nouveaux cas ajoutés. | B-A1–6/8/9/10. |
| B9-1 / L1b-1 | Vérifier diff et commandes puis soumettre à `rust_review_coordinator` (deux cycles maximum). | B-A9, G-EXECUTION-2. |
| B8-2 / L1b-2 | Tests desktop de composition et widgets egui avec snapshot injecté, sans inférence native. | B-A5/7/8. |
| B9-2 / L1b-2 | Vérifier diff et commandes puis revue indépendante selon la même limite. | B-A9. |

Ordre : B1 précède B2/B3 ; B2+B3 précèdent B4 ; B3 précède B5. B8-1/B9-1 closent L1b-1. **CLEAN de L1b-1** précède toute écriture L1b-2 ; B6 précède B7 puis B8-2/B9-2. Chaque sous-lot reste sous dix tâches. L1b-1 conserve les interfaces publiques utilisées par le desktop actuel (`StoredPaths`, `PathDefaults`, `ImportApplication::new`, API des réglages) et ajoute des interfaces compatibles : le desktop inchangé doit compiler et garder son comportement jusqu'à L1b-2.

## 3. Chemins candidats par sous-lot à figer par la revue

**L1b-1 : `lot.paths.write`, 19 chemins core/adapters exclusivement :**

```text
crates/whisper-core/src/lib.rs
crates/whisper-core/src/application.rs
crates/whisper-core/src/model_selection_app.rs
crates/whisper-core/src/model_registry.rs
crates/whisper-core/src/engine_capabilities.rs
crates/whisper-core/src/path_settings.rs
crates/whisper-core/tests/engine_capabilities.rs
crates/whisper-core/tests/model_selection_app.rs
crates/whisper-core/tests/model_preferences.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/model_catalog.rs
crates/whisper-adapters/src/path_settings.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/queue_store.rs
crates/whisper-adapters/tests/model_catalog.rs
crates/whisper-adapters/tests/model_preferences.rs
crates/whisper-adapters/tests/microphone_settings.rs
crates/whisper-adapters/tests/legacy_model_requests.rs
```

**L1b-2 : `lot.paths.write`, 5 chemins desktop exclusivement, après CLEAN de L1b-1 :**

```text
crates/whisper-desktop/src/lib.rs
crates/whisper-desktop/src/main.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-desktop/tests/model_selection_ui.rs
```

Les nouveaux modules/tests sont à créer si absents ; chaque chemin est recontrôlé au préflight. `worker_ipc.rs`, `archive.rs` et `queue_store.rs` sont inclus en L1b-1 pour les validations de file et de reprise ; aucun changement de protocole IPC. **Dans `engine_capabilities.rs` et `tests/engine_capabilities.rs` rouverts depuis L1a, seules des additions sont permises : aucune assertion existante supprimée, réécrite ou affaiblie. Le reviewer compare leur diff à `6ccb1dfe` et vérifie chaque assertion L1a.** `domain.rs`, `ports.rs`, `ipc.rs`, `direct_live.rs`, workers, manifests et lockfile restent en lecture. Les chemins desktop restent en lecture en L1b-1 et les chemins core/adapters en L1b-2. Un besoin hors liste retourne `PLAN_CHANGE_REQUIRED` avant édition.

**`lot.paths.read` principaux :** règles et parents Git, `Cargo.toml`, `Cargo.lock`, les manifests des trois crates, modules L1a, `compute_policy.rs`, `gpu_selection.rs`, `ports.rs`, `domain.rs`, `ipc.rs`, `worker_ipc.rs`, `direct_live.rs`, tests import/live/GPU/settings actuels, fiches Turbo et artefacts Test effectivement disponibles. Les entrées locales modèles/audio sont lues seulement si la recette correspondante est autorisée et disponible.

**`lot.paths.runtime_write` :** `target/parakeet-tests/l1b-1/` puis `target/parakeet-tests/l1b-2/`, chacune pour Cargo, temporaires, fixtures et sorties propres. Le test `model_selection_ui.rs` utilise un snapshot injecté, **sans construire `DesktopApp`** tant que cela démarrerait le journal réel. Aucun essai ne modifie le profil utilisateur ni `sound example/`. Les tests écrivant à des chemins fixes hors racine demandent une révision du plan avant exécution. **`models/` est en lecture seule dans les deux sous-lots** ; aucun téléchargement ni écriture sous cette racine.

## 4. Cartes d'acceptation L1b

Les cartes suivantes sont **NOT RUN** avant implémentation. L1b-1 couvre B-A1–6, A8–10 ; L1b-2 couvre A5, A7–9. Les recettes réelles sont distinctes des tests de contrat ; si un modèle est absent ou différent au préflight, sa recette reste `NOT RUN` sans acquisition. La validation dans la fenêtre Windows reste distincte.

| Carte | Stimulus et point d'observation | Critère observable ; limite |
| --- | --- | --- |
| B-A1 | Deux modèles, fiches attribuées et fichiers présents, absents, corrompus ou annulés. | Deux entrées visibles ; fichier `Ready` **et** capacité admissible pour sélection, causes distinctes. Aucune preuve Turbo créée par fixture ; Parakeet produit reste non lançable. |
| B-A2 | Fichier modifié/supprimé entre projection et commande. | Refus expliqué avant effet moteur/capture, préférence intacte, contrôle répété avant délégation ; contrôle de l'ouverture worker encore futur. |
| B-A3 | Auto seul, manuel seul, aucun mode, FR/EN et code invalide ; commandes directes start/enqueue/process/reprise/live. | Projection et garde communs ; zéro appel moteur sur refus, aucune substitution. |
| B-A4 | JSON v1 Turbo connu/absent, chemin inconnu ; v2 modèle absent ; sauvegarde préexistante identique ou conflictuelle, échec d'écriture. | Identité connue restaurée sans ancien chemin source ; inconnu diagnostiqué et bloqué ; `settings.json.v1.bak` existe **avant la première écriture v2**, identique octet pour octet au v1 original, jamais écrasée. Conflit/échec laisse v1 intact avec ACK d'échec ; autres champs préservés. |
| B-A5 | Auto calcul sauvegardé, FollowProbe, sonde et ACK anciens/échoués, choix plus récent. | Choix courant préservé ; sauvegarde échouée visible ; aucune présélection tardive n'écrase Auto. |
| B-A6 | RAM/VRAM/disque inconnus, mesures comparables et autre GPU/volume. | Inconnu distinct d'insuffisant ; refus limité à la combinaison démontrée insuffisante. |
| B-A7 | egui avec snapshot injecté et préférence indisponible. | Deux modèles et langues avec causes, choix interdits grisés, plus de champ modèle libre ; audio/destination utilisables. |
| B-A8 | Matrice Turbo exhaustive de §1 bis : un modèle, 3 calculs, 2 parcours, Auto langue et les 100 codes + 100 noms exacts ; Parakeet non qualifié. | Test paramétré comptant les **1 206 combinaisons**, admission et commande sans inférence native, et couverture UI des 100 identités. **Aucune option Turbo existante nouvellement grisée ou refusée après L1b** selon sa disponibilité habituelle. Statut `Existant reconnu` avec preuve « comportement de production actuel, décision utilisateur du 16/10/2026 », explicitement non mesurée ; capacité Turbo nouvelle soumise à preuve. Aucun worker Parakeet lancé. |
| B-A9 | Diff, compilation et tests ciblés ; comparaison des deux fichiers L1a rouverts au commit `6ccb1dfe`. | Chemins attendus seulement, pas de dépendance/IPC/natif/réseau ajouté ; pas de test sauté silencieusement. **Toutes les assertions L1a existantes restent présentes et de même force ; seules des additions dans ces deux fichiers. Le reviewer le vérifie dans le diff.** |
| B-A10 | File et checkpoint : chemin Turbo historique explicitement connu, chemin inconnu, modèle Test absent, divergences source/SHA/job/génération/FIFO/préfixe. | Connu rattaché en mémoire à Turbo sans réécriture durable ; exécution via chemin catalogue vérifié. Inconnu refusé avec message explicite à l'utilisateur et zéro effet moteur. Contrôles d'intégrité existants préservés ; pas de reprise automatique. |

Rapprochement inverse : registre/catalogue → B-A1/2/6/8 ; admission/application → B-A2/3/5/8/10 ; settings → B-A4/5 ; composition/UI → B-A5/7/8 ; diff/tests → B-A9. Chaque sous-lot reste sous dix tâches et dix fichiers de rapport ; ce document est un seul fichier.

## 5. Préflight, commandes et sortie attendue

**Avant toute écriture de chaque sous-lot :** vérifier HEAD contenant L1a `6ccb1dfe`, design, revue et version Git exactes de ce plan ; relever branche, modifications et `git diff --check`, préserver `models/` et `sound example/`. Vérifier G-TURBO et la présence/taille/SHA de G-TEST-MODELS pour chaque recette réelle ; absence/divergence donne `NOT RUN` sans téléchargement. Obtenir l'acceptation du plan et l'accord précis du **sous-lot et de ses chemins** dans la session d'implémentation ; pour L1b-2, vérifier le verdict CLEAN de L1b-1. Vérifier chemins et effets indirects des commandes. Les variables `WHISPER_L01_*`, `WHISPER_L02_*` et `WHISPER_L04_*` sont retirées pendant les tests unitaires puis restaurées. `CARGO_TARGET_DIR`, `TEMP` et `TMP` pointent sous `target/parakeet-tests/l1b-1/` ou `l1b-2/` selon le sous-lot, puis retrouvent leurs valeurs antérieures. Aucun reset/nettoyage global.

**L1b-1, commandes proposées après candidat, non exécutées :**

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-core
cargo test --offline --locked -p whisper-adapters --test model_catalog --test model_preferences --test microphone_settings --test legacy_model_requests
cargo build --offline --locked --workspace
git diff --check
git diff --name-only
```

**L1b-2, commandes proposées après CLEAN de L1b-1 et candidat desktop, non exécutées :**

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-desktop --lib
cargo test --offline --locked -p whisper-desktop --test model_selection_ui --test gpu_selection_ui --test gpu_inventory_ui
cargo build --offline --locked --workspace
git diff --check
git diff --name-only
```

Les tests ciblés comptent leurs scénarios réellement exécutés ; les anciens tests à branche native conditionnelle restent `NOT RUN` sans données. Le desktop inchangé doit compiler après L1b-1. La compilation des workers ne vaut ni nouvelle inférence, ni qualification GPU. La recette réelle de chaque modèle est distincte des contrats et du paquet final. Un besoin de chemin source, dépendance, protocole ou écriture imprévu entraîne `PLAN_CHANGE_REQUIRED` avant modification. Chaque candidat rapporte diff, sorties, cartes et verdict indépendant. Aucun push.

**Statut de sortie de plan :** `DRAFT v3` avant revue indépendante et acceptation. Même si la revue conclut CLEAN, aucun code L1b-1 ni L1b-2 n'est autorisé sans accord d'implémentation écrit, précis, pour le sous-lot concerné.
