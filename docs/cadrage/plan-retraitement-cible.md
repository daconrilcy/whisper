Avancée : RTC-01 v3 relu CLEAN, registre global implémenté ; tests ciblés et build PASS sous garde.
Blocage : le gate workspace crée des dossiers historiques `target/direct-live-*` hors des racines RTC-01.
Décision : faire relire l'amendement v4 limité aux chemins de sortie de trois fichiers de tests avant la reprise.

# PLAN-RETRAITEMENT-CIBLE v4 — AMENDEMENT DU GATE RTC-01 À RELIRE

Base : [mandat](mandat-retraitement-cible.md), [design](design-retraitement-cible.md) accepté par l'utilisateur le 20/10/2026 et revu CLEAN au commit `25075b44f67a36084bd3d6e85cedcd42e69954d5`. Auteur documentaire : session Codex principale ; proposition technique en lecture seule de `rust_plan_writer`. HEAD de départ du plan : `25075b44`. **Ce plan ne donne aucune autorisation de code, de suppression de données utilisateur, d'installation ou de push.** Chaque lot demande ensuite un accord écrit précis sur ses tâches et chemins. L'unsafe Windows de T6 exige un accord technique distinct ; l'acceptation du design ne le donne pas.

Le plan v2 a été relu CLEAN puis accepté pour RTC-01 le 20/10/2026 avec un plafond de deux snapshots. En cours d'implémentation, `QueueStore::open` (`crates/whisper-adapters/src/queue_store.rs:107-108`) et les sidecars de source ont révélé que les opérations sont rangées par destination. `ImportRuntime` (`worker_ipc.rs:1288, 1843-1858`) ne reçoit aucune racine permettant d'énumérer toutes les destinations au redémarrage. Le candidat partiel est préservé sans commit ; `cargo fmt --all -- --check` a échoué sur une accolade de `worker_ipc.rs:2705` et le format d'un test ; aucun test Cargo n'a encore tourné. L'utilisateur a accepté le 20/10/2026 un **registre central durable** et l'ouverture du seul chemin supplémentaire `crates/whisper-desktop/src/root.rs`, sous réserve de cette revue avant reprise du code. Le présent amendement ne touche ni RTC-02 ni la frontière Windows T6.

L'amendement v3 du registre central a été relu CLEAN et commité en `d08ef6e8`. Sur le candidat v3 non commité, la garde RTC-01 a exécuté `Fmt` PASS, `SourceBinding` 6/6 PASS, `Create` 1/1 PASS, `CoreLib` 34/34 PASS et `BuildWorkspace` PASS. La suite workspace a été arrêtée le 20/10/2026 à 17:44:21 (heure de Paris) sur la création de `target/direct-live-auto-startup-15304`, attribuée à `crates/whisper-adapters/src/direct_live.rs:1175-1276`. Le document [tolérance des écritures historiques](tolerance-ecritures-tests.md) identifie aussi `target/direct-live-*`, `target/direct-live-tests/` et `target/gpu-selection-tests/` comme sorties de tests existants. **Aucune tolérance d'écriture hors racine n'est ajoutée pour RTC-01.** L'utilisateur a autorisé le 20/10/2026 l'ouverture de trois chemins de tests, sous réserve de revue de ce delta, pour les rediriger sous la racine du lot. Le dossier observé n'a été ni effacé ni déplacé.

## 1. État de la mesure de référence et préflight E0

L'utilisateur a autorisé le 20/10/2026 une campagne isolée Parakeet CPU sur `C:\dev\whisper\sound example\spkr0.wav` (9 600 044 octets), `C:\dev\whisper\target\parakeet-tests\l2\l2b\r5-2\fixtures\spkr0_x6.wav` (présence observée, taille à relever au lancement) et `C:\Users\cyril\Downloads\kamhunt-old-school-preacher-clips-3-pastor-darryl-hussey-victorious-129623.mp3` (484 832 octets). Sources en lecture seule ; écritures exclusivement sous `target/parakeet-tests/retraitement-cible/mesure-avant/` et `target/parakeet-tests/cargo-shared/`, garde à échec fermé, aucun ancien traitement réel supprimé, aucun réseau ni code produit modifié. L'accord dit d'arrêter avant toute instrumentation si les journaux existants sont insuffisants.

