Avancée : famille 1 extraite ; caractérisation 2a revue CLEAN, N1 verte (691 réussites, 148 ignorés).
Blocage : aucun pour 2a ; extraction 2b et familles 3 à 4 restent à réaliser dans A.
Décision : démarrage du sous-lot A autorisé ; arrêt obligatoire à sa clôture, B et C restent à engager séparément.

# REFACTO-POLL — cartographie et plan accepté

## Autorité, objectif et périmètre

Décision utilisateur dans la session principale du 2 novembre 2026 : créer
`C:/wt/poll`, branche `refacto/poll`, depuis
`9cd15ca624fd0fd1f25441ec11fc4790073d6958`, versionner ce document en premier
commit, exécuter la référence N1 puis commencer A. Le premier commit est
documentaire ; la référence précède tout commit Na/Nb. Aucun push autorisé.

Objectif produit : sécuriser la maintenance des parcours import et microphone
en découpant `ImportRuntime::poll()` sans aucun changement de comportement.
La session principale écrit les documents. L'implémentation Rust est déléguée
à `rust_implementer`, mode IMPLEMENT ; chaque candidat Na/Nb est confié à
`rust_review_coordinator`, avec reviewer indépendant et corrections par
l'implémenteur. Deux cycles revue/correction au maximum par commit ; au-delà,
arrêt et livraison avec réserves à arbitrer.

Chemins autorisés : `crates/whisper-adapters/src/worker_ipc/runtime/poll.rs`,
nouveaux fichiers sous `worker_ipc/runtime/poll/`, déclarations `mod`
nécessaires, `docs/architecture/`. Les tests nouveaux sont sous `poll/`,
dans des fichiers séparés. Aucun autre agent ne modifie `worker_ipc/`
pendant le lot ; vérifier les activités concurrentes pertinentes avant écriture.

Interdits : modifier un test existant, manifeste, Cargo.lock, signature publique
ou autre module de production ; corriger un bug rencontré ; renommer,
simplifier ou factoriser entre handlers. `poll_unified_sessions()` et
`poll_parakeet_worker()` sont hors périmètre. Les handlers similaires restent
séparés ; les branches inatteignables sont conservées et documentées sans test
artificiel. Toute injection exigeant du code de production est proposée
séparément avant modification : Na n'ajoute aucun comportement de production.
Une déclaration de module de tests sous cfg(test) est un raccordement de tests.

Autorisation complémentaire dans la même session : après l'arrêt Structure
sur 870 tests contre 867 entrées, l'utilisateur autorise les ajouts à
`ci/tests-baseline.txt` limités aux nouveaux tests du sous-lot A et la reprise
complète des vérifications 1a. Ni les règles de la garde ni les entrées
existantes ne sont modifiées. Avant cet arrêt, deux échecs Cargo fmt --check
avaient été corrigés et suivis de nouveaux contrôles, contrairement à l'arrêt
obligatoire : cet écart d'exécution sera repris au bilan A. Aucun test candidat
n'avait alors été compilé ou exécuté ; aucun Na/Nb commité.

Reprise autorisée pour le registre : sept nouveaux tests déclarés, Structure
PASS (874 tests/874 entrées), SelfTest PASS, PE PASS, fmt --check PASS. Première
compilation Clippy du candidat : échec E0432, import `whisper_core::BackendKind`
dans `poll/tests.rs:13` ; le type est dans `whisper_core::ipc::BackendKind`.
Arrêt respecté, puis autorisation explicite de corriger cet import et de
reprendre les vérifications complètes reçue dans la session. Les tests
candidats ne sont pas encore qualifiés.

Après correction de l'import, Clippy a passé ; le test nouveau des phases de
progression attendait Ready avant l'avertissement Preparing déjà dû et a
échoué. Arrêt respecté. L'utilisateur autorise ensuite explicitement, pour les
seuls nouveaux tests du sous-lot A, la correction des erreurs de compilation
et des attentes contraires au comportement existant, puis leur revérification
sans arbitrage répété. Cette dérogation de session n'autorise pas un changement
produit, de tests existants ou de garde ; les régressions existantes restent
soumises à arrêt et arbitrage. Revue indépendante et critères inchangés.

## Carte du temps 1 — source de référence

