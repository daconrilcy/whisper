Avancée : séquencement DRAFT de l'évolution Parakeet et du registre commun, proposé par `rust_plan_writer`.
Blocage : design parent `DESIGN BLOCKED` ; faisabilités natives et contrats structurants non fermés, chemins de distribution encore à définir.
Décision : `DESIGN_CHANGE_REQUIRED` ; ce document n'est ni `PLANS READY`, ni une autorisation de coder, d'essayer le moteur ou de publier.

# Plan d'implémentation Parakeet — projet conditionnel du 15 octobre 2026

**Identité :** `P-PARA-DRAFT-01`. **Objectif produit :** ajouter Parakeet TDT 0.6B v3 à Turbo dans l'application Windows, pour import et micro, CPU/GPU/Auto calcul, avec catalogue fermé, Auto langue Parakeet seulement après qualification, acquisition finale et conservation des garanties audio et de sorties. **Parents Git :** `docs/cadrage/evo-parakeet.md` et `docs/cadrage/evo-engine-capabilities.md` à `87f46877dcc8d96f1170da4b97eda6bb1b3c6712`. **Propriétaire du découpage :** `rust_plan_writer`, contribution `PLAN-PARA-DRAFT v0.1` en lecture seule ; auteur Git : session principale. **Statut :** DRAFT, non exécutable. Une modification du design parent impose de rapprocher ce plan du nouveau commit avant revue `PLANS READY`.

Le choix DEC-PARA-19 retient la transcription multilingue sans langue imposée pour Parakeet. Il ne démontre pas que le GGUF et NeMo-Speech.cpp v0.2.0 intégrés la réalisent. Le manuel de ce couple reste `Unsupported` ; aucun parcours Parakeet ne démarre avant qualification d'Auto pour la configuration exacte. La restitution d'un identifiant de langue détectée est facultative et distincte. Les comparaisons de vitesse et de charge sont différées ; aucun seuil de perte de qualité n'est inventé.

## 1. Entrées à fermer avant un plan READY

| Gate | Fermeture nécessaire ; owner | Lots concernés |
| --- | --- | --- |
| G-DESIGN | `PARA-REV-01` et `RISK-T-01a–e` : artefact/DLL, inférence CPU, GPU réel sur la cible, fenêtres et queue micro, Auto langue du candidat exact. Un essai ciblé doit avoir un mandat distinct et une revue indépendante ; la compilation partielle antérieure ne suffit pas. Architecte et exécutant autorisé. | L2–L4, puis L6 |
| G-CAP | `Q-CAP-02/03` : politique d'admission des capacités `Unknown/Partial`, besoins/observations RAM, VRAM et disque comparables, sans seuil inventé. Architecte ; utilisateur si la politique change le risque produit. | L1–L4 |
| G-CAPTURE | `Q-PARA-08/09` : pression de file, limite WAV, écriture partielle, Stop, stagnation et constat de fin du worker. Architecte ; aucun délai candidat 2 s/5 s n'est accepté par ce plan. | L4 |
| G-ACQ | `Q-PARA-02/10/11`, `RISK-T-ACQ01` : magasin/publication et fermeture pendant acquisition, source/empreinte/licence des deux modèles, mécanisme de distribution et modification ciblée de RG-08 pour la réparation applicative. Architecte ; utilisateur pour le cadre. | L1 pour identité finale ; L5–L6 |
| G-QUAL | `Q-PARA-05` : corpus, langues, référence annotée, normalisation et méthode de restitution acceptés ; `Q-PARA-06/07` fermées avant import/migration. Analyste, domaine et utilisateur pour représentativité ou comportement visible. | L1–L2, L7 |

Le parent n'est pas `DESIGN READY` et la revue du plan n'a pas eu lieu. Les lignes ci-dessous sont des **hypothèses de découpage**, pas des décisions structurantes cachées dans des tâches. `G-DESIGN` doit être fermé puis le design revu avant de convertir ces lots en `lot.paths` autorisés. La source native C++/CUDA, le FFI Rust, leurs dépendances, essais et installations nécessitent les accords précis applicables dans la session d'exécution ; l'ancien SPIKE n'en donne pas de nouveaux.

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
| L1.2 | Évaluation applicative unique des modèles et modes linguistiques ; refus identique dans le widget et avant séance, sans substitution. | `crates/whisper-core/src/application.rs`, `ports.rs` ; tests Auto qualifié, manuel absent et aucun mode (PARA-05/16, CAP-33). |
| L1.3 | Constat des artefacts et du PC injecté par composition, avec `Unknown` distinct d'`Insufficient`. | `crates/whisper-adapters/src/lib.rs`, modules catalogue/hôte à nommer, `crates/whisper-desktop/src/root.rs` ; tests absent, invalide, mesure manquante/tardive (CAP-32). |
| L1.4 | Migration du chemin modèle Turbo connu vers préférence `ModelId`, distincte d'Auto calcul ; préférence Parakeet absente conservée. | `crates/whisper-core/src/path_settings.rs`, `crates/whisper-adapters/src/path_settings.rs` ; fixture de migration isolée, réouverture et échec d'écriture (PARA-08). |
| L1.5 | Liste de modèles et de langues issue de l'application ; retirer chemin modèle et langue libre, préserver champs audio/sortie. | `crates/whisper-desktop/src/ui.rs`, `root.rs` ; inspection du vrai widget et refus côté application (PARA-05/06/16). |
| L1.6 | Vérifier non-régression Turbo, sélections périmées, modèle absent et source de test exclusivement locale. | Tests `crates/whisper-core/tests/`, `crates/whisper-adapters/tests/`, `crates/whisper-desktop/tests/` à nommer exactement ; AC-PARA-07A/08 et CAP-31–34. |

