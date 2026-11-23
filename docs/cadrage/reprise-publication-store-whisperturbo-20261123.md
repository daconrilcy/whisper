Avancée : P6, les prototypes MSIX P8/P9 et le nettoyage post-P9 sont documentés ; cette note donne le relais de publication.
Blocage : le compte Partner Center individuel n’est pas encore créé/vérifié et aucune identité Store n’est disponible.
Décision : arrêter ici, avant l’inscription Partner Center, la réservation des noms, l’ouverture d’une soumission ou tout téléversement.

# Reprise de la publication Microsoft Store — WhisperTurbo — 23 novembre 2026

## But et limite de cette passation

Cette note permet au prochain agent de reprendre le travail à partir des preuves déjà acquises. Elle ne rouvre pas les validations P5/P6 et ne demande ni nouveau test de vitesse/qualité, ni nouvel essai matériel équivalent. Le but restant est de préparer deux éditions Windows distinctes — CPU et NVIDIA — pour une première distribution par Microsoft Store, puis de soumettre les packages après les décisions et autorisations requises.

**Point d’arrêt explicite :** le propriétaire n’a pas encore commencé le processus d’inscription chez Microsoft. Le prochain agent ne doit pas ouvrir Partner Center, créer/vérifier le compte, réserver des noms, créer un brouillon, téléverser ou soumettre. Reprendre après que le propriétaire aura terminé lui-même l’inscription et fourni les informations exactes demandées ci-dessous. Toute soumission/publication exigera une autorisation distincte, donnée au moment venu.

## Références à reprendre, sans recommencer les recettes

- Décision de distribution et droits : [clôture P6](raw/distribution-duale-p6-20261122/rapport-p6-20261122.md). Décisions acquises : Microsoft Store seulement pour cette première étape, compte individuel visé, logiciel livré sous forme d’exécutables et droit d’installation/utilisation, deux compositions CPU et NVIDIA séparées, AMD différé. Les déclarations de droits et l’inventaire Visual C++/CUDA sont déjà consignés dans ce rapport avec leurs limites.
- Préparation des exigences et des décisions Store : [préparation Store après P9](preparation-store-p9-20261123.md). Elle contient les références Microsoft actuelles, le processus proposé et les champs de fiche qui restent à obtenir.
- Prototype CPU : [bilan P8](bilan-prototype-msix-cpu-20261123.md), package MSIX de test `0.1.0.0`, SHA-256 `c7cdfa92b1c4e52b6e42d4e9cfd871f2ab5939b02f2d23d4173420d1fae3a33e`. Installation et import WAV CPU sur le témoin local sont PASS. Le bilan précise ses limites de provenance et de portée.
- Prototype NVIDIA : [bilan P9](bilan-prototype-msix-nvidia-20261123.md), MSIX de test `0.1.0.0`, SHA-256 `72694f261d8035c820b2fc26e662a34fcdfc35d7d70e9c4bad96d69eb9780260`. Installation côte à côte et un import WAV Turbo/CUDA sur la GTX 1080 Ti sont PASS pour ce prototype.
- Nettoyage : [suivi post-P9](suivi-nettoyage-msix-nvidia-20261123.md). P9 et son certificat de test sont désinstallés/retirés ; P8, les transcriptions et les rapports sont conservés. Les artefacts de test, dont un PFX, sont restés sous `target/` et ne sont pas destinés au Store.
- Texte contractuel encore provisoire : [conditions d’utilisation](conditions-utilisation-whisperturbo-proposition.md). Il contient des champs éditeur, coordonnées, date et version à compléter, et doit être relu contre les notices des packages de production.

Les hashes P8/P9 ci-dessus identifient uniquement les packages prototypes test-signés. Ils ne sont pas des empreintes de livraison Store. Les captures, certificats locaux, PFX et packages sous `target/` ne doivent pas être réutilisés comme artefacts de publication.

## État du dépôt de préparation au moment de cette note

Branche `codex/msix-cpu-spike`, commit documentaire `400c3111` (nettoyage P9 et préparation Store). Les changements produit non committés du prototype P9 présents à cet instant sont `ci/guard-level1.ps1`, `ci/package-msix-nvidia.ps1` et `packaging/msix/nvidia/`. Ils sont à préserver et à examiner dans leur contexte ; leur présence dans le worktree n’en fait pas automatiquement une base de production approuvée. Le candidat P8 source du P9 est identifié dans le bilan P9 par le commit `a3593dabb5b456f40457efc0a6cbec589be99a70`.

À la reprise, l’agent doit d’abord lire l’`AGENTS.md` applicable, contrôler la branche et l’état Git, retrouver les preuves citées, puis décider avec le plan actuel quel commit et quels fichiers versionnés constituent la base source exacte. Ne pas écraser, nettoyer ou régénérer le worktree/`target/` avant d’avoir vérifié ses utilisateurs, les preuves et les règles de rétention. Si la provenance d’un binaire final exact ne peut être reliée à ses sources et à ses paramètres de build, signaler ce manque et établir une nouvelle construction traçable avant certification ; ne pas prétendre que les prototypes sont reproductibles.

