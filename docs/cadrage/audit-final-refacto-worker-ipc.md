Avancée : audit indépendant du delta cumulé A–F3 terminé ; aucun écart actionnable identifié.
Blocage : aucun pour la clôture structurelle ; qualification native et Windows hors périmètre.
Décision : conformité du périmètre structurel ajusté retenue sur les preuves examinées, sans modification du code ni push.

# Rust Implementation Plan Audit

## Scope

- Date : 30 octobre 2026.
- Auditeur : nouvel intervenant indépendant, distinct de l'implémenteur et des reviewers des lots ; contribution remise en lecture seule. Rapport consigné par la session principale.
- Cadrage lu intégralement : [refacto-worker-ipc.md](C:/dev/whisper/docs/cadrage/refacto-worker-ipc.md), DEC-01–06, REQ-01–07, AC-01–07 et carte des propriétaires.
- Plan lu intégralement : [plan-refacto-worker-ipc.md](C:/dev/whisper/docs/cadrage/plan-refacto-worker-ipc.md), treize lots A, B, C, D1–D5, E1–E2, F1–F3 et ajustements consignés jusqu'au §38.
- Branche : evo-startup-perf. Base pré-implémentation `fa4772c26a66ad7f4347c095ae13260c55ce216b` ; cible finale `c753f7c62f9c9264ccb256c4b495efb94ccb548b`.
- Étendue : delta cumulé de 36 fichiers modifiés ou ajoutés, comprenant façade, propriétaires extraits, caractérisations, fixtures, correction du témoin microphone et plan versionné.
- Les verdicts historiques CLEAN ont été recoupés ; ils ne constituent pas des preuves autonomes.
- Instructions consultées : AGENTS racine et adapters, cartes d'architecture, compétences rust-audit-from-plan et rust-implementation-plan-audit.
- État contrôlé par l'auditeur : HEAD conforme, aucun changement suivi, seuls docs/architecture/, models/ et sound example/ non suivis préexistants, préservés.

Commandes et contrôles réellement exécutés pendant l'audit :

1. Lectures Git sans optional-locks : état, HEAD, historique, noms/statistiques du delta et sources aux commits comparés.
2. `git --no-optional-locks diff --check fa4772c26a66ad7f4347c095ae13260c55ce216b c753f7c62f9c9264ccb256c4b495efb94ccb548b` : réussi.
3. Recherches et lectures des sources, déclarations, réexports, consommateurs et conditions de compilation.
4. Comparaisons en mémoire des signatures, déclarations, constantes, corps et littéraux ; aucun fichier auxiliaire créé.
5. Lecture des index de campagne et journaux associés des treize lots.
6. Contrôle en lecture seule des treize archives : comptes, SHA-256 des ZIP et correspondance de chaque entrée avec sa source conservée, sans extraction.

Aucun Cargo, metadata, formatage, compilation, test, worker natif ou interface Windows exécuté pendant cet audit. Les résultats de campagne ci-dessous proviennent des journaux existants recoupés.

## Summary

- Nombre d'écarts actionnables : **0**.
- Écart le plus risqué : **None**.
- Verdict : **conforme au cadrage et au plan dans leur périmètre structurel ajusté et autorisé**.
- Phase gate : **Continue**, au sens où aucun écart identifié ne bloque la clôture structurelle ; aucune nouvelle implémentation ou campagne autorisée par ce verdict.

La comparaison cumulée retrouve 221 des 222 corps historiques de fonctions identiques après normalisation des espaces et virgules finales de présentation. Le seul corps restructuré est effect : ses trente bras, son préambule et son épilogue ont été vérifiés séparément. Cette preuve statique et les témoins automatisés étayent la préservation ; ils ne constituent pas une démonstration absolue d'équivalence de toute exécution possible.

