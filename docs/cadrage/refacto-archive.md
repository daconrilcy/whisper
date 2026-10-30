Avancée : périmètre, contrats, couverture statique et partition du refactoring définis.
Blocage : caractérisation préalable non exécutée ; revue DESIGN à réaliser.
Décision : préserver les comportements existants, puis extraire séquentiellement les responsabilités.

# Refactorisation d’archive.rs

## 1. Identité, autorité et sources

- ID : DESIGN-ARCHIVE v1, 30 octobre 2026 ; candidat de cadrage.
- Base inspectée : `2829ce4569a90f494e6ff5f73a8887ee452074cf`, branche `evo-startup-perf`.
- Auteur des fichiers : session Codex principale. Contributions en lecture seule : BRIEF-ARCHIVE v1 (`rust_product_framer`), REQ-ARCHIVE v1 (`rust_requirements_analyst`), DOMAIN-ARCHIVE v1 (`rust_domain_architect`), ARCH-ARCHIVE v1 (`rust_architect`), coordonnées par `rust_predev_orchestrator`.
- Sources d’autorité : demande utilisateur de traiter archive.rs en dernier gros chantier, de vérifier puis compléter les tests de reprise/corruption avant découpage et de séparer publication, reprise, identité et retrait ; qualification revue PASS dans cette session.
- Mission autorisée : « tu peux faire le cadrage puis rediger le plan de refacto avec les agents dediés ». Cadrage, plan et commits documentaires locaux seulement ; aucune implémentation, campagne Cargo, exécution de tests ou push.
- Règles : AGENTS racine/adapters, skills rust-predev-orchestration et rust-predev-design, règles et handoff-contract du pack courant sous `C:/Users/cyril/.codex/`.
- Vérifications : lectures statiques des sources, tests, manifestes, consignes et documents des précédents refactorings. Aucun test produit exécuté : NOT RUN.

Les sections suivantes constituent une synthèse attribuée des contributions, sans revendication de copie intégrale. Les choix techniques dans les invariants relèvent de l’architecte par délégation de conception ; le reviewer indépendant seul ferme ses constats et déclare CLEAN.

## 2. Besoin, périmètre et préalables

Rendre la persistance plus facile à comprendre et maintenir, en conservant fichiers, publications, contrôles d’identité et résultats de reprise actuellement observables. Version : refactorisation interne à comportement constant, sans nouveau mécanisme de reprise.

| Capacité | Résultat à préserver |
| --- | --- |
| CAP-01 | Préparation import, journal de segments et confirmations |
| CAP-02 | Persistance et publication LiveArchive |
| CAP-03 | Visibilité, réservation des noms et possession des exports |
| CAP-04 | Scan, diagnostic et reprise des archives interrompues ou corrompues |
| CAP-05 | Intégrité, identité, provenance et périmètre des chemins |
| CAP-06 | Remplacement/retraitement et retrait contrôlé des anciens traitements |
| CAP-07 | Tests de caractérisation préalables aux extractions |
| CAP-08 | Publication, reprise, identité et retrait localisables |
| CAP-09 | Archive après les gros chantiers préalables |

Le retrait désigne la responsabilité existante de remplacement/nettoyage, sans suppression des parcours historiques ni abandon de compatibilité.

Hors changement : moteurs CPU/GPU/Auto, capture, qualité/vitesse, UI, installation, packaging, réglages et interfaces OS. Exclus : correction fonctionnelle implicite, migration, format ou politique d’effacement nouveaux, nouvelle dépendance ou mécanisme de reprise. Toute anomalie découverte est signalée séparément ; sa correction demande une décision distincte.

Précondition « dernier gros chantier » : les deux grands préalables connus sont clos selon [audit-final-refacto-worker-ipc.md](audit-final-refacto-worker-ipc.md), lignes 1–15, cible `c753f7c62f9c9264ccb256c4b495efb94ccb548b`, et [plan-refacto-application-core.md](plan-refacto-application-core.md), bilan T1–T10 et façade finale HEAD `2829ce45`. Ces clôtures structurelles ne valent pas validation native/Windows. Elles ne prouvent pas l’absence d’autres travaux : avant code, actualiser Git, préalables et responsabilités concurrentes.

