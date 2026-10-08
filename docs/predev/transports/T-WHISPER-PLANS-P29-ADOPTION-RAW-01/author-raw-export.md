**Attestation d’auteur — contribution DRAFT**

Auteur : `/root/plans_writer_l04`, rôle `rust_plan_writer`, owner proposé `AUTH-PLAN-L04`. Base PLANS : **P-WHISPER-28**. Parent : **D-WHISPER-28**, digest `50e2ab1ac3afd17d9e4a1e51932aaa37f812e3162db0207626656108352a6911`.

Après lecture intégrale des deux fichiers persistés, **j’adopte leurs octets comme ma proposition complète de plan L04** sur ce parent :

- `00_PLANS_INDEX.md` : SHA256 `27958c3ef71774f88d4fbbf1a3daffb3a5a090d2c27b7c4c64e12b06494e806c`.
- `17_L04_D28_PLAN.md` : SHA256 `bcb55f2bc6a1529dea8b216349af121602cb677729f76ea49fcc1fdcb2b811a2`.

Contrôle canonique exécuté : manifeste **P-WHISPER-29**, scope **PLANS**, parent **D-WHISPER-28**, digest `6fe6b6c9313074d38dff1b2ad816be3a8f5094dcc0190cbc64a754445c80529f`, résultat `ok:true`. SHA256 du manifeste : `67f15ebaa62752b7cb45d89404d9038daff86582fb71680a81e99591d645f1ae`.

La target binaire `whisper-worker-cpu` est confirmée par le package du même nom et `src/main.rs`, sans déclaration contraire dans son manifest ; la commande proposée `--bin whisper-worker-cpu` convient.

**Réserves précises :** l’adoption constitue une nouvelle attestation des fichiers persistés, pas une affirmation de copie verbatim de ma première réponse. Les six chemins supplémentaires, la réconciliation du candidat partiel et l’environnement restent des préconditions de lancement explicitement ouvertes. Dans les exclusions, « publication » désigne la distribution du produit ; la publication locale des archives décrite au §4 reste requise.

Aucune correction préalable requise par l’auteur, aucune décision DESIGN manquante identifiée. Restent au coordinateur : transport vérifiable de cette attestation, revue indépendante exacte P29/D28, traitement des findings éventuels et contrôle du gate. **Aucun CLEAN/READY auto-déclaré ; aucun build, test produit ou fichier modifié.**