Avancée : le mandat d’un prototype local MSIX CPU est prêt à arbitrage, sans changement produit.
Blocage : les fichiers source du worktree sont déjà modifiés ou non suivis, dont plusieurs chemins du mandat ; la base exacte doit être stabilisée avant exécution.
Décision : autoriser ou amender les deux sous-lots P8-A/P8-B, après confirmation d’une base source propre et du test de signature local décrit ci-dessous.

# Mandat proposé — prototype local MSIX CPU — 22 novembre 2026

## Objectif et limite

Établir, sur Windows 11 x64, si l’édition CPU duale peut s’exécuter comme application MSIX avec ses modèles, workers, DLL CPU et notices lus depuis le package, et ses données utilisateur écrites hors de celui-ci. Le prototype est local, signé pour test seulement, non soumis au Store et non destiné aux utilisateurs. Il ne ferme pas à lui seul la compatibilité MSIX de l’édition NVIDIA ni le parcours Snapdragon sans pack OpenGL.

Les choix déjà acceptés restent deux fiches Store séparées CPU et NVIDIA, x64. AMD reste différé. Le test ne mesure ni vitesse ni qualité. Il ne relance pas une matrice fonctionnelle complète déjà acceptée.

## Préflight obligatoire avant toute modification

Le worktree `C:\wt\distcpu-l2-reprise`, branche `codex/cpu-distribution-l2-reprise`, est actuellement sale : plusieurs chemins ciblés ci-dessous sont modifiés ou non suivis. Le P7 n’a pas établi de source propre équivalente aux binaires assemblés. Avant P8-A, il faut donc :

1. relever chaque diff/untracked sur les chemins visés et préserver son contenu ;
2. identifier le commit source exact du candidat P7 ainsi que ses empreintes et preuves de build ;
3. démontrer que ce commit correspond bien aux binaires P7 retenus. Si cette correspondance ne peut pas être établie, arrêter et demander à l’utilisateur de choisir une nouvelle base ; ne pas prétendre reproduire le candidat P7 ;
4. créer un worktree court `C:\wt\whisper-msix-cpu-spike` depuis le commit approuvé, sans recopier les changements du worktree sale ;
5. arrêter et demander arbitrage si un changement ciblé non attribuable au commit approuvé est requis.

Aucun worktree, patch, commit de source, build ou test n’est créé par ce mandat préparatoire.

## P8-A — contrat de chemins MSIX (8 chemins)

**But.** Séparer la racine immuable des actifs package de la racine de données de l’utilisateur, tout en conservant exactement les chemins et comportements de l’édition duale ZIP non empaquetée.

**Chemins autorisés :**

1. `crates/whisper-adapters/Cargo.toml` — activer la feature `Storage` du crate `windows` déjà déclaré, sans nouvelle dépendance ; aucun changement à `Cargo.lock`.
2. `crates/whisper-adapters/src/path_settings.rs` — exposer la racine de données empaquetée par l’API projetée sûre `ApplicationData::Current().LocalFolder().Path()` ; l’absence d’identité package est un état explicite, sans repli silencieux en mode MSIX.
3. `crates/whisper-adapters/src/path_settings/msix_data_root_tests.rs` — tests isolés du calcul/échec de résolution.
4. `crates/whisper-adapters/src/parakeet_import.rs` — prendre les workers, le modèle et les DLL CPU depuis les actifs package et garder la validation d’identité/runtime actuelle.
5. `crates/whisper-adapters/src/parakeet_import/msix_path_tests.rs` — tests isolés du routage des ressources.
6. `crates/whisper-desktop/src/root.rs` — composer les actifs package et l’emplacement de données MSIX.
7. `crates/whisper-desktop/src/root/distribution_edition.rs` — fonctions distinctes pour racine package en lecture seule et racine utilisateur LocalState.
8. `crates/whisper-desktop/src/root/tests/msix_paths.rs` — tests isolés des chemins duals empaquetés et non empaquetés.

Le dossier de données MSIX utilisé sera `ApplicationData.LocalFolder\WhisperTurbo` pour settings, logs et état propre à l’application. Modèles, workers, DLL et notices restent dans le dossier du package. Les WAV/MP3 d’entrée et les exports restent aux chemins choisis par l’utilisateur. Aucun changement d’archive métier, de format, d’empreinte ni de moteur n’est inclus.

## P8-B — lancement et package local (9 chemins)

**But.** Produire et installer un seul package CPU de test dont le point d’entrée conserve le préflight graphique du bootstrap, puis lance le desktop depuis le package sans recopier ses ressources dans `%LOCALAPPDATA%\WhisperTurbo`.

