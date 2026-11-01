Avancée : lot R8-FIX autorisé dans la session du 1er novembre 2026 ; prérequis GARDE-N1-CATEGORIES terminé sur 2dc28e80.
Blocage : aucun au lancement ; tout échec de campagne impose un arrêt et l'arbitrage prévu.
Décision : compilation séparée et contrôle PE, sans code Rust produit, manifeste, installation ni push.

# Plan R8-FIX

Autorité : demande utilisateur « Lot R8-FIX : worker CPU sans CUDA, compilation séparée et contrôle PE ». Branche `fix/r8-worker-cpu-sans-cuda`, worktree dédié depuis `garde/n1-categories` (2dc28e80). Diagnostic consultable dans le worktree `../diag-r8-cuda-cpu/docs/architecture/R8-DIAGNOSTIC.md` et ses annexes `raw/r8/lecture.json` et `pe-binaries.json` ; leur absence sur la branche de base n'autorise pas de fusion de code.

Objectif produit : rendre le worker CPU livré indépendant des DLL CUDA au chargement et protéger la livraison contre une régression.

## Sept tâches

1. Relever Git, espace libre et références R8 ; préparer les contrôles préalables de la garde N1 existante. Conserver tous les fichiers préexistants.
2. Créer `ci/check-worker-imports.ps1` : lecture des imports PE directs et différés, refus des DLL CUDA pour CPU/Parakeet/desktop et exigence cudart + nvcuda pour GPU ; chemin, DLL, verdict et SHA-256. Auto-tests synthétiques (dont différés et non-PE) et refus du CPU release réel au SHA du diagnostic.
3. Créer `ci/build-workers.ps1` : une invocation Cargo release offline/locked par worker, une cible neuve distincte par configuration, via `ci/guard-level1.ps1`, contrôle PE obligatoire des sorties. Variantes Parakeet déjà prévues seulement ; aucune dépendance nouvelle. Échec fermé avant publication.
4. Aligner les sources workers de `scripts/install-personal.ps1` sur ces sorties sans changer les destinations ni la liste copiée. Remplacer les instructions de compilation commune dans les plans/procédures actifs autorisés par un renvoi au script. Mettre à jour la section N1 d'`AGENTS.md` et la garde N1 (jamais la garde historique) pour les commandes du lot, dont listing des tests.
5. Via la garde : deux graphes de features, CPU release A, CPU test no-run B en cible neuve, GPU release et variantes Parakeet prévues ; contrôle PE des binaires/harness. Si A/B échouent, mesure D avec CUDA_PATH/GGML_* retirés puis arrêt pour arbitrage. Aucun worker ni modèle ni test natif exécuté.
6. Lister les tests des commandes N1 demandées et du GPU ; comparer les identités aux 834 de la base. Exécuter les suites N1 en environnement nettoyé après qualification PE. Complément explicitement accepté par l'utilisateur dans cette session : ajouter `cargo test -p whisper-worker-common` (et son listing), après qualification PE, car ses tests sont exclus des membres par défaut. Le GPU reste N2 et seule sa liste est demandée. Aucun manifeste modifié.
7. Revue indépendante et au plus deux cycles de correction ; session principale seule auteure du bilan `docs/architecture/R8-FIX.md` (deux pages maximum), avec résultats, coûts et limites. Journaux complets sous `docs/architecture/raw/r8-fix/` ; pas de campagne documentaire supplémentaire. Commits locaux, aucun push.

## Périmètre et preuves

Écritures autorisées : nouveaux scripts sous `ci/`, garde N1 `ci/guard-level1.ps1`, livraison `scripts/install-personal.ps1`, plans de livraison identifiés par lecture.json, section niveaux N1 d'AGENTS.md, documents d'architecture et présent plan. Les contributions documentaires spécialisées sont retournées à la session principale, qui les écrit et les commite. Les sorties Cargo/cache/temp restent sous les racines contrôlées de la garde ; la règle de cible neuve par worker autorisée par R8-FIX prévaut ici sur la cible partagée par défaut. Espace initial : 164,92 Gio environ (177082441728 octets).

Interdits : Rust de production, manifestes, Cargo.lock, .cargo/, models/, garde historique, dépendances nouvelles, réseau produit, installation/copie vers installation, worker/modèle/tests natifs, nettoyage des cibles existantes. Ne pas exécuter l'installeur. Arrêt sur échec de commande ou violation de garde ; ne pas relancer une campagne en échec sans arbitrage.

Acceptation : CPU release et harness sans import CUDA ; GPU release importe cudart et nvcuda ; témoin réel rejeté ; imports différés couverts ; commandes de livraison séparées ; couverture des 834 identités ; trois suites N1 sans échec ; revue CLEAN ou réserves explicites. Coûts mesurés en durée et espace. La réinstallation et les observations Windows appartiennent au lot suivant.