## Informations à recueillir après l’inscription du propriétaire

Le propriétaire fournira, pour chacune des fiches retenues, les valeurs exactes copiées de Partner Center après réservation du nom :

- décision explicite : **deux fiches CPU/NVIDIA** (recommandées dans la note de préparation, mais pas encore arbitrées) ou une fiche unique avec une solution de sélection des éditions à définir ;
- nom public réservé et identifiant produit ;
- `Identity/Name`, `Identity/Publisher` et `PublisherDisplayName`, sans reformater casse, espaces ni ponctuation ;
- état de vérification du compte individuel et éditeur affiché ;
- prix/gratuité, marchés, langues et catégorie choisis ;
- contact de support et URL publique de politique de confidentialité ;
- texte final des conditions, identité et coordonnées d’éditeur, date d’effet et version.

Ne pas demander de mot de passe, clé de compte ou secret Partner Center dans un rapport, un commit ou le chat. Le propriétaire effectue l’authentification dans son navigateur et transmet les valeurs non secrètes nécessaires.

## Travail restant après réception de ces informations

1. **Fixer le périmètre de fiches et le contrat d’identité.** Confirmer que les deux fiches sont retenues ou appliquer l’arbitrage reçu. Reporter les identités exactes Partner Center dans les manifestes CPU/NVIDIA. Chaque fiche doit avoir son nom, éditeur et version cohérents. Les identités prototypes `WhisperTurbo.CpuPrototype` et `WhisperTurbo.NvidiaPrototype`, les éditeurs `CN=... Prototype` et l’affichage `local test` ne sont pas réutilisables.
2. **Établir les sources de production et fabriquer les candidats finaux.** Pour chaque édition, identifier les sources et dépendances versionnées, les empreintes d’entrées, le modèle/notice attendus, les options de compilation et la version à quatre composantes. Produire en Release deux MSIX pour soumission Store, avec inventaires et notices exacts, en suivant le mode de signature alors demandé par Partner Center. Ne pas réutiliser les PFX ou certificats de prototype. Les runtimes CUDA restent propres à l’édition NVIDIA ; le package CPU ne doit pas embarquer CUDA. Conserver séparément les manifestes de contenu et hashes des candidats.
3. **Fermer les capacités et les notices.** Vérifier chaque capacité du manifeste contre le code et les parcours livrés. Les manifestes prototypes déclarent `runFullTrust` et `microphone`; préparer la justification de la capacité restreinte `runFullTrust` dans les informations de certification. Revoir les notices Visual C++, CUDA, Whisper/ggml et modèles pour les deux compositions exactes. Compléter et relire les conditions d’utilisation avec l’éditeur ; ne pas convertir les déclarations de P6 en garanties non vérifiées.
4. **Certification et vérification du package final exact.** Pour chaque package de production, faire l’installation propre, vérifier les parcours essentiels et les permissions micro, les identités/routages et la suppression/réinstallation, puis lancer Windows App Certification Kit (WACK). Consigner empreinte, version, machine et résultats. Les validations manuelles P5 et prototypes P8/P9 restent des preuves historiques utiles, mais ne remplacent pas les contrôles de certification du package final. Aucun nouvel essai de vitesse ou de qualité n’est requis.
5. **Préparer les pages Store.** Produire les descriptions, fonctionnalités, captures de la version finale, catégorie, questionnaire d’âge, prix, marchés, langues, support, politique de confidentialité et conditions. Une application Windows desktop doit fournir une URL publique de politique de confidentialité même sans collecte de données personnelles ; vérifier le texte de cette politique par rapport au comportement réel du produit.
6. **Revue pré-soumission et arbitrage du propriétaire.** Présenter pour chaque fiche le package exact, son identité, son SHA-256, le résultat WACK, les notices/conditions, les éléments marketing et les décisions de prix/marchés. Corriger les constats selon la limite de revue du projet. S’arrêter avant tout téléversement ou clic de soumission tant qu’une autorisation distincte et précise n’est pas donnée.

La certification Microsoft, la décision de publication et la disponibilité publique sont trois états différents. Un package prêt à téléverser n’est pas encore une publication autorisée ; la décision finale reste au propriétaire.

## Critère de reprise

La prochaine tâche peut commencer quand le propriétaire aura confirmé que son compte individuel Partner Center est créé et vérifié, qu’il aura donné les identités exactes et qu’il aura choisi le nombre de fiches. Jusque-là, les agents peuvent lire ces références et préparer la liste des besoins, mais ne doivent pas démarrer le processus Partner ni modifier les identités prototypes en supposant des valeurs.

Statut de mise en attente : [suivi des étapes Store suspendues jusqu’à l’inscription Partner Center](attente-publication-store-20261123.md).
