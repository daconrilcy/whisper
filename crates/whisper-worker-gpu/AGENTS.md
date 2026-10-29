# whisper-worker-gpu

Complète le `AGENTS.md` racine, notamment les autorisations et la garde permanente.

## Rôle et modules actuels

Worker Turbo CUDA : exécute les commandes IPC et contrôle l'identité du GPU utilisé.

| Module | Responsabilité |
|---|---|
| `main.rs` | Sessions et modules inline `decoder`, `encoder`, `ipc` |
| `native_engine` | Contexte whisper-rs CUDA et transcription |
| `native_ffi` | Requêtes CUDA/GGML et corrélation des identités Windows |
| `gpu_attestation` | Comparaison de l'identité native au candidat |
| `gpu_qualification` | Résultats par périphérique |

Préserver l'attestation fraîche et concordante requise avant exécution ; aucun contournement. Ne pas ajouter de repli CPU au mode GPU manuel.

Les modules inline de `main.rs` sont des points de concentration. Une extraction vers des fichiers dédiés est souhaitable pour une évolution importante, si le plan l'inclut ; ne pas en faire un préalable automatique à un correctif borné.

## Contrat commun des workers

- Les commandes et événements échangés avec le parent utilisent stdin/stdout et les protocoles de `whisper-core`. Réserver stderr aux diagnostics ; aucun texte libre sur stdout.
- Préserver la limite actuelle de 1 Mio par trame, le comptage du séparateur et le refus des trames excessives ou incomplètes avant désérialisation.
- Le parent fournit la configuration ; le worker contrôle les identités requises par son protocole (modèle, empreinte, périphérique). Ne pas déplacer dans le worker la politique de choix du moteur ou de repli.
- Aucune nouvelle écriture applicative hors des chemins autorisés transmis par le parent. Les temporaires et caches maîtrisables restent soumis à la garde racine.
- Respecter les restrictions `unsafe_code` de la crate et les frontières FFI existantes ; justifier chaque nouveau bloc autorisé par ses invariants `SAFETY`.
- Les hooks nouveaux suivent la politique racine. Ne pas supposer que toutes les crates disposent de `recette-hooks`, ni déplacer implicitement les features de qualification existantes.
- Compilation, tests sans modèle et exécution native restent soumis à la même garde permanente et aux autorisations de leur lot. Aucun lancement de modèle, audio, micro ou GPU n'est autorisé par ce fichier.

## Partage et tests

Les workers Turbo CPU et GPU partagent des responsabilités de décodage, encodage MP3 interne et transport. Comparer les deux parcours lors d'une correction commune ; ne reporter que les changements applicables et autorisés. Si l'autre worker sort du périmètre accepté, signaler l'impact avant de le modifier. L'encodage interne n'autorise pas une nouvelle sortie MP3 utilisateur.

Une crate `whisper-worker-common` n'existe pas : sa création éventuelle relève d'un plan dédié validé, sans bloquer les corrections bornées dans les modules actuels. Le worker Parakeet conserve son protocole et son moteur propres ; aucun report mécanique de code Turbo.

Placer les nouveaux tests dans des fichiers séparés ou `tests/`, avec des supports partagés selon leur visibilité. Distinguer tests de protocole, compilation et preuve du moteur natif réellement exécuté.