Préserver les non suivis `docs/architecture/`, `models/`, `sound example/`. Les cartes architecture sont statiques et non suivies ; confronter leurs affirmations aux sources. Espace C relevé au début de cette mission : 69,02 Gio ; à mesurer à nouveau avant toute campagne.

## 3. Exigences, cas d’usage et acceptation

Owner : rust_requirements_analyst, REQ-ARCHIVE v1. Une ligne REQ/UC/AC correspond à la capacité de même numéro.

| REQ / UC | Acceptation observable |
| --- | --- |
| REQ-01 / UC-01 Préparer, persister, interrompre et rouvrir un import | AC-01 : mêmes segments, réparation, séquence, fichiers et erreurs sur préfixe valide/queue partielle ; aucune duplication nouvelle |
| REQ-02 / UC-02 Confirmer des fenêtres micro et publier/retrouver un passage | AC-02 : mêmes snapshots, CURRENT, alias, pending et refus pour PCM/coverage/texte incohérents |
| REQ-03 / UC-03 Publier avec collision/interruption exports-CURRENT-index | AC-03 : mêmes actifs, octets et identité des tiers ; résolution répétée donnant mêmes catalogue/exports |
| REQ-04 / UC-04 Scanner/reprendre des archives saines, interrompues, corrompues ou transplantées | AC-04 : mêmes entrées, catégories de diagnostic et mutations sur deux scans/reprises successifs |
| REQ-05 / UC-05 Présenter identité/schéma/stratégie/génération/destination/chemin divergent | AC-05 : mêmes refus avant les mêmes effets ; comparer erreurs, pending, index, réservations et racines |
| REQ-06 / UC-06 Publier un successeur puis découvrir/reprendre un retrait partiel | AC-06 : successeur avant suppression ; mêmes contrôles source/identité/conteneur, inconnus conservés et effets découverte/reprise |
| REQ-07 / UC-07 Identifier une lacune et établir le témoin baseline | AC-07 : chaque point recensé possède test/assertions exercées ou limite explicite avant l’extraction concernée ; nouveaux tests sur code antérieur |
| REQ-08 / UC-08 Maintenir chacune des quatre responsabilités | AC-08 : carte fonctions-responsabilité ; signatures, schémas, diagnostics contractuels, ordre des effets et AC-01–07 préservés |
| REQ-09 / UC-09 Préparer la première modification | AC-09 : préflight Git/préalables/concurrence ; collision réelle arbitrée avant code |

Comparer avant/après sur les mêmes fixtures et scénarios. Toute différence est expliquée et soumise à décision avant poursuite. Aucun nouveau seuil de délai, volume, performance ou perte admissible.

États visibles à préserver : traitement, publication complète, publication interrompue à résoudre, archive récupérable, corruption diagnostiquée, nettoyage restant et cause durable. Les scans ne sont pas tous en lecture seule : certains réparent queues/alias ; certaines découvertes cleanup rejouent index/preuves. Préserver ces effets ciblés. Aucun nouveau comportement d’annulation ou d’arrêt ; caractériser les interruptions par les états durables actuels.

## 4. Domaine et confirmations

Owner : rust_domain_architect, DOMAIN-ARCHIVE v1.

| Concept | Propriétaire/données |
| --- | --- |
| D-01 Génération import | ArchiveStore.current : JobId, Generation, source, modèle, stratégie, séquence |
| D-02 Passage live | LiveArchive : verrou écrivain, tentative, segments, fenêtres, PCM, snapshots |
| D-03 Intention | CreateNew/Replace, actifs attendus, réservation, stockage, cibles cleanup |
| D-04 Publication | CURRENT, index, visibilité, ordinal de succès |
| D-05 Preuve | Hash, taille, schéma, stratégie, provenance, identité physique, chemin canonique |
| D-06 Plan de retrait | Successeur, source, index, cibles, ancêtres/conteneurs, possession, progrès, erreur |
| D-07 Projection reprise | Entrées complètes/interrompues/corrompues et réparations actuelles |

### Import

Sources archive.rs baseline : 3593–3905, 3919–4038, 6129–6177, 6752–6898.

