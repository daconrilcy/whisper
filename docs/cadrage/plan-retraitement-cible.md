Avancée : plan DRAFT en dix tâches pour la création indépendante, le remplacement ciblé et leur validation.
Blocage : la mesure R11 actuelle est NOT RUN ; les journaux et recettes existants ne couvrent pas la matrice autorisée, donc aucune instrumentation n'a été lancée.
Décision : faire relire ce plan et borner une instrumentation de référence avant PLANS READY et avant tout code produit.

# PLAN-RETRAITEMENT-CIBLE v1 — DRAFT

Base : [mandat](mandat-retraitement-cible.md), [design](design-retraitement-cible.md) accepté par l'utilisateur le 20/10/2026 et revu CLEAN au commit `25075b44f67a36084bd3d6e85cedcd42e69954d5`. Auteur documentaire : session Codex principale ; proposition technique en lecture seule de `rust_plan_writer`. HEAD de départ du plan : `25075b44`. **Ce plan ne donne aucune autorisation de code, de suppression de données utilisateur, d'installation ou de push.** Chaque lot demande ensuite un accord écrit précis sur ses tâches et chemins. L'unsafe Windows de T6 exige un accord technique distinct ; l'acceptation du design ne le donne pas.

## 1. État de la mesure de référence et préflight E0

L'utilisateur a autorisé le 20/10/2026 une campagne isolée Parakeet CPU sur `C:\dev\whisper\sound example\spkr0.wav` (9 600 044 octets), `C:\dev\whisper\target\parakeet-tests\l2\l2b\r5-2\fixtures\spkr0_x6.wav` (présence observée, taille à relever au lancement) et `C:\Users\cyril\Downloads\kamhunt-old-school-preacher-clips-3-pastor-darryl-hussey-victorious-129623.mp3` (484 832 octets). Sources en lecture seule ; écritures exclusivement sous `target/parakeet-tests/retraitement-cible/mesure-avant/` et `target/parakeet-tests/cargo-shared/`, garde à échec fermé, aucun ancien traitement réel supprimé, aucun réseau ni code produit modifié. L'accord dit d'arrêter avant toute instrumentation si les journaux existants sont insuffisants.

Le préflight en lecture seule a constaté : aucun événement `archive_sha256`/`archive_audio_probe` sous `target/parakeet-tests/r11/` ; la recette `l6_personal_en5_twice_release` (`parakeet_import.rs:229`) ne couvre que l'import WAV et son helper impose RIFF/WAVE (`:2103-2109`) ; le test MP3 R11 (`import_mp3.rs:12`) ne lance pas le worker Parakeet CPU ; les recettes R11 de doublon utilisent des fixtures de test, pas les trois sources de E0. Les traces R9 historiques ne couvrent pas le binaire/parcours R11. **E0 : NOT RUN, arrêt avant adaptation de la garde, création de harness, Cargo ou transcription.** Aucun fichier utilisateur n'a été modifié. Le comptage statique quatre/six SHA de l'adaptateur, documenté dans le design, ne remplace pas la mesure.

Un amendement de mesure à faire autoriser avant PLANS READY devra lister les fichiers source d'instrumentation ou de harness strictement nécessaires, la commande exacte, la garde adaptée et ses auto-tests, les trois sources ci-dessus et leurs SHA au préflight, le moteur/profil, les racines d'écriture, la ventilation admission/confirmation/préparation/probe/scan/worker/temps total, ainsi que les scénarios sans doublon/Créer/Remplacer. S'il faut écrire un fichier hors des chemins autorisés, `PLAN_CHANGE_REQUIRED` avant édition. La comparaison avant/après ne pourra être déclarée PASS qu'avec la même matrice réellement exécutée. Aucun seuil de temps n'est inventé ; le gain en lectures ne vaut pas une baisse prouvée du temps total.

## 2. Dépendances et trois lots séquentiels

Les lots partagent notamment `archive.rs`, `worker_ipc.rs`, `application.rs` et `ui.rs` : un seul propriétaire de code à la fois. L'auteur principal vérifie design, plan accepté en Git, accord courant, état Git et préflight avant de confier chaque lot à `rust_implementer` en mode IMPLEMENT ; le candidat passe ensuite à `rust_review_coordinator`, avec au plus deux cycles revue/correction. Un constat requis non fermé donne LIVRÉ AVEC RÉSERVES et arbitrage utilisateur. La recette réelle dans la fenêtre Windows est faite par l'utilisateur.

