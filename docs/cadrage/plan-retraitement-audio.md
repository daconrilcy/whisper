Avancée : plan de retraitement d'un audio déjà publié, proposé pour revue et acceptation.
Blocage : la cause exacte du refus observé après « Importer et transcrire » doit être établie avant le code de remplacement.
Décision : aucune implémentation ni publication n'est autorisée par ce plan seul.

# PLAN-RETRAITEMENT-AUDIO v1 — Import déjà publié

Base lue : `evo-startup-perf` à `bb0cb741`, le 19/10/2026. Statut : **DRAFT — revue indépendante à faire**. Auteur documentaire : session Codex principale. Le résultat de la qualification du besoin a été relu `PASS` ; cette relecture ne vaut pas revue du présent plan.

## 1. Besoin et décisions

Après « Importer et transcrire », un fichier déjà traité doit donner un choix compréhensible : **Annuler**, **Remplacer**, **Créer un nouveau**. Le doublon est un fichier de contenu identique dans la destination choisie, même si le chemin ou le nom du fichier source change. « Remplacer » vise **toutes** les transcriptions de ce contenu dans cette destination. Elles restent utilisables jusqu'à la publication complète du nouveau traitement et restent intactes si celui-ci échoue ou est annulé. « Créer un nouveau » garde les anciennes publications et crée un nom numéroté unique, commun aux sorties TXT et SRT. Ces décisions proviennent des réponses de l'utilisateur dans cette session du 19/10/2026.

**Interprétation technique proposée à accepter avec ce plan :** « même contenu » signifie mêmes octets du fichier audio, attestés par le SHA-256 de source déjà conservé. Une conversion WAV↔MP3 ou un réencodage peut donner un autre SHA ; l'identité acoustique après décodage n'est pas incluse dans ce lot. La comparaison est limitée aux publications vérifiées de la destination choisie. Les pending non publiés suivent leur reprise actuelle et ne sont pas présentés comme des doublons terminés.

**Nom proposé à accepter avec ce plan :** pour une nouvelle publication, un nom de base assaini tiré du fichier source, puis le plus petit suffixe libre ` (2)`, ` (3)`, etc. Une paire lisible `<nom>.txt` et `<nom>.srt` est placée sous `destination/exports/`, en plus des artefacts internes vérifiables `destination/transcriptions/<job_id>/<generation>/transcript.txt` et `.srt`. Le même nom et les chemins réels apparaissent dans l'historique. Un nom déjà utilisé par une paire ou un fichier tiers n'est jamais écrasé. Pour les publications antérieures sans paire lisible, leurs artefacts internes restent la référence ; aucune migration globale n'est requise. Les noms sont déterminés à la confirmation puis réservés durablement pour prévenir deux imports concurrents.

**Sens de remplacer proposé à accepter avec ce plan :** une nouvelle publication complète devient l'unique publication active de ce contenu. Toutes les publications précédentes sont retirées de l'historique actif dans une seule transition durable, puis conservées comme archives remplacées consultables pour retour arrière, sans suppression automatique. Leurs générations internes restent dans `transcriptions/` pour préserver les preuves et les reprises ; un registre sous `destination/.whisper-publications/` indique leur état remplacé. Les exports lisibles anciens ne sont plus présentés comme résultats actifs ; ceux créés par ce lot sont rangés sous `destination/transcriptions-remplacees/` après la transition. Les fichiers tiers et les autres contenus ne sont jamais touchés. Le registre est la source de vérité de visibilité ; un échec de rangement physique après la transition est signalé et réconcilié au redémarrage, sans réactiver les anciennes dans l'historique actif. Ces dossiers de gestion sont hors du parcours `ArchiveStore::scan` des générations.

Ces trois interprétations sont des choix du plan, **pas** des décisions déjà données mot pour mot par l'utilisateur. L'acceptation explicite du plan les fixe ; tout désaccord impose une v2 avant code.

## 2. État observé et diagnostic préalable

