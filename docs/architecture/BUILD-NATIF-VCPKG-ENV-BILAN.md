Avancée : deux préparations vcpkg réelles terminées, mêmes versions MSVC et SDK.
Blocage : perte supposée de VSCMD_SKIP_SENDTELEMETRY non reproduite dans le témoin.
Décision : aucun correctif KEEP, aucune nouvelle compilation ; arrêt conformément à l'autorisation conditionnelle.

# BUILD-NATIF — témoin vcpkg, 5 novembre 2026

| Préparation, même exécutable épinglé et triplet cible | Valeur en sortie | Code |
| --- | --- | --- |
| Sans VCPKG_KEEP_ENV_VARS | VSCMD_SKIP_SENDTELEMETRY=1 | 0 |
| Avec conservation explicite | Même valeur 1, présente deux fois dans stdout | 0 |

Les versions sont identiques : **MSVC 14.44.35207, SDK 10.0.26100.0**. Le SHA de vcpkg correspond au pin. Aucun port compilé pendant ces préparations.

**Verdict du témoin : FAIL — hypothèse non reproduite.** Les commandes ont réussi ; c'est la condition « variable perdue sans KEEP » qui n'est pas observée. L'autorisation ne permettait le correctif que sur preuve de cette perte. Le pilote, son manifeste et la garde restent donc inchangés ; aucun ajout préventif inutile.

La portée est limitée à `vcpkg env --triplet=x64-windows-static-md`. Ni les environnements internes de compilation des ports ni le triplet hôte ne sont qualifiés par ce témoin. La télémétrie de cpu7c2 reste inexpliquée : aucune responsabilité supplémentaire attribuée.

La fonction exécutée et les sorties brutes sont conservées dans les preuves. Sa seule addition préparatoire au fichier de tests a été retirée après comparaison avec HEAD ; aucun diff du code CI/natif/Rust. Revue indépendante des conclusions : **CLEAN**.

**Équivalence toujours non démontrée.** Compteurs cumulés inchangés : 2 CPU commencées /4, 0 GPU /2, 0 N2 /10 ; N1 non exécuté. Aucun remplacement installé, nettoyage, fusion ou push. Anciennes archives, bilans et dossiers protégés conservés.

**Preuves :** [archive vérifiée](raw/build-natif-vcpkg-env/temoin.zip), **10 fichiers + manifeste**, 11 407 octets ; SHA-256 `a00dd7c558fb48a186d043a0b0ce001dc072266af53a270e35778fcc95e9f113`. ZIP rouvert, noms uniques, nombres/tailles/empreintes conformes. Le [plan et résultat détaillé](../cadrage/build-natif-vcpkg-environnement.md) consignent la condition autorisée et les limites. Fusion complète et remplacement des DLL déconseillés tant que les preuves initiales manquent.
