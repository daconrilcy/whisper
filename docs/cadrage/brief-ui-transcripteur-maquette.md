Avancée : les décisions de fluidité demandées ont été arrêtées et intégrées à la décomposition.
Blocage : aucun pour le cadrage ; un plan reste nécessaire avant toute implémentation.
Décision : brief de conception uniquement ; aucune refonte ni implémentation n'est autorisée par ce document.

# Brief UI — maquette du transcripteur Whisper

## 1. Objet et autorité

Ce brief décompose l'image jointe le 23 novembre 2026 et intègre les décisions complémentaires données par l'utilisateur dans cette session. Il sert au cadrage de l'interface Windows Whisper existante. Il ne donne pas d'autorisation d'implémentation.

L'image est une référence de composition et d'apparence. Ses noms de fichiers, dates, durées, extraits, modèle/GPU affichés et message de reprise sont des données ou états illustratifs, sauf décision explicite rapportée dans ce brief.

Les décisions marquées **confirmées** viennent de l'objectif produit ou des précisions utilisateur. La section 12 consigne les choix de cadrage arrêtés par la session principale à la demande de l'utilisateur, pour favoriser une utilisation fluide. Ils précisent le comportement attendu sans autoriser l'implémentation.

## 2. Référence visuelle et sources

### Référence

- Image jointe : `C:\Users\cyril\AppData\Local\Temp\codex-clipboard-3e321154-644f-4153-9eaa-3b196d6d1c3a.png`, 1536 × 864 pixels.
- Elle montre une barre supérieure, un bandeau d'avertissement, deux cartes côte à côte (import et microphone), puis une liste d'historique.
- La rangée de capsules sous le choix du microphone est une représentation visuelle de référence. Les cibles nommées et valeurs affichées dans cette image ne sont pas des données produit.
- Le fichier se trouve dans un répertoire temporaire ; une comparaison visuelle durable dépend de sa conservation dans un emplacement accessible au lot futur.

### Dépôt inspecté

Racine `C:\dev\whisper`, branche observée `codex/remove-failing-github-ci`. Un changement préexistant de `AGENTS.md` était présent et a été préservé.

Sources principales consultées :

- `AGENTS.md` et `crates/whisper-desktop/AGENTS.md`.
- `crates/whisper-desktop/src/ui/model/view.rs` et les vues `settings.rs`, `microphone.rs`, `queue.rs`, `history.rs`, `capture.rs`, `unified.rs`, `dialogs.rs` et `notices.rs`.
- `crates/whisper-desktop/src/ui/model/controller/actions.rs`.
- `crates/whisper-core/src/application/contracts.rs`, `crates/whisper-core/src/ports.rs`, `crates/whisper-core/src/domain.rs` et `crates/whisper-core/src/microphone_level.rs`.
- `docs/cadrage/refacto-ui.md`, `docs/cadrage/plan-refacto-ui.md` et `docs/cadrage/evo-mic-selection.md`.

Le desktop utilise egui/eframe et une séparation modèle, contrôleur, vue. La présentation doit rester dans cette architecture : la vue émet des commandes, les politiques d'import, moteur et traitement restent dans le cœur. La référence historique `refacto-ui.md` préserve l'expérience existante et ne vaut pas approbation de la nouvelle composition.

## 3. Décisions utilisateur intégrées

| ID | Décision confirmée | Modules concernés |
|---|---|---|
| DU-01 | Par défaut, le bouton principal effectue import puis lancement du traitement dans le même parcours. Un doublon présente une fenêtre supplémentaire. | UI-04, UI-09 |
| DU-02 | Le menu déroulant du bouton propose « Importer seulement », qui garde la file et ses actions « Traiter » et « Annuler ». | UI-04, UI-07 |
| DU-03 | La liste des traitements expose les états « Importé », « Traité » et « Annulé ». | UI-07 |
| DU-04 | Pour le traitement Parakeet, le résultat est considéré définitif à la fin du traitement sans étape additionnelle de vérification utilisateur ni présentation comme brouillon. | UI-06, UI-07 |
| DU-05 | Le spectre existant est conservé comme base de mesure et restylé selon la représentation de la maquette. | UI-05 |
| DU-06 | L'engrenage ouvre la configuration avancée : choix moteur et calcul CPU/GPU, autres réglages absents de la façade et configuration des dossiers par défaut. | UI-02, UI-03 |
| DU-07 | Les noms des fichiers importés ou traités sont des liens qui ouvrent le répertoire contenant le fichier correspondant. | UI-04, UI-07 |

