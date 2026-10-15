Avancée : séquencement DRAFT de l'évolution Parakeet et du registre commun, proposé par `rust_plan_writer`.
Blocage : design parent `DESIGN BLOCKED` ; faisabilité native et composition du paquet final non démontrées.
Décision : `DESIGN_CHANGE_REQUIRED` ; ce document n'est ni `PLANS READY`, ni une autorisation de coder, d'essayer le moteur ou de publier.

# Plan d'implémentation Parakeet — projet conditionnel du 15 octobre 2026

**Identité :** `P-PARA-DRAFT-02`. **Objectif produit :** ajouter Parakeet TDT 0.6B v3 à Turbo dans l'application Windows, pour import et micro, CPU/GPU/Auto calcul, avec catalogue fermé, Auto langue Parakeet seulement après qualification, acquisition finale et conservation des garanties audio et de sorties. **Parents Git :** `docs/cadrage/evo-parakeet.md` à `7caabc0a` (distribution et ABI proposées, `PARA-REV-07` fermé en revue ciblée) et `docs/cadrage/evo-engine-capabilities.md` à `79f64a9a` (gate CAP-02/03, revue CLEAN). **Propriétaire du découpage :** `rust_plan_writer`, contribution `PLAN-PARA-DRAFT v0.2` en lecture seule ; auteur Git : session principale. **Statut :** DRAFT, non exécutable. Une modification du design parent impose de rapprocher ce plan du nouveau commit avant revue `PLANS READY`.

Le choix DEC-PARA-19 retient la transcription multilingue sans langue imposée pour Parakeet. Il ne démontre pas que le GGUF et NeMo-Speech.cpp v0.2.0 intégrés la réalisent. Le manuel de ce couple reste `Unsupported` ; aucun parcours Parakeet ne démarre avant qualification d'Auto pour la configuration exacte. La restitution d'un identifiant de langue détectée est facultative et distincte. Les comparaisons de vitesse et de charge sont différées ; aucun seuil de perte de qualité n'est inventé.

## 1. Entrées à fermer avant un plan READY

| Gate | Fermeture nécessaire ; owner | Lots concernés |
| --- | --- | --- |
| G-DESIGN | `PARA-REV-01` et `RISK-T-01a–e` : artefact/DLL, inférence CPU, GPU réel sur la cible, fenêtres et queue micro, Auto langue du candidat exact. `SPIKE-PARA-02` est autorisé dans cette session et en cours sous son mandat précis ; ses résultats restent `NOT RUN` ou partiels tant qu'ils ne sont pas rapportés et revus. Architecte, exécutant technique distinct et revue indépendante. | Tous avant readiness |
| G-CAP | `Q-CAP-02/03` fermées côté produit/contrat : capacités CPU/GPU/langue non qualifiées bloquées ; adéquation RAM/VRAM seulement indéterminée permet de préparer une configuration qualifiée, avec erreur réelle expliquée. Revue documentaire CLEAN à `79f64a9a` ; essais produit `NOT RUN`. | L1–L4 |
| G-CAPTURE | `Q-PARA-08/09` fermées comme contrats proposés : pool fini, refus définitif après trou, frontière WAV, restauration partielle et constat de fin physique. Tests futurs `NOT RUN` ; aucun délai candidat 2 s/5 s. | L4 |
| G-ACQ | `Q-PARA-02/10/11` et références distantes des deux modèles définies. Magasin, annulation et publication proposés ; installateur Rust mono-EXE choisi sous réserve des preuves natives, transport réseau et dépendances à figer. RG-08 actif à modifier avec accord précis avant réseau applicatif. | L1, L5–L6 |
| G-QUAL | `Q-PARA-05/06/07` fermées : corpus FR/EN ciblé, import avant/après publication et ancien chemin inconnu conservé pour diagnostic avec choix explicite. Données et références à préparer ; tests `NOT RUN`. | L1–L2, L7 |

Le parent n'est pas `DESIGN READY` et la revue du plan révisé n'a pas eu lieu. Les lignes ci-dessous sont des **hypothèses de découpage**, pas des décisions structurantes cachées dans des tâches. `G-DESIGN` doit être fermé puis le design revu avant de convertir ces lots en `lot.paths` autorisés. Les gates `CODE` (dépendances et interfaces des lots précédents) et `EXECUTION` (autorisation précise des chemins, données, ressources et écritures) sont à contrôler avant chaque campagne. La source native C++/CUDA, le FFI Rust, leurs dépendances, essais et installations nécessitent les accords précis applicables dans la session d'exécution ; l'ancien SPIKE n'en donne pas de nouveaux.

## 2. Index et dépendances

