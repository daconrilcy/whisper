Avancée : le besoin, les exigences, le domaine et l'architecture de sélection GPU sont cadrés sur le dépôt courant.
Blocage : l'ordre Windows, l'appariement et l'attestation par carte restent à prouver ; un schéma requis du pack Rust manque.
Décision : conserver ce cadrage DRAFT, sans plan exécutable ni autorisation d'implémentation.

# Qualification et sélection des GPU — cadrage du 14 octobre 2026

**État : DRAFT ; gate DESIGN BLOCKED.** Base inspectée : branche `evo-startup-perf`, commit `ecae272270c9db441fc957ab10e7c099e1d45ee5`, dépôt propre au préflight. Auteur Git : session Codex principale. Apports en lecture seule : product framer `PROD-GPUSEL-v0.2`, requirements analyst `GPUSEL-REQ-v0.2`, domain architect `DOM-GS-v0.2`, Rust architect `ARCH-GS-v0.1`, puis interprétation du SPIKE `ARCH-GS-v0.2`. Les vérifications du **produit** sont **NOT RUN**. Le SPIKE-GS01 autorisé dans cette conversation a été exécuté par `/root/spike_gpu_executor` sur la base Git `aa50ac7c7753c22cbbe7ccb71d371fffadbcbaae` ; ses observations locales ne qualifient pas le produit. Les verdicts documentaires concernent leurs commits exacts, pas ce libellé d'état à lui seul.

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

**Capacités du brief, owner product framer.** BF-GS01 : qualifier et montrer les cartes ; BF-GS02 : rendre l'incertitude visible ; BF-GS03 : choisir une carte et identifier celle qui calcule ; BF-GS04 : appliquer l'ordre Windows sans choix ; BF-GS05 : respecter les règles sans GPU ou après échec ; BF-GS06 : mémoriser et substituer temporairement ; BF-GS07 : garder la séance fixe ; BF-GS08 : préserver l'audio, les sorties et la réactivité. BF-GS05 renvoie donc à REQ/UC/AC-GS06, tandis que BF-GS06 renvoie à REQ/UC/AC-GS05 : les numéros décrivent deux inventaires d'owners différents, explicitement reliés dans la matrice.

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

| Concept | Définition et propriétaire |
|---|---|
| DOM-GS01, domaine | Carte observée : identité Windows opaque, présence, nom/état facultatifs et rang sourcé ; homonymes possibles. |
| DOM-GS02, domaine | Qualification attribuée à une carte et au moteur : utilisable confirmée, non prise en charge confirmée ou indéterminée ; disponibilité séparée. |
| DOM-GS03, application | Catalogue d'une génération avec couverture complète/partielle/échec et causes. |
| DOM-GS04, domaine/application | Préférence durable : absence de choix ou identité Windows choisie, distincte d'une substitution. |
| DOM-GS05, domaine | Résolution de séance : mode, préférence et catalogue vers GPU candidat, CPU, attente ou refus motivé. |
| DOM-GS06, domaine/application | Moteur effectif CPU ou GPU identifié, attesté pour une séance précise. |
| DOM-GS07, application/domaine | Séance : identité, mode, candidat, moteur attesté, début du traitement, tentative CPU et intention d'arrêt. |

| Invariant | Garantie et lien principal |
|---|---|
| INV-GS01 | Présence ≠ support ≠ disponibilité ≠ moteur prêt ; l'incertitude n'est jamais un positif par défaut — GS01–02. |
| INV-GS02 | Une identité, et jamais le nom ou le rang seuls, rattache préférence, qualification, candidat et attestation — GS01/03. |
| INV-GS03 | Préférence compatible prioritaire ; sinon première compatible selon l'ordre Windows, sans tri substitutif — GS04/05. |
| INV-GS04 | Substitution temporaire avec message sans changer la préférence — GS05. |
| INV-GS05 | Candidat fixe après résolution ; une panne d'initialisation ne relance pas une recherche d'autre GPU — GS03/06/07. |
| INV-GS06 | Auto : une tentative CPU avant traitement ; aucune en GPU manuel ni après ce début — GS06. |
| INV-GS07 | Admission unique après attestation de la bonne carte et séance ; résultats obsolètes sans effet — GS03/07. |
| INV-GS08 | Réemploi seulement pour carte/configuration compatibles ; le cache ne remplace pas le choix — GS03/07. |
| INV-GS09 | Les couleurs et états visibles proviennent de l'application, sans politique parallèle en UI/composition — GS01/02/08. |
| INV-GS10 | L'audio et les exports suivent leurs contrats existants ; la qualification ne les modifie pas — GS07/08. |

