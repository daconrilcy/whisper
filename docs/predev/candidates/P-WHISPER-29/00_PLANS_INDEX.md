# Plans Whisper — P-WHISPER-29

Statut à rédaction : DRAFT. Owner : AUTH-PLAN-L04. Date : 2026-10-08.

Successeur immuable de P-WHISPER-28. Parent DESIGN exact : D-WHISPER-28, digest `50e2ab1ac3afd17d9e4a1e51932aaa37f812e3162db0207626656108352a6911`.

Le statut courant se lit dans `C:/dev/whisper/docs/predev/state.json`, après contrôle canonique sur une racine documentaire unifiée et vérifiée. L'inscription DRAFT dans ce candidat décrit son état à rédaction. Le manifeste PLANS et la revue indépendante de P29 restent à produire. Aucun CLEAN ou READY PLANS n'est revendiqué ici.

## Entrée opérationnelle

`17_L04_D28_PLAN.md` est la spécification opératoire de L-WHISPER-04. Elle remplace pour L04 les instructions héritées des fichiers 00 à 16 concernant le parent D22, la portée de quinze chemins, la baseline dix présences/cinq absences, l'identité worker assimilée au passage, la cessation systématique de capture sur panne worker, le checkpoint calculé depuis le dernier mot et le lancement fondé sur la seule autorisation ancienne.

Les fichiers hérités sont conservés comme sources historiques. Leurs preuves L00–L03 gardent leur identité, leur candidat exécuté et leurs limites. Leurs campagnes non exécutées restent NOT RUN.

## Périmètre et graphe

L00 → L01 → L02 → L03 → L04 → L05 → L06 → L07. Les dépendances de lancement entre lots sont EXECUTION. Les contrats acceptés sont DOCUMENT ; les API et artefacts fournis sont CODE. Aucune dépendance DOCUMENT ne rend disponible un code absent.

L00–L03 sont historiques/completed selon leurs preuves canoniques. L04 demeure planned jusqu'à sa clôture attribuée. L05–L07 conservent leurs objectifs et couvertures de `01_LOTS.md` et consomment les contrats corrigés D28 lorsque pertinents. Les détails ouverts avant L05 ou L07 restent à leurs échéances. Les chemins partagés sont exécutés séquentiellement.

L04 contient cinq incréments internes séquentiels I04-A à I04-E. Ils ne constituent pas de nouveaux lots completed dans le ledger.

## Autorités et limites

La préparation et la revue des plans sont autorisées. `USER_AUTHORIZATION_L04.md` autorise l'implémentation et le préflight sur quinze chemins historiques. Six chemins supplémentaires du design accepté restent sans autorisation d'édition applicable à cette rédaction. `17_L04_D28_PLAN.md` les liste et impose le contrôle avant code.

Le railguard est déjà actif ; aucune nouvelle activation ou exception n'est proposée. Le préflight vérifie le fichier actif et son attestation d'équivalence.

PRODUCT_VALIDATION, DELIVERY_QUALIFICATION, probes d'environnement, préflight ciblé L04, builds et tests de ce candidat : NOT RUN.

## Sources et reprise

Parent D28 : `59_CHANGE_L04_DECISIONS_D24.md`, `60_CHANGE_L04_DOMAIN_D24.md`, `61_CHANGE_L04_ARCHITECTURE_D24.md`, `62_CHANGE_L04_REQUIREMENTS_D24.md`, `57_L04_CHANGE_BASELINE.json`, `52_L04_SOURCE_BASELINE_D20.md` et snapshots code utiles. Sources historiques : corpus P28, autorisation PLANS, réponses DETAIL-P03/Q-04/Q-06/Q-09, preuves L03 et railguard.

Le coordinateur fait persister le corpus complet et le paquet brut auteur en TRANSPORT, vérifie reçus/hashes, puis conserve le checkpoint de reprise. La revue PLANS doit porter P29 exact, son manifeste et D28 exact. Toute correction d'octets demande un successeur et une revue exacte.
