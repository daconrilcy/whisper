Avancée : Clippy frais exécuté sur six crates et priorités proposées.
Blocage : deux workers Turbo restent non mesurés, les builds natifs étant suspendus.
Décision : cadrer les alertes de production observées ; différer les alertes structurelles déjà référencées.

# Whisper — cadrage des corrections prioritaires Clippy

**Nature :** cadrage fondé sur le relevé Clippy du 12 novembre 2026.
**État :** cadrage transmis avec réserve de couverture ; le plan associé ne vaut pas autorisation d’implémentation.

## BESOIN

Établir un ordre de correction pour les alertes Clippy mesurées, puis fournir un plan d’implémentation borné qui préserve les comportements du produit. Le premier objectif est de traiter les alertes de production pouvant affecter le coût mémoire du polling ou la lisibilité des parcours d’arrêt et de reprise.

**Observations :** Clippy a terminé avec le code 0, sous la garde N1, hors ligne, avec `--all-targets` sur `whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common` et `whisper-worker-parakeet-cpu`. La sortie JSON compte 176 diagnostics distincts. Le cliquet structurel a trouvé 157 clés/sites contre 168 dans la baseline, sans nouvelle alerte structurelle.

**Couverture manquante :** `whisper-worker-cpu` et `whisper-worker-gpu` n’ont pas été compilés par cette mesure. Les artefacts `whisper-rs` n’étaient pas présents dans la cible utilisée et les builds natifs sont suspendus par les consignes du dépôt. Leurs alertes restent inconnues.

**Priorités proposées :**

1. **P1 — contrôles de polling volumineux.** Deux `large_enum_variant` et un `result_large_err` pointent vers `PollCollectionControl`, `PriorityControl` et le résultat de `prepare_poll_event`. Clippy estime la variante la plus grande à 592 octets. Examiner ensemble la représentation interne et son effet sur le chemin fréquent de polling.
2. **P2 — complexité de parcours en production.** `parakeet_live::Lifecycle::stop` atteint 26/25 en complexité cognitive et 146/100 lignes ; `run_controlled_unified_import` atteint 32/25. Extraire uniquement des étapes cohérentes si cela clarifie les transitions et les erreurs.
3. **P2 — reprise d’import.** `resume_interrupted` dans le runtime des workers atteint 357/100 lignes. Étudier un découpage par étapes de reprise, en préservant l’ordre des vérifications et écritures.
4. **P3 — alertes restantes et tests.** Trier les diagnostics non structurels restants à partir du JSON frais. Les alertes de tests ne justifient pas de réduire les assertions ni de refactoriser leurs scénarios pour satisfaire un seuil structurel.

Le rang est une proposition de séquencement technique. Les valeurs Clippy ne démontrent pas à elles seules un défaut fonctionnel. Pour P1, la mesure de taille justifie une vérification du coût ; elle ne prouve pas un impact utilisateur.

**Critères d’acceptation :**

- **CA-01 — traçabilité :** chaque site retenu est associé à son lint, crate, chemin, span JSON et message observé.
- **CA-02 — couverture :** le bilan identifie les six crates mesurées et les deux workers non mesurés sans présenter la campagne comme un Clippy complet des huit crates.
- **CA-03 — priorité :** l’ordre distingue risque potentiel, effort et inconnues. Aucun rang n’est présenté comme une gravité fonctionnelle démontrée sans preuve.
- **CA-04 — comportement :** chaque correction planifiée indique les sorties, transitions, identités et effets à conserver, avec des vérifications existantes appropriées.
- **CA-05 — périmètre :** le plan contient au plus dix tâches, exclut la réduction générale des alertes structurelles et ne supprime ni ne masque de lint pour obtenir un résultat vert.
- **CA-06 — validation :** le plan distingue contrôles réalisables sous N1, travaux suspendus et validation utilisateur éventuelle.

**Hors périmètre :** changements Rust ou CI dans ce cadrage, modification de la baseline, refactorisation globale des fonctions longues, builds natifs suspendus, exécution de tests GPU de niveau 2, recette Windows, commit, push et nettoyage.

## ACCORD PRÉCIS POUR CETTE SESSION

**Objectif autorisé :** rédiger le cadrage et son plan d’implémentation dans `docs/cadrage/`, conformément à la demande de l’utilisateur et à `AGENTS.md`.

**Base :** dépôt `C:\dev\whisper`, `main` à `150b1862`. La campagne Clippy a été lancée dans `C:\wt\reduc-adapters-tests`, tree `1558f55fcfd81c0584efa9d2d247003e4db38bdb`, identique à l’arbre de `main` pendant l’inspection. Journal JSON : `C:\wt\reduc-adapters-tests\target\parakeet-tests\clippy-frais-20261112\scratch\20261112T124840439-db9b41b6b97f48088c110ab9cdad2ffc\cargo-command.stdout.log`.

**Accord non donné :** aucune implémentation, commande Cargo supplémentaire, construction native, modification de garde, commit, push ou nettoyage n’est autorisé par la demande de cadrage et de plan.

## À FAIRE

1. **[CA-01, CA-02]** Rattacher les alertes prioritaires au journal JSON et aux six crates compilées.
2. **[CA-03]** Classer les alertes de production selon le risque potentiel et la portée de correction.
3. **[CA-04]** Lire les contrats voisins pour poser les invariants de polling, arrêt et reprise.
4. **[CA-05]** Construire un plan de dix tâches maximum et séparer les priorités mesurées du résiduel structurel.
5. **[CA-02, CA-06]** Décrire les limites de couverture et les gates de validation, sans déclencher de campagne.
6. **[CA-01 à CA-06]** Relire indépendamment le cadrage et le plan avant livraison.

## DÉCISIONS À VALIDER

- **D-01 — workers Turbo :** recommandation de conserver CPU et GPU hors de ce plan tant que leur compilation nécessite la reprise des builds natifs suspendus. Leur état reste « non mesuré ».
- **D-02 — alertes structurelles :** recommandation de ne pas réduire les sites déjà référencés dans le cadre de ces corrections prioritaires. Le contrôle observé indique zéro nouveau site structurel. Une évolution séparée pourra traiter leur dette si un objectif produit le justifie.

Ces décisions délimitent le plan courant ; elles n’empêchent pas l’utilisateur de demander ultérieurement un cadrage distinct.

## RAPPORT

La campagne fraîche a réussi sur six crates. Elle fournit une base pour prioriser trois groupes de production, mais ne permet pas de conclure sur les deux workers Turbo. Le plan associé conserve les alertes structurelles déjà présentes et définit les preuves nécessaires pour chaque correction. Toute implémentation nécessitera ensuite le lot, les chemins, les prérequis et l’accord précis requis par `AGENTS.md`.
