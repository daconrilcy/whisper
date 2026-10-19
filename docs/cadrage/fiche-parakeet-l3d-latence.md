Avancée : l'import Parakeet GPU Test tient le seuil de 15 s au premier EN5 sur trois sessions release neuves.
Blocage : aucun constat de revue ouvert sur L3d ; la validation dans la fenêtre Windows reste réservée à l'utilisateur.
Décision : retenir le candidat L3d pour le profil Test, sans changer la clé du worker GPU figé.

# Fiche de qualification L3d — latence et persistance de l'import GPU

Date : 19/10/2026. Plan : [PLAN-PARA-L3d v4](plan-parakeet-l3d-latence-import.md). Profil : Test, import Parakeet `import-single-pass` FR/EN jusqu'à 30 min, GTX 1080 Ti unique. Le worker GPU reste celui de la [fiche L3b](fiche-parakeet-l3-gpu-test.md) : SHA-256 `5054612a05fbfd1039e108def3aa4e598268d2b4274b8c1dc96cd6b8e3f2cd6d`. Modèle : SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`. Les neuf DLL GPU de L3b, le worker CPU R6 et ses cinq DLL restent inchangés ; la garde a comparé les 26 empreintes protégées avant et après la campagne finale, toutes identiques.

| Carte | Résultat | Statut |
| --- | --- | --- |
| D1 — attribution du délai | 99 événements `Segment` et 99 `Persisted`, environ 7,2 s de parcours de rafraîchissements pour environ 0,3 s d'écritures d'archive cumulées dans le diagnostic initial. | PASS |
| D2 — drainage et saturation | Au plus quatre événements par `Refresh`. `Ready` reste observable avant `Failed`. Une file de huit effets saturée au neuvième segment conserve l'effet ; après reprise, les segments 1 à 9 sont persistés une seule fois et dans l'ordre avant publication. Déconnexion fatale distincte et testée. | PASS |
| D3 — contrôles manquants | Test R5 de confinement exécuté une fois et PASS ; recette historique L3b FR/EN réellement exécutée et PASS. Garde : 12/12 auto-tests. | PASS |
| D4 — EN5 release | Trois sessions neuves : premiers imports **10,744 / 10,775 / 10,772 s** ; seconds imports **9,735 / 9,702 / 9,708 s**. TXT/SRT publiés ; un SHA complet adaptateur du modèle par session, aucune relecture au second import. | PASS |
| D4 — autres recettes | EN30 **99,24 s** et TXT/SRT ; A4 FR87/FR27/EN15 ; cinq scénarios R8 : tous PASS sous garde. | PASS |
| Gate | `cargo fmt --all -- --check`, `cargo build --offline --locked --workspace`, `cargo test --offline --locked --workspace --no-fail-fast` : codes 0, aucun test échoué. | PASS |

La suite workspace signale 34 tests `#[ignore]` comme **NOT RUN** dans ce gate. Les cinq scénarios R8 ont été lancés séparément et ont passé ; les 29 autres restent des recettes matérielles, de mutation isolée ou de référence hors du périmètre L3d. Les noms exacts sont dans `target/parakeet-tests/l3/l3d/logs/Workspace.stdout.log`. Les trois journaux EN5 conservés sont sous `target/parakeet-tests/l3/l3d/logs/en5-release-captured-session-{2,3,4}/` ; les SHA du gate sont dans `Workspace-sha-before.json` et `Workspace-sha-after.json` du même dossier `logs/`.

La revue indépendante a rendu **CLEAN** au second passage : L3D-R1, risque de perte lors de la saturation de la file de persistance, est fermé après une correction. La validation finale dans la fenêtre Windows n'est pas couverte par ces tests.
