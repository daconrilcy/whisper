Avancée : T7 est terminé et sa revue indépendante est CLEAN.
Blocage : le polling global reste à 24 ; les phases T8–T9 n’ont pas été autorisées.
Décision : conserver le checkpoint T6 figé et suivre les résultats ci-dessous.

# Suivi post-T6 — REFACTO-POLL-PARAKEET

## T6 : inventaire de clôture précisé

Mesures sans suivre les reparse points : target/refacto-poll-parakeet 1 967 614 512 octets ; target/n2-native 13 481 992 679 ; cible lot target/parakeet-tests/cargo-shared 7 686 784 144 ; cache Cargo persistant target/cargo-home-shared 651 825 324 ; remise deliverables/refacto-poll-parakeet 38 651 904. Aucun nettoyage n’a été exécuté. L’espace libre au moment de la clôture T6 n’a pas été capturé. Les usages et reparse points n’ayant pas été revérifiés pour chaque racine, aucune commande de suppression n’est proposée ; conserver les racines.

## T7 : extraction Ready

Branche refacto-poll-parakeet, delta de T6 443e6a8a à 6dd93f08. Commits d’extraction/correction : 6896c368, 7e47dbbd, 12f00aa3, 93aad801, e9f713f8, 1cbe9266, 6dd93f08. Seul crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs a changé pendant T7.

handle_parakeet_ready : 99 lignes, 3 arguments, complexité cognitive 12. poll_parakeet_worker : complexité 24 ; le reste du polling reste pour T8/T9. Les tests P07/P08 ciblés passent 2/2 ; cargo test -p whisper-adapters passe avec Q/D ; Clippy JSON avec recette-hooks passe ; N1 passe 944/0/129. Références gelées et assertions existantes inchangées. Revue indépendante finale CLEAN pour 6dd93f08 ; T7-R1 fermé. La revue constate que les tests ne ciblent pas explicitement les lectures de qualification sur backend invalide ; l’ordre inconditionnel est démontré par le diff.

Aucun merge ni push. La recette dans la fenêtre Windows reste non exécutée. Ce suivi complète les faits postérieurs au bilan figé sans modifier son récit de clôture.