Références ci-dessous relatives à
`crates/whisper-adapters/src/worker_ipc/runtime/poll.rs` sur la base.
`poll()` : lignes 134–1231, 1 098 lignes physiques ; complexité 102 selon le
relevé Clippy versionné `docs/architecture/raw/clippy-complexity-production.json`.
Le nombre physique n'est pas une nouvelle mesure `clippy::too_many_lines`.
La fonction ne contient pas de boucle ; chaque appel s'arrête au premier retour.

| Lignes | Sources / branches | Effets et contrôle |
| --- | --- | --- |
| 137–151 | Sessions unifiées, document, file Parakeet | Cet ordre de priorité ; libération terminale unifiée sur erreur ; retour immédiat. |
| 152–172 | Test micro : événement/absence/erreur ; worker expiré | Erreur : invalider sonde, arrêter capture, retirer test ; retour Err. Expiration : retirer idle_worker. |
| 173–198 | Direct : événement/absence/erreur, terminé ou actif | Poll puis fail en erreur ; récupérer éventuellement le worker avant retrait Direct ; retour immédiat si événement/erreur. |
| 199–215 | Live Parakeet : événement/absence/erreur | Erreur capture transformée en événement avec final automatique, mise en file, retrait session, retour. |
| 216–278 | Compteurs reprise, reset, avertissement, journal dégradé | take des différés ; reset seulement si worker prêt ; diagnostic, indicateur de dégradation et retour prioritaire. |
| 279–316 | Télémétrie, capture, fenêtre différée | Télémétrie avant capture ; observations capture/drain ; compteur d'équité ; attente persistance par return None. |
| 317–357 | Décodage VAD, début fenêtre, Ready synthétique | En erreur : terminer worker, retirer worker/VAD, vider événements, retirer actif ; sinon Ready synthétique unique. |
| 358–440 | Parakeet, Stop différé, receive Turbo | Priorité dans cet ordre ; attente sortie/persistance ; transport vide vers télémétrie ; erreur vers reprise ou nettoyage terminal. |
| 441–493 | Identité, ImportStarted, Failed incomplet | Lier seulement erreur startup entièrement sans identité ; filtrer périmés ; ACK Auto attendu sinon ProtocolMismatch ; sortie immédiate. |
| 494–580 | Ready/Progress/Failed/Stopped diagnostics ; phases | Enregistrer avant validation métier ; ouvrir/observer/fermer phases ; pas de retour propre. |
| 581–607 | Stopped corrélé live/reprise | Différer tant que persistance ou processus non sorti ; return None. |
| 608–635 | LiveMp3Packet | Vérifier identité, append archive ; en erreur arrêt capture/drain/failure/Stop ; sorties immédiates, sinon poursuivre si aucune archive. |
| 636–693 | WindowFinished | Refuser hors live ; ignorer replay ; mapper séquences avec erreurs ; différer si persistance, sinon confirmer. |
| 694–732 | Stopped pending live en reprise | Retirer tentative ; erreur/annulation/choix CPU/démarrage CPU ; retraits d'états propres à chaque branche. |
| 733–760 | Stopped import AutoRetryPending | Cloner requête avant retrait actif/transport ; annulation sinon démarrer CPU ; succès ou erreur immédiate. |
| 761–815 | Stopped live : failure et drain | Failure mémorisée ; gates campagne avant/après drain, résultat enregistré ; erreurs et nettoyages distincts. |
| 816–893 | Stopped live : reprise | Lire durable/confirmé/checksum ; vérifier barrière et couverture ; retirer tentative ; annulation, choix CPU, Stop mémorisé ou reprise CPU. |
| 894–913 | Stopped live : publication | Publier puis retirer live/archive/transport/actif ; erreur ou Published. |
| 914–941 | Segment de préfixe / End prématuré | Pop préfixe ; comparer séquence/plage/texte ; identique consommé sans émission ; mismatch : retirer transport/actif, vider préfixe, Err. |
| 943–977 | GpuSessionAttested | Comparer candidat et identifiants ; succès gpu_attested puis None ; refus via handle_worker_failure et traitement final. |
| 978–1082 | Ready corrélé/backend attendu ou refus | Attestation préalable ; startup_ready ; BeginImport Auto sauf annulation ; marquer worker_ready ; activer live ou demander Stop ; Ready/message CPU ou erreur. |
| 1083–1094 | Progress | Convertir vers ImportEvent::Progress puis traitement final. |
| 1095–1138 | Segment courant | Mapper séquence ; erreur immédiate si impossible ; live : incrémenter persistance et LiveProvisional ; sinon Segment. |
| 1139–1168 | End | Prendre transport avant recherche requête ; conserver éventuellement pending_idle_worker ; retirer actif ; End puis traitement final. |
| 1169–1182 | Stopped normal | Retirer transport/actif/pending_live/live_archive dans cet ordre ; Stopped puis traitement final. |
| 1183–1202 | Failed corrélé, Failed résiduel, autre | handle_worker_failure ou ProtocolMismatch/StaleResponse ; résultat vers traitement final commun. |
| 1203–1230 | Résultat en erreur live / succès / terminal | Erreur live : stop capture, drain, mémoriser failure, Stop transport et None ; succès non terminal : télémétrie due ; Some(result). |

