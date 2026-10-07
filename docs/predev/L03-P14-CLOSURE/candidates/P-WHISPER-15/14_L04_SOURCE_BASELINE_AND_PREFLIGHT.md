# Sources courantes et preflight L04 — P-WHISPER-15

## Autorité et portée

P15 DRAFT, base P14 exacte, parent DESIGN D19 conservé. Le mandat humain couvre la préparation documentaire et la revue indépendante. L’autorisation d’implémentation L04 demeure NOT_REQUESTED.

Couverture conservée : REQ-07/08/11/14/15/16/18/24, UC-07/09/11/13/19, AC-07/09/11/13/19, TECH-D18-01/03/04 et TASK-P04. Q-04/Q-06/Q-09 et DETAIL-P03 restent ceux acceptés dans le registre. Aucun nouveau contrat produit ou paramètre technique n’est décidé dans ce document.

## Inventaire exact

HEAD observé : `1310bbcddb8c32d7988211ff0cdd76224dae62b0` sur `main`, cwd `C:\dev\whisper`, capture du 2026-10-07.

| Chemin L04 | État observé |
| --- | --- |
| crates/whisper-adapters/src/supervisor.rs | Absent; création future attendue |
| crates/whisper-adapters/tests/worker_control.rs | Absent; création future attendue |
| crates/whisper-core/tests/compute_policy.rs | Absent; création future attendue |
| crates/whisper-worker-cpu/src/native_engine.rs | Présent; output L01 vérifié |
| crates/whisper-worker-gpu/src/native_engine.rs | Absent; création future attendue |

SHA-256 CPU courant : `9d415e118185e0c97917098bf79902f649722a909f1486ef619c3626e88203d7`.

Cette empreinte correspond à l’output déclaré par L01 completed dans `transports/T-WHISPER-L01-CLOSURE-02/execution-evidence.json`, SHA-256 du paquet `f354c02310a3376fa7a181e596321eae42a4e0f7d8c30306b3c1f47031425920`. Le chemin appartient au périmètre L01; L01 est un ancêtre L04 via L03→L02→L01. Le fichier n’est pas présenté comme source initiale D19.

La capture du dépôt conserve les commandes, le cwd, la branche, HEAD et les sorties observées. Les quatre absences sont vérifiées par `Test-Path -LiteralPath` et consignées sans hash. Cet inventaire ne prétend pas démontrer une absence à une date ultérieure. Aucun fichier vide ni empreinte fictive n’est créé.

## Représentation canonique code_state

baseline demeure vide parce qu’aucun des cinq chemins ne possède un snapshot lié au manifest D19.

ledger contient uniquement l’output CPU réellement prouvé de L01. current contient uniquement le fichier CPU correspondant. Les chemins de code sont relatifs à code-root; les références de preuve sont relatives à la racine documentaire.

```json
{
  "baseline": [],
  "ledger": [
    {
      "lot_id": "L-WHISPER-01",
      "path": "crates/whisper-worker-cpu/src/native_engine.rs",
      "sha256": "9d415e118185e0c97917098bf79902f649722a909f1486ef619c3626e88203d7",
      "evidence": [
        {
          "path": "transports/T-WHISPER-L01-CLOSURE-02/execution-evidence.json",
          "sha256": "f354c02310a3376fa7a181e596321eae42a4e0f7d8c30306b3c1f47031425920"
        }
      ]
    }
  ],
  "current": [
    {
      "path": "crates/whisper-worker-cpu/src/native_engine.rs",
      "sha256": "9d415e118185e0c97917098bf79902f649722a909f1486ef619c3626e88203d7"
    }
  ]
}
```

Les quatre absences relèvent de `repository_evidence`, pas de baseline/current/ledger. Le reviewer confirme l’exhaustivité de cet inventaire. Le contrôleur vérifie `current = baseline + outputs du ledger`; il ne contrôle pas mécaniquement les absences attendues.

Avant le premier changement, refaire l’inventaire complet. Apparition d’un chemin attendu absent, disparition du CPU ou différence d’empreinte : suspendre et réconcilier la source de progression avant code.

## Drift et dépendances

Classification proposée : `expected_previous_lots`. `previous_lots` contient seulement L-WHISPER-03, prérequis direct L04. Les preuves de drift expliquent la progression L01→L02→L03 et l’héritage CPU L01; le ledger conserve le producteur réel L01.

