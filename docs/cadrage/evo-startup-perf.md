Avancée : les quatre choix de réactivité sont validés ; la fenêtre apparaît vite sur les essais locaux.
Blocage : les 19 s signalées ne sont pas reproduites ; utilisabilité et capture réelle restent à mesurer.
Décision : préparer seulement les capacités au lancement, charger le modèle à la demande et conserver le moteur cinq minutes après traitement.

# Réactivité au démarrage et du micro — cadrage du 14 octobre 2026

**Base.** `evo-startup-perf` part de `ad7c5955` (`evo-mic-selection`), descendant d'Auto `1ec9de14`, GPU `994b660a` et micro direct. Q1–Q4 acceptées par l'utilisateur le 14 octobre 2026 ; aucune implémentation autorisée. CA1 <2 s pour interface utilisable, CA2 <3 s pour premiers échantillons, CA3 réactivité, CA4 audio conservé, CA5 réemploi, CA6 compatibilité et CA7 qualité/débit restent des **cibles à vérifier sur le produit**. Essai final dans la fenêtre réservé à l'utilisateur.

## Observations et causes

Trois essais le 14 octobre sur ce PC (GTX 1080 Ti, modèle local présent), **Release reconstruit**, caches système/fichier actifs, 2 s entre instances. Chronomètre PowerShell depuis `Start-Process`, interrogation ~50 ms. Handle/réponse Windows ne prouvent ni rendu ni action réussie ; disparition du processus GPU ne date pas son `Ready`. Aucun vrai démarrage à froid mesuré.

| Événement / condition | Essais 1 / 2 / 3 après lancement | Portée |
|---|---:|---|
| Fenêtre créée | 368 / 63 / 77 ms | mesuré, utilisabilité non prouvée |
| Fenêtre déclarée répondante | 458 / 143 / 152 ms | mesuré, commandes encore possiblement désactivées |
| Worker de sonde GPU observé → disparu | 78→2163 / 30→2195 / 21→2181 ms | processus mesuré, ~2,1 s d'activité observée |
| Interface utilisable ; capture effective ; moteur prêt | **non mesurés** | pas d'horodatage/interaction fiable disponible |
| Valeurs utilisateur : lancement→interface ; clic micro→parole | 19 s ; 19 s | observations rapportées, conditions/caches et jalons exacts inconnus |

**Code certain, durées par phase inconnues.** La sonde CUDA est *déjà asynchrone* (`desktop/src/root.rs:59-73`) ; elle charge le modèle puis détruit son worker (`adapters/src/worker_ipc.rs:365-415`). Micro/import restent désactivés pendant cette sonde (`desktop/src/ui.rs:294,482-491`). Préférences et inventaire WASAPI sont synchrones à la construction (`desktop/src/root.rs:22-30`, `desktop/src/ui.rs:104-114`, `adapters/src/capture.rs:204-217`), sans coût isolé. Au clic micro, nouveau worker et chargement ; `DirectLive` ouvre WASAPI seulement après `Ready` (`adapters/src/direct_live.rs:366-402,677-695`), puis détruit le worker (`:808-812`). Les workers CPU/GPU terminent après une séance et refusent une nouvelle commande (`worker-cpu/src/main.rs:1150-1160`, `worker-gpu/src/main.rs:1811-1821`). Ces séquences expliquent une attente et une répétition, **sans attribuer les 19 s au GPU**. Moteur `Ready`, flux ouvert et premier bloc sont distincts, sans trois horodatages existants. `WHISPER_CPU_TIMINGS` ne mesure que le chargement CPU et perturbe l'inférence diagnostique.

**Réemploi.** `AsyncImportIo` est asynchrone ; capture, conversion, WAV et séquences existent (`adapters/src/worker_ipc.rs`, `capture.rs`, `direct_live.rs`). Le tampon vaut 10 min à 16 kHz (`direct_live.rs:26`), mais capture/WAV commencent après `Ready`. Saturation CPAL peut perdre des blocs : CA4 exige arrêt signalé sans poursuite trompeuse. Auto garde son unique repli CPU **avant traitement audio** (`docs/cadrage/evo-auto.md`). Le bilan GPU antérieur donne 0,26× GPU/2,30× CPU sur 60 s et qualité comparable.

## Architecture retenue et décisions Q1–Q4

**Q1 — combinée, acceptée.** Afficher l'interface sans attendre GPU ; découvrir ses capacités en arrière-plan **sans charger le modèle complet**. Charger large-v3-turbo à la première demande utile, dans un worker conservable : supprimer la sonde actuelle qui charge puis détruit le modèle. CPU reste utilisable pendant la découverte ; le micro démarre sa capture même si la vérification GPU/Auto continue. Une découverte légère ne vaut pas attestation CUDA. **Q6 structurante**, owner architecte/coordinateur, tâche 2 : prouver une attestation sans modèle compatible avec la présélection actuelle ; si impossible, revenir au cadrage utilisateur avant de changer cette présélection. Un résultat tardif ne remplace jamais le choix utilisateur. Auto vérifie avant **chaque** séance et ne replie qu'une fois sur CPU avant la première inférence, jamais pendant transcription.

