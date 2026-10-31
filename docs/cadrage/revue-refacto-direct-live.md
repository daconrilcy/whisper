Avancée : revue indépendante DESIGN terminée ; DL-D01 fermé.
Blocage : aucun constat requis ouvert sur le design ; exécution non autorisée.
Décision : conserver la séparation entre revue documentaire, exécution et validation Windows.

# Revues indépendantes — refactoring direct_live

Reviewer : `rust_design_reviewer`, agent `revue_design_plan`, distinct des auteurs des contributions et de la session principale qui écrit les documents. Aucun code ou document modifié par le reviewer, aucune commande Cargo, test ou délégation. Les retours bruts sont retranscrits ci-dessous ; les corrections restent attribuées aux auteurs, la fermeture au reviewer.

## DESIGN — retour initial brut

Avancée : revue DESIGN terminée sur `78ad061183e6379b022fcf92abf20fddbf12af9f`.
Blocage : aucun blocage produit ; une contradiction de visibilité est à corriger.
Décision : **DESIGN FINDINGS** — un constat IMPORTANT / REQUIRED.

**Indépendance.** Je n’ai participé ni aux contributions ni à la rédaction du candidat. Lecture seule : aucune modification, commande Cargo, exécution de test ou délégation.

**Candidat vérifié :** `docs/cadrage/refacto-direct-live.md`, DESIGN-DL v1. Le contenu de travail correspond au fichier commité. Base produit comparée : `17cd4927ea431ab7fb5b8c0fde7cda8c7ccd07e0` ; seul le document de cadrage a changé.

**DL-D01 — IMPORTANT / REQUIRED — Préserver explicitement la visibilité crate de `pump_capture`.**

- **Preuve :** le design déplace `pump_capture` vers `direct_live/capture.rs` (ligne 84) puis prescrit les méthodes transverses d’impl en « `pub(super)` au plus » (ligne 94). Or cette méthode est actuellement `pub(crate)` dans `direct_live.rs:470` et appelée depuis `worker_ipc/runtime/support.rs:437`, ainsi que les tests IPC aux lignes 515 et 542. Le consommateur `runtime/support.rs` manque dans le relevé ligne 98.
- **Conséquence :** appliquer littéralement cette restriction rendrait le pompage temps réel inaccessible au runtime et casserait la compilation, contrairement à REQ-02/AC-02 et au contrat d’API inchangée.
- **Fermeture attendue :** distinguer explicitement les helpers internes nouvellement exposés en `pub(super)` des méthodes existantes `pub(crate)` qui conservent leur visibilité, notamment `pump_capture` ; compléter le consommateur temps réel dans la carte et ses contrôles futurs. Aucun élargissement de périmètre produit n’est nécessaire.

**Couverture vérifiée dans les deux sens :**

- CA-01/02 → REQ-01/02 → UC-01/02, AC-01/02 → découpage et mesures : couverts, sous réserve DL-D01.
- Capture/gain/télémétrie/arrêt → REQ-03 → AC-03/04 → DOM-01/02/03, INV-01/02/04, parent/capture/worker/WAV et RISK-F01 : couverts.
- CPU/GPU/Auto/erreurs → REQ-04 → AC-05 → transitions Ready/repli/Failed, INV-03/04, worker et RISK-F02 : couverts sans nouveau comportement.
- Exports Turbo/Parakeet → REQ-05 → AC-06 → DOM-04, INV-05, outputs/WAV et RISK-F03 : couverts.
- Conservation/séparation des tests → REQ-06 → AC-07 → tests/d2_corpus/test_support et RISK-T01 : couverts.
- Aucun module proposé ne manque de justification dans les exigences ; aucun changement de fonctionnalité, dépendance, protocole ou format ajouté.

**Preuves consultées :** AGENTS racine/adapters ; skill et contrat de cadrage ; règles métier ; manifeste racine/adapters ; production de `direct_live.rs`, imports et inventaire lexical de ses tests ; références Parakeet/microphone/IPC, dont le pompage runtime ; gardes archive et core/t10 pour corroborer leur inapplicabilité automatique.

La faisabilité du découpage est étayée statiquement. Les tests produit et la livraison native/Windows restent correctement **NOT RUN**. Le prérequis de garde ouvert interdit une future campagne mais ne constitue pas, à lui seul, un défaut de cette conception structurelle.

## Correction proposée par l’architecte

ARCH-DL-v2 corrige DEC-T02 : visibilités existantes conservées, notamment pump_capture pub(crate), nouveaux helpers limités à pub(super), consommateur runtime/support et appels IPC ajoutés. Intégré par la session principale dans DESIGN-DL v2. DL-D01 reste ouvert jusqu’au verdict indépendant ci-après.

## DESIGN — retour de fermeture brut

Avancée : correction DESIGN vérifiée sur `1a2522bfa8caececde517c898496bbbd035a3a03`.
Blocage : aucun constat requis ouvert sur la conception.
Décision : **DESIGN CLEAN** — DL-D01 fermé.

Candidat : `docs/cadrage/refacto-direct-live.md`, **DESIGN-DL v2**. Le fichier de travail correspond à la révision Git revue. Mon indépendance et la lecture seule sont conservées.

**DL-D01 — CLOSED.** DEC-T02 distingue désormais les API existantes `pub(crate)`, notamment `pump_capture`, des nouveaux helpers limités à `pub(super)`. Le consommateur `worker_ipc/runtime/support.rs:437`, les appels IPC aux lignes 515/542 et le contrôle des visibilités figurent explicitement dans le document. La contradiction relevée dans v1 est levée, sans changement de périmètre ni de comportement.

La comparaison v1→v2 ne révèle aucun autre changement de conception. La couverture CA-01 à CA-06 et la traçabilité précédemment vérifiées restent valides. Aucun REQUIRED ni ADVISORY résiduel.

Ce verdict permet le passage documentaire à la planification autorisée. Il ne vaut ni autorisation d’implémentation ni validation compilée/native/Windows ; le prérequis de campagne reste à résoudre avant Cargo.
