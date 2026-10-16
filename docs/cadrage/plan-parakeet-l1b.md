Avancée : L1a est livré CLEAN ; les décisions du 16/10/2026 scindent L1b en L1b-1 core/adapters puis L1b-2 desktop.
Blocage : le plan révisé attend la revue indépendante et l'accord écrit précis de chaque sous-lot ; Parakeet produit reste non qualifié.
Décision : `PLAN-PARA-L1b DRAFT v2` fixe chemins, gates et cartes ; aucun code L1b n'est autorisé par ce document.

# Plan Parakeet L1b v2 — raccordement du catalogue à l'application

**Base Git :** L1a `6ccb1dfe` sur `evo-startup-perf` ; design `docs/cadrage/evo-parakeet.md` et `docs/cadrage/evo-engine-capabilities.md`, révisés selon `DEC-PARA-24–28` et `DEC-CAP-07`. Ce document remplace DRAFT v1. Contributions `rust_plan_writer` et `rust_architect` en lecture seule ; la session principale rédige et versionne. **Périmètre :** L1b-1 puis L1b-2, sans backend Parakeet, paquet final ou changement du micro.

## 1. Résultat, limites et gates

L1b branche les règles pures de L1a dans l'application Windows. Le catalogue fermé montre Turbo et Parakeet ; un modèle absent, corrompu ou non admissible reste visible avec cause, grisé et non lançable. La langue manuelle devient une liste et Auto langue suit les capacités admissibles. L'application refuse la même demande inadmissible lorsqu'elle contourne le widget, notamment à l'ajout, au traitement et à la reprise d'un import, et aux deux voies live. Le choix du modèle, le choix Auto calcul et les paramètres v1 sont restaurés sans substitution silencieuse. **Toutes les options Turbo aujourd'hui exposées** (modèle, CPU/GPU, Auto calcul, import, live et langues utilisables) ont le statut distinct `Existant reconnu` et la provenance « comportement de production actuel, décision utilisateur du 16/10/2026 ». Cette décision de périmètre n'est pas une qualification mesurée. Aucune de ces options ne devient grisée ou refusée par L1b ; une capacité Turbo nouvelle exige une preuve.

**Parakeet reste grisé dans L1b.** Les inférences FR/EN du banc isolé ne qualifient pas le runtime, les workers, les profils et le paquet du produit. Le manuel GGUF/NeMo v0.2.0 est `Unsupported` ; Auto langue du candidat produit exige `AC-PARA-16F` avant activation. Un test peut injecter une fiche synthétique qualifiée pour éprouver la règle, jamais l'embarquer comme qualification réelle.