| Lot | Résultat démontrable visé | REQ/AC-PARA et CAP | Dépendance de code |
| --- | --- | --- | --- |
| L1 — catalogue et capacités | Turbo préservé ; deux modèles visibles, sélection/admission cohérentes, langues et préférences | 05/06/08/16 ; CAP-31–34 | Aucune |
| L2 — import CPU | Premier parcours Parakeet complet WAV/MP3 → texte, TXT/SRT, modèle et CPU attestés | 01/02/09/12/13/16/18 | L1 |
| L3 — GPU et Auto calcul | Inférence sur GPU attendu ; Auto reste sur le même modèle, replis bornés | 03/04/15/18 | L2 |
| L4 — microphone | Blocs confirmés, dernière phrase, Stop/drainage, TXT/SRT/WAV et réemploi | 10–14/16–18 | L2 et L3 pour les combinaisons annoncées |
| L5 — acquisition | Installation/acquisition et réparation explicite annulables, autre modèle valide préservé | 05/07B–E/08/13/18 | L1 ; contrats G-ACQ |
| L6 — distribution | Installable par utilisateur courant ; paquet exact qualifié CPU/GPU, Auto langue compris | 07A/07F/16F ; CAP-34 | L3–L5 |
| L7 — qualité et recette | Écarts Turbo/Parakeet rapportés ; essai final dans la fenêtre Windows | 01/09–19 selon parcours, surtout 19 | L6 ; G-QUAL |

L1–L6 modifient potentiellement les mêmes contrats IPC, workers, `ui.rs` et `root.rs`. Aucun travail parallèle sur ces chemins n'est présumé. L5 peut avancer en conception indépendamment de L2–L4 ; son intégration ne commence qu'après la réconciliation des contrats partagés. Chaque lot compte au plus six tâches ci-dessous et ce plan reste un seul fichier documentaire.

## 3. Tâches proposées, chemins et contrôles

### L1 — catalogue, capacités, langues et préférences (6 tâches)

| Tâche | Travail et résultat | Chemins d'écriture candidats ; contrôle futur |
| --- | --- | --- |
| L1.1 | Types purs `ModelId`, révision, qualification, préférence et causes de refus ; distinguer présence, moteur, ressources et calcul. | `crates/whisper-core/src/domain.rs`, `ports.rs`, `lib.rs`, modules dédiés à définir ; tests purs de CAP-31/34. |
| L1.2 | Évaluation applicative unique des modèles et modes linguistiques ; ancienne langue libre ambiguë/périmée refusée avec choix explicite, sans substitution ; refus identique dans le widget et avant séance. | `crates/whisper-core/src/application.rs`, `ports.rs` ; tests Auto qualifié, manuel absent, aucun mode et valeur ancienne ambiguë (PARA-05/16, CAP-33). |
| L1.3 | Constat des artefacts et du PC injecté par composition, avec `Unknown` distinct d'`Insufficient`. | `crates/whisper-adapters/src/lib.rs`, modules catalogue/hôte à nommer, `crates/whisper-desktop/src/root.rs` ; tests absent, invalide, mesure manquante/tardive (CAP-32). |
| L1.4 | Migrer v1 vers v2 : Turbo exact vérifié ou intention du chemin historique connu → `ModelId::Turbo` ; le fichier manquant ne devient pas source alternative. Chemin inconnu/invérifiable → valeur diagnostique conservée, lancement bloqué et choix explicite. Préserver les champs v1 réellement stockés : dernier dossier audio, destination, préférence micro, identifiant GPU ; audio courant vide. Aucun calcul ne vient de v1 : créer la préférence v2 `Auto`/`FollowProbe`, avec sonde tardive et sauvegarde échouée contrôlées. | `crates/whisper-core/src/path_settings.rs`, `crates/whisper-adapters/src/path_settings.rs`, `crates/whisper-desktop/src/root.rs`, `ui.rs`, tests `crates/whisper-core/tests/model_preferences.rs` et `crates/whisper-adapters/tests/model_preferences.rs` ; migration/réouverture isolées (PARA-08). |
| L1.5 | Liste de modèles et de langues issue de l'application ; retirer chemin modèle et langue libre, montrer le refus et le choix explicite d'une ancienne valeur ambiguë/périmée, préserver champs audio/sortie. | `crates/whisper-desktop/src/ui.rs`, `root.rs` ; inspection du vrai widget et refus côté application (PARA-05/06/16). |
| L1.6 | Vérifier non-régression Turbo, sélections périmées, modèle absent et source de test exclusivement locale. | Tests `crates/whisper-core/tests/`, `crates/whisper-adapters/tests/`, `crates/whisper-desktop/tests/` à nommer exactement ; AC-PARA-07A/08 et CAP-31–34. |

**Gate L1 :** G-CAP, règle Q-PARA-07 et identité du catalogue. Aucun modèle inconnu ou fichier arbitraire ne devient sélectionnable. Les chemins de modules/tests nouveaux restent candidats tant que le design n'en a pas fixé le contrat et le plan final le `lot.paths`.

