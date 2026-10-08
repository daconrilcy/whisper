# DOM-L04-D24-v3 — paquet domaine pour TRANSPORT

**DRAFT, non persisté.** Auteur : `/root/d24_domain_arch`, rôle `rust_domain_architect`, lecture seule. Aucun fichier modifié. Base : D-WHISPER-23 DRAFT, parent D22, digest `cd90c9360820705405dfa870e160c14f2ab273b7f55851a551c878902fe277bb`. Sources : D23/02/03/07/08/50/55/56/58, réponses Q-04/Q-06/Q-09 et DETAIL-P03, constitution et engineering-contract.

## Décisions utilisateur acceptées

- **DEC-L04-CAPTURE-AUTO-01 :** “live capture continues under bounds during GPU→CPU fallback”.
- **DEC-L04-STRICT-FINISH-01 :** “after a forced GPU failure and explicit CPU choice, finish the same passage from preserved PCM without reopening the microphone”.

## Concepts, invariants et transitions

Le domaine possède les invariants de groupe, passage live, import et confirmation. L’application possède scheduler, identité durable, tentative worker, intentions Stop/Quitter et obligations de progrès. Les adaptateurs exécutent capture/persistance/observation ; le worker possède moteur/encodeur ; UI présente et commande ; root assemble.

L’identité durable du passage/archive ou de l’import est distincte de `(génération, instance)` de tentative. Un secours ne crée pas un passage, ne change pas la configuration/source et n’ajoute aucune pause de groupe.

`T` = couverture transcription confirmée ; `A` = PCM contigu durable ; `C` = capture connue. **`0 ≤ T ≤ A ≤ C`.** `T` couvre aussi les plages sans texte ; il n’est pas le dernier mot. DurableAck exige audio, résultats et record validés/durables. `[0,T)` reste immuable ; `[T,A)` est rejouable ; `[A,C)` et buffers non sauvegardés sont exposés à perte. Les valeurs inconnues restent inconnues. L’import rejoue depuis sa source revalidée.

Résultats périmés rejetés avant effet. Transactions de stockage admises réglées par leur identité avant fixation du checkpoint. MP3 pending reconstructible, aucune plage réencodée réappendue en double. Publication vérifiée, pointeur dernier ; aucune atomicité multi-fichiers promise.

**Auto :** GPU actif → panne éligible établie → barrière → ancienne tentative arrêtée/sortie établie et confirmations réglées → génération suivante persistée → CPU attesté → replay depuis T puis nouvelles plages. Capture existante continue sous bornes. Saturation/erreur empêchant conservation ferme l’entrée et préserve Recoverable ; aucune perte silencieuse.

**GPU forcé :** panne → capture fermée → stabilisation → attente choix explicite. CPU choisi → replay PCM conservé → finalisation du même passage, **micro toujours fermé**.

CPU forcé : CPU directement, aucune initialisation GPU. Échec CPU : récupérable/interrompu ; aucune boucle automatique. Stop ferme capture et reste mémorisé pendant la barrière. Quitter bloque nouveaux travaux ; live clôture, import annule/préserve source. Attendre arrêt/stabilisation ; enfant bloqué implique UI interactive ouverte, aucune force. Masquer fenêtre ne modifie pas le travail. Import en attente démarre après clôture stabilisée. Crash Running devient Interrupted, sans reprise automatique.

Backend effectif attesté, motif du secours, retard, état récupérable et pertes connues visibles. Q-04 suit indépendamment chaque obligation ; timer avertit/diagnostique seulement, jamais arrêt/secours/publication. Q-09 borne diagnostics privés ; saturation/pertes/suspension visibles, récupération produit indépendante.

Couverture : REQ-07/08/11/18, UC-07/09/13, AC-07/09/13 révisés ; connexions REQ-03/12/14–17/24/25 maintenues. REQ-26/UC-21/AC-21 proposés pour Q-09. Contrats techniques/migration/isolation pending, revue indépendante et campagnes native/UI/coupures restent à faire. Aucun READY/CLEAN. Les tests produit et qualification restent NOT RUN.
