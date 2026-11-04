Avancée : 7a/7b commis ; 8a validé après fermeture ciblée de la dernière réserve.
Blocage : aucun pour 8a ; extraction 8b et clôture C restent à réaliser.
Décision : exécuter C selon le plan accepté, sans simplification ni fusion, en commits locaux seulement.

# REFACTO-POLL — sous-lot C

## Autorité et référence

Le 3 novembre 2026, après clôture B et vérification de son nettoyage,
l'utilisateur demande « ok passe à l'étape suivante ». Cette demande engage
C, familles 7 à 9 du [plan accepté](REFACTO-POLL-PLAN.md), dans `C:/wt/poll`,
branche `refacto/poll`, depuis `105b686e14a19cde2d2f637a5990dce0c51da515`.
Les bilans A/B restent figés ; leurs preuves natives ne qualifient pas par
anticipation le candidat final C. Aucun push ni nouvelle campagne N2 autorisé.

Objectif : terminer le découpage de `ImportRuntime::poll()` en conservant
événements, priorités, effets, contrôles et ordres de destruction exacts.
État initial : poll à 489 lignes Clippy, complexité 52 ; six handlers B
conformes aux seuils. État Git suivi propre ; `models/` et `sound example/`
non suivis, conservés. Espace libre initial : 586,78 Gio.

## Séquence et chemins

1. Référence N1 sur la source inchangée, puis versionnement de ce suivi.
2. **7a** : tests de caractérisation nouveaux, séparés sous
   `crates/whisper-adapters/src/worker_ipc/runtime/poll/`, verts sur la logique
   non extraite de la famille 7 ; aucune modification de production.
3. **7b** : extraction de collecte/priorités en sous-handlers distincts,
   après revue indépendante de 7a et commit des tests.
4. **8a** : caractérisation du routage restant et du traitement final commun,
   sur leur logique inchangée, puis revue et commit.
5. **8b** : extraction restante, sans fusion ; poll doit atteindre les seuils.
6. **9** : mesures finales, vérifications, archivage et bilan figé d'une page,
   puis arrêt. Famille documentaire : aucune extraction ni Na/Nb artificiel.

Chemins Rust : `runtime/poll.rs`, nouveaux fichiers sous `runtime/poll/`,
déclarations de modules nécessaires. `ci/tests-baseline.txt` reçoit seulement
les entrées des nouveaux tests, indispensables au contrôle Structure.
Documents : présent suivi et bilan sous `docs/architecture/`. Journaux et
temporaires : `target/parakeet-tests/refacto-poll-c/`. Aucun autre chemin
produit, test existant, manifeste, lockfile, dépendance ou signature publique
modifié. Les supports cfg(test) existants peuvent être utilisés sans extension.

## Risques de caractérisation

**Famille 7.** Priorités simultanées sessions unifiées/document/file ; libération
terminale ; test micro événement/absence/erreur ; expiration idle ; Direct
actif/terminé, récupération éventuelle de worker, erreur ; capture Parakeet en
erreur et final automatique ; priorités compteurs/reset/observations/télémétrie,
capture/équité ; attente de fenêtre persistée ; erreurs VAD/début de fenêtre,
Ready synthétique unique ; collecte Parakeet événement/absence/erreur ; Stop
différé bloqué par persistance ou sortie processus ; transport vide/erreur,
avec ou sans reprise. Affiner branche par branche avant extraction, en citant
les tests traversant poll et leurs oracles sur événements, état restant et
absence d'effets. Un test du seul helper ne protège pas la collecte de poll.

**Famille 8.** Progress, Stopped normal, Failed corrélé/résiduel et événement
inattendu ; erreur live différée avec Stop/drain/failure ; télémétrie après un
résultat non terminal ; distinction entre retour immédiat et traitement final.
Les familles déjà extraites restent inchangées.

Conserver et documenter les branches inatteignables sans état artificiel.
L'exception de fixtures défensives accordée à 6a ne s'étend pas à C. Si une
branche exige un nouveau point d'injection de production, arrêter cette étape
et présenter séparément le changement proposé ; aucun ajout produit dans Na.
Aucune limite artificielle de taille pour Na. Aucune mutation obligatoire
supplémentaire : celles des familles 2/3/5/6 sont conservées comme preuves
historiques, sans les présenter comme exécutées sur C.

## Contrats d'extraction et vérifications

Sorties explicites : point suivant, sortie immédiate (y compris None), passage
au traitement final commun. Collecte : distinguer événement acquis et sortie.
Ne pas créer de booléen ambigu, boucle de drainage, clone ou état produit
pour contourner les emprunts. Aucun changement dans poll_unified_sessions ou
poll_parakeet_worker. Ne pas fusionner les nettoyages ou handlers similaires.

Chaque Nb déplace au plus 800 lignes. Chaque handler et, au terme de 8b,
poll respectent 100 lignes Clippy et une complexité cognitive de 25. Si cela
exige de réordonner du code : arrêt et signalement. Aucun allow ou référence
Clippy ajouté pour contourner un seuil ; aucune référence Clippy nouvelle.

Référence avant le premier commit, puis à chaque Na/Nb et à la clôture :
garde N1 existante, SelfTest, Structure, format, Clippy workspace/all-targets,
`cargo test`, puis sélections common/CPU/Parakeet CPU. Commandes via la garde,
offline/locked ; cible partagée `C:/dev/whisper/target/parakeet-tests/cargo-shared`,
cache persistant du worktree `target/cargo-home-shared`, autres racines isolées
sous le lot, espace libre minimal 30 Gio. Arrêt sur échec selon AGENTS ; pas
d'extension de la dérogation historique accordée aux nouveaux tests A.

Le contrôle PE interne des harnais workers reste celui de la garde. Aucune
campagne PE supplémentaire pour le seul diff adapters/tests/documents ; le
bilan distingue ce périmètre des contrôles déjà intégrés aux appels N1.

La session principale conserve documents et Git. L'implémenteur Rust est seul
auteur des changements Rust/tests ; il préserve les travaux des autres. Chaque
candidat Na/Nb passe au coordinateur et au reviewer indépendant, deux cycles
revue/correction maximum par livrable. Diff de déplacement relu avant Nb.
Archive des petites preuves avec arborescence, nombre et SHA-256 par entrée
vérifiés ; caches partagés/persistants et ressources préservés. Six tâches,
deux rapports prévus ; aucun nouvel outillage ou contrôle distant.

## Exécution — référence C

Avant tout changement Rust et premier commit C : SelfTest 35 positifs / 79
négatifs ; Structure 931 tests / 931 entrées, 166 native-gated, zéro retour
silencieux ; format et Clippy workspace/all-targets réussis. N1 : défaut
685 réussites / 128 ignorés, common 11/1, CPU 12/9, Parakeet CPU 26/11,
soit **734 réussites, zéro échec, 149 ignorés**. Les cas ignorés ne sont pas
présentés comme exécutés. Journaux `target/parakeet-tests/refacto-poll-c/reference-*.log`.
Aucun changement de source suivi lors de la référence ; lecture F7 déléguée
sans édition ni autre Cargo concurrent. Le registre conserve ses 931 entrées.

## Préparation de 7a

L'erreur capture Parakeet exige une racine D2 dans le constructeur de capture
test existant. L'alternative sans modification de production est vérifiée :
comme `parakeet_live/diagnostics_absence_tests.rs`, le test parent relance le
test exact dans current_exe avec Command::env, racine temporaire et sentinel
propres à l'enfant. Aucun set_var unsafe, modèle ou worker natif. Les seules
entrées TEST/CHILD_ENV nécessaires sont déclarées au registre existant ; la
garde et ses règles restent inchangées. Le parent doit vérifier le succès de
l'enfant et l'exécution effective d'un test. Le premier doute sur une injection
nécessaire est ainsi levé par réemploi d'un mécanisme existant, sans dérogation.

### Arrêt Structure 7a, avant compilation des tests

SelfTest 7a réussi (35/79). Structure échoue : 942 tests découverts, 166
native-gated, une erreur d'environnement, zéro erreur de registre, helper ou
retour silencieux. Le test
`parakeet_poll::poll_parakeet_capture_error_returns_stopped_event_with_automatic_final_import`
lit `CHILD_ROOT`, déclaré comme constante au niveau module. Le contrôleur
résout les constantes seulement dans le corps du test (`ci/check-structure.rs`,
env_reads) : il ne peut donc pas relier ce nom à l'entrée CHILD_ENV déjà
déclarée pour WHISPER_POLL_PARAKEET_CAPTURE_CHILD_ROOT.

