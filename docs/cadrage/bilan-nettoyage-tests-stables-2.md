Avancée : suppression humaine des trois racines prévue, exécutée et vérifiée le 9 novembre 2026.
Blocage : aucun ; archive, scratch et cache persistant conservés.
Décision : nettoyage clos en commit local, sans merge ni push ; bilan du lot inchangé hors lien vers ce complément.

# TESTS-STABLES-2 — complément de nettoyage

Après remise des commandes PowerShell limitées aux trois racines du [plan](plan-tests-stables-2.md#racines-présentes-et-commandes-proposées-à-lhumain), l'utilisateur confirme : « ok commande executée avec succes. » La session principale a effectué les vérifications suivantes le **9 novembre 2026 à 13 h 28 Europe/Paris**, depuis `C:/wt/tests-stables-2`, branche `tests-stables-2`, base documentaire `74de511c`. Aucun nettoyage supplémentaire exécuté par l'agent.

| Chemin absolu sélectionné | Taille avant suppression | Vérification après exécution humaine |
|---|---:|---|
| `C:/wt/tests-stables-2/target/tests-stables-2/targets/n1` | 4 194 884 389 octets | Absent |
| `C:/wt/tests-stables-2/target/tests-stables-2/temp` | 678 140 595 octets | Absent |
| `C:/wt/tests-stables-2/target/tests-stables-2/evidence` | 668 200 octets | Absent |

Volume de fichiers sélectionnés : **4 873 693 184 octets**, soit environ **4,87 Go**. Espace libre observé sur C: après suppression : **470 242 742 272 octets**, soit **470,24 Go** ; cette mesure porte sur le volume entier.

Éléments conservés et vérifiés :

- [Archive des preuves](../architecture/raw/TESTS-STABLES-2-PREUVES.zip) : présente, **1 386 051 octets**, **865 membres** après réouverture. SHA-256 inchangé : `edf283407b5cb1d3476b421f2e6b0355c5b7a5929b290a8f8c639c23ac0830ff`. L'archive est identique à celle dont chaque membre avait été vérifié avant suppression.
- `C:/wt/tests-stables-2/target/tests-stables-2/scratch` : présent, **837 fichiers / 9 797 101 octets**, **2 jonctions internes** conservées ; mêmes inventaires quantitatifs qu'avant nettoyage.
- `C:/wt/tests-stables-2/target/cargo-home-shared` : présent, **22 417 fichiers / 651 825 324 octets**, aucun reparse point ; mêmes inventaires quantitatifs qu'avant nettoyage.
- [Bilan figé du lot](../architecture/TESTS-STABLES-2-BILAN.md), plan, worktree et branche conservés. Aucun changement de source, validation Cargo ou nouvelle revue dans ce complément.

La cible de compilation est désormais absente : aucune nouvelle campagne ne doit supposer sa présence via `-ExistingTarget n1`. Ce constat décrit l'état après nettoyage et n'autorise aucune reconstruction. Les trois cas restent non reproduits, avec les réserves du bilan initial.
