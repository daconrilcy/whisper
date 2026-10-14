Avancée : la sélection est réalisable avec CPAL/WASAPI déjà présent.
Blocage : la capture utilise aujourd'hui le défaut Windows ; aucun code n'est autorisé.
Décision : au redémarrage, une source préférée indisponible cède la capture au défaut avec un message ; en séance, une coupure arrête la transcription.

# Sélection du microphone — cadrage du 14 octobre 2026

**Base.** Branche `evo-mic-selection` créée depuis `evo-auto` à `1ec9de14f2666149f0f827c00d20e8102e5efe4f`. Cadrage seul. Préserver large-v3-turbo, imports WAV/MP3, blocs confirmés, sorties TXT/SRT/WAV et CPU/GPU/Auto. Hors périmètre : multi-micro, mixage, loopback, enregistrement autonome, bascule automatique en séance, niveaux, débruitage et moteur vocal.

## Existant et réemploi

| Composant | Actuel → adaptation |
|---|---|
| `crates/whisper-adapters/src/capture.rs` | CPAL 0.18.2/WASAPI ouvre `default_input_device`, convertit en mono 16 kHz et signale erreurs de flux/format. `start_device(Device)` ouvre déjà une source explicite ; réutiliser son chemin avec le tampon direct. |
| `crates/whisper-adapters/src/direct_live.rs` | Après `WorkerEvent::Ready`, ouvre toujours le défaut ; erreurs de capture arrêtent le flux. Raccorder source figée et arrêt informé, y compris CPU/GPU/Auto ; vérifier la finalisation partielle, actuellement différente selon le moteur. |
| `crates/whisper-desktop/src/ui.rs`, `crates/whisper-core/src/ports.rs` | La ComboBox du calcul est réutilisable ; `LiveRequest` ne porte pas de micro. Ajouter choix, actualisation et messages au seul parcours micro. |
| `crates/whisper-core/src/path_settings.rs`, `crates/whisper-adapters/src/path_settings.rs` | `PathSettingsApp` et `WindowsPathPort` mémorisent les chemins dans `settings.json`. Étendre ce stockage avec l'ID préféré, sans perdre les anciens chemins ni effacer l'ID lors d'un recours temporaire au défaut. |

**Capacité CPAL inutilisée.** `HostTrait::input_devices/device_by_id/default_input_device`, `DeviceTrait::description/id` et `DeviceId` (`Display/FromStr`) couvrent liste, nom, identité et réouverture. Sous WASAPI, l'ID provient de `IMMDevice::GetId`; il reste utilisable après reconnexion si Windows le conserve, mais peut changer avec le pilote. Le défaut actuel est le rôle `eConsole`. Aucun nouvel ajout de dépendance prévu. L'historique local (`6da33583`, `935bd7cf`, `6799cd94`) ne contient pas de sélection utilisateur déjà raccordée.

**PC observé.** Windows indique quatre entrées présentes : NewEye 60s, iPhone de Mantu Hands-Free HF Audio, Jabra EVOLVE 20 MS et WF-1000XM5. L'application n'énumère pas encore CPAL : ouverture individuelle et correspondance avec cette liste restent non attestées.

## Contrat et acceptation

L'application conserve une **préférence** `Défaut Windows | ID stable`, distincte de la **source effective**. L'adaptateur découvre les seules entrées WASAPI actives ; l'application pilote catalogue et résolution, `root.rs` injecte les ports et l'UI ne reçoit que nom, ID opaque, compatibilité et statut. Un format incompatible est signalé et non sélectionnable. Réénumérer à l'ouverture des paramètres audio et via « Actualiser », sans surveillance permanente ; une actualisation ne change ni préférence ni source effective.

