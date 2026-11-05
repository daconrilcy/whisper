Avancée : mécanisme Windows reproduit dans une fixture ; fenêtre Explorateur et deux handles identifiés dans le reliquat.
Blocage : libération et contre-vérification du dossier réel non exécutées ; couverture de la sonde partielle.
Décision : libérer normalement la fenêtre concernée puis contrôler les handles avant tout nouveau déplacement.

# BUILD-NATIF — diagnostic Windows du 5 novembre 2026

## Résultat et portée

Un candidat concret au blocage actuel est identifié : **explorer.exe, PID 16536**, maintient deux handles (`0x40cdc`, `0x40e60`) sur `C:/wb/native/cpu/userprofile/AppData/Local/Microsoft`. Une interrogation COM en lecture seule trouve une fenêtre de l'Explorateur exactement à cet emplacement, HWND 91885904. Il ne s'agit donc pas d'une attribution par simple présence du processus. Le handle supplémentaire de la sonde sur la racine appartient à son propre processus et constitue son témoin positif, pas un verrou tiers.

Le mécanisme causal a ensuite été reproduit sur cette machine dans une fixture neuve : un handle de sous-dossier, avec les mêmes droits d'accès observés côté Explorer et un partage READ/WRITE/DELETE, suffit à faire échouer le déplacement du parent avec **Win32 5, HRESULT 0x80070005**. Après fermeture de ce seul handle de test, le même déplacement réussit ; SHA du témoin inchangé. C'est cohérent avec les contraintes Windows documentées sur les descendants ouverts, même quand l'ouverture du droit DELETE réussit.

**La levée du blocage sur le reliquat réel et la responsabilité exclusive dans l'échec historique de 09:42 Paris restent à contre-vérifier** : aucun handle Explorer n'a été fermé et aucun déplacement du reliquat n'a été retenté. La présence des handles et de la fenêtre est établie ; le mécanisme correspondant est démontré dans la fixture. Ne pas présenter ce test comme une reprise réelle déjà réussie, ni modifier les ACL ou utiliser une élévation.

## Vérifications réellement exécutées

| Contrôle | Observation |
| --- | --- |
| Attributs et ACL du reliquat et des racines | Racine ordinaire ; pas de ReadOnly sur la racine ni règle de refus explicite relevée ; ne vaut pas calcul complet des droits effectifs |
| Ouvertures Windows des droits nécessaires, sans opération de déplacement | **32/32 réussies**, droit DELETE sur les 31 objets source et FILE_ADD_SUBDIRECTORY sur le parent destination ; OPEN_EXISTING, partage lecture/écriture/suppression, aucun DeleteOnClose |
| Journaux Defender 1123/1124 après 09:30 Paris | Aucun événement correspondant ; ne prouve pas l'absence de tout filtre de sécurité |
| Sonde handles autorisée explicitement, sans élévation/pilote/service | 44 490 handles fichiers ; 28 296 examinés ; 16 194 non résolus/inaccessibles ; témoin positif trouvé ; deux handles tiers correspondent à l'Explorateur |
| Fenêtres Explorer, COM Shell.Windows | Une fenêtre correspondante, même chemin que les handles |
| Test causal autorisé, dossier factice uniquement | Handle enfant ouvert : refus 5/0x80070005 ; handle fermé : déplacement réussi ; SHA témoin identique |
| Intégrité du reliquat après diagnostic | Contenu identique au reçu avant refus ; aucun déplacement ou changement d'ACL/attributs |

La première invocation de la sonde d'accès a échoué au marshaling PowerShell avant l'appel Win32 ; les paramètres ont été typés explicitement puis les 32 ouvertures ont réussi. Le premier relevé de handles n'était pas exploitable : tous les pointeurs Object étaient masqués à zéro et leur regroupement était incorrect. La version corrigée distingue PID/handle lorsque ce pointeur est nul, garde un handle connu de la sonde comme contrôle positif et indique les processus non inspectables. Le premier résultat est conservé sous le nom `result-initial-unvalidated.json`, sans verdict négatif utilisable.

Les déclarations CreateFileW déjà versionnées ont été réutilisées pour la sonde d'accès. Le complément C# temporaire de recherche des détenteurs a été autorisé précisément par l'utilisateur. Il duplique seulement des handles dans son propre processus, puis ferme ses propres duplications ; aucune fermeture distante, aucun watcher, aucun processus tiers arrêté. Le code et les résultats sont conservés dans l'archive de diagnostic ; ils ne constituent pas un nouveau contrôle permanent dépendant de target/.

## Suite recommandée, non exécutée

Fermer la fenêtre Explorateur située dans le dossier Microsoft ci-dessus, ou la faire naviguer hors de **C:/wb/native/cpu**. Ne pas tuer explorer.exe ni fermer ses handles de force. Relever ensuite l'absence des handles tiers ; si elle n'est pas établie, ne pas déplacer. Un déplacement contrôlé ne pourra être une contre-épreuve qu'après l'autorisation applicable à la reprise et la vérification des autres prérequis. Ne pas relancer directement une compilation sur l'espace partiellement préservé.

Les deux parties de l'ancien espace, les bilans figés et les archives antérieures restent conservés. Aucun essai N1/N2 ni compilation native lancé. Le test causal supplémentaire a été autorisé explicitement après la sonde en lecture seule et ne déplace qu'un dossier factice neuf sous target/native-build/. Les limites de couverture de la sonde empêchent de garantir l'absence d'autres détenteurs.

Sources Microsoft : [droits et contraintes de renommage](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information), [accès DELETE et partage des handles](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilea).

Preuves : [archive vérifiée](../architecture/raw/build-natif-refus-windows/diagnostic.zip), **16 fichiers + manifeste**, 20 144 octets ; SHA-256 `bc4de469b4593fa88f07d9dd8ef80ad16b52d5ed3ba44a199651a654f1b1392a`. ZIP rouvert, noms uniques, nombres, tailles et empreintes de chaque entrée contrôlés. Revue indépendante : BN-W01 corrigé, conclusions CLEAN ; mécanisme reproduit distinct de la levée réelle non exécutée. Aucun nettoyage ni modification des archives précédentes.
