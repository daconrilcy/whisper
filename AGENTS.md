# Whisper
Objectif produit : dans l'application Windows existante, transcrire un fichier WAV/MP3 ou la voix captée par le micro avec le modèle local large-v3-turbo et un choix Auto, CPU ou GPU ; afficher le texte micro confirmé par blocs et enregistrer TXT, SRT et WAV de même nom à l'arrêt. L'utilisateur peut consulter les entrées microphone Windows, choisir une source explicite ou le microphone Windows par défaut, et retrouver ce choix après fermeture. Si la source explicite est indisponible à la réouverture, l'application utilise le défaut Windows avec un message et conserve l'identifiant préféré pour une reconnexion ; si elle devient indisponible pendant la session, aucun autre microphone n'est choisi automatiquement. Auto vérifie avant chaque transcription si le moteur existant peut utiliser un GPU compatible, le privilégie si disponible et utilise sinon le CPU, sans changer la préférence Auto ; le moteur effectif est identifiable. Les choix manuels CPU/GPU restent disponibles et fixes pendant une session. La présélection existante reste GPU si CUDA est confirmé au démarrage, sinon CPU avec un message clair. Le GPU traite l'audio plus vite que sa durée et affiche le premier texte micro en moins de 15 secondes, avec une qualité comparable au CPU. Le CPU reste disponible.
Évolution de réactivité à cadrer : rendre l'interface utilisable rapidement, démarrer la capture microphone sans attendre inutilement le moteur, préserver sans perte l'audio annoncé comme capturé et réutiliser les composants déjà prêts lorsque cela reste compatible avec CPU, GPU, Auto, large-v3-turbo et la qualité actuelle. Les cibles proposées sont moins de 2 s pour l'interface et moins de 3 s pour la capture ; leur faisabilité et leurs compromis restent à établir dans `docs/cadrage/evo-startup-perf.md`. Ce cadrage n'autorise aucune implémentation.
Différés : bascule pendant une transcription, supervision CPU/GPU, tray, reprise après panne, changement de modèle, sortie MP3. En Auto, si le GPU est indisponible ou échoue à l'initialisation, une seule tentative CPU est acceptée avant le traitement audio, avec cause et moteur effectif affichés ; aucun repli après ce début ni en mode GPU manuel. Cette décision de cadrage n'autorise pas l'implémentation.
Évolution micro : livrée avec réserves, validée par l'utilisateur dans la fenêtre Windows ; bilan dans `docs/cadrage/bilan-evo-micro-direct.md`. Le cadrage initial reste dans `docs/cadrage/evo-micro-direct.md`.
Évolution GPU : livrée et validée par l'utilisateur dans la fenêtre Windows ; bilan dans `docs/cadrage/bilan-evo-gpu.md`. Le cadrage initial est dans `docs/cadrage/evo-gpu.md`.
Évolution Auto : décision de repli acceptée le 14 octobre 2026, inventaire et plan dans `docs/cadrage/evo-auto.md` ; aucune implémentation autorisée par ce cadrage.
Évolution navigation des chemins : décisions Q1–Q4 acceptées le 14 octobre 2026 dans `docs/cadrage/evo-arbo.md` ; lot 1, tâches 1 à 6, autorisé en commits locaux. Les trois chemins se choisissent avec les fenêtres standard Windows et restent saisissables ; modèle et sortie sont mémorisés, tandis que l'audio est vide au redémarrage et seul son dernier dossier est retenu. Les chemins manquants reviennent aux valeurs par défaut avec un message clair. L'essai final dans la fenêtre est réservé à l'utilisateur.
Évolution sélection du microphone : besoin, inventaire et plan dans `docs/cadrage/evo-mic-selection.md` ; aucune implémentation autorisée par ce cadrage.
État du produit : MVP validé le 13 octobre 2026 (`v0.1.0-mvp`) ; évolution micro locale `v0.2.0-micro`, livrée avec réserve de vitesse ; évolution GPU locale `v0.3.0-gpu`.
Historique : design D-WHISPER-30, plan P-WHISPER-31 et state.json en phase IMPLEMENTATION ; L00–L03 completed, L04–L07 planned. Ces documents décrivent une V1 plus large et ne donnent pas d'autorisations nouvelles.
Les nouveaux livrables de cadrage vont dans `docs/cadrage/`, suivis par Git. `docs/predev/` reste une archive historique en lecture.
Stack déclarée : Rust ; autre technologie ou code système seulement sur accord précis dans la session en cours.
G1 est arrêté définitivement. Une tâche Whisper doit servir l'objectif produit ; utiliser Git et les tests utiles, sans broker ni campagne documentaire ajoutée.

## Passage du cadrage à l'implémentation Rust

