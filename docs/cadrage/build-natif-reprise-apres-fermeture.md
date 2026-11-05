Avancée : après fermeture des fenêtres par l'utilisateur, handles Explorer absents et reliquat préservé séparément.
Blocage : aucun au préflight actuel ; qualification binaire et fonctionnelle toujours à exécuter.
Décision : reprise autorisée du processus, sans modifier les règles d'arrêt ni les budgets cumulés.

# BUILD-NATIF — reprise du 5 novembre 2026

L'utilisateur confirme avoir fermé toutes les fenêtres et demande de vérifier les handles puis de retenter le processus. La sonde corrigée, avec contrôle positif, ne trouve plus de handle tiers accessible sous C:/wb/native/cpu ; 223 PID restent inaccessibles. La limite ne devient pas une preuve d'absence globale. La tentative de préservation réussit ensuite avec le même helper Directory.Move et sans changement d'ACL/attributs : `previous-workspace-residual` contient les mêmes octets et attributs. Le reçu d'échec précédent est conservé ; nouveau reçu `target/native-build/guard/NativeDllCpu-cpu7c/residual-preservation-after-close.json`. La première partie reste dans `previous-workspace`, sans fusion.

Le mécanisme reproduit dans la fixture, les handles Explorer observés avant fermeture, leur disparition puis la réussite réelle après fermeture forment la contre-vérification attendue. Aucun handle tiers n'a été fermé par l'agent. Aucun changement du script de compilation depuis les revues CLEAN ; seuls diagnostic, preuves et documents de reprise changent.

## Plan de reprise et arrêts

1. **CPU : cpu7c2 puis cpu8**, chacun après préparation d'un espace neuf au même chemin physique, script inchangé. Relever le getter ggml_commit complet, les cinq SHA, exports/imports et contrôle sans CUDA. Arrêt à toute violation de garde, erreur ou écart de répétabilité inexpliqué.
2. **CPU fonctionnel puis N1**, après réussite statique : référence et runtime alternatif épinglés, seulement les cinq DLL CPU remplacées ; FR court et Auto→CPU EN, quatre cas N2 au total, mêmes TXT/SRT/WAV. Comparer N1 à main selon le plan initial ; ne rien déduire du seul diff Rust vide.
3. **GPU seulement si le temps restant le permet** : deux constructions, surfaces PE et SHA, trois copies CUDA vérifiées, EN GPU attesté référence/alternatif. Sinon NON EXÉCUTÉ motivé.
4. **Clôture distincte** : revue indépendante des conclusions, nouvelle archive vérifiée et nouveau bilan, anciens bilans/archives intacts. Aucun nettoyage, remplacement installé, push ou fusion.

Budgets initiaux maintenus : une construction CPU ancienne commencée (cpu7) sur quatre ; les refus de préparation cpu7b/cpu7c n'ont pas démarré de compilateur. Reste au plus trois constructions CPU, deux GPU et dix cas N2. Les deux cycles du correctif de jonction/déplacement sont clos CLEAN ; pas de nouvelle correction implicite de la garde. Borne conservatrice initiale conservée : **11:00 UTC / 12:00 Paris**. Aucun travail de campagne au-delà sans décision distincte.

Les bilans figés [clôture](../architecture/BUILD-NATIF-CLOTURE-BILAN.md) et [jonction](../architecture/BUILD-NATIF-JONCTION-BILAN.md) restent des instantanés exacts à leur clôture. Le [diagnostic Windows](build-natif-refus-windows.md) décrit les preuves précédentes ; ce document consigne le fait nouveau et l'autorisation de reprise.
