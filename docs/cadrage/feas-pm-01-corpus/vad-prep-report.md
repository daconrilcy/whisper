Avancée : le VAD WebRTC existant a été évalué hors produit sur les quatre WAV complets, pour quatre modes et trois durées de trame ; les 48 flux de décisions brutes sont conservés.
Blocage : la règle stricte de 400 ms manque la vraie pause anglaise E1 dans toutes les configurations ; une tolérance d'une trame reste une hypothèse non qualifiée. Un rejeu surveillé ultérieur a détecté une écriture hors racine non attribuée et la campagne a été arrêtée.
Décision : PARTIAL pour l'intérêt du VAD ; aucun profil ni règle de coupe n'est retenu, le protocole FEAS-PM-01 et le code produit restent inchangés.

# Étude acoustique préparatoire du VAD WebRTC

## Source et méthode

L'adaptateur [`vad.rs`](../../../crates/whisper-adapters/src/vad.rs) utilise `webrtc-vad = 0.4.0`, PCM mono 16 kHz, trames de **320 échantillons (20 ms)** et mode **LowBitrate (1)**. Il renvoie une décision vocale booléenne par trame et n'applique pas de lissage. [`worker_ipc.rs`](../../../crates/whisper-adapters/src/worker_ipc.rs) crée une instance VAD pour la capture, la fait avancer chronologiquement et compte les échantillons classés parole ; il ne convertit pas ces décisions en pauses ou points de découpage. La bibliothèque présente les modes `Quality (0)`, `LowBitrate (1)`, `Aggressive (2)` et `VeryAggressive (3)` ; elle accepte des trames de **10, 20 ou 30 ms** à 16 kHz. Les trames de 10/30 ms sont une comparaison hors produit : l'adaptateur actuel reste fixé à 20 ms.

Les quatre WAV et leurs SHA-256 sont ceux du [manifeste approuvé](manifest.json). Chaque couple WAV × mode × durée a été traité depuis le début du WAV avec **une instance VAD neuve**, sans réinitialisation dans le fichier et sans extraction isolée des huit zones. Le dernier cadre incomplet à 30 ms a été complété par des zéros, puis tous les intervalles dérivés ont été limités à la durée réelle du WAV. Aucune inférence Parakeet ni stratégie A/B n'a été exécutée.

Les [48 décisions brutes](vad-prep/vad_raw.json) sont des suites `1` (parole) / `0` (non-parole), une position par trame, avec mode, durée, nombre d'échantillons et empreinte du WAV. Le [calcul dérivé](vad-prep/vad_analysis.json) distingue :

1. **Décision VAD** : chaque trame `0/1` brute, état interne conservé.
2. **Pause candidate stricte** : suite continue de `0` d'au moins **400 ms**.
3. **Coupe proposée** : milieu de chaque pause candidate, en retenant chronologiquement les coupes espacées d'au moins **1 s**. Aucune coupe n'a été éliminée par cette règle d'espacement dans les 48 passages.

Les [sources du banc](vad-prep/vad_probe.rs), de [l'exécution](vad-prep/run_probe.py) et de [l'analyse](vad-prep/analyze_probe.py) sont conservées. Le banc a été compilé directement avec `rustc 1.98.1` et la bibliothèque `webrtc-vad 0.4.0` déjà compilée dans `target/parakeet-tests/cargo-shared/` ; aucun Cargo, téléchargement ou installation n'a été lancé. Les sorties brutes et dérivées ont les SHA-256 respectifs `7f2749120f58265f376d537a8db3a64a8fcdb132414639f4a9056828fb65091a` et `a55f6fddbe1a01a376d5a5978c789a4ceeb62623565d7e885063ca0f0ea04bf7`. Une seconde compilation surveillée a reproduit les **48 suites de décisions à l'identique** ; son binaire avait une empreinte différente, enregistrée dans sa sortie propre. Pour un futur rejeu, les scripts archivés doivent être copiés dans la racine de lot autorisée sous `target/parakeet-tests/`, puis exécutés avec les sorties de compilation dans `cargo-shared` et les variables temporaires confinées ; **le rejeu reste suspendu à l'arbitrage de l'incident ci-dessous**.

## Détection stricte de 400 ms

Le tableau compte toutes les pauses candidates sur chaque WAV complet, **avant** vérification humaine des candidats non étiquetés. L'espacement de 1 s ne change aucun de ces nombres.

| Trame | Mode | FR avec pauses | EN avec pauses | FR continu 60 s | EN continu 60 s |
| --- | --- | ---: | ---: | ---: | ---: |
| 10 ms | Quality | 4 | 0 | 0 | 0 |
| 10 ms | LowBitrate | 5 | 0 | 0 | 0 |
| 10 ms | Aggressive | 6 | 0 | 0 | 0 |
| 10 ms | VeryAggressive | 11 | 0 | 0 | 0 |
| **20 ms** | **Quality** | **3** | **0** | **0** | **0** |
| **20 ms** | **LowBitrate actuel** | **5** | **0** | **0** | **0** |
| **20 ms** | **Aggressive** | **5** | **0** | **0** | **0** |
| **20 ms** | **VeryAggressive** | **10** | **0** | **0** | **0** |
| 30 ms | Quality | 2 | 0 | 0 | 0 |
| 30 ms | LowBitrate | 2 | 0 | 0 | 0 |
| 30 ms | Aggressive | 3 | 0 | 0 | 0 |
| 30 ms | VeryAggressive | 10 | 0 | 0 | 0 |

