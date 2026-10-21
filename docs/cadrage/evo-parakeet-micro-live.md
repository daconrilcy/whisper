Avancée : le parcours microphone Parakeet est cadré à partir du choix de blocs confirmés pendant la capture.
Blocage : la stabilité des mots aux frontières, le délai du premier bloc et le débit GPU ne sont pas démontrés sur ce parcours.
Décision : garder Parakeet micro indisponible ; soumettre la faisabilité et le design à une décision ultérieure avant tout lot de code.

# Évolution Parakeet — microphone avec blocs confirmés

**État au 21 octobre 2026 :** cadrage documentaire, DESIGN BLOCKED sur les risques PM-01 et PM-02 ; plan d'implémentation non prêt. Ce document ne donne aucune autorisation de code, de test natif, d'installation ou de push.

**Autorité et base.** L'utilisateur a demandé Parakeet sur le microphone, a choisi explicitement « Blocs pendant la capture », puis a autorisé ce cadrage dans la présente session. Base inspectée : `evo-startup-perf` à `9e755120`. Les dossiers non suivis `models/` et `sound example/` sont préservés. Ce document actualise pour ce seul périmètre le report L4 décrit dans [evo-parakeet.md](evo-parakeet.md) et [note-parakeet-micro-l4.md](note-parakeet-micro-l4.md) ; il ne réinterprète pas leurs essais sur fichiers comme une preuve micro.

## 1. Besoin et limites

Avec Parakeet choisi et disponible, l'utilisateur démarre le microphone Windows, voit des blocs **définitifs avant l'arrêt**, puis obtient TXT, SRT et WAV complets et de même nom de base. Modèle, calcul demandé et calcul effectif restent identifiables. Le choix de microphone et les règles existantes en cas d'indisponibilité sont conservés. CPU, GPU manuel et Auto restent des parcours distincts ; aucun changement silencieux vers Turbo ni vers une autre source micro.

Une transcription affichée seulement après l'arrêt ne satisfait pas ce besoin. Nemotron 3.5, raccourci clavier global, injection de texte dans une autre application, acquisition de modèle, nouvel installateur, pause/reprise, changement de modèle en séance et reprise après panne sont hors périmètre. Ni les segments de 5 à 10 s, ni une VAD, ni les cibles proposées pour l'évolution de démarrage de l'interface ne sont acceptés ici comme réglages ou seuils Parakeet.

