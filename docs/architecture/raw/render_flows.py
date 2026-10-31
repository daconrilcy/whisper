"""Flux statiques : références recalculées sur les fonctions présentes."""
from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[3]; DOC=ROOT/'docs/architecture'; RAW=DOC/'raw'; OUT=DOC/'flows'
F=json.loads((RAW/'files.json').read_text(encoding='utf8')); N=json.loads((RAW/'functions.json').read_text(encoding='utf8'))
def ref(p,a,b=None): return '`crates/'+p+':'+str(a)+('-'+str(b) if b else '')+'`'
def fn(p,n):
 f=next(x for x in N if x['path']=='crates/'+p and x['name']==n and not x['test']); return ref(p,f['start'],f['end'])
def src(p):
 f=next(x for x in F if x['path']=='crates/'+p); return ref(p,1,f['total'])
def refs(*pairs): return ' ; '.join(fn(p,n) for p,n in pairs)
U='whisper-desktop/src/ui/model/controller/actions.rs'; Q='whisper-desktop/src/ui/model/view/queue.rs'; C='whisper-core/src/application/runtime/'; W='whisper-adapters/src/worker_ipc/'; A='whisper-adapters/src/archive/'; S='whisper-adapters/src/source_binding.rs'; P='whisper-adapters/src/parakeet_import.rs'
SERVICE='Les commandes sont émises depuis le processus desktop. AsyncImportIo crée un thread de service principal et un thread de service de finalisation automatique ; leurs canaux effets/arrêts/événements sont bornés. '+fn(W+'service/mod.rs','start_inner')+'.'
IPC='ChildWorkerTransport lance un processus enfant et trois threads pour stderr, stdin et stdout. '+fn(W+'ipc/mod.rs','spawn')+'. Le worker Turbo démarre un thread lecteur stdin puis exécute les sessions sur sa boucle principale. '+fn('whisper-worker-cpu/src/startup.rs','run')+' ; '+fn('whisper-worker-gpu/src/startup.rs','run')+'.'
LIMIT='[Non vérifié] Lecture statique seulement : aucun processus produit, microphone, modèle, GPU ou fenêtre lancé. L’entrée dans les moteurs natifs constitue une frontière : leurs appels internes et threads ne sont pas suivis.'
def render(name,title,entry,steps,diagram,threads,limit):
 text='Avancée : parcours principal statique relevé et sourcé.\nBlocage : exécution et branches détaillées signalées [Non vérifié].\nDécision : les points de rupture restent explicites.\n\n# '+title+'\n\n## Point d’entrée\n\n'+entry+'\n\n## Appels, données et branches\n\n| Étape | Parcours constaté | Sources |\n| --- | --- | --- |\n'
 text+='\n'.join('| '+i+' | '+d+' | '+s+' |' for i,d,s in steps)+'\n\n## Processus et threads\n\n'+threads+'\n\n## Séquence Mermaid\n\nLes identifiants des messages renvoient aux étapes sourcées ci-dessus. Les échanges entre processus sont des messages IPC. Le diagramme représente le parcours décrit, pas tous les cas d’échec.\n\n```mermaid\n'+diagram.strip()+'\n```\n\n## Points de rupture et limites\n\n'+LIMIT+'\n\n'+limit+'\n'
 (OUT/(name+'.md')).write_text(text,encoding='utf8')
