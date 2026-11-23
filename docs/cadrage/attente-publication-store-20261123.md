Avancée : les bilans P1–P7 et les prototypes MSIX P8/P9 restent acquis et conservés.
Blocage : le propriétaire n’a pas commencé l’inscription Partner Center ; les identités et arbitrages Store manquent.
Décision : toutes les étapes de publication sont mises en attente avant Partner Center, sans relancer les recettes produit déjà closes.

# Mise en attente de la publication Microsoft Store — 23 novembre 2026

## État

Le propriétaire indique qu’il n’a pas encore commencé le processus Partner Center et demande de mettre en attente les travaux de publication. Aucune inscription, vérification de compte, réservation de nom, création de fiche, brouillon, téléversement ou soumission ne doit être entreprise par un agent.

## Étapes suspendues

- inscription/vérification du compte développeur individuel par le propriétaire ;
- arbitrage explicite du nombre de fiches (deux CPU/NVIDIA recommandées, pas encore confirmées) et réservation des noms ;
- collecte des identités exactes Partner Center et des choix de prix, marchés, langues, catégorie, support et confidentialité ;
- finalisation des conditions d’utilisation avec les coordonnées et l’identité de l’éditeur ;
- revalidation de la provenance de production, fabrication des MSIX finaux et contrôles de certification sur ces packages exacts ;
- préparation des éléments de fiches, revue finale, puis toute soumission ou publication.

## Éléments préservés et non répétition

P1–P6 sont clos selon [le plan de distribution](evo-distribution-deux-modeles.md) et [la clôture P6](raw/distribution-duale-p6-20261122/rapport-p6-20261122.md) ; P7 est clos CLEAN selon son [rapport](raw/distribution-duale-p7-20261122/rapport-p7-20261122.md). Les acceptations fonctionnelles utilisateur de P5 restent valides dans leur portée déclarée, AMD demeure différé, et aucun essai supplémentaire de vitesse ou qualité n’est demandé.

Les prototypes P8/P9, leurs limites, hashes et le nettoyage P9 sont détaillés dans [la préparation Store](preparation-store-p9-20261123.md), les [bilans CPU](bilan-prototype-msix-cpu-20261123.md) et [NVIDIA](bilan-prototype-msix-nvidia-20261123.md), et le [suivi de nettoyage](suivi-nettoyage-msix-nvidia-20261123.md). Ils ne sont pas des livrables de publication. Ne pas réinstaller P9, réimporter le certificat de test, relancer les mêmes validations fonctionnelles ou supprimer les fichiers de preuve/cache simplement pour reprendre la publication.

## Reprise

Reprendre uniquement après que le propriétaire aura terminé son inscription et transmis les identités non secrètes ainsi que les choix nécessaires. Relire alors `AGENTS.md`, cette note et la [passation de publication](reprise-publication-store-whisperturbo-20261123.md), contrôler l’état Git et les artefacts conservés, puis établir un mandat limité pour les étapes de production. L’autorisation de préparer un package ne vaut pas autorisation de le téléverser ou de le publier ; ces actes restent soumis à une décision distincte du propriétaire.
