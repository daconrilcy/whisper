Avancée : les huit extraits ciblés (15,6 s) sont étiquetés humainement et liés aux SHA-256 des WAV ; la grille RMS a été testée sur les quatre fichiers.
Blocage : aucune plage de bruit seul n'est certifiée dans les deux continus ; les bases automatiques donnent des coupures sur de la parole ou des micro-pauses avec respiration.
Décision : aucun seuil relatif au bruit n'est qualifié pour FEAS-PM-01 ; examiner l'intérêt du VAD déjà présent, sans lancer l'étude.

# Revue acoustique ciblée du corpus FEAS-PM-01

## Extraits étiquetés

Les bornes sont relatives aux WAV approuvés ; le [manifeste](manifest.json) lie chaque extrait aux échantillons et au SHA-256 du WAV source. L'utilisateur a écouté les huit extraits (15,6 s au total) et donné explicitement les étiquettes ci-dessous le 21 octobre 2026. Les temps indiqués par l'utilisateur sont approximatifs ; aucun front acoustique à l'échantillon près n'est ainsi certifié.

| Code | Source et intervalle | Étiquette donnée par l'utilisateur |
| --- | --- | --- |
| F1 | FR avec pauses, 0–7,2 s | Anglais jusque vers 4 s ; **vraie pause vers 5–6 s** ; français dès environ 6,5 s. |
| F2 | FR avec pauses, 12,4–13,8 s | **Vraie pause**, avec reprise de souffle. |
| E1 | EN avec pauses, 11,8–13,1 s | **Vraie pause**, avec reprise de souffle. |
| FC1 | FR continu, 3–4,7 s | Très courte pause pour reprendre son souffle ; durée admissible de 400 ms non établie. |
| FC2 | FR continu, 32,5–33,5 s | **Aucune pause** ; emphase sur un mot. La coupure RMS +9 dB ici est fausse. |
| EC1 | EN continu, 3,7–4,9 s | Petite pause avec bruit de respiration ; admissibilité de 400 ms non établie. |
| EC2 | EN continu, 34–34,9 s | Micro-pause avec bruit de respiration ; admissibilité de 400 ms non établie. |
| EC3 | EN continu, 54,5–55,4 s | **Aucune pause**. La coupure RMS +12 dB ici est fausse. |

## Méthode et mesures provisoires

Chaque WAV est PCM16 mono à 16 kHz. Le calcul indépendant lit tous les échantillons par blocs non recouvrants de 320 échantillons (20 ms) et prend `20 log10(RMS/32768)` en dBFS. Une pause candidate est une suite d'au moins 20 trames sous le seuil (400 ms). L'espacement d'au moins 1 s entre deux coupes sera appliqué après la détection ; il ne peut réparer une fausse pause. Pour cette comparaison préliminaire, les niveaux de base sont les 95e percentiles de la plage la plus calme de 200 ms issus d'[auto_annotations.json](auto_annotations.json). Cette plage reste **bruit seul non confirmé**, particulièrement dans les deux continus.

| Source | Base provisoire dBFS | Seuil +6 dB | Pauses candidates +6 | Seuil +9 dB | Candidates +9 | Seuil +12 dB | Candidates +12 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| FR avec pauses | −65,17 | −59,17 | 1 | −56,17 | 1 | −53,17 | 2 |
| EN avec pauses | −63,29 | −57,29 | 1 | −54,29 | 1 | −51,29 | 1 |
| FR continu | −36,05 | −30,05 | 2 | −27,05 | 4 | −24,05 | 5 |
| EN continu | −34,33 | −28,33 | 5 | −25,33 | 8 | −22,33 | 12 |

La pause française candidate à 12,84–13,40 s n'est captée par aucune des trois marges avec la base provisoire ; elle apparaît à +15 dB (12,96–13,40 s), tandis que les continus ont alors respectivement 7 et 35 coupures candidates. La pause anglaise à 12,08–12,68 s est captée dès +6 dB. Un seuil fixe de −35 dBFS donne, **avec cette méthode RMS précise**, 13, 3, 0 et 0 coupures respectivement ; ce résultat n'est ni un seuil relatif qualifié ni une étiquette humaine. Les décomptes FFmpeg précédents diffèrent parce que son détecteur de silence n'emploie pas la même règle RMS à 20 ms.