render('import','Import de fichier',
 'L’action UI start_import construit EnqueueImport ; l’action Traiter déclenche ProcessQueued séparément. '+fn(U,'start_import')+' ; '+fn(Q,'queue_ui')+'.',[
 ('I1','UI → contrôleur dispatch → garde DesktopAdmissionApplication → ImportApplication::dispatch : vérification modèle/calcul puis émission Enqueue.',refs((U,'start_import'),('whisper-desktop/src/ui/model/controller.rs','dispatch'),('whisper-desktop/src/root.rs','dispatch'),(C+'commands.rs','dispatch'),(C+'queue.rs','handle_enqueue_command'))),
 ('I2','AsyncImportIo::submit → canaux → service_loop → ServiceRuntime::apply_effect → ImportRuntime::effect.',refs((W+'service/mod.rs','submit'),(W+'service/mod.rs','service_loop'),(W+'runtime/support.rs','apply_effect'),(W+'runtime/effects.rs','effect'))),
 ('I3','enqueue vérifie le modèle, acquiert le binding source, réserve la publication et persiste QueueStore::enqueue ; une branche demande un choix de doublon.',refs((W+'runtime/queue.rs','enqueue'),(W+'runtime/source_binding.rs','ensure_source_binding'),(S,'acquire_with_hook'),('whisper-adapters/src/queue_store.rs','enqueue'))),
 ('I4','Traiter → ProcessQueued → validation FIFO et identité → préparation ArchiveStore → statut Running et événement Prepared. Le cœur transforme Prepared en StartWorker.',refs((Q,'queue_ui'),(C+'import.rs','handle_resume_import_command'),(W+'runtime/queue.rs','process_queued'),(A+'import.rs','prepare_with_source_binding'),(C+'import.rs','handle_import_preparation'))),
 ('I5','Turbo : start_worker_effect choisit la route, start_worker_with_candidate_override sélectionne ou réutilise le processus puis envoie StartImport/StartGpuImport.',refs((W+'runtime/import.rs','start_worker_effect'),(W+'runtime/import/worker_start.rs','start_worker_with_candidate_override'),(W+'ipc/mod.rs','spawn'))),
 ('I6','Worker : startup → worker::run → run_session → run_import ; vérification SHA-256, moteur en cache, decoder partagé et transcribe natif ; émission des segments puis End.',refs(('whisper-worker-cpu/src/worker.rs','run'),('whisper-worker-cpu/src/session/mod.rs','run_session'),('whisper-worker-cpu/src/session/import.rs','run_import'),('whisper-worker-common/src/decoder.rs','decode_wav_windows'),('whisper-worker-cpu/src/native_engine.rs','transcribe'),('whisper-worker-gpu/src/session/import.rs','run_import'))),
 ('I7','Retour : transport → poll du runtime → canaux événements → poll_once du cœur → persistance de segments et publication quand les acquittements et End concordent.',refs((W+'runtime/poll.rs','poll'),(C+'poll.rs','poll_once'),(C+'import.rs','handle_import_persistence'),(C+'import.rs','maybe_publish'))),
 ('I8','Parakeet historique : start_parakeet_worker crée ParakeetProcess et transmet ses demandes ; la boucle poll_parakeet_worker gère résultats et fenêtres VAD.',refs((W+'runtime/import/worker_start.rs','start_parakeet_worker'),(P,'spawn_profile_with_log_root'),(W+'runtime/poll/parakeet.rs','poll_parakeet_worker'))),
 ('I9','Parakeet unifié : start_controlled_unified_import décode la source en parties et construit finaliseur/document ; poll_unified_sessions ordonnance les fenêtres et l’exécuteur appelle le worker natif via ParakeetProcess.',refs((W+'runtime/import.rs','start_controlled_unified_import'),('whisper-adapters/src/parakeet_audio.rs','decode_file_to_parts'),(W+'runtime/poll_unified.rs','poll_unified_sessions'),(W+'parakeet_unified/executor.rs','start_instance'),(W+'parakeet_unified/native.rs','transcribe_scheduled'))),
 ],'''sequenceDiagram
 participant UI as desktop UI
 participant Core as core application
 participant Service as adapters service
 participant Disk as source et archive
 participant Worker as worker enfant
 UI->>Core: I1 EnqueueImport
 Core->>Service: I2 Enqueue
 Service->>Disk: I3 binding et file persistée
 UI->>Core: I4 ProcessQueued explicite
 Core->>Service: I4 ProcessQueued
 Service->>Disk: I4 préparation archive
 Service-->>Core: I4 Prepared
 Core->>Service: I4 StartWorker
 alt Turbo
 Service->>Worker: I5 StartImport ou StartGpuImport
 Worker-->>Service: I6 segments et End
 else Parakeet historique
 Service->>Worker: I8 Transcribe
 Worker-->>Service: I8 Result
 else Parakeet unifié
 Service->>Worker: I9 fenêtres ordonnancées
 Worker-->>Service: I9 résultats corrélés
 end
 Service-->>Core: I7 événements
 Core->>Service: I7 PersistSegment puis Publish
 Service->>Disk: I7 journal et publication
 Core-->>UI: I7 AppView
 ''',SERVICE+'\n\n'+IPC+'\n\nLe chemin unifié utilise un thread d’inférence supplémentaire. '+fn(W+'parakeet_unified/executor.rs','start_instance')+'.',
 '[Non vérifié] Le flux se rompt à CpuEngine/GpuEngine::transcribe et au moteur Parakeet natif. Les branches complètes de VAD, doublon, annulation et reprise ne sont pas toutes déroulées. La persistance finale est détaillée dans [publication](publication.md).')