Arrêt respecté, aucune correction ni relance après échec. Les tests 7a ne sont
pas encore compilés/exécutés et aucune extraction commencée. Correction
minimale proposée : déplacer seulement cette constante dans le corps du
nouveau test, comme le patron existant diagnostics_absence_tests, sans changer
sa valeur, ses assertions, le registre ou la garde. Reprise des vérifications
et revue indépendante après arbitrage utilisateur. Aucun code produit changé.

L'utilisateur a autorisé ce seul déplacement et la reprise complète de 7a.
Après correction : SelfTest 35/79 et Structure 942/942 réussis. Le contrôle
fmt s'arrête ensuite sur `direct_live_poll.rs` (assertions à répartir sur
plusieurs lignes) et `parakeet_poll.rs` (appel de constructeur à regrouper).
Aucun changement de valeur ou d'assertion demandé par le formateur. Arrêt
respecté avant Clippy et compilation/exécution des tests ; aucune correction
ou relance effectuée après cet échec. Preuves sous
`7a-retry-fmt/scratch/20261103T172247468-7cadd5750bc94af6a63a0bd64d6d8cea/`.

Formatage ciblé autorisé par l'utilisateur, puis reprise : SelfTest 35/79,
Structure 942/942 et format réussis. Clippy workspace/all-targets s'arrête
ensuite avec E0603 dans collection_priority.rs : chemin via backends::native
privé au lieu du réexport NativeUnifiedRoute. Import Condvar inutilisé
également signalé. Tests non exécutés, aucune correction/reprise après arrêt.
Preuves : `7a-format-retry-clippy/scratch/20261103T172410264-694aa00cbd3f47b7841ea0f43bed2c1f/`.
Première revue indépendante du candidat figé demandée avant arbitrage des
corrections, pour regrouper les constats sans réduire la caractérisation.

### Matrice provisoire et lacunes déclarées avant revue

Les onze nouveaux tests, non encore compilés avec succès, sont répartis en
quatre fichiers : collection_priority (3), direct_live_poll (2), parakeet_poll
(4), transport_poll (2). Ils visent les priorités session/document/file/micro,
absence/erreur micro, expiration idle, cycle Direct et cache, collecte Parakeet
Some/None/Err, erreur capture/final automatique et transport sans reprise.
Ils ne constituent à ce stade aucune preuve d'exécution.

Compléments aux tests existants traversant le parent poll : observations et
compteurs/reset ; équité capture/worker dans microphone_and_finalization ;
confirmation différée dans window_confirmations ; libération terminale
unifiée et erreur de page document dans tests/unified_capture. La revue doit
confirmer leur portée N1 et leurs oracles, sans compter un test helper-only.

Deux routes atteignables restent explicitement NON COUVERTES :

- `transport.receive()` en erreur avec `recovery_session = true` ; les tests
  6a injectent Stopped et ne prouvent pas cette collecte en erreur.
- `deferred_live_stopped` retenu par `live_pending_persistence > 0`, puis par
  un processus encore vivant ; les tests 6a existants utilisent un enfant
  déjà sorti, et le test take_ready_live_stop ne traverse que le helper.

Ces routes doivent recevoir de nouveaux tests de caractérisation avant 7b.
Leur ajout fait partie de 7a ; aucun plafond de taille ne justifie leur omission.

Inatteignabilité annoncée à vérifier indépendamment : l'unique affectation
Some de parakeet_vad est après le refus DurationUnsupported du profil VAD
dans worker_start ; les affectations de poll sont des retraits. Pour Direct,
la construction et les réinstallations fournissent Some(worker), done est
marqué après Stopped corrélé avec stop_sent, et la récupération du worker est
suivie du retrait de Direct dans le même appel parent. Ne pas fabriquer de
fixtures pour ces branches si ces invariants sont confirmés.

### Première revue indépendante 7a — FINDINGS

Candidat figé, aucun Cargo ni correction pendant la revue. Constats :

- R-7A-1 BLOQUANT : utiliser le réexport NativeUnifiedRoute, sans élargir
  la visibilité de production. Retirer l'import Condvar inutilisé du seul
  nouveau fichier fait partie du nettoyage ciblé proposé.
- R-7A-2 BLOQUANT : le drain documentaire reste présent après Ready ;
  corriger l'oracle pour vérifier sa présence, puis son retrait après End.
- R-7A-3 BLOQUANT : la trace Ready précède l'envoi du signal ; elle ne prouve
  pas que Ready est consommable. Synchroniser sur l'état réellement prêt par
  un moyen de test existant, avec attente bornée, sans pause arbitraire.
- R-7A-4 BLOQUANT : ajouter des tests parent poll des deux attentes du Stop
  différé (persistance puis processus), absence de terminal/publication
  prématuré, état retenu et progression après levée des barrières.
- R-7A-5 IMPORTANT : le nouveau test de priorité Parakeet n'a pas d'événement
  concurrent legacy. Préparer deux événements distincts prêts, vérifier
  leur ordre et la conservation du second.

V-7A-MATRIX : matrice traçable requise avant 7b. V-7A-N1 : suite complète
requise après corrections. Contrairement au premier relevé de l'implémenteur,
`gpu_import_failed_then_eof_retries_cpu_and_accepts_same_job_ready` couvre
bien receive Err avec recovery pour l'import Auto/CUDA jusqu'au Ready CPU.
Il n'établit pas les attentes du Stop différé. Le voisin live(None) démarre
CPU et ne prouve pas une EOF GPU. La qualification de cette branche est donc
affinée, sans créer une absence globale de couverture inexistante.

La revue confirme les inatteignables VAD et Direct ci-dessus ; aucun test
artificiel ni nouveau point d'injection de production justifié. Les cinq
constats restent ouverts ; reprise suspendue jusqu'à arbitrage utilisateur.

Le reviewer précise R-7A-3 : `UnifiedInferenceExecutor::is_ready`, visibilité
`pub(in crate::worker_ipc)`, est accessible via runtime.unified_executor.
Après notification du Ready retenu, attendre son résultat vrai de manière
bornée, avant le premier poll, et propager les erreurs. Cette méthode mémorise
ready_seen sans consommer l'ImportEvent::Ready que le parent produira ensuite.
Aucun nouveau hook ou élargissement de visibilité requis.

L'utilisateur a autorisé explicitement le paquet R-7A-1 à R-7A-5, les seules
nouvelles entrées de registre nécessaires, puis N1 complète et seconde revue.
Corrections déléguées au même implémenteur, fichiers de tests 7a seulement.
La seconde revue doit porter sur le candidat corrigé et ses preuves complètes ;
le compteur de revue reste à un cycle effectué. Aucun commit 7a ou 7b engagé.

### Première exécution des tests 7a corrigés — arrêt

SelfTest 35/79, Structure 944/944 (166 native-gated, zéro retour silencieux),
format et Clippy workspace/all-targets réussis. Deux tests des attentes Stop
ont été ajoutés, soit treize tests nouveaux au total. La première commande
`cargo test -p whisper-adapters` a continué après le retour différé de l'outil.
L'implémenteur n'avait pas exposé son session_id et a d'abord observé seulement
les fichiers metadata. Lecture ultérieure ciblée : logs Cargo présents,
compilation terminée, résultat lib **350 réussites, cinq échecs, 49 ignorés**.
Ce retour différé n'est pas réinterprété comme une absence d'exécution.
Pas de réexécution pour récupérer le flux ou le code du wrapper.

Échecs, tous dans les nouveaux tests :

- poll_unified_session_ready_precedes_ready_document_drain ;
- poll_direct_none_keeps_capture_and_direct_error_retires_without_worker_cache ;
- poll_cleans_active_parakeet_state_after_transport_decode_error ;
- poll_parakeet_capture_error_returns_stopped_event_with_automatic_final_import ;
- poll_returns_none_while_parakeet_worker_is_alive_without_an_event.

Preuves : `7a-reviewed-adapters/scratch/20261103T174024941-411fd50418de4456ae0abe0db2eb795a/cargo-command.{stdout,stderr}.log`.
La suite défaut/common/CPU/Parakeet n'est pas engagée sur ce candidat.
Arrêt respecté ; aucun correctif supplémentaire après constat. Diagnostic
lecture seule borné demandé à l'auteur pour préparer l'arbitrage groupé,
sans changement produit ni affaiblissement des assertions. La seconde revue
reste à effectuer après correction autorisée et vérifications complètes.

