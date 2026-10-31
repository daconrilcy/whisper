Avancée : DESIGN/PLANS CLEAN ; contrôles C1–C9 réussis et revue Rust CLEAN sur le candidat inchangé.
Blocage : aucun pour cette revue ; essais natifs, microphone/GPU réel et fenêtre Windows non exécutés.
Décision : clôturer la revue Rust en conservant avertissements et limites de qualification explicites.

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

## PLANS — retour brut

Avancée : revue PLANS terminée sur `d38a20e41fc8c21d2bf513a1de0d04d481142297`.
Blocage : aucun constat requis sur le plan ; GO et garde de campagne restent des prérequis d’exécution.
Décision : **PLANS CLEAN**.

**Indépendance.** Je n’ai participé ni à la proposition du plan ni à sa rédaction. Lecture seule maintenue : aucun fichier modifié, aucune commande Cargo ni test exécuté.

**Candidat vérifié :** `docs/cadrage/plan-refacto-direct-live.md`, PLAN-DL v1, contenu de travail identique au fichier commité. Parent vérifié : DESIGN-DL v2 au commit `5071a8f0a4d31162d2208364dc25b199805794d1`. Sa différence avec le design déclaré CLEAN sur `1a2522bf` se limite au statut et à la référence du verdict.

**Contrôles effectués :**

- Huit tâches et huit chemins Rust, tous justifiés par le design ; aucun autre fichier de code implicitement ouvert.
- Dépendances séquentielles sans cycle : EXEC-BASELINE est bien produit par T1 avant les extractions. Aucun travail concurrent annoncé sur l’état partagé.
- REQ-01/02 → T1/T3–T7, mesures, API/imports et compilation ; REQ-03 → T2/T3/T5/T7, PCM/gain/backlog/arrêt ; REQ-04 → T2/T6/T7, identité/attestation/repli ; REQ-05 → T2/T4/T7, exports/compensations ; REQ-06 → T1/T2/T7, inventaires et conditions. T8 couvre la revue et la clôture. Aucun chemin ou scénario obligatoire orphelin.
- Noms et appartenances des tests C4a/b/c et C5 vérifiés directement dans les sources ; inclusion du test IPC dans `worker_ipc::tests` confirmée. Les filtres C2/C3 séparent les 18 tests contrôlés et six tests purs du D2 natif.
- Le besoin de `WHISPER_MIC_LEVEL_D2_ROOT` pour les diagnostics synthétiques est pris en compte ; aucune activation du moteur natif déduite.
- Carte d’exécutabilité, propriétaires, résultats comportementaux attendus, limites NOT RUN, arrêt et récupération sont cohérents.
- Préservation explicite de `pump_capture pub(crate)` et des consommateurs vérifiée ; DL-D01 reste fermé.
- Les prérequis non établis ne sont pas présentés comme acquis : autorisation d’implémentation, garde applicable, environnement et ressources natives. Aucun contournement ni ancienne dérogation transférée.
- Aucun ajout de fonctionnalité, format, dépendance, protocole, politique d’erreur ou garantie de durabilité.

**Résultat :** aucun REQUIRED ni ADVISORY. Le plan est admissible pour **PLANS READY documentaire**. Ce verdict ne vaut pas acceptation utilisateur du lot, autorisation de coder ou de lancer Cargo ; compilation, tests produit, moteurs natifs et fenêtre Windows restent **NOT RUN**.

## Clôture documentaire par la session principale

Le périmètre livré est limité aux trois documents prévus. Une correction DESIGN (DL-D01) a été vérifiée puis fermée par le même reviewer ; aucun cycle de correction PLANS n’a été nécessaire. Les mises à jour finales portent uniquement sur les statuts et la transcription des verdicts.

Vérifications réalisées : lecture des règles et sources citées, correspondance des chemins, absence de modification produit dans le diff cumulé, `git diff --check` sur les changements documentaires. Aucune compilation, aucun test ni campagne lancé. Espace libre final relevé avant clôture : environ 165 Gio. Les trois familles de fichiers non suivis initiales ont été préservées ; aucune autorisation de push utilisée.

