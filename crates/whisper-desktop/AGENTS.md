# whisper-desktop

Complète le `AGENTS.md` racine ; sa garde permanente et la validation manuelle Windows restent applicables.

## Rôle

Composition du cœur et des adaptateurs, interface egui/eframe. L'interface affiche la vue et convertit les interactions en commandes ; les politiques de tâches, de moteurs et de modèles appartiennent au cœur.

- `root` compose, `ui` expose la façade de l'interface, `reactivity_log` trace la réactivité. Pour le nouveau code, faire passer les opérations disque, processus et workers par les services appropriés. Des accès directs subsistent dans les modules UI : ne pas prétendre que la séparation est totale ni les déplacer hors du lot accepté.
- Centraliser les nouveaux choix de configuration des workers dans `root`. Préserver les variables et routages actuels : `personal-install`, candidat unifié Parakeet, profil Test et installation personnelle. Ne pas renommer les variables historiques pour une règle de style.
- Toute modification du routage doit préciser les chemins effectifs et ressources utilisés ; appliquer les règles de remise Windows du fichier racine.

## Placement et tests

La décomposition MVC existe déjà ; utiliser ses chemins actuels :

| Module | Responsabilité |
|---|---|
| `ui.rs` | Façade et réexports publics à préserver |
| `ui/model.rs` | État de `DesktopApp`, construction et composition interne |
| `ui/model/preferences.rs` | Types, canaux et runtime de préférences |
| `ui/model/controller.rs`, `controller/actions.rs`, `controller/observability.rs` | Rafraîchissement, commandes et observation de l'interface |
| `ui/model/view.rs`, `view/` | Intégration eframe et rendu des réglages, captures, dialogues, historique, file et sessions |
| `ui/presentation.rs`, `presentation/` | Conversions et présentation des textes, langues, chemins, modèles, GPU et microphone |

Conserver les responsabilités dans ces modules et leurs visibilités internes ; ne pas regrouper à nouveau l'état, les commandes et le rendu dans `ui.rs`. Les politiques métier restent dans le cœur, les conversions de présentation dans `ui/presentation/`.

Les tests privés sont dans `ui/tests.rs` et `ui/tests/`, les tests d'intégration dans `tests/`. Préserver les scénarios et assertions des conversions, interactions et sessions unifiées ; utiliser les supports existants selon leur visibilité. Les tests automatiques ne prouvent pas la validation du rendu dans la fenêtre Windows, réservée à l'utilisateur.
