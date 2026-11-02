Avancée : quatre releases conformes ; 665 tests N1 réussis, zéro échec, 142 ignorés ; 834 identités couvertes ; revue indépendante CLEAN.
Blocage : aucun pour R8-FIX ; réserve de diagnostic GPU au démarrage de la recette CPU ultérieure.
Décision : lot livré en commits locaux ; qualification manuelle CPU sans CUDA ajoutée le 2 novembre 2026, sans push.

# R8-FIX — CPU livré sans import CUDA

Branche `fix/r8-worker-cpu-sans-cuda`, base `garde/n1-categories` **2dc28e80**, worktree `C:/dev/whisper/.wt/r8`. [Plan et accords de reprise](../cadrage/plan-r8-fix.md).

| Binaire | Commande via `ci/build-workers.ps1` et garde N1 | DLL CUDA / attendu | Verdict | SHA-256 |
| --- | --- | --- | --- | --- |
| Turbo CPU | build release `-p whisper-worker-cpu` | Non / non | PASS | `eb33d96b57ee47e5dcbe3238467df4f5656d055757374dbff5d58a5f751bbfef` |
| Turbo GPU | build release `-p whisper-worker-gpu` | cudart + nvcuda / oui | PASS | `5ff4c494b86754d563c5d41cc7cdcd1b2748aed9c9ffa5f929be29973cf6001b` |
| Parakeet CPU | build release `-p whisper-worker-parakeet-cpu --features parakeet-personal` | Non / non | PASS | `c9ba3a74eae6bb987f505d22c38679bea1b2a450dc3aaa67d911946d1b940e2a` |
| Parakeet GPU | même paquet, features `parakeet-personal,parakeet-cuda`, cible distincte, alias GPU | Non / non statique | PASS | `5783f06129dcbaec614ae825df44a3fdbf1002992f788a6b0e45f7756a543eba` |
| Harnais CPU (3 intégration + unitaire) et binaire package | test `--no-run -p whisper-worker-cpu` | Non / non | 5 PASS | SHA individuels dans les preuves brutes |
| Desktop debug | `cargo test` (membres par défaut) | Non / non | PASS | `cc6a7ea16ee358094a7c8b158cae4224fcdff5d6ebf592eb69ab5184ab6e050c` |

Toutes les invocations Cargo utilisent `--offline --locked`. Chaque configuration commence dans une cible neuve distincte. Le contrôle PE lit imports directs **et différés**, sans dumpbin ni chargement du binaire. Le témoin CPU historique (`67e668ff75c0c59522c1e3cdb3529518938209d962e3c66aa5662bbad7001cf4`) est refusé. Fixtures différées/non-PE et règles GPU positives/négatives : PASS. CPU `test --no-run` en cible neuve : cinq EXE sans CUDA ; Parakeet : six EXE sans CUDA ; common : un harnais sans CUDA. Aucun test natif exécuté. Graphes features : CPU seul sans cuda ; CPU+GPU avec cuda unifié.

| Commande tests | Identités listées | N1 : réussis / échecs / ignorés ; durée |
| --- | ---: | --- |
| `cargo test` | 737 | 616 / 0 / 121 ; 9,95 s |
| `cargo test -p whisper-worker-cpu` | 21 | 12 / 0 / 9 ; 7,69 s |
| `cargo test -p whisper-worker-parakeet-cpu` | 37 | 26 / 0 / 11 ; 4,53 s |
| `cargo test -p whisper-worker-common` (complément accepté) | 12 | 11 / 0 / 1 ; 1,78 s |
| `cargo test -p whisper-worker-gpu` | 27 | N2, liste seulement |
| **Total** | **834** | **665 / 0 / 142** sur 807 N1 ; 27 GPU listés seulement |

Comparaison des listes à la référence workspace : 834 identités, aucune manquante/ajoutée, multiplicité conservée. Le harnais GPU actuel listé n'importe pas CUDA ; le binaire GPU normal en importe. Son classement N2 demandé reste conservé ; aucun succès de ses corps de tests n'est revendiqué.