render('queue','File d’attente persistée',
 'Ajout : start_import → EnqueueImport. Consommation : bouton Traiter → ProcessQueued. '+fn(U,'start_import')+' ; '+fn(Q,'queue_ui')+'.',[
 ('Q1','EnqueueImport passe par l’admission du modèle puis le cœur soumet Enqueue sans démarrage worker immédiat.',refs((C+'commands.rs','dispatch'),(C+'queue.rs','handle_enqueue_command'))),
 ('Q2','Le service acquiert la source, contrôle doublons/réservations puis ajoute la requête durable.',refs((W+'runtime/queue.rs','enqueue'),('whisper-adapters/src/queue_store.rs','enqueue'))),
 ('Q3','QueueStore écrit les opérations du journal puis les rejoue dans son index ; la file est renvoyée par ImportEvent::Queue.',refs(('whisper-adapters/src/queue_store.rs','append'),('whisper-adapters/src/queue_store.rs','replay'),(W+'runtime/queue.rs','enqueue'))),
 ('Q4','Le clic Traiter est refusé si un travail est actif côté cœur ; le service vérifie que l’identité demandée est la première de la file et prépare son archive.',refs((C+'import.rs','handle_resume_import_command'),(W+'runtime/queue.rs','process_queued'))),
 ('Q5','La préparation passe l’entrée à Running et émet Prepared ; le cœur demande alors StartWorker.',refs((W+'runtime/queue.rs','process_queued'),(C+'import.rs','handle_import_preparation'))),
 ('Q6','Retirer émet RemoveQueued ; la publication réussie retire aussi l’entrée et renvoie la file actualisée.',refs((Q,'queue_ui'),(W+'runtime/queue.rs','remove_queued'),(W+'runtime/import.rs','publish_inner'))),
 ],'''sequenceDiagram
 participant UI as desktop
 participant Core as core
 participant Service as adapters runtime
 participant Queue as QueueStore disque
 UI->>Core: Q1 EnqueueImport
 Core->>Service: Q1 Enqueue
 Service->>Queue: Q2 enqueue
 Queue->>Queue: Q3 append et synchronisation
 Service-->>Core: Q3 Queue et admission
 UI->>Core: Q4 ProcessQueued
 Core->>Service: Q4 ProcessQueued
 Service->>Queue: Q4 contrôle FIFO et identité
 Service->>Queue: Q5 Running
 Service-->>Core: Q5 Prepared
 Core->>Service: Q5 StartWorker
 opt retrait explicite ou publication
 Service->>Queue: Q6 Remove
 Service-->>Core: Q6 Queue actualisée
 end
 ''',SERVICE+'\n\nLe journal est manipulé dans le runtime de service ; le processus worker rejoint le [flux import](import.md) après Prepared. '+fn(W+'runtime/queue.rs','process_queued')+'.',
 '[Non vérifié] Les séquences complètes des conflits de doublon et des erreurs de journal ne sont pas déroulées. La voie de finalisation automatique possède ses propres canaux/files ; elle ne doit pas être confondue avec cette file utilisateur. '+fn(W+'service/mod.rs','start_inner')+' ; '+fn(W+'service/mod.rs','submit')+'.')
