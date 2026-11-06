Avancée : trois N1 complets et 50 répétitions PASS, sans CARGO_TARGET_DIR ; 840 → 843 tests.
Blocage : absence de fenêtres Windows NON ATTESTÉE ; cargo simple depuis main NON EXÉCUTÉ (prévu humain après fusion).
Décision : LIVRÉ AVEC RÉSERVES ; fusion technique recommandée, laissée à l’humain ; aucun merge/push.

# TESTS-HYGIENE — bilan figé du 6 novembre 2026

Branche `tests-hygiene`, base `9f5ab7aa`, worktree `C:/wt/tests-hygiene`. Aucun code produit, dépendance ou Cargo.lock modifié.

**Corrections.** A : worker Rust dans son temporaire possédé/nettoyé, commun lib/intégration, sans CARGO_TARGET_DIR. B : PE vérifiés (CPU/GPU, Auto, sélection/historique), verrou d’échec de lancement ; repli CPU unique et assertions conservés ; contrôle source et témoins négatifs, 125 lignes. C : filtrage des seules notices Diagnostic avant événement métier, délai inchangé de 3 s, témoin déterministe ; cause historique exacte NON PROUVÉE. La collision PID révélée en répétition 15 est corrigée par racines uniques, réservation atomique et nettoyage après fermeture des ressources ; JSON, identité et FIFO inchangés.

| Vérification finale, candidat corrigé | Résultat |
|---|---|
| N1 complet ×3, garde sans variable | Chaque fois **715 passés / 0 échec / 128 ignorés**, 59 cibles identiques ; 35,70 / 11,64 / 11,13 s au total |
| Bibliothèque adapters ×50 | Chaque fois **372 / 0 / 49** ; 5,51–6,32 s de tests ; C, témoin et unicité **50/50** |
| Gardes et structure | N1 SelfTest **49 positifs / 75 négatifs PASS** ; native SelfTest **PASS** sans Cargo/diagnostic natif ; registre **961** |
| Format et lint | Rustfmt exact-stdin **10 sources PASS** ; 3 fragments include relus ; Clippy adapters all-targets **exit 0**, avertissements, sans `-D warnings` |

Trois ajouts, aucun retrait : contrôle PE, diagnostic prioritaire, unicité sans création. Recherche environnement : **61 → 60 occurrences**, CARGO_TARGET_DIR supprimé ; 7 enfants configurés, 51 natives/qualification, 2 replis conservés. Liste fichier:ligne archivée.

**Limites.** TH-R1/R2/R3 fermés ; N7 CLEAN statique. TH-R4 **MINEUR** ouvert : arguments invalides pour `fmt --check` sans variable, voie non utilisée. Seul le C autorisé de webrtc-vad 0.4.0 compilé ; aucun build ggml/NeMo/Protobuf/CUDA. Mémoire validée **12,8 → 17,3 %**, nouvelle WHEA : **0** ; aucun exécutable survivant identifié dans les racines du lot.

**Rétention.** 8 racines utilisateur préexistantes non attribuées, 1 657 921 octets ; 76 racines du lot, 212 881 842 octets logiques, inventaire dédupliqué. Commandes `-WhatIf` proposées, aucun nettoyage exécuté ; caches/espaces natifs conservés. Les échecs antérieurs restent archivés, exclus de la validation finale.

**Preuves :** [archive vérifiée](raw/TESTS-HYGIENE-20261106T070200.zip), 537 fichiers + manifeste ; nombres, tailles et SHA-256 vérifiés, inventaires et commandes de nettoyage inclus.

SHA-256 : `BFDAA362A4F723EC1DF6F648979F523C948F744E8A4A60B7C4E1B3CB47344EA8`.
