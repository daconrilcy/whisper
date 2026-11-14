# Plan P30 — Isoler les vecteurs du test de séquence MP3

Avancée : cible de test autonome choisie pour le dernier lot de la tranche P21–P30.
Blocage : aucun ; conserver intégralement l’oracle de parité des paquets.
Décision : placer les quatre vecteurs SHA-256 attendus dans un support de test dédié.

## Résumé et preuve

Le Clippy structurel post-P29 compte 102 sites. Le test `packet_sequence_matches_original_encoder` dans `crates/whisper-worker-common/src/encoder/tests.rs` déclenche `too_many_lines` à 265 lignes. Il encode un PCM synthétique et compare la taille et le SHA-256 de chaque paquet à quatre séquences attendues (début, passage entre blocs, fin, flush).

## Périmètre

- Fichiers autorisés : `crates/whisper-worker-common/src/encoder/tests.rs` et nouveau `crates/whisper-worker-common/src/encoder/tests/packet_sequence.rs`.
- Déplacer uniquement les données constantes de test; la logique d’encodage et les assertions restent inchangées.
- Le scénario continue à comparer chaque paquet, dans l’ordre, par paire (taille, SHA-256), et vérifie `push_pcm([])`, le flush idempotent et le refus d’écriture après finalisation.
- Aucun code de production, dépendance, baseline, comportement de codec ou autre alerte ne change.

## Tâche P30-T1

Déclarer le sous-module privé de données de test, déplacer les quatre slices attendues en constantes `pub(super)` puis les importer dans le scénario. Garder toutes les assertions et séquences d’entrée dans le test d’origine.

**Acceptation :** le test passe sous 100 lignes et son alerte disparaît; le décompte Clippy passe de 102 à 101 ou moins sans alerte structurelle nouvelle; les valeurs et l’ordre des quatre vecteurs restent identiques; N1 et la revue indépendante passent; aucun changement hors des deux fichiers autorisés.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur une cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE. Témoin PE réel facultatif, à signaler non exécuté.

**Arrêt :** échec de garde, compilation ou test; modification nécessaire de l’oracle ou du comportement; nouveau chemin hors budget; alerte structurelle nouvelle. Préserver logs et état, ne pas modifier les autres lots.

**Retour arrière :** retirer uniquement la déclaration du module, les constantes déplacées et leurs imports si le résultat du test diffère.
