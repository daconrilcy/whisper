Avancée : dix essais Auto alternés PASS ; relevés préalables conservés.
Blocage : médiane candidat +12,06 %, au-delà du seuil 10 % ; modes conformes.
Décision : arrêt sans diagnostic/correction ni fusion ; bilan figé le 4 novembre 2026, aucun push.

# POLL-INTEGRATION — seconde reprise, 4 novembre 2026

Candidat figé 28b96d9a ; référence 00c2d87e portant production 6316a54b et
banc 1be46656, profils identiques. Classification hors poll déjà CLEAN,
aucune nouvelle modification source. Durées : corps Rust hors compilation.
Ordre B1/C1/B2/C2/B3/C3/B4/C4/B5/C5 ; modes rapide <30 s, lent ≥30 s.
Critères : médiane candidat ≤ médiane base ×1,10 et pas de mode lent du
candidat absent de la base. Aucun critère de fréquence supplémentaire.

Avant chaque exécution, état précédent consigné hors campagne active :
processus whisper-* avec informations accessibles, fichiers de verrou,
réservations et pending sous le run précédent. Une présence de fichier ne
prouve pas un verrou détenu ; aucun verrou acquis pour observer. Worker
conservé en mémoire non observable de l'extérieur après sortie du harnais,
distinct des processus workers encore vivants. Ressources non suivies préservées.

| Révision | 1 | 2 | 3 | 4 | 5 | Médiane | Modes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Base | 25,12 R | 25,29 R | 40,55 L | 25,23 R | 40,58 L | 25,29 s | 3 R / 2 L |
| Candidat | 25,08 R | 25,19 R | 28,34 R | 39,72 L | 40,74 L | 28,34 s | 3 R / 2 L |

Temps en secondes ; R<30, L≥30. Plafond 25,29×1,10=27,819 s ; médiane
candidat 28,34 s, +12,0601 % : **NON CONFORME**. Aucun mode lent propre au
candidat : critère modes conforme, insuffisant pour lever l'autre échec.
Reprise interrompue, direct Turbo CPU, imports FR/EN, fetch, tag
avant-fusion-poll, fusion et N1 main : **NON EXÉCUTÉS**. Aucun push.

Les dix relevés logs/poll-integration-reprise2/*-before.json (sous
target/n2-native/) observent chacun zéro processus whisper* et neuf artefacts
du run immédiatement précédent : locks, répertoire reservations et pending.
Première observation sur C3 de la reprise précédente ; lectures hors garde
active, sans modifier les artefacts ni prétendre qu'un verrou est détenu.

Bimodalité présente dans base et candidat, environ 15 s entre les groupes,
avec un essai candidat rapide à 28,34 s. Constat produit hors périmètre pour
un lot de diagnostic distinct ; aucune cause établie ni correction engagée.
Archive target/n2-native/archive/POLL-INTEGRATION-SECONDE-REPRISE-20261104.zip :
211 entrées, 158 691 octets source / 119 452 archivés, nombre/tailles/SHA-256
de chaque entrée vérifiés. SHA-256 archive :
57EF4951EB2920B85B5A9D187F89191AD45584D54EFC915276EBC0C9C28B5B83.
Cache target/n2-native/cargo-home : 651 827 048 octets ; aucune suppression
tentée. Commande utilisateur après vérification du chemin et absence de campagne :
`Remove-Item -LiteralPath 'C:\wt\poll\target\n2-native\cargo-home' -Recurse -Force`.
Ressources/caches partagés et preuves brutes préservés ; 565 336 338 432
octets libres. Main inchangé, aucun tag créé, aucun push.
Revue indépendante : CLEAN sur les dix reçus, observations, calculs et
compte rendu contrôlés ; archivage vérifié ensuite par la session principale.
Budget : un cycle de correction de revue limité au compte rendu, non consommé.
