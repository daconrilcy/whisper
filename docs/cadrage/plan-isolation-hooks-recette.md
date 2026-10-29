Avancée : lot d’isolation L2a2/L04 validé par l’utilisateur dans cette session.
Blocage : exécution Cargo locale conditionnée à la garde permanente ; CI distante non exécutée avant publication.
Décision : implémenter une feature de recette opt-in et interdire cette feature dans le profil release.

# PLAN-HOOKS-RECETTE-01 — lot unique

## Accord et objectif

Le 28 octobre 2026, après la demande « Vérifier si les hooks L2a2 et L04 sont présents dans un build release. Si oui, les mettre derrière une feature de recette désactivée par défaut, et faire échouer la CI si elle est active en release », l’utilisateur répond **« Je valide »** à ce lot : feature `recette-hooks` désactivée par défaut, conditionnement des corps des hooks, refus de compilation en profil release indépendamment de `debug_assertions`, CI GitHub Actions et implémentation/revue spécialisées. Cette validation autorise la formalisation Git et l’implémentation bornée ci-dessous ; aucun push ni exécution de workflow distant n’est demandé.

Objectif produit : empêcher qu’un binaire de distribution puisse embarquer les barrières/injections L2a2 ou L04 à la suite d’une variable de compilation héritée, tout en conservant une recette explicite hors release. Pas de changement des résultats de transcription, des données persistées, de la sélection native, des stratégies Parakeet ou de l’UI.

## État et constat de départ

Base avant formalisation : `dd8d2f109f10e3c9dcd17fe249eea86ff760c56c`, fichiers suivis propres ; préexistants non suivis à préserver : `docs/architecture/`, `models/`, `sound example/`. Plus de 109 Gio libres relevés avant lot.

Les barrières L2a2 contrôlent `option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD")` sans exclusion de release : `crates/whisper-adapters/src/worker_ipc.rs:399-487`. Les hooks L04 suivent la même logique avec `WHISPER_L04_CAMPAIGN_BUILD` : `crates/whisper-adapters/src/archive.rs:131-161`, `crates/whisper-worker-cpu/src/main.rs:10-83`, `crates/whisper-worker-gpu/src/main.rs:651-724`. Les autres usages de la variable L2a2 participant à l’injection sont dans `parakeet_import.rs` et `l3_gpu_candidate_allowed` de `worker_ipc.rs` ; ils sont inclus dans l’inventaire du lot.

Une archive déjà construite `target/parakeet-tests/cargo-shared/release/deps/libwhisper_adapters-fbd8566ad094d216.rlib` contient les marqueurs ASCII `unknown L2a-2 acceptance barrier`, `.l2a2-acceptance` et `worker-death`. Son fichier `.d` enregistre `PARAKEET_L2_ACCEPTANCE_TEST_BUILD=1`. Ce constat ne qualifie pas un EXE installé. Aucun marqueur recherché n’a été retrouvé dans les trois EXE desktop/CPU/GPU release inspectés ; une recherche de chaînes seule ne prouve pas une absence exhaustive de code. Aucune compilation n’a été lancée pour ce constat.

## Tâches et responsabilités

1. **Principale** : versionner ce plan et transmettre la base, le périmètre, les critères et la garde au responsable d’implémentation.
2. **rust_implementer / IMPLEMENT** : déclarer `recette-hooks = []` dans les crates contenant les hooks, avec `default = []` préservé ; ajouter un relais desktop seulement s’il est nécessaire à l’usage explicite de la recette.
3. **Même implémenteur** : exclure par compilation les corps, états et injections de recette quand la feature est absente, même si les anciennes variables de build sont définies. Des façades neutres minimales peuvent préserver les appels produits ; aucun corps d’injection effectif dans ce cas. Préserver les conditions et sémantiques de recette existantes lorsque la feature est explicitement active hors release. Ne pas transformer une vérification réelle d’identité/capacité en hook facultatif.
4. **Même implémenteur** : interdire `recette-hooks` en profil Cargo release avec un diagnostic explicite. Ne pas utiliser `debug_assertions` comme approximation du profil. Le garde doit fonctionner pour chaque crate porteuse, y compris une sélection directe `-p` et une propagation transitive de feature.
5. **Même implémenteur** : ajouter la CI GitHub Actions de cette politique et des contrôles focalisés positifs/négatifs. Le refus attendu doit être attribué à cette politique, pas à une dépendance manquante, CUDA absent ou une autre erreur. Ne pas imposer de lancement audio/native/GPU pour vérifier la politique.
6. **rust_review_coordinator** : confier le candidat concret à un `rust_reviewer` indépendant en lecture seule ; corrections au même implémenteur, maximum deux cycles revue/correction. Le reviewer seul statue sur les constats et CLEAN.
7. **Principale** : consigner résultat, limites de validation et diff final dans ce document. Pas de nouveau dossier de recette Windows ni de publication.