### L2 — premier import Parakeet CPU (6 tâches)

| Tâche | Travail et résultat | Chemins d'écriture candidats ; contrôle futur |
| --- | --- | --- |
| L2.1 | Implémenter le worker et la façade du backend CPU acceptés par le design, avec propriété/libération des handles natifs. | Nouveau module/worker Parakeet à choisir après G-DESIGN, `Cargo.toml`/`Cargo.lock` seulement si approuvés ; essai CPU natif attribué. |
| L2.2 | Versionner IPC : release, modèle/révision, séance/tentative, Auto langue et calcul ; refuser worker/version incompatible. | `crates/whisper-core/src/ipc.rs`, `crates/whisper-adapters/src/worker_ipc.rs`, worker retenu ; test identité fausse et résultat périmé. |
| L2.3 | Router l'import WAV/MP3 vers le modèle sélectionné, sans secours Turbo ; recontrôler l'artefact à l'ouverture. | `crates/whisper-core/src/application.rs`, `crates/whisper-adapters/src/worker_ipc.rs`, `decoder.rs`, `crates/whisper-desktop/src/root.rs` selon nécessité ; modèle/CPU attestés. |
| L2.4 | Publier TXT/SRT lisibles et garder la source importée intacte. Stop/Quit avant engagement conserve pending et rejette tout résultat tardif ; après engagement, annoncer « arrêt trop tardif » et rapporter la vraie issue. | `crates/whisper-adapters/src/archive.rs`, `supervisor.rs`, `crates/whisper-core/src/application.rs`, test `crates/whisper-adapters/tests/import_cancellation.rs` ; barrières déterministes avant/après publication et source avant/après. |
| L2.5 | Émettre les sondes réellement observées sans annonce de gain. | Journal existant, `crates/whisper-desktop/src/reactivity_log.rs` et raccords nécessaires ; modèle, CPU, première inférence et sorties corrélés. |
| L2.6 | Vérifier refus de mode manuel Parakeet, Auto non qualifié, erreurs moteur et non-régression Turbo. | `crates/whisper-core/tests/import_contract.rs`, `crates/whisper-adapters/tests/import_cpu.rs`, `import_mp3.rs`, `worker_control.rs`, tests worker à fixer. |

**Gate L2 :** preuve CPU et Auto langue ciblée sur le candidat exact, contrats ABI/IPC acceptés, Q-PARA-06. Premier chemin de bout en bout attendu : WAV français et seconde langue annotée → texte intelligible, TXT/SRT ordonnés, CPU effectivement utilisé et source intacte ; un chargement réussi seul ne suffit pas.

### L3 — GPU et Auto calcul (5 tâches)

| Tâche | Travail et résultat | Chemins d'écriture candidats ; contrôle futur |
| --- | --- | --- |
| L3.1 | Intégrer variante native GPU et relier identité du GPU demandé à l'inférence réellement exécutée. | Worker GPU/Parakeet retenu, `crates/whisper-worker-gpu/src/gpu_attestation.rs`, `native_ffi.rs` si réutilisés ; essai GPU natif ciblé. |
| L3.2 | Vérifier modèle, langue et GPU avant audio ; refuser périphérique incorrect ou backend indisponible. | `crates/whisper-core/src/gpu_selection.rs`, `ipc.rs`, `crates/whisper-adapters/src/worker_ipc.rs` ; erreur sans faux `Ready GPU`. |
| L3.3 | Auto calcul : GPU compatible puis au plus une tentative CPU du **même modèle** avant première inférence ; aucun repli tardif. | `crates/whisper-core/src/compute_policy.rs`, `application.rs`, routeur IPC ; tests avant/après début audio. |
| L3.4 | Garder CPU/GPU manuels fixes pendant la séance, Auto restauré s'il a été sauvegardé, et cause/calcul effectif visibles ; un choix manuel sauvegarde `FollowProbe` pour le prochain démarrage. | `crates/whisper-desktop/src/ui.rs`, `root.rs`, `crates/whisper-core/src/path_settings.rs` si requis ; échec GPU manuel, sonde tardive et préférence après redémarrage. |
| L3.5 | Vérifier Turbo et Parakeet sur combinaisons admises, identité GPU fausse, résultat d'une ancienne tentative. | Tests existants `gpu_selection`, `gpu_selection_worker`, `gpu_selection_native`, `compute_policy` à compléter selon le plan final. |

**Gate L3 :** RISK-T-01c démontré sur GPU attendu ; compilation, inventaire ou `Ready Cuda` ne prouvent pas l'inférence. CUDA et DLL restent privés au paquet retenu ; aucun pilote ou installation système n'est implicitement autorisé.

