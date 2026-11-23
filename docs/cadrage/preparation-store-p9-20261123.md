Avancée : les prototypes MSIX CPU et NVIDIA ont démontré les parcours Windows locaux retenus ; le nettoyage P9 est terminé.
Blocage : l’identité Partner Center, le nombre de fiches, les noms, les conditions finales et les éléments de fiche restent à fournir ou arbitrer.
Décision : poursuivre par MSIX ; recommander deux fiches distinctes CPU et NVIDIA, sans ouvrir de soumission ni téléverser de package.

# Préparation Microsoft Store après P9 — 23 novembre 2026

## Périmètre retenu

Les décisions P6 restent la référence : première distribution par Microsoft Store, compte développeur individuel, droit d’installer et d’utiliser, deux compositions séparées CPU et NVIDIA, AMD différé, pas d’autre canal pour cette première étape. La propriété personnelle du code et la provenance des DLL Visual C++/CUDA ont été consignées dans le rapport P6 ; elles reposent sur les déclarations du propriétaire et les vérifications archivées, pas sur une certification indépendante.

P8 a produit un MSIX CPU de test de 2 188 205 438 octets. P9 a produit un MSIX NVIDIA de test de 3 340 753 443 octets et passé un import Turbo CUDA. Les deux prototypes ont des identités et éditeurs de test, version `0.1.0.0`, et ne sont pas prêts à être soumis. P9 a été désinstallé et son certificat local retiré ; P8 reste installé. Les artefacts et certificats de test dans `target/` restent hors de toute distribution.

## Format et fiches Store

Microsoft recommande MSIX pour une application Windows et fournit signature Store et hébergement pour les paquets MSIX acceptés. Le Store re-signe les paquets après certification ; aucun certificat de signature commercial n’est requis pour la distribution Store. Un certificat local reste utile pour les essais, mais le paquet signé par ce certificat ne doit pas être distribué hors du Store. [Démarrer avec Microsoft Store](https://learn.microsoft.com/en-us/windows/apps/publish/get-started), [exigences MSIX et signature](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/app-package-requirements?pivots=store-installer-msix).

**Recommandation : deux fiches**, « WhisperTurbo CPU » et « WhisperTurbo NVIDIA » (noms de travail seulement, disponibilité non vérifiée). Les candidats CPU et NVIDIA visent tous deux x64 et des versions Windows qui se recouvrent. Microsoft indique que le Store choisit les packages applicables à l’appareil et leur version ; **l’inférence**, vu que les deux éditions ont la même architecture et les mêmes systèmes cibles, est qu’un même produit ne permettrait pas un choix CPU/GPU selon la présence d’un GPU. Deux fiches donnent à l’utilisateur un choix explicite et préservent la séparation CPU/CUDA retenue en P6. Microsoft impose un nom Store unique et permet de le réserver avant soumission ; une réservation non utilisée expire après trois mois. [Réserver un nom MSIX](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/reserve-your-apps-name?pivots=store-installer-msix).

Le nombre de fiches n’a pas été explicitement arbitré par P6 : deux paquets distincts sont acquis, deux fiches restent la recommandation et nécessitent confirmation avant leur réservation dans Partner Center.

## Adaptations requises avant soumission

1. **Compte et identité.** Le propriétaire termine la création/vérification du compte individuel Partner Center. Pour chaque fiche, relever sur la page Product identity les valeurs exactes `Identity/Name`, `Identity/Publisher` et `PublisherDisplayName`, ainsi que le nom réservé. Les manifestes prototypes actuels (`WhisperTurbo.CpuPrototype` et `WhisperTurbo.NvidiaPrototype`, éditeurs `CN=... Prototype`, affichage « local test ») doivent être remplacés par ces valeurs exactes. Les champs sont sensibles à la casse et à la ponctuation. La réservation d’un nom ne vaut ni soumission ni publication. [Checklist de préparation MSIX](https://learn.microsoft.com/en-us/windows/apps/publish/store-readiness-checklist).
2. **Versions et packages de production.** Définir une version de lancement à quatre parties ; `0.1.0.0` reste une version de prototype. Reconstruire les deux packages en Release avec les identités Partner Center, notices et manifeste définitifs. Le Store autorise jusqu’à 25 Go par MSIX ; les deux prototypes mesurés restent sous cette limite. Revalider chaque empreinte, la signature locale d’essai, l’inventaire et le routage.
3. **Capacités.** Les deux manifestes déclarent `runFullTrust` et `microphone`. Garder uniquement les capacités réellement utilisées et préparer dans les notes de certification la justification de `runFullTrust`, capacité restreinte de l’application desktop. Microsoft précise que les capacités restreintes exigent des informations de justification à la soumission, et que `runFullTrust` est requis pour les applications desktop en confiance complète. [Déclaration des capacités](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations), [checklist Store](https://learn.microsoft.com/en-us/windows/apps/publish/store-readiness-checklist).
4. **Vérifications de certification.** Sur chaque package final exact : installation sur environnement propre, parcours essentiels avec clavier et lecteur d’écran, puis Windows App Certification Kit (WACK). Les recettes CPU/GPU manuelles déjà effectuées ne remplacent pas WACK. Aucune nouvelle mesure de vitesse ou de qualité n’est prévue.
5. **Fiche Store et conditions.** Préparer une description, fonctionnalités, captures d’écran de la version finale, catégorie, questionnaire d’âge IARC, prix, marchés, support et URL de confidentialité. Une application desktop Win32/packagée doit fournir une URL de politique de confidentialité même si elle ne collecte pas de données personnelles. La proposition de conditions d’utilisation reste à compléter avec identité de l’éditeur, coordonnées, date et version, puis à relire contre les notices des deux packages exacts. [Checklist MSIX](https://learn.microsoft.com/en-us/windows/apps/publish/store-readiness-checklist), [création de soumission MSIX](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/create-app-submission).

## Décisions restantes du propriétaire

- Confirmer **deux fiches Store** CPU/NVIDIA (recommandé) ou demander une fiche unique, ce qui impose une autre solution de choix et de composition des packages.
- Terminer la vérification du compte individuel et réserver les noms définitifs dans Partner Center ; aucune identité réelle n’est disponible dans le dépôt.
- Choisir prix, marchés et langues ; fournir contact de support et URL de confidentialité ; compléter l’éditeur et la date des conditions.
- Relire les descriptions/captures et le questionnaire d’âge. L’écran final **Submit for certification** et la publication sont une décision ultérieure distincte.

## État des actions

Aucun compte n’a été créé ou ouvert, aucun nom n’a été réservé, aucun draft Partner Center n’a été créé, aucun fichier n’a été téléversé et aucune soumission n’a été envoyée. La présente note prépare ces étapes et ne les autorise pas.
