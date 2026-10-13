# Évolution Whisper — micro en direct (proposition, 13 octobre 2026)

Avancée : parcours et réemploi du code identifiés sur la branche `evo-micro-direct` issue de `main` (`dc51f4a`, tag MVP ancêtre `v0.1.0-mvp`).
Blocage : critères de délai et comportement du texte/fermeture à décider ; aucun essai micro dans la fenêtre n'a été fait.
Décision : valider les choix produit en fin de document avant d'autoriser la réalisation.

## Besoin et périmètre

Dans Whisper Windows, l'utilisateur démarre le micro, parle et lit le texte pendant la capture ; à l'arrêt, l'application termine le traitement et fournit un TXT et un SRT. Calcul CPU et modèle déjà local, comme pour le MVP. Le résultat doit comprendre la dernière parole captée avant l'arrêt. Le parcours WAV/MP3 du MVP reste disponible. Les champs « critère de réussite » et « hors périmètre » de la demande étant laissés ouverts, les précisions ci-dessous sont **proposées**, pas acceptées. Différés proposés : GPU/Auto, tray, reprise après panne, sortie MP3, pause/reprise et capture audio système. Aucun installateur ou téléchargement n'entre dans ce lot. D-WHISPER-30 et P-WHISPER-31 ont été consultés comme inspiration, sans reprendre leur V1 ni leurs seuils.

## Exigences et acceptation proposées

Tous les AC sont à qualifier sur la fenêtre Windows avec micro et modèle locaux ; les tests automatiques actuels ne les valident pas.

| ID | Parcours et constat vérifiable |
|---|---|
| AC1 | **Démarrer** : avec micro et modèle disponibles, cliquer Démarrer ; l'état d'écoute apparaît et une phrase parlée donne du texte. Un second démarrage ne crée pas deux sessions. |
| AC2 | **Lire pendant la capture** : prononcer deux phrases séparées par un silence ; voir des ajouts dans l'ordre **avant** Arrêter, sans doublon. Filmer/chronométrer parole et affichage pour caractériser le délai CPU, sans seuil encore accepté. |
| AC3 | **Arrêter et enregistrer** : prononcer une dernière phrase, cliquer Arrêter ; voir la finalisation, puis ouvrir les deux fichiers annoncés. Le TXT contient les passages confirmés et la dernière phrase ; le SRT a des intervalles croissants sur le temps de capture, silences compris. |
| AC4 | **Échouer clairement** : micro absent, modèle absent ou dossier inaccessible donnent une cause et une action visibles ; aucun succès ne masque un export incomplet. |
| AC5 | **Cycle de vie** : fermer pendant capture et demander un import concurrent ; constater la politique décidée, sans capture cachée ni texte présenté comme enregistré. |
| AC6 | **Préserver le MVP** : deux séances micro distinctes produisent deux résultats distincts ; WAV et MP3 continuent chacun à produire TXT/SRT sur CPU. |

Scénario de réussite proposé : séance courte avec parole, silence, reprise et phrase terminale ; texte observable avant l'arrêt, puis TXT/SRT lisibles et cohérents avec la session. Langue, durée, qualité de reconnaissance et délai maximal restent à fixer seulement si requis par l'usage.

## Domaine, architecture et réemploi

Une session porte une identité, une configuration CPU figée et des segments horodatés. États métier proposés : `Prêt → Préparation → Capture → Vidage → Export → Terminé`, avec `Erreur` et action explicite. Arrêter ferme d'abord l'admission audio, traite le reliquat puis exporte ; un succès exige les deux fichiers. Les segments d'une autre session ou déjà acceptés ne doivent pas être ajoutés. Une perte de capture ou un export partiel ne vaut jamais résultat complet. La fermeture forcée du processus peut perdre le non-exporté : la reprise après panne reste différée.

Le code offre déjà `LiveRequest`, états/événements et texte provisoire dans `whisper-core` ; capture micro CPAL, conversion mono 16 kHz et file bornée dans `whisper-adapters` ; traitement par fenêtres dans le worker CPU ; boutons et affichage micro dans `whisper-desktop`, masqués par `IMPORT_CPU_MVP`. La publication live actuelle entraîne aussi MP3, journalisation et reprise historiques : réafficher le bouton seul ne livre pas le périmètre TXT/SRT. Architecture proposée : cœur sans dépendance OS/UI pour identités et transitions ; application pour arrêt et coordination ; adaptateurs pour micro, worker et écriture ; UI pour états et texte. Le calcul reste hors du fil d'interface. Saturation ou erreur ferme la capture avec message et ne publie pas de succès ; finalisation attend le dernier fragment. Cadence CPU et arrêt réel sont à vérifier sur le matériel cible.

## Plan proposé (réalisation ultérieure, 8 tâches)

1. Fixer les choix ouverts et le protocole d'essai micro/modèle local — AC1–AC5.
2. Définir états, identité des segments et contrat d'arrêt CPU dans le cœur — AC1–AC3, AC5.
3. Adapter la capture/conversion et la file bornée pour la session simple — AC1–AC2, AC4.
4. Raccorder le worker CPU aux fenêtres et aux résultats ordonnés — AC2–AC3.
5. Détacher l'export TXT/SRT du MP3 et de la reprise différés — AC3–AC4.
6. Exposer démarrage, texte, arrêt, attente et erreurs dans la fenêtre — AC1–AC5.
7. Tester contrats de fin de capture, doublons, erreurs et non-régression import — AC1–AC6.
8. Qualifier dans la fenêtre avec micro/modèle locaux et deux séances ; relever le délai observé — AC1–AC6.

## Décisions attendues

1. **Q1, structure, avant tâches 2/6, AC2–AC3 — Texte** : passages confirmés ajoutés (recommandé, stables) ou provisoire révisable (plus rapide, corrections à gérer).
2. **Q2, structure, avant tâches 3/8, AC2 — Délai** : accepter le retard CPU visible (recommandé) ou fixer un délai maximal avec scénario et seuil à démontrer ; préciser langue, durée et qualité si elles conditionnent la réussite.
3. **Q3, structure, avant tâches 2/6, AC3–AC5 — Fermeture/concurrence** : fermer = arrêter puis finaliser, en gardant la fenêtre si échec ; import et micro exclusifs (recommandé), ou annulation/perte et calcul parallèle.
4. **Q4, qualification, avant tâche 8, AC1–AC4 — Prérequis** : confirmer un micro Windows accessible et le modèle local du MVP ; si absents, choisir comment les obtenir. Toute installation ou acquisition fera l'objet d'un accord précis distinct.

Sources : demande de cette session, `AGENTS.md`, `docs/cadrage/bilan-mvp.md`, inspection des crates ; contributions en lecture seule des spécialistes besoin, exigences, domaine et architecture. Statut : **cadrage proposé**, code micro non qualifié, réalisation non autorisée.
