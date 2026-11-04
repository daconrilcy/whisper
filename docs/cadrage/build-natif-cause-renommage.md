Avancée : compilation CPU cpu4 réussie sous garde corrigée ; cinq DLL produites, installation inchangée.
Blocage : écarts PE et tailles non expliqués ; répétabilité et équivalence fonctionnelle non exécutées.
Décision : arrêt après l'unique compilation autorisée ; verdict BUILD-NATIF NON REPRODUIT.

# BUILD-NATIF — diagnostic du nom null et proposition, 4 novembre 2026

Demande initiale : expliquer le nom null et proposer un correctif. Le diagnostic initial n'a changé aucun code de garde ni lancé CMake. L'utilisateur a ensuite autorisé sa mise en œuvre : « ok mets en oeuvre le correction ». Ce document vivant conserve les deux étapes et leurs preuves ; dixième rapport du lot, aucun rapport supplémentaire créé. Le bilan cpu3 reste figé.

## Cause et preuves

`ci/guard-native.ps1:1462` utilise FileSystemWatcher ; la fonction Receive-WatcherEvents, ligne 1498, lit directement SourceEventArgs. Dans cpu3, **Name est déjà null dans RenamedEventArgs**, avant sérialisation PowerShell. FullPath combine alors racine et nom absent : `native/` n'est pas une destination observée.