### L4 — microphone, arrêt et réemploi (6 tâches)

| Tâche | Travail et résultat | Chemins d'écriture candidats ; contrôle futur |
| --- | --- | --- |
| L4.1 | Relier fenêtres offline Parakeet au PCM admis, plages absolues et timebase source ; confirmer sans trou/doublon. | `crates/whisper-adapters/src/direct_live.rs`, routeur/worker retenus ; parole aux bords, silences et queue courte. |
| L4.2 | Appliquer admission après écriture WAV complète, file finie, saturation et écriture partielle selon Q-PARA-08. | `crates/whisper-adapters/src/capture.rs`, `direct_live.rs` ; frontière admise observée, refus signalé. |
| L4.3 | Stop avant/après moteur prêt : fermer source, drainer audio admis, traiter dernière phrase et publier TXT/SRT/WAV de même base. | `direct_live.rs`, `archive.rs`, `crates/whisper-core/src/application.rs` ; sorties complètes ou état incomplet explicite. |
| L4.4 | Superviser worker bloqué hors UI, invalider ancienne génération et distinguer demande/fin physique selon Q-PARA-09. | `crates/whisper-adapters/src/supervisor.rs`, `worker_ipc.rs` ; test de processus retardé/erreur sans faux succès. |
| L4.5 | Réemployer seulement contexte compatible, sans texte d'une séance ou d'un autre modèle ; préserver choix micro Windows. | Workers retenus, `crates/whisper-core/src/microphone.rs` seulement si nécessaire ; deux séances et changement de modèle. |
| L4.6 | Vérifier blocs ordonnés, Stop, dernière phrase, exports, panne et fermeture normale sur CPU/GPU/Auto annoncés. | `crates/whisper-core/tests/live_contract.rs`, tests `live_capture.rs`, `live_archive.rs`, `microphone_settings.rs`, worker live à fixer. |

**Gate L4 :** RISK-T-01d, Q-PARA-08/09 et contrat de fenêtres/arrêt acceptés. La limite RAM d'une fenêtre ne borne pas le WAV conservé sur disque ; aucune durée maximale de drainage n'est promise.

### L5 — acquisition et réparation explicite (6 tâches)

**Entrées :** L1, `PORT-CAT`, `PORT-ACQ`, sources/révisions/tailles/SHA/licences des deux modèles, choix réseau/stockage revus et modification précise autorisée de RG-08 avant réparation réseau dans l'application. Une acquisition globale et un verrou Windows détenu par handle protègent le magasin ; aucun modèle valide utilisé n'est remplacé.

| Tâche | Travail et résultat | Chemins d'écriture candidats ; contrôle futur |
| --- | --- | --- |
| L5.1 | Magasin Test/Final et catalogue fermé ; Test ne lit que `models/`, Final la racine du design ; taille/SHA revérifiés avant ouverture. | `crates/whisper-adapters/src/model_store.rs`, `src/lib.rs`, `.gitignore` si requis ; absence/corruption et isolation des provenances. |
| L5.2 | Verrou détenu physiquement, exclusivité globale et seconde opération `Busy`. | `crates/whisper-adapters/src/model_acquisition.rs`, `Cargo.toml` si feature autorisée ; deux consommateurs installateur/application. |
| L5.3 | Transfert HTTPS à tampon borné vers `.part-<operation_id>` ; taille, SHA, synchronisation et fermeture avant publication sous verrou. Destination valide → `AlreadyReady` ; invalide sans lecteur → quarantaine ciblée ; erreurs typées. | `model_acquisition.rs`, `model_store.rs`, `Cargo.lock` si résolution justifiée ; réseau/TLS, hash, taille, accès et disque. |
| L5.4 | Sérialiser annulation, publication et fermeture : annulation acceptée invalide la génération, confirmation après fin IO ; publication engagée donne l'issue réelle. | `crates/whisper-core/src/application.rs`, `ports.rs`, adaptateur acquisition ; barrières avant/après engagement et résultat tardif. |
| L5.5 | Raccorder vérification/acquisition à l'installation et réparation explicitement déclenchée ; afficher progrès, cause et issue, préserver autre modèle valide et préférence. | `crates/whisper-bootstrap/src/main.rs`, `crates/whisper-desktop/src/root.rs`, `ui.rs` ; installation/réparation/fermeture. |
| L5.6 | Fermer AC-PARA-07B–E/12/13/18 et non-régression de provenance, sans audio ni texte réseau. | `crates/whisper-adapters/tests/model_acquisition.rs`, `model_store.rs`, `crates/whisper-core/tests/acquisition_contract.rs`, `crates/whisper-desktop/tests/model_acquisition_ui.rs` ; E7a–d. |