## Chemins autorisés

Manifestes et points d’entrée :

```text
crates/whisper-adapters/Cargo.toml
crates/whisper-worker-cpu/Cargo.toml
crates/whisper-worker-gpu/Cargo.toml
crates/whisper-desktop/Cargo.toml (relais éventuel seulement)
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/src/parakeet_import.rs
crates/whisper-adapters/src/staging.rs (état/appels campagne seulement)
crates/whisper-adapters/src/supervisor.rs (état/appels campagne seulement)
crates/whisper-worker-cpu/src/main.rs
crates/whisper-worker-gpu/src/main.rs
```

Supports nouveaux permis pour cette seule politique : `crates/whisper-adapters/build.rs`, `crates/whisper-worker-cpu/build.rs`, `crates/whisper-worker-gpu/build.rs`, un garde Rust partagé `scripts/recette-build.rs` si nécessaire, un contrôle Rust focalisé `scripts/check-recette-policy.rs`, un test focalisé `crates/whisper-adapters/tests/recette_hooks.rs`, et `.github/workflows/recette-hooks.yml`. L’implémenteur choisit le sous-ensemble minimal ; pas d’édition de code produit par la principale. Les tests internes des fichiers source autorisés peuvent être adaptés uniquement pour conserver des assertions utiles dans les deux états de feature. Si les tests d’intégration existants doivent être modifiés pour cette politique, remonter leurs chemins précis avant édition.

Document de la principale : `docs/cadrage/plan-isolation-hooks-recette.md`. Aucun changement de dépendance, Cargo.lock, protocole, moteur natif, modèle, DLL, build CUDA, installation, donnée utilisateur, ressource système ni historique de campagne. Ne pas modifier les documents de cartographie déjà remis pour les réécrire comme une nouvelle baseline.

## Critères d’acceptation

- **CA1** : feature absente des defaults dans toutes les crates porteuses ; la seule présence des variables historiques n’active aucun corps/état/injection L2a2/L04 compilé. Les vérifications de sécurité produit restent actives.
- **CA2** : recette explicitement activée et hors release : les mécanismes historiques restent disponibles ; les contrôles focalisés distinguent activation et neutralité sans lancer la campagne native.
- **CA3** : profil release avec feature active : échec attribué à la politique, pour adapters, worker CPU et worker GPU ; il reste un échec si les assertions debug sont forcées dans release. Release sans la feature reste admis, même si les anciennes variables sont présentes.
- **CA4** : la CI utilise la même règle et détecte une régression de CA1–CA3, avec contrôles positifs ; ni succès silencieux après une erreur étrangère, ni simple recherche textuelle présentée comme une compilation réussie.
- **CA5** : les anciens scripts de campagne ne suffisent plus à activer la recette sans feature et les recettes release sont volontairement refusées. Ce changement de contrat est documenté, sans réécrire ni relancer les anciennes campagnes.
- **CA6** : diff limité aux chemins ci-dessus ; aucune modification des parcours de transcription hors appels de recette. Distinction entre vérifications statiques, compilations réellement exécutées, CI distante non exécutée et validation Windows non demandée.

## Vérifications et écritures

La garde permanente AGENTS.md reste intégralement applicable aux commandes Cargo et à toute compilation locale. Le lot n’autorise aucune dérogation nouvelle à son confinement/préflight, aucune installation ni écriture hors dépôt. Racine des sorties de ce lot : `target/parakeet-tests/recette-hooks/` ; compilation Cargo uniquement dans `target/parakeet-tests/cargo-shared/`. CARGO_HOME/TEMP/TMP doivent être canoniques sous la racine du lot avant Cargo, avec les variables prescrites, les auto-tests de refus et la surveillance pendant commande. Aucun réemploi d’une ancienne dérogation de campagne. Aucun nettoyage.

