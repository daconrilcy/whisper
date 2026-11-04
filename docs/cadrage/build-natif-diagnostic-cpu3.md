Avancée : noms bruts et manifeste préalable implémentés, testés et relus ; unique essai cpu3 effectué.
Blocage : nouveau nom réellement null dans une notification de renommage ; destination effective inconnue.
Décision : diagnostic terminé, aucune relance ; NON REPRODUIT pour la chaîne native.

# BUILD-NATIF — bilan figé du diagnostic cpu3, 4 novembre 2026

Autorisation précise reçue pour l'observation brute, le manifeste préalable et **un seul** essai sous garde, sans assouplissement. Plan `3a5d460e`, candidat `be51d504`. Seuls `ci/guard-native.ps1` et `ci/build-native.ps1` modifiés : capture des noms/racine et snapshots atomiques aux étapes du worker. Aucun changement des profils, racines, options natives ou versions.

| Vérification | Résultat réel |
| --- | --- |
| Contrôles et revue avant cpu3 | 8 négatifs du script, snapshots lisibles/remplacement/refus hors racine ; garde : noms complets/null/vides, refus inchangés et témoin réel ; revue indépendante CLEAN |
| Invariants de la garde | Texte des fonctions Get-EventDisposition, Get-RunWatchSpecs, Assert-NativeBuildProfile et New-NativeBuildProcessInfo identique à e015bfae |
| Essai unique cpu3 | FAIL sous garde ; 21:05:42–21:06:00 heure de Paris ; CMake en préparation d'Abseil, aucun payload ggml/NeMo reconstruit |
| Observation décisive | 8 778 événements enregistrés, 56 Renamed ; un seul nouveau nom null, aucun nouveau nom vide |
| Manifeste après arrêt | Lisible : status STARTED, stage CONFIGURE, 36 options demandées, trois pins contrôlés et 19 patches ; résultat de garde FAIL, aucune réussite déduite du snapshot |
| Installation / suites | 14/14 SHA-256 installés inchangés ; aucune autre compilation, GPU, N1 ou N2 |

**Événement bloquant :** le 4 novembre à **20:05:58.3729840 UTC** (21:05:58 à Paris), args_type `System.IO.RenamedEventArgs`, watcher_root `C:/wt/build-natif/native`, raw_name **null**, name_is_null **true**, name_is_empty **false**. raw_old_name est `cache/vcpkg/buildtrees/abseil/x64-windows-static-md-rel/CMakeFiles/CMakeScratch/TryCompile-w9d121/CMakeFiles/rules.ninja.tmp0a2bd`. La garde interrompt l'arbre à la détection (20:05:58.9204782 UTC).

**Conclusion établie :** la notification cpu3 est incomplète ; `native/` dans FullPath correspond à la racine de l'observateur combinée au nom absent, pas à une destination effectivement fournie. La destination réelle et la cause de l'absence du nom restent inconnues. Ce résultat ne démontre ni fuite réelle ni innocuité, et n'attribue pas rétroactivement les événements cpu1/cpu2. Les autres renommages complets sont conservés pour comparaison. Règles d'arrêt et périmètres strictement maintenus.

**Archive vérifiée :** [cpu3-diagnostic.zip](../architecture/raw/native-build/cpu3-diagnostic.zip), SHA-256 `c97a8133d6a159f39064d28c797a6f01dfb9df2d7e5e7525a05c0c96d6109147` ; 30 traces/fixtures, tailles et SHA vérifiés, plus manifeste (31 entrées), 170 185 octets. Inclut le manifeste CONFIGURE et les observations brutes. Espace libre après archivage : 530 789 261 312 octets. Aucun processus cmake/cl/ninja/vcpkg observé actif après arrêt. Aucun nettoyage, fusion, push, remplacement de DLL ou suppression du spike.

Revue indépendante des conclusions, comptages, manifeste et archive : **CLEAN documentaire**, sans levée de l'arrêt ni autorisation de suite.

L'autorisation diagnostique unique est consommée. Pas de fusion recommandée en l'état ; aucune levée de conservation. La poursuite du lot reste suspendue jusqu'à une décision explicite sur un diagnostic complémentaire permettant d'établir les destinations, sans considérer les noms absents comme des écritures autorisées. Aucun essai supplémentaire proposé comme déjà autorisé. Tout fait ultérieur ira dans un nouveau document.

Complément ultérieur : [cause du nom null et proposition de correction](build-natif-cause-renommage.md).
