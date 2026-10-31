# whisper-core

Complète le `AGENTS.md` racine, notamment ses autorisations et sa garde permanente. Aucune consigne ici n'autorise un refactoring ou une campagne.

## Rôle et frontières

Domaine et application : commandes, événements, états, politiques et ports. Le cœur décide et émet des effets (`ImportEffect`) exécutés par les adaptateurs.

- Pour le nouveau code de domaine, aucune I/O, aucun processus ou thread, aucun accès direct à l'environnement, à l'horloge système ou à stdout/stderr. Recevoir temps et configuration par les commandes ou ports appropriés.
- Des accès à l'horloge subsistent : horloge monotone dans `microphone_level.rs` et horodatage dans `application/runtime/import.rs`. Leur migration exige un périmètre accepté et la préservation du comportement. La trace historique L3D du cœur a été retirée ; ne pas la réintroduire.
- Seule dépendance externe actuelle : `serde`. Toute extension suit la validation du fichier racine.
- Un changement des types de `ports.rs`, `ipc.rs` ou `parakeet_ipc.rs` exige une analyse des consommateurs. Si le format échangé change, traiter explicitement compatibilité, version du protocole et mise à jour coordonnée du parent et des workers dans le plan.

## Carte actuelle

| Modules | Responsabilité |
|---|---|
| `application.rs` | Façade de réexports des contrats et d'`ImportApplication` |
| `application/contracts.rs`, `application/event_metadata.rs` | Contrats publics et identité des événements |
| `application/runtime/mod.rs` | État privé d'`ImportApplication` et composition du runtime |
| `application/runtime/commands.rs`, `events.rs`, `poll.rs` | Dispatch, traitement des événements et rafraîchissement |
| `application/runtime/import.rs`, `live.rs`, `queue.rs`, `closing.rs` | Import, direct, file et fermeture |
| `application/runtime/automatic_final.rs`, `unified.rs`, `parakeet_views.rs`, `microphone.rs` | Final automatique, sessions unifiées, projections et microphone |
| `domain`, `ports` | Identités, configuration, effets et ports |
| `ipc`, `parakeet_ipc` | Protocoles workers |
| `compute_policy` | Choix du moteur et replis autorisés |
| `engine_capabilities`, `model_catalog`, `model_registry`, `model_selection_app` | Modèles, capacités et admission |
| `gpu_inventory`, `gpu_inventory_app`, `gpu_selection` | Qualification et sélection GPU |
| `microphone`, `microphone_level` | Choix du micro, gains et mesures |
| `parakeet_scheduler`, `parakeet_session`, `parakeet_windows` | Ordonnancement, sessions et fenêtres |
| `path_settings`, `audio` | Préférences et détection de silence |

## Placement et tests

- Placer une politique nouvelle dans un module pur du domaine concerné. Maintenir les contrats publics existants tant que leur déplacement n'est pas prévu.
- Utiliser les handlers existants dans `application/runtime/`. Conserver l'état et les modules internes privés ; ne pas élargir l'API pour faciliter un découpage ou un test. Préserver les réexports d'`application.rs` et de `lib.rs`.
- Préserver les réentrées de fermeture dans `closing.rs` (`CancelDuplicateImport` et `Stop` via `dispatch`), les ACK avant publication et le filtrage des identités et générations.
- Tester le cœur sans adaptateurs avec des faux ports en mémoire. Tests de contrat dans `tests/`, tests privés du runtime dans `application/runtime/tests/`, déclarés depuis son `tests.rs`. Placer les nouveaux tests unitaires dans des fichiers séparés.
