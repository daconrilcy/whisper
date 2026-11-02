Avancée : plan GARDE-DEPENDANCES remis par l’utilisateur, transcrit pour exécution locale.
Blocage : aucun avant préparation du candidat ; téléchargement et N1 attendent la revue CLEAN.
Décision : implémentation déléguée des scripts ; une seule correction après revue, aucun push.

# GARDE-DEPENDANCES

Objectif : permettre la vérification CPU du produit après rétention, sans dépendance à un ancien lot jetable. Base `e8f9094f`, branche `codex/garde-dependances`, worktree `C:/wt/garde-deps`.

1. Versionner sous `ci/fixtures/pe/` les fixtures synthétiques direct cublas, sans CUDA, différé CUDA, GPU cudart/nvcuda et non-PE ; préserver les cas positifs/négatifs des règles existantes. Rendre facultatif le témoin réel par paire `-WitnessPath` / `-WitnessSha256`, refus CPU exigé ; absence annoncée NON EXÉCUTÉ.
2. Dans `ci/guard-level1.ps1` et ses auto-tests, remplacer le cache d’ancien lot par `target/cargo-home-shared/` du worktree, persistant déclaré. Ajouter seulement `fetch --locked` en réseau, sans compilation/exécution ; refuser fetch sans locked, maintenir offline/locked ailleurs et diagnostic cache absent orientant vers fetch. Auto-tests sans Cargo ni dépendance au cache.
3. Mettre AGENTS.md en cohérence : racine persistante autorisée, références permanentes versionnées ou persistantes déclarées, protection de ces références et du cache lors de la rétention.
4. Revue indépendante du candidat scripts/fixtures/consignes ; un cycle de correction maximum. Sans CLEAN, ne pas commencer fetch/N1. Commits locaux permis ; aucun push, tag ou modification CI distante.
5. Après CLEAN, sur la tête locale du lot : garde fetch --locked, fmt, clippy workspace all-targets, tests par défaut et deux workers CPU, Structure, auto-tests N1/imports. Avant chaque sélection worker : compilation no-run gardée puis contrôle PE des harnais. Même nettoyage WHISPER/PARAKEET/CUDA/GGML, LOCALAPPDATA vide, TEMP/TMP isolés, cible Cargo canonique partagée, 30 Gio libres minimum. Arrêt au premier échec. Aucun Rust, manifeste ni Cargo.lock modifié. Comparer aux références fusion : 616/121, 12/9, 26/11 ; 92 avertissements, Structure PASS sans retour silencieux. Identifier les nouveaux tests/avertissements par fichier et commit ; absence/perte/nouvel ignoré bloque.
6. Rapporter le test settings comme réellement exécuté ou non ; archiver journaux, vérifier nombre/SHA-256, supprimer seulement cibles/caches propres au lot selon règles ; protéger les caches persistants. Compte rendu d’une page dans docs/architecture/, figé après revue, sans modifier les rapports initiaux du worktree principal.

Périmètre : `ci/check-worker-imports.ps1`, `ci/fixtures/`, `ci/guard-level1.ps1` et ses auto-tests, `AGENTS.md`, `docs/architecture/`. Le délégué possède les scripts et fixtures ; la session principale écrit les documents et consignes. Les profils de revue restent en lecture seule. Aucun élargissement au code produit ni aux workflows.