Ces décisions modifient certains comportements existants ; elles ne se limitent pas à une retouche visuelle.

## 4. Carte des modules

| ID | Module | Responsabilité et référence |
|---|---|---|
| UI-01 | Coque et composition | Fond, largeur utile, cartes, adaptation au redimensionnement. En-tête et bandeau en haut, import/micro côte à côte, traitements en dessous. |
| UI-02 | En-tête et état effectif | Marque, moteur/modèle réellement utilisés, raccourci « Dossier de sortie » et engrenage vers les réglages avancés. |
| UI-03 | Configuration avancée | Moteur, Auto/CPU/GPU, modèle, réglages et chemins par défaut, en réutilisant les vues et contrôleurs existants. |
| UI-04 | Import audio | Sélection du fichier, action principale import+traitement, menu « Importer seulement », dialogue de doublon et lien vers le dossier source. |
| UI-05 | Microphone et spectre | Source, actualisation, test, jauge, spectre restylé, gain et démarrage. |
| UI-06 | Traitement actif et résultat | État, progression, moteur effectif, blocs micro confirmés, arrêt et publication du résultat définitif. |
| UI-07 | Liste des traitements et historique | Statuts demandés, actions Traiter/Annuler pour le mode import seul, noms-liens, résultats et historique. |
| UI-08 | Récupération | Avertissement et action définie précisément par le scénario récupérable. « Reprendre la session » de l'image n'est pas une exigence générique. |
| UI-09 | Messages et dialogues | Doublons, erreurs, indisponibilités, choix de reprise et confirmations. |

La maquette n'affiche pas le parcours en traitement. Une variante active est nécessaire pour fixer la place du texte confirmé, de l'arrêt, de la progression et du statut final.

## 5. UI-01/02/03 — Coque, en-tête et configuration avancée

### Composition de la façade

La façade met en évidence les deux modes d'entrée et les traitements récents. Le moteur effectif reste identifiable. Le raccourci « Dossier de sortie » ouvre le dossier de sortie actuellement configuré ; le bouton engrenage ouvre la configuration avancée sans encombrer la façade avec les réglages détaillés.

La barre système Windows standard est conservée. Les espacements et couleurs tirés de la capture servent de repères visuels estimés ; ils ne sont pas des tokens mesurés et la fidélité pixel exacte n'est pas exigée.

### Configuration avancée

Le panneau regroupe au minimum :

- préférence de calcul Auto/CPU/GPU et choix du moteur/modèle ;
- réglages avancés actuellement absents de la façade ;
- configuration des dossiers par défaut.

Réutiliser notamment `view/settings.rs`, `PathSettingsApp`, `SelectionWidgetState` et les contrôleurs de préférences. La préférence choisie et le moteur effectif ne doivent pas être confondus. Les règles actuelles de verrouillage pendant une tâche restent à appliquer.

Le panneau permet de régler le dossier initial de recherche audio et le dossier de sortie par défaut. La recherche audio privilégie le dernier dossier valide, puis le dossier initial configuré, puis la valeur de secours existante. Le modèle reste sélectionné par le catalogue actuel. Les chemins de workers, DLL, installation, caches et journaux ne sont pas exposés comme réglages utilisateur. Une nouvelle destination s'applique aux nouveaux traitements ; les entrées déjà admises conservent leur destination.

## 6. UI-04 — Import à deux modes

### Apparence et interaction

Conserver la carte d'import de la référence : titre, sous-titre, zone de sélection et bouton violet. Le bouton est scindé en action principale et menu déroulant :

1. action principale par défaut : « Importer et transcrire » ;
2. option de menu : « Importer seulement ».

La sélection multiple et le glisser-déposer sont visibles/suggérés par la référence mais ne sont pas explicitement demandés ici ; leur support doit être vérifié et cadré séparément.

### Parcours

| Mode | Résultat d'admission | Suite |
|---|---|---|
| Importer et transcrire | Une entrée admise est identifiée dans la file avec l'intention de traitement. | Le traitement démarre dès que le moteur est libre, sans second clic « Traiter ». Si un traitement est actif, l'entrée affiche « En attente » et sa position. |
| Importer seulement | Une ligne « Importé » apparaît. | Aucun traitement ne démarre sans clic ultérieur sur « Traiter ». |
| Doublon, dans l'un ou l'autre mode | Une fenêtre supplémentaire demande une décision. | Après résolution, le mode choisi initialement reste respecté. Annuler le dialogue n'engage ni remplacement ni traitement. |