L’implémenteur peut exécuter les contrôles statiques en lecture et les vérifications qui respectent ces règles. Si aucune garde existante ne peut servir ce lot sans l’élargir ou la modifier hors de ce plan, ne pas lancer de commande Cargo/rustc ni créer une nouvelle infrastructure de campagne ; produire le candidat et signaler précisément les vérifications non exécutées à la principale pour décision. Les étapes CI sont écrites et relues, sans prétendre à une exécution distante avant push.

Les vérifications attendues couvrent la syntaxe/format des seuls Rust touchés, la cohérence TOML/features, les combinaisons CA1–CA3 et les tests ciblés nécessaires à leur contrat. Les tests natifs existants ne sont pas relancés par ce lot. Les petites sorties éventuelles restent dans la racine du lot ; pas de manifeste d’empreintes ou dossier de qualification supplémentaire.

## État de livraison

**Candidat implémenté, validation bloquée.** Le candidat working-tree repose sur le plan commité `df28cfc7e8b85dd0e97ed894a9816dfcdd97aa22`. Le même implémenteur a modifié les quatre manifestes adapters/CPU/GPU/desktop, les sources adapters `archive`, `parakeet_import`, `staging`, `supervisor`, `worker_ipc`, les deux `main.rs` Turbo, et ajouté `scripts/recette-build.rs` et `.github/workflows/recette-hooks.yml` : treize fichiers de code/configuration. Aucun fichier de la cartographie, modèle ou audio préexistant modifié. Aucun commit du candidat ni push.

La feature est explicitement opt-in sur les trois crates porteuses, avec relais desktop. Le garde partagé refuse le profil Cargo release quand elle est active ; les corps et états d’injection sont exclus quand elle est absente. Les tests comportementaux focalisés sont dans les sources adapters autorisées ; aucun nouveau fichier de test n’a été nécessaire. Le workflow se nomme **Recette hooks policy** et contient des contrôles Cargo positifs/négatifs, ainsi que des sondes reprenant les blocs L04 réels des workers sans nécessiter d’inférence CUDA.

| Contrôle réellement effectué | Résultat | Portée |
| --- | --- | --- |
| rustfmt ciblé, huit fichiers Rust et extraction des blocs L04 | PASS | Syntaxe/format, pas de compilation/type-check |
| Lecture des quatre manifestes par tomllib | PASS | Déclarations de features, defaults, relais, chemin du garde |
| Analyse syntaxique YAML et PowerShell du workflow | PASS | Syntaxe, sans exécution des étapes |
| git diff --check, auteur puis reviewer | PASS | Intégrité textuelle du diff |
| Cargo/rustc, tests recette_policy_, sondes workers | NOT RUN | Aucune garde autorisée applicable à cette racine |
| Exécution GitHub Actions distante | NOT RUN | Aucun push ni lancement distant autorisé |

**Revue indépendante : `BLOCKED`, aucun défaut source actionnable.** Un reviewer, une passe, zéro correction, candidat figé epoch 0. Le coordinateur et le reviewer n’ont modifié aucun fichier. La revue source couvre notamment les conditions de compilation et la conservation des contrôles d’identité produit ; elle ne remplace pas les compilations. Aucun verdict CLEAN n’est prononcé.

Preuves matérielles ouvertes : **G1** CA1/CA2, tests adapters avec variables historiques définies et feature absente/présente hors release ; **G2** CA3, refus release pour adapters, CPU, GPU et relais desktop, assertions debug désactivées puis forcées ; **G3** CA4, exécution des sondes des blocs L04 et des contrôles du workflow. La CI distante pourra rester non exécutée tant que la publication n’est pas demandée, sans être présentée comme verte.

## Décision d’exécution — autorisation reçue

Les gardes existantes `target/parakeet-tests/parakeet-unified/lot-04/validate-pu-l4.ps1` et `target/parakeet-tests/r11/guard-campaign.ps1` sont liées à leurs racines, modes et autorisations historiques. Elles n’ont été ni exécutées ni modifiées pour ce lot.

