# ARCH-L04-D20-BASELINE-v1 — apport brut proposé

**Acteur :** `/root/l04_design_delta`, architecte en lecture seule.  
**Owner proposé :** `AUTH-ARCH-P04`.  
**Date :** 2026-10-07.  
**Statut :** proposition DRAFT, non persistée par cet acteur.  
**Objet :** D-WHISPER-20, delta documentaire minimal permettant de lier la baseline L04 au manifeste DESIGN.

**Bases vérifiées :**

- DESIGN actif : D-WHISPER-19, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`, READY/CLEAN dans le state.
- PLANS actif : P-WHISPER-24, digest `fb071cd5784eeac49e4bdac286e0891e00119e23c1d32df2ab5c07e2c12a605b`, READY/CLEAN, parent D19.
- HEAD observé : `ffd93125604be5dc6439587b7edad4f420f5cccc`, branche `main`.
- Manifeste D19, SHA du fichier : `7f6465f9254611d432da50ab64034c7ae92b088351396f2c3eb3249c71866cb7`.
- Manifeste P24, SHA du fichier : `38c2c7d3e3d53a72e13408a3fb43cd875625024280e0641a153d6f0da5166fef`.

L’information disponible suffit à cette proposition. Aucune question produit ou architecturale bloquante supplémentaire n’est identifiée. Les preuves à persister, les revues exactes et le préflight restent à réaliser par les acteurs compétents.

## Décision technique proposée

**TECH-D20-L04-BASELINE-01**, nature `decision`, version 1, owner `AUTH-ARCH-P04`, approval `proposed`.

Créer D20 comme successeur immuable de D19. Conserver son corpus métier, ses décisions et preuves de faisabilité ; ajouter un complément explicite `52_L04_SOURCE_BASELINE_D20.md`, un inventaire courant et dix snapshots documentaires immuables.

La baseline ajoutée correspond au début de L04, après progression L00–L03. Elle ne doit jamais être présentée comme la baseline originale de D19 ni comme une preuve de qualification L04. La révision proposée est `BASELINE-L04-D20-ffd93125604be5dc6439587b7edad4f420f5cccc`.

Options examinées :

| Option | Conséquence | Statut |
| --- | --- | --- |
| Modifier D19 ou P24 | Altère les candidats déjà revus et leurs empreintes | Rejetée |
| Changer le contrôleur pour accepter un snapshot PLANS à la place de DESIGN | Modifie la règle centrale et demande un périmètre de contrôle supplémentaire | Non retenue |
| D20 avec snapshots DESIGN, puis PLANS successeur parent D20 | Suit le contrôle actuel et conserve les candidats historiques | Proposée |

**Conséquence obligatoire :** préparer P-WHISPER-25 parent D20, reprenant P24 avec les seules modifications de lignée, références et instructions de préflight nécessaires. Le schéma exige que le parent PLANS égale le DESIGN courant et que le manifeste PLANS inclue les octets du manifeste DESIGN parent. D20 ne peut donc devenir DESIGN courant avec P24 demeurant PLANS courant.

L’utilisateur a autorisé une proposition élargissant les chemins et le DESIGN si nécessaire, puis explicitement autorisé implémentation L04 sur les quinze chemins et préflight. Capturer ces sources et leur portée ; aucune nouvelle autorisation n’est nécessaire pour cette proposition technique.

## Baseline et chemins futurs

L’hôte peut conserver les mêmes chemins relatifs de snapshots que P24 sous `candidates/D-WHISPER-20/`. Chaque snapshot doit être copié en octets bruts, sans conversion de fins de ligne, encodage ou normalisation. Les dix références doivent figurer directement dans `manifest.json` DESIGN D20.

Les hashes suivants ont été comparés en lecture seule entre fichiers actuels et snapshots P24 ; ils sont égaux.

| Chemin du code | SHA-256 |
| --- | --- |
| `crates/whisper-worker-cpu/src/native_engine.rs` | `9d415e118185e0c97917098bf79902f649722a909f1486ef619c3626e88203d7` |
| `crates/whisper-adapters/src/lib.rs` | `9b71f5efe1d6669ca6e38e865915b6e4bfb6504558e076f520eaf86f20d09174` |
| `crates/whisper-adapters/src/worker_ipc.rs` | `4f1bfa7a9bdf0811046204831224a348901dfc36b64cec225ebab53ed284e2c2` |
| `crates/whisper-core/src/application.rs` | `47c32fcc0f9d2709670723ffea636a0324bdef51a62f9d0c9b35dbc4f2294c69` |
| `crates/whisper-core/src/lib.rs` | `5b047a792ee30b73fed1e3cab2c8bc1a47b8f41f781ceae750905cbd27607d3a` |
| `crates/whisper-desktop/src/root.rs` | `9a3713112e3dfc3872e15e3cb581c7c6a53222e5f62864144ba4d111caa98599` |
| `crates/whisper-desktop/src/ui.rs` | `148900588d475028b2c0c27c65ed05116dd93aecf62568c78cef6a554b913648` |
| `crates/whisper-worker-gpu/src/main.rs` | `047430149342e5fec4e77d8b9009ee377af8c0b3c50a9c96b3fb52536d96f953` |
| `crates/whisper-worker-gpu/Cargo.toml` | `5ef21c0437965d8ea28c01bdd33f5c5d3ec867e2498ab4329b3a24ad54553b89` |
| `Cargo.lock` | `9d714d4315a5dd63865faa6bc8eb9bcb84eb9abdfdd2182a1fafbb4c2b60b67f` |

Les huit fichiers Rust proviennent de `P24/sources/L04-consumer-baseline/<chemin-code>.txt`. Les deux autres proviennent de `P24/sources/L04-config-baseline/crates__whisper-worker-gpu__Cargo.toml.txt` et `Cargo.lock.txt`.

Ces cinq chemins sont actuellement absents et restent des créations futures :

- `crates/whisper-adapters/src/supervisor.rs`
- `crates/whisper-adapters/tests/worker_control.rs`
- `crates/whisper-core/src/compute_policy.rs`
- `crates/whisper-core/tests/compute_policy.rs`
- `crates/whisper-worker-gpu/src/native_engine.rs`

Consigner leurs observations d’absence dans l’inventaire D20. Ne pas créer de fichiers vides, hashes fictifs ou entrées `baseline`/`current` pour ces chemins. Les quinze chemins restent l’allowlist autorisée ; l’inventaire initial de fichiers présents n’en contient que dix. Après création autorisée, ces nouveaux outputs seront enregistrés dans les preuves de réalisation du lot et, pour les lots suivants, dans le ledger L04 completed.

Pour le périmètre L04, `code_state.baseline` porte les dix fichiers présents, avec `path`, `sha256`, `snapshot` D20, `revision` et `dirty_hash` de l’inventaire effectivement persisté. `code_state.current` porte ces mêmes dix chemins et leurs hashes courants. Un ledger propre à cette baseline peut être vide au lancement ; les preuves historiques L01/L03 restent explicitement référencées comme provenance et prérequis d’exécution.

## Matrice de responsabilités conservée

| Portée physique | Responsabilité et dépendances | Contrôle requis lors de réalisation |
| --- | --- | --- |
| Core `application.rs`, `lib.rs`, nouveau `compute_policy.rs` | Scheduler et policy purs ; aucune dépendance adaptateur, native ou UI | Imports, API publique, Cargo/features et test de policy |
| Adapters `lib.rs`, `worker_ipc.rs`, nouveau `supervisor.rs` | Implémentation des ports, supervision parent, IPC ; consomment core et infrastructure ; aucun widget | Imports, contrats IPC et tests `worker_control.rs` |
| Desktop `root.rs` | Composition des implémentations concrètes ; aucun état métier parallèle | Revue de composition |
| Desktop `ui.rs` | Vues et commandes application ; aucune opération native, stockage ou attente arbitrairement bloquante | Revue des consommateurs et qualification de la vraie interface |
| Workers CPU/GPU `native_engine.rs`, GPU `main.rs` | Contexte moteur/native dans l’enfant ; protocole corrélé ; aucun contrôle métier parallèle | Revue protocole/FFI et essais attribués CPU/GPU |
| GPU `Cargo.toml`, `Cargo.lock` | Graphe de dépendances et features nécessaires aux contrats existants | Diff Cargo et versions verrouillées exactes |

D20 réaffirme les contrats concrets D19/08 et D19/50 : identité job/génération/séquence, refus des événements obsolètes, saturation explicite, admission Stop indépendante du trafic data, inférence/FFI hors UI, Quitter coopératif et latched, attente du `Stopped` corrélé et d’un état durable stabilisé. Aucun délai maximal de sortie native, kill ou fallback GPU Strict n’est ajouté.

Les propriétés d’archives restent inchangées : journalisation/confirmations par le parent ; import sans copie audio MP3 archivée ; live avec artefact MP3 de l’enfant et publication coordonnée par le parent. Aucune transaction atomique entre fichiers n’est supposée ; publication et réconciliation suivent D19/08 et D19/50.

Les détails Q-04/Q-06/Q-09 et DETAIL-P03 doivent être référencés depuis leurs apports exacts conservés, avec acteur, version et acceptation réelles. Le complément D20 ne convertit pas silencieusement un paramètre technique proposé en choix utilisateur.

## Identité et provenance

L’observation Git suivante a renvoyé une liste vide :

`git diff --name-only c75ae195b4431abd8f8c1dfd7abb45e81f0ad79b ffd93125604be5dc6439587b7edad4f420f5cccc -- crates Cargo.lock Cargo.toml rust-toolchain.toml`

Le diff courant sur cette portée était également vide. Les dix égalités de hash et cinq absences confirment la continuité du périmètre L04 au HEAD actuel. Cette preuve ne revendique pas une worktree globalement propre.

L’inventaire historique P24 est conservé comme historique. Son fichier `sources/progress/L04-repository-inventory-2026-10-07.md` porte une observation plus ancienne au HEAD `1310bbcd…`, tandis que son complément `14` indique `c75ae195…`. D20 doit dater son propre inventaire à `ffd93125…` et expliciter ces observations distinctes ; ne réécrire aucune des deux sources P24.

**RISK-T-D20-BASELINE-01**, owner `AUTH-COORD`, version 1 : dérive d’identité ou d’octets entre inventaire, persistance, revue et lancement. Fermeture : preuve de dépôt datée, égalité code/snapshot, absence des cinq créations futures, hash du dirty inventory, transport exact et vérification immédiatement avant préflight. Une mutation de code imprévue suspend le lot.

## Critères de handoff et preuves

La présente correction ne crée aucune nouvelle hypothèse de faisabilité structurante, dépendance native ou décision de redistribution. Les versions/features et limites natives restent celles des sources D19 et P24. Aucun SPIKE supplémentaire n’est proposé.

| Preuve | Classement | État de cet apport |
| --- | --- | --- |
| Compatibilité du delta avec frontières et contrôleur | DESIGN_FEASIBILITY documentaire | Analysée, proposée |
| Égalités de hashes et absence des cinq chemins | Contrôle de provenance documentaire | Observé en lecture seule ; à persister par l’hôte |
| CPU/GPU, saturation, Stop/Quitter, diagnostics, archive, vraie UI | PRODUCT_VALIDATION | NOT RUN |
| Packaging et redistribution du candidat final | DELIVERY_QUALIFICATION | NOT RUN |
| `check-state --lot L-WHISPER-04` | Gate documentaire de lancement | NOT RUN |

Séquence proposée : persistance D20 DRAFT et brut exact en TRANSPORT ; reçu/hash/checkpoint vérifiés ; revue DESIGN D20 indépendante ; P25 parent D20 et revue PLANS exacte ; checkpoint implémentation avec quinze chemins, source utilisateur L04, détails acceptés et preuves courantes ; contrôles/promotion et préflight du lot.

Aucun résultat de contrôleur, revue CLEAN D20/P25 ou préflight réussi n’est revendiqué ici.

**Références utilisées :** `rust-predev-design/SKILL.md`, les cinq `agent-rules`, `engineering-contract.md`, `handoff-contract.md`, `deliverable-quality.md`, `execution-evidence.md`, `workflow-schema.md` sections Manifest et `code_state`, `predev_control.py:231–241` et `813–864`, state courant, D19/03/08/50, P24/13/14 et ses dix snapshots.
