Avancée : le cadrage retient 37 diagnostics locaux aux tests et leurs supports.
Blocage : toute implémentation reste conditionnée à une voie N1/Clippy admise et à la coordination avec le plan TOP5 existant.
Décision : aucun code, changement de baseline, modification de garde ou test natif n’est autorisé par ce cadrage.

# Cadrage — prochaines corrections Clippy

**ID :** D-WHISPER-CLIPPY-SUIVANTES-20261113
**Base :** `main@9f1b09c8b1977aa96afeccc080e7048c5fc32961`.
**État :** cadrage préparé pour revue et décision utilisateur.
**Preuve de départ :** Clippy gardé frais sur cette base, six crates, sortie 0. Journal : `C:/wt/clippy-frais-20261113/target/parakeet-tests/refacto-reprise-import/scratch/20261113T132530667-2da56dff435345a8b2c55a057d858d89/cargo-command.stdout.log`.

## Besoin et périmètre

Préserver la lisibilité et les oracles des tests du démarrage micro, de l’import, de la reprise et de la publication en retirant des avertissements Clippy sans modifier les résultats, transitions, données audio ou sorties observées. Le commit de base contient déjà le découpage des grands scénarios de test ; le présent cadrage traite leur état Clippy après ce changement.

La mesure a produit 237 émissions d’avertissement. Après déduplication par lint, crate, fichier et ligne primaire, elle compte 178 sites : 141 sites structurels et 37 autres diagnostics locaux aux tests. Le cliquet structurel passe à 141 sites contre 168 dans la baseline, sans nouveau site. Les deux workers Turbo ne font pas partie de la mesure. Aucun chiffre ne constitue une preuve de défaut fonctionnel.

**Capacités produit concernées :** transcription micro avec CPU/GPU/Auto, démarrage contrôlé du worker, événements et durabilité audio ; leurs contrats existants demeurent l’oracle. **Hors périmètre :** évolution fonctionnelle, nouveau modèle, changement IPC, format de sortie, UI, worker Turbo GPU, campagne native et réduction générale de la dette Clippy.

## Inventaire précis proposé

| Travail | Sites observés au commit de base | Fichiers concernés |
|---|---|---|
| Variables déclarées mais inutilisées | 22 `unused_variables` | `src/worker_ipc/tests/native_execution.rs` (4), `src/worker_ipc/tests/unified_capture_helpers.rs` (7), `tests/reprocessing_cleanup.rs` (6), `tests/support/reprocessing_cleanup_helpers.rs` (5) |
| Derniers réemprunts d’option | 9 `needless_option_as_deref` (`Option<&mut NativeImportTiming>`) | `tests/parakeet_unified_native.rs` (2), `tests/support/native_import_workflow.rs` (7) |
| Diagnostics isolés | 1 `collapsible_if`, 1 `needless_borrow`, 1 `unused_mut`, 1 `unreachable_code` | `src/worker_ipc/tests/native_execution.rs` (3) et `src/worker_ipc/tests/unified_capture_helpers.rs` (1) |
| Champs signalés inutilisés | 2 `dead_code` | `src/worker_ipc/tests/native_execution_helpers.rs` (`NativeGpuResumeRun.app`), `tests/support/reprocessing_cleanup_helpers.rs` (`ReplacementScenario.root`) |

Le `unreachable_code` a son span primaire dans `core/panic.rs`, mais sa chaîne d’expansion aboutit au `unreachable!` de `native_execution.rs:2298`, après une boucle divergente. Il s’agit donc bien d’un avertissement du dépôt. Le retirer doit laisser les retours, erreurs, échéances et assertions de la boucle inchangés.

Les deux champs `dead_code` ont des rôles différents : `NativeGpuResumeRun.app` conserve une application/runtime et son transport en vie pendant les vérifications de reprise ; `ReplacementScenario.root` n’est jamais lu après construction. Le premier ne doit pas être retiré si cela avance la destruction des ressources ; un nom indiquant une possession de durée de vie peut être évalué. Le second peut être supprimé après vérification de tous ses usages.

### REQ et critères d’acceptation

