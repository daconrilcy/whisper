Avancée : le besoin et une architecture documentaire sont définis pour lister les GPU actuellement présents.
Blocage : la couverture réelle de Windows et la validation dans la fenêtre restent à vérifier.
Décision : la liste est informative, près de « Calcul », et exclut les anciens GPU débranchés.

# Inventaire des GPU Windows — cadrage du 14 octobre 2026

**État : DESIGN READY** après revue indépendante CLEAN du candidat `51dd0bdbdebef400690d03b65772d9b9f00a9909` ; plan futur en préparation. Ce cadrage n'autorise aucune implémentation. Base : branche `evo-startup-perf`, commit `49797d4264791dbad9fff16c9f44cd2eb4f1a89c` avant ce document. Sources : demande et réponses de l'utilisateur du 14 octobre 2026, expression de besoin revue PASS avec réserve sur l'inventaire partiel, `AGENTS.md`, `docs/cadrage/evo-gpu.md`, `docs/cadrage/evo-auto.md`, `docs/cadrage/evo-startup-perf.md`, `docs/cadrage/bilan-evo-gpu.md` et code courant. Contributions en lecture seule : product framer `PROD-GPUINV-v1.1`, requirements analyst `GPUINV-REQ-v1.1`, domain architect `DOM-GI-v0.3`, Rust architect `ARCHGI-v0.3`. L'auteur de ce document est la session principale.

## Besoin, décisions et périmètre

L'utilisateur voit près de « Calcul » les GPU **actuellement présents** selon Windows, y compris un GPU Intel intégré, les cartes désactivées et les cartes virtuelles présentes. Chaque carte a un nom identifiable et un état que Windows permet d'établir. La liste informe seulement : elle ne permet pas de choisir une carte de transcription. Les anciennes cartes débranchées sont exclues par décision explicite du 14 octobre 2026. Un GPU recensé n'est pas nécessairement utilisable par le moteur CUDA ; la liste n'est pas une attestation de compatibilité.

La sélection d'un GPU précis, le calcul Intel, une surveillance continue du matériel et une modification des politiques Auto/CPU/GPU sont hors de ce périmètre. Les imports WAV/MP3, le micro, le modèle local large-v3-turbo, les blocs confirmés et les sorties TXT/SRT/WAV gardent leurs contrats existants. Aucun seuil de durée propre à l'inventaire n'est ajouté. L'essai final dans la fenêtre Windows reste à l'utilisateur.

## Exigences et critères d'acceptation futurs

| ID | Exigence | Vérification observable ; état actuel |
|---|---|---|
| REQ-GI01 / AC-GI01 | Une ligne par identité Windows de GPU dont la présence actuelle est confirmée, sans filtre de constructeur, d'activation, de virtualisation ou de CUDA ; jamais dédupliquer par nom. Exclure les identités confirmées absentes. | Cible multiGPU Intel + autre carte, cas présent désactivé/virtuel, ancien GPU absent et deux noms identiques : comparer la liste à l'observation Windows. **NOT RUN**. |
| REQ-GI02 / AC-GI02 | Garder une carte présente si son nom ou son état manque ; afficher « inconnu » et une limite de couverture. Une présence indéterminable reste une incertitude signalée, pas une carte confirmée présente ou absente. | Jeux contrôlés pour nom/état manquants, présence vraie/fausse/inconnue et conflit de propriétés ; vérifier les lignes et avertissements. **NOT RUN**. |
| REQ-GI03 / AC-GI03 | Liste informative près de « Calcul » ; aucune sélection de carte ni capacité CUDA déduite de sa présence. | Dans le vrai widget, manipuler Auto/CPU/GPU et constater qu'aucune commande de choix de carte n'existe. **NOT RUN**. |
| REQ-GI04 / AC-GI04 | Distinguer attente, liste complète, liste vide confirmée, liste partielle et échec. L'inventaire n'ajoute aucune condition à la transcription. | Publier chaque résultat contrôlé ; zéro carte avec présence inconnue reste « partiel », jamais « aucun GPU ». Vérifier message et commandes. **NOT RUN**. |
| REQ-GI05 / AC-GI05, AC-GI07 | Collecte et résultat sans effet sur préférence Calcul, moteur effectif, audio ou sorties ; aucune attente de l'inventaire avant les actions de transcription. | Retenir artificiellement la réponse, choisir un mode, démarrer et arrêter un import ou le micro avant de libérer la réponse ; livrer ensuite un résultat tardif. **NOT RUN**. |
| REQ-GI06 / AC-GI06 | Collecter au lancement et à la réouverture ; fermer sans attente ajoutée et ignorer le résultat de l'ancienne instance. Choix de cycle proposé par l'architecture. | Fermer pendant une collecte, rouvrir et livrer l'ancien résultat : seule la nouvelle observation s'affiche. **NOT RUN**. |
| REQ-GI07 / AC-GI08 | Préserver les contrats de transcription existants sous inventaire nominal, en attente, partiel ou échoué. | Parcours WAV/MP3 et micro, CPU/GPU/Auto, arrêt/fermeture, blocs, textes, WAV et erreurs existantes ; qualification finale par l'utilisateur. **NOT RUN**. |

