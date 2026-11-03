Avancée : prérequis de cent passages micro ciblés réussi, zéro échec ; préparation B relue CLEAN.
Blocage : aucun sur la série ciblée et la suite N1 complète.
Décision : préparation B validée, commencer 5a puis 5b ; pas de C, N2 sur autorisation distincte, aucun push.

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
Drop ou comportement compilé normalement. Un faux worker ayant envoyé Stopped
resterait disponible pour répondre à une requête fichier sous le TEMP isolé
du lot. Lors du retrait de live, l'observateur attendrait cette réponse avec
un délai borné. L'inversion avec le retrait de transport tuerait et attendrait
le worker avant cette observation, faisant échouer le test. Vérifier la réponse
écrite par le worker, pas seulement la présence de la requête. Ce support fera
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

## Dette explicitement demandée

Le helper d'archive A, `archive/test_support.rs`, remplace sous `cfg(test)` le
handle MP3 par une ouverture en lecture seule pour provoquer une vraie erreur
I/O. Le remplacer par **un trait injecté dans un lot ultérieur**. Cette dette
est consignée ici ; B n'implémente pas ce trait et ne refond pas l'archive.

Préparation, garde éventuelle, répétition, support éventuel, quatre Na/Nb,
N2 sur autorisation et clôture restent au plus dix tâches. Clôturer B par un
compte rendu figé d'une page avec mutations, résultats, mesures, limites et
références de comparaison ; aucun livrable ou outillage supplémentaire ajouté.
