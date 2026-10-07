# Cadrage Whisper Windows — successeur DESIGN DRAFT

**Candidate D-WHISPER-22 · successeur de D-WHISPER-21 · périmètre 2 · 2026-10-07 · DRAFT**

D22 conserve les 23 règles effectives de D21 et ses corrections de baseline. D20-001 a été fermé dans D21. D20-002 et D21-001 ont été corrigés ici : les liens sont résolus explicitement vers le state contrôlé; le rapport D20 est qualifié de transcription sémantique, non verbatim. Le rapport source du reviewer D21 est lui-même une transcription sémantique vérifiée, avec sa limite documentée. D22 attend sa revue exacte et sa promotion contrôlée.

## Statut courant contrôlé

Cette entrée réside sous `docs/predev/L04-PREFLIGHT/candidates/D-WHISPER-22/`. Depuis ce dossier, le lien [`state.json`](../../../state.json) se résout vers `docs/predev/state.json`; il a été contrôlé sur disque. Le lien [`state.pending.D20-host.json`](../../../state.pending.D20-host.json) se résout vers le pending sous `docs/predev/`; il est distinct du state actif. Les commandes de contrôle utilisent `--root docs/predev --state docs/predev/state.json`.

À la préparation du D22, le state contrôlé (`17044672f0788be7b50c375c13d7b5b5740f0743ac46fc1ed974f9dd2e60fd27`) lie D19 READY/CLEAN et P24 READY/CLEAN, L03 `completed`, L04 `planned`. Le checkpoint repository identity demeure ancien et n’autorise pas le préflight. Les états DRAFT/OPEN des candidats précédents sont historiques; pour connaître le statut présent ou après une promotion, ouvrir toujours le `state.json` ci-dessus et exécuter le contrôle sans `--lot`.

## Carte de D22

- `52_L04_SOURCE_BASELINE_D20.md` conserve le baseline L04 observé au HEAD `ffd93125604be5dc6439587b7edad4f420f5cccc` : dix snapshots existants et cinq créations futures absentes.
- `rules/` conserve les snapshots historiques de D19; `rules-effective/` et `sources/EFFECTIVE_RULES_MANIFEST.json` lient les 23 règles effectives actuelles.
- `sources/reviews/R-WHISPER-DESIGN-20-v1-transcription.md` qualifie les différences de la transcription D20 et garde le verdict/finding sémantiquement vérifiés sans revendication d’égalité octet.
- `sources/reviews/R-WHISPER-DESIGN-21-v1-transcription.md` consigne les deux findings D21 et leur qualification source; les preuves transport sont dans `sources/review-transports/T-WHISPER-REVIEW-D21-TRANSCRIPTION-01/`.
- `54_D21_FINDINGS_CORRECTIONS.md` documente la résolution des findings.

L’allowlist reste les quinze chemins exacts de P24 inchangé. L04 demeure `planned` dans l’état actif. Aucun préflight `check-state --lot L-WHISPER-04`, code produit, test produit ou qualification n’est revendiqué par D22.