**Action précise proposée** : autoriser un unique lanceur PowerShell `target/parakeet-tests/recette-hooks/validate-recette-hooks.ps1`, écrit par le même implémenteur, pour exécuter seulement les contrôles G1–G3. Il préparerait CARGO_HOME/TEMP/TMP et les caches nécessaires sous `target/parakeet-tests/recette-hooks/`, conserverait CARGO_TARGET_DIR sur `target/parakeet-tests/cargo-shared/`, vérifierait canoniquement les racines et les refus de chemins avant commande, surveillerait les écritures pendant commande et conserverait les sorties en cas d’échec. Aucune réutilisation de dérogation, aucune installation, suppression, exécution audio/micro/GPU, nouvelle recette native ou écriture volontaire hors de ces deux racines. La surveillance ne serait pas annoncée comme un confinement préventif du compte Windows ; toute violation imposerait arrêt et arbitrage selon AGENTS.md.

La matrice reprend les tests `recette_policy_` avec variables historiques fixées, les builds/checks positifs et les refus attribués explicitement au garde, puis les sondes L04 du workflow. Elle est hors ligne/locked pour les commandes Cargo du workspace. Les dépendances natives installées peuvent être lues ; les caches hors des racines ne sont pas ouverts en écriture. L’espace libre est contrôlé avant et après ; aucun nettoyage des anciens lots ni de cargo-shared. La création de ce lanceur et l’exécution attendent l’accord précis de l’utilisateur, car elles élargissent les supports d’exécution du lot. Le candidat produit reste en attente de cette validation.

**Accord précis reçu dans cette session : « Tu as mon autorisation ».** En réponse à la demande du lanceur dédié et des contrôles bornés aux deux racines nommées ci-dessus, l’utilisateur autorise sa création par le même implémenteur et l’exécution de G1–G3 après préflight/auto-tests. La phrase précédente décrivait l’état avant cet accord et ne constitue plus un blocage d’autorisation. Aucune dérogation de confinement/surveillance ajoutée, aucune autorisation de push, d’installation, de recette native ou de nettoyage. Les refus release attendus font partie des tests négatifs et doivent être attribués au diagnostic du garde ; un autre échec ou une écriture interdite impose l’arrêt et la conservation des sorties. Pendant l’exécution, aucun changement source ou documentaire parallèle de cette session. Après les contrôles, les résultats retournent au même reviewer pour la fermeture des preuves G1–G3.

## Résultat du premier préflight autorisé

**Arrêt avant toute compilation.** Le même implémenteur a créé le seul lanceur prévu et l’a analysé avec le parseur PowerShell (PASS), puis exécuté `pwsh -NoLogo -NoProfile -NonInteractive -File target/parakeet-tests/recette-hooks/validate-recette-hooks.ps1 -Mode Preflight`. L’exécution termine en code 1 : `Error reading the C:\Windows\Temp directory.` lors de l’activation du FileSystemWatcher de ce chemin. Le marqueur final de préflight n’a pas été créé et le test d’observation d’une écriture autorisée n’a pas commencé.

Les refus des quatre variables manquantes/hors racine, d’un mauvais répertoire Cargo et d’une jonction Windows ont passé avant cette erreur. Le cache hors ligne a été copié dans `target/parakeet-tests/recette-hooks/cargo-home/registry/` ; il reste conservé, sans nettoyage. Espace libre rapporté : **118 212 866 048 octets avant**, **117 348 573 184 octets après**. Les deux racines sont sans jonction. Les sorties d’arrêt restent dans `target/parakeet-tests/recette-hooks/logs/STOP.txt` et `completed-before-stop.txt`. Aucun événement d’écriture hors racine rapporté, aucune commande Cargo/rustc, aucun build/test et aucune CI distante exécutés. G1–G3 restent **NOT RUN** et le verdict indépendant antérieur **BLOCKED** reste inchangé. Aucune modification source, CI ou du lanceur après l’échec, aucune relance automatique.

