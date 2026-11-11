Avancée : R2-bis documenté ; R2B-03 caractérisé rouge puis protégé par le correctif candidat ; N1 gardée réussie.
Blocage : revue indépendante BLOCKED sur R2B-04, publication incomplète pouvant autoriser le retrait d’un snapshot.
Décision : arrêt sans autre correction ; recommandation d’abandonner 91de456e.

# REVUE-SNAPSHOTS-LEGACY — bilan final

- Base examinée : `4cd1a64b` → `91de456e`, amendement R2-bis du 11 novembre (`ee695b1a`). L’amendement autorise le retrait après archive complète concordante dans Bound et Retiring ; archive incomplète, divergente, diagnostiquée ou en reprise de publication : protégée.
- Parcours recensés : `SourceBinding::retire` effectue le retrait normal ; `recover_new_entry` et `resume_retirement` exécutent la récupération explicite RTC-05. Le helper candidat `archive_generation_is_unverifiable` vérifie l’état physique dans l’inspection et avant les deux suppressions. Le pending R2 exact reste soumis au garde source existant.
- Couverture : Bound concordant, SHA divergent, archive incomplète ; Retiring concordant et diagnostic ; archive divergente complète omise par index inactif. Cette dernière échoue avant le helper (run `20261111T093844519-0799b8792c0d44e1a5d332816e72d3e7`), puis passe après (`20261111T094927027-da3c15cc5f20422392526883f56fd8b9`).
- Validation finale : N1 gardée `cargo test --no-fail-fast`, sans `CARGO_TARGET_DIR` : code 0 ; adapters lib 574 réussites, 0 échec, 49 ignorés ; autres cibles réussies. Auto-test `guard-level1.ps1` : 64 cas positifs, 99 négatifs.
- Revue indépendante, question limitée aux suppressions non autorisées : **BLOCKED — R2B-04**. Avec `pending`, réservation et source exacte, un manifeste divergent et `r11-publication.json` peuvent subsister sans `CURRENT`; le scan peut traiter cet état comme un pending R2 vérifié, puis Bound/Retiring peut retirer snapshot et fiche malgré la publication incomplète. Aucun correctif supplémentaire n’est appliqué, conformément à la consigne finale.
- Limite acceptée : R2B-02, une réservation de publication résiduelle peut bloquer une récupération autrement autorisée ; refus par défaut conservé.
- Commits : amendement `ee695b1a`, test Retiring `277e3119`, bilan à commiter séparément. Le helper R2B-03 et son test sont laissés non commités. Aucun merge/push ni accès aux données utilisateur.

**RECOMMANDATION : abandon de 91de456e.**
