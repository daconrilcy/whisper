Avancée : préflight de la mesure R11 terminé ; les traces et recettes existantes ne permettent pas la ventilation demandée.
Blocage : une instrumentation limitée et un test applicatif de référence exigent un accord écrit distinct avant tout code.
Décision : proposer trois tâches E0, isolées du plan produit, pour mesurer sans modifier les règles de retraitement.

# PLAN-MESURE-RETRAITEMENT-E0 v1 — DRAFT

Base : [mandat de cadrage](mandat-retraitement-cible.md), [design accepté](design-retraitement-cible.md) et [plan produit DRAFT](plan-retraitement-cible.md). Auteur documentaire : session Codex principale, contribution technique en lecture seule de `rust_plan_writer`. L'accord utilisateur du 20/10/2026 permet une **campagne isolée sans code nouveau** ; son préflight s'est arrêté avant toute compilation ou transcription, car aucun journal R11 exploitable n'existe et les harness actuels ne couvrent pas trois sources et trois parcours. **Le présent mini-plan n'est pas un accord pour créer le test ou éditer la production.** Aucun résultat de temps R11 n'est revendiqué.

## 1. Trois tâches, chemins fermés

| Tâche | Modification proposée | Résultat et limite |
| --- | --- | --- |
| E0.1 Traces | Dans `archive.rs`, chronométrer les scans de publications ; dans `worker_ipc.rs`, les phases admission et confirmation, en utilisant `record_r7_timing` déjà conditionné par l'environnement. Identifier opération, PID, phase, issue et durée ; ne modifier ni vérification SHA, ni décision, ni format persistant. | Scans et phases attribuables ; l'instrumentation peut ajouter un léger coût de journalisation, à signaler. |
| E0.2 Harness | Ajouter un unique test `r11_application_baseline`, ignoré par défaut, qui utilise le vrai service d'import et Parakeet CPU, mais des destinations de fixture neuves sous E0. Pour chacune des trois sources : sans doublon, puis doublon confirmé Créer et Remplacer ; répéter une seconde fois dans la même session. Aucun ancien traitement utilisateur n'est ouvert ou supprimé. | TXT/SRT et états réellement vérifiés ; temps par opération, hors attente humaine ; un résultat non publié est FAIL. |
| E0.3 Campagne | Adapter une copie de la garde sous E0, vérifier ses auto-tests négatifs, lancer les commandes bornées, relever événements et sorties, puis produire le tableau de référence. | Nombre et durée des SHA adaptateur, probes, scans, phases et temps applicatif ; lecture/décodage worker séparés si ses événements existants suffisent, sinon NOT RUN. |

**Les seuls chemins Rust proposés à l'écriture E0 sont :**

```text
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/tests/reprocessing_baseline.rs (nouveau)
```

Pas de `Cargo.toml`, `Cargo.lock`, worker, modèle, DLL, C++, unsafe, code desktop, changement de protocole, migration ou suppression de données réelles. Si un autre chemin devient nécessaire, arrêt et `PLAN_CHANGE_REQUIRED` avant édition. Les modifications d'instrumentation doivent être isolées dans un commit candidat et retirables sans altérer le parcours mesuré ; leur présence et leur éventuel effet sur les durées sont documentés.

## 2. Sources, racines et préflight

Racine unique d'exécution `R = C:\dev\whisper\target\parakeet-tests\retraitement-cible\mesure-avant\`. Toutes les destinations de test, copies de fixtures, temporaires, journaux, scripts de garde et résultats sont sous `R`. Compilation uniquement sous `C:\dev\whisper\target\parakeet-tests\cargo-shared\`. Les trois sources en lecture seule sont :

```text
C:\dev\whisper\sound example\spkr0.wav
C:\dev\whisper\target\parakeet-tests\l2\l2b\r5-2\fixtures\spkr0_x6.wav
C:\Users\cyril\Downloads\kamhunt-old-school-preacher-clips-3-pastor-darryl-hussey-victorious-129623.mp3
```

