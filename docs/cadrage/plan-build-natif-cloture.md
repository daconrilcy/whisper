Avancée : T1 codé et relu ; auto-tests et témoin MSVC corrigé PASS ; une construction CPU commencée.
Blocage : cpu7 interrompu puis cpu7b refusé avant lancement sur une jonction Windows interne.
Décision : arrêt sur prérequis après deux cycles ; bilan de clôture distinct, aucune fusion ni suppression.

# Plan BUILD-NATIF-CLOTURE

Objectif accepté : **ÉQUIVALENT ET REPRODUCTIBLE** si surfaces PE conformes à l'installation, sorties TXT/SRT/WAV identiques sur les recettes définies et deux constructions successives identiques octet pour octet. Autres verdicts : ÉQUIVALENT NON REPRODUCTIBLE / NON ÉQUIVALENT. L'identité binaire avec les DLL installées n'est pas recherchée.

Worktree existant `C:/wt/build-natif`, branche `build-natif`, départ `c085347d`. Main relevé à `b9935b2422d47350cdd5b368265afa84e12d42ab` ; aucun écart Rust/Cargo avec main lors du relevé. Budget total **4 h**, **4 constructions CPU**, **2 GPU**, **10 essais N2**, **2 cycles de correction**. Compteurs au cadrage : **0 / 0 / 0**. Les lectures de préparation ne compilent rien ; `cl /?` a seulement affiché son aide. Espace libre initial : **512 234 889 216 octets**, NTFS. Vérifier au moins 30 Gio avant chaque campagne et la place nécessaire aux copies.

Le présent plan traduit l'autorisation globale de l'utilisateur pour les exécutions listées ; il ne crée pas d'autorisation de passer un blocage ni de modifier une clause explicitement maintenue. `docs/cadrage/build-natif-cause-renommage.md` devient strictement lecture seule. Archives existantes, installation, `target/parakeet-feasibility/` et `target/poll-preserved/` préservés. Aucun code Rust produit, version, pin, correctif historique ou option ISA changé ; GGML_NATIVE reste ON. Aucun push, fusion ni nettoyage final exécuté.

## Choix technique unique pour les chemins

**Chemin physique fixe retenu : `C:/wb/native/cpu` et `C:/wb/native/gpu`.** Sources matérialisées, sources patchées, répertoire CMake, outils vcpkg compilant des dépendances et temporaires doivent utiliser des chemins constants à l'intérieur de cet espace. Les caches de téléchargement restent persistants et explicitement déclarés ; ne pas faire dépendre une preuve de reproductibilité d'une bibliothèque binaire fabriquée antérieurement dans un autre chemin non attesté. Les sorties/manifeste de chaque essai restent sous `target/native-build/<variante>/<id>/`.

Ce choix évite un remappage MSVC insuffisamment documenté et garde les options de génération de code. L'aide du cl 19.44.35229 et son cl.xml installé ne décrivent pas d'option native de remappage ; les notes Microsoft 17.1 mentionnent /pathmap avec /clr mais n'établissent pas ici un contrat pour ce build C/C++ natif. Aucune option expérimentale supposée n'est retenue.

Chaque construction doit repartir d'un espace neuf au **même chemin physique**. Préserver l'espace précédent par déplacement vers sa sortie de lot, sans suppression, après vérification des chemins absolus, absence de liens et de processus utilisateurs ; puis recréer le chemin fixe. Ne jamais déplacer une référence protégée ou un chemin non possédé. Les mesures et copies des DLL sont faites avant ce déplacement. Cette préparation du workspace n'est pas un nettoyage final des preuves.

## Blocages établis avant implémentation

1. **Racines NativeBuild incompatibles avec le chemin retenu.** `ci/guard-native.ps1:3460` impose les six champs exacts du profil, dont cache=`native/cache`, outputs=`target/native-build`. `Invoke-NativeBuildProcess`, ligne 3524 et suivantes, ne déclare en écriture que `$script:Lot` et `$CacheRoot`. `C:/wb/native/<variante>` n'appartient à aucune de ces racines. Y écrire sans changer le contrat violerait la garde même si aucun observateur ne le voyait. `ci/build-native.ps1` impose aussi ses chemins de sortie sous le dépôt. Ni jonction, subst, exécution directe hors garde ni alias de chemin ne servent de contournement.
2. **Épinglage N2 rigide de l'ancien runtime.** PersonalAutoFallback, PersonalUnified et PersonalUnifiedGpuOnly fixent `requiredFileCount=21`, `requiredBytes=3196008875` dans `ci/native-profiles.json`. `ci/guard-native.ps1:1151` refuse tout autre total, en plus de vérifier les SHA individuels. Une copie contenant les DLL cpu6 est déjà plus petite de 1 024 octets. Les futurs artefacts stabilisés doivent donc être épinglés séparément selon leurs tailles et empreintes réelles, sans maquiller les fichiers pour conserver l'ancien total.
3. **Sélection CPU courte.** PersonalUnified contient les trois recettes FR CPU, EN GPU attesté et Auto→CPU ; PersonalAutoFallback ne contient que le repli. Pour exécuter exactement les deux recettes CPU sans dépendre d'un GPU, il manque un profil CPU correspondant. Les sélections Rust existent ; aucun nouveau test ou code Rust n'est nécessaire. Le plan doit conserver le refus de préparer artificiellement settings/logs au premier lancement.

