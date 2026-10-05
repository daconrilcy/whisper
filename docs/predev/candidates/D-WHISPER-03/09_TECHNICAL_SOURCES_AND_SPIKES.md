# Sources techniques et preuves à produire — 2026-10-05

## Lecture des sources primaires

Les sources primaires ci-dessous décrivent les interfaces et contraintes. Les essais Rust réellement exécutés et leurs limites sont détaillés dans `19_SPIKE_RESULTS.md` et `evidence/`. Les versions restent candidates au produit final.

| Sujet | Source primaire consultée le 2026-10-05 | Ce qu'elle établit / ce qu'elle ne prouve pas |
|---|---|---|
| `whisper-rs 0.16.0` | [features crate](https://docs.rs/crate/whisper-rs/0.16.0/features), [BUILDING.md](https://docs.rs/crate/whisper-rs/0.16.0/source/BUILDING.md) | Feature `cuda` et instructions Windows MSVC/CMake/LLVM/CUDA présentes. Build CPU/CUDA et runtime GTX 1080 Ti maintenant observés sur un échantillon ; voir `19`. |
| `whisper.cpp`/ggml | [CMakeLists officiel](https://github.com/ggml-org/whisper.cpp/blob/master/ggml/CMakeLists.txt) | Option `GGML_CUDA` existe dans la branche courante. La version native embarquée dans la crate doit être figée et confrontée à ce résultat ; master n'est pas la preuve de cette révision. |
| Modèle | [README modèles officiel](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md) | `large-v3-turbo` ggml multilingue listé avec taille indicative et SHA-1 upstream. Pour l'installateur, figer URL, licence et **SHA-256 des octets distribués** ; modèle téléchargé et SHA-256 vérifié dans `19`. |
| GPU cible | [table NVIDIA legacy](https://developer.nvidia.com/cuda/gpus/legacy) | GTX 1080 Ti est de capacité 6.1. Cela ne garantit pas qu'un toolkit CUDA actuel, la crate native et les DLL livrées la supportent. |
| MP3 | [encodeur Media Foundation](https://learn.microsoft.com/en-us/windows/win32/medfound/mp3-audio-encoder), [Sink Writer](https://learn.microsoft.com/en-us/windows/win32/medfound/using-the-sink-writer) | Encodeur MP3 documenté et besoin de type entrée/sortie/encodeur. Ne garantit ni intégration Rust, ni format/paramètres finalement choisis, ni récupération après interruption. |
| Publication fichiers | [MoveFileEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw) | Des drapeaux de remplacement et write-through existent ; aucun appel ne publie atomiquement MP3+TXT+SRT ensemble. Le marqueur Complete de `08` est un protocole à éprouver. |

CPAL, tray-icon, global-hotkey, autostart, moteur VAD et installateur restent **candidats**, sans décision de version. Le texte joint initial contient des liens tiers et des propositions ; il n'est pas une preuve de disponibilité de la combinaison exacte. L'essai manuel Python CPU/GPU rapporté par l'utilisateur est conservé avec son statut dans `06`; les commandes/logs ne sont pas attachés.

## SPIKE-01 — faisabilité moteur, GPU et livraison

Mandat proposé : dans un espace d'essai séparé du dépôt produit, figer toolchain, crate et révision native ; compiler CPU puis CUDA x64 MSVC ; identifier toolkit/driver/DLL et capacité 6.1 effectivement utilisée ; charger le modèle livré par installateur, vérifier SHA-256 et licence ; exécuter même échantillon en CPU et GPU, comparer sorties/offsets, mesurer ressources et tracer le backend effectif ; installer sur le PC cible sans outil de développement requis à l'exécution. Tester GPU absent/incompatible, Auto→CPU et GPU forcé arrêté. Livrables : manifest des versions et fichiers, commandes, logs, hashes, métriques, dépendances runtime, résultat reproductible et décision d'architecture. Critère de réussite : les chemins CPU/GPU et premier lancement offline fonctionnent sur la cible, le backend GPU est démontré, le fallback respecte `AC-07` et le modèle est intact ; sinon résultat FAIL avec variante ou réduction de périmètre à faire accepter. **Mandat explicitement accordé par l’utilisateur ; résultat PARTIAL dans `19`.**

## SPIKE-02 — durabilité, arrêt et formats

Mandat proposé : fixture Windows sur volume du dossier choisi, écrire des fragments et une file versionnés ; injecter coupure aux points avant/après sync et publication ; vérifier recovery, absence de faux Complete, perte du fragment ouvert et temps d'arrêt ; saturer canaux et bloquer worker/encodeur ; qualifier WAV/MP3 d'entrée, MP3/TXT/SRT de sortie et VAD sur corpus défini. Livrables : matrice d'injection, fichiers avant/après, mesures, journal de réconciliation, paramètres d'encodage, limites observées. Critère de réussite : `AC-03/08/09/10/11/12/13` démontrés avec politique Q résolue, sans suppression de source et sans perte silencieuse ; sinon FAIL et design révisé. **Mandat explicitement accordé par l’utilisateur ; résultat PARTIAL dans `19`.**

Les SPIKE ne sont pas des tests du produit final. La revue doit juger les preuves et les lacunes observées, sans convertir PARTIAL en PASS. Voir aussi le modèle officiel : [révision LFS](https://huggingface.co/ggerganov/whisper.cpp/commit/6034871ec87c84e342efab769d4c5c06cd126db3).