## Invariants et risques

- Trois contrôles distincts : poursuivre au point suivant, sortir immédiatement
  (y compris None), ou passer par le traitement final commun 1203–1229.
  Aucun booléen ambigu ni boucle de drainage nouvelle. Type de retour explicite
  proposé : variantes Next, Return(Option<PollResult>), Finish(PollResult) ; la
  collecte distingue également événement acquis et sortie immédiate.
- 447–453 : liaison startup avant rejet des événements corrélés périmés. Les
  identités partielles suivent 475–493 ; ne pas renforcer ou généraliser ce filtre.
- 494–580 précède les validations métier suivantes, notamment Ready. Conserver
  les effets d'observation même avant une erreur métier.
- 314–316, 372–385, 581–607, 688–692 : barrières persistance/sortie processus
  distinctes ; ordre Segment provisoire, persistance, confirmation à préserver.
- 716, 746, 867, 882, 1017, 1044 : conserver chaque contrôle PublicationGate à
  sa place, sans les fusionner. Une annulation ne doit pas lancer une reprise CPU.
- 429–436, 694–913, 1174–1177 : ordres de nettoyage propres aux branches ; ne
  pas extraire de nettoyage commun ni réordonner les destructions.
- Service `worker_ipc/service/mod.rs:686–799` : Stop, réservation des places
  EVENTS_PER_STAGE, poll_event seulement si place, conversion Err vers Failed,
  émission FIFO. Aucun changement dans ce service. L'équité capture/worker de
  poll 279–312, 389 reste identique.
- Emprunts de self : conserver les durées nécessaires aux lectures puis mutations,
  sans ajouter des clones ou un nouvel état produit pour contourner les emprunts.
- Alternatives apparemment inatteignables : fallback après push/pop Parakeet
  209–212 ; revérification MP3 616–619 ; else unreachable 650–652 ; requête
  absente 753–757 ; Failed résiduel 1192–1196. Les conserver, vérifier leur
  justification, ne pas ajouter de test violant artificiellement les invariants.

## Couverture existante et manque de preuve

Relevé statique, pas une preuve d'exécution ni une couverture instrumentée.
Abréviations de chemins : T = `worker_ipc/tests/reprocessing_and_transport.rs`,
M = `worker_ipc/tests/microphone_and_finalization.rs`,
R = `worker_ipc/tests/resume_and_admission.rs`, tous sous `src/` de la crate.

| Branche / parcours | Test existant (fichier:ligne) | Portée |
| --- | --- | --- |
| GPU attestation positive, Ready, admission, ACK | auto_gpu_ready_is_attested_and_explicitly_admitted (T:807) | Traverse poll ; refus distincts à ajouter. |
| CPU Ready et message repli | auto_without_gpu_starts_cpu_with_normalized_compute_and_same_identity (T:737) | Traverse poll. |
| Échec startup CPU | auto_cpu_startup_failure_ends_without_another_attempt (T:774) | Traverse poll. |
| Sortie avant admission GPU | auto_import_gpu_exit_after_ready_before_begin_retries_once_on_cpu (T:852) | Traverse poll, reprise CPU unique. |
| Failed/EOF import GPU | gpu_import_failed_then_eof_retries_cpu_and_accepts_same_job_ready (T:918) | Traverse poll. |
| Failed/EOF pending live GPU | gpu_live_failed_then_eof_retries_cpu_and_accepts_ready_before_capture_activation (T:966) | Accepte aussi erreur activation capture. |
| Stop empêchant reprise import | stop_latched_after_gpu_failure_prevents_cpu_worker_launch (T:1079) | Traverse poll. |
| Capture/télémétrie/segment live/fenêtre/End | microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end (M:202) | Nominal seulement. |
| Compteurs reprise prioritaires | gpu_choice_keeps_drained_counters_ahead_of_following_events (T:1592) | Appelle poll directement. |
| MP3/Failed périmés | stale_mp3_and_failed_events_have_old_attempt_identity (T:1276) | Helper seulement, pas les effets de poll. |
| Stop/persistance | stopped_worker_waits_until_live_segment_persistence_is_acknowledged (T:1509) | Helper seulement ; pas sortie processus. |
| Mapping replay | replay_windows_map_after_confirmed_prefix_without_duplicate_coverage (T:1523) | Helper seulement. |
| Reprise après interruption | r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication (R:408) | Ignoré sans native-tests, pas une protection N1. |
| Choix CPU manuel | forced_gpu_initialization_failure_waits_for_explicit_cpu_choice (T:1033) | Vérifie un refus avant démarrage faute de preuve GPU, pas le choix CPU après échec. |

