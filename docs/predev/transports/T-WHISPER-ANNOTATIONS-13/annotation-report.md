# Annotation automatique provisoire des WAV existants — 2026-10-05

## Autorité et portée

L'utilisateur a d'abord répondu « Annoter les WAV existants avant le seuil (recommandé) », puis, après explication que la session ne peut pas écouter les fichiers, « Annotations automatiques provisoires à vérifier ensuite (recommandé) ». Ces réponses autorisent une préparation automatique suivie d'une vérification humaine ultérieure. Elles n'acceptent **aucun seuil VAD** et ne transforment pas l'activité acoustique détectée automatiquement en vérité parole/silence.

## Fichiers et méthode

26 WAV existants : 24 FLEURS FR/EN déjà inventoriés dans `29` et `evidence/fleurs/sample-metadata.json`, plus le WAV micro FR fourni et le WAV public EN de `21`. Aucun audio n'est ajouté au corpus documentaire ; `evidence/annotation-candidates.json` contient noms, empreintes SHA-256, durées et intervalles candidats. Les scripts reproductibles sont `evidence/propose_annotations.py.txt` et `evidence/compare_vad_auto.py.txt` ; les résultats comparatifs sont `evidence/vad-automatic-comparison.json`.

Décodage local mono PCM16 16 kHz ; énergie RMS par trame de 20 ms. Seuil adaptatif `max(0,00007, 2 × P20(RMS), 0,10 × P90(RMS))` ; fermeture de trous ≤ 120 ms, suppression de segments < 80 ms. Une première borne fixe de 0,003 a donné zéro intervalle sur cinq clips EN très faibles ; elle a été rejetée et le script final emploie le seuil adaptatif. Les 26 fichiers produisent au moins un intervalle candidat. Hashs des WAV et durée sont dans le JSON. Aucune écoute humaine ni transcription de contenu n'a eu lieu.

## Comparaison exploratoire, signaux propres

Les repères RMS sont comparés aux décisions WebRTC VAD sur les WAV propres en trames de 20 ms. Les valeurs ci-dessous sont des **proxies contre une annotation automatique**, et non des estimations de précision/rappel réels du VAD.

| Groupe | Mode WebRTC | Précision proxy | Rappel proxy |
|---|---:|---:|---:|
| FLEURS FR (12) | 1 | 0,8215 | 1,0000 |
| FLEURS FR (12) | 2 | 0,8516 | 0,9979 |
| FLEURS EN (12) | 1 | 0,8873 | 0,9107 |
| FLEURS EN (12) | 2 | 0,9050 | 0,8092 |
| WAV micro FR (1) | 1 | 0,8346 | 0,9953 |
| WAV micro FR (1) | 2 | 0,9054 | 0,9883 |
| WAV public EN (1) | 1 | 0,6963 | 1,0000 |
| WAV public EN (1) | 2 | 0,7125 | 1,0000 |

Ces proxies ne permettent pas de retenir un mode : les repères d'énergie peuvent contenir du bruit, omettre des consonnes faibles et partager des erreurs avec le VAD. Les conditions -12 dB et bruit synthétique de `29` gardent leurs observations antérieures, sans nouvelle vérité annotée. Le micro EN et le bruit courant réel de DEC-26 demeurent absents du corpus disponible.

## Suite nécessaire

Faire écouter et corriger les intervalles candidats par une personne, sur les mêmes WAV, avant de calculer précision/rappel VAD ou de soumettre un seuil produit. Si cette vérification ne peut être faite, conserver `AC-08` et Q-08 ouverts et ne publier que les proxies ci-dessus. Les autres preuves d'intégration de 008 restent ouvertes.
