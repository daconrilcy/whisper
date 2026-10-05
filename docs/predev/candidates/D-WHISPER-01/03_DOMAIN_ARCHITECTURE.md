# Domaine et architecture — proposition

## Parcours

Live demandé → vérifier qu’aucun live/import actif ne bloque → capturer microphone → afficher texte progressif/mesurer délai → arrêter/finaliser. Import demandé durant live reste en file. Live demandé pendant import actif est refusé avec invitation à réessayer. Fermer fenêtre masque l’app sans interrompre le travail. Quitter en live arrête, finalise puis quitte ; si finalisation échoue, préserver récupérable, signaler puis quitter. Quitter en import actif annule proprement et préserve la source.

Après crash ou Quitter, les imports non commencés sont persistés. Au prochain démarrage, signaler la file et attendre choix Traiter ou Retirer. Retirer enlève seulement l’entrée. Un import commencé interrompu par crash est distingué et ne redémarre pas silencieusement ; sa réconciliation reste à définir.

## Concepts et états candidats

Concepts : `LiveSession`, `ImportRequest`, file opérationnelle durable, `TranscriptionJob`, révision progressive de segment, intervalles VAD, mode de calcul effectif, jeu d’archives MP3/TXT/SRT. États visibles candidats : `Idle`, `LiveCapture`, `Finalizing`, `ImportQueued`, `ImportRunning`, `Cancelling`, `InterruptedRecoverable`, `RestoredQueueAwaitingChoice`, `Completed`, `FailedRecoverable`, `Failed`.

## Invariants

1. Une capture live au maximum ; import pendant live démarre après lui.
2. Live demandé pendant import actif n’est pas différé.
3. Masquer fenêtre ne change pas l’état du travail.
4. File restaurée n’exécute rien avant le choix utilisateur ; retirer ne supprime pas la source.
5. Audio archivé conserve silences ; VAD exclut silences du compteur parlé ; SRT utilise axe original.
6. Seuls les fragments confirmés sont garantis ; exposition du fragment ouvert et buffers à mesurer.
7. Archive incomplète n’est pas présentée comme terminée.
8. GPU forcé en échec ne bascule pas seul ; Auto peut basculer CPU avec signalement et déduplication.

## Couches obligatoires Rust

La constitution centrale impose responsabilités distinctes : `domain` (invariants purs), `application` (orchestration et ports), `adapters` (OS, micro, codecs, moteur, stockage), `ui` (présentation via API application), composition root (assemblage). Domain n’importe pas OS/IO/UI ; application n’importe pas adaptateurs concrets ; UI ne contourne pas application. Crates/modules physiques restent à justifier et les dépendances devront être vérifiées par Cargo/features ou visibilité/imports/revue.

## Exécution candidate

Inference et IO bloquants hors thread UI. Worker moteur supervisé séparément proposé pour isoler pannes natives/GPU. IPC versionné porte job/session/génération/séquence/offsets/erreurs ; canal d’arrêt indépendant ; résultats obsolètes rejetés. Files bornées et rétropression explicite avant perte audio silencieuse. Fragments audio ordonnés confirmés après fermeture/synchronisation. File durable séparée de l’historique. Préparer trois sorties, contrôler leur cohérence puis publier ensemble. VAD produit intervalles sur axe original sans filtrer les échantillons archivés.

Auto peut reprendre CPU au dernier intervalle confirmé avec déduplication ; GPU forcé échoué attend choix. Prouver le GPU réellement actif, pas seulement l’option. Encapsuler unsafe dans l’adaptateur natif ; aucun partage ad hoc de contexte via Send/Sync. Pour chaque port préciser ownership, erreurs, annulation, bornes, durabilité et contraintes de thread.

Architecture proposée issue du cadrage conversationnel, non revue et sans autorisation de coder. Reste à déterminer : annulation/IO bloqué, protocole worker, atomicité réelle, import actif après crash et limites de finalisation.
