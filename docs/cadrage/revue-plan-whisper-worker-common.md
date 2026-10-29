Avancée : revue PLANS indépendante terminée sur les fichiers Git.
Blocage : aucun constat REQUIRED ; réalisation conditionnée aux accords et à G-EXEC.
Décision : PLANS CLEAN — PLANS READY documentaire pour soumission à l'utilisateur.

# Revue du plan whisper-worker-common

Date : 29 octobre 2026. Reviewer : agent `rust_design_reviewer`, tâche common_review, distinct du plan writer et des auteurs du design. Aucun rôle de rédaction/correction, aucune mutation, aucun Cargo/test/build ni sous-agent pendant la revue. Auteur de ce fichier : session principale ; résultat ci-dessous intégré du retour du reviewer.

Objet revu : **P-WCOMMON-01 v1**, `docs/cadrage/plan-whisper-worker-common.md`, commit `8b1d7bdbe1a8ef264af78307339f4bf1b38ae164`.

Design parent vérifié : **D-WCOMMON-01 v1**, contenu de `087af34797e91269b0614f95cd5e6c62cbe0a428`, DESIGN CLEAN ; statut documentaire dans `revue-whisper-worker-common.md` au commit `466c756a5e26338117c3b86d9d7a8b8959ba4403`.

## Verdict indépendant

**PLANS CLEAN**, prêt documentaire pour soumission à l'utilisateur. Findings REQUIRED : aucun. Aucun finding ADVISORY nécessaire.

La vérification réciproque confirme :

- WC01 à WC08 disposent de tâches, chemins, données, commandes proposées, observations et limites ; WC07 demeure soumis à l'acceptation de la liste fermée.
- Les écritures correspondent au design : common, façades, manifestes/lockfile et tests nommés ; consommateurs indirects en lecture seule.
- Neuf tâches séquentielles, aucun cycle ni parallélisme de code ; quatre rapports prévus.
- T1 précède la substitution et caractérise l'ancien code ; oracle MP3 observé puis figé, sans recalcul par la nouvelle implémentation ni auto-régénération.
- Couverture PCM non silencieux, fenêtres/plages/refus, gapless conditionnel, cycle MP3, limites IPC LF inclus, erreurs sérialisation/I/O, intégrité, raccordement et hooks.
- Builds/tests CPU et GPU séparés ; SHA2 production pour recette-hooks ; refus release conservé statiquement sans échec artificiel.
- Cohérence locale lockfile et exception explicite à locked distinguées des contrôles offline/locked suivants ; aucune dérive externe autorisée.
- Mandat/racines/garde compatible/variables/cache privé/auto-tests/surveillance/espace libre sont des prérequis explicites. Aucune garde/dérogation/autorisation de campagne inventée.

Le reviewer a vérifié la stabilité avant/après et l'identité aux contenus Git : plan `66E3D46B6502858386E8FB4BC1B1DAA2BA0010D2CF5FC99D01B7C7B5BD3F4E46`, design parent `EA710CFFBC3944A59824A6C64B976BBDDD264B0721337ED6C2783D3169A902C4`. Aucun manifeste d'empreintes distinct créé. Aucun code produit modifié dans les commits documentaires comparés à la base.

## Remise par la session principale

**DESIGN READY et PLANS READY documentaires**, sur les contenus Git identifiés ci-dessus. Les mentions DRAFT/à revoir dans les fichiers sources correspondent à leurs états de soumission ; les deux comptes rendus de revue établissent le statut courant sans altérer les contenus examinés.

À accepter avant réalisation : liste fermée decoder/encoder/transport IPC/integrity SHA-256 et plan ; accord précis de code/manifestes/lockfile/campagne et résolution concrète de G-EXEC. La préparation de ces documents ne fournit aucun de ces accords.

Contrôles de cette mission : lectures/comparaisons statiques, état Git, portée des commits et git diff --check. Tests, builds, preuve native et fenêtre Windows : **NOT RUN**. Pas de push. Aucun code, manifeste ou lockfile modifié. Éléments non suivis préexistants préservés.

Budget PLANS : une revue, aucun finding ni cycle de correction. Livrable terminé ; aucune implémentation lancée.
