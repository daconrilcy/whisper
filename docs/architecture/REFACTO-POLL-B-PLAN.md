Avancée : familles 5 et 6 caractérisées et extraites CLEAN ; quatre mutations obligatoires détectées ; N1 734/148.
Blocage : aucun pour préparer le complément N2 comparatif autorisé ; tests/profils et ressources restent à mettre en place.
Décision : réaliser uniquement le complément N2 approuvé, puis figer B et arrêter ; C et push non autorisés.

# REFACTO-POLL — sous-lot B

## Décision utilisateur et prérequis

Le 3 novembre 2026, l'utilisateur autorise B : familles 5 et 6, chacune
caractérisée dans Na puis extraite dans Nb. Référence source après clôture A :
`20dd1c1f88ee0d64899b17b9b590b8c64535103e`, branche `refacto/poll`, worktree
`C:/wt/poll`. Les bilans A restent figés.

Avant le premier commit B, exécuter **100 fois de suite**, au niveau 1 par
la garde, le test corrigé :
`worker_ipc::tests::microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end`.
Consigner le nombre effectivement exécuté et les échecs ; **au premier échec,
arrêt avant B pour diagnostic**, sans relance silencieuse.

La garde actuelle n'accepte aucun filtre de test ni `--exact`. L'utilisateur
choisit explicitement le 3 novembre **100 exécutions du paquet adapters
complet avec la garde inchangée**. Aucune extension de garde ni commande de
test directe n'est prévue. Chaque passage doit contenir exactement une
réussite du test micro ciblé ; conserver aussi les résultats du paquet et
arrêter la campagne sur tout échec conformément à la garde permanente.

### Arrêt du prérequis le 3 novembre

SelfTest de la garde : 33 cas positifs et 64 négatifs réussis. Passages 1 à 7 :
code 0, test micro exact réussi une fois par passage. Passage 8 : code Cargo
101, **318 réussites, un échec, 48 ignorés** ; seul le test micro échoue.
Nombre de répétitions du test micro : **8 exécutées, 7 réussies, 1 en échec**.
Pas de passage 9, pas de relance et aucun commit B.

Erreur observée : `Option::unwrap()` sur None à
`worker_ipc/tests/microphone_and_finalization.rs:341`. La boucle d'attente
des trois télémétries ignore `Some(Err(_))` de poll ; le direct est absent
avant l'enqueue suivant. Les chemins de retrait possibles comprennent une
erreur de transport et une finalisation Stopped ; les données disponibles
ne permettent pas d'attribuer la cause primaire. Ce n'est pas la précédente
assertion d'absence de compteurs. Le faux worker est normalement en attente
de Stop et le test ne l'a pas demandé volontairement à ce stade.

Preuves conservées sous `target/parakeet-tests/refacto-poll-b/` :
`repeat-selftest.log`, `repeat-001.log` à `repeat-008.log` et logs Cargo.
Échec : `scratch/20261103T091327439-e46450208334465b9c166f3be6f234dd/`.
Fixture conservée : `temp/tmp/legacy-spectrum-fairness-116968-1992/` ;
checkpoint PCM de 24 000 échantillons durables, sans attribution causale.
Sources et garde inchangées depuis 20dd1c1f ; seul ce plan est non suivi.
PE : **non concerné**. Aucun support staging, Na/Nb, N2 ou C commencé.

Proposition diagnostique distincte : modifier seulement l'attente du test
existant pour échouer immédiatement avec le détail de toute erreur de poll
et de toute disparition du direct après poll, en conservant le délai de
deux secondes et les trois assertions ; puis au plus trois passages adapters
par la garde, arrêt au premier échec pour lire son diagnostic. Cette
modification et cette reprise nécessitent l'arbitrage après arrêt ; elles
ne sont pas réalisées par le présent document.

### Diagnostic ciblé autorisé et exécuté

Autorisation reçue dans la session : ajouter ce diagnostic au test existant,
puis au plus trois passages adapters avec arrêt au premier échec. Seule la
boucle d'attente du test micro est modifiée : erreur poll rendue immédiatement
visible avec état live/active/transport ; après événement ou None, présence
du direct exigée. Délai de deux secondes, condition des trois indicateurs,
injections, sommeil et assertions finales conservés. Aucun changement produit.

Les trois passages `diagnostic-001.log` à `diagnostic-003.log` ont tous le code
0 et exactement une réussite du test micro, sans échec. L'échec du passage 8
n'est donc **pas reproduit**, sa cause primaire reste inconnue ; le diagnostic
ajouté n'est pas une correction de cette cause. La limite des trois essais
diagnostiques est atteinte, aucune quatrième exécution n'est lancée.
Le prérequis de cent réussites consécutives reste non satisfait et aucun
commit B n'est créé. PE non concerné. Revue indépendante du seul diagnostic :
**CLEAN**, un passage, aucune correction ni constat ouvert ; cela ne qualifie
pas la cause initiale.

L'utilisateur autorise ensuite explicitement une **nouvelle série de 100**
passages adapters, diagnostic ajouté, après avis favorable indépendant et
avec arrêt au premier échec. Elle est engagée après le verdict CLEAN ; logs
`retry-001.log` et suivants, distincts des essais précédents. Aucun commit
avant réussite complète de cette nouvelle série.

### Nouvel arrêt : saturation PCM au passage 13

Nouvelle série : **13 passages exécutés, 12 réussites micro et un échec**.
Pas de passage 14. `retry-013.log` et
`scratch/20261103T092832190-5e51c290859a4a9e816492fa9af1dc2d/`
conservent le résultat : 318 réussites, un échec, 48 ignorés dans le harnais
unitaire adapters, code Cargo 101. Le test micro échoue à la ligne 447 avec
`legacy worker event failed: (JobId(881), Generation(1), "CaptureSaturated: bounded PCM writer queue is full")`.
Cette fois l'erreur apparaît dans la boucle des événements worker, après
l'attente télémétrique, et non dans le diagnostic ajouté à celle-ci.

Le chemin capture traduit WouldBlock de `staging.try_append_frame` en ce
message et mémorise la failure avant de demander Stop. La file a une capacité
de 50 frames de 320 échantillons ; le test injecte des blocs de 960 échantillons
toutes les 5 ms dans ses deux boucles d'attente, soit 60 ms d'audio par bloc
à 16 kHz. Cela identifie une alimentation synthétique accélérée ; le journal
ne démontre pas à lui seul que ce rythme est l'unique cause de la saturation.
L'ancien unwrap sur None au passage 8 reste sans attribution causale certaine.

Sources produit inchangées, garde inchangée, diagnostic test non commité,
aucun commit B ni support staging, extraction, N2 ou C. Diagnostic de la
fixture poursuivi seulement en lecture seule ; aucune relance après l'échec.

Proposition minimale après diagnostic en lecture seule : dans ce seul test,
espacer les blocs de 960 échantillons d'au moins 60 ms à 16 kHz, en remplaçant
les trois sommeils actuels (10 ms au préchauffage, puis 5 ms dans chacune
des deux boucles) par 60 ms. Conserver blocs, injections entre événements,
quatre appels poll par rafraîchissement là où ils existent, délais de deux
et trois secondes, diagnostic et toutes les assertions. Aucun changement
des files ou délais du produit. Cette correction cible la pression synthétique
observée, sans prétendre résoudre l'ancien passage 8 ; elle reste soumise à
autorisation distincte, revue indépendante et nouvelle série complète de cent
passages avec arrêt au premier échec. Ne pas compter les douze précédents
dans cette future série sur un candidat différent.

