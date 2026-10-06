# Contrats d’intégration — L-WHISPER-02 / P-WHISPER-07

Ce document précise les frontières et preuves de L02. Il conserve les décisions du DESIGN D19 et les bornes actives. Aucun plafond mémoire nouveau, comportement GPU ou changement de garantie produit n’est ajouté.

## Entrées et sorties

| Frontière | Entrée | Sortie / erreur | Owner |
|---|---|---|---|
| Admission FIFO | ImportRequest, identité source, snapshot destination/config | Reçu durable par ID/ordre ou erreur visible ; aucun ACK anticipé | Application → QueueStore |
| Restauration | Journaux/file/pending persistés | Candidats AwaitingChoice/Interrupted ou diagnostic ; aucun démarrage | Recovery → Application |
| Traiter | ID restauré et action explicite | Revalidation source puis admission ordonnée, ou erreur visible | Application |
| Retirer | ID et action explicite | Retrait durable de l’entrée sans suppression de la source/archive | Application → QueueStore |
| Reprise Interrupted | ID, confirmation explicite, identité et checkpoint vérifiés | Nouvelle génération/reprise sans doublon, ou refus visible | Application → adapters |
| Confirmation fragment | Segment admis, identité/génération/plage | Reçu après sync ; préfixe confirmé stable | Journal/ArchiveStore |
| Publication | End et ensemble de reçus cohérents | TXT/SRT/manifeste vérifiés, marqueur Complete publié dernier | ArchiveStore |
| Décodage | Source revalidée et plage bornée | PCM mono 16 kHz, offsets cohérents, ou erreur codec/source | Worker CPU |

## Propriété des chemins

| Chemin/groupe | Responsabilité |
|---|---|
| `core/application.rs` | Commandes/vues de file et reprise ; unique propriétaire admission/scheduler avec états domain existants. |
| `core/ports.rs` | Contrats typés des effets/événements durables, scan, retrait/reprise ; ACK/erreurs attribués à ID/génération. |
| `adapters/queue_store.rs` | File versionnée, ordre stable, écritures/sync, ACK/retrait/restore ; aucune décision de départ. |
| `adapters/journal.rs` | Enregistrements et validation du préfixe confirmé ; reçus après sync. |
| `adapters/recovery.rs` | Scan idempotent de file/pending/générations ; diagnostic et restitution sans lancement. |
| `adapters/archive.rs` | Préparation WAV/MP3 selon Q-07, identité/durée cohérentes, pending, reprise/publication durable. |
| `adapters/worker_ipc.rs` | Exécuter les effets et restituer reçus ; IO/process hors UI, événements corrélés aux générations. |
| `adapters/decoder.rs` | Proxy et validation des réponses/plages ; pas de décodage natif dans le parent. |
| `adapters/lib.rs` | Déclarer les modules d’adaptateurs ajoutés. |
| `worker-cpu/decoder.rs` | Décodeur WAV/MP3 Q-07, profils refusés, gapless applicable, erreurs détectées et PCM borné. |
| `desktop/root.rs` | Assembler ports/adaptateurs/application, sans règle métier. |
| `desktop/ui.rs` | Consumer réel des états, commandes et choix ; pas d’IO bloquante. |

`decode_wav_windows` reste un wrapper compatible au point d’appel courant. Le module application/ports existant conserve les types publics ; aucune modification de `core/lib.rs` ni manifest Cargo n’est prévue dans ce plan. Tout fait contraire constaté au préflight est remonté avant édition.

## TASK-P02 — journal, FIFO et récupération

1. Ajouter admission, restauration, retrait et reprise à application/ports ; réutiliser états/transitions de domain.
2. Implémenter queue_store/journal/recovery et reçus durables.
3. Raccorder archive/worker_ipc ; préserver publication et rejet des réponses périmées.
4. Raccorder root/UI à la façade existante.
5. Vérifier les coupures, scans, choix, FIFO, source intacte et parcours UI.

## TASK-P02-MP3 — import MP3 réel

1. Appliquer Q-07 dans préparation archive et vrai worker CPU.
2. Réconcilier durée préparée/durée décodée et chronologie source ; gapless lorsque les métadonnées utiles existent, sans double retrait.
3. Réutiliser worker CPU/publisher/historique ; source non copiée, non modifiée et jamais supprimée.
4. Vérifier MP3 nominal, refus, corruption détectable, source changée, annulation/crash jusqu’au consumer TXT/SRT.

Un détail technique qui change une frontière, garantie ou invariant retourne à l’architecte ; aucune décision structurante n’est cachée dans le plan. Les limites mémoire restent celles déjà documentées. Le GPU est hors L02 ; l’encodage MP3 live appartient à L03.

## Vérification

`02_VERIFICATION_AND_PREFLIGHT.md` fixe les commandes V-DURABLE, V-SCHEDULER, V-IMPORT-MP3 et V-UI, toutes PRODUCT_VALIDATION / PROPOSED / NOT RUN. Un arrêt de processus ne prouve pas la résistance à toute panne électrique. L02 demeure séquentiel après L01 EXECUTION.
