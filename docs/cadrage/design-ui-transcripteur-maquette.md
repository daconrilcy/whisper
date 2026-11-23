Avancée : le brief UI est transformé en exigences, parcours, états et architecture vérifiables.
Blocage : deux contrats de persistance doivent être confirmés au préflight du plan avant de coder.
Décision : DESIGN proposé pour revue ; aucun code produit ni comportement n'est autorisé par ce document.

# Design — refonte de l’interface du transcripteur

## 1. Autorité et sources

- Auteur : session Codex principale. Version 1, 23 novembre 2026.
- Source fonctionnelle : [brief-ui-transcripteur-maquette.md](brief-ui-transcripteur-maquette.md), notamment DU-01–07, UI-01–09, IMP/STA/PAR/SPE/CFG/LNK/VIS/ACC et décisions de fluidité §12.
- Base technique lue : dépôt `C:\dev\whisper`, commit `540aebf77b1c0c0f423c08285c09816dea7176a2`, branche `codex/ui-transcriber-mockup`. `AGENTS.md` racine et desktop ; vues `view.rs`, `view/queue.rs`, contrats `whisper-core/application/contracts.rs` et `domain.rs`.
- Les changements préexistants à `AGENTS.md` sont hors de ce livrable et doivent être préservés.
- Aucun test, build, commande Cargo ou observation de la fenêtre n’a été exécuté. La capture de référence citée dans le brief est sous Temp : sa disponibilité au futur lot n’est pas garantie.

## 2. Objectif produit et périmètre

Réorganiser la façade Windows autour des entrées fichier et micro et d’une liste de traitements, avec une configuration avancée, tout en conservant les capacités produit existantes : WAV/MP3, large-v3-turbo, Auto/CPU/GPU, microphone, texte confirmé, résultats TXT/SRT/WAV, choix de microphone, récupération existante et messages d’état.

Le périmètre comprend la composition visuelle, les parcours d’import immédiat ou différé, la file FIFO, annulation, reprise confirmée des travaux interrompus, historique/lien vers fichiers et dossiers, configuration avancée, présentation Parakeet finale, spectre à bandes, messages et adaptation DPI/fenêtre.

Exclus/différés : support nouveau de sélection multiple ou glisser-déposer, reprise générique d’un enregistrement micro, changement de modèle, nouveau moteur, changement de politiques CPU/GPU/Auto, supervision/tray/reprise après panne, MP3 en sortie et ajout d’un système de stockage ou d’une dépendance non prévu. Le chrome Windows standard est conservé.

## 3. Capacités et exigences

| ID | Exigence vérifiable | Source |
|---|---|---|
| REQ-01 | L’accueil présente en-tête/état effectif, message, cartes import et micro, puis traitements ; la préférence de calcul reste distincte du moteur effectif. | UI-01/02, CFG-01 |
| REQ-02 | Le bouton principal admet puis traite le fichier sans second clic ; « Importer seulement » admet sans démarrer. | DU-01/02, IMP-01/02 |
| REQ-03 | L’admission confirmée fournit l’identité utilisée pour traiter. Un doublon suspend seulement sa propre décision et conserve l’intention choisie ; annuler le dialogue ne remplace ni ne traite. | UI-04, IMP-03/04 |
| REQ-04 | Les intentions immédiates et clics « Traiter » sont séquencés FIFO à l’admission/confirmation ; au plus un calcul d’import est actif. Un micro ou calcul déjà actif n’est pas préempté. L’attente micro est indiquée sans estimation. | IMP-05, §12 |
| REQ-05 | Une fermeture avant démarrage laisse l’intention interrompue ; aucune intention ne démarre au lancement sans confirmation utilisateur. | IMP-06, §12 |
| REQ-06 | Les états visibles distinguent Importé, En attente, Traitement, Annulation, Traité, Annulé, Interrompu et Échec. Une ligne annulée reste dans l’historique ; l’annulation ne supprime ni source ni résultat publié. Un résultat publié avant acquittement de l’arrêt reste Traité. | DU-03, STA-01/02 |
| REQ-07 | Les liens ouvrent le répertoire parent de la source et, lorsqu’il existe, le répertoire du TXT publié. Échec de chemin visible, sans création ni substitution silencieuse. | DU-07, LNK-01–03 |
| REQ-08 | Les réglages avancés réutilisent les préférences et contrôleurs existants, exposent moteur/calcul et dossiers par défaut acceptés. Une nouvelle destination n’altère pas les entrées admises. Le raccourci ouvre la destination effective. | DU-06, CFG-01/02 |
| REQ-09 | Le spectre présente 64 bandes linéaires stables, chacune agrégée par moyenne pondérée de puissance ; façade après gain/limiteur, détails avant/après. Silence, mesure absente/périmée et saturation restent compréhensibles et distincts. | DU-05, SPE-01/02, §12 |
| REQ-10 | Les blocs micro confirmés restent distingués du provisoire et du résultat final. Pour Parakeet, succès définitif seulement après calcul final requis et publication réussie ; aucune validation humaine ou présentation brouillon. TXT/SRT/WAV de même nom sont annoncés disponibles ensemble après finalisation. | DU-04, PAR-01–04 |
| REQ-11 | La récupération reste spécifique à l’opération ; aucune reprise micro générique ou suppression implicite n’est ajoutée. Erreurs de moteur, source, traitement, publication et chemins sont visibles. | UI-08/09 |
| REQ-12 | La hiérarchie reste lisible à 1536×864 et 1024×720 unités logiques, cartes empilées sous 1200, minimum 960×640 ; vérification aux échelles 100/150/200 %. États et contrôles ne reposent pas uniquement sur la couleur. | VIS-01, ACC-01, §12 |