Le [parseur Windows officiel .NET](https://raw.githubusercontent.com/dotnet/runtime/main/src/libraries/System.IO.FileSystem.Watcher/src/System/IO/FileSystemWatcher.Windows.cs), lignes 256–361, conserve l'ancien nom seulement pendant le traitement d'un tampon. Il assemble les notifications OLD/NEW supposées consécutives ; une autre action intercalée, ou une fin de tampon sans NEW, conduit à publier un renommage sans nouveau nom. Le code consulté est la branche publique main, pas une preuve du binaire exact installé. Le mécanisme est corroboré sur la machine réelle par la reproduction suivante ; aucun paquet Win32 historique de cpu3 ne permet de départager ces déclencheurs.

Un seul essai synthétique comparatif, PowerShell 7.6.9 / .NET 10.0.14 : deux processus écrivent chacun 1 000 fichiers `rules.ninja.tmp<hex>`, puis les renomment vers `rules.ninja` avec remplacement, avec écritures de bruit. Trois observateurs récursifs simultanés, tampon 65 536 octets, fonctions de collecte existantes, sous `target/native-build/rename-probe-5b56aa5a/`. Aucun CMake, modèle ou DLL du produit chargé.

| Filtre | Événements / Renamed observés | Nouveau null / ancien null | Erreurs observées |
| --- | --- | --- | --- |
| Noms + contenu, comme la garde | 16 065 / 2 016 | 17 / 17 | 0 |
| Noms seuls | 6 028 / 1 997 | 1 / 1 | 0 |
| Contenu seul | 10 019 / 0 | 0 / 0 | 0 |

Dans names.json, un événement ancien-seul `rules.ninja.tmp00120` est immédiatement suivi d'un nouveau-seul `rules.ninja`, même dossier worker-1. **Cela ne suffit pas à apparier des événements de campagne sans identité.** La séparation des filtres réduit le défaut mais ne suffit pas comme correctif. Limite de mesure : capture avant arrêt des observateurs après 200 ms de silence ; comptages observés uniquement, couverture complète non prouvée. L'absence d'erreur enregistrée ne prouve pas l'absence de perte. Le défaut est reproduit sur des mouvements connus internes à la fixture ; l'innocuité du renommage historique cpu3 reste non établie.

## Contrat de correction accepté

Remplacer la collecte FileSystemWatcher **pour NativeBuild uniquement** par un adaptateur Windows `ReadDirectoryChangesExW`, information étendue avec FileId. [Microsoft recommande cet identifiant pour réunir les deux notifications](https://devblogs.microsoft.com/oldnewthing/20260508-00/?p=112310). Le composant d'interopérabilité versionné sous `ci/`, chargé par `ci/guard-native.ps1`, est autorisé par cet accord : aucun service, privilège supplémentaire ou surveillance supplémentaire.

1. Conserver exactement racines, récursivité et filtres existants ; journaliser les enregistrements bruts, identités, séquences et frontières de lecture/drainage. Vérifier Windows/API et NTFS pour **toutes** les racines avant lancement ; indisponibilité = refus, sans repli silencieux. [Limites de l'API](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-readdirectorychangesexw).
2. Définir un drainage comme un instantané fini des notifications déjà reçues au contrôle courant. Assembler uniquement OLD/NEW complets, ordonnés et non ambigus de la même identité de fichier et du même volume, dans la même session de surveillance. Les frontières de tampon et événements intercalés ne détruisent plus la paire **si ses deux moitiés sont déjà disponibles**. Ne pas attendre une livraison ultérieure, ne pas maintenir une moitié au-delà du contrôle, ne pas allonger les délais. Même FileId ne suffit pas en présence de plusieurs appariements possibles ; refuser.
3. Transmettre la paire complète au classement existant, sans modifier `Get-EventDisposition`, profils, racines ou exceptions Temp. Une destination hors périmètre reste bloquante. Orphelin à la fin du drainage, identité insuffisante, ambiguïté, paquet invalide, perte, zéro octet ou erreur API = FAIL et interruption actuelle. Ni proximité temporelle, ni nom similaire, ni état disque final ne permettent d'autoriser un événement incomplet.
4. Qualifier sans CMake : OLD/NEW séparés par une action et par deux tampons déjà reçus ; moitiés manquantes ; identités différentes ; renommages répétés/ambigus ; notifications invalides et débordement ; écritures refusées, renommages traversant une frontière, règles Temp inchangées. Puis témoin réel sur chaque racine et stress concurrent avec compte attendu, collecte jusqu'à quiescence et drainage final vérifié. Réutiliser ce rapport et son archive pour les résultats. Revue indépendante avant toute demande de relance native.

Cette proposition corrige une limite d'assemblage, sans garantir que toute compilation passera : une moitié réellement absente devra encore arrêter. Un besoin d'attente supplémentaire serait une autre décision, non proposée ici. Versions, options CMake, sources, Rust produit et DLL installées restent hors du correctif.

## Conservation et statut

[Archive des observations](../architecture/raw/native-build/rename-collector-diagnostic.zip) : 5 fichiers de preuves et manifeste, 6 entrées vérifiées par tailles/SHA-256 ; 474 879 octets ; SHA-256 `c24d63c8c6d235f6126f5293549168d5dc83fa2d2d371f83c7d91481f79a2607`. Volume C: NTFS ; 530 753 126 400 octets libres après archive. Aucune suppression, fusion, publication ou compilation. Verdict BUILD-NATIF inchangé : **NON REPRODUIT**.

Revue indépendante initiale : premier constat sur l'attente intertampons intégré dans le contrat de drainage ci-dessus ; second passage **CLEAN documentaire**, constat BN-D01 fermé, rapport/renvois/archive vérifiés. Aucun correctif d'exécution livré à cette étape initiale.

## Mise en œuvre et validation autorisées

`ci/native-directory-watch.cs` collecte les paquets étendus sur les 80 racines réelles, filtres et récursivité identiques. Les lectures sont armées avant tout enfant ; noms relatifs et paquets validés ; identité session/volume/FileId, ordre et création concordants exigés. Les moitiés non réunies au drainage, erreurs API, débordements et saturation de la file deviennent WatcherError. La saturation et les défaillances du lecteur restent terminales. La mémoire native n'est libérée qu'après complétion prouvée de l'I/O, sinon conservée avec échec. `ci/guard-native.ps1` ne sélectionne cette voie que pour NativeBuild ; aucune attente de moitié manquante. Les autres profils conservent FileSystemWatcher.

`ci/test-native-directory-watch.ps1`, appelé par SelfTest NativeDllCpu/Gpu : **37 contrôles PASS, une observation de limite**, ouverture/armement des 80 racines réelles sans y écrire, **1 000 renommages réels concurrents attendus/observés**, contrôles d'identité/paquet/orphelin/ambiguïté, six frontières OLD/NEW réelles, saturation terminale et deux échecs ciblés de durée de vie. Avant le PASS final, deux échecs de tests ont été conservés : attente incorrecte OLD/NEW pour un mouvement interparents ; résolution PowerShell de Expression.New comme constructeur. Ils ont corrigé les tests, sans masquer de perte de surveillance ni relancer CMake.

**Limite observée et héritée :** le mouvement réel interparents de la fixture user-temp vers allowed est notifié Deleted puis Created : classements inchangés UNATTRIBUTED_TEMP et ALLOWED (Changed du dossier allowed : ALLOWED). Il ne déclenche donc pas l'arrêt. Les garanties de frontière ci-dessus concernent les paires Renamed effectivement reçues ; aucune garantie universelle de détection des mouvements interparents n'est établie. Pas de fusion DEL+ADD inventée dans ce correctif. Cette limite existait dans le modèle de classement antérieur ; elle reste un risque de surveillance, sans démontrer une fuite de compilation. Aucun nouveau périmètre de correction n'en est déduit.

Le texte de Get-EventDisposition, Get-RunWatchSpecs, Start-WatchState, Assert-NativeBuildProfile et New-NativeBuildProcessInfo est identique à `2f834d96`. Vérifications CPU/GPU `-CheckOnly` et `git diff --check` réussies. La revue code a détecté BN-C01 (mémoire libérable sur exception après armement/publication) : correction et tests ciblés inspectés au second passage, **CLEAN**, aucun constat ouvert sur ce delta borné. Ce CLEAN ne qualifie ni l'absence universelle d'écritures hors racine ni les DLL.

Preuves du correctif : `target/native-build/guard/selftest-ef0bb8ff5cae4e98a167578c3b9b5443/`. [Archive de validation](../architecture/raw/native-build/native-collector-validation.zip), 39 fichiers et manifeste, **40 entrées**, tailles/SHA-256 vérifiés après correction du manifeste décrite plus bas ; **2 754 234 octets**, SHA-256 **291dcaec4ccae29e1a66497d863846d90cf096c85c359b53e10bb57d3bc6613c**. Les essais antérieurs sont inclus, leurs contenus inchangés ; l'archive diagnostique précédente est conservée. Espace libre relevé après archivage initial : 530 404 126 720 octets. Aucun CMake, Cargo ou N2 n'avait été engagé par le seul accord de correction.

Vérification indépendante des conclusions et de cette archive : **CLEAN documentaire**. Les empreintes sources archivées correspondent au candidat relu ; aucun code rouvert lors de cette vérification.

## Compilation cpu4 autorisée

Accord précis : « Je t'autorise à relancer une compilation CPU sous la garde corrigée, sans assouplir ses règles, et à t'arrêter au premier échec. » Plan `77f4413b`, candidat code `82ea85e6`. Une seule commande `./ci/build-native.ps1 -Variant cpu -BuildId cpu4`, aucun assouplissement. Préflight PASS : worktree propre, versions attendues, dossiers de sortie neufs, racines sans reparse, plus de 30 Gio libres, 14 empreintes installées conformes.

**Résultat : PASS de la garde, sortie 0**, du 4 novembre **21:05:57,796 à 21:14:29,858 UTC** (22:05–22:14 Paris), soit **512,061395 s**. Manifeste **BUILT_NOT_QUALIFIED**, cinq DLL dans `target/native-build/cpu/cpu4/build/bin/`. Sources déclarées PINNED_AND_TRACKED_CLEAN et 19 patches ; GGML_NATIVE=ON, GGML_CUDA=OFF, Release, MultiThreadedDLL (/MD), triplet x64-windows-static-md effectifs. La valeur historique de GGML_NATIVE reste hypothétique et le risque de portabilité CPU est maintenu. Aucun nouveau processus cmake/cl/ninja/vcpkg observé actif après clôture.

Surveillance enregistrée : **120 870 ALLOWED, 353 UNATTRIBUTED_TEMP, aucune VIOLATION**. Ce PASS démontre l'achèvement selon les contrôles existants, pas l'absence universelle d'écriture hors racine ; la limite DEL+ADD reste applicable. **14/14 SHA-256 installés inchangés** après exécution.

Observations statiques postérieures, en lecture seule, sans charger les DLL : dumpbin exports nom+ordinal et imports DLL+symboles ; contrôle PE existant `ci/check-worker-imports.ps1 -Kind ParakeetCpu` pour chaque DLL. Les empreintes reconstruites concordent avec le manifeste.

| DLL | Octets reconstruits / écart installé | Exports nom+ordinal | Imports DLL+symboles | Sans import CUDA |
| --- | --- | --- | --- | --- |
| ggml-base.dll | 684 032 / +0,451 % | identiques, 691 | différents | PASS |
| ggml-cpu.dll | 947 200 / 0 % | identiques, 64 | différents | PASS |
| ggml.dll | 59 392 / −4,132 % | identiques, 16 | différents | PASS |
| nemo_speech_asr.dll | 4 114 432 / −1,856 % | différents, 10 992 → 10 994 | différents | PASS |
| nemo_speech_asr_c.dll | 54 272 / −3,636 % | identiques, 40 | identiques | PASS |

NeMo possède deux noms d'exports supplémentaires (`??_H@YAXPEAX_K1P6APEAX0@Z@Z` et `??_I@YAXPEAX_K1P6AX0@Z@Z`), décalant des ordinaux. Causes et innocuité non établies. Les écarts d'imports concernent notamment le runtime C++ ; aucune attribution automatique à /Brepro ou aux chemins. Les deux écarts de taille supérieurs à 2 % restent **non expliqués**. Aucun paramètre corrigé pour les réduire. Le contrôle CUDA vise les tables PE directes et différées de ces cinq fichiers ; aucune résolution récursive des dépendances ni exécution prouvée.

| DLL | SHA-256 installée | SHA-256 cpu4 |
| --- | --- | --- |
| ggml-base.dll | 398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a | 6a7e45b9e93817adba2d1a54f2d2b9c47b547226a2a538723f9134f4e383e13b |
| ggml-cpu.dll | f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0 | c78fd21135ffcdee3a282ed74a81c2663355ef6638ea24ff05035c581cdeb722 |
| ggml.dll | 2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794 | 3eb3bc924d7f682108d023e24bebcc688eee96d8d9241606358f10ceb7cf2ce6 |
| nemo_speech_asr.dll | 923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019 | 1058ca459801f8bc706487120233dc8d6e220c465c2508e1aaaea1cefd2d1526 |
| nemo_speech_asr_c.dll | 9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764 | fa5c9c0d2254d097c9e6be354b61b737e7873c45ad0e637e62ef901ff447e7f8 |

Répétabilité : **NON EXÉCUTÉE**, une seule compilation autorisée. Fonctionnel : **aucun essai N2**, sorties TXT/SRT/WAV non comparées ; N1 et GPU non exécutés. Le succès CPU ne vaut pas équivalence et ne démontre rien sur le GPU. Revue indépendante des preuves et conclusions : **CLEAN**, verdict du lot **NON REPRODUIT** justifié. Aucune fusion recommandée ni levée de conservation. Autorisation consommée, aucune autre compilation engagée.

[Archive cpu4 vérifiée](../architecture/raw/native-build/cpu4-build.zip) : **44 fichiers et manifeste, 45 entrées**, tailles/SHA-256 contrôlés pour chaque preuve à partir du manifeste relu dans le ZIP. Inclut les cinq DLL, cache CMake, manifeste de compilation, stdout/stderr, observations brutes et décisions de garde, pré/postflight et relevés PE. **13 564 657 octets**, SHA-256 **8de8a4e729d7016ab096763bf9afe879bc90deb9d12ed0d3c2f0278156e1d8a7**. Espace libre après correction des deux manifestes d'archive : **521 140 023 296 octets**. Aucun nettoyage ni remplacement de l'installation, aucune correction de code, fusion ou publication. Les traces antérieures et les chemins protégés restent conservés.

**Relecture d'archivage BN-A01 :** le manifeste interne initial de cpu4 contenait des champs null, erreur de génération de preuves : Select-Object appliqué aux OrderedDictionary ne lisait pas leurs clés. Le précédent ZIP de validation du collecteur avait le même défaut. Les deux manifestes ont été régénérés avec objets explicites, chaque preuve d'abord comparée au fichier source conservé, puis **chaque entrée du manifeste relu depuis le ZIP** vérifiée par taille et SHA-256, avec noms uniques et compte exact. Les contenus des 44 et 39 preuves respectives sont inchangés. Seuls les manifestes internes et donc les empreintes/tailles des ZIP changent. Le manifeste de compilation cpu4 était correct et n'a pas été modifié. Aucun script de garde, outil natif ou essai relancé pour cette correction documentaire. La formulation obsolète « statique NON EXÉCUTÉE » du manuel a aussi été corrigée : partiellement évaluée, non établie.

Deuxième et dernier passage de revue documentaire : **CLEAN**, BN-A01 et le constat mineur fermés ; 44/44 preuves cpu4 et 39/39 preuves du collecteur vérifiées indépendamment depuis les manifestes internes, ainsi que tailles et SHA-256 globaux. Aucun code rouvert ou essai relancé.