### Diagnostic des cinq échecs, sans réexécution

Le 3 novembre, l'utilisateur demande de poursuivre. Diagnostic des journaux
et sources en lecture seule, sans changement de test ni nouveau Cargo :

- Priorité unifiée : l'assertion in_flight immédiate est incorrecte. Avec
  READY retenu, poll_unified retourne avant begin_call et conserve la fenêtre
  dans pending_prepared. Attendre de façon bornée le PCM durable et cet état
  accessible au test, avec in_flight restant vide ; ajouter ensuite le drain
  document, relâcher READY, attendre is_ready, vérifier READY session avant
  READY document sans imposer leur adjacence. Le retrait reste vérifié à End.
- Deux fixtures Parakeet : leur exécutable est créé hors de runtime_root.
  Les placer sous le runtime temporaire déclaré, sans affaiblir le validateur
  PathOutsideRuntime ni les oracles absence/erreur et nettoyage.
- Direct : les deux scénarios réutilisent destination/identité et donc le
  même WAV, qui existe après le premier. Isoler leurs racines et identités,
  sans retirer le WAV comme contournement ni relâcher les assertions.
- Le test de capture Parakeet ne provoque pas l'erreur attendue : pump_capture
  est appelé par ServiceRuntime::pump_realtime, qui traite l'erreur et retire
  lui-même parakeet_live. Appeler seulement poll ne pompe donc pas le bloc.
  Ce parcours de service ne remplace pas la preuve de la branche Err de
  live.poll() à extraire.

Pour ce dernier cas, proposition concrète sans injection nouvelle : capture
mono valide persistée par le pump existant pendant que le faux worker attend
une gâchette ; worker Rust contrôlé fournissant une trame Ready CPU valide ;
consommation du Ready par poll ; création d'un répertoire window-1.wav sous
le window_root de cette seule fixture. Le prochain schedule_window depuis
live.poll tente write_pcm16_wav(create_new) et rencontre une vraie erreur I/O.
Vérifier ensuite Stopped, complete=false, automatic_final présent et retrait
de parakeet_live. Nom et entrée de registre du nouveau test doivent annoncer
une erreur d'écriture de fenêtre, sans prétendre tester une panne matérielle
de capture ou un moteur natif. Aucun modèle ou worker natif n'est nécessaire.
Si la fixture contrôlée exige finalement un support de production nouveau,
arrêter et le proposer séparément avant modification, sans substitution de
parcours ni assertion affaiblie. Corrections et reprise restent à autoriser.

L'utilisateur a ensuite autorisé précisément ces cinq corrections, leurs
ajustements de noms et nouvelles entrées de registre, puis les vérifications
complètes et la seconde revue. Même implémenteur chargé de la correction ;
aucune modification de production, test existant ou règle de garde autorisée.
Le même coordinateur confirme la conservation du premier passage de revue
et la reprise du même reviewer après gel du candidat corrigé et preuves N1.
La proposition de support nouveau reste soumise à un arrêt distinct si elle
devient nécessaire ; aucune extension de production n'est autorisée ici.

### Arrêt sur les assertions de chemins optionnels du nouveau test I/O

Corrections autorisées appliquées dans les seuls nouveaux tests et leurs
entrées de registre. Le faux worker Ready utilise le protocole existant ;
aucun support de production ajouté. Fixture mono de 32 000 échantillons,
seuil réel WINDOW_STEP_SAMPLES, et collision dans le dossier temporaire.
SelfTest 35/79, Structure 944/944, format réussis. Clippy s'arrête avec deux
E0277 : txt_path et srt_path de ParakeetCaptureStopped sont Option<String>,
et Path::new reçoit l'Option dans les nouvelles assertions d'existence.
Fichier parakeet_poll.rs, lignes 332 et 333. Aucun test exécuté sur cette
révision, aucune correction ou relance après l'échec.

Source de l'oracle : stop_after_capture_error utilise export_parakeet_outputs,
qui renvoie Some pour les exports TXT/SRT de capture si leur écriture réussit.
Ces exports incomplets se distinguent de l'import final automatique à venir.
Correction ciblée proposée : exiger explicitement la présence des deux
chemins optionnels avant de vérifier leurs fichiers, sans rendre ces
assertions facultatives ni modifier la production.
Preuve : `7a-fixture-retry-clippy/scratch/20261103T214553747-5a16328ff9484ed48b3ad456f96a083d/`.

L'utilisateur autorise uniquement de déballer explicitement ces deux chemins
optionnels en exigeant Some, puis de maintenir les vérifications d'existence
des fichiers ; reprise complète des vérifications 7a et seconde revue. Aucune
assertion rendue facultative, aucun autre correctif autorisé après cet arrêt.

Après cette correction autorisée : SelfTest 35/79, Structure 944/944, format
et Clippy workspace/all-targets réussis. Exécution adapters terminée avec
retour effectif récupéré par session_id puis write_stdin : Cargo 101,
garde/commande 1, **353 réussites, deux échecs, 49 ignorés**. Les scénarios
Direct et les deux fixtures de chemin Parakeet passent désormais.

Restent en échec :

- le nouveau test d'erreur d'écriture de fenêtre : le rustc appelé par la
  fixture refuse le JSON littéral passé directement à println! (accolades
  interprétées comme un format Rust) ; le faux worker n'est pas exécuté ;
- priorité unifiée : l'attente pending_prepared avec durable_samples >= 64 000
  expire après cinq secondes. Aucune augmentation de délai ou nouvelle
  variante essayée sans diagnostic de la précondition réelle.

Preuves : `7a-option-retry-adapters/scratch/20261103T214915161-c4fb99b84b984f1bae0d56969835e5c4/`.
Aucun correctif ou commande après échec ; suite défaut/common/CPU/Parakeet
non engagée. Diagnostic lecture seule borné demandé avant arbitrage groupé.

Diagnostic des deux échecs terminé sans essai supplémentaire :

- imprimer le payload Ready JSON comme donnée, pas comme format println ;
- les 64 000 échantillons atteignent PREVIEW_SAMPLES et le seuil d'enqueue
  du preview. Les logs n'exposent pas séparément ACK durable, preview et
  pending_prepared : la cause de la précondition absente reste indéterminée.
  Aucun délai augmenté, aucune nouvelle valeur essayée.

Le test existant
`unified_runtime_keeps_reserved_final_turn_through_preparation_with_peer_preview_ready`
fournit une source PCM déjà fermée/durable et vérifie un pending_prepared
réel avec hold_next_child_ready. Adapter ce montage dans le nouveau test
permettrait de vérifier la priorité READY session/document par le parent
poll, sans dépendre de la progression d'une capture active. Limite à annoncer
dans la matrice : ce cas ne prouve pas capture active -> ACK -> preview sous
READY retenu ; le helper poll_unified_sessions reste hors extraction.
Alternative : instrumentation des seuls champs de test existants et un
passage adapters diagnostique borné pour identifier l'étape manquante, sans
modification de production ni nouvelle garde. La fixture et la reprise
restent soumises à arbitrage ; aucun remplacement silencieux effectué.

L'utilisateur choisit explicitement « JSON + fixture PCM durable +
vérifications ». Correction déléguée au même auteur : impression du payload
Ready comme donnée et montage à source fermée/durable dans le nouveau test
de priorité parent, avec les API accessibles existantes. L'ancien test de
référence n'est pas modifié, aucune visibilité élargie. Sa limite de preuve
capture active reste consignée ; aucun diagnostic d'un bug produit n'est
déduit du timeout. Reprise complète N1 et seconde revue autorisées, toujours
avec arrêt au premier nouvel échec. Aucun 7b ou commit 7a commencé.

Reprise autorisée terminée : SelfTest 35/79, Structure 944/944, format et
Clippy workspace/all-targets réussis. Adapters : **354 réussites, un échec,
49 ignorés**, Cargo 101 et garde 1. Le nouveau test de priorité avec source
PCM durable passe ; sa limite capture active reste celle approuvée ci-dessus.
Le faux worker JSON compile désormais. Le seul échec est l'assertion de
ParakeetCaptureProgress immédiatement après le pump dans le nouveau test I/O
(parakeet_poll.rs:285), avant libération du faux Ready.
Preuves : `7a-pcm-retry-adapters/scratch/20261103T220232409-c4839a308b5d49efb8971eaebd8d6dd1/`.

