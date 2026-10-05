# Questions, risques et readiness — D-WHISPER-03

## Questions à trancher

`STRUCTURING` bloque les contrats ou la preuve indiqués ; `DETAIL` est réversible au lot désigné et ne suspend pas les autres sujets. Aucune réponse n'est imputée au silence. Échéance : avant revue finale DESIGN pour les STRUCTURING, avant préflight du lot mentionné pour les DETAIL. Les lots n'existent pas encore ; le responsable indiqué n'est pas une autorisation d'implémenter.

| ID | Classe / owner et autorité | Options et conséquence ; dépendance |
|---|---|---|
| Q-01 | STRUCTURING / product framer, utilisateur — ACCEPTED | Plusieurs imports en **ordre de demande**, un par un après finalisation live. Source : réponse utilisateur du présent chat du 2026-10-05. `UC-03/12`, scheduler et recovery. |
| Q-02 | STRUCTURING / product framer, utilisateur — ACCEPTED en principe | Import commencé après crash : **confirmation avant reprise**. Le point exact de reprise (début ou checkpoint vérifié) est détail technique à prouver, sans exécution automatique. Source : réponse utilisateur du présent chat. |
| Q-03 | DETAIL / requirements analyst, règle produit déjà acceptée | Raccourci durant Preparing/Finalizing/Import : refuser l'action avec état visible ; **aucun live mis en file pendant import** conformément à REQ-04. Le texte précis du message reste détail UI. Affecte `AC-16` et lot UI/OS seulement. |
| Q-04 | DETAIL / requirements analyst | L’utilisateur a accepté un avertissement vers 60 s sans progrès, diagnostic et arrêt seulement si panne avérée (`06`). Définir avant lot supervision les signaux de progrès par état et une marge de temporisation ; aucune interprétation en arrêt automatique au seul temps. Affecte `AC-09`. |
| Q-05 | STRUCTURING / product framer, utilisateur — ACCEPTED pour suppression | Supprimer l'historique **efface MP3/TXT/SRT après confirmation**, jamais la source importée. Changement de dossier pendant job : snapshot du chemin actuel proposé, à confirmer comme détail avant lot stockage. Source : réponse utilisateur du présent chat. |
| Q-06 | STRUCTURING / product framer + rust architect, utilisateur — ACCEPTED pour UX | Quitter si worker/encodeur bloqué : **rester ouvert jusqu'à résolution ou action manuelle**, signaler l'état et préserver le récupérable. L'action manuelle exacte et le diagnostic technique sont à concevoir/mesurer. Source : réponse utilisateur du présent chat. |
| Q-07 | DETAIL / requirements analyst + rust architect | Deux profils MP3 et un WAV lus en SPIKE-02 (`19`) ; compléter les profils courants, l’encodeur MP3 de sortie et les rejets avant lot codecs/validation `AC-03/10`. Compatibilité maximale souhaitée, sans codec/bitrate imposé. |
| Q-08 | STRUCTURING / requirements analyst, utilisateur — démarche ACCEPTED, seuils OPEN | Mesurer VAD, perte du fragment non confirmé et délai du texte live sur corpus, puis proposer les seuils à l’utilisateur pour décision. Aucune valeur n’est acceptée. Affecte `AC-02/08/13` et validation qualité. |
| Q-09 | STRUCTURING / product framer, utilisateur — ACCEPTED pour contenu | Logs techniques locaux (IDs, erreurs, durées) **sans audio ni texte transcrit**. Durée de rétention à choisir comme détail avant lot diagnostics ; aucun envoi cloud. Source : réponse utilisateur du présent chat. |
| Q-10 | STRUCTURING / rust architect puis autorité produit si réduction | Si `SPIKE-01` échoue : autre backend/version/modèle ou réduction du GPU V1 à accepter explicitement ; affects `AC-05/07`, installation et plateforme. |
| Q-11 | STRUCTURING / product framer, utilisateur — ACCEPTED | Stop + Reprise : **un dossier, un MP3 par passage, TXT/SRT cumulés** ; chaque nouveau passage démarre sur impulsion utilisateur. Source : réponse du présent chat du 2026-10-05. Affecte `AC-20`, journal et marqueur Complete. |

