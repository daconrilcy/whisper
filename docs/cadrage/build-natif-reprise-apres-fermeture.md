Avancée : après fermeture des fenêtres par l'utilisateur, handles Explorer non retrouvés et reliquat préservé séparément.
Blocage : cpu7c2 interrompu pendant les dépendances sur une ambiguïté de renommage de télémétrie MSVC.
Décision : arrêt avant cpu8 ; complément vcpkg soumis à arbitrage, aucun assouplissement ni relance implicite.

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

## Résultat du passage

cpu7c2 démarre le 5 novembre à 10:21 Paris et s'arrête sur la garde à 10:23 Paris pendant l'installation/compilation des dépendances vcpkg. Le relevé en échec contient deux paires OLD/NEW pour une même identité de fichier sous `localappdata/Microsoft/VSApplicationInsights`, reçues à dix secondes d'écart dans un même drainage. Le collecteur refuse explicitement ce cas ; aucune règle modifiée. Aucune des cinq DLL livrables produite, getter et répétabilité non contrôlés, cpu8/N2/N1/GPU non exécutés. Compteur cumulé : **deux constructions CPU commencées sur quatre**, zéro GPU, zéro cas N2.

Le manifeste porte `VSCMD_SKIP_SENDTELEMETRY=1` dans le pilote. La conservation dans l'environnement interne vcpkg reste une hypothèse à vérifier, pas une cause attribuée définitivement : [Microsoft documente](https://learn.microsoft.com/en-us/vcpkg/commands/env) le nettoyage des variables et le mécanisme VCPKG_KEEP_ENV_VARS. Un arbitrage est demandé pour deux préparations vcpkg comparées, puis un correctif borné et les deux constructions restantes seulement si le témoin confirme cette perte. Aucun de ces nouveaux essais ni correctifs exécuté dans ce passage.

Revue indépendante : conclusions Windows CLEAN, reçu de préservation conservant les 30 entrées (dont trois fichiers). Le parseur PE préparatoire ne permet à ce stade aucune conclusion de qualification ; son relevé d'exports se limite aux noms/ordinaux, sans cible d'éventuelle redirection. Aucune identité PE nouvelle annoncée.

Traces : [archive cpu7c2](../architecture/raw/build-natif-reprise/cpu7c2-stop.zip), 21 fichiers + manifeste, 8 389 706 octets, SHA-256 `efe7d3241b05a7c7e7b9c37de253ab239a6cc9560ab8f56be3a0b4014eee43cd`. ZIP rouvert ; nombres, tailles et empreintes vérifiés. Aucun nettoyage exécuté ; dossier cpu7c2, workspace et deux parties précédentes conservés.
