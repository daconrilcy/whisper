Avancée : design du retraitement ciblé établi sur le mandat du 20/10/2026 et les deux décisions complémentaires de l'utilisateur.
Blocage : le coût de bout en bout du parcours R11 actuel n'est pas mesuré ; sa campagne exige un accord précis.
Décision : proposer une source figée vérifiée et une suppression ciblée, puis soumettre ce design à la revue indépendante avant tout plan ou code.

# DESIGN-RETRAITEMENT-CIBLE v1 — candidat à la revue

Base : `evo-startup-perf` à `e4b0ffca7169f8db69113a66af089886b93de9f5`. Auteur documentaire : session Codex principale. Contributions en lecture seule : `rust_product_framer`, `rust_requirements_analyst`, `rust_domain_architect`, `rust_architect`. Source d'autorité : [mandat de cadrage](mandat-retraitement-cible.md), complété le 20/10/2026 par les réponses de l'utilisateur : « tous » comprend aussi les traitements R11 remplacés, vérifiés et présents ; après une interruption du nettoyage, l'application affiche les éléments restants et demande une reprise explicite. **Statut : DRAFT, sans autorisation d'implémentation, de campagne supplémentaire, de suppression de données utilisateur ou de push.** Le [plan R11](plan-retraitement-audio.md) et son [bilan](bilan-retraitement-audio.md) décrivent l'existant ; les nouvelles décisions ne sont pas assimilées à des fonctions déjà livrées.

## 1. Besoin, résultat et périmètre

Pour un audio reconnu par le SHA-256 de ses **octets**, l'utilisateur peut créer un traitement neuf sans dépendre des anciens, ou remplacer exactement les anciens traitements qu'il confirme : dernier publié avec succès, tous les traitements vérifiés encore présents, ou un traitement choisi par Parcourir dans la destination actuelle. Un MP3 et un WAV aux octets différents sont distincts, même si leur son se ressemble. Le nouveau résultat est publié dans un dossier unique avant toute suppression ; il possède sa seule paire finale `transcript.txt` et `transcript.srt` dans ce dossier. Aucun nouvel export TXT/SRT n'est publié sous `exports/`.

Le remplacement doit préserver les anciens pendant le calcul et conserver le nouveau si le nettoyage échoue. Les exports historiques non sélectionnés restent en place. La fermeture du dialogue annule l'import avant confirmation ; après publication, une reprise destructive n'a lieu que sur demande explicite de l'utilisateur. Aucun seuil de latence, aucune migration globale, aucun changement de moteur ou de durée limite n'est introduit. La recette dans la fenêtre Windows reste une validation ultérieure par l'utilisateur.

## 2. Exigences et acceptation

Tous les critères ci-dessous sont des **scénarios futurs NOT RUN**. Ils tracent les décisions du mandat ; ils ne prétendent pas qualifier le candidat.

