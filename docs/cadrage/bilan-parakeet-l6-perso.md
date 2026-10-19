Avancée : L6-perso CPU+GPU est implémenté, revu CLEAN, installé localement et ses imports EN5 installés passent.
Blocage : le rollback entre deux générations distinctes et la recette visuelle Windows restent NOT RUN.
Décision : livrer avec ces réserves explicites, sans annoncer la restauration inter-génération comme qualifiée.

# Bilan d'exécution — L6-perso

Date : 19/10/2026. Base Git : `b1bd1914cf51f9b7b9cb72b4547138a5021a256d`, branche `evo-startup-perf`. Autorisation utilisateur : implémentation du [plan v4](plan-parakeet-l6-perso.md) dans cette session. Revue Rust indépendante : **CLEAN**, R1–R5 fermés après deux cycles de correction. Le code et le script ont été revus avant `-Publish` ; aucun push n'a été effectué. La [fiche](fiche-parakeet-l6-perso.md) consigne les SHA exacts et les preuves de recette.

| Carte | Résultat | Statut |
| --- | --- | --- |
| P1 — profil et isolation | `personal-install` et `parakeet-personal` absentes par défaut ; racines Test/Perso séparées ; refus des chemins et clés divergents. | PASS |
| P2 — identités CPU/GPU | Deux builds release séparés et figés ; modèle et 14 DLL vérifiés ; deux clés Perso distinctes inscrites après les recettes. | PASS |
| P3 — réglages | Copie initiale et `.bak` identiques, réglages historiques inchangés ; simulation de mise à jour après préférence modifiée passe. | PASS |
| P4 — déploiement | StageOnly, publication réelle, backup initial et backup complet de mise à jour passent ; 13/13 injections de panne et rollback réel C→C passent. Restauration B→C non exécutée. | **PARTIAL** |
| P5 — release | Desktop release sans injection, deux workers Parakeet release figés, deux workers Turbo copiés ; 24/24 fichiers du staging avant publication. | PASS sur cet hôte |
| P6 — qualification | A4 FR87/FR27/EN15, EN5, EN30, R8 5/5 par backend, A2 GPU, trois sessions EN5 GPU ≤15 s, smoke CPU/GPU publié TXT/SRT, refus >30 min et routes manuel/Auto passent. La fenêtre Windows reste à l'utilisateur. | PASS automatisé ; UI NOT RUN |
| P7 — tests sans campagne | Workspace sans variables et avec variables : 52 cibles, zéro FAIL, garde code 0. Sans campagne : 34 ignorés et 39 lignes `NOT RUN` explicites. | PASS |
| P8 — clôture | Revue CLEAN et présente fiche/bilan/recette ; espace libre final 62,30 Gio. Rollback inter-génération et UI restent réservés. | **LIVRÉ AVEC RÉSERVES** |

## Publication, sauvegardes et état final

`candidate-20261019-b` a été publié sous `%LOCALAPPDATA%\Whisper`, puis EN5 a été importé par les deux backends installés. Le smoke CPU écrit un TXT de 4 981 octets et un SRT de 8 240 octets ; le smoke CUDA0 écrit un TXT de 4 988 octets et un SRT de 8 247 octets sous `target/parakeet-tests/l6-perso/outputs/final-smoke/`. Leurs processus lisent worker, modèle et DLL de la racine personnelle, sans dépendance au chemin du dépôt pour Parakeet. Les durées **80,97 s CPU et 44,38 s CUDA0** sont celles du harness complet, pas des temps d'inférence ni du parcours produit release isolé ; les trois temps GPU `StartImport`→TXT/SRT sont dans la fiche.

Une deuxième publication a créé un backup complet de la génération B (`before-candidate-20261019-c`, 26/26 fichiers vérifiés). Deux tentatives de rollback vers B ont été arrêtées par la garde sur des événements du dossier `backups` avant mutation de l'installation active. La garde privée a ensuite été calibrée avec 25/25 auto-tests, mais le dernier essai a restauré une sauvegarde de la **même** génération C : installateur et garde code 0, snapshot final 23/23 fichiers et 27/27 SHA protégés inchangés. Cela prouve un retour arrière transactionnel sur génération complète ; cela **ne prouve pas** la restauration B→C. Un dossier `.backup-…pending` incomplet reste isolé dans `backups/` et n'a pas été supprimé. L'installation active finale est complète ; desktop fermé, 62,30 Gio libres.

## Gates, revue et périmètre

La garde d'exécution privée est `target/parakeet-tests/l6-perso/guard-campaign.ps1`. Les deux commandes de workspace `cargo test --offline --locked --workspace --no-fail-fast`, avec puis sans variables de campagne, ont code 0 et aucune déviation attribuée ; sorties `logs/Workspace.stdout.log` et `logs/WorkspaceNoCampaign.stdout.log`. Le dernier `InstallerSelfTest` a 13/13 PASS. Les 39 lignes `NOT RUN` concernent des tests à variables absentes, qui peuvent apparaître `ok` dans libtest après retour anticipé ; aucun n'est compté comme preuve de recette. Les essais A4/R8 ont été exécutés séparément avec leurs variables et leur copie figée.

Le reviewer indépendant a d'abord relevé cinq constats : reprise pending Perso dans le build normal, cohérence desktop staging/publié, sûreté du backup/rollback, règle `.bak` après évolution des préférences et confinement des logs GPU. Tous ont été fermés par des tests ou une inspection ciblée après deux corrections ; verdict de code **CLEAN**. La publication et les tests installés sont des preuves postérieures à cette revue, distinctes du verdict de code. Aucun troisième cycle de correction de code n'a été effectué.

L'inventaire `dumpbin /dependents` des cinq EXE et 14 DLL installés révèle que les deux workers Turbo utilisent le CUDA Toolkit présent sur cet ordinateur pour `cublas64_12.dll` et `cudart64_12.dll`. L'installation est autonome vis-à-vis du **dépôt**, mais pas démontrée portable vers un PC dépourvu de ce toolkit ; cette portabilité est hors du périmètre personnel décidé. Le driver NVIDIA et le runtime VC viennent du système. Le micro reste Turbo ; Parakeet live, VAD adaptative, B8, acquisition réseau, installateur distribuable et autres langues restent hors périmètre.

## Suites limitées

1. L'utilisateur exécute la [recette dans la fenêtre Windows](recette-parakeet-l6-perso.md), notamment un fichier français réel, EN5, les modes CPU/GPU/Auto et le refus >30 min. Cette validation n'est pas attribuée aux tests automatisés.
2. Si une preuve complète du retour arrière entre générations est nécessaire, préparer une campagne dédiée avec backup distinct et garde prévalidée ; ne pas déduire son PASS du rollback C→C. Le dossier pending incomplet est conservé jusqu'à décision de nettoyage explicite.
