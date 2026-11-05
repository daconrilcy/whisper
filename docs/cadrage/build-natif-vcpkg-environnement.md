Avancée : les deux préparations vcpkg réelles réussissent, avec mêmes versions MSVC/SDK.
Blocage : la variable vaut déjà 1 sans conservation explicite ; hypothèse de perte non reproduite dans ce témoin.
Décision : condition du correctif non remplie ; aucun KEEP ajouté, aucune compilation relancée, garde inchangée.

# BUILD-NATIF — complément environnement vcpkg

Autorisation reçue dans cette session : deux préparations vcpkg réelles avec/sans conservation de VSCMD_SKIP_SENDTELEMETRY ; si le témoin confirme sa perte, ajouter seulement `VCPKG_KEEP_ENV_VARS=VSCMD_SKIP_SENDTELEMETRY` au pilote et au manifeste, revue indépendante puis les deux compilations CPU restantes. Aucun changement de collecteur, arrêt, pin, version, patch source ou ISA. Arrêt au premier échec inattendu.

1. Témoin réel via l'exécutable vcpkg épinglé et vérifié par SHA : `vcpkg env` non interactif, triplet existant inchangé, profils/temporaires isolés sous le lot, seule différence KEEP_ENV_VARS. Exiger absence de la variable sans conservation, valeur 1 avec conservation ; versions MSVC/SDK identiques. Aucun port compilé. Résultats et codes conservés.
2. Seulement si PASS : modification bornée de `ci/build-native.ps1`, test de nécessité dans `ci/test-native-workspace.ps1`, manifeste des deux variables ; revue indépendante, deux cycles maximum pour ce complément. Aucun autre paramètre natif ajouté.
3. CPU7c3 puis CPU8 sous la garde publique, espaces neufs au même chemin et code identique entre les deux. Compteurs cumulés : deux constructions CPU commencées sur quatre ; ces deux appels sont les derniers autorisés. Vérifier commit décodé, SHA 5/5, surfaces PE selon les critères acceptés, contrôle CPU sans CUDA.
4. Si réussite et temps restant : N2 CPU référence/reconstruit, puis N1 et GPU selon le plan initial et ses points d'arrêt. Zéro N2/GPU exécuté à l'ouverture de ce complément. Dix cas N2 et deux builds GPU au maximum ; borne conservatrice conservée **11:00 UTC / 12:00 Paris**. Si le temps manque, arrêter et signaler les NON EXÉCUTÉS.
5. Nouveau bilan/archives vérifiés et revue des conclusions. Les bilans figés précédents, archives existantes, installation et dossiers protégés restent intacts ; aucune fusion, suppression ou installation.

Le mécanisme documenté par Microsoft conserve une liste explicite de variables dans l'environnement nettoyé : [VCPKG_KEEP_ENV_VARS](https://learn.microsoft.com/en-us/vcpkg/users/config-environment), [vcpkg env](https://learn.microsoft.com/en-us/vcpkg/commands/env). Les valeurs ainsi conservées ne changent pas les ABI calculées par vcpkg ; le réglage est destiné seulement au lanceur de télémétrie et sera figé/consigné, pas aux options de compilation.

## Résultat du complément

Exécutable vérifié par SHA-256 `13b8175e99a884c5ad34249218754b45541a1a63f216e92603aee57a285ac741`, appel `env --triplet=x64-windows-static-md`, deux profils isolés dont seul le réglage fonctionnel KEEP diffère ; leurs chemins d'isolation et dossiers courants sont distincts. Sans KEEP, stdout contient déjà `VSCMD_SKIP_SENDTELEMETRY=1` ; avec KEEP, deux lignes de cette même valeur sont présentes. Les deux commandes retournent **0**, MSVC `14.44.35207` et SDK `10.0.26100.0` identiques. Le témoin échoue donc à démontrer la perte attendue : **FAIL du témoin de nécessité, pas échec de lancement de vcpkg**.

La condition de l'autorisation n'étant pas remplie, arrêt sans ajout KEEP ni changement de manifeste ou de pilote. La fonction de diagnostic a été conservée avec les preuves puis sa seule addition préparatoire à `ci/test-native-workspace.ps1` a été retirée après comparaison exacte du reste avec HEAD. Aucun diff dans ci/, native/ ou le code Rust/Cargo. Le témoin couvre l'appel env du triplet cible ; il ne démontre pas le comportement des environnements internes de ports ou du triplet hôte. L'origine de la télémétrie observée reste indéterminée ; ne pas corriger au hasard ces autres chemins.

Revue indépendante des preuves et conclusions : **CLEAN**, arrêt justifié. Pas de nouvelle compilation CPU/GPU ni de cas N2, N1 non exécuté ; compteurs cumulés toujours 2 CPU commencées sur 4, zéro GPU, zéro N2. Les deux compilations restantes ne sont pas consommées. Le bilan antérieur reste figé ; nouveau [bilan du témoin](../architecture/BUILD-NATIF-VCPKG-ENV-BILAN.md).
