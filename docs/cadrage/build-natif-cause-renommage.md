Avancée : cpu6 PASS, zéro C4530 et concordance des exports/imports installés pour les cinq DLL.
Blocage : répétabilité corrigée et fonctionnel non éprouvés ; chemins et métadonnée ggml_commit variables.
Décision : effet du correctif confirmé, équivalence complète non établie ; BUILD-NATIF NON REPRODUIT.

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

## cpu5 et analyse des écarts

Le 5 novembre, autorisation « Relance un essai puis analyse raison des écarts ». Plan `d27a5d9d`, code et configuration inchangés. Une seule commande CPU BuildId cpu5, **garde PASS/sortie 0**, de **23:16:22,887 à 23:17:16,877 UTC le 4 novembre** (00:16–00:17 le 5 novembre à Paris), **53,9897883 s**, cinq DLL et manifeste BUILT_NOT_QUALIFIED. Réutilisation des dépendances épinglées en cache, nouveau dossier de compilation : ce temps n'est pas une mesure de performance des DLL. Aucun troisième build, micro-essai corrigé ou exécution de DLL.

Pré/postflight : 14/14 DLL installées inchangées, aucun cmake/cl/ninja/vcpkg actif observé après exécution. **15 039 ALLOWED, 28 UNATTRIBUTED_TEMP**, aucune violation enregistrée ; limites de surveillance précédentes inchangées. Options effectives égales après remplacement des seuls chemins cpu4/cpu5 ; build.ninja égal après ce remplacement. Exports nom+ordinal et imports DLL+symboles identiques cpu4/cpu5 **5/5** ; contrôle PE CPU sans import CUDA **5/5 PASS**. Les écarts avec l'installation observés pour cpu4 restent présents.

| DLL | SHA-256 cpu5 | Identité brute avec cpu4 | Cause des différences cpu4/cpu5 |
| --- | --- | --- | --- |
| ggml-base.dll | 5827f88478bb7d34e1123c4177910a26f0191e995ffa750f928fb68ed2d5cdb6 | non | chemins et REPRO présents ; .text/.pdata restent différents |
| ggml-cpu.dll | 348d2f442560864b6120417de87c49a4c28cafa4592211f03bad9ddcf372e38a | non | chemins et champs PE/REPRO identifiés |
| ggml.dll | 85027282667a77ef01b2eb37a92ac6fd7d9212b61e222917ca4c995052858f2a | non | chemins et champs PE/REPRO identifiés |
| nemo_speech_asr.dll | 1058ca459801f8bc706487120233dc8d6e220c465c2508e1aaaea1cefd2d1526 | oui | aucune différence |
| nemo_speech_asr_c.dll | fa5c9c0d2254d097c9e6be354b61b737e7873c45ad0e637e62ef901ff447e7f8 | oui | aucune différence |

