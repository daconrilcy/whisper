Avancée : design L1 revu CLEAN, plan L1a revu PLANS READY-L1a et accepté par l'utilisateur avec ses cinq chemins d'écriture.
Blocage : l'accord écrit précis d'implémentation du lot reste à obtenir ; le raccordement produit, le micro et le paquet final ont des gates distinctes.
Décision : conserver `PLAN-PARA-L1a v1` comme premier lot accepté ; cette acceptation documentaire n'autorise pas le code.

# Plan d'implémentation Parakeet — premier lot L1a

**Parents Git :** `docs/cadrage/evo-parakeet.md` et `docs/cadrage/evo-engine-capabilities.md` à `471821e5`, bilan de faisabilité à `8fee7329`. La revue indépendante du design a conclu **CLEAN sur le seul périmètre L1 catalogue/capacités**. La revue indépendante du plan corrigé à `f6545e38` a conclu **CLEAN — PLANS READY-L1a**, sans REQUIRED restant. L'utilisateur a accepté `PLAN-PARA-L1a v1` et ses cinq chemins d'écriture dans sa réponse explicite du 16 octobre 2026. Ces décisions ne déclarent pas prêts les lots natifs, le micro ni la distribution et ne valent pas accord d'implémentation. **Contribution :** `rust_plan_writer`, en lecture seule ; rédaction et versionnement par la session principale. La version DRAFT précédente reste disponible dans l'historique Git.

Le but produit demeure l'ajout de Parakeet TDT 0.6B v3 à Turbo dans l'application Windows. L1a met en place le contrat du registre de moteurs avant le raccordement. Il n'active aucun modèle nouveau. Les résultats du banc FR/EN CPU/CUDA0 sont des preuves de faisabilité isolées, sans qualification du candidat produit ni de son paquet final.

## 1. Index et gates

| Lot | Résultat démontrable | Prérequis et état |
| --- | --- | --- |
| **L1a — socle pur** | Deux identités fermées, fiches de capacités injectées, projection et validation déterministes avec causes. | Design L1 et plan L1a revus ; premier lot et cinq chemins acceptés. Accord d'implémentation précis encore requis. |
| L1b — raccordement | Catalogue réel, disponibilité, préférences/migration, liste de modèles et de langues dans l'UI, refus avant séance. | L1a ; fiches initiales Turbo justifiées pour conserver les parcours existants ; plan/chemins et revue propres. **DRAFT**. |
| L2 — import Parakeet CPU | WAV/MP3, texte, TXT/SRT, worker et horodatages. | L1b ; contrat natif/IPC et preuve du candidat exact ; cas sans silence admissible à trancher pour la découpe longue ; acceptation 60 min VAD pendant l'intégration. **DRAFT**. |
| L3 — GPU et Auto calcul | GPU attendu réellement utilisé, modes manuels fixes et repli Auto borné avec le même modèle. | L2 ; attestation produit GPU. **DRAFT**. |
| L4 — microphone | Blocs confirmés et TXT/SRT/WAV, premier texte GPU <15 s. | L2/L3 ; stratégie incrémentale et frontières live encore à qualifier. **DESIGN CHANGE REQUIRED** pour ce parcours. |
| L5 — acquisition | Installation puis réparation explicite et annulable des modèles. | L1b ; contrat réseau/stockage, modification autorisée de RG-08 avant réseau produit. **DRAFT**. |
| L6 — distribution | Paquet installable utilisateur courant et candidat exact qualifié FR/EN CPU/GPU. | L3–L5 ; inventaire transitif et `AC-PARA-16F`. **DRAFT**. |
| L7 — qualité et recette | Comparaison attribuée à une vérité terrain et essai final Windows. | Parcours concernés et corpus accepté. **DRAFT**. |

Les anciens détails L1–L7 ne sont pas des `lot.paths` autorisés. Aucun autre lot n'est exécutable par le présent plan. La politique **fichier** reste passe unique jusqu'à 30 min, puis VAD 60–120 s sur silences ≥400 ms avec marge 200 ms ; elle ne sert pas de contrat micro. Le format visant 20 min n'est pas retenu. Le test d'intégration de 60 min à nombreuses coupures reste obligatoire dans un lot consommateur, sans nouvel essai de faisabilité ici.

## 2. L1a — contrat et tâches (5)

**Résultat :** avec deux identités connues et un snapshot injecté, la même fonction pure produit les options visibles et refuse une demande qui ne satisfait pas leurs règles. Les causes sont explicites ; la préférence conservée est distincte de la sélection exécutable. Cela démontre un contrat testable du framework, sans prétendre que l'UI ou le moteur l'utilise déjà.