| Lot | Précondition | Résultat démontrable | Écriture d'exécution future |
| --- | --- | --- | --- |
| RTC-01 Création (T1–T4) | Plan accepté, E0 mesuré ou arbitrage explicite sur son absence, accord écrit RTC-01. | Source figée, import « Créer un nouveau » indépendant des anciens, paire unique interne, aucun nettoyage destructif. | `target/parakeet-tests/retraitement-cible/rtc-01/` |
| RTC-02 Remplacement (T5–T8) | RTC-01 revu, validation requise PASS, accord écrit RTC-02 **et** accord précis sur la frontière Windows T6. | Dernier/tous/choisi, suppression liée aux handles, partiel et reprise explicite. | `target/parakeet-tests/retraitement-cible/rtc-02/` |
| RTC-03 Validation (T9–T10) | Candidat RTC-02 revu et accord sur campagne finale. | Comparaison avant/après, suite complète, rapport des NOT RUN, recette Windows remise. | `target/parakeet-tests/retraitement-cible/rtc-03/` |

## 3. Tâches et chemins d'écriture proposés

Les tâches ci-dessous sont au nombre de **dix** au total. Le design RTC-01 à RTC-11 et ses risques RTC-R1 à R6 sont la référence de comportement. Un chemin absent de la liste du lot ou un worker à modifier impose `PLAN_CHANGE_REQUIRED`, avant toute édition.

| Tâche | Entrée → sortie vérifiable | Carte |
| --- | --- | --- |
| T1 Contrats purs | Commandes/états R11 → identités audio/traitement/opération, modes Créer/Dernier/Tous/Choisi, sélection figée, erreurs et progression ; aucun accès disque dans core. | C01, C05, C12 |
| T2 Source liée | Original → snapshot unique copié et hashé dans un seul flux, scellé et protégé pendant consommation ; probe sur snapshot ; admission/confirmation/préparation réutilisent le binding. Copie impossible : erreur attribuée, anciens intacts. Un snapshot par import accepté, nettoyage sur annulation ou fin ; vérifier l'espace avant copie et refuser explicitement si insuffisant, sans modifier silencieusement la file. | C01, C03, C04, C13 |
| T3 Publication unique | Nouveau résultat → dossier inédit, TXT/SRT internes + manifeste vérifiés puis point de succès ; aucun nouveau `exports/` ni `export.*.pending`. Créer ne dépend pas de l'intégrité d'anciens dossiers. | C02, C04 |
| T4 Parcours Créer | UI/commande → traitement sur snapshot, paire lisible interne et résultat affiché ; annulation avant confirmation sans ancien effet ; tests du premier parcours complet. | C01–C04, C12 |
| T5 Catalogue et choix | Publications actives/inactives R11 → candidats vérifiés, ordre de succès monotone pour nouvelles publications, ordre historique inconnu visible ; Dernier/Tous/Parcourir résolus en identités et preuves figées. | C05, C06 |
| T6 Frontière Windows | Handle ouvert → FileId128/volume/attributs et disposition de suppression sur **le même handle**, sans suppression par chemin ; tests Windows de substitution et partage. Accord unsafe distinct préalable. | C07 |
| T7 Nettoyage | Nouveau publié + intention → verrou de destination, successeur protégé, SHA final original sous handle, cibles disjointes et possédées, suppression des seuls objets choisis. Échec initial = zéro suppression ; échec partiel = arrêt des suivantes, nouveau conservé, restants exacts. | C07–C10 |
| T8 Reprise/UI | Panne post-publication → nouveau visible, restants nommés, demande de reprise explicite ; revalidation de la sélection initiale sans l'élargir ni publier un second successeur. | C06, C10–C12 |
| T9 Mesure et non-régression | Candidat + E0 → lectures de l'original/snapshot/worker, probe, scan, copie et temps total comparés ; tests CPU/GPU/Auto, Turbo/Parakeet, file/reprise, micro/durée ; variantes non exercées NOT RUN. | C13, C14 |
| T10 Revue et remise | Diff/tests → deux cycles maximum, bilan de résultats, réserves, espace disque final et procédure de recette Windows ; aucun push/installation implicite. | C15 |

**RTC-01, chemins de code/tests proposés exclusivement :**

```text
crates/whisper-core/src/ports.rs
crates/whisper-core/src/application.rs
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/source_binding.rs
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/src/queue_store.rs
crates/whisper-adapters/tests/source_binding.rs
crates/whisper-adapters/tests/reprocessing_create.rs
crates/whisper-desktop/src/ui.rs
```

