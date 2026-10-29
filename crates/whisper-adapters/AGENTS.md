# whisper-adapters

Complète le `AGENTS.md` racine, notamment ses autorisations et sa garde permanente. Les découpages ci-dessous ne donnent aucune autorisation de refonte.

## Rôle et carte actuelle

Exécuter les ports du cœur : processus, IPC, disque, capture Windows et services techniques. Les politiques métier restent dans le cœur.

| Modules | Responsabilité |
|---|---|
| `worker_ipc` | Transport workers, `ImportRuntime`, service asynchrone et publication |
| `archive` | Archive durable, publication, reprise et retrait des traitements |
| `queue_store`, `recovery` | File persistée et récupération |
| `staging`, `source_binding`, `windows_file_identity` | PCM, réservation des sources et identité des fichiers |
| `capture`, `microphone_probe`, `vad` | WASAPI, niveau et spectre, activité vocale |
| `direct_live` | Direct Turbo |
| `parakeet_import`, `parakeet_live`, `parakeet_pipeline`, `parakeet_audio`, `parakeet_document`, `parakeet_confirmation` | Chaîne Parakeet |
| `gpu_inventory`, `model_catalog`, `supervisor` | GPU, modèles, exécutables et diagnostics |
| `decoder`, `path_settings`, `journal` | Décodage, préférences et réexport |

## Invariants à préserver

- Conserver le filtrage des événements par `JobId`, `Generation` et, lorsque le protocole le porte, `instance_id`. Un résultat ancien ne doit pas atteindre la session courante.
- Préserver `PublicationGate` dans `worker_ipc.rs`, les ACK de persistance, la publication atomique et les contrôles d'intégrité propres à chaque parcours. Aucun export utilisateur nouveau ne contourne le chemin de publication applicable. Le module `publication/` n'existe pas actuellement.
- Conserver les vérifications SHA-256 et d'identité aux endroits exigés par le contrat ; ne pas les rendre optionnelles pour une recette ni inventer un recalcul universel qui invalide le cache validé.
- Conserver les bornes des canaux et la contre-pression. Ne pas introduire un canal non borné dans les flux audio ou workers.
- Éviter les dépendances circulaires. Ne pas ajouter de dépendance d'`archive` vers `worker_ipc` ; une extraction d'un diagnostic commun doit rester dans le périmètre accepté.

## Placement et tests

`worker_ipc.rs` (notamment `effect`, `poll`, `poll_unified_sessions`), `archive.rs`, `parakeet_live.rs` et `source_binding.rs` sont des points de concentration. Éviter de les agrandir sans justification ; ne pas imposer une extraction disproportionnée à un correctif limité.

Pour un plan de découpage autorisé, les responsabilités candidates sont : transport dans `ipc/`, état et handlers dans `runtime/` (import, file, direct, reprise, doublons), sélection dans `backends/`, sessions dans `parakeet_unified/`, publication dans `publication/`, service asynchrone dans `service/`. Ces modules sont des propositions, pas des chemins déjà disponibles.

Nouveaux tests dans des fichiers séparés ou `tests/`, sans agrandir le gros bloc inline de `worker_ipc.rs`. Partager les faux workers dans un support de tests accessible aux tests concernés ; ne pas exposer d'API produit uniquement pour les fixtures.
