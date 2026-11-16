# Préparation de l'assemblage L3 Turbo

Avancée : orientations de livraison acceptées et tâches T31–T33 précisées pour le candidat Turbo.
Blocage : gate L2 ouverte, droits et ressources finaux incomplets, primitive de publication à établir.
Décision : l'implémentation et les campagnes L3 attendent un mandat de lot après fermeture des prérequis.

Ce document prépare L3 à partir du [plan de distribution](plan-implementation-distribution-cpu-x64.md) et de l'[inventaire des ressources](inventaire-contrat-installation-turbo.md). Il ne déclare pas L2 terminé. État de travail : `C:/wt/distcpu-l2-reprise`, base L1 `26da7b6e`, changements T21/T22 non commités. Les candidats desktop et GPU compilés sont des entrées de travail ; les octets à livrer ne sont pas figés.

## Entrées et prérequis de L3

1. Fermer la gate L2 prévue : composition Turbo revue et vérifiée, worker CPU et GPU release identifiés, imports et options de compilation attribués à leurs PE, N1 et Clippy applicables réussis ; relier le desktop final aux références de workers selon le contrat retenu. La preuve de reconstruction après changement du marqueur et des références reste ouverte.
2. Établir la liste exacte des dépendances par PE, y compris transitive et dynamique, ainsi que les licences, notices et conditions de redistribution. Les DLL CUDA locales repérées et les fichiers Visual C++ restent des candidats, pas des ressources approuvées. Confirmer les droits du code propre et du distributeur. Une ressource ou un droit non établi bloque l'assemblage.
3. Fixer la provenance du modèle GGML correspondant à l'empreinte `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`, ses conditions d'utilisation et le protocole de sélection locale. Le modèle est fourni séparément, hors ligne.
4. Confirmer le support des versions Windows Home/Pro à la publication et les prérequis GPU. Les essais matériels v2, ARM64 et l'inférence GPU sont différés selon L4 ; ne pas leur attribuer de PASS.

## Ordre de réalisation proposé

### T31 — Entrée et compatibilité

Dans `crates/whisper-bootstrap/`, remplacer le point d'entrée actuellement minimal par le parcours d'installation autorisé. Réutiliser la politique CPUID existante dans `whisper-core` et la lecture adaptée dans `whisper-adapters`. L'entrée d'installation est construite pour x86-64 v1, sans dépendance d'exécution au worker ou au modèle avant le préflight ; son PE et ses imports sont vérifiés. Conserver `forbid(unsafe_code)` et demander une décision précise si une primitive système ou une dépendance nouvelle devient nécessaire. Un processeur sous v2 ou un état indéterminé produit un message distinct et aucun effet durable.

### T32 — Transaction d'installation et refus

Préparer dans un répertoire temporaire identifié, sur le volume de `%LOCALAPPDATA%`, l'arborescence `app/` et `models/` avec les exécutables, DLL locales et notices retenus. Copier le modèle choisi par l'utilisateur et vérifier le SHA de la copie avant engagement. Vérifier racine finale absente à l'entrée et publier sans écraser une racine apparue concurremment. La primitive de publication et son comportement en concurrence doivent être démontrés avant de choisir une implémentation ; l'absence de primitive conforme interrompt cette tâche. Ne pas créer artificiellement les réglages ou journaux que le produit crée lui-même.

Couvrir avec des tests du vrai parcours décisionnel : CPU indéterminé, CPU refusé, modèle absent ou erroné, ressource altérée sur copie identifiée, annulation avant engagement et conflit de racine simultané. Comparer les états avant/après et vérifier les messages. Un échec après engagement est visible sans promettre un rollback hors périmètre. Le nettoyage ne vise que le temporaire dont le processus établit la propriété.

### T33 — Assemblage et première utilisation

À partir des artefacts finaux identifiés et des droits clos, préparer un candidat unique contenant installateur, desktop, deux workers, DLL autorisées et notices ; le modèle est sélectionné séparément hors ligne. Retenir les empreintes des artefacts et la configuration de build dans les preuves prévues du lot, sans inventer de manifeste de qualification supplémentaire. Vérifier les imports de chaque PE et les résolutions des DLL, y compris les DLL transitées depuis CUDA. Sur un profil Windows propre au parcours de recette, vérifier la création effective des réglages/journaux par le produit, le démarrage et l'inférence WAV CPU sans dépôt ni SDK CUDA ou outils de développement. La fenêtre et la recette finale utilisateur restent sous son contrôle quand le plan le prévoit.

## Vérifications et critères de sortie

Avant chaque campagne, relever branche, diff, espace libre ≥ 30 Gio, ressources actives et racines autorisées ; exécuter le SelfTest et les commandes Cargo applicables via `ci/guard-level1.ps1` suivant les signatures autorisées. Les catégories attendues sont format, tests ciblés T31/T32, N1 complet et Clippy structurel, builds séparés v1 et Turbo, puis contrôles PE/imports. Les commandes et cibles exactes dépendent de la conception et des profils de garde qui devront être arrêtés et relus avec le lot ; elles ne sont pas encore une recette exécutable.

Gate L3→L4 : les trois tâches sont exécutées, le candidat et ses ressources sont identifiés, les droits sont fermés, les refus avant mutation et le premier lancement sont démontrés. L'absence d'un prérequis ou d'un résultat laisse la gate ouverte. Les essais utilisateur, v2 exact et Windows ARM64 conservent leur statut et leurs conditions L4.
