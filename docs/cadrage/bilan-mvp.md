# Bilan du MVP Whisper — 13 octobre 2026

Avancée : le parcours WAV et MP3 vers TXT et SRT sur CPU est validé dans la fenêtre Windows (`v0.1.0-mvp`).
Blocage : aucun pour le MVP sur ce PC avec le modèle local présent.
Décision : choisir une fonction différée seulement si son usage justifie un prochain lot.

## Ce qui fonctionne

- Dans la fenêtre, l'import d'un WAV ou d'un MP3 utilise le mode CPU. La file affiche les imports en attente ; l'utilisateur lance leur traitement avec « Traiter ».
- Un import terminé produit un TXT lisible et un SRT horodaté dans le dossier de sortie affiché, `Documents\WhisperTranscriptions` par défaut et modifiable. Les sources audio restent intactes.
- Le chemin du modèle local est visible ; son absence donne un message compréhensible. Les erreurs et interruptions ne présentent pas de fichier partiel ou corrompu comme résultat terminé.
- L'utilisateur a confirmé les huit étapes du protocole dans la fenêtre, dont les deux formats, les sorties et le cas du modèle absent. Compilation et tests existants passent hors ligne.

## Limites connues

- Le modèle doit déjà être installé localement et son chemin renseigné ; aucun téléchargement automatique ni installateur complet n'est fourni.
- Les imports en file exigent l'action « Traiter ». La reprise automatique après une panne n'est pas couverte.
- L'état `IMPLEMENTATION` de `state.json` et les lots L04–L07 concernent la V1 historique plus large ; le code L04 partiel reste dans le dépôt sans être qualifié comme fonction du MVP.

## Fonctions différées — ordre de valeur probable à confirmer par l'usage

1. **Micro en direct** : transcription sans préparer un fichier audio.
2. **GPU et mode Auto** : réduire l'attente sur les machines compatibles.
3. **Reprise après panne** : reprendre un travail interrompu lors de longues transcriptions.
4. **Tray** : garder l'application accessible en arrière-plan.
5. **Sortie MP3** : conserver ou partager un audio produit par les parcours futurs.

Les réglages système et l'installation complète prévus dans l'ancien plan restent également hors MVP.
