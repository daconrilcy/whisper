# CHANGE proposé — reprise L04

Statut : DRAFT. Ce document suspend le handoff de code jusqu'à la nouvelle conception DESIGN, son CLEAN, le PLANS successor et le checkpoint contrôlé.

## Motif observé

Le paquet candidat partiel de L04 a satisfait les commandes de build et tests rapportées par son auteur, mais le lot n'est pas terminé. L'Auto fallback GPU vers CPU pendant le live/import et les événements visibles de supervision restent incomplets. Aucun log de campagne, revue indépendante, qualification GPU réelle, ni UI n'est fourni. Le fingerprint transmis est `6e659ac8a01b1355340058f36a1753ed6b541e3db3cced75caa6780688d96320`.

L'architecture doit aussi clarifier la différence entre l'identité durable du passage/archive et la génération d'une tentative worker. Le worker encode le PCM avant l'inférence; une relance CPU doit préserver le PCM et les segments confirmés, reconstruire le MP3 pending sans double encodage et rejeter les résultats de tentative obsolète.

## Portée de CHANGE proposée

Conserver les quinze chemins P28 et ajouter exactement les six chemins listés dans `57_L04_CHANGE_BASELINE.json` : archive live, test archive, IPC core, ports core, test live contract et worker CPU. Cette portée est proposée par l'architecte technique et reste à confirmer par la revue successor. Aucun autre chemin n'est autorisé par ce staging.

Réviser les contrats REQ-07/UC-07/AC-07, REQ-11/UC-09/AC-09; proposer REQ-26/UC-21/AC-21 pour les diagnostics bornés Q-09, sous revue. Respecter les choix Auto/CPU/GPU forcé, Q-04/Q-06/Q-09 acceptés, confirmations durables et responsabilités du worker/application.

## Garde et sortie

D23 est DRAFT et ne confère aucune autorisation d'implémenter. Les contributions spécialistes, transports exacts, revues indépendantes, PLANS successor, autorisation de code, Railguard, recapture des 21 chemins et `CheckCurrent -Lot L-WHISPER-04` restent des étapes requises. Les activités produit/UI/GPU non exécutées demeurent NOT RUN.
