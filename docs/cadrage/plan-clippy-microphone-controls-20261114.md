# Plan P21 — Décomposer le scénario UI de test microphone

Avancée : relevé Clippy frais à 116 alertes; le candidat cible une fonction de test exécutée par N1.
Blocage : aucun; l’ordre des frames, commandes et accusés de réception doit rester identique.
Décision : isoler quatre phases du scénario dans des helpers privés de test.

## Résumé et source

Le relevé frais du 14 novembre 2026 a passé le cliquet Clippy à 116 sites sur 116 baseline. Dans `crates/whisper-desktop/src/ui/tests/microphone_controls.rs`, `microphone_test_button_stops_after_neutral_gain_ack_and_reopens` déclenche `too_many_lines` (210/100) et `cognitive_complexity` (37/25). Le test couvre le bouton de test micro désactivé pendant une capture, le démarrage après arrêt, l’ACK du gain neutre, les mesures/spectres, l’arrêt et la réouverture, puis le routage de l’arrêt d’une capture directe.

## Périmètre

- Code : `crates/whisper-desktop/src/ui/tests/microphone_controls.rs` seulement.
- Le plan et son commit local sont rédigés par la session principale. L’implémentation est confiée à `rust_implementer`; la revue à `rust_review_coordinator`.
- Ne changer aucune politique de capture, présentation UI, donnée de test, valeur attendue, appel de frame, identité, message, ordre de commande ou oracle.
- Ne pas ajouter `allow`/`expect`, retirer un assert, modifier une autre fixture ou toucher les changements préexistants des P1–P20.
- P21 est la première tâche de la tranche nouvellement autorisée P21–P30.

## Tâche P21-T1 — Extraire les phases du scénario

Créer des helpers privés dans le même fichier et déplacer les blocs existants sans modifier leur contenu ni leur ordre :

1. préparer/cliquer le bouton de test, vérifier le blocage pendant la capture et le démarrage à gain 0 après sa fin;
2. vérifier les frames d’ACK, les compteurs de gain, la mesure, le badge et la présentation du spectre;
3. arrêter puis rouvrir le test micro, en gardant la transcription terminée et en récupérant `(live_job, live_generation)`;
4. basculer vers la capture directe, cliquer Stop et vérifier Draining, phase Stopped, dernier niveau et retrait du badge périmé.

Le test principal orchestre ces phases avec le même `application`, `app`, `context` et `baseline_shapes`. Les helpers empruntent les mêmes objets; ils ne créent pas d’application ou de contexte additionnel. Conserver exactement le nombre, l’ordre et les arguments des appels `run_ui_frame` et `run_ui_frame_at_size`, ainsi que les mêmes commandes et assertions.

### Critères d’acceptation

- La fonction initiale passe sous 100 lignes et sous complexité cognitive 25; les deux alertes disparaissent sans nouvel avertissement structurel.
- Le scénario conserve les mêmes séquences de frames, clics et commandes, les mêmes messages, résultats persistés, identités et assertions.
- Le test ciblé s’exécute et passe dans N1; Clippy frais des six crates passe le cliquet à 114 sites ou moins contre 116 avant P21.
- `fmt --check`, SelfTest de la garde, N1 complet et `git diff --check` réussissent via la garde et selon `AGENTS.md`.
- Seul le fichier Rust déclaré est modifié par l’implémenteur; aucun commit de code.

## Gates et arrêt

- Worktree dédié : `C:/wt/cs`; racine gardée : `target/parakeet-tests/clippy-corrections-suivantes-20261113`.
- La cible fraîche P21 est `p21-fresh-20261114`; les commandes suivantes réutilisent uniquement cette cible existante via la garde.
- Clippy exact à six membres et quatre lints structurels, format gardé, N1 complet avec `-WithoutCargoTargetDir` et modèle Parakeet personnel validé par la garde; pas de `--ignored`, `native-tests`, téléchargement ou niveau GPU 2.
- Arrêter au premier échec de préflight, format, compilation, Clippy, N1 ou contrôle PE. Préserver les logs; toute reprise suit l’autorisation d’arbitrage applicable.
- Revue indépendante requise; au plus deux cycles revue/correction. La tranche s’arrête après dix tâches P21–P30 et requiert alors un nouvel arbitrage avant P31.

## Hors périmètre et retour arrière

Aucun changement de production, manifeste, dépendance, baseline, garde, test additionnel ou scénario UI. Si une extraction impose de changer un état, un ordre de frame, une séquence de commande ou un oracle, annuler seulement cette extraction dans le périmètre attribué et rapporter le constat au coordinateur.

## Vérifications

Toutes les commandes Cargo passent par `ci/guard-level1.ps1` depuis ce worktree, avec le `LotRoot` ci-dessus et le cache partagé déclaré. Exécuter SelfTest; `fmt --check` sur cible fraîche; Clippy six membres (`whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common`, `whisper-worker-parakeet-cpu`) avec `--all-targets --message-format=json` et les quatre lints structurels; puis N1 complet selon la procédure sans variable `CARGO_TARGET_DIR`, modèle personnel à empreinte épinglée et contrôles de harnais worker. Ne pas relancer la compilation si une porte échoue.
