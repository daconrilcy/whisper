# whisper-desktop

Complète le `AGENTS.md` racine ; sa garde permanente et la validation manuelle Windows restent applicables.

## Rôle

Composition du cœur et des adaptateurs, interface egui/eframe. L'interface affiche la vue et convertit les interactions en commandes ; les politiques de tâches, de moteurs et de modèles appartiennent au cœur.

- `root` compose, `ui` affiche, `reactivity_log` trace la réactivité. Pour le nouveau code, faire passer les opérations disque, processus et workers par les services appropriés. Des accès directs existent dans l'UI : ne pas prétendre que la séparation est déjà totale ni les déplacer hors du lot accepté.
- Centraliser les nouveaux choix de configuration des workers dans `root`. Préserver les variables et routages actuels : `personal-install`, candidat unifié Parakeet, profil Test et installation personnelle. Ne pas renommer les variables historiques pour une règle de style.
- Toute modification du routage doit préciser les chemins effectifs et ressources utilisés ; appliquer les règles de remise Windows du fichier racine.

## Placement et tests

`ui.rs` est un point de concentration. Pour un découpage accepté, envisager `ui/mod.rs` pour le routage et `ui/import.rs`, `live.rs`, `history.rs`, `settings.rs`, `dialogs.rs`, `widgets.rs` pour les responsabilités correspondantes. Cette carte est une proposition, pas un préalable à chaque correction.

Placer les nouveaux tests de conversion des interactions et de présentation dans un fichier séparé ou `tests/`, sans agrandir le bloc inline existant. Les tests automatiques ne prouvent pas la validation du rendu dans la fenêtre Windows, réservée à l'utilisateur.