**RTC-02, chemins de code/tests proposés exclusivement :**

```text
crates/whisper-core/src/ports.rs
crates/whisper-core/src/application.rs
crates/whisper-adapters/Cargo.toml
Cargo.lock (seulement si Cargo le modifie réellement après accord)
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/windows_file_identity.rs
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/src/queue_store.rs
crates/whisper-adapters/tests/reprocessing_cleanup.rs
crates/whisper-desktop/src/ui.rs
```

RTC-03 n'ajoute pas de chemin produit : une correction reste dans les chemins RTC-01/02 explicitement rouverts par l'accord sur la correction. Les chemins documentaires de plan/bilan, si autorisés, sont sous `docs/cadrage/`. Aucun worker, `root.rs`, test historique, modèle, DLL, `sound example/`, installation personnelle ni destination réelle n'est ouvert par ce plan.

## 4. Frontière Windows proposée pour T6

Le module privé `crates/whisper-adapters/src/windows_file_identity.rs` serait le seul propriétaire de l'unsafe nouveau. `windows = 0.62.2` existe ; T6 propose sa feature `Win32_Storage_FileSystem` dans le manifeste adaptateur et `Cargo.lock` seulement si nécessaire. L'exception à `unsafe_code = deny` serait locale à ce module. Ouverture par `std::os::windows::fs::OpenOptionsExt` ; appels natifs limités à `GetFileInformationByHandleEx` (`FileIdInfo`, `FileAttributeTagInfo`) et `SetFileInformationByHandle` (`FileDispositionInfo`). Le handle `File` reste possédé par Rust, sans transfert ni double fermeture. Aucun `DeleteFileW`, disposition POSIX, suppression par chemin après vérification, `remove_dir_all`, C++, worker modifié ou nouvelle DLL. Toute API supplémentaire demande un nouveau plan et accord. La décision d'autoriser ce périmètre technique appartient à l'utilisateur avant T6 ; **elle n'est pas prise par l'acceptation du design ou du plan.**

## 5. Cartes d'acceptation

Toutes les cartes sont **NOT RUN** à ce stade. Une variante non exercée reste explicitement NOT RUN même si une autre variante de la carte passe.

| Carte | Fixture/scénario | Preuve exigée |
| --- | --- | --- |
| C01 | WAV copié/renommé, MP3 aux octets différents, source illisible. | Identité juste ; snapshot réellement consommé et lié au SHA ; aucun ancien audio requis. |
| C02 | Ancien intact/absent/corrompu, deux Créer. | Deux dossiers neufs, chacun une paire interne finale ; aucun nouvel export et anciens inchangés. |
| C03 | Source modifiée/renommée pendant copie ou worker, même taille/date restaurées. | Mutation empêchée ou erreur attribuée ; résultat lié au snapshot ; zéro suppression ancienne. |
| C04 | Erreur d'espace/copie, annulation, collision, arrêt avant succès. | Pas de publication partielle annoncée ni suppression ancienne. |
| C05 | Plusieurs succès, pending/échec plus récents, R11 inactif, ordre historique inconnu, nouveau concurrent. | Dernier prouvé ; Tous inclut actifs et inactifs vérifiés présents ; historique ambigu demande choix ; cibles figées. |
| C06 | Parcourir valide absent de liste, autre audio, tiers, hors destination, chemin redirigé. | Seul dossier vérifié accepté avant traitement ; aucun droit de suppression par chemin brut. |
| C07 | Substitution d'ancien avant/après ouverture, successeur alias ou modifié, ancêtre renommé. | Identité divergente refusée ; disposition sur handle vérifié ; successeur protégé pendant passe. |
| C08 | Original changé juste avant nettoyage, successeur incomplet, index divergent. | Nouveau conservé ; zéro ancien supprimé et cause visible. |
| C09 | Export historique possédé/non possédé, homonyme tiers, R11 déplacé, fichier inconnu dans dossier. | Seuls objets possédés retirés ; autres nommés parmi restants ; dossier non vide préservé. |
| C10 | Trois cibles, échec sur deuxième, arrêt avant/après une suppression. | Aucune troisième suppression après erreur ; progression durable et précise. |
| C11 | Interruption à chaque frontière de publication/nettoyage, reprise répétée. | Aucun second successeur, aucune cible ajoutée, nouvelle vérification avant reprise. |
| C12 | Dialogue fermé, confirmation répétée, réponse ancienne, réouverture. | Annulation avant traitement ; réponse périmée ignorée ; reprise destructive demandée à l'utilisateur. |
| C13 | E0 puis candidat, mêmes WAV/MP3 et parcours, deux passages. | Tableau par opération/PID : nombre/octet/temps SHA, probe, scan, copie, worker, temps total et attente humaine distincte ; pas de seuil de temps fictif. |
| C14 | Contrats moteur, micro, file/reprise, limite 30 min, suite workspace. | Préférences/refus inchangés, liste d'échecs et de NOT RUN ; test pur ≠ recette native. |
| C15 | Candidat Git et paquet de recette. | Revue indépendante ≤2 cycles ; validation de la fenêtre Windows réservée à l'utilisateur. |

