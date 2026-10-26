Avancée : revue indépendante du plan terminée, sans correction requise.
Blocage : aucun constat REQUIRED ; prérequis d'exécution non satisfaits.
Décision : CLEAN — PLANS READY pour acceptation utilisateur, sans autorisation de coder.

# Revue du plan d'implémentation Parakeet unifié

## Identité et indépendance

- Objet : `docs/cadrage/plan-parakeet-transcription-unifiee.md`, P-PARA-UNIFIED-01 v0.1.
- Candidat examiné : Git `325471621ee182511cb3ff522294d73975967e4d`.
- Design parent : D-PARA-UNIFIED-01 v0.2, accepté pour planification au Git `d8d170f9`. Le reviewer a vérifié que S-USER-07 ne modifie pas les contrats techniques auparavant revus CLEAN.
- Reviewer : `/root/parakeet_design_review`, rôle `rust_design_reviewer`, même reviewer du design, sans participation à la rédaction ni aux corrections du plan.
- Auteur de ce compte rendu : session principale, consignation du résultat reçu du reviewer. Aucun verdict attribué au rédacteur du plan ou à l'auteur documentaire.
- Date : 26 octobre 2026. Lecture complète du plan, comparaison avec le design et lectures ciblées des modules, interfaces de processus et profils.
- Fichier et empreinte transitoire inchangés avant/après ; contrôle Git des espaces réussi. Aucun fichier modifié ni test, garde, Cargo ou binaire exécuté par le reviewer. Aucun manifeste d'empreintes créé.

## VERDICT : CLEAN

**REQUIRED : aucun.** Aucun cycle de correction nécessaire pour ce livrable PLANS. La correction du design précédent est un historique distinct ; aucun troisième cycle n'est ouvert.

### ADVISORY — PLAN-PU-A1

**Low / MINEUR — Expliciter le cycle de vie qui borne les tentatives du worker.**

T5 exige correctement des tables de tentatives bornées. La table enfant actuelle est toutefois un `BTreeMap` cumulatif créé dans `crates/whisper-worker-parakeet-cpu/src/main.rs:95`, alimenté à la ligne 253 et sans éviction observée.

Cela n'impose pas d'ajouter ce fichier au périmètre : les chemins G4 autorisés peuvent maîtriser la durée de vie du processus grâce aux interfaces existantes de `shutdown` dans `crates/whisper-adapters/src/parakeet_import.rs:812` et de destruction. Le rédacteur peut préciser avant T5 la stratégie retenue et sa vérification sur plusieurs sessions successives. Borner seulement une table du parent ne bornerait pas, à lui seul, celle du processus enfant. Toute autre solution nécessitant un chemin supplémentaire reste soumise au PLAN_CHANGE prévu.

Ce conseil n'empêche pas CLEAN : le contrat à satisfaire et les adaptateurs permettant de le réaliser figurent dans le périmètre. Owner de la précision : rédacteur/architecte dans le contrat accepté, avant T5 ; aucun changement technique ajouté à la clôture du plan.

## Résultat des contrôles

| Point contrôlé | Conclusion du reviewer |
| --- | --- |
| Taille et séquencement | PASS : dix tâches au total, quatre lots, progression séquentielle et dépendances DOCUMENT/CODE/EXECUTION cohérentes. |
| Premier parcours complet | PASS : T1–T4 conduisent au vrai parcours applicatif d'import avec worker contrôlé, avant capture et interface. |
| Couverture du design | PASS : REQ/UC/AC-01 à 10 trouvent tâches et scénarios ; contrôle inverse des chemins réalisé ; aucun élargissement produit. |
| Ressources et longues sessions | PASS : source segmentée, décodage continu, index/document paginés, publication streaming, rotation réelle >1 Gio ; aucune preuve d'infini annoncée. |
| Équité et concurrence | PASS : T5 reprend P=1, alternance A/B, préparation incluse et V-PU-EQUITY-01 avec témoin négatif ; T7 couvre fermeture et périmés. |
| Chemins et contrôles | PASS : modules nouveaux et lib.rs, chemins partagés, tests, composition, protections de source/publication identifiés. |
| Commandes et profils | PASS : offline/locked proposé ; C5 sans personal-install correspond au profil Test. Ressources effectives et lanceur à vérifier avant remise ; aucun paquet autonome présumé. |
| Qualification native | PASS documentaire : qualification single-pass non assimilée à la nouvelle stratégie ; activation finale dépendant de T9 ; C4 distingue les cas réellement exécutés. |
| Garde et permissions | PASS documentaire : E-GUARD non satisfait, aucun Cargo avant enveloppe conforme, chemins canoniques, auto-tests, surveillance et accord précis. Aucune ancienne dérogation transférée. |
| Validation et remise | PASS : Q-PU-03/Q-RQ-01 ont leurs échéances ; revue Rust indépendante, corrections bornées et recette Windows utilisateur distinctes. |

## Sources effectivement consultées

Plan complet ; design et sa revue ; diff d'acceptation `03506050..d8d170f9` ; AGENTS ; skill/règles et contrats centraux ; `whisper-desktop/Cargo.toml`, worker Parakeet `Cargo.toml` et `main.rs` ; `parakeet_ipc.rs` ; composition `root.rs` ; gestion du processus dans `parakeet_import.rs` ; cycle de vie du direct dans `parakeet_live.rs` ; inventaire des tests existants cités par le plan.

## Portée de la clôture

Toutes les vérifications produit et de livraison restent **NOT RUN**. PLANS READY qualifie le document, pas E-GUARD, les entrées natives ou une campagne. L'accord précis de réalisation d'un lot reste requis.

Le reviewer permet à la session principale de consigner ce verdict et de finaliser uniquement les métadonnées de readiness et ce compte rendu, sans changement technique supplémentaire. Le plan reste soumis à acceptation utilisateur ; aucun code, campagne ou push n'est autorisé par ce verdict.