Parcours : **UC-GI01** consulter plusieurs cartes ; **UC-GI02** comprendre nom ou état inconnu ; **UC-GI03** lire la liste et choisir indépendamment Auto/CPU/GPU ; **UC-GI04** comprendre vide, partiel ou échec ; **UC-GI05** transcrire pendant une collecte ; **UC-GI06** fermer puis rouvrir ; **UC-GI07** conserver les résultats audio. Chaque UC renvoie respectivement à REQ/AC de même fonction dans le tableau. Les états d'inventaire ne deviennent pas des états de transcription.

## Domaine et états visibles

Une observation de carte porte une **identité Windows opaque**, un nom et un état facultatifs, et une présence observée `Current`, `Absent` ou `Unknown`. Le résultat porte les cartes `Current` normalisées, les causes d'incertitude et une couverture complète ou partielle. `Absent` ne contribue pas à la liste actuelle ; `Unknown` n'est pas affiché comme carte actuellement présente, mais rend la limite visible. La présence est distincte de l'activation et de la capacité de calcul. En particulier, une carte désactivée peut être actuellement présente.

Invariants : dédupliquer par identité et jamais par nom ; conserver une carte présente malgré des propriétés inconnues ; ne jamais convertir une présence inconnue en vraie ou fausse ; ne pas déduire l'absence du seul code Windows 24 ; garder le choix de calcul et tous les flux audio indépendants. Une collision de données pour une même identité rend les champs contradictoires inconnus et le résultat partiel.

La collecte passe de `non demandée` à `en cours`, puis à un seul résultat terminal `complet`, `partiel` ou `échec`. Seule une énumération terminée sans carte `Current` ni présence `Unknown` peut annoncer « aucun GPU présent ». Une erreur après certaines cartes conserve ces observations et signale le résultat partiel. Une erreur sans observation exploitable est un échec. Fermer révoque la publication de cette génération ; la nouvelle instance obtient une nouvelle génération. Ces données restent en mémoire, sans préférence ni historique persistant.

## Architecture proposée et preuves

| Propriétaire | Frontière et chemins proposés | Contrat |
|---|---|---|
| Domaine Rust | `crates/whisper-core/src/gpu_inventory.rs` | Types et normalisation purs, sans Windows. |
| Application Rust | `crates/whisper-core/src/gpu_inventory_app.rs` | Port de collecte, génération active, publication unique ; aucun effet sur les commandes de transcription. |
| Adaptateur Windows | `crates/whisper-adapters/src/gpu_inventory.rs` | Observation locale, erreurs et thread de collecte ; retourne des données Rust possédées. |
| Présentation | `crates/whisper-desktop/src/ui.rs` | Vue du résultat près de « Calcul », sans type WMI ni appel Windows bloquant. |
| Composition | `crates/whisper-desktop/src/root.rs` ; exports `lib.rs`, manifestes et `Cargo.lock` si nécessaires | Injecte l'adaptateur ; aucune décision métier dans la composition. |

**Matrice des dépendances et contrôles futurs (GI-D01).** Le domaine de `whisper-core` reste pur et ne dépend ni de WMI ni de `whisper-adapters`. L'application de `whisper-core` définit et possède le port de collecte ainsi que l'état de génération ; elle reçoit des données Rust possédées, sans référence à une connexion Windows. `whisper-adapters` réalise ce port sous `cfg(windows)` ; `root.rs` l'injecte dans l'application ; `ui.rs` lit uniquement la vue applicative et n'importe ni WMI ni le type concret de l'adaptateur. L'inventaire ne dépend pas des workers, du moteur de transcription ou des modules audio. Lors d'une implémentation future, vérifier le graphe Cargo et les features ciblées Windows, les imports et visibilités de chaque module, puis examiner les consommateurs de la vue et du port pour détecter toute dépendance inverse ou effet sur les commandes de transcription. Garder `unsafe_code=forbid` hors exceptions existantes du worker GPU.

