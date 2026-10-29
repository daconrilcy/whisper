Avancée : candidat complet relu indépendamment ; deux constats importants en correction au premier cycle.
Blocage : aucun arbitrage restant pour ces corrections, fichier de tests complémentaire autorisé.
Décision : corriger MF-R1/MF-R2 puis faire vérifier leur fermeture par le même reviewer ; aucun push.

# Whisper — Plan de refactorisation des points d'entrée

## 1. Identité et autorité

**Plan : P-MAIN-FACADES-01, version 1, 29 octobre 2026.**

**Design parent : D-MAIN-FACADES-01**, `docs/cadrage/evo-main-facades.md`, décision de planification versionnée au commit `e58bfa2fc2118c15cf815656d3d3fbec0af42a1c`. Revue du contenu de design : CLEAN sur `a6e6031e`, clôture documentaire `2bd9fced`.

**Base produit :** `301fbbeaaba8cede4ce079828125ba271c9928af`, branche `evo-startup-perf`. **Statut : CLEAN — PLANS documentaire, en attente de validation utilisateur et des prérequis d'exécution.**

La demande « redige le plan de refacto a partir de ce cadrage » autorise sa rédaction à partir du design relu. Elle n'autorise ni implémentation, ni campagne, ni modification de garde, ni push. L'auteur principal écrit et commite localement ce document. Contribution : `rust_plan_writer`, `/root/plan_facades`, P-MAIN-FACADES-01-v1, base documentaire ci-dessus ; apport brut restitué dans la session avant intégration. La présente intégration ne revendique pas une copie littérale de cet apport. Le reviewer indépendant juge les constats et le statut du plan.

Sources lues : cadrage intégral, AGENTS racine et workers, manifestes, branches ciblées des entrées, tests existants, refus release des hooks et gardes citées en §3. Inventaire en lecture seule, aucune preuve runtime. État initial : aucun changement suivi, `docs/architecture/`, `models/`, `sound example/` non suivis et préservés. La carte d'architecture est une orientation statique confrontée aux sources, pas une autorisation de modification.

Objectif : rendre les cinq main.rs lisibles comme façades, extraire les traitements des trois workers par responsabilité et préserver leur comportement. Un seul lot de réalisation proposé, **huit tâches séquentielles**. Les jalons CPU/GPU/Parakeet sont des sorties vérifiables internes au lot, sans nouvelle phase documentaire. Première façade et parcours minimal démontrable à T3.

## 2. Périmètre et invariants

Le design §5 fait référence pour responsabilités et consommateurs. DEC-T01 à DEC-T05 inchangées : modules privés dans les binaires actuels, aucun lib.rs, crate commune, dépendance, port spéculatif ou API publique entre crates.

Conserver `crates/whisper-desktop/src/main.rs` et sa composition, ainsi que `crates/whisper-bootstrap/src/main.rs`, sans activer l'installation évoquée en commentaire. Aucun plafond arbitraire de lignes pour les façades.

Ne pas modifier manifestes, Cargo.lock, protocoles core, UI/adapters, modèles, DLL, packaging ou frontières FFI. Moteurs existants conservés ; liaisons strictement nécessaires bornées en §4.

Contrats sensibles : deux writers CPU distincts dont writer borné vers ipc_output ; framing 1 Mio séparateur compris, refus excessif/partiel avant JSON ; canal Turbo capacité8 ; cache et attestation GPU sur hit ; corrélations/contrôles d'arrêt/ordre des effets ; Stop live Turbo finalisant MP3 distinct de Shutdown ; EOF Parakeet distinct de Shutdown ; last_request_id et job_attempts mis à jour aux points actuels ; InvalidTimestamp/NativeCallStarted/racines/identités/cfg/instrumentation/refus release hooks inchangés ; résumé JSON stdout --decode-only conservé. Le parent garde choix et repli moteur. Aucun changement fonctionnel ni seuil de performance.

## 3. Entrées du lot et conditions d'exécution

| Type | Entrée | Condition |
|---|---|---|
| DOCUMENT | Design parent, revue CLEAN, décision utilisateur | Version Git vérifiée ; aucun changement technique implicite |
| DOCUMENT | Présent plan et revue PLANS | Plan accepté, chemins/scénarios identifiés |
| DOCUMENT | Accord précis de session | Implémentation, campagnes et racines autorisées ; les lectures seules ne valent pas droit d'écriture |
| CODE | Trois workers/tests existants | Base comparée au code actuel, écarts ordinaires préservés, responsabilités coordonnées |
| EXECUTION | Environnement/espace | ≥30 Gio libres ; dépendances/ressources disponibles hors ligne |
| EXECUTION | Garde applicable | Préflight, auto-tests de refus et surveillance conformes à AGENTS |
| EXECUTION | Référence avant extraction | Binaires/configurations de référence préservés et résultats utiles disponibles, sinon blocage/réserve explicite |

Racines d'écriture **proposées**, ni créées ni autorisées par la rédaction :