Préparation pending/journal → validation segment, append/newline et sync_all avant progression/confirmation → TXT/SRT/manifeste vérifié/intention/staging (prêt, non actif) → commit sous verrou/catalogue attendu : plan cleanup non publié si utile, exports, CURRENT puis index → preuves/tentative cleanup, réconciliation remplacés, retrait réservation et état courant libéré.

CURRENT est un commit local distinct de la visibilité du catalogue R11. CURRENT sans activation index reste publication à récupérer. Les exports sont installés avant CURRENT : aucune simultanéité nouvelle de tous les fichiers n’est promise. Une erreur tardive peut suivre une publication durable ; le rejeu de la même génération conserve success_ordinal.

### Live

Source archive.rs:1015–1663. Verrou écrivain/pending → tentatives Reserved/Started/Retired (checksum/sync avant mémoire) → segment durable distinct de fenêtre confirmée → coverage contiguë/synchronisée avant curseurs → PCM couvert, MP3 sync/rename, snapshots TXT/SRT/manifeste vérifiés puis CURRENT. Alias et retrait pending best effort après commit.

Snapshots sources de vérité ; alias réparables. Coverage synchronisée retrouvable sans ACK reçu. La génération suivante peut rester refusée tant que le passage précédent n’est pas résolu.

### Retrait

Sources archive.rs:1846–1912, 3780–3885, 4477–4623, 4814–5326. Un plan prépublication published=false n’autorise rien. Après CURRENT, la découverte peut vérifier successeur, rejouer index/preuves et marquer publié sans supprimer les anciennes cibles. Commit/reprise explicite retire sous contrôles destination/source/successeur/index/cibles/ancêtres/identités. Suppression avant enregistrement durable du progrès : caractériser cet intervalle. Contenu inconnu : ne pas étendre l’autorité au conteneur entier. Publication réussie et nettoyage restant peuvent coexister ; pas de rollback destructeur ajouté.

| Invariant | Contrat |
| --- | --- |
| INV-01 | Identité attendue avant progression |
| INV-02 | Segment durable, fenêtre confirmée, commit local et visibilité catalogue distincts |
| INV-03 | Hash, identité physique et autorité de retrait non interchangeables |
| INV-04 | Rejeu conservant choix/séquences/effets propres au parcours ; pas d’activation universelle de CURRENT |
| INV-05 | Réparations ciblées des scans et leur ordre préservés |
| INV-06 | Successeur publié/preuves actuelles avant retrait ; découverte distincte de suppression |
| INV-07 | Formats, diagnostics contractuels, ordre d’effets et legacy inchangés |
| INV-08 | Tests préalables et ordre des chantiers obligatoires |

Un fragment sans newline peut être tronqué par la réparation actuelle ; une ligne complète corrompue relève d’un autre refus. Aucune garantie générale powerfail déduite.

## 5. Matrice statique pannes/couverture

Owner : rust_requirements_analyst. Chemins ci-dessous sous crates/whisper-adapters/. U=src/archive/tests/, I=tests/. R=état disque reconstruit, E=erreur simulée/refus, N=nominal. Les assertions ont été lues, jamais exécutées ici. Numéros de lignes sur baseline §1.