**Diagnostic en lecture seule par la principale.** Après arrêt du processus du préflight, un essai d’initialisation puis destruction immédiate de FileSystemWatcher récursifs, sans écrire ni lancer Cargo, reproduit l’échec sur `C:\Windows\Temp`, tandis que les observateurs sur `C:\Windows` et `C:\` s’initialisent. Ce diagnostic confirme une possibilité technique de surveiller le même sous-arbre depuis son parent accessible ; il ne prouve pas encore la réception d’événements dans le sous-arbre protégé. Aucun changement d’ACL, aucune élévation et aucun contournement de permissions effectués.

**Arbitrage de reprise demandé** : adapter uniquement le lanceur afin d’observer `C:\Windows\Temp` via un observateur récursif de son parent accessible `C:\Windows`, puis rejouer préflight/auto-tests avant G1–G3. Conserver les deux seules racines d’écriture autorisées et tous les refus existants ; aucune exclusion silencieuse des écritures interdites ni réduction de surveillance. Le test de surveillance et toute erreur/overflow doivent toujours être traités à échec fermé. Cette reprise n’est pas exécutée sans nouvel arbitrage après l’arrêt de préflight consigné ci-dessus.

**Accord de reprise reçu le 29 octobre 2026 : « Oui je t’autorise ».** L’utilisateur autorise précisément l’adaptation ci-dessus et la reprise du préflight puis de G1–G3, avec le même auteur et le même reviewer. Le candidat Rust reste figé, les preuves du premier arrêt restent conservées. Aucun autre élargissement des destinations, droits ou exceptions ; toute nouvelle violation ou panne inattendue conserve la règle d’arrêt. Pendant la surveillance, aucune écriture Git/documentaire parallèle de cette session.

## Résultat de la reprise du 29 octobre 2026

Le même auteur a adapté uniquement le lanceur : observateur récursif sur `C:\Windows`, réutilisation du cache existant après contrôles canoniques/no reparse, nouvelles sorties dans `logs/reprise-20261029/` sans écraser les traces initiales. Les copies source et locale du cache comptent chacune 29 834 fichiers, 784 885 524 octets, sans jonction relevée. Aucun changement Rust/CI/manifeste.

**Préflight PASS, code 0.** Les refus des variables, chemins et jonction ont passé, et le marqueur autorisé a effectivement été observé par le watcher. Espace avant préflight : 116 878 446 592 octets. La réception réelle d’un événement sous `C:\Windows\Temp` reste non démontrée ; aucun fichier n’y a été créé pour la provoquer.

**Run arrêté, code 1.** Le lanceur a appelé Cargo pour `test --offline --locked -p whisper-adapters --lib recette_policy_ -- --ignored`, puis la surveillance a signalé `Changed C:\Windows\Prefetch\RUSTC.EXE-2C0897BD.pf` à `2026-10-28T23:40:54.7285024Z`, soit le **29 octobre 2026 à 00:40:54 heure de Paris**. L’événement est hors des deux racines d’écriture. FileSystemWatcher ne fournit pas le PID ; l’attribution à Cargo/rustc du lot reste indéterminée. Les sorties Cargo sont vides, aucun code Cargo exploitable ni résultat de test, **zéro test démontré**. G1 est interrompu, G2/G3 restent NOT RUN. Aucun cargo.exe/rustc.exe restant observé après arrêt, aucune suppression ou restauration du Prefetch, aucune reprise automatique. Espace avant Run : 116 877 758 464 octets ; après : 116 876 771 328. `logs/reprise-20261029/STOP.txt` et toutes les autres sorties restent conservés.

**Point de périmètre identifié.** Le remplacement de la racine directe `Windows\Temp` par son parent a capturé également `Windows\Prefetch`, qui n’était pas dans le sous-arbre initialement visé. Le lanceur ne filtrait pas les événements du parent à ce seul sous-arbre. La règle de refus appliquée a provoqué l’arrêt ; aucune écriture Prefetch n’est devenue autorisée.

**Arbitrage proposé avant reprise** : limiter les événements traités par ce seul observateur parent au sous-arbre initial `C:\Windows\Temp`, en conservant la surveillance des autres racines d’origine et les mêmes refus. Les erreurs/overflows de l’observateur restent fatales, les deux destinations volontaires restent inchangées et l’incident conserve son attribution indéterminée. Il s’agirait de rétablir la couverture initialement visée, sans élévation, modification d’ACL ni autorisation d’écriture Windows. L’événement hors racines ayant été observé, cette correction et la reprise attendent un nouvel arbitrage selon la règle d’arrêt.

**Accord précis du 29 octobre 2026 : « Oui je t autorise ».** L’utilisateur autorise le filtrage du seul observateur parent aux événements du sous-arbre initial `C:\Windows\Temp`, puis la reprise des contrôles. Les autres surveillances, tous les refus, les deux racines d’écriture et le traitement fatal des erreurs/overflows restent inchangés. Les traces des deux arrêts sont conservées ; aucune écriture Windows, élévation, suppression ou nouvelle exception n’est autorisée. Le même auteur adapte uniquement le lanceur et exécute la matrice ; les sources Rust/CI restent figées pendant cette validation.

## Résultat de la reprise avec filtre Windows\Temp

L’auteur a modifié le seul observateur parent : les événements hors du sous-arbre initial Temp sont hors de sa couverture, tandis que les renommages entrants/sortants contrôlent leurs deux chemins. Les autres observateurs et les refus sont inchangés. Syntaxe PowerShell PASS ; préflight PASS, code 0, avec refus variables/chemins/jonction, Temp hors racine, filtrage du parent, renommages traversants, chemin de renommage manquant et overflow fatal. Le marqueur autorisé a été observé. Espace avant préflight : 116 458 065 920 octets.

**Nouvel arrêt pendant G1, Run code 1** : création de `C:\Users\cyril\AppData\Local\Temp\etilqs_sozDzKGP1XaRa4p`, observée à `2026-10-29T03:37:45.8422925Z`, soit le 29 octobre à **04:37:45 heure de Paris**. Ce chemin est sous la couverture conservée du profil utilisateur, hors des deux racines. FileSystemWatcher ne fournit aucun PID ; son attribution à Cargo/rustc ou à un autre processus demeure indéterminée. Le nom du fichier ne prouve pas son origine. L’événement unique est consigné deux fois dans `logs/reprise-20261029-filtre/STOP.txt` par les deux niveaux de traitement de l’arrêt.

G1 avait été démarré, mais stdout/stderr sont vides, sans code Cargo exploitable : **zéro test démontré**, G1 interrompu, G2/G3 NOT RUN. Aucun processus Cargo/rustc restant observé après arrêt ; aucune suppression/restauration, correction ou relance après cet incident. Espace libre avant Run : 116 457 746 432 octets ; après : 116 456 902 656. Racines sans reparse et caches/anciennes traces conservés. Le candidat Rust/CI et le verdict BLOCKED restent inchangés.

**Décision proposée, non autorisée** : pour les seuls contrôles G1–G3 de ce lot, consigner comme `UNATTRIBUTED` les événements observés uniquement sous `%LOCALAPPDATA%\Temp` sans attribution démontrée, avec chemin et horodatage, sans qu’ils arrêtent à eux seuls les contrôles. Leur origine et nature resteraient indéterminées ; ils ne deviendraient ni écritures Cargo prouvées ni destinations d’écriture autorisées. Toute écriture hors racines attribuée à la campagne, toute modification inexpliquée d’un autre emplacement sensible, violation du préflight, erreur/overflow de surveillance ou échec de compilation/test inattendu conserverait l’arrêt. CARGO_HOME/TEMP/TMP et les caches applicatifs maîtrisables resteraient canoniques dans les deux racines ; aucune destination volontaire dans le Temp utilisateur. Cette dérogation limitée demanderait un accord précis nouveau et ne réutiliserait aucune dérogation historique d’un autre lot. Alternative : conserver le candidat et la validation BLOCKED sans reprendre.

**Accord précis reçu le 29 octobre 2026 : « Oui je t autorise ».** L’utilisateur accepte cette seule exception pour G1–G3 du lot hooks : événements uniquement sous le Temp utilisateur, sans attribution démontrée, consignés `UNATTRIBUTED` avec chemin/heure, sans arrêt à eux seuls. Aucune nouvelle destination volontaire d’écriture ni garantie d’innocuité ou d’origine ; les autres arrêts décrits ci-dessus restent obligatoires. Le même auteur adapte le seul lanceur, rejoue préflight/auto-tests et reprend la matrice. Aucune modification source/CI pendant les contrôles, aucun nettoyage, élévation, installation ou push. Le même reviewer recevra les preuves avant toute annonce CLEAN.
