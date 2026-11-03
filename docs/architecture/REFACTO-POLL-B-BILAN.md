Avancée : familles 5/6 extraites et revues CLEAN ; N1 finale 734 réussites, zéro échec, 149 ignorés ; sept corps N2 PASS.
Blocage : suppression du cache isolé refusée par l'outil ; cache conservé.
Décision : arrêt après B ; C non commencé, aucun push ; limites de qualification ci-dessous.

# Bilan REFACTO-POLL-B — 3 novembre 2026

Worktree `C:/wt/poll`, branche `refacto/poll`. [Plan et preuves](REFACTO-POLL-B-PLAN.md).
Préparation `6316a54b` : **100 passages micro consécutifs, zéro échec**, via la
garde limitée au test exact après arbitrages sur les paquets complets.

| Famille | Na / Nb | Tests nouveaux | Handlers, lignes physiques |
| --- | --- | ---: | --- |
| Stop avant activation / reprise | `cfce1612` / `3bbb95fd` | 10 | 48, 37 |
| Stop d'un direct actif | `067ba749` / `7e30d667` | 13 | 24, 59, 68, 28 |

Extractions : 69 puis 152 lignes déplacées, chacune sous 800 ; ordre,
contrôles et handlers séparés préservés. Chaque handler respecte 100 lignes
et complexité 25. `poll()` reste à **489 lignes Clippy / complexité 52** :
reliquat pour C, seuil global non atteint ; aucune référence Clippy créée.
Témoin PCM exclusivement cfg(test), séparé dans `692ce032` ; banc N2 `1be46656`.

**Mutations détectées puis restaurées.** F5 : retrait PublicationGate détecté
par `poll_stopped_manual_gpu_choice_is_cancelled_by_publication_gate` ; reprise
CPU forcée malgré Stop détectée par
`poll_stopped_import_auto_retry_drops_request_when_gate_is_cancelled`.
F6 : inversion live_archive/live détectée par
`poll_stopped_live_failure_keeps_archive_lock_until_writer_shutdown` ; barrière
hors borne laissant publier détectée par
`poll_stopped_live_defensive_barrier_past_sequence_rejects_publishable_prefix`.
Les seules fixtures défensives et le témoin étaient explicitement autorisés.

**Vérifications.** N1 finale : défaut 685/128, common 11/1, CPU 12/9,
Parakeet CPU 26/11 ; Structure 931/931. Format, Clippy et auto-tests de garde
réussis ; PE **non concerné**. Revues indépendantes des Na/Nb et du banc : CLEAN.

| N2, durée du corps | Référence | B |
| --- | ---: | ---: |
| Reprise interrompue | 13,21 s, pré-B | 13,35 s |
| Direct Turbo CPU | 42,44 s, pré-B | 42,54 s |
| Import long FR / EN | 175,79 / 195,44 s, historique | 168,57 / 187,17 s |
| Auto sans candidat GPU | 26,14 s, historique | 41,55 s |

Référence pré-B `00c2d87e` : production `6316a54b`, même banc direct/profils.
Sept corps PASS, preuves vérifiées indépendamment ; reprise à transcripteur contrôlé, direct à capture simulée
avec vrai moteur CPU. Publications vérifiées par les tests. Aucune preuve
microphone physique, fenêtre Windows, WER ou qualité lexicale. Écart Auto
+15,41 s inexpliqué ; aucune conclusion statistique de performance.

**Limites et rétention.** Premier lancement long interrompu pendant compilation
sur deux Changed d'origine inconnue ; reprise des trois corps autorisée puis
réussie, garde inchangée. Écart de procédure Clippy 5b consigné au plan.
Dette A : remplacer ultérieurement le helper archive cfg(test) par un trait
injecté ; test GPU historique au nom trompeur inchangé.
[Archive](../../target/parakeet-tests/refacto-poll-b/archive/REFACTO-POLL-B-logs-20261103.zip) :
1 517 fichiers, nombre et SHA-256 de chaque entrée vérifiés. Cache isolé
`target/n2-native/cargo-home` conservé après refus de suppression : 651 937 640
octets ; commande utilisateur au plan. Caches partagés/persistants, modèles,
audios et preuves préservés. Aucune suppression ; 586,41 Gio libres.
Ce bilan est figé à son commit de clôture.