Arrêt sans correction ni suite N1 restante. Diagnostic lecture seule demandé
sur la préemption par Started/télémétrie et sur l'attente du Stopped après
collision, ainsi que le contrat Ready encore non exercé. Les identités,
32 000 échantillons capturés/durables, complete=false, automatic_final et
les exports/nettoyages attendus ne doivent pas être rendus facultatifs.

Diagnostic lecture seule de ce dernier échec : le constructeur place
ParakeetLiveStarted en tête, et le pump ajoute la progression après un gain
éventuel. La télémétrie peut préempter la file et, après InferenceReady,
précéder également schedule_window. Les assertions d'adjacence ne sont donc
pas des propriétés de l'origine. Proposition : consommer Started attendu,
puis boucles à nombre borné d'appels poll qui n'acceptent que les événements
intermédiaires connus de la même identité jusqu'à la progression exacte et
au Stopped attendu. Toute erreur ou terminal inattendu échoue. Aucun sleep
ajouté, délai Ready existant conservé ; pas de budget cumulatif différent.
Le JSON Ready correspond statiquement au schéma v1, au profil import-single-pass
et au backend CPU attendus par accept_ready ; aucune acceptation effective
n'est encore prouvée, l'échec ayant précédé ready.release. Aucun autre écart
concret trouvé dans ce contrat ; aucune correction ni nouvelle exécution.

L'utilisateur autorise uniquement ces attentes d'événements dans le nouveau
test I/O : Started attendu, puis événements intermédiaires connus jusqu'à
la progression exacte et au Stopped. Identités strictes, appels poll bornés,
aucune pause ni changement de délai Ready ; tous les contrôles d'échantillons,
état incomplet/import final, fichiers et nettoyage conservés. Reprise complète
N1 et seconde revue autorisées ; même auteur, aucun autre changement de portée.

Reprise après correction des attentes : SelfTest 35/79, Structure 944/944,
format et Clippy workspace/all-targets réussis. Adapters termine avec 354
réussites, un échec, 49 ignorés, Cargo 101 / garde 1. La boucle stricte voit
un vrai ParakeetCaptureProgress : captured_samples=31 840 et
audio_durable_samples=31 840. L'égalité à 32 000 échoue avant la libération
Ready et la collision ; elle n'a pas été abaissée. Aucun test suivant ou
correctif après l'arrêt.
Preuves : `7a-events-retry-adapters/scratch/20261103T221425752-c92598c6848449ba83ce5a874b21ece3/`.
Diagnostic lecture seule demandé sur Pcm16Converter (tampon, délai et flush),
pour préparer réellement assez d'audio durable sans modifier le seuil produit.

Le diagnostic statique identifie les 160 échantillons : Pcm16Converter saute
delay_remaining dans collect_compensated, puis finish draine jusqu'à la cible
exacte calculée depuis les frames d'entrée. Deux secondes / 32 000 entrées
donnent donc 31 840 avant Stop ; la première fenêtre requiert 32 000 et n'est
pas tentée. Aucun constat de perte audio produit n'en est déduit.

Proposition explicite de la session principale, non encore autorisée : trois
secondes / 48 000 entrées mono, progression de la bonne identité cohérente
(captured==durable) et au moins 32 000 pour la fenêtre ; après l'erreur I/O et
le tail de conversion à Stop, vérifier exactement 48 000 échantillons dans
le WAV final. L'erreur reste physique dans window-1.wav sous la fixture et
le seuil produit reste inchangé. Ce nouvel oracle remplace l'égalité à
32 000 avant Stop ; aucune modification appliquée avant arbitrage.

Le 4 novembre 2026, l'utilisateur autorise cette fixture de trois secondes
et ce nouvel oracle de conservation finale, puis les vérifications complètes
et la seconde revue. Correction limitée au nouveau test I/O et à son format :
aucun seuil ou comportement produit, test existant ou garde modifié. Reprise
déléguée au même auteur ; arrêt au premier nouvel échec. Contrôle de reprise :
HEAD f34a212b, espace libre 582,94 Gio, changements suivis limités aux tests,
registre et présent document ; ressources non suivies préservées.

### Validation finale 7a — 4 novembre 2026

Le candidat Rust et le registre sont figés. SelfTest 35 positifs / 79 négatifs,
Structure 944 tests / 944 entrées (166 flags native-gated, aucun test exécuté
par ce contrôle), format et Clippy workspace/all-targets réussis. Les
avertissements Clippy existants restent visibles, aucune référence modifiée.

Suite N1 permanente : défaut 698 réussites / 128 ignorés ; common 11/1 ;
CPU 12/9 ; Parakeet CPU 26/11, soit **747 réussites, zéro échec, 149 ignorés**.
Sélection adapters complémentaire : 435 réussites, zéro échec, 125 ignorés,
déjà incluse dans le défaut et non ajoutée aux 747. Tous les retours de garde
sont zéro et les terminaisons effectives sont capturées.
Preuves sous `target/parakeet-tests/refacto-poll-c/7a-three-seconds-*/scratch/` :
adapters `20261104T043053093-3782b448b1f4416d9dd23ed7eaea27e3`,
défaut `20261104T043134647-5f4b5d2f35ef4a82936a6894993b6744`,
common `20261104T043218045-62ce4411bf73484ca614922948a2f14c`,
CPU `20261104T043223145-c8d22c425c534c3db0420404020c5ae0`,
Parakeet CPU `20261104T043236082-3f58855ee0374246924de8f814ef18f3`.

Le test I/O reçoit une progression cohérente >= 32 000 mais < 48 000 avant
Stop, puis lit indépendamment le WAV final : RIFF/WAVE, PCM mono 16 kHz,
16 bits, longueurs RIFF/fichier cohérentes, data 96 000 octets, fichier
96 044 octets ; exactement 48 000 échantillons après récupération du tail.
L'arrêt reste incomplet, import final automatique présent, TXT/SRT attendus
et état nettoyé. Aucun moteur natif ou microphone matériel qualifié par ce cas.

Seconde revue confiée au même reviewer via le même coordinateur, R-7A-1 à
R-7A-5 et V-7A-MATRIX/N1 à confirmer fermés. Le compteur de revue n'est pas
réinitialisé. Aucun code de production, test existant ou garde modifié.

## Matrice finale du candidat 7a revu — 4 novembre 2026

Cette section remplace le relevé provisoire pour le verdict de ce candidat.
Références sous `crates/whisper-adapters/src/worker_ipc/` : C =
`runtime/poll/tests/collection_priority.rs`, D = `runtime/poll/tests/direct_live_poll.rs`,
P = `runtime/poll/tests/parakeet_poll.rs`, T = `runtime/poll/tests/transport_poll.rs`,
A = `runtime/poll/tests.rs`, W = `runtime/poll/tests/window_confirmations.rs`,
M = `tests/microphone_and_finalization.rs`, U = `tests/unified_capture.rs`,
R = `tests/reprocessing_and_transport.rs`,
G1 = `runtime/poll/tests/deferred_window_poll.rs`,
G2 = `runtime/poll/tests/capture_fairness.rs`.

Les treize tests nouveaux et les tests N1 existants cités passent dans les
preuves `7a-three-seconds-default` ; ce relevé est une traçabilité statique
des oracles, pas une couverture instrumentée ni une qualification native.
État courant complété par G1/G2, verts dans 7a-complement-retry-default
et vérifiés indépendamment lors de l'unique passe ciblée autorisée.

