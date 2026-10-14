Avancée : le choix d'un microphone est réalisable avec la bibliothèque audio déjà présente.
Blocage : le moteur utilise aujourd'hui seulement le microphone Windows par défaut ; les deux choix produit ci-dessous attendent arbitrage.
Décision : retenir l'arrêt informé en cas de déconnexion et l'actualisation à l'ouverture des paramètres ou sur demande.

# Sélection du microphone — cadrage du 14 octobre 2026

**Base et portée.** Branche locale `evo-mic-selection` créée depuis `evo-auto` au commit `1ec9de14f2666149f0f827c00d20e8102e5efe4f` (arbre initial propre). Cadrage seul ; tests de la nouvelle fonction non exécutés. Garder imports WAV/MP3, modèle large-v3-turbo, blocs confirmés, sorties TXT/SRT/WAV, et calcul CPU/GPU/Auto. Aucun nouvel objectif de vitesse CPU n'est introduit. Exclus : plusieurs micros simultanés, mixage, loopback, enregistrement autonome, changement automatique en séance, niveaux, débruitage et moteur vocal.

## Fonctionnement et réemploi

| État | Composant et rôle actuel | Adaptation / risque |
|---|---|---|
| Utilisé | `crates/whisper-adapters/src/capture.rs` : CPAL `=0.18.2`, hôte WASAPI ; `start_direct_default` ouvre `default_input_device`, convertit en mono 16 kHz et remonte erreurs de format, ouverture, flux ou saturation. `start_device(Device)` sait ouvrir un objet explicite, mais utilise le tampon de l'ancien parcours live. | Ajouter ouverture directe par source explicite avec le même tampon et les mêmes erreurs ; exclure les sorties/loopback. |
| Utilisé | `crates/whisper-adapters/src/direct_live.rs::DirectLive::handle_worker` ouvre le défaut après `WorkerEvent::Ready`, pour CPU/GPU/Auto. `drain_capture` arrête le flux en erreur ; `fail` n'a pas la même finalisation partielle sur tous les moteurs. | Résoudre le choix avant capture et conserver l'identité de séance ; traiter la finalisation en cas de coupure sans promettre de sorties non vérifiées. |
| Utilisé | `crates/whisper-desktop/src/ui.rs::start_live` émet `StartDirectLiveConfigured` ; `crates/whisper-core/src/ports.rs::LiveRequest` ne porte pas de choix micro. L'UI a déjà une `egui::ComboBox` pour le calcul. | Ajouter le sélecteur et transmettre le choix seulement au parcours micro ; figer la source pendant une séance. |
| Utilisé | `crates/whisper-core/src/path_settings.rs::PathSettingsApp`, `crates/whisper-adapters/src/path_settings.rs::WindowsPathPort`, `%LOCALAPPDATA%/Whisper/settings.json` : préférences de chemins sauvegardées ; `root.rs::compose` les charge. | Étendre le schéma avec une préférence micro facultative, compatible avec les réglages existants ; conserver l'ID même si la source manque. Risque de perte du choix lors d'une sauvegarde des chemins. |
| Disponible mais inutilisé | CPAL 0.18.2 fournit `HostTrait::input_devices`, `device_by_id`, `default_input_device`, `DeviceTrait::description/id` et `DeviceId` sérialisable en texte (`Display`/`FromStr`). WASAPI utilise `IMMDevice::GetId` et énumère les endpoints actifs ; le défaut est le rôle Windows `eConsole`. | Réutiliser ces API dans l'adaptateur ; marquer non sélectionnables les formats incompatibles avec la capture actuelle, vérifier l'ouverture au départ (permission non garantie). Aucune dépendance prévue. |

**Historique.** `6da33583` introduit la capture ; `935bd7cf` adapte son tampon ; `6799cd94` ajoute la persistance des chemins. Recherche locale dans code et historique : aucune énumération, préférence micro, détection de branchement ou sélection UI déjà raccordée. Auto est présent dans le code de cette base ; son ancien document de cadrage ne décrit pas seul son état courant.

**PC observé.** `Get-PnpDevice -Class AudioEndpoint -PresentOnly` montre quatre entrées `SWD\MMDEVAPI\{0.0.1.00000000}` en état OK : `Microphone (NewEye 60s)`, `Microphone (iPhone de Mantu Hands-Free HF Audio)`, `Headset Microphone (2- Jabra EVOLVE 20 MS)`, `Headset (WF-1000XM5)`. Relevé Windows seulement : l'application n'a pas encore de commande d'énumération CPAL, donc correspondance moteur, défaut effectif et ouverture individuelle non attestés ; aucun réglage système modifié.

## Architecture, critères et scénarios