Au profil actuellement employé par l'application (LowBitrate, 20 ms), F1 est retrouvé à **4,78–6,10 s** et F2 à **13,00–13,40 s**. La vraie pause anglaise E1 n'est retrouvée par **aucune** configuration stricte. Sur E1, la plus longue suite brute de non-parole va de **240 ms** (Quality/LowBitrate 20 ms) à **380 ms** (VeryAggressive 20 ms), sous les 400 ms requis. Les deux continus complets ont **zéro** pause candidate dans les douze configurations. Les courtes décisions non vocales sur FC2 (emphase) et EC3 (sans pause) restent sous 400 ms ; leur classification vocale n'est donc pas équivalente à une coupe erronée. FC1, EC1 et EC2 contiennent des respirations ou micro-pauses humainement ambiguës : aucune conclusion de conformité n'est inférée de leur absence de coupe.

## Sensibilité à une tolérance de trame

À titre de **sensibilité seulement**, le calcul dérivé étend chaque suite brute de `0` d'**une trame de chaque côté**, sans fusionner les suites, avant de comparer sa durée à 400 ms. Ce n'est pas une règle approuvée : elle peut transformer **380 ms de non-parole observée** en une pause candidate de **420 ms** et doit donc être distinguée du critère strict.

À 20 ms et en mode `VeryAggressive`, cette tolérance proposerait F1 et F2, puis **E1 à 12,26–12,68 s**, sans coupe dans les deux continus. Elle proposerait aussi une zone anglaise **63,78–64,18 s**, non étiquetée humainement, et dix zones françaises dont seules F1 et F2 ont une étiquette précise. Les huit autres zones françaises à vérifier sont approximativement **16,46–16,92 ; 18,28–18,92 ; 25,34–25,78 ; 26,92–27,48 ; 32,18–32,82 ; 38,38–39,12 ; 49,58–50,02 ; 63,32–63,92 s**. L'espacement de 1 s ne supprime aucune de ces coupes. Les essais de 10 et 30 ms donnent également E1 avec ce mode et cette tolérance, mais ajoutent respectivement 11 et 12 candidats français : ils n'apportent pas de justification pour changer la trame du produit.

Il n'y a **aucune zone de coupe ≥400 ms à faire relire dans les deux WAV continus** pour les règles examinées. Les zones non étiquetées des deux WAV avec pauses devraient être contrôlées avant d'adopter la variante tolérante. Aucun réglage n'a été choisi sur les seuls huit extraits humains.

## Verdict et éventuelle révision documentaire

**PARTIAL.** Le VAD maintient la capture continue sans coupes de 400 ms sur les deux minutes testées et retrouve deux des trois pauses humaines au profil existant. En revanche, il **échoue à E1** sous la règle stricte dans les douze configurations. La variante avec tolérance d'une trame est prometteuse, mais elle modifie la signification du seuil de 400 ms et introduit des coupes non validées. Un `PASS` et une qualification du détecteur seraient prématurés.

Si une évaluation ultérieure confirme cette variante sur les zones non étiquetées et sur un contrôle indépendant, une modification **proposée, non appliquée** du protocole serait de remplacer la calibration « 95e percentile du bruit seul + marge RMS » par un prérequis VAD explicite : mode, durée de trame, état conservé par fichier, définition de la pause (y compris une éventuelle tolérance), règle du milieu de pause, espacement de 1 s et absence de coupe inadmissible sur les deux continus. La décision devra traiter E1 et la zone anglaise à 64 s sans choisir les paramètres uniquement pour ce corpus. Le texte de `evo-parakeet-micro-live.md` n'est pas modifié par cette proposition ; FEAS-PM-01 reste interdit et demanderait son autorisation distincte.

## Réserve de confinement du dernier rejeu

Une compilation et un rejeu surveillés avec la [garde locale](vad-prep/guarded_replay.ps1) ont observé **39 événements d'écriture dans les racines autorisées et aucun hors racine** ; les 48 décisions étaient identiques au premier passage. Un rejeu ultérieur, entrepris pour compléter les métadonnées de provenance, a détecté vers **15:40 UTC le 21 octobre 2026** une écriture à `C:\Users\cyril\AppData\Local\Temp\etilqs_CEMNtn3OliYXaHw`. Le moniteur de fichiers ne permet pas de l'attribuer au banc ou à un autre processus. Conformément à `AGENTS.md`, la campagne a été arrêtée ; ce fichier n'a pas été effacé ou restauré, et les sorties de ce rejeu ne sont pas retenues comme preuve qualifiée. Un arbitrage est requis avant toute nouvelle exécution de campagne. Les fichiers bruts conservés ci-dessus précèdent ce rejeu et leurs 48 suites ont été confirmées identiques par le passage surveillé sans anomalie.
