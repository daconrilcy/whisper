Avancée : le Clippy gardé après P51 signale `microphone_test_owns_one_capture_then_releases_before_live_restart` à 208 lignes et complexité 26.
Blocage : le test couvre l’exclusivité de capture, le redémarrage live, l’identité micro, le retry Auto et les événements tardifs.
Décision : extraire des helpers privés de scénario en conservant le test, son ordre et tous ses oracles.

# Plan P52 — Décomposer le test de capture micro exclusive

## Constat et preuve

Le Clippy standard gardé après P51 signale `too_many_lines (208/100)` et `cognitive_complexity (26/25)` sur `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs::microphone_test_owns_one_capture_then_releases_before_live_restart` (ligne 450 du candidat P51). Le ratchet courant est à 80 sites.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-core/src/application/runtime/tests/selection_and_microphone.rs`.
- Extraire le scénario en helpers privés locaux appelés directement par le même test; garder chaque helper sous 100 lignes et sous le seuil cognitif 25.
- Préserver les helpers P51 et les hunks micro déjà présents dans ce fichier. Ne pas reformater ou réécrire leurs zones.
- Ne modifier aucun code de production, identité de test, assertion, valeur, événement, fixture, feature, baseline ou manifeste. Aucun autre fichier n’est autorisé.

## Critères d’acceptation P52-T1

Le test conserve son nom et exécute dans le même ordre : démarrage micro Device `mic:A` au gain 6; refus du direct live concurrent sans effet additionnel; événements Started/Stopped et vues attendues; redémarrage live dans la génération suivante; acquisition attendue et phase direct `None`; rejet du spectre d’une autre source puis admission de `mic:A`; invalidation au retry et rejet du spectre pré-barrière; nettoyage immédiat au Stop, rejet des événements tardifs et du meter d’ancienne génération. Les identifiants, valeurs, horodatages monotones et assertions restent inchangés.

Le corps principal descend sous 100 lignes, le fichier reste sous 1 500 lignes, et aucun nouveau lint structurel ne paraît. Le Clippy seuil cognitif 0 passe et le ratchet descend de 80 à 79 sites ou moins. La revue indépendante déclare CLEAN; fmt-check, N1, PE et les sélections des trois workers passent.

## Vérifications et limites

Via la garde de niveau 1 avec le `LotRoot` admis `target/parakeet-tests/clippy-corrections-suivantes-20261113` : SelfTest, `fmt --check`, mesure cognitive puis Clippy standard exact à six membres, N1 `test --no-fail-fast` sur cible fraîche `n1-p52-20261114`, SelfTest PE, compilations `--no-run`, contrôles PE par chemin exact, inventaires `-- --list` et suites CPU, Parakeet CPU et common. Tous les profils Cargo utilisent les contraintes `--offline --locked`, isolation et conditions d’arrêt existantes.

Les tests natifs/micro nécessitant vrais modèle, runtime ou matériel restent ignorés et sont signalés `NON EXÉCUTÉS`; le résultat N1 n’est pas une qualification microphone. Le témoin PE réel reste `NON EXÉCUTÉ`. Aucun test GPU niveau 2, build natif, fenêtre Windows, téléchargement ni push.

Retour arrière limité au seul corps de ce test; conserver le candidat P51 et les hunks préexistants. L’utilisateur a autorisé P51–P60 le 14 novembre 2026; P52 est la deuxième tâche de cette tranche.