# Plan P29 — Découper la projection des capacités moteur

Avancée : plan borné pour la projection pure des choix de moteur.
Blocage : aucun ; préserver exactement les règles de sélection et de refus.
Décision : extraire le calcul des options de configuration et de langue.

## Résumé et preuve

La mesure Clippy structurelle post-P28 signale `engine_capabilities::project` à 121 lignes (seuil 100) dans `crates/whisper-core/src/engine_capabilities.rs:185`. Le fichier est propre dans le worktree. La fonction vérifie d'abord la génération, puis projette modèles, configurations, langues, causes de refus et préférences depuis un snapshot immuable.

## Périmètre

- Fichier de production autorisé : `crates/whisper-core/src/engine_capabilities.rs`.
- Tests : réutiliser les tests existants du module et les contrats déjà présents; aucun test nouveau nécessaire pour un découpage purement mécanique.
- Aucun type public, dépendance, champ, politique, algorithme, ordre de sortie ou cause de refus ne change.
- Ne pas modifier les alertes restantes, la baseline, les autres lots ou les manifestes.

## Tâche P29-T1

Extraire en fonctions privées les transformations imbriquées de modèle/configuration/langue de `project`, sans changer leur ordre d'évaluation ni leurs résultats. Garder dans `project` le contrôle de génération et la composition de `EngineProjection`.

**Acceptation :** `project` passe sous 100 lignes; l'alerte ciblée disparaît; le décompte Clippy structurel passe de 103 à 102 ou moins sans nouvelle alerte; les tests N1 sont verts; diff limité au fichier autorisé; revue indépendante CLEAN.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, Clippy structurel six crates via la garde (mesure seuil 0 et ratchet standard), N1 `cargo test --no-fail-fast` via la garde sur cible fraîche, trois sélections workers et SelfTest des fixtures PE. Témoin PE réel facultatif et signalé non exécuté.

**Arrêt :** échec de garde, compilation ou test; altération fonctionnelle requise; fichier ou comportement supplémentaire nécessaire; nouvelle alerte structurelle. Préserver logs et état, sans toucher aux autres modifications du worktree.

**Retour arrière :** retirer uniquement les helpers et appels introduits par P29 si une équivalence stricte ne peut être établie.
