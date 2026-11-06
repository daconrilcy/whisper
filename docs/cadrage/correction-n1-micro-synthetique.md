Avancée : dépendance au microphone réel confirmée dans trois tests N1 ; correction autorisée.
Blocage : N1 rouge ; aucune fusion ni validation verte revendiquée.
Décision : injecter une capture synthétique, préserver les assertions et rejouer N1 avant revue.

# Correction N1 — capture synthétique

## Mandat accepté

Autorisation courante : « Je t'autorise », en réponse au correctif précis dans `crates/whisper-adapters/src/direct_live.rs` et `direct_live/tests.rs`, avec complément documentaire, tests ciblés puis N1 complet via garde, lints, build et revue indépendante. Objectif : rendre les tests Auto CPU/GPU indépendants du microphone réel, sans modifier le comportement produit. Référence initiale : `671e979e`, source du lot `df81bf92` ; seuls les dossiers de remise sont non suivis. Les deux sources sont identiques à `origin/main` (`48827173`), ce qui confirme une dépendance matérielle existante, sans rendre le N1 rouge acceptable pour une fusion.

Périmètre : ces deux fichiers Rust ; présent complément et son archive de journaux `docs/cadrage/raw/correction-n1-micro-synthetique.zip` ; le bilan précédent reçoit seulement un lien. Exécution dans `target/parakeet-tests/inspect-micro-demarrage/correction-n1/`, cible/cache partagés existants et environnement N1 isolé. Aucune dépendance, feature, API publique, unsafe, configuration Windows ou modèle ajouté. Aucun moteur natif suspendu ou worker livré reconstruit, aucun téléchargement ; libfvad reste la seule compilation C autorisée pour les vérifications non-workers.

## Quatre tâches

1. Principal : versionner ce périmètre, relever Git/espace et préserver les ressources du lot clos.
2. `rust_implementer`, IMPLEMENT : injection privée de l'ouverture de capture dans le vrai démarrage ; production conserve `start_direct_with_gain`. Les trois tests utilisent `CaptureStream::empty_test_capture` et des échantillons contrôlés si nécessaires, en conservant workers simulés, ordre et assertions.
3. Implémenteur : trois tests ciblés, puis N1 exact sans `CARGO_TARGET_DIR` via garde, format, Clippy et build personnel. Aucun skip, ignore nouveau, retour anticipé, assertion affaiblie ou réglage du microphone pour obtenir un succès. Arrêt/arbitrage après échec selon AGENTS.
4. `rust_review_coordinator` : reviewer indépendant, deux cycles maximum ; principal archive/vérifie les preuves, consigne les résultats et commite localement, sans push. Remise corrigée si la revue et les contrôles le permettent, sans écraser avant/après du lot précédent.

## Acceptation

- Les trois scénarios Auto (repli avec capture ouverte, absence de worker GPU, arrêt avant Ready GPU) passent avec capture synthétique, même sans microphone Windows disponible.
- Les mêmes invariants sont exercés : préférence Auto, repli CPU unique, preuve GPU effacée, Ready CPU accepté et audio capturé conservé lors de l'arrêt/repli.
- N1 complet exécuté, code 0 et zéro échec ; nombre/noms réellement exécutés, ignorés et commandes rapportés. Les résultats 722/0 et 767/3 ne sont pas assimilés sans préciser sélection/version/environnement.
- Format/lints/build et revue indépendants du contrôle Windows. Les sélections workers suspendues du lot précédent restent explicitement non exécutées ; aucun succès natif déduit.

Exécution : NON EXÉCUTÉE au moment du mandat ; résultats à compléter avant clôture de ce document.