**Frontière :** `model_catalog` définit les types et identités fermées ; `engine_capabilities` évalue les données fournies. Aucun IO, FFI, `unsafe`, sonde PC, lecture du disque, calcul natif, travailleur, téléchargement ou changement des requêtes `ImportRequest`/`LiveRequest`. `ports.rs` reste en lecture pour L1a et sera réexaminé dans L1b avec ses consommateurs. Aucune modification de `AppFacade`, de l'UI, des réglages ou du pipeline Turbo n'est permise par L1a.

| Tâche | Sortie observable et critère | Trace |
| --- | --- | --- |
| L1a.1 | Types `ModelId` Turbo/Parakeet et clé de configuration incluant identité d'artefact/révision, runtime/build, worker/IPC, backend, matériel et profil pertinents ; une preuve d'une autre clé ne se transfère pas. | CAP-31/34, DEC-CAP-01/02. |
| L1a.2 | Entrées injectées distinctes pour présence du modèle, support, qualification, langue Auto/manuelle, calcul et ressources RAM/VRAM/disque ; `Unknown` n'est ni `Unsupported` ni `Insufficient`. | CAP-31/32, PORT-CAT, CAP-02/03. |
| L1a.3 | Projection et validation d'une demande par la même règle : aucune substitution de modèle, langue ou calcul, causes typées et préférence inchangée ; résultat d'une génération périmée ignoré. | CAP-32/33, INV-PARA-01, DEC-PARA-19. |
| L1a.4 | Tests du catalogue avec fiches **synthétiques** : deux identités, états de disponibilité, non héritage de qualification ; Turbo injecté admissible lorsque Parakeet produit n'est pas qualifié. | CAP-31/34, AC-PARA-05/08 partiels. |
| L1a.5 | Tests langue/ressources/préférences : Auto seul, manuel seul, aucun mode, langue refusée, demande contournant une option grisée, mémoire inconnue et insuffisance comparable. | CAP-32/33, AC-PARA-16A–E partiels. |

**Règles indispensables.** Un modèle manque/corrompu/annulé reste dans la projection avec cause et ne devient pas sélectionnable ; seul un état vérifié `Ready` permet une sélection, sous réserve des capacités qualifiées. Auto langue et choix manuel sont deux capacités indépendantes ; Parakeet manuel est `Unsupported` dans le couple GGUF/NeMo v0.2.0 examiné, tandis qu'Auto FR/EN du **banc** ne qualifie pas le futur candidat produit. Une fiche de test Auto qualifiée exerce la règle future, sans autoriser son activation. `language_count` n'est pas un critère. Une ressource RAM/VRAM non mesurée ne prouve aucune pénurie ; seules des valeurs comparables peuvent établir `Insufficient`. La demande inadmissible reste refusée même si elle contourne une projection UI.

**Turbo.** L1a ne branche pas le registre sur le produit actuel. Un test injecte une fiche Turbo qualifiée et vérifie l'admissibilité de ce cas ; cette fixture ne crée **aucune preuve réelle** sur Turbo. Avant L1b, les fiches Turbo initiales doivent être justifiées pour les choix aujourd'hui disponibles : brancher des fiches entièrement inconnues risquerait de désactiver le moteur existant. Cette entrée documentaire est obligatoire pour le raccordement.

## 3. `lot.paths` exacts de L1a

**`lot.paths.write` (et aucun autre chemin source) :**

```text
crates/whisper-core/src/lib.rs
crates/whisper-core/src/model_catalog.rs
crates/whisper-core/src/engine_capabilities.rs
crates/whisper-core/tests/model_catalog.rs
crates/whisper-core/tests/engine_capabilities.rs
```

`lib.rs` exporte les deux modules nouveaux. Les quatre autres chemins n'existent pas à la base du plan. Une nécessité d'écrire `domain.rs`, `ports.rs`, `application.rs`, un manifest ou un autre chemin exige un changement explicite du plan avant l'édition.

**`lot.paths.read` :** `AGENTS.md`, `RAILGUARD.md`, `Cargo.toml`, `Cargo.lock`, `crates/whisper-core/Cargo.toml`, `crates/whisper-core/src/{domain.rs,ports.rs,application.rs,compute_policy.rs,gpu_selection.rs,path_settings.rs}`, `crates/whisper-core/tests/{compute_policy.rs,gpu_selection.rs}`, les deux parents design, ce plan et `docs/cadrage/bilan-parakeet-faisabilite-cloture.md`.

**`lot.paths.runtime_write` :** `target/parakeet-tests/l1a/` exclusivement pour cible Cargo et temporaire de campagne. Les fixtures sont construites en mémoire dans les deux tests nouveaux ; aucun audio, modèle, DLL, réglage utilisateur ni journal hors dépôt n'est une entrée ou sortie de L1a. En PowerShell, `CARGO_TARGET_DIR`, `TEMP` et `TMP` du processus de campagne pointent vers des sous-dossiers de cette racine puis sont restaurés. La campagne exécute uniquement les tests `whisper-core` ; les tests des autres crates qui construisent directement d'autres chemins sous `target/` n'en font pas partie. Les commandes ne nettoient pas globalement `target/`.

