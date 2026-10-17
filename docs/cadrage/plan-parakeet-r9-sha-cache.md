Avancée : R7 et R8 sont committés ; R9 peut cibler les lectures SHA répétées de l’adaptateur.
Blocage : l’identifiant Windows de fichier requis par R9 demande une dépendance directe absente des chemins autorisés.
Décision : soumettre ce plan à revue ; aucune édition de code avant résolution de ce périmètre.

# PLAN-PARA-R9 v1 — cache des empreintes de l’adaptateur

**Statut : DRAFT v2, après revue v1 NOT READY ; second cycle différé jusqu’aux décisions R9-P1/P2.** Base : `wip/l2b-candidate`, commit `30cdd25b`. Ce R9 est distinct du R8 historique de robustesse, relu CLEAN. L’autorisation conditionnelle de R9 ouvre seulement `crates/whisper-adapters` (sources et tests) et `docs/cadrage/`, avec implémentation après plan CLEAN. Aucun push.

**Traçabilité :** directive utilisateur R9 du 17/10/2026 (cache, métrique EN5, recettes EN30/A4/R8, suite workspace), puis clarification de la même session (plafond des lectures de l’adaptateur seul, SHA du worker séparé, worker gelé). Le contrat d’intégrité et de refus du modèle provient du design versionné `docs/cadrage/evo-parakeet.md` et de la fiche Test R6 ; les anciens passages de ce design relatifs à la VAD ne rouvrent pas B8. `docs/cadrage/bilan-parakeet-r7.md` fournit la base de temps ; `docs/cadrage/bilan-parakeet-r8-reprise.md` fixe les cinq scénarios de robustesse.

## But et preuve initiale

R7 a observé huit lectures SHA complètes du modèle dans l’adaptateur et une dans l’archive par import : au moins 72,764 s pour EN5 et 136,567 s pour EN30 (`bilan-parakeet-r7.md`). Le calcul se répète dans `parakeet_import.rs` (`verify`, `qualification_key`, `verify_worker_runtime`) et dans `archive.rs` avant pending ; `model_catalog.rs::inspect`, créé par l’adaptateur, relit aussi le modèle. Le cache doit couvrir **toutes** les lectures complètes du GGUF faites par `whisper-adapters` dans une session du processus applicatif, y compris le catalogue et l’archive, et refuser tout artefact dont l’empreinte observée diverge de la fiche Test.

Le plafond de **une lecture complète par identité de fichier et par session** porte sur le seul adaptateur : compteur `<= 1` au premier import, `0` au deuxième import de la même session. La lecture du worker gelé `1d8f57b9…` au démarrage (`whisper-worker-parakeet-cpu/src/main.rs:64`) est hors plafond, conservée comme attestation du fichier effectivement ouvert. Elle est relevée séparément, attendue une fois par démarrage. Le parcours actuel démarre un processus worker par import (`worker_ipc.rs`, chemin de spawn) ; indiquer par mesure si ce comportement est confirmé, sans le changer.

## Contrat du cache

1. Avant **chaque délégation**, relever sur un handle du fichier régulier la taille, la date de modification et l’identité physique Windows (volume et identifiant de fichier), et confronter ces trois valeurs à l’entrée en cache. Ne pas déduire l’identité du seul chemin canonique. Un échec de lecture d’identité refuse la délégation.
2. À la première identité rencontrée pendant la session, calculer SHA-256 en lecture complète une seule fois et mémoriser ensemble chemin, identité, empreinte et handle/garantie de validité. Une identité inchangée réutilise l’empreinte. Identité changée : recalculer ; empreinte différente de la fiche : refuser **avant pending**, avec cause explicite, sans bascule Turbo. Synchroniser les imports concurrents pour ne pas calculer deux fois le même contenu.
3. Si taille ou date change pendant le calcul, ne pas mettre l’entrée en cache ; refuser ou recommencer une seule fois sous contrôle, puis échouer fermé. Une modification du contenu qui conserve volontairement taille, date et identifiant échappe à ces trois critères : le worker garde sa vérification SHA indépendante, qui refuse avant inférence. Documenter cette limite sans prétendre garantir un contenu immuable à partir des seules métadonnées.
4. Le cache vit dans le processus applicatif et meurt avec lui. Aucun cache durable, aucune préférence modifiable, aucune modification de la fiche, de la clé Test ou du worker. Les lectures SHA des autres artefacts (DLL, exécutable, source, sorties) restent soumises aux règles actuelles ; les compter séparément pour le diagnostic.

## Gate technique préalable — PLAN_CHANGE_REQUIRED

Sur Rust stable 1.98.1, `std::os::windows::fs::MetadataExt::file_index()` et `volume_serial_number()` sont instables (`windows_by_handle`, source locale `std/src/os/windows/fs.rs`). Le crate `same-file 1.0.6`, déjà dans `Cargo.lock` et le cache hors ligne, expose un `Handle` sûr qui compare volume et index en gardant le handle ; il n’est toutefois pas une dépendance directe de `whisper-adapters`. L’ajouter requiert **au minimum** `crates/whisper-adapters/Cargo.toml` ; `Cargo.lock` doit être inclus si Cargo le modifie. Ces deux chemins ne sont pas couverts par l’autorisation R9. L’autre voie Win32 demanderait une feature/dépendance et du code `unsafe`, également hors périmètre.