Autorisation reçue dans la session pour ces trois pauses à 60 ms, la revue
indépendante et une nouvelle série de 100 passages adapters sous garde,
avec arrêt au premier échec. Aucun autre changement au test ou au produit
n'est inclus dans cette correction de cadence.

Correction de cadence appliquée aux trois sites, puis revue indépendante
**CLEAN**, aucun constat ouvert. Série distincte `paced-001.log` et suivants
engagée après cet avis, avec garde inchangée et arrêt au premier échec.

### Arrêt du paquet au passage 46, micro toujours vert

La série à cadence corrigée s'arrête au **passage 46** : les 46 exécutions
du micro sont réussies, zéro échec micro ; les passages 1 à 45 du paquet
entier sont verts. Au passage 46, le harnais d'intégration `live_archive`
échoue : `a_live_group_has_one_archive_writer_at_a_time` (ligne 93,
suppression de fixture, erreur Windows 145 DirectoryNotEmpty) et
`archive_characterization_attempt_journal_corruption_is_not_read_by_recovery`
(ligne 693, récupération attendue 1, obtenue 0). Résultat de ce harnais :
10 réussites, deux échecs ; code Cargo 101. Aucun passage 47.

Preuves : `paced-046.log` et
`scratch/20261103T093936933-79088f38fbeb4d5bbcee02d13f353f76/`.
Le test micro y figure explicitement réussi. La règle d'arrêt sur échec de
campagne s'applique aussi aux autres tests du paquet ; les 46 réussites
micro ne permettent donc pas d'annoncer le prérequis 100 satisfait.
Sources produit et garde inchangées ; aucune correction archive autorisée
ou réalisée. Diagnostic complémentaire seulement en lecture seule, pas
de relance ni commit B. La cause de ces deux échecs reste à établir.

Diagnostic statique : les deux tests utilisent `tests/live_archive.rs::test_root`,
dont le nom combine seulement PID et horloge système en nanosecondes, sans
compteur distinct par appel. Une collision entre threads du même harnais est
donc possible. La fixture restante du job 0x5402,
`temp/tmp/whisper-live-archive-107652-1793695182270404200`, n'a plus son
pending.json ni son verrou, mais conserve PCM, checkpoint et journal ; cela
concorde avec un nettoyage concurrent. Les logs ne donnent pas les chemins
par test : collision plausible, **non démontrée**. Ne pas attribuer ces
échecs au comportement produit sur cette seule base.

Proposition séparée : rendre ce seul helper de test unique par ajout d'un
compteur atomique par processus au suffixe existant, sans modifier assertions,
nettoyages ou sources produit ; revue indépendante puis nouvelle série entière
de cent paquets via garde inchangée, arrêt au premier échec. Ce fichier de test
existant est hors périmètre B autorisé ; aucune modification avant arbitrage.

Autorisation distincte reçue pour cette seule isolation de `test_root()`,
sa revue indépendante et une nouvelle série de cent paquets sous garde avec
arrêt au premier échec. L'ajout d'un compteur ne constitue pas une preuve
rétrospective de collision au passage 46.

Compteur ajouté, puis revue indépendante **CLEAN**, aucun constat ouvert.
Assertions et nettoyages inchangés. Nouvelle série `isolated-001.log` et
suivants engagée après ce verdict favorable ; garde inchangée.

### Arrêt sur fixture de préférences résiduelle

`isolated-001.log` : micro réussi, harnais live_archive 12 réussites ; arrêt
du paquet sur `matching_backup_is_reused_once_and_backup_io_failure_leaves_v1_untouched`,
`tests/model_preferences.rs:251`, création d'un dossier déjà existant,
erreur Windows 183. Harnais préférences : cinq réussites, un échec ; Cargo
101. Pas de second passage. Logs sous
`scratch/20261103T094337630-10d1f3bea65f4ec0a562bb65f0456305/`.

Son helper réutilise un chemin `nom-PID`, sans nettoyage. TEMP/TMP de la
garde sont communs aux passages d'une même racine de lot ; 67 anciens
dossiers portant ce préfixe sont conservés, avec le backup déjà créé. La
réutilisation d'un PID explique très probablement ce résidu, mais le log
ne donne pas le PID du processus en échec : aucune identité historique
n'est inventée. Aucun fichier model_preferences modifié.

Proposition de reprise sans autre correction source : nouveau LotRoot
distinct et initialement absent pour chaque passage sous
`target/parakeet-tests/refacto-poll-b/isolated-runs/run-NNN`. La garde
existante isole TEMP/TMP/LOCALAPPDATA sous chaque racine, en préservant la
cible Cargo partagée et le cache Cargo persistant. Aucun nettoyage des
anciennes preuves ou nouveau script. Reprise de cent passages entiers
soumise à arbitrage après échec, arrêt au premier échec. Les anciens
passages ne sont pas ajoutés à cette série.

L'utilisateur autorise cette reprise de cent passages avec dossiers distincts.
La campagne `distinct-NNN.log` vérifie avant chaque lancement que
`isolated-runs/run-NNN` n'existe pas, puis transmet cette racine à la garde
inchangée. Arrêt au premier échec ; aucune autre source modifiée.

### Arrêt de la série à racines distinctes au passage 15

`distinct-001.log` à `distinct-014.log` : paquet vert, micro réussi une fois.
`distinct-015.log` : micro réussi, mais
`queue_store::tests::historical_queue_json_without_strategy_reads_as_legacy_without_rewriting`
échoue à `queue_store.rs:797` sur une deuxième ligne JSON contenant un entier
4 au lieu d'un enregistrement. Harnais unitaire : 318 réussites, un échec,
48 ignorés, Cargo 101. **15 réussites micro, zéro échec micro ; 14 paquets
verts et un en échec.** Aucun passage 16.

Preuves : `isolated-runs/run-015/scratch/20261103T094824278-28dc3523b8d1499fb6fb02265a2c9f02/`.
La racine du passage était neuve. Le helper queue_store::tests::root combine
PID et SystemTime::as_nanos, sans compteur ; six tests l'utilisent en parallèle.
Le dossier queue du passage est vide après la campagne malgré l'échec avant
nettoyage du test concerné ; une collision avec nettoyage d'un autre test
est plausible, non prouvée faute de chemin enregistré par test.

Arrêt conservé, aucune modification de queue_store. Deux options bornées à
arbitrer : sélection strictement exacte du seul test micro dans la garde avec
auto-tests positifs/négatifs et revue indépendante, puis cent passages ciblés ;
ou correction du seul helper queue_store sous cfg(test), puis nouvelle série
de cent paquets à racines distinctes. Recommander le ciblage pour finir la
caractérisation demandée du micro sans accumuler des corrections hors poll.
La décision initiale de paquets complets reste applicable tant que l'utilisateur
n'autorise pas explicitement son remplacement. Aucun commit B, support staging,
extraction, N2 ou C commencé.

### Sélection exacte désormais autorisée

L'utilisateur remplace explicitement le choix des paquets complets par
l'autorisation de limiter la garde au test micro exact, avec auto-tests,
revue indépendante et cent passages ciblés. Chemin supplémentaire autorisé :
`ci/guard-level1.ps1`, parseur et auto-tests existants uniquement. Forme
admise : `test -p whisper-adapters` suivi du nom micro complet puis
`-- --exact`. Aucun autre filtre, paquet, feature, option d'exécution ou
nouveau mode ; garde et isolation des racines restent applicables. Les
passages ciblés auront des racines distinctes initialement absentes.
Aucune correction du helper queue_store n'est autorisée ou appliquée.

