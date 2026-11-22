Avancée : P7 est CLEAN ; cette note cadre l’étape de préparation Microsoft Store à partir des deux ZIP P7.
Blocage : le choix du format Store et du nombre de fiches Store reste à arbitrer ; les ZIP P7 ne sont pas des packages Store prêts à soumettre.
Décision : recommandation à confirmer — conserver la séparation CPU/NVIDIA avec deux fiches Store et étudier la faisabilité MSIX avant toute implémentation.

# Préparation de la distribution Microsoft Store

Date : 22 novembre 2026. Cette note est une lecture documentaire et statique ; aucun compte, produit ou brouillon Partner Center n’a été créé, et aucun package n’a été téléversé.

## Décisions déjà acquises

Microsoft Store seulement pour la première distribution, compte développeur individuel, utilisateurs autorisés à installer et utiliser WhisperTurbo, deux éditions distinctes CPU et NVIDIA, pas de redistribution par les utilisateurs. AMD reste différé. Les conditions d’utilisation n’ont pas encore l’identité de l’éditeur, la date d’effet ni la version finale. Voir la [clôture P6](raw/distribution-duale-p6-20261122/rapport-p6-20261122.md), la [proposition de conditions](conditions-utilisation-whisperturbo-proposition.md) et le [rapport P7](raw/distribution-duale-p7-20261122/rapport-p7-20261122.md).

## Exigences Microsoft vérifiées

- Microsoft recommande MSIX pour le Store : Microsoft fournit l’hébergement et la signature, et Windows gère les mises à jour. La voie Win32 MSI/EXE existe aussi, mais l’éditeur doit héberger une URL HTTPS versionnée et maintenir les mises à jour. [Distribution des applications Win32](https://learn.microsoft.com/en-us/windows/apps/distribute-through-store/how-to-distribute-your-win32-app-through-microsoft-store)
- Pour MSI/EXE, le package doit être un installateur `.exe` ou `.msi` autonome et hors ligne ; son lancement ne doit pas afficher d’interface d’installation ; l’installateur et tous ses fichiers PE doivent être signés par un certificat d’une autorité du Microsoft Trusted Root Program. [Exigences MSI/EXE](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/app-package-requirements)
- Pour une combinaison architecture/langue donnée, Partner Center n’accepte qu’un seul package MSI/EXE. La sélection de package annoncée par Microsoft est fondée sur l’appareil et l’architecture, pas sur la présence d’un GPU. Les deux éditions P7 étant x64, elles ne peuvent donc pas être proposées comme deux variantes x64 sous une même fiche MSI/EXE. [Gestion des packages MSI/EXE](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/app-package-management)
- L’ouverture d’un nouveau compte individuel passe par le parcours Microsoft dédié, nécessite un compte Microsoft personnel et une vérification d’identité ; Microsoft indique que l’inscription individuelle est actuellement sans frais. Le propriétaire doit faire cette vérification lui-même. Le type individuel ne peut pas ensuite être converti en compte société. [Compte individuel Partner Center](https://learn.microsoft.com/en-us/windows/apps/publish/partner-center/open-a-developer-account?tabs=individual)

## Écart des candidats P7 avec les voies Store

Les deux artefacts P7 sont des ZIP, pas des fichiers MSIX, EXE ou MSI directement soumis. Le bootstrap inclus est interactif et copie le paquet sous `%LOCALAPPDATA%\WhisperTurbo`. En outre, les binaires P7 exacts connus — bootstrap SHA-256 `cc57716b3a0058cae584ae885d0b2cc56fffa4c9681af939cd8d1f33e95ccfe5` et desktop SHA-256 `3469f6947c277e8a9932337df2def6acc01952fa1fdd19a2156a5c40c2634010` — sont `NotSigned` dans la sortie release vérifiée. Ils ne satisfont donc pas, en l’état, la voie MSI/EXE, qui exige la signature de tous les PE et une installation silencieuse.

Une voie MSIX nécessiterait une vérification de compatibilité et un assemblage spécifique. Le desktop dual résout actuellement modèles, workers, réglages et journaux depuis `%LOCALAPPDATA%\WhisperTurbo`; le bootstrap installe ses actifs dans ce même emplacement. Il faut établir comment empaqueter ces actifs en lecture seule et écrire les données utilisateur dans les emplacements MSIX autorisés, sans casser le routage CPU/GPU ni la détection du prérequis graphique. Ce travail n’est pas inclus dans P7 et n’a pas été implémenté.

## Arbitrage nécessaire avant le lot d’implémentation Store

1. **Deux fiches Store — recommandation si la séparation juridique CPU/NVIDIA est maintenue.** Chaque fiche aurait son unique package x64 ; le client choisit explicitement CPU ou NVIDIA. Le choix du format reste à faire : MSIX (Store recommande, pas de certificat éditeur ni d’hébergement tiers) ou installateur MSI/EXE dédié (signature payante/CA, installation silencieuse, URL versionnée maintenue par l’éditeur). Pour MSIX, commencer par une étude de compatibilité ciblée avant tout code.
2. **Une seule fiche Store.** Il faut abandonner la sélection automatique de deux ZIP x64 par Partner Center et choisir un autre modèle de distribution. Le paquet universel réintroduirait les DLL CUDA dans l’édition CPU, ce qui contredit le motif de séparation consigné en P6. Aucun mécanisme alternatif n’est retenu dans cette note.

La recommandation est l’option 1 avec deux fiches et une étude MSIX, car elle garde les ZIP CPU/NVIDIA séparés et évite d’ajouter l’hébergement externe et le certificat exigés par la voie MSI/EXE. Cela reste à confirmer par le propriétaire ; aucune implémentation, création de fiche ou soumission n’est autorisée par cette recommandation seule.

Après arbitrage, le lot correspondant devra finaliser les conditions (éditeur, coordonnées, date/version), vérifier le texte au regard des notices des packages exacts et des règles Store courantes, établir le format accepté et sa recette, puis réserver les noms et remplir le brouillon Partner Center. La soumission pour certification et la publication resteront une décision séparée.