**`lot.paths.write` candidat L5 :** `.gitignore`, `Cargo.lock`, `crates/whisper-adapters/{Cargo.toml,src/lib.rs,src/model_store.rs,src/model_acquisition.rs,tests/model_store.rs,tests/model_acquisition.rs}`, `crates/whisper-core/{src/application.rs,src/ports.rs,tests/acquisition_contract.rs}`, `crates/whisper-bootstrap/src/main.rs`, `crates/whisper-desktop/{src/root.rs,src/ui.rs,tests/model_acquisition_ui.rs}`. Les chemins réellement requis sont à figer après choix du transport et du design ; aucun ajout implicite. `RAILGUARD.md` est une entrée documentaire distincte. L5 partage `application.rs`, `ports.rs`, `root.rs` et `ui.rs` avec L1–L4 ; réconcilier ces interfaces avant intégration.

### L6 — distribution Windows et paquet réel (6 tâches)

**Entrées :** L3–L5 ; `DEC-PARA-INSTALL-01 v1` choisit le bootstrap Rust mono-EXE et des releases immuables, `DEC-PARA-ABI-01 v1` cible `nemo_speech_asr_c.dll`. La liste complète de DLL/exports, le build natif et les droits réels restent à éprouver après `SPIKE-PARA-02` ; aucune écriture sous `%LOCALAPPDATA%` n'est autorisée par ce plan.

| Tâche | Travail et résultat | Chemins d'écriture candidats ; contrôle futur |
| --- | --- | --- |
| L6.1 | Assembler payload non compressé et embarquer les entrées fermées par `build.rs`/`include_bytes!` : desktop, workers Turbo/Parakeet CPU/CUDA, DLL privées, catalogue et notices ; vérifier versions, tailles/SHA et imports/exports, refuser payload incomplet. | `crates/whisper-bootstrap/build.rs`, `src/payload.rs`, `src/package.rs`, `src/lib.rs`, `Cargo.toml` ; inventaire/version et inspection PE. |
| L6.2 | Installer sous `%LOCALAPPDATA%\Programs\Whisper\releases\<ReleaseId>` pour l'utilisateur courant, sans élévation ; publier une release préparée et vérifiée, réutiliser l'identique, refuser le conflit, préserver modèles valides. | `crates/whisper-bootstrap/src/main.rs`, `install.rs` ; racine injectée, chemin avec espaces, accès refusé, relance idempotente. |
| L6.3 | Raccorder profil Final, racine et workers privés au desktop ; aucun secours Test/Downloads ni dépendance globale. | `crates/whisper-desktop/src/root.rs`, `crates/whisper-bootstrap/src/package.rs` ; témoins Test/Downloads seulement. |
| L6.4 | Vérifier installation/réparation avec compte standard, modèles déjà valides, réseau absent et réouverture depuis paquet. | `crates/whisper-bootstrap/tests/install_contract.rs`, `package_contract.rs` ; E8a/b. |
| L6.5 | Qualifier `AC-PARA-16F` sur le paquet exact en FR et EN, CPU puis GPU annoncés, sans langue imposée et avec identité effective ; appeler l'ABI C via worker privé. | Entrée IPC native/arguments et test à figer après faisabilité ; E8c. |
| L6.6 | Rapporter issue par variante, licences/notices et limites ; une configuration non éprouvée reste indisponible. | Plan/bilan existant ; E8a–c. |

**`lot.paths.write` candidat L6 :** `crates/whisper-bootstrap/{Cargo.toml,build.rs,src/main.rs,src/lib.rs,src/payload.rs,src/package.rs,src/install.rs,tests/install_contract.rs,tests/package_contract.rs}`, `crates/whisper-desktop/src/root.rs` ; le test `package_language.rs` et le bilan documentaire sont candidats supplémentaires. La composition complète des DLL, l'interface IPC exécutable, les dépendances natives réelles et les commandes de fabrication restent `PLAN_CHANGE_REQUIRED` après faisabilité. Aucun script/outillage de paquet nouveau n'est présumé.

**Contrôles L6 proposés, `NOT RUN` :** pour chaque EXE/DLL du payload, inspecter machine x64 et dépendances PE (`dumpbin /headers`, `dumpbin /dependents`) puis les exports réellement appelés de `nemo_speech_asr_c.dll` (`dumpbin /exports`). Vérifier les modules chargés lors d'un démarrage depuis un chemin avec espaces, avec PATH d'essai restreint et sans chemins du dépôt ; cette inspection complète les dépendances statiques. Les interfaces futures proposées `whisper-bootstrap.exe --verify-payload`, `--install --no-launch` et worker `--verify-runtime` sont **absentes aujourd'hui** : leurs arguments/observations et la vraie requête IPC FR/EN CPU/GPU doivent être figés après la faisabilité. Le premier mode serait en lecture seule ; l'installation finale demande l'accord d'écriture hors dépôt, distinct d'une installation test sous `target/`.