| Métrique | Avant | Cible | Réel audité |
| --- | --- | --- | --- |
| Façade physique | 13 214 lignes | déclarative et fine | 130 lignes |
| Corps algorithmiques de production racine | présents | 0 | 0 |
| ImportRuntime | 1 dans la façade | 1 sous runtime | 1, runtime/mod.rs:44 |
| Dispatcher | 30 bras avec logique intégrée | 30 délégations exhaustives | 30, sans fourre-tout principal |
| Préambule/épilogue effect | centralisés | même propriétaire et chemins | conservés sous runtime/effects |
| Fichiers production finaux, façade comprise | façade monolithique | propriétaires bornés | 26 fichiers |
| Plus grand fichier production final | façade 13 214 lignes | nouveau fichier ≤1 500 lignes production | runtime/import.rs, 1 444 lignes physiques |
| #[path] ordinaires ajoutés | 0 | 0 | 0 |
| Fonctions #[test] dans fichiers production ciblés | 0 | aucun ajout inline | 0 |
| Signatures historiques | base Git | conservation hors visibilité interne justifiée | aucune signature manquante/modifiée détectée |
| Déclarations/types/traits/constantes/statics comparés | 51 | conservation et unicité | tous retrouvés, attributs conservés |
| Constantes/statics historiques | base Git | valeurs et unicité conservées | aucune perte, duplication ou nouvelle constante détectée |
| API publique | transport/service/probes/GPU historiques | chemins/signatures compatibles | conservée |
| Manifestes/lockfile/protocole/core | base Git | inchangés | aucun delta |
| Unsafe/dépendances/features nouveaux | aucun demandé | aucun | aucun détecté |
| Clippy F3 | 23 core + 146 adapters | pas de nouveau diagnostic non justifié | 169 avant/après, mêmes en-têtes en multiensemble |

Les seuils de fonction/complexité/arguments/types demeurent des cibles de revue avec dette héritée autorisée et attribuée ; ils ne sont pas un objectif de conformité globale de ce chantier.

## Deviations

**No actionable deviations found.**

Aucune correction minimale à prescrire dans le périmètre audité. Les limites de preuve et la dette héritée sont distinguées des écarts.

## Plan Coverage

### Décisions et exigences

Tous les chemins abrégés ci-dessous sont sous `crates/whisper-adapters/src/worker_ipc/`, sauf la façade `crates/whisper-adapters/src/worker_ipc.rs`.

| Élément | Statut | Preuve et appréciation |
| --- | --- | --- |
| DEC-01 — extraction interne | Implemented | Aucun delta manifeste, Cargo.lock, protocole, core ou desktop ; réexports [façade:32](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc.rs:32). |
| DEC-02 — modules privés/API | Implemented | Déclarations privées et réexports ciblés [façade:1](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc.rs:1), zéro nouveau #[path] ordinaire ; visibilité partagée limitée au sous-arbre, pub(crate) historique de gpu_evidence_matches conservé. |
| DEC-03 — état unique | Implemented | [runtime/mod.rs:44](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:44), quatre constructeurs à partir de 140 ; aucun clone d'ImportRuntime ou contexte artificiel. |
| DEC-04 — effet et retours | Implemented | [runtime/effects.rs:4](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:4), préambule/épilogue identiques ; deux retours doublons, deux direct et treize reprise préservés par Return/Continue. |
| DEC-05 — façade fine | Implemented | 130 lignes, aucun corps algorithmique de production ; runtime/mod reste état/constructeurs/déclarations. |
| DEC-06 — PublicationGate | Implemented | [publication/mod.rs:4](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/publication/mod.rs:4), cinq constantes et machine uniquement ; publication archive/exports restent runtime/service, aucune dépendance inverse archive introduite. |
| REQ-01 / AC-01 — transport/identité | Implemented | [ipc/mod.rs:29](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/ipc/mod.rs:29), [ipc/framing.rs:6](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/ipc/framing.rs:6) ; corps et limite 1 048 576 octets conservés, témoins framing/attestation avant/après. |
| REQ-02 / AC-02 — Stop/publication/ACK | Implemented | [publication/mod.rs:13](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/publication/mod.rs:13), [publication/tests.rs:1](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/publication/tests.rs:1), épilogue [effects.rs:162](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/effects.rs:162) ; commit en erreur restant COMMITTING conservé. |
| REQ-03 / AC-03 — admission bornée | Implemented | [service/mod.rs:684](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/service/mod.rs:684), capacités [support.rs:58](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/support.rs:58) ; réservations, saturation/déconnexion, Stop et attente 5 ms conservés. |
| REQ-04 / AC-04 — file/doublons/reprise | Implemented | [queue.rs:4](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/queue.rs:4), [duplicates.rs:214](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/duplicates.rs:214), [recovery.rs:9](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:9), [source_binding.rs:97](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/source_binding.rs:97) ; FIFO, refus courts, restitution et legacy recoupés. |
| REQ-05 / AC-05 — capture/sessions | Implemented | [live.rs:105](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/live.rs:105), [poll_unified.rs:368](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:368), [executor.rs:35](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/parakeet_unified/executor.rs:35) ; durées de vie/canaux/ACK/drain/fermeture préservés statiquement et huit témoins unifiés dans chaque suite F3 époque 0. |
| REQ-06 / AC-06 — moteurs/preuves/cache | Implemented | [backends/mod.rs:96](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/backends/mod.rs:96), backends/native ; orchestration retry [poll_unified.rs:21](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:21), démarrage runtime/import/worker_start ; sept T-F et intégrations publiques recoupés. |
| REQ-07 / AC-07 — sans élargissement | Implemented | API/traits/signatures historiques conservés, trente bras délégués, façade fine, aucun nouveau contrat inter-crates/dépendance/feature/unsafe ; consommateurs repérés et workspace contrôlé dans la campagne finale conservée. |