**Chemins autorisés :**

1. `crates/whisper-bootstrap/src/gui.rs` — si le processus a une identité MSIX, valider le package, exécuter le préflight Glow, puis lancer le desktop empaqueté ; hors MSIX, conserver le bootstrap ZIP actuel.
2. `crates/whisper-bootstrap/src/gui/msix_tests.rs` — tests isolés du choix du parcours empaqueté/non empaqueté et des erreurs.
3. `packaging/msix/cpu/Package.appxmanifest` — identité locale de test, application de confiance complète, `runFullTrust` et capacité microphone ; aucune identité d’éditeur de production n’est inventée.
4. `packaging/msix/cpu/Assets/StoreLogo.png` — icône 50×50 dérivée de l’icône intégrée à l’exécutable desktop ; échec fermé si aucune icône source exploitable n’existe.
5. `packaging/msix/cpu/Assets/Square44x44Logo.png` — icône 44×44 dérivée de la même source.
6. `packaging/msix/cpu/Assets/Square150x150Logo.png` — icône 150×150 dérivée de la même source.
7. `packaging/msix/cpu/Assets/Wide310x150Logo.png` — vignette 310×150 dérivée de la même source, sans nouveau nom ou logo de marque.
8. `ci/package-msix-cpu.ps1` — staging sous `target/parakeet-tests/msix-cpu-prototype-20261122/`, contrôle de composition CPU, génération contrôlée des quatre PNG, `MakeAppx.exe pack`/`validate`, signature de développement, vérification d’empreintes et rapport. Le script refuse CUDA, les workers GPU et les sorties hors de la racine du lot.
9. `ci/guard-level1.ps1` — nouveau sélecteur `-MsixCpuPrototype`, SelfTest positif/négatif et profils exacts P8 nécessaires aux commandes Cargo, sans élargir les catégories autorisées.

Le staging part des ressources CPU approuvées et des nouvelles sorties compilées du prototype, jamais des ZIP P7 modifiés en place. L’archive P7 reste en lecture seule. La signature est uniquement un certificat local de développement, séparé des clés de production et jamais inclus dans le MSIX.

## Vérifications et recette demandées

1. **Préflight source et outils :** base propre approuvée ; modèle Parakeet présent à `C:\dev\whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf` et SHA conforme ; `MakeAppx.exe` et `SignTool.exe` présents dans le Windows SDK ; au moins 30 Gio libres. Toute différence arrête le lot avant Cargo.
2. **Garde :** SelfTest P8 positif/négatif, puis `fmt --check`, N1 et Clippy selon la règle permanente de `AGENTS.md`, avec commandes Cargo via `ci/guard-level1.ps1`, cible neuve dédiée et modèle explicite. L’implémentation ajoute les profils au SelfTest avant de les invoquer.
3. **Assemblage :** produire `target/parakeet-tests/msix-cpu-prototype-20261122/package/WhisperTurbo-CPU-prototype.msix`; vérifier `MakeAppx validate`, signature, inventaire strictement CPU, identité et empreintes. Aucun artefact n’est déclaré distribuable.
4. **Recette locale, une seule session :** installer le MSIX depuis le menu Démarrer sur Windows 11 x64 ; ouvrir le desktop ; vérifier que le package n’est pas modifié, que settings et logs sont dans LocalState et qu’un import WAV CPU utilise le worker/modèle empaqueté puis écrit TXT/SRT/WAV dans le dossier choisi. Faire un démarrage/arrêt micro CPU pour vérifier l’accès après consentement Windows. Pas de test GPU, pas de transcription de référence supplémentaire destinée à juger qualité/vitesse.
5. **Preuve :** rapport donnant la base source, les SHAs des ressources, les chemins réellement observés, les commandes/codes, les éventuelles erreurs et le résultat par critère. Un échec de lancement, de résolution d’actif, d’écriture utilisateur, de microphone ou de signature vaut `FAIL`/`INCOMPLET`, jamais PASS.

### Commandes prévues après mise à jour du SelfTest de la garde

Depuis la racine du worktree P8, la garde devra accepter un nouveau profil borné `-MsixCpuPrototype` et la racine exacte ci-dessous. Ces formes sont la cible de l’extension, pas des commandes exécutables avec la garde actuelle : celle-ci ne connaît pas ce paramètre et contraint `-DualModelEdition` à la racine P3 sans cible fraîche.

