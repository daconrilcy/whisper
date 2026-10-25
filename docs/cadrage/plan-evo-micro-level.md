Avancée : R1 à R7 fermés, harnais diagnostique relu et build personnel réussi ; 16 sorties Turbo CPU conservées.
Blocage : R8 produit préexistant empêche l'acceptation d'un résultat Parakeet CPU ; D2 Parakeet non qualifié.
Décision : demander l'accord précis pour corriger R8 côté adaptateur, préserver l'attestation GPU et vérifier les 16 cas CPU restants.

# Whisper — Plan du réglage du niveau microphone

## Statut et sources

- Date : 25 octobre 2026. Auteur des fichiers : session Codex principale ; proposition de tâches par `rust_plan_writer`, contrôle indépendant par `rust_design_reviewer`.
- Référence produit : `878cd1e9a3a37bd062af9802bec68f2eb45f5337`. Conception : [evo-micro-level.md](evo-micro-level.md), revue **DESIGN CLEAN** sur `d10516808804f461ca1597b78d0d176f3f3baa27` ; R-MIC-D01 fermé. Précisions ultérieures limitées au statut et à la fixture spectrale bin 64, sans changement de Q1–Q5.
- Accord produit exact dans cette session : « ok sur les recommandation Q1 à Q5 ». Accord de rédaction : « tu peux passer à la rédaction du plan ».
- **Plan accepté et implémentation autorisée le 25 octobre 2026 dans la session de réalisation**, par la demande « implemente le plan C:/dev/whisper/docs/cadrage/plan-evo-micro-level.md realisé à partir de la note de cadrage C:/dev/whisper/docs/cadrage/evo-micro-level.md ». Autorisation complémentaire « oui je t'autorise » pour adapter et vérifier la garde PowerShell existante puis exécuter les tests prévus, avec les seules destinations de campagne `target/parakeet-tests/micro-level/` et `target/parakeet-tests/cargo-shared/`. Aucun téléchargement, installation, push ou changement des ressources personnelles n'est autorisé. La recette dans la fenêtre reste à l'utilisateur.
- Deux fichiers de rapport : le cadrage et ce plan. Aucun nouveau railguard, manifeste d'empreintes, journal ou mécanisme d'agent. Aucun push.

## Résultat du lot proposé

Un lot de neuf tâches, L-MIC-LEVEL, pour régler le microphone avant/pendant capture, afficher une jauge et fournir une sonde réutilisable, mémoriser les gains par identité et préserver Turbo/Parakeet, imports et politiques moteur.

Décisions figées par Q1–Q5 : WAV réglé unique, test micro explicite sans moteur/fichier/haut-parleur, gain -24..+24 dB pas 1 dB neutre/initial 0 dB, transition abrupte au prochain bloc acquis. Sonde A mono natif pré-gain, B après gain/limitation avant rééchantillonnage ; RMS 100 ms jusqu'à 20 Hz, FFT Hann 2048 jusqu'à 10 Hz, périmée après 500 ms. Le corpus de qualité précis reste à définir avant T8/T9.

## Prérequis communs et règles de reprise

