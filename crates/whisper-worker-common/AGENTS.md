# whisper-worker-common

Complète le `AGENTS.md` racine, notamment ses autorisations, la délégation Rust et la garde permanente. Cette carte de l'existant n'autorise aucune refonte ni campagne.

## Rôle et frontières

Bibliothèque technique partagée par les workers Turbo CPU et GPU. Elle dépend de `whisper-core`, sans dépendre des adaptateurs, du desktop ou des exécutables workers. Le worker Parakeet n'en est pas consommateur actuellement.

| Module | Responsabilité |
|---|---|
| `src/lib.rs` | Exposition des cinq modules partagés |
| `decoder` | Décodage Symphonia WAV/MP3 par fenêtres et plages bornées |
| `encoder` | Encodage MP3 interne du direct |
| `ipc` | Lecture/écriture générique des trames JSON bornées |
| `integrity` | SHA-256 et comparaison des empreintes |
| `observation` | Type callback `MemoryObserver` fourni par le consommateur |

- Conserver les moteurs natifs, l'attestation GPU, les sessions et la politique de choix/repli hors de cette bibliothèque. Les façades locales des workers transmettent les paramètres et callbacks appropriés.
- La crate hérite de `forbid(unsafe_code)` du workspace. Ne pas introduire de FFI ni affaiblir ce niveau.
- La seule feature actuelle est `l01-memory-qualification`, inactive par défaut. Elle instrumente via le callback fourni ; ne pas ajouter de journal ou de destination de sortie implicite. Elle reste distincte de `recette-hooks`, absente ici.

## Contrats à préserver

- Trames IPC : limite de 1 048 576 octets séparateur inclus, rejet des trames excessives ou incomplètes avant désérialisation et sérialisation bornée avant émission. Aucun texte libre sur stdout.
- Décodage : détection par contenu, profils audio admis, gestion gapless MP3, conversion mono 16 kHz et fenêtres de 80 000 échantillons maximum. Préserver les offsets, le nombre d'échantillons et les rejets explicites ; ne pas charger tout l'audio en mémoire.
- Encodage : préserver le cycle `push_pcm`/`finish` et le refus après finalisation. Le MP3 est un transport interne ; aucune nouvelle sortie MP3 utilisateur n'est autorisée par ce module.
- Préserver les vérifications d'intégrité et les erreurs transmises aux workers. Aucun choix de moteur, cache de modèle ou accès au GPU ne doit être ajouté ici.
- Les lectures et écritures techniques restent soumises aux chemins autorisés et à la garde racine ; cette bibliothèque ne crée aucune destination applicative supplémentaire.

## Placement et tests

Placer le changement commun dans le module existant, sans recopier son algorithme dans les workers. Une modification de contrat partagé exige l'analyse des deux consommateurs et de leurs façades ; signaler un impact hors périmètre avant toute extension.

Les tests unitaires sont dans `src/<module>/tests.rs`. Vérifier les cas de bornes, les erreurs et la parité CPU/GPU concernée dans le lot autorisé. Les tests de codec et de protocole ne prouvent ni une transcription native ni une validation dans la fenêtre Windows. Toute commande Cargo reste soumise à la garde et aux autorisations racine.