Le dernier test reste strictement inchangé. Consigner l'écart nom/contenu pour
un lot ultérieur ; famille 5 ajoute un nouveau test de la vraie branche.

### Branches sans caractérisation directe identifiée

| Famille responsable | Scénarios à couvrir par nouveaux tests avant extraction |
| --- | --- |
| 1 observations | Avertissement/reset préalable quand concerné, dégradation journal unique et sans actif ; diagnostics Ready/Progress/Failed/Stopped ; ouverture/observation/fermeture phases et ordre avant validation. |
| 2 segments/reprise/End | Préfixe identique sans réémission ; différences séquence/plage/texte ; End prématuré ; Segment import/live ; mapping impossible ; transfert worker à End et états restants. |
| 3 attestation/Ready/admission | Candidat différent ; Ready sans attestation ; backend différent ; annulation avant BeginImport ; erreur d'envoi ; ACK inattendu/répété ; activation live en erreur ; filtrage périmé/partiel avant ces effets. |
| 4 MP3/fenêtres | Append réussi/en erreur/sans archive ; erreur arrêt capture/drain/failure/Stop ; fenêtre hors live/replay ; mappings invalides ; segment nul ; confirmation immédiate/différée puis débloquée. |
| 5 Stop pending/import | Retrait tentative en erreur ; annulation ; vrai choix CPU manuel ; succès/échec reprise CPU ; état après échec lancement. |
| 6 Stop live actif | Failure mémorisée ; drain final en erreur ; barrière invalide ; couverture > durable ; retrait tentative en erreur ; annulation/choix CPU/Stop mémorisé ; reprise CPU ; publication succès/échec et ordres de nettoyage. |
| 7 collecte/priorités | Sources simultanées unifiées/document/file ; libération terminale ; test micro erreur ; idle expiré ; Direct terminé avec/sans worker ou erreur ; Parakeet capture en erreur/final automatique ; VAD erreurs et Ready unique ; collecte Parakeet erreur/absence ; Stop différé bloqué par persistance/processus ; transport vide/erreur avec/sans reprise. |
| 8 routage/final commun | Progress ; Stopped normal ; événement inattendu ; erreur live différée ; télémétrie non terminale ; sorties immédiates opposées au traitement final. |

Chaque Na affine cette matrice au niveau des branches de sa famille, cite les
tests nouveaux et fixe résultats, états restants, ordre et absence d'effets.
Ne pas compter un test de helper comme caractérisation de poll. Aucune limite
artificielle de taille à Na. Si une injection produit est nécessaire, arrêt et
proposition séparée ; aucune extraction de cette famille sans caractérisation.

## Séquence acceptée et arrêts

| Sous-lot | Famille | Na puis Nb |
| --- | --- | --- |
| A | 1 | Diagnostics et suivi de progression. |
| A | 2 | Segments, préfixe de reprise, End import. |
| A | 3 | Attestation GPU, Ready, admission Auto. |
| A | 4 | MP3 et confirmations de fenêtres. |
| B | 5 | Stop pending live et reprise import, handlers distincts. |
| B | 6 | Stop live actif : drain, décision reprise, publication ; sous-handlers distincts. |
| C | 7 | Collecte/priorités : micro, Direct, Parakeet, transport, sous-handlers distincts. |
| C | 8 | Routage restant, Stopped/Failed, traitement final ; poll aux seuils. |
| C | 9 | Mesures finales et documentation ; pas d'extraction à inventer pour une étape documentaire. |

