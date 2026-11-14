# Plan P31 — Découper le support de cycle microphone UI

Avancée : cible de test synthétique propre, une méthode de 160 lignes.
Blocage : aucun ; garder les valeurs, événements et transitions de vue identiques.
Décision : extraire la préparation de l’état, du compteur et du spectre micro.

## Résumé et preuve

Le Clippy structurel post-P30 compte 101 sites. `MicrophoneCycleApplication::dispatch`, dans `crates/whisper-desktop/src/ui/tests/support.rs:51`, déclenche `too_many_lines` à 160 lignes. La branche `StartMicrophoneTest` construit l’état initial du faux runtime, une mesure micro et un spectre synthétiques, puis ajoute quatre événements ordonnés à la file. Les autres commandes gèrent l’arrêt, les sessions unifiées et le rafraîchissement.

## Périmètre

- Fichier autorisé : `crates/whisper-desktop/src/ui/tests/support.rs`.
- Extraire uniquement des helpers privés du support de test; ne pas toucher au produit ni aux scénarios consommateurs.
- Préserver la commande enregistrée avant dispatch, toutes les valeurs et deux appels distincts à `monotonic_millis`, l’ordre des quatre événements, les transitions et erreurs des autres branches, ainsi que la vue retournée.
- Aucun changement aux tests baseline, APIs, dépendances ou autres alertes.

## Tâche P31-T1

Déplacer la mise à jour de la vue initiale, la création du `MicrophoneMeter`, la création du `MicrophoneSpectrum` et/ou l’ajout de la séquence d’événements vers des helpers privés. Garder dans `dispatch` une délégation lisible et tous les autres bras de commande inchangés.

**Acceptation :** `dispatch` passe sous 100 lignes; le site visé disparaît; le ratchet diminue de 101 à 100 ou moins sans alerte structurelle nouvelle; les tests UI existants passent; revue indépendante CLEAN; diff limité au fichier autorisé.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE. Témoin PE réel facultatif, à signaler non exécuté.

**Arrêt :** échec de garde, compilation ou test; besoin de changer un scénario UI, son oracle ou le produit; fichier hors périmètre nécessaire; nouvelle alerte structurelle. Préserver logs et état.

**Retour arrière :** retirer uniquement les helpers introduits dans ce fichier si l’état, l’ordre des événements ou les valeurs divergent.
