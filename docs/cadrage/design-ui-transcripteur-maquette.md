Avancée : les écarts relevés en revue sont corrigés ; les parcours actifs et les contrats durables sont décrits.
Blocage : l’effet Windows d’ouverture de dossier proposé requiert un accord précis avant implémentation.
Décision : DESIGN livré avec réserves après deux cycles de revue/correction ; aucun code produit n'est autorisé par ce document.

# Design — refonte de l’interface du transcripteur

## 1. Autorité et sources

- Auteur : session Codex principale. Version 3, 23 novembre 2026 (corrections des cycles 1 et 2 ; réserves de la seconde revue conservées).
- Source fonctionnelle : [brief-ui-transcripteur-maquette.md](brief-ui-transcripteur-maquette.md), notamment DU-01–07, UI-01–09, IMP/STA/PAR/SPE/CFG/LNK/VIS/ACC et décisions de fluidité §12.
- Base technique lue : dépôt `C:\dev\whisper`, commit `540aebf77b1c0c0f423c08285c09816dea7176a2`, branche `codex/ui-transcriber-mockup`. `AGENTS.md` racine et desktop ; vues `view.rs`, `view/queue.rs`, contrats `whisper-core/application/contracts.rs` et `domain.rs`.
- Les changements préexistants à `AGENTS.md` sont hors de ce livrable et doivent être préservés.
- Aucun test, build, commande Cargo ou observation de la fenêtre n’a été exécuté. La capture de référence citée dans le brief est sous Temp : sa disponibilité au futur lot n’est pas garantie.

## 2. Objectif produit et périmètre

Réorganiser la façade Windows autour des entrées fichier et micro et d’une liste de traitements, avec une configuration avancée, tout en conservant les capacités produit existantes : WAV/MP3, large-v3-turbo, Auto/CPU/GPU, microphone, texte confirmé, résultats TXT/SRT/WAV, choix de microphone, récupération existante et messages d’état. Les changements requis de file, stockage durable, préférences et intégration Windows sont inclus dans le design, car la vue seule ne peut pas fournir les comportements acceptés.

Le périmètre comprend la composition visuelle, les parcours d’import immédiat ou différé, la file FIFO, annulation, reprise confirmée des travaux interrompus, historique/lien vers fichiers et dossiers, configuration avancée, présentation Parakeet finale, spectre à bandes, messages et adaptation DPI/fenêtre.

Exclus/différés : support nouveau de sélection multiple ou glisser-déposer, reprise générique d’un enregistrement micro, changement de modèle, nouveau moteur, changement des politiques de calcul CPU/GPU/Auto, supervision/tray/reprise après panne et MP3 en sortie. Le chrome Windows standard est conservé. Une extension additive du journal de file et des réglages existants est nécessaire ; elle ne remplace pas le stockage par un système différent. L’ouverture de dossier nécessite un effet Windows porté par un adaptateur, dont la solution doit être validée avant le code.

## 3. Capacités et exigences