La préférence applicative est `Défaut Windows` ou `Identifiant stable` ; elle est distincte du moteur de calcul. L'application pilote les ports de catalogue et de résolution ; l'adaptateur WASAPI les réalise hors callback, `root.rs` les injecte et l'UI ne reçoit que leurs résultats métier. Le catalogue donne nom, ID, défaut et compatibilité ; une entrée incompatible est visible mais non sélectionnable. Au début de chaque capture, `Défaut Windows` est résolu par `default_input_device().id()`, puis `device_by_id` ouvre l'endpoint **concret**, jamais le handle virtuel `DefaultInput` qui peut réagir à un changement de défaut. Une source explicite manquante ou refusée produit un message sans substitution. L'application conserve la préférence même absente ; l'UI distingue les homonymes. CPU/GPU/Auto reste orthogonal ; la capture démarre après `Ready` du moteur.

| Critère | Comportement et vérification future |
|---|---|
| CA1–CA2 | 0 entrée : défaut visible, message « aucun microphone » au départ ; 1 ou plusieurs : noms, défaut et compatibilité ; entrée incompatible désactivée avec motif ; homonymes distingués par suffixe d'ID unique. Tests d'inventaires 0/1/n, formats incompatibles et UI. |
| CA3 | Deux sources à signaux distincts : l'explicite alimente réellement la transcription avec CPU/GPU/Auto ; un changement du défaut Windows ne change pas la séance active mais s'applique au départ suivant. Test de port injecté et essai natif. |
| CA4 | Au premier lancement : défaut. Après choix explicite, fermeture/redémarrage et ordre de liste modifié : même ID restauré. Tests de lecture/écriture et migration de `settings.json`. |
| CA5 | Source branchée après lancement : visible et sélectionnable à la réouverture des paramètres ou après actualisation, sans perdre le choix courant. Test d'inventaire variable. |
| CA6 | Source explicite absente, désactivée, occupée ou accès Windows refusé avant capture : cause claire, aucun repli ; préférence gardée. Tests d'erreur et essai natif. |
| CA7 | Imports, modèle, résultats, arrêt, choix CPU/GPU/Auto conservés. Tests existants et parcours Windows par l'utilisateur. |

Une coupure pendant la capture doit cesser l'usage de cette source et signaler la séance incomplète ; la politique de finalisation dépend de Q1. La source active reste figée même si la liste ou le défaut Windows change en séance.

## Plan proposé — implémentation non autorisée

1. Définir préférence `Défaut/ID`, états absent/refusé et contrat de sélection dans `whisper-core` ; réutiliser `LiveRequest` — CA2–CA4/CA6 ; tests de domaine.
2. Ajouter inventaire WASAPI des seules entrées, nom/ID/défaut/compatibilité dans `capture.rs` ; réutiliser CPAL — CA1–CA2/CA5 ; tests 0/1/n, format incompatible et comparaison native.
3. Résoudre le défaut en ID puis ouvrir l'endpoint concret avec son tampon direct existant ; réutiliser `device_by_id` et `start_device_with_buffer` — CA3/CA6 ; tests deux sources, défaut changé en séance et erreurs.
4. Raccorder choix et erreurs à `DirectLive`/application, sans toucher au worker de reconnaissance — CA3/CA6–CA7 ; tests CPU/GPU/Auto et absence de repli.
5. Étendre les préférences dans `PathSettingsApp`/`WindowsPathPort` avec migration non destructive — CA4 ; tests ancien JSON, redémarrage et sauvegarde des chemins.
6. Ajouter liste déroulante et actualisation dans `ui.rs`, homonymes et statut absent — CA1–CA2/CA5–CA6 ; tests UI 0/1/n, branchement et choix conservé.
7. Selon Q1, raccorder arrêt informé/finalisation de coupure dans `direct_live.rs` — CA6–CA7 ; tests coupure CPU/GPU/Auto, sorties et absence de bascule.
8. Rejouer build/tests et parcours natifs micro/import par l'utilisateur — CA1–CA7 ; vérifier défaut changé, préférence restaurée, qualité et régressions.

**Risques.** ID d'endpoint modifié par réinstallation/pilote : garder l'ID préféré et montrer l'absence, sans substitution. L'accès Windows peut refuser un endpoint pourtant listé. Le rôle WASAPI actuel est `eConsole`, à conserver. La coupure en séance peut produire des fichiers partiels différemment selon le moteur ; le plan devra vérifier ce résultat. Aucun essai de capture individuelle n'a été réalisé dans ce cadrage.

**Arbitrages produit structurants, autorité utilisateur.** Q1 déconnexion pendant transcription : **A**, arrêter et informer (recommandé, source consentie conservée) ; **B**, basculer vers le défaut (changerait la source sans accord et contredirait le hors périmètre). Q2 actualisation : **A**, à l'ouverture des paramètres et via « Actualiser » (recommandé) ; **B**, surveillance permanente (complexité supplémentaire). Réponses nécessaires avant tâches 6–7 ; design et plan proposés, non prêts à exécuter. Aucun code autorisé.