Revue du candidat garde : constat R-GUARD-1, un cas négatif dupliquait la
commande positive et aurait fait échouer SelfTest. Détecté en lecture seule,
avant toute exécution ; corrigé par un nom de test volontairement erroné.
Analyse syntaxique PowerShell et diff-check réussis ; seconde revue par
le même reviewer demandée. Aucun auto-test ni Cargo lancé sur ce candidat
avant cette fermeture ; aucune référence ou protection de racine changée.

SelfTest complet exécuté : code 0, **35 positifs et 79 négatifs** réussis,
preuve `exact-selftest.log`. Vérification V-GUARD-1 fournie au même reviewer,
qui rend **CLEAN** ; R-GUARD-1 clos, aucun constat ouvert. Série de cent
passages ciblés engagée ensuite : `exact-NNN.log`, chaque LotRoot
`exact-runs/run-NNN` initialement absent ; contrôle d'une et une seule
réussite du micro dans la sortie réelle de chaque Cargo. Arrêt au premier échec.

Résultat final : **100 passages consécutifs, 100 réussites micro, zéro échec**,
codes de sortie de la garde et de Cargo à zéro. Journaux `exact-001.log` à
`exact-100.log`, chacun relié à son stdout/stderr Cargo sous `exact-runs/`.
Les anciens passages interrompus ne sont pas additionnés à cette série.
Le prérequis avant premier commit B est satisfait. Les causes historiques
non prouvées restent décrites ci-dessus ; aucune réussite rétrospective
n'est attribuée aux campagnes interrompues.

Contrôle factuel indépendant des cent journaux : 100 codes de garde 0,
100 racines distinctes et exactement une réussite micro par stdout ; aucune
exécution filtrée à zéro n'est comptée. Suite N1 avant premier commit de
préparation : Structure, fmt, Clippy, défaut, common, CPU et Parakeet CPU
réussis. Journaux `preparation-*.log`, racines fraîches `preparation-checks/`.
Contrôles PE **non concernés** : aucun worker, manifeste, lock, `.cargo/`
ni script de compilation modifié ; le parseur de garde est un contrôle
d'autorisation des commandes, sans changement du mécanisme de compilation.

## État d'exécution 5a

Préparation commitée dans `6316a54b9681716b1d643dc8f2d60e8038d6b256` après
cent passages micro réussis et N1 complète : défaut 662/127, common 11/1,
CPU 12/9, Parakeet CPU 26/11, soit **711 réussites, 148 ignorés**.

Neuf nouveaux tests dans `poll/tests/stop_recovery.rs`, déclaration de module
et neuf entrées de registre. SelfTest 35/79, Structure **916 tests/916 entrées**
et fmt --check réussis. Premier appel adapters sous garde : compilation
arrêtée sur E0594 à la ligne 100 du nouveau fichier, assignation à
`request.config.compute` alors que la déclaration ligne 99 n'est pas mutable.
Un seul diagnostic de compilation ; aucun test candidat exécuté, aucune
mutation ni extraction. Preuve :
`5a-adapters/scratch/20261103T100940554-985f586cc6734f8b977f8840d8dbb9cd/`
sous la racine B, stdout et stderr conservés. Aucune relance ni correction
effectuée après cet échec.

Correction proposée : ajouter uniquement `mut` à cette variable de fixture,
puis reprendre les vérifications 5a complètes, les deux mutations prévues
et la revue indépendante. Aucun code de production concerné ; PE non concerné.

Autorisation précise reçue dans la session : ajouter `mut` à cette seule
variable, puis reprendre toutes les vérifications 5a, mutations et revue.
Aucune autre correction ou extension de périmètre n'est déduite de cet accord.

Reprise 5a : dix tests après ajout du cas AwaitingChoice annulé destiné à M1,
SelfTest 35/79, Structure 917/917 et fmt réussis. Compilation réussie, puis
`cargo test -p whisper-adapters` arrêté : 327 réussites, deux échecs, 48 ignorés.
Les deux échecs sont dans les tests nouveaux, pas dans le code produit.
Preuves : `5a-retry-adapters/scratch/20261103T101148223-082c2ffe849646d0b63de2e849b1fae7/`.
Pas de relance, modification ou mutation après cet arrêt.

Revue indépendante initiale : **FINDINGS**, deux constats bloquant la
caractérisation avant extraction. R-5A-1 : la recherche globale Started
rejette à tort le GPU initial de tentative 1 ; lire les records JSONL et
caractériser la réservation CPU de tentative 2, sans Started CPU.
R-5A-2 : l'attente d'erreur de reprise import reçoit LiveAttemptStarting ;
elle doit distinguer les événements de l'ancienne tentative et observer
la vraie transition Stopped puis l'échec de démarrage CPU, avec les états
restants. Une course Start/Stop dans le faux worker est possible, mais non
démontrée comme cause précise. Aucun défaut de production établi.

Correction proposée seulement dans les nouveaux tests : oracle JSONL par
tentative et synchronisation de la fixture/attente d'événements corrélés
pour la reprise import, sans ignorer d'erreur ni rendre le test vert par
retour anticipé. Reprise N1 complète, deux mutations et seconde revue par
le même reviewer après arbitrage ; aucun changement de production autorisé.

L'utilisateur autorise précisément les corrections R-5A-1/R-5A-2 dans les
tests nouveaux, puis N1, deux mutations et seconde revue. Le coordinateur
fait corriger par le même implémenteur ; aucun nouveau responsable d'écriture.

Corrections appliquées dans le seul fichier de tests nouveau : journal JSONL
par tentative ; faux worker et attente du Stop corrélés. Reprise sous garde :
SelfTest 35/79, Structure 917/917, fmt et paquet adapters réussis (harnais
unitaire : 329 réussites, zéro échec, 48 ignorés). Racines `5a-corrected-*`.
La suite N1 complète et les deux mutants restent à terminer avant fermeture.

N1 avant mutations sur candidat final : défaut 672/127, common 11/1,
CPU 12/9, Parakeet CPU 26/11, soit **721 réussites et 148 ignorés** ;
SelfTest 35/79, Structure 917/917, fmt et Clippy workspace/all-targets
réussis (38 avertissements adapters existants). Preuves `5a-final2-*`.

Deux mutations locales non commitées détectées, puis restaurées :

- **M1**, retrait du bloc PublicationGate pending-live de poll.rs:504–507 :
  `poll_stopped_manual_gpu_choice_is_cancelled_by_publication_gate` échoue
  sur l'événement ; `poll_stopped_auto_retry_cancels_pending_live_without_starting_cpu`
  échoue sur Cancelled au lieu de Stopped. Preuve `5a-m1/scratch/20261103T102806324-879dd9f78bb44309b20c006d706efe0a/`.
- **M2**, neutralisation du contrôle import dans poll et du contrôle redondant
  de worker_start.rs:43 en reprise CPU forcée :
  `poll_stopped_import_auto_retry_drops_request_when_gate_is_cancelled`
  observe **cpu_started=true et cpu_worker_alive=true** avant l'échec de son
  assertion Stopped. Le test existant `stop_latched_after_gpu_failure_prevents_cpu_worker_launch`
  détecte aussi ce mutant, sans modification du test existant. Preuve
  `5a-m2/scratch/20261103T102853240-1f882db9683547e887074fe24e742177/`.