| Exigence | Résultat vérifiable | Scénarios d'acceptation |
| --- | --- | --- |
| RTC-01 Identité | Même contenu renommé = même audio ; MP3/WAV aux octets différents = audios distincts ; anciens repérés par leur SHA enregistré sans relire leur ancien audio. | Copie WAV renommée, MP3 du même son, source illisible. |
| RTC-02 Création | Chaque choix « Créer un nouveau » réserve un dossier inédit et traite sans vérifier l'intégrité des anciens dossiers ; ceux-ci restent inchangés. | Anciens intact, absent et corrompu ; deux créations consécutives. |
| RTC-03 Sélection | « Dernier » vise une publication réussie d'ordre prouvé ; « Tous » inclut actifs et R11 inactifs présents et vérifiés ; la sélection confirmée est un ensemble concret figé. | Plusieurs succès, pending/échec plus récents, R11 inactif, autre audio ; aucune extension au redémarrage. |
| RTC-04 Parcourir | Un traitement absent de la liste peut être choisi uniquement dans la destination courante, avec publication Whisper, même SHA et propriété prouvés ; un dossier inconnu est refusé avant traitement. | Dossier valide hors liste, autre audio, tiers, extérieur, lien sortant. |
| RTC-05 Publication | Nouveau dossier et paire vérifiés avant d'annoncer le succès ; aucune suppression d'ancien avant ce succès ; échec ou annulation préalable laisse les anciens intacts. | Arrêt à chaque phase, paire incomplète, collision de dossier. |
| RTC-06 Identité avant suppression | Les octets consommés sont liés au SHA confirmé. Juste avant le nettoyage, la source, le nouveau publié et les cibles portent encore les identités attendues ; source changée ou non vérifiable = **zéro ancien supprimé**. | Mutation après admission, pendant calcul et avant suppression, y compris même taille/date restaurées. |
| RTC-07 Nettoyage | Seuls dossiers et exports historiques sélectionnés et possédés sont retirés ; un tiers ou une preuve manquante reste, avec motif ; le nouveau reste publié en cas d'échec partiel. | Export possédé/non possédé, fichier tiers, suppression refusée, cible substituée. |
| RTC-08 Reprise | Après interruption post-publication, montrer nouveau et éléments restants, puis demander la reprise ; revalider la sélection initiale, sans nouvelle cible implicite ni double publication. | Arrêt avant première suppression et après une suppression sur plusieurs. |
| RTC-09 Sortie unique | Une seule paire TXT/SRT finale par nouveau traitement, dans sa génération ; aucun nouveau TXT/SRT final sous `exports/` ; historiques non ciblés intacts. | Inventaire disque et chemins présentés après nouveau/dernier/tous/choisi. |
| RTC-10 Coût | Mesurer le parcours actuel, puis démontrer moins de lectures complètes de l'original sans affaiblir RTC-06 ; distinguer lectures adaptateur/worker, probe, scan, copie et temps total. | WAV court, WAV proche 30 min, MP3 représentatif si disponible ; premier et second passages de même session. |
| RTC-11 Non-régression | CPU/GPU/Auto, Turbo/Parakeet, micro, file/reprise et refus de durée gardent leurs règles hors retraitement. | Tests de contrats pertinents ; liste des recettes réellement exécutées et NOT RUN. |

## 3. Parcours et états

La destination choisie est la frontière de recherche et de suppression. L'interface transporte des identités de traitement, pas un simple chemin donnant droit à la suppression. Le travail coûteux s'exécute sur le service de fond existant, jamais sur le fil UI. Les réponses tardives portent l'opération et la génération auxquelles elles appartiennent.

| État visible | État durable et transition autorisée | Échec, fermeture ou annulation |
| --- | --- | --- |
| Identification | Source vérifiée ; recherche des précédents pour Remplacer. | Source illisible : refus. Pour Créer, un ancien corrompu ne bloque pas le traitement. |
| Choix requis | Dernier/tous/choisi résolus en cibles précises ; confirmation enregistrée. | Fermer annule l'import sans effet sur les anciens. Ordre du dernier indémontrable : demander un choix explicite. |
| En cours | Nouveau pending et source figée ; anciens disponibles. | Échec/annulation avant publication : aucune suppression d'ancien. |
| Publication | Paire locale et données de contrôle vérifiées, puis point de visibilité durable. | Une paire seulement partielle n'est jamais annoncée publiée. |
| Nouveau publié, nettoyage à confirmer ou en cours | Intention durable et progression par cible ; contrôles destructifs avant la première suppression. | Source ou cible incertaine : conserver le nouveau et tous les anciens non supprimés ; afficher la cause. |
| Terminé ou anciens restants | Nouveau consultable, état réel de chaque cible conservé. | Au redémarrage, présenter les restants ; reprise seulement après commande explicite et nouvelle vérification. |

Une suppression déjà réalisée n'est pas annulée fictivement après panne. Une reprise peut constater l'absence d'un élément attendu, mais ne suit jamais un nouveau fichier placé au même chemin sans revalider son identité.

## 4. Lectures actuelles : preuve de code et limite des temps

Dans le code courant, `archive.rs:2118-2129` calcule SHA → durée → SHA ; `worker_ipc.rs:2100` l'appelle à l'admission. `worker_ipc.rs:2211` répète l'identification à la confirmation de doublon, aussi bien pour Remplacer que pour Créer un nouveau. `archive.rs:2182-2204` effectue encore SHA → durée → SHA à la préparation. `published_imports` lit les identités enregistrées (`archive.rs:1620-1649`) ; il ne rehash pas les anciens fichiers audio. Ces nombres sont **statiques et conditionnels**, pas un décompte mesuré d'un import complet.