### Responsabilités et résolution effective des aliases

- ipc ne dépend pas de runtime/service/archive ; EVENTS_PER_STAGE rejoint support par l'alias façade.
- backends n'importe ni ImportRuntime ni service/runtime ; démarrage et gestion d'échec restent sur le runtime.
- retry_native_gpu_startup_on_cpu, NativeAutoCpuRetry, FailedManualGpuPeer et manual_gpu_failure_event appartiennent au polling unifié ; le prédicat pur de retry appartient au backend.
- parakeet_unified utilise les routes backend et les primitives existantes. Le writer WAV de native résout vers [support.rs:61](C:/dev/whisper/crates/whisper-adapters/src/worker_ipc/support.rs:61), sans dépendance runtime masquée.
- NEXT_IMPORT_JOB_ID et NEXT_INSTANCE ont une seule définition sous support ; NEXT_AUTO_EXPORT_STAGE appartient à service/automatic_final.
- runtime/support porte les helpers applicatifs, injections existantes et impl ServiceRuntime ; le contrat demeure déclaré sous service.
- Les chemins publics consommés par desktop/bootstrap/tests restent disponibles depuis la façade historique.

### Treize lots et budgets

Tailles et fichiers touchés recalculés à partir des commits, indépendamment des déclarations de clôture.

| Lot | Statut | Invariant | Façade avant → après | Budget production constaté |
| --- | --- | --- | --- | --- |
| A | Implemented | transport/framing ; compteurs/diagnostic techniques uniques | 13 214 → 12 726 | 4 d'extraction ; correction test microphone autorisée au §13 |
| B | Implemented | une gate et cinq constantes | 12 726 → 12 667 | 2 / 2 |
| C | Implemented | service/boucle/contrat et finale automatique | 12 667 → 11 569 | 3 / 3 |
| D1 | Implemented | quatre bras file, état unique | 11 569 → 10 826 | 3 / 3 |
| D2 | Implemented | doublons et retours courts | 10 826 → 10 400 | 3 / 3 |
| D3 | Implemented | huit bras direct/micro | 10 400 → 9 503 | 3 / 4 |
| D4 | Implemented | cinq bras reprise, bindings/legacy | 9 503 → 8 647 | 4 / 4 |
| D5 | Implemented | trente bras, préambule/épilogue uniques | 8 647 → 7 476 | 4 / 4 |
| E1 | Implemented | polling et auxiliaires sans réordonnancement | 7 476 → 5 316 | 5 / 5 |
| E2 | Implemented | polling unifié/drain/retry sous runtime | 5 316 → 3 991 | 3 / 4 |
| F1 | Implemented | primitives backend, démarrage sous runtime | 3 991 → 2 669 | 6 / 6 après ajustement §33 |
| F2 | Implemented | source/état, exécuteur, transcripteur attribués | 2 669 → 1 607 | 4 / 5 |
| F3 | Implemented | façade déclarative, runtime/mod borné | 1 607 → 130 | 8 / 8 |

Modifications annexes justifiées : correction du témoin micro autorisée au §13 (source microphone_probe limitée aux tests/déclaration du module de régression), racine temporaire ajustée en A, fixture legacy D4 bornée avec lien/SHA/assertions préservés, caractérisations séparées de publication/FIFO/doublon/échec automatique/pagination et budget F1 ajusté avant implémentation. Aucun élargissement non justifié identifié.

### Preuves automatisées et époques

Index logs/guard.jsonl, stdout/stderr et archives des treize racines examinés. Les dernières suites abouties correspondent aux bilans :