- Une demande d'implémentation après cadrage ne suffit pas à déduire l'autorisation de coder : vérifier le plan accepté dans Git, le lot et ses chemins, les prérequis applicables et l'accord écrit précis donné dans la session en cours. Si une entrée manque, préparer le périmètre concret et demander uniquement la décision nécessaire ; ne pas modifier le code produit.
- Lorsque ces entrées sont réunies et que le profil est disponible, la session principale **délègue l'implémentation du lot à `rust_implementer` en mode `IMPLEMENT`**, avec le plan, les critères d'acceptation, les chemins autorisés, l'état Git et les vérifications attendues. Elle ne réalise pas en parallèle la même modification de code. Si le profil est indisponible, signaler cette limite et demander l'arbitrage de l'utilisateur avant de changer de responsable.
- Une fois un candidat Rust concret produit, la session principale **confie sa revue à `rust_review_coordinator`**. Celui-ci fait intervenir un `rust_reviewer` indépendant et en lecture seule ; les corrections reviennent à `rust_implementer`, puis au même reviewer pour vérifier leur fermeture. Respecter la limite commune de deux cycles revue/correction et rendre `LIVRÉ AVEC RÉSERVES` avec les constats ouverts si elle est atteinte.
- L'implémenteur rapporte le diff et ses vérifications ; le reviewer seul juge les constats et déclare `CLEAN`. Distinguer ces résultats de la validation produit dans la fenêtre Windows, réservée à l'utilisateur lorsqu'elle est prévue. Les profils de cadrage restent en lecture seule pour leurs contributions documentaires ; ce routage ne leur donne aucun droit de modifier le code.

## Règle permanente des campagnes locales

### Niveau d'exécution 1 — catégories de commandes Cargo

Le niveau 1 valide les catégories suivantes par une liste d’autorisation ; tout argument non reconnu est refusé par défaut. Il n’arme aucun `FileSystemWatcher`, abonnement aux événements du système de fichiers ou arrêt sur un événement `.git` d’origine inconnue. Les auto-tests vérifient arguments, variables, racines et jonctions sans surveillance. Cette règle prévaut pour ce périmètre sur les obligations de surveillance et leurs auto-tests historiques ci-dessous.

| Catégorie | Sous-commandes | Conditions |
| --- | --- | --- |
| Lecture | `tree`, `metadata`, `fmt --check` | aucune écriture hors des racines autorisées du lot et de la cible Cargo |
| Compilation | `check`, `build`, `test --no-run`, `clippy` | `--workspace`, un ou plusieurs `-p <membre>`, `--release` autorisés |
| Tests | `test` | sans `--ignored`, `--include-ignored`, `--features native-tests` |
| Dépendances | `fetch --locked` | seule commande Cargo autorisée en réseau ; téléchargement seul, sans compilation ni exécution |

Un contrôle permanent ne dépend jamais d’un fichier jetable sous `target/` ni d’un dossier de lot : ses références sont versionnées ou dans un emplacement persistant explicitement déclaré et protégé par la rétention. Les fixtures PE synthétiques permanentes sont versionnées sous `ci/fixtures/pe/`. Un témoin réel est facultatif et explicitement sélectionné avec son SHA-256 ; son absence est annoncée `témoin réel : NON EXÉCUTÉ`, sans remplacer les auto-tests permanents.

Suite de référence niveau 1 pour les tests CPU sans CUDA : `cargo test` (membres par défaut), `cargo test -p whisper-worker-cpu`, `cargo test -p whisper-worker-parakeet-cpu` et `cargo test -p whisper-worker-common`. Chaque sélection worker passe d’abord le contrôle PE sur ses harnais ; les inventaires utilisent le suffixe explicite `-- --list`, autorisé par la garde. Le GPU est niveau 2 : `cargo test -p whisper-worker-gpu -- --list` sert seulement à comparer les identités et son corps de tests ne s’exécute pas dans la campagne niveau 1. `cargo test --workspace` reste une commande permise, mais n’est plus la référence niveau 1.

- Toute validation N1 inclut, dans un shell dédié, une exécution `cargo test --no-fail-fast` sans `CARGO_TARGET_DIR`, via la garde qui fixe la cible autorisée uniquement par `--config build.target-dir=...` et vérifie le `target_directory` effectif ; aucun fichier de configuration Cargo ni profil utilisateur n'est modifié.