| Source | Fait observé | Conséquence pour le plan |
| --- | --- | --- |
| `crates/whisper-desktop/src/ui.rs:591-615` | Le bouton crée normalement un `JobId` neuf et une génération 1. | L'erreur rapportée après ce bouton n'est pas expliquée par une réutilisation évidente de cet ID dans l'UI. Ne pas remplacer aveuglément l'erreur d'archive par un dialogue. |
| `crates/whisper-adapters/src/worker_ipc.rs:1930-1950` | L'admission en file identifie et vérifie la source avant `QueueStore::enqueue`. | Le contrôle de doublon peut suivre cette identification dans le fil d'arrière-plan ; aucun SHA sur le fil UI. |
| `crates/whisper-adapters/src/archive.rs:1680-1695` | L'identité actuelle est le SHA-256 des octets du fichier et sa durée décodée. | Réutiliser cette identité et revalider à la confirmation. |
| `crates/whisper-adapters/src/archive.rs:1835-1858, 2876-2880` | L'archive est rangée sous `destination/transcriptions/<job_id>/<generation>` ; le message cité survient quand la **même génération** possède déjà `pending.json` et `CURRENT`. | Une génération publiée ne doit jamais être rouverte. Établir par reproduction et trace de l'ID comment ce cas survient après le bouton. |
| `crates/whisper-adapters/src/archive.rs:2124-2189, 2679-2752` | Publication par manifest vérifié puis `CURRENT`, scan vérifiant TXT/SRT et journal de segments. | Conserver ces invariants ; ajouter un registre de visibilité des remplacements, pas une réécriture sur place d'une génération publiée. |
| `crates/whisper-adapters/src/queue_store.rs:108-187` | File FIFO durable distincte de l'archive. | Aucun pending ni entrée en file avant le choix de l'utilisateur pour un doublon. |

Diagnostic R0 obligatoire, **lecture et test ciblé avant toute édition produit** : reproduire avec une destination temporaire sous la racine du lot, deux clics « Importer et transcrire » sur la même source et le parcours « Traiter », puis relever à chaque transition `job_id`, `generation`, statut de file et dossier d'archive. Distinguer l'erreur directe au clic de l'erreur différée lors du traitement de la file. Si la cause exige un fichier hors liste, une correction du worker natif ou un changement non additif du protocole, arrêter avec `PLAN_CHANGE_REQUIRED`. Ce diagnostic ne doit pas écraser ni modifier la publication réelle de l'utilisateur.

## 3. Contrat de parcours

1. L'UI envoie une demande de préadmission asynchrone. L'adaptateur vérifie le fichier, calcule son SHA-256 et lit uniquement les publications complètes et vérifiées de la destination. Si aucune n'existe, l'admission normale en file continue. Une archive corrompue ou inaccessible donne une erreur explicite, jamais un faux « aucun doublon ».
2. Si une ou plusieurs publications actives portent le même SHA, l'application rend un dialogue modal indiquant le nombre de traitements existants et les trois issues. À ce stade, aucun nouveau pending, worker, publication ou export n'existe. Annuler efface seulement l'intention transitoire.
3. La confirmation passe un choix typé au cœur de l'application. L'adaptateur recalcule/revalide l'identité du fichier, la destination, l'ensemble des publications correspondantes et le nom réservé **avant** l'enqueue. Si les données ont changé, l'application redemande le choix avec l'état courant ; elle ne transforme pas le choix en une autre action.
4. « Créer un nouveau » utilise un nouveau `JobId`, préserve toutes les publications actives et attribue un nom lisible libre pour la paire TXT/SRT. « Remplacer » utilise également un nouveau `JobId` ; les anciennes restent actives pendant l'inférence et toute reprise éventuelle. Aucun mode n'écrit dans une génération dont `CURRENT` existe.
5. Après vérification complète du nouveau manifest et des exports, une transition durable unique rend le nouveau résultat actif et, pour « Remplacer », masque toutes les anciennes publications de ce contenu. Le registre couvre les publications historiques sans registre (lecture compatible) et permet une réconciliation idempotente après arrêt brutal. Une erreur avant cette transition laisse les anciennes actives. Les sorties partielles du nouveau traitement sont diagnostiquées, pas affichées comme une réussite.
6. La file FIFO, les reprises interrompues, l'admission des moteurs Turbo/Parakeet, le calcul CPU/GPU/Auto, les limites de durée et le micro ne changent pas de règle. Un dossier de sortie tiers n'est jamais supprimé ou remplacé ; une collision de nom est résolue par un autre suffixe ou par un refus explicite.