Le préflight en lecture seule a constaté : aucun événement `archive_sha256`/`archive_audio_probe` sous `target/parakeet-tests/r11/` ; la recette `l6_personal_en5_twice_release` (`parakeet_import.rs:229`) ne couvre que l'import WAV et son helper impose RIFF/WAVE (`:2103-2109`) ; le test MP3 R11 (`import_mp3.rs:12`) ne lance pas le worker Parakeet CPU ; les recettes R11 de doublon utilisent des fixtures de test, pas les trois sources de E0. Les traces R9 historiques ne couvrent pas le binaire/parcours R11. Le [mini-plan E0](plan-mesure-retraitement-e0.md) a ensuite borné l'instrumentation autorisée par l'utilisateur le 20/10/2026. Le candidat E0 `7b2b5666` a été revu CLEAN au deuxième cycle et la [campagne E0](bilan-mesure-retraitement-e0.md) a exécuté 18 imports applicatifs Parakeet CPU, 0 FAIL : **E0 mesurée**. Les lectures natives internes et certaines phases sans identifiant restent NOT RUN par cellule ; le bilan les isole explicitement. Aucun fichier utilisateur n'a été modifié. La référence observée confirme 4 SHA de l'original pour un import initial et 6 pour Créer ou Remplacer, sans prétendre mesurer un gain du futur candidat.

Le mini-plan E0 a borné les trois chemins source, les commandes, la garde et ses auto-tests, les trois sources, le moteur/profil, les racines d'écriture et les scénarios sans doublon/Créer/Remplacer. Le bilan distingue les phases attribuables par import des phases agrégées ou NOT RUN. S'il faut écrire un fichier hors des chemins autorisés, `PLAN_CHANGE_REQUIRED` avant édition. La comparaison avant/après ne pourra être déclarée PASS qu'avec la même matrice réellement exécutée. Aucun seuil de temps n'est inventé ; le gain en lectures ne vaut pas une baisse prouvée du temps total.

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
| T2 Source liée | Original → snapshot unique copié et hashé dans un seul flux, scellé et protégé pendant consommation ; probe sur snapshot ; admission/confirmation/préparation réutilisent le binding. Copie impossible : erreur attribuée, anciens intacts. **Maximum deux snapshots simultanément retenus par le service d'import de l'application, toutes destinations confondues : un actif et un en attente.** Une troisième demande nécessitant un snapshot est refusée avant pending avec un message indiquant de terminer ou annuler un traitement puis de réessayer ; aucun import accepté n'est silencieusement abandonné. Vérifier aussi l'espace avant copie. À la reprise, reconstruire le compte depuis le registre durable central et les opérations/snapshots associés **avant** toute nouvelle admission ; les pending historiques sans snapshot sont préservés et ne sont rescellés que lorsqu'un emplacement est libre et que leur source est revérifiée. Un snapshot possédé est retiré après fin ou annulation vérifiée ; un orphelin incertain reste signalé et ne libère pas artificiellement un emplacement. | C01, C03, C04, C13 |
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
crates/whisper-desktop/src/root.rs (ajout v3 : injection du registre central uniquement)
crates/whisper-adapters/src/direct_live.rs (ajout v4 : chemins des tests uniquement)
crates/whisper-adapters/src/gpu_inventory.rs (ajout v4 : chemins des tests uniquement)
crates/whisper-adapters/tests/gpu_settings.rs (ajout v4 : chemins des tests uniquement)
```

**Amendement RTC-01/gate v4.** Dans les trois nouveaux chemins, seules les fonctions/tests `#[cfg(test)]` ou fichiers de tests sont éditables. Remplacer leurs destinations `../../target/direct-live-*`, `../../target/direct-live-tests` et `../../target/gpu-selection-tests` par des sous-dossiers nommés de `std::env::temp_dir()` ; la garde fixe `TEMP/TMP` canoniquement sous `target/parakeet-tests/retraitement-cible/rtc-01/`. Conserver les noms distinctifs et la séparation entre tests concurrents. Ne modifier ni logique produit, ni assertions, ni statut `#[ignore]`. Un test relancé sans TEMP/TMP conforme au lot ne doit pas ouvrir un chemin de sortie extérieur pendant la campagne ; la garde refuse avant Cargo. Les sorties historiques déjà présentes restent intactes. La revue du candidat vérifie le diff exact de ces trois chemins. Tout autre test écrivant hors racines arrête la campagne et demande un nouvel arbitrage, sans élargir silencieusement la tolérance.