Paramètres de chaque appel :
- Racine de lot explicitement contrôlée dans le worktree ; `CARGO_HOME`, `TEMP`, `TMP` et `LOCALAPPDATA` canoniquement isolés ; contrôle des jonctions et liens, espace libre minimal de 30 Gio.
- `CARGO_TARGET_DIR` : cible partagée existante `C:/dev/whisper/target/parakeet-tests/cargo-shared/`, ou répertoire **neuf** sous la racine du lot via `-FreshTarget <nom>`. Pour le niveau 1, cette possibilité prévaut sur l’exigence historique de cible unique ci-dessous.
- Pour les validations et corrections successives d’un lot N1, réutiliser par défaut la cible partagée autorisée, y compris lors du passage entre implémenteur et relecteur. Réserver `-FreshTarget` à une compilation propre exigée par le plan ou nécessaire à un diagnostic reproductible. Avant de créer une cible neuve, préciser le contrôle qui l’exige et la cible à supprimer à la clôture. Une correction ou une nouvelle revue ne justifie pas à elle seule une nouvelle cible.
- `-p` accepte uniquement les membres du workspace lus dans `cargo metadata` exécuté avec les mêmes contrôles. Les auto-tests de validation utilisent des fixtures de membres sans lancer Cargo pour les cas négatifs.
- `-UnsetEnv` retire explicitement seulement `CUDA_PATH`, `CUDA_PATH_V*`, `GGML_*`, `WHISPER_*`, `PARAKEET_*` ; le nettoyage existant des variables Whisper et Parakeet reste actif.
- `CARGO_HOME` de niveau 1 : emplacement persistant déclaré `target/cargo-home-shared/` sous le worktree, contrôlé canoniquement sans lien ni jonction. Cette racine distincte du lot est autorisée pour le cache uniquement et protégée par la rétention ; `TEMP`, `TMP` et `LOCALAPPDATA` restent isolés sous le lot. Les dispositions de ce paragraphe prévalent sur l’isolation historique de `CARGO_HOME` sous le lot.
- Capture de stdout/stderr et des codes de sortie, arrêt sur échec ; `--offline --locked` pour les commandes qui les acceptent (`fmt --check` est validé séparément). Exception unique : `fetch --locked`, sans `--offline`, autorisé en réseau sans compilation ni exécution ; `fetch` sans `--locked` est refusé. Un cache absent avant compilation donne un diagnostic renvoyant à `fetch --locked` via la garde. `-DryRun` valide la commande demandée sans l’exécuter ; la lecture des membres via metadata reste soumise à la garde et n’est pas un prérequis à fetch.

Toujours refusés :
- `run`, `install`, `uninstall`, `update`, `publish`, `clean`, `add`, `remove`, `fix`, `generate-lockfile`, `vendor` et toute commande externe `cargo-*` non listée.
- `--fix`, `--allow-dirty`, `--all-features`, `--features native-tests`, `--features recette-hooks` avec `--release`, `--ignored`, `--include-ignored`, `--config`, `-Z`, `--target-dir`, `--manifest-path` hors du worktree et tout argument inconnu.

Les contrôles Git utiles se font séparément avec `--no-optional-locks` ; une activité Git concurrente doit être identifiée par sa commande et son ascendance disponibles, sans attribuer rétroactivement un événement à ce processus ni le terminer automatiquement. Le niveau 1 ne démontre pas l’absence d’écriture hors racine. Les recettes natives restent soumises à leur périmètre autorisé.

Politique de décision des agents sur les activités Git concurrentes :

- La présence de `git.exe` ne constitue pas à elle seule un conflit. Une commande inconnue, une ligne de commande indisponible ou un processus disparu avant identification ne justifient pas automatiquement le blocage d'une campagne, son interruption ou l'envoi de Ctrl+C. Une consultation Git identifiée n'est pas à elle seule bloquante ; une activité dans un autre dépôt ne l'est pas non plus sans ressource partagée pertinente pour l'opération. L'ascendance ChatGPT.exe ou Codex ne donne aucune exemption générale.
- Observation non concluante : la consigner dans le compte rendu existant et poursuivre si aucune autre condition bloquante n'est établie. Risque de concurrence pertinent : suspendre les prochaines opérations exposées et investiguer, sans interrompre automatiquement une commande indépendante déjà en cours. Violation démontrée nécessitant un arrêt : appliquer la procédure d'interruption de la garde concernée.
- Les modifications concurrentes non autorisées des ressources protégées, les changements invalidant les entrées de validation et les verrous effectivement bloquants restent des motifs de suspension ou d'arrêt selon les garanties applicables. Évaluer les événements `.git` selon les ressources affectées et les invariants protégés, sans attribution à un processus par simple coïncidence temporelle ; cette analyse ne lève aucune condition d'arrêt de la garde applicable.
- Une absence temporaire de sortie ne constitue pas à elle seule une violation d'intégrité ; les délais d'expiration et mécanismes de détection de blocage restent indépendants. Cette politique n'ajoute aucune surveillance au SelfTest N1 et ne désactive aucun contrôle d'intégrité.
- Pour FUSION-PILE, la décision utilisateur du 2 novembre 2026 remplace les restrictions globales « Aucune activité Git » et « Toute activité inconnue est bloquante » par la vérification des activités concurrentes pertinentes pour l'opération. Les consignes des prochains lots suivent cette même politique, sans supprimer les protections contre les conflits établis.

