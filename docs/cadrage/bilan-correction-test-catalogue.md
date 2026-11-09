Avancée : correctif des tests implémenté, revue indépendante CLEAN et validations N1 réussies le 9 novembre 2026.
Blocage : aucun ; tests ignorés, qualification native et fenêtre Windows non exécutés.
Décision : livré en commits locaux sur codex/fix-catalog-test, sans push ni fusion.

# Bilan — synchronisation des tests du catalogue desktop

Plan : [périmètre accepté](plan-correction-test-catalogue.md). Base produit `cc8e5439`, plan `66fa3dbf`, correctif `5644e06b`, correction de revue `ce64bc1d`. Worktree : `C:/wt/fix-catalog-test`.

**Changement.** Seul fichier Rust modifié : `crates/whisper-desktop/src/root/model_catalog/tests.rs` (107 ajouts, 7 retraits). L'attente bornée utilise la génération effectivement admise par l'application, les pauses de la fixture sont uniques par modèle et ses demandes sont fermées lors du déroulement d'un panic. Les huit scénarios existants conservent leurs assertions. Le nouveau test `final_catalog_publication_waits_until_application_admission_receives_it` retarde explicitement l'admission : attente avant livraison, résultat après livraison, file/génération active inchangées et absence d'effets. Aucun code produit, manifeste ou ordre de publication modifié.

**Revue indépendante.** `rust_implementer` a réalisé le candidat ; `rust_review_coordinator` a fait intervenir un `rust_reviewer` en lecture seule. Deux passes de revue, une correction : R1 IMPORTANT concernait un envoi de rendez-vous tardif après abandon du récepteur. Fermé sur `ce64bc1d`, verdict **CLEAN**, aucun constat ouvert. L'échec explicite côté test si le rendez-vous attendu n'arrive pas reste conservé.

**Vérifications exécutées.** Garde N1 SelfTest : 64 positifs/99 négatifs. Format et Clippy desktop avec les quatre lints de structure : exit 0, avertissements existants hors delta. Tests catalogue ciblés : 25/0. Le premier format-check avait échoué sur deux mises en forme ; correction manuelle et reprise explicitement autorisées par l'utilisateur, journaux conservés. Sur le candidat final :

| Sélection N1 | Réussis | Échecs | Ignorés |
|---|---:|---:|---:|
| Membres par défaut, `test --no-fail-fast` | 878 | 0 | 129 |
| `whisper-worker-cpu` | 12 | 0 | 9 |
| `whisper-worker-parakeet-cpu` | 26 | 0 | 11 |
| `whisper-worker-common` | 11 | 0 | 1 |
| **Total** | **927** | **0** | **150** |

Toutes les commandes N1 finales utilisent la garde, `--offline --locked` et `-WithoutCargoTargetDir` (`CARGO_TARGET_DIR_child=False`). Les trois compilations workers `--no-run` et les contrôles PE préalables des **10/10 harnais** passent ; SelfTest PE : 4 positifs/15 négatifs, témoin réel NON EXÉCUTÉ. Aucun corps de test GPU ni test natif activé. Le modèle personnel de 713 975 456 octets a conservé son SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`.

**Preuves et rétention.** [Archive des journaux](raw/correction-test-catalogue.zip) : 73 fichiers, 115 154 octets ; chaque entrée vérifiée par taille et SHA-256 contre sa source. SHA-256 ZIP : `7fc9b2b4e0edba33ad07ce49dad833c10480402ad2294aa6c06db46443d23dfd`. Commandes N1 détaillées et sorties PE dans `evidence/`, sorties Cargo dans `scratch/`. Les preuves durables sont dans cette archive Git. Aucun nettoyage exécuté ; aucune cible de compilation jetable créée. Les caches persistants présents du worktree `target/cargo-home-shared/` et `target/parakeet-tests/cargo-shared/` sont conservés, ainsi que les temporaires non sélectionnés pour suppression. Espace libre : environ 449,4 Gio avant préparation, 444,1 Gio après validation ; aucun Cargo/rustc restant observé.

La cause historique exacte n'est pas retracée rétroactivement. La revue, le scénario déterministe et N1 valident ce correctif de tests ; ils ne constituent pas une qualification native ou une validation interactive Windows. Bilan figé à la clôture.
