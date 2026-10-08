# R-WHISPER-PLANS-17-v1 — FINDINGS

Candidat exact P-WHISPER-17, scope PLANS, parent D-WHISPER-19. Digest avant/après : `b8b3b10759ae0ba3f3c714e3b193d5abe867de740acb22d466bcb7eebef96459`. P14 reste PLANS actif; P17 reste DRAFT. State hash `577257c8be408cc414aa81b78c01498db5eb5fd9ac2711896ba0bfb98ac8edde`.

Résumé du résultat retourné par le reviewer indépendant dans cette conversation le 2026-10-07; le reviewer a refusé de modifier des fichiers en raison de ses instructions read-only. Cette conservation est une transcription du message final, non un export effectué par le reviewer; cette limite de provenance reste explicite.

## Findings REQUIRED

- **P16-REQ-001 — High — partiellement clos.** L’allowlist 13 couvre application/worker_ipc/UI, mais `main.rs` GPU est un stub L00, alors que D19/50 TECH-D18-04 demande moteur/décodeur/encodeur bloquants en enfant. Le Cargo GPU n’a pas `sha2`, `serde`, `rusty_mp3` comme le CPU. Décrire protocole, identité/hash, décodage MP3, encodage live, arrêt et chemins/manifests requis. Aucun test produit pré-code exigé.
- **P16-REQ-002 — High — ouvert.** Les références courantes annoncent `docs/predev/candidates/P-WHISPER-17` alors que le candidat se trouve sous `docs/predev/L03-P14-CLOSURE/`; le digest D19 textuel est erroné. Utiliser les vraies racines et commandes vérifiées, distinguer P14 actif et parent de persistance P16.
- **P16-REQ-005 — Medium — partiellement clos.** Les sources utilisateur P16, la revue P16 et l’attribution P17 sont manifestées; les transports P15/P16 sont byte-identiques aux sources transport accessibles; la revue P15 exacte absente est correctement déclarée. L’apport `coordinateur-P17-corrections.md` n’a pas de transport propre. Transporter l’apport réel avec attribution/base puis le lier au checkpoint.
- **P16-REQ-006 — Medium — ouvert.** Le checkpoint vérifié ne cite encore aucun transport P15/P16/P17 dans `raw_contributions`. Produire un checkpoint de reprise vérifié avec contributions, revues, hashes, tâches et prochain acteur; P14 peut rester actif.

## Findings fermés sur P17

- **P16-REQ-003 — CLOSED.** Inventaire 13 chemins; 8 présents et 5 absents, snapshots consumer exacts, CPU native_engine et preuve L01 séparés, futur `code_state` laissé au préflight.
- **P16-REQ-004 — CLOSED pour résolution.** Le contrôleur donne current→CODE_ROOT, evidence et snapshot→DOC_ROOT; 23/23 snapshots concordent.

## Advisory

Le test du contrôleur emploie design.md et review.md déjà protégés par manifeste/autres références; il n’est pas discriminant. Le snapshot CPU contient main.rs hors allowlist alors que native_engine est vérifié séparément; préciser. Aucun log brut 30/30 retrouvé par le reviewer.

Aucune modification D19 n’est démontrée nécessaire. Les campagnes produit, UI, livraison, autorisation code et `check-state --lot L-WHISPER-04` restent distincts et NOT RUN.