- Avant chaque lot, relever l'espace libre du volume de travail. Sous 30 Gio libres, arrêter avant toute compilation ou campagne de test.
- Utiliser un seul `CARGO_TARGET_DIR` de compilation : `target/parakeet-tests/cargo-shared/`. Les racines de lot reçoivent uniquement sorties, temporaires et preuves. La garde à échec fermé vérifie séparément cette racine Cargo canonique et les racines d'écriture du lot ; aucune commande Cargo n'est lancée si une variable sort de sa racine autorisée.
- **Aucune écriture de campagne hors des racines autorisées, même temporaire ou indirecte par Cargo, rustc, un script ou un processus enfant.** Avant le premier Cargo, fixer et vérifier canoniquement `TEMP` et `TMP` sous la racine d'exécution du lot, `CARGO_HOME` sur l’emplacement persistant déclaré de niveau 1 (sous le lot pour les autres campagnes historiques), et `CARGO_TARGET_DIR` sur la racine autorisée. Ne pas utiliser le cache Cargo du profil utilisateur en écriture. Une variable absente, une résolution hors racine autorisée ou une jonction qui y mène interdit le lancement de Cargo.
- La garde doit prouver par des auto-tests qu'elle refuse ces chemins hors racine avant Cargo et surveiller les écritures pendant la commande. Une écriture hors racine attribuée à la campagne, une modification inexpliquée du dépôt ou d'un autre emplacement sensible surveillé, une erreur de surveillance, une violation du préflight ou un échec de compilation ou de test fait échouer et arrêter la campagne ; consigner le chemin et l'heure, ne pas effacer ni restaurer sans état antérieur vérifié, et demander un arbitrage avant de reprendre. Un contrôle qui détecte la fuite seulement après l'écriture ne la rend pas autorisée.
- **Temp Windows — règle permanente révisée avec accord utilisateur du 31 octobre 2026 :** un événement Created, Changed ou Deleted observé uniquement sous le `%LOCALAPPDATA%\Temp` réel, sans attribution démontrée à la campagne, est consigné `UNATTRIBUTED` avec chemin, type d'événement et horodatage ; il ne provoque pas à lui seul l'arrêt. Pour Renamed, cette disposition ne s'applique que si l'ancien et le nouveau chemin restent tous deux sous ce Temp réel. Un renommage traversant sa frontière reste bloquant. Une origine inconnue reste inconnue : elle n'est ni une écriture Cargo prouvée, ni une écriture autorisée ; aucun nom ou préfixe, y compris `etilqs_*`, n'établit à lui seul l'origine.
- Cette révision n'autorise aucune écriture volontaire de la campagne dans le Temp utilisateur : `TEMP` et `TMP` restent canoniquement isolés sous le lot, `CARGO_HOME` sous sa racine autorisée selon le niveau, et les écritures attribuées à la campagne hors racine restent bloquantes, y compris dans le Temp réel. Les auto-tests doivent vérifier séparément la journalisation sans arrêt d'événements Temp non attribués et l'arrêt des écritures attribuées, des renommages traversants et des autres violations. Le risque résiduel d'écritures Temp d'origine indéterminée est accepté ; une surveillance sans PID ne prouve pas l'absence de fuite et n'est pas un confinement préventif. Cette règle remplace uniquement le traitement des événements Temp non attribués dans les clauses historiques archivées ; leurs autres limites ne sont pas étendues.

Dérogations historiques : [lot 1 Parakeet micro et FEAS-PM-01](docs/cadrage/derogations-historiques-campagnes-locales.md).

- Pendant les campagnes, fixer dans l'environnement `CARGO_INCREMENTAL=0` et `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, sans modifier `Cargo.toml` pour ces réglages.
- À la clôture du lot, archiver les petites preuves en préservant leur arborescence et en vérifiant nombre et SHA-256. Après cette vérification, supprimer les cibles de compilation propres au lot (dont les cibles neuves `-FreshTarget`) et ses caches `cargo-home` isolés : ces intermédiaires ne sont pas conservés avec les preuves. Avant suppression, identifier les chemins exacts, vérifier que leurs chemins absolus résolus restent dans le dépôt et appartiennent au lot, sans lien ni jonction menant ailleurs, et qu'aucun processus ni lot actif ne les utilise. Si ces conditions ne sont pas établies, conserver les chemins concernés et signaler le blocage ; ne pas élargir la suppression à leur dossier parent.
- Pour une recette ou un assemblage, utiliser directement l’entrée de référence ou un staging déjà préparé lorsque le plan, la garde et le routage du produit le permettent. Lorsque la recette exige une copie isolée, réutiliser cette copie entre scénarios compatibles du même lot. Éviter une deuxième copie du même modèle, runtime ou paquet sauf si l’isolation ou le scénario l’exige. À la clôture, conserver les livrables, résultats et petites preuves nécessaires, puis appliquer la rétention aux copies de staging et aux cibles jetables. L’orchestrateur transmet ces règles lors de la reprise d’un lot depuis un worktree antérieur.
- Préserver le cache partagé canonique `target/parakeet-tests/cargo-shared/`, le cache Cargo persistant déclaré `target/cargo-home-shared/` de chaque worktree et toute référence déclarée d’un contrôle permanent, ainsi que les preuves archivées, les livrables, les modèles, les audios sources et toute entrée encore nécessaire à un lot actif. Les caches persistants ne sont pas des caches isolés propres au lot à supprimer lors de sa clôture. Les copies et WAV temporaires restent supprimables uniquement s'ils figurent dans un manifeste approuvé. Consigner les chemins supprimés et l'espace libre après nettoyage dans le bilan avant de le figer ; tout nettoyage ultérieur fait l'objet d'un nouveau document selon la règle de clôture des bilans.
- L'agent exécute la suppression lorsque son outil l'autorise. Si l'outil refuse la suppression, l'agent produit la liste exacte des chemins, leur taille, la vérification d'archivage des preuves et la commande à exécuter ; l'utilisateur supprime, puis l'agent vérifie l'absence des chemins. Un refus de l'outil ne doit pas être contourné par un autre mécanisme de suppression.