| ID | Exigence vérifiable | Source |
|---|---|---|
| REQ-01 | L’accueil présente en-tête/état effectif, message, cartes import et micro, puis traitements ; la préférence de calcul reste distincte du moteur effectif. | UI-01/02, CFG-01 |
| REQ-02 | Le bouton principal admet puis traite le fichier sans second clic ; « Importer seulement » admet sans démarrer. | DU-01/02, IMP-01/02 |
| REQ-03 | L’admission confirmée fournit l’identité utilisée pour traiter. Un doublon suspend seulement sa propre décision et conserve l’intention choisie ; annuler le dialogue ne remplace ni ne traite. | UI-04, IMP-03/04 |
| REQ-04 | Les intentions immédiates et clics « Traiter » sont séquencés FIFO à l’admission/confirmation ; au plus un calcul d’import est actif. Un micro ou calcul déjà actif n’est pas préempté. L’attente micro est indiquée sans estimation. | IMP-05, §12 |
| REQ-05 | La fermeture conserve les intentions de calcul non démarrées comme interrompues ; au lancement elles demandent confirmation, et aucun calcul ne redémarre sans celle-ci. Un simple import sans intention de calcul reste importé. | IMP-06, §12 |
| REQ-06 | Les états visibles distinguent Importé, En attente, Traitement, Annulation, Traité, Annulé, Interrompu et Échec. Une ligne annulée reste durablement consultable dans l’historique sans expiration propre ; l’annulation ne supprime ni source ni résultat publié. Un résultat publié avant acquittement de l’arrêt reste Traité. | DU-03, STA-01/02 |
| REQ-07 | Les liens ouvrent le répertoire parent de la source et, lorsqu’il existe, le répertoire du TXT publié. Échec de chemin visible, sans création ni substitution silencieuse. | DU-07, LNK-01–03 |
| REQ-08 | Les réglages avancés réutilisent les préférences et contrôleurs existants, exposent moteur/calcul, dossier initial audio et dossier de sortie. Le dernier dossier audio valide reste distinct du dossier initial ; au parcours audio, priorité au dernier dossier valide, puis au dossier initial configurable, puis au secours existant. Une nouvelle destination n’altère pas les entrées admises. Le raccourci ouvre la destination effective. | UI-03/§5, DU-06, CFG-01/02 |
| REQ-09 | Le spectre présente 64 bandes linéaires stables, chacune agrégée par moyenne pondérée de puissance ; façade après gain/limiteur, détails avant/après. Silence, mesure absente/périmée et saturation restent compréhensibles et distincts. | DU-05, SPE-01/02, §12 |
| REQ-10 | Les blocs micro confirmés restent distingués du provisoire et du résultat final. Pour Parakeet, succès définitif seulement quand chacun des fichiers attendus TXT/SRT/WAV existe et est accessible ; une publication partielle n’est pas un succès final et doit être signalée selon les possibilités de récupération existantes. Aucune validation humaine ou présentation brouillon. L’atomicité externe de la publication n’est pas exigée. | DU-04, PAR-01–04 |
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

Le journal d’import existant conserve l’admission et les métadonnées de la source. Une intention de calcul (immédiate après admission/résolution du doublon, ou explicite après clic Traiter) est enregistrée indépendamment de l’état d’import. Le cœur sélectionne le plus ancien calcul demandé ; les lignes importées sans intention n’entrent pas dans cette FIFO. Un job actif direct/micro occupe le slot jusqu’à sa fin ou son arrêt. Une résolution de doublon reste hors FIFO jusqu’à confirmation, puis prend sa place dans l’ordre à cet instant.

Contrat de transition durable proposé : admission sans calcul → `Importé`; admission avec calcul → `En attente` et intention associée; sélection du slot → `Traitement`; fermeture avant départ ou arrêt non confirmé → `Interrompu` avec intention conservée mais suspendue; confirmation utilisateur → remise en file; annulation demandée → `Annulation`, puis entrée terminale `Annulé` après confirmation d’arrêt; publication réussie avant acquittement d’arrêt → `Traité`; erreur → `Échec`. Annulé et traité ne sont pas supprimés du journal. Une nouvelle tentative reçoit une nouvelle identité. Au chargement, les anciens enregistrements sans champ d’intention sont interprétés comme import seul afin de ne jamais démarrer un ancien import de façon inattendue. Les enregistrements `Running` historiques sont normalisés en `Interrompu` selon le mécanisme de récupération existant. L’implémentation conserve le journal append-only `queue.jsonl` et ajoute des enregistrements upsert/versionnés compatibles avec le lecteur des anciennes lignes ; toute migration destructive est exclue.

### Composition des états de l’écran

- **Repos :** en-tête fixe ; bandeau/message ; cartes Import et Micro côte à côte ; liste des traitements ensuite. Sous 1200 unités, Import puis Micro sont empilés et la liste devient compacte avec défilement vertical.
- **Capture micro :** l’en-tête fixe conserve l’état effectif et Arrêter hors de la zone défilante. La carte Micro montre source, mesure et spectre. Le panneau d’activité met le texte confirmé au premier plan ; le provisoire est secondaire, marqué « hypothèse remplaçable ». Progression, capture et durabilité restent visibles sans masquer la file.
- **Traitement fichier :** panneau d’activité avec phase, progression, moteur effectif, source et Annuler ; la file reste dessous et les imports seuls gardent Traiter/Annuler.
- **Finalisation/publication :** l’action d’arrêt reste visible jusqu’à acquittement ; calcul final, publication et échec sont distincts. Le résultat et ses liens ne deviennent disponibles qu’après contrôle de chaque sortie. En mode compact le panneau actif précède la liste et l’action reste hors défilement.
- **Session unifiée Parakeet :** choix de session et document/audio paginés actuels sont recomposés dans la zone d’activité ; aucun second calcul de finalisation n’est ajouté.

