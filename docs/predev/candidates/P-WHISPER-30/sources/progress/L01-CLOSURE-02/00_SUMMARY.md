# L-WHISPER-01 — reprise de clôture (2026-10-06)

Candidat code L01 : SHA-256 canonique `ca361b9890ece948b804d2692625e38d0e2138493633d87014972d8d9343c5f5` sur les 21 chemins autorisés de `state.json`. Baseline : `3f7361749944e5e68a8da506ccd429f68f6bf8e5`. Les résultats d’anciennes empreintes sont historiques; seule la revue/validation finale de cette empreinte compte.

La preuve UI du vrai DesktopApp est conservée : `ui-cancel.png` montre l’import WAV D19, Stop par le bouton, l’état terminal `Cancelled`, source et archive pending conservées, historique `Récupérable`, aucune transcription publiée. `ui-error.png` montre l’erreur visible `SourceMissing`.

La qualification mémoire instrumentée utilise le feature facultatif `l01-memory-qualification`, désactivé par défaut. Sur un WAV généré de 640 123 échantillons, le worker a exécuté 9 états natifs consécutifs (8 fenêtres de 80 000 et une queue de 123). Chaque début d’état, fin d’inférence et destruction a une lecture synchronisée des PrivateBytes du processus worker. Les PrivateBytes post-destruction vont de 1 639 038 976 à 1 640 292 352 octets; premier→dernier −204 800 octets, avec 4 hausses et 4 baisses. Les PrivateBytes à la fin d’inférence sont entre 2 491 691 008 et 2 584 109 056 octets. Les pipes stdin et stdout rapportent chacune 65 536 octets par direction. Les données brutes et l’analyse sont jointes dans `memory-long-*.jsonl` et `memory-long-analysis.json`.

Cette campagne montre un retour dans une bande stable après destruction pour ce parcours. Elle ne fournit ni borne universelle de mémoire, ni attribution des allocations internes au modèle/contexte natif; les compteurs caractérisent le processus entier. Détails et commandes de validation dans `validation.md`.