| Type | Prérequis | Échéance et conséquence |
| --- | --- | --- |
| DOCUMENT | Référence DESIGN revue, décisions Q1–Q5, plan relu puis accepté avec chemins du lot | Avant implémentation. La présente mission ne fournit pas l'accord de coder. |
| AUTORISATION | Accord précis de session pour le lot, ses chemins et destinations, et la campagne correspondante | Avant toute mutation produit ou exécution concernée. Les dérogations FEAS et anciens lots Parakeet ne s'appliquent pas. |
| EXECUTION | Relever l'espace libre, arrêter sous 30 Gio ; vérifier Git et prérequis du profil retenu | Avant chaque lot/campagne ; aucune mesure historique ne remplace ce relevé. |
| EXECUTION / D3 | Identifier une garde applicable ; vérifier séparément racine Cargo partagée et racines d'écriture, refus des chemins hors racine par auto-tests, surveillance pendant commande | Avant toute commande Cargo, même metadata ou tests T1. Aucune garde générique n'est qualifiée ici ; aucune création ni dérogation n'est autorisée par ce plan. |
| EXECUTION | `CARGO_HOME`, `TEMP`, `TMP` canoniquement sous la racine d'exécution ; cache nécessaire copié sous cette racine, jamais écrit dans le profil utilisateur ; refuser absence/jonction hors racine | Avant Cargo. Proposition de racine : `target/parakeet-tests/micro-level/`, **à autoriser**, pas une destination activée. |
| EXECUTION | `CARGO_TARGET_DIR=target/parakeet-tests/cargo-shared/`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only` dans l'environnement | Avant Cargo ; ne pas modifier Cargo.toml pour ces variables. |
| DOCUMENT / D1 | Calculs et tolérances ARCH-v1.2 du cadrage, fixture sinus bin 64 et oracle Parseval pour cas généraux | Avant T5 ; détails techniques, aucun nouveau seuil de qualité vocale. |
| DOCUMENT / D2 | Corpus exact, référence/transcription attendue, durée/charge, voix faible/forte, variations et règle de comparaison confirmés avec l'utilisateur | Avant T8/T9 ; ne pas présenter la qualification qualité comme prête ou réussie auparavant. |

En cas d'échec de garde, build ou test, arrêter la campagne et préserver les preuves et changements ordinaires. Une écriture hors racine attribuable impose l'arrêt et l'arbitrage, pas une restauration/effacement improvisé. Aucun reset global, nettoyage de `cargo-shared`, suppression de corpus, modèle, transcription ou réglage personnel.

La clôture future respecte AGENTS.md : archivage des petites preuves en préservant arborescence, contrôle du nombre et SHA-256, puis retrait des seules copies/WAV temporaires d'un manifeste préalablement approuvé, et relevé d'espace final. Aucun de ces fichiers ou nettoyages n'est créé/exécuté par la rédaction actuelle.

## Périmètre de chemins du lot futur

Chemins relatifs à `C:\dev\whisper`. Cette liste décrit les éditions proposées à faire accepter, pas des modifications autorisées maintenant. Les fichiers nouveaux sont indiqués explicitement.

| Groupe | Chemins proposés | Justification |
| --- | --- | --- |
| Core | `crates/whisper-core/src/{microphone.rs,application.rs,ports.rs,path_settings.rs,lib.rs}` et nouveau `microphone_level.rs` dans ce dossier | T1 contrats, T3/T6 application, T4 réglages et exports. |
| Adapters | `crates/whisper-adapters/src/{capture.rs,direct_live.rs,parakeet_live.rs,worker_ipc.rs,path_settings.rs,lib.rs}` et nouveau `microphone_probe.rs` | T2 capture/conversion, T3 routage/drains, T4 stockage, T5 sonde, T6 test et exports. |
| Desktop | `crates/whisper-desktop/src/{ui.rs,root.rs,lib.rs,main.rs}` | T7 présentation et injections ; inclure `run()` et l'injection initiale du port, sans règle métier dans la composition. |
| Dépendance | `crates/whisper-adapters/Cargo.toml`, `Cargo.lock` | T5 déclaration directe Rustfft `=6.4.1` déjà verrouillé. Aucun téléchargement implicite. |
| Tests nouveaux | `crates/whisper-core/tests/microphone_level.rs`, `crates/whisper-adapters/tests/{microphone_level.rs,microphone_probe.rs}`, `crates/whisper-desktop/tests/microphone_level_ui.rs` | Contrats T1/T2, sonde T5 et présentation T7. |
| Tests existants éditables | `crates/whisper-core/tests/live_contract.rs`, `crates/whisper-adapters/tests/{live_capture.rs,microphone_settings.rs,model_preferences.rs}` | Intégration T3, compatibilité/mémoire T4. Fixtures privées strictement sous `cfg(test)` dans les modules déjà listés ; aucune API de production ajoutée uniquement pour les tests. |
| Extension acceptée pendant réalisation | `crates/whisper-adapters/tests/live_archive.rs`, `crates/whisper-core/tests/{model_selection_app.rs,parakeet_import_contract.rs}` | Accord utilisateur exact « Autoriser ces trois adaptations de tests » : seulement 11 initialisations `microphone_gain_db: 0` (8 + 2 + 1), sans changement d'assertion ; régressions correspondantes autorisées. L'édition anticipée reste consignée dans l'historique ci-dessous. |
| Documentation | `docs/cadrage/evo-micro-level.md`, `docs/cadrage/plan-evo-micro-level.md` | Référence/plan et résultats utiles, deux fichiers maximum prévus pour le rapport de ce lot. |

Sources de régression en lecture/exécution future : `crates/whisper-core/tests/{compute_policy.rs,import_contract.rs}`, `crates/whisper-adapters/tests/{import_cpu.rs,import_mp3.rs,parakeet_import.rs,live_archive.rs}`. Sources indirectes en lecture : `crates/whisper-adapters/src/{supervisor.rs,decoder.rs,archive.rs}`, workers et contrats moteur existants, manifests racine/core/desktop. Aucun changement de worker, d'API moteur ou de façade native prévu. Une modification devenue nécessaire hors liste revient au périmètre avant édition.

Précision de réalisation : `live_archive.rs`, initialement limité à lecture/exécution, bénéficie de la seule extension de fixtures acceptée ci-dessus ; ses assertions restent intactes.

Les destinations d'exécution sont distinctes des chemins d'édition. Les tests utilisent des profils/ports/fixtures isolés sous les racines autorisées, jamais les transcriptions ou réglages personnels réels. Aucun audio, modèle ou corpus préexistant n'est modifié.

## Neuf tâches proposées

Chaque tâche dépend aussi des prérequis communs. Tests automatiques futurs par `rust_implementer`, contrôle des preuves et du code par reviewer indépendant ; aucune tâche n'est marquée exécutée.

| ID | Entrées et dépendances | Travail, sortie et critère de fin |
| --- | --- | --- |
| **T1 — Contrats purs** | DOCUMENT ; D3 avant tests | Core : gain borné, source prévue/ouverte, contexte de sélection, révision et frontière, mesures/validité. Commandes/confirmations via ports purs. Tests : gain invalide et ancien contexte refusés, identités distinctes, silence/absence distingués. **CA-01 à CA-06**. |
| **T2 — Acquisition et conversion** | CODE T1 | Capture : restituer l'identité ouverte, charger le gain avant `play()`, marquer gain/révision/index au prochain bloc acquis. Appliquer A -> gain -> compteur -> B -> PCM existant. Tests numériques : neutre conforme, gain borné, backlog inchangé, comptes/ordre/durée exacts et drains. **CA-01/02/06/07**. |
| **T3 — Premier parcours complet** | CODE T2 ; EXECUTION tests T1/T2 réussis | Relier application/ports, `worker_ipc`, Turbo et Parakeet, y compris arrêt. Test de bout en bout à ports simulés : commande -> acquisition -> conversion -> WAV finalisé ; moteur retardé et arrêt sans gain rétroactif. Même PCM pour WAV et consommateurs. **CA-01/02/07**. |
| **T4 — Mémorisation** | CODE T1/T2 | Carte optionnelle identité -> gain v2, absence=0 dB. Préférence absente conservée, source de repli réglée sous sa propre identité. Entrée invalide neutralisée seule ; les deux producteurs préservent les autres préférences. Tests : deux IDs homonymes, profils v1/v2, sauvegardes désordonnées et erreur visible. **CA-05/06**. |
| **T5 — Sonde bornée** | CODE T2 ; D1 traduit en tests ; disponibilité hors ligne de Rustfft vérifiée | Dépendance directe et module sonde : RMS/pic/compteurs/PSD A/B, file séparée quatre blocs <=20 ms, fenêtres reconstruites après perte, instantanés partagés, publications bornées. Aucun calcul FFT/IO/UI dans le rappel. Tests numériques et deux lecteurs dont un suspendu ; aucune perte audio provoquée par la sonde. **CA-03/04/07**. |
| **T6 — Cycle du test** | CODE T3/T5 | Un seul propriétaire de flux. Test explicite sans moteur/fichier/haut-parleur ; arrêt/fermeture/erreur invalident immédiatement les mesures. Arrêter puis rouvrir pour transcription ; défaut changé : message et nouvelle action explicite. Vérifier absence de prétest dans WAV et absence de double ouverture. **CA-02/03/06/07**. |
| **T7 — Commande et jauge** | CODE T3/T4/T5/T6 | UI et injections : source prévue puis ouverte, gain demandé/appliqué, RMS/pic/alerte, états absence/silence/péremption. Réglage désactivé si cible non identifiée, source fixe en capture, sauvegarde échouée signalée. Tests UI avec horloge simulée et événements d'ancienne génération. **CA-01 à CA-06**. |
| **T8 — Validation du candidat** | CODE T7 ; EXECUTION vérifications précédentes réussies ; D2/D3 fermés | Régressions imports/politiques moteur, PCM/sorties, concurrence et migrations ; build du profil choisi. Confier le candidat à `rust_review_coordinator`, avec `rust_reviewer` indépendant et corrections par `rust_implementer`, deux cycles maximum. Sortie : candidat identifié et résultats attribués, distincts de la recette Windows. **CA-01 à CA-07**. |
| **T9 — Recette Windows utilisateur** | EXECUTION T8 ; D2 ; dossier/ressources de recette autorisés précisément | Vérifier les ressources du profil réellement construit, remettre le candidat. L'utilisateur vérifie deux micros, défaut/repli/reconnexion, gain pendant capture, jauge, volume Windows et sorties. Consigner résultats et limites ; aucune automatisation de la fenêtre. **CA-01 à CA-07**. |

Les contrats/fichiers communs interdisent d'annoncer une réalisation parallèle sans coordination. Graphe sans cycle : T1 -> T2 -> T3 ; T4 après T1/T2 ; T5 après T2 ; T6 après T3/T5 ; T7 après T3/T4/T5/T6 ; T8 après T7 ; T9 après T8. Les appels de tests de toute tâche dépendent de D3.

## Carte d'exécutabilité

Toutes les preuves produit de cette carte sont **NOT RUN**. Les chemins directs/indirects et les propriétaires sont ceux du tableau précédent ; les commandes envisagées sont précisées ensuite.

| Scénario | Tâches / données et stimulus | Observation et oracle |
| --- | --- | --- |
| V1 / CA-01/02 | T1/T2/T3 : PCM mono/stéréo connu, -24/0/+24 dB ; backlog imposé, changements puis arrêt | Facteur/limitation et révision/index corrects ; nombre/ordre/axe exacts ; pas de changement rétroactif ; neutre conforme à la référence. Volume Windows vérifié séparément par l'utilisateur. |
| V2 / CA-03/04 | T5/T7 : silence, amplitudes connues, fenêtre multirévision, horloge simulée avant/après 500 ms | RMS/pic <=1e-6 FS, dB hors silence <=0,001 dB ; silence distinct d'absence ; vieille session ou livraison tardive jamais présentée actuelle. |
| V3 / CA-04 | T5 : Hann périodique N=2048, sinus non saturé au **bin 64**, multitone et écrêtage | Pour cette fixture loin de DC/Nyquist seulement, puissance intégrée amplitude²/2 et rapport gain² sans clamp. Oracle général : Parseval pondéré du cadrage, erreur <=`1e-12 + 1e-6*référence`. Compteurs exacts aux limites et dépassements. |
| V4 / CA-02/04/07 | T5/T8 : deux lecteurs dont un suspendu, débordement de la file sonde, reprise et charge moteur disponible | Aucun blocage audio/autre lecteur ; pertes sonde comptées, fenêtre invalidée puis reconstruite ; nombre audio exact ; buffers bornés, pas de FFT/IO/UI dans le rappel. |
| V5 / CA-05/06 | T4/T6/T7 : profils v1/v2, IDs homonymes A/B, défaut changé, A absent, consultation impossible, ancien contexte, sauvegardes désordonnées/échouées | Gains jamais réattribués ; choix préféré conservé ; gain de repli propre ; aucun gain modifiable sans cible ; deux producteurs préservent les autres champs et accusé d'échec visible. |
| V6 / CA-03/06/07 | T6/T7 : test -> arrêt -> capture, défaut changé entre ouvertures, échec d'accès, fermeture et déconnexion | Aucun prétest enregistré, aucun moteur/fichier en test, aucun double flux ni bascule automatique ; cause visible, jauge invalidée et flux libéré. |
| V7 / CA-07 | T8/T9 : corpus D2, Turbo/Parakeet et CPU/GPU/Auto disponibles, voix faible/forte et gain abrupt | Référence 0 dB, continuité, WAV/TXT/SRT utilisables, imports témoins et politiques moteur inchangés selon D2. Distinguer moteur demandé/effectif et scénarios exécutés/indisponibles. |

Coût proposé : fixtures synthétiques <=1 million d'échantillons chacune et budget cumulé cible <=10 minutes hors compilation. Ces bornes ne sont ni des performances observées ni un nouveau critère du produit. Budget compilation à estimer avant la campagne ; corpus réel/durée/charge relèvent de D2. Une limite atteinte conduit à un bilan et à une décision de reprise, pas à des relances indéfinies.

## Commandes futures et remise

Cwd `C:\dev\whisper`, PowerShell. Commandes **proposées uniquement**, derrière la garde applicable et après tous les prérequis ; aucune commande Cargo directe de campagne :

```text
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-core --test microphone_level --test live_contract --test compute_policy --test import_contract
cargo test --offline --locked -p whisper-adapters --test microphone_level --test microphone_probe --test live_capture --test microphone_settings --test model_preferences
cargo test --offline --locked -p whisper-desktop --test microphone_level_ui
cargo test --offline --locked -p whisper-adapters --lib capture::tests
cargo test --offline --locked -p whisper-adapters --test import_cpu --test import_mp3 --test parakeet_import --test live_archive
```

Préciser au préflight les filtres complémentaires des tests unitaires de drain Turbo/Parakeet, les variables et ressources requises par les scénarios natifs. Une commande verte avec scénario ignoré ou non activé ne prouve pas ce scénario : rapporter les sous-cas réellement exécutés, PARTIAL ou NOT RUN. Ne pas étendre les essais sans besoin nouveau après succès des vérifications requises.

Si le profil personnel est choisi après autorisation, la commande de build envisagée est :

```text
cargo build --release --offline --locked -p whisper-desktop -p whisper-worker-cpu -p whisper-worker-gpu --features whisper-desktop/personal-install
```

Vérifier les dépendances natives déjà requises sans installer ou remplacer implicitement un runtime. Profil/commandes Test à déterminer si ce profil est retenu ; ne pas annoncer une recette complète d'un profil non choisi.

T9 applique AGENTS.md : dossier à autoriser précisément ; pour le profil personnel, seuls les deux workers Turbo accompagnent le desktop. Vérifier présence et SHA-256 de ces workers et des ressources Parakeet réellement chargées depuis l'installation stable (deux workers, cinq/neuf DLL et deux modèles). Pour un profil Test, relever tous les chemins effectifs et remettre au besoin un lanceur dans le dossier autorisé. Ne pas déclarer le dossier autonome si des ressources externes sont utilisées.

## Revue et état de livraison documentaire

- DESIGN : `rust_design_reviewer`, CLEAN sur `d10516808804f461ca1597b78d0d176f3f3baa27`, R-MIC-D01 fermé ; conseil de fixture bin 64 intégré dans les deux documents.
- PLANS : `rust_design_reviewer`, **CLEAN sur `316de9bcce2d8aeef85098e4e45e0a8b6689b7ae`**, après lecture directe des deux fichiers Git ; aucun constat requis ouvert. Couverture CA-01 à CA-07 / T1 à T9 / V1 à V7, chemins indirects, graphe et prérequis contrôlés. Le conseil spectral est fermé par la fixture bin 64 ; aucun changement de Q1–Q5. Le présent ajout consigne uniquement le verdict, sans modifier les tâches ou contrats.
- Acceptation et autorisation de réalisation : **reçues le 25 octobre 2026**, selon l'accord précis consigné dans le statut ci-dessus. Les mentions de propositions/non-autorisation dans le corps décrivent la rédaction initiale ; les chemins d'édition sont désormais ceux du lot accepté. La garde doit encore satisfaire D3 avant toute commande Cargo ; aucune ancienne dérogation ne s'applique.
- D2 : l'utilisateur a répondu « Oui, utiliser ce corpus et cette méthode » pour les quatre WAV FR/EN existants de `docs/cadrage/feas-pm-01-corpus/manifest.json`, associés aux quatre références `candidate_*_copie.txt` corrigées. Référence à 0 dB, variantes faible/forte et changement de gain pendant le flux ; comparer Turbo/Parakeet sur les moteurs disponibles, continuité, durée et texte, sans nouveau seuil WER. Les sources restent intactes. Les paramètres précis et les cas exécutés seront rapportés ; cet accord n'est pas une preuve de qualité.
- Profil de remise choisi par l'utilisateur : **profil personnel existant**. Les ressources installées restent utilisées en lecture ; le dossier de recette éventuel reste sous `target/parakeet-tests/micro-level/`.
- Préflight initial de réalisation : HEAD `b2808898001056f6d63ea9daf505979d3cda2ccd`, branche `evo-startup-perf`, aucun changement suivi ; `models/` et `sound example/` non suivis à préserver. Espace libre C: relevé à 134 716 538 880 octets (environ 125,5 Gio). Lectures et contrôles Git uniquement à ce stade ; aucune preuve de compilation, capture ou test produit acquise.
- Vérification D2 en lecture seule : les quatre WAV et les quatre références `human_approved_reference` correspondent aux SHA-256 du manifeste existant. Durées : FR pauses 65,02 s, EN pauses 65 s, FR continu 60 s, EN continu 60 s. Cette vérification d'identité ne constitue pas une comparaison vocale.
- Préparation de remise personnelle en lecture seule : présence et SHA-256 relevés pour les deux workers Parakeet de `%LOCALAPPDATA%/Whisper/worker/`, les cinq DLL CPU et neuf DLL GPU de `native/`, et les deux modèles de `models/`. Aucun fichier de l'installation n'a été modifié. La vérification des deux workers Turbo du nouveau binaire et la recette utilisateur restent à effectuer.

### Arrêt de la première campagne de réalisation

- Garde adaptée sous `target/parakeet-tests/micro-level/guard-micro-level.ps1` ; auto-tests annoncés réussis (exit 0), y compris refus de variables hors racine et événement interdit synthétique sans création effective hors racine. La surveillance n'est pas un confinement préventif.
- `Fmt` via garde : exit 1, diagnostic uniquement de formatage préexistant dans `crates/whisper-core/src/model_selection_app.rs:144`, hors périmètre ; diagnostic dans `logs/Fmt.stdout.log`. Aucun succès de formatage global n'est annoncé.
- `CoreLevel` via garde : campagne interrompue avant tout résultat de test. Création observée de `C:/Users/cyril/AppData/Local/Temp/etilqs_jhCWsj2WNLO1hcS` à `2026-10-25T17:59:12.7919744+01:00`, détection à `17:59:12.8300323+01:00`, action `process-tree-terminated`, exit 1. Le watcher ne démontre ni processus auteur ni provenance ; aucune attribution à Cargo ou à un service externe n'est établie. Aucune ancienne dérogation n'est appliquée.
- Preuves conservées sous `target/parakeet-tests/micro-level/logs/` : `guard-events.jsonl`, `guard.jsonl`, `CoreLevel.stdout.log`, `CoreLevel.stderr.log`. Aucun effacement ni restauration du fichier temporaire observé ; son absence lors de la lecture ultérieure n'établit pas son auteur.
- État : T1/T2 partiels, T3 à T9 non réalisés, aucun test PASS, aucun binaire remis, aucune revue de candidat complet. Le contrôle des processus après arrêt n'a retourné aucun `cargo.exe` ou `rustc.exe`. Espace libre après arrêt : 133 871 005 696 octets (environ 124,7 Gio).
- Écart de périmètre observé après arrêt : `crates/whisper-core/src/model_selection_app.rs` présente désormais un changement de mise en forme de l'expression `matches!` (5 lignes en une), alors que le fichier était propre à la base. L'implémenteur confirme que `rustfmt` lancé sur le `lib.rs` autorisé a parcouru récursivement ses modules et touché ce fichier hors périmètre. Ce changement est attribué à l'implémenteur, sans modification fonctionnelle relevée dans ce diff de formatage ; aucune restauration automatique n'est effectuée. Une reprise devra empêcher la récursion hors des chemins autorisés.

### Arbitrage de reprise

- Le 25 octobre 2026, dans cette même session, l'utilisateur répond « oui je t'autorise » aux deux actions proposées : reprendre uniquement ce lot en consignant sans arrêt automatique les événements du Temp utilisateur dont l'origine reste indéterminée, et annuler le seul changement de formatage hors périmètre identifié ci-dessus.
- Aucune destination applicative volontaire n'est ajoutée. Toute écriture extérieure attribuée à la campagne impose toujours l'arrêt ; la dérogation ne couvre aucun autre emplacement. Préflight, auto-tests, variables locales et surveillance restent obligatoires. Les événements indéterminés sont conservés comme tels et ne deviennent ni des écritures Cargo prouvées ni des écritures autorisées. La surveillance reste distincte d'un confinement préventif.
- Cette autorisation permet la reprise des vérifications prévues ; elle n'est pas leur résultat. Aucun résultat de la campagne interrompue n'est présenté comme PASS.

### Vérifications après reprise — état intermédiaire

- Le seul formatage hors périmètre de `model_selection_app.rs` a été annulé ; le contrôle Git confirme un diff vide pour ce fichier. Le défaut de formatage initial reste distinct et n'est pas corrigé dans ce lot.
- Garde réadaptée et auto-tests réussis : seuls les événements d'origine indéterminée sous le Temp utilisateur initial relèvent de la dérogation. Les signatures de noms évoquant la campagne provoquent un arrêt conservateur `SUSPECTED_CAMPAIGN`, sans prétendre constituer une attribution par processus. Les autres chemins extérieurs restent refusés. Les sorties sont uniques par lancement pour préserver les premiers logs.
- `CoreLevel-20261025T180238161-f85e2fb2` : exit 0, 23 tests réussis (3 + 11 + 6 + 3), aucun ignoré. Cette preuve couvre seulement le candidat intermédiaire testé ; des changements ultérieurs peuvent exiger sa réactualisation.
- Première compilation `CaptureTests-20261025T180341698-f1147b1a` : exit 101, erreurs E0615/E0382 dans le delta de `capture.rs`. Campagne arrêtée, traces conservées ; corrections locales autorisées par l'orchestrateur dans le périmètre accepté, sans nouvelle dérogation.
- Nouvelle campagne `CaptureTests-20261025T180444515-0c7be309` : exit 0, 8 tests réussis, 2 ignorés nécessitant du matériel, 156 hors filtre. `AdapterLevel-20261025T180533856-b23c59f4` : exit 0, 12 tests réussis (3 live_capture, 1 microphone_level, 2 microphone_settings, 6 model_preferences). Les essais matériels ignorés ne sont pas prouvés.
- Disponibilité hors ligne de Rustfft 6.4.1 annoncée vérifiée dans le cache local du lot ; aucun téléchargement effectué. T3 à T9 restent à réaliser ou à vérifier ; aucune livraison complète ni revue CLEAN n'est annoncée.

### Progression intermédiaire T3 à T5

- T3 : commande core `CoreGain` réussie (1 test) ; `LiveGain-20261025T181322043-8e2a6493` réussie (1 test), chaîne application simulée -> effet runtime -> acquisition -> conversion -> arrêt/WAV finalisé alors que le moteur n'est pas prêt. Un premier échec de fixture de recyclage des blocs est conservé séparément. Cela ne prouve pas encore tous les cas de régression ou une capture Windows.
- Auto-test de garde : un échec de retrait de la jonction de test a été conservé, puis corrigé par suppression non récursive du seul lien vérifié. La cible `models/` est restée présente. L'invocation est ensuite effectuée avec PowerShell 7 déjà installé ; aucun outil n'a été installé. Les auto-tests suivants ont réussi.
- T4 : 4 tests core de réglages réussis après correction des nouveaux faux ports ; suites `AdapterLevel` réussies avec racine de profils de test sous `micro-level/`, dont 2 tests microphone_settings et 6 model_preferences réellement activés. Compatibilité/migration et conservation de la carte sont vérifiées à ce stade ; le contrôle des confirmations réelles anciennes reste à consolider.
- T5 : `ProbeUnit-20261025T182812508-e83d5496` réussie (5 tests), incluant puissance Hann bin 64, Parseval multitone, rapport gain² et révisions mixtes. Les tests ont révélé puis permis de corriger un défaut produit : une publication spectrale ne doit pas effacer le RMS courant. Tests d'intégration de sonde : 3 réussis, dont deux lecteurs et débordement/reconstruction. Tests de capture : 9 réussis, 2 matériels ignorés, dont égalité exacte du PCM et de sa fin avec/sans collecte.
- Ces preuves sont intermédiaires ; elles seront réévaluées si les éditions suivantes les affectent. Le cycle test/UI T6/T7, la validation complète T8, le build personnel et la recette T9 ne sont pas annoncés réalisés. Aucun verdict de revue indépendante n'a encore été demandé.

### Checkpoint figé T1 à T7 avant revue

- Candidat : working tree sur la base `b2808898001056f6d63ea9daf505979d3cda2ccd`, sans commit produit. Les sources sont figées pendant la revue indépendante déclenchée via `rust_review_coordinator` et `rust_reviewer` ; aucun verdict encore acquis. Le build personnel est en cours sous garde, sans installation ni fenêtre lancée.
- Dernières suites ciblées annoncées réussies : CoreLevel (dont 6 live_contract et 4 microphone_level), CoreGain (3), CoreSettings (4), AdapterLevel (dont 1 microphone_level, 2 microphone_settings et 6 model_preferences), Probe (3), ProbeUnit (5), CaptureTests (9 réussis, 2 matériels ignorés), LiveGain (2), DesktopLevel (2). Les logs uniques sous `micro-level/logs/` déterminent les sous-cas et le candidat de chaque preuve ; aucun succès synthétique n'est une preuve de capture matérielle.
- T6 : flux synthétique de test sans moteur/worker/WAV, identité, gain initial/révisions, invalidation sur stop/erreur, propriétaire unique, test -> arrêt -> capture et rejet d'ancienne génération contrôlés dans les tests ciblés. T7 : présentation attente/absence/silence/péremption strictement après 500 ms, avis de défaut changé et nouvelle action explicite contrôlés. Les opérations matérielles et la fenêtre Windows restent non exécutées.
- Incident de périmètre : l'implémenteur a ajouté trop tôt `microphone_gain_db: 0` à onze constructions de requêtes dans trois fichiers hors liste d'édition : 8 dans `crates/whisper-adapters/tests/live_archive.rs`, 2 dans `crates/whisper-core/tests/model_selection_app.rs`, 1 dans `crates/whisper-core/tests/parakeet_import_contract.rs`. Aucune assertion changée. Leur adaptation est motivée par le nouveau champ requis de `LiveRequest`. Ces changements sont conservés et figés ; la demande précise d'extension est soumise à l'utilisateur, sans réponse à ce checkpoint. Ces trois cibles ne sont pas exécutées avant accord.
- D2 : corpus et méthode acceptés, identités vérifiées, mais aucune comparaison avec les vrais moteurs n'a été exécutée. Les anciens scripts VAD ne couvrent pas ce parcours et ne sont pas présentés comme preuve de qualité. La recette T9 reste non exécutée et réservée à l'utilisateur.

## Premier bilan de réalisation, avant la reprise R7 (historique)

### Verdict et état du lot

**LIVRÉ AVEC RÉSERVES**, candidat de travail non commité sur la base `b2808898001056f6d63ea9daf505979d3cda2ccd`. Trois passes de revue, deux cycles de correction, limite atteinte. Le `rust_reviewer` indépendant ferme **R1 à R6** ; **R7 reste ouvert**. Le candidat personnel courant compile, mais T8/D2 n'est pas validé et T9 reste à l'utilisateur. Aucun push, installation, ouverture automatisée de fenêtre ou effacement de source/corpus/réglage personnel n'a eu lieu.

| Tâches | État de réalisation et preuve | Limite |
| --- | --- | --- |
| T1 à T7 | Code et suites ciblées réalisés ; R1 à R6 fermés par reviewer après correction. Gain par bloc, acquittement après acquisition, identité, réglages et sonde testés sur scénarios simulés. | Deux tests matériels ignorés ; le parcours Windows réel reste à observer. |
| T8, régressions | CoreLevel 24 tests, CoreFixtureRegression 13, AdapterLevel 12, DirectLiveTests 15, CaptureTests 10 réussis et 2 ignorés ; suites d'accusé/UI/sonde/réglages ciblées réussies sur les candidats concernés. ImportRegression : 24 réussis et 8 ignorés. | `import_cpu` 5 exécutés (~29 s), `import_mp3` 4, `live_archive` 8, `parakeet_import` 7 exécutés et 8 ignorés ; ces derniers ne prouvent pas les chemins natifs ignorés. |
| T8, revue | R1 identité/gain à l'ouverture, R2 accusé au premier bloc acquis, R3 demandes durant ouverture, R4 restauration partielle JSON, R5 frontière avant copie, R6 cadence RMS : fermés. Dernier test R3 `CoreGain-20261025T192456050-6f135a25`, 4 tests réussis. | R7 du harnais D2 reste ouvert, aucun verdict CLEAN du lot. |
| T8, build personnel | `PersonalRelease-20261025T194551209-7a3c0f8d`, exit 0 en 29 s, après SelfTest exit 0 ; code courant, profil `personal-install`. | Un build réussi ne valide ni R7, ni D2, ni la fenêtre Windows. |
| T8, qualité D2 | Campagne CPU autorisée et engagée, arrêtée sur constat de revue. | Preuves de gain invalides ; voir détail ci-dessous. |
| T9 | Dossier de recette personnel courant préparé, ressources installées vérifiées en lecture seule. | Aucune observation de l'utilisateur dans la fenêtre à ce stade ; comparaison GPU réservée à cette recette. |

Le formatage ciblé passe avec `skip_children=true` et `git diff --check` passe. Le contrôle global `cargo fmt --all -- --check` reste en échec sur le seul formatage préexistant de `crates/whisper-core/src/model_selection_app.rs:144`, hors périmètre ; ce fichier n'a plus de diff. Les échecs de compilation/tests corrigés sont conservés séparément ; ils ne sont pas présentés comme des réussites historiques.

### Campagne D2 autorisée, puis arrêtée sur R7

- Accord utilisateur précis : « Autoriser cette campagne CPU complète » pour 32 transcriptions sérialisées = quatre WAV approuvés × Turbo CPU/Parakeet CPU × 0 dB, −6 dB, +6 dB, transition −6→+6 dB à mi-parcours. Plafond de 60 minutes, arrêt et bilan s'il est atteint ; sorties sous `target/parakeet-tests/micro-level/d2/`, aucun nouveau seuil WER. Deuxième décision : « Réserver la comparaison GPU à ma recette ». Aucune destination GPU hors racine n'est autorisée.
- Le harnais privé `cfg(test)` et le mode de garde `D2Corpus` sont ajoutés dans les modules déjà autorisés. Les ressources CPU installées et les corpus sont utilisés en lecture seule. Une première compilation a échoué sur une conversion de SHA de modèle, corrigée localement avec trace conservée ; aucune donnée vocale n'avait alors été traitée.
- Run de données `D2Corpus-20261025T193558669-df0676a2`, invocation à 19:35:58 environ, exit −1 après **475 s**, arrêt dirigé à la suite du constat du reviewer, **pas un dépassement du plafond de 60 minutes**. Trois cas Turbo CPU FR pauses terminés (0/−6/+6 dB) ; cas 4 transition commencé puis interrompu ; 28 autres non lancés. Les trois sorties ne sont pas comptées PASS pour D2. Journaux, `d2/checkpoint.jsonl` et résultats partiels conservés ; aucun worker/Cargo résiduel.
- **R7 IMPORTANT, ouvert** : `CaptureStream::enqueue_test_block` applique/limite déjà le gain aux échantillons du flux simulé ; `Pcm16Converter` l'applique à nouveau. Les variantes annoncées −6/+6 deviennent donc approximativement −12/+12 dB, avec limitation prématurée ; la sonde A reçoit un signal déjà réglé. Ce helper est réservé aux tests et n'est pas utilisé par le rappel audio réel. Le constat invalide la preuve T8, sans démontrer un double gain dans le produit.
- Correction concrète proposée, **non exécutée** : injecter le PCM brut dans le helper, transporter seulement gain/révision/indice ; laisser le convertisseur appliquer le gain une fois. Prouver A brut, B/WAV à un seul facteur et la limitation au bon endroit, puis rejouer les cas D2 invalidés. Les deux cycles de revue/correction étant épuisés, un nouvel accord utilisateur précis est requis avant cette correction ou relance. Les 475 secondes de campagne déjà consommées doivent être prises en compte dans toute décision de budget de reprise.

### Incidents de campagne expliqués et conservés

Le premier PersonalRelease a été arrêté par la garde sur une modification de `plan-evo-micro-level.md` à 18:52:00, effectuée par la session principale pendant la surveillance. L'origine est expliquée : erreur de coordination documentaire, sans attribution à Cargo. Les preuves et binaires partiels ont été conservés. Après gel des documents, la relance epoch0 a réussi ; le build courant ci-dessus la remplace pour la recette, après corrections. Aucun événement du dépôt sensible n'a été ignoré pour contourner la garde. Les événements Temp indéterminés restent consignés selon la seule dérogation de cette session ; ils ne constituent pas un confinement ni une attribution de processus.

### Remise personnelle et clôture des preuves

Dossier courant : `target/parakeet-tests/micro-level/recette-personnelle/`, contenant **exactement trois EXE**, chacun identique par SHA-256 au résultat Cargo du dernier build :

| Fichier | Octets | SHA-256 |
| --- | --- | --- |
| `whisper-desktop.exe` | 14 270 976 | `D135D05E5F4F0DFD309321605D3A03EDB818C887D4DA18B0BA440E6B1FDF1A86` |
| `whisper-worker-cpu.exe` | 20 809 728 | `3613ADC5884C5D28BCB85992647233E6F72653EC93FC205E1ECCC9284C393E7A` |
| `whisper-worker-gpu.exe` | 20 882 432 | `3537A0E5E1B1E1A20316651AD2156E32213C67A23B78DA0DFD0E152CE2D24234` |

Ce dossier utilise les ressources stables de `%LOCALAPPDATA%/Whisper/` : deux workers Parakeet dans `worker/`, cinq DLL CPU et neuf DLL GPU dans `native/`, deux modèles dans `models/`. Présence et SHA-256 revérifiés en lecture seule avant remise ; aucune copie Parakeet ou DLL superflue ajoutée. Le dossier n'est pas autonome et rien n'a été installé ni lancé. La recette future devra appliquer les observations de T9 (deux micros, défaut/repli/reconnexion, gain pendant capture, jauge, volume Windows inchangé, sorties) et la comparaison GPU choisie par l'utilisateur ; ce bilan ne prétend pas les avoir exécutées.

Petites preuves archivées sous `target/parakeet-tests/micro-level/archives/t8-evidence-20261025-194703/` : **177 fichiers, 873 685 octets**, arborescence conservée, taille et SHA-256 source/destination vérifiés pour chaque fichier. Aucun WAV ni cache Cargo inclus ; aucun manifeste supplémentaire créé. Aucune suppression de temporaire/copie ou nettoyage de `cargo-shared` effectué. Espace libre final relevé : **130 713 419 776 octets** (environ 121,7 Gio). Surveillance et processus de campagne terminés avant mise à jour de ce bilan.

## Bilan de la reprise R7 et du premier rejeu CPU (historique)

### Accord précis et fermeture de R7

L'utilisateur autorise dans cette session : **« Oui, je t’autorise un cycle supplémentaire limité à R7, puis le rejeu des 32 cas CPU avec un plafond supplémentaire de 52 minutes. »** Cet accord constitue la seule exception à la limite précédente : aucun cycle de correction autre que R7 ni changement de logique produit supplémentaire n'en découle.

Correction limitée à `crates/whisper-adapters/src/capture.rs` (injection du PCM brut, gain/révision comme métadonnées) et `crates/whisper-adapters/src/direct_live.rs` (preuve privée). `CaptureTests-20261025T195324467-f855d6ca` : 10 réussis, 2 ignorés matériels ; `DirectLiveTests-20261025T195344880-f6847e14` : 16 réussis, aucun ignoré, dont `test_capture_gain_preserves_probe_a_and_applies_once_to_wav_pcm`. Ce test prouve A égal à la source, B après un seul facteur +6 dB avec limitation et PCM WAV final attendu. SelfTest avant chaque Cargo, formatage ciblé et `git diff --check` réussis.

Le **même reviewer indépendant ferme R7** sur ce candidat (epoch4). **R1 à R7 sont donc fermés**. La validation globale reste **BLOCKED/PARTIAL sur D2 Parakeet**, et non CLEAN : un défaut du harnais corrigé ne vaut pas qualification des moteurs. Aucun correctif supplémentaire n'est appliqué après l'échec décrit ci-dessous.

### Rejeu D2 : 16 sorties Turbo, premier Parakeet en échec

| Élément | Observation |
| --- | --- |
| Run | `D2Corpus-20261025T195639074-9f3cb469`, sources Rust figées, nouvelle racine `target/parakeet-tests/micro-level/d2/r7-replay/`. |
| Limite | Garde adaptée au seul délai D2 : **3 120 000 ms (52 min)**. Départ 19:56:39, échéance 20:48:39 ; aucune remise à zéro. Le checkpoint du test conserve une ancienne métadonnée `timeout_seconds=3600` ; le plafond effectif est celui de la garde externe. |
| Résultat | **Exit 101 après 1 882 s**, arrêt sur échec au cas 17, **pas un timeout**. Aucune reprise de cas 17 ni lancement des cas 18 à 32. |
| Turbo CPU | **16 cas `case-complete`**, quatre WAV × quatre variantes ; WAV/TXT/SRT et références conservés. Total **16 001 280 échantillons en entrée et en sortie**, tous les comptes individuels égaux. Durées de cas environ 96,7 à 142,9 s. |
| Transitions | Deux révisions par cas abrupt : −6 dB à l'indice 0, +6 dB au milieu exact (FR pauses 520 160, EN pauses 520 000, continus 480 000). Les autres variantes appliquent le gain annoncé à l'indice 0. |
| Parakeet CPU | Cas 17, FR pauses, 0 dB : panic `Parakeet CPU returned no segments`. **Zéro cas Parakeet qualifié**, 15 autres non exécutés. |
| Preuves cas 17 | WAV de 1 040 320 échantillons (2 080 684 octets), TXT/SRT incomplets vides ; JSON `complete=false`, aucun intervalle confirmé, tout l'audio non résolu, transcription pending. `d2-engine-status.json` absent. |

La comparaison descriptive des textes Turbo reste distincte d'un verdict qualité : sur FR pauses, une séquence introductive présente à 0 dB manque à −6 dB et dans la variante commençant à −6 dB ; les sorties anglaises montrent aussi des différences de ponctuation et de découpage. Aucun nouveau seuil WER ni équivalence de qualité n'est déclaré. Les textes, références et sorties sont disponibles dans le checkpoint et les dossiers des 16 cas. Les comptes PCM corrects ne prouvent pas la qualité du texte ni une capture matérielle Windows.

### Diagnostic en lecture seule de l'échec Parakeet

Le chemin configuré est `C:/Users/cyril/AppData/Local/Whisper/worker/whisper-worker-parakeet-cpu.exe`, présent, SHA-256 `84C371D4DF55043ADA4A1D96B80180ED0C03B208410C599262C8EA3762304E60`. Le modèle installé `parakeet-tdt-0.6b-v3.q8_0.gguf` correspond au hash attendu ; les ressources et WAV ont été lus sans modification. Le code de test crée le parcours de capture puis un thread qui devrait appeler `ParakeetProcess::spawn_live`, mais **les preuves conservées n'attestent pas l'invocation effective du binaire** (pas de PID, de Ready ou de journal de lancement). Le dossier de session est vide. Le stderr du worker personnel CPU est redirigé vers `Stdio::null()` et le stdout IPC n'est pas archivé.

Le harnais ne conserve que certains événements de `live.poll()` et panique sur l'absence de segments avant d'écrire le statut d'arrêt. Le `diagnostic=null` du JSON décrit la transcription automatique finale en attente ; ce n'est pas le diagnostic perdu de `ParakeetCaptureStopped`. **L'origine demeure indéterminée** : invocation/Ready/initialisation native ou résultat vide ne sont pas départagés. Aucun défaut du produit Parakeet n'est établi par ce test échoué.

Plus petite étape proposée, **non appliquée** : dans le harnais de `direct_live.rs`, enregistrer événements, diagnostic d'arrêt, `complete` et compte d'intervalles/segments avant les assertions ; compléter les traces de test de `parakeet_live.rs` pour le lancement/Ready/Failed (PID existant si disponible). Aucune modification de logique produit ou nouvelle frontière native n'est proposée comme autorisée. Une nouvelle décision utilisateur doit définir ce diagnostic ciblé et une éventuelle reprise des **16 seuls cas Parakeet**, sans rejouer les sorties Turbo déjà produites. Sur les 52 minutes supplémentaires autorisées, 1 882 secondes ont été consommées par le run échoué (reste nominal 1 238 secondes, sans relance effectuée).

### Provenance, remise courante et archives

D2 Turbo a réellement exécuté le worker **installé** `C:/Users/cyril/AppData/Local/Whisper/app/whisper-worker-cpu.exe` avec le modèle installé `ggml-large-v3-turbo.bin`. Il n'a pas exécuté les nouveaux workers Turbo du dossier de recette. Les sources des workers ne changent pas dans ce lot ; les preuves sur les ressources installées et sur les EXE recompilés sont rapportées séparément.

Après l'échec, sans modification Rust, `PersonalRelease-20261025T203252434-073359fe` a réussi sous garde (SelfTest exit 0, build exit 0 en **27 s**). Dossier `target/parakeet-tests/micro-level/recette-personnelle/`, exactement trois EXE identiques par SHA-256 aux sorties du build :

| Fichier | Octets | SHA-256 courant |
| --- | --- | --- |
| `whisper-desktop.exe` | 14 270 976 | `691AF60D977D525B1823B5B5ABF75AEBE49F61DA6A0F68A63BFB03B88F8981FE` |
| `whisper-worker-cpu.exe` | 20 809 728 | `3613ADC5884C5D28BCB85992647233E6F72653EC93FC205E1ECCC9284C393E7A` |
| `whisper-worker-gpu.exe` | 20 882 432 | `3537A0E5E1B1E1A20316651AD2156E32213C67A23B78DA0DFD0E152CE2D24234` |

Ressources Parakeet/CPU/GPU/modèles installées revérifiées en lecture seule ; rien installé ni lancé dans la fenêtre. La recette reste dépendante de cette installation, sans copies Parakeet ou DLL superflues. La comparaison GPU reste réservée à l'utilisateur selon son choix explicite.

Archive supplémentaire `target/parakeet-tests/micro-level/archives/d2-replay-r7-epoch4-20261025-203402/` : **214 fichiers, 1 484 539 octets**, tailles et SHA-256 source/destination vérifiés individuellement ; 177 fichiers de logs/preuves, 36 petites sorties D2 et le garde. Aucune inclusion des 17 WAV ni d'un cache, aucun manifeste ajouté ; anciennes archives et résultats invalides conservés. Aucune suppression ni nettoyage du target partagé. Espace final relevé **130 584 543 232 octets** (environ 121,6 Gio). Tous les processus de campagne et la surveillance sont terminés avant cette mise à jour documentaire. Sources/code non commités ; pas de push.

## Bilan courant : diagnostic du harnais Parakeet et réserve produit R8

### Autorisation et périmètre réalisé

L'utilisateur répond **« je t'autorise »** à la proposition d'**un cycle limité au diagnostic/correctif du seul harnais Parakeet CPU, sans modifier la logique produit, puis la reprise des 16 cas Parakeet avec un plafond de 20 minutes, sans rejouer Turbo**. Modifications limitées aux tests et traces de `direct_live.rs`/`parakeet_live.rs`, et à la configuration du garde ; aucun changement du protocole produit ou des workers autorisé par cet accord.

Le harnais enregistre désormais `d2-engine-status.json` et tous les événements de `live.poll()` **avant** les assertions sur les segments/texte. Une trace de test ajoute chemin configuré, PID après spawn réussi, Ready/Failed, résultats de fenêtres, erreurs et état du processus. Les sorties vont uniquement sous `target/parakeet-tests/micro-level/d2/parakeet-replay/`. Garde : filtre exact `parakeet-cpu`, **16 cas sérialisés**, délai **1 200 000 ms**, rejet pré-Cargo d'un filtre Turbo ; auto-test de garde et test synthétique de statut réussis.

Le même reviewer a contrôlé le harnais et son périmètre. Une compilation release a ensuite signalé six E0609 : six appels de trace utilisaient un champ réservé aux tests hors de `cfg(test)`. Cette erreur d'instrumentation introduite dans ce cycle a été corrigée en protégeant ces six appels par `#[cfg(test)]`, sans changer le contrat `NativeCallStarted`/`Result` ni le comportement attendu CPU/GPU. Le reviewer valide l'isolation cfg et le build ; certains appels du thread restent syntaxiquement présents avec une méthode no-op hors tests, sans preuve d'un coût CPU optimisé strictement nul. Aucun nouveau run natif n'a suivi cette correction de compilation.