## 4. Cartes d'exécution et verdict attendu

Toutes les cartes sont **NOT RUN** avant implémentation. L'implémenteur produit le diff et les sorties ; un reviewer indépendant juge les constats sur le candidat.

| Carte | Stimulus en mémoire | Observation et critère d'acceptation | Limite |
| --- | --- | --- | --- |
| A1 | Turbo/Parakeet et clés variant révision, backend ou profil. | Exactement les deux identités ; aucune qualification héritée d'une autre clé. | Pas de modèle réel. |
| A2 | `Ready`, `Missing`, `Corrupt`, `Cancelled`. | Deux entrées conservées, seule configuration vérifiée admissible, causes cohérentes. | Pas d'inspection du disque. |
| A3 | Turbo qualifié injecté ; Parakeet présent mais produit non qualifié. | Turbo admissible, Parakeet refusé, aucun repli implicite. | Fiche Turbo synthétique. |
| A4 | Auto absent + manuel qualifié ; Auto qualifié + manuel absent ; aucun mode ; FR/EN et code non pris en charge. | Projection et validation concordent ; choix manuel explicite s'il est requis ; aucun mode entraîne refus. | Aucun appel natif de langue. |
| A5 | Préférence modèle/langue devenue inadmissible ; demande injectée hors widget. | Refus avec cause et préférence intacte ; aucun choix substitué. | Pas de persistance ni réouverture. |
| A6 | RAM/VRAM/disque inconnus, insuffisance comparable, observation autre GPU/unité/profil. | `Unknown` reste distinct d'`Insufficient` ; seule combinaison démontrée insuffisante est écartée. | Valeurs de test synthétiques. |
| A7 | Snapshot de génération ancienne après un choix plus récent. | Résultat périmé écarté ; demande et préférence courantes préservées. | Pas de concurrence réelle. |
| A8 | Diff des cinq chemins, tests core existants et nouveaux. | Aucune dépendance OS/UI/FFI/IO nouvelle ; tests sans saut conditionnel et compilation sans régression. | Ne qualifie pas le produit Windows. |

Rapprochement inverse : `lib.rs` → A1/A8 ; `model_catalog.rs` et son test → A1–A3 ; `engine_capabilities.rs` et son test → A1–A7. Les cinq tâches et les huit cartes restent sous la limite de dix tâches. Un besoin d'extension de chemins, d'infrastructure ou de règle produit retourne `PLAN_CHANGE_REQUIRED` ou `DESIGN_CHANGE_REQUIRED` avec cause et impact ; aucun accès supplémentaire n'est implicite.

## 5. Préflight, vérifications et routage

Avant toute écriture de code : relire les règles et parents Git exacts, le verdict indépendant sur ce plan et l'acceptation déjà donnée par l'utilisateur pour **L1a et ses cinq chemins**. La demande d'implémentation ultérieure doit donner **l'accord écrit précis dans sa session** pour le lot et ses chemins, conformément à `AGENTS.md`. Relever `git status --short --branch`, `git rev-parse HEAD`, `git diff --check` et préserver tout autre travail, notamment `sound example/`. Dans le processus de campagne, retirer temporairement les variables `WHISPER_L01_*`, `WHISPER_L02_*` et `WHISPER_L04_*` héritées, puis les restaurer ; elles ne doivent pas déclencher de parcours natif durant L1a.

Après code autorisé, dans `C:\dev\whisper` avec Cargo offline/locked et les variables temporaires bornées ci-dessus :

```powershell
cargo fmt --all -- --check
cargo test --offline --locked -p whisper-core --test model_catalog
cargo test --offline --locked -p whisper-core --test engine_capabilities
cargo build --offline --locked --workspace
cargo test --offline --locked -p whisper-core
git diff --check
git diff --name-only
```

Les deux tests dédiés doivent exercer réellement A1–A7, sans succès conditionnel si une entrée manque. Le build workspace vérifie la compilation des consommateurs actuels de `whisper-core` ; **seuls les tests `whisper-core` sont lancés**. Les tests des autres crates et leurs sous-scénarios natifs restent `NOT RUN`, jamais annoncés comme qualification GPU ou micro. Aucun benchmark ni nouvelle campagne de faisabilité n'est demandé. Le résultat du lot distingue tests de règle pure, validation Rust et recette produit dans la fenêtre Windows.

Après le candidat, la session principale confie la revue à `rust_review_coordinator` ; `rust_reviewer` juge indépendamment, `rust_implementer` corrige le cas échéant, pour au plus deux cycles revue/correction. La fenêtre Windows finale reste à l'utilisateur. Aucun push, téléchargement, déplacement de modèles, code natif, écriture hors dépôt ou changement de railguard n'est compris dans L1a.
