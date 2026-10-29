Avancée : revue DESIGN terminée sur le corpus Git vérifié.
Blocage : aucun constat REQUIRED ouvert.
Décision : CLEAN — DESIGN READY pour la planification, sans autorisation d'implémentation ou de campagne.

# Revue indépendante du cadrage tests et diagnostics

Date : 29 octobre 2026. Reviewer : rust_design_reviewer, /root/revue_design_refacto.
Résultat brut utile remis par le reviewer et versionné par la session principale.
Objet revu : D-REF-TESTS-DIAG v1.0, docs/cadrage/refacto-tests-diagnostics.md,
commit `197d821ef98ea8c239d62f11cd40bfe9fdb1c74e`.
Base produit : `f82063ae63b2cc075eaff54693eb62ea36a18c65`.
Objet Git du fichier vérifié avant/après : `a0943cea88e229db0286c2706150fbeed412f80f`, identique au fichier courant au moment de la revue.
Seul le cadrage différait de la base ; aucune modification suivie locale.

## Indépendance et méthode

Le reviewer n'a participé ni à la rédaction ni à la correction du cadrage. Revue en lecture seule, sans sous-agent, écriture, Cargo ou test.
Sources consultées : skill rust-predev-design, cinq règles métier, handoff-contract.md, deliverable-quality.md ; AGENTS racine et crates adapters/core/desktop ; corpus Git, manifestes workspace/adapters, adapters/src/lib.rs ; sources cibles et consommateurs model_catalog.rs, parakeet_import.rs, staging.rs, supervisor.rs ; ancres/supports/diagnostic/TLS/garde/noms proposés.

## Couverture vérifiée dans les deux sens

| Périmètre | Résultat |
| --- | --- |
| CAP-01 à07 | Reliées aux REQ/UC/AC-01 à07, invariants, architecture, risques et acceptation |
| CAP-08 à10 | Exclusions rattachées à mission mécanique et AGENTS ; aucun différé produit ajouté |
| REQ-01 à04 | Layout fini, portées privées conservées ; ancres présentes et ordonnées |
| REQ-05 | Liste finie, variantes cfg et consommateurs indirects couverts ; littéraux conservés |
| REQ-06 | Extraction atomique contexte/TLS/garde/fonction/horloge ; neuf appels archive et six consommateurs externes vérifiés |
| REQ-07 | Statique, découverte harness et exécution distinguées ; arrêts et NOT RUN explicites |

235 attributs #[test] lexicaux et caractéristiques du diagnostic cohérents avec les sources.
include! conserve la portée sans API publique nouvelle ; déplacement fichier/ligne des panic explicité.
Exceptions architecturales historiques ni aggravées ni présentées comme corrigées.
Persistence/concurrency : invariants de conservation applicables ; aucun nouveau parcours/migration/packaging/railguard requis. Aucun SPIKE invoqué comme preuve.

## Verdict

Findings REQUIRED : aucun. Suggestions ADVISORY : aucune nécessaire.
**CLEAN — DESIGN READY pour la planification.** Revue initiale, aucune correction requise.
Le verdict ne démontre ni compilation des futurs fragments, ni non-régression exécutée, ni conformité d'une future campagne.
Le plan doit préserver le préalable mandat/garde compatible et les limites des tests écrivant dans des racines historiques.
