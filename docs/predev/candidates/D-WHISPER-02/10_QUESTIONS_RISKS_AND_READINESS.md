# Questions, risques et readiness — D-WHISPER-02

## Questions à trancher

`STRUCTURING` bloque les contrats ou la preuve indiqués ; `DETAIL` est réversible au lot désigné et ne suspend pas les autres sujets. Aucune réponse n'est imputée au silence. Échéance : avant revue finale DESIGN pour les STRUCTURING, avant préflight du lot mentionné pour les DETAIL. Les lots n'existent pas encore ; le responsable indiqué n'est pas une autorisation d'implémenter.

| ID | Classe / owner et autorité | Options et conséquence ; dépendance |
|---|---|---|
| Q-01 | STRUCTURING / product framer, utilisateur — ACCEPTED | Plusieurs imports en **ordre de demande**, un par un après finalisation live. Source : réponse utilisateur du présent chat du 2026-10-05. `UC-03/12`, scheduler et recovery. |
| Q-02 | STRUCTURING / product framer, utilisateur — ACCEPTED en principe | Import commencé après crash : **confirmation avant reprise**. Le point exact de reprise (début ou checkpoint vérifié) est détail technique à prouver, sans exécution automatique. Source : réponse utilisateur du présent chat. |
| Q-03 | DETAIL / requirements analyst, utilisateur ou règle produit acceptée | Raccourci durant Preparing/Finalizing/Import : ignorer avec état visible, ou mettre en file une action. Affecte `AC-16` et lot UI/OS seulement. |
| Q-04 | STRUCTURING / requirements analyst, utilisateur | Environ 60 s sans progrès : fenêtre indicative configurable et critères de progrès par état, ou seuil fixe ; diagnostic doit distinguer silence, calcul long et panne avérée. Affecte `AC-09`/supervision. Aucun arrêt automatique au temps seul n'est accepté. |
| Q-05 | STRUCTURING / product framer, utilisateur — ACCEPTED pour suppression | Supprimer l'historique **efface MP3/TXT/SRT après confirmation**, jamais la source importée. Changement de dossier pendant job : snapshot du chemin actuel proposé, à confirmer comme détail avant lot stockage. Source : réponse utilisateur du présent chat. |
| Q-06 | STRUCTURING / product framer + rust architect, utilisateur — ACCEPTED pour UX | Quitter si worker/encodeur bloqué : **rester ouvert jusqu'à résolution ou action manuelle**, signaler l'état et préserver le récupérable. L'action manuelle exacte et le diagnostic technique sont à concevoir/mesurer. Source : réponse utilisateur du présent chat. |
| Q-07 | DETAIL / requirements analyst + rust architect | Profils WAV/MP3 courants acceptés, format MP3 de sortie et échantillons rejetés ; matrice à figer avant lot codecs et validation `AC-03/10`. Compatibilité maximale souhaitée par l'utilisateur, sans codec/bitrate imposé. |
| Q-08 | STRUCTURING / requirements analyst, utilisateur sur risque qualité | Corpus et seuil d'erreur VAD/qualité, ainsi que méthode de mesure de la perte non confirmée ; sans seuil accepté, produire des mesures et obtenir décision avant exigence de qualité. Affecte `AC-02/08/13`, `SPIKE-02`. |
| Q-09 | STRUCTURING / product framer, utilisateur — ACCEPTED pour contenu | Logs techniques locaux (IDs, erreurs, durées) **sans audio ni texte transcrit**. Durée de rétention à choisir comme détail avant lot diagnostics ; aucun envoi cloud. Source : réponse utilisateur du présent chat. |
| Q-10 | STRUCTURING / rust architect puis autorité produit si réduction | Si `SPIKE-01` échoue : autre backend/version/modèle ou réduction du GPU V1 à accepter explicitement ; affects `AC-05/07`, installation et plateforme. |

## Registre des risques

| ID / owner | Risque et preuve nécessaire | Statut |
|---|---|---|
| R-01 / rust architect | CUDA/GTX 1080 Ti, crate native, modèle et installateur combinés peuvent échouer ; `SPIKE-01` avec versions/hashes/logs. | OPEN, critique, NOT RUN |
| R-02 / domain architect + rust architect | Perte/faux Complete/corruption après coupure et changement dossier ; `SPIKE-02` + `AC-10/12/13/17`. | OPEN, critique, NOT RUN |
| R-03 / domain architect | Concurrence live/import, file et annulation peuvent créer doublon ou perte ; transitions `08` + Q-01/02/06 et tests futurs. | OPEN |
| R-04 / rust architect | Fallback et texte progressif peuvent dupliquer ou déplacer offsets ; identité/offsets `08`, essais CPU/GPU `SPIKE-01`. | OPEN |
| R-05 / requirements analyst | VAD peut compter les silences de travers ; corpus et seuil Q-08, `AC-08`. | OPEN |
| R-06 / rust architect | Tray, autostart, hotkey, ressources OS ou logs locaux peuvent échouer ; `AC-14/15/16`, Q-09. | OPEN |

## État des rubriques DESIGN

| Rubrique | Applicabilité / état | Preuve et reste |
|---|---|---|
| scope | applicable / GAP | sources `06`, couverture `07` ; décisions produit récentes intégrées, source complète à figer dans le manifeste |
| use_cases | applicable / GAP | `UC-01..18` et `AC` dans `07` ; variantes Q à résoudre |
| errors | applicable / GAP | tables `07/08` ; critères de diagnostic et arrêt Q-04/06 |
| domain | applicable / GAP | états/invariants `08` ; points techniques de recovery à prouver |
| dependencies | applicable / GAP | matrice `08` ; contrôles post-code NOT RUN, ports techniques SPIKE |
| persistence | applicable / GAP | protocole proposé `08` ; `SPIKE-02` NOT RUN |
| concurrency | applicable / GAP | ownership/arrêt `08` ; bornes réelles et Q-06 ouvertes |
| platform | applicable / GAP | sources `09`, cible `01`, `SPIKE-01` NOT RUN |
| acceptance | applicable / GAP | `AC-01..18` futurs ; Q-07/08 et méthodes à figer |
| risks | applicable / GAP | R-01..06 ; preuves critiques NOT RUN |
| railguard | applicable / GAP | proposition `11`, revue/autorité d'activation en attente |

## Gate et reprise

Le candidat D-WHISPER-01 a reçu le verdict brut `R-WHISPER-DESIGN-01` FINDINGS, dix REQUIRED ouverts, pour digest `049feb9aea7564b1aeba219de992f7f0aabf902d47f4a11d7102e47547963712` dans le présent chat. Seul le reviewer indépendant peut fermer ces IDs. Les contenus D-WHISPER-02 sont des corrections proposées, sans CLEAN présumé. Le manifeste D-WHISPER-02, ses sources, ce registre et le nouveau rapport de revue doivent être liés avant toute promotion. Prochain acteur : reviewer sur le candidat persisté exact, puis auteurs des findings restants et propriétaire des décisions Q. Aucun plan writer, produit Rust ou publication n'est autorisé par ce DRAFT.

Tests du produit : NOT RUN. `SPIKE-01/02` : NOT RUN. L'intégrité du manifeste n'est pas preuve de qualité. La session actuelle ne démontre pas une isolation native des agents ; ne pas transférer la qualification d'une autre topologie.