| PF / source archive.rs | Test/assertions constatés | Statut et complément |
| --- | --- | --- |
| PF-01 initialize_pending:6963–6984 | U/recovery resumed_prefix_copy_retries_after_pending_creation_without_duplication:722 : séquences 2 puis 3, copie sans duplication | R partiel ; pending absent/interrompu/tronqué/divergent |
| PF-02 persist_segment:3593–3632 | U/recovery segment_recovery_keeps_synced_prefix_and_drops_only_a_partial_tail:328 : ligne complète/newline conservée | R partiel ; ligne entière corrompue, fragment initial, queue>8Kio ; append/sync seulement seam existant |
| PF-03 prepare_publish TXT/SRT:3633–3698 | Pas de panne ciblée entre les deux trouvée | G-01 : partiels/temporaire/catalogue/retry |
| PF-04 manifeste:3699–3717 | Hooks before/after ; tests mécanisme sans preuve parcours | G-02 : absent/tronqué/artefact divergent |
| PF-05 intention/staging:3718–3733 | R7 construit fichiers, sans chaque interruption | G-03 : intention seule, TXT staged seul, SRT divergent |
| PF-06 exports/ownership:5561–5748 | U/recovery R7:2, assertions197–322 : SRT mêmes octets identité tierce conservée, pas CURRENT, ancien actif, reconfirmation/retry idempotent | E+R ; G-04 : JSON tronqué/version inconnue/branches v1-v2-v3/preuve avant export |
| PF-07 install_owned_export:5837–5916 | U/recovery R7:215–246 : after_final_copy en erreur, octets/identité conservés, pas CURRENT | E, pas kill malgré commentaire |
| PF-08 CURRENT/index:3845–3870 | U/replacement_and_segmented r11_recovery_between_current_and_index_keeps_legacy_visible_until_explicit_activation:237–372 : ancien seul avant résolution, nouveau seul après | R ; G-05 : erreur index/résolution répétée |
| PF-09 index/migration:5482–5518 | U/replacement_and_segmented r11_create_new_preserves_existing_publications_and_index_write_is_atomic:90–135 : deux actifs, write/read égaux | N seulement pour atomicité ; G-05 : absent/tronqué/tmp/rename échoué |
| PF-10 reconcile_replaced_exports:6244–6347 | U/replacement_and_segmented r11_replaced_export_reconciliation_never_moves_a_third_party_same_name_file:172–235 : tiers inchangé/non déplacé | E refus ; G-06 : déplacement TXT/SRT partiel/retry |
| PF-11 fin/réservation:3870–3895,2538–2551 | U/recovery published_running_row_is_removed_durably_and_recovery_scans_are_stable:539–648 : file nettoyée/scans stables | R connexe ; G-06 : réservation résiduelle/suppression refusée |
| PF-12 scan_generation:6752–6854 | U/recovery scan_preserves_healthy_entries_and_reports_each_corrupt_generation:352–536 : scans égaux, sain complet, corruption, traversal/transplant refusés | R ; assertions catégories/présence, pas tout message/mutation |
| PF-13 resume:3234–3592 | U/recovery resume_rejects_canonical_cross_destination_without_mutating_either_root:651 et copie préfixe:722 | E+R ; snapshots deux racines et absence duplication |
| PF-14 schéma/provenance:6855–6898,7448–7714 | U/archive_and_contracts pending_archive_versions_bind_strategy_and_keep_legacy_default:625–665 ; create_only_publication_returns_verified_internal_unified_artifacts_without_external_exports:30–249 : sentinelles/index/réservation conservés, divergences/traversal refusés | E+R présents |
| PF-15 identité Parakeet | I/parakeet_archive:445,611,806,872,932,993 : clés, retraits L2a/R5-2, GPU physique, source/modèle/préfixe ; pending/source conservés | E+R conditionnels ; NOT RUN sans variables ; mutation GGUF:333 ignorée |
| PF-16 live tentative/verrou:1034–1310 | I/live_archive attempt_journal_reserves_monotonically_and_enforces_transitions:20 ; a_live_group_has_one_archive_writer_at_a_time:69 | N+E ; G-07 : checksum/queue tentatives/frontière sync-ACK |
| PF-17 coverage:1310–1465,6628 | I/live_archive recovery_adopts_a_synced_silent_window_commit_without_a_durable_ack:567 : durable=confirmé8000, queue ignorée ; refus next-gen:97,138,205,266 | R ; G-07 : checksum/ordre coverage, segment avant coverage |
| PF-18 publication live:1485–1659 | I/live_archive passage_publishes_durable_pcm_mp3_and_confirmed_text_as_one_generation:328 : deux générations, ancien avant CURRENT, scans stables, résolution, alias réparé | R ; G-08 : rename MP3/snapshots partiels/manifeste-CURRENT invalides |
| PF-19 cleanup preuves:3780–3885,4477–4623 | I/reprocessing_cleanup replace_all_publishes_first_stops_partial_cleanup_and_resumes_only_on_request:739 : avant CURRENT/index, ancien avant commit, successeur actif et anciennes cibles après découverte | R conditionnel, réutiliser sans doublon |
| PF-20 cleanup progrès:4814–5326 | I/reprocessing_cleanup:504,531,555,739 : reparse/identité/source refusés, fichiers préservés/parent legacy conservé ; U/archive_and_contracts legacy cleanup:667 | E+R Windows ; G-09 : plan tronqué/progrès-absence/successeur-index divergents |
| PF-21 stockage:7253,7328,7337 | Pas de test ciblé trouvé provoquant sync/rename échoué | G-10 : closure streamed en erreur/obstacle rename/final-temporaire-retry |