- Présence Windows, support du moteur, disponibilité temporaire et préparation avec large-v3-turbo sont des états distincts. Une désactivation, un pilote inaccessible ou une VRAM insuffisante ne prouvent pas à eux seuls un non-support définitif.
- Préférence durable ≠ candidat de séance ≠ GPU effectif attesté. Nom commercial, rang, `CUDA0` et état « Ready Cuda » ne prouvent pas l'identité de la carte.
- Catalogue : `non demandé → en cours(génération) → complet | partiel | échec`. Une erreur après observations conserve le partiel ; résultat obsolète ou doublon sans effet. La fermeture invalide la publication de cette instance.
- Préférence : choix courant, sauvegarde demandée, puis sauvegarde confirmée ou échouée pour une révision précise. Une substitution temporaire ou un repli CPU n'écrit pas une nouvelle préférence.
- Séance : résolution → préparation → prêt attesté → traitement → finalisation → terminé, ou erreur/annulation. Le candidat est fixe après résolution. L'admission GPU exige une preuve corrélée à la bonne séance ; Auto consomme au plus une tentative CPU avant traitement. Aucun autre GPU n'est essayé après l'échec de préparation du candidat sans nouvelle décision.
- Le cache doit correspondre au GPU et à la configuration de la séance, et être réattesté lors de sa réutilisation. Aucun résultat tardif ne remplace un choix utilisateur. L'audio déjà admis et les blocs confirmés gardent leurs garanties actuelles ; aucune reprise automatique après crash n'est introduite.

## Architecture Rust proposée

Owner : Rust architect. Les modules ci-dessous sont des **frontières candidates**, pas une liste d'écriture autorisée. `whisper-core` conserve les règles pures et les ports applicatifs ; `whisper-adapters` porte Windows, le transport et les préférences ; `whisper-worker-gpu` prépare/atteste le moteur ; `desktop/ui.rs` affiche les vues ; `desktop/root.rs` compose. Les contrôles futurs combinent graphe Cargo, imports, visibilités et consommateurs réels. Le code natif éventuel doit conserver les lints `unsafe` existants ; aucune exception nouvelle n'est acceptée par ce document.

| Module physique | Dépendances permises | Dépendances interdites | Contrôle futur |
|---|---|---|---|
| Core, domaine pur (`gpu_inventory.rs`, sélection) | `std`, types purs, sérialisation justifiée | WMI, DXGI, CUDA, IO, UI et couche application | Imports/visibilité et graphe Cargo |
| Core, application (`gpu_inventory_app.rs`, ports de préférence/séance) | Domaine et ports qu'elle possède | Adaptateurs concrets, `eframe`, worker natif | Imports et API publiques |
| Adapters, inventaire/transport | API core, Windows, IPC | UI/root, règles métier parallèles | Cargo/features et imports |
| Adapters, stockage des réglages | Contrat application, JSON et fichier profil | Résolution GPU/substitution métier | Propriétaire des données et revues des écritures |
| Worker GPU, orchestration IPC | Types IPC purs, moteur natif | UI, préférences, recherche silencieuse d'un autre GPU | Chemin d'admission et événements |
| Worker GPU, moteur/façade native | `whisper-rs`/sys et types purs nécessaires | UI, persistance | Pointeurs/retours/handles et lints `unsafe` |
| Desktop `ui.rs`/`lib.rs` | API/vues core | Adaptateurs concrets, appels Windows/natifs bloquants | Imports des consommateurs |
| Desktop `root.rs`/`main.rs` | Couches nécessaires à la composition | État métier parallèle | Injection et sélection effective |

