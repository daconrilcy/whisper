Avancée : T10 validé sur `1656ac8` ; N1 ×3, corpus, N2 4/4 et build release réussis.
Blocage : recette interactive Windows non exécutée ; elle reste à faire par l’utilisateur.
Décision : clôture technique T10, sans merge ni push.

# Suivi T10 — REFACTO-POLL-PARAKEET

Branche `refacto-poll-parakeet`, commit du candidat produit `1656ac859c1e5712421cbc86bda1c5af553e8dfc`. La correction finale concerne une fixture de polling ; les assertions et références gelées restent inchangées. Revue indépendante de cette correction : **CLEAN**.

| Fonction | Avant → après | Nettoyage : fonction (complexité) | Polling : fonction (complexité) |
|---|---:|---|---|
| `run_cleanup_plan_locked` | 32 → 3 | `validate_cleanup_successor_records` (1) | `active_parakeet_ready_context` (1) |
| `poll_parakeet_worker` | 30 → 9 | `open_verified_cleanup_index` (1) | `parakeet_ready_identity_matches` (2) |
| — | — | `validate_cleanup_successor_files` (5) | `parakeet_ready_backend_matches` (2) |
| — | — | `adopt_missing_cleanup_containers` (6) | `prepare_single_pass_parakeet_transcription` (2) |
| — | — | `hold_cleanup_target_ancestors` (6) | `handle_parakeet_ready` (12) |
| — | — | `verify_cleanup_target_objects` (5) | `handle_single_pass_parakeet_result` (5) |
| — | — | `capture_unselected_cleanup_entries` (5) | `handle_parakeet_result` (2) |
| — | — | `remove_cleanup_target_object` (1) | `handle_parakeet_failure` (4) |
| — | — | `remove_cleanup_target_objects` (4) | `poll_stopped_parakeet_exit` (6) |
| — | — | `remove_cleanup_containers` (5) | `poll_idle_parakeet_exit` (4) |
Mesure Clippy JSON, même configuration `recette-hooks` et lints structurels. Maximum extrait : 12 ; toutes les fonctions sont ≤15. Clippy qualité termine avec succès et n’ajoute aucun avertissement dans les fichiers touchés. La portée du verrou reste celle de la fonction d’origine : le verrou externe est gardé pendant toute l’exécution; aucune frontière ni étape d’acquisition/libération n’a été déplacée. La revue finale du refactoring nettoyage avait vérifié les handles d’index, d’ancêtres et de cibles, l’ordre suppression/marquage et l’écriture finale.

**Équivalence et validation.** N1 final ×3 : chacun 944 réussites, 0 échec, 129 ignorés. `cargo test -p whisper-adapters` passe : 22 tests d’équivalence polling et 44 nettoyage ; les traces brutes/stabilisées correspondent aux références versionnées. ZIP de preuves complété : 6 912 entrées ; les 6 683 précédentes conservent leur contenu SHA-256. S2 : filtre exact ×50, 50 réussites, cause historique non démontrée (correctif préventif). Workers : CPU 12/0/9, Parakeet CPU 26/0/11, common 11/0/1 ; contrôles PE réussis. `cargo fmt --check` et Clippy passent.

N2 PersonalUnified : 4/4 PASS. Les 8 sorties TXT/SRT sont identiques octet par octet aux références (mêmes tailles et SHA-256) ; WHEA reste à 1 événement, RecordId 527992. Build release Personal réussi ; remise dans `deliverables/refacto-poll-parakeet/` : desktop et deux workers Turbo avec leurs empreintes consignées dans les preuves. Aucun build natif CMake/ggml/NeMo/CUDA exécuté.

Espace libre : début T10 417 581 723 648 octets ; clôture 414 027 091 968 octets. Racines conservées : `target/refacto-poll-parakeet` : 3 635 824 142 octets ; `target/n2-native` : 16 678 719 446 octets ; `deliverables/refacto-poll-parakeet` : 38 652 928 octets. Les caches Cargo partagés restent préservés. Aucun nettoyage exécuté. L’archive de preuves est `docs/architecture/raw/REFACTO-POLL-PARAKEET-PREUVES.zip` (24 560 887 octets, SHA-256 `146A93C75893A40EFFD6B5E452653870ACA8D75FD1BFA3FD82C03C31850D6646`).

**Recette humaine à exécuter sur l’application release remise :** (1) importer un fichier Parakeet en CPU et vérifier les sorties ; (2) lancer le micro en GPU 30 s, arrêter et vérifier le texte confirmé et les fichiers produits ; (3) démarrer une courte nouvelle session ; (4) annuler un import pendant son traitement, vérifier la disparition des fichiers temporaires et la conservation des sorties déjà publiées. Résultat : **NON EXÉCUTÉ**, fenêtre Windows réservée à l’utilisateur. Aucun défaut produit observé ou corrigé dans T10. Aucun merge ni push.
