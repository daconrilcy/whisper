Avancée : les sources, paramètres et bornes des WAV synthétiques de `target/parakeet-tests/` sont inventoriés avant nettoyage.
Blocage : les mélanges B9 anciens n'ont pas de graine de bruit consignée ; leur régénération reproduit le scénario, sans garantir le même SHA.
Décision : les entrées encore utilisées par la garde R7 et les WAV B9 restent conservés dans ce nettoyage.

# Régénération des WAV synthétiques Parakeet

Source réelle EN : `C:\dev\whisper\sound example\spkr0.wav` (300 s, PCM mono 16 kHz). Sources FR : `C:\dev\whisper\target\parakeet-feasibility\spike02\fixtures\{fr87,fr60,fr27,fr25}.wav`, conservées intégralement. Tous les exemples ci-dessous écrivent dans une nouvelle racine d'essai ; les chemins d'origine ne doivent pas être écrasés. `ffmpeg.exe` doit être disponible. Les commandes reproduisent le contenu PCM et les durées ; l'en-tête WAV peut différer selon la version de ffmpeg.

## Répétitions EN et FR

Pour EN ×2, ×6 ou ×12, écrire un fichier liste comportant respectivement 2, 6 ou 12 lignes identiques `file 'C:/dev/whisper/sound example/spkr0.wav'`, puis :

```powershell
ffmpeg.exe -hide_banner -loglevel error -f concat -safe 0 -i liste-en.txt -ar 16000 -ac 1 -c:a pcm_s16le spkr0_xN.wav
```

Cette recette couvre `l2/l2b/runtime/audio/spkr0_x12.wav` et `l2/l2b/r5-2/fixtures/spkr0_x6.wav`. Le fichier `spkr0_x12.wav` du banc `spike02/` reste conservé.

Pour le FR, assembler `fr87, fr60, fr27, fr25, fr87, fr27` dans cet ordre, avec une seconde de silence entre deux extraits. Générer chaque silence par `ffmpeg.exe -f lavfi -i anullsrc=r=16000:cl=mono -t 1 -c:a pcm_s16le silence-1s.wav`, puis utiliser une liste concat de ces onze entrées :

```powershell
ffmpeg.exe -hide_banner -loglevel error -f concat -safe 0 -i liste-fr.txt -ar 16000 -ac 1 -c:a pcm_s16le fr_concat_318.wav
```

Pour ×6, répéter six fois le chemin de `fr_concat_318.wav` dans `liste-fr-x6.txt`, puis lancer la même commande concat. Cette recette couvre `l2/l2b/runtime/audio/fr_concat_x6.wav` et `l2/l2b/r5-2/fixtures/fr_concat_x6.wav`. Le montage `spike02/fixtures/synthetic/fr_concat_318.wav` reste conservé.

## Silences de limite

Les fichiers `exact-30-min.wav` et `exact-qualified-limit.wav` ont 28 800 000 échantillons ; les fichiers `over-30-min-one-sample.wav` et `over-qualified-limit-one-sample.wav` en ont 28 800 001. `a4-silence-1800.wav` a 28 800 000 échantillons et `a4-silence-1800_1.wav` en a 28 801 600. Pour chacun, remplacer `N` par ce nombre :

```powershell
ffmpeg.exe -hide_banner -loglevel error -f lavfi -i anullsrc=r=16000:cl=mono -af "atrim=end_sample=N,asetpts=PTS-STARTPTS" -c:a pcm_s16le silence-N.wav
```

Cela couvre les fichiers de limite directement sous `l2/l2a-1/output/`, `l2/l2a-2/output/`, `l2/l2a-2/archive/parakeet-duration-boundary/`, `l2/l2a-2/output/archives/parakeet-duration-boundary/`, `l2/l2b/archive/parakeet-duration-boundary/`, `l2/l2b/archive-regression/parakeet-duration-boundary/`, `l2/l2b/r6/archive/parakeet-duration-boundary/`, `l2/l2b/r7/archive/parakeet-duration-boundary/` et `l2/l2b/runtime/archive-r4/parakeet-duration-boundary/`.

## Fenêtres VAD FR

Les deux séries `l2/l2b/long/fr-long/vad/window-*.wav` et `l2/l2b/r5-2/archive/long/fr-long/vad/window-*.wav` sont identiques octet pour octet. Les 22 fenêtres sont des extractions PCM du montage `fr_concat_x6.wav`. Les bornes ci-dessous ont été retrouvées par comparaison exacte des échantillons. Pour chaque ligne, substituer `DEBUT`, `FIN` et `INDEX` :

```powershell
ffmpeg.exe -hide_banner -loglevel error -i fr_concat_x6.wav -af "atrim=start_sample=DEBUT:end_sample=FIN,asetpts=PTS-STARTPTS" -ar 16000 -ac 1 -c:a pcm_s16le window-INDEX.wav
```

| Index | Début inclus | Fin exclue | Index | Début inclus | Fin exclue |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 000 | 0 | 1467840 | 011 | 15385920 | 16810880 |
| 001 | 1461440 | 2908480 | 012 | 16804480 | 18234880 |
| 002 | 2902080 | 4208160 | 013 | 18228480 | 19534560 |
| 003 | 4201760 | 5624320 | 014 | 19528160 | 20950720 |
| 004 | 5617920 | 7042720 | 015 | 20944320 | 22369120 |
| 005 | 7036320 | 8433280 | 016 | 22362720 | 23759680 |
| 006 | 8426880 | 9851520 | 017 | 23753280 | 25177920 |
| 007 | 9845120 | 11167680 | 018 | 25171520 | 26494080 |
| 008 | 11161280 | 12607840 | 019 | 26487680 | 27934240 |
| 009 | 12601440 | 13991520 | 020 | 27927840 | 29317920 |
| 010 | 13985120 | 15392320 | 021 | 29311520 | 30652800 |

## Bruit rose B9

Les sept WAV de `l2/l2b/noise/` et les trois mélanges de `l2/l2b/r5-2/fixtures/spkr0_x12_noise_{20,30,40}.wav` reproduisent le scénario suivant. Créer `spkr0_x12.wav` comme ci-dessus. Pour chaque niveau `N = -40, -30, -20` dBFS :

```powershell
ffmpeg.exe -hide_banner -loglevel error -f lavfi -i "anoisesrc=c=pink:r=16000:a=1:seed=1" -t 3600 -af "volume=NdB" -ac 1 -c:a pcm_s16le pink_N.wav
ffmpeg.exe -hide_banner -loglevel error -i spkr0_x12.wav -i pink_N.wav -filter_complex "[0:a][1:a]amix=inputs=2:duration=first:normalize=0[a]" -map "[a]" -ar 16000 -ac 1 -c:a pcm_s16le spkr0_x12_pink_N.wav
```

Pour `pink-40-calibration.wav`, utiliser la première commande avec `-t 5`. Les paramètres originaux précis, notamment la graine, ne sont pas archivés ; ces commandes ne garantissent donc pas les SHA des anciens mélanges. **Leurs fichiers existants ne figurent pas au manifeste de suppression actuel.**
