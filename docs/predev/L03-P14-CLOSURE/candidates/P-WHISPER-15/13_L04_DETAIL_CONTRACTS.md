# Contrats détaillés L04 — P-WHISPER-14

## Autorité, lignée et statut

Base PLANS immuable P-WHISPER-13, digest `fbd7b0d2033cbdda8f362921539fdf3349e1a6c6a976ad1ebf82e4840cc203ad`. Parent DESIGN immuable D-WHISPER-19, digest exact `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. Le registre actif reste P10 jusqu'à une revue indépendante exacte P14 CLEAN et une promotion canonique distincte. Ces réponses n'autorisent pas le code L04/L05. Tous les essais décrits sont NOT RUN.

Contribution source : `sources/architecture/ARCH-Q04-Q06-Q09-v2.md`, apport rédigé par `/root/q04_q06_q09_arch` (`rust_architect`), conservé fidèlement dans `transports/T-WHISPER-Q04-Q06-Q09-ANSWER-02/`, manifest digest `88282258694c7414dd9e4f6ff55250b717b6977b6e29dcc06384c1a5627fc7f8` et source hash `2e8a58d22731124ee54f8650c6f52815ec27eacd97bde347ef22c154b8d5b7b0`. L'auteur y fournit des réponses techniques proposées et des sources/limites; l'acceptation et la fermeture canonique relèvent de l'autorité compétente. La demande utilisateur directe délègue la résolution/consignation de Q-04/Q-06/Q-09 au coordinateur/implémenteur. Pour Q-09, la source humaine `sources/authorization/user-Q09-acceptance.md` contient exclusivement le choix transmis : « Rotation bornée (Recommended) ». Les limites du choix accepté sont attribuées à l'option proposée et documentée par l'architecte; les autres paramètres demeurent des détails techniques proposés par l'architecte, pas des choix séparés attribués à l'utilisateur.

P14 ne change ni D19, ni L04 allowlist. La liste L04 de l'état actif contient cinq chemins; elle est reproduite dans `01_LOTS.md`. Tout besoin d'un sixième chemin, y compris UI/root/IPC/ports/configuration, est un CHANGE et un arrêt préflight avant édition.

## Q-04 — progrès et diagnostic

Suivre chaque obligation pendante séparément par étage avec un progrès réel monotone. Idle, Queued, AwaitingChoice et terminaux stables n'attendent aucun progrès. Silence VAD est normal si capture et persistance sont actives. L'activité d'un étage ne masque pas un DurableAck, une inférence ou une obligation indépendante bloquée. Les événements répétés/obsolètes et la présence/heartbeat du processus ne réinitialisent pas les compteurs.

- Preparing : préparation achevée et Ready/attestation corrélée.
- Capturing : échantillons reçus; persistance suivie séparément.
- Import : décodage réel, segments achevés et reçu durable.
- Inférence : fenêtre soumise puis progression/résultat achevé.
- Draining : obligations de buffers, reçus durables et fin de flux.
- Finalizing : encodeur fini, artefact fermé/synchronisé/vérifié et publication confirmée.
- Cancelling/Quitting : commande admise, confirmation d'arrêt et ressources/état durable stabilisés.

Valeurs techniques proposées par l'architecte dans la source v2 : polling au plus une fois par seconde ; avertissement à 60 s d'absence de progrès d'une obligation ouverte, prochaine observation nominale au plus tard à 61 s sans garantie de scheduling OS ; diagnostic inchangé rafraîchi au plus toutes les 30 s et immédiatement sur erreur/transition ; un avertissement par épisode/obligation ; horloge monotone contrôlée. Une anomalie/reprise OS n'établit pas de panne.

Distinguer attente attendue, lenteur/absence de réponse suspectée et panne établie. Diagnostic fermé aux identifiants techniques, phase, backend attesté, âge de progrès/reçu, profondeur/capacité de file, codes contrôlés et état connu du processus; toute valeur indisponible reste inconnue. Panne établie seulement sur événement explicite, par exemple erreur de capture/disque, sortie enfant inattendue, protocole invalide ou saturation constatée; alors cesser l'entrée concernée, préserver Recoverable et alerter. Aucun timer ne déclenche arrêt, kill, fallback ou publication. Les conditions Auto et le rejet de génération obsolète restent ceux du design accepté.

## Q-06 — Quitter sans confirmation de l'enfant

Réutiliser l'invariant accepté DETAIL-P03/D19 : Quitter reste coopératif et latched; l'application demeure ouverte et réactive tant que l'enfant courant n'a pas confirmé `Stopped`. En live, couper l'entrée puis drainer/finaliser; en import, annuler/invalider la génération et préserver la source. Ne pas admettre de nouveau travail et ne pas lancer des commandes Stop concurrentes au clic répété. Montrer l'attente/le diagnostic; aucun faux `Complete`, aucune terminaison forcée ni geste de force. Préserver la portion confirmée et récupérable; les buffers non confirmés n'ont pas de garantie de durabilité. Une confirmation tardive du bon enfant et un état durable stabilisé permettent la sortie normale; un événement ancien ne confirme jamais l'arrêt courant. Aucun IO/FFI/join/attente arbitrairement bloquante sur UI et aucun délai maximal d'arrêt natif promis.

Les formulations de messages UI, l'état UI interactif et les consommateurs sont des propositions à intégrer au consumer existant. Si cela nécessite un chemin hors allowlist L04, arrêter sur CHANGE avant code. La validation d'interface réelle est NOT RUN.

## Q-09 — rotation des diagnostics

Le choix transmis par l'utilisateur porte sur l'option de rotation bornée. Le paquet proposé fixe quatre fichiers de 2 MiB chacun, fichier actif inclus (8 MiB total), et 7 jours maximum; ces limites sont celles de l'option décrite, pas une affirmation que l'utilisateur a séparément sélectionné d'autres paramètres techniques. Le plafond peut raccourcir la rétention; aucun minimum n'est garanti. Expiration au démarrage et avant écriture/rotation; pas de tâche OS pendant l'arrêt. Renouveler l'actif avant dépassement ou à la première écriture après expiration, puis supprimer les fichiers clos les plus anciens.

Portée: fichiers connus créés par l'application dans son répertoire local de diagnostic dédié. Exclure journal produit, fragments, pending/récupération, MP3/TXT/SRT, manifests/pointeurs, sources importées et dossiers d'archives utilisateur. Schéma fermé: versions, UTC, IDs opaques, états/backend, codes enum/numéro OS, durées, compteurs/capacités. Interdire audio/PCM, transcription/segment, chemins/noms libres, commande/environnement, dumps et messages natifs libres; aucun réseau, aucun `Debug`/`Display` natif libre.

Paramètres proposés par l'architecte, non attribués au choix utilisateur : writer parent/adaptateur; admission non bloquante; file ≤128 enregistrements de ≤4 KiB; budget payload mémoire 512 KiB (overhead mesuré séparément); snapshots remplaçables et répétitions regroupées; pertes exposées; événements hors schéma/plafond refusés sans troncature ambiguë; réserver la place avant écriture; purge impossible ou erreur open/write suspend l'écriture et expose une dégradation sans dépasser le budget; aucune sync durable par enregistrement, suffixe incomplet ignoré après coupure.

Une saturation peut perdre des événements diagnostiques mais ne perd ni commande Stop ni fragment produit confirmé; la récupération ne dépend jamais de ces logs. Vérifier le périmètre strictement borné et la continuité durable indépendamment.

## Vérifications futures et limites de preuve

Tous NOT RUN : horloge 59/60/61 s et absence d'action timer; silence VAD avec reçus actifs; inférence longue et worker vivant; capture progressant avec DurableAck figé; saturation data et admission Stop; erreurs explicites et Recoverable; enfant bloqué avec vraie fenêtre ouverte et événement ancien refusé; sentinelles de fuite; bornes de taille/âge, fichier inconnu et purge refusée; coupure pendant rotation. Chaque preuve devra conserver stimulus, branche, effet, témoin, fingerprint, logs/hashes et limites. Une fixture pure ne qualifie ni enfant natif ni vraie UI.

DESIGN_FEASIBILITY : complément proposé compatible, aucun SPIKE ou nouveau succès expérimental revendiqué. PRODUCT_VALIDATION et DELIVERY_QUALIFICATION : NOT RUN. Pas de campagne runtime, build, code ou UI prouvés par cette réponse. Aucun L04/L05 n'est autorisé à démarrer par P13.




