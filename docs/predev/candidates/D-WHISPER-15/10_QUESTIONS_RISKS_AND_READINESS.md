# Questions, risques et readiness — D-WHISPER-13

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
| Q-08 | STRUCTURING / requirements analyst, utilisateur — démarche ACCEPTED, seuils OPEN | Mesures initiales dans `21` et corpus FR/EN elargi dans `29` ; corpus à élargir et annoter avant proposition de seuils VAD, perte et délai live à l’utilisateur. Aucune valeur n’est acceptée. Affecte `AC-02/08/13` et validation qualité. |
| Q-09 | STRUCTURING / product framer, utilisateur — ACCEPTED pour contenu | Logs techniques locaux (IDs, erreurs, durées) **sans audio ni texte transcrit**. Durée de rétention à choisir comme détail avant lot diagnostics ; aucun envoi cloud. Source : réponse utilisateur du présent chat. |
| Q-10 | STRUCTURING / rust architect puis autorité produit si réduction | Si `SPIKE-01` échoue : autre backend/version/modèle ou réduction du GPU V1 à accepter explicitement ; affects `AC-05/07`, installation et plateforme. L’essai sur PC propre est différé explicitement par `CHANGE-001`, sans réduire la fonction d’installation. |
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
| scope | applicable / GAP en attente de revue D13 | sources `06` et snapshots `13..23`, couverture `07`, `CHANGE-001` ; D4 était PASS documentaire |
| use_cases | applicable / GAP en attente de revue D13 | `UC-01..20` et `AC` dans `07` ; D4 était PASS documentaire, variantes Q ouvertes |
| errors | applicable / GAP en attente de revue D13 | tables `07/08` ; D4 était PASS documentaire, essais critiques restants |
| domain | applicable / GAP en attente de revue D13 | états/invariants `08` ; D4 était PASS documentaire |
| dependencies | applicable / GAP en attente de revue D13 | matrice `08` ; D4 était PASS documentaire, contrôles post-code NOT RUN |
| persistence | applicable / GAP | protocole proposé `08` ; `SPIKE-02` partiel |
| concurrency | applicable / GAP | ownership/arrêt `08` ; bornes réelles et Q-06 ouvertes |
| platform | applicable / GAP | sources `09`, cible `01`, `SPIKE-01` partiel |
| acceptance | applicable / GAP | `AC-01..20` futurs ; Q-07/08 et méthodes à figer |
| risks | applicable / GAP | R-01..06 ; preuves critiques partielles |
| railguard | applicable / GAP en attente de revue D13 | proposition `11`, D4 était PASS documentaire ; activation ultérieure |

## Gate et reprise

R-WHISPER-DESIGN-12 maintient 001 ferme et 008 ouvert. D-WHISPER-13 ajoute DEC-30/31 et des annotations automatiques provisoires, sans CLEAN presume. Prochain acteur : reviewer sur le candidat manifeste ; les essais restants et decisions de qualite demeurent ouverts. Aucun plan writer ni produit Rust ne sont autorises par ce DRAFT.

Tests du produit : NOT RUN. `SPIKE-01/02` : essais partiels autorisés, voir `09` et `evidence/`. L'intégrité du manifeste n'est pas preuve de qualité. La session actuelle ne démontre pas une isolation native des agents ; ne pas transférer la qualification d'une autre topologie.

## Décisions de reprise

DEC-26..28 sont dans `32_USER_DECISIONS.md`. La portée du finding 008 et le travail restant sont dans `33_CLOSURE_MATRIX_008.md`. Le retard CPU doit être visible sans garantie temps réel ; les seuils V1 et la validation du corpus cible restent OPEN. Les rubriques déjà jugées PASS par D9 ne sont pas présumées PASS pour D11 avant revue.

DEC-29 et ses limites sont dans `38_PROVISIONAL_THRESHOLDS.md`. Les deux PASS de SPIKE ne changent pas les GAP de persistance, concurrence, plateforme, acceptation et risques de 008.

DEC-30/31 et la méthode automatique sont dans `39_AUTO_ANNOTATION.md` ; Q-08 reste ouvert jusqu’à vérification humaine des intervalles et choix du seuil VAD.

## Checkpoint D14

DEC-32 maintient DESIGN DRAFT et demande E1/E2/E3 complet/E4/E5/E6 selon `41`. E0 et le smoke E3 de `42` sont des preuves partielles, pas un PASS de conception ni un test produit. Q-08/Q-10 et le finding 008 restent ouverts avant DESIGN READY.

## Checkpoint D15

La correction de provenance 014 est proposée dans `43` et `45`, en attente de revue indépendante du candidat D15. Le finding technique 008 reste ouvert. DESIGN demeure DRAFT ; aucun plan ni code produit autorisé.