## 4. Critères d’acceptation

Les critères ci-dessous reformulent les critères du brief en observations reproductibles. Ils restent NOT RUN.

| AC | Scénario et résultat observable |
|---|---|
| AC-01 | Avec entrée valide, action principale entraîne une admission identifiée puis un calcul, sans clic supplémentaire ; mode Importer seulement ne lance rien avant Traiter. |
| AC-02 | Résoudre et annuler un doublon dans chacun des deux modes conserve le mode initial ; annuler le dialogue n’entraîne ni remplacement ni calcul. |
| AC-03 | Pendant A, admettre B immédiat puis C seul ; B s’exécute après A, C seulement après clic et dans l’ordre des demandes, sans calculs simultanés. |
| AC-04 | Fermer avec B en attente puis relancer : B est durablement présent en Interrompu et ne repart qu’après confirmation ; un import seul reste Importé. Les anciennes entrées ne démarrent jamais implicitement après migration. |
| AC-05 | Annuler une attente ou un calcul conserve durablement la ligne/source et tout résultat publié ; annulation active reste en Annulation jusqu’à acquittement ; échec/interruption ne sont pas affichés comme annulés ou traités. |
| AC-06 | Cliquer source et résultat ouvre le bon dossier même si leurs parents diffèrent ; chemin absent produit un message sans substitution. L’action passe par le service d’effet Windows défini au design, pas par une commande construite dans la vue. |
| AC-07 | Réglages avancés exposent les choix existants, le dossier initial audio et le dossier de sortie ; dernier audio et dossier initial restent distincts et la priorité dernier valide → initial valide → secours est observée après redémarrage. La préférence et le moteur effectif sont identifiables, et les tâches admises gardent leur destination. |
| AC-08 | Entrées synthétiques de spectre localisées modifient seulement les bandes correspondantes ; moyenne puissance, silence, absence et péremption respectent la définition du brief. |
| AC-09 | Capture micro expose les blocs confirmés ; fin réussie ne montre le résultat définitif qu’après vérification de l’existence/accessibilité de TXT/SRT/WAV. Échec ou publication partielle ne montre pas de succès et reste signalé ; aucune action de vérification humaine. |
| AC-10 | Rendu contrôlé aux tailles et DPI de REQ-12 dans les états repos, import actif, capture, finalisation/publication et résultat ; cartes, historique, texte confirmé, provisoire, progression, arrêt et focus restent utilisables. Aucun état ne dépend de la couleur seule. |
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
| Réglages | `whisper-core/src/path_settings.rs`, `whisper-adapters/src/path_settings.rs`, `whisper-desktop/src/ui/model/view/settings.rs`, modèles/préférences | ajouter séparément le dossier audio initial dans le snapshot réglages, compatible avec les enregistrements existants ; garder `audio_directory` comme dernier dossier utilisé |
| Import/file/historique | `whisper-core/src/application/contracts.rs`, `domain.rs`, `ports.rs`, `application/runtime/{queue,import,events,poll,closing}.rs`; `whisper-adapters/src/queue_store.rs`, `recovery.rs`, `worker_ipc/runtime/{queue,import}.rs`; vues `queue.rs`, `history.rs` | intention persistée distincte du fait d’être importé ; séquence des demandes de calcul ; FIFO cœur ; événements terminaux `Published`/`Stopped`/`Failed` projetés puis persistés ; journal append-only avec états terminaux conservés ; ancienne ligne sans intention = import seul |
| Micro/spectre/résultat | `view/microphone.rs`, `capture.rs`, `unified.rs`, core `microphone_level` et contrats, adaptateur publication existant | conserver RMS/pic, fraîcheur >500 ms et calculs finaux ; l’événement de succès n’est émis qu’après contrôle de disponibilité des sorties requises ; publication partielle = erreur récupérable, pas final réussi |
| Dialogues et liens | `view/dialogs.rs`, contrôleurs, nouveau port d’ouverture de dossier dans core et adaptateur Windows à identifier sous `whisper-adapters` | aucune capacité de lancement de dossier n’est présente dans les sources inspectées ; proposer un effet borné, injectable/testable et messages d’erreur ; le choix d’intégration Windows reste soumis à accord avant code |
| Tests | `ui/tests.rs`, `ui/tests/`, tests core/adapters existants | tests UI dans desktop, invariants de file/publication dans leur crate propriétaire ; aucun champ de production ajouté seulement pour tester |