- Tout chemin déclaré à conserver est vérifié présent au moment où la règle est écrite ; un chemin absent est un blocage signalé, jamais une protection silencieuse. Aucune preuve à conserver sous target/.
- **DLL natives BUILD-NATIF :** les builds natifs sont suspendus depuis la WHEA du 5 novembre 2026, non qualifiée. Leur éventuelle reprise exige une décision précise ; leur production passe uniquement par `ci/build-native.ps1` et sa voie surveillée. La [perte du banc historique](docs/architecture/PERTE-SPIKE02-NOTE.md) n'autorise aucune reconstruction.
- **Lot INSPECT-CIBLEE — décision du 6 novembre 2026 :** worktree `C:/wt/inspect-ciblee`, cible Cargo externe unique `C:/wb/inspect-ciblee/cargo-target`, y compris sans variable via la garde ; cache Cargo persistant du worktree et temporaires isolés selon N1. Aucun déplacement ni suppression par l'agent ; les commandes de rétention sont proposées à l'humain. Les trois N1 sans variable servent aussi de preuve P6. Cette dérogation reste limitée à ce lot et ne lève ni la suspension native ni les autres contrôles.

### Diagnostic d'un blocage

- Commencer par le test le moins coûteux qui permet de trancher entre les hypothèses, même s'il doit être exécuté par l'utilisateur hors de la garde. Le décrire en une commande copiable, avec le résultat attendu pour chaque hypothèse, et attendre la réponse avant de construire de l'outillage.
- Pour un écart entre deux environnements (garde ou terminal, machine A ou B), réduire automatiquement la liste des différences par un script (retrait successif, une variable ou un paramètre à la fois) plutôt que tester des combinaisons à la main.
- Budget par diagnostic : 3 essais ou 30 minutes. Au-delà, arrêter et présenter les hypothèses restantes, les tests possibles et leur coût.
- Toute liste de variables ou d'options transmises par la garde à un outil natif (`nvidia-smi`, workers, DLL) a un auto-test sur la machine réelle, avec un témoin négatif qui prouve la nécessité de chaque élément ajouté.

## Remise d'une recette manuelle Windows

- Adapter le dossier de remise au **profil réellement compilé**. Pour le profil personnel, placer uniquement `whisper-worker-cpu.exe` et `whisper-worker-gpu.exe` à côté de l'EXE desktop : il charge les deux workers Parakeet depuis `%LOCALAPPDATA%\Whisper\worker\`, leurs cinq/neuf DLL depuis `native/cpu/` et `native/gpu/`, et ses deux modèles depuis `models/` dans cette installation stable. Vérifier la présence et les SHA-256 de ces ressources installées avant remise, ainsi que les deux workers Turbo livrés. Ne pas ajouter au dossier de recette des copies Parakeet ou DLL que ce profil n'ouvrira pas.
- Pour un binaire de profil Test, relever les chemins effectifs de tous les workers et runtimes. Si des variables d'environnement sont requises, remettre un lanceur dans le même dossier que l'EXE et vérifier ses chemins sans ouvrir la fenêtre. Ne pas annoncer un dossier autonome si le profil utilise des ressources canoniques hors du dossier. Une recette de paquet autonome exige une qualification de ce routage avant remise.
- La validation dans la fenêtre Windows reste celle de l'utilisateur ; indiquer ce qui a été construit et vérifié automatiquement et ce qui reste à observer manuellement.

## Consignes de développement et portée des fichiers locaux

Les `AGENTS.md` des huit crates complètent ce fichier pour leur sous-arborescence, y compris `whisper-worker-common`. Lire celui de la crate avant de la modifier. Ils ne créent aucune dérogation aux autorisations, à la délégation Rust, à la garde permanente des campagnes ou à la recette Windows ci-dessus. En cas de contradiction sur ces sujets, arrêter l'action concernée et demander l'arbitrage de l'utilisateur. Une instruction locale n'est pas un confinement technique.

### Préparation et architecture

