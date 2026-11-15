Avancée : le plan séquence la distribution x64 autour des décisions produit acceptées.
Blocage : les bancs v2/ARM64, la compatibilité Turbo au plancher et la faisabilité du refus de l’installeur restent à prouver.
Décision : ce plan ordonne des gates et des lots ; il n’autorise ni code, ni campagne, ni activation CI.

# Plan d’implémentation — Distribution Whisper Windows x64

**État : PLAN DRAFT, en attente de seconde revue indépendante.** Ce plan décline le cadrage livré avec réserves [evo-distribution-cpu-x64.md](evo-distribution-cpu-x64.md), commité en `7d7dd487` sur `codex/cpu-distribution-design-plan`. Il consigne les décisions utilisateur du 15 novembre 2026 : Windows 11 Home/Pro encore supporté au moment de publier ; édition `large-v3-turbo` avec CPU, NVIDIA et Auto, sans Parakeet ; installation neuve seulement. Le design n’a pas de verdict `CLEAN` après ses deux cycles ; le plan reste conditionnel et n’est pas une entrée en implémentation.

Le paquet vise x86-64 avec plancher applicatif x86-64-v2. Les instructions AVX2+ restent facultatives et conditionnées à une détection dans le processus qui les utilise ; aucune variante optimisée supplémentaire n’est requise pour la première édition. Windows on Arm est une cible de qualification CI x64 sous émulation, pas une cible de support public. macOS et ARM64 natif sont exclus.

## Séquence et gates

| Lot | Tâches | Dépendance et sortie |
|---|---:|---|
| L0 — Faisabilité préalable | 5 | Décision d’admission de la garde, mandat distinct et bancs disponibles → gates v2, refus, chemin Turbo et CI caractérisés. |
| L1 — Profil CPU portable | 3 | L0 PASS + profil de campagne accepté et lot code autorisé → desktop et worker Turbo x64 exécutent une transcription sous v2. |
| L2 — Édition et ressources | 4 | L1 PASS + validation GPU autorisée → profil Turbo complet, dépendances et routes fermées. |
| L3 — Installation neuve | 4 | L1/L2 PASS + effets/format acceptés → paquet installé, refus vérifié et prérequis publiés. |
| L4 — Qualification et remise | 4 | L3 PASS + mandat d’exécution CI/validation → même paquet éprouvé sur les deux bancs et essais produit attribués. |

Dépendances : `L0 → L1 → L2 → L3 → L4`. Un échec d’un candidat final crée un nouveau candidat et relance les preuves touchées ; il ne transforme pas rétroactivement un résultat négatif en succès. Chaque lot contient au plus quatre tâches. Les gates techniques ouverts demeurent bloquants avant les tâches qui en dépendent.

**Préflight commun à répéter avant chaque lot autorisé :** relever branche, HEAD, état Git et espace libre ; lire les `AGENTS.md` des crates touchées, vérifier le plan et ses chemins autorisés, le mandat écrit précis, la concurrence pertinente et l’état des caches. Tout worktree, racine de campagne, cible Cargo, temporaire, preuve et nettoyage doit satisfaire la règle permanente de `AGENTS.md`. N1 passe par `ci/guard-level1.ps1`, N2 par la voie GPU admise. Pas de Cargo direct, de réseau, de cible arbitraire, de campagne native suspendue ni d’écriture hors racines autorisées. Clippy et les sélections Cargo sont fixés au préflight à partir de la matrice réellement admise par la garde ; une option refusée n’est pas contournée. Le format est vérifié par la voie autorisée. En échec : arrêter la campagne, conserver l’état et les preuves, diagnostiquer dans la limite de trois essais ou 30 minutes.

