Avancée : stabilisations codées et relues CLEAN ; auto-tests et témoin MSVC corrigé PASS.
Blocage : cpu7 interrompu par la garde ; cpu7b refusé avant lancement sur une jonction Windows interne.
Décision : arrêt du lot après deux cycles de correction ; aucune fusion, installation ou suppression.

# BUILD-NATIF-CLOTURE — 5 novembre 2026

| Critère du candidat de clôture | CPU | GPU |
| --- | --- | --- |
| a — exports/imports identiques ; CPU sans CUDA | NON EXÉCUTÉ | NON EXÉCUTÉ |
| b — mêmes SHA TXT/SRT/WAV en N2 | NON EXÉCUTÉ | NON EXÉCUTÉ |
| c — deux constructions neuves, mêmes SHA DLL | NON EXÉCUTÉ | NON EXÉCUTÉ |

**Verdict : NON ÉQUIVALENT, au sens de qualification non démontrée.** Aucun écart fonctionnel n'a été mesuré. cpu6 reste une preuve statique antérieure, pas une qualification du nouveau candidat. N1 non exécuté ; aucun changement Rust/Cargo contre main. Consommation : **1 construction CPU commencée /4**, un refus avant lancement, **0 GPU /2**, **0 N2 /10** ; arrêt sur prérequis, pas sur épuisement du temps.

cpu7 s'arrête pendant les dépendances : deux renommages d'une même identité dans un drainage, cas explicitement refusé. La suppression du lanceur de télémétrie MSVC est ensuite attestée sans changer la garde. cpu7b refuse de préserver l'espace fixe car Windows y a créé `INetCache/Content.IE5`, jonction vers `INetCache/IE` interne. Rien déplacé ; aucune nouvelle DLL livrable produite. Les **14 DLL installées sont inchangées**. Sources épinglées et 19 patches préservés ; décodage du commit reconstruit non atteint.

**Preuves :** [archive vérifiée](raw/build-natif-cloture/closure.zip), **127 fichiers + manifeste**, 7 139 857 octets ; SHA-256 `42a6228fa49927437a4bf84efa6d7aad7069148a0c6187c08625c30cdf250ec9`. Archive rouverte : nombres, tailles et SHA de chaque fichier conformes. Détails, revues, propositions de nettoyage et autorisations dans l'archive et le [plan](../cadrage/plan-build-natif-cloture.md). Aucun nettoyage exécuté ; espaces, dossiers protégés et archives conservés.

**Fusion : ne pas fusionner la branche complète.** Pour protéger la chaîne sans changer le runtime, une intégration séparée du script, des contrôles de flags et corrections de garde déjà éprouvés jusqu'à `c085347d` reste envisageable avec leurs réserves ; elle ne vaut pas équivalence. Les stabilisations de clôture attendent une politique de préservation explicite pour cette jonction et une qualification réelle.

**Décisions :** remplacement des DLL installées et suppression de `target/parakeet-feasibility/` **déconseillés et différés** jusqu'aux preuves a/b/c. Aucun accord de fusion ou de suppression n'est déduit de ce bilan.

Portabilité : GGML_NATIVE=ON reste inchangé.
La qualification présente ne couvre aucun autre processeur.
Proposer un lot prioritaire : base AVX2 ou variantes choisies au lancement.
Définir processeurs cibles et tests négatifs d'instructions.
Ce lot restera distinct de la reproductibilité.
