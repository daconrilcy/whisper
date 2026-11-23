Avancée : décomposition de la maquette mise à jour avec les décisions utilisateur du 23 novembre 2026.
Blocage : quelques détails de cycle de vie restent à préciser avant un plan d'implémentation.
Décision : brief de conception uniquement ; aucune refonte ni implémentation n'est autorisée par ce document.

# Brief UI — maquette du transcripteur Whisper

## 1. Objet et autorité

Ce brief décompose l'image jointe le 23 novembre 2026 et intègre les décisions complémentaires données par l'utilisateur dans cette session. Il sert au cadrage de l'interface Windows Whisper existante. Il ne donne pas d'autorisation d'implémentation.

L'image est une référence de composition et d'apparence. Ses noms de fichiers, dates, durées, extraits, modèle/GPU affichés et message de reprise sont des données ou états illustratifs, sauf décision explicite rapportée dans ce brief.

Les décisions marquées **confirmées** viennent de l'objectif produit ou des précisions utilisateur. Les propositions de style et les détails indiqués « à décider » ne deviennent des exigences qu'après acceptation.

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
| UI-02 | En-tête et état effectif | Marque, moteur/modèle réellement utilisés et engrenage vers les réglages avancés. |
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

La façade met en évidence les deux modes d'entrée et les traitements récents. Le moteur effectif reste identifiable. Le bouton engrenage est le point d'entrée vers la configuration avancée ; les réglages détaillés n'encombrent pas la façade.

La capture ne précise pas si la barre système Windows est remplacée. Le chrome personnalisé, les effets de lueur et le niveau de fidélité pixel sont des choix visuels à confirmer. Les espacements et couleurs tirés de la capture sont des estimations, pas encore des tokens mesurés.

### Configuration avancée

Le panneau regroupe au minimum :

- préférence de calcul Auto/CPU/GPU et choix du moteur/modèle ;
- réglages avancés actuellement absents de la façade ;
- configuration des dossiers par défaut.

Réutiliser notamment `view/settings.rs`, `PathSettingsApp`, `SelectionWidgetState` et les contrôleurs de préférences. La préférence choisie et le moteur effectif ne doivent pas être confondus. Les règles actuelles de verrouillage pendant une tâche restent à appliquer.

« Dossiers par défaut » doit être raccordé aux concepts existants. L'UI conserve notamment le dernier dossier audio et la destination ; certains chemins sont composés par `root` et ne sont pas aujourd'hui des réglages utilisateur. Ne pas exposer automatiquement les chemins de workers, DLL ou installation. Le périmètre exact des dossiers réglables reste à confirmer.

## 6. UI-04 — Import à deux modes

### Apparence et interaction

Conserver la carte d'import de la référence : titre, sous-titre, zone de sélection et bouton violet. Le bouton est scindé en action principale et menu déroulant :

1. action principale par défaut : « Importer et transcrire » ;
2. option de menu : « Importer seulement ».

La sélection multiple et le glisser-déposer sont visibles/suggérés par la référence mais ne sont pas explicitement demandés ici ; leur support doit être vérifié et cadré séparément.

### Parcours

| Mode | Résultat d'admission | Suite |
|---|---|---|
| Importer et transcrire | Une entrée admise est identifiée dans la file. | Le traitement démarre sans second clic « Traiter ». |
| Importer seulement | Une ligne « Importé » apparaît. | L'utilisateur peut ensuite choisir « Traiter » ou « Annuler ». |
| Doublon, dans l'un ou l'autre mode | Une fenêtre supplémentaire demande une décision. | Après résolution, le mode choisi initialement reste respecté. Annuler le dialogue n'engage ni remplacement ni traitement. |

Le comportement de doublon ne doit pas perdre l'intention initiale. Un choix qui résout un doublon ne doit pas démarrer le traitement si l'utilisateur avait choisi « Importer seulement ».

### Raccordement et risque

Actuellement, `controller/actions.rs::start_import` envoie `AppCommand::EnqueueImport`; `view/queue.rs` envoie plus tard `AppCommand::ProcessQueued` depuis le bouton « Traiter ». L'admission est asynchrone et peut résoudre l'identité de la nouvelle entrée. Le nouveau parcours doit attendre la confirmation et traiter l'identité effectivement admise, jamais une entrée supposée.

