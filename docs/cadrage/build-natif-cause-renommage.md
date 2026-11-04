Avancée : collecte NativeBuild par identité implémentée, auto-tests réussis et revue indépendante CLEAN.
Blocage : correctif non éprouvé en compilation native ; déclencheur Win32 exact de cpu3 non enregistré.
Décision : correction bornée livrée ; aucune relance native, équivalence des DLL toujours non démontrée.

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

Preuves finales : `target/native-build/guard/selftest-ef0bb8ff5cae4e98a167578c3b9b5443/`. [Archive de validation](../architecture/raw/native-build/native-collector-validation.zip), 39 fichiers et manifeste, **40 entrées**, tailles/SHA-256 vérifiés ; 2 752 626 octets ; SHA-256 `b945e3848876b5c5270e4bd61629d901dd6f0a6d2c1a94112e89171fcaccd639`. Les essais antérieurs sont inclus ; l'archive diagnostique précédente est conservée. Espace libre après archive : 530 404 126 720 octets. Aucun nettoyage, CMake, Cargo, N2, remplacement de DLL, fusion ou push. Prochaine compilation native non engagée par cet accord.

Vérification indépendante des conclusions et de cette archive : **CLEAN documentaire**. Les empreintes sources archivées correspondent au candidat relu ; aucun code rouvert lors de cette vérification.
