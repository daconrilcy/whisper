Avancée : les cinq plus grands sites `too_many_lines` sont identifiés et leurs scénarios sont tracés.
Blocage : aucune voie N1 complète enregistrée n’est établie pour le futur lot de correction.
Décision : proposer un refactor test-only en cinq étapes ; aucune modification de code n’est autorisée par ce cadrage.

# Cadrage — refactor des cinq plus grandes alertes `too_many_lines`

**ID :** D-WHISPER-CLIPPY-TOP5-20261113  
**État :** proposition soumise à revue indépendante.  
**Base de code :** `main@4889e916486bb13cb9998751fe0c47edfe351f2e`. Les alertes proviennent du Clippy frais du 13 novembre 2026, rapporté dans [rapport-clippy-frais-20261113.md](rapport-clippy-frais-20261113.md). La sélection trie les sites uniques `too_many_lines` du Clippy `--all-targets`, après déduplication par crate, chemin, ligne et lint.

## Besoin et portée

Réduire les cinq plus grandes fonctions/helpers de test signalés par `clippy::too_many_lines`, pour rendre leurs scénarios de transcription, publication, nettoyage et reprise plus faciles à comprendre et à maintenir. Le travail prévu reste dans les supports de test de `whisper-adapters` : aucune politique, sortie ou transition du produit ne doit changer.

Le relevé totalise 151 sites structurels uniques dans six crates. Les cinq retenus sont tous des helpers ou scénarios de test. Deux portent aussi une alerte `cognitive_complexity` (`run_native_personal_ab_c` : 171/25 ; `native_queued_gpu_resume_uses_current_attestation_and_rebinds_snapshot` : 84/25). Ces deux lints seront mesurés comme effets secondaires, sans élargir le but à une refonte générale de la complexité.

## Sites retenus et contrats à préserver

| Rang | Site observé au commit de base | Mesure | Conditions et contrat du scénario |
|---|---|---:|---|
| 1 | [`run_native_personal_ab_c`](../../crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:1496) | 1623/100 | Helper `cfg(windows)` appelé par les scénarios natifs CPU/GPU ignorés hors `native-tests`. Garder l’ordre A1 → Stop A idempotent → drain audio durable → démarrage B ; l’ACK B durable précède le résultat A1. Préserver les identités, générations, incarnations/PID, limites d’inférence, révisions document, événements IPC et exports A/B. C ne repart qu’après le réapage du worker précédent. |
| 2 | [`replace_all_publishes_first_stops_partial_cleanup_and_resumes_only_on_request`](../../crates/whisper-adapters/tests/reprocessing_cleanup.rs:906) | 697/100 | Test ignoré sans `native-tests`, marqué `TODO(DEBT-11)` ; le vrai modèle Turbo reste requis par le catalogue malgré le faux worker IPC synthétique. Préserver publication du successeur avant retrait, ordre des quatre confirmations, changements de source refusés, écritures d’index/catalogue, idempotence des récupérations, refus des alias/preuves altérées, reprise explicite avec le même `cleanup_id`, sélection des fichiers et concurrence sous verrou. |
| 3 | [`native_queued_gpu_resume_uses_current_attestation_and_rebinds_snapshot`](../../crates/whisper-adapters/src/worker_ipc/tests/native_execution.rs:3368) | 621/100 | Test `cfg(windows)`, ignoré hors `native-tests`, GPU et fixture FR native. Garder le même `JobId`, la génération N interrompue puis N+1, l’audio et le préfixe de checkpoint, la liaison du snapshot, l’attestation GPU courante sans repli, la corrélation des événements/résultats, la couverture contiguë du préfixe et du suffixe, les exports et la libération finale des liaisons. |
| 4 | [`run_native_import_from_path_timed`](../../crates/whisper-adapters/tests/parakeet_unified_native.rs:296) | 459/100 | Helper chronométré appelé par les scénarios natifs de cette intégration et par le parcours VoxPopuli. Préserver les branches CPU/GPU/Auto, la préparation de source et son SHA, la génération et l’identité avant `StartImport`, le profil/attestation de route, les points et l’ordre de mesure, `Complete`, l’archive et ses WAV/TXT/SRT, ainsi que la pagination des parties PCM, leurs en-têtes/bornes et le total indexé. Le WAV logique de première partie et le total indexé de session restent des mesures distinctes. |
| 5 | [`unified_runtime_keeps_reserved_final_turn_through_preparation_with_peer_preview_ready`](../../crates/whisper-adapters/src/worker_ipc/tests/unified_capture.rs:2137) | 443/100 | Test ignoré sans `native-tests`, marqué `TODO(DEBT-10)` et nécessitant le modèle Parakeet du catalogue. Préserver l’identité A/B, la réservation du tour final A face à l’aperçu B, les barrières READY, l’absence d’inférence avant libération, l’unicité des appels actifs, le réapage puis le spawn de la seconde incarnation, l’ordre A→B→A, les ACK/publications et les archives par identité. |