Si le moteur est déjà occupé, définir si l'admission immédiate est refusée avec un message ou si elle attend en file pour démarrer ensuite. Recommandation de cadrage : rendre l'attente et son état explicites, sans lancer un deuxième traitement simultané.

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
- Agréger les 1025 bins en bandes d'affichage selon la largeur disponible. La règle d'agrégation doit préserver la relation d'amplitude et documenter l'unité ; le nombre de bandes exact reste à décider.
- Indiquer la plage −120 à 0 dBFS/Hz et les extrêmes fréquentiels, ou fournir ces informations accessibles au clavier/lecteur d'écran.
- Silence : pistes neutres et état « Silence ». Aucune mesure : état d'attente. Mesure périmée (>500 ms selon le contrat actuel) : état neutre avec indication visible. La saturation est affichée séparément.

Le choix de présenter en façade la série après gain seulement, ou les vues avant/après, reste à décider. Recommandation : série après gain/limiteur sur la façade et comparaison avant/après dans les détails avancés. Ce choix déplacerait un élément actuellement visible et doit être accepté dans le cadrage.

## 8. UI-06 — Traitement Parakeet et résultat définitif

La décision utilisateur supprime l'étape additionnelle de vérification par l'utilisateur et la présentation du résultat comme brouillon pour le traitement Parakeet. À la fin réussie du traitement et de la publication, le résultat est directement présenté comme définitif et disponible.

« Définitif » décrit le statut de livraison du résultat par l'application ; cela ne promet pas l'absence d'erreur de transcription.

Dans `view/capture.rs`, le flux actuel expose « Brouillon final — non vérifié », « à relire » et une action de lecture. Le cœur porte également `AutomaticFinalTranscriptionStatus::ProducedUnverified`. Une simple modification du libellé ne suffit donc pas à définir les transitions et le moment où le résultat devient final.

Proposition de parcours : préparation → capture/traitement avec affichage des blocs micro confirmés → publication → résultat définitif, ou erreur/échec de publication. Les blocs confirmés visibles pendant la capture restent identifiés comme tels ; ils ne sont pas confondus avec le résultat final Parakeet. Le texte provisoire, s'il est affiché, garde une distinction supplémentaire et ne remplace pas les blocs confirmés. Ne jamais montrer un résultat comme publié tant que les fichiers attendus ne sont pas accessibles.

À l'arrêt du microphone, le parcours produit doit rendre accessibles TXT, SRT et WAV de même nom après finalisation. L'absence ou l'échec de publication d'un des fichiers doit rester visible et ne pas être présenté comme une finalisation réussie.

La portée exacte de « traitement Parakeet » doit être reliée aux flux concernés (import, capture historique, session unifiée). La présente décision supprime l'étape de vérification humaine ; elle ne supprime pas par défaut un calcul automatique complet déjà prévu par un flux. Si un second calcul automatique est visé, son coût et le document faisant autorité doivent être décidés séparément.

## 9. UI-07 — Liste, statuts, annulation et liens

### Statuts demandés

| Libellé demandé | Sens proposé | Précaution |
|---|---|---|
| Importé | Admission confirmée, traitement non démarré. | Ne pas confondre avec une admission en attente de choix ou échouée. |
| Traité | Traitement et publication du résultat définitif achevés. | Ne pas l'afficher à la fin du calcul si la publication échoue. |
| Annulé | L'utilisateur a annulé une entrée importée en attente. | Conserver la ligne si le statut doit rester consultable ; ne pas supprimer le fichier source. |

Ces trois états ne suffisent pas à décrire le cycle en cours. « Traitement… », « Annulation… », « Interrompu » et « Échec » peuvent être nécessaires. Les états transitoires doivent rester distincts d'« Annulé » : une erreur ou interruption n'est pas une annulation utilisateur.

La file actuelle porte `Queued`, `AwaitingChoice`, `Running` et `Interrupted`; « Retirer » enlève une entrée en attente. L'arrêt d'un travail actif est une autre opération. Le libellé « Annuler » doit conduire à une action définie selon que l'entrée est en attente ou en cours. La décision de ne pas effacer les sources/résultats par annulation doit être conservée, sauf décision produit ultérieure explicite.

### Liens vers les dossiers

