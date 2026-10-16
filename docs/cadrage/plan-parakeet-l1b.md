Avancée : L1a est livré CLEAN au commit `6ccb1dfe` ; le raccordement catalogue, préférences et interface est découpé pour L1b.
Blocage : les fiches Turbo initiales et la présence des modèles sous la racine Test ne sont pas établies ; le candidat produit Parakeet reste non qualifié.
Décision : `PLAN-PARA-L1b DRAFT v1` est soumis à revue ; ce document ne vaut ni PLANS READY ni autorisation d'implémenter.

# Plan Parakeet L1b — raccordement du catalogue à l'application

**Base Git :** `6ccb1dfe` sur `evo-startup-perf` ; design `docs/cadrage/evo-parakeet.md` et `docs/cadrage/evo-engine-capabilities.md` au commit `471821e5` ; plan L1a accepté dans `docs/cadrage/plan-evo-parakeet.md`. **Contribution :** `rust_plan_writer`, `PLAN-PARA-L1b DRAFT v1`, en lecture seule ; la session principale rédige et versionne ce document. **Périmètre :** L1b seul, sans inférence Parakeet, acquisition réseau, paquet final ou changement du micro.

## 1. Résultat, limites et gates

L1b branche les règles pures de L1a dans l'application Windows. Le catalogue fermé montre Turbo et Parakeet ; un modèle absent, corrompu ou non qualifié reste visible avec cause, grisé et non lançable. La langue manuelle devient une liste et Auto langue suit les capacités réellement qualifiées. L'application refuse la même demande inadmissible lorsqu'elle contourne le widget, notamment à l'ajout, au traitement et à la reprise d'un import, et aux deux voies live. Le choix du modèle, le choix Auto calcul de démarrage et les autres paramètres conservés par v1 sont restaurés sans substitution silencieuse. Le parcours Turbo existant doit rester utilisable dans la portée réellement démontrée.

**Parakeet reste grisé dans L1b.** Les inférences FR/EN du banc isolé ne qualifient pas le runtime, les workers, les profils et le paquet du produit. Le manuel GGUF/NeMo v0.2.0 est `Unsupported` ; Auto langue du candidat produit exige `AC-PARA-16F` avant activation. Un test peut injecter une fiche synthétique qualifiée pour éprouver la règle, jamais l'embarquer comme qualification réelle.

