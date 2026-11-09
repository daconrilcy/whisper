Avancée : B achevé, 190 appels sans échec ; trois tests non reproduits, modifications provisoires retirées après revue.
Blocage : aucun pour la clôture ; causes historiques non établies et précision du temps B limitée.
Décision : livré avec réserves de causalité, validations finales réussies ; aucun correctif Rust revendiqué.

# TESTS-STABLES-2 — plan approuvé

Autorisation de session : « J’approuve le plan. Lance B avec rust_implementer, puis les corrections limitées aux tests et les validations prévues. Respecte les budgets et les arrêts prévus, avec revue finale indépendante. Commits locaux, sans merge ni push. » Reprise à 12 h 00 Europe/Paris, espace libre 472 081 215 488 octets. L'accord ne lève aucune condition d'arrêt ni n'étend le budget initial.

**Décision de reprise du 9 novembre 2026, vers 12 h 04 Europe/Paris, prévalant uniquement sur les clauses B incompatibles ci-dessous :** « J’autorise cet assouplissement du seul plafond B : aucun nouvel appel après 45 minutes, mais laisse finir celui en cours et consigne le dépassement. Reprends avec rust_implementer, sans nouvel outillage. Les autres contraintes restent inchangées. » L'absence de terminaison forcée garantie à 45 minutes n'est donc plus bloquante pour B. Conserver le compteur cumulé des appels de répétition et l'admission avec marge ; à 45 minutes, ne plus lancer d'appel, attendre la fin de celui déjà engagé et consigner sa durée ainsi que le dépassement. Ne pas interrompre cet appel pour la seule borne B. Aucun outil/mécanisme nouveau ; budget global, arrêts sur échec/WHEA et autres contraintes inchangés. B consommé à cette reprise : 0 minute. Libre : 472 089 272 320 octets.

## Mandat et limites

Fiabiliser N1 pour protéger la persistance des réglages et l'absence d'altération du PCM/WAV par la mesure microphone. Base `08062bd8`, branche `tests-stables-2`, worktree exclusif `C:/wt/tests-stables-2`, créé depuis main le 9 novembre 2026. Aucune modification de fichier dans `C:/dev/whisper` ; seule l'inscription Git du worktree a été effectuée depuis le dépôt principal. État initial propre. Libre initial : **472 641 638 400 octets, 472,64 Go / 440,18 Gio**, supérieur aux 150 Go demandés.

Début du lot : 9 novembre 2026 vers 11 h 46, Europe/Paris. Budget ferme : une journée, plan et revue compris ; arrêt au plus tard le 10 novembre à 11 h 46, sans prolongation implicite durant l'attente d'accord. Étape B : **45 minutes cumulées maximum**, compilation/préparation comptées séparément mais incluses dans la journée. Au terme d'un budget : bilan des résultats réellement acquis et dette restante.

Enveloppe prévisionnelle : plan/revue/préparation 2 h, B 45 min, causes/corrections 2 h, validation 2 h, revue finale/archivage 1 h ; ce sont des maxima d'organisation, pas une extension. Le temps actif de préparation, compilation, inventaires, mesures et arrêts est consigné séparément ; B inclut toute la durée murale des appels de répétition (garde comprise) et leur interruption. Une compilation survenant dans une répétition compte donc dans B. Les commandes de préparation explicites ne comptent pas dans B. Les diagnostics d'un blocage restent limités à 3 essais/30 min par AGENTS.md ; les séries B explicitement demandées ne constituent pas une permission de multiplier les essais de diagnostic d'environnement.

Admission temporelle : mesurer le premier appel de chaque sélection, puis n'en lancer un autre que si le temps restant dépasse sa durée maximale observée plus 2 minutes de marge. Ne pas engager N1 en B sans marge pour les trois appels estimés. La garde utilise `WaitForExit` sans timeout : ne pas lui attribuer une borne qu'elle n'a pas. Utiliser une session PowerShell dédiée interruptible par les outils existants (Ctrl+C), surveillée par la session principale avec points de contrôle au plus toutes les 30 s ; arrêter les prochaines commandes puis interrompre l'appel dédié avant 44 min 30 s de B, ou avant la limite journalière avec réserve de clôture. Vérifier les processus enfants identifiés et conserver le résultat comme interrompu, jamais comme réussi ; aucune terminaison de processus inconnu. Si l'interruption ciblée n'est pas disponible ou si sa fin ne peut être garantie, ne pas engager la série exposée : STOP et arbitrage, sans nouvel outil ni timeout ajouté. Une borne dépassée ne serait pas requalifiée en respect du budget.

« Non reproduit » décrit uniquement les répétitions effectivement terminées sans retrouver le symptôme ; si la série prévue n'est pas terminée, ajouter **INCOMPLET (n/N)** et la raison. Absence d'exécution = NON EXÉCUTÉ, pas non reproduit. Une validation obligatoire incomplète reste une dette et interdit de présenter le lot comme entièrement validé.