### Parcours et états visibles

| Parcours | États et transitions attendus |
|---|---|
| Import-transcrire | sélection → admission/doublon → attente FIFO ou traitement → publication → traité ; erreur/fermeture avant départ → échec/interrompu, jamais relancé sans confirmation |
| Import seul | sélection → admission/doublon → importé ; clic Traiter → attente FIFO ou traitement ; annuler retire l’intention mais conserve la ligne |
| Annulation active | traitement → annulation demandée → annulé après arrêt coopératif ; si publication déjà réussie, traité |
| Micro | choix/source et mesure → capture, blocs confirmés → arrêt/drain/finalisation/publication → définitif ; échec publication distinct, sans succès annoncé |
| Récupération | incident identifié → choix explicite conforme au contrat existant → reprise ou abandon sans effacement implicite |
| Lien de fichier | clic source/TXT → ouverture du dossier parent correspondant ou message de chemin indisponible |

### Machine de file proposée

`Importé` (pas d’intention) ; `En attente` (intention admise) ; `Traitement` ; `Annulation` ; `Traité` ; `Annulé` ; `Interrompu` ; `Échec` ; `Doublon à résoudre` (état de dialogue, ne réserve pas le slot FIFO). Les transitions et identités doivent rester conformes au cœur ; l’UI ne crée pas sa propre politique de concurrence. Une nouvelle tentative est une nouvelle identité. Le stockage durable de certaines transitions est une question ouverte Q-01.

## 4. Critères d’acceptation

Les critères ci-dessous reformulent les critères du brief en observations reproductibles. Ils restent NOT RUN.

| AC | Scénario et résultat observable |
|---|---|
| AC-01 | Avec entrée valide, action principale entraîne une admission identifiée puis un calcul, sans clic supplémentaire ; mode Importer seulement ne lance rien avant Traiter. |
| AC-02 | Résoudre et annuler un doublon dans chacun des deux modes conserve le mode initial ; annuler le dialogue n’entraîne ni remplacement ni calcul. |
| AC-03 | Pendant A, admettre B immédiat puis C seul ; B s’exécute après A, C seulement après clic et dans l’ordre des demandes, sans calculs simultanés. |
| AC-04 | Fermer avec B en attente puis relancer : B est interrompu et ne repart qu’après confirmation. |
| AC-05 | Annuler une attente ou un calcul conserve ligne/source et tout résultat publié ; échec/interruption ne sont pas affichés comme annulés ou traités. |
| AC-06 | Cliquer source et résultat ouvre le bon dossier même si leurs parents diffèrent ; chemin absent produit un message sans substitution. |
| AC-07 | Réglages avancés exposent les choix existants et chemins convenus ; la préférence et le moteur effectif sont identifiables, et les tâches admises gardent leur destination. |
| AC-08 | Entrées synthétiques de spectre localisées modifient seulement les bandes correspondantes ; moyenne puissance, silence, absence et péremption respectent la définition du brief. |
| AC-09 | Capture micro expose les blocs confirmés ; fin réussie publie TXT/SRT/WAV et montre un résultat définitif sans action de vérification. Échec de publication ne montre pas de succès. |
| AC-10 | Rendu contrôlé aux tailles et DPI de REQ-12 : cartes, historique, arrêt et focus restent utilisables ; aucun état ne dépend de la couleur seule. |
| AC-11 | Les scénarios existants d’import/micro/récupération, CPU/GPU/Auto, chemins, fermeture et préférences restent conformes ; comparer aux tests et contrats actuels avant/après. |

## 5. Domaine et responsabilités

- Le **cœur** porte admissions, transitions, identité, FIFO, exclusion mutuelle, annulation coopérative, reprise explicite et vérité de publication.
- Les **adaptateurs** exécutent fichiers, ouverture de dossier, persistance/scan d’archive et flux audio selon les ports existants.
- Le **desktop/root** compose et injecte services. L’UI transforme les gestes en commandes et projette les états ; elle ne séquence pas les calculs par ses propres flags.
- La **vue** présente le spectre à partir des données du cœur, sans changer la mesure. Toute agrégation visuelle reste une projection pure, testable.

État métier, opération durable et présentation ne sont pas synonymes : « traité » dépend de la publication ; « annulé » dépend d’une demande utilisateur confirmée ; une erreur ou fermeture donne interruption/échec ; le résultat confirmé de capture n’est pas le final publié.

## 6. Architecture et contrats à conserver

