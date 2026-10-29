Avancée : brief et exigences établis ; domaine et architecture en préparation.
Blocage : aucun pour le cadrage ; exécution Cargo et tests non autorisée.
Décision : préparer le refactor à logique constante, sans migration des contrats historiques.

# Cadrage — séparation des tests et diagnostics

ID : D-REF-TESTS-DIAG. Version : 0.1, 29 octobre 2026. Statut : DRAFT.
Base produit : `f82063ae63b2cc075eaff54693eb62ea36a18c65`, branche `evo-startup-perf`.
Auteur documentaire : session Codex principale. Contributions proposées attribuées ci-dessous.

## 1. Sources, autorité et périmètre

S1 — demande originale de l'utilisateur dans cette session :

> ## déplacements mécaniques (risque quasi nul, gain immédiat)
>
> **1a. Sortir les tests de `worker_ipc.rs`** vers des fichiers séparés (`worker_ipc/tests/*.rs`).
>
> - Le fichier passe d'environ 29 000 à environ 12 500 lignes **sans toucher une ligne de logique**.
> - Toutes les étapes suivantes deviennent relisibles.
> - Faites de même pour `application.rs` (3 035 lignes de tests), `ui.rs` (2 410) et `archive.rs` (2 169).
>
> **1b. Renommer les identifiants de lots** (`record_r7_timing`, `l2a2_*`, `l04_*`...). C'est mécanique, et ça casse au passage le lien de dépendance entre `archive.rs` et `worker_ipc.rs` en déplaçant `record_r7_timing` dans un module `diagnostics`.

S2 — mission actuelle de l'utilisateur : « fais le cadrage puis le plan de refacto avec les agents dediés ».
S3 — `AGENTS.md` racine et des crates concernées, sources et manifestes actuels : contraintes applicables.
S4 — qualification dans cette session : revue indépendante PASS, pas autorisation d'implémenter.
S5 — `docs/architecture/` : relevés statiques non suivis, à confronter aux sources ; non intégrés à ce commit.

S2 autorise cadrage et planification. L'auteur principal écrit et commite localement les documents selon S3.
Aucun code, Cargo (même metadata), build, test, installation, push, élévation ou campagne n'est autorisé par cette mission.
Les choix techniques conformes sont délégués à l'architecte ; aucune acceptation de migration des contrats historiques n'est déduite de S2.
Les dossiers non suivis `docs/architecture/`, `models/` et `sound example/` sont préservés.

## 2. Brief produit

Contribution : `rust_product_framer`, `/root/cadrage_refacto/brief`, BRIEF-REF-TESTS-DIAG-v1 ; base S1–S4 et commit produit ci-dessus.

Objectif : rendre quatre unités Rust plus faciles à lire et faire évoluer par séparation des tests, clarification des noms internes et isolement du diagnostic commun exposé par worker_ipc.
Bénéficiaires : mainteneurs/relecteurs et utilisateurs Windows protégés contre régression.
Version : tranche « tests et diagnostics », aucune release déduite. Résultat utilisateur Windows : transcription, affichage, sauvegarde, erreurs et diagnostics identiques.