Matrice des familles inspectées, pas preuve exhaustive de tous appels disque/tests/pannes OS. Aucun test cité ne démontre une coupure électrique ni un kill à ces frontières.

Préconditions : recovery/replacement cités non ignorés ; certaines assertions cfg(windows). live_archive utilise PCM/faux MP3, sans validation encodage. parakeet_archive peut retourner NOT RUN sans PARAKEET_L2_ARCHIVE_ROOT/RUNTIME_ROOT/MODEL/EN_WAV. reprocessing_cleanup requiert PARAKEET_RTC02_EXEC_ROOT, modèle/catalogue Turbo, faux worker compilé et primitives Windows. reprocessing_baseline::r11_application_baseline ignoré, vraies imports et garde historique. Build L04 spécial susceptible d’écrire C:/WhisperLive/L04-campaign/runs : exclu campagne standard sans accord spécifique. Exit0/feature activée ne prouvent pas scénario exercé.

## 6. Architecture cible

Owner : rust_architect, ARCH-ARCHIVE v1. Décisions techniques acceptées sous délégation de conception, à revoir ; aucun GO code.

| DEC | Choix/justification |
| --- | --- |
| DEC-T01 | archive.rs point d’entrée types/records ; enfants privés et blocs impl déplacés ; ni nouvelle crate ni service générique |
| DEC-T02 | Conserver import/live/retrait distincts, sans transaction multifichier/protocole commun nouveau |
| DEC-T03 | Helpers directs, états reconstruits et seams existants ; aucun trait/champ/hook/feature produit nouveau |
| DEC-T04 | Quatre extractions séquentielles ; pas de modifications concurrentes d’archive.rs |

### Partition

Préfixe crates/whisper-adapters/src/. Estimations sur blocs actuels, à mesurer après déplacement. 16 enfants, pas 16 tâches/rapports.

| Cible | Contenu | Lignes estimées |
| --- | --- | ---: |
| archive.rs | Modules/imports, records/états, types publics, constructeur/accesseurs, browse_treatment_folder, generation_path, court load_or_migrate_publication_index | 600–800 |
| archive/campaign.rs | Fonctions/statiques campagne, branches feature/no-op :32–512 | 490–550 |
| archive/storage.rs | write_replace_streamed, copy_replace_streamed, write_new_synced, write_replace_synced, temps/formatage | 120–180 |
| archive/source.rs | identify_source, segmented_pcm_manifest, sha256_file, source_output_samples*, hex | 230–300 |
| archive/journal.rs | persist_segment, lecture/visite/validation segments, préfixe unified, ligne bornée/réparation | 340–420 |
| archive/import.rs | Qualification pending mutable, confirmations VAD, prepare*, prepare_inner* :2632–2754,2838–3233 | 550–650 |
| archive/live.rs | impl LiveArchive, resolve_live_publication :1033–1665,2755–2822 | 730–850 |
| archive/publication.rs | prepare_publish, commit_publish, publish :3633–3906 | 300–380 |
| archive/publication/catalog.rs | Catalogue/verrou/réservations/reconfirmation/résolution/réconciliation/index/intention, hors orchestrateur court | 1050–1250 |
| archive/publication/exports.rs | Hardlink/copie/ownership/identité exports/vérification :5521–6128 | 650–750 |
| archive/recovery.rs | resume_from*, préfixe, scans import, validation pending/VAD, initialisation/diagnostics | 780–900 |
| archive/recovery/live.rs | Scan live/alias/CURRENT/manifeste/préfixe confirmé/JSONL borné :6349–6737 | 420–500 |
| archive/identity.rs | ArchiveQualificationKey, registre, identité job, lecture/published unified, artefacts/chemins/intention/provenance/PCM | 900–1100 |
| archive/cleanup.rs | API cleanup, chemins/read/write/remaining/preserved/comparison/chemin sûr | 260–350 |
| archive/cleanup/proofs.rs | Cibles/ancêtres/fichiers/successeur/conteneurs/reprise preuves/identités, cfg Windows et non-Windows | 570–680 |
| archive/cleanup/execution.rs | runtime_identity, run_cleanup_plan_locked, cfg Windows et non-Windows | 550–650 |