Échecs attendus dans la campagne de mutation, expressément exigée par le
plan ; aucun échec étranger aux effets forcés signalé. Les blocs originaux
sont restaurés exactement ; reprise N1 après restauration avant revue finale.
Les mutants ne seront inclus dans aucun commit.

Après restauration : suite `5a-post-*` entièrement réussie, SelfTest 35/79,
Structure 917/917, fmt, adapters, Clippy et N1 défaut/common/CPU/Parakeet CPU.
Comptes consolidés identiques : **721 réussites, 148 ignorés**. Diff de
production vide, diff-check réussi. Seconde revue : R-5A-1/R-5A-2 et les
deux mutations confirmés, sans nouveau constat ; verdict final attendu sur
les preuves de validation complète. Verdict final : **CLEAN**, deux revues,
une correction, R-5A-1/R-5A-2 clos et aucune requête ouverte. Les cent passages
préparatoires précèdent 6316a54b ; ils ne sont pas présentés comme cent
répétitions du candidat 5a. L'autorisation B initiale couvre la suite 5b,
après commit Na, avec nouvelle validation et revue indépendante.

## État d'exécution 5b

5a commité dans `cfce1612eab3cd48c77450f1a86a382c531119c3`. Extraction
limitée à poll.rs et au nouveau `poll/stop_recovery.rs`, deux handlers
séparés et leurs contrôles ; aucune modification des tests. SelfTest 35/79,
Structure 917/917 et fmt réussis. Un appel Clippy avec options après `--`
a été refusé par le parseur avant Cargo. L'implémenteur a ensuite ajouté
les annotations warn de seuils aux handlers et poursuivi avec Clippy
ordinaire avant arbitrage : **écart à la règle d'arrêt**, consigné ici.
Cette poursuite ne crée aucune dérogation pour les appels suivants.

Clippy ordinaire a arrêté la compilation : **deux E0624**, méthodes
`poll_stopped_pending_live_recovery` et `poll_stopped_import_auto_retry`
privées dans le sous-module alors qu'elles sont appelées par poll.rs.
Aucun test 5b exécuté, aucune relance ni édition après cet échec.
Preuve : `5b-clippy/scratch/20261103T103530453-30c5ba8be4d340fe8432e37c217b1328/`.
Revue indépendante initiale en cours, lecture seule sur ce candidat figé.

Correction proposée : rendre uniquement ces deux méthodes `pub(super)`,
visibles depuis leur module parent, sans modifier leurs corps ; reprendre
les vérifications 5b complètes et la revue. Aucun élargissement d'API publique
livrée, aucune extension du parseur pour des options Clippy.

Autorisation précise reçue pour `pub(super)` sur les deux méthodes, sans
changer leurs corps, puis N1 et revue indépendante. Aucun autre changement
de code n'est inclus dans cet accord.

Correction des deux visibilités appliquée sans changer les corps. Reprise
`5b-retry-*` entièrement verte : SelfTest 35/79, Structure 917/917, fmt,
Clippy workspace/all-targets, adapters et N1. Comptes consolidés : défaut
672/127, common 11/1, CPU 12/9, Parakeet CPU 26/11, soit **721 réussites,
148 ignorés**. Diff-check réussi. PE non concerné.

Extraction : **69 lignes de production retirées de poll**, sous la limite
800 ; module nouveau de 93 lignes physiques. Handlers séparés de 48 et
37 lignes physiques, annotations Clippy actives, aucun dépassement de
100 lignes ou de complexité 25. `poll()` : **639 lignes Clippy et complexité
66** ; mesure transitoire, reliquat des familles 6 à 8, seuil final à atteindre
au terme du découpage accepté. Aucune référence Clippy ajoutée ou modifiée.
Seconde revue indépendante de 5b demandée avec ces preuves.

Verdict final 5b : **CLEAN**, deux revues et une correction, R-5B-1 clos,
V-5B-1 satisfait, aucune requête ouverte. Ce verdict ne clôt pas B.

## Support de preuve d'ordre F6, séparé de 6a

5b commité dans `3bbb95fd7d6132d93c2d04c3ce332df04e27a161`.
Support exclusivement cfg(test) autorisé dans la session : déclaration de
module dans staging.rs et nouveau staging/test_support.rs. L'enveloppe du
JoinHandle attend le vrai writer, puis appelle l'observateur. Aucun champ,
Drop, writer_loop, nouveau trait ou comportement normalement compilé modifié.
L'oracle 6a stockera le résultat du probe dans un état externe vérifié par
le test ; une panique dans l'observateur seule ne prouverait pas l'échec,
car le Drop existant ignore le résultat de join.

Candidat support gelé avant N1 `6-support-*` et revue indépendante ; aucun
test 6a ou changement de logique Stop F6 engagé dans ce commit de support.

N1 support initiale verte (721/148, Structure 917/917, SelfTest 35/79,
fmt et Clippy), avec un avertissement dead_code non masqué en attente
d'utilisation 6a. Revue initiale : R-6SUP-1, la fonction libre du module
privé serait inaccessible depuis les tests sibling de poll. Correction
dans le périmètre autorisé : méthode `pub(crate)` de PcmWriter, définie
dans ce même support cfg(test), module restant privé. Aucun champ/Drop/
writer_loop modifié ; nouvelle validation avant commit et seconde revue.

Correction appliquée : méthode `PcmWriter::observe_shutdown` pub(crate),
module privé inchangé. N1 corrigée exécutée par la session principale sous
`6-support-method-*`, wrappers `.log` à la racine B : tous codes 0,
SelfTest 35/79, Structure 917/917, fmt, Clippy et tests défaut/common/CPU/
Parakeet CPU. Total **721 réussites, 148 ignorés** ; avertissement temporaire
de helper inutilisé visible, non masqué. Diff-check réussi.

La fermeture indépendante de R-6SUP-1/V-6SUP-1 a été temporairement
interrompue par une limite d'utilisation du coordinateur. Le même coordinateur
a repris avec le même reviewer, sans changement de modèle ni responsable.
Verdict final indépendant : **CLEAN**, deux revues, une correction,
R-6SUP-1 clos et V-6SUP-1 satisfait. Aucune nouvelle requête ouverte.
Les commits 5a/5b déjà CLEAN restent inchangés. Le support sera commité
séparément avant toute caractérisation 6a.

Support commité dans `692ce032d59530538fa90acb387502971e29df04`.

## Atteignabilité F6 et arbitrage avant 6a

Audit lecture seule des affectations de recovery/barrier, du transport et
de la confirmation. Dans worker_start.rs, les passages live vers
AutoRetryPending/AwaitingChoicePending affectent simultanément
`barrier_m = Some(inference_sequence.saturating_sub(1))` et ferment
l'admission. La séquence d'inférence ne décroît pas ; le démarrage CPU
remet ensemble recovery à None et barrier_m à None. Les erreurs de transport
live passent par ce même traitement de panne. Une barrière absente ou
supérieure au watermark n'est donc pas produite par ces parcours normaux.

La confirmation transmet à persist_window_finished le compteur durable
réel, et l'archive refuse un fragment au-delà de ce compteur avant de
modifier le préfixe confirmé. Le compteur durable ne décroît pas :
`confirmed <= durable` reste invariant. Les branches négatives de barrière
et couverture sont défensives ; aucun état incohérent n'est créé par les
parcours normaux examinés. Les erreurs des gates de campagne sont également
inaccessibles en N1 standard, où ces gates sont des no-op.