| Parcours actuel | SHA complets de l'original côté adaptateur | Probes de durée | Scans de publications | Lecture worker |
| --- | ---: | ---: | --- | --- |
| Import sans dialogue de doublon | 4 : deux à l'admission, deux à la préparation | 2 | Au moins admission et préparation, suivant réservation | Turbo : contrôles avant/après décodage ; Parakeet : SHA avant décodage. À compter séparément. |
| Dialogue, puis Créer un nouveau | 6 : admission, confirmation, préparation | 3 | Admission, confirmation, préparation | Identique selon moteur, hors compte adaptateur. |
| Dialogue, puis Remplacer | 6, plus deux par reconfirmation éventuelle | 3, plus une par reconfirmation | Admission, confirmation, préparation et contrôles de remplacement | Identique selon moteur, hors compte adaptateur. |

`archive.rs:4728-4748` journalise déjà chaque `archive_sha256` ; `:4753-4766` journalise `archive_audio_probe` avec la campagne R7. Les traces **historiques R9**, prises sur d'autres binaires et parcours, montrent par PID deux SHA de source de 9 600 044 octets autour de 93–97 ms chacun pour `spkr0.wav`, et deux SHA de 57 600 078 octets à 562,98 et 563,38 ms pour `spkr0_x6.wav` ; le probe y est voisin de 0,13 ms. Le journal EN5 contient plusieurs PID indépendants : leurs lectures ne sont pas additionnées en un seul import. Ces valeurs ne démontrent ni le coût R11 actuel ni un gain futur. Les 22,4/64,6 ms ponctuels de `Get-FileHash` consignés dans le mandat sont également hors parcours applicatif. **Temps R11 actuel, MP3, scans et bout en bout : NOT RUN.**

## 5. Comparaison des stratégies et choix proposé

| Stratégie | Lectures complètes de l'original côté adaptateur | Garantie et coût | Décision proposée |
| --- | ---: | --- | --- |
| Réutiliser SHA et métadonnées, puis SHA final | Environ 2 | Une modification transitoire pendant le décodage peut échapper aux contrôles ; ne prouve pas les octets réellement consommés. | Écartée seule pour RTC-06. |
| Garder l'original ouvert avec partage lecture seul pendant tout le parcours | 1 | Stabilise le fichier mais bloque son édition pendant le dialogue et la file ; chemin transmis aux workers et chemins indirects à qualifier. | Alternative de repli si le snapshot n'est pas qualifié. |
| Copier et hasher l'original dans le même flux vers un snapshot privé protégé ; rehasher l'original juste avant toute suppression | 1 pour Créer ; 2 pour Remplacer | Ajoute l'écriture temporaire de S octets, l'espace S par import en attente et les lectures du snapshot par les workers. Lie les octets transcrits et vérifie l'identité au moment destructif. | **Candidate retenue pour la revue**, sans promesse de gain de temps avant mesure. |

Le snapshot est créé par un flux unique de blocs : chaque bloc lu de l'original est écrit dans un fichier unique et alimente SHA-256. L'original est ouvert en lecture avec partage Windows en lecture seulement pendant la copie ; une ouverture incompatible échoue. Le snapshot est synchronisé, scellé, puis transmis comme chemin d'exécution au worker ; un handle non partageable en écriture/suppression reste détenu jusqu'à la fin de sa consommation. Le probe de durée lit le snapshot. Le SHA et la durée confirmés sont réutilisés à l'admission, la confirmation et la préparation. Les vérifications SHA internes des workers restent en place et sont rapportées séparément. Aucun nouveau FFI CUDA, code C++ ou changement de modèle n'est impliqué par ce mécanisme.

Le chemin original sert à l'affichage, à l'identité persistée et au contrôle final. Pour Remplacer, après publication complète, l'application ouvre l'original avec partage lecture seul, calcule son SHA et exige l'égalité avec le snapshot confirmé, l'intention durable et le manifeste du nouveau traitement. Elle maintient ce handle pendant la passe de nettoyage. Une différence, une absence ou un chemin non vérifiable conserve le nouveau et interdit **toute** suppression ancienne. Après panne, aucun handle n'est supposé survivre : reprendre une transcription exige de resceller la source et d'en vérifier l'identité ; reprendre seulement le nettoyage exige confirmation explicite et SHA final renouvelé.

Les chemins locaux réguliers vérifiables constituent le cas nominal. Un lien/reparse, un chemin dont la canonicalisation ou la provenance change, ou un stockage offrant des garanties d'ouverture incertaines ne peut autoriser une suppression. Le design ne promet pas de résistance à un acteur hostile ayant accès en écriture aux fichiers de contrôle privés de l'application. Les primitives de partage sont celles de [`OpenOptionsExt::share_mode`](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.share_mode) et de [`CreateFileW`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew) ; leur intégration avec les workers sera vérifiée par les futurs tests, pas déclarée qualifiée ici.