| Branche F7 | Test parent poll exact | Oracle et portée | Limite / état |
| --- | --- | --- | --- |
| Session unifiée prête avant document prêt | C `poll_ready_unified_import_precedes_ready_document_drain` | Ready session précède Ready document ; drain reste présent après Ready et disparaît à End ; vrai signal is_ready consommable. | Source PCM fermée/durable, route CPU contrôlée approuvée ; pas preuve capture active → ACK → preview sous Ready retenu. |
| Erreur terminale unifiée et libération | U `unified_terminal_failures_release_all_parent_state_and_preserve_source_audio` | Appelle runtime.poll ; identité et erreur contrôlée ; sessions, archives, drains et scheduler libérés ; source WAV récupérable encore présente. | Ne revendique pas une comparaison audio bit à bit. |
| Document avant file Parakeet et micro | C `poll_drains_document_before_queued_event_and_microphone_block` | Ready/End document avant événement en file, puis événement micro ; vérifie conservation des sources retardées. | Fixture contrôlée, pas d'entrée Windows réelle. |
| Erreur de lecture document | U `controlled_document_drain_terminates_after_page_read_error` | RuntimeImportPort.poll délègue à runtime.poll ; Failed corrélé, drain vidé, terminal unique. | Les tests appelant seulement poll_unified_document et les tests natifs ignorés ne sont pas comptés ici. |
| Micro événement, absence, erreur | C `poll_microphone_capture_absence_is_none_and_bad_frame_cleans_up` et précédent | None sans destruction, événement attendu, erreur de framing et nettoyage capture/test. | Capture synthétique existante. |
| Expiration idle et Direct None/Ready/erreur/terminal/cache | D `poll_direct_none_keeps_capture_and_direct_error_retires_without_worker_cache`, `poll_direct_terminal_reuses_worker_after_expiring_an_old_idle_worker` | Direct conservé sur None, retiré sur erreur sans cache ; ancien idle expiré retiré ; événement terminal et worker réutilisable observés au parent. | Direct terminé sans worker : inatteignable ci-dessous. |
| Live Parakeet événements et erreur de poll | P `poll_parakeet_window_write_error_returns_stopped_event_with_automatic_final_import` | Started/progression/Ready, vraie erreur d'écriture de fenêtre depuis live.poll, Stopped incomplet + import final ; 48 000 PCM dans WAV final, TXT/SRT et nettoyage. | Capture/worker contrôlés ; pas panne microphone matériel ni moteur natif. |
| Collecte Parakeet Some/None/Err et priorité legacy | P `poll_collects_queued_parakeet_event_before_legacy_transport`, `poll_returns_none_while_parakeet_worker_is_alive_without_an_event`, `poll_cleans_active_parakeet_state_after_transport_decode_error` | Deux événements Progress distincts prêts : Parakeet d'abord, legacy ensuite ; processus quiet conservé ; erreur IPC et états nettoyés. | Faux processus Rust sous runtime de fixture. |
| Transport sans événement / erreur sans reprise | T `poll_transport_absence_returns_none_without_releasing_active_attempt`, `poll_transport_error_without_recovery_releases_runtime_attempt` | None conserve tentative ; EOF/WorkerExited produit erreur et libère les états. | Transport Some également traversé par les tests Ready/segments/fenêtres A. |
| EOF transport avec reprise | R `gpu_import_failed_then_eof_retries_cpu_and_accepts_same_job_ready` | receive Err → reprise CPU Auto → Ready même job. | Le voisin live(None) part CPU ; il n'est pas annoncé comme preuve d'une EOF GPU live. |
| Stop différé : persistance et vie du processus | T `poll_stopped_live_waits_for_pending_segment_persistence_before_cleanup`, `poll_stopped_live_waits_for_child_exit_before_cleanup` | Vrai Stopped reçu au parent, mise en attente, état conservé sans terminal/publication prématuré, progression après ACK ou sortie enfant. | Deux causes éprouvées séparément, pas de test helper-only compté. |
| Compteurs, reset, observations prioritaires | A `poll_returns_recovery_counters_then_ready_attempt_reset_before_observations`, `poll_keeps_attempt_reset_pending_until_live_worker_is_ready`, `poll_reports_degraded_diagnostics_once_for_the_active_attempt`, `poll_defers_degraded_notice_until_an_active_attempt_exists` | Ordre compteurs/reset/observations, reset retenu avant worker_ready, avis de dégradation unique et conservé sans actif. | Caractérisations A inchangées, pas preuve de tous les cas télémétrie/capture concurrents. |
| Capture et télémétrie nominales sous activité worker | M `microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end` | Parent poll observe compteurs, RMS, spectre, puis Provisional/WindowFinished/End alors que la capture est alimentée ; progression bornée. | L'assertion refresh_events<=4 suit quatre appels ; elle ne prouve pas un quota exact de service worker en quatre UI events. |
| Quota, accumulation/reset et priorité télémétrie/capture concurrents | G2 `poll_serves_telemetry_then_worker_at_capture_quota_and_resumes` | Deux événements de capture au parent font progresser le compteur 0→1→2 ; télémétrie prioritaire avec capture éligible à 1 puis au quota 2 ; Progress retenu est servi au quota, remet le compteur à 0 puis capture reprend. | Compteur jamais écrit par le test ; blocs synthétiques et probe existant, invariants capturé ≥ durable ≥ confirmé. Helper-only historique non compté comme preuve parent. |
| Création de fenêtre différée et confirmation après persistance | W `poll_defers_window_confirmation_until_segment_persistence_then_confirms` | Segment → pending=1 ; WindowFinished différé, confirmé=0 ; PersistSegment → pending=0, Persisted puis WindowFinished, confirmé=320. | Ce test ne rappelle pas poll pendant l'attente déjà constituée. |
| Retour précoce None avec fenêtre déjà différée et persistance pending | G1 `poll_preserves_queued_worker_event_until_deferred_window_is_persisted` | Après Segment puis WindowFinished différé, troisième poll sous pending=1 et deferred présent retourne None malgré Progress prêt ; confirmé reste 0. PersistSegment confirme la fenêtre, puis le même Progress est rendu. | Parent poll traversé, source capturée/durable cohérente ; aucun nouveau point d'injection produit. |
| VAD decode / début fenêtre / Ready synthétique | Aucun test artificiel | Unique construction Some de parakeet_vad après le garde DurationUnsupported du profil VAD dans worker_start ; autres affectations None/retraits. | Inatteignable dans le flux normal actuel, confirmé par reviewer ; conservé. |
| Direct terminé sans worker | Aucun test artificiel | Construction/réinstallations avec Some(worker) ; take_idle_worker seulement après done, puis retrait Direct au même poll parent. | Inatteignable dans le cycle normal, confirmé par reviewer ; conservé. |

### Verdict historique après les deux cycles de revue 7a, avant complément

Le reviewer ferme R-7A-1 à R-7A-5 et V-7A-N1 : les treize nouveaux tests
et la suite N1 sont verts. V-7A-MATRIX reste ouvert sur les deux lacunes
ci-dessus. Le test helper de budget n'est pas une preuve du parent ; la
création d'une fenêtre différée n'est pas une preuve de son attente ultérieure.
Statut : **LIVRÉ AVEC RÉSERVES pour 7a**, aucune extraction 7b ni commit Na
engagé. La limite de deux cycles n'est pas remise à zéro. Un complément de
caractérisation et sa vérification indépendante ciblée nécessitent une
décision explicite de l'utilisateur avant poursuite ; pas de simplification
ou d'extraction autorisée sur la base de cette matrice incomplète.

### Complément proposé après les deux cycles — à arbitrer

Deux tests nouveaux sous poll/tests, sans changer les tests existants :

1. `deferred_window_poll.rs::poll_preserves_queued_worker_event_until_deferred_window_is_persisted` :
   vrais Segment puis WindowFinished au parent, persistance en attente et
   fenêtre déjà différée ; Progress legacy distinct prêt mais retenu pendant
   le poll anticipé None, confirmation toujours nulle. Après PersistSegment,
   vérifier Persisted/WindowFinished puis Progress resté en file. Aucun
   événement concurrent inférieur ne doit être consommé avant l'ACK.
2. `capture_fairness.rs::poll_serves_telemetry_then_worker_at_capture_quota_and_resumes` :
   deux événements de capture réellement renvoyés par le parent, compteur
   observé 0→1→2 ; dès le compteur 1, télémétrie probe rendue prête par
   submit/processed_chunks, bloc de capture éligible et Progress legacy prêts.
   Vérifier télémétrie avant ces sources, compteur inchangé et bloc retenu,
   puis seconde capture avant le worker. Au quota 2, un autre bloc reste
   retenu, Progress est servi et remet le compteur à zéro ; la capture reprend.
   La priorité télémétrie/capture est ainsi observée pendant que la capture
   est éligible, pas seulement quand le quota l'interdit déjà. L'oracle
   porte sur la limite actuelle de deux événements de capture par collecte
   worker, pas sur un quota de quatre appels UI déduit du nom d'un helper.

Les helpers de durabilité et de publication probe peuvent être locaux aux
nouveaux fichiers, avec PcmWriter::append_frame/drain et les API probe
existantes. Conserver les invariants capturé/durable/confirmé et ne pas écrire
directement le compteur d'équité pour simuler son accumulation ou reset.
Aucun nouveau point de production ni élargissement de visibilité. Seuls les
raccordements de modules et les nouvelles entrées TEST sont à ajouter.