Une mutation sur un préfixe valide qui saute toute la reprise et publie
serait détectable, mais ne prouverait pas spécifiquement le rejet d'une
barrière invalide. Supprimer seulement le contrôle de barrière entraîne
une reprise CPU, pas la publication exigée. Ne pas présenter ces variantes
comme preuve suffisante du critère demandé.

Proposition d'exception limitée, soumise à l'utilisateur : dans les seuls
nouveaux tests sous poll/, construire normalement le préfixe durable et
confirmé, puis altérer sous cfg(test) l'état privé de fixture pour une
barrière absente/hors borne ou une couverture supérieure au durable réel.
Aucun champ, helper, hook ou branche de production ajouté. Chaque test
défensif annonce cette corruption contrôlée et attend StorageCorrupt sans
publication. Pour la mutation obligatoire, faire passer le cas à barrière
invalide vers la publication existante tout en gardant le préfixe publiable ;
le test doit constater son échec, avec Published et exports effectivement
créés sous mutant, puis restauration exacte des sources. Les autres branches
inatteignables restent documentées sans fixture artificielle.

Alternative : maintenir strictement l'interdiction des fixtures artificielles
et adapter explicitement le critère de mutation de barrière ; ne pas lui
attribuer une preuve invalide. Aucun test 6a écrit ni campagne lancée pendant
cette vérification d'atteignabilité. Worktree propre après le commit support.

**Arbitrage reçu :** l'utilisateur autorise ces seules fixtures défensives
et la suite 6a/6b. Exception limitée aux nouveaux tests sous poll/ : état
privé de fixture volontairement incohérent pour barrière absente/hors borne
ou couverture au-delà du durable, sans code de production ajouté. Les tests
attendent StorageCorrupt et l'absence de publication ; leur nature défensive
est annoncée explicitement, distincte des parcours normaux. Les autres
branches inatteignables restent documentées sans test artificiel. L'exigence
de mutation produisant une publication malgré barrière invalide est maintenue.

## Oracle d'ordre M1 confirmé avant écriture 6a

Le premier probe envisagé live/transport n'est pas utilisable : la mise en
attente des Stopped corrélés attend déjà `transport.try_wait() != None`.
Le worker est donc sorti avant le nettoyage. Aucun observer transport ou
processus auxiliaire n'est ajouté pour contourner ce contrat.

L'alternative live/live_archive reste dans la mutation autorisée. Dans
LiveArchive::prepare, le verrou de groupe est testé avant la création des
fichiers pending ; le test existant de writer unique couvre déjà le conflit
dans un même processus. Pendant le callback PCM, une préparation du même
groupe renvoie `Recoverable: another live archive writer owns the passage`
tant que live_archive existe. Après inversion des retraits, le verrou est
libéré ; la préparation échoue plus loin sur create_new existant, avec une
autre erreur. Le callback conserve le booléen de conflit, et le test l'exige
après poll. Cette preuve porte sur les durées de vie réelles, sans trace
textuelle des affectations ni nouveau point de production.

## État d'exécution 6a

Treize nouveaux tests dans `poll/tests/active_live_stop.rs`, déclaration de
module et treize entrées nouvelles du registre. Scope de production intact.
SelfTest 35/79 et Structure **930 tests/930 entrées** réussis. Arrêt sur
fmt --check : la seule différence demandée est le déplacement de
`mod active_live_stop;` avant `mod manual_gpu_choice;` dans poll/tests.rs.
Aucun format automatique appliqué, aucune édition ou relance après ce
résultat ; aucun Clippy ou test candidat exécuté, aucune mutation.
Preuve : `6a-fmt/scratch/20261103T110731562-16cdf2f322f24f5cb0fcf4b7edada0b9/`.
Revue initiale en lecture seule sur le candidat gelé. Correction proposée :
ce seul déplacement, puis reprise des vérifications, mutations et revue.

Autorisation précise reçue pour le déplacement de déclaration uniquement,
puis la reprise 6a. Déplacement appliqué, candidat gelé de nouveau pour
les contrôles sous racines fraîches `6a-retry-*` et la revue en lecture seule.

Baseline 6a verte : SelfTest 35/79, Structure 930/930, fmt, Clippy workspace/
all-targets, adapters (342 réussites unitaires, zéro échec, 48 ignorés) et
suite N1 complète. Clippy signale 39 avertissements adapters, dont un import
LiveRequest inutilisé dans le nouveau fichier ; aucun allow ni référence
ajouté pour le masquer. Revue initiale : aucun défaut de code, seulement
les prérequis de preuves M1/M2 et N1 finale.

Mutations F6 exécutées séparément sous garde, puis restaurées exactement :

- **M1**, inversion live_archive/live dans le nettoyage de failure mémorisée :
  seul `poll_stopped_live_failure_keeps_archive_lock_until_writer_shutdown`
  échoue, sur le booléen externe indiquant que le verrou n'est plus détenu.
  Harnais : 341 réussites, un échec, 48 ignorés. Preuve
  `6a-m1-live-archive/scratch/20261103T111145504-75c404ca2c254725b03ac3b8f2dd4273/`.
- **M2**, laisser le seul watermark présent mais hors borne atteindre la
  publication : seul `poll_stopped_live_defensive_barrier_past_sequence_rejects_publishable_prefix`
  échoue après **Ok(Published), artifacts_published=true**. Harnais :
  341 réussites, un échec, 48 ignorés. Preuve
  `6a-m2-past-barrier/scratch/20261103T111322646-3d88dde2bf9f4d0da16634b1bd148d18/`.
  Une première variante plus large détectait également l'absence de barrière ;
  elle a été restaurée puis affinée pour isoler le cas hors borne.

Ces échecs sont les effets exigés par les mutations, aucun échec étranger
signalé. Production restaurée à HEAD, diff-check réussi. N1 après restauration
en cours avant fermeture indépendante et commit Na6 ; aucune extraction.

N1 après restauration terminée : SelfTest 35/79, Structure 930/930, format,
workspace Clippy avant retrait du seul import inutilisé ; après ce retrait,
format et Clippy ciblé adapters/all-targets de nouveau verts, aucun warning
nouveau dans le fichier 6a. Suite finale `6a-final-clean-*` : défaut 685/127,
common 11/1, CPU 12/9, Parakeet CPU 26/11, soit **734 réussites et 148 ignorés**.
Les treize tests sont inclus et verts dans le défaut. Production identique
à HEAD, diff-check réussi. Une répétition adapters-confirm a seulement servi
à obtenir explicitement le code de sortie après une session initiale dont
le retour n'était pas affiché ; aucun échec de la première n'était signalé.
Preuves finales transmises au même reviewer, sans nouveau cycle de code.

Verdict final 6a : **CLEAN**, deux revues et une correction ; V-6A-1,
V-6A-2 et V-6A-N1 satisfaits, aucun constat ou demande ouvert. L'avis
ne clôt pas B et n'est pas présenté comme validation N2.

## État d'exécution 6b

