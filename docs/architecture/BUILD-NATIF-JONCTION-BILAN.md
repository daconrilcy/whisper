Avancée : jonction Windows traitée ; cible conservée ; correctifs et tests relus CLEAN.
Blocage : préservation partielle initiale, puis Windows refuse Directory.Move du reliquat avec Access denied.
Décision : arrêt avant compilation ; aucune fusion, installation, élévation ou modification d'ACL.

# BUILD-NATIF — complément jonction, 5 novembre 2026

Le retrait non récursif du seul alias `INetCache/Content.IE5` a réussi : métadonnées enregistrées avant action, cible `INetCache/IE` ordinaire et contenu préservé. Les autres liens restent refusés. **Aucun classement ou arrêt de garde n'est assoupli.**

La préservation par `Move-Item` a ensuite échoué sur un élément masqué/système/lecture seule et n'est pas atomique : une première partie, dont marqueur cpu7, sources et build, est dans `target/native-build/guard/NativeDllCpu-cpu7c/previous-workspace`. Le reliquat `userprofile/` et `xdg/` reste dans `C:/wb/native/cpu`.

Après accord explicite, le correctif `Directory.Move` sur un même volume a passé ses trois tests réels et sa revue. La tentative autorisée de préservation séparée du reliquat a néanmoins reçu **Access denied**. Aucun autre mécanisme tenté. La destination `previous-workspace-residual` n'existe pas ; le reliquat est inchangé après refus : **3 fichiers, 27 dossiers, zéro jonction**. Cause précise non établie ; les ACL ont seulement été lues et aucun verrou n'est attribué à un processus.

| Vérification | Résultat |
| --- | --- |
| Suppression isolée : cible non vide et SHA conservés | PASS |
| SelfTest : collecteur 1 000 remplacements, flags 41, workspace 12, jonction 11, déplacement 3, négatifs historiques 8 | PASS |
| Revue du retrait, de la capture et du déplacement | CLEAN |
| cpu7c2 / cpu8 ; critères PE, répétabilité et N2 du candidat | NON EXÉCUTÉ |
| N1 / GPU | NON EXÉCUTÉ |

**Verdict d'équivalence : non démontré.** Une seule construction CPU ancienne commencée sur quatre (cpu7) ; cpu7c s'est arrêté avant compilateur. Zéro GPU et zéro N2. Les 14 DLL installées sont inchangées. Aucun changement Rust ; aucune suppression des espaces ou archives protégés.

**Preuves :** [archive vérifiée](raw/build-natif-jonction/complement.zip), **89 fichiers + manifeste**, 1 879 810 octets ; SHA-256 `03a8fbadc4b54e0cc54b09ed597d7c56d8847125028de89e10f2a148aa60125b`. Archive rouverte : noms, nombres, tailles et empreintes conformes. Procédure, autorisations et limites dans le [complément de cadrage](../cadrage/build-natif-jonction-windows.md). Aucun nettoyage exécuté.

**Recommandation :** ne pas fusionner la branche complète ni remplacer les DLL. Préserver les deux parties en l'état ; identifier la cause Windows du refus avant un nouveau déplacement. Aucun droit de forcer, changer les ACL ou supprimer le reliquat n'est déduit de cette clôture.