### Résultat du run diagnostique

- `D2Corpus-20261025T204718998-919dc125` : **exit 101 après 2 s**, invocation à 20:47:18.384 ; arrêt sur le premier cas Parakeet CPU/FR pauses/0 dB. Ce n'est pas un dépassement de la limite de 20 minutes. **15 autres cas non exécutés**, aucune relance ; les 16 sorties Turbo antérieures sont préservées sans rejeu.
- Preuves sous `d2/parakeet-replay/01-parakeet-cpu-fr_pauses-0db/` : statut d'arrêt `complete=false`, zéro segment confirmé, intervalle `[0, 1 040 320]` non résolu, diagnostic « une ou plusieurs hypothèses n'ont pas été produites ». Gain 0 dB/révision 0/indice 0 consigné ; le WAV est préservé, TXT/SRT incomplets vides.
- `d2-worker-events.jsonl` atteste cette fois l'invocation effective de `C:/Users/cyril/AppData/Local/Whisper/worker/whisper-worker-parakeet-cpu.exe`, **PID 38872**, puis **Ready CPU**, profil `import-single-pass`, hash worker `84C371D4DF55043ADA4A1D96B80180ED0C03B208410C599262C8EA3762304E60`. Le modèle installé et le WAV approuvé sont lus sans mutation.
- Première fenêtre : `worker-transcribe-error` « Réponse Parakeet tardive ou étrangère à la fenêtre ». Le processus était encore actif avant `terminate` ; le code 1 est relevé après terminaison. Aucun crash d'initialisation n'est donc déduit de ce code seul. Le stderr personnel CPU reste redirigé vers `Stdio::null()` et la trame IPC rejetée n'est pas reproduite intégralement dans la trace ; les événements décodés et les statuts disponibles sont conservés.

