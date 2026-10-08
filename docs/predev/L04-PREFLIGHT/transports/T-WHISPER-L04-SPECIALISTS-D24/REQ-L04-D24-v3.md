# REQ-L04-D24-v3 — contribution brute

**DRAFT proposé pour archivage TRANSPORT ; aucun fichier écrit.** Auteur : `/root/d23_req_delta`, rôle `requirements analyst` ; owner proposé `AUTH-REQ-D24`. Base : D23 DRAFT, digest `cd90c936082f…`; sources D23/01/02/06/07/08/55/56/58, P28/13/16 et réponses Q-04/Q-06/Q-09.

## Décisions utilisateur vérifiées

Transcript `2026-10-07T17:38:53Z`, appel `call_UKxxAOMwEjfmNhT5qPipA71d` : `SRC-D24-01`, index 0, **« Continuer sous bornes (recommandé) »** pendant fallback live Auto GPU→CPU, SHA `eb74a5c939f13232fcd130f1df5bbdf6af604ea9f4ce68a99d2800cb24509fc4`. `SRC-D24-02`, index 1, **« Oui, finir le même passage »** après choix CPU explicite suivant panne GPU forcé, depuis PCM conservé sans rouvrir micro, SHA `50b7022a2d1968dfaf3b9f109e1dca59c8ae92437ff40af8b5621db894195fac`.

## Mutations REQ/UC/AC

**REQ-07 / UC-07 / AC-07 :** Auto reprend CPU dans le même passage/job, conserve confirmations et offsets, rejette résultats anciens ; capture live continue sous bornes. Avec audio/modèle hashés, injecter erreur GPU après confirmation puis saturation et événement ancien : constater capture/replay, backend/retard visibles, identités uniques, Recoverable exact, Stop prioritaire et MP3/TXT/SRT cohérents. Import : source inchangée.

**REQ-08 / UC-07 / AC-07 :** GPU forcé attend choix CPU après arrêt capture. Injecter panne, attendre puis choisir CPU : aucun CPU avant choix, même passage traité depuis PCM, aucun micro rouvert ni nouvel audio. Stop/Quitter et événements anciens ne relancent rien.

**REQ-11 / UC-09 / AC-09 :** Q-04 suit chaque obligation indépendamment. Horloge 59/60/61 s, silence actif, DurableAck figé et panne explicite : avertissement distinct de panne, aucun arrêt/fallback au seul temps ; Quitter attend confirmation courante avec vraie UI réactive.

**REQ-26 nouvelle, proposée sous revue DESIGN / UC-21 / AC-21 :** Q-09 borne les diagnostics et expose pertes/suspension. Saturation, purge refusée, sentinelles et coupure : vérifier quatre fichiers ≤2 MiB, total ≤8 MiB, règle sept jours, file128×4 KiB, confidentialité, Stop/confirmations indépendants.

**Q-D24-BOUND-01**, détail technique : architecte Rust owner/autorité ; échéance avant gel PLANS/préflight L04 ; REQ-07/AC-07. Options : réutiliser capacités démontrées ou redimensionner avec preuves supplémentaires. Publier charge cible, budgets PCM/spool, métriques/seuils/méthode ; aucune valeur PCM inventée. Les bornes Q-09 concernent diagnostics.

Mesurer erreur→attestation CPU→résultat→rendu, sans plafond inventé. Comparer intervalles captés/durables/confirmés/rejoués ; confirmé conservé, non-confirmé perdable, perte connue ou inconnue visible. Les bornes PCM/files/spool et Q-09 sont distinctes. Risques OPEN : doublons, perte silencieuse, événement ancien, faux diagnostic, saturation capture, fuite/nettoyage, réouverture micro. **PRODUCT_VALIDATION/native/UI : NOT RUN.**
