# D21 findings correction record for D-WHISPER-22

## D20-002 — fermé dans D22

`00_START_HERE.md` donne des liens relatifs calculés depuis `candidates/D-WHISPER-22/` : `../../../state.json` et `../../../state.pending.D20-host.json`. Les deux destinations existent sous `docs/predev/`; leurs hashes/chemins sont vérifiables en ouvrant les liens. Le state actif est distingué du pending non actif et les commandes donnent la racine de contrôle exacte. Les promotions futures doivent être suivies par une relecture du state actif.

## D21-001 — fermé dans D22 avec limite explicite

D22 ne qualifie plus le rapport D20 de copie exacte. Le rapport courant sous `sources/reviews/R-WHISPER-DESIGN-20-v1-transcription.md` s’intitule « transcription sémantique vérifiée », précise qu’aucune égalité en octets n’est revendiquée, nomme les deux écarts connus (mise en forme backticks/gras et bloc mémoire terminal absent) et rapporte la confirmation d’auteur sur verdict, findings, preuves et conditions. D20/D21 sont préservés immuables avec leurs noms/hash historiques; le transport précédemment suffixé `EXACT` n’est pas présenté comme preuve d’identité à la sortie conversationnelle.

Le rapport D21 et son reçu/manifest transport sont conservés en `sources/reviews/` et `sources/review-transports/T-WHISPER-REVIEW-D21-TRANSCRIPTION-01/`. Sa qualification non verbatim reste visible.

## D20-001 et gates restants

D20-001 demeure fermé par les 23 règles actuelles liées dans D21 et réancrées au chemin du candidat D22; l’index et le freeze sont recalculés pour les chemins D22. P24/D19 demeurent inchangés. D22 est DRAFT jusqu’à la revue exacte. P25 successeur PLANS parent D22 sera nécessaire si le DESIGN est READY. Le checkpoint doit ensuite lier candidats revus, baseline courante, autorisation des quinze chemins, détails acceptés et identité HEAD concordante avant `check-state --lot L-WHISPER-04`.

Aucun code ni préflight de lot n’est exécuté dans ce candidat.