**Blocage de la garde au HEAD de référence :** aucune racine/profil de campagne « distribution CPU » n’est enregistrée. La garde refuse les sélections `-p`/`--workspace` sur une telle racine avant Cargo ; `-DryRun` est également interdit tant que l’admission n’existe pas. Avant toute commande Cargo, y compris DryRun, l’utilisateur doit autoriser un lot distinct qui ajoute et versionne le profil : racine, membres, catégories/arguments Cargo, cible, contrôles PE et auto-tests. Ce changement exige une revue. N1 doit y inclure `--no-fail-fast` avec `-WithoutCargoTargetDir`, les sélections autorisées, compilation de harnais, contrôle PE exact avant inventaire/exécution et Clippy avec le cliquet. La compilation GPU N2 doit nommer sa voie/profil acceptés. Sans cette décision et la garde versionnée, T03 et les tâches Cargo/N1/N2 restent bloquées ; aucun contournement n’est permis.

Les commandes ci-dessous sont des catégories de validation à inclure dans le mandat propre au lot, et non des autorisations. Toute commande précise et toute racine doivent être acceptées par la garde existante. Les campagnes N1 qui touchent du Rust incluent le `cargo test --no-fail-fast` dédié, les références N1 des membres par défaut et workers, puis Clippy applicable ; campagne GPU = niveau 2. Le profil Turbo portable est vérifié séparément des réglages Rust/CMake du profil personnel. La vérification de codegen ne repose pas sur Clippy, qui retire `RUSTFLAGS`.

## L0 — Faisabilité préalable, mandat distinct

L0 ne modifie pas le produit et ne lance aucune campagne sans mandat précis. L0 ne construit pas les DLL natives NeMo/llama suspendues. Avant toute compilation Turbo, le mandat doit nommer l’outil, la cible, les racines, le témoin, les entrées modèle/audio et les commandes passées par les contrôles autorisés.

| ID | Périmètre, preuve et sortie | Gate de sortie |
|---|---|---|
| **T00 — Admission de la garde (BLOQUANT)** | La session principale demande un mandat séparé pour un changement limité de `ci/guard-level1.ps1`, avec profil/root versionné pour ce lot et auto-tests. Nommer les membres (core, adapters, desktop, worker CPU, bootstrap selon lots), commandes autorisées, racines/cibles, `--no-fail-fast`/`-WithoutCargoTargetDir`, compilation de harnais, contrôle PE par chemin avant inventaire/exécution et Clippy avec cliquet. Décrire séparément la voie N2 de qualification GPU. Aucun fichier de garde n’est modifié par le présent plan. | Accord écrit, profil versionné et revu, auto-tests PASS avant toute commande Cargo, même `-DryRun`. Sinon tâches Cargo et N1/N2 `BLOCKED`. |
| **T01 — Banc CPU et refus minimal** | Responsable de faisabilité désigné avant le mandat ; diagnostic plafonné à 30 minutes ou trois essais. Vérifier un Windows capable de démarrer un PE x64 socle qui expose des capacités sous v2, puis un témoin complet v2 sans AVX, AVX2, FMA, F16C, BMI1/2 ni extension supérieure. Relever capacités dans le processus. Sur le premier témoin, vérifier un contrôle de refus minimal avant mutation. | Banc et masque observés, ou `NOT RUN` avec environnements/méthodes tentés. Le vrai installeur reste à éprouver en T32. |
| **T02 — Banc Windows on Arm** | Responsable CI désigné ; même budget de diagnostic. Identifier un hôte Windows ARM64 et vérifier qu’un PE x64 s’exécute émulé ; relever hôte, image, architecture du processus et méthode reproductible. | Banc accessible/reproductible ou `NOT RUN` motivé. N’active aucun runner ni workflow. |
| **T03 — Turbo au plancher** | Lecture seule initiale des sources et CMake épinglés pour établir les options candidates. Toute production de CMakeCache/objets/PE ou inférence exige T00 PASS et un accord précis couvrant la compilation potentielle de `whisper-rs-sys`/GGML statique sous Cargo, ses commandes, racines, cible, modèle/audio et témoin. Elle n’est ni le build des DLL NeMo/llama, ni autorisée par la décision produit de distribution. Après accord, examiner flags/artefacts liés/imports et inférer via IPC sur v2 sans AVX. | Même candidat démarre et infère sous v2, sans ISA supérieure requise inconditionnellement. Sans accord exprès, seule la lecture est permise et la faisabilité reste ouverte. |
| **T04 — Faisabilité de la preuve de bout en bout** | Inventorier les environnements et données déjà accessibles pour le paquet, l’installation, les deux jobs CI et la recette manuelle Intel/AMD/NVIDIA/micro ; vérifier la possibilité de relever identité de paquet, capacités et résultats. Aucune ressource ni service n’est créé. | Entrées, propriétaires et limites de preuve recensés. Besoin d’infrastructure externe revient à l’utilisateur avant L4. |

