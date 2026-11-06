Avancée : Parakeet CPU exécuté, 26 réussis/0 échec/11 ignorés ; common déjà vérifié.
Blocage : Turbo CPU nécessite une décision native précise, réutilisation du cache non démontrée.
Décision : accord ponctuel demandé avant toute compilation CMake Turbo CPU.

# Validation des workers — mandat du 6 novembre 2026

Demande courante : « il reste à faire : les tests des workers, toujours non exécutés ». Candidat `3251ed41`, source corrigée `a2ef198d`. N1 membres par défaut : 774 réussis/0 échec/128 ignorés ; worker-common : 11 réussis/1 ignoré avec PE préalable, preuve encore applicable car ses entrées sont inchangées. Les sélections restantes sont worker-cpu et worker-parakeet-cpu, pas les corps de tests GPU de niveau 2.

La demande autorise les compilations Rust nécessaires aux tests, y compris le binaire Parakeet référencé par CARGO_BIN_EXE, dans la cible de test. Aucune installation ou copie de remise n'est remplacée ; aucun moteur natif, DLL, dépendance ou configuration produit n'est reconstruit/modifié. La suspension native reste en vigueur : ni cache présent ni harnais synthétique ne prouvent à eux seuls l'absence de rerun de whisper-rs-sys/build.rs.

Trois tâches :

1. Principal/implémenteur VERIFY : état Git, espace ≥30 Gio, préflight des graphes, scripts et environnements ; budget de diagnostic trois essais ou trente minutes, sans outillage nouveau.
2. Parakeet CPU : via garde, `test --offline --locked -p whisper-worker-parakeet-cpu --no-run`, PE de tous harnais `-Kind ParakeetCpu` et du binaire CPU éventuel avant exécution ; puis sélection `test --offline --locked -p whisper-worker-parakeet-cpu`, sans native-tests/ignored. Turbo CPU : même séquence seulement si la réutilisation sans production native suspendue est démontrée avant appel. Sinon arrêt de cette sélection et décision précise ; aucune option de contournement des build.rs.
3. Principal : consigner compteurs, codes, ignorés et limites, archiver/vérifier les petites preuves et commiter localement. Aucun push ni modification Rust prévue ; un échec impose arrêt/arbitrage selon AGENTS, pas de correction implicite.

Cargo passe exclusivement par `ci/guard-level1.ps1 -LotRoot target/parakeet-tests/inspect-micro-demarrage/validation-workers`. Cache/cible partagés, temporaires isolés, offline/locked, garde et PE existants inchangés. Complément et archive `docs/cadrage/raw/validation-workers-inspect-micro-demarrage.zip` ; les bilans figés peuvent seulement recevoir ce lien. Aucune cible fraîche ni cache isolé demandé.

Résultats Parakeet CPU : graphe gardé sans production native ; compilation Rust `--no-run` réussie, puis PE ParakeetCpu réussi sur les cinq harnais et le binaire avant première exécution. Sélection complète code 0 : main 23 réussis ; decoder_parity 0 réussi/1 ignoré ; native_diagnostics_absence 2 réussis/2 ignorés ; native_import 1 réussi/8 ignorés ; worker_contract zéro test. Total 26 réussis, zéro échec, 11 ignorés. Preuves dans la racine de lot : `pe-parakeet.log`, scratch `20261106T144918193-6efb4d14e9534301bcf3652f273cc1ed` et `20261106T144950582-926ad01665f24a6c834149f3b050ceae`. Aucun worker installé ou de remise remplacé.

Turbo CPU : trois lectures diagnostiques, aucun appel compilant. L'unique bibliothèque whisper-rs-sys sans feature CUDA observée (`ab8a2755052481cb`) référence dans son fichier .d les sources de `C:/wt/build-natif/target/cargo-home-shared/`, tandis que la garde utilise le cache du dépôt courant. La réutilisation sans rerun n'est pas démontrée ; les autres variantes examinées sont CUDA et ne constituent pas une preuve CPU. `whisper-rs-sys 0.15.0/build.rs:301` appelle `config.build()` (CMake). Preuve `cpu-preflight.log`, aucun contournement ni modification de cache/environnement/source.

Arbitrage demandé dans la session : exception ponctuelle de compilation native CPU de whisper-rs-sys par son build.rs/CMake lors de la seule sélection Cargo worker-cpu via garde N1, puis PE et tests. Pas de CUDA/GPU, téléchargement, installation ou remplacement des workers livrés. La suspension depuis la WHEA du 5 novembre reste applicable tant que cet accord précis n'est pas reçu. Aucun résultat CPU revendiqué.