| Capacité | Résultat / sort | Source |
| --- | --- | --- |
| CAP-01 | Tests worker_ipc vers worker_ipc/tests/*.rs, chaque test/auxiliaire/cfg conservé | S1 |
| CAP-02 | Séparation tests application.rs, destinations locales à concevoir | S1 |
| CAP-03 | Séparation tests ui.rs, mêmes scénarios/observations | S1 |
| CAP-04 | Séparation tests archive.rs, sélections/dispositifs préservés | S1 |
| CAP-05 | Noms internes explicites ; liste finie, pas renommage global par motif | S1, S3 |
| CAP-06 | Diagnostic commun indépendant, suppression archive vers worker_ipc ; contexte/garde/effets préservés | S1 |
| CAP-07 | Démontrer conservation/non-régression lors réalisation future ; pas exécuté au cadrage | S1, S3 |
| CAP-08 | Migration env/protocoles/formats/dossiers/interfaces campagnes exclue faute accord ; reprise par mission distincte | S3 |
| CAP-09 | Refonte autres responsabilités production/changements comportement hors mission | S1, S2 |
| CAP-10 | Aucun nouveau chantier installation/démarrage OS/réglages/tray/archives ; contrats actuels et différés inchangés | S1–S3 |

Parcours : lecture/localisation tests ; découverte/sélection sans disparition ni ignoré compté succès ; diagnostic depuis archive+IPC identique ; usage Windows inchangé sans nouvel écran/état/annulation/reprise.
« Risque quasi nul », « gain immédiat », ~12 500 lignes et volumes initiaux sont appréciations/estimations, aucun seuil accepté. Pas de nouvelle API publique, dépendance ou décision produit nécessaire.

## 3. Observations actuelles

| Source | Lignes | Début principal module tests |
| --- | ---: | ---: |
| crates/whisper-adapters/src/worker_ipc.rs | 29 509 | 13 513 |
| crates/whisper-core/src/application.rs | 7 550 | 4 516 |
| crates/whisper-desktop/src/ui.rs | 6 195 | 3 870 |
| crates/whisper-adapters/src/archive.rs | 10 092 | 7 968 |

Des auxiliaires cfg(test) dispersés nécessitent un inventaire distinct. Ces lignes ne sont pas une mesure des seules lignes de production.
`archive.rs` contient neuf appels à `crate::worker_ipc::record_r7_timing` ; worker_ipc dépend déjà d'archive.
Le diagnostic possède aussi R7TimingContext, un stockage local au thread et R7TimingContextGuard (`worker_ipc.rs:175–205`).
Aucun module diagnostics dans adapters/lib.rs ; ceux des workers CPU/GPU sont dans d'autres crates.
Les contrats historiques incluent `PARAKEET_R7_*`, `PARAKEET_L2_*`, `WHISPER_L04_*`, `.l2a2-acceptance`, `whisper-l04-fault/1` et les sélections `--exact`.
`archive.rs:9610–9670` comporte notamment des sous-processus et écritures vers `C:\WhisperLive\L04-campaign\runs` : conserver ces tests n'autorise pas leur exécution.
Les gardes repérées sous `target/parakeet-tests/micro-level/` et `retraitement-cible/rtc-05/` sont historiques, pas des lanceurs génériques de ce refactor.

## 4. Exigences, scénarios et acceptation

Contribution : `rust_requirements_analyst`, `/root/cadrage_refacto/requirements`, REQ-REF-TESTS-DIAG-v1 ; base S1–S4 et commit produit ci-dessus.
Chaque ligne établit CAP-n ↔ REQ-n ↔ UC-n ↔ AC-n ; REQ-07 est transverse. Tous les tests produit : NOT RUN.

| IDs | Exigence et scénario d'acceptation observable | Contrôle futur |
| --- | --- | --- |
| CAP-01 / REQ-01 / UC-01 / AC-01 | Inventaire IPC puis extraction : chaque test/auxiliaire exactement une fois après correspondance de noms, aucune assertion/cfg perdu ; fragments attachés aux types restants recensés/justifiés | Diff et inventaires puis harness si autorisé |
| CAP-02 / REQ-02 / UC-02 / AC-02 | Application : mêmes entrées, commandes, événements, assertions/cfg et transitions après déplacement | Diff, découverte/exécution futures |
| CAP-03 / REQ-03 / UC-03 / AC-03 | UI : mêmes scénarios/observations, accès privés conservés ; aucune API publique ni branche produit pour tests | Diff, consommateurs, tests futurs ; pas preuve rendu Windows |
| CAP-04 / REQ-04 / UC-04 / AC-04 | Archive : scénarios durables/attributs/sélections conservés ; chaque --exact sélectionne le test attendu, sans zéro-test silencieux ni parent réinvoqué | Revue et exécution autorisée uniquement |
| CAP-05 / REQ-05 / UC-05 / AC-05 | Liste finie ancien/nouveau : tous consommateurs traités ; env/formats/dossiers/protocoles/messages/interfaces historiques identiques, reliquats motivés | Recherche et diff, glob insuffisant |
| CAP-06 / REQ-06 / UC-06 / AC-06 | Timing activé admissible, désactivé, config absente/invalide, contextes emboîtés et fin portée : émission/refus/champs/messages/restauration identiques ; diagnostics indépendant d'IPC, aucune destination nouvelle | Comparaison fonction/TLS/garde/appels, vérification ciblée future |
| CAP-07 / REQ-07 / UC-07 / AC-07 | Comparaison bases/configs/listes : cardinalités identiques après table noms ; aucun perdu/dupliqué/nouvel ignored/logique changée ; manifests/lock/contrats/fonctions préservés | Sources, listes et résultats réels avec statut par scénario |

Raccordements autorisables au futur plan : modules, imports, chemins et visibilité minimale nécessaires ; pas de nouvelle API publique pour tests.
Mesures : cardinalité identique par configuration ; zéro reliquat test inexpliqué ; zéro appel timing archive via IPC et zéro dépendance diagnostics vers IPC ; zéro évolution hors raccordements/liste noms.
Une comparaison statique n'est ni une découverte conditionnelle du harness ni une exécution.

### Cas dégradés et états

Aucun état produit nouveau. Test introuvable/dupliqué/inaccessible ou --exact non résolu : arrêt de tranche.
Élargissement d'API publique nécessaire : arbitrage. Échec de vérification préalable/après : aucune suite dépendante.
Diagnostic désactivé/config invalide : conserver non-écriture/messages ; fin de portée : conserver restauration TLS sans nouvelle propagation interthread.
Interruption édition/validation : conserver diff et résultat partiel, pas reset. Tests hors racines autorisées : NOT RUN jusqu'au mandat, sans changement de logique pour contourner.

### Risques fonctionnels

| ID | Risque et contrôle |
| --- | --- |
| RISK-F01 | Perte/sélection vide test → AC-01 à AC-04, AC-07 |
| RISK-F02 | Élargissement privé altérant API/données/synchronisation → AC-01 à AC-03, AC-07 |
| RISK-F03 | Perte contexte/refus/champs timing → AC-06 |
| RISK-F04 | Renommage modifiant contrat de campagne → AC-05 |
| RISK-F05 | Test écrivant hors racines, notamment C:\WhisperLive\L04-campaign\runs → AC-04/07, prérequis EXECUTION |
| RISK-F06 | Lignes retirées présentées comme performance/non-régression prouvée → séparation des preuves |

## 5. Domaine et architecture

À compléter par rust_domain_architect puis rust_architect après versionnement de cette base. Cette rubrique ouverte interdit de déclarer DESIGN READY.

## 6. Applicabilité et readiness

Aucune nouvelle migration/persistance ; effets existants à préserver. Concurrence inchangée, dont TLS/gardes/bornes.
Windows, pile native et routage inchangés ; aucun nouveau réseau, donnée ou destination. Aucun EXE livré ni UI validée.
Pas de SPIKE demandé à ce stade. Pas de nouveau railguard ; AGENTS existants demeurent applicables.
Terminé : brief/exigences. Restant : domaine, architecture, nomenclature/layout, traçabilité complète et revue indépendante Git.

## 7. Validation et remise

Lectures et recherches uniquement ; git diff --check exécuté. Cargo, build, tests, natif, fenêtre Windows : NOT RUN.
Le futur plan doit couvrir AC-01 à AC-07 en dix tâches maximum et justifier chaque chemin.
Les autorisations d'implémentation, de campagne et de publication restent distinctes de la readiness documentaire.