**Source Windows proposée.** Interroger localement `ROOT\CIMV2`, classe `Win32_PnPEntity`, restreinte au GUID de classe Display `{4d36e968-e325-11ce-bfc1-08002be10318}`. Lire `DeviceID`, `Name`, `ConfigManagerErrorCode` et `Present` **sans filtrer `Present` dans la requête**, afin de signaler les valeurs inconnues. `Present=true` inclut la carte, même désactivée ou virtuelle ; `false` l'exclut ; valeur absente, illisible ou contradictoire rend l'inventaire partiel. Le code 22 signifie « désactivé » ; le code 24 reste ambigu. Microsoft documente `Present` pour la présence actuelle, mais cette propriété n'est pas prise en charge sur Windows Vista à 8.1. La cible déclarée dans `rust-toolchain.toml` est `x86_64-pc-windows-msvc`, dont le support Rust officiel commence à Windows 10 ; la couverture native de la source reste à vérifier. [Win32_PnPEntity](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-pnpentity), [classe Display](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/system-defined-device-setup-classes-available-to-vendors), [support des plateformes Rust](https://doc.rust-lang.org/rustc/platform-support.html).

**Bibliothèque candidate, non ajoutée.** `wmi = "=0.18.4"`, ciblée Windows, permettrait un accès Rust sûr via `exec_query` puis lecture indépendante des propriétés. Son graphe exact avec Rust 1.98.1 et le `Cargo.lock` courant n'a pas été compilé. Le dépôt interdit `unsafe_code` par défaut ; l'alternative SetupAPI directe exigerait du code système à autoriser séparément. Une énumération DXGI des adaptateurs disponibles ne démontre pas l'inclusion des cartes désactivées. [Manifest de `wmi` v0.18.4](https://raw.githubusercontent.com/ohadravid/wmi-rs/v0.18.4/Cargo.toml), [API de requête](https://docs.rs/wmi/0.18.4/src/wmi/query.rs.html), [SetupDiGetClassDevs](https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsw), [DXGI EnumAdapters](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgifactory-enumadapters).

**Concurrence proposée.** Une collecte asynchrone par processus, un thread et un résultat de capacité un. La connexion WMI reste sur son thread. `start` répond immédiatement (accepté, occupé ou échec), `poll` ne bloque pas l'UI. La fermeture abandonne la publication sans attendre le thread ni promettre l'annulation physique de l'appel Windows. Le parcours de transcription ne dépend ni de `start` ni de `poll`. La portée de ces contrats et le comportement quand WMI ne répond pas sont à vérifier lors de l'implémentation et de la revue.

## Traçabilité, risques et état de preuve

| Source produit | Exigences / AC | Domaine | Architecture / risque |
|---|---|---|---|
| GPU actuellement présents, Intel/désactivés/virtuels, anciens absents exclus | GI01–02 / GI01–02, GI04 | Identité, présence, couverture | WMI Display/Present ; omission ou faux inventaire complet |
| Liste informative près de Calcul | GI03 / GI03 | Présence ≠ calcul | UI ; confusion avec CUDA |
| Vide, partiel, erreur | GI02, GI04 / GI02, GI04 | Résultat terminal | Adaptateur/application ; perte de lignes ou erreur masquée |
| Aucun effet sur transcription | GI05–07 / GI05–08 | Génération et indépendance | Collecte non bloquante ; résultat tardif ou régression audio |

Preuve disponible : lecture du dépôt et des documentations primaires citées. **NOT RUN** : compilation du candidat `wmi`, essai de collecte locale, comparaison Windows avec Intel/désactivé/virtuel/ancien périphérique, tests de comportement et validation de la fenêtre. Une cible native adéquate et des jeux contrôlés devront être identifiés ; aucune modification du système n'est autorisée pour les créer dans ce cadrage. L'observation d'une carte présente désactivée avec `Present=true` reste à confirmer sur une cible réelle. Si Windows ne fournit pas cette distinction de façon fiable sur les cibles visées, revenir à une décision de périmètre plutôt que de supprimer silencieusement ces cartes.

Le plan d'implémentation documentaire sera ajouté après revue DESIGN du présent candidat Git. Aucun code, installation, téléchargement, exécution de test, modification système ni push ne découle de ce document.