Le comportement de doublon ne doit pas perdre l'intention initiale. Un choix qui résout un doublon ne doit pas démarrer le traitement si l'utilisateur avait choisi « Importer seulement ».

### Raccordement et risque

Actuellement, `controller/actions.rs::start_import` envoie `AppCommand::EnqueueImport`; `view/queue.rs` envoie plus tard `AppCommand::ProcessQueued` depuis le bouton « Traiter ». L'admission est asynchrone et peut résoudre l'identité de la nouvelle entrée. Le nouveau parcours doit attendre la confirmation et traiter l'identité effectivement admise, jamais une entrée supposée.

Les demandes « Importer et transcrire » et les demandes « Traiter » ajoutées par l'utilisateur entrent dans une file FIFO selon l'ordre de leur admission. Un seul traitement de cette file démarre à la fois ; un traitement ou une capture micro déjà actifs ne sont pas interrompus. La file démarre l'entrée suivante lorsque la ressource est libérée. Un import seul ne rejoint cette file qu'après le clic « Traiter ». Une entrée bloquée sur un dialogue de doublon ne bloque pas les autres entrées ; après résolution, elle rejoint la file selon l'ordre de confirmation. L'attente d'une capture micro affiche « En attente de la fin de l'enregistrement », sans estimation de durée. La fermeture de l'application convertit les intentions de traitement non démarrées en entrées interrompues à confirmer au prochain lancement ; elle ne lance pas un traitement de façon inattendue au démarrage.

Le nom du fichier source est cliquable et ouvre son répertoire contenant. La cible doit être le chemin réel de la source, y compris si le chemin de sortie diffère.

## 7. UI-05 — Microphone et description du spectre

### Contrôles microphone

Conserver le choix explicite ou défaut Windows, l'actualisation, le test microphone, la jauge, le gain et le démarrage. Le spectre ne doit pas masquer la mesure de niveau RMS/pic ni les messages de source indisponible, saturation ou mesure périmée. Les états et valeurs ne doivent pas être communiqués uniquement par la couleur.

### Données du spectre actuel

Le cœur fournit une FFT de 2048 trames et 1025 bins en densité spectrale (milli-dBFS/Hz), avec séries avant gain et après gain/limiteur. La vue actuelle trace deux courbes de fréquence linéaire entre 0 et Nyquist, plage commune −120 à 0 dBFS/Hz, hauteur 116 unités logiques ; une mesure âgée de plus de 500 ms est signalée périmée.

La rangée de la maquette fait environ 675 × 24 pixels raster et ressemble à une jauge segmentée. La décision utilisateur impose de conserver le spectre existant comme base : les capsules doivent donc représenter des bandes fréquentielles, pas un simple niveau global rempli depuis la gauche.

### Représentation visuelle proposée

- Bande horizontale sur la largeur de la carte, fréquences basses à gauche et hautes à droite, de 0 Hz à Nyquist.
- Piste arrondie bleu sombre par bande ; portion remplie verticalement depuis le bas en turquoise/vert selon l'amplitude spectrale de cette bande.
- Valeur d'une fréquence représentée indépendamment des autres ; pas d'animation simulée lorsque les données n'évoluent pas.
- Hauteur de référence proche de 24 pixels raster ; capsules fines et régulièrement espacées, environ 7–8 pixels de large avec 4 pixels d'intervalle à l'échelle de la capture. Adapter aux unités logiques et DPI sans imposer ces mesures raster.
- Agréger les 1025 bins en 64 bandes fixes de fréquence linéaire entre 0 Hz et Nyquist. Pour chaque bande, calculer la moyenne pondérée de la puissance linéaire, puis la présenter en dBFS/Hz. Le nombre de bandes et leur signification fréquentielle restent stables au redimensionnement ; seule leur largeur visuelle change.
- Indiquer la plage −120 à 0 dBFS/Hz et les extrêmes fréquentiels, ou fournir ces informations accessibles au clavier/lecteur d'écran.
- Silence : pistes neutres et état « Silence ». Aucune mesure : état d'attente. Mesure périmée (>500 ms selon le contrat actuel) : état neutre avec indication visible. La saturation est affichée séparément.

La façade affiche la série après gain/limiteur. La comparaison avant/après est disponible dans les détails avancés. La plage verticale reste −120 à 0 dBFS/Hz. Chaque capsule représente une bande indépendante, dont la hauteur remplie depuis le bas suit l'amplitude normalisée ; les pistes inactives restent bleu sombre et le signal est turquoise. La saturation demeure un indicateur séparé.