- Une ligne importée expose le nom de la source comme lien vers le répertoire parent de cette source.
- Une ligne traitée expose un nom associé au résultat publié comme lien vers le répertoire parent du résultat.
- Si source et résultat sont dans des dossiers différents, présenter deux liens nommés explicitement plutôt qu'une cible qui change selon l'état.
- Un chemin devenu indisponible déclenche un message clair ; l'application ne crée pas de répertoire et ne substitue pas silencieusement une autre cible.

La cible exacte du nom affiché après traitement doit être fixée (fichier source, TXT, SRT ou dossier des résultats). Le bouton « Ouvrir » de la maquette n'est pas nécessaire s'il duplique le lien ; une action séparée pour ouvrir un document est à accepter explicitement.

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
| STA-01 | Libellés confirmés, sémantique à valider | Les statuts Importé/Traité/Annulé correspondent aux transitions définies ; erreur, interruption et traitement en cours restent distinguables. |
| STA-02 | Proposition | L'annulation d'une entrée importée laisse la source intacte et n'efface pas un résultat déjà publié. |
| PAR-01 | Confirmé | Après publication réussie du résultat Parakeet, il est marqué définitif sans validation/lecture humaine obligatoire ni libellé « brouillon/non vérifié ». |
| PAR-02 | Confirmé | Pendant la capture micro, les blocs confirmés sont visibles et identifiés comme confirmés, distincts du résultat définitif et de tout texte provisoire. |
| PAR-03 | Confirmé | Après arrêt et finalisation micro réussis, TXT, SRT et WAV de même nom sont accessibles ; aucun succès final n'est annoncé si l'un des fichiers attendus n'est pas publié. |
| PAR-04 | Proposition | Si le traitement ou la publication échoue, le résultat n'est pas présenté comme définitif. |
| SPE-01 | Confirmé | Un signal fréquentiel sur une plage donnée modifie les capsules correspondantes, pas toute la rangée comme une jauge unique. |
| SPE-02 | Proposition | Silence, mesure absente, périmée et saturation ont des états distincts visibles. |
| CFG-01 | Confirmé | L'engrenage ouvre la configuration avancée où sont accessibles les choix moteur/calcul et les dossiers par défaut acceptés. |
| LNK-01 | Confirmé | Le lien du nom d'une source importée ouvre le dossier contenant la source. |
| LNK-02 | Confirmé | Le lien d'un résultat traité ouvre le dossier contenant le résultat associé. |
| LNK-03 | Proposition | Si le dossier n'est plus accessible, l'interface signale l'échec sans créer ni substituer un dossier. |
| VIS-01 | Proposition | À viewport et DPI fixés, l'accueil respecte la hiérarchie de la référence : en-tête, entrées côte à côte, puis liste. |
| ACC-01 | Proposition | Contrôles nommés, focus visible et états lisibles sans dépendre de la couleur seule. |

La validation finale du rendu dans la fenêtre Windows reste à observer par l'utilisateur selon le lot autorisé. Aucun de ces critères n'a été exécuté dans ce travail de cadrage.

## 12. Questions de cadrage restantes

1. Quand le moteur est occupé, le traitement immédiat attend-il en file ou affiche-t-il une indisponibilité ?
2. « Annuler » conserve-t-il une ligne annulée dans la liste, et combien de temps ? Comment un fichier annulé est-il réimporté ?
3. Quel fichier exact le lien d'une entrée traitée désigne-t-il lorsque TXT/SRT/WAV sont produits dans un dossier distinct ?
4. Quels dossiers par défaut sont configurables depuis le panneau avancé, au-delà des dossiers audio et sortie ?
5. Pour Parakeet, la décision retire-t-elle uniquement l'étape de vérification humaine, ou aussi un calcul final automatique distinct ? Recommandation : conserver le calcul automatique tant qu'un changement de qualité/coût et la source faisant autorité n'ont pas été cadrés.
6. Le spectre affiche-t-il avant et après gain ensemble, ou seulement la série après traitement en façade ? Quelle règle d'agrégation des bins est acceptée ?
7. Quelles tailles logiques/DPI et quel niveau de fidélité visuelle servent à vérifier la maquette ?

Les réponses sont nécessaires avant le plan d'implémentation de la nouvelle UI. Le présent brief ne modifie pas le code et ne crée pas d'autorisation de le modifier.
