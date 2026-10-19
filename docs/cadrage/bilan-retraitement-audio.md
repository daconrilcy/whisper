Avancée : candidat R11 implémenté dans les neuf chemins du plan ; tests automatisés verts sous garde.
Blocage : la revue indépendante relève une reprise impossible après une coupure entre la création des exports et `CURRENT` (R5, D-A5).
Décision : LIVRÉ AVEC RÉSERVES après deux cycles ; aucun code commité ni installation avant arbitrage utilisateur.

# Bilan provisoire R11 — retraitement d'un audio déjà publié

Date : 19/10/2026. Branche : `evo-startup-perf`. Base : `e3918aab2fe7702e1f2e93e8fea7c3d7338c7446`. Candidat : arbre de travail, patch SHA-256 `D2F2CA61563A39CEE9D4F76489B5AC6E08DEC856CC000A22A82C46A7E6636DF7`.

## Réalisation et vérification

Le candidat ajoute la détection par SHA-256 des octets de l'audio dans la destination choisie, un dialogue Annuler / Remplacer / Créer un nouveau, une réservation avant file, des noms TXT/SRT numérotés, et une visibilité durable des publications remplacées. Les neuf fichiers modifiés sont exactement ceux du [plan R11](plan-retraitement-audio.md). Les ressources non suivies `models/` et `sound example/` sont préservées.

Le diagnostic R11.1 a obtenu deux publications distinctes pour deux imports successifs dans le même processus ; la réutilisation forcée du même couple job/génération reproduit `generation is already published`. La cause exacte du cas observé dans l'application installée n'est donc pas attestée par ce diagnostic.

Vérifications finales rapportées par l'implémenteur : `cargo fmt --all -- --check` PASS ; test ciblé de collision avec fichier tiers aux mêmes octets PASS ; suite `whisper-adapters` PASS ; `cargo test --offline --locked --workspace --no-fail-fast` PASS sous la garde R11, 52 groupes de résultats, 0 échec, 34 tests ignorés ; `git diff --check` PASS. La garde n'a relevé aucun écart d'écriture. Les tests ignorés ne sont pas des preuves de réussite. La recette D-A8 dans la fenêtre Windows est NOT RUN.

## Revue indépendante et réserve

Deux cycles de revue/correction ont été utilisés. Les constats R1 à R4 sont fermés. Le verdict final est **LIVRÉ AVEC RÉSERVES** en raison de R5, classé BLOQUANT pour D-A5.

Dans `archive.rs`, `commit_publish` appelle `install_publication_exports` avant d'écrire `CURRENT`. Si le processus s'interrompt après la création d'un export et avant son marqueur de propriété, la reprise dans une nouvelle génération retrouve ce chemin sans provenance et le refuse comme collision. La coupure `import.before_current` et la reprise du pending donnent le scénario de reproduction. Le refus protège les fichiers tiers, mais empêche de terminer cette reprise. La correction devra récupérer uniquement les exports dont la propriété peut être prouvée, même si un fichier tiers a exactement les mêmes octets ; elle devra préserver les anciennes publications actives jusqu'au commit.

Références de revue : `crates/whisper-adapters/src/archive.rs::commit_publish` lignes 2533–2536 ; `install_publication_exports` lignes 2887–2935 ; `resume_from_with_qualification` lignes 2273–2307. Un échec après le premier des deux exports peut créer le même blocage.

Conformément au plan, R11 n'est pas livrable tant que D-A1 à D-A7 ne sont pas satisfaits ensemble. Aucun troisième cycle de correction n'est lancé sans décision de l'utilisateur. Le candidat reste non commité et l'installation personnelle n'est pas modifiée.