### L7 — qualité et recette (4 tâches)

| Tâche | Travail et résultat | Chemins candidats |
| --- | --- | --- |
| L7.1 | Préparer FR lu/spontané et EN lu/spontané, silences et phrase aux frontières ; références humaines relues, WAV maîtres et MP3 dérivés, mêmes octets aux deux moteurs. | `crates/whisper-adapters/tests/fixtures/parakeet_quality/{README.md,references.json}` candidats ; audio local sous `target/parakeet-quality/corpus/` hors Git. |
| L7.2 | Comparer WER/CER par extrait/langue et écarts SRT appariables selon normalisation NFC/casse/ponctuation ; attribuer calcul et variante. | `crates/whisper-adapters/tests/parakeet_quality.rs` candidat ; résultats sous `target/parakeet-quality/results/`. |
| L7.3 | Sortie absente, calcul non attesté, langue non qualifiée ou segment non appariable → `NonComparable` motivé, jamais score fabriqué. | Même test et corpus. |
| L7.4 | Soumettre import/micro, dernière phrase et sorties à l'utilisateur dans la vraie fenêtre Windows ; consigner observations et `NOT RUN`, sans seuil qualité/vitesse ajouté. | `docs/cadrage/bilan-evo-parakeet.md` candidat, unique bilan L7. |

Les sources audio autorisées et références sont des prérequis matériels, pas des fichiers présumés disponibles. Aucun parcours UI Windows n'est réputé validé par un test de build.

## 4. Cartes de vérification et couverture

Toutes les cartes sont **NOT RUN**. Les chemins de tests ajoutés et les données locales devront être exacts dans le plan révisé ; les coûts ci-dessous sont des ordres de grandeur de campagne, hors compilation et préparation des données, et non des seuils produit.

| Carte / lot | Stimulus et témoin | Observation et attendu | Owner / prérequis |
| --- | --- | --- | --- |
| E1 / L1 | Turbo/Parakeet valide, absent, invalide, fichier arbitraire ; demande contournant UI. | Deux entrées, seules configurations admises lançables ; cause et refus applicatif identiques. AC-05/07A ; CAP-31/34. | Implémenteur, reviewer ; G-CAP. |
| E2 / L1 | Auto langue qualifié seul, manuel seul, aucun mode, ancienne langue libre ambiguë/périmée restaurée ou injectée, RAM/VRAM indéterminée après contrôles, résultat tardif. | Ancienne valeur refusée et choix explicite, sans substitution ; capacité obligatoire inconnue bloquée ; ressources seules indéterminées permettent préparation, dont échec réel expliqué ; préférence inchangée. AC-08/16 ; CAP-32/33. | Implémenteur ; Q-CAP-02/03. |
| E3 / L2 | WAV/MP3 annotés français et autre langue, source intacte, runtime absent ou IPC incompatible. | Texte et TXT/SRT ordonnés ; modèle/CPU effectifs ; aucune substitution ni faux succès. AC-01/02/09/13/16/18. | Implémenteur, validateur ; G-DESIGN CPU/Auto. |
| E4 / L3 | GPU visé, identité fausse, échec avant/après première entrée audio en inférence, capture éventuellement déjà active. | GPU natif attesté ; refus manuel ; Auto au plus une tentative CPU du même modèle avant première entrée audio en inférence, aucun repli après. AC-03/04/15. | Validateur, reviewer ; RISK-T-01c. |
| E5 / L4 | Parole aux bords, silence, queue courte, Stop avant Ready, saturation/écriture partielle ; tentative d'import pendant micro actif et de micro pendant import actif. | Blocs sans doublon, dernière plage admise conservée, TXT/SRT/WAV cohérents ou incomplet annoncé ; second parcours concurrent refusé sans perturber le premier. AC-09/10/11/13/16. | Implémenteur, validateur ; Q-PARA-08 et RISK-T-01d. |
| E6 / L4 | Deux séances, changement de modèle, micro explicite absent au redémarrage puis perte de la source pendant capture alors qu'une autre source existe, worker bloqué. | Aucun résultat périmé, défaut Windows avec message à la réouverture, aucune substitution pendant la séance interrompue, fin physique non inventée. AC-12/14/17. | Implémenteur, utilisateur pour fenêtre ; Q-PARA-09. |
| E7 / L5 | Hors ligne, transfert interrompu, mauvais hash, disque plein, annulation/publication concurrente. | Modèle partiel grisé, autre valide préservé, annulation confirmée ou déjà terminée, préférence conservée. AC-07B–E/13/18. | À affecter ; G-ACQ. |
| E8 / L6 | Compte standard, chemin avec espaces, paquet réel et deux langues par variante annoncée. | Installation/réouverture correctes ; AC-16F conclut par variante ; pas de secours Test/Downloads ni faux language ID. AC-07F/16F. | Validateur, utilisateur pour fenêtre ; paquet exact. |
| E9 / L7 | Corpus annoté accepté et mêmes extraits Turbo/Parakeet ; extrait ou sortie absent, langue/calcul non qualifié, horodatage non alignable. | Écarts texte/temps rapportés par langue/calcul lorsque comparables ; sinon `NonComparable` avec cause, sans score fabriqué ni vitesse ou qualité extrapolée. AC-19. | Analyste, validateur, utilisateur ; Q-PARA-05. |