Proposition de décision : autoriser exceptionnellement ce complément 7a,
N1 complète puis une vérification indépendante supplémentaire ciblée sur
ces deux tests et V-7A-MATRIX, sans remettre à zéro les deux cycles passés ni
ouvrir une nouvelle boucle automatique. Tant que la réserve n'est pas fermée,
aucune extraction 7b et aucun commit Na annoncé validé. Aucune écriture Rust
ou nouvelle campagne engagée sur cette proposition avant décision utilisateur.

Le 4 novembre, l'utilisateur autorise explicitement ce complément ciblé :
deux nouveaux tests, raccordements et nouvelles entrées de registre, N1
complète puis **une vérification indépendante supplémentaire ciblée** pour
fermer V-7A-MATRIX. Les deux cycles précédents restent consignés ; aucune
remise à zéro ni nouvelle boucle automatique. Même auteur et même reviewer,
aucun code produit, test existant ou garde modifié. Aucun 7b avant fermeture.

Les deux nouveaux fichiers deferred_window_poll.rs et capture_fairness.rs,
leurs déclarations et entrées TEST ont été ajoutés ; formatage ciblé et
diff-check réussis. SelfTest du complément passe 35/79. Structure s'arrête
avant compilation : invocation par `powershell -NoProfile -ExecutionPolicy
Bypass -File ...`, ProcessStartInfo.ArgumentList absent/null à la ligne 308
de la garde. Aucun Cargo ni test exécuté sur les nouveaux compléments ;
aucune correction ou relance après cet arrêt.

Diagnostic en lecture seule : powershell 5.1.26100.9711 ne possède pas cette
propriété ; pwsh 7.6.9 la possède. Les campagnes précédentes utilisaient
pwsh. Reprise proposée avec pwsh -NoProfile -File, garde inchangée et sans
ExecutionPolicy Bypass ; aucun autre ajustement de périmètre. L'échec du
lanceur reste consigné ; il ne vaut aucun résultat de tests complémentaires.

L'utilisateur autorise la reprise avec PowerShell 7. Même garde et mêmes
tests, nouveaux LotRoot 7a-complement-pwsh-*, sans ExecutionPolicy Bypass.
La suite entière doit passer avant l'unique vérification indépendante ciblée
déjà autorisée ; arrêt au premier nouvel échec, pas de correction implicite.

Avec pwsh : SelfTest 35/79, Structure 946/946 et format réussis. Clippy
workspace/all-targets s'arrête, Cargo 101 / garde 1. Lecture directe du
stderr : quatre E0308 concernent `total_samples: Some(320)`, attendu u64
(deux sites dans chacun des nouveaux fichiers), pas confirmed_fragment_samples.
Une E0596 concerne drain via as_ref dans capture_fairness ; un second drain
de même forme doit également être corrigé avant reprise (lignes 61 et 139).
Proposition minimale : conserver la valeur 320 sans Some, emprunt mutable
pour ces drains. Aucune correction appliquée ni autre commande après arrêt.
Preuves : `7a-complement-pwsh-clippy/scratch/20261104T074953011-5bbf7d987155468eae7322d15fd4ffe3/`.

La lecture des fixtures identifie aussi une précondition non valide commune :
un premier bloc source de 320 fournit seulement 160 échantillons convertis,
aucune trame complète de 320 admise/stagée par poll_live_capture. Les deux
amorçages attendent à tort captured/admitted/durable=320 dès ce bloc.
Une seconde entrée rend une trame complète. La vérification statique de
staging.rs précise que le writer synchronise après 25 trames ou sur Drain,
sans synchronisation périodique ni sur file vide : avec ces quelques trames,
le durable ne progresse donc pas spontanément avant le drain explicite.
L'hypothèse précédente de course de cet ACK n'est pas confirmée par le code.

Correction proposée avant arbitrage : amorcer les deux fixtures par deux
blocs source de 320, vérifier captured/admitted=320 puis drain=320 ; conserver
ensuite les oracles de progression 640, 960 et 1280 et les drains explicites
de la fixture d'équité. Corriger les quatre total_samples en u64 et les deux
emprunts mutables. Ne pas écrire le compteur d'équité, changer les assertions
de priorité, ni ajouter d'injection produit. Après accord : N1 complète avec
pwsh, arrêt au premier échec, puis l'unique vérification indépendante ciblée
déjà autorisée. Aucune de ces corrections n'est encore appliquée.

L'utilisateur autorise explicitement ces corrections et la reprise le
4 novembre : quatre valeurs total_samples, deux emprunts mutables et
amorçage par deux blocs de 320 dans les deux nouvelles fixtures. Le même
implémenteur reçoit ce périmètre, sans modification de production, test
existant ou garde ; N1 complète avec pwsh et arrêt au premier échec, puis
l'unique vérification indépendante ciblée autorisée.

Reprise autorisée : corrections appliquées aux deux seuls nouveaux fichiers.
SelfTest 35/79, Structure 946/946 (166 native-gated recensés), format et Clippy
workspace/all-targets réussis. Les deux tests complémentaires passent dans
le paquet adapters ; cargo test par défaut et les trois sélections workers
passent également. Preuves sous 7a-complement-retry-* : défaut 700 réussites /
128 ignorés, common 11/1, CPU 12/9, Parakeet CPU 26/11, soit 749 réussites,
zéro échec et 149 ignorés sans double compter la sélection adapters. Les
tests ignorés et les 166 tests native-gated recensés ne constituent aucune
preuve native exécutée. Le candidat est figé pour l'unique vérification
indépendante ciblée G1/G2 et V-7A-MATRIX ; aucune extraction 7b engagée.

Le même reviewer ferme V-7A-MATRIX après l'unique vérification ciblée
autorisée : G1 protège le troisième poll avant ACK et conserve Progress ;
G2 observe le compteur parent 0→1→2, la télémétrie prioritaire à 1/2,
le service worker, son reset à 0 puis la capture reprise. La matrice courante
référence les deux oracles. R-7A-1 à R-7A-5 et V-N1 restent fermés ; aucune
réserve 7a ouverte. Les deux cycles passés ne sont pas remis à zéro.
Cette clôture autorise le commit local 7a puis 7b selon le plan accepté ;
elle ne constitue ni nouvelle revue générale ni qualification native.

## Extraction 7b — en cours

7a commité localement sous 52db4b95, neuf chemins exacts, aucun code produit
modifié. Seuls models/ et sound example/ restent non suivis et préservés.
Le même implémenteur reçoit 7b : poll.rs et nouveaux modules de collecte
sous poll/, ordre inchangé, sorties explicites, handlers distincts, seuils
100 lignes / complexité 25, déplacement maximal 800 lignes. Aucun changement
de tests, registre, garde, manifeste ou lockfile. N1 complète puis revue
indépendante propre à ce nouveau livrable, deux cycles maximum ; arrêt au
premier échec. La conformité finale de poll est attendue après 8b.

Le candidat 7b est écrit dans poll.rs et le nouveau poll/collection.rs,
environ 250 lignes déplacées selon l'implémenteur. SelfTest passe 35/79.
L'appel suivant est refusé avant Cargo : pwsh -NoProfile -File
ci/guard-level1.ps1 -Structure -LotRoot
target/parakeet-tests/refacto-poll-c/7b-structure ; erreur ligne 119,
« A Cargo command is required ». Le paramètre existant correct est
-Mode Structure, déjà passé dans 7a-complement-retry-structure. Aucun test,
Clippy ou formatage depuis cet arrêt, aucune session active. Reprise proposée
avec -Mode Structure, nouvelle racine 7b-structure-retry, puis vérifications
N1 prévues et revue indépendante ; garde et candidat inchangés.

L'utilisateur autorise explicitement cette reprise : -Mode Structure puis
suite N1 et revue indépendante prévues, garde et candidat inchangés. Même
implémenteur, nouvelles racines de vérification, arrêt au premier nouvel
échec ; aucun changement de périmètre ni correction implicite.

Structure repris passe 946/946 (166 native-gated recensés, zéro retour
silencieux), format passe. Clippy workspace/all-targets s'arrête Cargo 101 :
collection.rs:311, receive() fournit PortError tandis que le nouveau
poll_transport_error attend std::io::Error. Une seule erreur E0308 révélée,
aucun test ni mesure ciblée lancé. Preuve :
7b-clippy/scratch/20261104T085254022-63a43d94ae9644b9b59d8251df93930b/.
Diagnostic statique : PortError est déjà accessible par les imports parents ;
correction proposée limitée au type de l'argument, corps inchangé. Aucune
correction ou relance depuis l'arrêt ; reprise N1 et revue à arbitrer.

