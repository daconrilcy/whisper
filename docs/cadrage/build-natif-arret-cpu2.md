Avancée : relance cpu2 autorisée et exécutée ; bootstrap vcpkg réussi et identité de l'exécutable vérifiée.
Blocage : configuration CMake interrompue sur un renommage dont le nouveau chemin journalisé est la racine de l'observateur.
Décision : NON REPRODUIT ; campagne suspendue, diagnostic complémentaire proposé sans assouplir la garde.

# BUILD-NATIF — bilan figé de cpu2, 4 novembre 2026

Après « oui je t'autorise », exécution de `./ci/build-native.ps1 -Variant cpu -BuildId cpu2` depuis le worktree, candidat `e8469120`. Aucun correctif ni relance après cet arrêt.

| Étape | Résultat prouvé |
| --- | --- |
| Bootstrap corrigé | PASS ; vcpkg.exe SHA-256 `13b8175e99a884c5ad34249218754b45541a1a63f216e92603aee57a285ac741`, conforme au pin |
| Configuration | CMake lancé ; vcpkg a préparé ses deux ports CMake et commencé la configuration d'Abseil |
| Arrêt | Renamed détecté à **19:50:02.3415863 UTC** (20:50 à Paris), événement à 19:50:02.2693716 UTC ; arbre arrêté |
| Production / équivalence | Zéro DLL dans cpu2 ; exports/imports reconstruits, GPU, N1 et N2 NON EXÉCUTÉS |
| Installation | 14/14 SHA-256 inchangés ; aucun remplacement ni nettoyage |

**Événement :** ancien chemin `native/cache/vcpkg/buildtrees/abseil/x64-windows-static-md-rel/CMakeFiles/CMakeScratch/TryCompile-xcpmr4/CMakeFiles/TargetDirectories.txt.tmpb24ff` sous le worktree (164 caractères absolus) ; nouveau chemin enregistré `C:/wt/build-natif/native/`. Le dossier native existe toujours. La destination effective n'est pas établie par ces seules données ; ne pas présenter cet événement comme une fuite prouvée ou un faux positif démontré.

**Diagnostic limité :** la garde conserve FullPath/OldFullPath mais pas Name/OldName ni la racine de l'observateur. Un test de constructeur .NET sans écriture reproduit un FullPath égal à la racine lorsque Name est vide. Commande copiable :

```powershell
[IO.RenamedEventArgs]::new([IO.WatcherChangeTypes]::Renamed,'C:\wt\build-natif\native','','cache\example.tmp') | Select-Object Name,FullPath,OldName
```

Le résultat confirme une ambiguïté possible, sans prouver que c'est ce qui s'est produit dans cpu2. Le [code Microsoft de FileSystemEventArgs](https://raw.githubusercontent.com/dotnet/runtime/main/src/libraries/System.IO.FileSystem.Watcher/src/System/IO/FileSystemEventArgs.cs) montre cette construction du chemin. Environnement du test : PowerShell 7.6.9, .NET 10.0.14. Aucun changement de politique d'arrêt, nouvelle surveillance ou essai natif de diagnostic.

**Lacune supplémentaire :** le manifeste du worker cpu2 est absent après l'interruption forcée ; son écriture était différée au finally. Les arguments CMake et le résultat FAIL de la garde sont conservés. Pour rendre un prochain échec observable, proposer de persister un manifeste STARTED avant les opérations externes.

**Archive :** [cpu2-attempt.zip](../architecture/raw/native-build/cpu2-attempt.zip), SHA-256 `3346324d95cbebcd1fc8a89396dafdae6c29c067603a9dc177babb8c5986453e` ; 30 traces, tailles et SHA vérifiés, plus manifeste (31 entrées). Espace libre après archivage : 530 838 962 176 octets. Sources/cache et cpu1/cpu2 conservés, aucune suppression exécutée ; archive originale spike02 toujours impossible. Aucun processus cmake/cl/ninja/vcpkg observé actif lors du contrôle après arrêt.

Revue indépendante des conclusions et des preuves : **CLEAN documentaire**, sans validation de la destination ni levée de l'arrêt.

**Arbitrage recommandé :** autoriser une instrumentation minimale de la garde existante (noms bruts, indicateurs null/vide, racine observée) et la persistance préalable du manifeste, avec tests ciblés et revue avant un seul essai diagnostique cpu3 sous garde. Maintenir l'arrêt sur renommage incomplet ou traversant ; ne pas déduire une destination depuis le nom du temporaire. Alternative : conserver BUILD-NATIF suspendu. Pas de fusion recommandée ni demande de suppression du spike. Ces corrections et cet essai ne sont pas exécutés par ce bilan.
