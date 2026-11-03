Avancée : levée des réserves A demandée le 3 novembre 2026 ; deux corrections de tests explicitement autorisées.
Blocage : vérifications et corrections à réaliser ; le bilan A figé n'est pas réécrit.
Décision : correction de l'attente micro et nouvelle couverture GPU manuel, sans extraction B/C ni N2 ni push.

# REFACTO-POLL-A — complément de levée des réserves

## Périmètre accepté et plan d'exécution

Demande : « fais les actions nécessaires pour lever toutes les réserves ».
Accord complémentaire explicite : corriger uniquement l'attente bornée du test
micro existant pour attendre aussi les compteurs durables ; ajouter un nouveau
test du choix CPU après échec GPU manuel sous `poll/tests/` et son entrée au
registre ; conserver le test GPU existant et ne pas commencer B/C.

Base de reprise : `918cb5fe`, branche `refacto/poll`, worktree `C:/wt/poll`.
Chemins Rust autorisés :
`crates/whisper-adapters/src/worker_ipc/tests/microphone_and_finalization.rs`
(condition de la boucle d'attente uniquement), nouveaux tests sous
`crates/whisper-adapters/src/worker_ipc/runtime/poll/tests/`, déclaration de
module nécessaire dans `poll/tests.rs`, nouvelles entrées uniquement dans
`ci/tests-baseline.txt`. Aucun produit, manifeste, dépendance, ancien test GPU
ou seuil de contrôle n'est modifié. Implémentation déléguée à rust_implementer,
revue indépendante via rust_review_coordinator, deux cycles maximum.

1. **Diagnostic micro minimal.** Une sélection adapters sous la garde sur le
   code d'origine ; lecture du résultat, sans prendre un succès pour une preuve
   de robustesse. Hypothèse étayée : la boucle attend meter/spectrum mais peut
   s'arrêter avant le compteur asynchrone. Budget diagnostic : trois essais ou
   trente minutes, aucun outillage supplémentaire.
2. **Complément PE par reprise.** Worktree propre, une seule activité d'écriture
   et aucun Cargo actif : la session principale peut sélectionner successivement
   en mode détaché la base `9cd15ca6` et les huit commits Na/Nb de A, puis remet
   la branche de reprise. Pour chaque révision : no-run gardé CPU/Parakeet CPU,
   scan de tous leurs harnais avant leurs tests, sorties/codes conservés. Les
   entrées workers/core/manifestes/gardes sont identiques entre ces révisions ;
   la reprise produit une preuve nouvelle datée, pas un ancien contrôle fictif.
   Aucun nouveau worktree, aucune modification de source ou d'historique.
3. **Corrections ciblées.** Ajouter saw_counters à l'attente existante en gardant
   son délai et ses assertions ; nouveau test traversant poll après un Failed
   GPU manuel corrélé, son Stop/sortie worker puis AwaitingCpuChoice, avec états
   et absence de lancement CPU automatique vérifiés. Aucune correction produit.
4. **Validation et revue.** N1 permanente complète sous garde : fmt, Clippy,
   Structure/SelfTest, tests défaut/common/CPU/Parakeet ; PE de tous les harnais
   avant les deux sélections workers. Référence A : 710 réussites/148 ignorés.
   Seuls les nouveaux tests augmentent ce compte ; aucun test n'est perdu.
   Si un échec inattendu existant/produit/préflight apparaît, arrêt et arbitrage.
5. **Clôture.** Consigner résultats, portée de la levée et limites historiques
   ici, archiver les petits journaux avec vérification nombre/SHA-256 ; ajouter
   seulement un lien à ce complément dans le bilan figé ; commit local et arrêt.

Racine des journaux/temporaires : `target/parakeet-tests/refacto-poll-reserves/`.
Garde N1 existante, cible Cargo partagée et cache persistant existants préservés,
offline/locked et environnement isolé ; espace libre vérifié avant campagne.

## Critères de fermeture

- Micro : attente des trois observations, borne inchangée, aucune assertion
  supprimée ; revue de la synchronisation et N1 verte, cause historique distinguée
  de la correction du défaut d'attente observé dans le test.
- GPU manuel : véritable branche d'attente de choix couverte par nouveau test,
  ancien test de préflight inchangé ; aucune extraction de famille 5.
- PE : résultats nouveaux attribués aux neuf révisions, harnais intégralement
  contrôlés avant exécution ; aucune qualification rétroactive des anciens logs.
- Écarts fmt : faits historiques conservés, validations courantes complètes et
  arrêt sur échec appliqué ; aucun effacement de trace ou dérogation permanente.

## Résultats

À compléter par la session principale après les opérations autorisées.