Menaces accidentelles uniquement, poste mono-utilisateur. Bibliothèque standard et supports existants ; aucune crate, primitive système, instrumentation, élévation ou modification produit. Pas d'ignore, retry de succès, sérialisation globale, assertion affaiblie/supprimée, ni délai augmenté. Si un défaut produit est établi : démonstration par test dans le périmètre, arrêt et rapport de dix lignes ; aucun `#[ignore]` sans nouvel accord précis. Si les moyens simples ne suffisent pas : même arrêt avec l'option la plus simple.

## A — inventaire établi à partir des preuves

Les deux bilans `docs/architecture/REFACTO-POLL-UNIFIED-PHASE1-BILAN.md` et `REFACTO-POLL-UNIFIED-BILAN.md` ont été lus. Les trois ZIP correspondant à `docs/architecture/raw/*REFACTO-POLL*.zip` ont été parcourus en mémoire, sans extraction ni lancement des commandes archivées. Recherche des lignes FAILED et des sections de panic dans les journaux texte ; rapprochement des noms avec les sources actuelles et des binaires avec stderr.

Tous les tests ci-dessous appartiennent à **whisper-adapters**. Les deux tests d'intégration sont à la racine de leur binaire : leur nom complet libtest n'a aucun préfixe de module supplémentaire.

| ID / famille | Nom complet libtest | Binaire / source actuelle | Occurrences d'échec distinctes |
|---|---|---|---:|
| S1 / réglages | `concurrent_path_and_preference_writes_merge_after_v1_migration` | intégration `model_preferences` ; `tests/model_preferences.rs:146` | 1 |
| S2 / réglages | `matching_backup_is_reused_once_and_backup_io_failure_leaves_v1_untouched` | intégration `model_preferences` ; `tests/model_preferences.rs:229` | 1 |
| P1 / probe | `microphone_probe::tests::probe_and_spectrum_reader_leave_stt_pcm_and_final_wav_byte_identical` | bibliothèque `whisper_adapters` ; `src/microphone_probe.rs:768`, exclusivement module `#[cfg(test)]` | 1 |

Preuves exactes dans `REFACTO-POLL-UNIFIED-PHASE1-PREUVES.zip`, préfixe commun `target/refacto-poll-unified/scratch/`, fichier `cargo-command.stdout.log` de chaque répertoire :

- **S1**, `20261107T133027777-449ec7e099834472b1c17972f7eb7262`, lignes 668–674 : panic `model_preferences.rs:178:5`, « assertion `left == right` failed ». Les octets `left` décodés sont une sauvegarde v1 contenant `C:\ConcurrentAudio`, `C:\ConcurrentOutput` et le gain -9 ; `right` représente la fixture initiale `C:\Audio`, `C:\Out`. Le journal conserve les deux tableaux d'octets complets. Ce n'est pas une erreur de verrou Windows démontrée.
- **S2**, `20261107T173307213-048b422323f04375b74731e462955ee3`, lignes 685–689 : panic `model_preferences.rs:251:57`, « called `Result::unwrap()` on an `Err` value: Os { code: 183, kind: AlreadyExists, message: "Impossible de créer un fichier déjà existant." } » lors du `create_dir` du chemin `.v1.bak` destiné à simuler un échec d'écriture.
- **P1**, `20261107T164126802-b4a8c5862a8046a190bfa030a4ca19e1`, lignes 542–551 : panic `microphone_probe.rs:844:13`, « spectrum reader never observed the analysis » ; suite bibliothèque **486/1/49**, 0 filtré, 26,44 s.