Pour 1–8, un commit Na de tests verts sur la logique non encore extraite de
cette famille, puis un commit Nb d'extraction. Les extractions précédentes
restent en place ; « code d'origine » désigne la logique inchangée de la famille
courante. Famille 9 est la clôture documentaire et métrologique du plan initial.
Pas de renommage, simplification ou fusion ; seules liaison paramètres, retours
de contrôle, imports et déclarations de modules nécessaires changent en Nb.
Au plus 800 lignes déplacées par Nb.

A compte huit tâches Na/Nb ; B quatre tâches Na/Nb plus N2 sur autorisation ;
C quatre tâches Na/Nb puis mesures/documentation. Si un sous-lot dépasse dix
tâches ou dix fichiers de rapport, arrêt et arbitrage. Chaque sous-lot se clôt
par un compte rendu figé d'une page sous docs/architecture/, puis arrêt.
Au démarrage présent, seul A est engagé. Aucun commencement B/C automatique.

### Preuves de détection

Familles 2, 3, 5, 6 : au moins une mutation locale non commitée par famille,
sur la logique avant extraction, détectée par au moins un nouveau test Na.
Exemples : ordre de nettoyage observable, sortie de contrôle changée, contrôle
PublicationGate supprimé. Consigner mutation exacte, test, commande, échec
attendu, restauration exacte du candidat, puis test vert et vérifications
requises. Une mutation non détectée impose de renforcer les tests avant Nb.
Ces mutations temporaires explicitement autorisées ne font pas partie du diff
Na ; aucune injection permanente de production n'est autorisée par ce mécanisme.

## Vérifications et critères

Avant le premier Na puis pour chaque commit : garde N1 existante, auto-tests,
racines canoniques, espace libre >= 30 Gio, environnement isolé ; aucune
surveillance ajoutée. TEMP/TMP/LOCALAPPDATA sous racine du sous-lot ; CARGO_HOME
persistant du worktree ; cible Cargo partagée existante autorisée. Si cache
absent, pas de contournement ; pas de téléchargement sans accord précis.

- `cargo fmt --check` et `cargo clippy --workspace --all-targets` par la garde.
- Suite permanente : `cargo test`, `cargo test -p whisper-worker-common`,
  `cargo test -p whisper-worker-cpu`, `cargo test -p whisper-worker-parakeet-cpu`.
  Chaque worker passe le contrôle PE de ses harnais avant exécution.
- Garde mode Structure : PASS. Chaque commande Cargo utilise offline/locked
  lorsque applicable ; garder stdout/stderr/codes et comptes par sélection.
- Référence annoncée : 660 PASS / 147 ignorés / zéro échec ; à confronter aux
  comptes effectivement mesurés sans réinterprétation silencieuse. Aucun test
  perdu, nouveaux tests seulement ; aucun ignoré exécuté en N1.
- Revue indépendante chaque commit ; diff color-moved pour Nb : instructions
  et ordre identiques, changements limités aux raccordements autorisés.
- Au résultat final : poll et chaque handler <= 25 de complexité et <= 100
  lignes Clippy ; mesurer aussi les handlers à chaque extraction. Les anciennes
  parties non encore extraites restent transitoirement au-dessus des seuils.
  Si atteindre un seuil exige de réordonner, arrêt. Aucun seuil contourné via
  une référence ; aucune référence Clippy créée si elle n'existe pas.
- N2 à la fin de B seulement sur autorisation distincte : import CPU long,
  reprise après interruption, repli Auto, direct Turbo CPU ; comparaison à la
  référence native identifiée. Aucun profil GPU sauf demande ; fenêtre Windows
  réservée à l'utilisateur lorsqu'elle est prévue.

Le bilan d'une page de chaque sous-lot indique commits, familles/tests/mutations,
mesures disponibles, résultats N1/N2 réellement exécutés, bugs consignés sans
correction, écarts nom/contenu des tests et limites restantes. Il est figé à
clôture. Petites preuves archivées selon AGENTS ; caches persistants préservés,
aucune suppression hors chemins vérifiés et autorisés. Aucun manifeste de
qualification supplémentaire ni nouvel outillage de campagne.