render('recovery','Reprise après interruption',
 'queue_ui affiche Reprendre… puis Confirmer la reprise pour les entrées Interrupted ; ResumeInterrupted ne part qu’après confirmation. '+fn(Q,'queue_ui')+'.',[
 ('R1','Le scan recovery ouvre file et archives, retire les entrées terminées et transforme Running en Interrupted, Queued en AwaitingChoice ; il ne lance pas le worker.',refs(('whisper-adapters/src/recovery.rs','scan'),(W+'runtime/queue.rs','scan_queue'))),
 ('R2','Le cœur reçoit ResumeInterrupted, construit la génération suivante et soumet l’effet avec l’identité précédente.',refs((Q,'queue_ui'),(C+'import.rs','handle_resume_import_command'))),
 ('R3','Le runtime valide modèle/source/file et peut exiger une reconfirmation de publication ou un choix de doublon.',refs((W+'runtime/recovery.rs','resume_interrupted'),('whisper-adapters/src/queue_store.rs','validate_resume_with_catalog'))),
 ('R4','Le binding source est transféré puis ArchiveStore::resume_from_with_qualification vérifie l’archive/préfixe et prépare la nouvelle génération.',refs((W+'runtime/source_binding.rs','transfer_resume_source_binding'),(A+'recovery.rs','resume_from_with_qualification'),(A+'journal.rs','validate_segment_journal'))),
 ('R5','La file de reprise est mise à jour ; le runtime conserve resume_prefix et le checkpoint unifié éventuel puis émet ResumeCheckpoint et Prepared.',refs((W+'runtime/recovery.rs','resume_interrupted'),('whisper-adapters/src/queue_store.rs','resume_interrupted'),(C+'import.rs','handle_import_preparation'))),
 ('R6','Turbo suit StartWorker et le contrôle du préfixe dans poll ; Parakeet unifié restaure document/checkpoint avant l’ordonnancement.',refs((W+'runtime/poll.rs','poll'),(W+'runtime/import.rs','start_controlled_unified_import'),(W+'parakeet_unified/mod.rs','restore_unified_document_prefix'))),
 ('R7','Le live a un scan distinct des préfixes confirmés et de la publication ; les reprises live et retraits de remplacement n’utilisent pas tous ResumeInterrupted.',refs((A+'recovery/live.rs','scan_live_recoveries'),(A+'live.rs','resolve_live_publication'),(A+'cleanup.rs','resume_replacement_cleanup'))),
 ],'''sequenceDiagram
 participant UI as desktop
 participant Core as core
 participant Runtime as adapters
 participant Disk as file sources et archives
 Runtime->>Disk: R1 scan et statuts récupérables
 Runtime-->>Core: R1 Queue
 UI->>Core: R2 confirmation ResumeInterrupted
 Core->>Runtime: R2 génération suivante et previous
 Runtime->>Disk: R3 valider identité et publication
 Runtime->>Disk: R4 binding et préfixe vérifié
 Runtime->>Disk: R5 nouvelle génération dans la file
 Runtime-->>Core: R5 ResumeCheckpoint puis Prepared
 Core->>Runtime: R6 StartWorker
 alt Turbo
 Runtime->>Runtime: R6 vérifier le préfixe rejoué
 else Parakeet unifié
 Runtime->>Disk: R6 restaurer document et checkpoint
 end
 ''',SERVICE+'\n\nAprès Prepared, le transport relance ou réutilise un worker selon le flux import. '+fn(W+'runtime/import/worker_start.rs','start_worker_with_candidate_override')+'.',
 '[Non vérifié] La reprise après coupure électrique, toutes les corruptions et tous les états de publication partielle ne sont pas exécutés. Rupture au contrôle natif des segments recalculés ; le schéma R2–R6 décrit l’import, les autres mécanismes sont seulement localisés en R7.')