La revue v1 a confirmé ce constat (**R9-P1 BLOQUANT**). Sans accord écrit précis pour ajouter `crates/whisper-adapters/Cargo.toml` et, si nécessaire, `Cargo.lock`, statut `NOT READY / PLAN_CHANGE_REQUIRED`. La présence transitive dans le lockfile ne vaut pas autorisation d’ajouter une dépendance. Ne pas remplacer l’identifiant physique par le chemin, la taille ou la date seuls. La machine de qualification actuelle est NTFS (`Get-Volume C`), et la portée de l’identité volume/index du crate devra être indiquée pour les autres systèmes de fichiers Windows.

**Second gate R9-P2 BLOQUANT :** la métrique `EN5 total <= inférence + 30 s` exige l’inférence de la même exécution. Le worker R6 gelé sait produire `PARAKEET_R6_TIMING_LOG`, mais `native_engine.rs::log_native_timing` limite son écriture à `…/l2b/r6`; un chemin R9 échoue, et écrire sous R6 sort de la racine d’exécution R9. Les temps du worker privé R7 ne sont pas substituables. Deux solutions nécessitent l’arbitrage utilisateur : autoriser **uniquement** un journal de chronométrage R9 nommé sous `…/l2b/r6` pour le worker gelé, en maintenant toutes les autres écritures sous R9 ; ou modifier le critère pour mesurer la durée demande IPC → réponse, laquelle inclut décodage et transport et n’est pas le temps d’inférence. Aucun worker n’est reconstruit ni modifié dans ce plan. Sans décision, EN5 reste non mesurable et R9 `NOT READY`.

## Lot proposé après fermeture du gate

Au plus huit tâches, sous réserve de confirmation des chemins en revue :

| Carte | Action et preuve |
| --- | --- |
| R9-1 | Préflight branche/HEAD/diff, espace libre >= 30 Gio, modèle, worker et cinq DLL SHA attendus ; garde PowerShell OK/KO avant Cargo. `CARGO_TARGET_DIR=target/parakeet-tests/cargo-shared/`, sorties/temp/preuves sous `target/parakeet-tests/l2/l2b/r9/`, variables de lecture explicitement distinguées. |
| R9-2 | Cache partagé et synchronisé de l’adaptateur ; identité taille/date/file ID vérifiée avant délégation ; compteur de lectures complètes par artefact et session. Échec fermé si identité indisponible ou instable. |
| R9-3 | Raccorder catalogue, préparation, archive et import à ce point unique ; aucune seconde lecture GGUF par l’adaptateur pour identité inchangée. Préserver les SHA de worker/DLL/source et le contrôle autonome du worker. |
| R9-4 | Tests unitaires : même identité deux imports `1` puis `0`, concurrence, taille/date/identifiant changés, empreinte altérée et refus avant pending, entrée absente, erreur de métadonnées ; Turbo inchangé. Les fixtures de test ne touchent pas `models/`. |
| R9-5 | Recettes Test réelles sur worker gelé : EN5 total demande→TXT/SRT `<=` inférence `+30 s`, démarrage worker et son SHA inclus ; EN30 PASS ; A4 FR87/FR27/EN15 PASS ; cinq scénarios R8 PASS. Séparer temps inférence, temps produit, lectures SHA adaptateur et worker, et nombre de démarrages. Si EN5 échoue, rapporter les temps mesurés et la cause sans modifier le critère. **Cette carte dépend de R9-P2.** |
| R9-6 | `cargo fmt --all -- --check`, tests ciblés, puis `cargo test --offline --locked --workspace --no-fail-fast` sous garde cargo-shared, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. Lister tous FAIL et NOT RUN ; ne pas transformer les tests ignorés en PASS. |
| R9-7 | Revue du candidat par `rust_review_coordinator`, deux cycles maximum ; commit local après CLEAN seulement, puis archivage des petites preuves, nettoyage par manifeste approuvé et bilan d’espace libre. |

**Chemins source nominés (huit dans le périmètre actuel) :** `crates/whisper-adapters/src/model_catalog.rs`, `src/parakeet_import.rs`, `src/archive.rs`, `src/worker_ipc.rs`, `tests/model_catalog.rs`, `tests/parakeet_import.rs`, `tests/parakeet_archive.rs` (son seul helper `archive_root()`), `tests/legacy_model_requests.rs` (non-régression de l’identité Turbo). Le point d’identité physique ajoute, sauf solution alternative validée, `crates/whisper-adapters/Cargo.toml` et potentiellement `Cargo.lock` : **non autorisés** aujourd’hui. `docs/cadrage/` reçoit le plan et le bilan. Aucun autre crate, worker, C++, réseau, `models/`, `sound example/`, VAD, B8 ou push. Si le nombre de fichiers de code dépasse dix, demander l’arbitrage prescrit par `AGENTS.md`.

### Recettes et garde à préparer dans les chemins admis

