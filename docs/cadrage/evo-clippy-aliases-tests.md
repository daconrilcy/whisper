Avancée : sept alertes `clippy::type_complexity` des supports de test `whisper-adapters` ont été confirmées par Clippy gardé ciblé.
Blocage : la disparition des alertes après refactor reste à vérifier sur un candidat.
Décision : cadrer des alias de types locaux, sans autoriser leur implémentation.

# Whisper — Cadrage des alias de types dans les supports de test

**Identifiant :** D-WHISPER-CLIPPY-ALIASES-01
**Nature :** évolution de maintenabilité, proposée pour planification.
**État :** DESIGN READY après revue indépendante CLEAN ; implémentation non autorisée par ce document.

## BESOIN

Rendre plus lisibles plusieurs types imbriqués dans les supports de test de `whisper-adapters` et retirer les alertes `clippy::type_complexity` correspondantes, sans changer les scénarios ni la logique testée. Ces supports caractérisent notamment les canaux de capture, les gates de publication et les workers contrôlés. Leur lisibilité aide à maintenir les vérifications du comportement de capture et de publication de l’application.

**Constat vérifié.** Le 11 novembre 2026, Clippy a été exécuté via `ci/guard-level1.ps1` sur `whisper-adapters`, toutes cibles, avec les quatre lints structurels. Sortie : code 0 ; `CLIPPY_STRUCTURE_RATCHET PASS keys=200 sites=201 baseline_keys=247 baseline_sites=248`. La sortie JSONL contient sept diagnostics `type_complexity` dans les trois fichiers ci-dessous. Le worktree de vérification (`3969179a`) ne différait pas du HEAD `main` (`adc6c3fd`) sur les sources de crates. Clippy a compilé les cibles demandées ; aucun test n’a été exécuté.

| Site observé | Propriétaire | Proposition |
|---|---|---|
| `worker_ipc/service/mod.rs:261`, tuple de `AsyncImportIo::for_test` | Canaux de service réservés aux tests | Alias `#[cfg(test)] pub(super)` conservant les sept éléments et leur ordre. |
| `worker_ipc/tests/unified_support.rs:185`, file `controlled_capture_blocks` | Fixture `RuntimeImportPort` | Alias local du type de file ; garder `Option` et le type sous-jacent. |
| `worker_ipc/tests/unified_support.rs:403,414,428`, retours de `held_worker`, `held_final_a_worker` et `held_worker_with_mode` | Worker de test retenu | Un alias privé commun pour les trois tuples identiques. |
| `worker_ipc/tests/microphone_and_finalization.rs:903`, `samples_rx` | Fixture `SampleDrivenWavCapture` | Alias du message échantillons/acquittement, conservé dans le `Receiver`. |
| `worker_ipc/tests/microphone_and_finalization.rs:974`, `gates` | Fixture `RuntimeAutomaticPublisher` | Réutiliser `PublicationGateRegistry`, déjà défini dans `worker_ipc.rs:119`. |

La sélection observée a réussi. La tentative workspace précédente a échoué dans la configuration CUDA de `whisper-rs-sys` ; elle n’est pas nécessaire pour évaluer ce périmètre et ne doit pas être répétée pour ce cadrage. Les builds natifs restent soumis à leur suspension dans `AGENTS.md`.

### Exigences et critères d’acceptation