## Exécution du sous-lot A

- 1a (`69fda611`) : treize tests nouveaux, logique produit inchangée, registre augmenté de
  treize entrées. N1 final : défaut 635/0/127 ignorés ; common 11/0/1 ; CPU
  12/0/9 ; Parakeet CPU 26/0/11, total des sélections 684/0/148. Structure
  880/880, SelfTest 33/64, PE 4/15 et harnais CPU/Parakeet PASS, témoin PE réel
  non exécuté, fmt/Clippy/diff-check PASS. Revue indépendante : R1 (phases et
  reset) et R2 (oracle Progress) fermés ; CLEAN après deux corrections.
  Derniers journaux sous `target/parakeet-tests/refacto-poll-a/scratch/` :
  Clippy `20261102T235053185-766b233103e5449188902f614c613d3a`, défaut
  `20261102T235102315-d342ec8846844352b8d3743ce6267e25`, common
  `20261102T235144993-eb5d40e6ee39438f8383b0e96f178c7d`, CPU
  `20261102T235150231-8752264e85a54ceeaff8b1c3265c215e`, Parakeet CPU
  `20261102T235201185-75c3ae4df476435caf3e0cb11a34dd22`.
- 1b : 141 lignes déplacées vers `poll/observations.rs`, appels aux mêmes
  emplacements et instructions conservées ; revue indépendante CLEAN sans
  correction. Handlers `take_observation_notice` : 58 lignes physiques,
  `observe_worker_event` : 96. Les deux lints locaux Clippy vérifient les bornes
  <= 100 lignes et <= 25 de complexité, sans prétendre une valeur cognitive
  exacte sous le seuil. N1 684/0/148, Structure 880/880, SelfTest et PE PASS,
  témoin réel non exécuté, fmt/Clippy/diff-check PASS. Logs scratch : Clippy
  `20261102T235540986-3c4112d895b54325b7f6c53f07d60c17`, défaut
  `20261102T235720901-8b20988e96944101825952791619b86c`, common
  `20261102T235804580-1c1a1e38ad72469a934c56d580016d69`, CPU
  `20261102T235810766-3afd07a9b8fe4c009d8ea945f65a3078`, Parakeet CPU
  `20261102T235822163-7a43d7303f2e4e5f9b305fe8e0f78ef2`.
- 2a : sept nouveaux tests, mutation de sortie du préfixe identique
  détectée puis restaurée. Une première N1 post-restauration a passé à
  691/0/148. La revue a demandé de borner l'heure de disponibilité du segment
  autour de poll (R2A-1) ; après cette correction, un test existant a échoué
  sur l'absence de compteurs durables (`microphone_and_finalization.rs:351`,
  log `20261103T001338306-cc2788864a7949a795a54f22417b1326`). Arrêt respecté.
  Autorisation explicite reçue : un essai diagnostique des tests adapters,
  puis seulement s'il passe la N1 complète et la reprise de revue 2a ; aucune
  correction du test existant ou du produit autorisée par cet accord.
  Diagnostic adapters PASS 379/0/124 ignorés, log
  `20261103T033805875-eefe89ea84a749818799691aae52943b`. N1 suivante PASS
  691/0/148, puis revue CLEAN, R2A-1 fermé après une correction. Échec micro
  non reproduit lors du diagnostic et de la N1, cause inconnue ; aucun
  contournement. Logs finaux scratch : Clippy
  `20261103T033933636-c6900a60bdee4979b2a157f8f087fb10`, défaut
  `20261103T033944593-2eb67fcbe3464e0d989fc6bef9fb12ec`, common
  `20261103T034010047-28231f72f8724ce19067e0bbabf44b7f`, CPU
  `20261103T034017059-e753d61a12814d3a95e70574d5c6644a`, Parakeet CPU
  `20261103T034030601-aafa491ce12044e58da563db4b60fb00`.
  Mutation F2 : `return None` du préfixe égal remplacé temporairement par
  `Some(Err(...))` ; détecteur
  `poll_consumes_matching_resume_segment_without_emitting_or_consuming_next_event`
  vert avant (log `20261103T000540434-c39db7324b3e4ac0b9456809680105d5`),
  rouge pendant (log `20261103T000557411-f96f2fd2dbf941a59419a9ebe727ef64`),
  puis vert après restauration exacte ; aucun diff produit résiduel.