La dépendance EXECUTION L04→L03 devra référencer après promotion la preuve canonique actuelle :

- path : `L03-P14-CLOSURE/transports/T-WHISPER-L03-CLOSURE-04/execution-evidence.json`
- sha256 : `247966a803d10445597dd9fc3915e855409a54cb09ff9b4e17997fc0f4c4ec53`

Le statut L03 completed et son candidat exécuté P14 sont conservés. Les validations CPU, la compilation CUDA, leurs échecs antérieurs et leurs limites restent attribués à L03. Micro réel, inférence GPU/runtime et qualification produit non exercés restent NOT RUN.

## Dossier documentaire puis gate d’exécution

Pour le dossier documentaire : vérifier D19/P15 manifests, gel effectif du pack, provenance brute et copies exactes, impact KEEP_D19 proposé et revue indépendante P15. Après verdict admissible, préparer/promouvoir le checkpoint documentaire avec `check-state` sans `--lot`. L04 reste planned et son autorisation demeure absente.

Pour le gate d’exécution : obtenir un mandat applicable aux cinq chemins; réattester le railguard actif et son équivalence; vérifier les détails requis et la clôture L03 actuelle; préparer la matrice d’environnement L04 consommable par `check-build-environment`; vérifier SDK/toolchain/natifs requis; capturer HEAD/branche/status et changements locaux; actualiser `repository_evidence`, drift et `code_state` à partir de preuves réelles; exécuter `check-state --lot L-WHISPER-04` sur le pending exact avant toute promotion du préflight.

Aucune réussite du gate documentaire n’équivaut à une réussite du gate d’implémentation.

## Frontières et intégration

Les cinq chemins sont le périmètre conservé. L04 doit démontrer que l’intégration nécessaire est disponible dans ce périmètre. Un besoin de modifier lib.rs/main.rs, UI/root, IPC, ports, manifests ou configuration déclenche CHANGE et bloque avant édition. Un module non intégré au consumer réel ne prouve aucun comportement opérationnel.

Une décision structurante manquante ou contradictoire avec D19 exige DESIGN_CHANGE_REQUIRED, avec contrats et lots touchés. Elle ne peut pas être ajoutée discrètement aux étapes.

## Commandes de capture proposées

Toutes PROPOSED / NOT RUN par ce document. Shell PowerShell, cwd `C:\dev\whisper`; données : dépôt courant et les cinq chemins exacts; aucune cible Cargo.

```powershell
git rev-parse --show-toplevel
git rev-parse HEAD
git branch --show-current
git status --short
git diff --stat
git diff -- crates/whisper-adapters/src/supervisor.rs crates/whisper-adapters/tests/worker_control.rs crates/whisper-core/tests/compute_policy.rs crates/whisper-worker-cpu/src/native_engine.rs crates/whisper-worker-gpu/src/native_engine.rs
Test-Path -LiteralPath crates/whisper-adapters/src/supervisor.rs
Test-Path -LiteralPath crates/whisper-adapters/tests/worker_control.rs
Test-Path -LiteralPath crates/whisper-core/tests/compute_policy.rs
Test-Path -LiteralPath crates/whisper-worker-gpu/src/native_engine.rs
Get-FileHash -Algorithm SHA256 -LiteralPath crates/whisper-worker-cpu/src/native_engine.rs
```

Attendu lors de la capture initiale : racine exacte, HEAD précité, quatre False et hash CPU précité. Conserver commandes, stdout/stderr/code, date et limites. Une sortie différente ne sera pas écrasée pour obtenir le résultat attendu.

## Vérifications futures et récupération

V-MODES/V-CONTROL, horloge/progrès, Stop/Quitter, saturation/rotation, native UI, inférence CPU/GPU et qualification restent PROPOSED / NOT RUN. Les commandes et scénarios de 01/02/13 restent à exécuter sous un mandat applicable sur le candidat réellement implémenté. Un exit code nul ne démontre pas à lui seul les AC.

Sur refus du contrôle, conserver current, pending, manifests, transports et données; corriger uniquement les entrées attribuables puis refaire le contrôle. Préserver les changements ordinaires du dépôt. Aucun reset global, suppression de source ou nettoyage récursif n’est prescrit.
