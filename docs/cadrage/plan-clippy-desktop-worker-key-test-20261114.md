# Plan P37 — Réduire le test de qualification de clé worker

Avancée : après P36, Clippy signale le test à 117 lignes et le ratchet compte 95 sites.
Blocage : aucun ; garder les trois états de qualification et leurs assertions intactes.
Décision : extraire les fabriques de requête/commande et la projection de sélection.

## Résumé et preuve

Le Clippy gardé après P36 signale `parakeet_worker_key_changes_regray_and_readmit_without_restarting`, dans `crates/whisper-desktop/src/root.rs:1458`. Le test autonome de qualification copie un worker, le valide, le modifie pour vérifier son refus, le restaure et vérifie la réadmission sans rendre indisponible Turbo.

## Périmètre

- Fichier autorisé : `crates/whisper-desktop/src/root.rs`.
- Extraire de ce test les petites fonctions privées qui construisent la requête/commande d’import Parakeet et calculent la sélection du modèle depuis `CatalogSnapshot`.
- Remplacer les closures locales par ces fonctions sans changer les données, appels, ordre des opérations ou assertions.
- Conserver les trois phases : clé valide, clé worker modifiée/refusée sans effet, worker restauré/réadmis.
- Aucun feature, fichier, test, protocole ou comportement produit ne change.

## Tâche P37-T1

Introduire les helpers de scénario ci-dessus puis simplifier uniquement le corps du test afin qu’il passe sous 100 lignes.

**Acceptation :** le test ciblé passe sous 100 lignes; Clippy descend de 95 à 94 sites ou moins sans nouvelle alerte; N1 passe; revue indépendante CLEAN; diff limité au fichier autorisé.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE. Le test exigeant le runtime installé reste ignoré par défaut et est signalé.

**Arrêt :** échec de garde, compilation ou test; changement des états de qualification, refus, effets ou disponibilité Turbo; fichier hors périmètre nécessaire; nouvelle alerte Clippy. Préserver les journaux.

**Retour arrière :** retirer uniquement les helpers et leurs appels si une phase ou assertion diverge.
