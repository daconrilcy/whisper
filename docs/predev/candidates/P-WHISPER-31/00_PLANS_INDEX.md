# Plans Whisper — P-WHISPER-31

Statut à rédaction : DRAFT. Auteur : /root/plan_p30. Owner : AUTH-PLAN-P31. Date : 2026-10-08.

Successeur immuable de P-WHISPER-30, digest `6befdd092dcb8e137584557e2f01514e7079f89849b43a52d4bd465c23863ae5`, resté DRAFT après revue avec findings. Parent DESIGN exact : D-WHISPER-30, digest `514d8fcf3e3ffdbb06852a81ffca8cc114bd6a0450ef83550443a995f29a1a2b`.

Le statut courant se lit dans `C:/dev/whisper/docs/predev/state.json`, après contrôle canonique sur une racine documentaire unifiée et vérifiée. L'inscription DRAFT dans ce candidat décrit son état à rédaction. Le manifeste PLANS et la revue indépendante de P31 restent à produire. Aucun CLEAN ou READY PLANS n'est revendiqué ici.

## Entrée opérationnelle

`17_L04_D28_PLAN.md` est la spécification opératoire D30/P31 de L-WHISPER-04 ; son nom est conservé pour la continuité des références. Elle reprend les contrats et les 21 chemins du plan P29, adopte CHANGE-CUDA129-01 et DETAIL-CUDA129-ENV-01 de D30/63 et l'extension utilisateur des six chemins. Pour L04, elle a priorité sur les instructions historiques héritées incompatibles des fichiers 01 à 15 ; `16_L04_VERIFICATION_MATRIX.md` est actualisé et renvoie à sa matrice V01–V15.

Les fichiers hérités sont conservés comme sources historiques. Leurs preuves L00–L03 gardent leur identité, leur candidat exécuté et leurs limites. Leurs campagnes non exécutées restent NOT RUN.

## Périmètre et graphe

L00 → L01 → L02 → L03 → L04 → L05 → L06 → L07. Les dépendances de lancement entre lots sont EXECUTION. Les contrats acceptés sont DOCUMENT ; les API et artefacts fournis sont CODE. Aucune dépendance DOCUMENT ne rend disponible un code absent.

L00–L03 sont historiques/completed selon leurs preuves canoniques. L04 demeure planned jusqu'à sa clôture attribuée. L05–L07 conservent leurs objectifs et couvertures de `01_LOTS.md` et consomment les contrats corrigés repris par D30 lorsque pertinents. Les détails ouverts avant L05 ou L07 restent à leurs échéances. Les chemins partagés sont exécutés séquentiellement.

L04 contient cinq incréments internes séquentiels I04-A à I04-E. Ils ne constituent pas de nouveaux lots completed dans le ledger.

## Autorités et limites

La préparation et la revue des plans sont autorisées. `USER_AUTHORIZATION_L04.md` et `USER_AUTHORIZATION_L04_EXTENSION_CUDA129.md` autorisent ensemble les 21 chemins exacts de L04 et son préflight. L'extension accepte CUDA 12.9 ; D30/63 précise CUDA 12.9 Update 1, NVCC 12.9.86 et la voie Visual Studio 2022 x64. Cette autorisation ne satisfait pas les conditions techniques de lancement : baseline partielle réconciliée, environnement vérifié, checkpoint IMPLEMENTATION et contrôle ciblé réussi restent requis.

Le railguard est déjà actif ; aucune nouvelle activation ou exception n'est proposée. Le préflight vérifie le fichier actif et son attestation d'équivalence.

PRODUCT_VALIDATION, DELIVERY_QUALIFICATION, probes d'environnement, préflight ciblé L04, builds et tests de ce candidat : NOT RUN.

## Sources et reprise

Parent D30 : contrats 59–62, `57_L04_CHANGE_BASELINE.json`, `52_L04_SOURCE_BASELINE_D20.md`, snapshots code utiles et `63_CHANGE_CUDA129.md`. Sources supplémentaires : autorisation historique L04, extension des six chemins/CUDA12.9, source technique CUDA et preuve historique L03 référencées par D30/63. P29 conserve ses preuves, son manifeste et sa revue historiques.

P31 est rédigé par `/root/plan_p30` sous l’autorité distincte `AUTH-PLAN-P31`. `AUTH-PLAN-L04` demeure l’autorité historique attribuée à `/root/plans_writer_l04` ; aucune adoption P31 par cet acteur n’est présumée. Le transport auteur P31 conserve la provenance de cette correction. Les sources brutes P30 restent historiques et inchangées.

Le coordinateur fait persister le corpus complet et le paquet brut auteur en TRANSPORT, vérifie reçus/hashes, puis conserve le checkpoint de reprise. La revue PLANS doit porter P31 exact, son manifeste et D30 exact. Toute correction d'octets demande un successeur et une revue exacte. Toutes campagnes P31 non exécutées restent NOT RUN.