Le test `tests/parakeet_import.rs::r9_application_recipe` sera un nouveau test ignoré, paramétré par `PARAKEET_R9_CASE=en5|en30|fr87|fr27|en15` ; il reprend le parcours `ImportApplication` de `r6_application_single_pass_recipe` et la sonde produit de `r7_application_timing`, sans réutiliser leurs racines d’écriture. Il vérifie TXT/SRT et l’absence de `InvalidTimestamp`. Entrées en lecture : `sound example/spkr0.wav` pour EN5 ; `target/parakeet-tests/l2/l2b/r5-2/fixtures/spkr0_x6.wav` pour EN30 ; `target/parakeet-feasibility/spike02/fixtures/{fr87,fr27,en15}.wav` pour A4 ; `models/parakeet-tdt-0.6b-v3.q8_0.gguf`, worker exact `target/parakeet-tests/l2/l2b/r6/worker/whisper-worker-parakeet-cpu.exe` (SHA `1d8f…`) et cinq DLL sous `…/l2b/native/`. Les entrées sont contrôlées par SHA au préflight ; absentes ou divergentes => cas NOT RUN, aucun téléchargement. Sorties de chaque cas sous `…/l2b/r9/{archive,logs,outputs,temp,tmp}/`, avec destination unique par cas. `PARAKEET_L2_RUNTIME_ROOT=…/l2b` reste ancre de lecture exacte. `tests/parakeet_archive.rs::archive_root()` accepte la racine canonique R9 injectée, sans changer le prédicat produit ni ses refus hors racine. Le helper `tests/parakeet_import.rs::r8_exec_root()` admet également la racine R9 canonique pour **rejouer** les cinq tests R8 sous R9 ; l’acceptation R8 historique et chaque assertion de refus restent intactes. Aucun test R9 n’écrit sous R8.

La garde PowerShell vérifie avant le **premier** Cargo puis chaque Cargo : espace libre >= 30 Gio ; `CARGO_TARGET_DIR` égal à `target/parakeet-tests/cargo-shared/` ; `TEMP`, `TMP`, `PARAKEET_R9_EXEC_ROOT`, archive, logs et chaque variable de sortie canoniquement sous `…/l2b/r9` ; racines de lecture exactes et fichiers présents ; aucune jonction sortante. Elle est exercée une fois avec cas OK et une fois avec racine KO, le second devant arrêter avant Cargo. `CARGO_INCREMENTAL=0` et `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. L’éventuel journal natif R6 de R9-P2 forme une exception **non autorisée** et ne peut entrer dans la garde avant décision.

Après garde, commandes de test ciblé prévues, sur le même exécutable gelé :

```powershell
$env:PARAKEET_R9_CASE = 'en5';  cargo test --offline --locked -p whisper-adapters --test parakeet_import r9_application_recipe -- --ignored --exact --nocapture
$env:PARAKEET_R9_CASE = 'en30'; cargo test --offline --locked -p whisper-adapters --test parakeet_import r9_application_recipe -- --ignored --exact --nocapture
foreach ($r9Case in @('fr87','fr27','en15')) { $env:PARAKEET_R9_CASE = $r9Case; cargo test --offline --locked -p whisper-adapters --test parakeet_import r9_application_recipe -- --ignored --exact --nocapture }
cargo test --offline --locked -p whisper-adapters --test parakeet_import incompatible_native_dll_is_refused_by_product_and_fails_at_loader_boundary -- --ignored --exact --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_import killed_native_worker_is_reported_as_exit_without_result -- --ignored --exact --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_import incomplete_result_frame_is_rejected_as_transport_decode_failure -- --ignored --exact --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_import product_route_incomplete_result_preserves_pending_source_and_rejects_completion -- --ignored --exact --nocapture
cargo test --offline --locked -p whisper-adapters --test parakeet_import product_route_worker_death_preserves_pending_source_and_rejects_completion -- --ignored --exact --nocapture
cargo fmt --all -- --check
cargo test --offline --locked --workspace --no-fail-fast
```

Délais applicatifs : EN5 `195 s`, EN30 `570 s` (`durée_audio/4 + 120 s`), FR/EN courts selon leur durée avec la même formule. Le compteur cache est relevé dans le processus de recette, hors calculs SHA du harness. Le chronomètre produit va de `StartImport` à TXT/SRT écrits ; le chronomètre d’inférence doit provenir du **même worker et de la même exécution**, suivant décision R9-P2. Chaque recette rapporte PASS/FAIL/NOT RUN, temps et lectures adaptateur/worker séparés. La suite workspace énumère ses ignorés comme NOT RUN.

## Critères de clôture

Compteur adaptateur modèle `<= 1` par identité dans la session, `0` au second import ; modification détectée puis refus avant pending ; worker contrôlé séparément (une lecture SHA par démarrage observé) ; EN5 total `<=` inférence `+30 s` avec ce démarrage ; EN30, A4 et cinq R8 PASS ; suite workspace complète verte hors NOT RUN explicités ; revue de code CLEAN. Toute carte non exécutée reste NOT RUN et le lot ne reçoit pas CLEAN par déduction.