La [documentation NeMo-Speech.cpp v0.2.0](https://github.com/NVIDIA/NeMo-Speech.cpp/blob/v0.2.0/docs/asr/models.md) décrit Parakeet TDT v3 comme un modèle à énoncé complet et indique que ses requêtes streaming sont rejetées. Le worker actuel charge `nemo_speech_asr_recognize_f32` dans [native_ffi.rs](../../crates/whisper-worker-parakeet-cpu/src/native_ffi.rs), mais son [IPC](../../crates/whisper-core/src/parakeet_ipc.rs) reçoit une source par chemin de fichier : l'API native en mémoire ne constitue pas un transport micro déjà raccordé. L'[interface](../../crates/whisper-desktop/src/ui.rs) refuse aujourd'hui Parakeet live. La note L4 rapporte une erreur lexicale à une frontière de 8 s déplacée de 0,2 s. Le choix d'un découpage court reste donc une hypothèse à vérifier.

## 2. Exigences et critères d'acceptation

Les critères ci-dessous sont **NOT RUN** pour Parakeet micro. Un banc injecté et une qualification dans la fenêtre Windows apportent des preuves différentes ; la recette dans la fenêtre appartient à l'utilisateur.

| ID | Exigence et résultat observable |
| --- | --- |
| PM-01 | **Admission.** Le modèle, le worker, la source et le profil live sont admissibles avant de proposer le départ. Deux clics Démarrer ne créent qu'une séance ; un profil absent ou non qualifié est refusé avec cause, sans capture fictive ni substitution vers Turbo. La qualification import ne rend pas le live admissible. |
| PM-02 | **Texte pendant la capture.** Sur des phrases FR/EN avec pauses, reprise après silence, parole continue, bruit et mots/nombres aux frontières, les blocs confirmés apparaissent avant Arrêter, restent immuables et ordonnés ; aucun mot annoté n'est omis ou répété du fait du découpage. Une référence relue humainement est nécessaire pour juger la qualité, sans assimiler tout écart avec une passe entière à une faute de frontière. |
| PM-03 | **Délai et débit.** Sur GPU qualifié, avec parole commençant rapidement après le clic, le premier bloc confirmé est visible en moins de 15 s, à froid et avec réemploi compatible. Mesurer séparément clic, admission de parole, moteur prêt, fenêtre disponible, inférence, confirmation et affichage. Couvrir phrase puis pause et parole continue au-delà de 15 s. Le GPU traite l'audio plus vite que sa durée sans retard qui croît durablement ; comparer sa qualité à celle du CPU sans inventer de seuil lexical. Aucun seuil de 15 s n'est transféré au CPU. |
| PM-04 | **Audio et fin de séance.** La capture continue pendant l'inférence ; tout audio annoncé admis est conservé. Un Arrêter unique, y compris avant que le moteur soit prêt ou pendant une inférence, ferme l'admission puis traite le reliquat. La fermeture normale suit ce parcours. TXT inclut la dernière parole ; SRT suit le temps de capture, silences compris ; WAV couvre l'audio admis. Succès seulement après vérification des trois sorties. |
| PM-05 | **Erreurs.** Déconnexion du micro, saturation, worker bloqué ou défaillant, écriture refusée, export partiel et résultat tardif d'une séance ancienne ont une cause et un état incomplet visibles. Aucun résultat obsolète, autre micro ou autre modèle ne contamine la séance. Les ressources lisibles sont conservées. |
| PM-06 | **Calcul.** CPU et GPU manuels restent fixes pendant la séance. Auto privilégie un GPU Parakeet admissible et, si son initialisation échoue avant le traitement audio, n'essaie le CPU qu'une fois avec cause et calcul effectif visibles ; aucun repli après le début du traitement. La préférence Auto et le modèle Parakeet ne changent pas. |
| PM-07 | **Source micro.** Préférence explicite et défaut Windows sont restitués comme aujourd'hui ; si la source préférée manque à la réouverture, l'usage du défaut est expliqué sans effacer son identifiant. Si la source disparaît pendant la séance, aucune autre n'est choisie automatiquement. |
| PM-08 | **Non-régression.** Imports WAV et MP3 Parakeet sur les calculs admissibles, micro Turbo, deux séances distinctes, préférences et exclusion réciproque micro/import gardent leurs comportements et sorties attendus. |

## 3. Parcours, états et engagements

Parcours nominal : choisir Parakeet et une source utilisables → Démarrer → capturer et conserver l'audio → inférer des segments complets sans interrompre la capture → confirmer seulement un préfixe stable → Arrêter ou fermer normalement → traiter le reliquat → vérifier TXT/SRT/WAV → Terminé. États visibles à préciser : préparation, capture avec éventuelle préparation ou transcription simultanée, arrêt demandé, traitement restant, export, complet, incomplet. Ces libellés ne prescrivent pas des types Rust.

Le domaine porte une identité de séance, une génération, une tentative moteur et une configuration figée. Trois bornes ont des sens différents :

- **A, audio admis :** avance seulement après conservation réussie dans un support relisible ; une capture reçue mais non conservée ne peut pas être déclarée admise.
- **C, texte confirmé :** avance monotoniquement avec `0 ≤ C ≤ A`. Le préfixe affiché devient immuable ; une fin d'inférence ne suffit pas à confirmer une frontière.
- **F, fin d'admission :** fixée une seule fois à l'arrêt. Le succès exige couverture de `[0,F)`, sans trou connu, et vérification des trois fichiers. Un silence sans mot reste de l'audio couvert.

Les résultats portant une autre séance, génération, tentative ou plage sont rejetés avant affichage et publication. Un arrêt répété ne crée ni second drain ni seconde publication. Une erreur terminale ferme l'admission et laisse un état incomplet explicite ; elle ne transforme pas une sortie partielle en succès. Le SRT utilise les positions d'échantillons de la capture. Aucune reprise après arrêt ou panne du processus n'est ajoutée.

## 4. Design candidat et preuves manquantes

Le cœur possède les règles de séance, de confirmation, d'arrêt et les ports sans dépendance OS ; les adaptateurs possèdent la capture, le support audio, l'ordonnancement, le transport et le worker ; le desktop présente les états et le root assemble les ressources. Le worker possède le recognizer et les appels natifs bloquants. Le [DirectLive Turbo](../../crates/whisper-adapters/src/direct_live.rs) fournit un précédent pour le backlog WAV et le drain, mais ses fenêtres et son seuil de silence ne sont pas qualifiés pour Parakeet. Les profils d'import Parakeet CPU/GPU ne valent pas qualification live ; un profil micro dédié et ses capacités propres sont nécessaires.

**Option candidate de première étude :** appels hors ligne répétés sur des fenêtres qui se recouvrent, suffixe non confirmé et confirmation d'un préfixe stable entre hypothèses compatibles. Une VAD peut aider à choisir les moments de traitement, mais ne garantit ni la stabilité à une coupe ni le délai en parole continue. Une seule inférence native en vol et une file représentée par plages éviteraient des copies audio RAM sans borne. Les tailles de fenêtre, recouvrement, zone retenue et règle d'alignement restent à mesurer ; aucune valeur n'est acceptée par ce document.

**Transport à comparer :** snapshots WAV immuables et identifiés par plage, compatibles avec l'IPC actuel à chemin, ou extension bornée du protocole Rust pour passer le PCM. Les snapshots ont un coût d'écriture et de latence à mesurer ; l'extension IPC modifie un contrat et demande sa propre revue. Le choix ne supprime pas le WAV final. La RAM doit être bornée ; le backlog disque peut croître avec la séance et impose une politique de saturation et de capacité WAV avant dépassement. Aucun nouveau runtime, DLL, appel FFI ou exception `unsafe` n'est approuvé.

| Risque | Impact produit | Preuve nécessaire | État |
| --- | --- | --- | --- |
| PM-01 — frontière | Mot coupé, omis, répété ou texte confirmé révisé | Corpus FR/EN annoté avec pauses, bruit et parole continue ; comparaison des hypothèses, offsets et préfixes confirmés | Ouvert ; essais fichier antérieurs insuffisants |
| PM-02 — latence et débit | Premier bloc ≥15 s ou retard qui augmente | Mesures à froid/réemploi, GPU/CPU, séparation capture → Ready → fenêtre → inférence → confirmation → affichage, charge soutenue | Ouvert ; aucun essai micro Parakeet |
| PM-03 — conservation et arrêt | Audio annoncé admis perdu, dernière phrase absente, faux succès | Injection d'arrêts/erreurs, couverture `[0,F)`, fichiers vérifiés, recette Windows | Ouvert ; contrats proposés |

Une étude de faisabilité bornée, **à autoriser séparément**, devra utiliser le modèle, les workers et les runtimes exacts ainsi qu'un corpus FR/EN annoté. Le dossier historique `target/parakeet-feasibility/spike02/` est absent dans cette base ; `sound example/spkr0.wav` existe mais sa langue et son contenu ne sont pas qualifiés. L'étude comparera un petit nombre de configurations, injectera l'audio au rythme réel et mesurera préfixes, frontières, délai et retard. Un événement confirmé dans un banc ne démontre pas le délai clic → bloc visible dans la fenêtre ; cette validation reste distincte. Toute campagne future doit respecter les racines, gardes d'écriture, variables Cargo et minimum de 30 Gio libres définis dans `AGENTS.md`. Aucune étude n'est exécutée par ce cadrage.

## 5. Traçabilité, suite et readiness

| Besoin | Critères | Contrat ou composant | Risque principal |
| --- | --- | --- | --- |
| Parakeet sélectionnable sans substitution | PM-01, PM-06 | Admission et profil live distinct | Capacité exacte non qualifiée |
| Blocs définitifs pendant capture | PM-02, PM-03 | Borne C ; segmentation/confirmation | PM-01, PM-02 |
| Audio complet, arrêt et sorties | PM-04, PM-05 | Bornes A/F ; drain/publication | PM-03 |
| Choix micro et continuité | PM-07, PM-08 | Capture Windows et chemins existants | Non-régression |

**Feuille de route conditionnelle, sans lot autorisé :** (1) apporter corpus/ressources et qualifier la faisabilité ; (2) fermer les choix de confirmation, transport et bornes par revue du design ; (3) détailler et faire revoir un plan de lot avec chemins exacts ; (4) implémenter admission/profil et contrats ; (5) raccorder capture, segmentation et worker ; (6) confirmer les blocs et finaliser TXT/SRT/WAV ; (7) raccorder l'UI ; (8) vérifier CPU/GPU/Auto, erreurs et non-régressions ; (9) remettre le profil réellement construit pour recette manuelle Windows. Chaque étape de code exige plus tard un plan accepté, son périmètre et un accord précis dans la session concernée.

**Readiness :** le périmètre documentaire est défini. Le design reste **BLOCKED** pour une activation de Parakeet micro tant que PM-01 et PM-02 ne sont pas démontrés et revus ; aucun plan n'est `READY`, aucun code n'est autorisé. Si le délai GPU ou la stabilité ne sont pas atteints, maintenir le profil indisponible et soumettre à l'utilisateur les résultats et les options de compromis. La qualification dans la fenêtre Windows restera celle de l'utilisateur.
