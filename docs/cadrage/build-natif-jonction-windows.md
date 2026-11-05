Avancée : jonction réelle retirée avec cible préservée ; code et tests relus CLEAN.
Blocage : préservation initiale partielle par Move-Item ; Directory.Move du reliquat ensuite refusé par Windows.
Décision : arrêt sur Access denied, deux parties conservées ; aucune nouvelle compilation ni preuve d'équivalence.

# BUILD-NATIF — complément proposé après clôture

## Cause établie en lecture seule

Le 5 novembre 2026, `fsutil reparsepoint query C:/wb/native/cpu/userprofile/AppData/Local/Microsoft/Windows/INetCache/Content.IE5` réussit : tag **0xa0000003**, jonction vers `C:/wb/native/cpu/userprofile/AppData/Local/Microsoft/Windows/INetCache/IE`. La cible existe comme répertoire ordinaire. Le marqueur du workspace indique cpu7 ; aucune compilation nouvelle n'est lancée dans cette analyse.

Dans `ci/native-workspace.ps1`, `Initialize-NativeWorkspace` refuse tout descendant ReparsePoint avant `Move-Item`. Ce refus de préservation est distinct du collecteur d'événements : ni `Assemble`, ni le classement des événements, ni leurs règles d'arrêt ne sont en cause ici. La jonction est interne aujourd'hui ; cela ne justifie pas d'accepter génériquement les jonctions. Un déplacement brut conserverait en outre une cible absolue vers C:/wb/native/cpu, donc vers le futur workspace après sa recréation.

## Correctif autorisé et exécuté jusqu'au blocage de déplacement

Ajouter une préparation étroite dans `ci/native-workspace.ps1`, appelée par `Initialize-NativeWorkspace` après les contrôles de propriétaire/processus et avant le refus général des ReparsePoint :

1. Inventorier les liens sans les suivre. Valider **l'ensemble** avant toute mutation : le seul cas admis à la préparation est le chemin relatif exact `userprofile/AppData/Local/Microsoft/Windows/INetCache/Content.IE5`, de type Junction/tag 0xa0000003, avec pour seule cible le voisin exact `INetCache/IE` du même workspace CPU ou GPU. Racine, ancêtres et cible doivent être ordinaires, sans lien intermédiaire ; cible existante ; propriétaire inactif et contrôles de concurrence conservés. Tout autre lien, cible externe/UNC, chaîne, métadonnée ambiguë ou changement constaté entraîne un arrêt avant mutation.
2. Écrire avant l'opération, dans la sortie autorisée du garde, un relevé du lien : chemins absolus et relatifs, type/tag, cible, propriétaire du workspace et opération prévue. Conserver ce relevé avec l'espace préservé ; l'alias pourra ainsi être expliqué ou reconstitué vers sa cible préservée, sans viser un futur build.
3. Revalider immédiatement les préconditions puis retirer **uniquement la jonction**, par une opération non récursive dont la sémantique est vérifiée sous le PowerShell/.NET épinglé. Candidat minimal : `[IO.Directory]::Delete($junction, $false)`. Ne jamais lui fournir un chemin non validé ; ne pas utiliser de suppression récursive, ne pas supprimer la cible, ne pas modifier ses attributs ou ACL. Si l'opération échoue, arrêter sans autre mécanisme de suppression.
4. Vérifier la disparition du lien et la conservation de la cible/contenu. Refaire ensuite le contrôle existant exigeant **zéro ReparsePoint**. Préserver l'ancien workspace par le déplacement déjà prévu et recréer le même chemin physique neuf. Si le lien réapparaît ou qu'un autre est présent, arrêter ; aucune boucle de nettoyage.

L'absence du lien est un cas normal : aucune suppression. Cette préparation appartient à l'ancien espace inactif ; elle ne s'exécute jamais comme réparation pendant une compilation. Les racines autorisées, surveillances, délais, règles Temp, refus de renommages ambigus et interruption sur erreur restent identiques. Aucun changement Rust, pin, patch ggml historique ou option ISA.

## Périmètre et vérifications avant reprise

Fichiers autorisés : `ci/native-workspace.ps1`, `ci/test-native-workspace.ps1` ; branchement des tests par le SelfTest existant. Complément documentaire dans ce fichier ; bilan antérieur figé et archive conservés. Revue indépendante, au plus deux cycles pour ce complément ; aucune extension implicite des budgets de construction. L'utilisateur a confirmé explicitement les cinq étapes proposées dans la session, y compris retrait hors dépôt et reprise des deux compilations après tests/revue.

Premier test réel dans une fixture neuve sous `target/native-build/` : **PASS**. Cible contenant un témoin, jonction retirée par `Directory.Delete(link,false)`, SHA du témoin inchangé ; aucune intervention dans le workspace réel. Preuve : `junction-probe-0087019da0f24cec8e0cf0128087de8b/result.json` sous cette racine.

