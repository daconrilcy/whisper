Avancée : corrections autorisées revues CLEAN ; neuf reprises PE réussies ; N1 finale 711 réussites / 148 ignorés / zéro échec.
Blocage : aucun constat courant ; l'attribution causale de l'incident micro ancien reste non établie.
Décision : clôture de la levée technique des réserves A ; B, C, N2 et push non commencés.

# REFACTO-POLL-A — complément du 3 novembre 2026

[Bilan historique figé](REFACTO-POLL-A-BILAN.md) : `918cb5fe`.
Plan accepté : `5bd997b8` ; correctifs : `f58e39c0`, branche `refacto/poll`,
worktree `C:/wt/poll`. Les anciennes constatations restent conservées.

| Réserve | Action et résultat |
| --- | --- |
| Attente micro | Boucle bornée à 2 s attendant désormais compteurs, niveau et spectre ; délai et assertions inchangés. Défaut d'attente corrigé, test vert, revue CLEAN. |
| Choix CPU après échec GPU manuel | [Nouveau test](../../crates/whisper-adapters/src/worker_ipc/runtime/poll/tests/manual_gpu_choice.rs) : candidat valide, Failed corrélé du faux worker, Stop/Stopped et sortie, AwaitingCpuChoice, requête conservée, aucun CPU automatique. Ancien test GPU de préflight intact. |
| Preuves PE partielles | Reprises datées sur les neuf révisions ci-dessous : tous les harnais contrôlés avant leurs tests, zéro échec. Couverture complétée sans reconstituer les anciens contrôles. |
| Relances fmt antérieures | Écart historique consigné ; contrôles courants complets et correction relue. Aucune trace effacée, aucune dérogation permanente ajoutée. |

**Reprises PE.** Base `9cd15ca6`, puis `69fda611`, `c0456006`, `6ff324fc`,
`2c90586e`, `df2fefb9`, `360ac380`, `69809a26`, `804362de`. Pour chacune :
no-run gardé, scans des quatre harnais CPU avant 12 réussites/9 ignorés ;
no-run puis scans des cinq harnais Parakeet avant 26 réussites/11 ignorés.
Total : **81 scans PASS, 342 réussites workers / 180 ignorés**. Révisions,
commandes, codes et journaux figurent dans `replay-<révision>.stdout.log`.
Sources workers/core/manifestes/gardes identiques sur l'intervalle ; retour
sur la branche sans changement d'historique avant les corrections.

**Validation actuelle.** Défaut 662/127 ignorés, common 11/1, CPU 12/9,
Parakeet CPU 26/11 : **711 réussites / 148 ignorés**, zéro échec. Fmt, Clippy,
Structure 907/907, SelfTest 33/64, fixtures PE 4/15 et neuf harnais : PASS.
Revue indépendante CLEAN, aucun constat ouvert. Aucun code produit modifié ;
une seule condition du test micro et un nouveau test GPU, plus raccord/registre.

**Portée.** Le diagnostic initial adapters a passé (398 réussites/124 ignorés).
Il ne prouve pas la cause de l'ancien échec. La correction supprime le défaut
d'attente identifié, sans attribuer rétroactivement cet incident à ce défaut.
Les reprises PE sont de nouvelles validations. Aucun matériel GPU, modèle
natif ou fenêtre Windows qualifié ici ; témoin PE réel facultatif non exécuté.

**Rétention.** [Journaux](../../target/parakeet-tests/refacto-poll-reserves/archive/REFACTO-POLL-A-reserves-20261103.zip) :
261 fichiers, arborescence conservée, nombre et SHA-256 de chaque entrée
vérifiés ; archive 357 709 octets. Aucune suppression de clôture ; caches
partagés/persistant préservés, 620,77 Gio libres après archivage.
Ce complément est figé à son commit de clôture.