Dans les stderr voisins : réglages `model_preferences-bc70b9ec63b9ad84.exe` (à distinguer de l'homonyme de whisper-core), probe `whisper_adapters-009060d04a3305f6.exe`. Les trois stdout/stderr montrent les suites des membres par défaut, sans filtrage : contexte **N1 complet**, et non répétition isolée. Charge machine non mesurée dans ces pièces ; aucun chevauchement N2 démontré, donc aucune causalité attribuée à une charge ou à N2. Les horodatages de noms sont des identifiants de run, pas une mesure de charge.

Décompte des lignes de résultats des trois identités, dans les seuls `cargo-command.stdout.log` : phase 1 **10 ok / 1 FAILED par test** ; phase 2 **9 ok / 0 FAILED par test** ; complément nettoyage **54/0 pour S1, 53/0 pour S2, 55/0 pour P1**. Ce sont des occurrences archivées, pas des campagnes comparables ni un taux expérimental : des copies entre archives sont possibles. Les trois échecs sont trois runs distincts de phase 1 ; aucune occurrence supplémentaire réglages/probe trouvée dans les deux autres ZIP. Les échecs catalogue et `unified_poll_*` sont exclus du lot.

## B — reproduction, ordre et commandes

Après accord seulement, un `rust_implementer` reçoit ce plan commité, les chemins et les critères. Aucun autre agent n'écrit les tests. Préflight : responsabilités exclusives du worktree, état Git, espace libre, chemins canoniques sans jonction, garde N1 et auto-tests permanents. Pas de Cargo direct, y compris metadata. Relever les événements WHEA avant/après toute première compilation susceptible de reconstruire `whisper-rs-sys` ; aucun build natif autonome ni N2.

Exception native exacte de la demande de session : « Aucun build natif (whisper-rs-sys via cargo toléré si une cible neuve l'exige ; compteur WHEA avant/après) ». Elle est limitée ici à la compilation Cargo du worker CPU sur la cible neuve : `crates/whisper-worker-cpu/Cargo.toml:21` dépend de `whisper-rs =0.16.0`, sans features par défaut ; `Cargo.lock:3829–3838` relie ce paquet à `whisper-rs-sys`. Confirmer ce graphe et la nécessité sur la cible choisie au préflight, sans modifier manifestes/lock. Relever le compteur et le dernier RecordId WHEA avant et après ; lecture impossible = préflight incomplet. **STOP au premier crash compilateur ou nouvel événement WHEA**, conserver les traces et demander arbitrage, sans relance automatique. Aucun autre build natif, ni invocation de `ci/build-native.ps1`, ni compilation GPU native n'est autorisé par ce lot.

Racine prévue : `target/tests-stables-2/`, sous ce worktree, avec sorties de garde et journaux. Cible neuve **`target/tests-stables-2/targets/n1`**, sélectionnée au premier appel par `-FreshTarget n1`, puis `-ExistingTarget n1` pour toute la campagne. Cache persistant **`target/cargo-home-shared/`**, à créer après accord par copie du registre local existant et sans jonction depuis `C:/dev/whisper/target/cargo-home-shared/registry/` (lecture seule de la source). Ces racines futures ne sont pas annoncées présentes. Pas de téléchargement implicite : si le cache manque encore, arrêt et demande précise pour `fetch --locked`. TEMP, TMP, LOCALAPPDATA isolés sous le lot par la garde ; `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`.

Tous les appels passent par `ci/guard-level1.ps1`, `-LotRoot target/tests-stables-2`, `-WithoutCargoTargetDir`, et la sélection de cible ci-dessus ; arguments via `-CargoArgsJson`. Le modèle personnel présent `C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf` (713 975 456 octets) est fourni par `-PersonalParakeetModelPath` et vérifié par la garde pour les suites qui l'exigent. Aucun lancement de fenêtre.

Liste fermée des sélections utiles, avec `--offline --locked` ajoutés/vérifiés par la garde :

| Usage | Arguments Cargo |
|---|---|
| Inventaire bibliothèque | `test -p whisper-adapters --lib -- --list` |
| Inventaire réglages | `test -p whisper-adapters --test model_preferences -- --list` |
| S1 seul | `test -p whisper-adapters --test model_preferences concurrent_path_and_preference_writes_merge_after_v1_migration -- --exact` |
| S2 seul | `test -p whisper-adapters --test model_preferences matching_backup_is_reused_once_and_backup_io_failure_leaves_v1_untouched -- --exact` |
| P1 seul | `test -p whisper-adapters --lib microphone_probe::tests::probe_and_spectrum_reader_leave_stt_pcm_and_final_wav_byte_identical -- --exact` |
| Suite réglages | `test -p whisper-adapters --test model_preferences` |
| Suite probe | `test -p whisper-adapters --lib` (binaire complet, pas seulement module) |
| N1 complet | `test --no-fail-fast` |
| Workers, chacun séparément | `test -p whisper-worker-cpu`, `test -p whisper-worker-parakeet-cpu`, `test -p whisper-worker-common` ; préparation par `--no-run` puis inventaire par `-- --list` |
| Format | `fmt --check` |
| Compilation / lints | `check -p whisper-adapters` ; `clippy -p whisper-adapters --all-targets --message-format=json -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity` |

La garde actuelle accepte déjà filtres, `--exact`, `--lib`, `--test`, `--all-targets` pour Clippy et `--no-fail-fast`. **Aucun fichier ci/ à modifier.** Les répétitions sont des appels successifs à la garde, sans nouvel outil de répétition ni lancement direct des harnais.

Ordre B : trois tests seuls ×50, puis les deux binaires ×20 (la suite réglages commune couvre S1 et S2), puis N1 complet ×3 seulement si l'enquête en a encore besoin. Avant chaque série, inventorier ; chaque appel exact doit exécuter exactement **1** test nommé, sans ignored. Suite réglages : **6** tests actuellement ; bibliothèque : identité et cardinalité fixées par l'inventaire du candidat. Un résultat vide ou une identité manquante fait échouer la vérification. Consigner le nombre demandé/réalisé, le nombre d'échecs, les codes et la durée murale par série ; ne jamais conclure 0/50 avec moins de 50 exécutions.

Arrêt au premier échec de commande selon AGENTS.md : conserver les sorties, analyser avant tout nouvel appel, demander l'arbitrage de reprise lorsqu'il est requis. Aucun échec ne déclenche de retry automatique ; les répétitions planifiées ne servent pas à remplacer un rouge par un vert. Le plafond B borne aussi les séries incomplètes : classement non reproduit avec effort exact, jamais « stable » par défaut.

## C–D — hypothèses et corrections bornées à confirmer

- **S1, ordre/parallélisme.** La fixture part de v1 et libère simultanément une écriture de chemins et la migration, alors que son assertion exige le backup initial. Le code actuel sauvegarde les octets présents au moment de la migration ; l'autre test `path_write_before_preference_migration_preserves_paths_and_v1_backup` couvre explicitement l'ordre inverse. Vérifier cette explication par un ordonnancement déterministe dans les tests. Correction proposée : établir/accuser la migration avant les deux écritures concurrentes du scénario « after_v1_migration », conserver les deux écritures réellement concurrentes et toutes les assertions de fusion et de backup exact. Ne pas remplacer l'égalité par une alternative permissive. Si cela contredit le contrat produit, arrêt.
- **S2, collision de chemins.** `test_dir` utilise seulement nom + PID et conserve les dossiers. Le PID peut être réutilisé entre processus ; l'erreur 183 est compatible avec une fixture résiduelle, sans prouver rétrospectivement l'origine. Réserver un dossier unique par invocation via les moyens standard existants ; vérifier le cas de réutilisation du même nom logique et de résidu ancien. Préserver le vrai `create_dir` de `.v1.bak` et l'assertion d'échec de migration : pas de remplacement par `create_dir_all` pour masquer le symptôme.
- **P1, ordonnanceur/délais.** Cinq blocs sont soumis avec des pauses de 20 ms ; file bornée à quatre chunks, calcul asynchrone et fraîcheur de 500 ms. Évaluer séparément perte de chunks et expiration de snapshot. Réutiliser le compteur `processed_chunks`, la barrière de publication de test et les dates injectables déjà présents, avec attente bornée sur progression réelle et diagnostics. Construire si possible un entrelacement déterministe qui expose l'ancienne hypothèse. Préserver l'égalité PCM et WAV, la capture synthétique et une vraie observation de l'analyse : exiger `Some(Available(_))`, car `Some(Unavailable)` ne prouve pas une analyse disponible. Diagnostic utile : chunks traités/perdus, publication et âge du snapshot. Aucun délai augmenté ni hook produit nouveau.

Contrat S1 à confronter **avant correction** : [DEC-PARA-27 et AC-PARA-08B](evo-parakeet.md) (lignes 65 et 130), [L1b B5 et B-A4](plan-parakeet-l1b.md) (lignes 63 et 124). Ils exigent la sauvegarde exacte du « v1 initial/original » avant remplacement, sans écrasement, avec préservation en cas d'échec. Le comportement observé du produit ne redéfinit pas ces mots. L'hypothèse « dernier v1 avant migration » n'est pas une décision produit de ce plan.

| Exigence / source | Scénario à conserver ou établir | Assertions / preuve |
|---|---|---|
| DEC-PARA-27, AC-PARA-08B, B-A4 | Migration avant écritures concurrentes, **snapshot chemins lu en v1 avant migration**, soumis seulement après ACK de migration | Backup exact des octets initiaux, ACK de migration puis ACK révisionnés des deux écritures, fusion de tous les champs ; barrière entre les deux écritures après migration maintenue. Ne pas relire le snapshot en v2 pour simplifier. |
| B5/B-A4, conservation des chemins | Test existant `path_write_before_preference_migration_preserves_paths_and_v1_backup`, chemins puis ACK puis migration | Octets du v1 observés avant migration et champs conservés ; confronter ce scénario au sens documenté de « v1 initial », sans en déduire seul le contrat. |
| AC-PARA-08B et entrelacement initial S1 | Expérience de test imposant chemins→ACK→migration sur la fixture initiale, puis comparaison avec les deux ordres | Identifier précisément les octets sauvegardés et les ACK ; si l'écart au contrat demeure ou exige de redéfinir « initial », **STOP/arbitrage avant modification S1**, pas de suppression silencieuse du scénario concurrent initial. |
| AC-PARA-08B/B-A4, S2 | Backup identique, conflit et chemin `.v1.bak` réellement non inscriptible | Sauvegarde non remplacée, ACK d'échec et v1 intact ; uniquement isolation de la racine de fixture. |

La proposition S1 est donc conditionnelle : elle n'autorise ni à contourner un défaut produit ni à réduire la couverture de concurrence. Le bilan indiquera le rapprochement contrat/scénarios et la preuve permettant le choix, ou la dette et l'arbitrage demandé.

Une cause n'est déclarée établie que par expérience avant/après ou scénario déterministe. Sinon : « non reproduit », effort, hypothèse et limites ; enrichir un message d'échec seulement si utile. Pas de mécanisme général de synchronisation ou de nettoyage ajouté au produit.

## Périmètre, validation et clôture — huit tâches

1. A et rédaction du présent plan par la session principale, commit local.
2. Revue du plan via `rust_review_coordinator`, reviewer indépendant en lecture seule, puis accord utilisateur avant B. Le mandat explicite de revue du plan prévaut ici sur l'usage habituellement post-implémentation du profil ; le coordinateur reste uniquement routeur. Deux cycles maximum pour ce livrable.
3. Préflight, inventaires et reproduction B par `rust_implementer` après accord.
4. C : établir les causes ou documenter les non-reproductions.
5. D : corrections et scénarios de preuve par le seul implémenteur dans les chemins autorisés.
6. Validation : chaque test corrigé **×50 à 0 échec**, chaque binaire concerné **×20 à 0 échec**, puis **N1 ×3 sans CARGO_TARGET_DIR**, mêmes identités/statuts, base attendue **878/0/129** (ajouts justifiés nominativement). Workers **12/0/9**, **26/0/11**, **11/0/1**, contrôles PE avant exécution et SelfTest PE ; témoin réel facultatif NON EXÉCUTÉ s'il n'est pas sélectionné. Format, check, Clippy avant/après pour vérifier aucun nouvel avertissement dans les fichiers touchés. La validation finale reste requise même si N1 n'a pas été nécessaire en B ; elle ne consomme pas le plafond B mais reste dans la journée.
7. Revue indépendante du diff final via le même coordinateur ; corrections par l'implémenteur et retour au même reviewer, deux cycles maximum. Tout changement invalide les vérifications qu'il affecte. Aucun CLEAN déduit des seuls tests.
8. Bilan figé d'une page, archive vérifiée, tailles des racines, libre final, dettes et commandes proposées pour suppression humaine. Commits locaux uniquement, aucun merge/push/suppression par l'agent.

Référence de validation : [bilan fix-catalog-test](bilan-correction-test-catalogue.md), tableau lignes 15–25 et [archive](raw/correction-test-catalogue.zip), notamment `evidence/n1-commands.txt`, `evidence/n1-results.txt` et le run `20261109T111458113…`. Extraire de ces journaux, en mémoire ou dans les preuves autorisées, les identités **binaire + nom du test + statut**, pas seulement les totaux. Chaque N1 final est comparé à cette référence et aux deux autres N1 ; chaque différence (nouveau scénario, renommage ou suppression) est listée et justifiée ; aucune disparition silencieuse d'un ignored ou d'un test exécuté.

Procédure workers : `ci/check-worker-imports.ps1 -SelfTest`, puis pour chaque sélection Cargo `test -p <worker> --no-run` sous garde, recueillir les chemins des **tous** harnais effectivement produits. Pour chaque chemin, `ci/check-worker-imports.ps1 -Kind <kind> -Path <chemin>` : CPU = `Cpu`, Parakeet CPU = `ParakeetCpu`, common = `Cpu`. La référence contient **4 / 5 / 1 harnais** respectivement ; tout écart est expliqué avant exécution. Exiger PASS de chaque PE avant l'inventaire `-- --list` et avant le corps des tests ; ne pas sélectionner un exécutable d'une autre compilation par glob approximatif. Les mêmes contrôles s'appliquent à tout harnais reconstruit. Le témoin réel facultatif est NON EXÉCUTÉ sans chemin/SHA explicitement sélectionnés.

Chemins de code limités : `crates/whisper-adapters/tests/model_preferences.rs` ; module `#[cfg(test)]` de `crates/whisper-adapters/src/microphone_probe.rs` ; si nouveaux tests/helper probe, fichier séparé `crates/whisper-adapters/src/microphone_probe/tests.rs` avec déclaration strictement de test. Pas d'extraction générale du module existant. Toute autre modification exige une extension précise avec fichiers/durée/justification.

Documents autorisés par ce plan : présent document (revue incluse), `docs/architecture/TESTS-STABLES-2-BILAN.md` et `docs/architecture/raw/TESTS-STABLES-2-PREUVES.zip`. Le bilan final n'est pas figé avant exécution/clôture. Les contributions des spécialistes sont remises à la session principale, seule auteure des documents.

Rétention : avant clôture, recenser les racines effectivement créées et leur taille, puis archiver uniquement les preuves utiles (inventaires, sorties, commandes, durées, revue, WHEA, résultats), en préservant l'arborescence et en comparant nombre et SHA-256 des membres à la source. Aucune preuve durable sous target. Proposer à l'humain les commandes sur les chemins exacts, après contrôle canonique, absence de jonction et d'utilisation active ; ne pas les exécuter. Cache persistant et entrées requises conservés, aucune suppression globale du worktree ou de target. L'archive et le bilan contiennent les limites de validation et les séries incomplètes.

## Revue indépendante

Premier routage via `rust_review_coordinator` : `rust_reviewer` refuse la revue d'un plan (profil réservé à un candidat implémenté), sans constat de fond. Le coordinateur a ensuite mandaté `rust_design_reviewer` pour la revue documentaire explicite de l'utilisateur. Cycle de fond 1 sur `a650b3a7` : quatre constats IMPORTANT TS2-P01–P04 (contrat/couverture S1, budgets applicables, exception native, identités/PE), et conseil TS2-A01 (Available/diagnostic probe). Corrections documentaires ci-dessus ; fermeture réservée à la seconde lecture indépendante. Aucun Cargo, aucune modification Rust, aucune répétition B.

Seconde lecture, même reviewer, sur `06f724f326abdcd067eb4d359e15ad938a75c1cf` : **CLEAN**, TS2-P01/P02/P03/P04 fermés, TS2-A01 intégré, aucun constat ouvert. Deux cycles de fond consommés ; pas de troisième revue du plan. La capacité d'interrompre Cargo et ses enfants dans le budget reste un prérequis à établir avant B : Ctrl+C seul ne la prouve pas, le STOP conditionnel du protocole s'applique. Ce verdict documentaire n'est ni l'accord utilisateur ni une validation d'exécution.

État de remise du plan : seul ce document modifié et commité, `git diff --check` réussi, aucun target créé. Racine effectivement créée : `C:/wt/tests-stables-2`, environ 277 Mo de fichiers du checkout (mesure 276 930 877 octets avant les dernières précisions). Libre observé pendant la préparation : 472 111 251 456 octets. Aucune suppression proposée à ce stade, le worktree est nécessaire au lot ; tailles et libre seront remesurés à la clôture. Bilan final et archive de campagne NON PRODUITS tant que le lot n'est pas exécuté ou arrêté.

## Arrêt du préflight après accord — 9 novembre 2026

Avancée : `rust_implementer`, mode IMPLEMENT, a vérifié le préflight en lecture seule sur `a2a9a193`.
Blocage : `ci/guard-level1.ps1:373–412` lance Cargo sans fenêtre, attend `WaitForExit()` sans timeout puis appelle `Dispose()` ; aucune terminaison des enfants n'est garantie.
Décision : arbitrage requis avant les répétitions, conformément au STOP conditionnel du plan approuvé.

Recherche bornée dans `ci/`, `scripts/`, `crates/` : pas d'autre voie d'annulation N1 réutilisable. `Stop-CargoProcessTree` dans `ci/guard-native.ps1:1712–1716` utilise `taskkill /T /F`, mais appartient à la garde native ; aucune invocation, copie ni adaptation effectuée. Aucun Cargo/SelfTest/répétition lancé, aucun code modifié : **B = 0/50 par test, NON EXÉCUTÉ, 0 minute de répétitions** ; C–D et validation finale NON EXÉCUTÉS. Libre : 472 077 066 240 octets. Aucun dossier de campagne créé par l'implémenteur ; aucun candidat Rust à soumettre à une revue finale. Ce point est un arrêt pour arbitrage, pas une clôture du lot ni une preuve de non-reproduction.

Option minimale proposée, non appliquée : garder l'admission avec marge et arrêter les nouveaux appels à la borne, mais laisser terminer l'appel déjà engagé sans garantie de terminaison à 45 minutes ; consigner tout dépassement. Cette option change le plafond strict B et exige donc une décision utilisateur explicite. Aucun mécanisme nouveau n'est conçu ou autorisé par ce compte rendu ; le budget global reste inchangé tant qu'une décision ne le modifie pas.

## B achevé et arrêt de validation — 9 novembre 2026

Après l'assouplissement précis consigné en tête de document, `rust_implementer` a exécuté le préflight : cache local copié sans jonction (22 414 fichiers, 651 731 116 octets), SelfTest garde **64 positifs / 99 négatifs**, inventaires **6** tests pour `model_preferences` et **553** pour la bibliothèque. Les commandes ont utilisé la garde, une cible neuve `target/tests-stables-2/targets/n1`, puis `-ExistingTarget n1`, sans `CARGO_TARGET_DIR`. Préparation des inventaires : compilations 45,12 s et 24,76 s ; aucun `whisper-rs-sys` ni build natif. WHEA avant : 1 événement, dernier RecordId 527992 ; aucun relevé après compilation native puisque celle-ci n'a pas eu lieu.

| Sélection B | Appels terminés / prévus | Résultat par appel | Durées des harnais, somme libtest |
|---|---:|---|---:|
| S1 seul | 50/50 | 1 réussi, 0 échec, 0 ignoré, 5 filtrés | 0,82 s |
| S2 seul | 50/50 | 1 réussi, 0 échec, 0 ignoré, 5 filtrés | 0,81 s |
| P1 seul | 50/50 | 1 réussi, 0 échec, 0 ignoré, 552 filtrés | 6,19 s |
| Binaire réglages | 20/20 | 6 réussis, 0 échec, 0 ignoré, 0 filtré | 0,76 s |
| Bibliothèque adaptateurs | 20/20 | 504 réussis, 0 échec, 49 ignorés préexistants, 0 filtré | 589,32 s |

**190 runIDs uniques, zéro échec Cargo**, contrôles des identités ciblées et cardinalités depuis les journaux complets. Les trois faux négatifs du contrôleur de sortie ont été réconciliés avec les stdout et codes observés, sans rejouer les appels : filtre exact avec cinq autres tests filtrés, contrôle console/espacement redondant, puis ligne de cardinalité absente du Tail30. Une erreur orale de décompte 11 au lieu de 10 a également été corrigée par runID avant reprise de l'appel 11. Aucun échec de test n'a été masqué.

**Budget :** la durée murale cumulée exacte des appels sous garde n'a pas été conservée intégralement ; les intervalles des fichiers du ledger ne la remplacent pas. B est cependant strictement inclus dans l'enveloppe horodatée par la session principale **12 h 04 min 11 s → 12 h 45 min 16 s Europe/Paris**, soit **41 min 05 s**, qui comprend aussi préparation, pauses, analyse et début des modifications. Le plafond de 45 minutes n'a donc pas été dépassé. Le total **597,90 s** du tableau est du temps harnais, distinct de la garde. Aucun N1 supplémentaire en B, les séries prévues étant terminées sans reproduction ; N1 final reste obligatoire.

**C–D et revue.** Les trois tests sont **non reproduits**, causes non établies ; hypothèses : ordre des écritures S1, réutilisation de racines nom+PID S2, ordonnancement/pertes de chunks/fraîcheur P1. Aucun défaut produit démontré. L'implémenteur a néanmoins préparé un diff provisoire de 60 ajouts/10 suppressions dans les deux fichiers autorisés. La revue indépendante via `rust_review_coordinator`/`rust_reviewer` a relevé TS2-R01 BLOQUANT (ordre S1 retiré alors que contrat v1 initial ouvert) et TS2-R02–R05 IMPORTANT (ACK/scénario S1, preuve de collision S2, compteur blocs/chunks P1, délais cumulés/fraîcheur P1). Les seules éditions de l'implémenteur ont été retirées ; comparaison à HEAD `26537d8b` et `git diff --check` réussies, fichiers alignés sur la base `08062bd8`. Seconde revue du même reviewer : **R01–R05 fermés par retrait**, verdict de clôture **BLOCKED**, aucun CLEAN global ; les causes historiques ne sont pas corrigées. Deux cycles utilisés pour ce candidat provisoire.

**Arrêt applicable.** La première tentative de format, après les éditions, avait omis `-ExistingTarget n1`. La garde a retourné 1 avec « Shared Cargo target must already exist: C:\wt\tests-stables-2\target\parakeet-tests\cargo-shared », avant Cargo/rustfmt. Aucun appel Cargo ultérieur. Le correctif de commande est connu : ajouter `-ExistingTarget n1` à **tous** les appels, y compris fmt, en gardant les autres paramètres du plan. La reprise après cet échec de préflight exige l'arbitrage prévu par AGENTS.md ; elle ne demande ni nouvelle cible, ni outil, ni modification de garde. Format, check, Clippy, N1 final ×3, contrôles PE et exécution workers restent **NON EXÉCUTÉS**.

**Preuves mises à l'abri :** [archive provisoire de B et de la revue](../architecture/raw/TESTS-STABLES-2-PREUVES.zip), **781 membres**, **1 065 417 octets**, SHA-256 `ecb8c6d6b824ae76fb0d648d7837c945432322fc1eda61681c918e322c071eeb` ; chaque membre vérifié par SHA-256 contre sa source après réouverture. Arborescence `scratch/` + `evidence/`, ledger 190 appels, handoff et coordination inclus. Pas d'archive de binaires/cache/modèles. Racines observées (octets, snapshot avant archive) : scratch 8 059 218 ; temp 584 132 820 ; cible n1 648 900 884 ; cache persistant 651 788 460 ; evidence environ 96 Ko. Libre relevé : **469 666 025 472 octets**. Rien supprimé ; ces racines sont encore nécessaires à la reprise, donc aucune commande de suppression à exécuter maintenant.

Ce document consigne l'arrêt intermédiaire. Le bilan figé d'une page sera écrit à la clôture ; l'archive sera complétée avec les validations réellement exécutées. Aucun merge/push, aucun code produit ou test modifié dans l'état remis.

## Autorisation de validation finale — 9 novembre 2026

À la demande précise « Autorisez-vous la reprise de fmt, check, Clippy, N1 ×3 et des contrôles workers avec -ExistingTarget n1 systématique ? », l'utilisateur répond « oui j'autorise ». Reprise vers 12 h 57 Europe/Paris, HEAD `ebbd8995`, arbre propre, libre 469 640 589 312 octets. Autorisation limitée aux validations déjà prévues, cible existante n1 sélectionnée à chaque appel, sources inchangées ; les autres conditions d'arrêt et le budget global restent applicables. La validation est déléguée au même `rust_implementer`, mode VERIFY ; aucune nouvelle correction Rust n'est demandée.

## Clôture — validations et rétention

Le 9 novembre 2026 vers 13 h 19 Europe/Paris, toutes les validations autorisées sont exécutées sur les sources inchangées de `08062bd8`. `fmt --check`, `check -p whisper-adapters`, Clippy all-targets JSON avec les quatre lints : **rc 0** ; 340 diagnostics Clippy préexistants, aucun nouveau delta source. N1 `test --no-fail-fast` sous garde et sans `CARGO_TARGET_DIR` : **878/0/129 ×3**, durées murales **103,407 / 35,957 / 36,157 s**. Les **1 007 identités binaire+test+statut** sont identiques à la référence fix-catalog et entre les trois runs. Workers CPU **12/0/9**, Parakeet CPU **26/0/11**, common **11/0/1** ; leurs 21/37/12 identités sont également identiques aux références. Chacun a passé préparation, PE avant inventaire et corps, inventaire `-- --list`, puis tests. **10/10 harnais PE PASS**, SelfTest **4 positifs/15 négatifs**, témoin réel NON EXÉCUTÉ.

L'exception autorisée `whisper-rs-sys` a été utilisée uniquement par la préparation Cargo du worker CPU sur la cible neuve ; WHEA avant/après : **1→1**, dernier RecordId **527992** inchangé. Aucun autre build natif autonome, corps GPU ni test ignoré activé. Deux erreurs de syntaxe shell (saisie check et lecture d'archive) ont été corrigées avant toute invocation concernée ; ce ne sont pas des échecs Cargo ni des rejeux de tests. Les commandes exécutées et les incidents sont distingués dans les preuves.

La seconde revue avait fermé TS2-R01–R05 par retrait du diff provisoire, avec validations alors manquantes. Les validations ci-dessus comblent ces manques sans aucun nouveau changement Rust ; aucun troisième cycle de revue/correction n'a été ajouté. La livraison est celle de **trois tests non reproduits avec preuves**, pas d'une correction des causes ni d'un CLEAN d'implémentation. Chaque test a effectivement été exécuté 50 fois isolément, dans 20 passages de sa suite et dans les 3 N1 finaux, sans retrouver son symptôme. Le [bilan figé d'une page](../architecture/TESTS-STABLES-2-BILAN.md) conserve les réserves.

Archive finale [TESTS-STABLES-2-PREUVES.zip](../architecture/raw/TESTS-STABLES-2-PREUVES.zip) : **865 membres**, **1 386 051 octets**, SHA-256 `edf283407b5cb1d3476b421f2e6b0355c5b7a5929b290a8f8c639c23ac0830ff`. Après réouverture, nombre et SHA-256 de chacun des 865 membres comparés à leur source : PASS. Journaux Cargo/metadata, ledger B, ledger final, identités/références/comparaisons, contrôles PE, WHEA, incidents, revue et rétention inclus. Aucun cache/binaire/modèle archivé. Libre final après archive : **465 881 767 936 octets**, contre **472 641 638 400** au début. Lot clôturé en moins de 2 h, avant la limite de la journée ; B borné à moins de 41 min 05 selon la preuve conservatrice décrite plus haut.

### Racines présentes et commandes proposées à l'humain

Toutes les racines ci-dessous sont sous **`C:/wt/tests-stables-2/`**. Les tailles sont les fichiers réguliers, sans suivre les jonctions ; elles ne représentent pas de l'espace déjà récupéré.

| Racine | Octets | Rétention |
|---|---:|---|
| `target/tests-stables-2/targets/n1` | 4 194 884 389 | Cible propre au lot, suppression humaine proposée |
| `target/tests-stables-2/temp` | 678 140 595 | Temporaires de tests, suppression humaine proposée |
| `target/tests-stables-2/evidence` | 668 200 | Copie des preuves désormais archivées, suppression humaine proposée |
| `target/tests-stables-2/scratch` | 9 797 101 | Conservé : deux jonctions internes de fixtures SelfTest |
| `target/cargo-home-shared` | 651 825 324 | Cache persistant déclaré, conservé |
| Worktree et branche | Checkout initial environ 277 Mo | Conservés avec documents et archive Git |

Le contrôle parent final a établi : les trois chemins proposés se résolvent à l'intérieur du worktree, leurs ancêtres et descendants n'ont aucun reparse point, aucun cargo/rustc/clippy-driver ni processus du lot actif n'est observé. La déclaration trop large « sans reparse » du handoff est corrigée pour scratch : les deux jonctions de `scratch/20261109T121001567-dd9a7b57cc054c75b771162ea5f4448b/` ciblent sa propre fixture `junction-target` ; aucune suppression globale de scratch n'est proposée. Aucune suppression exécutée par l'agent.

**À exécuter par l'humain uniquement après clôture**, en revérifiant qu'aucun lot n'a repris sur ces chemins et qu'ils sont toujours sans jonction. L'archive finale ci-dessus a déjà été vérifiée ; ne pas supprimer son fichier ni un dossier parent des trois racines :

```powershell
Remove-Item -LiteralPath 'C:\wt\tests-stables-2\target\tests-stables-2\targets\n1' -Recurse -Force
Remove-Item -LiteralPath 'C:\wt\tests-stables-2\target\tests-stables-2\temp' -Recurse -Force
Remove-Item -LiteralPath 'C:\wt\tests-stables-2\target\tests-stables-2\evidence' -Recurse -Force
```

Environ **4,87 Go** de fichiers sont concernés. Préserver le cache persistant, scratch, le worktree, la branche, les modèles/audios et ressources installés, les archives et tous les autres lots. Toute suppression ultérieure et sa vérification feront l'objet d'un nouveau document ; le bilan de ce lot est figé. Aucun merge ni push.