La couverture inverse est : PARA-01–04 → E3/E4/E9 ; 05–08 → E1/E2/E7/E8 ; 09 → E3/E9 ; 10–15 → E4–E6/E7 ; 16 → E2/E3/E5/E8 ; 17 → E6 ; 18 → E3/E7 ; 19 → E9. CAP-31–34 → E1/E2/E8. Toute carte sans données/chemins observables lors de la révision impose `PLAN_CHANGE_REQUIRED` ; un code retour nul ne vaut pas réussite d'AC.

**Sous-cartes nécessaires au plan final, toutes `NOT RUN`.** Les budgets ci-dessous sont des plafonds candidats hors compilation et préparation des données ; ils ne sont pas des délais promis au produit. Les chemins sous `target/` sont des écritures de campagne à inclure dans `lot.paths.runtime_write` avant exécution.

| Carte | Stimulus, chemins et observation | Budget candidat / prérequis |
| --- | --- | --- |
| E1m / L1.4 | Fixtures v1 Turbo exact, historique absent, inconnu/invérifiable, `gpu_device_id` sans calcul, échec sauvegarde et ACK ancien ; v2 Auto explicite. `target/parakeet-tests/migration/`. Réouverture : diagnostic/choix explicite, aucun calcul déduit de v1, `FollowProbe` par défaut, Auto sauvegardé résiste à la sonde tardive, autres réglages préservés. | ≤10 min ; API de migration/injection. |
| E3c / L2.4 | WAV/MP3 et barrières avant/après publication ; `target/parakeet-tests/import/`. Avant : source et pending intacts, annulation, résultat tardif rejeté ; après : arrêt trop tardif et issue réelle ; Quit identique. | ≤15 min ; hooks déterministes et worker qualifié. |
| E7a / L5.1–3 | Petits artefacts synthétiques du catalogue de test, corruption, écritures courtes, accès refusé ; `target/parakeet-tests/acquisition/`. Seul fichier entier validé publié ; autre modèle valide inchangé. | ≤15 min ; IO/transport injectables. |
| E7b / L5.2/4 | Deux consommateurs et barrières avant/après publication ; `Busy` ou verrou réel refusé, annulation acceptée interdit publication, confirmation après fin IO, événement périmé sans effet. | ≤15 min ; handles Windows, aucun jugement par durée. |
| E7c / L5.3/5 | Certificat/nom refusé, HTTP ou rétrogradation HTTPS→HTTP refusés, redirection HTTPS dans la borne permise puis dépassement refusé, interruption/disque plein ; erreurs typées, aucun `Ready`, aucun audio/texte réseau, partiel ignoré après relance. | ≤15 min ; politique transport figée ; double réseau distinct du TLS réel. |
| E7d / L5.4/5 | Réparation explicite puis fermeture pendant IO ; fermeture en cours visible, fin physique constatée, autre modèle conservé. | ≤15 min ; barrière déterministe et fenêtre utilisateur distincte. |
| E8a / L6.1–4 | Paquet complet puis worker/DLL absent ou version incompatible ; `target/parakeet-package/install space/`, témoins seulement dans Test/Downloads. Erreur expliquée, aucun secours, réouverture offline si modèles Finals valides. | ≤30 min ; paquet/layout figés. |
| E8b / L6.2/4 | Compte standard et `%LOCALAPPDATA%\Programs\Whisper`, réparation/réouverture, chemin avec espaces ; droits réels et profil Final seuls observés. | ≤30 min ; accord écrit précis pour écritures hors dépôt ; sinon `NOT RUN`. |
| E8c / L6.5/6 | FR/EN annotés, CPU puis GPU du paquet exact ; `target/parakeet-package/results/`. Appels et sorties corrélés à calcul/paquet, verdict par variante, aucune langue détectée inventée. | ≤45 min ; entrée native/arguments et GPU attendu figés. |
| E9a / L7.1–3 | Même corpus gelé pour les deux moteurs ; `target/parakeet-quality/results/`. WER/CER, SRT et `NonComparable` attribués, aucune extrapolation. | ≤60 min ; audio/références autorisés et paquet qualifié. |
| E9b / L7.4 | Fenêtre Windows réelle import/micro ; blocs, dernière phrase et sorties observés, réserves et sous-cas non exécutés consignés. | ≤30 min ; utilisateur et écritures runtime autorisées. |

