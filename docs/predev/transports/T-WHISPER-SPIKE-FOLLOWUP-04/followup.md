# Suivi borné des SPIKE — 2026-10-05

Base documentaire D-WHISPER-04. Mandat d'essai SPIKE-01/02 accordé par l'utilisateur. Ce transport ajoute des observations ; il ne modifie pas le candidat examiné et ne revendique pas la fermeture de WHISPER-DESIGN-008.

## CPU forcé

Le binaire CPU-only `whisper-rs 0.16.0` construit en release (SHA-256 `b07ffc3f59a6b3387e61663157967140d995a1f00141f6067de3f77efdc3f928`) a été exécuté sur le PC possédant la GTX 1080 Ti avec PATH réduit à Windows. Sur le WAV utilisateur, le log `cpu-only-binary-run.log.txt` indique `use gpu = 0`, `no GPU found`, une transcription et 14 878 ms d'exécution. Le mode CPU forcé peut donc être assuré par un moteur CPU-only séparé. Cela n'est pas encore un paquet/installeur intégré ni un test de fallback en cours de job.

## Échantillons et sensibilité au seuil

- WAV utilisateur `C:\WhisperLive\micro_test.wav` : SHA-256 `716e788c790b600fe472d0ebb868094c9d9b97fdf1d581133dd3df46a30de467`, mono 16 kHz, 14,592 s. Aucune copie du contenu audio ni du texte transcrit dans ce transport.
- WAV anglais public `jfk.wav` du dépôt officiel [whisper.cpp](https://github.com/ggml-org/whisper.cpp/blob/master/README.md), SHA-256 `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`, 11 s. La source officielle donne le chemin `samples/jfk.wav` ; téléchargement le 2026-10-05. Un seul extrait de discours propre, pas un corpus représentatif.
- `ffmpeg silencedetect` avec minimum 0,3 s : WAV français, silence total détecté 13,64 s à −25 dB, 9,34 s à −35 dB, 6,39 s à −45 dB ; WAV anglais, 3,56 s, 0 s, 0 s respectivement. Ces valeurs montrent une forte sensibilité au seuil sur ces deux entrées. Le filtre de niveau n'est pas un VAD validé et il n'existe pas d'annotation de référence ; aucun taux d'erreur ni seuil V1 ne peut en être déduit.
- Le moteur GPU Rust a traité le WAV anglais en 1 905 ms (`jfk-gpu-run.log.txt`). Ce test ajoute une entrée anglaise, sans prouver VAD, latence live ni qualité générale.

Commandes reproductibles pour chaque WAV : `ffmpeg -hide_banner -nostats -i INPUT -af silencedetect=noise=-35dB:d=0.3 -f null NUL`, en répétant avec −25 et −45 dB. Le calcul du total additionne les `silence_duration` du journal. Ces résultats ne sont pas des seuils acceptés par l'utilisateur.