## 4. Critères d'acceptation

| Carte | Scénario et résultat observable |
| --- | --- |
| D-A1 | Deux imports d'un même WAV/MP3 au même chemin ou d'une copie renommée, même destination : dialogue avant file/pending lors du second clic ; un fichier différent ou une autre destination suit le parcours normal. Le diagnostic R0 documente séparément la cause du message initial. |
| D-A2 | Annuler : aucune nouvelle entrée en file, aucun pending/worker/export ; toutes les publications précédentes restent visibles et vérifiables. |
| D-A3 | Créer un nouveau deux fois : trois résultats actifs distincts ; chaque nouvelle paire TXT/SRT a le même nom de base avec suffixe libre et aucun fichier existant n'est écrasé. Le nom est visible dans l'historique. |
| D-A4 | Remplacer après plusieurs publications du même contenu : pendant le nouveau traitement, toutes les anciennes restent utilisables ; après succès, une seule nouvelle publication active, ses TXT/SRT et manifest vérifiés ; toutes les anciennes restent disponibles en sauvegarde interne. Un autre contenu et une autre destination restent inchangés. |
| D-A5 | Échec, annulation, fermeture avant commit, puis redémarrage : anciennes publications intactes et actives ; aucun résultat partiel présenté comme complet. Interruption après commit : visibilité et sauvegarde réconciliées une seule fois. |
| D-A6 | Source modifiée, archive illisible, collision créée entre dialogue et confirmation, confirmation répétée ou double clic : refus/reconfirmation explicite, sans erreur brute `generation is already published`, sans perte ni double publication. |
| D-A7 | File FIFO/reprise de pending existant, Turbo/Parakeet CPU/GPU/Auto, seuil Parakeet 30 min et micro : règles et refus existants conservés. Pas de bascule silencieuse de modèle ou backend. |
| D-A8 | Tests ciblés et suite workspace complète `--no-fail-fast` sous garde : 0 FAIL, tous les NOT RUN listés. Recette dans la fenêtre Windows avec un vrai audio déjà traité : dialogue, TXT/SRT nommés, remplacement et annulation observés par l'utilisateur. |

## 5. Lot unique R11 — tâches et chemins exacts

Le lot reste **non livrable** tant que D-A1 à D-A7 ne sont pas satisfaits ensemble. Huit tâches séquentielles au maximum ; chaque tâche est liée aux cartes ci-dessus. L'implémentation Rust, après accord précis, est déléguée à `rust_implementer` ; la revue du candidat à `rust_review_coordinator`, au plus deux cycles revue/correction.

| Tâche | Travail et preuve attendue | Chemins d'écriture proposés |
| --- | --- | --- |
| R11.1 | R0 : reproduire, tracer l'identité, conclure la cause sans toucher aux données réelles. | Exécution sous `target/parakeet-tests/r11/` uniquement. |
| R11.2 | Contrat typé de préadmission/choix et états de dialogue ; annulation et demande devenue obsolète. | `crates/whisper-core/src/ports.rs`, `crates/whisper-core/src/application.rs` |
| R11.3 | Recherche des publications vérifiées par SHA dans la destination, prise en compte des legacy et erreur sur archive corrompue ; aucun effet de bord sur pending. | `crates/whisper-adapters/src/archive.rs` |
| R11.4 | Deuxième validation sous verrou avant enqueue, réservation durable de nom/choix, respect FIFO ; aucune réouverture de génération publiée. | `crates/whisper-adapters/src/worker_ipc.rs`, `crates/whisper-adapters/src/queue_store.rs`, `crates/whisper-adapters/src/archive.rs` |
| R11.5 | Exports TXT/SRT nommés, manifest vérifié, registre de visibilité et remplacement de toutes les anciennes de manière récupérable ; migration en lecture des publications antérieures. | `crates/whisper-adapters/src/archive.rs` |
| R11.6 | Dialogue « déjà traité » et nom/historique lisibles ; pas de SHA sur le fil UI. | `crates/whisper-desktop/src/ui.rs`, `crates/whisper-core/src/ports.rs`, `crates/whisper-core/src/application.rs` |
| R11.7 | Tests déterministes : même chemin, copie renommée, autre destination, plusieurs anciennes, trois choix, conflits, coupures avant/après commit, reprise, double confirmation, non-régression Turbo/Parakeet. | Tests intégrés aux six fichiers source ci-dessus ; `crates/whisper-adapters/tests/import_cpu.rs`, `crates/whisper-adapters/tests/import_mp3.rs` si les fixtures existantes suffisent. |
| R11.8 | Gate sous garde, revue indépendante, rapport et bilan des NOT RUN ; recette Windows remise à l'utilisateur. | `docs/cadrage/bilan-retraitement-audio.md` seulement après candidat et revue. |

