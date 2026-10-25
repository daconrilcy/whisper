Avancée : plan documentaire du niveau microphone rédigé à partir des décisions Q1 à Q5 acceptées.
Blocage : aucune implémentation ni campagne autorisée ; corpus de qualité et préflight d'exécution à fermer à leurs échéances.
Décision : soumettre les neuf tâches à validation après revue indépendante, sans les exécuter.

# Whisper — Plan du réglage du niveau microphone

## Statut et sources

- Date : 25 octobre 2026. Auteur des fichiers : session Codex principale ; proposition de tâches par `rust_plan_writer`, contrôle indépendant par `rust_design_reviewer`.
- Référence produit : `878cd1e9a3a37bd062af9802bec68f2eb45f5337`. Conception : [evo-micro-level.md](evo-micro-level.md), revue **DESIGN CLEAN** sur `d10516808804f461ca1597b78d0d176f3f3baa27` ; R-MIC-D01 fermé. Précisions ultérieures limitées au statut et à la fixture spectrale bin 64, sans changement de Q1–Q5.
- Accord produit exact dans cette session : « ok sur les recommandation Q1 à Q5 ». Accord de rédaction : « tu peux passer à la rédaction du plan ».
- **Plan proposé, non encore accepté par l'utilisateur. Aucune autorisation d'implémenter.** La revue du plan ne vaut pas acceptation produit, autorisation de campagne, qualification Windows ou publication.
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
| Documentation | `docs/cadrage/evo-micro-level.md`, `docs/cadrage/plan-evo-micro-level.md` | Référence/plan et résultats utiles, deux fichiers maximum prévus pour le rapport de ce lot. |

Sources de régression en lecture/exécution future : `crates/whisper-core/tests/{compute_policy.rs,import_contract.rs}`, `crates/whisper-adapters/tests/{import_cpu.rs,import_mp3.rs,parakeet_import.rs,live_archive.rs}`. Sources indirectes en lecture : `crates/whisper-adapters/src/{supervisor.rs,decoder.rs,archive.rs}`, workers et contrats moteur existants, manifests racine/core/desktop. Aucun changement de worker, d'API moteur ou de façade native prévu. Une modification devenue nécessaire hors liste revient au périmètre avant édition.

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
- PLANS : à relire sur la version Git contenant ce plan et les précisions documentaires de la conception.
- Acceptation du présent plan par l'utilisateur : **non reçue**. Autorisation d'implémenter ou d'exécuter une campagne : **non reçue**.
- Vérifications réellement exécutées pour cette mission : lectures ciblées et contrôles Git/documentaires uniquement. Aucune compilation, capture, installation, fenêtre pilotée ou exécution de test produit.