6a commité dans `067ba74963f22e205b1c32a92db7f215489f5592`.
Extraction limitée à poll.rs et au nouveau `poll/active_live_stop.rs` :
152 lignes retirées de poll, quatre fonctions séparées, tests et registre
inchangés. Arrêt initial respecté après fmt --check sur la présentation
du nouveau module, SelfTest 35/79 et Structure 930/930 déjà réussis.
La session principale a arbitré le formatage de ce seul nouveau fichier,
opération d'édition déjà comprise dans le mandat 6b, à périmètre constant.
Aucun échec de compilation ou de test ni nouvelle permission externe n'est
en cause. Rustfmt ciblé exécuté, diff-check réussi ; mesure courante du
module final de **190 lignes physiques**, remplaçant le comptage provisoire 183.

Candidat gelé pour N1 sous racines fraîches `6b-retry-*` et revue
indépendante en lecture seule. Les mesures des quatre fonctions et du
reliquat poll restent à obtenir ; aucune extraction de C n'est engagée.

N1 finale `6b-retry-*` entièrement verte : SelfTest 35/79, Structure 930/930,
fmt, Clippy adapters/all-targets, défaut 685/127, common 11/1, CPU 12/9 et
Parakeet CPU 26/11, soit **734 réussites, zéro échec et 148 ignorés**.
Les treize tests 6a restent verts. Mesures : quatre handlers de **24, 59,
68 et 28 lignes**, aucun diagnostic de complexité au seuil 25 avec warn
actif. `poll()` : **489 lignes Clippy, complexité 52**. 152 lignes déplacées,
sous la limite 800 ; aucun allow ni référence Clippy ajouté. PE non concerné.

Verdict indépendant final 6b : **CLEAN**, deux revues, une correction de
présentation ; aucun constat ouvert, V-6B-N1 satisfait. Les handlers
préservent les contrôles, priorités, nettoyages et retours immédiats. Le
reliquat de poll reste pour C ; il n'est pas présenté comme conforme aux
seuils finaux. N2 non exécutée, B pas encore clos.

## Proposition N2 précise, non exécutée et soumise à autorisation

Production B livrée dans `7e30d667c92138516ce721cc5c559869b03b697c`.
Le critère original exige des parcours traversant ImportRuntime::poll.
Le nom d'un profil worker ou d'un test DirectLive seul ne suffit pas.

| Parcours | Cas retenu / profil | Référence comparable |
| --- | --- | --- |
| Import CPU long | PersonalLongCpu : `native_personal_cpu_imports_annotated_french_long_audio`, puis version english ; Parakeet CPU réel, application/AsyncImportIo/poll. | `1d30690e` : corps FR 175,79 s, EN 195,44 s ; publications validées, sans WER. |
| Reprise après interruption | Nouveau profil PollInterruptedResume pour `worker_ipc::tests::import_application_resumes_a_verified_interrupted_unified_import`, existant. Passe RuntimeImportPort/poll ; modèle Parakeet réel vérifié mais transcripteur contrôlé, aucune inférence native dans ce cas. | Aucun résultat équivalent retrouvé ; établir une référence pré-B avec le même cas. |
| Repli Auto | PersonalAutoFallback : `native_personal_auto_without_candidate_reports_preaudio_cpu_fallback`, Parakeet CPU réel via application/poll. | `5a53d776` : corps 26,14 s, Cargo 27,203 s. Absence de candidat GPU, pas échec d'initialisation GPU. |
| Direct Turbo CPU | Nouveau profil PollTurboCpuDirect et nouveau test unitaire natif `native_direct_turbo_cpu_finishes_through_poll` dans `poll/tests/native_direct_cpu.rs`. Moteur et worker CPU réels, capture simulée ; appels par ImportRuntime::poll. | Aucun résultat équivalent retrouvé ; établir une référence pré-B avec ce même test. Le contrat worker TurboCpuLive à `19e17019` (24,36 s) ne remplace pas ce parcours parent. |

Le support existant `DirectLive::from_test_capture_with_cpu_worker` est
pub(crate) sous cfg(test), prend LiveRequest/capture/worker/instance, et
démarre le vrai worker. `runtime.direct_live` est accessible aux tests frères.
Aucun nouveau point d'injection, champ ou changement de production requis.
Le nouveau test sera ignoré en N1 sans native-tests ; pas de retour silencieux
en cas de ressource absente. Vérifier les événements corrélés, le moteur CPU,
Stop et les sorties TXT/SRT/WAV cohérentes d'un flux simulé déterministe.
Ne pas présenter ce cas comme une validation du microphone Windows ou de l'UI.

**Chemins supplémentaires proposés pour ce complément N2 uniquement :**
nouveau `crates/whisper-adapters/src/worker_ipc/runtime/poll/tests/native_direct_cpu.rs`,
sa déclaration dans poll/tests.rs, son entrée dans ci/tests-baseline.txt,
et deux profils dans `ci/native-profiles.json`. Features native-tests existantes
seulement ; aucun manifeste Cargo, lock, script de garde, dépendance, ancien
test ou code de production modifié. Revue indépendante et N1 avant commit
de qualification ; vérifications SelfTest/Preflight de la garde native avant Run.

**Préparation de ressources, proposée sans écriture déjà réalisée :**

- Reconstituer sous `C:/wt/poll/target/n2-native/resources/personal-runtime/Whisper`
  les 21 fichiers / 3 196 008 875 octets attendus par les profils existants.
  Source préservée `C:/dev/whisper/target/pile-preserved/r8/target/n2-native/resources/personal-runtime/Whisper`
  : 19 fichiers / 857 478 144 octets présents ; ajouter ses deux modèles depuis
  `C:/dev/whisper/models/`, sans modifier ces originaux.
- Copier les deux modèles également sous `C:/wt/poll/models/` pour le cas
  de reprise existant qui impose le modèle catalogue à cet emplacement.
  Identités relues : Turbo `1fc70f77…e8e2bc69`, Parakeet `e3880d0a…b7f4d32e`.
  Worker CPU du seed présent, SHA `eb33d96b…751bbfef`, commun aux deux versions
  du parent ; ne pas confondre comparaison du parent et comparaison de workers.
- Copier les 297 clips et deux TSV de provenance nécessaires depuis
  `C:/dev/whisper/sound example/` vers les mêmes chemins relatifs sous
  `C:/wt/poll/sound example/`. Copier les quatre fichiers WAV/annotations longs
  depuis le répertoire préservé `.../resources/long-cpu/` vers les resources N2
  du worktree ; seuls les fichiers nécessaires aux deux cas, pas tout le corpus.
- Réutiliser comme source du MP3 court la copie préservée
  `C:/dev/whisper/target/pile-preserved/r8/target/n2-native/runs/PersonalAutoFallback-d5751e2d5e8845509332e49d37436f82/writes/l04/fixtures/en-short.mp3`
  (24 237 octets, présence vérifiée), la recopier sous les resources N2.
- Contrôler chemins canoniques, absence de jonction, tailles et SHA source/copie
  avant/après. Préparer le cache Cargo isolé de N2 depuis le cache persistant
  existant du worktree, sans téléchargement. Cible Cargo partagée inchangée ;
  TEMP/TMP/LOCALAPPDATA restent ceux de la garde. Aucun settings/log/config produit
  n'est précréé. Aucun original, installation Windows ou cache partagé supprimé.

