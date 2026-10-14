Avancée : le besoin, les exigences, le domaine et l'architecture de sélection GPU sont cadrés sur le dépôt courant.
Blocage : l'ordre Windows, l'appariement et l'attestation par carte restent à prouver ; un schéma requis du pack Rust manque.
Décision : conserver ce cadrage DRAFT, sans plan exécutable ni autorisation d'implémentation.

# Qualification et sélection des GPU — cadrage du 14 octobre 2026

**État : DRAFT ; gate DESIGN BLOCKED.** Base inspectée : branche `evo-startup-perf`, commit `ecae272270c9db441fc957ab10e7c099e1d45ee5`, dépôt propre au préflight. Auteur Git : session Codex principale. Apports en lecture seule : product framer `PROD-GPUSEL-v0.2`, requirements analyst `GPUSEL-REQ-v0.2`, domain architect `DOM-GS-v0.2`, Rust architect `ARCH-GS-v0.1`. Les vérifications du produit, l'essai matériel et la revue indépendante de ce document sont **NOT RUN** à cette version.

## Besoin, autorités et périmètre

L'inventaire Windows existant montre les cartes présentes, mais leur présence ne prouve pas leur compatibilité avec le moteur livré. L'utilisateur veut voir les cartes confirmées utilisables en vert, les cartes confirmées non prises en charge en gris, choisir une carte pour l'inférence et voir quelle carte exécute effectivement le calcul. Un état indéterminé explicite est proposé lorsque la preuve manque ; il ne doit être ni vert ni présenté comme un non-support confirmé.

| Source | Autorité et décision |
|---|---|
| S-GS01, demande de cette conversation | Utilisateur : qualification par carte, couleurs, choix et sélection par défaut selon l'ordre Windows. |
| S-GS02, réponse 1 | Utilisateur : en GPU manuel, échec expliqué sans repli CPU automatique. |
| S-GS03, réponse 2 | Utilisateur : choix explicite prioritaire en GPU manuel et Auto. |
| S-GS04, réponse 3 | Utilisateur : choix mémorisé après fermeture et message si la carte manque. |
| S-GS05, réponse 4 | Utilisateur : premier GPU Windows s'il est compatible, sinon premier compatible dans cette même liste. |
| S-GS06, réponse 5 | Utilisateur : conserver la préférence absente ou incompatible, employer temporairement le premier autre GPU compatible et informer. |
| S-GS07, qualification revue dans cette conversation | Critères CA-01 à CA-08 proposés par l'assistant ; leur rédaction technique ne constitue pas une nouvelle décision utilisateur. |
| S-GS08, `AGENTS.md`, `evo-gpu-inventory.md`, `evo-auto.md`, `evo-startup-perf.md`, `evo-gpu.md`, `bilan-evo-gpu.md` | Git : modes, réactivité, qualité et contrats audio existants. Certaines descriptions de code anciennes sont historiques. |
| S-GS09, `evo-intel-gpu.md` | Git : décision utilisateur du 14 octobre 2026 d'arrêter l'implémentation Intel. |

Le périmètre comprend la qualification, le choix pour WAV/MP3 et microphone, l'ordre par défaut, la préférence durable, la substitution temporaire avant sélection, l'attestation de la carte réellement utilisée et la continuité des parcours CPU/Auto/GPU. Intel peut rester visible dans l'inventaire ; sa présence ne réactive ni un moteur Intel ni une étape NVIDIA → Intel → CPU. Restent différés : changement de GPU pendant une transcription, supervision continue, reprise après crash, autre modèle, tray et sortie MP3. Les contrats large-v3-turbo, blocs confirmés et TXT/SRT/WAV restent applicables.

## Exigences, parcours et acceptation

Owner : requirements analyst. `REQ-GS01..08`, `UC-GS01..08` et `AC-GS01..08` correspondent aux huit capacités `BF-GS01..08` du product framer. Tous les AC sont des vérifications futures **PRODUCT_VALIDATION — NOT RUN** ; les données contrôlées ne prouvent pas seules la réalité d'une machine multicarte.

