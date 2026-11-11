# Whisper — Cadrage des corrections Clippy

Avancée : Clippy frais exécuté sur le workspace ; priorités de correction définies.
Blocage : aucun pour le cadrage ; les lots d’implémentation restent à autoriser séparément.
Décision : commencer par les imports `normalize_pci`, puis regrouper les corrections locales à faible risque.

## BESOIN

Réduire les alertes émises par Clippy dans le workspace Whisper, par catégories, en préservant le comportement produit. La demande inclut l’import `normalize_pci`. Le cadrage propose un ordre de travail ; il n’autorise pas l’implémentation.

**Contraintes et invariants :** aucun changement volontaire de politique, sortie, protocole, identité de session, publication durable, capture ou interaction utilisateur. Une correction dont l’équivalence n’est pas évidente s’arrête au constat et devient une décision distincte. Pas de lint `allow` pour masquer un diagnostic.

**Critères d’acceptation du cadrage :**

- **CA-01 — Baseline :** rapporter la commande, les options, les émissions et les sites distincts par catégorie, crate et cible, avec la méthode de déduplication et ses limites.
- **CA-02 — `normalize_pci` :** nommer le lint, les deux sites et les émissions ; préserver l’implémentation de normalisation et le contrôle PCI.
- **CA-03 — Préservation :** chaque priorité décrit les invariants et scénarios à préserver ; une correction à risque comportemental reste une décision ouverte.
- **CA-04 — Validation future :** chaque lot propose un critère de fermeture comparant le même périmètre de Clippy et les tests concernés, sans nouveaux diagnostics attribuables au lot.

## BASELINE FRAÎCHE

- **Révision :** `2972f11ebc90a65224f32712d18d695facde0005` (`main`), worktree détaché.
- **Commande gardée :** `cargo clippy --offline --locked --workspace --all-targets --message-format=json -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity`.
- **Garde :** SelfTest N1 réussi, 64 positifs et 99 négatifs. Clippy s’est terminé avec code 0 ; compilation de toutes les cibles demandées, sans exécution des tests.
- **Mesure :** 438 émissions de niveau warning, 344 sites distincts après déduplication par lint, chemin primaire normalisé, intervalle de lignes et message. Les 94 émissions en sus correspondent à des répétitions entre compilations/cibles, et ne sont pas des sites supplémentaires.
- **Sorties brutes archivées :** [stdout JSONL](raw/clippy-frais-20261111/scratch/20261111T142302626-d482a9e6065e4eb98249340a9cf0925a/cargo-command.stdout.log), stderr et sorties `metadata` au même chemin. Quatre fichiers, 1 415 993 octets ; nombre et SHA-256 comparés aux originaux avant leur archivage.

Les nombres « 338 » et « 340 » de l’ancien bilan REFACTO-POLL-PARAKEET concernaient un relevé adapters/core et un candidat spécifique. Ils ne sont pas comparables directement à cette mesure workspace. L’ancien relevé workspace de `88850e8` rapportait 436 émissions et 339 sites distincts, avec une méthode et des options qui ne sont pas identiques à la commande fraîche ci-dessus.

### Répartition par catégorie

| Catégorie / lint | Émissions | Sites distincts | Lecture pour la priorité |
|---|---:|---:|---|
| `clippy::too_many_lines` | 261 | 192 | Plus grand volume ; découper par fonctions/responsabilités après analyse des effets. |
| `clippy::cognitive_complexity` | 42 | 40 | Revoir avec les mêmes hotspots que les longues fonctions. |
| `clippy::too_many_arguments` | 10 | 8 | Examiner d’abord les frontières et contrats touchés ; ne pas changer d’API automatiquement. |
| `clippy::type_complexity` | 9 | 8 | Examiner la lisibilité et les contrats de types avant toute abstraction. |
| **Structure, total** | **322** | **248** | Cumul des quatre lints ci-dessus ; un même site peut porter plusieurs lints. |
| Avertissements Rust : `dead_code`, `unused_imports`, `unused_variables` | 29 | 27 | `dead_code` exige une vérification des configurations/usages ; les imports inutilisés sont des corrections locales candidates. |
| Autres lints Clippy | 87 | 69 | Petits groupes de diagnostics ; examiner le message et le diff proposé par site. |
| **Total** | **438** | **344** | Les avertissements Rust sont séparés de Clippy, mais inclus dans la sortie mesurée. |

Les crates adapters concentrent 265 des 344 sites distincts ; worker GPU en compte 20, core 18, worker Parakeet CPU 15, desktop 14, worker CPU 11 et worker-common 1. Par chemin source, 208 sites sont dans des chemins de tests et 136 dans d’autres chemins ; cette classification ne repère pas tous les tests `cfg(test)` inline. **[Déduction]** L’association des diagnostics `compiler-message` au prochain `compiler-artifact` de même package, cible et kind attribue 343 émissions à une compilation de test (`profile.test`) et 95 à une compilation non-test. C’est une attribution du flux Cargo, pas un champ présent sur le diagnostic ; les 438 émissions restent le total directement observé.

