# Plan P24 — Découper la caractérisation de reprise d’archive

Avancée : Clippy standard frais passe le ratchet à 111 sites; un test de reprise d’archive dépasse la taille et la complexité visées.
Blocage : aucun; le scénario doit garder ses mêmes fichiers, publications, collisions et assertions.
Décision : extraire des helpers privés de préparation/validation dans le module de test existant.

## Source et périmètre

Le Clippy frais des six crates signale `r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision` dans `crates/whisper-adapters/src/archive/tests/recovery.rs` (plus de 300 lignes, complexité cognitive supérieure au seuil 25). Cette caractérisation vérifie la reprise entre deux exports et la non-écrasement d’une collision identique. Le fichier n’a pas de modification préalable dans le worktree.

Révision 1 (commit suivant) : le périmètre comprend `crates/whisper-adapters/src/archive/tests/recovery.rs`, `crates/whisper-adapters/src/archive/tests.rs` et le nouveau `crates/whisper-adapters/src/archive/tests/recovery_support.rs`. La première tentative tout-en-un aurait augmenté le fichier déjà long de 156 lignes nettes, au-delà du plafond de lot; elle est abandonnée et ne doit pas être gardée comme structure finale. Extraire la préparation des artefacts/publications et les validations dans un support inclus dans le même module de tests (`include!("tests/recovery_support.rs")`), et laisser le scénario dans son fichier d’origine. Préserver chaque valeur, écriture, appel, ordre et assertion; ne pas changer le contrat d’archive, les fixtures, la portée du test, ni les autres tests. Aucun changement de dépendance, API de production, baseline ou règle de garde.

## P24-T1 — Extraire les étapes du scénario

Réduire le corps du test à une séquence lisible qui délègue à des helpers privés nommés selon la préparation et la vérification du scénario. Les helpers sont définis dans `recovery_support.rs`, inclus dans le module de tests existant; ils gardent les types concrets du domaine et ne masquent pas les assertions derrière des contrôles affaiblis. Maintenir `recovery.rs` sous son plafond de taille, sans réduire artificiellement le scénario. Le Clippy au seuil 0 doit montrer les helpers sous 25 points; le Clippy standard projet avec ratchet demeure la gate finale.

## Acceptation

- L’alerte ciblée de longueur et de complexité cognitive disparaît; le Clippy standard reste au ratchet sans nouvelle clé et compte au plus 110 sites.
- Le test conserve les deux publications, l’intention Replace, le scénario de copie/reprise et l’absence d’écrasement; aucune assertion n’est supprimée ou affaiblie.
- SelfTest, fmt-check, mesure seuil 0 via le profil gardé, Clippy standard six crates avec ratchet, N1 complet, sélections workers/PE et `git diff --check` passent.
- Aucun autre chemin source n’est modifié; les changements préexistants sont préservés. Le nouveau support reste limité aux helpers de cette caractérisation.

## Gates et arrêt

Worktree `C:/wt/cs`; `LotRoot` `target/parakeet-tests/clippy-corrections-suivantes-20261113`; cible `p21-fresh-20261114`. Toute commande Cargo passe exclusivement par `ci/guard-level1.ps1`; mesure seule avec `-MeasureCognitiveComplexity`, puis Clippy standard ratcheté. Arrêter et préserver les logs à tout échec de porte. P24 est la quatrième tâche de la tranche P21–P30; demander un nouvel arbitrage après P30.

## Retour arrière

Si une extraction change la préparation des publications, l’ordre des opérations disque ou l’oracle de collision, retirer uniquement l’extraction fautive et signaler l’écart; ne pas simplifier le scénario pour faire passer les lints. La révision 1 résout uniquement le plafond de taille; elle ne modifie ni le comportement ni les critères d’acceptation.