**Gate L1 :** G-CAP, règle Q-PARA-07 et identité du catalogue. Aucun modèle inconnu ou fichier arbitraire ne devient sélectionnable. Les chemins de modules/tests nouveaux restent candidats tant que le design n'en a pas fixé le contrat et le plan final le `lot.paths`.

### L2 — premier import Parakeet CPU (6 tâches)

| Tâche | Travail et résultat | Chemins d'écriture candidats ; contrôle futur |
| --- | --- | --- |
| L2.1 | Implémenter le worker et la façade du backend CPU acceptés par le design, avec propriété/libération des handles natifs. | Nouveau module/worker Parakeet à choisir après G-DESIGN, `Cargo.toml`/`Cargo.lock` seulement si approuvés ; essai CPU natif attribué. |
| L2.2 | Versionner IPC : release, modèle/révision, séance/tentative, Auto langue et calcul ; refuser worker/version incompatible. | `crates/whisper-core/src/ipc.rs`, `crates/whisper-adapters/src/worker_ipc.rs`, worker retenu ; test identité fausse et résultat périmé. |
| L2.3 | Router l'import WAV/MP3 vers le modèle sélectionné, sans secours Turbo ; recontrôler l'artefact à l'ouverture. | `crates/whisper-core/src/application.rs`, `crates/whisper-adapters/src/worker_ipc.rs`, `decoder.rs`, `crates/whisper-desktop/src/root.rs` selon nécessité ; modèle/CPU attestés. |
| L2.4 | Publier TXT/SRT lisibles et source importée intacte ; annulation et sortie incomplète selon Q-PARA-06. | `crates/whisper-adapters/src/archive.rs`, `supervisor.rs` si requis ; WAV/MP3 annotés et source avant/après. |
| L2.5 | Émettre les sondes réellement observées sans annonce de gain. | Journal existant, `crates/whisper-desktop/src/reactivity_log.rs` et raccords nécessaires ; modèle, CPU, première inférence et sorties corrélés. |
| L2.6 | Vérifier refus de mode manuel Parakeet, Auto non qualifié, erreurs moteur et non-régression Turbo. | `crates/whisper-core/tests/import_contract.rs`, `crates/whisper-adapters/tests/import_cpu.rs`, `import_mp3.rs`, `worker_control.rs`, tests worker à fixer. |

**Gate L2 :** preuve CPU et Auto langue ciblée sur le candidat exact, contrats ABI/IPC acceptés, Q-PARA-06. Premier chemin de bout en bout attendu : WAV français et seconde langue annotée → texte intelligible, TXT/SRT ordonnés, CPU effectivement utilisé et source intacte ; un chargement réussi seul ne suffit pas.

### L3 — GPU et Auto calcul (5 tâches)

| Tâche | Travail et résultat | Chemins d'écriture candidats ; contrôle futur |
| --- | --- | --- |
| L3.1 | Intégrer variante native GPU et relier identité du GPU demandé à l'inférence réellement exécutée. | Worker GPU/Parakeet retenu, `crates/whisper-worker-gpu/src/gpu_attestation.rs`, `native_ffi.rs` si réutilisés ; essai GPU natif ciblé. |
| L3.2 | Vérifier modèle, langue et GPU avant audio ; refuser périphérique incorrect ou backend indisponible. | `crates/whisper-core/src/gpu_selection.rs`, `ipc.rs`, `crates/whisper-adapters/src/worker_ipc.rs` ; erreur sans faux `Ready GPU`. |
| L3.3 | Auto calcul : GPU compatible puis au plus une tentative CPU du **même modèle** avant première inférence ; aucun repli tardif. | `crates/whisper-core/src/compute_policy.rs`, `application.rs`, routeur IPC ; tests avant/après début audio. |
| L3.4 | Garder CPU/GPU manuels fixes, préférence Auto et cause/calcul effectif visibles. | `crates/whisper-desktop/src/ui.rs`, `root.rs` ; essais d'échec GPU manuel et de préférence après redémarrage. |
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

