Avancée : cause du refus « DuplicateStateChanged » localisée dans la réservation du remplacement.
Blocage : le code compare le catalogue historique complet à la liste des seuls traitements présents affichée à l'utilisateur.
Décision : autorisation demandée pour un correctif Rust limité à deux fichiers ; aucune donnée personnelle n'est modifiée par la recette.

# Correctif du catalogue de remplacement après RTC-05

## Constat

Dans la destination observée le 21 octobre 2026, l'index `.whisper-publications/index.json` contient dix traitements du même audio, mais seuls deux dossiers de traitement avec `CURRENT` existent encore sous `transcriptions/`. Le parcours d'import filtre le catalogue par `verified_present` dans `worker_ipc.rs` avant d'afficher les choix. Lors de la confirmation, `ArchiveStore::reserve_reprocessing_import` compare cette liste filtrée au résultat non filtré de `reprocessing_catalog` (`archive.rs`, autour de la ligne 2000). Ce désaccord permanent produit « DuplicateStateChanged: le catalogue des traitements a changé ». « Créer un nouveau » suit une réservation distincte et fonctionne.

## Lot proposé

- Dans `crates/whisper-adapters/src/archive.rs`, recharger le catalogue sous le verrou de publication et comparer au catalogue présenté **uniquement ses entrées `verified_present`**, dans le même ordre. Conserver le catalogue complet pour les autres vérifications qui en ont besoin ; les traitements absents ne sont jamais sélectionnés comme cibles ni supprimés.
- Dans `crates/whisper-adapters/tests/reprocessing_cleanup.rs`, ajouter une fixture avec des entrées historiques absentes et au moins deux traitements présents du même audio. Prouver que « Remplacer le dernier », « Remplacer tous » et le traitement choisi peuvent réserver les seules cibles présentes ; prouver qu'une cible qui disparaît ou change entre le dialogue et la confirmation reste refusée. La nouvelle publication précède toujours tout nettoyage et un échec conserve les anciens dossiers.

Chemins de code autorisés à demander : ces deux fichiers seulement. Aucun changement de protocole, de façade Windows, de suppression native, de Cargo ou de l'interface. Écritures d'exécution seulement sous `target/parakeet-tests/retraitement-cible/catalogue-verifie/` et le `cargo-shared` canonique, après garde à échec fermé et contrôle des 30 Gio libres. Aucun test ne cible `WhisperTranscriptions` ou les réglages personnels ; aucun réseau, push ou paquet avant revue CLEAN.

Vérifications : tests ciblés de remplacement, `cargo fmt --all -- --check`, puis `cargo test --offline --locked --workspace --no-fail-fast` sous garde. Implémentation par `rust_implementer` ; revue par `rust_review_coordinator` et reviewer indépendant, deux cycles maximum. La recette dans la fenêtre Windows reste à l'utilisateur.