## 8. UI-06 — Traitement Parakeet et résultat définitif

La décision utilisateur supprime l'étape additionnelle de vérification par l'utilisateur et la présentation du résultat comme brouillon pour le traitement Parakeet. À la fin réussie du traitement et de la publication, le résultat est directement présenté comme définitif et disponible.

« Définitif » décrit le statut de livraison du résultat par l'application ; cela ne promet pas l'absence d'erreur de transcription.

Dans `view/capture.rs`, le flux actuel expose « Brouillon final — non vérifié », « à relire » et une action de lecture. Le cœur porte également `AutomaticFinalTranscriptionStatus::ProducedUnverified`. Une simple modification du libellé ne suffit donc pas à définir les transitions et le moment où le résultat devient final.

Proposition de parcours : préparation → capture/traitement avec affichage des blocs micro confirmés → publication → résultat définitif, ou erreur/échec de publication. Les blocs confirmés visibles pendant la capture restent identifiés comme tels ; ils ne sont pas confondus avec le résultat final Parakeet. Le texte provisoire, s'il est affiché, garde une distinction supplémentaire et ne remplace pas les blocs confirmés. Ne jamais montrer un résultat comme publié tant que les fichiers attendus ne sont pas accessibles.

À l'arrêt du microphone, le parcours produit doit rendre accessibles TXT, SRT et WAV de même nom après finalisation. L'absence ou l'échec de publication d'un des fichiers doit rester visible et ne pas être présenté comme une finalisation réussie.

La fin du traitement Parakeet inclut tout calcul automatique final déjà requis par le flux, ses contrôles techniques, et la publication. Cette séquence reste une seule opération utilisateur : aucun bouton de vérification, relecture obligatoire ou statut brouillon. Le calcul automatique final n'est pas supprimé par cette décision. Import, capture historique et session unifiée utilisent le résultat publié comme document définitif selon leur contrat existant ; aucun deuxième calcul n'est ajouté à la session unifiée pour harmoniser l'interface.

## 9. UI-07 — Liste, statuts, annulation et liens

### Statuts demandés

| Libellé demandé | Sens proposé | Précaution |
|---|---|---|
| Importé | Admission confirmée en mode « Importer seulement », traitement non demandé. | Ne pas confondre avec une admission en attente de choix ou échouée. |
| Traité | Traitement et publication du résultat définitif achevés. | Ne pas l'afficher à la fin du calcul si la publication échoue. |
| Annulé | L'utilisateur a annulé une demande en attente ou un traitement actif. | Conserver la ligne dans l'historique ; ne pas supprimer source ni résultat déjà publié. |

Ces trois états ne suffisent pas à décrire le cycle en cours. « Traitement… », « Annulation… », « Interrompu » et « Échec » peuvent être nécessaires. Les états transitoires doivent rester distincts d'« Annulé » : une erreur ou interruption n'est pas une annulation utilisateur.

La file actuelle porte `Queued`, `AwaitingChoice`, `Running` et `Interrupted`; « Retirer » enlève une entrée en attente. La nouvelle liste conserve les états « En attente », « Traitement… », « Annulation… », « Interrompu » et « Échec » en plus des trois statuts demandés. « Annuler » enlève une intention en attente ou demande l'arrêt coopératif d'un traitement actif ; la ligne reste dans l'historique et les fichiers ne sont pas supprimés. Si la publication a réussi avant la confirmation d'arrêt, l'état reste « Traité ». Une annulation suivie d'un nouvel essai crée une nouvelle entrée et identité ; les règles de doublon continuent de s'appliquer aux fichiers déjà publiés, pas au seul fait qu'une entrée précédente a été annulée. Les annulations sont conservées avec l'historique, sans expiration propre à ce statut.

### Liens vers les dossiers

- Une ligne importée expose le nom de la source comme lien vers le répertoire parent de cette source.
- Une ligne traitée conserve le lien du nom audio vers le dossier source et ajoute le nom TXT publié comme lien vers le répertoire contenant les résultats.
- Si source et résultat sont dans des dossiers différents, les deux liens restent visibles et nommés explicitement ; une même cible ne change pas selon l'état.
- Un chemin devenu indisponible déclenche un message clair ; l'application ne crée pas de répertoire et ne substitue pas silencieusement une autre cible.