**Gate L0→L1 :** T00 PASS ; T01 refuse et admet avec les témoins pertinents ; T03 prouve le chemin Turbo au plancher ; T02/T04 établissent la faisabilité des deux tests CI. À défaut, tâches dépendantes bloquées et décision précise à demander. Un résultat de faisabilité ne vaut pas recette du candidat final.

## L1 — Profil release CPU portable

Chemins candidats (à reconfirmer avant mandat) : `.cargo/config.toml` ou configuration isolée de distribution, `ci/` pour le profil et sa recette, `crates/whisper-core/` pour la politique, `crates/whisper-adapters/` pour la lecture des capacités, `crates/whisper-worker-cpu/`, `crates/whisper-desktop/` et tests dédiés. Ne modifier aucun manifeste ou `Cargo.lock` sauf si le lot accepté l’identifie explicitement.

| ID | Changement borné | Critères et vérifications |
|---|---|---|
| **T11 — Plafond ISA** | Isoler le profil public x64 du profil personnel ; fixer explicitement le plafond des compilations Rust et C/C++/CMake du chemin CPU. Ne pas ajouter de variante AVX2 dans cette édition. | AC-02 : commandes/flags réels et PE des exécutables/bibliothèques examinés ; chemin inconditionnel exécutable sur T01 v2 sans AVX. Une commande refusée par garde bloque, sans contournement. |
| **T12 — Politique et probe** | Implémenter la règle pure v2 dans `crates/whisper-core/src/cpu_compatibility.rs` et le relevé de capacités dans `crates/whisper-adapters/src/cpu_capabilities.rs`, avec leurs modules et fichiers de tests séparés. Le résultat indéterminé ne permet pas de démarrer un chemin optionnel ; aucun changement de moteur métier. | Tests unitaires pour chaque bit requis/manquant et observation du desktop et worker ; imports compatibles, aucune `unsafe` hors contrat, aucune nouvelle dépendance non autorisée. Le bootstrap réutilise cette politique en T31 sans la redéfinir. AC-01/02. |
| **T13 — Flux Turbo CPU** | Composer le desktop release, son worker CPU et le modèle d’édition par des chemins explicites ; supprimer toute dépendance à l’arbre de build pour ce profil. | AC-02/05 : une courte inférence réelle sur le témoin T01 par l’IPC produit TXT/SRT lisibles, garde la source intacte et constate le moteur effectif. N1/Clippy applicables réussis via la garde. |

**Gate L1→L2 :** même candidat de développement démarre et transcrit sous v2 sans AVX ; imports CPU exempts de CUDA ; sorties observées. Le profil de campagne T00 est PASS. Revoir les options communes avant de qualifier GPU/Auto.

## L2 — Edition et fermeture des ressources

Chemins candidats : composition desktop, workers CPU/GPU, tests, recette de paquet et documentation minimale. Les profils NVIDIA/Auto sont partagés avec l’application actuelle ; relire les contrats de leurs crates et le railguard avant toute modification. Aucune ressource Parakeet ne fait partie de l’édition.