| Gate | Entrée nécessaire avant implémentation ou conclusion | Si absente |
| --- | --- | --- |
| G-DESIGN | Design L1 à `471821e5`, L1a `6ccb1dfe` et revue indépendante du plan L1b exact. | `PLAN_CHANGE_REQUIRED` ou `DESIGN_CHANGE_REQUIRED` selon l'écart. |
| G-TURBO | Fiches initiales attribuées au **candidat Turbo exact** : modèle/empreinte, runtime et build, worker/IPC, profil import/live, CPU/GPU, Auto et langues réellement démontrés. Les preuves et limites sont liées à chaque clé de configuration ; la présence de `set_language` ne qualifie aucune langue. | Ne pas brancher une fiche artificielle, désactiver Turbo par défaut, ni affirmer une couverture non prouvée. L'admission de la capacité concernée reste bloquée ; compléter la preuve ou réviser le design/lot. |
| G-TEST-MODELS | Profil Test résolu depuis `C:\dev\whisper\models\` hors Git, sans secours Downloads, chemin historique ou installation Final. Modèles locaux existants en lecture seule ; aucune copie autorisée par ce plan. | Tests synthétiques possibles ; recette réelle Turbo/Parakeet `NOT RUN`. Ne pas annoncer un parcours Test utilisable. |
| G-EXECUTION | Plan revu et accepté, lot et chemins autorisés précisément dans la session d'implémentation. | Aucune écriture de code. |

La fiche Turbo ne peut être déduite du simple succès d'un build ou d'un test conditionnel sans données. Si la preuve n'établit qu'une sous-configuration, seule celle-ci peut recevoir `Qualified`. Les autres choix existants réclament une décision de périmètre explicite avant leur désactivation. Les chiffres RAM/VRAM/disque restent `Unknown` sans mesure comparable, pas `Insufficient` inventé.

## 2. Contrats et tâches (9)

**Architecture retenue pour le plan :** un module applicatif `model_selection_app.rs` possède la projection et l'admission ; `model_registry.rs` porte les fiches attribuées. Un port de catalogue et l'adaptateur `model_catalog.rs` constatent les artefacts et l'hôte hors du thread UI. La composition injecte leurs résultats et ne reconstruit aucune liste de langues ou de modèles admissibles dans `ui.rs`. Le protocole Turbo `ImportRequest`/`LiveRequest` et les workers restent inchangés : après admission, l'application construit la requête avec le chemin du catalogue, sans accepter un chemin fourni par une commande UI. Le contrôle d'empreinte est refait immédiatement avant délégation ; il ne constitue pas à lui seul une attestation de l'artefact finalement ouvert par le worker, qui reste un contrôle des lots moteurs.

| Tâche | Travail et preuve attendue | Traçabilité |
| --- | --- | --- |
| B1 | Déclarer les deux modèles, leurs artefacts Test et fiches par clés exactes ; Turbo seulement selon G-TURBO, Parakeet produit non qualifié. Conserver preuve et limite de chaque capacité. | CAP-31/34, DEC-CAP-01/02. |
| B2 | Ajouter observation locale injectée : fichier présent, taille/empreinte, runtime et ressource disponible/inconnue, erreurs, génération ; aucune recherche libre ni IO sur le thread UI. | PORT-CAT/HOST, CAP-32, AC-PARA-05/07A. |
| B3 | Raccorder la projection L1a à la sélection : préférence distincte de sélection exécutable, refus typé, résultat tardif écarté. | INV-PARA-01/15, CAP-32/33. |
| B4 | Ajouter le garde d'admission applicatif avant start/enqueue/process/reprise import et start live ; une commande contournant le widget ne fournit ni modèle arbitraire, ni langue/calcul non admis. Recontrôler l'artefact juste avant l'effet. | AC-PARA-05/08/16A–E. |
| B5 | Migrer les réglages v1 vers v2 : Turbo historique vérifié ou intention du chemin historique connu, chemin inconnu gardé en diagnostic et bloqué ; conserver dossier audio, destination, micro et GPU. Ajouter modèle préféré et Auto/FollowProbe avec acquittements de sauvegarde révisionnés. | PORT-PREF, AC-PARA-08, Q-PARA-07. |
| B6 | Composer le profil Test depuis la racine du dépôt, injecter catalogue/préférences, garantir qu'Auto sauvegardé précède la sonde tardive et qu'un choix plus récent n'est pas écrasé. | DEC-PARA-06/09, PORT-CAT/PREF. |
| B7 | Remplacer le chemin « Modèle local » et la saisie libre de langue par des listes consommant la projection ; griser avec cause, conserver les champs audio et destination. | DEC-PARA-02/10/16, CAP-33. |
| B8 | Tests isolés des états de fichier, de l'admission, de migration, concurrence des résultats et contrôles egui, sans inférence native ; aucune fixture synthétique ne devient une fiche de production. | Cartes B-A1–B-A8. |
| B9 | Vérifier le diff, les commandes et la matrice de chemins ; soumettre un candidat à `rust_review_coordinator` pour deux cycles revue/correction maximum. | RG-01/07/09/10, B-A9. |

Ordre : B1 précède B2/B3 ; B2+B3 précèdent B4 ; B3 précède B5 ; B2–B5 précèdent B6 ; B3+B5+B6 précèdent B7. B8 accompagne les tâches, B9 suit le candidat. Aucun travail parallèle indépendant n'est présumé sur `path_settings.rs`, la composition ou l'UI.

## 3. Chemins candidats à figer par la revue

**`lot.paths.write` proposé, 18 chemins source/tests :**

```text
crates/whisper-core/src/lib.rs
crates/whisper-core/src/application.rs
crates/whisper-core/src/model_selection_app.rs
crates/whisper-core/src/model_registry.rs
crates/whisper-core/src/path_settings.rs
crates/whisper-core/tests/model_selection_app.rs
crates/whisper-core/tests/model_preferences.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/model_catalog.rs
crates/whisper-adapters/src/path_settings.rs
crates/whisper-adapters/tests/model_catalog.rs
crates/whisper-adapters/tests/model_preferences.rs
crates/whisper-adapters/tests/microphone_settings.rs
crates/whisper-desktop/src/lib.rs
crates/whisper-desktop/src/main.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-desktop/tests/model_selection_ui.rs
```

Les nouveaux modules et tests nommés ci-dessus sont à créer ; l'existence de chaque chemin sera recontrôlée au préflight. `microphone_settings.rs` construit `StoredPaths` directement et doit être adapté à v2. `model_catalog.rs` et `engine_capabilities.rs` de L1a, `domain.rs`, `ports.rs`, `ipc.rs`, `worker_ipc.rs`, `direct_live.rs`, workers, manifests et lockfile restent **en lecture**. Un besoin réel de modifier l'un de ces chemins retourne `PLAN_CHANGE_REQUIRED` avant édition ; aucun ajout automatique à la liste.

**`lot.paths.read` principaux :** règles et parents Git, `Cargo.toml`, `Cargo.lock`, les manifests des trois crates, modules L1a, `compute_policy.rs`, `gpu_selection.rs`, `ports.rs`, `domain.rs`, `ipc.rs`, `worker_ipc.rs`, `direct_live.rs`, tests import/live/GPU/settings actuels, fiches Turbo et artefacts Test effectivement disponibles. Les entrées locales modèles/audio sont lues seulement si la recette correspondante est autorisée et disponible.

**`lot.paths.runtime_write` proposé :** `target/parakeet-tests/l1b/` pour cible Cargo, temporaires, fixtures artificielles, settings v1/v2, sorties de tests et journaux. Le test d'intégration `model_selection_ui.rs` exerce les widgets egui avec un snapshot injecté, **sans construire `DesktopApp`** ; sa construction démarrerait le journal réel à moins de le rediriger explicitement. Les essais UI/desktop n'écrivent ni dans le profil utilisateur réel ni dans `models/`. `sound example/` demeure intact. Les tests existants qui construisent des chemins fixes sous `target/` hors de cette racine sont exclus de la campagne tant qu'ils ne sont pas inclus dans un plan révisé ; `tests/microphone_settings.rs` est adapté pour utiliser la racine L1b.

## 4. Cartes d'acceptation L1b

Les cartes suivantes sont **NOT RUN** avant implémentation. Les données de test sont construites dans la racine runtime. Une recette dans la fenêtre Windows avec de vrais modèles et un profil Test conforme reste séparée, réservée à l'utilisateur et `NOT RUN` si les modèles sont absents.

| Carte | Stimulus et point d'observation | Critère observable ; limite |
| --- | --- | --- |
| B-A1 | Deux modèles, fiches attribuées et fichiers synthétiques présents, absents, corrompus ou annulés. | Deux entrées restent visibles ; seul `Ready` et qualifié est admissible, causes distinctes. Aucune preuve Turbo créée par fixture. |
| B-A2 | Fichier modifié/supprimé entre projection et commande ; ancienne entrée de file ou checkpoint portant un chemin hors catalogue, puis reprise. | Refus expliqué avant effet moteur/capture, préférence intacte, aucune réécriture silencieuse de la requête durable ou du checkpoint ; contrôle d'ouverture worker encore futur. |
| B-A3 | Auto seul, manuel seul, aucun mode, FR/EN et code invalide ; commandes directes start/enqueue/process/reprise/live. | Projection et garde communs ; zéro appel moteur sur refus, aucune substitution. |
| B-A4 | JSON v1 Turbo connu/absent, chemin inconnu ; v2 modèle devenu absent. | Identité connue restaurée sans utiliser l'ancien chemin comme source ; inconnu gardé pour diagnostic et bloqué ; autres champs préservés. |
| B-A5 | Auto calcul sauvegardé, FollowProbe, sonde et ACK anciens/échoués, choix plus récent. | Choix courant préservé ; sauvegarde échouée visible ; aucune présélection tardive n'écrase Auto. |
| B-A6 | RAM/VRAM/disque inconnus, mesures comparables et autre GPU/volume. | Inconnu distinct d'insuffisant ; refus limité à la combinaison démontrée insuffisante. |
| B-A7 | egui avec snapshot injecté et préférence indisponible. | Deux modèles et langues avec causes, choix interdits grisées, plus de champ modèle libre ; audio/destination utilisables. |
| B-A8 | Turbo courant et fiche G-TURBO attestée, CPU/GPU/Auto, import/live ; Parakeet non qualifié. | Turbo n'est pas dégradé dans la portée prouvée ; aucune demande Parakeet ne lance de worker. L'inférence réelle reste `NOT RUN` ici. |
| B-A9 | Diff, compilation et tests ciblés. | Chemins attendus seulement, pas de dépendance/IPC/natif/réseau ajouté ; pas de test sauté silencieusement. |

Rapprochement inverse : registre/catalogue → B-A1/2/6/8 ; admission/application → B-A2/3/5/8 ; settings → B-A4/5 ; composition/UI → B-A5/7/8 ; tous les chemins et les tests → B-A9. Les neuf tâches et neuf cartes restent sous la limite de dix tâches du projet ; ce document constitue un seul fichier de rapport.

## 5. Préflight, commandes et sortie attendue

**Avant toute écriture :** vérifier HEAD contenant `6ccb1dfe`, design, revue et version Git exactes de ce plan ; relever branche, modifications et `git diff --check`, préserver `sound example/`. Fermer G-TURBO et vérifier G-TEST-MODELS pour chaque recette réelle. Obtenir l'acceptation du plan et l'accord précis du lot/chemins dans la session d'implémentation. Vérifier les chemins source/tests et tous les effets indirects des commandes. Les variables `WHISPER_L01_*`, `WHISPER_L02_*` et `WHISPER_L04_*` sont retirées pendant les tests unitaires puis restaurées. `CARGO_TARGET_DIR`, `TEMP` et `TMP` pointent sous `target/parakeet-tests/l1b/` pour la campagne et retrouvent leurs valeurs antérieures. Aucun reset/nettoyage global.

**Commandes proposées, non exécutées :**

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-core
cargo test --offline --locked -p whisper-adapters --test model_catalog --test model_preferences --test microphone_settings
cargo test --offline --locked -p whisper-desktop --lib
cargo test --offline --locked -p whisper-desktop --test model_selection_ui --test gpu_selection_ui --test gpu_inventory_ui
cargo build --offline --locked --workspace
git diff --check
git diff --name-only
```

Les tests ciblés doivent compter leurs scénarios réellement exécutés ; les anciens tests à branche native conditionnelle restent `NOT RUN` sans données, jamais `PASS` implicite. La compilation des workers existants ne vaut ni nouvelle inférence, ni qualification d'identité GPU. Le raccordement du catalogue, la recette Windows et le paquet final ont des verdicts distincts. Un besoin de chemin source, de dépendance, de téléchargement, d'IO hors runtime ou de modification du protocole hors plan entraîne `PLAN_CHANGE_REQUIRED` avant modification.

**Statut de sortie de plan :** en l'état, `DRAFT` car G-TURBO et G-TEST-MODELS ne sont pas établis. Une revue indépendante de ce candidat Git doit rendre ses constats. Si elle juge le découpage suffisamment précis malgré les gates, le document peut devenir `PLANS READY-L1b sous prérequis`, sans autoriser le code ; si G-TURBO est nécessaire dès l'implémentation, sa fermeture reste un préflight bloquant. Le propriétaire des décisions de périmètre demeure l'utilisateur.
