Avancée : P41–P50 terminés ; le reviewer indépendant déclare P50 CLEAN et le ratchet Clippy passe de 91 à 81 sites.
Blocage : la suppression gardée des dix cibles N1 (36 778 770 270 octets) a été refusée par l’outil ; elles restent présentes.
Décision : demander l’exécution opérateur du nettoyage puis arbitrer la tranche suivante P51–P60, conformément à la limite de dix tâches.

# Bilan Clippy P41–P50 — 14 novembre 2026

Les dix plans sont versionnés dans `docs/cadrage/plan-clippy-…-20261114.md` : P41 `parakeet-decoder-open-track`, P42 `cpu-live-retry`, P43 `source-binding-release`, P44 `live-window-pump`, P45 `gpu-inventory-start`, P46 `microphone-probe-push`, P47 `publish-inner`, P48 `controlled-unified-import`, P49 `recover-published-cleanup`, P50 `parakeet-normalize-words`. Les changements source restent dans le worktree `C:/wt/cs`; les modifications préexistantes ont été préservées.

| Validation de clôture | Résultat |
|---|---|
| Clippy, mesure cognitive au seuil 0 | PASS, 12 208 diagnostics mesurés |
| Clippy standard et ratchet | PASS, 81 sites (maximum autorisé par P50 : 81) |
| N1 fraîche P50 | 570 réussis, 0 échec, 49 ignorés |
| Sélections worker CPU / Parakeet CPU / common | PASS ; 4 / 5 / 1 harnais contrôlés PE, suites exécutées |
| SelfTest PE / témoin PE réel | PASS (4 positifs, 15 négatifs) / NON EXÉCUTÉ |
| fmt-check, SelfTest de garde, diff-check | PASS |
| Revue indépendante P50 | CLEAN, sans constat |

Les tests nécessitant modèles, DLL natives ou audio réel restent ignorés ; le résultat N1 ne constitue pas une qualification native. L’archive [clippy-corrections-P41-P50-20261114.zip](raw/clippy-corrections-P41-P50-20261114.zip) contient 671 fichiers issus de 173 dossiers scratch (312 745 362 octets source). Chaque membre a été vérifié par taille et SHA-256. SHA-256 ZIP : `9a72df406e6de9ae30ed0efbd600e8caeb4cd4cc26747719f07dc96dc8741c8d`.

Le nettoyage reste à faire après refus de suppression par l’outil. Les dix chemins exacts, déjà contrôlés sous `target/.../targets/`, sans reparse point ni processus Cargo/rustc les utilisant, sont `n1-p41-20261114` à `n1-p50-20261114` sous `C:/wt/cs/target/parakeet-tests/clippy-corrections-suivantes-20261113/targets/`. Leur taille totale est de 36 778 770 270 octets. Les caches partagés et persistants ne sont pas concernés. Après leur suppression par l’opérateur, le contrôle de rétention sera repris.
