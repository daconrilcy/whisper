# whisper-worker-parakeet-cpu

Complète le `AGENTS.md` racine, notamment les autorisations et la garde permanente.

## Rôle et modules actuels

Worker Parakeet CPU, ou CUDA avec `parakeet-cuda`. Charge le modèle et les DLL puis utilise `whisper-core::parakeet_ipc`.

| Module | Responsabilité |
|---|---|
| `main.rs`, `startup`, `args` | Entrée minimale, arguments, contrôles de démarrage et annonce Ready |
| `session`, `transcription` | Boucle de commandes Parakeet et traitement des transcriptions |
| `runtime_paths`, `gpu_identity`, `integrity` | Racines autorisées par profil, identité backend/GPU et empreintes |
| `ipc` | Trames et événements du protocole Parakeet |
| `decoder`, `decode_only`, `reference_decoder` | Décodage audio, mode décodage seul et référence sous feature |
| `native_engine` | Moteur natif et normalisation des mots horodatés |
| `native_ffi` | DLL, symboles, pointeurs et résultats natifs |

- Préserver les contrôles canoniques des racines de runtime et de modèle propres au profil. Aucun élargissement sans accord précis.
- `runtime_paths` distingue installation personnelle (et staging prévu), racines Test CPU/CUDA et profils historiques de qualification. Conserver ces distinctions et l'exception bornée du modèle partagé historique ; une racine autorisée en lecture n'autorise aucune écriture de campagne.
- Indiquer les modes CPU/CUDA concernés et les vérifier séparément lorsqu'ils appartiennent au lot ; une compilation CPU ne prouve pas le mode CUDA.
- Le chargement des DLL et les pointeurs natifs restent dans `native_ffi`.
- Préserver la distinction entre timestamps natifs invalides, normalisation et preuve de transcription ; ne pas transformer un rejet en succès silencieux.
- Cette crate n'a pas de feature `recette-hooks`. Les features actuelles sont `parakeet-qualification-reference`, `parakeet-cuda` et `parakeet-personal` ; leur changement nécessite un périmètre accepté.

## Contrat commun des workers

- Les commandes et événements échangés avec le parent utilisent stdin/stdout et les protocoles de `whisper-core`. Réserver stderr aux diagnostics ; aucun texte libre sur stdout.
- Préserver la limite actuelle de 1 Mio par trame, le comptage du séparateur et le refus des trames excessives ou incomplètes avant désérialisation.
- Le parent fournit la configuration ; le worker contrôle les identités requises par son protocole (modèle, empreinte, périphérique). Ne pas déplacer dans le worker la politique de choix du moteur ou de repli.
- Aucune nouvelle écriture applicative hors des chemins autorisés transmis par le parent. Les temporaires et caches maîtrisables restent soumis à la garde racine.
- Respecter les restrictions `unsafe_code` de la crate et les frontières FFI existantes ; justifier chaque nouveau bloc autorisé par ses invariants `SAFETY`.
- Les hooks nouveaux suivent la politique racine. Ne pas supposer que toutes les crates disposent de `recette-hooks`, ni déplacer implicitement les features de qualification existantes.
- Compilation, tests sans modèle et exécution native restent soumis à la même garde permanente et aux autorisations de leur lot. Aucun lancement de modèle, audio, micro ou GPU n'est autorisé par ce fichier.

## Partage et tests

Cette crate dépend de `whisper-core` et conserve son protocole `parakeet_ipc`, son décodage et son moteur propres. Elle ne dépend pas de `whisper-worker-common`, utilisée par les workers Turbo CPU/GPU ; aucun report mécanique de code Turbo ni ajout de dépendance implicite.

Garder `main.rs` minimal et placer les évolutions dans les modules déjà extraits. Préserver la séparation entre contrôles d'admission au démarrage, cycle de session, transcription et FFI. Pour un changement de routage historique, examiner aussi le client `whisper-adapters/src/parakeet_import/historical_runtime.rs` sans le modifier hors du périmètre autorisé.

Placer les nouveaux tests dans des fichiers séparés ou `tests/`, avec des supports partagés selon leur visibilité. Distinguer tests de protocole, compilation et preuve du moteur natif réellement exécuté.
