# Plan P50 — Séparer les deux passes de normalisation des mots

Avancée : le Clippy gardé après P49 compte 82 sites et relève `normalize_words` à 133 lignes.
Blocage : aucun diff antérieur dans `native_engine.rs`; préserver strictement les règles temporelles Parakeet.
Décision : isoler la construction des mots lexicaux puis la normalisation de leurs intervalles.

## Constat

L’alerte `too_many_lines` vise `crates/whisper-worker-parakeet-cpu/src/native_engine.rs`, fonction `normalize_words`. Elle valide les mots et timestamps natifs dans une première passe, puis normalise les chevauchements et produit les mots finaux dans une seconde passe.

## Périmètre

- Fichier autorisé : `crates/whisper-worker-parakeet-cpu/src/native_engine.rs`.
- Réduire `normalize_words` sous 100 lignes en séparant ses deux passes en helpers privés locaux; garder chaque helper sous 100 lignes.
- Préserver l’ordre ponctuation/validation des timestamps, le clamp du dernier mot dans la tolérance de trame, les bornes de durée, les règles de chevauchement jusqu’à `TDT_FRAME_MS`, les intervalles partagés et leurs répartitions entières.
- Préserver types d’erreur, indices, texte brut diagnostique et ordre des mots produits.
- Aucun changement de FFI, API, feature, test ou autre fichier source.

## Tâche P50-T1

Extraire la construction de la liste lexicale et sa normalisation d’intervalles sans modifier les algorithmes ni leurs décisions.

**Acceptation :** `normalize_words` sous 100 lignes; Clippy passe seuil 0 et le ratchet descend à 81 sites ou moins sans nouvelle alerte; N1 et sélections worker passent; revue indépendante CLEAN; diff source limité au fichier autorisé.

**Vérifications :** SelfTest de la garde, fmt-check, mesure cognitive seuil 0 et Clippy standard six crates via la garde, N1 sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation, test ou ratchet; fichier hors périmètre nécessaire; changement de mots acceptés, timestamps, chevauchements, erreurs ou preuve de transcription.

**Retour arrière :** restaurer `normalize_words` initiale sans toucher au reste du fichier.