Pas de nouvelle dépendance, crate ou format durable alternatif proposés. Les changements du journal existant, du contrat core, des réglages sérialisés et l’ajout d’un port/adaptateur Windows font partie du périmètre proposé, avec rétrocompatibilité requise. L’ouverture d’Explorer est un effet système ; aucun code ne sera fait sans décision technique précise sur le port et son adaptateur Windows.

## 7. Questions structurantes et risques

| ID | Question / risque | Classe / propriétaire | Résolution attendue et impact |
|---|---|---|---|
| Q-01 | Autoriser la méthode proposée pour ouvrir le parent sous Windows : `FolderOpenerPort` dans `whisper-core/src/ports.rs`, adaptateur `whisper-adapters/src/open_folder.rs`, injection `whisper-desktop/src/root.rs`; vérifier que le parent existe, puis appeler `explorer.exe` avec `std::process::Command::new(...).arg(parent).spawn()` sans shell. | Accord écrit précis de l’utilisateur avant T5/code système | Le besoin DU-07/LNK est confirmé. La méthode n’ajoute aucune dépendance ; elle utilise un effet système Windows. Elle est proposée mais non autorisée. Q-01 bloque la mise en œuvre des liens, qui restent dans le périmètre. |
| Q-02 | La capture image temporaire sera-t-elle disponible pour comparaison future ? | Détail réversible, propriétaire : utilisateur | Si absente, reconstruire l’évaluation à partir de la description du brief ; la fidélité pixel n’est pas requise. Pas de blocage code. |
| Q-03 | Date, durée, extrait et nom convivial de la maquette sont-ils requis dans l’historique ? | Hors périmètre actuel, décision utilisateur si réintroduit | Ils ne sont pas promis ; exigeraient source de données et critères nouveaux. |

Risques principaux : les migrations du journal pourraient promouvoir par erreur des imports anciens vers un calcul ; la clôture pourrait effacer une intention ; deux files distinctes pourraient violer l’ordre global ; le statut annulé pourrait se confondre avec une suppression ; une publication partielle pourrait être annoncée comme finale ; le spectre pourrait être agrégé en dB au lieu de puissance ; les tailles/DPI peuvent couper l’arrêt ; le lancement Windows de dossier doit rester via l’adaptateur.

## 8. Traçabilité et readiness

Couverture : UI-01/02 → REQ-01/08, AC-07/10 ; UI-03 → REQ-08, AC-07 ; UI-04/09 → REQ-02/03/04/07/11, AC-01/02/03/06 ; UI-05 → REQ-09, AC-08/10 ; UI-06 → REQ-10, AC-09/11 ; UI-07 → REQ-04/06/07, AC-03/04/05/06 ; UI-08 → REQ-05/11, AC-04/11. Les critères du brief VIS-01, ACC-01, IMP-01–06, STA-01/02, PAR-01–04, SPE-01/02, CFG-01/02 et LNK-01–03 sont couverts par AC-01–11.

| Point de readiness | Statut |
|---|---|
| Besoin et portée | PASS — brief source et exclusions explicites |
| Exigences et acceptation | PASS — REQ-01–12, AC-01–11 |
| Domaine et états | PASS — intention et états durables décrits, migration sans auto-démarrage précisée |
| Architecture | PASS avec Q-01 — propriétaires core/adapters/desktop identifiés ; effet Windows proposé, non autorisé |
| Faisabilité | GAP avant implémentation tant que la méthode Windows proposée n’est pas approuvée |
| Autorisation | Cadrage et plan seulement ; implémentation non autorisée |
| Fenêtre Windows | NOT RUN, observation utilisateur au lot de livraison |

**Readiness DESIGN : LIVRÉ AVEC RÉSERVES après deux cycles de revue/correction ; Q-01 bloque la mise en œuvre de DU-07/LNK.** Aucun test produit n’a été exécuté.