## 6. Catalogue, publication et nettoyage

**Catalogue.** Une entrée porte destination canonique, `job_id`, génération, SHA source, succès prouvé, statut actif/remplacé, présence, ordre connu/inconnu et preuves de propriété. Le scan des historiques ne demande pas leurs anciens fichiers audio. Pour Créer un nouveau, une corruption d'ancienne génération n'empêche ni réservation ni publication du nouvel identifiant. Pour Remplacer, seules les entrées vérifiées et présentes peuvent être ciblées ; les autres sont signalées séparément. « Tous » est résolu une fois en un ensemble figé comprenant les R11 inactifs encore présents. Un nouveau traitement apparu ensuite n'entre pas implicitement dans cet ensemble.

**Dernier succès.** `PublicationIndex` ne stocke pas d'ordinal (`archive.rs:828-840`). Le code ajoute un succès à la fin de la liste (`archive.rs:3728-3750`), mais la migration sans index scanne les fichiers (`archive.rs:3082-3107`) et la liste publique trie par identifiant. Un index versionné doit attribuer sous verrou un ordinal monotone au moment du succès ; un rejeu conserve le même ordinal. Les historiques R11 indexés ne peuvent recevoir un ordre que si la provenance de leur séquence est démontrée. Pour plusieurs historiques à ordre inconnu, « Dernier » est indisponible avec une explication et un choix explicite ; ni date de dossier, ni `job_id`, ni tri lexical ne sont substitués. Une nouvelle publication ordonnée est plus récente que les historiques présents lors de la migration.

**Point de visibilité.** Les deux fichiers internes et le manifeste sont vérifiés avant le marqueur final de publication. Le succès n'est annoncé qu'après ce marqueur et l'index. Les étapes ne forment pas une transaction multifichier : la récupération distingue préparé, publié et nettoyage incomplet. L'intention durable associe opération, successeur, SHA, mode, **identités exactes** des cibles, preuves attendues des dossiers/exports et progression par élément. Elle est conservée avant publication du successeur ; la récupération ne crée pas de seconde publication. Les nouveaux exports `exports/` et les temporaires `export.*.pending` de l'ancien format ne sont pas créés pour ce parcours.

**Propriété.** Un ancien dossier sélectionné doit rester une génération publiée Whisper du SHA attendu, sous la destination courante, avec chemin canonique et artefacts connus. Un dossier Parcourir absent de la liste suit les mêmes preuves. `installed-exports.json` et `InstalledExportOwnership` (`archive.rs:791-809`) apportent les liens de provenance disponibles pour les anciens exports ; les exports R11 déplacés dans `transcriptions-remplacees/` sont inclus seulement si leur propriété demeure prouvable. Égalité des octets TXT/SRT, nom ressemblant ou simple présence dans `exports/` ne prouvent pas la propriété. Une provenance historique insuffisante mène à un élément conservé et nommé comme restant.

**Suppression.** Inventorier les artefacts attendus et contrôler qu'aucun élément tiers ou chemin redirigé ne sera suivi. Supprimer seulement les éléments prouvés de chaque cible et éventuellement son dossier devenu vide ; pas de `remove_dir_all` fondé sur le nom ou un manifeste partiel. Ne pas retirer la seule preuve d'un export encore à examiner : retenir cette preuve dans l'intention avant toute suppression. Chaque élément reçoit un état durable : à traiter, retiré, déjà absent et attribuable, conservé faute de preuve, ou échec. Une erreur n'efface pas le nouveau succès. Au redémarrage, l'application affiche les éléments restants et demande si l'utilisateur veut reprendre leur nettoyage ; elle revalide source, successeur, cible et propriété sous la sélection initiale figée.

## 7. Frontières techniques proposées