| Gate | Entrée nécessaire avant implémentation ou conclusion | Si absente |
| --- | --- | --- |
| G-DESIGN | Design révisé selon `DEC-CAP-07` et `DEC-PARA-24–28`, L1a `6ccb1dfe`, puis revue indépendante du plan v2 exact. | `PLAN_CHANGE_REQUIRED` ou `DESIGN_CHANGE_REQUIRED`. |
| G-TURBO | La matrice actuellement exposée reçoit `Existant reconnu` avec provenance exacte ci-dessus ; `Qualified` reste distinct. Toute capacité nouvelle exige une preuve. | Préserver tous les choix existants et réviser le plan si cette matrice ne peut être tenue. |
| G-TEST-MODELS | Profil Test fermé sous `C:\dev\whisper\models\` hors Git, sans secours Downloads/Final. Les deux fichiers exacts existent au contrôle du 16/10/2026. Taille/SHA vérifiés pour chaque recette réelle. | Continuer les tâches indépendantes ; acquérir uniquement le modèle manquant dans l'exécution autorisée, puis vérifier et faire la recette réelle correspondante avant PASS. |
| G-EXECUTION-1 | Plan revu et accepté, accord écrit précis pour L1b-1 et ses chemins, préflight complet. | Aucune écriture de code L1b-1. |
| G-EXECUTION-2 | Verdict **CLEAN** de L1b-1, puis accord écrit précis pour L1b-2 et ses chemins, préflight propre. | Ne pas commencer L1b-2. |

Au contrôle du 16/10/2026, Turbo mesure `1 624 555 275` octets, SHA-256 `1FC70F774D38EB169993AC391EEA357EF47C88757EF72EE5943879B7E8E2BC69` ; Parakeet `713 975 456` octets, SHA-256 `E3880D0AAAAF2C308EA2C35016B2B895C423EB3FDA924C1B463D1C19B7F4D32E`. Ce constat ne vaut pas recette d'inférence. En cas d'absence ultérieure, seul l'artefact fermé manquant est téléchargé depuis la source épinglée du design vers le temporaire du sous-lot, vérifié, puis publié sous `models/` hors Git. Échec de téléchargement ou de recette : `FAIL`/`NOT RUN` explicite, sans bloquer le travail de contrat indépendant. Les chiffres RAM/VRAM/disque restent `Unknown` sans mesure comparable, pas `Insufficient` inventé.

## 2. Contrats et tâches par sous-lot

**Architecture retenue pour le plan :** un module applicatif `model_selection_app.rs` possède la projection et l'admission ; `model_registry.rs` porte les fiches attribuées. Un port de catalogue et l'adaptateur `model_catalog.rs` constatent les artefacts et l'hôte hors du thread UI. La composition injecte leurs résultats et ne reconstruit aucune liste de langues ou de modèles admissibles dans `ui.rs`. Le protocole Turbo `ImportRequest`/`LiveRequest` et les workers restent inchangés : après admission, l'application construit la requête avec le chemin du catalogue, sans accepter un chemin fourni par une commande UI. Le contrôle d'empreinte est refait immédiatement avant délégation ; il ne constitue pas à lui seul une attestation de l'artefact finalement ouvert par le worker, qui reste un contrôle des lots moteurs.

| Tâche | Travail et preuve attendue | Traçabilité |
| --- | --- | --- |
| B1 / L1b-1 | Déclarer les deux modèles et fiches exactes ; `Existant reconnu` Turbo distinct de `Qualified`, sans extension à une capacité nouvelle ; Parakeet produit non qualifié. Adapter `engine_capabilities.rs` et ses tests. | DEC-CAP-07, DEC-PARA-24, B-A1/8. |
| B2 / L1b-1 | Ajouter observation locale injectée : fichier présent, taille/empreinte, runtime et ressource disponible/inconnue, erreurs, génération ; aucune recherche libre ni IO sur le thread UI. | PORT-CAT/HOST, CAP-32, AC-PARA-05/07A. |
| B3 / L1b-1 | Raccorder la projection L1a à la sélection : préférence distincte de sélection exécutable, refus typé, résultat tardif écarté. | INV-PARA-01/15, CAP-32/33. |
| B4 / L1b-1 | Garde d'admission avant start/enqueue/process/reprise import et start live, y compris commandes directes ; contrôle de l'artefact avant effet. Chemin Turbo historique **explicitement connu** dans file/checkpoint rattaché en mémoire à Turbo, requête exécutable avec chemin du catalogue vérifié ; chemin inconnu refusé avec message. Préserver donnée durable et contrôles source/SHA/job/génération/FIFO/préfixe confirmé. | AC-PARA-05/08C/16A–E, B-A2/3/10. |
| B5 / L1b-1 | Migrer v1→v2 : avant première écriture v2, copier les octets v1 dans `settings.json.v1.bak`, création exclusive, jamais écrasée ; conflit/échec bloque v2 et préserve v1. Résoudre Turbo historique connu, inconnu gardé en diagnostic ; conserver audio, destination, micro, GPU, modèle préféré et Auto/FollowProbe avec ACK révisionnés. | PORT-PREF, AC-PARA-08/08B, B-A4/5. |
| B6 / L1b-2 | Composer le profil Test depuis la racine du dépôt, injecter catalogue/préférences, garantir qu'Auto sauvegardé précède la sonde tardive et qu'un choix plus récent n'est pas écrasé. | DEC-PARA-06/09, PORT-CAT/PREF. |
| B7 / L1b-2 | Remplacer le chemin « Modèle local » et la saisie libre de langue par des listes consommant la projection ; griser avec cause, conserver audio et destination et toute la matrice Turbo actuelle. | DEC-PARA-02/10/16/24, CAP-33, B-A7/8. |
| B8-1 / L1b-1 | Tests core/adapters des états, admission, migration/sauvegarde, files/checkpoints et matrice Turbo ; fixture synthétique sans qualification de production. | B-A1–6/8/10. |
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

Les nouveaux modules/tests sont à créer si absents ; chaque chemin est recontrôlé au préflight. `worker_ipc.rs`, `archive.rs` et `queue_store.rs` sont inclus en L1b-1 pour les validations de file et de reprise ; aucun changement de protocole IPC. `domain.rs`, `ports.rs`, `ipc.rs`, `direct_live.rs`, workers, manifests et lockfile restent en lecture. Les chemins desktop restent en lecture en L1b-1 et les chemins core/adapters en L1b-2. Un besoin hors liste retourne `PLAN_CHANGE_REQUIRED` avant édition.

**`lot.paths.read` principaux :** règles et parents Git, `Cargo.toml`, `Cargo.lock`, les manifests des trois crates, modules L1a, `compute_policy.rs`, `gpu_selection.rs`, `ports.rs`, `domain.rs`, `ipc.rs`, `worker_ipc.rs`, `direct_live.rs`, tests import/live/GPU/settings actuels, fiches Turbo et artefacts Test effectivement disponibles. Les entrées locales modèles/audio sont lues seulement si la recette correspondante est autorisée et disponible.

**`lot.paths.runtime_write` :** `target/parakeet-tests/l1b-1/` puis `target/parakeet-tests/l1b-2/`, chacune pour Cargo, temporaires, fixtures et sorties propres. Le test `model_selection_ui.rs` utilise un snapshot injecté, **sans construire `DesktopApp`** tant que cela démarrerait le journal réel. Aucun essai ne modifie le profil utilisateur ni `sound example/`. Les tests écrivant à des chemins fixes hors racine demandent une révision du plan avant exécution. Les modèles locaux sont lus ; seul le téléchargement conditionnel d'un artefact manquant décrit par G-TEST-MODELS peut écrire sous `models/` hors Git lors d'une exécution autorisée.

## 4. Cartes d'acceptation L1b

Les cartes suivantes sont **NOT RUN** avant implémentation. L1b-1 couvre B-A1–6, A8–10 ; L1b-2 couvre A5, A7–9. Les recettes réelles sont distinctes des tests de contrat ; un modèle absent est acquis puis sa recette réalisée lors d'une exécution autorisée. La validation dans la fenêtre Windows reste distincte.

| Carte | Stimulus et point d'observation | Critère observable ; limite |
| --- | --- | --- |
| B-A1 | Deux modèles, fiches attribuées et fichiers présents, absents, corrompus ou annulés. | Deux entrées visibles ; fichier `Ready` **et** capacité admissible pour sélection, causes distinctes. Aucune preuve Turbo créée par fixture ; Parakeet produit reste non lançable. |
| B-A2 | Fichier modifié/supprimé entre projection et commande. | Refus expliqué avant effet moteur/capture, préférence intacte, contrôle répété avant délégation ; contrôle de l'ouverture worker encore futur. |
| B-A3 | Auto seul, manuel seul, aucun mode, FR/EN et code invalide ; commandes directes start/enqueue/process/reprise/live. | Projection et garde communs ; zéro appel moteur sur refus, aucune substitution. |
| B-A4 | JSON v1 Turbo connu/absent, chemin inconnu ; v2 modèle absent ; sauvegarde préexistante identique ou conflictuelle, échec d'écriture. | Identité connue restaurée sans ancien chemin source ; inconnu diagnostiqué et bloqué ; `settings.json.v1.bak` existe **avant la première écriture v2**, identique octet pour octet au v1 original, jamais écrasée. Conflit/échec laisse v1 intact avec ACK d'échec ; autres champs préservés. |
| B-A5 | Auto calcul sauvegardé, FollowProbe, sonde et ACK anciens/échoués, choix plus récent. | Choix courant préservé ; sauvegarde échouée visible ; aucune présélection tardive n'écrase Auto. |
| B-A6 | RAM/VRAM/disque inconnus, mesures comparables et autre GPU/volume. | Inconnu distinct d'insuffisant ; refus limité à la combinaison démontrée insuffisante. |
| B-A7 | egui avec snapshot injecté et préférence indisponible. | Deux modèles et langues avec causes, choix interdits grisés, plus de champ modèle libre ; audio/destination utilisables. |
| B-A8 | Matrice Turbo actuellement exposée : modèle, CPU/GPU, Auto calcul, import, live et langues aujourd'hui utilisables ; Parakeet non qualifié. | **Aucune option Turbo existante nouvellement grisée ou refusée après L1b** selon sa disponibilité habituelle. Statut `Existant reconnu` avec preuve « comportement de production actuel, décision utilisateur du 16/10/2026 », explicitement non mesurée ; capacité Turbo nouvelle soumise à preuve. Aucun worker Parakeet lancé. |
| B-A9 | Diff, compilation et tests ciblés. | Chemins attendus seulement, pas de dépendance/IPC/natif/réseau ajouté ; pas de test sauté silencieusement. |
| B-A10 | File et checkpoint : chemin Turbo historique explicitement connu, chemin inconnu, modèle Test absent, divergences source/SHA/job/génération/FIFO/préfixe. | Connu rattaché en mémoire à Turbo sans réécriture durable ; exécution via chemin catalogue vérifié. Inconnu refusé avec message explicite à l'utilisateur et zéro effet moteur. Contrôles d'intégrité existants préservés ; pas de reprise automatique. |

Rapprochement inverse : registre/catalogue → B-A1/2/6/8 ; admission/application → B-A2/3/5/8/10 ; settings → B-A4/5 ; composition/UI → B-A5/7/8 ; diff/tests → B-A9. Chaque sous-lot reste sous dix tâches et dix fichiers de rapport ; ce document est un seul fichier.

## 5. Préflight, commandes et sortie attendue

**Avant toute écriture de chaque sous-lot :** vérifier HEAD contenant L1a `6ccb1dfe`, design, revue et version Git exactes de ce plan ; relever branche, modifications et `git diff --check`, préserver `models/` et `sound example/`. Vérifier G-TURBO et G-TEST-MODELS pour chaque recette réelle. Obtenir l'acceptation du plan et l'accord précis du **sous-lot et de ses chemins** dans la session d'implémentation ; pour L1b-2, vérifier le verdict CLEAN de L1b-1. Vérifier chemins et effets indirects des commandes. Les variables `WHISPER_L01_*`, `WHISPER_L02_*` et `WHISPER_L04_*` sont retirées pendant les tests unitaires puis restaurées. `CARGO_TARGET_DIR`, `TEMP` et `TMP` pointent sous `target/parakeet-tests/l1b-1/` ou `l1b-2/` selon le sous-lot, puis retrouvent leurs valeurs antérieures. Aucun reset/nettoyage global.

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

**Statut de sortie de plan :** `DRAFT v2` avant revue indépendante et acceptation. Même si la revue conclut CLEAN, aucun code L1b-1 ni L1b-2 n'est autorisé sans accord d'implémentation écrit, précis, pour le sous-lot concerné.
