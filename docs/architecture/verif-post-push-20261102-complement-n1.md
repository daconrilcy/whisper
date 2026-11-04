Avancée : reprise autorisée sur e8f9094f ; préflight et auto-test des imports PE exécutés.
Blocage : auto-test PE en échec, témoin négatif réel absent ; aucune commande Cargo lancée.
Décision : arrêt sans correction conformément au budget ; N1 reste non qualifié.

# Complément VERIF-POST-PUSH — 2 novembre 2026

Suite au [rapport figé](verif-post-push-20261102.md), l’utilisateur autorise la poursuite de N1 malgré le tag absent, sans push ni correction CI. Le rapport initial est préservé.

**Candidat et environnement.** `HEAD = origin/main = e8f9094f735c33db1f5ba59ba7041cf329bcf027`, aucun changement suivi. Worktree principal, autres chats récents inactifs dans l’inventaire disponible. Préflight : 622,92 Gio libres. La cible partagée `target/parakeet-tests/cargo-shared/` et le cache source codé dans la garde, `target/parakeet-tests/refacto-parakeet-live/cargo-home/registry/src`, sont absents. Aucun cache, cible ou environnement Cargo préparé.

**Premier contrôle, avant Cargo.** Les auto-tests sont placés au préflight conformément aux obligations de la garde. `pwsh -NoProfile -File ci/check-worker-imports.ps1 -SelfTest` échoue, code **1**, à `ci/check-worker-imports.ps1:191` :

> Required real negative witness is missing: C:\dev\whisper\target\parakeet-tests\cargo-shared\release\whisper-worker-cpu.exe

Ce témoin est épinglé par le script au SHA-256 `67e668ff75c0c59522c1e3cdb3529518938209d962e3c66aa5662bbad7001cf4`. Le contrôle ne peut donc pas être remplacé par un EXE quelconque. Échec d’environnement, aucun test produit en échec identifié, aucun commit de production incriminé. Aucun diagnostic supplémentaire ni correction entrepris.

| Vérification prévue | Résultat | Référence fusion | Écart / explication |
|---|---|---|---|
| `cargo fmt --check` | NON EXÉCUTÉ | succès attendu | arrêt au préflight |
| `cargo clippy --workspace --all-targets` | NON EXÉCUTÉ | 0 erreur / 92 avertissements | comparaison impossible |
| `cargo test` | NON EXÉCUTÉ | 616 PASS / 121 ignorés / 0 échec | comparaison impossible |
| `cargo test -p whisper-worker-cpu` | NON EXÉCUTÉ | 12 / 9 / 0 | comparaison impossible |
| `cargo test -p whisper-worker-parakeet-cpu` | NON EXÉCUTÉ | 26 / 11 / 0 | comparaison impossible |
| Garde mode Structure | NON EXÉCUTÉ | PASS / 0 retour silencieux | arrêt au préflight |
| Auto-test garde N1 | NON EXÉCUTÉ | succès attendu | arrêt après échec PE |
| Auto-test imports PE | **ÉCHEC, code 1** | succès attendu | témoin négatif réel absent |

**Portée et suite proposée.** Le test `settings` documenté dans le rapport initial reste non exécuté dans ce lot. Tag et CI inchangés ; aucun workflow relancé. Pour reprendre : retrouver le témoin exact dans les ressources conservées et préparer un cache hors ligne isolé ; à défaut, prévoir un lot distinct de révision du prérequis PE, par nouveau commit, sans affaiblir silencieusement le contrôle. Cette proposition n’est pas une correction effectuée ni une autorisation nouvelle.

**Rétention et clôture.** Journaux sous `target/parakeet-tests/verif-post-push-n1-20261102/`, archivés dans `journaux.zip` : 4 fichiers, nombre et SHA-256 de chaque entrée vérifiés par la session principale. Aucun cargo-home ni cible propre au lot créé, aucune suppression nécessaire ; 624,32 Gio libres à clôture. Aucun code, test, manifeste, AGENTS, tag, commit ou push modifié/créé. `git diff --check` et `git diff --exit-code` réussissent ; seuls les deux rapports sont ajoutés aux non-suivis préexistants `models/` et `sound example/`. Revue indépendante : **CLEAN sur le rapport**, sans qualification N1 ni revérification indépendante de l’archive. Complément figé ; rapport initial intact.
