Avancée : sous-lot A réalisé, 39 tests nouveaux, huit commits Na/Nb revus CLEAN ; N1 finale 710 réussites / 148 ignorés / zéro échec.
Blocage : aucun sur le candidat final ; réserves historiques de vérification ci-dessous.
Décision : clôture et arrêt après A ; B, C et la campagne N2 ne sont pas commencés. Aucun push.

# Bilan REFACTO-POLL-A — 3 novembre 2026

Base `9cd15ca6`, branche `refacto/poll`, worktree `C:/wt/poll`.
[Plan et preuves détaillées](REFACTO-POLL-PLAN.md), premier commit `7bf61aa4`.

| Famille | Commits Na / Nb | Tests ajoutés | Lignes physiques des handlers, dans l'ordre du module |
| --- | --- | ---: | --- |
| [Observations](../../crates/whisper-adapters/src/worker_ipc/runtime/poll/observations.rs) | `69fda611` / `c0456006` | 13 | 58, 96 |
| [Segments / reprise](../../crates/whisper-adapters/src/worker_ipc/runtime/poll/segments.rs) | `6ff324fc` / `2c90586e` | 7 | 36, 45, 29 |
| [Attestation / admission](../../crates/whisper-adapters/src/worker_ipc/runtime/poll/readiness.rs) | `df2fefb9` / `360ac380` | 11 | 23, 23, 89 |
| [MP3 / fenêtres](../../crates/whisper-adapters/src/worker_ipc/runtime/poll/live_windows.rs) | `69809a26` / `804362de` | 8 | 37, 66 |

**Mesures.** `poll()` : lignes Clippy 1 085 → 700, complexité 102 → 73,
physiques 1 098 → 707 ; seuils finaux réservés à B/C. Dix handlers ≤ 100 lignes
Clippy et ≤ 25 de complexité, lints actifs ; extractions sous 800 lignes.
Instructions/ordre préservés ; aucune fusion, simplification, correction de
bug produit ou modification de test existant.

**Caractérisation.** [39 tests séparés](../../crates/whisper-adapters/src/worker_ipc/runtime/poll/tests.rs).
Mutations F2 : retour silencieux → erreur ; F3 : garde d'annulation retirée →
BeginImport indu avant Stop. Détecteurs rouges, code restauré exactement puis
N1 verte ; noms/journaux au plan. Helper archive exclusivement `cfg(test)`
autorisé pour provoquer une vraie erreur MP3, absent du produit normal.

**Vérifications.** N1 : référence 671/148, final défaut 661/127, common 11/1,
CPU 12/9, Parakeet CPU 26/11, zéro échec. Fmt, Clippy, SelfTest 33/64,
Structure 906/906, fixtures PE 4/15 et neuf harnais PE avant leurs tests : PASS.
Témoin PE réel, N2, matériel natif et fenêtre Windows : NON EXÉCUTÉS.

**Réserves.** Échec micro existant non reproduit au diagnostic ni aux N1
suivantes, cause inconnue. Preuves PE avant 4a partielles, sans qualification
rétroactive. Premières relances après échec fmt consignées avant arbitrage.
Test GPU manuel au nom trompeur inchangé ; vraie branche de choix CPU prévue
en famille 5.

**Rétention.** [Archive des journaux](../../target/parakeet-tests/refacto-poll-a/archive/REFACTO-POLL-A-logs-20261103.zip) :
828 fichiers, arborescence conservée, nombre et SHA-256 de chaque entrée vérifiés.
Aucune cible/cache isolé propre au lot ; caches partagés/persistant préservés.
Aucune suppression de clôture ; 620,77 Gio libres après archivage.
Ce bilan est figé à son commit de clôture.
