Avancée : correctif implémenté ; 87 tests PASS, zéro échec ; revue indépendante CLEAN.
Blocage : aucun pour le code ; rendu Windows et nouvelle remise d'exécutable non exécutés.
Décision : livrer le correctif en commit local, sans installation, nouveau paquet ni push.

# Message de démarrage lorsque le GPU est indisponible

## Objectif et critères

Sur le second PC de recette, Turbo et Parakeet fonctionnent en Auto et CPU,
mais le démarrage affiche `WorkerExited without End`, des erreurs de transport
et un inventaire indéterminé. Présenter cette situation clairement sans prétendre
qu'aucun GPU matériel n'existe : Windows signale notamment Intel Iris Xe et Parsec.

- AC1 : en cas d'échec de détection, message principal en langage courant :
  « GPU indisponible pour la transcription. Vous pouvez utiliser le CPU. »
- AC2 : les diagnostics techniques de démarrage et d'inventaire restent
  consultables dans une section repliée « Détails de la détection GPU » ;
  aucune chaîne interne brute dans le résumé visible par défaut.
- AC3 : conserver les distinctions disponible, chargement, partiel et échec ;
  ne pas annoncer une absence matérielle ou un CPU effectivement utilisé avant traitement.
- AC4 : aucun changement de choix CPU/GPU/Auto, de préférence mémorisée,
  d'activation du bouton GPU, de détection, d'IPC ou de repli.
- AC5 : préserver le cas GPU disponible et tester les cas échec, inventaire
  partiel, détails techniques et choix explicite CPU/Auto.

## Six tâches et responsabilités

1. Session principale : versionner ce plan et recueillir l'accord précis.
2. `rust_implementer` en mode IMPLEMENT : lire les consignes desktop et
   ajuster uniquement la présentation du message et des inventaires.
3. Même implémenteur : ajouter les tests ciblés dans le fichier séparé existant.
4. Vérifier via `ci/guard-level1.ps1` : SelfTest, puis fmt, tests desktop,
   check et Clippy desktop ; offline/locked quand applicable, environnement
   isolé sous `target/parakeet-tests/message-demarrage-cpu`, cible partagée
   canonique, variables CUDA/GGML nettoyées. Espace libre relevé : environ 77 Gio.
5. `rust_review_coordinator` : revue indépendante et corrections par
   l'implémenteur, deux cycles maximum ; CLEAN ou réserves explicites.
6. Session principale : consigner le résultat dans ce document et commiter
   localement. Validation du rendu Windows ultérieure par l'utilisateur.

Chemins Rust autorisés : `crates/whisper-desktop/src/ui/presentation/gpu.rs`,
`ui/model/controller.rs`, `ui/model/view/` pour le seul rendu des messages GPU,
`ui/tests/compute_and_catalog.rs` et `ui/presentation.rs` pour exposer le rendu
(tous relatifs à la même crate ; ajout de ce dernier fichier autorisé explicitement).
Maximum dix fichiers modifiés, rapport compris ; dépassement soumis à arbitrage.
Manifestes, lockfile, adapters/core/workers, gardes, installation et paquet
portable existant exclus. Aucune compilation release, exécution native,
réinstallation ou nouveau paquet dans ce lot ; aucun push.

La validation manuelle CPU sans CUDA déjà consignée reste acquise pour le
paquet précédemment remis ; elle ne valide pas encore ce changement d'affichage.

## Accords d'exécution

L'utilisateur a accepté ce plan et demandé l'implémentation dans la session.
SelfTest N1 : 30 positifs et 58 négatifs PASS. Le premier appel fmt a été
refusé avant Cargo : deux noms de variables étaient transmis comme une seule
chaîne avec virgule. La campagne a été arrêtée. L'utilisateur a ensuite
autorisé explicitement la reprise avec transmission corrigée et l'ajout
de `ui/presentation.rs`. Aucun Cargo n'avait démarré lors du refus.

Après reprise, fmt et tests desktop ont passé avant un retrait de réexport ;
check a passé après ce retrait, mais Clippy `--all-targets` a échoué (E0425,
fonction de présentation introuvable depuis le test). La campagne est arrêtée.
Ces succès antérieurs ne valident pas le candidat final. Première revue
indépendante : R1 BLOQUANT, rétablir l'accès du test au helper ; R2 IMPORTANT,
remplacer la couverture booléenne insuffisante par des tests des projections
et du rendu réel (détails, inventaire, états et choix CPU/Auto). Aucune correction
ou reprise de campagne après cet échec sans arbitrage utilisateur.