| BF / REQ / UC / AC | Contrat et scénario observable futur |
|---|---|
| GS01 — qualification visible | Afficher chaque carte Windows présente par identité, même avec noms identiques. Avec A utilisable, B non supportée, C non appariée, D désactivée et E en échec temporaire : A seule est verte, B grise, C indéterminée ; D/E ne deviennent pas « non prises en charge » sur ce seul motif. Statut lisible sans dépendre de la couleur. |
| GS02 — incertitude | Distinguer attente, complet, partiel, vide confirmé et échec ; conserver les observations fiables. Une collecte ou qualification inachevée n'annonce pas « aucun GPU compatible ». Pendant une réponse retenue, l'interface et le parcours CPU restent utilisables. |
| GS03 — choix et effectivité | En modes GPU manuel et Auto, pour import WAV/MP3 et micro, le GPU B choisi parmi A/B compatibles est celui de l'inférence. Rapprocher identité Windows, candidat envoyé, contexte natif de cette séance et affichage ; utiliser un témoin A/B inversé. « CUDA actif » seul ne satisfait pas le critère. Une preuve manquante ou discordante empêche d'annoncer B comme effectif. |
| GS04 — ordre par défaut | Sans préférence, pour Windows A non supportée, B/C compatibles et moteur énumérant C/B, retenir et attester B ; si A est compatible, retenir A. Ne pas remplacer l'ordre Windows par le tri du nom, du `BTreeMap` ou par l'ordinal moteur. Si le rang de candidats manque, signaler la limite au lieu de fabriquer un premier. |
| GS05 — préférence durable | Après choix de P et redémarrage, P reste préférée. Si P est absente ou incompatible **avant la sélection**, Q, premier autre compatible dans l'ordre Windows, est temporairement choisi avec message ; Q n'écrase pas P. Quand P revient compatible, elle reprend priorité à la prochaine séance. L'échec de sauvegarde est visible et n'est pas présenté comme une mémorisation réussie. |
| GS06 — échec borné | Sans GPU compatible : GPU manuel échoue avec cause ; Auto utilise CPU et affiche le moteur effectif. Si le candidat compatible choisi échoue à l'initialisation, ne pas essayer silencieusement un autre GPU ; en Auto, une seule tentative CPU avant le traitement audio, aucune après son début ; en GPU manuel, aucun repli CPU. Échec CPU terminal. Une erreur de modèle ou d'entrée conserve sa propre cause. |
| GS07 — cycle de séance | Candidat et moteur effectif restent fixes pendant la transcription ; modification de préférence ou résultat de découverte tardif ne modifient pas cette séance. Annulation interdit une admission ou un repli tardif. Fermeture du micro suit arrêt/finalisation ; avant moteur prêt, l'audio réellement admis dans le WAV est conservé selon la décision startup Q4. L'annulation/fermeture de l'import doit reprendre son contrat courant, à préciser avant plan. |
| GS08 — continuité | Préserver CPU, large-v3-turbo, imports, microphone, blocs confirmés, qualité comparable, TXT/SRT/WAV, et une interface utilisable pendant une qualification lente. Vérifier séparément : premier bloc GPU **affiché** en `<15 s` depuis « Démarrer » avec parole immédiate ; traitement GPU plus rapide que la durée audio ; plages admises au WAV présentes une fois, en ordre, après arrêt/fermeture coopératifs ou panne worker avec application vivante. Attente/progrès affichés honnêtement, sans nouveau délai d'abandon inventé. |

Parcours : UC01 consulter les cartes, UC02 comprendre le partiel, UC03 choisir, UC04 démarrer sans choix, UC05 retrouver la préférence, UC06 comprendre l'échec, UC07 arrêter/fermer/recommencer, UC08 transcrire en préservant le résultat. Chacun couvre attente et erreur, ainsi qu'annulation, fermeture et nouvelle séance lorsqu'elles s'appliquent. La cible startup d'interface `<2 s` et celle de capture `<3 s` demeurent proposées dans `evo-startup-perf.md` ; elles ne sont pas annoncées acquises. La capture active, l'admission WAV, le moteur prêt et le texte rendu sont quatre jalons distincts.

