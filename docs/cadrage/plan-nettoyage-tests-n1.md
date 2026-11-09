# Mini-lot — nettoyage des racines temporaires N1

Statut : proposition à relire et à accepter ; aucune implémentation autorisée par ce document. Base : `main` 2f344624. Responsable du plan : session principale. Objectif : éviter l'accumulation des copies de modèle produites par les tests, sans changer le produit ni les assertions.

## Périmètre

Le lot cible les sept scénarios Windows `unified_poll` qui appellent `prepare_fake_personal_worker` : le scénario de repli Auto, les trois scénarios preview/final et les trois scénarios EOF/EOF avec pair/candidat natif. La fonction dans `crates/whisper-adapters/src/worker_ipc/tests/unified_poll/auto_fallback.rs` copie 713 975 456 octets de modèle par scénario et vérifie les empreintes de la source et de la copie. Les tests parents créent les racines et vérifient des traces après le retour de leurs processus enfants ; ils restent propriétaires des racines jusqu'à la fin des assertions.

Chemins d'implémentation proposés : `crates/whisper-adapters/src/worker_ipc/tests/unified_poll/auto_fallback.rs`, `preview_fallback.rs`, `equivalence.rs` et un petit support de test avec ses tests sous `crates/whisper-adapters/src/worker_ipc/tests/unified_poll/`, déclaré dans `crates/whisper-adapters/src/worker_ipc/tests.rs` si nécessaire. Vérifier les sept parcours lors de l'implémentation. Les autres racines N1, les racines de campagne et les racines historiques sont hors périmètre ; leur volume résiduel sera indiqué. Pas de modification produit, dépendance, lien physique, partage de modèle, ACL, ETW ni nouvelle surveillance.

## Contrat proposé

1. Chaque test parent concerné crée exclusivement sa racine unique et en conserve une garde `Drop` jusqu'après la dernière assertion et la fin de ses processus enfants. L'enfant ne possède pas cette garde ; les traces restent lisibles par le parent. La création doit échouer si la racine existe déjà : ne jamais adopter un dossier préexistant. Ne pas modifier le contrat commun d'allocation des chemins, également utilisé ailleurs.
2. Au retour normal ou lors d'un déroulement de panic, `Drop` tente `std::fs::remove_dir_all` sur la racine créée par ce test, sans suivre les liens. Une jonction ou un lien contenu dans la racine ne doit pas entraîner la suppression de sa cible. Aucun chemin fourni par l'environnement n'est directement adopté comme racine à effacer ; aucun nettoyage par motif, parent commun ou autre scénario.
3. Une erreur de suppression est signalée avec le chemin et conservée ; `Drop` ne provoque pas de second panic. La suppression n'est pas garantie si le processus est arrêté brutalement, si l'exécution utilise `panic=abort` ou si Windows refuse l'accès. Le test ne prétend pas que tous les descendants sont terminés sans preuve ; si un fichier est verrouillé, la racine peut rester.
4. Les copies indépendantes et leurs vérifications SHA-256 restent inchangées ; aucune assertion n'est affaiblie et aucun test n'est ignoré. Les originaux et les cibles des liens ne sont jamais modifiés par le nettoyage.

## Tâches et acceptation

| Tâche | Résultat vérifiable |
|---|---|
| 1. Préflight | Worktree dédié depuis `main`, état Git et espace libre ; garde N1 existante et ses auto-tests ; données de test sélectionnées et vérifiées par la garde. Pas de Cargo direct. |
| 2. Garde de test | Racine propre créée exclusivement, garde conservée par le parent des sept scénarios, suppression après assertions et tentative pendant un panic. L'échec de création ne donne aucun droit d'effacement. |
| 3. Essais ciblés | Vérifier succès, déroulement de panic, refus d'adopter une racine préexistante, deux racines indépendantes, et lien interne vers un répertoire témoin externe dont les fichiers et empreintes restent intacts après suppression. Vérifier aussi que l'échec de suppression est signalé sans double panic ; ne pas prétendre simuler un arrêt brutal. |
| 4. Vérification N1 | Sous la garde et sans `CARGO_TARGET_DIR`, `cargo test --no-fail-fast` complet ; comparer à 865/0/129, expliquer tout écart. Vérifier la disparition des sept racines lorsque Windows accepte leur suppression, et mesurer le volume résiduel. Relever nombre, taille et SHA-256 des originaux avant/après. |
| 5. Revue | Revue indépendante du diff et des résultats, sécurité des originaux en priorité ; deux cycles maximum selon `AGENTS.md`. Rapport : résultats, racines restantes, espace libre initial/final et limites. |

Avant exécution, préciser avec la garde existante le chemin du worktree, la cible Cargo autorisée, la racine de campagne et le modèle personnel ; la garde effectue ses vérifications d'isolation et son contrôle d'espace. Le nettoyage des racines de campagne reste réservé à l'humain. Aucun historique n'est supprimé par ce lot.

## Risques et décision

Le gain visé est la disparition des sept copies après chaque exécution réussie, soit environ 4,65 Gio de modèle ; ce n'est pas une borne globale du N1. Le comportement de `remove_dir_all` et le test de jonction doivent être vérifiés sur Windows dans le contexte réel ; aucune garantie contre une substitution malveillante concurrente n'est revendiquée. Si cette limite ou un verrou Windows exige une nouvelle primitive système, arrêter et demander une décision plutôt qu'élargir le lot.

À décider : accepter ce périmètre limité et autoriser explicitement l'implémentation et les validations sous garde après revue du plan. Les tests et la revue d'implémentation sont à ce stade **non exécutés**.
