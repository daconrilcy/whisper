Avancée : revue indépendante et préconditions vérifiées en lecture seule sur 927c8809.
Blocage : banc, références de comparaison et périmètre du diff incompatibles avec le plan.
Décision : arrêt avant N2, sans correction ni fusion ; bilan figé le 4 novembre 2026.

# POLL-INTEGRATION — arrêt au préflight

Worktree `C:/wt/poll`, branche `refacto/poll`, candidat initial `927c8809`.
Revue indépendante : **FINDINGS**, budget zéro correction respecté ; aucun
code, test, garde ou ancien bilan modifié. Résultats C conservés.

**Auto : six exécutions prévues, aucune lancée.**

| Production | Mesure 1 | Mesure 2 | Mesure 3 | Moyenne / phase dominante |
| --- | --- | --- | --- | --- |
| Base 6316a54b | NON EXÉCUTÉ | NON EXÉCUTÉ | NON EXÉCUTÉ | Non mesurées |
| Candidat 927c8809 | NON EXÉCUTÉ | NON EXÉCUTÉ | NON EXÉCUTÉ | Non mesurées |

**Matrice B réellement disponible : cinq scénarios, sept exécutions historiques**
(deux références pré-B puis cinq validations B), pas sept scénarios distincts.

| Scénario | Pré-B 00c2d87e ou historique | B | Tête |
| --- | ---: | ---: | --- |
| Reprise interrompue | 13,21 s, pré-B | 13,35 s | NON EXÉCUTÉ |
| Direct Turbo CPU | 42,44 s, pré-B | 42,54 s | NON EXÉCUTÉ |
| Import long FR | 175,79 s, historique | 168,57 s | NON EXÉCUTÉ |
| Import long EN | 195,44 s, historique | 187,17 s | NON EXÉCUTÉ |
| Auto sans candidat GPU | 26,14 s, historique | 41,55 s | NON EXÉCUTÉ |

**Écarts établis et arbitrage nécessaire :**
- **Banc Auto :** candidat GPU=None, sans inventaire GPU ; cinq phases non
  disponibles dans les journaux, observateur cfg(test) inaccessible au test
  d'intégration. Choisir des traces suffisantes ou un périmètre distinct.
- **Comparaison :** préciser les cas et la référence décisionnelle. Appliquer
  ±10 % aux deux valeurs Auto impose des plages disjointes : 23,526–28,754 s
  et 37,395–45,705 s. Un écart contemporain ≤10 % n'établirait pas l'origine
  environnementale de l'ancien écart.
- **Diff classé :** extraction poll et helpers cfg(test) dans le périmètre ;
  modifications de guard-level1.ps1, native-profiles.json, tests-baseline.txt,
  live_archive.rs, cadence/diagnostics micro et documents hors liste stricte.
  Changements historiquement autorisés, à reconnaître explicitement.
- **Propreté :** models/ et sound example/ non suivis dans poll ; mêmes
  ressources et deux documents non suivis dans main, préservés. Références
  locales main=origin/main=9cd15ca6, ancêtre du candidat ; serveur et autres
  préconditions finales non vérifiés.

**Fusion et N1 main : NON EXÉCUTÉES.** Aucun commit de fusion ni tag
avant-fusion-poll créé, aucun push. N1 C historique 754/149/0 ne vaut pas
validation main. Reprise conditionnée à un plan réarbitré.

Sources : [bilan B](REFACTO-POLL-B-BILAN.md), [plan B](REFACTO-POLL-B-PLAN.md),
[bilan C](REFACTO-POLL-C-BILAN.md),
[test Auto](../../crates/whisper-adapters/tests/parakeet_unified_native.rs#L988),
[observateur natif](../../crates/whisper-adapters/src/worker_ipc/parakeet_unified/native.rs#L69).

[Reprise après arbitrage : mesures Auto et arrêt au seuil de performance](POLL-INTEGRATION-REPRISE-BILAN.md).