| Lot | Réussis défaut / recette / l01 / personal / combinaison | Preuve de clôture |
| --- | --- | --- |
| A | 266 / 270 / 266 / 266 / 270 | suites après correction du témoin, échecs initiaux conservés |
| B | 268 / 272 / 268 / 268 / 272 | suites candidat et gate |
| C | 268 / 272 / 268 / 268 / 272 | suites, incident et disposition distincts |
| D1 | 269 / 273 / 269 / 269 / 273 | suites et caractérisation file |
| D2 | 270 / 274 / 270 / 270 / 274 | suites après rétablissement microphone |
| D3 | 270 / 274 / 270 / 270 / 274 | suites et capture/ACK |
| D4 | 270 / 274 / 270 / 270 / 274 | suites avec legacy, contrôle ciblé après remise en forme fixture |
| D5 | 271 / 275 / 271 / 271 / 275 | suites époque 0, pagination/contrôles ciblés époque 1 |
| E1 | 272 / 276 / 272 / 272 / 276 | suites après reprise autorisée du formatage |
| E2 | 272 / 276 / 272 / 272 / 276 | suites époque 0, littéraux/format/workspace époque 1 |
| F1 | 272 / 276 / 272 / 272 / 276 | suites abouties, échec l01 et relance intégrale conservés |
| F2 | 272 / 276 / 272 / 272 / 276 | suites avant restriction visibilité, contrôles/panique après |
| F3 | 272 / 276 / 272 / 272 / 276 | suites époque 0, writer WAV/compilation/structure époque 1 |

Chaque suite aboutie conserve 16 ignorés, l'exclusion D2 corpus natif et, pour la combinaison concernée, l'exclusion l3c. Ce ne sont pas des exécutions réussies de ces corps.

F3 époque 0 : `20261030T153335468-886122d9`, `20261030T153641826-91373de9`, `20261030T153934616-ec915d8c`, `20261030T154236358-7ebec17f`, `20261030T154511158-33280d28`.

Journaux recoupés : les 36 filtres T-A à T-F sont chacun non nuls avant/après, sans NOT RUN dans leur sortie ; huit témoins unifiés, legacy et catalogue injecté réussissent dans chaque suite candidat époque 0 ; worker_control (5) et gpu_qualification_transport (1) passent avant/après. Les deux conditionnels passent isolément avant/après ; leurs retours NOT RUN dans les suites générales restent distingués.

| Contrôle F3 époque 1 | Run réussi |
| --- | --- |
| fmt --check | 20261030T160547953-2827a9e7 |
| native_window_wav_is_short_pcm16_mono_16khz_riff, un test réellement exécuté | 20261030T160549733-2a34de77 |
| check workspace | 20261030T160613585-c62820f1 |
| check adapters l01-memory-qualification | 20261030T160617695-1a0a9347 |
| Clippy avec quatre lints structurels | 20261030T160621168-dac84796 |

Preuve finale composée : les cinq suites n'ont pas été rejouées après le déplacement pur du writer WAV. Son corps/signature historiques, sa résolution vers le support transversal et les contrôles ciblés postérieurs ont été vérifiés.

### Conservation

Les treize archives correspondent aux comptes et SHA-256 consignés. Nombres d'entrées respectifs : 175, 75, 112, 88, 100, 100, 80, 104, 94, 90, 114, 148, 262. Lecture sans extraction ; aucune source correspondante manquante ou différente. SHA-256 de l'archive F3 : `94500FC481E916C88BA9ED8AB450985CAB65609FAAED85D58CEFD69BA4825FF4`. Incidents historiques conservés et SelfTest distingués des incidents réels.

## Gate Assessment

- Les treize invariants structurels sont satisfaits sur le candidat examiné ; aucun lot dépendant bloqué par un objectif partiel identifié.
- Ajustements confrontés aux modifications réelles : témoin A, fixture D4, budget F1 et frontière WAV F3.
- Aucune tranche corrective recommandée dans ce périmètre.
- Aucune réduction globale de complexité, migration d'erreurs typées, qualification native ou refonte supplémentaire exigée par ce plan.

Dette héritée réelle et attribuée : Clippy final signale notamment effect à 227 lignes, poll à 1 087 lignes/complexité 101, poll_unified_sessions à 960 lignes/complexité 64 et signatures VAD/retry à 9/10 arguments. Ces corps préservés ne contredisent pas l'exception encadrée du cadrage §7.

## Residual Risk

- Aucune nouvelle campagne exécutée pendant l'audit ; tests/compilations issus des preuves conservées et recoupées.
- Comparaisons statiques et résultats automatisés ne prouvent pas toute interaction concurrente ou matérielle possible.
- Preuves finales D5, E2, F2 et F3 composées : aucune revendication de suites complètes après chaque dernière retouche.
- Qualification native CPU/GPU/Auto, qualité/vitesse d'inférence et fenêtre Windows non réalisées par ce chantier, explicitement hors clôture structurelle ; absence non classée comme écart.
- Autorisations historiques documentées dans le plan ; l'audit ne reconstitue pas les échanges originaux des sessions.
- L'examen indépendant n'a modifié aucun fichier, index, historique ou ressource. Seule écriture de restitution par la session principale : le présent rapport ; aucun code modifié, aucun commit ni push, aucun nettoyage.