Fonctions historiques longues déplacées d’abord sans transformation de contrôle : prepare_inner_impl (~267), resume_from_with_qualification (~274), published_unified_files (~145), commit_publish (~153), run_cleanup_plan_locked (~512). Écarts nommés/non aggravés : pas de décomposition cachée ni suppression générale de lint. Cibles AGENTS de structure conservées pour le code nouveau.

### Dépendances, visibilité et concurrence

Types/records dans parent ; enfants accèdent aux champs privés. Helpers partagés pub(super) au besoin, imports explicites. Conserver API/signatures/chemins : ArchiveStore, LiveArchive, ArchiveQualificationKey, SegmentRecord, ArchiveEntry, PublicationLock, UnifiedArchiveResumeCheckpoint, scan_live_recoveries, sha256_file. Conserver visibilité actuelle des réexports campaign_gate, record_campaign_operation_result, assert_campaign_archive, source_output_samples et réexport SegmentRecord par journal.rs de la crate.

Tests inclus depuis archive/tests.rs restent archive::tests avec imports parent ; publication_support.rs garde accès cfg(test). Nouveaux tests séparés, pas d’API produit rendue publique. Feuilles stockage/source/journal sans orchestration workers. Aucun archive→worker_ipc/desktop/UI ; aucun verrou/thread/canal/runtime/unsafe/dépendance nouveau.

load_or_migrate_publication_index reste orchestrateur parent car il appelle ArchiveStore::scan_raw ; pas retour catalogue feuille vers reprise. Opérations synchrones potentiellement bloquantes disque/hash/verrou ; ordre acquisition et propriétés concurrence inchangés.

Manifestes actuels : huit crates dont whisper-worker-common, édition2024, rust-version1.98.1 ; cartes sept crates historiques. adapters conserve unsafe_code=deny. Aucune modification manifeste/Cargo.lock.

Contrôles futurs : comparer signatures/réexports/consommateurs, imports et direction des dépendances ; absence nouvelles features, unsafe ou changements de format ; conserver cfg et accès tests ; format/build/tests/lints dans campagne autorisée.

## 7. Caractérisation préalable

| Groupe | Scénarios minimaux avant extraction |
| --- | --- |
| C-PUB / PF03–05,08–09,21 | Closure streamed en erreur ; obstacle rename ; TXT seul, manifeste absent/tronqué/divergent, intention/staging partiels, index absent/tronqué/tmp, retry/visibilité |
| C-OWN / PF06–07,10–11 | Ownership v1/v2/v3, JSON tronqué/version inconnue, réconciliation partielle, réservation résiduelle, tiers mêmes nom/octets |
| C-REC / PF01–02,12–18 | Pending/journal tronqués, fragment initial, queue>8Kio, ligne entière corrompue, checksum/ordre coverage/tentative, snapshots live partiels/manifeste-CURRENT invalides |
| C-RET / PF19–20 | Plan tronqué, objet disparu/progrès non inscrit et inverse, successeur/index divergents, découverte sans suppression, source/tiers conservés |

Fixtures disque, helpers/API actuels et install_publication_exports_with_copy_hook existant. Aucun mécanisme d’injection produit nouveau. Pour chaque scénario : sortie/erreur réelle baseline, état disque/catalogue, données conservées/retirées, identité sentinelles, répétition. Anomalie signalée séparément sans correction.

sync_all non reproductible sans nouvelle injection reste NON EXERCÉ ; reconstitution après sync ne prouve pas panne d’appel. Pas garantie powerfail ou fsync dossier ajoutée. Le plan distingue lacune obligatoire caractérisable et limite technique non démontrée, sans prétendre fermer cette dernière.

## 8. Campagnes futures

Garde existante : target/parakeet-tests/refacto-worker-ipc-f3/guard.ps1, interface -Mode SelfTest|Run -CargoArguments, racine F3 codée en dur. Pas réutilisable immédiatement pour archive : adaptation bornée à inscrire au plan et au GO, aucune dérogation historique transférée.