## Domaine et invariants

Owner : domain architect. `DOM-GS01..07` couvrent carte/identité Windows, qualification, catalogue, préférence, candidat et séance. `INV-GS01..10` relient ces concepts aux exigences ci-dessus.

- Présence Windows, support du moteur, disponibilité temporaire et préparation avec large-v3-turbo sont des états distincts. Une désactivation, un pilote inaccessible ou une VRAM insuffisante ne prouvent pas à eux seuls un non-support définitif.
- Préférence durable ≠ candidat de séance ≠ GPU effectif attesté. Nom commercial, rang, `CUDA0` et état « Ready Cuda » ne prouvent pas l'identité de la carte.
- Catalogue : `non demandé → en cours(génération) → complet | partiel | échec`. Une erreur après observations conserve le partiel ; résultat obsolète ou doublon sans effet. La fermeture invalide la publication de cette instance.
- Préférence : choix courant, sauvegarde demandée, puis sauvegarde confirmée ou échouée pour une révision précise. Une substitution temporaire ou un repli CPU n'écrit pas une nouvelle préférence.
- Séance : résolution → préparation → prêt attesté → traitement → finalisation → terminé, ou erreur/annulation. Le candidat est fixe après résolution. L'admission GPU exige une preuve corrélée à la bonne séance ; Auto consomme au plus une tentative CPU avant traitement. Aucun autre GPU n'est essayé après l'échec de préparation du candidat sans nouvelle décision.
- Le cache doit correspondre au GPU et à la configuration de la séance, et être réattesté lors de sa réutilisation. Aucun résultat tardif ne remplace un choix utilisateur. L'audio déjà admis et les blocs confirmés gardent leurs garanties actuelles ; aucune reprise automatique après crash n'est introduite.

## Architecture Rust proposée

Owner : Rust architect. Les modules ci-dessous sont des **frontières candidates**, pas une liste d'écriture autorisée. `whisper-core` conserve les règles pures et les ports applicatifs ; `whisper-adapters` porte Windows, le transport et les préférences ; `whisper-worker-gpu` prépare/atteste le moteur ; `desktop/ui.rs` affiche les vues ; `desktop/root.rs` compose. Les contrôles futurs combinent graphe Cargo, imports, visibilités et consommateurs réels. Le code natif éventuel doit conserver les lints `unsafe` existants ; aucune exception nouvelle n'est acceptée par ce document.

| Contrat / owner | Entrée, effet et refus |
|---|---|
| PORT-GS01, application/Windows | Collecte hors thread UI avec génération, identités présentes, couverture et rang Windows dont la source est explicite. Une collecte en vol, un résultat terminal ; fermeture sans attendre un appel Windows bloqué. Aucun effet sur la séance. |
| PORT-GS02, application/moteur | Qualification individuelle sans charger le modèle complet : ordinal Whisper, identité exploitable, support, indisponibilité et cause. Erreur ou appariement ambigu donne « indéterminé », pas gris confirmé. Résultat corrélé ; aucun pointeur natif par IPC. |
| CONTRACT-GS03, domaine | Jointure univoque entre identité Windows et périphérique moteur. Refuser nom identique, ordinal égal par hasard, collision ou identité absente comme preuve de correspondance. |
| CONTRACT-GS04, domaine/application | Résoudre mode, préférence et catalogue courant vers candidat fixe, CPU ou erreur ; priorité à la préférence compatible, sinon premier compatible au rang démontré. Conserver la préférence indisponible. Échec de préparation du candidat : aucun autre GPU ; politique CPU Auto existante seulement. |
| PORT-GS05, application/worker | Transmettre séance, candidat, ordinal résolu, modèle/config ; revalider identité et ordinal dans **ce worker** avant chargement. Discordance, disparition, init refusée ou CPU inattendu : échec avant admission audio GPU. |
| CONTRACT-GS06, application/worker | Attester contexte effectivement préparé, carte et séance avant de publier le moteur effectif ; refaire preuve sur cache hit, rejeter les résultats périmés. Une ligne CUDA d'un ancien contexte ne suffit pas. |
| PORT-GS07, application/stockage | Champ GPU optionnel dans les réglages existants, un propriétaire du fichier partagé, accusé de révision écrite. Aucun snapshot GPU obsolète ne peut écraser modèle, destination ou microphone ; échec de sauvegarde visible. |
| CONTRACT-GS08, worker | Cache indexé aussi par identité GPU et paramètres influençant le contexte. Changer de carte invalide l'ancien contexte ; un seul contexte conservé suffit, aucun pool multiGPU exigé. |