L’apport en lecture seule de l’architecte de domaine confirme les phases d’extraction candidates : préparation, exécution par étapes, observation/attestation, vérifications terminales. Le détail de ces contrats est repris dans le tableau ci-dessus ; les sources restent l’autorité pour les assertions exactes.

## Exigences et acceptation

- **REQ-01 — Cinq sites précis :** le travail couvre uniquement les cinq sites classés ci-dessus sur cette base, dans les quatre fichiers listés. Aucun sixième hotspot n’est ajouté automatiquement.
- **REQ-02 — Lisibilité sans masquage :** découper chaque scénario selon ses phases cohérentes, sans `allow`/`expect`, changement de seuil, baseline ou suppression d’assertion.
- **REQ-03 — Équivalence de test :** conserver noms des tests publics au harnais, attributs `cfg`, feature, raisons `ignore`, fixtures, valeurs, assertions, ordres, délais, identités et sorties observées. Les marqueurs `TODO(DEBT-10/11)` et les prérequis modèles restent en place.
- **REQ-04 — Frontière de production :** aucun fichier de production, manifeste, lockfile, protocole IPC, catalogue ou garde ne change ; pas de dépendance/API nouvelle.
- **AC-01 — Retrait des cinq diagnostics :** le Clippy JSON final ne signale plus `too_many_lines` sur les cinq fonctions ciblées ; aucune nouvelle clé structurelle n’est introduite par les helpers extraits.
- **AC-02 — Contrôle global :** le cliquet Clippy reste PASS et la baseline versionnée reste inchangée. Les deux alertes cognitives sur les sites 1 et 3 sont rapportées séparément, qu’elles disparaissent ou restent.
- **AC-03 — Contrat préservé :** revue ligne à ligne des assertions, séquences d’effets, corrélations, ordres et limites décrits dans le tableau ; aucun oracle n’est affaibli.
- **AC-04 — Couverture honnête :** Clippy compile les cibles mais n’exécute pas ces scénarios. Les trois tests ignorés ainsi que les appelants natifs restent `NON EXÉCUTÉS` sous N1 ; leur exécution nécessite une campagne native séparément autorisée.
- **AC-05 — Validation attribuée :** la validation future distingue le résultat Clippy gardé, les tests N1 par défaut, la revue du diff et toute exécution native (`NON EXÉCUTÉE` ici).

## Architecture, risques et décisions

Les extractions restent privées dans les mêmes modules de test/intégration. Un petit contexte de scénario est acceptable si cela évite une longue liste d’arguments ; chaque helper nouveau reste sous les seuils Clippy applicables. Les temporisations, canaux, verrous, `Condvar`, drains et accès fichier gardent leur ordre et leur durée de vie. Aucun nouveau fichier ne paraît nécessaire à ce stade.

- **R-01 — Ordre concurrent altéré :** déplacer un drain, un verrou, un ACK ou un délai peut changer le résultat observé. Extraire par phase sans réordonner les opérations ; comparer les séquences avant/après.
- **R-02 — Assertion affaiblie :** factoriser les oracles peut réduire le nombre de vérifications ou perdre l’identité fautive. Garder les mêmes valeurs et messages diagnostiques utiles.
- **R-03 — Précondition native masquée :** Clippy et N1 ne prouvent pas ces scénarios ignorés. Garder `ignore`, la feature et les prérequis ; ne pas présenter la compilation comme exécution.
- **R-04 — Dépendance modèle héritée :** `DEBT-10/11` signalent une séparation différée entre scénario synthétique et modèle réel. Cette refactorisation ne résout ni ne contourne ces prérequis.
- **R-05 — Gate du futur lot :** la garde actuelle exige une sélection `-p` et n’accepte que les racines/profils enregistrés. Le profil de Clippy de six crates est enregistré sous `refacto-reprise-import`, mais cela ne donne pas automatiquement une racine ou une matrice N1 à ce nouveau lot. Le préflight d’implémentation doit établir une voie autorisée avant toute commande Cargo ; aucune modification de garde ou réutilisation implicite d’une racine étrangère n’est proposée ici.

**D-01 — Sélection :** retenir les cinq plus grandes alertes `too_many_lines` parmi les sites uniques du relevé toutes cibles ; elles sont toutes dans du code de test.  
**D-02 — But :** viser le retrait des cinq alertes et l’absence de nouvelles clés structurelles ; ne pas imposer une réduction globale de toutes les alertes de test.  
**D-03 — Tests ignorés :** garder les conditions et limites natives actuelles.  
**D-04 — Implémentation :** non autorisée par le cadrage ni le plan ; un accord précis ultérieur devra nommer le lot et ses chemins.

## Readiness

Le périmètre et les oracles des cinq cas sont tracés vers des sources datées. La préparation d’implémentation demeure conditionnée au préflight N1/Clippy du futur lot et à un accord explicite distinct. Aucune commande Cargo, aucun test, aucun build natif et aucune modification Rust ne sont exécutés pour ce cadrage.