Ces constats empêchaient de garantir simultanément la démarche demandée et « garde inchangée ». L'utilisateur a arbitré ci-dessous avant toute implémentation.

## Arbitrage proposé, borné et vérifiable

**Recommandation : autoriser une exception précise à « garde inchangée », sans alléger ses contrôles.** Adapter uniquement les déclarations/contrats nécessaires dans `ci/guard-native.ps1` et `ci/native-profiles.json` :

- déclarer, valider canoniquement et surveiller les seuls workspaces fixes CPU/GPU ci-dessus ; aucune racine globale `C:/`, aucun lien accepté, mêmes classements et arrêts ;
- ajouter les profils de clôture aux sélections exactes existantes : CPU = FR court + Auto→CPU ; GPU = EN avec candidat attesté ;
- épingler séparément les runtimes référence et reconstruits au moyen des manifestes attestés : mêmes 21 chemins, tailles et SHA vérifiés, seules les 5 DLL CPU ou 6 DLL ggml/NeMo GPU remplacées ; trois DLL CUDA copiées identiques à l'installation. Aucun total libre, fichier supplémentaire ou exception de hash ;
- conserver les délais, l'isolation, la vérification des identités, les refus de C4530, les règles Temp et l'interruption ; tests négatifs et revue indépendante avant exécution.

Alternative : maintenir la garde absolument inchangée ; arrêter la réalisation avant code/compilation tant qu'une autre solution conforme, documentée et vérifiable n'est pas établie. Aucune autorisation d'assouplissement général n'est demandée.

**Arbitrage reçu : « Autoriser cette extension bornée (recommandé) »** à la question précise visant `ci/guard-native.ps1` et `ci/native-profiles.json`, les deux racines fixes, l'épinglage par tailles/SHA des runtimes reconstruits et les sélections CPU/GPU exactes, sans modification des règles d'arrêt. Cette exception lève les blocages déclaratifs ; toutes les compilations et recettes du tableau sont autorisées dans leurs budgets. Le reset du workspace déplace les intermédiaires possédés et conserve les preuves ; aucun nettoyage final exécuté. Compte tenu de la préparation déjà consommée, borne conservatrice de travail pour ce passage : **5 novembre 11:00 UTC (12:00 Paris)** au plus tard.

## Tâches et points d'arrêt après arbitrage

| Tâche | Réalisation autorisée par le lot | Gate / point d'arrêt |
| --- | --- | --- |
| T0 — cadrage/préflight | État Git, espace, sources/outils, contrats des gardes, plan versionné ; confirmer les pins et les entrées de recettes réelles | Arrêt actuel sur les incompatibilités ci-dessus |
| T1 — stabilisation | `ci/build-native.ps1`, `native/build-config.json`, correctif versionné sous `native/patches/` et tests CI séparés ; adaptations déclaratives de garde seulement si arbitrées | Pins/outils/19 correctifs historiques inchangés ; tests et revue du code, deux cycles maximum |
| T2 — CPU répétable | Deux commandes CPU successives cpu7/cpu8 ; deux workspaces neufs au chemin fixe ; copier les sorties distinctes et leurs manifestes | SHA identiques 5/5 ; sinon localiser les différences et n'utiliser les deux réserves CPU que pour une cause identifiée et corrigeable ; arrêt sinon |
| T3 — CPU fonctionnel | Runtimes référence/alternatif dédiés sous le lot ; seules cinq DLL CPU de cpu8 changent ; deux recettes × deux runtimes, soit **4 N2** ; SHA TXT/SRT/WAV et mêmes assertions PASS, durées relevées | Arrêt sur garde, test, entrée manquante ou sortie différente ; aucune écriture dans l'installation |
| T4 — N1 | Gardes N1 existantes, même sélection que main : tests par défaut, worker-cpu, worker-parakeet-cpu, worker-common ; contrôle PE des harnais et inventaires requis | Nombres, identités et résultats identiques à main ; aucune réussite déduite du seul diff Rust vide |
| T5 — GPU | Si budget restant : deux constructions GPU neuves ; SHA identiques 6/6, surfaces PE ; copier les trois DLL CUDA du toolkit attendu et vérifier les SHA installés ; EN GPU référence/alternatif, soit **2 N2** | Candidat matériel attesté obligatoire ; sinon NON EXÉCUTÉ motivé ; aucun changement CUDA/llama/ISA |
| T6 — revue/clôture | Revue indépendante des conclusions ; archive vérifiée depuis son manifeste relu, bilan figé d'une page et recommandation de fusion ; proposition de nettoyage seulement | Compteurs et temps vérifiés ; quatre réserves N2 au maximum, jamais des répétitions non expliquées ; aucun push/fusion/remplacement/suppression |

