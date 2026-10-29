# whisper-core

Complète le `AGENTS.md` racine, notamment ses autorisations et sa garde permanente. Aucune consigne ici n'autorise un refactoring ou une campagne.

## Rôle et frontières

Domaine et application : commandes, événements, états, politiques et ports. Le cœur décide et émet des effets (`ImportEffect`) exécutés par les adaptateurs.

- Pour le nouveau code de domaine, aucune I/O, aucun processus ou thread, aucun accès direct à l'environnement, à l'horloge système ou à stdout/stderr. Recevoir temps et configuration par les commandes ou ports appropriés.
- Des exceptions existent : traces dans `application.rs`, horloge monotone dans `microphone_level.rs` et horodatage dans l'application. Ne pas les présenter comme déjà corrigées ; leur migration exige un périmètre accepté et la préservation du comportement.
- Seule dépendance externe actuelle : `serde`. Toute extension suit la validation du fichier racine.
- Un changement des types de `ports.rs`, `ipc.rs` ou `parakeet_ipc.rs` exige une analyse des consommateurs. Si le format échangé change, traiter explicitement compatibilité, version du protocole et mise à jour coordonnée du parent et des workers dans le plan.

## Carte actuelle

| Modules | Responsabilité |
|---|---|
| `application`, `domain`, `ports` | Machine d'états, identités, configuration et effets |
| `ipc`, `parakeet_ipc` | Protocoles workers |
| `compute_policy` | Choix du moteur et replis autorisés |
| `engine_capabilities`, `model_catalog`, `model_registry`, `model_selection_app` | Modèles, capacités et admission |
| `gpu_inventory`, `gpu_inventory_app`, `gpu_selection` | Qualification et sélection GPU |
| `microphone`, `microphone_level` | Choix du micro, gains et mesures |
| `parakeet_scheduler`, `parakeet_session`, `parakeet_windows` | Ordonnancement, sessions et fenêtres |
| `path_settings`, `audio` | Préférences et détection de silence |

## Placement et tests

- Placer une politique nouvelle dans un module pur du domaine concerné. Maintenir les contrats publics existants tant que leur déplacement n'est pas prévu.
- `application.rs`, `handle_event` et `dispatch` sont des points de concentration : préférer des handlers dédiés pour une évolution importante. Un découpage `application/import.rs`, `live.rs`, `queue.rs` est une piste à valider, pas une arborescence existante ou imposée.
- Tester le cœur sans adaptateurs avec des faux ports en mémoire. Tests de contrat dans `tests/`, nouveaux tests unitaires dans un fichier séparé, sans agrandir le bloc inline de `application.rs`.
