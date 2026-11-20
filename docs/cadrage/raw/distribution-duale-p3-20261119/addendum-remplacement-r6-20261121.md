Avancée : les tests R6 de sauvegarde, migration et restauration du bootstrap sont retrouvés dans le journal P3 et réussissent.
Blocage : cette preuve couvre les tests automatisés du contrat de remplacement ; aucun essai manuel de remplacement d’une installation réelle n’est enregistré ici.
Décision : la phrase du plan qui disait que les états R6 restaient à tester était périmée ; P3 utilise désormais cette preuve automatisée et conserve sa limite explicite.

# Addendum — tests R6 de remplacement — 21 novembre 2026

Cet addendum rattache au dossier P3 les résultats automatisés R6 déjà exécutés. Il ne relance ni Cargo ni une installation et ne modifie pas le candidat.

Le journal original a été archivé hors de `target/` sous `journal-r6-remplacement-stdout-20261120.log` ; il correspond octet pour octet à `target/parakeet-tests/distribution-duale-p3-20261119/scratch/20261120T225314584-8661348b7ae34ca1acc4ab4f231f9bbd/cargo-command.stdout.log` : 137 759 octets ; SHA-256 `CDEE1A39361ECCF6EEF1C9D8364E4CB50BA2271B611FC3D5A9B636C886AD561C`. Le binaire de test du bootstrap y termine avec `42 passed; 0 failed; 0 ignored`. Les 19 tests `installer::replacement::tests` suivants sont explicitement marqués `ok` dans ce journal :

```text
close_poll_propagates_probe_errors
close_poll_skips_prompt_when_absent_and_succeeds_after_normal_close
close_poll_times_out_after_thirty_seconds_without_a_kill_action
free_space_threshold_covers_backup_staging_and_rollback_with_margin
free_space_threshold_rejects_overflow
free_space_threshold_uses_ceil_ten_percent_and_strict_boundaries
backup_copy_failure_does_not_modify_source
publish_failure_restores_old_root_without_discarding_recovery_paths
failed_restore_reports_recovery_incomplete_and_keeps_all_paths
failed_publish_restores_verified_backup_and_preserves_artifacts
publication_boundary_rejects_reactivated_whisper_without_moving_root
missing_or_altered_immutable_resource_refuses_existing_install
migration_preserves_settings_and_logs_bytes_from_verified_backup
altered_or_omitted_migration_is_rejected_before_publication
post_publish_verification_failure_restores_old_root_and_preserves_new_tree
publish_success_keeps_verified_backup_and_previous_root
publication_boundary_rejects_changed_or_added_user_state_without_moving_root
recognized_install_allows_only_optional_ordinary_user_state
post_publish_user_state_mismatch_restores_old_root_and_keeps_new_tree
```

Ces tests couvrent la fermeture normale sans arrêt forcé, le calcul d’espace libre, la préservation de la source en cas d’échec de sauvegarde, la reconnaissance des ressources immuables et données utilisateur, la migration depuis une sauvegarde vérifiée, la publication ainsi que les restaurations après échec. La preuve ne représente pas une recette manuelle ou de bout en bout sur une installation utilisateur réelle.

## Références

- Compte rendu de l’installation vierge et du routage : [preuve P3](preuve-p3-installation-routage-20261120.md).
- [Addendum d’identité P3](addendum-identites-p3-20261120.md).
- P4 demeure INCOMPLET selon le [préflight des notices et des droits](../distribution-duale-p4-20261120/preflight-identite-notices-p4-20261120.md).
