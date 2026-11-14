# Plan P22 — Découper le test de progression du live micro historique

Avancée : mesure Clippy après P21 à 114 sites; le candidat est une caractérisation live exécutée en N1.
Blocage : aucun; les séquences de polling, fenêtres audio, événements et nettoyage doivent rester identiques.
Décision : isoler le montage et les phases du scénario dans des helpers privés de test.

## Source et périmètre

Le Clippy gardé du 14 novembre après P21 relève `microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end` dans `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs` : 258 lignes et complexité cognitive 28. La mesure initiale au seuil 0 via `guard-level1.ps1 -MeasureCognitiveComplexity` confirme 28/0. Le test monte un fake worker, un runtime live déterministe et une capture micro synthétique, vérifie la progression des compteurs/spectre sous polling borné, puis vérifie texte provisoire, confirmation et fin.

Périmètre de code unique : `crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs`. Pas de changement de production, protocole, harnais, valeurs d’attente, temporisations, scénario, assertions, nettoyage, autre fichier, baseline ou garde. Les résultats P1–P21 restent intacts.

## P22-T1 — Extraire le montage et les boucles de progression

Isoler dans le même fichier des helpers privés, selon les responsabilités visibles dans le scénario :

1. création du request/live runtime et du fake worker, en gardant le même `root`, les mêmes `JobId`/`Generation`, capture, probe, archive, staging, `WorkerPaths`, état actif et transport;
2. première progression des blocs synthétiques et mesure des événements compteurs/mètre/spectre, en conservant exactement 12 blocs, 960 échantillons, quatre polls par bloc et les mêmes assertions de budget;
3. injection du segment et événements worker, puis polling jusqu’au texte/confirmation/End avec la même échéance de trois secondes, blocs silencieux, fréquence de polling et assertions;
4. arrêt de capture, fermeture du transport et suppression du même répertoire au même stade.

Garder dans le test principal une orchestration courte qui lie ces phases. Ne pas cacher les oracles dans un helper générique et ne pas ajouter une nouvelle abstraction publique. Les helpers restent sous 100 lignes; le mode mesure gardé au seuil 0 doit établir un score cognitif individuel ≤20 pour chacun. Le Clippy normal au seuil projet 25 demeure la vérification ratchetée.

## Acceptation et vérifications

- Les deux alertes ciblées disparaissent; aucun diagnostic structurel nouveau; ratchet à 112 sites ou moins contre 114 avant P22.
- N1 exécute et réussit ce test; ses messages `legacy runtime poll failed` et `legacy worker event failed`, contrôles de budget et ordre capture → texte → confirmation → End sont inchangés.
- SelfTest de la garde étendue, mesure cognitive gardée au seuil 0 (scores baseline 28 puis scores finaux de tous les helpers), `fmt --check`, Clippy standard des six membres avec les quatre lints structurels et ratchet, N1 complet sans variable `CARGO_TARGET_DIR` via garde avec modèle personnel validé, sélections workers CPU/Parakeet CPU/common et leurs contrôles PE, et `git diff --check` : PASS.
- Plan commité localement par la session principale; l’implémenteur modifie seulement le fichier autorisé, sans commit de code; revue indépendante obligatoire, au plus deux cycles.

## Gates et arrêt

Worktree `C:/wt/cs`; `LotRoot` gardé `target/parakeet-tests/clippy-corrections-suivantes-20261113`; réutiliser la cible P21 existante `p21-fresh-20261114` via la garde. La mesure seule utilise le mode `-MeasureCognitiveComplexity`, le profil Clippy six crates exact, une configuration temporaire sous run-root et le seuil 0; elle ne modifie pas la baseline et ne remplace jamais le Clippy standard final. Arrêter au premier échec de préflight, format, compilation, Clippy, N1, test worker ou PE; préserver les logs et reprendre uniquement selon l’autorisation d’arbitrage. P22 est la deuxième tâche de la tranche P21–P30; s’arrêter après P30 pour nouvel arbitrage.

## Retour arrière

Si une extraction change l’identité/ordre d’un événement, le nombre de blocs/polls, l’échéance, une assertion, un `Drop`/cleanup, retirer uniquement l’extraction fautive et rapporter l’écart; ne pas réécrire le test pour le faire passer.
