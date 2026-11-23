Avancée : le prototype MSIX CPU a passé le lancement, l’import WAV CPU et le contrôle micro sur Windows 11 x64.
Blocage : le candidat MSIX NVIDIA n’existe pas encore ; le rapport P8 ne lie pas ses exécutables à un commit source exact.
Décision : arbitrer le mandat P9 NVIDIA ci-dessous avant toute modification, compilation ou installation de ce candidat.

# Mandat proposé — prototype MSIX NVIDIA — 23 novembre 2026

## Objectif et limite

Vérifier qu’un paquet MSIX NVIDIA distinct peut démarrer et charger les ressources CUDA depuis son package, puis traiter un unique WAV en GPU sur la GTX 1080 Ti locale. Le paquet reste un prototype signé pour test, sans soumission au Microsoft Store. L’édition CPU P8 reste installée et ne doit pas être remplacée. Pas d’essai micro, de matrice CPU/Auto, ni de mesure de vitesse ou de qualité ; les recettes fonctionnelles déjà réalisées sur les éditions ZIP ne sont pas répétées.

## Candidat NVIDIA existant en lecture seule

- Archive P7 : `C:\wt\distcpu-l2-reprise\deliverables\distribution-duale-p7-20261122\nvidia\WhisperTurbo-Dual-NVIDIA.zip` ; 3 328 787 880 octets ; SHA-256 `82f059632e0b2fd2a2051a825cbb4295a7824458f27a9bdf65719833df85e959`.
- Manifeste associé : SHA-256 `cbaf342fa9ddc52353734627cb03b327e390a68c61e76c3d6128acfa232637ef`.
- Le manifeste épingle les exécutables, les deux modèles, les DLL CPU et GPU et les notices. L’édition NVIDIA inclut `app/cublas64_12.dll`, `app/cublasLt64_12.dll`, `app/cudart64_12.dll`, ainsi que `native/gpu/ggml-cuda.dll`. Ces fichiers devront rester privés au package ; aucun runtime CUDA ne sera ajouté au paquet CPU.
- Le certificat public de test P8 est déjà présent dans `Cert:\LocalMachine\TrustedPeople` : `CN=WhisperTurbo MSIX CPU Prototype`, empreinte `C27F44B3B7F8CDE72A6C39838008D308BA3F1B82`. Le `.pfx` ne sera jamais importé. Le manifeste NVIDIA de test devra avoir un nom/famille distincts du CPU tout en ayant un Publisher compatible avec ce certificat. Aucun retrait de certificat ou désinstallation P8 n’est inclus.

## Lot proposé (8 étapes)

1. Geler et identifier le commit source exact avant de construire ; résoudre ou accepter explicitement la réserve de traçabilité P8. Ne pas présenter le rapport P8 comme une preuve de commit puisqu’il ne le contient pas.
2. Vérifier en lecture seule que l’archive P7 NVIDIA et son manifeste correspondent octet pour octet aux ressources attendues ; préserver les originaux.
3. Étendre le packager MSIX au variant NVIDIA avec un manifeste et une identité séparés de CPU ; refuser les DLL GPU dans l’édition CPU et refuser les sorties hors de la cible P9.
4. Ajouter au niveau 1 de la garde les seuls profils Cargo nécessaires à ce variant, avec cas positifs et négatifs au SelfTest.
5. Faire relire indépendamment les changements ; corriger au plus deux cycles. En cas d’échec de gate, arrêter et arbitrer.
6. Exécuter les vérifications et builds prescrits par la garde, puis assembler le MSIX NVIDIA signé avec le certificat de test déjà approuvé. Enregistrer le commit source, les SHA des entrées, le manifeste, les ressources chargées et l’identité du MSIX.
7. Après accord d’exécution distinct, installer le package à famille séparée sur le PC Windows 11 x64 disposant de la GTX 1080 Ti, lancer le desktop et exécuter un seul import Turbo d’un WAV de référence en GPU manuel. Vérifier dans le diagnostic que le backend est CUDA, que le worker et les DLL chargées proviennent de `WindowsApps`, et que TXT/SRT sont produits dans le dossier utilisateur.
8. Archiver le rapport compact hors de `target/` et rendre `PASS`, `FAIL` ou `INCOMPLET` par critère. L’échec de résolution CUDA ou le chargement depuis un chemin externe au package ne peut être déclaré PASS.

## Préconditions et arrêts

- Aucun téléchargement, changement de `Cargo.toml`/`Cargo.lock`, mode développeur, capacité système supplémentaire ou modification de `Root` n’est autorisé par ce mandat préparatoire.
- Respecter la garde Cargo permanente de `AGENTS.md`, la cible et les caches déclarés, l’espace libre minimal de 30 Gio et les règles des crates. Le packager refuse la présence d’une ressource CUDA non épinglée.
- L’installation MSIX NVIDIA est un changement AppX distinct ; elle ne sera exécutée qu’après l’accord précis sur le candidat final, son SHA-256 et la commande d’installation. Aucun test ne remplace ni ne modifie le paquet CPU actuellement installé.
- Le résultat ne qualifiera ni les autres modèles NVIDIA, ni d’autres cartes, ni le Snapdragon, ni la distribution publique. La recette Snapdragon Glow est un mandat séparé si elle reste nécessaire après les preuves déjà obtenues avec le pack Microsoft installé.

**Statut :** proposé pour arbitrage ; aucune modification produit, compilation, assemblage ou installation NVIDIA n’a été effectuée.
