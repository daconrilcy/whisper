Avancée : les 14 contrôles du Dell sont déclarés PASS par l’utilisateur.
Blocage : le CSV de recette, les sorties et les captures ne sont pas joints à cette attestation.
Décision : clôturer le sous-ensemble Dell sur déclaration du propriétaire, sans déclarer P5 globalement clos.

# Attestation de recette P5 — Dell

Le 22 novembre 2026, l’utilisateur indique que tous les tests préparés pour le Dell sont passés. Cette attestation couvre les 14 cellules de la matrice Dell du candidat dual P4 : pour Turbo et Parakeet, CPU et Auto avec import WAV, import MP3 et micro, plus le refus du GPU manuel.

Le candidat attendu est `WhisperTurbo-Dual-x64-P4-20261121-c08ba39d.zip`, SHA-256 `b7580b7067d421dc88c665c61262261a6cf723ee28a7239671da5f0847fba848`. La recette a été préparée depuis le kit Dell portant cette archive. Le détail des cellules, les sorties produites, les captures et le CSV rempli n’ont pas été transmis dans la conversation ; le statut PASS est donc une déclaration du propriétaire et non une vérification indépendante des artefacts.

L’utilisateur indique également que l’application est fonctionnelle sur les trois environnements testés. Cette déclaration générale ne précise pas le SHA des paquets employés sur GTX et Snapdragon ni les cellules exécutées avec le candidat dual P4. Elle n’est pas substituée aux critères d’attribution de P5.

Cette attestation ferme le sous-ensemble Dell tel que déclaré. Elle ne clôt pas P5 ni P6 : le rapport P5 conserve la réserve d’état des captures micro Parakeet GTX, et l’attribution des essais Snapdragon au ZIP P4 exact avec les observations ARM64 prévues reste à établir.
