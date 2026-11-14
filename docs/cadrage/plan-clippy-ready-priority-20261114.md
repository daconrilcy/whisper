# Plan P26 — Découper la caractérisation READY des imports unifiés

Avancée : Clippy standard après P25 passe le ratchet à 108 sites; un test de priorité READY fait 173 lignes.
Blocage : aucun; l’ordre session READY → document READY → End et la rétention du drain doivent rester démontrés.
Décision : extraire les étapes de préparation, d’attente et de vérification dans le même fichier de test.

## Source et périmètre

Le Clippy frais six crates signale `poll_ready_unified_import_precedes_ready_document_drain` dans `crates/whisper-adapters/src/worker_ipc/runtime/poll/tests/collection_priority.rs:24` (173/100). Le test autonome prouve qu’un import unifié préparé reste derrière la vraie barrière READY et que `ImportRuntime::poll` retourne le READY de session avant le READY document, puis conserve le drain jusqu’à End. N1 le classe `A_VERIFIER / AUTONOME`.

Périmètre code unique : `crates/whisper-adapters/src/worker_ipc/runtime/poll/tests/collection_priority.rs`, intact dans le worktree. Utiliser seulement des helpers privés dans le même fichier. Préserver le writer PCM et ses 80 000 échantillons, la route contrôlée, la vraie READY gate, les délais et ordres de polling, l’ordre des événements Ready et End, et toutes les assertions d’état du drain. Aucun changement produit, fichier supplémentaire, dépendance, baseline, garde ou autre test.

## P26-T1 — Séparer les phases de priorité

Garder dans le test l’enchaînement visible des phases. Extraire les helpers privés nécessaires pour préparer le PCM/la session, attendre la préparation avant libération de READY, collecter les deux Ready dans l’ordre, et vérifier End puis le retrait du drain. Les helpers doivent conserver les mêmes deadlines, pauses, messages d’échec et événements acceptés; ne pas masquer les assertions dans une abstraction générique. Maintenir le fichier sous 600 lignes et chaque helper sous 100 lignes. Clippy seuil 0 doit confirmer chaque helper sous 25 points; le Clippy standard ratcheté reste la gate finale.

## Acceptation

- L’alerte `too_many_lines` ciblée disparaît; aucun site nouveau; le ratchet passe à 107 sites ou moins.
- Le test garde ses preuves d’ordre `session Ready < document Ready < End`, de barrière réelle avant polling et de rétention du drain; aucun délai ni assertion n’est affaibli.
- SelfTest, `fmt --check`, mesure seuil 0 gardée, Clippy standard six crates ratcheté, N1 complet, sélections workers, fixtures PE et `git diff --check` passent.
- Seul le fichier autorisé est modifié; les changements préexistants sont préservés.

## Gates et arrêt

Worktree `C:/wt/cs`; `LotRoot` `target/parakeet-tests/clippy-corrections-suivantes-20261113`; cible Clippy existante `p21-fresh-20261114`, puis cible N1 neuve `n1-p26-20261114`. Toute commande Cargo passe via `ci/guard-level1.ps1`. Mesure seuil 0 et Clippy standard avec le profil exact six crates; N1 via `-WithoutCargoTargetDir`, cible fraîche et modèle validé. Arrêter et préserver les logs à tout échec de porte. P26 est la sixième tâche de la tranche P21–P30; nouvel arbitrage après P30.

## Retour arrière

Si le refactoring modifie une condition de barrière, l’ordre d’observation des événements, le nombre d’échantillons, les délais ou l’état final du drain, retirer uniquement la modification fautive et rapporter la preuve affectée; ne pas assouplir le test.