L'utilisateur autorise le remplacement du seul type std::io::Error par
PortError dans poll_transport_error, sans changement du corps, puis N1,
mesures des handlers et revue indépendante 7b. Même implémenteur, nouvelles
racines 7b-typefix-*, aucun autre changement autorisé ; arrêt au premier
échec de vérification.

Après le seul correctif autorisé, SelfTest 35/79, Structure 946/946, format,
Clippy workspace/all-targets et N1 passent. Défaut 700/128, common 11/1,
CPU 12/9, Parakeet CPU 26/11 : 749 réussites, zéro échec, 149 ignorés.
Preuves sous 7b-typefix-*. Aucun test, garde, manifeste ou lockfile modifié.

Relevé implémenteur : 12 nouvelles fonctions, maximum 47 lignes physiques ;
251 lignes supprimées de poll.rs, nouveau collection.rs de 361 lignes,
617 lignes de diff textuel (suivi et nouveau), sous le plafond de 800.
Clippy relève poll à 242 lignes ; sa conformité finale reste attendue après
8b. Les lints lignes/complexité ne sont pas activés sur les nouveaux handlers
dans cette exécution : leur comptage physique/statique n'est pas présenté
comme une mesure Clippy. La garde actuelle refuse les arguments Clippy après
-- ; aucune extension ou autre moyen de mesure ajouté sans autorisation.
Clippy signale aussi large_enum_variant sur le nouvel enum de contrôle
PollCollectionControl (592/160 octets). Candidat figé transmis au même
coordinateur pour la revue indépendante 7b cycle 1, incluant ces deux points.

Revue 7b cycle 1 : aucun écart d'ordre, sortie ou nettoyage trouvé dans le
diff exact ; pas de fusion, 251 lignes déplacées sous 800. Les 47 lignes
physiques maximum suffisent pour le seuil 100, mais cognitive_complexity
n'est pas actif sur les 12 handlers : verdict BLOCKED par le seul préalable
V-7B-METRICS, seuil 25 obligatoire non encore démontré. large_enum_variant
reste une limite non bloquante, sans régression fonctionnelle ni allocation
nouvelle démontrée. poll à 242 lignes reste transitoire avant 8b.

Proposition précise du reviewer : annotation permanente
#[warn(clippy::too_many_lines, clippy::cognitive_complexity)] devant l'unique
impl ImportRuntime dans collection.rs, aucun corps modifié. Les seuils
100/25 existent déjà dans clippy.toml. Cette activation locale permet le
Clippy ordinaire via garde inchangée, sans allow ni référence nouvelle.
Après autorisation source distincte : SelfTest, Clippy, N1 prévue puis
vérification indépendante cycle 2 pour V-7B-METRICS ; aucune modification
appliquée sur cette proposition avant arbitrage.

L'utilisateur autorise explicitement l'activation locale permanente des
deux lints devant l'impl, sans changement des corps, puis N1 et la seconde
vérification indépendante 7b. Le même implémenteur reçoit cette correction
unique, racines 7b-metrics-*, garde et seuils existants inchangés, arrêt au
premier échec. Aucun changement de large_enum_variant ou autre code admis.

Après activation, SelfTest 35/79, Structure 946/946, format et Clippy
workspace/all-targets passent sous 7b-metrics-*. Les deux lints sont actifs
sur les 12 méthodes et ne signalent aucun dépassement des seuils 100/25 ;
Clippy ne publie pas leur score exact sous seuil. N1 : défaut 700/128,
common 11/1, CPU 12/9, Parakeet CPU 26/11, soit 749 réussites, zéro échec,
149 ignorés. Diff-check réussi ; 251 lignes retirées de poll.rs, module
collection.rs de 362 lignes, diff total 618 lignes. Seule l'annotation a
changé depuis la première revue ; candidat figé transmis pour la seconde.

Seconde revue du même reviewer : CLEAN pour 7b, V-7B-METRICS fermé.
L'annotation permanente couvre les 12 méthodes ; aucun dépassement 100/25
avec Clippy ordinaire via garde. Conclusions comportementales du premier
cycle maintenues : ordre, sorties et nettoyages conservés. PE supplémentaire :
non concerné (aucun worker, manifeste, lock, .cargo ou script de compilation
touché). Le warning large_enum_variant demeure non bloquant ; poll reste
transitoirement à 242 lignes jusqu'à 8b. Cette validation ne clôt pas C.

## Caractérisation 8a — en cours

7b commité localement sous 985f40e9, trois chemins exacts. Même implémenteur
chargé de caractériser le routage restant et le traitement final commun sur
leur logique inchangée : mapping branches/tests parent existants/lacunes,
puis nouveaux fichiers sous poll/tests/, déclarations et nouvelles entrées
de registre seulement. Aucun code produit, test existant ou garde modifié ;
arrêt si nouveau point d'injection requis. N1 complète puis revue indépendante
8a, deux cycles maximum. Les livrables 7a et 7b restent clos.

Mapping initial 8a de l'implémenteur : Progress/observations/phases et
Stopped/fermeture phases ont des tests parent existants, mais le drapeau de
télémétrie après résultat non terminal et l'état après Stopped normal restent
à isoler. Failed corrélé est testé surtout sur les phases, pas sur l'erreur
exacte et l'état ; Failed résiduel et événement inattendu exigent d'abord
la vérification d'atteignabilité après filtrage. L'erreur worker corrélée
pendant live manque de caractérisation parent sur stop capture/drain/failure/
Stop transport/None puis terminal. Les sorties immédiates famille 7 sont
couvertes ; ajouter l'oracle du passage au traitement commun si nécessaire.
Supports existants uniquement ; pas d'état impossible ou nouveau seam pour
forcer un ordre d'effacement non observable. Limites à documenter au lieu
d'inventer un test artificiel.

Joignabilité relevée : Failed corrélé atteint handle_worker_failure ; Failed
identifié étranger est filtré, Failed partiel consommé par ProtocolMismatch.
L'arm Failed résiduel tardif est donc inatteignable, conservé sans test
artificiel. CudaAvailable sans identité atteint le fallback StaleResponse.
L'ordre exact des quatre retraits du Stopped normal n'est pas observable
par les supports existants et reste un objet de revue du déplacement.

Cinq cas ajoutés dans le nouveau terminal_routing.rs, déclarés et enregistrés :
Failed corrélé, CudaAvailable inattendu, Stopped normal, erreur live différée,
Progress puis télémétrie. Aucun code produit ni test existant modifié.
SelfTest 35/79 et Structure 951/951 passent (166 native-gated recensés,
zéro retour silencieux). Format s'arrête : mod terminal_routing doit suivre
mod stop_recovery, plutôt que mod direct_live_poll. Aucun Clippy ni test
Cargo lancé ; aucune correction ou relance depuis l'arrêt. Preuve :
8a-fmt/scratch/20261104T091631861-f93937c3d3a1493dbd2c9307eba99234/.
Proposition : déplacer cette seule déclaration puis reprendre N1 et revue.

L'utilisateur autorise le seul déplacement de mod terminal_routing après
mod stop_recovery, puis la reprise N1 et revue indépendante 8a. Même auteur,
corps des tests et production inchangés, nouvelles racines 8a-orderfix-*,
arrêt au premier nouvel échec.

Reprise 8a-orderfix : SelfTest 35/79, Structure 951/951 et format réussis.
Clippy s'arrête Cargo 101 sur E0505, terminal_routing.rs:188 :
LiveArchive::prepare(&request.destination, request) emprunte le chemin en
déplaçant la requête. Aucun test N1 lancé, aucun changement après arrêt.
Preuve : 8a-orderfix-clippy/scratch/20261104T091737611-dcb857893e7f477a818b6f26ef66c8d3/.
Correction proposée : passer request.destination.clone() au lieu de
&request.destination, suivant les fixtures d'archive existantes, sans modifier
les assertions ou le produit ; reprise N1 et revue après arbitrage.

L'utilisateur autorise cette seule copie du chemin de destination dans le
nouveau test, puis N1 et revue indépendante 8a. Même implémenteur, aucune
assertion ni production modifiée, nouvelles racines 8a-borrowfix-*, arrêt
au premier nouvel échec.