Ressources Parakeet CPU à vérifier et à lire seulement : `C:\dev\whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf`, `C:\dev\whisper\target\parakeet-tests\l2\l2b\r6\worker\whisper-worker-parakeet-cpu.exe` et les cinq DLL de `C:\dev\whisper\target\parakeet-tests\l2\l2b\native\`. Avant et après la campagne, relever taille et SHA-256 des trois sources, du modèle, du worker et des cinq DLL, hors fenêtres de mesure ; une différence donne FAIL et stoppe les suites. Vérifier HEAD, `git status --short`, accord précis E0, chemins canoniques et espace libre C: ≥30 Gio avant Cargo. `models/`, `sound example/`, `target/parakeet-feasibility/spike02/`, workers/DLL gelés, destination personnelle et `%LOCALAPPDATA%\Whisper` restent protégés.

Variables écrivant sous `R` : `TEMP=R\temp`, `TMP=R\tmp`, `PARAKEET_R7_EXEC_ROOT=R`, `PARAKEET_R7_TIMING_LOG=R\logs\timing.jsonl`, `PARAKEET_L2_ARCHIVE_ROOT=R\archive`, `PARAKEET_L2_LOG_DIR=R\logs` et `WHISPER_RTC_BASELINE_ROOT=R`. Fixer `PARAKEET_R7_TIMING_ACTIVE=1`, `CARGO_INCREMENTAL=0` et `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. `CARGO_TARGET_DIR` doit désigner exactement le Cargo partagé. Le harness reçoit les trois chemins sources par `WHISPER_RTC_BASELINE_WAV_SHORT`, `WHISPER_RTC_BASELINE_WAV_LONG` et `WHISPER_RTC_BASELINE_MP3` ; aucune variable manquante n'est convertie en succès silencieux. Le runtime, le modèle, le worker et le dossier DLL sont des variables de **lecture** validées canoniquement. N'activer ni campagnes L04/R8, ni recettes personnelles.

La garde R11 existante ne peut pas être lancée telle quelle : elle fixe `$lot=r11` et possède des modes personnels. Sa copie E0 doit permettre **uniquement** les écritures R et Cargo partagé, surveiller les chemins protégés, propager le code réel des commandes et échouer avant Cargo si une variable d'écriture s'échappe. Ses auto-tests négatifs doivent couvrir TEMP extérieur, Cargo extérieur, destination personnelle, log extérieur et lien/redirection sortant ; un auto-test positif prouve l'écriture permise sous R. Aucun auto-test ne touche un vrai dossier protégé. La garde et le harness ne suppriment aucun résultat de passage antérieur : ils créent des sous-racines inédites.

## 3. Exécution future et table attendue

Après accord spécifique et candidate revue, lancer depuis `C:\dev\whisper`, sous la garde E0 et sans réseau :

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-adapters --test reprocessing_baseline --no-run
cargo test --offline --locked -p whisper-adapters --test reprocessing_baseline r11_application_baseline -- --ignored --exact --nocapture --test-threads=1
```

Au plus 18 imports mesurés (trois sources × trois parcours × deux passages), 10 min de calcul par import et trois heures de campagne au total ; dépassement ou premier échec : arrêter les cellules suivantes et les marquer NOT RUN. Ces bornes sont des limites d'exécution, **pas** des critères de rapidité du produit. Les préparations de fixtures ne sont pas incluses dans les fenêtres mesurées. Chaque ligne du tableau de sortie précise source, taille/durée, mode, passage, PID/opération, nombre et octets SHA de l'original côté adaptateur, durée SHA, nombre/durée des probes, scans, admission, confirmation, préparation, décodage/contrôles worker quand observables, temps total de la demande aux TXT/SRT, ainsi que part harness et attente humaine séparées. Deux passages successifs ne sont pas appelés « froid » et « chaud » sans maîtrise du cache. Un champ absent est NOT RUN, jamais extrapolé depuis R9 ou `Get-FileHash`.

Le test doit attester que les trois parcours ont réellement été empruntés, que les sorties sont écrites sous R, et que la mesure n'a supprimé aucun traitement utilisateur. Le nombre de lectures worker est rapporté distinctement des lectures adaptateur. Si les journaux du worker gelé ne ventilent pas ses durées, conserver NOT RUN ; ne pas ouvrir `whisper-worker-parakeet-cpu/src/main.rs` en cours de campagne. La suite workspace complète reste le gate du futur lot produit, pas un résultat présumé de ce mini-plan. Le candidat d'instrumentation passe à un reviewer indépendant avant usage ; deux cycles au plus.

## 4. État

**DRAFT, E0.1–E0.3 NOT RUN.** La campagne sans code a été autorisée mais s'est arrêtée à son préflight. Un nouvel accord écrit doit viser précisément les trois chemins Rust ci-dessus, la copie de garde et ses sorties sous R, le Cargo partagé, les trois sources et les bornes d'exécution. Sans cet accord, la référence R11 reste NOT RUN et le plan produit reste CLEAN documentaire mais non PLANS READY. Aucun code de mesure, build, import ou push n'est autorisé par ce document.