Le transport conserve ses files audio ; la qualification ne bloque pas la voie audio et n'ajoute pas de supervision continue. La migration de préférence est additive et doit préserver les réglages existants. Une ancienne version pourrait supprimer un champ inconnu en réécrivant le fichier : aucune garantie de retour arrière vers l'ancienne application n'est affirmée. L'écriture temporaire/synchronisée/renommée actuelle ne promet pas une transaction commune avec TXT/SRT/WAV ni une récupération après panne système.

## Preuves, risques et questions

Sources techniques lues par l'architecte le 14 octobre 2026 ; aucune exécution native GPUSEL :

- `gpu_inventory.rs` utilise un `BTreeMap` par identité ; la requête WMI actuelle ne démontre pas un ordre Windows. [DXGI `EnumAdapters1`](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgifactory1-enumadapters1) documente un ordre d'adaptateurs, mais sa fusion avec toutes les cartes de l'inventaire WMI n'est pas démontrée.
- Le [Device Instance ID Windows](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/device-instance-ids) persiste entre redémarrages, sans promesse après réinstallation ou déplacement matériel.
- `whisper-rs 0.16.0` local expose `WhisperContextParameters::gpu_device` ; le sys natif sélectionne un ordinal GGML, distinct de l'ordre Windows. Le chargement produit actuel utilise `use_gpu(true)` sans sélectionner explicitement la carte. GGML CUDA expose des propriétés PCI ; CUDA local expose UUID/LUID/PCI, avec limites pour LUID en TCC. La jointure complète PnP↔DXGI↔GGML reste non prouvée.
- Le code courant sonde CUDA sans modèle complet et réemploie un moteur en cache par chemin/hash du modèle. `Ready` et le contrôle littéral `using CUDA0 backend` attestent le backend sur le processus, pas l'identité de la carte pour chaque séance. Un log « using » peut précéder un échec d'initialisation.
- Les réglages actuels sont enregistrés par fichier temporaire, synchronisation et renommage ; l'ajout du champ GPU ne doit pas écraser les autres préférences.

| Faisabilité structurante | État | Preuve encore nécessaire |
|---|---|---|
| Identité Windows durable ; API native de sélection d'ordinal | PASS documentaire, chacune dans sa portée | Leur existence ne démontre ni la jointure ni l'usage effectif. |
| Ordre Windows et fusion WMI/DXGI, y compris candidats non DXGI | PARTIAL | Rang sourcé et couverture de chaque carte candidate ; aucun rang inventé. |
| Appariement identité Windows ↔ ordinal dans le worker | PARTIAL | Jointure univoque, collisions/refus, stabilité entre processus. |
| Attestation du contexte réellement initialisé et de la carte | PARTIAL | Bonne carte, bonne séance, échec refusé avant admission, témoin discordant. |
| Qualification individuelle sans modèle | PARTIAL | La sonde actuelle trouve le premier CUDA initialisable, pas chaque carte. |
| Cache A→B et statut temporaire | Contrats proposés ; PRODUCT_VALIDATION NOT RUN | Vérification du futur candidat sans faux vert ni contexte A réutilisé pour B. |

