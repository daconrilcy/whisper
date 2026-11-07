Avancée : T6–T9 automatisés terminés, revue finale CLEAN ; poll 6/6, N1 864/0/129 ×3, N2 4/4.
Blocage : aucun pour la remise ; recette dans la fenêtre Windows NON EXÉCUTÉE.
Décision : remettre le profil Personal pour recette ; bilan non figé jusqu’à décision de clôture, preuves et dossiers conservés.

# REFACTO-POLL-UNIFIED — remise de phase 2, 7 novembre 2026

Phase 1 intégrée localement à `main` par avance rapide vers `e22eed93`. Sur `refacto-poll-unified` : autorisation `7e6cdf28`, T6 récupération `3a0655c0`, T7 clôture documentaire `56560a50`, T8 lancement des appels `63195ef5` (inclut la réduction d’un helper T6 de 108 à moins de 100 lignes). Quatre sources : `runtime/poll_unified.rs` et ses nouveaux modules `recovery.rs`, `document.rs`, `calls.rs`. Tests, assertions et références inchangés. Aucun merge de phase 2 ni push.

Complexité cognitive **production / test**, même Clippy, seuil de mesure 0 ; détails et closures dans les diagnostics archivés :

| Fonctions | Scores |
|---|---|
| `poll_unified_sessions` | **64/71 → 36/38 → 6/6** |
| T3 : `start_unified_executor`, `ensure_unified_executor`, `restart_unified_executor` | 4/7 ; 3/5 ; 2/2 |
| T4 : `collect_unified_capture`, `publish_unified_capture`, `finish_failed_unified_capture` | 8/8 ; 5/6 ; 3/3 |
| T5 : `finish_failed_unified_inference`, `commit_unified_window`, `finish_failed_unified_commit`, `apply_unified_result` | 3/3 ; 3/3 ; 3/3 ; 7/7 |
| T6 : `recover_disconnected_unified_executor`, `recover_unified_reap_failure`, `finish_failed_idle_capture` | 4/4 ; 10/10 ; 2/2 |
| T7 : `finish_unified_document`, `finish_unified_document_state` | sans diagnostic¹ ; 5/6 |
| T8 : `launch_next_unified_call`, `reserve_next_unified_call`, `prepare_and_dispatch_unified_call` | 1/1 ; 4/4 ; 4/4 |
| T8 : `prepare_unified_window`, `dispatch_prepared_final`, `dispatch_prepared_preview` | 2/2 ; 2/2 ; 2/2 |
| T8 : `ensure_unified_backend_ready`, `handle_unified_readiness_error`, `begin_and_dispatch_unified_call` | 3/3 ; 3/3 ; 3/3 |

¹ Pas de score numérique inventé : mesure active dans la même unité, deux closures diagnostiquées à 1. Fonctions nouvelles ≤100 lignes, fichiers ≤1 500, arguments ≤7 ; pas de nouvel avertissement structurel dans le delta. Voisins remesurés sans modification : `poll_parakeet_worker` 30, `run_cleanup_plan_locked` 32. Format et Clippy qualité réussis, avertissements préexistants conservés.

**Validation et revue.** Chaque extraction a passé N1/corpus avant commit. Les trois N1 finaux sur `63195ef5` donnent **864/0/129**, mêmes 993 identités/statuts entre eux et avec la phase 1. Common **11/0/1**, Parakeet CPU **26/0/11**, deux harnais PE PASS et auto-test PE **4 positifs/15 négatifs** ; témoin réel facultatif NON EXÉCUTÉ. Turbo CPU NON EXÉCUTÉ selon D4(b), identité des sources/lock et dépendances vérifiées. N2 `PersonalUnified-ad787fbd9ffd4af0a11abd11f1bc740d` **4/4** (CPU/FR, GPU/EN, GPU/FR, Auto/EN), **8/8 TXT/SRT** identiques aux références baseline et finales de phase 1 ; WHEA **1→1**, RecordId **527992**. [Revue indépendante finale](../cadrage/revue-refacto-poll-unified.md#revue-du-candidat-de-phase-2) : **CLEAN**, un passage de fond, aucune correction, preuves T9 réadjudées par le même reviewer.

**Remise Personal.** Build Rust release sous N1 réussi, sans build natif, sur HEAD documentaire `761f4313` (source `63195ef5`). [Dossier de remise](../../deliverables/refacto-poll-unified/) : desktop et seuls workers Turbo CPU/GPU, trois EXE vérifiés ; vingt ressources installées vérifiées, Parakeet/DLL/modèles lus dans l’installation personnelle. Desktop SHA-256 `9f11d6c68c5899b31a88307a4bb021952b724c304aeb07fa5e1c0e41f94be364`. Aucune fenêtre ouverte. Recette §8 à l’utilisateur : pour Parakeet puis Turbo, import CPU, micro GPU 30 s avec Stop, puis session courte après chaque Stop ; contrôler texte, audio complet, TXT/SRT/WAV de même nom et absence de session bloquée.

**Limites conservées.** Premier N2 `673523…` interrompu sur une écriture sous l’ancien LOCALAPPDATA N1, auteur non établi. Après accord de reprise, PID VCTIP 72772 revérifié puis disparu avant l’arrêt envisagé ; aucun processus arrêté. Reprise réussie sous garde inchangée, sans preuve de causalité. Nouveau VCTIP observé après build, laissé en place. Échecs préliminaires corrigés et échec desktop isolé puis non reproduit conservés ; cause de ce dernier non établie. Aucun défaut produit corrigé hors périmètre.

**Preuves et rétention.** [Archive de phase 2](raw/refacto-poll-unified-preuves.zip) : **3 613 membres**, **4 087 265 octets**, SHA-256 `308c0e63a3b103859df8eced5aa07fd6d65f9888777803351823c38175677570` ; nombre et chaque SHA-256 vérifiés après réouverture. Logs, traces/documents/index, références, sorties N2, remise et inventaire inclus ; archive intermédiaire conservée dans Git. Lot N1 total **347,28 Gio** dont phase 2 **37,64 Gio** ; N2 **21,44 Gio** (points de réanalyse non suivis). Caches partagés/persistants **4,77/0,61 Gio** conservés, remise **38 641 664 octets** ; environ **89,41 Gio libres**. [Phase 1 figée](REFACTO-POLL-UNIFIED-PHASE1-BILAN.md), modèles, WAV et intermédiaires préservés ; aucune suppression.