- Aucun commit directement sur `main`, y compris pour la documentation et le cadrage : tout passe par la branche du lot, fusionnée par l'utilisateur.
- Aucun `git push` par un agent, quelle que soit la branche : le push et la suppression de branches distantes sont faits par l'utilisateur.
- Relever l'état Git et préserver les changements des autres tâches, y compris les fichiers non suivis. Une branche ou un état Git propre ne prouve pas l'absence d'un autre agent actif ; coordonner les responsabilités avant des modifications concurrentes.
- Les worktrees sont créés hors du dépôt principal, sous une racine courte (par exemple `C:/wt/<tâche>`), jamais sous `C:/dev/whisper/.wt/` ni `target/`. Cela évite que les outils du dépôt principal parcourent les worktrees et que les chemins dépassent la limite Windows de 260 caractères.
- Lire la carte `docs/architecture/ARCHITECTURE.md` et la fiche de la crate dans `docs/architecture/crates/` si elles sont présentes. Ces documents sont des relevés statiques à confronter aux sources et aux manifestes actuels ; ils ne valent ni plan de refonte ni autorisation. S'ils manquent, consulter les sources sans recréer une campagne documentaire.
- Le cœur porte le domaine, les politiques et les effets ; les adaptateurs exécutent les entrées/sorties ; le desktop compose et affiche ; les workers exécutent les commandes IPC du parent ; bootstrap reste minimal.
- Dépendances internes actuelles : `desktop → adapters + core`, `adapters → core`, `worker-cpu/worker-gpu → worker-common + core`, `worker-common → core`, `worker-parakeet-cpu → core`, `bootstrap → adapters + core` (noms abrégés, préfixe `whisper-`). Le cœur ne dépend actuellement que de `serde`. Toute nouvelle crate ou dépendance, interne ou externe, doit appartenir au plan et être explicitement validée avant modification du manifeste.
- Identifier le module cible avant de coder. Les découpages suggérés dans les fichiers locaux sont des pistes pour un plan accepté, pas une obligation de refondre lors d'un correctif limité.

### Structure, nommage et tests

| Règle | Cible pour le code nouveau |
|---|---|
| Taille d'un fichier de production | 1 500 lignes de production maximum |
| Taille d'une fonction | 100 lignes selon `clippy::too_many_lines` |
| Complexité cognitive | 25 selon `clippy::cognitive_complexity` |
| Nombre d'arguments | 7 selon `clippy::too_many_arguments` |
| Complexité des types | 250 selon `clippy::type_complexity` |

- Ces cibles guident la revue locale. `clippy.toml` règle les seuils mais n'active pas les lints et ne bloque pas seul les dépassements. `ci/check-structure.rs` et `ci/tests-baseline.txt` existent et contrôlent la classification des tests ; ils ne remplacent pas le cliquet Clippy structurel. La référence Clippy versionnée est `ci/clippy-structure-baseline.tsv`.
- La création de `ci/check-structure` est permise dans un lot dédié et local, avec plan accepté, périmètre et accord écrit précis dans la session en cours. Ce lot doit définir les contrôles des retours anticipés silencieux dans les tests (« NOT RUN », « skip ») et des seuils de structure, ainsi que le traitement explicite de l'existant. Le script est lancé par la garde de niveau 1, sans surveillance du système de fichiers, avec les prérequis et l'arrêt sur échec de ce niveau. Cette permission ne crée ni le script ni une autorisation d'implémentation du lot. Aucun contrôle CI distant (GitHub ou autre) n'est demandé ou autorisé par ces consignes.
- Ne pas aggraver sans justification les gros fichiers ou fonctions existants. Préférer une extraction limitée lorsqu'elle sert le changement accepté. Si elle dépasse les chemins ou le lot autorisés, exposer l'écart et demander uniquement l'arbitrage nécessaire ; ne pas étendre le refactoring automatiquement.
- Nommer le nouveau code selon sa responsabilité produit, sans identifiant de lot ou de campagne. Préserver les identifiants historiques qui font partie d'un contrat existant (protocole, variable d'environnement, trace ou script) tant que leur migration n'est pas autorisée.
- Placer les nouveaux tests unitaires dans un fichier séparé du code de production, par exemple `src/<module>/tests.rs` déclaré par `#[cfg(test)] mod tests;`. Placer les tests d'intégration dans `tests/`. Les faux workers et fixtures partagés vont dans les supports de tests adaptés à leur visibilité ; un `tests/support/` n'est pas automatiquement accessible aux tests unitaires.
- Éviter les ajouts aux gros blocs de tests inline existants. Ne pas ajouter de champs ou branches de production uniquement pour les tests si un helper ou une injection simple suffit ; ne pas imposer un nouveau trait à chaque test.
- Les hooks de recette nouveaux restent explicitement activables, inactifs par défaut et absents du produit livré. Préserver les gardes `recette-hooks` existantes et leur refus en release par `scripts/recette-build.rs`. Cette feature n'existe pas dans toutes les crates ; les features de qualification existantes restent distinctes et ne sont pas migrées implicitement.
- Signaler les écarts concrets et leur risque dans le compte rendu ou le document du lot déjà prévu. Ne pas créer systématiquement `docs/debt.md`, des marqueurs `DEBT-*` ou un nouveau livrable hors plan.
- La garde, les lanceurs et les scripts de recette ne créent jamais un état que le produit doit créer lui-même (dossiers `settings`, `logs`, fichiers de configuration, registre). Un échec dû à cet état absent est un constat sur le produit, à corriger dans le produit et à couvrir par un test « premier lancement » sur un `LOCALAPPDATA` vide, jamais un prérequis d'environnement à préparer.