À chaque nouveau départ en mode défaut, résoudre `default_input_device().id()` puis ouvrir l'endpoint **concret** par `device_by_id` : le handle virtuel `DefaultInput` ne doit pas changer la source active. Une source explicite sélectionnée reste fixe pendant la séance. **À la réouverture seulement**, si la source préférée est absente ou incompatible selon l'inventaire, utiliser le défaut Windows avec un message et garder l'ID préféré. Si le défaut est lui-même inaccessible, refuser le départ avec une erreur. Un refus d'accès révélé seulement à l'ouverture de la capture produit une erreur, sans autre bascule.

| Critère | Scénario et test futur |
|---|---|
| CA1–CA2 | Inventaires 0/1/n, homonymes et format incompatible : liste lisible, option exacte « Microphone par défaut de Windows », identités distinguables, erreur d'énumération distincte du vide ; tests catalogue/UI. |
| CA3 | Deux sources à signaux distincts : seule la source choisie alimente CPU/GPU/Auto. Changement du défaut Windows pendant une séance : source active fixe ; prochain départ : nouveau défaut. Tests injectés puis essai Windows. |
| CA4 | Premier lancement et ancien `settings.json` sans ID → défaut. Choix explicite conservé après fermeture/redémarrage malgré ordre changé ; reconnexion avec même ID → choix retrouvé ; nouvel ID → défaut annoncé et nouvelle sélection explicite. Tests persistance/identité. |
| CA5 | Micro connecté après lancement : visible après réouverture des paramètres ou « Actualiser », sans changement automatique de source ; test d'inventaires successifs. |
| CA6 | Préféré absent/incompatible à la réouverture : défaut annoncé, ID conservé. Déconnexion en transcription : arrêter et informer, sans repli ni changement de préférence, en CPU/GPU/Auto. Accès refusé seulement à l'ouverture : erreur claire. Tests des moments et erreurs. |
| CA7 | Rejouer micro et imports WAV/MP3, modèle, blocs et sorties ; aucun changement de politique CPU/GPU/Auto. Tests existants et essai final dans la fenêtre réservé à l'utilisateur. |

## Plan proposé — huit tâches, aucun code autorisé

1. Définir préférence, source effective et états dans `whisper-core/LiveRequest` — CA2–CA4/CA6 ; tests d'identité et défaut initial.
2. Inventorier entrées/nom/ID/compatibilité dans `capture.rs` via CPAL — CA1–CA2/CA5 ; tests 0/1/n, homonymes et format.
3. Résoudre l'ID et ouvrir le flux direct concret — CA3/CA6 ; tests deux sources, défaut modifié et indisponibilité.
4. Transmettre la source à `DirectLive` sans toucher au worker — CA3/CA7 ; tests CPU/GPU/Auto et absence de bascule.
5. Étendre l'unique stockage des préférences — CA4/CA6 ; tests migration, redémarrage, ID conservé ou changé et recours annoncé au défaut.
6. Ajouter ComboBox, « Actualiser » et messages dans `ui.rs` — CA1–CA2/CA5–CA6 ; tests liste, reconnexion et choix conservé.
7. Arrêter et finaliser après coupure, sans changement de source/préférence — CA6–CA7 ; tests erreurs et sorties CPU/GPU/Auto.
8. Rejouer build/tests et parcours natifs micro/import — CA1–CA7 ; tests reconnexion même/nouvel ID, défaut, qualité et régressions.

**Risques et décisions.** Une identité Windows changée ne doit jamais désigner un autre micro : garder l'ancien ID et demander une re-sélection du nouvel endpoint. L'inventaire ne prouve pas la permission d'ouvrir : si le refus n'est connu qu'au départ, afficher l'erreur sans repli. Les sorties partielles après coupure doivent être vérifiées selon le moteur. Q1 validée : arrêter/informer en séance, sans bascule. Q2 validée : actualiser à l'ouverture et sur demande, sans surveillance. Q3 validée : au redémarrage seulement, source préférée indisponible selon l'inventaire → défaut annoncé, préférence conservée. Ces décisions n'autorisent pas l'implémentation.
