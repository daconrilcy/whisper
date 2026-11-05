Avancée : complément limité de transmission vcpkg accepté explicitement par l'utilisateur.
Blocage : perte de VSCMD_SKIP_SENDTELEMETRY dans vcpkg non encore démontrée ; aucune correction à appliquer sans témoin.
Décision : comparer les deux environnements, puis conserver uniquement cette variable si la perte est confirmée ; garde inchangée.

# BUILD-NATIF — complément environnement vcpkg

Autorisation reçue dans cette session : deux préparations vcpkg réelles avec/sans conservation de VSCMD_SKIP_SENDTELEMETRY ; si le témoin confirme sa perte, ajouter seulement `VCPKG_KEEP_ENV_VARS=VSCMD_SKIP_SENDTELEMETRY` au pilote et au manifeste, revue indépendante puis les deux compilations CPU restantes. Aucun changement de collecteur, arrêt, pin, version, patch source ou ISA. Arrêt au premier échec inattendu.

1. Témoin réel via l'exécutable vcpkg épinglé et vérifié par SHA : `vcpkg env` non interactif, triplet existant inchangé, profils/temporaires isolés sous le lot, seule différence KEEP_ENV_VARS. Exiger absence de la variable sans conservation, valeur 1 avec conservation ; versions MSVC/SDK identiques. Aucun port compilé. Résultats et codes conservés.
2. Seulement si PASS : modification bornée de `ci/build-native.ps1`, test de nécessité dans `ci/test-native-workspace.ps1`, manifeste des deux variables ; revue indépendante, deux cycles maximum pour ce complément. Aucun autre paramètre natif ajouté.
3. CPU7c3 puis CPU8 sous la garde publique, espaces neufs au même chemin et code identique entre les deux. Compteurs cumulés : deux constructions CPU commencées sur quatre ; ces deux appels sont les derniers autorisés. Vérifier commit décodé, SHA 5/5, surfaces PE selon les critères acceptés, contrôle CPU sans CUDA.
4. Si réussite et temps restant : N2 CPU référence/reconstruit, puis N1 et GPU selon le plan initial et ses points d'arrêt. Zéro N2/GPU exécuté à l'ouverture de ce complément. Dix cas N2 et deux builds GPU au maximum ; borne conservatrice conservée **11:00 UTC / 12:00 Paris**. Si le temps manque, arrêter et signaler les NON EXÉCUTÉS.
5. Nouveau bilan/archives vérifiés et revue des conclusions. Les bilans figés précédents, archives existantes, installation et dossiers protégés restent intacts ; aucune fusion, suppression ou installation.

Le mécanisme documenté par Microsoft conserve une liste explicite de variables dans l'environnement nettoyé : [VCPKG_KEEP_ENV_VARS](https://learn.microsoft.com/en-us/vcpkg/users/config-environment), [vcpkg env](https://learn.microsoft.com/en-us/vcpkg/commands/env). Les valeurs ainsi conservées ne changent pas les ABI calculées par vcpkg ; le réglage est destiné seulement au lanceur de télémétrie et sera figé/consigné, pas aux options de compilation.