### L5 — acquisition et réparation finale (découpage pendant, 4 résultats)

1. Vérifier à l'installation uniquement les artefacts absents, via sources/révisions/tailles/empreintes et licences figées ; préserver tout modèle valide.
2. Télécharger vers un temporaire privé, valider avant publication locale ; fichier partiel, réseau interrompu ou disque plein ne deviennent jamais `Ready`.
3. Offrir la réparation **explicitement déclenchée** dans l'application après modification autorisée de RG-08 ; annulation et publication sérialisées, sans résultat tardif activé.
4. Rendre causes et états visibles ; l'échec d'un modèle laisse utilisable un autre modèle valide et conserve la préférence.

**Gate L5 :** G-ACQ. `crates/whisper-bootstrap/src/main.rs`, `crates/whisper-adapters/src/` et `crates/whisper-desktop/src/` sont des surfaces à examiner, **pas** des chemins autorisés : mécanisme de magasin, réseau, concurrence, fermeture et fichiers exacts restent à concevoir. Le plan writer devra convertir ces quatre résultats en tâches avec `lot.paths`, tests et erreurs observables avant `PLANS READY`.

### L6 — distribution Windows et paquet réel (découpage pendant, 4 résultats)

1. Composer l'installation par utilisateur courant sous `%LOCALAPPDATA%\Programs\Whisper`, workers et DLL privés ; aucune dépendance globale ou secours Downloads/Test.
2. Vérifier compte standard, chemin avec espaces, installation hors ligne avec modèles déjà valides et réouverture.
3. Exécuter `AC-PARA-16F` sur le **paquet complet**, avec au moins français et seconde langue annotée, pour chaque variante CPU/GPU annoncée ; rapporter PASS/FAIL/INCONCLUSIVE séparément et ne pas inventer d'identifiant de langue.
4. Vérifier isolation Test/Final, contenu, licences, empreintes et la disponibilité réelle après réparation/annulation.

**Gate L6 :** mécanisme d'installation, contenu du paquet, chemins/tests exacts et droits du magasin définis par l'architecte. Aucun essai ou écriture sous `%LOCALAPPDATA%` n'est autorisé par ce plan. Les résultats ci-dessus ne constituent pas encore un `lot.paths` exécutable.

### L7 — qualité et recette (4 tâches)

1. Geler corpus, références humaines, langues et méthode après Q-PARA-05 ; conserver la source et attribuer chaque variante réellement qualifiée.
2. Comparer sorties Turbo/Parakeet : texte et horodatages par langue/calcul, erreurs et limites, sans seuil de perte initial ni promesse de vitesse.
3. Vérifier les régressions import/micro, sortie, préférence, modèle indisponible et qualité de la dernière phrase.
4. Soumettre la fenêtre Windows à l'utilisateur pour l'essai final ; consigner ce qui a été observé et les sous-cas `NOT RUN`.

Les chemins des fixtures/résultats et le budget de comparaison dépendent du corpus accepté. Aucun parcours UI Windows n'est réputé validé par un test de build.

## 4. Cartes de vérification et couverture

Toutes les cartes sont **NOT RUN**. Les chemins de tests ajoutés et les données locales devront être exacts dans le plan révisé ; les coûts ci-dessous sont des ordres de grandeur de campagne, hors compilation et préparation des données, et non des seuils produit.

