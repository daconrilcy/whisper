# whisper-worker-parakeet-cpu

Complète le `AGENTS.md` racine, notamment les autorisations et la garde permanente.

## Rôle et modules actuels

Worker Parakeet CPU, ou CUDA avec `parakeet-cuda`. Charge le modèle et les DLL puis utilise `whisper-core::parakeet_ipc`.

| Module | Responsabilité |
|---|---|
| `main.rs` | Boucle du protocole Parakeet |
| `decoder` | Décodage audio |
| `native_engine` | Moteur natif et normalisation des mots horodatés |
| `native_ffi` | DLL, symboles, pointeurs et résultats natifs |

- Préserver les contrôles canoniques des racines de runtime et de modèle propres au profil. Aucun élargissement sans accord précis.
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

Les workers Turbo CPU et GPU partagent des responsabilités de décodage, encodage MP3 interne et transport. Comparer les deux parcours lors d'une correction commune ; ne reporter que les changements applicables et autorisés. Si l'autre worker sort du périmètre accepté, signaler l'impact avant de le modifier. L'encodage interne n'autorise pas une nouvelle sortie MP3 utilisateur.

Une crate `whisper-worker-common` n'existe pas : sa création éventuelle relève d'un plan dédié validé, sans bloquer les corrections bornées dans les modules actuels. Le worker Parakeet conserve son protocole et son moteur propres ; aucun report mécanique de code Turbo.

Placer les nouveaux tests dans des fichiers séparés ou `tests/`, avec des supports partagés selon leur visibilité. Distinguer tests de protocole, compilation et preuve du moteur natif réellement exécuté.
