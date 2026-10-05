# CHANGE-001 — essai sur PC propre différé

**Source de l'autorité produit :** message utilisateur du chat courant, item `01a10bae-9b2c-78d2-944c-5e72d703271c` : « sors le besoin d'un test sur PC propre - hors scope pour l'instant ». Le même message demande les mesures VAD, perte et délai live. Le message `01a10ba4-e6c6-7090-af5a-6204e012873c` confirme qu'aucune VM ni second PC Windows n'est disponible. Captures fidèles dans `14_USER_MESSAGES.json`.

**Effet V1 / version de périmètre 2 :** la qualification sur Windows propre est différée hors du gate DESIGN actuel. Le comportement produit accepté reste : modèle acquis et vérifié pendant l'installation, premier lancement sans téléchargement caché, choix CPU/Auto/GPU conforme. Sur le PC actuel, le paquet portable et son PATH réduit demeurent des preuves partielles. Aucune réussite d'installation propre n'est affirmée.

**Impact :** `REQ-05/AC-05`, `SPIKE-01`, `R-01`, readiness/gates et question de plateforme sont ajustés. L'essai sur machine propre demeure en backlog sans échéance acceptée ; son retour exigera un environnement disponible et une décision explicite. Les mesures demandées sont ajoutées dans `21_MEASUREMENTS.md`. `D-WHISPER-04` et sa revue ne sont pas transposés à D5 ; le nouveau manifeste et une revue indépendante sont requis. Aucun PLANS ni code produit existant à migrer.
