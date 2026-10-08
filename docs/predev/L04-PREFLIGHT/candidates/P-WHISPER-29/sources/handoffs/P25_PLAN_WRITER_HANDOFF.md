# P25 — handoff du plan writer (transcription sémantique)

**Provenance et limite.** Résumé sémantique par `AUTH-COORD` du handoff final de `/root/p25_plan`. Ce n’est pas un export verbatim de la réponse conversationnelle; le texte est condensé. L’agent n’a écrit ni persisté de fichier. Le plan reste une proposition DRAFT, non encore CLEAN.

## Bases et verdict d’architecture

Bases vérifiées par l’agent : P24 `fb071cd5784eeac49e4bdac286e0891e00119e23c1d32df2ab5c07e2c12a605b`; D22 `8c41b42e365bca68be8b8de91a07516ed0c9858b92d16d15f5311ff34cac87a1`, revue DESIGN `R-WHISPER-DESIGN-22-v1 CLEAN`. Aucun `DESIGN_CHANGE_REQUIRED`; P25 doit succéder à P24 et avoir D22 pour parent. P25 ne devient READY qu’après revue PLANS exacte.

## Delta du candidat P25

Garder les fichiers P24 octet pour octet sauf les documents explicitement révisés; exclure metadata racine et laisser l’hôte générer manifest/proposal/receipt. Ajouter le contrat `15_L04_CHECKPOINT_CONTRACT.md`, la matrice `build-environment-L04.json`, la capture `USER_AUTHORIZATION_L04.md`, metadata de P24, metadata reçues D22, et le rapport/transports DESIGN D22 en maintenant leur qualification sémantique. Le helper hôte ajoute la référence au manifeste parent.

Mettre à jour l’index P25 pour identifier P24 digest et D22 parent exact, state courant, autorisation existante, L04 `planned`, futurs contrôles NOT RUN. Dans `01_LOTS.md`, garder les quinze paths exacts autorisés, lier parent D22, enregistrer l’autorisation L04, DETAIL-P03/Q-04/Q-06/Q-09 answered, dix snapshots baseline D22/five absences et étapes d’implémentation séquentielles. Les lots adjacents partagent les chemins et ne sont pas parallélisables.

Réécrire la procédure de préflight pour distinguer les racines `docs/predev/L04-PREFLIGHT`, `docs/predev` et code root; vérifier les manifests, recapturer HEAD/status/diff/hashes, autorisations, railguard, détails, dépendances L03 et environnement, puis lancer `check-state --lot L-WHISPER-04`, promotion contrôlée et nouveau check. Toutes les commandes dans le plan sont PROPOSED / NOT RUN. Les futures campagnes Cargo/native/UI/produit restent NOT RUN.

Le correctif de résolution central des preuves relève déjà de D22; P25 ne demande pas de correction du contrôleur. Le contenu courant de DETAIL-P03 et des Q doit être conservé sans réattribution de paramètres techniques. P24/CLEAN ne substitue pas le verdict exact P25.

## Contrat du pending de lancement

Après CLEAN P25, préparer un pending séparé, phase IMPLEMENTATION, D22 et P25 READY, L04 toujours `planned`, quinze paths, prerequisite L03 completed, DETAILS answered, railguard actif, source d’autorisation et autorité `AUTH-IMPLEMENTER-L04` déléguée à quinze paths. `code_state.baseline` et `current` ont les dix entrées existantes; baseline inclut SHA, snapshot exact lié au manifeste D22, revision D0 et `dirty_hash`. Ledger L04 initial vide. Les cinq futurs chemins restent absents de ces listes.

Le checkpoint doit capturer identité du HEAD courant distincte du HEAD D0 historique, preuve d’inventaire scoped, status/diff/hash sur les quinze paths, dix égalités et cinq absences, changements hors scope séparés. `dirty_hash` doit provenir d’un payload canonique conservé; ne pas supposer un worktree global propre. Drift none seulement si l’égalité est recapturée et prouvée. Les roots de manifests/snapshots/evidence et `--code-root` doivent rester cohérents; ne pas réécrire D22. Le check sans `--lot`, le preflight lot et les contrôles pré/post promotion sont distincts.

L04 demeure planned : le schéma n’a pas de statut `ready` pour un lot. L’identité vieillie du checkpoint actif doit être réconciliée avant le futur lot.

## Environnement et limites

Copier la matrice L03, garder Rust/Cargo 1.98.1, MSVC, CMake, Libclang, toolchain/lockfile; inclure NVCC correspondant à CUDA 12.8, Ninja et `CUDA_PATH` directory uniquement s’ils sont requis par le procédé build accepté. Une présence/version CUDA ne qualifie pas l’inférence. Ne pas installer implicitement.

## Ordre de préparation donné

1. Gate contrôlé DESIGN READY D22.
2. Persistance helper P25 et `verify-manifest`.
3. Transport qualifié du handoff du plan writer et checkpoint de reprise.
4. Revue PLANS indépendante exacte P25/D22 avec vérification pre/post.
5. Après CLEAN, construire les préconditions du pending de lancement puis le futur preflight.

**Restant :** DESIGN gate enregistré, P25 exact/persisté, transport plan writer, revue PLANS, résolution cohérente des racines, preuves dirty_hash/inventaire, puis contrôle L04. Aucun préflight, CLEAN/READY PLANS ou implémentation n’a été déclaré par le plan writer.