render('direct','Transcription microphone en direct',
 'start_live construit LiveRequest avec source, gain, modèle et calcul. Le candidat Parakeet unifié utilise StartUnifiedLiveConfigured ; les autres choix passent par StartDirectLiveConfigured. '+fn(U,'start_live')+'.',[
 ('D1','Le contrôleur vérifie la source microphone et construit la commande ; le cœur admet le modèle puis émet StartDirectLive ou StartUnifiedLive.',refs((U,'microphone_for_live'),(U,'start_live'),(C+'commands.rs','dispatch'),(C+'live.rs','start_live_command'),(C+'unified.rs','start_unified_live_command'))),
 ('D2','Le service route StartDirectLive vers Turbo DirectLive ou ParakeetLive selon le modèle ; StartUnifiedLive rejoint la capture unifiée et ses conditions d’admission.',refs((W+'runtime/effects.rs','effect'),(W+'runtime/live.rs','start_direct_live'),(W+'runtime/live.rs','start_unified_live'))),
 ('D3','Turbo DirectLive démarre/réutilise le worker, ouvre capture et convertisseur PCM, démarre le probe et crée WavOutput. La capture n’attend pas l’événement Ready du worker pour être possédée par la session.',refs(('whisper-adapters/src/direct_live.rs','start'),('whisper-adapters/src/capture.rs','start_direct_with_gain'),('whisper-adapters/src/direct_live/wav.rs','create'))),
 ('D4','DirectLive::poll draine la capture vers le WAV et le tampon ; send_next attend worker_ready puis envoie une fenêtre LiveWindow ou Stop lorsque la session est drainée.',refs(('whisper-adapters/src/direct_live.rs','poll'),('whisper-adapters/src/direct_live/capture.rs','drain_capture'),('whisper-adapters/src/direct_live.rs','send_next'))),
 ('D5','Le worker transcrit les fenêtres et émet les segments/WindowFinished ; handle_worker confirme la fenêtre, expose DirectConfirmed puis écrit les exports à l’arrêt.',refs(('whisper-worker-cpu/src/session/live.rs','run_live_service'),('whisper-worker-gpu/src/session/live.rs','run_live_service'),('whisper-adapters/src/direct_live/worker.rs','handle_worker'),('whisper-adapters/src/direct_live/outputs.rs','export_segments'))),
 ('D6','Parakeet historique ouvre capture/parties audio et démarre un thread d’inférence ; les fenêtres sont soumises, les mots corrélés puis confirmés via ConfirmationTracker.',refs(('whisper-adapters/src/parakeet_live/lifecycle.rs','start_with_capture'),('whisper-adapters/src/parakeet_live/windows.rs','schedule_window'),('whisper-adapters/src/parakeet_live/inference.rs','inference_thread'),('whisper-adapters/src/parakeet_live/events.rs','poll'),('whisper-adapters/src/parakeet_confirmation.rs','ingest_observed'))),
 ('D7','Unifié : la capture alimente des parties PCM et un document ; poll_unified_sessions ordonnance preview/final, l’exécuteur traite les fenêtres et engage leurs résultats dans le document.',refs((W+'runtime/live.rs','start_unified_capture'),('whisper-adapters/src/parakeet_live/capture.rs','pump'),(W+'runtime/poll_unified.rs','poll_unified_sessions'),('whisper-adapters/src/parakeet_pipeline.rs','commit_prepared'))),
 ('D8','L’arrêt draine capture/écriture. Parakeet historique peut déclencher une transcription finale automatique sur une voie de service distincte ; le cœur suit séparément son statut et ses segments.',refs(('whisper-adapters/src/parakeet_live/lifecycle.rs','stop'),(C+'live.rs','enqueue_capture_final'),(C+'automatic_final.rs','start_next_automatic_final'),(W+'service/mod.rs','submit'),(W+'service/automatic_final.rs','publish_automatic_final_exports'))),
 ],'''sequenceDiagram
 participant UI as desktop
 participant Core as core application
 participant Service as adapters service
 participant Capture as capture et disque
 participant Infer as exécuteur ou worker
 UI->>Core: D1 démarrer LiveRequest
 Core->>Service: D2 effet live choisi
 Service->>Capture: D3 ou D6 ou D7 ouvrir capture et sortie audio
 alt Turbo direct
 Service->>Infer: D3 démarrer worker
 loop capture et fenêtres
 Capture-->>Service: D4 blocs PCM
 Service->>Capture: D4 append WAV
 Service->>Infer: D4 LiveWindow après Ready
 Infer-->>Service: D5 segments et WindowFinished
 Service-->>Core: D5 DirectConfirmed
 end
 else Parakeet historique
 Service->>Infer: D6 fenêtres sur thread inférence
 Infer-->>Service: D6 hypothèses de mots
 Service-->>Core: D6 blocs confirmés
 else Parakeet unifié
 Capture->>Capture: D7 parties PCM durables
 Service->>Infer: D7 preview ou final ordonnancé
 Infer-->>Service: D7 résultat corrélé
 Service->>Capture: D7 document et couverture
 end
 UI->>Core: D8 arrêt
 Core->>Service: D8 effet arrêt
 Service->>Capture: D8 drain et fermeture
 opt finalisation automatique historique
 Core->>Service: D8 import final sur voie séparée
 end
 ''',SERVICE+'\n\n'+IPC+'\n\nParakeet historique démarre son thread d’inférence ; les parties audio ont un thread écrivain. '+fn('whisper-adapters/src/parakeet_live/lifecycle.rs','start_with_capture')+' ; '+fn('whisper-adapters/src/parakeet_audio.rs','start_with_limit')+'. L’unifié possède son exécuteur dédié. '+fn(W+'parakeet_unified/executor.rs','start_instance')+'. Le probe microphone possède aussi un thread. '+fn('whisper-adapters/src/microphone_probe.rs','start')+'.',
 '[Non vérifié] Latence, continuité réelle et absence de perte non mesurées. La branche historique StartLive existe aussi dans '+fn(W+'runtime/live.rs','start_legacy_live')+' ; son déroulé détaillé se rompt ici. Les étapes D3–D5 décrivent Turbo ; elles ne sont pas attribuées aux sorties Parakeet.')
