Avancée : correctif de présentation préparé à la demande du 2 novembre 2026.
Blocage : accord sur ce plan versionné requis avant modification Rust.
Décision : proposer un lot limité au desktop, sans changer la détection ni la politique CPU/GPU.

# Message de démarrage lorsque le GPU est indisponible

## Objectif et critères

Sur le second PC de recette, Turbo et Parakeet fonctionnent en Auto et CPU,
mais le démarrage affiche `WorkerExited without End`, des erreurs de transport
et un inventaire indéterminé. Présenter cette situation clairement sans prétendre
qu'aucun GPU matériel n'existe : Windows signale notamment Intel Iris Xe et Parsec.

- AC1 : en cas d'échec de détection, message principal en langage courant :
  « GPU indisponible pour la transcription. Vous pouvez utiliser le CPU. »
- AC2 : les diagnostics techniques de démarrage et d'inventaire restent
  consultables dans une section repliée « Détails de la détection GPU » ;
  aucune chaîne interne brute dans le résumé visible par défaut.
- AC3 : conserver les distinctions disponible, chargement, partiel et échec ;
  ne pas annoncer une absence matérielle ou un CPU effectivement utilisé avant traitement.
- AC4 : aucun changement de choix CPU/GPU/Auto, de préférence mémorisée,
  d'activation du bouton GPU, de détection, d'IPC ou de repli.
- AC5 : préserver le cas GPU disponible et tester les cas échec, inventaire
  partiel, détails techniques et choix explicite CPU/Auto.

## Six tâches et responsabilités

1. Session principale : versionner ce plan et recueillir l'accord précis.
2. `rust_implementer` en mode IMPLEMENT : lire les consignes desktop et
   ajuster uniquement la présentation du message et des inventaires.
3. Même implémenteur : ajouter les tests ciblés dans le fichier séparé existant.
4. Vérifier via `ci/guard-level1.ps1` : SelfTest, puis fmt, tests desktop,
   check et Clippy desktop ; offline/locked quand applicable, environnement
   isolé sous `target/parakeet-tests/message-demarrage-cpu`, cible partagée
   canonique, variables CUDA/GGML nettoyées. Espace libre relevé : environ 77 Gio.
5. `rust_review_coordinator` : revue indépendante et corrections par
   l'implémenteur, deux cycles maximum ; CLEAN ou réserves explicites.
6. Session principale : consigner le résultat dans ce document et commiter
   localement. Validation du rendu Windows ultérieure par l'utilisateur.

Chemins Rust autorisés : `crates/whisper-desktop/src/ui/presentation/gpu.rs`,
`ui/model/controller.rs`, `ui/model/view/` pour le seul rendu des messages GPU,
`ui/tests/compute_and_catalog.rs` (tous relatifs à la même crate).
Maximum dix fichiers modifiés, rapport compris ; dépassement soumis à arbitrage.
Manifestes, lockfile, adapters/core/workers, gardes, installation et paquet
portable existant exclus. Aucune compilation release, exécution native,
réinstallation ou nouveau paquet dans ce lot ; aucun push.

La validation manuelle CPU sans CUDA déjà consignée reste acquise pour le
paquet précédemment remis ; elle ne valide pas encore ce changement d'affichage.