Le runtime ne propose pas de fermeture des sous-agents ; leurs missions sont terminées et ils n’ont pas été réaffectés hors de leur rôle. Aucun autre artefact ou mécanisme d’outillage n’a été créé.

## Revue du candidat Rust après implémentation

Cette section complète les verdicts documentaires ci-dessus, sans les remplacer par un verdict de code CLEAN. Autorité actuelle et historique de garde/capture : voir le suivi du plan, notamment la demande utilisateur « continue la refacto sans te soucier de ce probleme » et la poursuite du code en séparant les vérifications NOT RUN.

Coordination : `rust_review_coordinator`, agent review_refactor_direct_live, sans édition ni revue du code par le coordinateur. Reviewer : agent `rust_reviewer_direct_live`, rôle rust_reviewer, indépendant de l’implémenteur et en lecture seule. Auteur du code : implementation_direct_live. Aucun code modifié ni commande Cargo exécutée pendant la revue ; aucune correction demandée. Candidat : huit chemins Rust de L-DL dans l’arbre de travail, base `102a3f8d44cfdb52355d70abc0f263f9ff5af86f`, epoch 0. Le fichier de plan modifié par la session principale et les non-suivis antérieurs sont distingués du delta Rust.

### Retour final brut du coordinateur

Avancée : revue statique indépendante terminée ; aucun défaut actionnable n’a été relevé et aucune correction n’est demandée.

Blocage : la compilation, les tests et les autres vérifications Cargo du plan restent non exécutés. Le reviewer ne propose aucune voie de vérification autorisée.

Décision : `BLOCKED` pour clôture de la revue, conformément à son verdict. Aucun cycle de correction n’a été engagé.

```yaml
verdict: BLOCKED
target:
  baseline: 102a3f8d44cfdb52355d70abc0f263f9ff5af86f
  candidate: working-tree, epoch 0
passes:
  review: 1
  correction: 0
  verification: 0
findings:
  closed: []
  unresolved: []
files_changed_by_worker:
  - crates/whisper-adapters/src/direct_live.rs
  - crates/whisper-adapters/src/direct_live/capture.rs
  - crates/whisper-adapters/src/direct_live/worker.rs
  - crates/whisper-adapters/src/direct_live/outputs.rs
  - crates/whisper-adapters/src/direct_live/wav.rs
  - crates/whisper-adapters/src/direct_live/tests.rs
  - crates/whisper-adapters/src/direct_live/d2_corpus.rs
  - crates/whisper-adapters/src/direct_live/test_support.rs
verification_summary:
  - "Revue statique : aucun défaut actionnable relevé."
  - "rustfmt ciblé et git diff --check : PASS, selon le handoff initial."
  - "Cargo fmt, Clippy, compilation et tests : NOT RUN."
  - "Validation native et fenêtre Windows : NOT RUN."
residual_risks:
  - "La lecture statique ne démontre pas la compilation ni le comportement des tests."
blocker: "La garde stricte reste bloquée ; aucune voie de vérification exécutable n’est autorisée dans ce mandat."
```

### Précisions attribuées transmises par le coordinateur

Le reviewer a examiné l’état DirectLive et les constantes, les corps de production/support déplacés, les différences de visibilité/formatage, l’API crate dont pump_capture et les consommateurs. Il rapporte les 18 tests initiaux contrôlés équivalents après normalisation, les sept cas D2 conservés et les trois témoins supplémentaires examinés. Le principal passe de 3 184 à 470 lignes et les modules privés correspondent au découpage prévu.

Gaps matériels : découverte compilée des 28 tests, tests ciblés et consommateurs, compilation des tests et des consommateurs adapters/desktop, Cargo fmt et Clippy du plan. Moteurs natifs et fenêtre Windows non validés. Le reviewer ne propose pas de voie VERIFY autorisée et le coordinateur n’a déclenché ni nouvel implémenteur, ni contournement de la garde, ni nouvelle tentative. Ce résultat autorise la remise du code avec ces limites ; il n’est pas une clôture de revue CLEAN.