```powershell
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122 -MsixCpuPrototype
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122 -FreshTarget msix-cpu -MsixCpuPrototype -CargoArgsJson '["fmt","--check"]'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122 -WithoutCargoTargetDir -ExistingTarget msix-cpu -MsixCpuPrototype -CargoArgsJson '["test","--no-fail-fast"]' -PersonalParakeetModelPath 'C:\dev\whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122 -ExistingTarget msix-cpu -MsixCpuPrototype -CargoArgsJson '["clippy","-p","whisper-core","-p","whisper-adapters","-p","whisper-desktop","-p","whisper-bootstrap","-p","whisper-worker-common","-p","whisper-worker-parakeet-cpu","--all-targets","--message-format=json","--","-W","clippy::too_many_lines","-W","clippy::cognitive_complexity","-W","clippy::too_many_arguments","-W","clippy::type_complexity"]' -PersonalParakeetModelPath 'C:\dev\whisper\models\parakeet-tdt-0.6b-v3.q8_0.gguf'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122 -ExistingTarget msix-cpu -MsixCpuPrototype -CargoArgsJson '["build","--release","-p","whisper-worker-cpu","-vv"]'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122 -ExistingTarget msix-cpu -MsixCpuPrototype -CargoArgsJson '["build","--release","-p","whisper-worker-parakeet-cpu","--features","parakeet-personal","-vv"]'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122 -ExistingTarget msix-cpu -MsixCpuPrototype -CargoArgsJson '["build","--release","-p","whisper-desktop","--features","personal-install","-vv"]'
pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122 -ExistingTarget msix-cpu -MsixCpuPrototype -CargoArgsJson '["build","--offline","--locked","--release","-p","whisper-bootstrap","-vv"]'
pwsh -NoProfile -File ci/package-msix-cpu.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122
pwsh -NoProfile -File ci/package-msix-cpu.ps1 -Mode BuildAndValidate -LotRoot target/parakeet-tests/msix-cpu-prototype-20261122
```

Le SelfTest doit refuser les profils non duals, les sorties hors lot, les DLL/GPU dans l’inventaire CPU, les empreintes modifiées, les commandes hors liste, les cibles non déclarées et toute signature autre que celle de développement. Le profil P8 autorise uniquement `fmt --check`, les tests N1, Clippy de la sélection listée et ces quatre builds release ; chaque forme doit être enregistrée puis testée positivement et négativement. Le SelfTest doit prouver qu’aucun Cargo n’est lancé. Toute forme de commande non reconnue arrête avant Cargo et exige une correction du mandat.

## Certificat local et changements système

Un MSIX destiné à être installé doit être signé et la signature doit être approuvée sur la machine. Pour ce prototype seulement, le mandat prévoit un certificat de test temporaire, sujet égal au `Publisher` local du manifeste, privé dans le dossier de lot et absent du package. Le certificat public sera ajouté à `Cert:\LocalMachine\TrustedPeople` ; l’opération nécessite élévation et validation UAC. Le script ne lance aucune élévation : arrêt si le certificat n’est pas déjà approuvé. Son installation, puis son retrait par empreinte et la suppression du fichier privé après conservation des preuves, devront être explicitement autorisés avant exécution. Aucune modification du magasin `Root`, du mode développeur ou du profil permanent WhisperTurbo.

L’installation locale du package et son retrait affectent l’enregistrement AppX du compte de test. Ils ne seront exécutés qu’après autorisation précise dans la session, sur le compte propriétaire ; la recette utilise un identifiant de package de test et n’altère pas les profils ZIP existants. Aucun upload Partner Center, ouverture de fiche, soumission ou publication.

## Critère d’arrêt et suite hors de ce mandat

P8-A ou P8-B s’arrête après deux cycles de revue/correction maximum, sur divergence source, changement de manifeste Cargo imprévu, dépendance non approuvée, fuite de sortie, nouvel unsafe, échec des gates ou signature non approuvée. Aucun téléchargement réseau et aucune modification de Cargo.lock. Après la recette CPU, un mandat séparé sera nécessaire pour l’édition NVIDIA (chargement réel des DLL CUDA du package) et pour le Snapdragon sans pack OpenGL. P8 CPU réussi ne ferme donc pas la compatibilité MSIX complète ni la publication.

## Références de faisabilité

- Étude : [compatibilité MSIX](etude-compatibilite-msix-20261122.md).
- Microsoft : [données et emplacement package MSIX](https://learn.microsoft.com/en-us/windows/msix/msix-containerization-overview), [capacités et `runFullTrust`/microphone](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/app-capability-declarations), [signature locale pour tests](https://learn.microsoft.com/en-us/windows/msix/package/sign-msix-package-guide).

**Statut :** mandat proposé, non autorisé pour implémentation ou exécution. La validation de ce document ne vaut pas accord de soumission ou publication.
