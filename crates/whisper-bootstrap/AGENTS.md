# whisper-bootstrap

Complète le `AGENTS.md` racine, notamment les autorisations et la garde permanente.

Point d'entrée minimal qui référence la vue, le protocole et le transport. L'installation est seulement évoquée en commentaire et n'est pas active.

- `src/main.rs` reste le seul point d'entrée ; ses dépendances internes sont `whisper-core` et `whisper-adapters`. Il ne compose pas la fenêtre desktop et ne pilote pas les sessions workers.
- Préserver `forbid(unsafe_code)` et les lints hérités du workspace.
- Ne pas ajouter de logique sans tâche dédiée acceptée.
- Ne pas activer une installation ou un accès réseau à partir de ce commentaire.
- Toute compilation, tout test et toute installation éventuelle suivent les autorisations et prérequis du fichier racine ; aucun niveau simplifié ne dispense de la garde.