| ID | Changement borné | Critères et vérifications |
|---|---|---|
| **T21 — Edition Turbo** | Décrire identité de l’édition, workers, modèle `large-v3-turbo`, chemins et modes CPU/NVIDIA/Auto ; composer les modes réellement remis. | AC-04 : chaque ressource a un consommateur, une provenance, une version et une résolution vérifiables. Parakeet n’est ni affiché comme disponible ni routé depuis cette édition. |
| **T22 — Fermeture NVIDIA et Auto** | Comparer profil distribué aux chemins NVIDIA/Auto partagés et corriger uniquement les effets du delta autorisé. Utiliser le profil N2 accepté pour le candidat distribué. | AC-09/10 applicables : profils manuel CPU/GPU, GPU disponible, échec d’initialisation avant traitement avec une seule tentative CPU Auto et absence de bascule après début ; moteur/cause observables. Chaque scénario obligatoire doit être `PASS` pour franchir L2. `FAIL` ou `NOT RUN` bloque le gate, sans empêcher la préparation indépendante du reste du plan. |
| **T23 — Dépendances et droits** | Inventorier imports PE statiques/dynamiques, runtimes système/CUDA, modèle et prérequis ; fermer licences et droits de redistribution. Définir livraison locale du modèle ; si une voie réseau est envisagée, la soumettre séparément au railguard et à l’accord requis. | AC-04 : profil propre sans SDK/outils de build démarre et atteint les ressources nécessaires ; aucune dépendance masquée par le PC de build. Ressource/droit inconnu bloque le paquet public. |
| **T24 — Contrat Windows/installation neuve** | Définir versions Windows 11 Home/Pro encore supportées au moment de publier, format d’installation Rust proposé, racines/effets, messages, préflight et point d’engagement. Mise à jour, désinstallation, migration et rollback sont hors lot. | Matrice OS distincte de v2 ; installation occupée/refusée ne remplace pas une installation existante. Le format et toute technologie hors stack demandent arbitrage précis. AC-03/04. |

**Gate L2→L3 :** édition complète, ressources et droits fermés ; chaque contrôle GPU/Auto applicable est `PASS` ; format et effets de l’installation neuve acceptés. `NOT RUN`, limitation ou résultat absent laisse le gate fermé. Les DLL natives suspendues ne sont pas une solution de repli implicite.

## L3 — Installateur et paquet complet

Chemins candidats : `crates/whisper-bootstrap/` pour l’entrée d’installation, `ci/` pour l’assemblage et `docs/` pour la configuration minimale. T31 réutilise la politique/probe de T12 ; elle n’est pas réécrite. Confirmer chaque fichier et lire son `AGENTS.md` avant autorisation.

| ID | Changement borné | Critères et vérifications |
|---|---|---|
| **T31 — Entrée d’installation au socle x64** | Intégrer dans le bootstrap la politique et l’adaptateur de T12 ; compiler l’entrée de refus au socle x64 et garder ses imports indépendants du worker/modèle. | Contrôle réel sur le témoin infra-v2 de T01 ; admission seulement après preuve de toutes les capacités, résultat indéterminé distinct. AC-03a/b. |
| **T32 — Installation neuve sans mutation au refus** | Ajouter l’ordre de contrôle avant tous effets durables du produit ; définir annulation avant engagement et l’erreur après engagement sans annoncer un faux succès. | Comparaison d’état réel avant/après refus ou lecture indéterminée ; message intelligible ; installation propre admise ; installation existante préservée. AC-03a/b. |
| **T33 — Assemblage et première utilisation** | Assembler le candidat unique, vérifier imports PE, workers et ressources ; suivre ses instructions depuis un profil Windows propre avec `%LOCALAPPDATA%` vide. Ne pas créer artificiellement les `settings` ou `logs` que le produit doit initialiser. | AC-04/05 : premier démarrage et inférence WAV sans dépôt, SDK CUDA ni outils de build ; observer l’initialisation réelle des données du premier lancement. Identité candidat et sorties liées à la recette. |
| **T34 — Brouillon de configuration** | Préparer la configuration minimale et les instructions attachées au candidat : x64-v2, Windows Home/Pro, dépendances, modèle et NVIDIA ; marquer les affirmations de compatibilité comme provisoires jusqu’à T41–T43. | AC-04 : plancher CPU séparé du support OS ; aucun modèle/runtime/droit manquant présenté comme résolu. T44 fige uniquement les configurations effectivement qualifiées. Publication réservée à un accord propre. |

**Gate L3→L4 :** installation neuve observée sur compatible et refusée avant mutation sur incompatible ; paquet propre passe premier lancement ; prérequis/licences fermés ; identité figée. Toute modification après ce gate crée un nouveau candidat.