### Sécurité et vérifications locales

- Respecter les niveaux `unsafe_code` réellement déclarés par chaque crate. Un commentaire `SAFETY` ne lève jamais un `forbid` ou un `deny`. Tout nouveau bloc unsafe dans un périmètre FFI autorisé justifie ses invariants ; aucun élargissement de ce périmètre sans accord précis.
- Aucun accès réseau ajouté au produit. Ne pas modifier `Cargo.lock` hors d'une tâche qui l'autorise explicitement.
- Lecture, analyse statique et `git diff --check` ne lancent pas de campagne. Pour un changement limité aux consignes et à `clippy.toml`, vérifier les chemins, le contenu, la fusion et la syntaxe sans lancer Cargo.
- Toute commande Cargo locale, même `metadata`, `check`, `clippy`, compilation ou test sans modèle, reste soumise à la règle permanente des campagnes et aux autorisations du lot. Aucune catégorie « compilation simple » ne dispense du préflight, de la garde ou de ses auto-tests. Utiliser `--offline --locked` lorsque la commande les accepte ; ne pas utiliser `--fix` ni télécharger des dépendances sans accord précis.
- Pour du Rust, effectuer les vérifications utiles prévues au lot : format, tests concernés, build et lints applicables. Les lints de structure peuvent être demandés explicitement à Clippy (`-W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity`) dans la campagne autorisée. Ne pas confondre une analyse relue, un test ignoré, un contrôle réussi et une validation native ou dans la fenêtre Windows.
- Clôture d'un lot touchant du Rust = N1 + Clippy (commande de la matrice) via `ci/guard-level1.ps1` ; toute nouvelle alerte structurelle bloque, sauf entrée ajoutée à la main dans `ci/clippy-structure-baseline.tsv` et justifiée dans le bilan du lot.
- Ne pas créer d'isolation Windows, de compte, de service, de tâche planifiée ou de surveillance supplémentaire à partir de ces consignes. Respecter la garde existante et la portée exacte de ses dérogations ; un outil manquant ne justifie pas son contournement.

### Compte rendu

Commencer les rapports par trois lignes : `Avancée : ...`, `Blocage : ...`, `Décision : ...`. Préciser les fichiers changés, les vérifications réellement exécutées et les limites restantes. Regrouper les décisions de produit ou de périmètre avec options et recommandation ; aucune limite à une seule question ne dispense de demander un accord devenu nécessaire. Ne pas pousser sans accord précis.

- Un bilan est figé à la clôture de son lot. Tout fait nouveau (validation, correction, remise, nettoyage ou autre complément) est consigné dans un nouveau document ; le bilan peut uniquement recevoir un lien vers ce document, sans ajout de récit ni réécriture de son état de clôture. L'accumulation de compléments ou de paragraphes denses ne doit pas contourner la limite d'une page.
- Les règles destinées aux agents et leurs modifications figurent dans les `AGENTS.md` applicables. Un bilan peut y renvoyer, mais ne porte aucune règle nouvelle ni dérogation normative.

<!-- BEGIN AWS Agent Toolkit rules -->
# AWS Guidance for the new AWS experience

This user has signed up for the new AWS experience. This experience lets you sign into AWS using a social provider and requires the following additional context.

Where this guidance conflicts with the project's own instructions, the project's instructions take precedence.

## Context
### Terminology:
- Say "project" instead of "account" — a project contains an AWS account and settings for sharing with other collaborators
- Say "team member" instead of "IAM user" — users are invited by email, not created or federated in IAM
- Say "AWS Settings" when referring to management tasks at https://settings.aws.com/ (project management, billing, team members, spend limits). Users view their actual AWS resources in the AWS Management Console.
- Say "selected Region" when referring to the user's Region — not "home Region"
- The user has a managed IAM experience. This includes a managed service control policies (SCP) and resource control policies (RCP) that govern the use of AWS. They will still need to use IAM to create policies to let services work with each other. If there are questions about the SCPs or RCPs, go to the documentation at https://docs.aws.amazon.com/accounts/latest/reference/scps-and-rcps-for-projects.html