### R8 : incompatibilité de contrat produit confirmée par le reviewer

**R8 HIGH/BLOQUANT, CA-07/V7**, réserve ouverte. La lecture indépendante des sources établit que le worker CPU n'émet `NativeCallStarted` que dans la branche `is_gpu_worker` de `crates/whisper-worker-parakeet-cpu/src/main.rs`. L'adaptateur, dans `ParakeetProcess::transcribe_window` de `crates/whisper-adapters/src/parakeet_live.rs`, exige pourtant `native_call_started` avant d'accepter un `WorkerEvent::Result`, quel que soit le backend. Un résultat CPU corrélé sans cet événement est alors rejeté par la branche générique « tardive ou étrangère ».

Le reviewer a vérifié que cette contradiction **existe déjà dans la base `b2808898`** ; elle n'est pas introduite par le réglage du niveau. La trace d'invocation/Ready CPU et l'erreur observée sont cohérentes avec ce blocage de contrat. La variante précise de la trame rejetée n'étant pas journalisée intégralement, cette limite d'observation est conservée ; elle n'annule pas la contradiction démontrée des sources. La réserve empêche de qualifier Parakeet CPU pour V7.

**Correction proposée, non appliquée** : exiger `NativeCallStarted` pour le backend **GPU attendu/validé seulement**. Accepter un résultat CPU corrélé sans ce marqueur, tout en conservant les contrôles actuels d'identité, session, modèle, fenêtre, ordre, durée et résultat ; préserver intégralement l'attestation GPU. Périmètre minimal proposé : `parakeet_live.rs` et ses tests privés. Preuves à prévoir : résultat CPU sans marqueur accepté ; résultat GPU sans marqueur refusé ; marqueur GPU valide puis résultat corrélé accepté ; contrôles de corrélation inchangés. Rejouer ensuite les **16 seuls cas Parakeet CPU**, sans toucher aux résultats Turbo.