## 6. Préflight, garde et vérifications futures

Avant chaque lot : relever `git rev-parse HEAD`, `git status --short`, diff, plan accepté et accord précis ; `Get-PSDrive C` doit montrer au moins 30 Gio libres. Vérifier les chemins canoniques des ressources lues. `models/` et `sound example/` non suivis restent intacts. `CARGO_TARGET_DIR` vaut exclusivement `C:\dev\whisper\target\parakeet-tests\cargo-shared\`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only` ; TEMP/TMP, sorties, archive et journaux restent sous la racine du lot. Toute variable d'écriture extérieure ou lien sortant fait échouer la garde **avant Cargo**. Les sources originales, modèles, workers, DLL et installations personnelles sont protégés en lecture.

La garde R11 dans `target/parakeet-tests/r11/guard-campaign.ps1` fixe `$lot=r11` et possède des modes autorisant des écritures personnelles : elle ne peut pas servir telle quelle. Une copie adaptée, sous la racine d'exécution du lot, doit être testée avant Cargo : mauvaise racine Cargo, TEMP extérieur, destination personnelle et lien sortant sont refusés ; les campagnes surveillent les chemins protégés et propagent les vrais codes de sortie. Cette adaptation est un prérequis d'exécution, pas une autorisation d'élargir les chemins de code. La garde des campagnes antérieures ne constitue pas une preuve de confinement de ce nouveau lot.

Commandes candidates uniquement après préflight, garde et accord du lot :

```powershell
cargo fmt --all -- --check
cargo build --offline --locked --workspace
cargo test --offline --locked -p whisper-core --lib
cargo test --offline --locked -p whisper-adapters --test source_binding -- --show-output
cargo test --offline --locked -p whisper-adapters --test reprocessing_create -- --show-output
cargo test --offline --locked -p whisper-adapters --test reprocessing_cleanup -- --show-output
cargo test --offline --locked --workspace --no-fail-fast -- --show-output
```

Les cibles de test nouvelles ne sont lancées qu'après leur création dans le lot autorisé. La suite workspace complète est obligatoire à la clôture de chaque lot, avec tous les FAIL et NOT RUN listés. La revue n'est pas remplacée par un exit code nul. À la clôture, archiver les petites preuves avec décompte et SHA concordants, supprimer seulement les copies et WAV temporaires nommés dans un manifeste approuvé, puis relever l'espace libre ; ne pas nettoyer `cargo-shared` ni une ressource active. Aucun nettoyage de données réelles n'est implicite.

## 7. Readiness et décisions à obtenir

| Décision/ preuve | État | Effet |
| --- | --- | --- |
| Design accepté et relu CLEAN | ACQUIS au `25075b44` | Base du plan. |
| E0 mesure R11 existante | NOT RUN : traces absentes et harness incomplet | Amendement borné de mesure, revue si nécessaire, puis accord spécifique avant instrumentation ; sinon arbitrage utilisateur explicite sur cette réserve. |
| Présent plan | DRAFT, revue indépendante à faire | Aucun code ouvert. |
| Accord RTC-01 | NON DONNÉ | Nécessaire avant T1–T4. |
| Accord RTC-02 et frontière Windows T6 | NON DONNÉS | Nécessaires après RTC-01 et avant T5–T8. |
| Validation automatique et fenêtre Windows | NOT RUN | Après candidats, par rôles prévus. |

Le premier retour de revue doit distinguer les constats requis par une exigence/acceptation/risque réel des conseils. Au maximum deux cycles de revue/correction pour ce plan ; au-delà, décision de l'utilisateur avec réserves. **PLANS READY n'est pas revendiqué.**