render('publication','Publication sur disque',
 'Pour l’import, les événements Persisted et End alimentent maybe_publish ; le cœur ne soumet Publish que lorsque les segments attendus sont acquittés. '+fn(C+'import.rs','handle_import_persistence')+' ; '+fn(C+'import.rs','maybe_publish')+'.',[
 ('P1','PersistSegment passe du cœur au runtime puis au journal ArchiveStore ; Persisted est renvoyé après la persistance.',refs((C+'import.rs','accept_segment'),(W+'runtime/import.rs','persist_segment'),(A+'journal.rs','persist_segment'))),
 ('P2','End et les acquittements déclenchent Publish ; le runtime sélectionne l’archive de la session et sa PublicationGate.',refs((C+'import.rs','maybe_publish'),(W+'runtime/import.rs','publish'))),
 ('P3','prepare_publish vérifie le nombre de segments et la source segmentée éventuelle, écrit transcript.txt/transcript.srt, crée et relit manifest.json et prépare les exports.',refs((A+'publication.rs','prepare_publish'),(A+'storage.rs','write_replace_streamed'),(A+'storage.rs','write_replace_synced'))),
 ('P4','PublicationGate::commit arbitre l’engagement ; commit_publish installe les exports, écrit CURRENT et applique les intentions de publication.',refs((W+'runtime/import.rs','publish_inner'),(W+'publication/mod.rs','commit'),(A+'publication.rs','commit_publish'),(A+'publication/exports.rs','install_publication_exports_with'))),
 ('P5','Le runtime retire l’entrée de file, relit l’archive deux fois et vérifie la génération complète, libère le binding source puis émet Published.',refs((W+'runtime/import.rs','publish_inner'),('whisper-adapters/src/queue_store.rs','remove'),(S,'retire_for_identity'))),
 ('P6','Les cartes import/unifiées sont actualisées depuis Published et les artefacts ; le mécanisme de retrait des traitements remplacés est distinct de la transaction principale.',refs((C+'import.rs','handle_import_terminal'),(C+'unified.rs','handle_unified_artifacts'),(A+'cleanup.rs','replacement_cleanups'),(A+'cleanup/execution.rs','run_cleanup_plan_locked'))),
 ('P7','Sorties live distinctes : DirectLive écrit TXT/SRT à côté du WAV ; Parakeet historique possède ses exports et son JSON compagnon ; LiveArchive publie MP3/texte/SRT/manifestes selon un autre chemin.',refs(('whisper-adapters/src/direct_live/outputs.rs','export_segments'),('whisper-adapters/src/direct_live/outputs.rs','export_parakeet_outputs'),(A+'live.rs','publish'))),
 ],'''sequenceDiagram
 participant Core as core
 participant Runtime as adapters runtime
 participant Archive as ArchiveStore
 participant Gate as PublicationGate
 participant Disk as disque
 Core->>Runtime: P1 PersistSegment
 Runtime->>Archive: P1 persist_segment
 Archive->>Disk: P1 journal synchronisé
 Runtime-->>Core: P1 Persisted
 Core->>Runtime: P2 Publish après End et acquittements
 Runtime->>Archive: P3 prepare_publish
 Archive->>Disk: P3 TXT SRT manifeste et staging
 Runtime->>Gate: P4 commit
 Gate->>Archive: P4 commit_publish
 Archive->>Disk: P4 exports puis CURRENT et index
 Runtime->>Disk: P5 retrait file et scans de vérification
 Runtime-->>Core: P5 Published
 Core->>Core: P6 état et artefacts présentés
 ''',SERVICE+'\n\nLa publication import s’exécute dans le runtime de service et ses opérations disque ; le worker a déjà émis ses segments/End. '+fn(W+'runtime/import.rs','publish_inner')+'. La finalisation automatique dispose de son propre thread et installe les exports via un staging dédié. '+fn(W+'service/mod.rs','start_inner')+' ; '+fn(W+'service/automatic_final.rs','publish_automatic_staged_pair')+'.',
 '[Non vérifié] L’ordre cité est celui des appels Rust ; résistance réelle aux coupures, atomicité du système de fichiers, comportement des liens physiques et cas de rollback partiel non testés. Le diagramme couvre P1–P6 import, pas les transactions live P7.')