**Liste fermée de code/test proposé :** `crates/whisper-core/src/ports.rs`, `crates/whisper-core/src/application.rs`, `crates/whisper-adapters/src/archive.rs`, `crates/whisper-adapters/src/worker_ipc.rs`, `crates/whisper-adapters/src/queue_store.rs`, `crates/whisper-adapters/tests/import_cpu.rs`, `crates/whisper-adapters/tests/import_mp3.rs`, `crates/whisper-desktop/src/ui.rs`. Aucun `Cargo.toml`, `Cargo.lock`, code C++, worker natif, modèle, DLL, installateur ou script d'installation n'est ouvert. Si une vérification exige un autre chemin, arrêter avec `PLAN_CHANGE_REQUIRED` avant édition.

## 6. Préflight, campagne et limites d'autorisation

Avant toute implémentation : vérifier HEAD, état Git, plan accepté et accord écrit sur **R11 et sa liste fermée**, espace libre ≥30 Gio, source audio et modèle locaux, ainsi que la garde. Ne jamais utiliser les données de transcription de l'utilisateur comme fixture modifiable. L'exécution écrit uniquement sous `target/parakeet-tests/r11/` et le `CARGO_TARGET_DIR` partagé `target/parakeet-tests/cargo-shared/` ; `TEMP`/`TMP` sont sous R11, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. Reprendre la garde à échec fermé et ses auto-tests existants sous une copie R11 hors Git ; elle vérifie racines canoniques et SHA des ressources protégées avant/après chaque commande, surveille les écritures hors tolérance et propage le code de sortie Cargo. Un besoin de modifier une racine historique ou le script hors du périmètre d'exécution R11 est `PLAN_CHANGE_REQUIRED`.

Commandes proposées, depuis `C:\dev\whisper` en PowerShell, **uniquement après garde réussie et accord d'implémentation** :

```powershell
cargo fmt --all -- --check
cargo build --offline --locked --workspace
cargo test --offline --locked -p whisper-core --lib
cargo test --offline --locked -p whisper-adapters --lib
cargo test --offline --locked -p whisper-adapters --test import_cpu
cargo test --offline --locked -p whisper-adapters --test import_mp3
cargo test --offline --locked --workspace --no-fail-fast
```

Chaque commande Cargo passe par la garde R11 et ses variables de campagne requises ; un code non nul, une écriture protégée ou un changement de SHA est FAIL. Le rapport distingue les vérifications automatisées de la recette dans la fenêtre Windows, réservée à l'utilisateur. Les tests dépendant de fixtures absentes sont NOT RUN avec motif ; une carte nécessitant cette fixture reste non satisfaite. À la clôture : archiver les seules petites preuves prévues, vérifier nombre/SHA, ne retirer que les copies/WAV temporaires d'un manifeste approuvé, relever l'espace libre. Aucun nettoyage de `cargo-shared` ni des données réelles.

**Hors périmètre :** détection d'équivalence acoustique entre encodages différents, micro/live, VAD longue, acquisition réseau, installation dans `%LOCALAPPDATA%\Whisper`, push. Écrire les nouveaux TXT/SRT dans la destination choisie et y conserver les archives remplacées fera partie de l'accord d'implémentation R11 à solliciter ; la rédaction du présent plan n'accorde pas cette écriture hors dépôt.

## 7. Readiness

Statut : **DRAFT / NOT IMPLEMENTATION READY** jusqu'à revue indépendante du plan puis acceptation explicite de ses trois interprétations proposées, de R11 et de ses chemins. Tests produit : **NOT RUN**. La recette Windows n'est pas remplacée par un test Rust vert.