- **REQ-01 — Types nommés :** les sept sites confirmés sont exprimés au moyen d’alias locaux adaptés ; l’alias `PublicationGateRegistry` existant est réutilisé plutôt que dupliqué.
- **REQ-02 — Équivalence des tests :** seules les déclarations d’alias et leurs références changent. Les types sous-jacents, l’ordre des tuples, les canaux, files, verrous, corps, appels, assertions, conditions de compilation et états `ignore` restent identiques.
- **UC-01 — Revue des types :** le mainteneur introduit les alias dans les supports désignés ; le relecteur compare le diff aux types sources et aux usages existants.
- **UC-02 — Vérification Clippy :** le validateur relance le Clippy gardé ciblé sur la même crate, toutes cibles et les mêmes lints.
- **AC-01 — Sites retirés :** les sept diagnostics identifiés par champ/fonction et contexte ne sont plus émis ; aucun diagnostic `type_complexity` n’apparaît sur les alias ni sur ces supports.
- **AC-02 — Clôture Clippy :** la commande ciblée réussit et le cliquet passe sans alerte structurelle nouvelle ; la baseline n’est pas modifiée. La comparaison porte sur les sites, pas seulement sur le total agrégé.
- **AC-03 — Préservation :** revue du diff et N1 requis par `AGENTS.md` confirment que les scénarios et leurs statuts restent inchangés. Clippy compile les cibles mais ne prouve pas l’exécution des tests ignorés.

## ARCHITECTURE ET RISQUES

Les cinq observations se résolvent par quatre alias proposés et la réutilisation d’un alias existant. Les fichiers `unified_support.rs` et `microphone_and_finalization.rs` sont inclus dans le module de tests `worker_ipc::tests`; leurs alias restent privés à ce support. `AsyncImportIo::for_test` est déjà `#[cfg(test)] pub(super)` ; son alias doit conserver ces attributs de compilation et de visibilité. Aucun type ou helper nouveau n’est exposé à l’API produit.

**Risque R-01 — Tuple permuté :** les tuples contiennent des positions aux types proches ; un changement d’ordre peut modifier les tests même si le code compile. Préserver explicitement l’ordre et les destructurations.

**Risque R-02 — Alias déplacé ou dupliqué :** les fichiers inclus partagent le module de tests. Placer les alias au niveau de portée approprié, réutiliser `PublicationGateRegistry` et arrêter si Clippy reporte le diagnostic sur l’alias lui-même au lieu d’élargir le refactor.

**Risque R-03 — Validation élargie par erreur :** le Clippy workspace a rencontré une configuration de build CUDA. Le lot proposé reste limité à `whisper-adapters` ; une commande qui déclenche une campagne native suspendue doit s’arrêter.

## PÉRIMÈTRE

**Inclus :** les sept sites et les trois fichiers du tableau ; quatre alias privés/protégés au plus et un réemploi ; Clippy ciblé, formatage, revue et validations requises pour un lot Rust.

**Exclus :** `transcription.rs` du worker Parakeet CPU et tout autre avertissement ; refonte des tuples en structures nommées ; changement de logique, de tests/scénarios, de visibilité au-delà du helper de test, de seuil ou de lint ; `allow`, baseline, garde, manifests, lockfile, dépendances, tests nouveaux, builds natifs, GPU exécuté, validation Windows, commit ou push du code produit.

## DÉCISIONS ET AUTORISATIONS

- **D-01 — Alias transparents :** proposition technique proportionnée aux diagnostics ; préserver les représentations et signatures effectives. Les noms précis restent un détail réversible de l’implémentation, à valider par le relecteur.
- **D-02 — Implémentation :** ouverte. La demande présente autorise le cadrage et le plan ; un accord précis sur le lot et ses chemins sera nécessaire avant toute modification Rust.
- **D-03 — Baseline :** conserver sans changement. Les alertes retirées sont tolérées par le cliquet ; aucune mise à jour de référence n’est nécessaire.

## RAPPORT

Le cadrage retient un micro-lot de lisibilité exclusivement dans des supports de test. Sept alertes sont confirmées sur la base source comparable ; les quatre alias proposés et le réemploi d’un alias existant ont une portée limitée. L’efficacité des alias, la revue du diff futur et la N1 restent à démontrer après une autorisation d’implémentation distincte.

**Revue indépendante : CLEAN** sur le contenu du commit `2f31c617acb1689268afebd7f5656df8044dc4cd` (`docs/cadrage/evo-clippy-aliases-tests.md`). Aucun constat requis ouvert. Avis consultatif pour le plan : rapprocher les diagnostics JSON avant/après par lint, chemin et champ/fonction, y compris les alias, car le seul PASS du cliquet ne prouve pas le retrait individuel des sept sites.