render('compute','Choix CPU, GPU et Auto',
 'Les contrôles UI présentent les options calcul/GPU ; dispatch détermine la commande et transmet le candidat. '+fn('whisper-desktop/src/ui/model/view/settings.rs','compute_options_ui')+' ; '+fn('whisper-desktop/src/ui/model/controller.rs','dispatch')+'.',[
 ('G1','Au démarrage compose lance un probe CUDA et retourne initialement Cpu avec message de vérification ; le contrôleur traite ensuite le résultat et les préférences reçues.',refs(('whisper-desktop/src/root.rs','compose'),('whisper-desktop/src/ui/model/controller.rs','poll_gpu_probe'),('whisper-desktop/src/ui/presentation/selection.rs','apply_startup_preference'))),
 ('G2','Le desktop résout les préférences et l’identité du GPU, puis transmet SetGpuCandidate avant la commande ; l’admission contrôle modèle, calcul et langue.',refs(('whisper-desktop/src/ui/model/controller.rs','dispatch'),('whisper-desktop/src/root.rs','dispatch'),('whisper-core/src/gpu_selection.rs','resolve'),('whisper-core/src/model_selection_app.rs','admit_import'))),
 ('G3','Turbo WorkerPaths::select s’appuie sur la présence du worker GPU et ComputePolicy::initial. CPU est fixe ; Auto tente GPU si présent, sinon CPU ; GPU manuel absent est une erreur.',refs(('whisper-adapters/src/supervisor.rs','select'),('whisper-core/src/compute_policy.rs','initial'))),
 ('G4','Le runtime lance/réutilise le worker et envoie StartImport/StartGpuImport ; le worker GPU charge CUDA, vérifie son identité et émet les événements d’attestation.',refs((W+'runtime/import/worker_start.rs','start_worker_with_candidate_override'),('whisper-worker-gpu/src/native_engine.rs','load'),('whisper-worker-gpu/src/gpu_attestation.rs','attest_native_identity'),('whisper-worker-gpu/src/session/import.rs','run_import'))),
 ('G5','Les erreurs de lancement/commande CUDA sous Auto peuvent relancer CPU ; les échecs worker sont examinés selon startup_ready et le mode demandé. La politique distingue explicitement audio commencé et échec avant traitement.',refs((W+'runtime/import/worker_start.rs','start_worker_with_candidate_override'),(W+'runtime/import/worker_start.rs','handle_worker_failure'),('whisper-core/src/compute_policy.rs','after_failure'))),
 ('G6','Turbo direct conserve la préférence Auto séparément du backend effectif ; il vérifie la preuve GPU et peut replier avant ready.',refs(('whisper-adapters/src/direct_live.rs','start'),('whisper-adapters/src/direct_live/worker.rs','should_retry_auto_gpu_before_ready'),('whisper-adapters/src/direct_live/worker.rs','retry_cpu'))),
 ('G7','Parakeet unifié résout les routes personnelles, modèle/langue et candidat courant ; Auto peut replier sur CPU lors des validations ou de l’initialisation avant disponibilité/appel en vol.',refs((W+'backends/native.rs','native_unified_route'),(W+'backends/native.rs','can_retry_native_auto_on_cpu'),(W+'runtime/poll_unified.rs','retry_native_gpu_startup_on_cpu'))),
 ],'''sequenceDiagram
 participant UI as desktop
 participant Core as core politique et admission
 participant Runtime as adapters
 participant GPU as worker GPU
 participant CPU as worker CPU
 UI->>Runtime: G1 probe CUDA au démarrage
 Runtime-->>UI: G1 résultat du probe
 UI->>Core: G2 modèle calcul langue et candidat
 Core->>Runtime: G2 commande admise et candidat
 Runtime->>Core: G3 ComputePolicy initial
 alt CPU ou Auto sans worker GPU
 Runtime->>CPU: G3 démarrer CPU
 else GPU disponible
 Runtime->>GPU: G4 démarrer et attester
 GPU-->>Runtime: G4 Ready ou échec
 opt Auto et échec avant traitement
 Runtime->>CPU: G5 tentative CPU
 end
 end
 Runtime-->>Core: G4 ou G5 moteur effectif et cause
 Core-->>UI: G4 ou G5 état visible
 Note over Runtime: G6 live direct et G7 unifié ont leurs contrôles propres
 ''',SERVICE+'\n\n'+IPC+'\n\nLes probes/catalogues ont leurs threads de collecte dans le processus desktop. '+fn('whisper-desktop/src/root.rs','compose')+' ; '+fn('whisper-adapters/src/gpu_inventory.rs','start')+'.',
 '[Non vérifié] Aucun GPU compatible ou gain de performance attesté par cette lecture. La présence du fichier worker en G3 n’est pas présentée comme une preuve de compatibilité matérielle. Le détail complet de toutes les erreurs d’initialisation et des sessions simultanées Parakeet n’est pas déroulé.')