`whisper-core` contient domaine **et** application : vérifier les imports internes, car un build core ne prouve pas leur séparation. La dépendance Cargo de `whisper-desktop` à `adapters` sert à `root.rs`/`main.rs` ; elle n'autorise pas son usage direct dans `ui.rs`.

| Choix architecte et statut documentaire | Option retenue / alternative | Justification et preuve ; limite |
|---|---|---|
| ARCH-GS-D01, choix technique proposé | Garder les crates existantes / créer des crates de domaine et qualification | Les frontières physiques et les imports restent vérifiables ; aucun besoin produit de nouvelle crate (`Cargo.toml`, modules courants). |
| ARCH-GS-D02, choix technique proposé | Passer l'ordinal résolu à `gpu_device` / modifier `CUDA_VISIBLE_DEVICES` ou accepter GPU implicite | API présente dans `whisper-rs 0.16.0` ; le chemin entier jusqu'à la carte Windows reste à prouver. |
| ARCH-GS-D03, hypothèse structurante ouverte | WMI pour couverture et DXGI pour rang / ordre de réception WMI | DXGI documente un ordre ; la fusion et la couverture WMI/DXGI restent partielles. Aucun ordre WMI n'est présumé. |
| ARCH-GS-D04, choix technique proposé | Mémoriser le Device Instance ID / nom, index ou LUID | Microsoft documente sa persistance entre redémarrages ; réinstallation/déplacement non garantis. |
| ARCH-GS-D05, hypothèse structurante ouverte | Apparier PnP↔DXGI↔GGML par identité matérielle univoque / autre source vérifiable | Propriétés PCI/LUID disponibles dans leur portée ; chemin complet et encapsulation native compatibles avec les lints non démontrés. |
| ARCH-GS-D06, choix technique proposé | Inclure carte/config dans la clé de cache et attester chaque séance / conserver cache chemin/hash seul | Le cache courant ne distingue pas A de B ; pas de pool multicarte exigé. |

Les choix techniques proposés sont sous délégation de conception, **non acceptés comme implémentation ou faisabilité démontrée**. D03/D05 doivent être résolus avant DESIGN READY. Aucune nouvelle dépendance directe Windows ou exception `unsafe` n'est autorisée par cette table.

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

Sources techniques et résultats du SPIKE-GS01 relus directement par l'architecte le 14 octobre 2026. Le banc et ses sorties sont sous `target/gpu-selection-feasibility/` : `src/main.rs`, `dxgi-probe/src/main.rs`, leurs manifestes, `discover-1/2.stdout.txt`, `dxgi-1/2.stdout.txt`, `wmi-pnp.json`, `nvidia-smi.txt`, `infer-0.stdout/stderr.txt`, `invalid-ordinal.stderr.txt`, `mismatch.stderr.txt` et la synthèse `result.md`. Ces fichiers sont ignorés par Git ; les faits ci-dessous conservent leur portée sans présenter le banc comme une preuve produit. Les codes retour ont été rapportés par l'exécutant, sans journal de build persistant ; les fichiers WMI et `nvidia-smi` n'enregistrent pas leurs codes retour.

