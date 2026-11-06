Avancée : demande explicite de terminer les tests workers restants ; préflight engagé.
Blocage : Turbo CPU peut relancer whisper-rs-sys/CMake, toujours suspendu.
Décision : exécuter les sélections compatibles et demander seulement l'arbitrage natif éventuellement nécessaire.

# Validation des workers — mandat du 6 novembre 2026

Demande courante : « il reste à faire : les tests des workers, toujours non exécutés ». Candidat `3251ed41`, source corrigée `a2ef198d`. N1 membres par défaut : 774 réussis/0 échec/128 ignorés ; worker-common : 11 réussis/1 ignoré avec PE préalable, preuve encore applicable car ses entrées sont inchangées. Les sélections restantes sont worker-cpu et worker-parakeet-cpu, pas les corps de tests GPU de niveau 2.

La demande autorise les compilations Rust nécessaires aux tests, y compris le binaire Parakeet référencé par CARGO_BIN_EXE, dans la cible de test. Aucune installation ou copie de remise n'est remplacée ; aucun moteur natif, DLL, dépendance ou configuration produit n'est reconstruit/modifié. La suspension native reste en vigueur : ni cache présent ni harnais synthétique ne prouvent à eux seuls l'absence de rerun de whisper-rs-sys/build.rs.

Trois tâches :

1. Principal/implémenteur VERIFY : état Git, espace ≥30 Gio, préflight des graphes, scripts et environnements ; budget de diagnostic trois essais ou trente minutes, sans outillage nouveau.
2. Parakeet CPU : via garde, `test --offline --locked -p whisper-worker-parakeet-cpu --no-run`, PE de tous harnais `-Kind ParakeetCpu` et du binaire CPU éventuel avant exécution ; puis sélection `test --offline --locked -p whisper-worker-parakeet-cpu`, sans native-tests/ignored. Turbo CPU : même séquence seulement si la réutilisation sans production native suspendue est démontrée avant appel. Sinon arrêt de cette sélection et décision précise ; aucune option de contournement des build.rs.
3. Principal : consigner compteurs, codes, ignorés et limites, archiver/vérifier les petites preuves et commiter localement. Aucun push ni modification Rust prévue ; un échec impose arrêt/arbitrage selon AGENTS, pas de correction implicite.

Cargo passe exclusivement par `ci/guard-level1.ps1 -LotRoot target/parakeet-tests/inspect-micro-demarrage/validation-workers`. Cache/cible partagés, temporaires isolés, offline/locked, garde et PE existants inchangés. Complément et archive `docs/cadrage/raw/validation-workers-inspect-micro-demarrage.zip` ; les bilans figés peuvent seulement recevoir ce lien. Aucune cible fraîche ni cache isolé demandé.

Résultats à compléter ; aucune sélection restante déclarée réussie à ce stade.
