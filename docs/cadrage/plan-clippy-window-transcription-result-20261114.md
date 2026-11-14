# Plan P35 — Isoler la conversion des mots d’une fenêtre

Avancée : cible de 114 lignes dont la corrélation d’événements reste dans la boucle.
Blocage : aucun ; identité, marqueur natif et preuve GPU doivent rester inchangés.
Décision : extraire seulement la conversion des mots validés en résultat de fenêtre.

## Résumé et preuve

Le Clippy structurel post-P34 compte 97 sites. `transcribe_window`, dans `crates/whisper-adapters/src/parakeet_live/inference.rs:376`, déclenche `too_many_lines` à 114 lignes. La fonction envoie une requête, vérifie l’annulation, corrèle les marqueurs natifs et résultats worker par requête/job/génération/attempt/identité/empreinte/durée, exige la preuve GPU applicable, valide les mots et rejette les résultats étrangers.

## Périmètre

- Fichier autorisé : `crates/whisper-adapters/src/parakeet_live/inference.rs`.
- Extraire uniquement la conversion du `text`, de `words` et du résultat de `validate_words` vers `WindowTranscription`.
- Garder `transcribe_window` responsable de l’envoi, de l’annulation, du polling, de la corrélation d’identité, du marqueur natif, de la preuve GPU, du traitement des erreurs worker et du délai entre polls.
- Préserver l’ordre d’appel à `validate_words`, le refus du texte sans mots horodatés, le mapping des plages en `SourceRange` à `SAMPLE_RATE`, ainsi que les messages et variantes `Usable`/`Unusable`.
- Aucun test, type de protocole, autre module, feature ou comportement public ne change.

## Tâche P35-T1

Introduire un helper privé, par exemple `window_transcription_from_words`, qui reçoit les mêmes valeurs et retourne le même `WindowTranscription`; remplacer seulement le bloc de conversion après la garde d’acceptation d’un résultat.

**Acceptation :** `transcribe_window` passe sous 100 lignes; Clippy descend de 97 à 96 sites ou moins sans nouvelle alerte; N1 passe; revue indépendante CLEAN; diff limité au fichier autorisé.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE. Les scénarios dépendant d’un worker/modèle réel demeurent ignorés et sont signalés comme tels.

**Arrêt :** échec de garde, compilation ou test; changement d’une corrélation, d’un refus, du seuil de mots ou d’une preuve GPU; fichier hors périmètre nécessaire; nouvelle alerte Clippy. Préserver les journaux.

**Retour arrière :** retirer uniquement le helper et son appel si un résultat ou message diverge.