- `gpu_inventory.rs` utilise un `BTreeMap` par identité ; la requête WMI actuelle ne démontre pas un ordre Windows. [DXGI `EnumAdapters1`](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgifactory1-enumadapters1) documente un ordre d'adaptateurs, mais sa fusion avec toutes les cartes de l'inventaire WMI n'est pas démontrée.
- Le [Device Instance ID Windows](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/device-instance-ids) persiste entre redémarrages, sans promesse après réinstallation ou déplacement matériel.
- `whisper-rs 0.16.0` local expose `WhisperContextParameters::gpu_device` ; le sys natif sélectionne un ordinal GGML, distinct de l'ordre Windows. Le chargement produit actuel utilise `use_gpu(true)` sans sélectionner explicitement la carte. GGML CUDA expose des propriétés PCI ; CUDA local expose UUID/LUID/PCI, avec limites pour LUID en TCC. La jointure complète PnP↔DXGI↔GGML reste non prouvée.
- Le code courant sonde CUDA sans modèle complet et réemploie un moteur en cache par chemin/hash du modèle. `Ready` et le contrôle littéral `using CUDA0 backend` attestent le backend sur le processus, pas l'identité de la carte pour chaque séance. Un log « using » peut précéder un échec d'initialisation.
- Les réglages actuels sont enregistrés par fichier temporaire, synchronisation et renommage ; l'ajout du champ GPU ne doit pas écraser les autres préférences.

| Faisabilité structurante | État | Preuve encore nécessaire |
|---|---|---|
| Identité Windows durable ; API native de sélection d'ordinal | PASS documentaire, chacune dans sa portée | Leur existence ne démontre ni la jointure ni l'usage effectif. |
| RISK-T-GS01.a/b — ordre Windows et fusion WMI/DXGI | PARTIAL | Deux relevés DXGI conservent le même ordre NVIDIA, Intel, NVIDIA, Basic Render Driver. Les deux entrées NVIDIA partagent nom/Vendor/Device/SubSys/Revision et ont des LUID différents ; WMI n'a qu'une carte PnP NVIDIA, avec Parsec non apparié par nom. Aucun rang DXGI univoque ni fusion complète pour les candidats n'est prouvé. |
| RISK-T-GS02.b — appariement identité Windows ↔ ordinal dans le worker | PARTIAL | Deux processus GGML voient CUDA0 au même PCI `0000:01:00.0`, également rapporté par `nvidia-smi`. Le lien PnP↔bonne entrée DXGI↔GGML demeure ambigu ; aucun redémarrage Windows n'a été essayé. |
| RISK-T-GS03.b — attestation du contexte et de la carte | PARTIAL ; chemin CUDA0 local exercé | `gpu_device(0)` a créé contexte/état et produit un texte non vide à partir de 233472 échantillons, dont 171136 non nuls, avec journal CUDA0 du même processus. Le banc recopie ensuite l'identité du **candidat** ; il n'interroge pas l'identité matérielle du contexte prêt. Les gardes d'ordinal 9 et de PCI discordant refusent avant modèle, mais ne testent ni échec natif après sélection ni A/B multiCUDA. |
| RISK-T-GS04.a — qualification individuelle sans modèle | PARTIAL | La découverte GGML énumère les propriétés sans modèle ; elle ne tente pas l'initialisation/libération de chaque backend et ne distingue donc pas tous les cas de non-support et d'indisponibilité temporaire. |
| Cache A→B et statut temporaire | Contrats proposés ; PRODUCT_VALIDATION NOT RUN | Vérification du futur candidat sans faux vert ni contexte A réutilisé pour B. |

Risques fonctionnels : faux vert, mauvaise carte malgré homonymes, faux premier Windows, préférence écrasée, repli CPU ou GPU hors règle, résultat tardif/cache d'une autre carte, attente/capture trompeuse, conclusion multicarte tirée d'une machine monoCUDA. Ces risques renvoient respectivement à GS01, GS03, GS04, GS05, GS06, GS07 et GS08.