## Registre des risques

| ID / owner | Risque et preuve nécessaire | Statut |
|---|---|---|
| R-01 / rust architect | CUDA/GTX 1080 Ti, crate native, modèle et installateur combinés peuvent échouer ; `SPIKE-01` avec versions/hashes/logs. | OPEN, critique, essais partiels dans `09` |
| R-02 / domain architect + rust architect | Perte/faux Complete/corruption après coupure et changement dossier ; `SPIKE-02` + `AC-10/12/13/17`. | OPEN, critique, essais partiels dans `09` |
| R-03 / domain architect | Concurrence live/import, file et annulation peuvent créer doublon ou perte ; transitions `08` + Q-01/02/06 et tests futurs. | OPEN |
| R-04 / rust architect | Fallback et texte progressif peuvent dupliquer ou déplacer offsets ; identité/offsets `08`, essais CPU/GPU `SPIKE-01`. | OPEN |
| R-05 / requirements analyst | VAD peut compter les silences de travers ; corpus et seuil Q-08, `AC-08`. | OPEN |
| R-06 / rust architect | Tray, autostart, hotkey, ressources OS ou logs locaux peuvent échouer ; `AC-14/15/16`, Q-09. | OPEN |

## État des rubriques DESIGN

| Rubrique | Applicabilité / état | Preuve et reste |
|---|---|---|
| scope | applicable / GAP | sources `06` et snapshots `13..18`, couverture `07` ; revue D3 encore nécessaire |
| use_cases | applicable / GAP | `UC-01..18` et `AC` dans `07` ; variantes Q à résoudre |
| errors | applicable / GAP | tables `07/08` ; critères de diagnostic et arrêt Q-04/06 |
| domain | applicable / GAP | états/invariants `08` ; points techniques de recovery à prouver |
| dependencies | applicable / GAP | matrice `08` ; contrôles post-code NOT RUN, ports techniques SPIKE |
| persistence | applicable / GAP | protocole proposé `08` ; `SPIKE-02` partiel |
| concurrency | applicable / GAP | ownership/arrêt `08` ; bornes réelles et Q-06 ouvertes |
| platform | applicable / GAP | sources `09`, cible `01`, `SPIKE-01` partiel |
| acceptance | applicable / GAP | `AC-01..20` futurs ; Q-07/08 et méthodes à figer |
| risks | applicable / GAP | R-01..06 ; preuves critiques partielles |
| railguard | applicable / GAP | proposition `11`, revue/autorité d'activation en attente |

## Gate et reprise

Le candidat D-WHISPER-01 a reçu `R-WHISPER-DESIGN-01` FINDINGS, dix REQUIRED ouverts, digest `049feb9aea7564b1aeba219de992f7f0aabf902d47f4a11d7102e47547963712`. La relecture D-WHISPER-02 `R-WHISPER-DESIGN-02` (digest `b8e236ecc53691dc3cbc5e60b1de99cc07fa3a488d89eae1d4de7a350ffe144d`) a fermé 003, 005, 006, 009 ; 001, 002, 004, 007, 008, 010 et le nouveau 011 restent ouverts. Les résultats bruts sont copiés dans `18_REVIEW_RAW.json`, mais seul le reviewer peut fermer les IDs sur un nouveau candidat. D-WHISPER-03 propose les corrections 001/002/004/007/010/011, sans CLEAN présumé ; 008 dispose de preuves partielles des SPIKE autorisés. Prochain acteur : reviewer sur le candidat manifesté ; les essais restants et décisions qualité demeurent ouverts. Aucun plan writer ni produit Rust n'est autorisé par ce DRAFT.

Tests du produit : NOT RUN. `SPIKE-01/02` : essais partiels autorisés, voir `09` et `evidence/`. L'intégrité du manifeste n'est pas preuve de qualité. La session actuelle ne démontre pas une isolation native des agents ; ne pas transférer la qualification d'une autre topologie.
