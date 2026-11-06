Avancée : DESIGN revu CLEAN ; D-01 fermé après un cycle de correction.
Blocage : aucun pour la conception ; plan à produire et relire.
Décision : passer à la planification autorisée, sans code.

# INSPECT-MICRO-DEMARRAGE — revues documentaires

Ce relevé attribue les verdicts aux reviewers ; il ne vaut ni autorisation d'implémentation ni validation du produit. Auteur de la transcription : session principale. Rapports bruts accessibles dans les échanges de cette session ; transcription utile, sans revendication de copie octet pour octet.

## DESIGN — revue initiale

Reviewer : `/root/design_review`, profil `rust_design_reviewer`, indépendant des auteurs. Objet : `evo-inspect-micro-demarrage.md`, commit `4ca07eb54560adf3a0d416402b962ad146619d16`. Le reviewer a vérifié la révision et l'absence de diff local avant/après. Revue en lecture seule, aucun Cargo/test.

Verdict rendu : **FINDINGS**.

### D-01 — BLOQUANT / High / REQUIRED

**Demandes durables refusées pour un modèle non sélectionné.** DEC-01 ne déclenche l'inspection que depuis la sélection. Or `DesktopAdmissionApplication::admission_result`, `root.rs:651–685`, contrôle aussi `ProcessQueued` et `ResumeInterrupted` avant les bindings ciblés. `ui/model/view/queue.rs` construit ces commandes depuis la requête durable, indépendamment du choix des réglages. Une entrée B non inspectée peut donc rester refusée avec A sélectionné, contrairement à CA-06/REQ-06 et INV-06.

Condition de fermeture émise par le reviewer : définir déclenchement ciblé, attente/rejeu ou reprise explicite, erreurs, générations et annulation pour ces demandes ; préserver le choix utilisateur et interdire l'admission optimiste. Tester le vrai garde de composition, file/reprise B avec A sélectionné. Aucun changement des algorithmes file/reprise n'est exigé.

Autres conclusions du reviewer : couverture CAP-01 à CAP-08 par REQ/UC/AC ; contrats ciblage, identité, concurrence, mesure et audio présents ; tests produit NOT RUN légitimes ; aucun SPIKE ni nouveau railguard requis.

## DESIGN — vérification de correction

Reviewer inchangé `/root/design_review`, révision `426c16ec2b0bbf8911e1ce80c744d7b1f37cd4b3` : document complet et diff `4ca07eb5..426c16ec` lus, HEAD et absence de diff local vérifiés directement. Verdict rendu : **CLEAN — D-01 CLOSED**, après un cycle de correction.

Le reviewer confirme : DEC-05 déclenche l'inspection ciblée sur commande explicite sans changer le choix, conserver/rejouer une commande ou réutiliser une preuve GPU ; relance soumise aux contrôles actuels, demandes indépendantes de la coalescence du choix et version d'observation distincte. AC-11 couvre le vrai wrapper pour file/reprise et les variants requis. Aucun REQUIRED restant.

Le CLEAN concerne uniquement le cadrage. Mesures humaines, tests produit et remise restent NOT RUN ; Q-M01/03/04 à arrêter avant référence. Aucun droit de coder déduit. La présente transcription n'était pas l'objet DESIGN revu.

## PLANS

Reviewer indépendant inchangé `/root/design_review`. Parent DESIGN `09f5f976` vérifié. Première revue PLANS sur `4420df17b9ce4f90440c3b6217e52fac8ccb0f33`, lecture seule, HEAD/diff local contrôlés avant/après : **FINDINGS**, sans DESIGN_CHANGE_REQUIRED.

### P-01 — BLOQUANT / High / REQUIRED

La commande commune de construction CPU/GPU peut unifier la feature CUDA de leur dépendance `whisper-rs 0.16.0` et rendre le CPU dépendant de CUDA, contraire à REQ-06/AC-08. [Référence primaire Cargo consultée par le reviewer](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification). Fermeture : workers inchangés existants identifiés/vérifiés ou constructions réellement séparées, PE explicite de l'EXE CPU livré, provenance et copies avant/après ; PE des harnais insuffisant pour qualifier les EXE.

### P-02 — IMPORTANT / Medium / REQUIRED

L'appellation build Rust ne garantit pas absence de production native : `whisper-rs-sys 0.15.0/build.rs` appelle CMake. La présence des DLL/cache ne prouve pas sa réutilisation sans reconstruction. Fermeture : voie autorisée démontrée avant appel, réutilisation workers inchangés ; si natif nécessaire, tâche bloquée et décision précise requise, aucun contournement.

Autres conclusions : dix tâches/graphe cohérents ; baseline avant correctifs ; onze AC reliés aux vrais consommateurs, notamment AC-11 ; garde, N1, PE, revue et remise humaine distingués ; aucun autre REQUIRED.

Correction PLAN v2 attribuée `/root/plan` : retirer toute construction workers, identifier/protéger mêmes EXE avant/après par provenance/SHA/PE ; préflight de toutes les commandes compilantes pour scripts natifs/réseau enfant. Découverte complémentaire statique : desktop→adapters→webrtc-vad, cc libfvad et git submodule possibles. Preuve de voie conforme non fournie : dépendance d'exécution explicite, pas de succès prétendu. Revue de correction à effectuer ; auteur ne ferme aucun constat.
