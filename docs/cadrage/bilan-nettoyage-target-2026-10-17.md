Avancée : les 119 chemins autorisés ont été supprimés après archivage vérifié ; la suite workspace et les dix essais isolés de live_archive passent.
Blocage : l'échec intermittent StorageUnavailable n'a pas été reproduit ; sa cause système exacte reste inconnue.
Décision : le R8 historique doit être rapproché de la clé Test R6 actuelle avant de pouvoir clore son critère.

# Bilan du nettoyage de target — 17 octobre 2026

Le manifeste exact est `C:\dev\whisper-evidence\nettoyage-2026-10-17-manifeste.tsv` et le journal d'exécution est `C:\dev\whisper-evidence\nettoyage-2026-10-17.log`. L'archivage a conservé l'arborescence de 1 396 petits fichiers (4 565 518 octets) sous `C:\dev\whisper-evidence\<lot>`. Le nombre de fichiers et chaque SHA-256 ont été vérifiés avant toute suppression, puis revérifiés juste avant l'exécution. Le bilan R7 a été déplacé dans `docs/cadrage/bilan-parakeet-r7.md` au commit `f5313734`. Les recettes des WAV synthétiques ont été commitées dans `docs/cadrage/recettes-wav-synthetiques.md` (`554f493e`) avant leur suppression. La règle de campagne est dans `AGENTS.md` (`ea4c2dd0`).

| Type | Entrées supprimées | Condition contrôlée |
| --- | ---: | --- |
| Répertoires Cargo clos | 16 | chemin nominatif du manifeste |
| Copies de modèles | 15 | taille et SHA-256 revérifiés ; 14 copies Turbo étaient des liens physiques recréés par le test `resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution` de `worker_ipc.rs` |
| DLL redondantes | 25 | taille et SHA-256 revérifiés |
| WAV synthétiques | 63 | recettes de régénération consignées avant suppression |

Le manifeste représente 88 187 460 469 octets logiques (82,13 Gio). L'espace libre est passé de 97 720 160 256 à 153 908 649 984 octets, soit un gain physique de 56 188 489 728 octets (52,33 Gio). Les fixtures `l2/l2a-2/native/r8-incompatible-*`, `spike02`, le worker R6 admis et ses DLL, les WAV bruités B9, les entrées R7 encore lues et le Cargo R7 ont été conservés. Aucun chemin absent du manifeste n'a été supprimé. Après reconstruction dans `cargo-shared`, l'espace libre relevé est d'environ 150,3 milliards d'octets.

La garde Cargo partagée a refusé la racine erronée `CARGO_TARGET_DIR=C:\dev\whisper\target` avant lancement, puis accepté la racine `target/parakeet-tests/cargo-shared`. `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only` et le seuil de 30 Gio ont été appliqués. La garde exécutée et ses journaux ont été archivés hors Git avec contrôle SHA-256.

Le test isolé `attempt_journal_reserves_monotonically_and_enforces_transitions` est passé dix fois de suite. Aucun `StorageUnavailable` n'a été observé ; l'erreur Windows exacte et son chemin ne peuvent donc pas être attribués. La suite `cargo test --offline --locked --workspace --no-fail-fast` est passée : 364 tests réussis, zéro échec et 29 ignorés ; `live_archive.rs` a passé ses huit tests. Les essais natifs ignorés restent `NOT RUN` dans cette commande, notamment les cas R8. Les journaux complets se trouvent sous `C:\dev\whisper-evidence\l2b-r7\`.

Le plan historique `plan-parakeet-l2a2-acceptance.md` fixe le worker `545a…` et une racine Cargo par lot. La fiche Test courante fixe le worker R6 `1d8f…` et la règle permanente fixe Cargo partagé. Les tests R8 existants peuvent être examinés sur la clé courante, mais leur exécution sur une autre clé ne clôt pas formellement le plan historique tel qu'écrit.