Le nom TXT est le lien principal du résultat, car il ouvre le dossier commun aux fichiers TXT/SRT/WAV de même nom. Le bouton « Ouvrir » de la maquette est remplacé par les liens source/résultat pour éviter une cible ambiguë. Le lien ouvre le dossier contenant, sans lancer d'éditeur ou de lecteur.

L'archive actuelle ne fournit pas directement toutes les colonnes de la maquette (date, durée, extrait, nom convivial). La projection de données nécessaires et la portée de l'historique restent à concevoir avant de promettre ces colonnes.

## 10. UI-08/09 — Récupération, messages et dialogues

La maquette montre « Un enregistrement en direct a été interrompu », « Reprendre la session » et « Ignorer ». Cette vue n'établit pas la signification de « reprendre ». Les scénarios de reprise d'import/publication déjà présents doivent rester identifiés selon leur opération ; une reprise générique du microphone n'est pas autorisée par cette image. « Ignorer » ne doit pas supprimer de données sans confirmation et décision explicite.

Le dialogue de doublon est requis par DU-01/02. Il doit montrer l'identité du fichier déjà connu et présenter des issues non ambiguës, tout en préservant le mode initial choisi. Les libellés exacts et les effets existants de remplacement/réutilisation doivent être raccordés aux contrats du cœur.

Les messages partagés couvrent les erreurs de source, de traitement, de publication, de moteur/GPU et de chemins. Distinguer la préférence Auto/CPU/GPU du moteur effectif et signaler explicitement un éventuel repli.

## 11. Critères observables proposés

| ID | Statut | Vérification future |
|---|---|---|
| IMP-01 | Confirmé | Avec un WAV/MP3 accepté et un moteur disponible, l'action principale admet le fichier puis déclenche le traitement sans second clic. |
| IMP-02 | Confirmé | « Importer seulement » crée une entrée importée et ne démarre aucun traitement avant « Traiter ». |
| IMP-03 | Confirmé | Un doublon déclenche un dialogue et, après résolution, respecte le choix initial entre traitement immédiat et import seul. |
| IMP-04 | Confirmé | Annuler le dialogue de doublon n'engage ni traitement ni remplacement. |
| STA-01 | Confirmé | Les statuts Importé/Traité/Annulé correspondent aux transitions définies en §9/§12 ; erreur, interruption, attente et traitement en cours restent distinguables. |
| STA-02 | Confirmé | L'annulation d'une entrée laisse la source intacte et n'efface pas un résultat déjà publié ; le statut reste dans l'historique. |
| IMP-05 | Choix de cadrage retenu | Avec un traitement A actif, admettre B en mode immédiat ; B attend puis démarre après A. Importer C en mode import seul, cliquer « Traiter » pendant B, puis vérifier que C démarre après B, dans cet ordre et sans traitements simultanés. |
| IMP-06 | Choix de cadrage retenu | Fermer l'application avec une demande immédiate en attente ; au lancement suivant, elle apparaît interrompue et ne redémarre qu'après confirmation. |
| PAR-01 | Confirmé | Après publication réussie du résultat Parakeet, il est marqué définitif sans validation/lecture humaine obligatoire ni libellé « brouillon/non vérifié ». |
| PAR-02 | Confirmé | Pendant la capture micro, les blocs confirmés sont visibles et identifiés comme confirmés, distincts du résultat définitif et de tout texte provisoire. |
| PAR-03 | Confirmé | Après arrêt et finalisation micro réussis, TXT, SRT et WAV de même nom sont accessibles ; aucun succès final n'est annoncé si l'un des fichiers attendus n'est pas publié. |
| PAR-04 | Proposition | Si le traitement ou la publication échoue, le résultat n'est pas présenté comme définitif. |
| SPE-01 | Confirmé | Un signal fréquentiel sur une plage donnée modifie les capsules correspondantes, pas toute la rangée comme une jauge unique. |
| SPE-02 | Proposition | Silence, mesure absente, périmée et saturation ont des états distincts visibles. |
| CFG-01 | Confirmé | L'engrenage ouvre la configuration avancée où sont accessibles les choix moteur/calcul et les dossiers par défaut acceptés. |
| CFG-02 | Choix de cadrage retenu | Le raccourci « Dossier de sortie » ouvre le dossier de sortie courant ; si le dossier n'est pas accessible, un message l'indique sans créer un répertoire de remplacement. |
| LNK-01 | Confirmé | Le lien du nom d'une source importée ouvre le dossier contenant la source. |
| LNK-02 | Confirmé | Le lien d'un résultat traité ouvre le dossier contenant le résultat associé. |
| LNK-03 | Choix de cadrage retenu | Si le dossier n'est plus accessible, l'interface signale l'échec sans créer ni substituer un dossier. |
| VIS-01 | Choix de cadrage retenu | À 1536 × 864 unités logiques, l'accueil respecte la hiérarchie de la référence ; sous 1200 unités, les cartes s'empilent et l'historique devient compact. |
| ACC-01 | Proposition | Contrôles nommés, focus visible et états lisibles sans dépendre de la couleur seule. |