## L4 — Qualification et remise

Le code de recette proposé est `ci/distribution-cpu-x64.ps1`, avec le workflow `.github/workflows/distribution-cpu-x64.yml` seulement après acceptation du mécanisme et mandat d’activation. Ces chemins ne sont pas préautorisés. Si le workflow n’est pas autorisé, garder les jobs `NOT RUN` et le candidat non qualifié. Les deux jobs utilisent exactement le même artefact x64.

| ID | Preuve attendue | Critères et vérifications |
|---|---|---|
| **T41 — CI témoin v2** | Ajouter au script proposé `ci/distribution-cpu-x64.ps1` le job v2, puis exécuter l’artefact figé : installer, démarrer desktop/worker et terminer une inférence Turbo CPU. Relever identité, OS, capacités des processus, flags et sorties. | AC-07 `PASS` seulement après observation complète. Runner non caractérisé, profil de garde absent, masque incomplet ou job non autorisé = `NOT RUN` et gate fermé. |
| **T42 — CI Windows on Arm** | Dans le même script de recette, ajouter le job Windows ARM64 et exécuter le même paquet sous émulation x64. Prouver architecture hôte et processus, installation et transcription. | AC-08 `PASS` seulement après observation complète ; pas de support ARM64 natif. Masquage Prism non requis, sauf preuve distincte qui le demande. Profil N2 absent n’autorise aucun contournement. |
| **T43 — Compatibilité produit Intel/AMD et moteurs** | Recette d’installation/import WAV et MP3 sur témoins Intel et AMD ; recette CPU/NVIDIA/Auto sur les profils concernés. Essai micro avec bloc confirmé, TXT/SRT/WAV complets et phrase finale. | AC-01/05/06/09/10 ; distinguer chaque environnement et sortie. Test GPU N2, ressources et banc conformes à leur mandat. |
| **T44 — Dossier de livraison** | Consolider résultats AC-01…AC-10, limites, versions supportées et configuration minimale ; figer les affirmations uniquement après preuves. Remettre le candidat et les instructions à l’utilisateur pour l’essai dans la fenêtre Windows. | Tout critère obligatoire/applicable `FAIL` ou `NOT RUN` interdit de qualifier le candidat ; les réserves et résultats absents restent distincts. La recette fenêtre finale est faite par l’utilisateur ; la qualification produit finale attend son résultat. Aucune publication ou push automatique. |

**Sortie L4 :** candidat qualifié automatiquement uniquement si tous les critères obligatoires/applicables sont `PASS`, avec preuves attribuées et remise pour la validation utilisateur. Tant que l’essai fenêtre réservé à l’utilisateur n’est pas retourné, le statut reste « candidat remis, validation produit en attente ». Un build vert, une détection simulée, un `NOT RUN` ou une seule architecture ne qualifie pas le paquet.

## Arbitrages et non-autorisations

- Si le banc v2, le refus réel ou le plafonnement du Turbo échoue, suspendre les lots dépendants et soumettre observations et options ; ne pas abaisser le plancher ni assouplir les critères sans décision produit.
- Toute extension de la garde, technologie hors stack, élévation, réseau produit, service, nouveau runner, activation de workflow ou construction DLL native requiert l’accord précis de la session selon `AGENTS.md`. La qualification CI exigée n’autorise pas à elle seule ces changements d’infrastructure.
- Tout lot Rust suit la délégation `rust_implementer`, puis la revue indépendante et la validation applicables. Le présent plan n’est pas une autorisation d’implémentation ; le mandat du premier lot reste à demander après revue et acceptation du plan.
- Les nettoyages de cibles et temporaires respectent la rétention : archiver et vérifier les preuves, identifier les chemins canoniques, préserver les caches partagés et ne jamais nettoyer les éléments d’un autre lot.

**Vérifications du document :** aucune campagne, build, commande Cargo ou exécution CI n’a été lancée pour écrire ce plan. Le futur candidat devra satisfaire les contrôles prévus via la garde ; les preuves matérielles et l’essai fenêtre restent à faire.
