Avancée : le lanceur restreint, les deux ACL temporaires et une session ETW courte ont été préparés ; les ACL ont été restaurées.
Blocage : le premier processus restreint s'est arrêté avant sa sonde avec `0xC0000142` ; aucune écriture du banc n'a pu être éprouvée.
Décision : NO GO pour qualifier le confinement préventif, les processus enfants ou une campagne FEAS-PM-01.

# FEAS-PM-01 — qualification courte du confinement, tentative Q1

## Périmètre et état

Cette tentative suit l'autorisation du 21 octobre 2026 pour un lanceur autonome, deux ACL NTFS, un Job Object et ETW. Elle ne comprend aucun essai Parakeet, CUDA, A/B ou FEAS-PM-01 et ne modifie pas le code produit. Les racines d'écriture du banc restent `target/parakeet-tests/micro-feasibility/` et `target/parakeet-tests/cargo-shared/`. Les preuves de la tentative sont dans `target/parakeet-tests/micro-feasibility/confinement-q1/` ; l'exécutable autonome est dans `target/parakeet-tests/cargo-shared/`.

| Point | Verdict | Preuve et limite |
| --- | --- | --- |
| Prévention des écritures | **FAIL — non démontrée** | Le premier essai positif n'a pas atteint `CreateFileW`. Aucune écriture positive ni aucun refus `ACCESS_DENIED` ne permettent de conclure. |
| Processus enfants | **FAIL — non démontré** | Aucun enfant de la sonde n'a été lancé. Le Job Object et l'absence d'héritage de handles ont été configurés par le lanceur, sans attestation dans la sonde. |
| Attribution ETW | **PARTIAL** | La trace montre la création et l'arrêt du processus restreint, avec zéro événement perdu selon `tracerpt`. Elle ne démontre pas encore l'attribution d'une opération d'écriture ou d'un refus de la sonde. |
| Restauration des ACL | **PASS pour les deux racines contrôlées** | La comparaison élevée finale indique SDDL et protections d'héritage identiques aux sauvegardes initiales ; le SID dédié n'est plus présent dans les chemins sondés. |
| Aptitude à une campagne longue | **NON DÉMONTRÉE** | 26 542 080 octets d'ETL en 24 s, avec une limite non circulaire de 256 Mo. Ce court essai ne permet pas de dimensionner une campagne longue. |

## Déroulement vérifiable

Avant modification, les chemins canoniques, points de réanalyse, ACL et l'absence du fichier cible hors racine ont été contrôlés. Les deux ACL ont été sauvegardées avec leur SDDL et leurs états d'héritage dans `confinement-q1/acl-before.json`. Une ACE `Modify` héritée pour le SID restrictif `S-1-5-21-4044558122-3660764095-593334293-1101` a été ajoutée exclusivement aux deux racines autorisées. Aucun changement global de permissions Windows n'a été requis.

Le lanceur autonome `confinement-q1/launcher.rs` crée un jeton `WRITE_RESTRICTED | DISABLE_MAX_PRIVILEGE`, puis crée le processus suspendu sans héritage de handles, l'attache au Job Object configuré avec `KILL_ON_JOB_CLOSE` sans drapeau de sortie, et le reprend. La session ETW `FEAS_PM01_Q1` a été déclarée prête avant le lancement de la sonde ; l'écriture de 68 octets du contrôleur observée avant `ready` lors de la qualification ETW antérieure ne doit pas être attribuée à ce banc.

Le lanceur a journalisé ses PID 76908 et 16864, mais le processus restreint s'est terminé avec `0xC0000142` (`STATUS_DLL_INIT_FAILED`) avant l'entrée dans la sonde. L'essai a été arrêté à ce point : pas de second contrôle positif, de test négatif, d'enfant, de handle hérité ou de chemin indirect. Le fichier cible hors racine est resté absent. La cause de l'échec d'initialisation n'est pas établie ; les événements sur `Conhost.exe` ne prouvent pas à eux seuls un défaut de droits sur le bureau Windows.

La session ETW a été arrêtée ; `logman query -ets` ne montre plus `FEAS_PM01_Q1`. Une session préexistante `MSDTC_TRACE_SESSION` n'a pas été touchée. `trace-summary.txt` rapporte 200 514 événements, zéro perdu et 405 tampons sur 24 secondes. La trace complète `trace.etl`, l'export `trace.xml`, les scripts et journaux sont conservés sous la racine du lot. Aucun fichier litigieux n'a été supprimé.

La restauration initiale par PowerShell a révélé une divergence du drapeau de protection SACL malgré un SDDL égal. La correction native autorisée pour la restauration a remis les deux drapeaux d'héritage à leur valeur initiale. `acl-native-restore.log` atteste, pour chacune des deux racines, `sddl=True access_inheritance=True audit_inheritance=True` à `2026-10-21T17:03:26Z`. Les vérifications en lecture seule confirment l'absence de l'ACE dédiée sur les racines et les descendants sondés ; elles ne constituent pas un audit exhaustif de l'arborescence.

## Revue indépendante et suite

La revue indépendante conclut **FINDINGS / NO GO** : le démarrage échoué empêche les preuves de prévention, de propagation aux enfants et de contournement ; le contrôleur ETW actuel n'arrête pas automatiquement le banc en réponse à un événement suspect pendant la fenêtre d'essai. Les booléens imprimés par le lanceur décrivent ses appels et ne sont pas une attestation du processus enfant.

Une nouvelle tentative exigerait d'abord un diagnostic de `0xC0000142`, puis une surveillance et un arrêt pendant l'essai, une attestation dans la sonde du jeton et du Job Object, et les contrôles positifs/négatifs prévus. Si le diagnostic demande des droits sur un bureau Windows distinct ou une autre modification système non autorisée, un accord précis supplémentaire sera nécessaire avant cette modification. Aucun nouveau test n'est lancé par ce rapport.