### Vérification supplémentaire demandée après remise

L’utilisateur a ensuite demandé les validations nécessaires. L’implémenteur initial, en mode VERIFY et sans mutation du candidat, a tenté le préflight strict avant les contrôles C1–C9. Résultat : auto-test FAIL, code 1, à 13:15:21.7861363 +01:00 le 31 octobre 2026 sur un événement réel Temp extérieur d’origine non attribuée. Journal : `target/parakeet-tests/refacto-direct-live/logs/selftest-validation-20261031.log` ; détail et autorité au §10 du plan. Aucun Cargo ni test produit lancé, aucune modification source. Les gaps matériels précédents ne sont donc pas fermés et aucun nouveau passage de revue n’a été lancé sans preuve nouvelle exploitable.

## Clôture Rust après révision permanente Temp et validations

L’utilisateur a ensuite explicitement choisi la modification permanente de la règle Temp dans AGENTS.md. Portée et risque résiduel acceptés au §11 du plan : journalisation des événements non attribués dans le seul Temp réel, sans autorisation d’écriture volontaire ; attribution démontrée et autres violations toujours bloquantes. L’adaptation de la garde et ses auto-tests ont réussi, puis toutes les vérifications C1–C9 ont terminé avec code 0 sur le même candidat Rust. Preuves détaillées au §12 du plan et dans `target/parakeet-tests/refacto-direct-live/archive/validation-20261031.zip`. Aucun correctif source n’est intervenu entre les deux passages de revue.

Le même reviewer rust_reviewer_direct_live a examiné ces nouvelles preuves en lecture seule. Il n’a pas exécuté lui-même les commandes Cargo : les résultats sont ceux du VERIFY de l’implémenteur initial, contrôlés dans la revue. Il confirme les 28 tests découverts, les 30 tests ciblés distincts réussis (C5 déjà couvert par C2), la compilation workspace --no-run, le format global, Clippy code 0 avec avertissements et les builds adapters/desktop. Les anciens gaps de vérification ne restent pas ouverts sur ce candidat.

### Retour final brut du coordinateur

Avancée : le même reviewer a repris la revue après réception des vérifications du candidat inchangé et a déclaré `CLEAN`.

Blocage : aucun pour la revue du refactoring. Les essais natifs, microphone/GPU réel et fenêtre Windows restent non validés.

Décision : aucun finding ni cycle de correction. Les deux réexports inutilisés sont un avis sans correction obligatoire.

```yaml
verdict: CLEAN
baseline: 102a3f8d44cfdb52355d70abc0f263f9ff5af86f
candidate: working-tree, epoch 0, inchangé
passes:
  review: 2
  correction: 0
  verification: 1
findings:
  closed: []
  unresolved: []
verification_gaps: []
residual_risks:
  - "Validation native, microphone/GPU réel et fenêtre Windows non effectuée."
  - "92 événements Temp UNATTRIBUTED journalisés ; leur origine demeure inconnue."
```

Avis du reviewer : les réexports ParakeetExportOutcome et export_parakeet_manifest sont conservés pour DEC-T02 ; les avertissements sont expliqués, sans correction obligatoire. Les 92 événements UNATTRIBUTED ont été journalisés conformément à la règle permanente explicitement acceptée ; ils ne deviennent pas une preuve d’absence de fuite ni une attribution à Cargo. Ce verdict clôt la revue du refactoring, sans valider le matériel ou la fenêtre Windows et sans autoriser un push.

Livraison Git : après ce verdict, l’utilisateur a explicitement autorisé le commit local des seuls fichiers de la tâche, sans push. La session principale inclut les huit fichiers Rust, AGENTS.md et les deux suivis documentaires ; aucune modification source supplémentaire ni fichier étranger n’est intégré. Le candidat Rust validé et relu reste inchangé.