Après correction d'emprunt : SelfTest 35/79, Structure 951/951, format et
Clippy réussis. La sélection défaut s'arrête Cargo 101 : 359 réussites,
3 échecs, 49 ignorés dans le harnais adapters ; les sélections suivantes
ne sont pas lancées. Nouveaux cas en échec :
poll_nonterminal_progress_returns_before_due_live_meter_telemetry (Meter
attendu au poll suivant), poll_normal_stopped_releases_pending_attempt_state
(réouverture StorageUnavailable, pending passage file exists),
poll_live_failed_drains_and_sends_stop_before_deferring_failure
(stop_requested attendu true, observé false). Import BackendKind inutilisé
également signalé. Preuve :
8a-borrowfix-default/scratch/20261104T091856496-278dc13aeacc46bfb026b10ced447f91/.
Aucune correction ou relance depuis l'arrêt. Diagnostic limité à trois
vérifications statiques et revue indépendante 8a cycle 1 engagés pour
confronter les oracles au comportement réel et grouper les corrections
éventuelles avant arbitrage ; aucun échec présenté comme test passé.

Revue 8a cycle 1 : R-8A-1 à 3 bloquants car Na doit être vert sur la logique
d'origine ; aucun défaut de production établi. Corrections proposées sur
les nouveaux tests seulement :
- R-8A-1 : compiler/attacher le faux worker avant publication de la mesure,
  puis prouver son Meter frais et conserver Progress/Meter corrélés et les
  assertions telemetry_due. latest_at rejette les snapshots après 500 ms ;
  la péremption pendant rustc est plausible d'après le code, non mesurée.
- R-8A-2 : remplacer la seconde préparation d'archive par acquisition directe
  du verrou group-<job>.writer.lock après retrait, sans supprimer le pending ;
  conserver toutes les assertions d'état final.
- R-8A-3 : vérifier probe valide avant puis invalidée après poll et flag
  stop_requested=false exact ; conserver None, erreur, drain et Stop corrélé.
V-8A-MATRIX reste ouvert : ajouter la vérification parent du terminal après
le Stop et l'erreur live différée, au lieu de s'arrêter au marqueur Stop.
Le Failed tardif est confirmé inatteignable, CudaAvailable testable. Retrait
de l'import BackendKind inutilisé proposé dans le nouveau fichier. Après
accord : N1 complète et seconde revue, aucun correctif produit ni support
nouveau de production. Aucune correction encore appliquée.

L'utilisateur autorise explicitement ce correctif groupé, limité aux nouveaux
tests : fraîcheur de télémétrie, acquisition directe du verrou, invalidation
réelle du probe, terminal ultérieur après Stop via supports existants et
retrait de l'import inutilisé. Les identités, erreurs, drain et nettoyages
restent stricts. Même implémenteur, production inchangée sur 985f40e9 ; N1
complète puis seconde et dernière revue 8a, arrêt au premier nouvel échec.

Correctif appliqué au seul nouveau terminal_routing.rs : snapshot publié
après compilation du faux worker, verrou acquis directement par try_lock
après nettoyage (pending préservé), probe valide avant puis invalidé après,
flag false exact, import inutilisé retiré. Le faux worker reçoit Stop puis
termine ; Stopped corrélé est injecté par l'API de test existante, et le
parent poll restitue l'erreur différée avec les mêmes job/génération/message,
puis retire live/transport/active/live_archive.

Preuves sous 8a-reviewfix-* : SelfTest 35/79, Structure 951/951 (166
native-gated recensés), format, Clippy et les cinq nouveaux tests passent.
N1 : défaut 705/128, common 11/1, CPU 12/9, Parakeet CPU 26/11, soit
754 réussites, zéro échec et 149 ignorés, sans sélection adapters doublée.
Candidat figé transmis au même reviewer pour la seconde et dernière passe
8a : R-8A-1 à 3, V-8A-N1 et V-8A-MATRIX. Aucun code produit modifié.

### Matrice courante famille 8

Tous les nouveaux tests ci-dessous résident dans poll/tests/terminal_routing.rs
et traversent le parent poll ; leurs résultats verts sont dans 8a-reviewfix-default.

| Branche | Test | Oracle et limite |
| --- | --- | --- |
| Progress et traitement final non terminal | poll_nonterminal_progress_returns_before_due_live_meter_telemetry | Progress exact, telemetry_due positionné, Meter frais au poll suivant puis flag effacé ; capture synthétique. |
| Stopped normal | poll_normal_stopped_releases_pending_attempt_state | Stopped corrélé, états retirés, verrou acquis directement ; l'ordre des retraits relève de la revue du diff. |
| Failed corrélé | poll_correlated_failed_returns_error_and_releases_attempt | Identité/erreur attendues et retrait de la tentative. |
| Événement inattendu | poll_unexpected_identity_free_event_returns_stale_response | CudaAvailable sans identité produit StaleResponse, état conservé selon le comportement actuel. |
| Erreur live différée puis terminal | poll_live_failed_drains_and_sends_stop_before_deferring_failure | None initial, probe invalidé, drain durable 320, erreur mémorisée, Stop reçu réellement ; Stopped corrélé par support existant puis erreur terminale exacte et nettoyage au parent. |
| Failed résiduel tardif | Aucun test artificiel | Inatteignable après les filtres et le traitement amont des identités partielles ; branche conservée. |

Les sorties immédiates et leurs priorités restent protégées par les tests
7a inchangés ; leurs oracles ne sont pas remplacés par des tests de helpers.

### Verdict 8a après deux cycles

Le reviewer ferme R-8A-1 à 3, V-8A-N1 et V-8A-MATRIX. Cinq nouveaux tests
passent ; N1 754 réussites, zéro échec, 149 ignorés ; Structure 951/951.
Statut : LIVRÉ AVEC RÉSERVES, pas de commit 8a ni extraction 8b.

R-8A-4 IMPORTANT reste ouvert : le faux worker écrit son marqueur Stop avant
son return. Le parent attend seulement ce marqueur puis injecte Stopped et
exige le terminal au premier poll. Or le processus peut encore être vivant ;
la vraie barrière try_wait impose alors légitimement None. Course constatée
statiquement, sans nouvel échec observé ni relance. Le marqueur n'est pas
une preuve de sortie effective du processus.

Correction minimale proposée : synchroniser le nouveau test sur la sortie
effective du faux processus avec attente bornée et traitement des erreurs,
puis conserver l'injection Stopped corrélée et l'assertion terminale stricte,
ainsi que marqueur Stop, erreur et nettoyage. Aucun changement produit ou
délai accru. La limite de deux cycles n'est pas remise à zéro : correction,
N1 et UNE vérification indépendante supplémentaire ciblée sur R-8A-4
nécessitent une décision exceptionnelle explicite ; aucun travail engagé
sur cette proposition avant arbitrage.

L'utilisateur autorise cette correction et l'unique vérification ciblée :
attendre la sortie effective du faux worker de façon bornée avant Stopped,
sans augmenter le délai ni assouplir les assertions ; N1 complète puis même
reviewer sur R-8A-4 seulement. Deux cycles antérieurs conservés, aucune
remise à zéro, aucun code produit changé. Même implémenteur, nouvelles
racines 8a-exitfix-*, arrêt au premier échec ; 8b reste suspendu.

Correction appliquée au seul nouveau test : après le marqueur, la même
échéance totale de 3 secondes couvre l'attente try_wait jusqu'à Some(status),
erreur et timeout font échouer, status.success exigé avant Stopped injecté.
Assertions terminales inchangées. Preuves 8a-exitfix-* : SelfTest 35/79,
Structure 951/951, format et Clippy réussis ; N1 défaut 705/128, common11/1,
CPU12/9, Parakeet26/11, soit 754 réussites, zéro échec, 149 ignorés.
Candidat figé transmis pour l'unique vérification ciblée R-8A-4 autorisée,
sans nouvelle revue générale ; aucune production modifiée.

Le même reviewer ferme R-8A-4 : try_wait fournit le vrai ExitStatus avant
Stopped, erreurs/timeout/statut sont vérifiés dans la même échéance 3 s,
assertions terminales conservées. N1 754/149/0 relue, Structure 951/951,
Clippy terminé. Aucun nouveau cycle général ; tous les constats 8a sont
fermés. Commit local 8a puis extraction 8b désormais possibles selon le plan.
