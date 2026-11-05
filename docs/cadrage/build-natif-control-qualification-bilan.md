Avancée : réserves du correctif NativeBuild levées ; qualification et revue indépendante CLEAN.
Blocage : aucun pour ce correctif sur la machine et les charges testées ; équivalence DLL/N2 distincte.
Décision : candidat qualifié pour reprendre les deux builds CPU autorisés, sans assouplissement de la garde.

# Qualification du contrôle NativeBuild — 5 novembre 2026

Complément autorisé : [plan](build-natif-control-qualification-plan.md), borne **13:00 Paris / 12:00 UTC**, deux cycles de revue maximum. Espace libre initial 506 653 741 056 octets. Aucun changement Rust, pin, ISA, CMake, DLL installée ou collecteur. Les versions et les refus restent ceux du plan BUILD-NATIF.

**Correctif.** `ci/native-build-control.cs` écrit directement en UTF-8 avec tampon réutilisé et écritures physiques groupées, synchrones ; normalisation et comparaisons des racines sans allocations répétées. Tous les champs JSON restent conservés ; aucune identité textuelle JSON revendiquée. Flush avant retour/arrêt et erreurs fatales inchangés. Tests et mesures dans `ci/test-native-build-control.ps1` et `ci/test-native-build-control.cs` ; temps et erreurs persistés avant les assertions. L'adaptateur et l'intégration de garde sont inchangés depuis `237323b3`.

| Contrôle réellement exécuté | Résultat |
| --- | --- |
| Politique et journaux complets, fixtures et traces historiques 82/83 | PASS ; comparaison JSON sémantique complète, seul `detected_at` variable exclu. Snapshot 83 toujours refusé. |
| Erreurs write/flush/dispose sur le chemin UTF-8 réel, nettoyage et reçu FAIL | PASS ; fermeture indépendante et conservation de l'erreur initiale. |
| Trois pointes de 55 414 records, assemblage/classement/journaux/flush | **486 / 443 / 445 ms**, PASS ; pause de 100 ms incluse dans le critère d'intervalle. |
| Première pointe à froid, processus neuf, queue bornée réelle alimentée synthétiquement | **558 ms**, **658 ms avec pause** ; aucun préchauffage réservé au test. |
| Acquisition Windows réelle avec 55 414 fichiers créés | **110 829 records et observations reçus et relus** ; identités, séquences contiguës et empreintes concordantes, zéro perte/duplication dans ce périmètre. Intervalle max **233,3 ms**, batch max **121,1 ms** incluant les contrôles de conservation. |
| SelfTest NativeDllCpu complet | PASS : racines réelles armées, 1 000 remplacements, frontières/ambiguïtés, saturation/erreurs, chemins/parents et manifeste. Aucun bootstrap/CMake. |
| Revue indépendante, cycle 2 final | **CLEAN** ; aucune réserve ouverte sur ce correctif. Qualification limitée aux charges et à la machine testées. |

Deux défauts du nouveau harnais ont arrêté les essais et reçu chacun un arbitrage utilisateur avant correction/reprise : conflit de nom C# local/fonction, puis partage Windows lors de la relecture de writers encore ouverts. Les traces d'échec restent conservées. Aucun seuil augmenté, événement filtré, demi-renommage retenu ou arrêt supprimé.

**Preuves archivées et vérifiées.** [Archive complète](../architecture/raw/build-natif-control-qualification/qualification-complete.zip) : **192 fichiers + manifeste**, 90 390 504 octets, SHA-256 `f256a896e990a3170714e530756c11b54c8cf84b546bae6851e8a7477ac456ac`. ZIP rouvert : manifeste relu, noms uniques, tailles et empreintes contrôlés, présence explicite des trois reçus finaux vérifiée. L'archive auxiliaire `qualification.zip` contient le snapshot de sources et le SelfTest (46 fichiers + manifeste ; SHA `0244d8f9257013122b8908bdc98d74dfda1531de8c1e2c7f684e5f88c0f98415`). Les fichiers générés `watched/file-*.txt` sont exclus des ZIP et conservés sur disque ; aucune suppression exécutée.

**Verdict et suite.** Réserves du correctif levées ; fusion de ce correctif recommandable au titre de sa qualification, sans fusion automatique. Aucune équivalence des DLL déduite : à cet instant **0 compilation CPU nouvelle**, deux tentatives restent disponibles ; N2 et N1 non exécutés par ce complément. La provenance historique exacte des fichiers de télémétrie reste non attribuée. Les archives et bilans antérieurs restent figés ; aucun remplacement de DLL, nettoyage, push ou fusion.

Suite : [reprise CPU cpu7c4/cpu8](build-natif-cpu7c4-cpu8-bilan.md).