Risques fonctionnels : faux vert, mauvaise carte malgré homonymes, faux premier Windows, préférence écrasée, repli CPU ou GPU hors règle, résultat tardif/cache d'une autre carte, attente/capture trompeuse, conclusion multicarte tirée d'une machine monoCUDA. Ces risques renvoient respectivement à GS01, GS03, GS04, GS05, GS06, GS07 et GS08.

Questions attribuées : **Q-GS01**, architecte, ordre Windows et fusion complète ; **Q-GS02**, architecte, jointure PnP/moteur et attestation par séance ; **Q-GS03**, architecte/domaine, support durable contre indisponibilité temporaire ; **Q-GS04**, UI/stockage, état visuel indéterminé et accusé de sauvegarde (détail avant lot). L'incapacité temporaire après choix d'un candidat compatible suit l'erreur de préparation de GS06, sans chercher arbitrairement un autre GPU. L'annulation/fermeture de l'import demande une lecture contractuelle avant plan ; les bornes d'absence de progrès et le corpus final ne sont pas élargis par ce document.

### Traçabilité

| Brief | REQ/UC/AC | Domaine | Architecture et risque |
|---|---|---|---|
| BF01–02 | GS01–02 | DOM01–03, INV01/02 | PORT01/02, jointure03 ; faux vert/incertitude |
| BF03 | GS03/07 | DOM04–07, INV02/05/07/08 | PORT05, attestation06/cache08 ; mauvaise carte |
| BF04 | GS04 | INV03 | Jointure03/résolution04 ; faux ordre |
| BF05 | GS06 | INV06 | Résolution04/préparation05 ; repli non autorisé |
| BF06 | GS05/04 | DOM04/05, INV04 | Stockage07/résolution04 ; préférence écrasée |
| BF07 | GS06–07 | DOM07, INV05–08 | Séance05/06/cache08 ; résultat tardif |
| BF08 | GS08/02/07 | INV09/10 | Collecte indépendante et contrats audio ; attente/perte |

## Suite proposée, état et autorisations

**SPIKE-GS01 — proposé, non autorisé, NOT RUN.** Pour fermer les seules faisabilités structurantes, l'architecte propose un exécutant Rust distinct et un banc isolé sous `target/gpu-selection-feasibility/`, deux heures au plus, sans installation, téléchargement ni modification du système. Relever l'ordre DXGI/PnP/GGML, construire la jointure, sélectionner l'ordinal correspondant, confirmer contexte et courte inférence non silencieuse sur le modèle local, puis vérifier le refus d'un ordinal invalide et d'une preuve discordante avant admission. Refaire la découverte dans un nouveau processus. Un seul GPU CUDA local peut démontrer le chemin et les refus locaux, **pas** la sélection entre deux GPU CUDA réels ; ce cas restera NOT RUN jusqu'à une cible appropriée. Chaque sous-condition reçoit son résultat brut et sa limite ; aucun succès global n'est déduit d'un résultat partiel. Ce mandat nécessiterait un accord écrit précis avant toute exécution.

La référence `C:\Users\cyril\.codex\skills\rust-predev-design\references\workflow-schema.md`, requise par `agent-rules/IMPLEMENTATION_RULES.md`, est absente. Aucun fichier historique n'est substitué. Ce manque bloque le gate et le plan writer ; il ne retire pas le contenu de cadrage ci-dessus. Le `RAILGUARD.md` courant contient par ailleurs des formulations historiques sur MP3 et l'ancien outillage : à analyser avant un lot affecté, sans réactiver G1.

**Readiness : DRAFT / DESIGN BLOCKED.** Les questions structurantes et le schéma requis restent ouverts. Revue indépendante de ce fichier Git : **NOT RUN** à cette version. Aucun plan exécutable ni autorisation de coder n'est créé. La suite possible est un essai de faisabilité précisément mandaté et la résolution du cadre ; le plan, sa revue, l'implémentation et l'essai final dans la fenêtre Windows restent distincts.
