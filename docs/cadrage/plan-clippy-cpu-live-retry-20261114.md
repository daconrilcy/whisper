# Plan P42 — Réduire la fonction de reprise CPU du direct

Avancée : P41 est CLEAN; le Clippy frais suivant relève `start_cpu_live_attempt` à 127 lignes.
Blocage : aucun établi; préserver les invariants de reprise et de publication du direct.
Décision : extraire les étapes cohérentes de reprise dans des helpers privés du même module.

## Constat

L'alerte courante de `too_many_lines` vise `crates/whisper-adapters/src/worker_ipc/runtime/live.rs`, fonction `start_cpu_live_attempt`. Le diff préexistant de ce fichier extrait `reset_live_start_state` avant cette fonction et ne la modifie pas.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/worker_ipc/runtime/live.rs`.
- Isoler les groupes cohérents de réservation/reprise d'archive, lancement du worker ou réinitialisation de l'état live dans des helpers privés si cela réduit la fonction sous 100 lignes.
- Préserver les contrôles PublicationGate, JobId/Generation, les erreurs et leurs textes, les nettoyages au lancement échoué, les compteurs de tentative, le replay jusqu'aux échantillons confirmés, les séquences et diagnostics.
- Aucun changement de protocole, comportement produit, test, API publique ou autre fichier source.

## Tâche P42-T1

Réduire `start_cpu_live_attempt` sous le seuil `too_many_lines` en gardant le même ordre d'effets et les mêmes erreurs observables.

**Acceptation :** fonction sous 100 lignes; Clippy ratchet sans alerte structurelle nouvelle; N1 et sélections worker requises réussissent; revue indépendante CLEAN; diff source limité au périmètre autorisé.

**Vérifications :** SelfTest de la garde, fmt-check, mesure cognitive seuil 0 et Clippy standard six crates via la garde, N1 sur cible fraîche, sélections CPU/Parakeet CPU/common et SelfTest PE.

**Arrêt :** échec de garde, compilation, test ou ratchet; modification nécessaire d'un fichier hors périmètre; changement d'ordre des effets, nettoyage, séquences de replay, diagnostic ou erreur observable.

**Retour arrière :** restaurer l'implémentation monolithique de cette fonction en conservant les changements préexistants hors de son corps.