Rapprochement inverse : E1m → migration L1 ; E3c → application/supervisor/archive L2 ; E7a–d → store/acquisition/bootstrap/UI L5 ; E8a–c → package/install/composition L6 ; E9a/b → corpus/test/bilan L7. Les journaux, réglages, modèles, DLL et sorties sont des chemins indirects à déclarer. Tout chemin découvert hors lot entraîne `PLAN_CHANGE_REQUIRED` avec cause, impact et owner.

## 5. Préflight, commandes et reprise futurs

**Pour chaque lot, avant toute écriture :** relire `AGENTS.md`, `RAILGUARD.md`, les deux parents Git et leurs revues exactes ; relever HEAD, branche et changements locaux ; vérifier les gates DOCUMENT/CODE/EXECUTION, `lot.paths`, dépendances, source des modèles, cible Windows et matériel. Le lot et ses chemins nécessitent un accord écrit précis **dans la session d'exécution**. Les nouveaux crates, FFI et code C++/CUDA, dépendances Cargo, scripts/installateur, téléchargements, écritures hors dépôt ou changement de RG-08 nécessitent les accords particuliers applicables. Ne pas inférer ces accords de la validation future du plan.

**Commandes proposées, toutes NOT RUN :** depuis `C:\dev\whisper` dans PowerShell, `git status --short --branch`, `git rev-parse HEAD`, `git diff --check`, puis, après code autorisé, `cargo fmt --all -- --check`, `cargo build --offline --locked --workspace`, `cargo test --offline --locked --workspace`. Après création des cibles annoncées, les tests ciblés proposés sont `cargo test --offline --locked -p whisper-adapters --test model_preferences`, `--test import_cancellation`, `--test model_store`, `--test model_acquisition`, `cargo test --offline --locked -p whisper-core --test acquisition_contract`, et `cargo test --offline --locked -p whisper-bootstrap --test install_contract` puis `--test package_contract`. Leur réussite signifie que les assertions des branches et effets E1m/E3c/E7/E8 concernés passent, pas seulement qu'un processus sort à zéro. Ces commandes ne prouvent ni l'inférence native, ni l'identité GPU, ni la capture réelle, ni la qualité dans la fenêtre. Les commandes des essais natifs, données, arguments, sorties et attentes doivent être fixés à partir du design révisé et du paquet retenu ; aucun téléchargement automatique n'est déduit d'un échec offline.

**Entrées de campagne à vérifier :** les tests existants `import_cpu`, `import_mp3` et `gpu_selection_worker` peuvent réussir sans lancer leur branche native si leurs variables `WHISPER_L01_*`, `WHISPER_L02_*` ou `WHISPER_L04_*` manquent. Le préflight doit prouver la présence des données et compter les branches réellement exécutées ; les futurs tests Parakeet dédiés refuseront une campagne native demandée mais non exécutée. Plusieurs tests existants écrivent dans `std::env::temp_dir()` : avant leur lancement, `TEMP` et `TMP` du processus de campagne doivent viser une racine `target/parakeet-tests/tmp/` inscrite dans `lot.paths.runtime_write`. Le code actuel de `root.rs` cherche des workers dans plusieurs emplacements ; le profil Final L6 doit éprouver l'absence effective de secours sibling, `target` ou chemin relatif.

**Routage de réalisation après autorisation :** la session principale confie chaque lot borné à `rust_implementer` en mode `IMPLEMENT`, puis le candidat concret à `rust_review_coordinator`, qui organise `rust_reviewer` indépendant et les corrections par `rust_implementer` ; deux cycles revue/correction au maximum. Validation native et produit restent distinctes ; la fenêtre Windows finale appartient à l'utilisateur. En cas d'échec, préserver Git, modèles valides, audio et sorties ; corriger seulement les chemins du lot ou soumettre un `PLAN_CHANGE`, sans reset global ni suppression générale de `target/`.

**Travail de plan restant :** achever l'essai `SPIKE-PARA-02` autorisé et revoir ses preuves ; confirmer dans le design l'ABI/DLL réelles, la composition transitive du paquet, le transport et l'entrée native, puis obtenir `DESIGN READY`. Le mécanisme d'installation Rust est choisi conditionnellement, sans qualification du paquet. Figer ensuite pour chaque lot les chemins directs/indirects et fichiers de tests, les sources FR/EN annotées, les commandes natives et les dépendances sans cycle, puis demander une revue indépendante sur le plan Git. Tant que ces éléments manquent, verdict `DESIGN_CHANGE_REQUIRED` et `PLANS NOT READY`. La modification proposée de RG-08 est une décision séparée à autoriser avant tout réseau applicatif ; aucun résultat ici ne l'active.
