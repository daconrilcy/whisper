Avancée : L3d est implémenté et ses recettes automatiques passent sous garde, dont trois premiers imports EN5 en moins de 15 s.
Blocage : aucun finding de code ouvert ni anomalie signalée lors de la recette dans la fenêtre Windows.
Décision : L3d est CLEAN et accepté manuellement pour le profil Test, sans push.

# Bilan L3d — latence du premier import GPU

Plan : [PLAN-PARA-L3d v4](plan-parakeet-l3d-latence-import.md), revu CLEAN. Base produit L3c : `27f58ceb`. Amendements documentaires : `c1e37142` et `11b15502`. La [fiche L3d](fiche-parakeet-l3d-latence.md) contient les mesures et références de preuves.

## Ce qui a changé

Le chemin applicatif traite une petite rafale d'événements disponibles à chaque `Refresh` et attend les accusés de persistance avant la publication. La revue a trouvé qu'une file pleine pouvait perdre un segment ; le contrat distingue maintenant une saturation réessayable d'une déconnexion fatale. Le segment refusé reste en attente, est renvoyé au prochain rafraîchissement et empêche la lecture d'une fin de travail anticipée. La borne finale de quatre événements évite la saturation observée avec la borne initiale de 32. Les racines et journaux de campagne L3d sont confinés par la garde.

## Résultats

| Contrôle | Résultat |
| --- | --- |
| Saturation/reprise déterministe | File remplie par huit effets, neuvième conservé ; ordre 1–9, une persistance chacun, accusés avant publication ; déconnexion refusée sans boucle. PASS. |
| EN5 GPU release, sessions neuves | Premiers imports 10,744 / 10,775 / 10,772 s ; seconds 9,735 / 9,702 / 9,708 s. Seuil premier import ≤ 15 s : PASS. |
| EN30, A4 et R8 | EN30 99,24 s PASS ; FR87/FR27/EN15 PASS ; R8 5/5 PASS. |
| Garde et gate | 12/12 auto-tests ; format, build offline verrouillé et suite workspace `--no-fail-fast` : codes 0 ; 26/26 SHA protégés identiques. |
| NOT RUN | 34 tests ignorés dans le gate workspace, dont cinq R8 exécutés séparément avec succès ; les 29 restants sont hors des recettes L3d. |
| Revue | Deux lectures indépendantes, une correction ; verdict final CLEAN, aucun constat ouvert. |

L'essai initial avec 32 événements par rafraîchissement avait échoué sur EN30 (`PortUnavailable` après saturation) et ne qualifie pas le produit. Le candidat final à quatre événements a été rejoué après la correction de persistance. Aucun worker, DLL, modèle, protocole natif, micro ou comportement Turbo n'a été modifié. Aucun push n'est compris dans ce bilan.

Le 19/10/2026, l'utilisateur a lancé l'exécutable release L3d avec le profil Test GPU et a déclaré sa recette dans la fenêtre Windows concluante (« tout est ok »). Aucun chronométrage manuel précis ni détail de sortie supplémentaire n'a été fourni ; les valeurs du tableau restent celles des campagnes automatiques.
