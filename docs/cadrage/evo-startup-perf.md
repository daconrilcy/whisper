Avancée : la fenêtre apparaît vite sur les essais locaux, mais l'attente avant usage et avant capture reste à réduire.
Blocage : les 19 s signalées ne sont pas reproduites ni décomposables avec les horodatages actuels ; la capture réelle exige une mesure dans la fenêtre.
Décision : arbitrer le préchargement, la durée de conservation du moteur et la capture pendant son chargement avant tout développement.

# Réactivité au démarrage et du micro — cadrage du 14 octobre 2026

**Base.** `evo-startup-perf` part de `ad7c5955` (`evo-mic-selection`), descendant d'Auto `1ec9de14`, GPU `994b660a` et micro direct. CA1 <2 s pour interface utilisable, CA2 <3 s pour premiers échantillons, CA3 réactivité, CA4 audio conservé, CA5 réemploi, CA6 compatibilité et CA7 qualité/débit sont des **cibles à qualifier**. Essai final dans la fenêtre réservé à l'utilisateur ; aucun code autorisé.

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

## Architecture proposée et arbitrages

**Recommandation conditionnelle : B + C.** Garder la sonde en arrière-plan et permettre les actions CPU pendant celle-ci. Partager **le même contexte/worker** entre sonde et séances compatibles : protocole IPC multi-séance, identité remise à zéro, état `Préparation → Prêt → Occupé → Inactif/Libéré`, invalidation si modèle/mode change. Auto réatteste le GPU à chaque séance, puis autorise une seule tentative CPU avant inférence. Domaine : préférence/garanties ; application : séquence/annulation ; adaptateurs et workers : capture/moteur/WAV ; UI : états. Un seul propriétaire du worker, aucun `Ready` issu d'un résultat périmé.

Pour le micro, ouvrir la source choisie et créer le WAV **avant** l'attente moteur ; n'afficher « Capture en cours — moteur en initialisation » qu'après le premier bloc audio admis. Mettre les blocs en ordre dans le WAV existant avec offsets et un tampon RAM borné, puis les remettre au moteur attesté sans trou ni doublon ; afficher ensuite « Capture et transcription opérationnelles ». Si WAV, file ou périphérique échoue, révoquer immédiatement l'état prêt, arrêter et conserver ce qui est confirmé. Un arrêt pendant chargement ferme la capture et conserve le WAV ; la poursuite du chargement et la production TXT/SRT suivent **Q4**, sans annoncer du texte non transcrit. Le code actuel abandonne un arrêt avant `Ready` (`direct_live.rs:168-178`) : adaptation indispensable. Débit disque, bornes de mémoire, durée d'attente et réemploi CUDA doivent être prouvés avant de promettre CA2/CA4. Aucune reprise après panne ajoutée.

| Décision utilisateur | Options et compromis | Recommandation |
|---|---|---|
| Q1 Préparation moteur | A demande seule : faible RAM/VRAM, premier clic lent ; B arrière-plan : clic rapide si prêt, ressources au repos ; combinée : préparation utile mais annulation/coordination. | B + demande si sonde non finie, sans bloquer UI. |
| Q2 Entre séances | permanent : clic rapide, RAM/VRAM durables ; destruction : sobre, délai répété ; expiration : compromis et rechargement après délai. | expiration après inactivité (5 min **à valider**), libération à la fermeture/changement incompatible. |
| Q3 Audio avant moteur | attendre `Ready` : simple, CA2 probablement manqué ; capturer/WAV tamponné : parole tôt, contraintes disque/erreur/arrêt. | capture anticipée avec état clair et arrêt explicite à saturation ; valider durée/garantie. |
| Q4 Arrêt avant moteur prêt | attendre et transcrire l'audio capté : texte tardif ; finaliser le WAV seul : résultat incomplet, réimport manuel. | attendre la transcription après fermeture de capture ; si moteur échoue, conserver WAV et signaler l'incomplet. |

## Plan borné — implémentation ultérieure seulement

| Tâche | Composants, réemploi, preuve et risque | CA |
|---|---|---|
| 1 | Établir protocole avant/après : lancement, rendu, action UI, premier bloc, `Ready`, premier texte ; Windows froid/chaud, CPU/GPU/Auto, 2 séances ; éviter les faux jalons. | 1–7 |
| 2 | `root/ui` : retirer l'attente globale de sonde, conserver choix GPU confirmé/CPU et messages ; essais interaction/erreur ; risque choix prématuré. | 1,3,6 |
| 3 | `worker_ipc` et workers CPU/GPU : protocole multi-séance, conserver contexte/modèle, réinitialiser état/identité, transférer sonde, réattester Auto ; prouver 2 séances sans rechargement, mesurer RAM/VRAM ; risque contexte périmé. | 1,5–7 |
| 4 | `core/application` : états séparés capture/moteur et génération/annulation ; tests transitions ; risque succès mensonger. | 2–4 |
| 5 | `capture/direct_live` : ouvrir WASAPI, convertir et écrire WAV avant moteur ; premier bloc admis comme signal ; tests ordre/erreur ; risque pertes/xrun. | 2,4,6 |
| 6 | `direct_live/worker_ipc` : rejouer le backlog WAV séquencé vers moteur prêt, bornes et arrêt à saturation ; tests zéro perte/doublon, stop/échec pré-Ready ; risque mémoire/disque. | 4,6,7 |
| 7 | `compute_policy/worker_ipc` : Auto réévalué par séance, unique repli CPU avant inférence, modes manuels fixes ; tests erreurs ; risque bascule tardive. | 5–7 |
| 8 | `ui` et validation : deux états, moteurs/erreurs visibles ; comparer WAV/TXT/SRT, qualité, débit et premier texte GPU <15 s, CPU/GPU/Auto et séances successives, caches froid/chaud ; qualification finale par l'utilisateur. | 1–7 |

`cargo build --offline --locked --workspace`, `cargo build --offline --locked --release --workspace` et `cargo test --offline --locked --workspace` : **réussis** sur cette branche avant modification documentaire ; tests natifs de capture ignorés par défaut. CA1/CA2 et CA4 en conditions réelles restent **non qualifiés**.
