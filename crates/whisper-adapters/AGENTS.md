# whisper-adapters

Complète le `AGENTS.md` racine, notamment ses autorisations et sa garde permanente. Cette carte de l'existant ne donne aucune autorisation de refonte ; la confronter aux sources avant une modification.

## Rôle et carte actuelle

Exécuter les ports du cœur : processus, IPC, disque, capture Windows et services techniques. Les politiques métier restent dans le cœur.

| Modules | Responsabilité |
|---|---|
| `worker_ipc.rs` | Façade publique, réexports et types partagés du transport et du service |
| `worker_ipc/ipc/`, `worker_ipc/backends/`, `worker_ipc/parakeet_unified/` | Trames et transport, sélection/routage, exécution des sessions unifiées |
| `worker_ipc/runtime/` | `ImportRuntime`, effets, import, direct, file, reprise, doublons et polling |
| `worker_ipc/publication/`, `worker_ipc/service/`, `worker_ipc/support.rs` | `PublicationGate`, service asynchrone et final automatique, utilitaires internes |
| `archive.rs`, `archive/` | Contrats d'archive ; import/direct, publication, reprise, identité/sources, stockage, journaux et retrait |
| `queue_store`, `recovery` | File persistée et récupération |
| `staging`, `source_binding`, `windows_file_identity` | PCM, réservation des sources et identité des fichiers |
| `capture`, `microphone_probe`, `vad` | WASAPI, niveau et spectre, activité vocale |
| `direct_live.rs`, `direct_live/` | Direct Turbo : capture, WAV, sorties et worker |
| `parakeet_import`, `parakeet_import/historical_runtime.rs` | Client processus Parakeet et routage des runtimes historiques |
| `parakeet_live.rs`, `parakeet_live/` | Capture Parakeet, cycle de vie, inférence, événements, fenêtres et traces |
| `parakeet_pipeline`, `parakeet_audio`, `parakeet_document`, `parakeet_confirmation` | Pipeline, audio durable, document et confirmation Parakeet |
| `gpu_inventory`, `model_catalog`, `supervisor` | GPU, modèles, exécutables et diagnostics |
| `decoder`, `path_settings`, `journal` | Décodage, préférences et réexport |

## Invariants à préserver

- Conserver le filtrage des événements par `JobId`, `Generation` et, lorsque le protocole le porte, `instance_id`. Un résultat ancien ne doit pas atteindre la session courante.
- Préserver `PublicationGate` dans `worker_ipc/publication/mod.rs`, les ACK de persistance, la publication atomique et les contrôles d'intégrité propres à chaque parcours. Aucun export utilisateur nouveau ne contourne le chemin de publication applicable. Distinguer cette garde de `archive/publication/` (intentions, réservations, index, installation des exports et catalogue).
- Conserver les vérifications SHA-256 et d'identité aux endroits exigés par le contrat ; ne pas les rendre optionnelles pour une recette ni inventer un recalcul universel qui invalide le cache validé.
- Conserver les bornes des canaux et la contre-pression. Ne pas introduire un canal non borné dans les flux audio ou workers.
- Éviter les dépendances circulaires. Ne pas ajouter de dépendance d'`archive` vers `worker_ipc` ; une extraction d'un diagnostic commun doit rester dans le périmètre accepté.

## Placement et tests

Placer une évolution dans le sous-module responsable déjà extrait. Conserver les façades et leurs réexports ; ne pas reconstituer les anciens blocs monolithiques dans `worker_ipc.rs`, `archive.rs` ou `parakeet_live.rs`. `archive.rs` et `source_binding.rs` restent volumineux : éviter de les agrandir sans justification, sans imposer une extraction disproportionnée à un correctif limité.

Préserver la séparation entre `archive/recovery/`, `archive/cleanup/` et `archive/publication/`, ainsi que les contrôles d'identité avant retrait. Pour Parakeet micro, préserver l'écriture audio avant inférence et la gestion du retard dans le WAV ; ne pas déplacer les règles métier du cœur dans les adaptateurs.

Nouveaux tests dans les fichiers séparés des sous-modules, `worker_ipc/tests/`, `archive/tests/` ou `tests/` selon leur visibilité. Préserver les caractérisations existantes. Partager les faux workers dans un support accessible aux tests concernés ; ne pas exposer d'API produit uniquement pour les fixtures. Les diagnostics persistés relèvent actuellement de `supervisor` ; ne pas confondre leur routage avec les traces spécifiques des parcours.