| Responsabilité | Module candidat | Contrat |
| --- | --- | --- |
| Identités, choix, états et erreurs métier | `whisper-core` | Aucune lecture de disque ni choix de chemin par l'UI. |
| Commandes, événements et orchestration | `whisper-core/src/application.rs` et ports existants | Confirmation, progression, réponse obsolète et reprise explicite. |
| Copie/hash, handle, probe et nettoyage du snapshot | `whisper-adapters`, module privé candidat | SourceBinding vérifié ; aucun SHA de l'original sur le fil UI. |
| Catalogue, intention durable, publication et suppression | `whisper-adapters/src/archive.rs`, extraction ciblée possible | Autorisation destructive par identités et propriété, sous verrou court. |
| Routage vers worker et conservation du binding pendant l'appel | `whisper-adapters/src/worker_ipc.rs` | Les workers existants reçoivent le chemin du snapshot ; leurs contrôles restent. |
| Dialogue, cible et messages | `whisper-desktop/src/ui.rs` | Affichage et choix seulement ; aucun accès direct à supprimer. |
| Assemblage des composants | `whisper-desktop/src/root.rs` si nécessaire | Ne change aucune règle métier. |

Le futur plan fixera les chemins exacts et des lots d'au plus dix tâches. Aucun de ces chemins n'est ouvert au code par le présent design. Une nouvelle technologie, un `unsafe`, une dépendance ou un worker modifié demanderait un amendement du plan et l'accord correspondant.

## 8. Mesure requise et risques

La mesure applicative actuelle demandée par le mandat reste **NOT RUN**. Une campagne bornée à proposer à l'utilisateur avant exécution utiliserait une racine isolée, par exemple `target/parakeet-tests/retraitement-cible/mesure-avant/`, le Cargo partagé et la garde existante ; aucune destination de transcription réelle de l'utilisateur. Elle fixerait HEAD, profil, moteur, tailles/durées et contexte de cache observé, puis comparerait un WAV court, un WAV proche de 30 min et un MP3 disponible, pour sans doublon, Créer après doublon et Remplacer après doublon. Deux passages consécutifs sont utiles ; ils ne sont pas appelés « froid/chaud » sans maîtrise du cache. Les traces `archive_sha256` et `archive_audio_probe` sont exploitées par PID/opération. Les événements actuels ne ventilent pas nécessairement tous les scans et phases ; toute instrumentation supplémentaire doit être autorisée dans un périmètre de code précis. Les données R9 restent historiques. Le temps humain du dialogue, le harness, les lectures du worker, la copie du snapshot et le temps produit doivent être séparés. Aucun ancien traitement réel n'est supprimé dans cette campagne de référence.

| Risque | Effet et contrôle | Preuve attendue |
| --- | --- | --- |
| RTC-R1 Source modifiée ou remplacée | Snapshot lié au résultat, SHA original sous handle avant suppression ; nouveau gardé si doute. | Mutations à admission, pendant décodage et avant suppression, même taille/date restaurées. |
| RTC-R2 Cible ou export tiers | Ensemble figé, appartenance canonique, provenance et identité de chaque élément ; conservé si doute. | Substitution de dossier, export homonyme, reparse, fichier ajouté. |
| RTC-R3 Ordre historique incertain | Aucun « dernier » arbitraire. | Index ancien/migré, égalités de dates, nouvelle publication. |
| RTC-R4 Panne multifichier | Intention durable, marqueur de succès, progression idempotente et reprise explicite. | Arrêt à chaque frontière, sans nouvelle cible ni seconde publication. |
| RTC-R5 Espace/temps de copie | Snapshot de taille S par import en attente ; défaut d'espace attribué, anciens conservés. | Mesurer taille et coût de copie sur WAV/MP3 ; limiter le nombre de snapshots simultanés dans le futur plan. |
| RTC-R6 Régression moteur | Chemin snapshot réellement lu par Turbo/Parakeet CPU/GPU ; contrôles worker conservés. | Contrats d'import, file et reprise ; recettes natives explicitement rapportées. |

## 9. État de sortie et prochaine décision

La stratégie est un **candidat de design**. La réduction *statique* proposée porte sur les lectures complètes de l'**original par l'adaptateur** : six au parcours de doublon actuel contre une pour Créer et deux pour Remplacer, sous réserve de l'intégration future. Elle ne promet pas encore de réduction du temps total : la copie et les lectures du snapshot peuvent compenser ce gain. Les preuves de coût du R11 actuel et les tests de mutation, d'interruption et de recette sont NOT RUN. La revue indépendante doit vérifier les exigences RTC-01 à RTC-11, les risques RTC-R1 à RTC-R6 et la cohérence de cette limite. Après revue, demander un accord précis pour la campagne de référence ; le plan exécutable ne sera rédigé qu'après acceptation du design et résolution ou arbitrage de cette mesure.