| Question, classification, owner | GS et échéance | Options / fermeture attendue |
|---|---|---|
| Q-GS01, STRUCTURING, architecte | GS01/04/05 ; avant gate DESIGN | WMI+DXGI avec fusion et rang démontrés, ou autre API Windows à ordre documenté ; un autre ordre produit demande l'utilisateur. |
| Q-GS02, STRUCTURING, architecte | GS03/04/07 ; avant gate DESIGN | Jointure PnP↔worker et preuve native par séance, ou autre chemin démontré ; réduction de la garantie demande l'utilisateur. |
| Q-GS03, STRUCTURING, architecte et domaine | GS01/02/06 ; avant gate DESIGN | Séparer non-support de l'incapacité temporaire ; après choix d'un candidat compatible, erreur de préparation puis manuel erreur/CPU Auto, sans autre GPU arbitraire. |
| Q-GS04, DETAIL, présentation et stockage | GS01/02/05 ; avant lots UI/persistance futurs | Libellé indéterminé neutre accessible et accusé de sauvegarde révisionné ; aucun état vert/gris inféré d'une donnée inconnue. |
| Q-AN-GS06, GAP de contrat, analyste et architecte | GS07/08 ; avant plan d'un lot import | Lire le comportement courant d'annulation et fermeture import (`desktop/src/ui.rs` retient actuellement la fermeture pendant occupation), puis définir le résultat observable sans modifier implicitement ce contrat. |

Les bornes d'absence de progrès et le corpus/matériel de validation finale ne sont pas élargis par ce document. Les questions STRUCTURING bloquent uniquement les dépendances indiquées ; le DETAIL reste borné à ses lots. Aucun choix technique ouvert n'est présenté comme une nouvelle décision produit acceptée.

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

**SPIKE-GS01 — autorisé et exécuté, résultat PARTIAL.** L'utilisateur a autorisé dans cette conversation le banc Rust isolé, puis un sous-test DXGI limité à `CreateDXGIFactory1`/`EnumAdapters1` et à une dépendance directe du banc à `windows` déjà disponible. L'exécutant distinct a réalisé les inventaires WMI/DXGI/GGML, deux découvertes dans des processus différents, une courte inférence non silencieuse sur CUDA0 et deux refus contrôlés avant modèle. Les builds finaux et l'inférence valide ont rapporté un code retour 0 ; l'ordinal absent et le PCI discordant un code 1. Ces codes ne figurent pas dans des journaux persistants. La jointure n'a pas été résolue ; les gardes du banc ne constituent pas les garanties du produit. La machine ne présente qu'un CUDA, donc la sélection entre deux GPU CUDA réels reste **PRODUCT_VALIDATION, NOT RUN**. Aucun nouvel essai n'est autorisé par ce résultat.

**SPIKE-GS02 — seulement proposé, non autorisé, NOT RUN.** L'architecte propose au plus 90 minutes dans le même dossier isolé, avec un exécutant Rust distinct, pour rechercher une identité matérielle univoque PnP↔DXGI↔GGML, expliquer les deux entrées NVIDIA, initialiser/libérer chaque backend CUDA sans modèle et chercher une preuve du contexte préparé distincte de l'identité du candidat. Les API Windows supplémentaires et leur encapsulation devront être nommées dans un mandat précis avant exécution ; ni SetupAPI, ni Configuration Manager, ni une autre couche native ne sont autorisés par le SPIKE-GS01. Chaque sous-condition non exercée resterait PARTIAL. Aucun téléchargement, pilote, élévation ou code produit n'est proposé.

La référence `C:\Users\cyril\.codex\skills\rust-predev-design\references\workflow-schema.md`, requise par `agent-rules/IMPLEMENTATION_RULES.md`, est absente. Aucun fichier historique n'est substitué. Ce manque bloque le gate et le plan writer ; il ne retire pas le contenu de cadrage ci-dessus. Le `RAILGUARD.md` courant contient par ailleurs des formulations historiques sur MP3 et l'ancien outillage : à analyser avant un lot affecté, sans réactiver G1.

**Readiness : DRAFT / DESIGN BLOCKED.** SPIKE-GS01 n'a pas fermé l'ordre/fusion, la jointure, l'attestation de carte ni la qualification individuelle. Les questions structurantes et le schéma requis restent ouverts. Une revue indépendante juge la qualité documentaire d'un commit précis ; elle ne ferme pas à elle seule ces préalables. Aucun plan exécutable ni autorisation de coder n'est créé. La suite possible est un nouvel essai de faisabilité précisément mandaté et la résolution du cadre ; le plan, sa revue, l'implémentation et l'essai final dans la fenêtre Windows restent distincts.
