# whisper-worker-gpu

Complète le `AGENTS.md` racine, notamment les autorisations et la garde permanente.

## Rôle et modules actuels

Worker Turbo CUDA : exécute les commandes IPC et contrôle l'identité du GPU utilisé.

| Module | Responsabilité |
|---|---|
| `main.rs`, `startup` | Déclarations, démarrage et lecture des commandes sur canal borné |
| `worker`, `session/` | Dispatch et sessions : contrôle, décodage, import et direct |
| `decoder`, `encoder`, `ipc`, `integrity` | Façades vers les fonctions partagées de `whisper-worker-common` |
| `engine_cache` | Réutilisation du moteur et de son identité GPU vérifiée |
| `native_engine` | Contexte whisper-rs CUDA et transcription |
| `native_ffi` | Requêtes CUDA/GGML et corrélation des identités Windows |
| `gpu_attestation` | Comparaison de l'identité native au candidat |
| `gpu_commands`, `gpu_qualification` | Commandes de qualification GPU et résultats par périphérique |
| `diagnostics`, `qualification`, `memory_qualification` | Échecs IPC et contrôles de qualification, dont mémoire sous feature |

Préserver l'attestation fraîche et concordante requise avant exécution ; aucun contournement. Ne pas ajouter de repli CPU au mode GPU manuel.

Les modules ont été extraits : utiliser les fichiers et sessions existants, garder `main.rs` minimal. Préserver les conditions de réutilisation et d'invalidation d'`engine_cache`, dont l'identité GPU. Le manifeste déclare `unsafe_code = "deny"` ; `native_ffi` porte l'exception locale existante, sans autoriser son élargissement.

## Contrat commun des workers

- Les commandes et événements échangés avec le parent utilisent stdin/stdout et les protocoles de `whisper-core`. Réserver stderr aux diagnostics ; aucun texte libre sur stdout.
- Préserver la limite actuelle de 1 Mio par trame, le comptage du séparateur et le refus des trames excessives ou incomplètes avant désérialisation.
- Le parent fournit la configuration ; le worker contrôle les identités requises par son protocole (modèle, empreinte, périphérique). Ne pas déplacer dans le worker la politique de choix du moteur ou de repli.
- Aucune nouvelle écriture applicative hors des chemins autorisés transmis par le parent. Les temporaires et caches maîtrisables restent soumis à la garde racine.
- Respecter les restrictions `unsafe_code` de la crate et les frontières FFI existantes ; justifier chaque nouveau bloc autorisé par ses invariants `SAFETY`.
- Les hooks nouveaux suivent la politique racine. Ne pas supposer que toutes les crates disposent de `recette-hooks`, ni déplacer implicitement les features de qualification existantes.
- Compilation, tests sans modèle et exécution native restent soumis à la même garde permanente et aux autorisations de leur lot. Aucun lancement de modèle, audio, micro ou GPU n'est autorisé par ce fichier.

## Partage et tests

Les workers Turbo CPU et GPU utilisent déjà `whisper-worker-common` pour le décodage, l'encodage MP3 interne, les trames, l'intégrité et l'interface d'observation mémoire. Lire son `AGENTS.md` avant de la modifier. Garder les façades locales fines et les observations propres au worker ; ne pas dupliquer l'algorithme partagé. Comparer les deux parcours lors d'une correction commune ; si l'autre crate sort du périmètre accepté, signaler l'impact avant de la modifier. L'encodage interne n'autorise pas une nouvelle sortie MP3 utilisateur.

`l01-memory-qualification` propage la feature correspondante vers la crate commune. `recette-hooks` reste distincte, inactive par défaut et refusée en release par le script racine. Le worker Parakeet conserve son protocole et son moteur propres ; aucun report mécanique de code Turbo.

Placer les nouveaux tests dans des fichiers séparés ou `tests/`, avec des supports partagés selon leur visibilité. Distinguer tests de protocole, compilation et preuve du moteur natif réellement exécuté.