- **REQ-01 — Avertissements de test :** examiner les 37 sites selon leurs configurations, effets et durée de vie ; corriger uniquement lorsque l’équivalence est établie. **UC-01 :** comparer le candidat aux mêmes cibles Clippy et aux oracles actuels. **AC-01 :** aucun oracle, nom de test, `cfg`, `ignore`, fixture, message utile, identité, ordre d’effet, point de mesure ou durée de vie nécessaire n’est retiré ou affaibli ; les diagnostics ciblés sont supprimés ou individuellement différés avec motif vérifiable.
- **REQ-02 — Mesure fidèle :** **AC-02 :** cliquet Clippy PASS, baseline inchangée, aucun nouveau site structurel ; rapport des tests effectivement exécutés. Clippy compilant un test ne prouve pas l’exécution des scénarios `ignore`/`native-tests`.

### Domaine, architecture et décisions

Les changements de test n’introduisent ni état métier ni transition produit. Ils conservent les contrats suivants : réemprunt des valeurs `Some` sans déplacer les mesures ; valeur `None` sans trace conservée ; ordonnancement/reprise et sorties A/B/C inchangés ; objets détenus jusqu’à la même fin de scénario.

**D-01 — Portée proposée :** traiter les 37 diagnostics dans les tests et leurs supports, sans refactor de production. Le site `start_live` est exclu : bien que son chemin existe dans le cœur et l’adaptateur et soit appelé par des tests, le contrôleur Windows construit `StartUnifiedLiveConfigured` ou `StartDirectLiveConfigured` (`crates/whisper-desktop/src/ui/model/controller/actions.rs:277-283`) ; aucun appelant produit de `StartLiveConfigured` n’a été établi sur cette base. Le site VAD à 102 lignes (`advance_parakeet_vad_decode`) est également différé : l’admission courante rejette ce profil avant l’installation du décodeur et les tests proches ne l’exercent pas. Les autres sites `too_many_lines` restent hors lot.

**D-02 — Baseline :** conserver `ci/clippy-structure-baseline.tsv` inchangé ; aucune permission d’`allow`, de hausse de seuil ou de masquage.

**D-03 — Coordination :** le design et le plan TOP5 existants décrivent une base antérieure et portent encore des états de planification, tandis que le commit `9f1b09c8` contient déjà le découpage de test. Le relevé frais de cette base devient l’entrée du nouveau plan ; les anciens documents ne sont ni réécrits ni interprétés comme une autorisation. Vérifier l’état de tout worktree candidat et séquencer les changements des fichiers communs, notamment `native_execution.rs` et `reprocessing_cleanup.rs`.

### Risques et limites

- **RISK-01 — Oracle ou durée de vie perdus :** un alias apparemment inutilisé peut porter un résultat ou garder un worker/runtime en vie. Examiner les usages, `Drop`, `Some`/`None` et la configuration avant retrait.
- **RISK-02 — Couverture surestimée :** les appelants natifs A/B/C, la reprise GPU, les scénarios d’import chronométrés et les autres corps portant `ignore`/`native-tests` ne sont pas prouvés par une compilation Clippy ou par les tests N1 usuels ; ne pas les annoncer exécutés.
- **RISK-03 — Gate de validation indisponible :** la politique actuelle de garde n’enregistre pas encore ce nouveau lot. L’exécution/implémentation attend une voie explicitement admise couvrant Clippy, N1, sélections worker et racines autorisées.

**Questions ouvertes :** Q-01, propriétaire utilisateur avant le premier changement Rust : autoriser une voie gardée pour le `LotRoot` exact de ce lot, sans élargissement implicite de la garde ? En attente, l’implémentation et Cargo restent interdits. Les détails réversibles de nommage des bindings de durée de vie et d’emplacement des petits helpers sont à la charge de l’implémenteur, puis du reviewer, sous réserve des invariants ci-dessus.

## Readiness

Le périmètre et les preuves Clippy sont attribués à `main@9f1b09c8`. Les 37 diagnostics sont localisés dans sept fichiers de tests/support. L’option de refactor de production a été écartée après vérification du flux UI ; le VAD historique reste différé. Q-01 est un gate d’implémentation, pas une autorisation. Le cadrage ne lance ni Cargo, ni test, ni campagne native et n’autorise aucune modification Rust, de garde ou de baseline.

## Carte de contenu

Brief/portée : besoin ci-dessus ; exigences et cas : REQ/UC/AC ; domaine : contrats existants, sans nouvel état ; architecture et preuve : sections précédentes ; acceptation/risques/décisions : mêmes sections ; readiness : présente. ADR, railguard, SPIKE et nouvelle machine d’états : non pertinents, car aucune technologie, dépendance, frontière produit ou capacité nouvelle n’est introduite.
