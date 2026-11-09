Avancée : périmètre accepté par l'utilisateur le 9 novembre 2026, avant implémentation.
Blocage : aucun à la préparation ; validation à exécuter sous la garde N1.
Décision : correctif des tests uniquement, revue indépendante et commits locaux.

# Correction de la synchronisation des tests du catalogue desktop

Base : `cc8e5439`. Objectif produit : fiabiliser les tests garantissant qu'une observation de modèle actualise le message de l'application sans rejouer une commande, changer la file ou produire des effets.

Autorisation de session : « Applique le correctif limité aux tests avec rust_implementer, ajoute le scénario déterministe, puis fais la revue indépendante et les validations N1 sous garde. Commits locaux uniquement. » Le worktree `C:/wt/fix-catalog-test`, sa branche `codex/fix-catalog-test` et ses fichiers N1 ont également été autorisés explicitement. Ce document transcrit le périmètre accepté dans la conversation ; il ne modifie aucune règle du dépôt.

## Périmètre et critères

Un seul fichier Rust autorisé : `crates/whisper-desktop/src/root/model_catalog/tests.rs`. Aucun changement produit, ordre de publication, API, manifeste ou dépendance.

- C1 : conserver la génération finale reçue sur le catalogue et attendre, par `Refresh` et avec le `WAIT` existant, sa réception effective par l'application. Ne pas attendre un message favorable ni retirer directement un élément de son récepteur. Diagnostic d'expiration : génération attendue/reçue, état du canal, dernier message ; attente non active.
- C2 : conserver les huit scénarios existants (Traiter/Reprendre, prêt/absent/corrompu/identité invalidée), leurs assertions de message, file, génération active et absence d'effets.
- C3 : les rendez-vous de la fixture sont uniques par modèle ; une réobservation retourne normalement le résultat synthétique. Fermer les demandes aussi pendant le déroulement d'une assertion, sans double panic. Ne pas masquer une erreur de synchronisation initiale.
- C4 : ajouter un scénario déterministe retardant la livraison à l'admission après publication de l'état d'inspection. Avant livraison : attente et absence d'effets ; après livraison : résultat attendu, file et génération active préservées, aucune commande rejouée. Utiliser les supports existants, sans hook produit.

## Lot (six tâches)

1. Préflight : état Git, responsabilités, espace libre supérieur à 30 Gio, racines canoniques sans jonctions ; plan commité avant délégation.
2. Implémentation par `rust_implementer`, mode IMPLEMENT, de C1–C4 ; lui seul écrit les tests.
3. Vérifications ciblées sous `ci/guard-level1.ps1` après SelfTest : format, tests `root::model_catalog::tests` du binaire desktop ; compilation de test et Clippy desktop avec lints applicables. Aucun Cargo direct.
4. Revue par `rust_review_coordinator` et son reviewer indépendant ; corrections éventuelles par l'implémenteur, deux cycles maximum.
5. N1 sur le candidat final sous garde : `test --no-fail-fast` des membres par défaut avec `-WithoutCargoTargetDir`, puis sélections CPU, Parakeet CPU et worker-common ; contrôles PE préalables des harnais workers et auto-tests PE permanents. `--offline --locked`, aucune feature native, aucun test GPU exécuté. Modèle personnel existant sélectionné et vérifié par la garde si nécessaire aux tests N1 ; aucun téléchargement implicite. Arrêt et arbitrage selon AGENTS.md en cas d'échec.
6. Remise : commits locaux, bilan d'une page `docs/cadrage/bilan-correction-test-catalogue.md` et archive des petites sorties `docs/cadrage/raw/correction-test-catalogue.zip`, nombre/contenu vérifiés. Pas de push ni fusion implicite.

Racine de campagne : `target/parakeet-tests/fix-catalog-test/` dans le worktree. Caches persistants du worktree : `target/cargo-home-shared/` et `target/parakeet-tests/cargo-shared/`, créés/vérifiés avant utilisation et conservés selon AGENTS.md ; initialisation du cache Cargo par copie du registre local existant, sans jonction. Temporaires et LOCALAPPDATA isolés par la garde. Aucun nettoyage historique ni suppression de copies/WAV non approuvés. La session principale écrit et commite les documents ; les spécialistes lui remettent leurs contributions.

Limites : le diagnostic établit un entrelacement possible et retrouve le symptôme dans les journaux, pas l'ordre historique exact. La validation Windows interactive et la qualification native ne sont pas requises pour ce changement de tests et ne seront pas revendiquées.
