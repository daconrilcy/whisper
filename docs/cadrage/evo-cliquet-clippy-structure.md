Avancée : le lot Clippy local est clôturé ; le cliquet est cadré comme phase suivante.
Blocage : l’identité stable des alertes et la référence initiale restent à établir avant implémentation.
Décision : conserver les 248 alertes structurelles actuelles et refuser seulement les nouvelles alertes des quatre lints suivis.

# Whisper — Cliquet contre les nouvelles alertes Clippy structurelles

**Nature :** cadrage d’une évolution de la garde et plan d’implémentation associé.
**État :** cadrage établi ; décision d’implémentation distincte requise.

## BESOIN

Après le premier lot de corrections Clippy, empêcher que les évolutions futures augmentent la dette structurelle. Le contrôle doit accepter le stock existant et signaler toute nouvelle alerte parmi `clippy::too_many_lines`, `clippy::cognitive_complexity`, `clippy::too_many_arguments` et `clippy::type_complexity`.

**Faits observés :** le bilan [CLIPPY-LOCAL](../architecture/CLIPPY-LOCAL-BILAN.md) compte 248 diagnostics structurels avant et après le lot, et rapporte zéro nouveau site dans la comparaison du lot. Les alertes hors structure sont passées de 96 à 25. Les changements du premier lot sont sur `main` et le dépôt était propre lors du cadrage.

La garde [guard-level1.ps1](../../ci/guard-level1.ps1) valide les commandes Cargo et capture leurs sorties ; elle accepte déjà Clippy avec JSON. Son mode `Structure` appelle [check-structure.rs](../../ci/check-structure.rs) avec [tests-baseline.txt](../../ci/tests-baseline.txt), mais ces références classifient les tests et ne constituent pas une référence Clippy. Le dépôt contient ces fichiers, contrairement à la description obsolète de l’`AGENTS.md` transmis dans la demande, qui indique qu’ils n’existent pas. L’état versionné du dépôt devra prévaloir et les consignes applicables devront être réconciliées avant tout lot d’implémentation.

**Comportement attendu :** la garde exécute Clippy selon la commande N1 autorisée, compare les seuls diagnostics des quatre lints à une référence versionnée, passe si aucun nouveau site n’est détecté et échoue avec le lint et le site concernés dans le cas contraire. La référence n’est jamais mise à jour automatiquement pour faire passer un échec.

### Critères d’acceptation

- **CA-01 — Stock conservé :** un résultat ne contenant que les alertes de la référence passe sans demander de réduction des 248 diagnostics initiaux.
- **CA-02 — Nouveauté bloquée :** un diagnostic inédit dans l’un des quatre lints fait échouer le contrôle et identifie le lint ainsi que le site détecté.
- **CA-03 — Retrait et déplacement :** la disparition d’une alerte acceptée ne fait pas échouer le contrôle ; déplacer des lignes sans changer le site ne crée pas une alerte nouvelle.
- **CA-04 — Portée :** seuls les quatre lints retenus alimentent le cliquet ; les autres avertissements ne sont ni ajoutés à la référence ni bloqués par ce mécanisme. Une erreur de la commande Clippy continue d’échouer via la garde.
- **CA-05 — Contrôle éprouvé :** les auto-tests couvrent référence inchangée, alerte nouvelle, alerte supprimée, déplacement de lignes, référence invalide et échec Clippy, sans ajouter de surveillance ni relâcher N1.
- **CA-06 — Référence gouvernée :** la référence initiale provient d’un Clippy frais sur le commit autorisé pour le lot. Une alerte nouvelle acceptée ultérieurement ne peut être inscrite que par une modification explicite et justifiée de cette référence, revue avec le changement qui l’introduit.

## PÉRIMÈTRE ET EXCLUSIONS

Inclus : analyse de l’identité des diagnostics, référence permanente versionnée, intégration minimale au chemin Clippy gardé, auto-tests positifs et négatifs, validation N1 et Clippy workspace.