Auto-tests du helper : absence de lien ; jonction attendue et cible non vide conservée ; mauvaise cible même interne ; cible externe ; mauvais emplacement/type ; ancêtre ou cible indirecte ; lien supplémentaire. Les cas négatifs ne doivent provoquer aucune mutation. Vérifier aussi arrêt sur échec de retrait, conservation du relevé si l'étape suivante échoue, et refus si le lien réapparaît. Puis SelfTest NativeBuild et revue, avant toute intervention sur C:/wb/native.

Après autorisation de mise en œuvre, du retrait hors dépôt et de reprise : préserver cpu7 par cette procédure, construire **cpu7c puis cpu8** dans deux espaces neufs au chemin CPU fixe. Les compteurs restent cumulatifs : une construction CPU déjà commencée sur quatre ; le refus cpu7b n'a pas lancé de compilateur. Vérifier commit décodé, SHA 5/5 et surfaces PE ; seulement ensuite runtimes N2 et N1 selon le plan initial. GPU reste limité à deux builds et N2 à dix cas au total. Arrêt au premier échec ; aucun remplacement de DLL installée ni nettoyage des dossiers protégés.

## Fondement technique

- [Microsoft RemoveDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-removedirectoryw), remarques : retirer une jonction ne supprime ni le répertoire cible ni son contenu, même non vide.
- [Microsoft Directory.Delete](https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.delete?view=net-9.0) : surcharge non récursive disponible ; son comportement réel sur la jonction devra être prouvé par la fixture avant emploi.
- [Microsoft Get-ChildItem](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/get-childitem?view=powershell-7) : parcours sans suivi des liens par défaut ; aucun `-FollowSymlink` dans cet inventaire.

## Journal du complément

Le premier SelfTest historique du collecteur échoue avant les nouveaux tests : « Fixture writer exit 1 », stderr non conservé, cause indéterminée. Aucun build ni intervention réelle. L'utilisateur autorise une capture stdout/stderr limitée à `ci/test-native-directory-watch.ps1` et une seule reprise. Cette reprise passe : 1 000 remplacements réels du collecteur ; 41 contrôles flags, 12 workspace, 11 jonction et 8 négatifs historiques. Preuve : `target/native-build/guard/selftest-eaaa66c48f2647da9e710b135272ba20/`, sortie du child SelfTest incluse. La capture ne modifie aucun classement d'événement ni délai du collecteur.

Revue indépendante : BN-J01 IMPORTANT, la capture des journaux pouvait court-circuiter le nettoyage. Correction par finally indépendants, fermeture au deuxième passage : **CLEAN**. Test ciblé du bloc finally réel extrait de son AST : erreur de capture simulée conservée, deux writers libérés et observateur fermé, PASS. Le SelfTest général PASS précède uniquement cette correction de nettoyage ; aucun changement au retrait de jonction depuis ce SelfTest.

cpu7c : retrait réel réussi, reçu `junction-preparation.json` = `REMOVED_TARGET_PRESERVED`. `Move-Item` échoue ensuite sur un élément masqué/système/lecture seule et déplace partiellement l'ancien espace vers `target/native-build/guard/NativeDllCpu-cpu7c/previous-workspace`. Le marqueur cpu7, src, build et d'autres dossiers s'y trouvent ; le reliquat C:/wb/native/cpu contient userprofile et xdg. Aucun compilateur démarré. Cette opération n'est pas présentée comme atomique.

L'utilisateur autorise un correctif de déplacement : `Directory.Move` du dossier entier sur le même volume, test des attributs, puis préservation séparée du reliquat vers `previous-workspace-residual`, sans fusion ni écrasement, enfin cpu7c2/cpu8 après revue. Trois tests réels passent (octets et attributs Hidden/System/ReadOnly conservés ; refus de destination existante ; refus de volume différent). SelfTest complet PASS, preuve `selftest-9f8c402e7bf14e4abbbb5452a0c10c10` ; revue **CLEAN**.

Récupération réelle : propriétaire établi depuis marqueur conservé et reçu, absence de processus concernés/liens, inventaire du reliquat enregistré avant action. `Directory.Move` est ensuite refusé : **Access to the path C:/wb/native/cpu is denied**. Arrêt sans autre mécanisme, modification d'attributs/ACL ou élévation. La destination résiduelle n'existe pas ; comparaison après refus = contenu inchangé, trois fichiers et 27 dossiers, zéro ReparsePoint. Les trois fichiers restants sont container.dat (0 octet), StartupProfileData-NonInteractive et telemetry.uuid. Les ACL des racines sont relevées sans modification ; aucune cause précise ni aucun processus détenteur n'est démontré. La présence de dllhost.exe ne suffit pas à attribuer un verrou.

État conservé : première partie dans `previous-workspace`, reliquat à C:/wb/native/cpu, reçus de retrait/récupération et script de récupération sous le lot. Seuls les liens des fixtures et l'alias réel approuvé ont été supprimés ; aucun contenu cible ni intermédiaire nettoyé. Le compteur reste une construction CPU ancienne commencée (cpu7), aucune nouvelle compilation CPU/GPU, aucun N2 ni N1. cpu7c2 et cpu8 ne sont pas lancés. Bilan antérieur et archives existantes intacts ; voir le nouveau [bilan du complément](../architecture/BUILD-NATIF-JONCTION-BILAN.md).
