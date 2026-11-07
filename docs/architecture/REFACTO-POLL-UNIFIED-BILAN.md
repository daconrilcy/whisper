Avancée : T6–T8 commités, revue source CLEAN ; complexité poll 6/6 et N1 final 864/0/129 ×3.
Blocage : N2 interrompu avant résultat ; build release, remise et recette humaine non exécutés.
Décision : bilan intermédiaire, lot non clos ; conserver les preuves et demander l’arbitrage de reprise N2.

# REFACTO-POLL-UNIFIED — phase 2, état au 7 novembre 2026

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

**Validation et revue.** Chaque extraction a passé N1/corpus avant commit. Les trois N1 finaux sur `63195ef5` donnent **864/0/129**, mêmes 993 identités/statuts entre eux et avec la phase 1. Common **11/0/1**, Parakeet CPU **26/0/11**, deux harnais PE PASS et auto-test PE **4 positifs/15 négatifs** ; témoin réel facultatif NON EXÉCUTÉ. Turbo CPU NON EXÉCUTÉ selon D4(b), identité des sources/lock et dépendances vérifiées. [Revue indépendante](../cadrage/revue-refacto-poll-unified.md#revue-du-candidat-de-phase-2) : un passage, aucune correction, **source CLEAN**, verdict global BLOCKED sur T9.

**N2 et limites.** `PersonalUnified-67352321b166469e9c41f5b8f1df2114` interrompu par la garde au premier cas : Created/Changed sous l’ancien `phase2/scratch/20261107T193538341-193cd17705ae40f4a17c07afbec3492b/localappdata/Microsoft/VSApplicationInsights/`, hors racine N2 ; auteur non établi, aucun PID dans l’événement. WHEA **1→1**, RecordId **527992**. VCTIP PID **72772**, créé à 19:35:39 locale, reste une hypothèse de processus persistant, sans attribution prouvée ; aucun arrêt effectué. N2 n’a produit aucune comparaison finale ; remise et checklist Windows du plan §8 restent à exécuter. Les échecs préliminaires (typage/emprunts corrigés, paramètre modèle omis, formats) et l’échec desktop isolé puis non reproduit restent conservés ; cause de ce dernier non établie.

**Preuves et rétention.** [Archive de phase 2](raw/refacto-poll-unified-preuves.zip) : **3 558 membres**, **4 052 990 octets**, SHA-256 `acf57b40275275a7876cd58d4191cf05a6a37c40fc9d614d2fbc2054116e8f95` ; archive rouverte, nombre et chaque SHA-256 comparés aux sources. Logs, traces/documents/index, références et diagnostics inclus ; modèles/WAV et intermédiaires restent en place. [Phase 1 figée](REFACTO-POLL-UNIFIED-PHASE1-BILAN.md) conservée. Aucun nettoyage ; caches partagés/persistants préservés. Inventaire de rétention final et clôture restent à T9.