| Zone | Propriétaire prévu | Contrat / preuve à vérifier |
|---|---|---|
| Composition et responsive | `crates/whisper-desktop/src/ui/model/view.rs` et vues | conserver MVC, contrôleurs et styles egui existants ; rendu mesuré aux viewports AC-10 |
| Réglages | `ui/model/view/settings.rs`, `ui/model/controller/actions.rs`, modèles/préférences | réutiliser `PathSettingsApp`, états de sélection et commandes ; pas de chemins techniques exposés |
| Import/file/historique | `view/actions.rs`, `view/queue.rs`, `view/history.rs`, `whisper-core/application/contracts.rs`, ports archive | commande issue d’une identité d’admission ; FIFO et annulation sous autorité cœur ; archive et reprise inspectées avant choix du stockage |
| Micro/spectre/résultat | `view/microphone.rs`, `capture.rs`, core `microphone_level` et contrats | conserver RMS/pic, fraîcheur >500 ms, calculs finaux existants et exigences TXT/SRT/WAV |
| Dialogues et liens | `view/dialogs.rs`, contrôleurs et service de chemin/adaptateur existants | annulation sans effet ; ouverture de parent de fichier via service existant, erreur visible |
| Tests | `ui/tests.rs`, `ui/tests/`, tests core/adapters existants | tests UI dans desktop, invariants de file/publication dans leur crate propriétaire ; aucun champ de production ajouté seulement pour tester |

Pas de dépendance, crate, port système, migration ou technologie nouvelle autorisés par ce design. Si le besoin de conserver les annulations/intents implique une nouvelle persistance, une dépendance, une modification de schéma ou un chemin hors périmètre, arrêter et demander une décision de portée avant cette modification (Q-01).

## 7. Questions structurantes et risques

| ID | Question / risque | Classe / propriétaire | Résolution attendue et impact |
|---|---|---|---|
| Q-01 | L’archive actuelle peut-elle conserver l’identité, l’annulation et les intentions en attente après fermeture sans nouveau format de persistance ? | Structurante, implémenteur au préflight puis auteur du design si GAP | Lire contrats et implémentation archive. Si schéma/stockage nouveau requis : STOP, arbitrage utilisateur ; impact lots file/historique/reprise. |
| Q-02 | Les fichiers TXT/SRT/WAV sont-ils publiés comme une unité atomique observable, ou l’application peut-elle en exposer un sous-ensemble ? | Structurante, implémenteur au préflight | Vérifier publication et récupération actuelles. AC-09 ne passe que lorsque l’état final suit le contrat réel ; tout renforcement de persistance hors code UI retourne en cadrage. |
| Q-03 | La capture image temporaire sera-t-elle disponible pour comparaison future ? | Détail réversible, propriétaire : utilisateur | Si absente, reconstruire l’évaluation à partir de la description du brief ; la fidélité pixel n’est pas requise. Pas de blocage code. |
| Q-04 | Date, durée, extrait et nom convivial de la maquette sont-ils requis dans l’historique ? | Hors périmètre actuel, décision utilisateur si réintroduit | Ils ne sont pas promis ; exigeraient source de données et critères nouveaux. |

Risques principaux : l’intention de file exécutée depuis l’UI pourrait perdre l’identité confirmée ; les lignes annulées pourraient ne pas être représentables par l’archive ; la publication multi-fichier pourrait ne pas fournir la garantie implicite du brief ; le spectre pourrait être agrégé incorrectement en dB au lieu de puissance ; les tailles et DPI peuvent couper les commandes ; un raccourci de dossier pourrait contourner l’adaptateur d’effets.

## 8. Traçabilité et readiness

Couverture : UI-01/02 → REQ-01/08, AC-07/10 ; UI-03 → REQ-08, AC-07 ; UI-04/09 → REQ-02/03/04/07/11, AC-01/02/03/06 ; UI-05 → REQ-09, AC-08/10 ; UI-06 → REQ-10, AC-09/11 ; UI-07 → REQ-04/06/07, AC-03/04/05/06 ; UI-08 → REQ-05/11, AC-04/11. Les critères du brief VIS-01, ACC-01, IMP-01–06, STA-01/02, PAR-01–04, SPE-01/02, CFG-01/02 et LNK-01–03 sont couverts par AC-01–11.

| Point de readiness | Statut |
|---|---|
| Besoin et portée | PASS — brief source et exclusions explicites |
| Exigences et acceptation | PASS — REQ-01–12, AC-01–11 |
| Domaine et états | PASS avec Q-01 — transitions UI définies, preuve de durabilité à confirmer |
| Architecture | PASS avec Q-01/Q-02 — responsabilités respectées, aucun nouveau port/dépendance présumé |
| Faisabilité | GAP avant code sur stockage des annulations/intents et atomicité publication |
| Autorisation | Cadrage et plan seulement ; implémentation non autorisée |
| Fenêtre Windows | NOT RUN, observation utilisateur au lot de livraison |

**Readiness DESIGN : PROPOSÉ, conditionné par résolution de Q-01/Q-02 au préflight.** Aucun test produit n’a été exécuté.
