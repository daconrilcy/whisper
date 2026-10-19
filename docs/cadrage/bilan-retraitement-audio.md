Avancée : candidat R11 implémenté ; R1 à R6 fermés, voie sans lien physique testée en adaptateur.
Blocage : une coupure entre création exclusive d'un export et persistance de son identité empêche sa reprise sûre (R7, D-A5/D-A7).
Décision : LIVRÉ AVEC RÉSERVES ; le correctif final nécessite un élargissement explicite du sous-lot à l'interface et aux contrats R11.

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

## Reprise demandée par l'utilisateur après le premier verdict

L'utilisateur a autorisé de nouveaux cycles de revue/correction pour lever les réserves. Le correctif R5 a été limité à `archive.rs` et `import_cpu.rs`. Il couvre deux interruptions : après la création du seul TXT, puis après les deux exports et avant `CURRENT`. La reprise dans une nouvelle génération aboutit, les anciennes publications restant actives jusqu'au commit. Le refus d'un fichier tiers ayant les mêmes octets reste testé.

La première nouvelle revue a fermé R5, puis ouvert R6 : un lien physique direct entre export et artefact interne permettait à l'édition du TXT exporté de corrompre l'artefact vérifié par le manifeste. Le correctif a isolé l'artefact interne avec une copie préparée distincte. Le test ciblé vérifie l'édition de l'export, le SHA interne, le scan d'archive et la recherche de doublon. La seconde revue a fermé R6 et confirmé R1 à R5 fermés.

La seconde revue a ouvert **R7 BLOQUANT** : `archive.rs::install_publication_exports`, lignes 2937–2940, exige `fs::hard_link` sans alternative. Une destination exFAT/FAT32 reste sélectionnable, mais ne permet pas cette publication TXT/SRT, contrairement à D-A3/D-A4. La preuve de fermeture demandée est : premier import et Remplacer sur une destination sans liens physiques, TXT/SRT vérifiés, reprise avant `CURRENT`, et fichier tiers en collision intact.

Le nouveau candidat conserve exactement les neuf chemins de code R11. Formatage et suite workspace complète sous garde : PASS, 52 groupes, 0 échec, 34 tests ignorés. La recette D-A8 dans la fenêtre Windows reste NOT RUN. Le diff de ces neuf chemins par rapport à `e3918aab` est identifié par le SHA-1 Git `5344ad96ef3526998978e1a0a1894dcba88669e6` selon le reviewer. Les deux cycles de ce nouveau passage sont épuisés ; aucune correction supplémentaire n'a été lancée.

Conformément au plan, R11 n'est pas livrable tant que D-A1 à D-A7 ne sont pas satisfaits ensemble. Le candidat reste non commité et l'installation personnelle n'est pas modifiée.

## Lot correctif R7 demandé ensuite

Le correctif R7 a été limité à `crates/whisper-adapters/src/archive.rs` et `crates/whisper-adapters/tests/import_cpu.rs`. Une injection force l'absence de liens physiques ; le repli crée les sorties par `create_new` sans écraser un fichier tiers. Les tests adaptateur passent (107 tests de bibliothèque, 2 ignorés), de même que le formatage, sous garde. Ils couvrent la reprise après un ou deux exports dont la propriété est attestée et le refus d'une collision avec un fichier tiers aux mêmes octets.

Une coupure immédiatement après `create_new`, avant la persistance de l'identité du fichier final, laisse toutefois un export sans provenance. La reprise refuse ce fichier et conserve le pending et les anciennes publications ; le test n'aboutit qu'après suppression manuelle du fichier ambigu. Le reviewer indépendant classe donc **R7 BLOQUANT** pour D-A5/D-A7. Le gate workspace complet de ce candidat est **NOT RUN**, arrêté sur ce défaut. Aucun code R7 n'a été commité.

L'adjudication du reviewer situe le refus dans `archive.rs::install_owned_export` vers la ligne 3224 et la reprise vers la ligne 3176. Elle conclut qu'une résolution sans adoption d'un fichier tiers demande une nouvelle confirmation visible et la transmission de ce choix. Les chemins supplémentaires minimaux sont `crates/whisper-desktop/src/ui.rs`, `crates/whisper-core/src/ports.rs`, `crates/whisper-core/src/application.rs` et `crates/whisper-adapters/src/worker_ipc.rs`. Ils figurent dans les neuf chemins du plan R11, mais pas dans le sous-périmètre R7 autorisé à ce stade. Aucune édition supplémentaire n'a été engagée.
