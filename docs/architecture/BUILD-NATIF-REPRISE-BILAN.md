Avancée : après fermeture d'Explorer, contrôles de handles valides et préservation réelle du reliquat réussie.
Blocage : cpu7c2 interrompu par la garde pendant les dépendances, sur renommages répétés d'un fichier de télémétrie MSVC.
Décision : arrêt avant cpu8 ; aucun nouveau correctif ni nouvelle compilation sans arbitrage.

# BUILD-NATIF — reprise du 5 novembre 2026

Le refus Windows de déplacement est levé. Aucun handle tiers n'est retrouvé dans le périmètre accessible, avec témoin positif valide ; 223 PID restent inaccessibles. Le même `Directory.Move` réussit après fermeture des fenêtres, sans changement de droits ou d'attributs. **Les octets et attributs du reliquat sont conservés**, séparément de la première partie. Revue des conclusions Windows : CLEAN.

cpu7c2 s'arrête à 10:23 Paris sous la garde : deux renommages successifs d'une même identité sous VSApplicationInsights figurent dans un drainage, cas explicitement refusé. Le pilote déclarait la désactivation de télémétrie ; sa transmission à l'environnement nettoyé de vcpkg reste à tester. Aucune attribution supplémentaire ni correction au hasard.

| Critère du candidat de clôture | CPU | GPU |
| --- | --- | --- |
| a — exports nom/ordinal et imports identiques ; CPU sans CUDA | NON EXÉCUTÉ | NON EXÉCUTÉ |
| b — mêmes SHA TXT/SRT/WAV | NON EXÉCUTÉ | NON EXÉCUTÉ |
| c — deux constructions neuves, mêmes SHA DLL | NON EXÉCUTÉ | NON EXÉCUTÉ |

**Verdict : NON ÉQUIVALENT, qualification non démontrée.** Aucune différence fonctionnelle mesurée. Aucune DLL livrable nouvelle produite ; cpu8, N1 et N2 non exécutés. Compteurs cumulés : **2 CPU commencées /4, 0 GPU /2, 0 N2 /10**. Aucun changement Rust, remplacement installé, nettoyage, push ou fusion.

**Preuves :** [archive vérifiée](raw/build-natif-reprise/cpu7c2-stop.zip), 21 fichiers + manifeste, 8 389 706 octets ; SHA-256 `efe7d3241b05a7c7e7b9c37de253ab239a6cc9560ab8f56be3a0b4014eee43cd`. Archive rouverte, nombres/tailles/SHA conformes. Anciennes archives, bilans figés et dossiers protégés préservés. Détails et arbitrage proposé dans la [reprise](../cadrage/build-natif-reprise-apres-fermeture.md).

**Fusion complète et remplacement des DLL déconseillés.** Le prochain complément proposé compare la conservation de VSCMD_SKIP_SENDTELEMETRY dans vcpkg avant tout correctif. Aucun accord de suppression de l'ancien spike ni d'assouplissement du collecteur n'est déduit de ce bilan.
