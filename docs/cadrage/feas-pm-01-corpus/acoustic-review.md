Avancée : huit extraits ciblés (15,6 s) ont été dérivés sans modifier les quatre WAV ; un balayage RMS de la grille de marges a été effectué.
Blocage : les étiquettes humaines des extraits et une plage de bruit seul dans chaque enregistrement continu manquent encore ; aucun seuil relatif n'est qualifié.
Décision : garder les paramètres acoustiques au statut provisoire et ne pas exécuter FEAS-PM-01.

# Revue acoustique ciblée du corpus FEAS-PM-01

## Extraits à valider

Les bornes sont relatives aux WAV approuvés ; le [manifeste](manifest.json) lie chaque extrait aux échantillons et au SHA-256 du WAV source. L'écoute des huit extraits dure 15,6 s au total. Une étiquette acoustique n'est acquise qu'après réponse explicite de l'utilisateur.

| Code | Source et intervalle | Décision humaine attendue |
| --- | --- | --- |
| F1 | FR avec pauses, 0–7,2 s | Dernière parole anglaise, première parole française, vraie pause ou bruit/hésitation entre elles. |
| F2 | FR avec pauses, 12,4–13,8 s | Pause réelle, hésitation voisée ou bruit. |
| E1 | EN avec pauses, 11,8–13,1 s | Pause réelle, hésitation voisée ou bruit. |
| FC1 | FR continu, 3–4,7 s | Parole faible, hésitation, bruit seul ou pause réelle. |
| FC2 | FR continu, 32,5–33,5 s | Même choix ; zone où une marge relative de +9 dB produit une coupure candidate. |
| EC1 | EN continu, 3,7–4,9 s | Même choix ; coupure candidate avec +9 dB. |
| EC2 | EN continu, 34–34,9 s | Même choix ; coupure candidate avec +9 dB. |
| EC3 | EN continu, 54,5–55,4 s | Même choix ; plage la plus calme de 200 ms, proposée automatiquement comme bruit, sans validation. |

## Méthode et mesures provisoires

Chaque WAV est PCM16 mono à 16 kHz. Le calcul indépendant lit tous les échantillons par blocs non recouvrants de 320 échantillons (20 ms) et prend `20 log10(RMS/32768)` en dBFS. Une pause candidate est une suite d'au moins 20 trames sous le seuil (400 ms). L'espacement d'au moins 1 s entre deux coupes sera appliqué après la détection ; il ne peut réparer une fausse pause. Pour cette comparaison préliminaire, les niveaux de base sont les 95e percentiles de la plage la plus calme de 200 ms issus d'[auto_annotations.json](auto_annotations.json). Cette plage reste **bruit seul non confirmé**, particulièrement dans les deux continus.

| Source | Base provisoire dBFS | Seuil +6 dB | Pauses candidates +6 | Seuil +9 dB | Candidates +9 | Seuil +12 dB | Candidates +12 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| FR avec pauses | −65,17 | −59,17 | 1 | −56,17 | 1 | −53,17 | 2 |
| EN avec pauses | −63,29 | −57,29 | 1 | −54,29 | 1 | −51,29 | 1 |
| FR continu | −36,05 | −30,05 | 2 | −27,05 | 4 | −24,05 | 5 |
| EN continu | −34,33 | −28,33 | 5 | −25,33 | 8 | −22,33 | 12 |

La pause française candidate à 12,84–13,40 s n'est captée par aucune des trois marges avec la base provisoire ; elle apparaît à +15 dB (12,96–13,40 s), tandis que les continus ont alors respectivement 7 et 35 coupures candidates. La pause anglaise à 12,08–12,68 s est captée dès +6 dB. Un seuil fixe de −35 dBFS donne, **avec cette méthode RMS précise**, 13, 3, 0 et 0 coupures respectivement ; ce résultat n'est ni un seuil relatif qualifié ni une étiquette humaine. Les décomptes FFmpeg précédents diffèrent parce que son détecteur de silence n'emploie pas la même règle RMS à 20 ms.

Un balayage de −10 à +30 dB par pas de 0,25 dB sur les mêmes bases provisoires situe la première détection de F2 à +14,5 dB et celle de E1 à +4,75 dB. La première coupure candidate apparaît déjà à +1,5 dB dans le FR continu et à +2 dB dans l'EN continu. Les bornes dépendent du pas du balayage et de la règle de 400 ms ; elles démontrent seulement qu'**avec ces quatre bases automatiques**, aucune marge commune ne satisfait simultanément les pauses présumées et l'absence de coupure des continus. Une base de bruit seul corrigée après écoute peut changer ce résultat.

La différence entre les bases provisoires des longs (environ −64 dBFS) et des continus (environ −35 dBFS) montre que la plage la plus calme des continus ne peut être assimilée au bruit de fond sans écoute. Les faibles niveaux dans les continus peuvent correspondre à une parole atténuée. Les coupures qui en résultent sont **candidates à examiner**, pas des fausses pauses prouvées. La grille +6/+9/+12, appliquée à ces bases provisoires, ne permet pas actuellement de détecter la pause française F2 tout en conservant zéro coupure dans les 60 s continus.

## Début anglophone du FR avec pauses

Le texte humain approuvé du WAV FR commence en anglais (`Thank you for one minute. Mr. Riviere.`), puis passe en français (`Merci, Madame la Presidente.`). Le relevé acoustique exploratoire situe une longue baisse d'énergie vers 4,44–6,92 s ; le premier mot français reconnu par Parakeet est proposé à 6,72 s. Ces temps sont automatiques. Si F1 confirme la transition et la pause, le scénario **français avec pauses** peut utiliser la portion française d'environ 6,9–65,02 s, soit environ 58,1 s et plusieurs pauses candidates ultérieures, en conservant le WAV complet et sa référence intégrale approuvée. L'extrait utilisé devra porter sa borne de départ et l'offset temporel correspondant ; aucune modification de la référence n'est nécessaire. L'aptitude définitive dépend des étiquettes F1 et F2.

## Suite minimale

Après les huit étiquettes, retenir uniquement une plage de bruit seul réellement reconnue dans chaque source. Si un continu n'en contient pas, chercher une courte plage adjacente dans le même OGG et faire confirmer sa comparabilité acoustique ; sinon signaler que la méthode de seuil relatif au bruit est inapplicable à ce fichier. Recalculer les percentiles 95 et les marges +6/+9/+12 sur ces plages, puis contrôler automatiquement les quatre WAV entiers, notamment les deux 60 s. Choisir une marge seulement si les pauses F2/E1 sont détectées et aucune hésitation ou parole faible étiquetée ne devient une pause ; sinon conclure à l'échec du seuil énergétique. Le dépôt possède déjà `webrtc-vad` dans `crates/whisper-adapters/src/vad.rs` : une évaluation ultérieure de son intérêt serait possible sans installer une dépendance, mais son usage pour Parakeet demanderait une décision technique distincte. Aucun essai A/B ni FEAS n'est inclus ici.