Exclus : résorber les 248 alertes existantes ; réduire les fonctions longues ou leur complexité comme objectif autonome ; modifier les seuils Clippy ; toucher aux 25 alertes hors structure ; changer la logique produit ; modifier Cargo.toml/Cargo.lock ; ajouter une dépendance ; ajouter un contrôle CI distant, une surveillance de fichiers ou une commande Cargo hors liste autorisée. Une fonction de production peut être restructurée lorsqu’une évolution produit la modifie et que le découpage sert cette évolution.

## ACCORD ET AUTORISATION

La demande actuelle autorise la rédaction de ce cadrage et du plan associé dans `docs/cadrage/`. Elle n’autorise pas la modification de `ci/guard-level1.ps1`, de `ci/check-structure.rs`, d’une nouvelle référence, des sources produit ou des configurations. L’implémentation devra recevoir un accord précis dans sa propre session, après résolution des décisions ci-dessous. Aucune commande Cargo, compilation, exécution de SelfTest, branche ou push n’a été demandé pour cette phase documentaire.

## À FAIRE — PLAN DE CADRAGE

1. **[CA-01, CA-02]** Confronter la garde et les sorties JSON Clippy au commit de départ retenu ; relever les champs de diagnostic réellement disponibles et les limites de la référence archivée.
2. **[CA-01, CA-03]** Comparer les clés candidates et arrêter une identité stable résistant au déplacement de lignes sans confondre deux fonctions d’un même fichier.
3. **[CA-01, CA-06]** Définir la génération initiale, le format versionné et la procédure manuelle de revue/ajout ultérieur d’une alerte acceptée.
4. **[CA-04, CA-05]** Définir les cas d’auto-test et l’intégration au chemin Clippy de la garde sans modifier les catégories autorisées ni ajouter de surveillance.
5. **[CA-01 à CA-06]** Confirmer les fichiers et commandes du plan d’implémentation, les arrêts, les vérifications et le retour arrière.

## DÉCISIONS À VALIDER AVANT LE GO D’IMPLÉMENTATION

- **D-01 — Identité stable (bloquante) :** une identité basée seulement sur le chemin et le nom de lint peut confondre plusieurs sites ; une identité incluant les lignes est sensible aux déplacements. Recommandation : établir et tester une clé composite incluant le lint, le package/cible, le chemin relatif normalisé et l’identité de fonction dérivée de données vérifiées, puis documenter les ambiguïtés restantes. Ne pas choisir une clé permissive qui pourrait masquer une alerte nouvelle.
- **D-02 — Référence fraîche (bloquante) :** produire la référence depuis le commit du futur lot, et non réutiliser sans vérification un export historique. Comparer son compte et ses sites aux 248 diagnostics du bilan ; tout écart doit être expliqué avant d’intégrer la référence.
- **D-03 — Consignes de garde (bloquante) :** le texte d’`AGENTS.md` transmis contredit l’état observé du dépôt au sujet de `check-structure`. Avant l’implémentation, vérifier les consignes versionnées et demander l’arbitrage nécessaire si elles interdisent ou conditionnent l’extension de la garde.
- **D-04 — Alerte volontairement acceptée (défaut recommandé) :** pas d’édition automatique de référence. Un changement intentionnel exige une entrée explicite, la justification de l’exception et la revue de l’impact sur CA-02.

## RAPPORT

La phase suivante recommandée est un petit lot de garde : conserver le niveau structurel actuel comme dette acceptée et bloquer son augmentation. La question technique déterminante est de reconnaître un même site au fil des déplacements sans laisser passer une alerte nouvelle. La référence devra être mesurée à nouveau au début du lot autorisé. La modification de la garde reste interdite jusqu’à un accord précis ultérieur.

**Revue de qualification : PASS.** La revue n’a relevé aucun point bloquant ; elle a demandé de préciser l’origine fraîche de la référence et le traitement explicite des nouvelles alertes acceptées, précisions intégrées ici.