### Constraints:
- MUST NOT access or use any AWS service or invoke an AWS service API, including read-only calls, through AWS CLI, MCP, SDK, console, or another interface unless the user has expressly approved the specific service and action in the current session. Toolkit setup, authentication, or approval for a different action does not grant this authorization.
- All projects share a single AWS Region determined by the user's contact address. Resources cannot be created in other Regions
- When developing:
  - MUST create all Regional resources in the project's assigned Region
  - You CAN create AWS WAF and Cloudwatch Logs resources in us-east-1 when there are global resources (like a global WAF instance) that require a connection to dependencies in us-east-1. You should not use these for any other reason, because resources in the selected Region will provide lower cost (due to no cross-Region traffic), increased availability (due to no cross-Region traffic), and easier manageability (due to not needing to look in another Region).
When you need to do an inventory of resources, you need to look in both the selected Region and us-east-1 for Cloudwatch Logs or WAF resources.
  - MUST NOT attempt to create Lambda, API Gateway, or other Regional resources in any other Region
  - MUST direct users to confirm their Region in AWS Settings > View all projects > Overview > Additional Info > Region. If the user cannot confirm their Region, check in ~/.aws/config
  - MUST NOT use Lambda@Edge — excluded from both Lambda and CloudFront
  - MUST NOT use CloudFormation StackSets — no multi-account or multi-Region deployments
  - MUST NOT attempt cross-Region actions — no cross-Region replication for DynamoDB/S3/RDS, no multi-Region KMS keys
  - MUST NOT use Route 53 cross-Region routing — geolocation, latency-based, and failover routing policies are not available
  - CloudFront is a global service and its actions ARE allowed in us-east-1. A user can create a CloudFront distribution pointing to their project-region Lambda function URL or API Gateway. However, Lambda and API Gateway themselves MUST NOT be created in us-east-1 — they must be in the project Region.
  - Reduced availability in eu-north-1 specifically: Amazon Rekognition, Amazon Textract, Amazon Personalize, AWS App Runner are not available in that Region.
- IAM permissions for human access are managed by AWS. Don't assign roles to team members unless absolutely necessary
- The user may have a spend limit if they are on the paid plan. The limit that pauses their project if it's exceeded. If resources suddenly become inaccessible, ask if they have a spend limit configured. Only project owners can modify a spend limit.
- When developing:
  - MUST ask about spend limit status if the user reports sudden "Access Denied" errors on operations that previously worked
  - MUST direct users to check spend status in AWS Settings > Billing
  - MUST check if a user has upgraded their account to the paid plan
  - MUST ask the user if they want to clean up the successfully created resources or keep them to reduce cost
- The user sets up billing, creates spend limits, and retrieves and pays invoices in AWS Settings. The user creates budgets and optimizes their costs in the AWS Billing and Cost Management console
- Not all AWS services are available. If a service isn't working, do the following:
  1. Run the command aws freetier get-account-plan-state
  2. If accountPlanType is FREE, check the Free Tier supported services list: https://docs.aws.amazon.com/accounts/latest/reference/supported-services-sign-up-new.html#supported-services-free-tier
  3. If accountPlanType is PAID, check the Paid Tier supported services list: https://docs.aws.amazon.com/accounts/latest/reference/supported-services-sign-up-new.html#supported-services-paid-plan
  4. If neither list shows the service, check the Not supported for this experience list: https://docs.aws.amazon.com/accounts/latest/reference/supported-services-sign-up-new.html#unsupported-services. The user will need to activate advanced features to access the service.
- Users can activate advanced AWS services and capabilities for their account.
- For any AWS task or question — building, configuring, migrating, debugging, or operating a service, writing SDK code, authoring CDK/CloudFormation, or answering how a service works — first call aws___search_documentation with topic agent_skills and load the matching skill with aws___retrieve_skill. Do this even when you believe you know the answer: skills carry current service behavior that your training may not. Prefer the skill's guidance over general knowledge.
Skip it only for work that does not involve AWS.

### Help level

- help_level (required): LOW, MEDIUM, or HIGH. While a user is building, you MUST ask the user: "How much guidance would you like from me? Low (I only flag security risks), medium (I ask a couple of clarifying questions if something seems off), or high (I explain what I'm doing, suggest alternatives, and flag best practices)."

You CAN update this rule file to save a user's help_level.

Constraints for each level:

**LOW:**
- MUST follow all constraints in this context file
- MUST execute the user's request without modification
- MUST NOT ask clarifying questions unless the action would create a security vulnerability
- MUST NOT suggest alternatives or improvements

**MEDIUM:**
- MUST execute the user's request
- MAY ask up to two clarifying questions per task if the request has an ambiguity or a potential issue
- MUST NOT repeat a question or suggestion the user has already dismissed
- MUST NOT explain trade-offs or alternatives unless the user asks

**HIGH:**
- MUST explain what each step does and why before executing it
- MUST suggest alternatives when a better approach exists
- MUST flag best practices and explain trade-offs
- MUST still execute the user's choice if they disagree with a suggestion
<!-- END AWS Agent Toolkit rules -->
