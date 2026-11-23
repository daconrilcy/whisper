Avancée : P9 a passé l’installation côte à côte et l’unique import WAV Turbo en GPU sur la GTX 1080 Ti.
Blocage : aucun sur le périmètre P9 ; le résultat ne qualifie ni la publication Store, ni d’autres machines ou modes.
Décision : prototype NVIDIA local validé pour ce scénario précis ; conserver le paquet de test et son certificat jusqu’à décision de rétention.

# Bilan P9 — prototype MSIX NVIDIA — 23 novembre 2026

## Candidat et installation

- Commit source P8 : `a3593dabb5b456f40457efc0a6cbec589be99a70`.
- MSIX : `C:\wt\whisper-msix-cpu-spike\target\parakeet-tests\msix-nvidia-prototype-20261123\package\WhisperTurbo-NVIDIA-prototype.msix`, SHA-256 `72694f261d8035c820b2fc26e662a34fcdfc35d7d70e9c4bad96d69eb9780260`, 3 340 753 443 octets.
- Certificat de test : sujet `CN=WhisperTurbo MSIX NVIDIA Prototype`, empreinte `2669EF242D2202EBE7D4496913FB84C38B34D154`, certificat public SHA-256 `de1015e83811f73df3598ee1e8d879106c8ea9c58d28c07a461039a3ac6aa48c`. Importé dans `Cert:\LocalMachine\TrustedPeople` pour cette recette.
- Installation P9 : `WhisperTurbo.NvidiaPrototype` 0.1.0.0, famille `WhisperTurbo.NvidiaPrototype_yxmgx5c9e80fr`, `Status=Ok`, sous `C:\Program Files\WindowsApps\WhisperTurbo.NvidiaPrototype_0.1.0.0_x64__yxmgx5c9e80fr`.
- Le paquet distinct P8 `WhisperTurbo.CpuPrototype` est resté installé (`Status=0`). Aucun remplacement ni désinstallation n’a eu lieu.

## Résultat de la recette autorisée

- Un seul import manuel du WAV `C:\wt\whisper-msix-cpu-spike\docs\cadrage\feas-pm-01-corpus\fr_prefix_0-30.wav`, SHA-256 `dc86235c75c80c2a424305e0b2f2b6cac60932ea5449eabc1b9419baa19703c`.
- Modèle sélectionné : Whisper large-v3-turbo. La fenêtre a affiché `Moteur actif : CUDA`, puis `Import JobId(807600250988270920058280) — Complete`, `480000 / 480000 échantillons` et `Transcription publiée et vérifiée`.
- Le journal worker relie ce job et cette empreinte source à `worker_started`, `backend_ready` et aux événements de progression, tous avec `backend=cuda`, jusqu’à 480 000 échantillons.
- Le worker GPU du job a été observé sous `WindowsApps`. `cudart64_12.dll`, `cublas64_12.dll` et `cublasLt64_12.dll` étaient chargées depuis le dossier `app` du même package. `nvcuda.dll` et `nvcuda64.dll` provenaient du pilote Windows/NVIDIA.
- Le manifeste de publication enregistre 13 segments et les empreintes des exports non vides :
  - TXT : `C:\Users\cyril\Documents\WhisperTranscriptions\transcriptions\000000000000ab0418eab051c7bac5a8\1\transcript.txt`, 669 octets, SHA-256 `caf82a03d382248670f1769643926be93631b9ed721b21142371e6e00b632d68`.
  - SRT : `C:\Users\cyril\Documents\WhisperTranscriptions\transcriptions\000000000000ab0418eab051c7bac5a8\1\transcript.srt`, 1 090 octets, SHA-256 `701339c7a6f4e8e9cb5e9b656d6e312231b72bf376ed6754d87350bf13243671`.

## Portée

**P9 : PASS** pour l’assemblage signé, l’installation côte à côte et un import WAV Turbo utilisant le backend CUDA et les runtimes CUDA du package MSIX. Le certificat est un certificat de test local. Cette recette ne constitue pas une validation de publication Store, une qualification de qualité ou de vitesse, ni une qualification d’autres modèles, GPU, machines ou modes. Aucun nouvel essai n’a été réalisé.

Les preuves machine restent aux emplacements indiqués ci-dessus ; le présent bilan compact est l’archive hors `target/` exigée par le mandat. Aucun cache, paquet ni certificat n’a été supprimé.

Suivi de nettoyage post-clôture : [désinstallation P9 et retrait du certificat de test](suivi-nettoyage-msix-nvidia-20261123.md).