**Amendement RTC-01/T2, registre des deux slots.** `root.rs` résout la racine persistante des réglages de l'application et injecte à `AsyncImportIo` un chemin dédié, par exemple `%LOCALAPPDATA%\Whisper\settings\reprocessing-snapshots.jsonl`. Le code produit n'utilise ni le dossier courant ni une variable de campagne comme repli si cette racine manque : nouvelle admission refusée avec cause explicite. Les constructeurs/tests peuvent injecter une racine isolée sous `target/parakeet-tests/retraitement-cible/rtc-01/`; la campagne ne touche jamais les réglages personnels. Le registre central associe chaque slot réservé à l'opération, à la destination canonique et au binding ; une réservation est durable **avant** la copie. La réservation, la libération et la récupération sont sérialisées pour éviter qu'un autre import ou processus dépasse deux slots. Après panne, toute entrée/snapshot ambigu est compté occupé et signalé ; il n'est pas effacé ni réattribué implicitement. Un slot ne se libère qu'après fin/annulation et retrait vérifié du snapshot. Une écriture interrompue du registre échoue en mode fermé. Aucun scan arbitraire de dossiers utilisateur, aucune modification de `root.rs` hors de la résolution/injection du registre et de ses tests directement liés. Les dix chemins v2 et les trois chemins de tests v4 sont les seuls autres chemins d'écriture RTC-01.

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

RTC-03 n'ajoute pas de chemin produit : une correction reste dans les chemins RTC-01/02 explicitement rouverts par l'accord sur la correction. Les chemins documentaires de plan/bilan, si autorisés, sont sous `docs/cadrage/`. Hormis `root.rs` et les trois chemins de tests RTC-01 explicitement listés ci-dessus, aucun worker, autre test historique, modèle, DLL, `sound example/`, installation personnelle ni destination réelle n'est ouvert par ce plan.

## 4. Frontière Windows proposée pour T6

Le module privé `crates/whisper-adapters/src/windows_file_identity.rs` serait le seul propriétaire de l'unsafe nouveau. `windows = 0.62.2` existe ; T6 propose sa feature `Win32_Storage_FileSystem` dans le manifeste adaptateur et `Cargo.lock` seulement si nécessaire. L'exception à `unsafe_code = deny` serait locale à ce module. Ouverture par `std::os::windows::fs::OpenOptionsExt` ; appels natifs limités à `GetFileInformationByHandleEx` (`FileIdInfo`, `FileAttributeTagInfo`) et `SetFileInformationByHandle` (`FileDispositionInfo`). Le handle `File` reste possédé par Rust, sans transfert ni double fermeture. Aucun `DeleteFileW`, disposition POSIX, suppression par chemin après vérification, `remove_dir_all`, C++, worker modifié ou nouvelle DLL. Toute API supplémentaire demande un nouveau plan et accord. La décision d'autoriser ce périmètre technique appartient à l'utilisateur avant T6 ; **elle n'est pas prise par l'acceptation du design ou du plan.**

## 5. Cartes d'acceptation

Toutes les cartes sont **NOT RUN** à ce stade. Une variante non exercée reste explicitement NOT RUN même si une autre variante de la carte passe.