Un balayage de −10 à +30 dB par pas de 0,25 dB sur les mêmes bases provisoires situe la première détection de F2 à +14,5 dB et celle de E1 à +4,75 dB. La première coupure candidate apparaît déjà à +1,5 dB dans le FR continu et à +2 dB dans l'EN continu. Les bornes dépendent du pas du balayage et de la règle de 400 ms ; elles démontrent seulement qu'**avec ces quatre bases automatiques**, aucune marge commune ne satisfait simultanément les pauses étiquetées et l'absence de coupure des continus. Une vraie base de bruit seul pourrait changer ce résultat.

L'écoute confirme que FC2, utilisée par le proxy de bruit FR continu à 33,0–33,2 s, contient une emphase et aucune pause. EC3, utilisée par le proxy EN continu à 54,8–55,0 s, n'est pas une pause non plus ; elle n'a pas été reconnue comme **bruit seul**. Ces bases ne constituent donc pas des calibrations validées. Sur les 60 s, +9 dB crée une fausse pause FR à 32,78–33,20 s, tandis que +12 dB crée une fausse pause EN à 54,62–55,02 s. Les autres coupures des continus ne sont pas toutes étiquetées : on ne peut annoncer un taux global de faux positifs. Les vraies pauses F2/E1 et les micro-pauses avec respiration montrent aussi qu'une baisse d'énergie ne suffit pas à définir l'admissibilité de toutes les coupes.

## Début anglophone du FR avec pauses

Le texte humain approuvé du WAV FR commence en anglais (`Thank you for one minute. Mr. Riviere.`), puis passe en français (`Merci, Madame la Presidente.`). F1 confirme l'anglais jusque vers 4 s, une vraie pause vers 5–6 s et le français dès environ 6,5 s ; F2 confirme une autre vraie pause française. Le scénario **français avec pauses** dispose donc d'environ 58,5 s de parole française, de sa référence textuelle approuvée et de plusieurs pauses candidates ultérieures. Il peut utiliser cette portion avec une borne de départ et un offset temporel explicites, sans modifier le WAV ou la référence intégrale. La borne exacte au premier mot français reste un temps provisoire (Parakeet propose 6,72 s), mais elle n'empêche pas de qualifier la disponibilité du scénario.

## Conclusion sur le seuil et le VAD

Les paramètres reproductibles examinés sont **RMS 20 ms**, pause candidate **≥400 ms**, au moins **1 s entre coupes**, base de bruit = **95e percentile d'une plage de bruit seul**, puis marges **+6/+9/+12 dB**. Aucune marge n'est retenue : les deux bases des continus ne sont pas du bruit seul certifié, la grille manque F2, et au moins deux coupures des continus sont humainement réfutées. Le fait que −35 dBFS ne coupe aucun continu avec cette règle n'en fait pas un seuil relatif au bruit qualifié. Le contrôle complet des deux fichiers de 60 s est **partiel** : les décomptes automatiques couvrent toute leur durée, mais seules les zones ciblées ont une étiquette humaine.

Un VAD est pertinent à examiner pour tester une décision de parole fondée sur davantage que le seul niveau RMS, notamment sur les respirations et les emphases étiquetées. Le dépôt contient déjà `webrtc-vad = 0.4.0` et l'adaptateur [`vad.rs`](../../../crates/whisper-adapters/src/vad.rs) traite des trames PCM de 20 ms ; `worker_ipc.rs` l'utilise déjà pour compter la parole en capture. Cela prouve la disponibilité technique d'un composant, **pas** sa capacité démontrée à détecter les bonnes coupes Parakeet sur ce corpus. Une [étude VAD hors produit](vad-prep-report.md) a depuis traité les quatre WAV complets ; elle conclut `PARTIAL` et ne change pas le protocole. Aucune stratégie A/B ni FEAS-PM-01 n'a été lancée.