**1. Défaut prouvé dans l'ajout de /Brepro.** `native/build-config.json:35` à `cb3ed03e` fournit CMAKE_CXX_FLAGS=/Brepro et CMAKE_C_FLAGS=/Brepro. Le CMake 4.4.4 installé construit normalement ses défauts dans Windows-MSVC.cmake:259,538, puis CMakeCXXInformation.cmake:98–100 et CMakeInitializeConfigs.cmake:8–15 initialisent le cache seulement s'il n'est pas déjà fixé. Notre valeur court-circuite donc les défauts dont /EHsc et les définitions Windows. Précision vérifiée lors de la correction : /GR et /W3 dépendent des politiques CMP0117/CMP0092 ; leur omission peut être normale pour le NeMo épinglé et ils ne sont pas réintroduits arbitrairement. Le cache et build.ninja n'ont pas /EHsc ; **30 avertissements C4530 dans cpu4 et 30 dans cpu5** demandent explicitement cette option, y compris gguf.cpp:559 et nemo/src/runtime/ggml/session.cpp:799 qui utilisent try/catch. [Microsoft documente le déroulement C++ incomplet sans /EHsc, /EHs ou /EHa](https://learn.microsoft.com/en-us/cpp/build/reference/eh-exception-handling-model?view=msvc-170). C'est un écart sémantique réel de notre configuration, pas un simple horodatage. L'absence de /GR explicite ne prouve pas une désactivation du RTTI.

**2. Écarts avec l'installation : explication partielle, pas causalité complète.** La perte de /EHsc est cohérente avec les imports de gestion d'exceptions/terminate manquants et les diminutions de taille. Les deux exports supplémentaires de NeMo sont décodés par undname en vector constructor iterator et vector destructor iterator ; la cible active WINDOWS_EXPORT_ALL_SYMBOLS (nemo/src/asr/CMakeLists.txt:30), qui expose ce code généré. Cela n'établit pas que le seul /EHsc explique les deux exports ou tous les octets manquants. Le cache original est absent ; aucune compilation A/B corrigée n'était autorisée. Les écarts supérieurs à 2 % restent donc non entièrement expliqués, sans innocuité ou équivalence déduite.

**3. Chemins et déterminisme cpu4/cpu5.** Les trois ggml contiennent des chemins absolus vers leurs copies llama sous leurs dossiers distincts. Les champs TimeDateStamp sont dérivés d'empreintes avec IMAGE_DEBUG_TYPE_REPRO, et ne sont pas de simples heures de compilation ([spécification PE](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format)). Pour ggml.dll et ggml-cpu.dll, remplacer uniquement les racines exactes par cpuX et neutraliser en mémoire les TimeDateStamp COFF/exports/debug et le payload REPRO rend les fichiers identiques ; aucun autre octet n'a été neutralisé. Leurs sections .text sont déjà identiques sans normalisation. Cette comparaison explique leurs écarts, **elle ne valide pas la reproductibilité binaire réelle**, dont seulement 2/5 SHA sont identiques.

**4. ggml-base reste ouvert.** Sa .text comporte **13 725 octets différents**, sa .pdata **151** ; la même normalisation ne suffit pas. Les adresses des exports restent identiques. Les neuf objets ggml-base ont des multiensembles identiques de blocs .text bruts ; cela ne compare ni les relocations ni l'association des blocs aux fonctions. Une différence de disposition/relocation à l'édition de liens est plausible, mais l'origine complète n'est pas prouvée. Aucun verdict d'équivalence fonctionnelle de ce code n'est donné.

**Correction recommandée, non implémentée :** préserver les défauts CMake au lieu de fournir des valeurs complètes CMAKE_C_FLAGS/CMAKE_CXX_FLAGS ; piste concrète CMAKE_C_FLAGS_INIT/CMAKE_CXX_FLAGS_INIT pour ajouter /Brepro avant l'initialisation de la plateforme, à vérifier dans un nouveau build autorisé ([contrat CMake](https://cmake.org/cmake/help/latest/variable/CMAKE_LANG_FLAGS_INIT.html)). Vérifier aussi que l'ajout aux flags d'édition de liens préserve leurs défauts. Ajouter au script des contrôles de la présence effective de /EHsc et /Brepro et du refus de C4530, puis comparer les résultats corrigés à l'installation. Cette correction rétablit les défauts perdus par notre ajout de déterminisme ; elle ne change ni pins ni options ISA historiques. Les chemins intégrés et le cas ggml-base restent à qualifier séparément ; aucun mécanisme de remappage non vérifié proposé comme acquis. Chemins de correction futurs : native/build-config.json et ci/build-native.ps1, sans modifier la garde. Aucun accord d'implémentation ou de nouvelle compilation déduit de l'analyse.

Revue indépendante des preuves, avec reproduction de la normalisation en mémoire : **CLEAN pour ces conclusions bornées**. Équivalence statique non établie ; N1, GPU et N2 non exécutés. Verdict **NON REPRODUIT**, pas de fusion recommandée.

[Archive cpu5 et analyse](../architecture/raw/native-build/cpu5-analysis.zip) : **58 preuves et manifeste, 59 entrées**, champs relus depuis le ZIP puis tailles et SHA-256 de chaque entrée vérifiés. Inclut cinq DLL, neuf objets ggml-base, journaux, CMakeCache/Ninja, observations PE/désassemblages, normalisation, avertissements et extraits des sources CMake ; aucun fichier de preuve source supprimé. **6 829 135 octets**, SHA-256 **e5c0bdcd42e777417b85f5192602f465cee7d9dd40d8ee84667b2f9d17aea716**. Espace libre après archive : **518 522 245 120 octets**. Aucun code/configuration changé, aucune suppression, remplacement de l'installation, fusion ou push ; dix rapports conservés, aucun bilan figé réécrit.

Relecture indépendante finale : **CLEAN documentaire**, 58/58 preuves du ZIP vérifiées ainsi que ses 59 entrées, sa taille et son empreinte ; aucun constat ouvert sur la fidélité de ces conclusions. Les causes encore non prouvées restent explicitement ouvertes, sans autorisation implicite de correction ou relance.

## Préservation des options et arrêt C4530

Autorisation « Fais les corrections recommandées », plan `7281f857`. Trois fichiers de code/configuration : `native/build-config.json`, `ci/build-native.ps1`, `ci/test-native-build-flags.ps1`. Les quatre flags complets sont remplacés par leurs *_FLAGS_INIT ; CUDA utilise aussi son initialisateur. Les clés d'écrasement complet sont refusées. Aucun pin, version, ISA ou fichier de garde modifié ; les politiques CMake restent celles du projet amont.

Après CONFIGURE et avant BUILD, le script contrôle les flags du cache global **combinés à Release** : /Brepro, /EHsc C++ et hôte CUDA, définitions Windows et machine x64 à l'édition de liens. Valeurs absentes, tokens partiels, modèles EH contradictoires, désactivations et architecture contraire sont refusés. Le runtime MSVC demandé est aussi comparé au cache. Le manifeste conserve les options effectives et `flagValidation`. Les fixtures positives restent explicitement synthétiques ; aucune configuration CMake réelle n'a été exécutée dans cette correction, ni preuve exhaustive des commandes Ninja par cible.

L'exécution journalisée lit stdout et stderr équitablement, écrit chaque ligne avant contrôle et échoue sur C4530. Le premier test a montré qu'une exception dans l'ancien pipeline PowerShell laissait l'émetteur vivant ; ce candidat a été rejeté, traces conservées. Le passage à un processus explicite et Kill(true) a ensuite été insuffisant : la revue **BN-F01** a démontré un descendant vivant après sortie du parent et expiration des flux. La correction réutilise six fonctions existantes de familles de processus de la garde, importées par AST sans exécuter son entrée. Elle observe uniquement les descendants des identités connues, vérifie les dates de création, refuse un descendant encore actif à la sortie du parent et ferme la famille même après cette sortie. Aucun arrêt par nom de processus. Les reçus `process-control.jsonl` indiquent identités, fermeture et SHA du helper ; cette empreinte est aussi dans le manifeste de build. Délais et politique de garde inchangés, aucun confinement supplémentaire revendiqué.

| Vérification réellement exécutée | Résultat |
| --- | --- |
| SelfTest du script, fixtures flags CPU/GPU, refus et processus réels possédés | 41 contrôles PASS + huit négatifs existants PASS |
| Émetteurs stdout/stderr, parent actif avec descendant, parent déjà sorti | Arrêt vérifié, descendants observés terminés, cleanupClosed=true |
| SelfTest complet sous garde NativeDllCpu | PASS, y compris 1 000 renommages témoins et contrôles existants |
| CheckOnly CPU et GPU | PASS, aucune compilation/configuration native |
| Comparaison des pins, versions, options ISA et fichiers de garde | Inchangés |
| Revue indépendante, deux passages | BN-F01 fermé ; CLEAN sur le delta borné |

Preuves finales : `target/native-build/guard/selftest-83fc5f1ce4374da8b29d8fec46e93ab3/` et flags du worker dans `target/native-build/manifest-selftests/0bc293bf63034f398b417805838c1643/flag-tests/`. Le test intermédiaire parent-exits a bien fermé sa famille mais attendait un autre message que le délai de flux ; le contrôle final refuse maintenant le descendant dès la sortie observée du parent. Les essais intermédiaires et la preuve indépendante de BN-F01 sont conservés ; aucun résultat échoué présenté comme réussi.

[Archive de correction](../architecture/raw/native-build/native-flags-correction.zip) : **118 preuves et manifeste, 119 entrées**, noms uniques, tailles/SHA contrôlés à partir du manifeste relu dans le ZIP ; **1 471 226 octets**, SHA-256 **cf3da6aff6437c3c033298be7f9b65780c4f9b0de59532b066fe2474aee48568**. Espace libre après archive : **516 146 368 512 octets**. Inclut receipts, logs, échecs intermédiaires, checks CPU/GPU et empreintes sources. Aucun nettoyage, remplacement de DLL, Rust, CMake, compilateur natif, Cargo, N2, fusion ou push. Aucune nouvelle DLL reconstruite ; les divergences historiques et le cas ggml-base restent ouverts. Verdict **NON REPRODUIT** ; la véritable construction corrigée reste à autoriser et éprouver.

Vérification indépendante des conclusions et de l'archive : **CLEAN documentaire**, 118/118 preuves, 119 entrées, taille et empreinte confirmées. Aucun code rouvert ou essai relancé pendant cette vérification.

## cpu6 effet réel du correctif et concordance statique

Autorisation « Lance une nouvelle compilation native et vérifie l'effet sur les DLL et leur équivalence ». Une commande CPU6, plan `338c7a9e`, code `647b30ac`, garde inchangée : **PASS/sortie 0**, du **5 novembre 02:05:48,787 à 02:06:53,582 UTC** (03:05–03:06 Paris), **64,7947984 s**. Cinq DLL, manifeste BUILT_NOT_QUALIFIED et flagValidation PASS. Les 63 commandes C++ de compile_commands.json comportent /EHsc ; toutes les 69 commandes C/C++ comportent /Brepro et /MD. **C4530 : 30 dans cpu5 → 0 dans cpu6.** /DWIN32, /D_WINDOWS et /machine:x64 sont conservés dans les flags contrôlés. Pins, versions, patches et ISA inchangés ; restauration des défauts dans leur ensemble, pas isolation expérimentale du seul /EHsc.

**14/14 DLL installées inchangées**, aucune opération de remplacement. **15 038 ALLOWED, 67 UNATTRIBUTED_TEMP**, aucune violation enregistrée. Les deux reçus de processus configure/build ont cleanupClosed=true ; aucun cmake/cl/ninja/vcpkg actif observé après clôture. Le PASS ne prouve pas l'absence universelle de fuite, les limites documentées de la garde restant applicables.

Comparaison en lecture seule des surfaces PE avec l'installation (exports nom+ordinal ; imports DLL+symboles) : **5/5 identiques pour les deux surfaces**. Le contrôle existant sans CUDA, tables directes et différées, passe **5/5** ; aucune résolution récursive ou exécution de DLL n'en est déduite.

| DLL | Octets installés | cpu5 | cpu6 | Écart cpu6/installé | Exports / imports |
| --- | --- | --- | --- | --- | --- |
| ggml-base.dll | 680 960 | 684 032 | 680 960 | 0 % | identiques / identiques |
| ggml-cpu.dll | 947 200 | 947 200 | 947 200 | 0 % | identiques / identiques |
| ggml.dll | 61 952 | 59 392 | 61 952 | 0 % | identiques / identiques |
| nemo_speech_asr.dll | 4 192 256 | 4 114 432 | 4 191 232 | −0,024 % | identiques / identiques |
| nemo_speech_asr_c.dll | 56 320 | 54 272 | 56 320 | 0 % | identiques / identiques |

Les deux noms d'exports supplémentaires de NeMo ont disparu ; son nombre revient à 10 992. Les différences d'imports CRT constatées sur cpu5 ont disparu. Les deux écarts de taille supérieurs à 2 % sont résorbés. Le résidu NeMo de **−1 024 octets est localisé à la taille brute de .rdata** ; les autres sections ont les mêmes tailles, mais leurs empreintes ne sont pas toutes identiques. Cause exacte et innocuité de ce résidu non démontrées ; la taille satisfait seulement le seuil statique de 2 %. Aucun octet de DLL modifié pour obtenir ces résultats.

| DLL | SHA-256 installée | SHA-256 cpu6 |
| --- | --- | --- |
| ggml-base.dll | 398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a | 28c3c1fd4e33ee4e26c8086c05f0aa1536e8b78734533862aafc5e739ff1e68d |
| ggml-cpu.dll | f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0 | 9669eacb3a104e830aeb3b8f519154904024369fa868b6814844640f909d2edc |
| ggml.dll | 2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794 | bb3c723c57000a32b5a1fdc9cff3d70b6fea0579bbeebc87509744f7580e749a |
| nemo_speech_asr.dll | 923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019 | 94a086e43cde1447bb34ad1af15d6566794489fe036a11d070e3a184e06b0512 |
| nemo_speech_asr_c.dll | 9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764 | 4b9c27b4a505865e28025ccd53a98d2859280d69db51c5b576ddb62e2a6b710b |

**Entrée variable supplémentaire établie :** dans la copie patchée sans .git indépendant, ggml/CMakeLists.txt:14–29 appelle Git depuis CMAKE_CURRENT_SOURCE_DIR ; Git remonte à `C:/wt/build-natif`. Le header généré `ggml/src/ggml-version.h` incorpore donc le commit **Whisper** courant. Le getter ggml_commit a été décodé statiquement (mapping PE et LEA x64 vers sa constante, aucune DLL chargée) : installé `6cf94e8b`, cpu4 `77f4413b`, cpu5 `d27a5d9d`, cpu6 `338c7a9e`. Le premier résout dans l'historique Whisper en `6cf94e8be401e95c153d72c17c5c1cd0e57a3e25` ; le vrai cache source llama reste épinglé à `bd4f514db14d87fded667787a7a963bfbaa98e89`. La provenance source ne change donc pas avec ces métadonnées. Une modification documentaire de HEAD peut cependant changer la DLL ; le champ n'est pas une preuve du commit llama. Cela démontre une autre entrée variable, sans expliquer à elle seule les 13 725 anciens octets .text de ggml-base.

**Conclusion bornée :** le correctif est efficace sur les options et sur les écarts de surfaces/tailles observés. La concordance statique est forte, **pas une équivalence complète** : SHA différents 5/5, une seule construction du candidat corrigé, options historiques complètes toujours indisponibles et aucune comparaison TXT/SRT/WAV. Répétabilité corrigée **NON EXÉCUTÉE**, fonctionnel/N2, N1 et GPU **NON EXÉCUTÉS**. Les chemins et la métadonnée dépendant du dépôt parent restent à stabiliser/qualifier ; aucune correction spéculative ou seconde compilation engagée. Revue indépendante des comparaisons PE et du décodage : **CLEAN pour ces conclusions bornées**. Verdict global **NON REPRODUIT**, pas de fusion ni levée de conservation recommandée.

[Archive cpu6 statique](../architecture/raw/native-build/cpu6-static.zip) : **41 preuves et manifeste, 42 entrées**, noms uniques et tailles/SHA vérifiés à partir du manifeste relu depuis le ZIP ; cinq DLL, journaux/garde, cache et commandes CMake/Ninja, comparaisons PE, header/métadonnées, pré/postflight et manifeste de build. **3 853 891 octets**, SHA-256 **6a2afe1d9d5b78486fc916b13200379441022dbfb379842e8dcc99e919e52b48**. Espace libre après archive : **513 831 768 064 octets**. Aucun code corrigé après cet essai, aucune autre compilation, suppression, modification de l'installation, fusion ou push ; dix rapports conservés et bilans figés inchangés.

Vérification indépendante finale : **CLEAN documentaire**, 41/41 preuves, 42 entrées, taille et empreinte confirmées ; 63/63 commandes C++ avec /EHsc et 69/69 commandes C/C++ avec /Brepro et /MD. Aucun fichier modifié ou essai exécuté pendant cette revue ; aucune équivalence complète revendiquée.