Fichiers : `ci/build-workers.ps1` centralise les constructions ; `ci/check-worker-imports.ps1` bloque les imports non conformes ; `ci/guard-level1.ps1` transporte les arguments/listes et réutilise une cible qualifiée ; `scripts/install-personal.ps1` aligne les sources et impose le contrôle avant copie ; section N1 d'`AGENTS.md` distingue les suites. Quatre plans de livraison (l6-perso, transcription unifiée, worker-common, micro-level) renvoient à la procédure canonique. Aucun Rust produit, manifeste, lockfile, modèle ni garde historique modifié.

Coût : releases CPU 38,27 s, GPU 155 s, Parakeet CPU 8,80 s/GPU 8,48 s ; cibles release **1,16 Gio**. CPU no-run 32,83 s ; Parakeet 6,93 s ; common 6,80 s ; inventaire GPU compilation 2 min 37 s. Neuf cibles **5,485 Gio**, cache Cargo **0,73 Gio**, hors temporaires divers. Espace libre initial **164,92 Gio**, final **156,42 Gio** ; la variation libre n'est pas une allocation attribuable exacte. Les échecs A/D initiaux (FTK1011, chemins de 269/271 caractères), défauts de transmission corrigés et liste vide invalidée sont conservés ; les chemins raccourcis autorisés ont permis les builds.

Revue indépendante **CLEAN** : R1 (publication), R2 (PE tronqué), R3 (restauration d'un snapshot CPU-CUDA) fermés après deux cycles. Témoins de restauration exacte après panne/redémarrage, contrôles conservés pour publication/rollback explicite. Bilan, archive, SHA et listes complètes d'imports des 21 artefacts contre-vérifiés.

Preuves : [résultats, chemins et SHA individuels](raw/r8-fix/r8-fix-results.json), [834 identités comparées](raw/r8-fix/r8-fix-coverage.json), [journaux complets](raw/r8-fix/r8-fix-campaign-logs.zip). Archive : **95 entrées** relues et comparées aux sources (tailles/SHA), **166 568 octets** ; SHA-256 `fdfa322bdef001221237cc1a08f3e57262cd37f19f65835c5655d31f8d77c6be`.

À la clôture initiale de R8-FIX : réinstallation, démarrage sans CUDA, inférence et fenêtre Windows restaient à qualifier dans les lots suivants. Aucune installation ni copie vers une installation dans R8-FIX ; aucune preuve d'absence d'écriture hors racines par confinement.

Complément du 2 novembre 2026 : l'utilisateur confirme sur son second PC Windows déclaré sans CUDA le démarrage du paquet portable, les transcriptions Turbo et Parakeet jugées bonnes, **Auto et CPU réussis pour chacun, sur CPU sans erreur**, et GPU grisé. Cette recette valide fonctionnellement le CPU sans CUDA dans ces conditions ; elle ne modifie pas les comptes N1 ni les verdicts PE. Le message initial `GPU indisponible` / `WorkerExited without End` et l'inventaire partiel (Parsec Virtual Display Adapter, Intel Iris Xe indéterminés) restent consignés comme réserve de diagnostic GPU. Aucun contrôle matériel supplémentaire ni qualification GPU ; exports, micro et performance non explicitement attestés. Paquet et limites détaillés dans le [bilan de fusion](BILAN-FUSION-PILE.md).
Nouvelle validation utilisateur du 2 novembre 2026, après reconstruction du desktop : le paquet portable actualisé (SHA-256 `844F5E3CAAA71F0ACEE870D2AFB83C05F27DC12EE545A88C28B459277F1E4E47`) fonctionne sur le PC sans CUDA avec **Turbo et Parakeet, chacun en CPU et Auto : PASS manuel** ; le message au démarrage est désormais correct. La réserve d'affichage de l'ancien paquet est donc levée pour cette recette. Ce retour n'ajoute aucune qualification GPU, mesure de performance ou attestation distincte des exports/micro ; comptes N1 et preuves PE inchangés. Détail dans le [bilan de fusion](BILAN-FUSION-PILE.md).
