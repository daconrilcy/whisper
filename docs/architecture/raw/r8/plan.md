# R8-DIAG — plan d’investigation, sans correction

Autorité : demande R8-DIAG de la présente session. Base Git 939fbbbe2016b453660cd0b834dc9f368995944e (lot tests autonomes livré, source CI récente), branche diag/r8-cuda-cpu ; worktree C:/dev/whisper/target/agent-worktrees/diag-r8-cuda-cpu. Les modèles et audio non suivis du checkout principal sont préservés. Espace relevé : 171,87 Gio. Début : 1er novembre 2026, 13 h 17 min 59 s Europe/Paris.

## Mandat délégué

1. Lecture des manifestes/configurations/build scripts verrouillés et procédures de livraison : établir la chaîne causale H1–H5 et citer fichier/ligne/version. Inspecter seulement les variables de build pertinentes ; ne pas exposer de secrets ni lire les credentials Cargo. Aucun Cargo, rustc, CMake, worker ou test exécuté à cette étape.
2. Compatibilité de la garde N1 : lecture uniquement. Les cibles neuves/distinctes A/B/D sont expressément demandées par ce lot et priment la règle générale de cible partagée ; aucune dérogation de garde ou création de mécanisme équivalent n’est déduite. Si les commandes et racines ne sont pas admises par la garde inchangée, suspendre les étapes Cargo et décrire l’arbitrage concret.
3. Lorsque la garde autorise réellement le protocole : quatre cargo tree offline/locked séparés, puis A build CPU debug et B test CPU no-run avec leurs targets neufs ; D seulement si H3 plausible. Aucun binaire ni test lancé. Arrêt sur échec. Pas de release workspace. Racine de compilation proposée : target/r8-diag sous ce worktree ; targets target-A, target-B, target-D, CARGO_HOME/TEMP/TMP isolés sous cette même racine.
4. Inspection statique des anciens artefacts C et des workers CPU livrés/installés les plus récents : chemins, taille, SHA-256, date, imports ordinaires/différés, provenance établie ou inconnue. Recherche bornée au dépôt/target et à l’installation Whisper connue ; aucun scan disque global, aucune exécution. Terminer H1–H5, impact et au plus quatre options sans en choisir une.

Auteur des documents et annexes : session principale exclusivement. L’investigateur rend sa contribution brute en lecture seule. Le rapport R8-DIAGNOSTIC.md reste limité à deux pages ; plan, sorties complètes et historique vont sous raw/r8. Le reviewer indépendant relit les fichiers Git et les preuves, sans modification ni exécution ; deux cycles revue/correction maximum.

## Blocage d’exécution déjà identifié en lecture

ci/guard-level1.ps1 du candidat fixe CARGO_TARGET_DIR à C:/dev/whisper/target/parakeet-tests/cargo-shared, les sorties à target/parakeet-tests/tests-autonomes et les argv à SelfTest/NoRun workspace/listes/tests/Clippy/Fmt/Structure. Il ne propose ni cargo tree ni cargo build CPU ni cible paramétrable. L’ancienne garde à surveillance n’est pas une voie N1. Aucune modification de garde n’est autorisée par R8-DIAG. Ne pas lancer ces gardes pour provoquer un rejet ou écrire dans leurs anciennes racines.

Conséquence : poursuivre les lectures et inspections statiques autorisées ; les quatre résolutions Cargo et A/B/D restent NON EXÉCUTÉES tant qu’une voie de garde conforme n’est pas établie ou autorisée. Les imports historiques ne remplacent pas la preuve froide A/B ; les conclusions doivent distinguer cause dans les sources, mesure PE et causalité contrôlée non mesurée.

## Livrables et acceptation

- R8-DIAGNOSTIC.md : trois lignes statut, verdict en trois phrases au plus, H1–H5 avec preuves, matrice A/B/C/D/livrés, impact, quatre options maximum et non-vérifié.
- raw/r8 : plan présent, données de lecture/PE et preuves brutes utiles ; aucune modification de source/manifeste/lock/config/script/garde/modèle.
- Chaque statut CONFIRMÉE/ÉCARTÉE/NON DÉTERMINÉE a une preuve attribuée ; un statut réservé faute de Cargo demeure explicite. Les informations sur un binaire installé ne prouvent pas à elles seules sa commande de compilation.
- Revue indépendante CLEAN ou réserves ; Git local uniquement, aucun push.