### Cas `normalize_pci`

Clippy frais signale `unused_imports` à deux sites du worker GPU, pour trois émissions au total : [import du harnais](../../crates/whisper-worker-gpu/tests/support/autonomous_harness.rs:47) et [import du module FFI](../../crates/whisper-worker-gpu/src/native_ffi.rs:9). Le second site est émis deux fois dans la sortie. La fonction utilisée pour normaliser et comparer les identifiants se trouve dans `native_ffi/pure.rs` ; elle doit rester intacte, ainsi que le contrôle d’identité PCI.

## ORDRE PROPOSÉ

1. **Imports `normalize_pci` — micro-lot prioritaire.** Retirer uniquement les deux imports signalés, après vérification de leur contexte de compilation. Ne pas modifier la fonction pure ni la comparaison PCI. Contrôle intermédiaire : Clippy de `whisper-worker-gpu` sur toutes ses cibles, sous la garde, avec les mêmes quatre options `-W`. Fermeture du lot : reprendre la commande workspace complète ci-dessus sur le candidat ; les deux sites ne produisent plus d’émission `normalize_pci`, aucun nouveau site n’apparaît dans la comparaison workspace, et les cibles GPU compilent. **[CA-02, CA-03, CA-04]**
2. **Autres corrections locales à équivalence directe.** Trier `unused_imports`, `unused_variables` et les lints Clippy non structurels par site. Commencer par ceux dont le changement est purement syntaxique et dont les appels/valeurs restent identiques. Pour `dead_code`, vérifier d’abord `cfg`, consommateurs indirects et durée de vie des ressources ; ne pas supprimer un élément avant cette analyse.
3. **Structure de production.** Sélectionner les fonctions qui cumulent longueur et complexité, en donnant la priorité à des extractions privées, pures et testables. Traiter les adaptateurs par petites zones, car ils portent la majorité des alertes. Garder séparés les changements qui touchent arguments/types publics, FFI, synchronisation, ordre des effets, protocole, identité de session ou publication.
4. **Structure des tests.** Réduire les fonctions de test trop longues/complexes sans affaiblir les assertions, les scénarios, l’ordre des opérations ni les identités observées. Distinguer les fichiers de test des modules inline `cfg(test)`.

Les rangs 2 à 4 sont des familles de travail, pas une autorisation de refactor global. Avant chaque sous-lot, relever les diagnostics précis et ses chemins, lier chaque correction à un scénario pertinent et arrêter au premier changement de comportement possible.

## PLAN FUTUR — 4 TÂCHES MAXIMUM

| Tâche | Critères | Résultat attendu |
|---|---|---|
| T1 — Imports `normalize_pci` | CA-02, CA-03, CA-04 | Deux sites corrigés sans changement de comparaison PCI ; vérification GPU ciblée et workspace complète sans nouvelle alerte attribuable au lot. |
| T2 — Lints locaux | CA-01, CA-03, CA-04 | Lots par crate et par source, corrections d’équivalence seulement ; `dead_code` traité après vérification de ses usages. |
| T3 — Structure production | CA-01, CA-03, CA-04 | Hotspots retenus par volume, risque et couverture ; extractions bornées et contrats préservés. |
| T4 — Structure tests et bilan | CA-01, CA-03, CA-04 | Scénarios et assertions préservés ; comparaison à commande et révision explicites ; liste des diagnostics résiduels. |

Chaque tâche devra avoir un GO précis, un périmètre de fichiers, une garde N1 valide et ses vérifications utiles. Une mesure avant/après n’est comparable qu’avec la même commande, les mêmes options et la même méthode de déduplication. Le présent cadrage n’accorde pas ces GO.

## DÉCISIONS ET LIMITES

- Recommandation : inclure les avertissements Rust émis par la commande dans le suivi global, mais garder `dead_code` distinct des simples corrections Clippy et exiger son analyse de configuration/usages avant correction.
- Aucun test Rust n’a été exécuté ; Clippy a compilé les cibles de test, ce qui n’établit pas leur réussite à l’exécution.
- La baseline catégorise les alertes ; elle ne choisit pas encore les hotspots exacts des lots 2 à 4. Leur priorité interne dépend de la revue des spans, des scénarios couverts et des effets voisins.
- Aucune implémentation, modification de manifeste, changement de lockfile, branche de travail, installation, campagne native ou validation Windows n’a été effectué.

## RAPPORT

La priorité la plus sûre est le micro-lot des deux imports inutilisés `normalize_pci`. Ensuite, corriger les lints locaux dont l’équivalence est démontrée, puis traiter progressivement les avertissements structurels par hotspot en séparant production et tests. Les catégories sensibles aux contrats, aux effets durables, au FFI ou aux ressources restent à décision explicite si leur correction dépasse un découpage privé sans changement observable.
