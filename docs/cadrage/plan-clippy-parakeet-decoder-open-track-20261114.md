# Plan P41 — Isoler l’ouverture et la validation de la piste audio

Avancée : le Clippy frais après P40 compte 91 sites et relève `decode_file` à 118 lignes.
Blocage : aucun ; conserver les refus de formats et les paramètres de piste existants.
Décision : extraire l’ouverture, le probing et la validation du profil en helper privé.

## Résumé et preuve

Clippy signale `crates/whisper-worker-parakeet-cpu/src/decoder.rs:25`, `decode_file(path)`, à 118 lignes. La fonction ouvre et sonde la source, choisit la piste audio, valide la fréquence et les canaux ainsi que le profil MP3 Q-07, puis installe le décodeur et parcourt les paquets pour produire les échantillons mono rééchantillonnés.

## Périmètre

- Fichier autorisé : `crates/whisper-worker-parakeet-cpu/src/decoder.rs`.
- Extraire uniquement l’ouverture/probing, la sélection de piste et la validation du format/layout dans un helper privé, avec un petit résultat interne contenant le lecteur format et les métadonnées retenues si nécessaire.
- Garder dans `decode_file` l’initialisation du décodeur, la boucle de paquets, le contrôle des changements de profil MP3 pendant la lecture, le rééchantillonnage, la limite de durée et le résultat final.
- Préserver les textes d’erreur, options gapless, codec, track ID, source rate, canaux et refus Q-07.
- Aucun test, type public, protocole, feature ou autre fichier ne change.

## Tâche P41-T1

Introduire un helper privé `open_audio_track` (ou nom équivalent) et simplifier uniquement le début de `decode_file`.

**Acceptation :** `decode_file` passe sous 100 lignes; Clippy descend de 91 à 90 sites ou moins sans nouvelle alerte; N1 passe; revue indépendante CLEAN; diff limité au fichier autorisé.

**Vérifications :** SelfTest de `ci/guard-level1.ps1`, `cargo fmt --check`, mesure Clippy seuil 0 et ratchet standard six crates via la garde, N1 complet sur cible fraîche, sélections worker CPU/Parakeet CPU/common et SelfTest PE. Les tests natifs qui requièrent un modèle restent signalés selon leur statut réel.

**Arrêt :** échec de garde, compilation ou test; divergence d’ouverture, piste, codec, profil MP3, erreurs, rééchantillonnage ou limite d’échantillons; fichier hors périmètre nécessaire; nouvelle alerte Clippy. Préserver les journaux.

**Retour arrière :** remettre l’ouverture et les contrôles dans `decode_file` si le résultat ou une erreur diverge.