Les commandes de builds restent une par variante, via le script public et la garde. Les compteurs concernent les constructions des DLL et les **cas** N2 réellement exécutés, pas seulement le nombre d'appels de profil ; la préparation des harnais N1/N2 reste soumise à sa garde et au budget temps. Pas de nouvelle demande entre étapes valides ; tout échec ou prérequis manquant suspend l'étape concernée.

## Détail de T1 et preuves attendues

### Arbitrages pendant exécution

- Lancement prématuré des auto-tests N2 pendant le SelfTest NativeBuild : arrêt conforme sur création du dossier N2 hors des racines de cette invocation. Aucun build ni cas N2 exécuté. L'utilisateur autorise ensuite explicitement la reprise strictement séquentielle ; les deux auto-tests repassent séparément.
- cpu7 démarre le 5 novembre à 07:51 UTC puis s'arrête à 07:53 UTC pendant la compilation des dépendances : quatre notifications OLD/NEW/OLD/NEW pour une même identité dans un drainage, sous VSApplicationInsights. Le collecteur refuse explicitement ce cas ; aucune règle n'est changée. Compteur : **1 tentative CPU**, aucune des cinq DLL produite par cette tentative.
- L'utilisateur autorise un correctif limité : `VSCMD_SKIP_SENDTELEMETRY=1` avant la préparation MSVC, comparaison réelle de deux préparations et reprise si PASS. Premier témoin fondé sur les fichiers restants : insuffisant/FAIL. Répétition explicitement autorisée avec mesure corrigée : mode diagnostic Microsoft identique dans les deux contextes, branche télémétrie synchrone présente sans variable, absente avec variable, versions MSVC/SDK identiques : PASS. Pas de compilation dans ces comparaisons. Cette preuve démontre la suppression du lanceur ; elle n'attribue pas rétroactivement un fichier ni ne garantit l'absence d'autres renommages répétés.
- Réserve de construction CPU : **cpu7b**, puis **cpu8**, espaces neufs au même chemin ; maximum initial de quatre tentatives maintenu. Les trois premiers commits de correction du patch ne sont jamais compilés avant applicabilité vérifiée ; le patch final passe `git apply --check` et `git diff --check`.
- cpu7b est refusé avant lancement : Windows a créé une jonction `userprofile/AppData/Local/Microsoft/Windows/INetCache/Content.IE5` vers `INetCache/IE` interne à l'espace cpu7 ; la préservation refuse toute jonction. Aucun déplacement, nouvelle compilation ou nettoyage. Les deux cycles de correction étant consommés, arrêt T2–T5 ; un seul build CPU commencé, zéro GPU, zéro N2, N1 non exécuté. La suite exige une politique explicite de préservation pour ce cas, sans déduire un droit d'assouplir les arrêts. Voir [bilan figé](../architecture/BUILD-NATIF-CLOTURE-BILAN.md).

Le ggml épinglé appelle Git sans respecter une valeur préalablement fournie : ggml/CMakeLists.txt:14–29 écrit toujours GGML_BUILD_COMMIT. Prévoir donc un petit patch versionné qui respecte la valeur explicitement passée par le script, sans la remplacer par HEAD Whisper ou un suffixe dirty. Passer le **commit llama complet**. Appliquer ce patch à la source matérialisée, après les 19 correctifs historiques restés identiques, et distinguer historique/local dans le manifeste ; ne jamais modifier les checkouts de cache ni les anciennes traces. Décoder le getter compilé après chaque construction, sans chargement de DLL, et exiger l'égalité exacte au pin complet.

Le contrôle de reproductibilité ne normalise ni ne modifie les DLL : comparer les SHA des artefacts réels. Vérifier aussi les références de chemins, flags globaux/Release et commandes de compilation ; garder les limites explicites des inspections. Pour l'équivalence, comparer exports nom+ordinal et imports DLL+symboles (directs/différés), appliquer le contrôle PE CPU sans CUDA, et comparer les fichiers de transcription, pas les journaux temporels. Les workers, modèles, audios et DLL CUDA hors remplacement gardent leurs empreintes.

Livrables de ce lot : ce plan ; code/tests bornés ; archives sous `docs/architecture/raw/build-natif-cloture/` ; mise à jour du manuel `docs/architecture/NATIVE-BUILD.md` ; **nouveau** `docs/architecture/BUILD-NATIF-CLOTURE-BILAN.md` figé en fin de lot, une page maximum, trois premières lignes Avancée/Blocage/Décision, matrice a/b/c CPU/GPU PASS/FAIL/NON EXÉCUTÉ, verdict autorisé, recommandation de fusion et décisions de remplacement/conservation. Le rapport vivant précédent reste intact. La décision de suppression de l'ancien spike n'est jamais déduite d'un verdict ou d'une fusion.

Portabilité à proposer dans le bilan, en cinq lignes : GGML_NATIVE=ON reste mesuré et inchangé ; la qualification présente ne couvre que cette machine ; lot séparé prioritaire pour une base AVX2 ou des variantes choisies au lancement ; définir les processeurs cibles et essais négatifs d'instructions ; ne pas confondre ce futur lot avec la reproductibilité actuelle.
