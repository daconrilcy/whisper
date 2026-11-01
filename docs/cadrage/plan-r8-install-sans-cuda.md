Avancée : réinstallation canonique et préparation d'une qualification CPU sans CUDA autorisées dans cette session après R8-FIX 5d5bb521.
Blocage : le composant Windows Sandbox n'est pas détecté ; choix des sources Parakeet à rendre compatible avec le registre actuel avant installation.
Décision : aucune modification Rust ; aucune activation Windows, élévation ni redémarrage sans accord distinct.

# Livraison et qualification R8 — six tâches

1. Préflight en lecture seule : Git, espace libre, SHA des deux modèles et quatorze DLL, quatre workers et identités Parakeet admises par le registre. Préserver les preuves R8-FIX et toutes les sources historiques.
2. Construire le desktop release `personal-install` avec `ci/guard-level1.ps1`, hors ligne/verrouillé, racines isolées sous `target/r8-install/`, cible fraîche distincte. Vérifier PE et enregistrer l'attestation exacte requise par l'installeur. Aucun worker/modèle lancé sur l'hôte.
3. Préparer les sources du script canonique sous le dépôt. Les nouveaux Parakeet R8-FIX ne correspondent pas aux identités admises : aucune fausse attestation ou promotion implicite. Faire arbitrer l'emploi des workers Parakeet déjà qualifiés ou une qualification distincte ; ne pas modifier le registre Rust.
4. Exécuter `scripts/install-personal.ps1` pour staging puis publication d'un candidat neuf sous le véritable `%LOCALAPPDATA%/Whisper`. Préserver sa récupération/sauvegarde. Relever et comparer les SHA, puis exécuter `ci/check-worker-imports.ps1` sur les cinq EXE effectivement installés. L'autorisation précise de préparation puis installation a été donnée dans la session présente.
5. Recette Windows Sandbox : autorisation précise reçue pour création et lancement, GPU virtuel et réseau désactivés, entrées en lecture seule, résultats sous le dépôt. Vérifier dans l'invité l'absence réelle de DLL CUDA avant essai d'une courte transcription CPU avec les copies des fichiers installés. Toute activation du composant Windows, élévation ou redémarrage requiert un accord supplémentaire. Ne modifier ni pilote ni DLL CUDA de l'hôte. La fenêtre produit reste réservée à l'utilisateur.
6. Bilan dans le présent document et preuves compactes sous `docs/architecture/raw/r8-install/` : distinguer installé/SHA/PE, environnement invité et inférence, tests non exécutés, coûts et limites. Git local seulement, aucun push.

Autorisations reçues : « Oui, préparer puis installer » (desktop gardé, ressources locales, attestations de livraison, installation et contrôle des fichiers installés) ; « Oui, autoriser la recette isolée » (sans activation système implicite). Arrêt sur échec ou prérequis divergent avant installation ; aucune acquisition ni nouvelle dépendance, aucun changement de code produit, manifeste, Cargo.lock ou garde historique.