| Carte / lot | Stimulus et témoin | Observation et attendu | Owner / prérequis |
| --- | --- | --- | --- |
| E1 / L1 | Turbo/Parakeet valide, absent, invalide, fichier arbitraire ; demande contournant UI. | Deux entrées, seules configurations admises lançables ; cause et refus applicatif identiques. AC-05/07A ; CAP-31/34. | Implémenteur, reviewer ; G-CAP. |
| E2 / L1 | Auto langue qualifié seul, manuel seul, aucun mode, mesure ressource absente, résultat tardif. | Grisage/autorisation cohérents, `Unknown` non traité comme insuffisant, préférence inchangée. AC-08/16 ; CAP-32/33. | Implémenteur ; Q-CAP-02/03. |
| E3 / L2 | WAV/MP3 annotés français et autre langue, source intacte, runtime absent ou IPC incompatible. | Texte et TXT/SRT ordonnés ; modèle/CPU effectifs ; aucune substitution ni faux succès. AC-01/02/09/13/16/18. | Implémenteur, validateur ; G-DESIGN CPU/Auto. |
| E4 / L3 | GPU visé, identité fausse, échec avant/après première inférence. | GPU natif attesté ; refus manuel ; Auto au plus une tentative CPU même modèle avant audio seulement. AC-03/04/15. | Validateur, reviewer ; RISK-T-01c. |
| E5 / L4 | Parole aux bords, silence, queue courte, Stop avant Ready, saturation/écriture partielle. | Blocs sans doublon, dernière plage admise conservée, TXT/SRT/WAV cohérents ou incomplet annoncé. AC-10/11/13/16. | Implémenteur, validateur ; Q-PARA-08 et RISK-T-01d. |
| E6 / L4 | Deux séances, changement de modèle, micro explicite absent au redémarrage, worker bloqué. | Aucun résultat périmé, défaut Windows avec message, aucune substitution pendant séance, fin physique non inventée. AC-12/14/17. | Implémenteur, utilisateur pour fenêtre ; Q-PARA-09. |
| E7 / L5 | Hors ligne, transfert interrompu, mauvais hash, disque plein, annulation/publication concurrente. | Modèle partiel grisé, autre valide préservé, annulation confirmée ou déjà terminée, préférence conservée. AC-07B–E/13/18. | À affecter ; G-ACQ. |
| E8 / L6 | Compte standard, chemin avec espaces, paquet réel et deux langues par variante annoncée. | Installation/réouverture correctes ; AC-16F conclut par variante ; pas de secours Test/Downloads ni faux language ID. AC-07F/16F. | Validateur, utilisateur pour fenêtre ; paquet exact. |
| E9 / L7 | Corpus annoté accepté et mêmes extraits Turbo/Parakeet. | Écarts texte/temps rapportés par langue/calcul, aucune vitesse ni qualité extrapolée. AC-19. | Analyste, validateur, utilisateur ; Q-PARA-05. |

La couverture inverse est : PARA-01–04 → E3/E4/E9 ; 05–08 → E1/E2/E7/E8 ; 09 → E3/E9 ; 10–15 → E4–E6/E7 ; 16 → E2/E3/E5/E8 ; 17 → E6 ; 18 → E3/E7 ; 19 → E9. CAP-31–34 → E1/E2/E8. Toute carte sans données/chemins observables lors de la révision impose `PLAN_CHANGE_REQUIRED` ; un code retour nul ne vaut pas réussite d'AC.

## 5. Préflight, commandes et reprise futurs

**Pour chaque lot, avant toute écriture :** relire `AGENTS.md`, `RAILGUARD.md`, les deux parents Git et leurs revues exactes ; relever HEAD, branche et changements locaux ; vérifier les gates DOCUMENT/CODE/EXECUTION, `lot.paths`, dépendances, source des modèles, cible Windows et matériel. Le lot et ses chemins nécessitent un accord écrit précis **dans la session d'exécution**. Les nouveaux crates, FFI et code C++/CUDA, dépendances Cargo, scripts/installateur, téléchargements, écritures hors dépôt ou changement de RG-08 nécessitent les accords particuliers applicables. Ne pas inférer ces accords de la validation future du plan.

**Commandes proposées, toutes NOT RUN :** depuis `C:\dev\whisper` dans PowerShell, `git status --short --branch`, `git rev-parse HEAD`, `git diff --check`, puis, après code autorisé, `cargo fmt --all -- --check`, `cargo build --offline --locked --workspace`, `cargo test --offline --locked --workspace`. Ces commandes contrôlent le format, la compilation et les contrats exécutés ; elles ne prouvent ni l'inférence native, ni l'identité GPU, ni la capture réelle, ni la qualité dans la fenêtre. Les commandes des essais natifs, données, arguments, sorties et attentes doivent être fixés à partir du design révisé et du paquet retenu ; aucun téléchargement automatique n'est déduit d'un échec offline.

**Routage de réalisation après autorisation :** la session principale confie chaque lot borné à `rust_implementer` en mode `IMPLEMENT`, puis le candidat concret à `rust_review_coordinator`, qui organise `rust_reviewer` indépendant et les corrections par `rust_implementer` ; deux cycles revue/correction au maximum. Validation native et produit restent distinctes ; la fenêtre Windows finale appartient à l'utilisateur. En cas d'échec, préserver Git, modèles valides, audio et sorties ; corriger seulement les chemins du lot ou soumettre un `PLAN_CHANGE`, sans reset global ni suppression générale de `target/`.

**Travail de plan restant :** après DESIGN READY, figer pour chaque lot ses chemins directs/indirects et fichiers de tests, cartes d'exécution avec données et observations, mécanisme L5/L6, dépendances sans cycle, puis demander une revue indépendante sur le plan Git. Tant que ces éléments manquent, verdict `DESIGN_CHANGE_REQUIRED` et `PLANS NOT READY`.