L'utilisateur a autorisé la correction de R1/R2 et la reprise des vérifications.
Après un nouvel E0425 portant sur deux appels du helper de rendu dans les tests
ajoutés, il a autorisé leur correction et une exception bornée au cycle en
cours : corriger les erreurs de montage/compilation des seuls tests ajoutés,
puis reprendre sans arbitrage à chaque erreur de ce type. Un échec de test
réellement exécuté, de la garde/préflight ou de compilation produit reste
soumis à arrêt et arbitrage. Les ajustements de formatage du candidat restent
des corrections ordinaires autorisées. Aucun élargissement au code produit,
aux racines, aux gardes, à l'installation ou à une campagne native.

Les tests ont ensuite compilé et exécuté 51 cas unitaires : 50 PASS, un échec
sur la visibilité du détail d'inventaire après clic. Arrêt respecté. Le reviewer
a identifié en lecture seule la capture des formes pendant l'animation
d'ouverture du volet, avant le dessin de toutes les lignes. L'utilisateur a
autorisé explicitement la désactivation de cette animation dans le seul
contexte du test, en conservant clic et assertions, puis la reprise des
vérifications et de la revue. L'animation du produit n'est pas modifiée.

Après correction de l'API egui utilisée par le seul test, les assertions sur
le résumé, le repli et l'ouverture des détails passent. Une assertion
supplémentaire sur le nom Intel a échoué : le reviewer a confirmé que la
fixture fournit l'inventaire historique, mais aucun catalogue qualifié portant
les noms affichés. Le rendu historique agrégé est inchangé. Après arrêt,
l'utilisateur a autorisé le retrait de cette seule assertion non étayée par
la fixture, puis la reprise des vérifications et de la revue. Les assertions
sur les détails worker/inventaire restent conservées.

## Bilan du candidat final — 2 novembre 2026

Le résumé de démarrage indique « GPU indisponible pour la transcription.
Vous pouvez utiliser le CPU. » Les diagnostics du worker, de l'inventaire et
de la qualification sont accessibles dans « Détails de la détection GPU »,
replié par défaut. Les états de chargement et les messages non liés au GPU
restent distincts. Aucun changement de détection, d'IPC, de préférence ou de
choix CPU/GPU/Auto. L'animation n'est neutralisée que dans le contexte de test.

Quatre fichiers Rust : `ui/presentation/gpu.rs` porte le résumé et la section
de détails ; `ui/presentation.rs` expose le rendu et le helper de test ;
`ui/model/view/notices.rs` l'intègre ; `ui/tests/compute_and_catalog.rs` couvre
les projections et le rendu egui, les détails repliés puis ouverts par clic,
les inventaires partiel/échoué, chargement/complet et la conservation CPU/Auto.

Vérifications finales via `ci/guard-level1.ps1`, racine
`target/parakeet-tests/message-demarrage-cpu`, cible partagée canonique,
environnement isolé, `CUDA_PATH` et `CUDA_PATH_V12_9` retirées :

| Vérification | Résultat | Dossier sous `scratch/` |
| --- | --- | --- |
| SelfTest | 30 positifs / 58 négatifs PASS, acquis avant les appels Cargo | journaux du lot |
| `cargo fmt --check` | PASS | `20261102T141226648-20a55a51b85a422a921bcb5ea2641668` |
| `cargo test --offline --locked -p whisper-desktop` | 87 PASS / 0 échec / 3 ignorés | `20261102T141233854-8d9f4287b7764b14ac7e349253e09efc` |
| `cargo check --offline --locked -p whisper-desktop` | PASS | `20261102T141249910-b051c008fbda4372b74c3b11cc1c9ffa` |
| `cargo clippy --offline --locked -p whisper-desktop --all-targets` | PASS ; quatre avertissements adapters hors delta, aucun desktop | `20261102T141305334-81ea4f837d8847899ac1860b106b7080` |

Chaque dossier final contient `cargo-command.stdout.log` et
`cargo-command.stderr.log`. `git diff --check` PASS. Revue indépendante :
deux passes, un cycle de correction ; R1 et R2 fermés, verdict **CLEAN**.
Les arrêts intermédiaires et accords sont conservés ci-dessus ; les succès
finaux portent sur le dernier candidat sans modification Rust ultérieure.
Espace libre de clôture : 83,12 Gio ; modèles et audios non suivis préservés.

Limites : aucune exécution native, aucun modèle chargé, aucune fenêtre Windows
ouverte par l'agent, aucune compilation release, installation ou modification
du paquet portable déjà qualifié. La validation CPU sans CUDA de l'utilisateur
reste celle de l'ancien paquet. Un nouveau paquet devra être construit et
vérifié dans un lot de remise autorisé avant validation visuelle du correctif.
