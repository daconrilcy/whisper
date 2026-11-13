Avancée : plan borné aux deux fichiers de production portant le résultat interne.
Blocage : le plan n’autorise ni code ni campagne Cargo.
Décision : préflight et profil de garde conditionnent tout futur lot.

# Plan de refactorisation — résultat nommé de transcription Parakeet

**Source :** [cadrage du résultat de transcription](cadrage-resultat-transcription-parakeet.md).
**État :** plan proposé, en attente d’acceptation et d’un accord d’implémentation distinct.
**Base de mesure :** commit `6cfc4106f6c89569767f069d67859d1b0410031e`, campagne N1 du 13 novembre 2026 ; détails et limites dans le cadrage.

## Objectif et budget

Fermer `clippy::type_complexity` sur le retour de `transcribe_one` en remplaçant son tuple de succès par un type nommé et en adaptant son unique appelant.

- Fichiers de production visés : `crates/whisper-worker-parakeet-cpu/src/transcription.rs` et `src/session.rs`.
- Aucun changement de manifeste, lockfile, garde, protocole, feature, algorithme, sélection moteur ou FFI.
- Aucun worker Turbo CPU/GPU, build natif, test GPU niveau 2, modèle ou matériel réel.
- Tests dans un fichier séparé si un scénario manquant le justifie ; ne pas réduire les assertions existantes.
- Pas de lint `allow`, hausse de baseline ou refactorisation de fonctions longues pour fermer cette alerte.

## Invariants

Le résultat nommé conserve exactement les cinq données actuelles : `duration_samples`, `text`, `words`, SHA-256 de la source et `native_inference_ms`. Il est accessible depuis le module frère `session` par la visibilité minimale nécessaire (par exemple `pub(super)` pour le type et ses champs, à confirmer au préflight), sans API publique. La validation du chemin, le hash et sa comparaison, le décodage, l’appel natif et le callback gardent leur ordre. L’erreur reste classée avec les mêmes codes et messages. `session.rs` construit la même `Event::Result`; aucun octet du protocole sérialisé ne change.

## Tâches futures

| Tâche | Travail | Critères de sortie |
|---|---|---|
| T0 — Préflight | Après accord d’implémentation, confirmer HEAD, état Git, espaces, processus pertinents, instructions de crate et sélection exacte de garde. Refaire la baseline Clippy sur le même périmètre six crates/all-targets. Exécuter SelfTest avant Cargo. Arrêter avant Cargo si le profil versionné ne couvre pas la commande ou si un prérequis manque ; ne pas modifier la garde. | CA-06, CA-07 ; alertes et état de départ attribués au HEAD courant. |
| T1 — Modèle de résultat | Confirmer l’arborescence puis définir dans `transcription.rs` une structure nommée à cinq champs sémantiques. Accorder au type et aux champs la seule visibilité nécessaire à l’accès depuis `session` (par exemple `pub(super)`, à confirmer), sans API publique ; conserver le type d’erreur existant. | CA-01, CA-03, CA-05 ; aucune donnée ni erreur modifiée, aucune API publique ajoutée. |
| T2 — Producteur et appelant | Construire le résultat depuis les mêmes valeurs puis adapter `session.rs` pour accéder aux champs nommés en émettant le même événement. | CA-01 à CA-05 ; callback et ordre des effets inchangés. |
| T3 — Vérifications ciblées | Réutiliser les tests existants sur classification des erreurs et contrat worker ; ajouter seulement une assertion nécessaire pour démontrer la correspondance des champs ou l’événement. | CA-02 à CA-04 ; aucune assertion affaiblie. |
| T4 — Validation gardée | Via `ci/guard-level1.ps1` et une sélection versionnée autorisée : exécuter la suite de référence N1 prévue par `AGENTS.md` (dont `cargo test --no-fail-fast` dans son shell dédié, puis les sélections worker prescrites), le format autorisé, puis Clippy JSON avec la même sélection des six crates, `--all-targets` et les quatre lints structurels que la baseline. Faire `git diff --check` séparément. Si le profil ou la racine du lot ne sont pas autorisés, arrêter avant Cargo et demander un arbitrage ; ne pas modifier la garde. | CA-06, CA-07 ; alerte ciblée fermée, résultats et limites documentés. |
| T5 — Revue et clôture | Confier le candidat au `rust_review_coordinator` et à son reviewer indépendant ; au plus deux cycles de correction. Après clôture, rapporter les constats ouverts, les validations réellement faites et le périmètre non couvert ; committer localement le lot fermé selon `AGENTS.md`. | CA-01 à CA-07 ; revue CLEAN ou livraison avec réserves, sans push. |

## Gates et arrêt

Ce plan ne constitue pas un accord d’implémentation. Le futur lot doit être explicitement accepté et autorisé dans sa session, confié à `rust_implementer` en mode IMPLEMENT, puis suivi du routage de revue Rust indiqué dans `AGENTS.md`. Toutes les commandes Cargo restent via la garde N1, avec ses préflight, isolation, limites de sélection et espace libre. Aucun contournement ni changement de garde n’est inclus.

Arrêter si le type nommé impose une modification IPC, un changement d’erreur ou de callback, une permutation ou transformation de valeur, un élargissement de fichiers, une sélection Cargo non autorisée, une alerte structurelle nouvelle ou un échec de vérification. Toute extension demande un arbitrage écrit avant poursuite.

## Hors périmètre

Les autres sites des 152 alertes structurelles, les deux workers Turbo non mesurés, `resume_interrupted`, le GPU niveau 2, les builds natifs suspendus, la validation de fenêtre Windows, l’édition de garde, le push et toute campagne additionnelle.
