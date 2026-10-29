Avancée : plan de refactorisation des trois workers détaillé à partir du cadrage relu.
Blocage : exécution conditionnée à une garde applicable et à un accord précis pour le lot et ses racines.
Décision : plan relu CLEAN, soumis à validation utilisateur ; aucun code, build, test ou push autorisé.

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