**Comparaison proposée :** utiliser une référence de production pré-B
`6316a54b9681716b1d643dc8f2d60e8038d6b256`, avec le même banc de qualification
test/données que le candidat. La session principale peut préparer une branche
locale `codex/poll-n2-reference` depuis ce commit, y appliquer uniquement ce
banc de tests et les profils (aucune modification du code de production),
exécuter reprise/direct, puis revenir à `refacto/poll` pour les cinq cas B
(deux longs, reprise, Auto et direct). Toute bascule Git se fait après fin des
processus de garde, sur état propre et avec responsabilité d'écriture coordonnée.
Le diff de production de la référence doit rester celui de 6316a54b ; le nouveau
test direct et les profils doivent être identiques dans les deux versions.
La référence ainsi construite est datée du présent essai, jamais présentée
comme une ancienne campagne retrouvée. Comparer aussi longs et Auto à leurs
derniers résultats historiques ci-dessus, avec leurs limites explicites.

Cela représente sept corps ciblés au total : deux sur la référence et cinq
sur B. Arrêt au premier échec ; pas de profil GPU, pas de D2 complet, pas de
fenêtre Windows ou de capture physique. Archiver les petites preuves avec
leur arborescence et vérification d'archivage ; conserver modèles, corpus,
sources et caches partagés protégés. Après résultats ou décision utilisateur
sur N2, figer le compte rendu B d'une page, puis arrêt avant C. Aucun push.

L'utilisateur autorise explicitement dans la session **le complément N2
comparatif** décrit ci-dessus : test direct parent avec support existant,
deux profils, copies de ressources aux trois racines du worktree et sept
exécutions CPU (deux pré-B avec le même banc, puis cinq B). L'accord inclut
la référence de production 6316a54b ; aucun code de production, profil GPU,
fenêtre Windows ou microphone physique n'est ajouté à ce périmètre.
Une simple exécution des profils existants ne serait pas annoncée comme
couverture complète des quatre parcours.

Avant versionnement de cet accord et de la préparation N2, suite N1
permanente répétée sous `n2-plan-n1-*` : Structure, défaut, common, CPU
et Parakeet CPU réussis. Source Rust inchangée depuis 7e30d667. Aucun
staging, test natif ou modification du banc N2 exécuté avant ce commit.

## Famille 5 — Stop avant activation et reprise d'import

**5a.** Ajouter des tests séparés sous `worker_ipc/runtime/poll/`, verts sur
le comportement existant. Couvrir les routes pending-live et reprise import :
erreur de retrait d'une tentative, annulation PublicationGate, attente du
choix CPU, et reprise CPU permise. Conserver le nouveau test GPU manuel ajouté
pendant la levée des réserves A ; ne pas modifier le test historique dont le
nom ne correspond pas à la branche réellement couverte.

Deux mutations locales non commitées, chacune détectée par au moins un test :

- supprimer le contrôle PublicationGate du Stop pending-live ; vérifier
  l'événement Stopped, le nettoyage propre à cette route et l'absence de reprise ;
- forcer la reprise import CPU malgré le Stop mémorisé ; vérifier que le faux
  worker CPU ne reçoit aucun démarrage dans le comportement de référence.

Consigner pour chaque mutant le changement exact, le test rouge et la
restauration avant extraction. Renforcer la caractérisation si le mutant
n'est pas détecté. La branche import sans requête sous une garde exigeant
`import_request.is_some()` reste conservée et documentée, sans état artificiel.

**5b.** Extraire les deux routes séparément, sans fusion des handlers et sans
réordonner copie de requête, retraits, contrôles, changement d'état ou lancement.
Préserver les sorties immédiates et l'absence de traitement final commun.

## Famille 6 — Stop d'un direct actif

**6a.** Caractériser les chemins observables : failure mémorisée, drainage,
instantané durable/confirmé, barrière et couverture, retrait de tentative,
annulation, attente du choix CPU, Stop mémorisé, reprise, publication et
nettoyages. Les gates de campagne désactivées hors `recette-hooks` sont
documentées ; leur couverture ne justifie aucune activation de feature.
Une panne de drainage exigeant une injection nouvelle est proposée séparément.

Deux mutations locales non commitées obligatoires :

- inverser deux retraits de nettoyage, avec un oracle sur leur ordre réel ;
- laisser une barrière de persistance invalide atteindre la publication.
  Construire un préfixe PCM et texte réellement confirmé et publiable, puis
  invalider seulement la barrière : référence en erreur StorageCorrupt, mutant
  allant jusqu'à la publication ; vérifier événement et absence des exports.

**Extension distincte autorisée le 3 novembre : témoin de destruction.** Les seuls
états finaux à None ne prouvent pas l'ordre. Ajouter exclusivement sous
`cfg(test)` une déclaration dans `staging.rs` et un support dans
`staging/test_support.rs`. Il envelopperait le JoinHandle existant du vrai
PcmWriter : attendre sa terminaison, puis appeler un observateur avant le
retour de son vrai Drop. Aucun nouveau champ ni changement de writer_loop,
Drop ou comportement compilé normalement. Le probe de worker initialement
envisagé ne peut pas distinguer live/transport : poll diffère Stopped jusqu'à
la sortie du processus. L'oracle retenu utilise l'API d'archive existante :
pendant le retrait de live, une seconde préparation du même groupe doit
refuser le verrou détenu par live_archive. Inverser live_archive et live
libère ce verrou avant l'observation, et le test échoue après poll sur un
résultat externe du callback. Ces deux retraits sont dans la liste autorisée
pour M1 ; aucun nouveau point d'injection n'est ajouté. Ce support fait
l'objet d'un changement séparé de Na, relu indépendamment, après réussite
des 100 répétitions. Aucun nouveau trait n'est autorisé par cette extension.

**6b.** Extraire des séquences contiguës : failure ; gates/drainage ; instantané,
validation et retrait ; décision de reprise ; publication. Les frontières
exactes restent à confirmer sur le candidat. Si les sous-handlers ne peuvent
tenir sous 100 lignes sans réordonner du code, arrêter et signaler l'écart.

## Vérifications et limites

À chaque commit : suite N1 par garde (`cargo test`, puis sélections
`whisper-worker-common`, `whisper-worker-cpu`, `whisper-worker-parakeet-cpu`),
Structure, format et lints utiles. Préflight, environnement isolé, espace libre
d'au moins 30 Gio et arrêt sur échec restent applicables. Registre des tests :
uniquement les entrées nouvelles correspondantes dans `ci/tests-baseline.txt`.

Contrôles PE : uniquement si le diff touche `crates/whisper-worker-*`, un
manifeste, Cargo.lock, `.cargo/` ou un script de compilation ; sinon écrire
**« non concerné »**. Ne pas réimposer les reprises PE historiques du lot A.

Chaque Nb déplace au plus 800 lignes. Chaque nouveau handler doit respecter
100 lignes et une complexité cognitive de 25 ; mesurer aussi poll(), dont
le reliquat C reste à extraire. Aucun réordonnancement pour atteindre un seuil,
aucune référence Clippy créée ou modifiée pour le contourner. Revue indépendante
par le coordinateur Rust, corrections par l'implémenteur, deux cycles au plus.
La session principale conserve documents et Git.

Après 6b, préparer la comparaison avec la dernière référence native disponible
pour import CPU long, reprise après interruption, repli Auto et direct Turbo CPU.
La campagne N2 requiert une **autorisation distincte** avant exécution ; un
profil existant ne vaut pas preuve que le scénario demandé a été qualifié.
Rapporter les éventuelles références manquantes. Aucun démarrage de C.

## Exécution du complément N2 — état suspendu le 3 novembre 2026

Avancée : quatre corps reprise/direct réussis ; banc et profils relus CLEAN.
Blocage : garde native interrompue avant résultat de l'import long français.
Décision : aucune relance ni clôture B avant arbitrage ; C non commencé.