La validation finale du rendu dans la fenêtre Windows reste à observer par l'utilisateur selon le lot autorisé. Aucun de ces critères n'a été exécuté dans ce travail de cadrage.

## 12. Décisions retenues pour la fluidité

À la demande de l'utilisateur, la session principale a choisi les précisions ci-dessous pour rendre les parcours fluides et prévisibles. Elles complètent DU-01 à DU-07 sans autoriser le code. Elles devront figurer dans un plan avant toute implémentation.

| Sujet | Décision retenue |
|---|---|
| Moteur occupé | File FIFO automatique pour les actions « Importer et transcrire » et les « Traiter » explicites. Un seul travail démarre à la fois ; aucun préemptage. Les attentes et leur position sont visibles. Les imports seuls ne sont jamais lancés automatiquement. |
| Annulation | Une ligne « Annulé » reste consultable dans l'historique, sans expiration propre à ce statut. Annuler ne supprime pas l'audio source ni un résultat publié. Une annulation en cours est coopérative ; un résultat déjà publié reste « Traité ». |
| Réessai | Un nouvel essai crée une nouvelle entrée. L'annulation précédente ne compte pas comme transcription terminée ; les protections anti-doublon restent actives pour les résultats déjà publiés. |
| Liens | Nom audio → dossier source. Nom TXT final → dossier des résultats contenant TXT/SRT/WAV. Les deux liens restent distincts. |
| Dossiers | Panneau avancé : dossier initial de recherche audio et dossier de sortie par défaut. La recherche audio utilise d'abord le dernier dossier valide. La sélection du modèle conserve son mécanisme actuel ; aucun dossier technique n'est exposé. |
| Parakeet | Supprimer le statut et l'étape de vérification humaine. Garder les calculs automatiques finaux existants et les contrôles techniques dans une opération unique ; le résultat devient définitif après publication réussie. |
| Spectre | 64 bandes linéaires stables, agrégées en puissance, série après gain/limiteur en façade et avant/après dans les détails. Capsules turquoise remplies depuis le bas selon l'amplitude de chaque bande. |
| Fenêtre | Chrome Windows standard. Viewport de référence 1536 × 864 unités logiques à 100 %, adaptation compacte à 1024 × 720, minimum 960 × 640. Empiler import puis microphone sous 1200 unités de largeur ; contrôler aussi l'échelle Windows à 150 % et 200 %. |

### Contrôles et détails d'interaction

- Les demandes immédiates sont ordonnées au moment où l'admission est confirmée. Si l'application ferme avant le début du traitement, elles réapparaissent comme interrompues et demandent confirmation avant de reprendre ; aucun calcul ne démarre de façon inattendue à l'ouverture.
- Une entrée qui attend une décision de doublon ne bloque pas la file ; après résolution elle rejoint la file FIFO. Le choix initial (immédiat ou import seul) reste associé au parcours.
- Une entrée importée seule reste dans cet état jusqu'au clic « Traiter ». Une erreur, interruption ou annulation ne devient jamais le statut « Traité ».
- L'historique conserve les entrées annulées selon sa politique générale. Les annulations seules ne déclenchent ni nouvelle règle d'effacement de fichiers ni suppression automatique de sources.
- Le regroupement des 1025 bins en 64 bandes utilise une moyenne pondérée de puissance linéaire par intervalle fréquentiel, puis une conversion en dBFS/Hz. Les bornes restent régulières entre 0 Hz et Nyquist.
- Tailles de fenêtre en unités logiques ; les cibles pixel de la capture ne sont pas appliquées telles quelles aux écrans DPI élevés. Sous 1200 unités, cartes empilées et liste d'historique compacte avec défilement vertical ; l'action d'arrêt reste accessible pendant une capture.

Ces décisions ferment les questions de comportement de la version précédente du brief. Le plan devra encore définir les détails de mise en œuvre, leurs critères de test et la validation Windows. Aucun test ni contrôle dans la fenêtre n'a été exécuté pour ce cadrage.