render('startup','Démarrage et composition',
 'Le binaire main appelle root::compose puis whisper_desktop::run. '+fn('whisper-desktop/src/main.rs','main')+'.',[
 ('S1','compose choisit les racines modèle/settings/workers selon le profil, puis ouvre les canaux catalogue/admission/préférences.',refs(('whisper-desktop/src/root.rs','compose'),('whisper-desktop/src/root.rs','personal_worker_paths'))),
 ('S2','Un thread inspecte le catalogue et recontrôle les ressources ; un autre prend en charge les préférences ; le probe GPU démarre en tâche de fond.',refs(('whisper-desktop/src/root.rs','compose'),('whisper-adapters/src/model_catalog.rs','inspect'),(W+'backends/mod.rs','probe_cuda_worker'))),
 ('S3','La route unifiée native est activée par personal-install et WHISPER_PARAKEET_UNIFIED_NATIVE_CANDIDATE=1 ; compose construit AsyncImportIo et la garde Application.',refs(('whisper-desktop/src/root.rs','compose'),('whisper-desktop/src/root.rs','guarded_application'))),
 ('S4','run ouvre eframe::run_native/Glow et construit DesktopApp avec chemins, microphones, probe, catalogue et préférences.',refs(('whisper-desktop/src/lib.rs','run'),('whisper-desktop/src/ui/model.rs','with_microphones'))),
 ('S5','Chaque frame prépare l’état, draine les entrées et rafraîchit Application ; les panneaux UI affichent les états/notifications et les jalons de rendu sont journalisés.',refs(('whisper-desktop/src/ui/model/view.rs','show_ui'),('whisper-desktop/src/ui/model/controller.rs','prepare_frame'),('whisper-desktop/src/ui/model/controller.rs','poll_frame_inputs'),('whisper-desktop/src/ui/model/controller/observability.rs','record_first_render'))),
 ],'''sequenceDiagram
 participant Main as desktop main
 participant Root as composition
 participant Bg as catalogue préférences et probe
 participant Service as adapters services
 participant UI as eframe et DesktopApp
 Main->>Root: S1 compose
 Root->>Bg: S2 lancer collecte et préférences
 Root->>Service: S3 AsyncImportIo et garde
 Root-->>Main: S3 DesktopComposition
 Main->>UI: S4 run
 loop frames
 Bg-->>UI: S5 résultats asynchrones
 UI->>UI: S5 prepare_frame et panneaux
 UI->>Service: S5 Refresh via Application
 Service-->>UI: S5 événements via le cœur
 end
 ''',SERVICE+'\n\nLes créations de threads catalogue, préférences et probe sont dans compose. '+fn('whisper-desktop/src/root.rs','compose')+'. Le journal de réactivité utilise un thread écrivain séparé. '+fn('whisper-desktop/src/reactivity_log.rs','start')+'.',
 '[Non vérifié] Délai avant première frame, blocages du pilote, initialisation du toolkit et performances de démarrage non mesurés. La frontière de cette lecture est eframe::run_native et les retours asynchrones des ressources natives.')
print('Sept fiches de flux produites, avec sources calculées sur les fonctions actuelles.')