Le banc de qualification est versionné dans `1be46656`. N1 : 734 réussites,
zéro échec, 149 ignorés ; SelfTest, Structure 931, format et Clippy réussis.
La garde native SelfTest et les quatre Preflight sont réussis. PE non concerné.
Les ressources copiées restent conservées ; runtime : 21 fichiers,
3 196 008 875 octets ; espace libre avant campagne : 596,79 Gio.
Le calcul préparatoire de taille sur des dictionnaires PowerShell a été
corrigé avant tout Preflight ; les fichiers eux-mêmes étaient intacts.

La référence locale `codex/poll-n2-reference`, `00c2d87e`, applique seulement
le commit du banc à `6316a54b` : quatre fichiers de qualification, 325 ajouts,
aucune modification du code de production. Le nouveau test direct et les
profils sont identiques au candidat B `1be46656`. Le registre et la déclaration
ne sont pas globalement identiques : les 23 tests B restent absents de la
référence. Le même reviewer confirme ce périmètre et les quatre reçus PASS.
L'association aux HEAD est enregistrée par l'orchestration ; les result.json
de la garde n'embarquent pas l'identité Git.

| Corps | Référence pré-B | B | Durées du corps, hors compilation |
| --- | --- | --- | --- |
| Reprise interrompue | e21ec5ec42f1416ab5904a072beb6935 PASS | 3725c5b343c84d42a6fbd4bdd412f743 PASS | 13,21 s / 13,35 s |
| Direct Turbo CPU | e81f38f6d83a47c79708a29f59d4a7cd PASS | ab895bb662ac4de8b8131450e80578fd PASS | 42,44 s / 42,54 s |

Reprise : transcripteur contrôlé, modèle réel vérifié ; direct : vrai worker
CPU épinglé identique, capture simulée de deux secondes. Aucune qualification
du microphone physique, de la fenêtre Windows ou de la qualité lexicale.
Les durées sont descriptives, sans conclusion statistique de performance.

Lancement PersonalLongCpu `3e531a3c5634426b84938340fa0516e7` interrompu par
la garde le 3 novembre à 15:33:48 UTC : deux événements Changed, à la racine
`target/parakeet-tests/refacto-poll-b` et dans
`5a-corrected-adapters/temp/tmp/archive/cleanup-test-58400-1793697636390224000/latest/destination/.whisper-publications/reservations`.
L'origine n'est pas établie. Une énumération récursive en lecture des anciennes
preuves était en cours ; cette simultanéité ne démontre pas une attribution.
Le stderr contient seulement la compilation adapters, stdout est vide : aucun
résultat de corps long. Aucun essai anglais ou Auto engagé. Aucun processus
Cargo, rustc ou worker CPU/Parakeet CPU restant lors du contrôle après arrêt.
Les preuves ne sont ni supprimées ni restaurées ; modèles et corpus conservés.

Preuves : `target/n2-native/logs/<profil>-<identifiant>/`, et
`target/parakeet-tests/refacto-poll-b/n2-*.log`, dont
`n2-reference-source-comparison.log`. Retour effectué sur `refacto/poll` ;
seuls `models/` et `sound example/` restent non suivis. Les trois corps N2
restants sont NON EXÉCUTÉS avec succès ; aucune qualification N2 complète.

## Reprise autorisée et résultats finaux N2

Après l'arrêt, trois vérifications en lecture seule ont été autorisées et
effectuées : reçus/observateur, chemins/attributs, chronologie/processus.
Aucune jonction sur les chemins concernés. Le dossier reservations a conservé
LastWrite 09:20:36 UTC et porte LastAccess 15:33:48 UTC ; son fichier JSON
conserve LastWrite 09:20:36 UTC. Ces éléments ne prouvent pas l'origine des
événements. La garde observe LastWrite/Size/noms, pas LastAccess. L'événement
reste d'origine inconnue ; aucune modification de garde ou de preuve.

L'utilisateur a ensuite expressément autorisé les trois corps restants,
garde inchangée, nouveaux journaux seulement sous target/n2-native, sans
parcours récursif des anciennes preuves pendant les essais et arrêt sur échec.
Cette reprise est réussie. L'observation de processus git add lors du départ
n'établit aucun conflit pertinent ni lien avec l'incident ; aucun processus
Git interrompu et aucune exemption déduite de son origine.

| Cas B | Corps actuel | Dernière référence historique | Reçu |
| --- | ---: | ---: | --- |
| Import français long | 168,57 s | 175,79 s, 1d30690e | PersonalLongCpu-20f0b556fa8247809038aa986b07ad0a |
| Import anglais long | 187,17 s | 195,44 s, 1d30690e | même profil, deuxième corps |
| Auto sans candidat GPU | 41,55 s | 26,14 s, 5a53d776 | PersonalAutoFallback-d2da2d209c86464782b55a25767721e1 |

Les sept corps autorisés ont donc un reçu PASS : deux pré-B, cinq B. Les
tests longs et Auto vérifient leurs publications ; aucune WER ni écoute
humaine ajoutée. L'écart Auto de +15,41 s est rapporté sans cause établie,
sans conclusion statistique ou promesse de vitesse. L'incident interrompu
reste conservé ; il n'est pas transformé rétrospectivement en test réussi.

N1 de clôture sous closing-* : défaut 685/128, common 11/1, CPU 12/9,
Parakeet CPU 26/11, soit 734 réussites, 149 ignorés, zéro échec ; Structure
931/931, silent-return-patterns=0. Source Rust inchangée depuis 1be46656.
Archive des petites preuves :
`target/parakeet-tests/refacto-poll-b/archive/REFACTO-POLL-B-logs-20261103.zip`,
1 517 entrées, 20 320 244 octets sources, 3 611 058 octets compressés ; nombre
et SHA-256 de chaque entrée vérifiés, arborescence relative conservée.

La commande de suppression du seul cache isolé N2 a été refusée par l'outil,
motif « blocked by policy », avant exécution. Aucun contournement. Contrôle
en lecture seule : `C:/wt/poll/target/n2-native/cargo-home`, 22 419 fichiers,
651 937 640 octets, aucun reparse dans ses ancêtres ou contenu, aucun processus
Cargo/rustc/rustdoc/cmake/ninja/cl/worker actif. Le cache est conservé ;
commande utilisateur de suppression du seul chemin :
`Remove-Item -LiteralPath 'C:\wt\poll\target\n2-native\cargo-home' -Recurse -Force`.
Recontrôler l'absence de campagne active immédiatement avant exécution.
Les caches partagés et persistants, modèles, corpus, WAV, sorties et preuves
restent conservés. Aucune suppression effectuée ; espace libre : 586,41 Gio.

## Dette explicitement demandée

Le helper d'archive A, `archive/test_support.rs`, remplace sous `cfg(test)` le
handle MP3 par une ouverture en lecture seule pour provoquer une vraie erreur
I/O. Le remplacer par **un trait injecté dans un lot ultérieur**. Cette dette
est consignée ici ; B n'implémente pas ce trait et ne refond pas l'archive.

Préparation, garde éventuelle, répétition, support éventuel, quatre Na/Nb,
N2 sur autorisation et clôture restent au plus dix tâches. Clôturer B par un
compte rendu figé d'une page avec mutations, résultats, mesures, limites et
références de comparaison ; aucun livrable ou outillage supplémentaire ajouté.