- `C:\dev\whisper\target\parakeet-tests\main-facades\` : temporaires, caches privés, fixtures, sorties ;
- `C:\dev\whisper\target\parakeet-tests\cargo-shared\` : unique sortie Cargo.

Dans la première : reference/, candidate/, fixtures/, logs/, cargo-home/, temp/, tmp/. Identifier chaque configuration dans les sorties. Après chaque build utile, préserver l'EXE dans le sous-dossier de sa configuration avant qu'un autre build n'écrase l'EXE partagé. Garder les ensembles référence et candidat distincts. Ne pas créer de checkout supplémentaire : avant extraction, conserver les binaires de référence construits depuis la base et les exécuter ensuite avec le même harness si nécessaire.

Avant **chaque Cargo**, vérifier canoniquement CARGO_HOME/TEMP/TMP sous la racine du lot, CARGO_TARGET_DIR exactement sur la racine partagée ; variables absentes ou jonctions hors racine = refus. CARGO_INCREMENTAL=0 et CARGO_PROFILE_DEV_DEBUG=line-tables-only. Aucun cache utilisateur en écriture ; copie privée du contenu nécessaire hors ligne seulement dans la racine autorisée.

Gardes observées : `target/parakeet-tests/micro-level/guard-micro-level.ps1` et `target/parakeet-tests/parakeet-unified/lot-04/validate-pu-l4.ps1`, avec racines/modes figés. **Aucune réutilisation directe pour ce lot démontrée.** Ne pas les copier/adapter tacitement.

**Q-EXEC-01 — bloquant avant tout Cargo :** coordinateur responsable, échéance avant T1 exécutée, présenter une garde réellement applicable et obtenir l'accord précis nécessaire. Si une modification de garde ou un autre artefact est requis, arbitrage distinct avant réalisation. Aucune ancienne dérogation applicable. Ce blocage d'exécution ne bloque pas la rédaction/revue documentaire du plan.

## 4. Chemins du lot

Périmètres proposés à accepter, pas autorisation actuelle. Chaque fichier créé porte la responsabilité ou le scénario indiqué. Tests séparés du code de production et sous leurs cfg d'origine ; aucun champ/branche produit pour les tests. Les helpers sont locaux aux tests d'intégration de leur crate, sans nouvelle crate commune.

### P-CPU — préfixe crates/whisper-worker-cpu/

| Nature | Chemins |
|---|---|
| Modifier | src/main.rs |
| Créer lancement/boucle/cache | src/startup.rs, src/worker.rs, src/engine_cache.rs |
| Créer sessions | src/session/mod.rs, src/session/import.rs, src/session/live.rs, src/session/decode.rs, src/session/control.rs |
| Créer contrôles/sortie/instrumentation | src/integrity.rs, src/diagnostics.rs, src/qualification.rs, src/memory_qualification.rs, src/ipc_output.rs |
| Adapter strictement | src/ipc.rs pour alias de transport/imports nécessaires ; writer existant inchangé |
| Créer tests unitaires | src/ipc_output/tests.rs, src/worker/tests.rs, src/session/control/tests.rs |
| Adapter test existant | tests/reusable_imports.rs : répertoire propre sous racine de test explicite |
| Créer tests de contrat | tests/worker_contract.rs et, si nécessaire, tests/support/worker.rs |
| Conserver | src/decoder.rs, src/encoder.rs, src/native_engine.rs ; tests/live_worker.rs témoin existant |

### P-GPU — préfixe crates/whisper-worker-gpu/

| Nature | Chemins |
|---|---|
| Modifier | src/main.rs |
| Externaliser inline | src/decoder.rs, src/encoder.rs, src/ipc.rs |
| Créer lancement/boucle/cache/commandes | src/startup.rs, src/worker.rs, src/engine_cache.rs, src/gpu_commands.rs |
| Créer sessions | src/session/mod.rs, src/session/import.rs, src/session/live.rs, src/session/decode.rs, src/session/control.rs |
| Créer contrôles/instrumentation | src/integrity.rs, src/diagnostics.rs, src/qualification.rs, src/memory_qualification.rs |
| Créer tests unitaires | src/decoder/tests.rs, src/ipc/tests.rs, src/worker/tests.rs, src/diagnostics/tests.rs, src/session/control/tests.rs, src/engine_cache/tests.rs |
| Créer tests de contrat | tests/worker_contract.rs et, si nécessaire, tests/support/worker.rs |
| Conserver | src/native_engine.rs, src/native_ffi.rs, src/gpu_attestation.rs, src/gpu_qualification.rs ; tests/gpu_selection_native.rs |

### P-PARAKEET — préfixe crates/whisper-worker-parakeet-cpu/

| Nature | Chemins |
|---|---|
| Modifier | src/main.rs |
| Créer lancement/session/configuration | src/startup.rs, src/session.rs, src/args.rs, src/runtime_paths.rs, src/gpu_identity.rs, src/integrity.rs |
| Créer traitements/transport/traces | src/transcription.rs, src/decode_only.rs, src/reference_decoder.rs, src/ipc.rs, src/timing.rs |
| Créer tests unitaires | src/runtime_paths/tests.rs, src/runtime_paths/personal_tests.rs, src/gpu_identity/tests.rs, src/ipc/tests.rs, src/reference_decoder/tests.rs |
| Créer tests de contrat | tests/worker_contract.rs et, si nécessaire, tests/support/worker.rs |
| Conserver | src/decoder.rs, src/native_engine.rs, src/native_ffi.rs ; tests/native_import.rs, tests/decoder_parity.rs |

### Liaisons et documentation

Conserver native_engine→memory_qualification par le module racine homonyme ; Parakeet crate::record_r7_timing par import de déclaration racine. Aucun moteur modifié seulement pour réparer une extraction. Si un changement moteur/FFI devient indispensable au-delà de ces liaisons, arrêter et revenir à l'architecte sans étendre les chemins.

Documentation du lot : présent plan pour suivi/résultats/réserves et cadrage pour décisions de statut. Aucun nouveau rapport ou manifeste systématique. Visibilités privées/pub(super) ou pub(crate) au plus, selon consommateurs du design. Répartir les grosses fonctions aux responsabilités sans réordonner les branches ; cibles AGENTS appliquées au code nouveau, dépassements historiques éventuellement conservés justifiés dans le suivi existant.

## 5. Tâches et dépendances

| Tâche | Entrées et travail | Sortie démontrable / contrôles |
|---|---|---|
| T1 — Préparer la référence | DOCUMENT design/plan/accord ; CODE base ; EXECUTION garde. Relever Git/espace/responsabilités/ressources. Préparer fixtures, construire et préserver références par configuration avant extraction ; adapter seulement routage du test reusable_imports. | Références utiles préservées, comparateur/scénarios fixés ; sinon arrêt ou NOT RUN attribué. Aucun changement produit. |
| T2 — Transport/instrumentation CPU | CODE/EXECUTION T1. P-CPU writer borné, qualification/mémoire, tests/déclarations. Préserver writers distincts et variantes no-op. | Bornes/framing/configurations CPU ; plus de tests dans parties extraites du main. |
| T3 — Façade CPU | CODE T2 et EXECUTION contrôles utiles T2. P-CPU lancement/boucle/cache/admission/import/live/decode/control/intégrité/diagnostics. Déplacer arrêt, ajouter cas manquants. | Première façade complète, refus modèle/arrêt inactif ; import réutilisable/live sous autorisation. |
| T4 — Modules GPU | CODE/EXECUTION T3. P-GPU decoder/encoder/ipc, tests inline et instrumentation ; aucun report mécanique CPU. | Modules externes/tests actifs, décodage/framing préservés. |
| T5 — Façade GPU | CODE/EXECUTION T4. P-GPU lancement/boucle/cache/sessions/commandes/diagnostics ; préserver attestation hit et corrélations. | Façade GPU, sonde distinguée de l'inférence ; refus/réutilisation couverts. |
| T6 — Contrôles Parakeet | CODE/EXECUTION T5 et Q-EXEC-02 résolue avec stratégie de couverture autorisée. P-PARAKEET args/racines/GPU/intégrité/IPC/timing/decode-only/référence/tests. Aucune racine produit élargie. | Tests déplacés sans changement implicite d'assertion/cfg, résultats/limites par configuration ; contrôles/profils, résumé decode-only et création exclusive conservés. |
| T7 — Façade Parakeet | CODE/EXECUTION T6. P-PARAKEET lancement/session/transcription ; ordre compteurs/refus/callbacks/arrêts conservé. | Façade Parakeet ; refus puis requête suivante, EOF/Shutdown distingués. |
| T8 — Vérifier et faire revoir | CODE T3/5/7 et EXECUTION contrôles utiles acquis. Vérifications croisées manquantes/affectées seulement, cinq façades/diff, revue indépendante et corrections bornées. | Bilan par scénario/configuration, verdict indépendant, réserves explicites. |

Graphe : T1→T2→T3→T4→T5→T6→T7→T8. Aucun travail parallèle sur ces contrats ou target partagé. Owner code : rust_implementer après autorisation. Revue : rust_review_coordinator avec reviewer indépendant, corrections par implémenteur ; maximum deux cycles. Jalons T3/5/7 compilables et vérifiés, pas permission de continuer après échec.

## 6. Données et ressources

| ID | Donnée proposée / observation | Lectures et écritures futures |
|---|---|---|
| F-01 | Trames valides/incomplètes/excessives, limite, limite+1, versions/corrélations fabriquées en Rust | Mémoire ; captures sous campagne |
| F-02 | Silence PCM16 mono16kHz de2s | Généré sous fixtures/ |
| F-03 | Dix secondes de sound example/spkr0.wav (RIFF PCM16 mono16kHz, header44 observé) | Source intacte, dérivé320044octets sous fixtures/ ; header corrigé par helper Rust test |
| F-04 | WAV silence60min et header déclarant60min+1échantillon | Générés campagne ; ~110Mio chacun si matérialisés, samples f32 ~220Mio pour60min |
| R-CPU | models/ggml-large-v3-turbo.bin : taille1624555275 observée | Lecture seule, identité à vérifier avant campagne |
| R-PARA | Modèle Parakeet installé ou copie Test déjà autorisée ; taille modèle dépôt713975456 observée | Lecture seule, pas substitution tacite d'un modèle par l'autre |
| R-NATIVE | %LOCALAPPDATA%/Whisper/native/cpu/ et native/gpu/ : ensembles5/9 DLL présents | Présence/tailles seulement constatées ; identité à contrôler avant usage, aucune modification |
| R-TEST | Racines Test admises actuellement | Lecture seulement ici, aucune nouvelle destination d'écriture implicite |

Tests de référence : PARAKEET_L2_FR_WAV et PARAKEET_L2_EN_LONG_WAV nécessaires. Pour ces tests purs seulement, PARAKEET_L2_RUNTIME_ROOT peut désigner fixtures/ de la campagne : ils ne lancent pas le contrôle runtime du worker. Le cas dépassement écrit/supprime son propre fichier sous cette racine. Ne pas réutiliser cette valeur pour prétendre lancer un worker Test.

Référence et candidat natifs utilisent leurs binaires respectifs avec mêmes modèle/DLL/audio/configuration. Aucun corpus annoté nouveau ni garantie de qualité supplémentaire. Les ressources installées sont des entrées en lecture ; aucune copie ou mutation d'installation non autorisée.

## 7. Commandes proposées et configurations

Toutes **NOT RUN**. Cwd C:\dev\whisper, PowerShell. Charges utiles exclusivement via garde applicable après préflight/accord, jamais autorisation de Cargo direct. Package et binaire portent le même nom pour les trois workers.

```text
cargo fmt --all -- --check
cargo build --offline --locked -p <package> --bin <binaire> [--features <features>]
cargo test --offline --locked -p <package> --bin <binaire> [--features <features>] -- --test-threads=1
cargo clippy --offline --locked -p <package> --all-targets [--features <features>] -- -W clippy::too_many_lines -W clippy::cognitive_complexity -W clippy::too_many_arguments -W clippy::type_complexity
```

Format workspace préalable défaillant hors périmètre : consigner sans modifier ces fichiers. Lints structurels relus selon cibles/exceptions du design ; aucune CI inventée ni --fix.

### Turbo CPU puis GPU

Quatre configurations développement : défaut ; l01-memory-qualification ; recette-hooks ; combinaison l01-memory-qualification,recette-hooks. Builds/tests binaires distincts, puis Clippy configurations touchées sans répéter natif inutilement.

Release : build normal sans hooks ; build --release --features recette-hooks **refusé comme prévu**, avec diagnostic `recette-hooks is forbidden in Cargo release profiles`. Un autre échec ne satisfait pas ce contrôle, aucun build hooks livré. Prévoir ce contrôle négatif comme résultat attendu identifié dans le lanceur approuvé ; l'échec d'une commande censée réussir impose l'arrêt.

```text
cargo test --offline --locked -p whisper-worker-cpu --test live_worker
cargo test --offline --locked -p whisper-worker-gpu --test gpu_selection_native
cargo test --offline --locked -p whisper-worker-cpu --release --test reusable_imports cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache -- --ignored --exact --test-threads=1
```

Dernière commande : WHISPER_TEST_REUSE_MODEL et nouvelle racine explicite après adaptation du test. Contrat de test proposé : WHISPER_TEST_WORK_ROOT, absence/hors campagne = refus, sous-dossier unique par exécution. Conserver assertions model_cache=load puis model_cache=hit, deux imports même processus et fin corrélée. Aucun chemin target/whisper-reusable-imports-<pid> implicite restant dans la campagne. La sonde GPU peut réussir avec refus CUDA : cela ne prouve pas l'inférence.

### Parakeet

Trois features indépendantes, huit ensembles obligatoires de compilation des branches affectées et d'inventaire des tests : défaut ; parakeet-cuda ; parakeet-qualification-reference ; parakeet-personal ; CUDA+référence ; CUDA+personnel ; référence+personnel ; CUDA+référence+personnel (utiliser noms complets dans --features). Aucun --all-features remplaçant la matrice. Ces ensembles ne sont pas présentés comme huit suites de tests entièrement vertes. Compiler ne prouve pas l'admission de chaque profil runtime.

**Incompatibilités préexistantes identifiées statiquement (PL-MF-01).** Abréviations C=parakeet-cuda, R=parakeet-qualification-reference, P=parakeet-personal ; L2=accepts_only_the_two_canonical_l2_test_roots, QUAL=qualification_profiles_are_confined_to_l2b.

| Configuration | Écart relevé dans ces deux tests existants |
|---|---|
| Défaut | Aucune incompatibilité identifiée dans ces deux tests |
| C | Aucune identifiée ; L2/QUAL inactifs |
| R | L2 rejette L2b alors que R l'ajoute aux racines |
| P | L2 attend L2a alors que la liste Test est vide en personnel |
| C,R | QUAL attend single-pass L2a admissible alors que C admet L3 |
| C,P | Aucune identifiée dans ces deux tests, tous deux inactifs |
| R,P | L2 et QUAL attendent des racines Test alors que la liste est vide |
| C,R,P | QUAL attend single-pass L2a admissible alors que la liste Test est vide |

Sources : main.rs Parakeet:758–784,837–865 et fonctions appelées:880–902 à la base produit. Ce ne sont pas des résultats de test : **NOT RUN — incompatibilité préexistante identifiée**. Déplacer ces tests avec assertions/cfg d'origine ; relier explicitement leurs noms qualifiés avant/après extraction. Aucune correction de ces assertions ou cfg autorisée par ce plan.

À T1, après résolution de Q-EXEC-01 seulement, compiler les tests de chaque ensemble avec `cargo test --offline --locked -p whisper-worker-parakeet-cpu --bin whisper-worker-parakeet-cpu [--features <ensemble>] --no-run`, puis inventorier avec la même commande sans --no-run, terminée par `-- --list`. Garder inventaires/diagnostics ; aucune de ces commandes ne prouve une assertion réussie. Avant de lancer les commandes de tests binaires génériques §7 dans une configuration affectée, appliquer cette stratégie, pas une suite complète aveugle.

Fixer avant lancement la sélection et ses exclusions, avec noms exacts issus de l'inventaire : framing, parser CUDA, racines personnelles et décodeur de référence si fixtures disponibles. Aucun --skip global ni nouveau #[ignore] pour masquer ces écarts ; la sélection n'est pas la suite entière. Si une suite complète autorisée échoue, arrêter selon AGENTS et garder les résultats ; distinguer défaut préalable/régression sans reclasser l'échec PASS ou l'absorber pour poursuivre.

**Q-EXEC-02 — entrée de T6 et clôture S-09 :** coordinateur responsable, présenter les incohérences et la stratégie précise de couverture à l'utilisateur avant la tâche dépendante. Corriger assertions/cfg historiques demande un périmètre test-only explicitement autorisé et un plan mis à jour ; cela ne découle pas de l'extraction. Une sélection limitée acceptée reste PARTIAL pour configurations non couvertes et ne clôt pas S-09 entièrement. Sans résolution, arrêter avant T6 ; préserver les preuves déjà acquises. À T8, comparer inventaires et résultats référence/candidat par configuration ; nouvelle régression = arrêt, incompatibilités historiques visibles, jamais déclarées résolues par déplacement.

Sans personnel : CPU admet racines L2a actuelles ; référence ajoute L2b ; CUDA admet L3a/L3b. Profils de qualification exigent L2b : CUDA+référence compilable ne promet pas profil de qualification exécutable. Avec personnel : runtime publié ou enfant direct .staging, single-pass seulement ; feature référence n'autorise pas profils privés dans ce runtime.

Sous référence, préparer F-03/F-04 et variables **avant tests binaires** : tests actifs, non ignorés. Garder parité courte, lecture60min et refus dépassement dans reference_decoder/tests.rs. Le test non ignoré additive_gpu_fields_default_when_reading_existing_cpu_ipc_frames de tests/native_import.rs peut être ciblé séparément. Aucun --ignored global sur ce fichier ni recette écrivant sous runtime canonique lancée aveuglément.

## 8. Scénarios et observations attendues

Nouveaux tests/worker_contract.rs : cas avec moteur réel ignorés explicitement par défaut, filtre exact dans campagne native autorisée. Cas contrôlés exécutables sans modèle seulement si branche atteinte sans chargement. Helpers peuvent choisir EXE de référence/candidat, capturer stdout/stderr et borner leurs propres enfants. Aucun hook/champ produit nouveau. Variables locales de harness proposées : WHISPER_TEST_WORKER_EXE et WHISPER_TEST_WORK_ROOT ; ressources moteur via paramètres/protocoles existants. Référence/candidat exécutés avec mêmes données et cfg ; aucun défaut implicite vers EXE d'installation.

Les noms de tests ci-dessous sont des cibles nouvelles à créer dans worker_contract.rs du worker concerné. Les tests unitaires déplacés conservent leurs noms/assertions et cfg. Commandes ciblées : `cargo test --offline --locked -p <worker> --test worker_contract <nom> [--features <features>] -- --exact --test-threads=1` ; options Cargo avant `--`, ajout `--ignored` après pour les seuls cas natifs explicitement ignorés. Présence du cas et nombre réellement exécuté vérifiés ; zéro test n'est pas PASS.

| Scénario | Tâche / acceptation | Chemins et données | Observation / méthode |
|---|---|---|---|
| S-01 Façades/responsabilités | T3/5/7/8 ; AC-01/02/05 | cinq main lus, P-CPU/GPU/PARAKEET | Revue intégrale, déclarations/lancement seuls, correspondance modules/consommateurs, tests sortis |
| S-02 Bornes IPC | T2/4/6 ; AC-03/04 | ipc_output/tests, ipc/tests ; F-01 | Tests binaires, limite/séparateur inchangés, limite+1/EOF partiel avant JSON, writer CPU distinct intact |
| S-03 Turbo inactif/refus | T3/5 ; AC-03/04 | worker/tests, control/tests, live_worker, worker_contract ; F-01 | turbo_idle_and_refusal_contract : Shutdown inactif sans erreur/Stopped, refus modèle avant Ready, versions/corrélations ; contrôles take_stop couverts unitairement |
| S-04 Turbo import/cache | T3/5 ; AC-03/04 | reusable_imports, worker_contract ; cache/session/native_engine ; F-02/03,R-CPU | turbo_reusable_import_contract : deux sessions même processus, load/hit, mêmes événements ; GPU attestation hit distincte, contrôle unitaire engine_cache + natif |
| S-05 Turbo live arrêt | T3/5 ; AC-03/04 | worker_contract ; session/live,encoder,control ; F-02/03,R-CPU | turbo_live_stop_and_shutdown_contract : Stop finalise/paquets/Stopped ; Shutdown distinct, source intacte |
| S-06 GPU identité/sonde | T5 ; AC-03/04 | diagnostics/tests, engine_cache/tests, gpu_selection_native, worker_contract ; native_ffi lu | gpu_identity_contract : refus typé/corrélé, bonne cible, sonde distincte inférence ; cache comparé selon critères réels |
| S-07 Parakeet refus suivant | T7 ; AC-03/04 | worker_contract ; session/ipc/transcription/native_engine ; F-01/03,R-PARA/NATIVE | parakeet_refusal_order_contract : identité invalide consomme request_id au même point, SourceHashMismatch réserve tentative ; requête suivante reproduit refus/acceptation |
| S-08 Parakeet erreurs/arrêt | T6/7 ; AC-03/04 | ipc/tests, worker_contract ; startup/session ; ressources si Ready requis | parakeet_errors_and_shutdown_contract : JSON/version→Failed/poursuite, framing→fatal/code2, Shutdown→Stopped, EOF sans Stopped |
| S-09 Racines/profils | T6 ; AC-04/06 ; Q-EXEC-02 | runtime_paths/tests, personal_tests, gpu_identity/tests, args/startup | NOT RUN ; couverture complète conditionnée à Q-EXEC-02. Compilations/inventaires/tests ciblés ne ferment que sous-cas réellement vérifiés ; mêmes admissions/refus/profils/parser NVIDIA |
| S-10 Decode-only | T6 ; AC-03/04 | worker_contract ; decode_only/runtime_paths/decoder ; F-03 | parakeet_decode_only_contract : même PCM/résumé JSON, création exclusive refuse sortie existante sans écrasement |
| S-11 Référence/décodage | T4/6 ; AC-03/04 | decoder/tests, reference_decoder/tests ; F-03/04 | Tests binaires, fenêtres contiguës/bornées, parité courte,60min admises et dépassement refusé |
| S-12 Native Parakeet | T7 ; AC-03/04/06 | worker_contract ; transcription/native_engine/native_ffi ; F-03,R-PARA/NATIVE | parakeet_native_transcription_contract : backend CPU/CUDA effectif, identités/résultats, InvalidTimestamp distinct et callback CUDA au bon point |
| S-13 Bilan/périmètre | T8 ; AC-05/06 | Git, présent plan, cinq façades et captures campagne | Chaque scénario/config PASS/FAIL/NOT RUN avec branche atteinte et limites, chemins justifiés |

S-04/05/07/08/12 peuvent nécessiter moteur réel ; aucune couverture prétendue via seuls tests sans modèle. Les témoins InvalidTimestamp et attestation/cache utilisent les fonctions et points d'injection existants lorsqu'ils existent ; sinon comparer refus/validation de la frontière extraite par helper de test simple, sans modifier FFI ou ajouter branche produit. Un chemin natif non atteint reste NOT RUN, même si son contrôle pur réussit.

S-10 : fixture personnelle limitée à l'enfant compilé parakeet-personal ; dossier vide <campagne>/decode-only/localappdata/Whisper, LOCALAPPDATA et PARAKEET_L2_RUNTIME_ROOT fixés uniquement pour cet enfant, PCM dans cette fixture. Branche --decode-only avant charge modèle/DLL ; ni environnement global ni installation/routage application modifiés, ni copie modèle/DLL. Proposition de fixture de test soumise à validation du plan, aucune création aujourd'hui.

S-07/08/12 : privilégier profil personnel et runtime installé en lecture seule, captures du harness sous campagne, traces optionnelles désactivées. Si la garde ne peut satisfaire les conditions AGENTS ou si le runtime impose une écriture hors racine, arrêt avant lancement/arbitrage ; ne pas s'appuyer sur les dérogations historiques. Les racines Test autorisées restent inchangées.

Les recettes privées référence de native_import.rs, vingt appels GPU et parité WAV/MP3 avec ffmpeg ne sont pas requis par défaut pour cette extraction. Conserver leurs tests existants et couvrir les fonctions extraites par tests ciblés ; pas prétendre ces recettes réalisées. Leur extension exige décision précise. Owner exécution : implémenteur ou validateur désigné ; reviewer n'exécute pas les preuves manquantes.

## 9. Ordre et budget des campagnes

Réservation proposée, non mesure ni garantie produit : référence T1 avec contrôles sans modèle puis parcours CPU court ; baselines utiles aux scénarios natifs préservées avant extraction. Après T2/3 : tests/configurations CPU puis natif court. Après T4/5 : tests/configurations GPU puis sonde/refus et inférence CUDA distincte. Après T6/7 : tests Parakeet par feature puis single-pass CPU/CUDA et decode-only. T8 : ne relancer que contrôles affectés/manquants.

Réserver environ1h CPU,2h GPU,2h Parakeet et1h de vérifications finales, selon caches/ressources ; enveloppes incluant compilation/diagnostics, pas autorisation de prolongation automatique. Tests contrôlés : enveloppe5min/commande hors compilation ; référence60min : jusqu'à1Gio mémoire et plusieurs centaines de Mio fixtures ; natif court : enveloppe10min/enfant. Dépassement : arrêter uniquement les enfants possédés par le harness, garder sorties et classer incomplet ; aucun processus utilisateur arrêté. Ce timeout de test n'ajoute aucun délai produit.

T1 contrôle coût réel données/caches et espace restant avant écriture/compilation. Aucun benchmark long ou campagne extensive ajouté pour un déplacement. Une compilation ratée, un contrôle hors résultat négatif attendu ou un manque ressource suspend les tâches consommatrices.

## 10. Comparaison, traçabilité et clôture

Comparer référence/candidat à mêmes ressources/entrées/configuration. Préserver ordre/données déterministes/diagnostics/événements/codes/effets. PID/durées/temp/empreinte binaire reconstruit variables : préserver règles et corrélations, normalisation explicitée sans masquer contrat. Pour audio généré, mêmes échantillons et header équivalent ; pour sorties encodées, vérifier paquets/fin/contenu déterministe selon format réellement produit.

| Besoin | Exigences / parcours / acceptation | Tâches et scénarios |
|---|---|---|
| CA-01 Façades | REQ-01 ; UC-01 ; AC-01 | T3/5/7/8 ; S-01 |
| CA-02 Responsabilités | REQ-02 ; UC-01/05 ; AC-02/06 | T2 à8 ; S-01/02/09/11/13 |
| CA-03 Comportement | REQ-03/06 ; UC-02/03/05 ; AC-03/06 | T1 à8 ; S-02 à08,S-10 à13 |
| CA-04 Contrats | REQ-04/06 ; UC-02/03/04 ; AC-04/06 | T1 à8 ; S-02 à12 |
| CA-05 Périmètre | REQ-05/06 ; UC-01/04/05 ; AC-05/06 | Toutes ; S-01/13, revue chemins |

Chaque chemin §4 appartient à une extraction ou un scénario ; ressources §6 et racines §3 sont les chemins indirects. Pas d'ajout par commodité : corriger le plan avant exécution ou DESIGN_CHANGE_REQUIRED si le design change. DOCUMENT seul n'autorise pas utilisation d'une API non encore fournie : CODE et EXECUTION du prédécesseur sont vérifiés séparément.

À chaque jalon, consigner dans ce document : révision/fichiers/commandes réellement exécutées/nombre de tests et scénarios atteints/sorties utiles/limites. Code0, test ignoré ou refus CUDA accepté ne prouvent pas transcription native. Terminer toute comparaison par statut de couverture explicite, pas une non-régression globale annoncée si scénarios requis manquants.

Arrêt sur échec, divergence inexpliquée, garde refusée, écriture attribuée hors racine, besoin manifeste/dépendance/FFI/hors-périmètre ou dépassement dix tâches/dix rapports. Préserver changements et preuves ; aucune suppression/restauration automatique. Reprise après résolution/accord requis depuis dernier état Git vérifié, sans reset global. À clôture relever espace libre et appliquer archivage petites preuves selon AGENTS. Aucun nettoyage du target partagé ni suppression copies/WAV sans périmètre approuvé ; aucun push prévu.

Recette Windows non requise pour la seule extraction structurelle ; si retenue pour parcours utilisateur final, autorisation distincte, profil réellement construit et utilisateur responsable. Respecter la remise AGENTS et ses ressources effectives ; pas de paquet présenté autonome sans preuve.

## 11. État de remise

Lectures et analyse statique réalisées pour rédaction. Toute compilation, test, natif, création fixture et recette reste **NOT RUN**.

Revue PLANS initiale sur d2a243c3 : **FINDINGS**, PL-MF-01 IMPORTANT/REQUIRED, traitement des tests de racines préexistants dans la matrice Parakeet. Correction documentaire proposée par rust_plan_writer, intégrée sans modifier design/code.

Revue de fermeture du 29 octobre 2026 : `rust_design_reviewer`, `/root/revue_design`, indépendant de rédaction/correction, a relu le diff et le candidat final au commit `1af669161a713e3de566e6ba3c0847b767ef0970`, parent `e58bfa2fc2118c15cf815656d3d3fbec0af42a1c`. Verdict **CLEAN — PLANS documentaire**, **PL-MF-01 CLOSED**, aucun autre constat requis. Un cycle de correction utilisé. Le résultat brut est restitué dans la session ; la présente clôture ne change que le suivi et le statut du plan examiné.

Acceptation plan : **EN ATTENTE**. Q-EXEC-01 garde : **À RÉSOUDRE avant toute campagne**. Q-EXEC-02 tests racines : **À RÉSOUDRE avant T6 et clôture S-09**. Implémentation/campagnes/recette/push : **NON AUTORISÉS**. Aucun DESIGN_CHANGE_REQUIRED identifié. Le verdict documentaire ne lève pas les conditions d'exécution.

## 12. Accord d'exécution du 29 octobre 2026 — avenant prioritaire

L'utilisateur a écrit dans la session : « Oui, j’autorise la garde PowerShell, les huit tâches et leurs tests dans main-facades/ et cargo-shared/, ainsi que la correction limitée des tests Parakeet avec mise à jour du plan. Lance la refacto. » Cet accord accepte le lot T1–T8 et les chemins du §4, avec les amendements suivants ; les mentions historiques d'attente ou d'absence d'autorisation ci-dessus sont remplacées pour ce périmètre.

- Q-EXEC-01 : création autorisée de `target/parakeet-tests/main-facades/guard-main-facades.ps1`, garde PowerShell propre à ce lot, avec préflight canonique, auto-tests de refus et surveillance. Aucune commande Cargo avant réussite de ces prérequis. Cette autorisation ne constitue pas une preuve de confinement et ne reprend aucune dérogation historique. Racines de campagne exclusives : `target/parakeet-tests/main-facades/` et `target/parakeet-tests/cargo-shared/` ; aucun nouveau service, compte, ACL ou tâche planifiée.
- Q-EXEC-02 : correction test-only autorisée des assertions et conditions de compilation des deux tests historiques `accepts_only_the_two_canonical_l2_test_roots` et `qualification_profiles_are_confined_to_l2b`, dans leur emplacement source puis dans les fichiers de tests déplacés prévus au §4. Leur oracle doit refléter les règles produit existantes pour chacun des huit ensembles de features. Aucun élargissement des racines produit, changement de politique runtime, masquage par `ignore` ou exclusion globale. Les interdictions antérieures de corriger ces deux tests sont levées dans cette seule limite. Comparer explicitement les résultats historiques et les corrections, sans attribuer les défauts préexistants à l'extraction.
- L'implémenteur possède le code, les tests et la garde ; la session principale possède ce document et son suivi. Revue indépendante selon T8, deux cycles maximum. La validation Windows reste celle de l'utilisateur si elle est ultérieurement demandée ; aucune recette ni publication ajoutée.

Préflight de lecture : HEAD `5317588669edec83db981c1b4517294e5b1d1f7a`, branche `evo-startup-perf`, aucun changement suivi avant cet avenant ; `docs/architecture/`, `models/` et `sound example/` non suivis à préserver. Environ 103 Gio libres avant préparation. Aucun Cargo exécuté à ce stade.

### Interruption de T1 — 29 octobre 2026

Implémenteur délégué : `rust_implementer`, `/root/implementation_facades`, base documentaire `6d19d4d4985a688efd709534d7f5c54c1c202818`. Garde créée sous la racine autorisée et cache Cargo privé préparé. Auto-tests `guard-main-facades.ps1 -Mode SelfTest` : PASS, code 0 ; refus des variables absentes/hors racine, observation d'un événement réel et refus d'une violation simulée. Ces contrôles ne prouvent pas un confinement du compte.

Première commande : `guard-main-facades.ps1 -Mode Run -CargoArguments @('fmt','--all','--','--check')`. Pendant son exécution, création observée de `C:\Users\cyril\AppData\Local\Temp\etilqs_qqHDyh1IVWPTziI` à **09:03:26.625 +01:00**. Origine non attribuée : ce constat ne prouve pas une écriture de Cargo ou de rustfmt. La garde a demandé l'arrêt, terminé avec code 1 et conservé `target/parakeet-tests/main-facades/STOP.json`, `logs/guard-events.jsonl` et les captures sous cette même racine. Stdout/stderr vides ; contrôle de format **INCOMPLET**, statut propre de Cargo indisponible. Aucun Cargo/rustfmt/rustc visible au contrôle ultérieur. Le fichier temporaire n'était alors plus visible ; aucune suppression ou restauration effectuée par l'implémenteur.

Seul changement suivi de l'implémenteur : `crates/whisper-worker-cpu/tests/reusable_imports.rs`, adaptation T1 vers une racine explicite canonisée, sous-répertoire unique et conservation des fichiers. `git diff --check` : PASS, code 0. Aucun binaire de référence construit, aucune extraction produit ; T1 BLOCKED, T2–T8 non commencées, builds/tests/scénarios natifs NOT RUN. Aucun candidat complet à soumettre à la revue indépendante à ce stade. Espace final : **110 606 336 000 octets** (environ 103 Gio). Aucune reprise, suppression ni application d'une ancienne dérogation ; décision utilisateur nécessaire sur cet événement avant reprise de T1.

### Reprise autorisée — dérogation Temp limitée à ce lot

Après présentation de l'interruption et de la dérogation précise, l'utilisateur a répondu dans la même session : « oui je t'autorise ». La garde peut être adaptée puis la campagne T1–T8 reprise : les événements observés uniquement sous `%LOCALAPPDATA%\Temp` sans attribution démontrée à la campagne sont consignés `UNATTRIBUTED`, avec chemin et horodatage, sans imposer à eux seuls l'arrêt. Leur origine demeure indéterminée ; ils ne constituent ni des écritures Cargo prouvées ni des écritures autorisées. Aucune destination volontaire n'est ajoutée dans le Temp utilisateur.

Toute écriture attribuée à la campagne hors de `main-facades/` et `cargo-shared/`, toute modification inexpliquée des autres emplacements sensibles, tout défaut du préflight ou de surveillance, et tout échec d'une commande censée réussir continuent d'imposer l'arrêt. La surveillance ne constitue pas un confinement préventif. Cette dérogation est exclusive au présent lot et ne couvre aucun autre chemin ou lot.

L'adaptation de la garde inclut des auto-tests renouvelés pour la classification `UNATTRIBUTED` et le maintien des refus obligatoires. Préserver l'ancien `STOP.json` par archivage sous la même racine avant levée de ce seul arrêt, sans effacer les preuves. Reprendre T1 et ses références ; aucun saut de prérequis ni extraction avant leurs résultats applicables. État de reprise : HEAD `a5f9df432457b287d6108bc8b31fc029c401de7d`, seul diff suivi du worker sur `tests/reusable_imports.rs`, environ 103 Gio libres.

### Nouvel arrêt de T1 — stockage Git interne

Ancien arrêt préservé sous `target/parakeet-tests/main-facades/STOP-20261029T090326.json`. Auto-tests renouvelés : PASS, code 0 ; classification Temp sans attribution, refus chemins extérieurs et défaut de surveillance. `cargo fmt --all -- --check` via garde : **PASS**, code 0, le 29 octobre à 09:08:27 +01:00.

Compilation CPU de référence démarrée à 09:08:32 et interrompue à 09:08:51, durant `whisper-rs-sys`, après événement observé à **09:08:50.959 +01:00** sur `C:\dev\whisper\.git\objects\pack\tmp_pack_Ykzlxg`. Origine indéterminée ; aucun processus Git actif au relevé ciblé ultérieur, attribution impossible. Ce chemin ne relève pas de la dérogation Temp. Nouveau `STOP.json` conservé, captures `target/parakeet-tests/main-facades/logs/15c17f56fbee46bd90190b011c11db04.stdout.txt` et `.stderr.txt`. Compilation **INCOMPLÈTE**, aucun code propre de Cargo disponible, aucun binaire de référence préservé. Aucune reprise ni suppression/restauration effectuée.

Diff suivi de l'implémenteur inchangé : seul `crates/whisper-worker-cpu/tests/reusable_imports.rs`, 18 ajouts et 5 suppressions ; `git diff --check` PASS. Espace final : **110 588 682 240 octets**. T1 reste interrompue et T2–T8 non commencées. L'observation ne prouve ni modification des sources ni écriture de Cargo dans Git ; décision requise sur ce nouveau chemin avant reprise.

### Reprise autorisée — fichiers temporaires de stockage Git

L'utilisateur a répondu « oui je t'autorise » à la proposition précise de consigner sans arrêt, pour ce lot uniquement, les événements non attribués sur les seuls fichiers `.git/objects/pack/tmp_pack_*`, en maintenant le contrôle des sources et références Git. Cette dérogation autorise l'adaptation de la garde, le renouvellement de ses auto-tests et la reprise de T1 puis T2–T8 selon leurs prérequis. Préserver le second arrêt et ses captures sous la racine du lot avant levée de cet arrêt.

La classification sans arrêt ne concerne que les fichiers directs dont le nom commence par `tmp_pack_` sous `C:/dev/whisper/.git/objects/pack/`, avec chemin et heure consignés et attribution indéterminée. Aucun autre fichier Git, aucune référence, aucun fichier source ou emplacement sensible supplémentaire n'est couvert. Aucun processus de campagne n'est autorisé à écrire volontairement dans Git ; toute écriture attribuée à la campagne hors racines reste un motif d'arrêt. Contrôler les sources et références Git avant et après les commandes ; préserver les différences légitimes déjà identifiées, arrêter en cas de changement inexpliqué. Aucun nettoyage, restauration ou nouvelle surveillance n'est ajouté. L'exception Temp précédemment accordée reste inchangée.

### Arrêt pré-lancement après cette adaptation

Auto-tests renouvelés PASS : exception tmp_pack limitée, refus autres chemins Git et changement d'état Git, contrôles Temp et surveillance. Second STOP archivé sous `main-facades/STOP-20261029T090850.json`. Lors du contrôle préalable, notification `Changed` sur le dossier `C:/dev/whisper/.git/objects/pack` à **09:11:52.572 +01:00**, détectée à 09:11:52.721 ; aucun événement enfant réel conservé permettant de la relier à un tmp_pack autorisé. La garde a arrêté avant Start-Process : **aucune nouvelle commande Cargo**. Événement préservé dans `logs/guard-events.jsonl` et `STOP.json` sous la racine du lot.

Lecture ultérieure : `pack-cf5f8a6c7e5b055802b7a15f072b1fb7ac611c3e.pack` présente LastWriteTime 09:12:09.558, après arrêt ; origine et signification indéterminées, aucun événement capturé correspondant ne permet une attribution. HEAD `6d7c1475`, index sans diff, seul diff suivi du worker sur reusable_imports.rs ; `git diff --check` PASS. La comparaison exhaustive des références avant/après n'est pas démontrable, état préalable seulement en mémoire perdu lors de l'exception. Espace libre relevé : **108 852 076 544 octets**. Aucune reprise, suppression ni restauration. Les exceptions existantes ne couvrent pas cet événement ; T1 demeure interrompue, T2–T8 non commencées.

### Reprise autorisée — événements non attribués du stockage d'objets Git

L'utilisateur répond « oui je t'autorise » à la proposition explicite suivante : pour ce lot, consigner sans arrêt les événements non attribués dans `.git/objects/`, dossiers compris, tout en maintenant l'arrêt sur changement inexpliqué des sources, de l'index ou des références Git, et sur toute écriture hors racines attribuée à la campagne. Cet accord remplace la restriction précédente aux seuls fichiers tmp_pack pour cette classification ; il ne concerne aucun autre emplacement Git. L'exception Temp reste inchangée. Aucune destination volontaire d'écriture de campagne n'est ajoutée dans Git.

Adapter la garde existante, renouveler les auto-tests de classification et de refus (autres chemins Git, sources, index, références, erreurs de surveillance), conserver sous la même racine le STOP précédent puis lever ce seul arrêt autorisé et reprendre T1–T8 selon leurs prérequis. Conserver sous la racine du lot les états nécessaires au contrôle sources/index/références avant/après, y compris en cas d'exception : ils appartiennent aux captures de campagne déjà prévues, aucun nouveau rapport ou manifeste de qualification. Aucune suppression/restauration, isolation Windows ou publication supplémentaire n'est autorisée.

### Accord nominatif pour les journaux Parakeet CUDA

Le contrôle existant `gpu_identity::build_gpu_evidence` exige un journal frais `PARAKEET_L3_ASR_LOG` dont le parent canonique se trouve sous le runtime ; un chemin sous la campagne est refusé `GpuLogPath`. En profil personnel, le runtime est `C:/Users/cyril/AppData/Local/Whisper`. Aucun journal ancien n'a été substitué et aucun lancement CUDA dépendant n'a été fait avant accord.

À la question précise sur la création de journaux frais `main-facades-*.log` uniquement dans `C:/Users/cyril/AppData/Local/Whisper/settings/logs/`, modèles et DLL en lecture seule, conservation des journaux et adaptation de la garde à cette seule exception, l'utilisateur a répondu **« Autoriser ces journaux CUDA »** dans la session. Cette écriture nominative devient une exception aux deux racines de campagne pour S-12 CUDA référence/candidat uniquement. Aucun autre fichier de l'installation, écriture volontaire dans Git, suppression ou restauration n'est autorisé. La garde existante doit vérifier ce chemin canonique et ce motif de noms, tester les refus des chemins voisins, surveiller les autres emplacements sensibles et conserver les captures de cette exécution sous main-facades/. Aucun service, compte, ACL ou nouvelle surveillance n'est ajouté.

### Point d'avancement avant comparaison native CUDA

Références préservées : quatre configurations Turbo CPU, quatre Turbo GPU, huit Parakeet compilées et tests Parakeet inventoriés. CPU : contrôle natif deux imports même processus avec cache load/hit réussi avant et après extraction ; tests binaires candidat 9 par configuration ; contrat inactif/refus et comparaison live Stop/Shutdown avec événements et paquets MP3 réussis. GPU : tests binaires candidat 20 par configuration, sonde et refus identité, deux imports CUDA load/hit et comparaison live référence/candidat réussis ; contrôle unitaire de l'attestation sur cache hit et éviction sur échec ajouté. Le sous-scénario historique MP3 gapless sans fixture ne vaut pas preuve : NOT RUN.

Parakeet : huit suites binaires réussies après extraction et correction test-only autorisée des deux tests de racines ; lecture 60 minutes et refus dépassement exécutés avec fixtures sous campagne. Contrat decode-only référence/candidat réussi (PCM/résumé JSON et refus d'écrasement). Refus successifs et consommation des compteurs vérifiés sur référence/candidat ; contrôle JSON/version/framing/EOF/Shutdown réussi sur les deux, code 0. Transcription native et contrôles finaux encore à terminer, aucune revue CLEAN revendiquée. Les erreurs locales d'import et de harness rencontrées ont été conservées avec leurs STOP puis corrigées dans les chemins autorisés avant relance ciblée ; ces échecs initiaux ne sont pas reclassés PASS. Les références/candidats et captures détaillées restent sous la racine de campagne ; le bilan final précisera commandes, nombres et limites.

### Candidat remis et première revue d'implémentation

Candidat working-tree sur HEAD documentaire `282a61eb1dcdf97a5d0e2f67193d867bdde8bddb` : cinq fichiers suivis modifiés et 57 fichiers Rust nouveaux sous les trois workers, aucun changement tiers repris. Les seize configurations de développement sont compilées et conservées, format et Clippy (4 CPU + 4 GPU + 8 Parakeet) code 0 ; avertissements de fonctions longues historiques conservés. Refus Release avec recette-hooks CPU/GPU confirmé par diagnostic exact attendu. Suites binaires : CPU 9/configuration, GPU 20/configuration ; Parakeet 22/23/25/23/26/23/26/26 réussis, helper de préparation de fixtures ignoré par défaut mais exécuté séparément. Aucun test historique perdu après comparaison des huit inventaires.

Imports Turbo : deux sessions, toutes trames référence/candidat et load/hit comparés avec succès CPU `ff70653962b84bef984bd8fee39ad153`, GPU `741a145baee14c16a939170b053ce98f`. Live réel deux secondes Stop/Shutdown, événements et MP3 comparés : CPU `15c04993604f4d2ba139c1778729560f`, GPU `b1a03959b422492181bad073e90b703b`. Parakeet CPU trois scénarios processus `9c1bab...` réussis ; CUDA transcription `8f6c5f...`, refus `437cde...`, erreurs/EOF/Shutdown `851673...` réussis. Backend CUDA0/identité GPU/empreinte journal/NativeCallStarted corrélés, normalisation limitée aux champs variables annoncés. Onze journaux CUDA frais autorisés conservés. Archive `target/parakeet-tests/main-facades/evidence-final-2.zip`, 804 entrées, 44 493 430 octets, arborescence et SHA-256 comparés ; espace relevé 107 035 570 176 octets, aucune suppression.

Limites conservées : fixture MP3 gapless historique absente = NOT RUN ; erreur InvalidTimestamp provoquée nativement = NOT RUN ; aucune validation fenêtre demandée ou effectuée. Les contrôles unitaires de normalisation et lecture statique de classification ne prouvent pas cette branche native. Les preuves ne valent pas verdict de revue.

Revue indépendante T8 coordonnée par `/root/review_facades`, reviewer `/root/review_facades/reviewer_facades` : **FINDINGS**, deux IMPORTANT, aucune régression fonctionnelle identifiée dans l'extraction à ce stade. **MF-R1** : témoin de frontière de classification manquant pour InvalidTimestamp/NativeCallStarted/IpcWrite et erreur ordinaire ; correction par helper privé appelé par transcribe_one et tests contrôlés sans injection native. **MF-R2** : certaines attentes finales d'enfants Turbo/decode-only ne sont pas bornées sur toute leur durée de vie ; correction par délai global, arrêt/attente/captures et échec au timeout, avec faux enfant contrôlé restant vivant. Premier cycle de correction en cours, fermeture réservée au reviewer.

### Accord nominatif de chemin pour MF-R1

À la question explicite sur l'ajout du seul fichier `crates/whisper-worker-parakeet-cpu/src/transcription/tests.rs` pour couvrir la frontière extraite InvalidTimestamp/NativeCallStarted sans modifier moteur ni FFI, l'utilisateur a répondu **« Autoriser ce fichier de tests »**. Ce fichier est ajouté au périmètre P-PARAKEET du §4 ; déclaration cfg(test) et helper privé dans transcription.rs déjà autorisé, aucun autre chemin ni comportement produit ajouté. L'erreur provoquée par le moteur natif reste NOT RUN. MF-R2 reste dans les chemins de harness déjà autorisés. Vérifications utiles invalidées à renouveler, puis relecture du même reviewer ; deux cycles maximum au total.
