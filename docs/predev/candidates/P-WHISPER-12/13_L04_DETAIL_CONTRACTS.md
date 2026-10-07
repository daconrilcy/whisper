# Contrats détaillés L04 — P-WHISPER-11

## Autorité et identité

Base DESIGN D-WHISPER-19 (`903926ba27c70fc3a0cc695d4e9e50914a64993e40c339683e557bc4c604d7529`), PLANS P-WHISPER-10 (`9b9edfcf09e39bd6017fb818fd0b9784923b8eb89707b18a9e3a1fff4d7b8f4a`). Réponse attribuée à `/root/q04_q06_q09_arch` (`rust_architect`, contribution `ARCH-Q04-Q06-Q09-v1`) et transcrite en `sources/architecture/ARCH-Q04-Q06-Q09-v1.md`; sources utilisateur sous `sources/authorization/`. Transport : `P10-HOST-WORK/transports/T-WHISPER-Q04-Q06-Q09-ANSWER-01/transport-manifest.json`.

Ce document est une précision de `reversible_detail`, sans changement de périmètre, d’invariant accepté, de garantie ni de frontière. Les textes source P10/D19 ne sont pas édités. L’état actif ne change qu’après revue P11 indépendante/CLEAN et promotion explicite. L’implémentation reste séparément autorisée ; tous les essais ci-dessous sont NOT RUN.

## Q-04 — supervision du progrès

- Suivre une progression monotone, propre à chaque étape et obligation attendue ; la présence du processus et un heartbeat seuls ne constituent pas une progression.
- Aucun progrès en attente pour `Idle`, `Queued`, `AwaitingChoice` et les états terminaux stables. Le silence de capture est normal. Le progrès de capture ne doit pas masquer un ACK durable de journal manquant.
- Import : source décodée, segments terminés, reçu durable. Inférence : plage soumise puis terminée. `Draining`, `Finalizing` et `Quitting` : obligations finies encore ouvertes et reçus.
- Sonder au plus toutes les 1 s ; avertir à 60 s sans progrès attendu sur une obligation ouverte ; rafraîchir un diagnostic inchangé au plus toutes les 30 s et immédiatement sur une erreur ou transition.
- Le minuteur seul n’arrête ni ne tue le worker, ne déclenche pas un fallback, et ne publie pas un résultat. Seule une panne observée séparément peut activer le comportement conçu pour cette panne.

Tests futurs à couvrir dans les chemins/tests L04 existants : silence normal, longue inférence avec jalons, worker figé, heartbeat présent sans ACK, progression d’une obligation qui ne masque pas le blocage d’une autre, rafraîchissement temporel/transition. Assertions d’absence d’arrêt, de kill, de fallback et de faux résultat sous timer seul.

## Q-06 — Quitter sans confirmation de l’enfant

Quit est latched/cooperatif. Tant que l’enfant n’a pas confirmé `Stopped`, garder l’application ouverte et réactive avec attente ou diagnostic visible. Ne jamais afficher `Complete` sans reçu de fin. Pas de terminaison forcée et pas d’envoi concurrent répété de Stop. Préserver la portion confirmée durablement et récupérable.

Ce comportement reprend la décision utilisateur acceptée dans D19 et la résolution DETAIL-P03. L’action manuelle et le diagnostic technique précis ne doivent pas simuler un arrêt confirmé ; aucune terminaison forcée nouvelle n’est ajoutée. Vérifier l’interface réellement rendue ainsi que le worker bloqué/non acquittant dans la campagne runtime future.

## Q-09 — rotation des logs diagnostiques

L’utilisateur a accepté l’option « Rotation bornée (Recommended) » sur le paquet proposé. Configuration : maximum 4 fichiers de 2 MiB chacun, fichier actif compris (8 MiB au total) ; âge maximal 7 jours ; supprimer oldest-first ; aucun minimum de rétention. Ne traiter que les fichiers connus dans un répertoire diagnostic dédié. Sources importées, archives audio, textes de session et données de reprise sont exclus de toute rotation/suppression.

Contenu diagnostique local limité à IDs, erreurs filtrées et durées ; aucune audio, transcription, chemin source libre, chaîne native non filtrée, réseau ou cloud. Une file best-effort ne contient pas plus de 128 enregistrements de 4 KiB chacun. La saturation signale la perte de logs, ne bloque pas Stop et ne change pas les acquittements du journal durable. La proposition et l’acceptation de l’option sont capturées sous les sources P11 indiquées.

Vérifier dans les essais L04/L05 futurs : rotation ancienneté/volume, préservation des fichiers protégés, limite maximale, noms inconnus ignorés, contenu filtré, file pleine et continuité de Stop/ACK. Tous ces essais restent NOT RUN.

## Limites

Cette réponse ferme les questions de conception, pas les validations. Pas de test/build/code/UI réelle/essai disque/journal exécuté dans ce paquet. Pas d’autorisation de démarrer L04 ou L05.
