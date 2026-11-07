Avancée : nettoyage exécuté par l'utilisateur ; absence des 457 dossiers vérifiée le 7 novembre 2026 à 21 h 53.
Blocage : aucun ; preuves, livrables, ressources installées, caches partagés et exclusions conservés.
Décision : nettoyage clôturé sur le périmètre approuvé, avec compte rendu local ; aucun push ni suppression supplémentaire.

# Refacto-poll — bilan figé du nettoyage

L'utilisateur a autorisé la suppression des **457 dossiers recensés** après les contrôles prévus, avec conservation des preuves, livrables et caches. Après le refus de l'outil décrit dans la [remise](nettoyage-refacto-poll-execution.md), il a exécuté la commande dans PowerShell avec les deux indicateurs activés. Son [retour de terminal](raw/nettoyage-refacto-poll-console.txt) affiche les 457 entrées, termine par la mesure du volume et ne contient aucune erreur. L'agent n'a exécuté aucune commande de suppression après ce refus.

**Périmètre constaté.** Les chemins R001 à R457 de la [liste exacte approuvée](nettoyage-refacto-poll-dossiers.csv) sont tous absents : **457/457, aucun restant**. Leur taille logique inventoriée était de **395 913 712 885 octets (368,723 Gio)**, dont les copies et audios du [manifeste approuvé](nettoyage-refacto-poll-copies.csv). La colonne historique `DeletionExecuted=NON` de l'inventaire décrit la préparation ; le présent bilan constate l'exécution humaine ultérieure. Les racines de campagne, leurs parents et les exclusions ne sont pas supprimés.

| Espace libre sur C: | Octets | Gio |
|---|---:|---:|
| Avant, contrôle de 21 h 47 | 95 724 077 056 | 89,150 |
| Après, retour PowerShell de l'utilisateur | 491 781 976 064 | 458,008 |
| Vérification de l'agent à 21:53:04 | 491 997 868 032 | 458,209 |

Le gain observé est de **368,858 Gio** dans le retour utilisateur, puis **369,059 Gio** à la vérification. Ces différences d'espace libre incluent l'allocation du système de fichiers et les autres variations du volume ; elles ne remplacent pas la taille logique de l'inventaire.

**Conservation vérifiée après nettoyage :**

- Les trois ZIP de preuves sous `C:\dev\whisper\docs\architecture\raw` ont les SHA-256 et nombres de membres attendus : **1 239**, **3 613** et **72 563**. Les deux copies historiques sous `C:\wt\refacto-poll\docs\architecture\raw` sont également identiques aux références. Les empreintes complètes restent dans la [proposition](nettoyage-refacto-poll.md#preuves-archivées).
- Les **3 EXE** de `C:\wt\refacto-poll\deliverables\refacto-poll-unified` et les **20 ressources installées** (workers, DLL et deux modèles) correspondent par taille et SHA-256 au `delivery-verification.json` archivé. L'EXE release du cache partagé du worktree correspond aussi à la référence. L'audio source canonique `C:\wb\native-qualif\reprise-5\steps\GpuReference2\n2-outputs\NativeClosureGpuReference-7abb1bbf7e06446aa132b4498f638e45\l04\fixtures\en-short.mp3` est présent.
- Les cinq caches sont présents et non vides : `target\parakeet-tests\cargo-shared` et `target\cargo-home-shared` dans **chacun** de `C:\dev\whisper` et `C:\wt\refacto-poll`, ainsi que `C:\dev\whisper\native\cache`. Leur contenu complet n'a pas été rehaché ; aucun de leurs chemins ne figure dans la suppression autorisée.
- Les **16 jonctions exclues** conservent leur type et leur cible ; leurs cibles sont présentes. Le worktree et la branche `refacto-poll-unified` restent à `ee3a4ccb`, sans changement suivi ni non suivi observé. Les bilans de phase 1 et phase 2 sont inchangés.

Vérifications finales : état Git, existence des chemins et des exclusions, SHA-256 des archives et ressources ci-dessus, liens documentaires et `git -c core.whitespace=cr-at-eol diff --check` (retour de terminal conservé octet pour octet, avec ses fins de ligne Windows). Aucun Cargo, nouveau test produit, lancement Windows ou nettoyage additionnel. Ce bilan et le retour de terminal sont les seuls nouveaux fichiers de cette vérification ; aucune nouvelle règle de rétention n'est ajoutée.
