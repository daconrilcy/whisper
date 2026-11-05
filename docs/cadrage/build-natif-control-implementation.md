Avancée : correctif de débit NativeBuild implémenté ; deux constats de fermeture corrigés et relus.
Blocage : première pointe de 55 414 notifications dépasse la limite d'une seconde ; qualification FAIL.
Décision : LIVRÉ AVEC RÉSERVES ; aucune reprise CPU, aucun assouplissement du seuil ou de la garde.

# Mise en œuvre du contrôle NativeBuild — 5 novembre 2026

Périmètre autorisé : [proposition](build-natif-telemetrie-interne.md), accord utilisateur de mise en œuvre, puis deux arbitrages précis après échecs des tests. Aucun Rust, pin, version, option ISA/CMake, fichier installé ou collecteur de notifications modifié. Espace libre initial : 507 005 091 840 octets.

**Changements.** `ci/native-build-control.cs` calcule les racines une fois, classe et journalise les observations en traitement typé avec writers persistants. `ci/guard-native.ps1` utilise ce chemin uniquement pour NativeBuild ; frontières des snapshots, Assemble, limites de queue, racines et refus inchangés. Les fermetures sont indépendantes : arrêt, drainage terminal même sur échec, journaux, abonnements de sortie et processus ; le reçu conserve erreur initiale et erreurs de fermeture. `ci/test-native-build-control.ps1` compare la politique et les enveloppes complètes, teste les erreurs et impose la gate de débit.

| Vérification effectivement exécutée | Résultat et limite |
| --- | --- |
| Analyse syntaxique PowerShell, diff sans erreur d'espaces | PASS |
| Premier essai | FAIL : ancien chemin vide ; référence lève une exception. Correction et reprise autorisées. |
| Deuxième essai | FAIL : comptage final PowerShell renvoie un tableau ; correction et reprise autorisées. |
| Troisième essai : parité de classement et journaux complets | Assertions franchies, comparaison JSON sémantique ; seul `detected_at` variable est exclu. Pas d'identité textuelle JSON revendiquée. |
| Erreurs write/flush/dispose, nettoyage et reçu FAIL | Assertions franchies ; reçu garde l'erreur initiale et les deux erreurs de nettoyage injectées. |
| Orphelins, groupe ambigu et rejeu historique 83 | Refus conservés, assertions franchies. |
| Pointe : assemblage, classement, journaux et flush de 55 414 records | **FAIL dès la première pointe : > 1 000 ms**. Temps exact non persisté avant l'assertion ; aucune estimation ajoutée. |
| Intervalle réel entre drainages, acquisition sous charge, SelfTest complet | **NON EXÉCUTÉ** après l'arrêt. La charge synthétique utilise Assemble, sans nouveau watcher. |
| DLL CPU/GPU, N2, N1 | **NON EXÉCUTÉ**. Deux compilations CPU commencées historiquement sur quatre ; deux restent disponibles. |

**Revue indépendante, deux cycles.** BN-B01 (exception Dispose sautant les autres nettoyages/reçu) et BN-B02 (drainage terminal absent sur échec) fermés. Verdict : CLEAN sur les corrections revues ; LIVRÉ AVEC RÉSERVES pour le livrable non qualifié. Aucun test exécuté par le reviewer. La gate échouée interdit une conclusion de débit suffisant ou d'équivalence des DLL.

**Preuves.** [Archive vérifiée](../architecture/raw/build-natif-control/implementation.zip) : 35 fichiers + manifeste, 9 251 545 octets, SHA-256 `6575e27137c91715e984671073c2fc681f689176a8d96bb3fa05dd2cbe34853d`. ZIP rouvert : noms uniques, nombre, tailles et empreintes de chaque fichier contrôlés. Sources candidates, journaux des trois essais et état d'exécution conservés. Aucun nettoyage, remplacement de DLL, suppression, fusion ou push.

**Recommandation.** Ne pas fusionner ce correctif comme qualifié et ne pas lancer les deux builds CPU. Le code est conservé comme candidat ; une suite demande une décision de périmètre/budget après cette gate FAIL et les deux cycles de revue. Recommandé : un complément borné pour mesurer le coût par étape et optimiser ce même chemin, sans changer le seuil ni les classifications. La borne initiale 11:00 UTC / 12:00 Paris n'est pas prolongée par ce bilan. L'origine historique exacte des fichiers de télémétrie reste non attribuée.