**Q2 — cinq minutes, accepté.** Adapter l'IPC et les workers CPU/GPU mono-séance pour conserver modèle et contexte entre demandes compatibles, avec identité/état de séance réinitialisés. Démarrer le délai seulement après transcription, drainage, WAV et TXT/SRT finalisés ; aucune expiration pendant démarrage, transcription ou WAV en attente. Détruire proprement à échéance et libérer les ressources incompatibles lors d'un changement CPU/GPU. Interdire chargements concurrents et workers orphelins ; un seul propriétaire gère `Préparation → Prêt → Occupé ↔ Inactif → Libéré`, erreur → libération sûre. Les cinq minutes pourront être ajustées après mesures RAM/VRAM.

**Q3 — capture anticipée, acceptée.** Afficher « Microphone en cours d'initialisation », puis « Capture active — moteur en préparation » après audio admis dans le **WAV temporaire**, puis « Capture et transcription opérationnelles ». Horodater séparément premier échantillon capturé et premier bloc conservé. WAV avec offsets et buffers bornés remet l'audio dans l'ordre au moteur pendant que la capture continue. Si la conservation échoue, arrêter aussitôt, signaler l'incomplet et garder le WAV déjà admis. **Q5 structurante**, owner architecte/coordinateur, tâche 1 avant tâches 5–6 : dimensionner RAM/disque, marge de saturation et attente sans progrès, sans délai d'abandon arbitraire ; revenir à l'utilisateur si la borne réduit CA4. Aucun trou, doublon ou réordonnancement silencieux.

**Q4 — arrêt pré-Ready, accepté.** Arrêter immédiatement la capture, vider les blocs admis et finaliser le WAV, puis poursuivre le chargement et transcrire cet enregistrement. Fermer la fenêtre suit le même arrêt, puis attend traitement/exports ; worker protégé jusque-là. Si le moteur échoue, conserver le WAV et afficher l'erreur. Le code actuel abandonne un arrêt avant `Ready` (`direct_live.rs:168-178`) : adaptation indispensable. Domaine : garanties/préférence ; application : états/séquence ; adaptateurs et workers : WASAPI/IPC/moteur/WAV ; UI : états/erreurs. Pas de reprise après panne ajoutée.

## Plan borné — implémentation ultérieure seulement

| Tâche | Composants, réemploi, preuve et risque | CA |
|---|---|---|
| 1 | Instrumenter première interaction effective, clic→premier échantillon et bloc conservé, moteur `Ready`, premier bloc confirmé affiché, RAM/VRAM ; comparer froid/chaud, CPU/GPU/Auto, séances successives ; résoudre Q5 avant capture anticipée ; risque faux jalon. | 1–7 |
| 2 | `root/ui` : découverte légère asynchrone sans modèle ; établir attestation CUDA/presélection possible sans modèle, respecter choix utilisateur tardif ; essais interaction/erreur ; risque confirmation infondée. | 1,3,6 |
| 3 | `worker_ipc`, workers CPU/GPU : modèle à la demande, protocole multi-séance, état/identité réinitialisés, garde d'activité, expiration 5 min après exports ; prouver 2 séances sans reload, mesurer RAM/VRAM ; risque contexte périmé. | 1,5–7 |
| 4 | `core/application` : états capture/moteur indépendants, génération/annulation et protection worker ; tests transitions ; risque destruction en cours. | 2–4 |
| 5 | `capture/direct_live` : ouvrir WASAPI, convertir et écrire WAV temporaire avant moteur ; premier bloc admis comme signal ; tests ordre/erreur ; risque pertes/xrun. | 2,4,6 |
| 6 | `direct_live/worker_ipc` : backlog WAV séquencé, bornes et arrêt à saturation ; stop pré-Ready finalise WAV puis transcrit, échec conserve WAV ; tests audio début/silence/fin, ordre, duplication, saturation ; risque mémoire/disque. | 4,6,7 |
| 7 | `compute_policy/worker_ipc` : Auto réévalué par séance, unique repli CPU avant inférence, modes manuels fixes ; tests erreurs ; risque bascule tardive. | 5–7 |
| 8 | `ui` et validation : trois états/erreurs ; micro explicite/défaut/indisponible, imports WAV/MP3, large-v3-turbo et paramètres inchangés ; comparer sorties, qualité, débit, premier texte GPU <15 s **depuis clic avec parole immédiate**, CPU/GPU/Auto et séances successives ; essai final utilisateur. | 1–7 |

`cargo build --offline --locked --workspace`, `cargo build --offline --locked --release --workspace` et `cargo test --offline --locked --workspace` : **réussis** sur cette branche avant modification documentaire ; tests natifs de capture ignorés par défaut. CA1/CA2 et CA4 en conditions réelles restent **non qualifiés**.
