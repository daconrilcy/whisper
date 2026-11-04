Avancée : renommages incomplets reproduits hors compilation ; mécanisme FileSystemWatcher identifié.
Blocage : déclencheur Win32 exact de cpu3 non enregistré ; correctif de collecte à implémenter et qualifier.
Décision : recommander la collecte par identité de fichier ; aucun assouplissement ni nouvel essai natif.

# BUILD-NATIF — diagnostic du nom null et proposition, 4 novembre 2026

Demande : expliquer le nom null et proposer un correctif. Aucun changement de garde ou lancement CMake dans ce diagnostic. Ce document est le dixième rapport du lot ; tout rapport supplémentaire nécessite arbitrage. Le bilan cpu3 reste figé.

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

## Correctif recommandé, à autoriser avant écriture

Remplacer la collecte FileSystemWatcher **pour NativeBuild uniquement** par un adaptateur Windows `ReadDirectoryChangesExW`, information étendue avec FileId. [Microsoft recommande cet identifiant pour réunir les deux notifications](https://devblogs.microsoft.com/oldnewthing/20260508-00/?p=112310). Un composant d'interopérabilité versionné sous `ci/`, chargé par `ci/guard-native.ps1`, constitue un nouveau périmètre technique : aucun service, privilège supplémentaire ou surveillance supplémentaire proposé.

1. Conserver exactement racines, récursivité et filtres existants ; journaliser les enregistrements bruts, identités, séquences et frontières de lecture/drainage. Vérifier Windows/API et NTFS pour **toutes** les racines avant lancement ; indisponibilité = refus, sans repli silencieux. [Limites de l'API](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-readdirectorychangesexw).
2. Définir un drainage comme un instantané fini des notifications déjà reçues au contrôle courant. Assembler uniquement OLD/NEW complets, ordonnés et non ambigus de la même identité de fichier et du même volume, dans la même session de surveillance. Les frontières de tampon et événements intercalés ne détruisent plus la paire **si ses deux moitiés sont déjà disponibles**. Ne pas attendre une livraison ultérieure, ne pas maintenir une moitié au-delà du contrôle, ne pas allonger les délais. Même FileId ne suffit pas en présence de plusieurs appariements possibles ; refuser.
3. Transmettre la paire complète au classement existant, sans modifier `Get-EventDisposition`, profils, racines ou exceptions Temp. Une destination hors périmètre reste bloquante. Orphelin à la fin du drainage, identité insuffisante, ambiguïté, paquet invalide, perte, zéro octet ou erreur API = FAIL et interruption actuelle. Ni proximité temporelle, ni nom similaire, ni état disque final ne permettent d'autoriser un événement incomplet.
4. Qualifier sans CMake : OLD/NEW séparés par une action et par deux tampons déjà reçus ; moitiés manquantes ; identités différentes ; renommages répétés/ambigus ; notifications invalides et débordement ; écritures refusées, renommages traversant une frontière, règles Temp inchangées. Puis témoin réel sur chaque racine et stress concurrent avec compte attendu, collecte jusqu'à quiescence et drainage final vérifié. Réutiliser ce rapport et son archive pour les résultats. Revue indépendante avant toute demande de relance native.

Cette proposition corrige une limite d'assemblage, sans garantir que toute compilation passera : une moitié réellement absente devra encore arrêter. Un besoin d'attente supplémentaire serait une autre décision, non proposée ici. Versions, options CMake, sources, Rust produit et DLL installées restent hors du correctif.

## Conservation et statut

[Archive des observations](../architecture/raw/native-build/rename-collector-diagnostic.zip) : 5 fichiers de preuves et manifeste, 6 entrées vérifiées par tailles/SHA-256 ; 474 879 octets ; SHA-256 `c24d63c8c6d235f6126f5293549168d5dc83fa2d2d371f83c7d91481f79a2607`. Volume C: NTFS ; 530 753 126 400 octets libres après archive. Aucune suppression, fusion, publication ou compilation. Verdict BUILD-NATIF inchangé : **NON REPRODUIT**.

Revue indépendante : premier constat sur l'attente intertampons intégré dans le contrat de drainage ci-dessus ; second passage **CLEAN documentaire**, constat BN-D01 fermé, rapport/renvois/archive vérifiés. Contrôle `git diff --check` réussi. Aucun correctif d'exécution livré par ce diagnostic.