L'accord courant exclut explicitement la logique produit : **aucune correction R8 ni reprise native n'a été effectuée**. Un accord précis de cycle produit ciblé R8 et de budget de campagne est nécessaire. Verdict courant : **R1 à R7 fermés, R8 ouvert ; D2 Parakeet BLOCKED**, recette Windows et comparaison GPU toujours réservées à l'utilisateur. Aucun CLEAN global ni acceptation produit finale.

### Remise et preuves courantes

- Échec de compilation conservé : `PersonalRelease-20261025T205047315-26524ab6`, exit 101 sur les six appels de trace ; corrigé dans le seul périmètre test autorisé.
- Build gardé courant : `PersonalRelease-20261025T205201068-5a65bc0e`, **exit 0 en 27 s**, SelfTest préalable réussi. Test synthétique de harnais : `D2HarnessSelfTest-20261025T205237890-0bd25891`, **1 réussi, 181 filtrés**. Formatage ciblé et `git diff --check` réussis. Le défaut de formatage global préexistant hors périmètre reste inchangé.
- Dossier `target/parakeet-tests/micro-level/recette-personnelle/`, exactement trois EXE correspondant au build réussi : desktop **14 330 880 octets**, SHA-256 `5823BADE8462FA03980A5EE6283C3FA13D1490E539FCB6A69045596F4F32633C` ; CPU **20 809 728 octets**, SHA `3613ADC5884C5D28BCB85992647233E6F72653EC93FC205E1ECCC9284C393E7A` ; GPU **20 882 432 octets**, SHA `3537A0E5E1B1E1A20316651AD2156E32213C67A23B78DA0DFD0E152CE2D24234`. Workers Parakeet, cinq/neuf DLL CPU/GPU et modèles installés revérifiés en lecture seule. Aucune installation ni fenêtre lancée.
- Nouvelle archive `target/parakeet-tests/micro-level/archives/d2-parakeet-diagnostic-20261025-2053/` : **24 fichiers, 2 267 458 octets**, arborescence conservée et SHA-256 de chaque copie vérifié ; aucun WAV ni cache. Archives et sorties précédentes intactes, aucune suppression ni manifeste supplémentaire.
- Aucun processus de campagne actif, surveillance terminée avant cette rédaction. Espace libre final : **130 509 869 056 octets** (environ 121,5 Gio). Code non commité, aucune publication.
