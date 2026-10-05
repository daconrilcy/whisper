# Mesures bornées VAD, perte et délai live — 2026-10-05

## Portée et provenance

Essais SPIKE-02 en dehors du dépôt produit, avec scripts et résultats exacts dans `evidence/measurements/`. Aucune implémentation de l'interface, du micro, de la transcription progressive finale ou du VAD retenu n'existe dans le dépôt. Les mesures ci-dessous qualifient des **prototypes et politiques d'essai**, pas des critères V1 acceptés. Aucun audio ni texte transcrit n'est inclus dans les logs documentaires.

Corpus de départ : `C:\WhisperLive\micro_test.wav`, voix FR utilisateur, 14,592 s, SHA-256 `716e788c790b600fe472d0ebb868094c9d9b97fdf1d581133dd3df46a30de467` ; `samples/jfk.wav` du [dépôt officiel whisper.cpp](https://github.com/ggml-org/whisper.cpp/blob/master/README.md), voix EN publique, 11 s, SHA-256 `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e`. Deux locuteurs seulement. Les variantes de VAD ajoutent silence numérique, atténuation de 12 dB et bruit blanc à −30 dBFS avec graine 20261005. Pour le replay GPU, les fixtures répètent chaque clip trois fois avec deux pauses de 3 s (FR 49,776 s ; EN 39 s) ; hashs dans `live-fixtures.json`. Le replay CPU utilise le WAV français original de 14,592 s, sans répétition. Les répétitions GPU sont utiles pour la charge, pas des locuteurs indépendants.

## VAD — test de sensibilité

WebRTC VAD (`webrtcvad-wheels 2.0.14`, 16 kHz mono PCM16, trames de 20 ms), modes 0 à 3 : 40 combinaisons enregistrées dans `vad-results.json`. Le tableau donne la **durée signalée comme parole**, sans l'appeler exactitude ; les blocs vocaux contiennent aussi des pauses internes non annotées.

| Cas | Durée | Mode 1 | Mode 2 |
|---|---:|---:|---:|
| FR propre | 14,58 s | 10,16 s | 9,30 s |
| FR à −12 dB | 14,58 s | 9,52 s | 8,70 s |
| FR + bruit −30 dBFS | 14,58 s | 14,38 s | 1,14 s |
| EN propre | 11,00 s | 9,68 s | 9,46 s |
| EN à −12 dB | 11,00 s | 9,32 s | 9,02 s |
| EN + bruit −30 dBFS | 11,00 s | 11,00 s | 7,84 s |
| Silence numérique, chaque langue | 5,00 s | 0 | 0 |
| Bruit seul −30 dBFS, FR | 5,00 s | 4,28 s | 0,08 s |
| Bruit seul −30 dBFS, EN | 5,00 s | 4,38 s | 0,08 s |

Le mode 1 signale presque tout le bruit seul ; le mode 2 réduit ce signal mais manque une grande part du bloc FR bruité. **Aucun mode ni seuil V1 n'est choisi.** Il faut des références annotées humainement, plusieurs voix/acoustiques et du bruit réel pour calculer précision/rappel et décider d'un seuil. Les silences sont conservés dans l'audio d'archive quel que soit le compteur VAD.

## Perte du fragment ouvert après arrêt de processus

100 exécutions indépendantes, 50 par cadence de confirmation de 0,5 s et de 1 s, avec positions de coupure fixes aux frontières et pseudo-aléatoires (graine 20261005). La médiane est la moyenne des deux valeurs centrales ; le P95 est le rang supérieur `ceil(0,95 × n)`. Le processus enfant simule des trames de 20 ms en mémoire ; à chaque fragment complet, il écrit, synchronise et renomme les fichiers, puis s'arrête brutalement au point prescrit (code 42). La reprise vérifie le hash des fragments confirmés ; la perte est `capturé − confirmé`, pas la taille de fichiers orphelins. Source et cas dans `measure_loss.py.txt` / `loss-results.json`.

| Cadence | Cas | Perte médiane | P95 (nearest rank) | Maximum | Reprise max |
|---|---:|---:|---:|---:|---:|
| 0,5 s | 50 | 260 ms | 480 ms | 480 ms | 26,3 ms |
| 1 s | 50 | 350 ms | 980 ms | 980 ms | 30,6 ms |

Ces observations portent sur le **seul fragment audio ouvert** et sur des sorties de processus ; elles ne prouvent pas une coupure électrique, la persistance du répertoire Windows, la cohérence audio+TXT/SRT ni la capture réelle. Aucun seuil de perte n'est accepté. La cadence de 0,5 s réduit la perte observée dans cette fixture au prix d'écritures plus fréquentes, à mesurer sous charge.

## Délai du texte live — replay cadencé

Le modèle `large-v3-turbo` reste chargé. Les séries GPU rejouent les fixtures FR et EN répétées ; la série CPU rejoue une seule fois le WAV FR original. Un thread lit les WAV à cadence de 20 ms, envoie des fenêtres de 5 s à un worker séparé, qui mesure attente de file, inférence et premier callback de segment ; la sortie console après `full()` sert de **proxy du rendu visible**. Les scripts Rust, fixtures, logs par fenêtre et agrégats sont dans `evidence/measurements/`. Le GPU est la GTX 1080 Ti ; CPU = binaire distinct sans CUDA. Les 21 fenêtres ont produit un callback et n'ont pas retourné d'erreur.

| Replay | Fenêtres | Médiane après fin de fenêtre | P95 (nearest rank) | Maximum | Attente file max |
|---|---:|---:|---:|---:|---:|
| GPU FR+EN | 18 | 552,5 ms | 652 ms | 652 ms | 0 ms |
| CPU FR | 3 | 22,052 s | 30,943 s | 30,943 s | 17,452 s |

La politique de fenêtre ajoute jusqu'à 5 s entre la capture d'un échantillon au début d'une fenêtre et sa fin : dans ce replay, la borne observée début de fenêtre→sortie console est 5,652 s GPU et 35,535 s CPU. Ce n'est pas le délai d'un mot annoté à son apparition dans l'UI. Le CPU traite chaque fenêtre de 5 s en ~13,5 s sur le clip FR original et accumule du retard ; la politique de fenêtrage/charge CPU doit être révisée avant de promettre du temps réel. Les clips courts, répétés et propres ne couvrent ni long live, ni tous les micros, ni la charge système.

## Décision après mesures

Les résultats permettent de comparer les options mais ne suffisent pas à arrêter les seuils V1 de qualité VAD, perte ou délai. Conformément à la réponse utilisateur, ces valeurs restent `OPEN` jusqu'à un corpus annoté et une décision explicite. Les méthodes et données ci-dessus sont conservées pour réplication et élargissement ; `AC-02/08/13` du futur produit restent NOT RUN.