| Carte | Fixture/scénario | Preuve exigée |
| --- | --- | --- |
| C01 | WAV copié/renommé, MP3 aux octets différents, source illisible. | Identité juste ; snapshot réellement consommé et lié au SHA ; aucun ancien audio requis. |
| C02 | Ancien intact/absent/corrompu, deux Créer. | Deux dossiers neufs, chacun une paire interne finale ; aucun nouvel export et anciens inchangés. |
| C03 | Source modifiée/renommée pendant copie ou worker, même taille/date restaurées. | Mutation empêchée ou erreur attribuée ; résultat lié au snapshot ; zéro suppression ancienne. |
| C04 | Erreur d'espace/copie, annulation, collision, arrêt avant succès ; deux imports vers **deux destinations distinctes** avec snapshots actifs/en attente puis troisième demande ; redémarrage avec les deux snapshots durables et admission tentée vers une **troisième destination**. | Pas de publication partielle annoncée ni suppression ancienne. Troisième demande refusée **avant pending** avec cause visible, deux premiers préservés. Après redémarrage, compte global reconstruit depuis le registre ; aucun troisième snapshot créé avant libération vérifiée d'un emplacement. Entrée du registre incomplète ou snapshot orphelin : slot conservé et cause visible. |
| C05 | Plusieurs succès, pending/échec plus récents, R11 inactif, ordre historique inconnu, nouveau concurrent. | Dernier prouvé ; Tous inclut actifs et inactifs vérifiés présents ; historique ambigu demande choix ; cibles figées. |
| C06 | Parcourir valide absent de liste, autre audio, tiers, hors destination, chemin redirigé. | Seul dossier vérifié accepté avant traitement ; aucun droit de suppression par chemin brut. |
| C07 | Substitution d'ancien avant/après ouverture, successeur alias ou modifié, ancêtre renommé. | Identité divergente refusée ; disposition sur handle vérifié ; successeur protégé pendant passe. |
| C08 | Original changé juste avant nettoyage, successeur incomplet, index divergent. | Nouveau conservé ; zéro ancien supprimé et cause visible. |
| C09 | Export historique possédé/non possédé, homonyme tiers, R11 déplacé, fichier inconnu dans dossier. | Seuls objets possédés retirés ; autres nommés parmi restants ; dossier non vide préservé. |
| C10 | Trois cibles, échec sur deuxième, arrêt avant/après une suppression. | Aucune troisième suppression après erreur ; progression durable et précise. |
| C11 | Interruption à chaque frontière de publication/nettoyage, reprise répétée. | Aucun second successeur, aucune cible ajoutée, nouvelle vérification avant reprise. |
| C12 | Dialogue fermé, confirmation répétée, réponse ancienne, réouverture. | Annulation avant traitement ; réponse périmée ignorée ; reprise destructive demandée à l'utilisateur. |
| C13 | E0 puis candidat, mêmes WAV/MP3 et parcours, deux passages. | Tableau par opération/PID : nombre/octet/temps SHA, probe, scan, copie, worker, temps total et attente humaine distincte ; pas de seuil de temps fictif. |
| C14 | Contrats moteur, micro, file/reprise, limite 30 min, suite workspace. | Préférences/refus inchangés, liste d'échecs et de NOT RUN ; test pur ≠ recette native. Les tests historiques `direct_live` et GPU continuent de tourner avec les mêmes assertions et écrivent seulement sous TEMP/TMP RTC-01. |
| C15 | Candidat Git et paquet de recette. | Revue indépendante ≤2 cycles ; validation de la fenêtre Windows réservée à l'utilisateur. |

## 6. Préflight, garde et vérifications futures

Avant chaque lot : relever `git rev-parse HEAD`, `git status --short`, diff, plan accepté et accord précis ; `Get-PSDrive C` doit montrer au moins 30 Gio libres. Vérifier les chemins canoniques des ressources lues. `models/` et `sound example/` non suivis restent intacts. `CARGO_TARGET_DIR` vaut exclusivement `C:\dev\whisper\target\parakeet-tests\cargo-shared\`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only` ; `CARGO_HOME` est préparé depuis le cache hors ligne existant puis fixé sous la racine d'exécution du lot, sans écriture dans le cache utilisateur. `CARGO_HOME`, TEMP/TMP, sorties, archive et journaux sont vérifiés canoniquement sous cette racine avant Cargo. Les auto-tests refusent une variable absente, un chemin extérieur et une redirection sortante pour chacune des racines d'écriture. Toute variable d'écriture extérieure ou lien sortant fait échouer la garde **avant Cargo**. Les sources originales, modèles, workers, DLL et installations personnelles sont protégés en lecture.

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
| Limite des copies simultanées | Décision utilisateur du 20/10/2026 : limiter et refuser clairement à saturation ; proposition du plan = 2 par service d'import | À accepter explicitement avec le plan ; C04 couvre saturation et reprise. |
| E0 mesure R11 existante | PASS 18/18 imports ; ventilation native par cellule PARTIAL/NOT RUN, signalée dans le bilan | Réutiliser la même matrice au lot RTC-03 sans extrapoler les champs absents. |
| Présent plan | v2 CLEAN ; v3 registre central CLEAN et commité `d08ef6e8` ; v4 gate test soumis à revue ciblée | Reprise du gate seulement si v4 CLEAN. |
| Accord RTC-01 | DONNÉ le 20/10/2026 pour les dix chemins v2 et deux snapshots ; compléments sur `root.rs` et les trois chemins de tests v4 | T1–T4 autorisés, sous réserve du verdict v4 pour la reprise du gate. |
| Accord RTC-02 et frontière Windows T6 | NON DONNÉS | Nécessaires après RTC-01 et avant T5–T8. |
| Validation automatique et fenêtre Windows | NOT RUN | Après candidats, par rôles prévus. |

La revue ciblée v4 vérifie l'exactitude des trois nouveaux chemins, leur seul code de test, le confinement de leurs sorties, l'absence d'affaiblissement des assertions et l'absence d'extension RTC-02. Au maximum deux cycles revue/correction pour cet amendement ; au-delà, décision de l'utilisateur avec réserves. **PLANS READY v4 n'est pas revendiqué avant son verdict.**
