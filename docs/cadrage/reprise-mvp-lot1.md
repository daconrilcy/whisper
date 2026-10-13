# Reprise MVP — lot 1 (13 octobre 2026)

Avancée : les critères et les deux fichiers audio de référence sont fixés ; la fenêtre Whisper a été observée.
Blocage : le contrôle de cette fenêtre refuse les clics et la saisie, donc l'import depuis l'interface reste non vérifié.
Décision : autoriser seulement les corrections fondées sur les écarts visibles, puis refaire l'essai dans la fenêtre.

## Critères d'acceptation et références (tâche 1)

- **M1 — Entrée** : chacun des deux fichiers ci-dessous peut être choisi dans la fenêtre et mène à un état terminé ; un fichier non pris en charge donne une erreur claire.
- **M2 — CPU** : le mode sélectionné et le moteur effectif sont CPU pour WAV et MP3, sans passage implicite par GPU/Auto.
- **M3 — Sorties** : chaque import terminé donne un TXT lisible non vide et un SRT non vide avec séquences numérotées, temps croissants et texte correspondant à la parole. La voix du WAV dit notamment « faire un point sur l'état du transcript » ; l'essai CPU existant produit trois sous-titres sur environ 15 secondes. Comparer le MP3 à l'écoute, sans imposer une transcription mot à mot.
- **M4 — Intégrité et erreurs** : les deux sources restent inchangées ; échec ou interruption signalé clairement, sans état terminé ni fichier partiel/corrompu présenté comme résultat. Aucune reprise après panne n'entre dans ce critère.
- **M5 — Fenêtre utilisable** : source, chemin du modèle local et dossier de sortie sont compréhensibles ; le résultat est retrouvable depuis le dossier affiché. Modèle absent : erreur claire, aucun téléchargement automatique.

Fichiers de référence existants : WAV `C:\WhisperLive\micro_test.wav` (466 988 octets) ; MP3 `C:\WhisperLive\L04-campaign\inputs\micro_test.mp3` (117 837 octets). Le modèle local présent est `ggml-large-v3-turbo.bin` dans `%LOCALAPPDATA%\Programs\WhisperBuildDeps\models\`. Les essais d'intégration CPU de la reprise précédente ont réussi sur ces fichiers ; ils ne remplacent pas l'essai dans la fenêtre.

## Vérification de la fenêtre (tâches 3 et 4, lecture seule)

`target/debug/whisper-desktop.exe` s'ouvre. La fenêtre affiche les champs source, modèle et dossier, mais le champ source est vide et l'import est désactivé. Le mode initial affiché est **Automatique** ; le bouton **Démarrer le micro** et les options live restent visibles. Une ancienne entrée d'import interrompu et un historique préexistant s'affichent depuis le dossier par défaut. Les tentatives de clic et de saisie via le contrôle Windows échouent avec `GetCursorPos failed: Accès refusé (0x80070005)` ; aucun nouvel import n'a été lancé, et TXT/SRT, chronologie et intégrité n'ont donc pas pu être vérifiés *dans la fenêtre*. Ce refus est une limite du contrôle de l'interface dans cette session, pas un défaut produit établi.

## Suite proposée, sans exécution

**Lot 2 limité à l'écart visible (tâche 2)** : présélectionner CPU et masquer ou désactiver uniquement les commandes GPU, Auto et micro qui gênent l'import CPU. Conserver leur code dans le dépôt. Les tâches 5 et 6 attendent un essai interactif pour constater leurs écarts éventuels ; aucune correction n'est présumée.

**Dossier de sortie — décision demandée pour la tâche 5** : option A, conserver `Documents\WhisperTranscriptions` par défaut en affichant le chemin et en laissant le modifier (**recommandée**, parcours immédiat sur ce PC) ; option B, exiger un dossier choisi avant chaque import (plus explicite, mais plus d'étapes). Le modèle reste local, avec son emplacement documenté et une erreur claire s'il manque.

**Tâche 7 à arbitrer** : `target/` est ajouté au `.gitignore`. `crates/whisper-adapters/archive/` contient un journal de diagnostic généré : recommandation, le garder hors Git et le nettoyer seulement après décision. `--help` est un dossier racine vide : recommandation, le supprimer après votre décision. Aucun des deux n'est touché dans ce lot.

## Protocole final pour votre essai (tâche 8, non exécutée)

1. Ouvrir Whisper et vérifier que le mode affiché est CPU.
2. Vérifier le chemin du modèle local et le dossier de sortie affichés.
3. Entrer le WAV de référence, puis lancer l'import.
4. Attendre « terminé » et ouvrir le TXT et le SRT produits.
5. Vérifier la phrase audible et l'ordre des sous-titres.
6. Faire la même chose avec le MP3 de référence.
7. Vérifier que les deux fichiers audio d'origine sont toujours intacts.
8. Essayer un chemin de modèle absent : relever le message et vérifier l'absence de faux résultat.