Avant tout Cargo : garde compatible autorisée, auto-tests/surveillance, espace≥30Gio, CARGO_TARGET_DIR canonique partagé, CARGO_HOME/TEMP/TMP sous racine lot, CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only, offline/locked conformément à AGENTS. Pas de lancement pendant cadrage.

Commandes futures sous lanceur gardé : tests archive::tests, intégration live, Parakeet/reprocessing configurés, format/build/Clippy applicables. Le plan précise filtres/données/préconditions/coût/observations et comptage scénarios exercés. Variables manquantes/NOT RUN ne ferment aucun scénario obligatoire. Aucun modèle/audio utilisateur réel modifié.

## 9. Risques, preuves, traçabilité et readiness

| Risque | Réduction |
| --- | --- |
| Visibilité avancée/ancien masqué | CURRENT/index/catalogue/états intermédiaires comparés |
| Duplication/perte préfixe | Journaux limites/séquences/répétitions/octets |
| Hash assimilé possession | Sentinelles mêmes octets identité distincte |
| Retrait sans preuve/confirmation | Source/successeur/index/identités/conteneurs, découverte sans retrait |
| Exit0 assimilé couverture | Préflight et scénarios réellement exercés |
| Correction/migration implicite | Baseline préalable/diff mécanique/anomalies séparées |
| Simulation assimilée kill/powerfail | Stimulus et limites explicites |
| Déplacement changeant imports/cfg/visibilité | API/consommateurs/tests parents/branches conservés |
| Taille fichier masquant fonction risquée | Écarts historiques nommés, non transformés silencieusement |
| Campagne hors racine | Exclusion L04/garde adaptée/sans dérogation transférée |

| Preuve | Classe / état |
| --- | --- |
| Modules privés et dépendances existantes suffisants | DESIGN_FEASIBILITY : avis documentaire favorable proposé, sous revue |
| C-PUB/OWN/REC/RET baseline | PRODUCT_VALIDATION : NOT RUN, préalables extractions |
| Comparaison après déplacement/consommateurs/format/build/lints | PRODUCT_VALIDATION : NOT RUN |
| Fenêtre Windows/moteur natif/paquet | DELIVERY_QUALIFICATION : NOT RUN, aucun binaire/recette demandé |

| CAP / REQ / AC | Domaine | Architecture / PF |
| --- | --- | --- |
| 01 | D01,INV01/02/04 | import/journal/recovery ; PF01/02/13 |
| 02 | D02,INV02/05 | live/recovery-live ; PF16–18 |
| 03 | D03/04,INV02–04 | publication/catalog/exports ; PF03–11 |
| 04 | D07,INV04/05 | recovery ; PF12/13/18/19 |
| 05 | D05,INV01/03 | identity/source ; PF14/15 |
| 06 | D06,INV03/06 | cleanup ; PF19/20 |
| 07 | INV07/08 | C-PUB/OWN/REC/RET, PF21 |
| 08 | Tous | Partition/API, déplacement constant |
| 09 | INV08 | Préflight/préalables/concurrence |

Questions : Q-01 détail coordinateur, actualiser préalables/concurrence avant code, arbitrage si collision. Q-REQ-01 mécanismes résolu par DEC-T03/groupes ; résultats baseline à produire. Q-REQ-02 détail plan writer : préflight/filtres/comptage avant lot tests. Aucune décision produit nouvelle ; correction/garantie/contrat/injection nouveaux demandent décision distincte avant lot affecté.

Readiness applicable : scope/use_cases/errors/domain/dependencies/persistence/concurrency/platform/acceptance/risks couverts §§2–9, proposés à la revue indépendante. Railguard nouveau N/A : ni demandé ni activé ; règles AGENTS existantes restent applicables. Installation/packaging UI hors changement justifient absence de nouveau parcours associé.

DESIGN READY proposé après revue indépendante du fichier Git, sans constat requis ni décision structurante manquante. Plan futur ≤10 tâches globales, caractérisation puis quatre extractions, avec chemins/prérequis DOCUMENT-CODE-EXECUTION/résultats/arrêts/retour arrière proportionné. Pas reset/nettoyage global.

Avant extraction : plan accepté, GO précis, Git/préalables/concurrence actualisés, garde compatible autorisée, caractérisation requise exercée sur baseline. Ce cadrage ne ferme aucune lacune d’exécution.
