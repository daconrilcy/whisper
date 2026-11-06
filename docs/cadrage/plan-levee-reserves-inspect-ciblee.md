Avancée : plan de levée des réserves établi sur le candidat 8f709378 ; aucune correction appliquée.
Blocage : reprise du lot clos à autoriser séparément ; migration P5 toujours non confirmée.
Décision : proposer un nouveau lot borné de sept tâches, sans rouvrir les deux cycles épuisés.

# Plan de correction — levée des réserves INSPECT-CIBLEE

## Header

- **Source audit** : `docs/architecture/INSPECT-CIBLEE-BILAN.md`, `docs/architecture/INSPECT-CIBLEE-PRESERVATION.md`, archive `docs/architecture/raw/INSPECT-CIBLEE-20261106T092341.zip` et constats R1–R4 de la revue indépendante.
- **Planning date** : 6 novembre 2026.
- **Target workspace** : `C:/wt/inspect-ciblee`, branche `inspect-ciblee`, candidat de départ `8f709378ec938a1e35ee7abdba38f966587b4840` ; `main` historique `34975d43`.
- **Planning mode** : plan only, no code changes. La présente demande autorise uniquement ce document et son commit local.
- **Confidence** : `high` pour la correction des tests et la séquence de validation, fondées sur les sources ; la durée du contrôle générique et la réalisation humaine de P5 restent à vérifier.
- **Plan quality check** : passed pour l'auto-contrôle documentaire ; aucune revue indépendante de ce nouveau plan ni validation d'exécution n'est revendiquée.

## 1. Audit Digest

Le changement produit demandé existe déjà : queue/recovery utilisent `LocalModelCatalog::inspect_requested_model`, comme `bind_import`. La réserve immédiate porte sur les tests et les preuves finales, pas sur une nouvelle optimisation à concevoir.

La revue précédente a fermé **R2** (préflight indépendant du chargement C#) et **R3 code** (racines d'archives incluses récursivement dans les specs). **R1 BLOQUANT / OPEN_VERIFICATION** reste sans sourceguard/N1 sur le candidat final. **R4 IMPORTANT / OPEN** reste en échec : dernier focus **5 réussites, 2 échecs**, tous deux Parakeet, avec `PathOutsideRuntime`.

L'absence de worker avait d'abord masqué le contrôle d'intégrité. Ajouter un PE puis corriger seulement sa racine ne suffit pas : `verify_qualification` appelle aussi `verify_worker_runtime` avant de comparer l'empreinte du modèle. Un répertoire de DLL vide entraînerait encore une autre erreur. Le plan retient donc la séparation explicite des preuves de comptage, de qualification et d'intégrité, sans fabriquer un runtime qualifié.

Autres réserves : contrôle générique P4 échoué à **1057,2679 ms** ; P5 humain non confirmé ; trois N1/P6, format/lints/build finaux non exécutés. Les écarts historiques aux règles d'arrêt et au budget d'essais restent consignés et ne peuvent être effacés par de nouveaux résultats verts.

## 2. Freshness Check

Inspection en lecture seule, sans Cargo : worktree propre au début, branche et HEAD ci-dessus confirmés. Fichiers lus : bilan, annexe complète, ancien plan, AGENTS racine et adapters, tests de caractérisation, `parakeet_import.rs`, `worker_ipc/backends/mod.rs`, tests du catalogue et scripts des gardes. Tous existent. Les deux destinations `C:/wb/preserved/poll-preserved` et `pile-preserved` sont encore absentes au relevé de planification.

Points confrontés aux sources :

- `inspection_runtime` déclare `runtime_root=<fixture>/parakeet-runtime`, mais place son modèle dans `<fixture>/models`, donc hors racine.
- Les deux tests Parakeet confondent encore une preuve d'inspection ciblée et l'attente d'un diagnostic d'intégrité obtenu après qualification du runtime.
- Les tests existants `model_catalog/tests.rs` vérifient déjà `Corrupt`, `Missing`, une empreinte observée et le réhash après altération ; ils sont distincts des erreurs d'admission d'un worker absent.
- `ci/test-native-build-control.ps1` dispose de `-ProtectionFixturesOnly`. Ce mode ne remplace pas le contrôle générique complet, qui impose **elapsed ≤1000 ms et elapsed+100 ms ≤1000 ms**, sans perte de ses **55 414** enregistrements, à chacune des trois itérations internes.
- La garde N1 utilise déjà `C:/wb/inspect-ciblee/cargo-target`, et le mode sans variable vérifie le `target_directory`. Son parseur accepte les commandes proposées ci-dessous.
- L'ancien plan décrit un lot clos ; ses autorisations/cycles ne sont pas réutilisés comme permission de coder. Les chiffres de nettoyage de l'ancienne annexe deviennent périmés dès une nouvelle campagne.

## 3. Planning Assumptions

| Type | Hypothèse et conséquence |
|---|---|
| Safe | Conserver le correctif produit et corriger seulement les tests ; aucune modification de `queue.rs`, `recovery.rs`, `backends/mod.rs` ou des règles de qualification n'est prévue. |
| Safe | Réutiliser les preuves rouges archivées sur `34975d43`, dont les assertions de comptage restent inchangées ; ne pas créer un nouveau worktree ni remplacer temporairement le code produit pour les rejouer. Le reviewer doit croiser sources archivées et assertions finales. |
| Validation needed | Le cache hors réseau, la cible et les gardes sont toujours disponibles et ne sont utilisés par aucun autre lot pertinent. Sinon arrêter avant Cargo ; pas de téléchargement implicite. |
| Validation needed | Le contrôle générique peut passer sans changement de source ni de seuil ; son dépassement antérieur n'a pas de cause établie. Un échec impose un arbitrage, pas une série de retries. |
| Decision gate | G-AUTORISATION : nouveau lot à accepter explicitement avant code/exécution. La réutilisation de la cible externe et les racines proposées doivent faire partie de cet accord. |
| Decision gate | G-P5 : retour humain et vérification des archives déplacées avant fermeture de P5 et vérification des racines réelles ; ne bloque pas la correction des tests ni N1. |

## 4. Finding Traceability

| Audit finding | Severity | Disposition | Task IDs / gate | Notes |
|---|---|---|---|---|
| R1 sourceguard final et N1 manquants | BLOQUANT | Planned | T3, T6, T7 | Ne pas réutiliser le 1/1 devenu périmé après édition. |
| R4 tests Parakeet invalides, intégrité non prouvée par le bon contrôle | IMPORTANT | Planned | T2, T3, T7 | Séparer comptage et diagnostic d'intégrité ; sept scénarios conservés. |
| R2 préflight PowerShell neuf | IMPORTANT | Already resolved | T4, T7 | Conserver la fermeture ; reprendre le test après changement pertinent seulement. |
| R3 couverture de code des archives | IMPORTANT | Already resolved | T4, T5, T7 | Cette fermeture n'inclut pas la migration physique. |
| P4 contrôle générique dépassé | Réserve de validation | Planned | T4 | Aucun assouplissement de 1000 ms ni du délai +100 ms. |
| P5 archives déplacées | Réserve opérationnelle | Decision gate | T5 / G-P5 | Exécution humaine exclusivement. |
| P6 zéro résidu sur trois N1 | Réserve de validation | Planned | T6 | Les mêmes trois N1 closent N1 et P6. |
| Format/lints/build finaux | Réserve de validation | Planned | T3, T6 | Pas de build natif ni de feature native-tests. |
| Écarts arrêt/budget | Risque de processus | Planned | T1, T7 | Prévenir la répétition ; ne pas réécrire l'historique. |
| P1–P3 et inventaire/propositions P7 | Livrés, limites documentées | Already resolved | T7 | Pas de nouvelle enquête VSS, suppression de worktree ou fusion C1001. |
| Verrou global SHA, inspections micro/démarrage | Dette hors lot | Deferred | Hors périmètre | Leur correction ne conditionne pas les critères d'INSPECT-CIBLEE. |
| Six mesures release | Condition initiale non applicable au périmètre N1 | Deferred | T7 | Conserver justification des compteurs ; aucune promesse de gain chronométré. |

## 5. Recommended Sequence

Corriger d'abord la signification des tests sans toucher au produit ; valider ensuite la source finale et la garde ; qualifier enfin le même candidat par trois N1 et une revue. P5 suit une voie humaine indépendante et bloque uniquement la clôture complète de PRESERVATION.

### Phase A : rétablir des preuves unitaires fiables

- **Objective / Closed invariant** : comptage ciblé indépendant de la disponibilité native, et intégrité vérifiée par un contrôle qui atteint réellement le modèle.
- **Before metrics** : focus 5/7 ; 2 assertions Parakeet hors du chemin attendu ; sourceguard final absent.
- **Target metrics** : au moins les 7 scénarios conservés, tous verts ; sourceguard 1 test exécuté/vert ; erreurs d'intégrité non confondues avec l'absence d'un worker.
- **Metric commands** : C2–C6, C9–C10 ci-dessous.
- **Stop conditions** : échec d'un prérequis/test, zéro test sélectionné, besoin de modifier le produit ou de qualifier des DLL réelles.
- **Dependencies** : G-AUTORISATION.
- **Expected outcome / Tasks** : candidat test-only stable ; T1, T2, T3.

### Phase B : fermer les réserves des gardes et de conservation

- **Objective / Closed invariant** : garde complète verte sans seuil affaibli ; archives réellement conservées hors target avant annonce de P5 terminé.
- **Before metrics** : un burst connu à 1057 ms ; deux destinations absentes ; tests synthétiques R2/R3 seuls verts.
- **Target metrics** : trois bursts internes conformes à leurs deux limites ; deux archives présentes avec comptes/octets/liens identiques et sources sans fichiers/liens restants.
- **Metric commands** : C7–C8, C11 ; contrôles P5 de l'annexe réinventoriés.
- **Stop conditions** : échec générique, migration partielle, cible liée inattendue ou contrôle de racine manquante qui crée cette racine.
- **Dependencies** : T1, G-P5 pour les contrôles réels uniquement.
- **Expected outcome / Tasks** : P4 et P5 démontrés séparément ; T4, T5.

### Phase C : qualifier et remettre le candidat

- **Objective / Closed invariant** : mêmes sources qualifiées par trois N1 sans variable, zéro nouveau résidu, revue indépendante et preuves durables.
- **Before metrics** : aucun N1 final ; R1/R4 ouverts.
- **Target metrics** : trois N1 zéro échec, sourceguard exécuté, R1/R4 fermés par reviewer ; P5 confirmé pour une levée complète.
- **Metric commands** : C12 et contrôles d'archive/revue T7.
- **Stop conditions** : premier échec, preuve périmée par mutation source, deux cycles atteints, P5 non confirmé à la clôture.
- **Dependencies** : T3/T4 ; T5 pour la levée complète des réserves.
- **Expected outcome / Tasks** : complément final qui distingue réserves levées et dettes différées ; T6, T7.

Pas de refonte structurelle : métriques de taille/façade, déplacement de modules et nouvelle architecture sont N/A. Vocabulaire conservé : `core` = domaine/politiques ; `adapters` = I/O et orchestration existante ; `desktop` = composition/UI ; `ci` = gardes ; support de tests privé sous `worker_ipc/tests/`. Aucune dépendance inverse ni API publique ajoutée.

## 6. Task Backlog

### T1 — ouvrir explicitement la reprise et relever les prérequis

- **Priority / Source finding / Rationale** : High ; écarts de campagne et preuves périmées ; empêcher toute reprise implicite du lot clos.
- **Target files/modules / Ownership bucket** : AGENTS et plan en lecture ; contexte Git et racines de campagne ; session principale pour l'autorisation, implémenteur pour le préflight.
- **Scope budget** : 0 fichier de production, 0 changement de garde ; aucun outil de coordination ajouté.
- **Closed invariant / Before / Target / Metric commands** : N/A structurel ; ancienne autorisation épuisée → nouveau périmètre écrit ; C0–C1.
- **Dependencies** : G-AUTORISATION ; **Stop conditions** : accord absent, chemins liés, cache absent, conflit pertinent ou santé hors limites.
- **Implementation outline** : enregistrer HEAD et fichiers sales ; vérifier possession du lot ; contrôler volume ≥30 Gio, mémoire engagée ≤85 %, événements WHEA de départ, cache et racines canoniques ; employer `target/inspect-ciblee/levee-reserves` comme **nouvelle racine proposée**, sans réutiliser les journaux antérieurs.
- **Tests / Verification commands / Command provenance** : C0/C1 puis C7a/C7b avant toute compilation ou test produit ; Repo docs et Source inferred. Les éventuels appels metadata internes aux auto-tests restent sous leur garde.
- **Acceptance criteria** : réutilisation de `C:/wb/inspect-ciblee/cargo-target` et du cache persistant du worktree explicitement couverte par l'accord ; TEMP/TMP/LOCALAPPDATA isolés par garde ; auto-tests verts ; contrôle santé actif dans le shell appelant pendant les appels longs sans service ni watcher supplémentaire. Les commandes de la matrice sont les appels internes : leur lancement seul ne remplace pas le contrôle santé. À défaut de mécanisme d'exécution permettant l'arrêt demandé en cours d'appel, arrêter au préflight, sans créer un outil permanent pour le contourner.
- **Rollback/fallback** : conserver l'état et rapporter le blocage ; aucune suppression/restauration automatique.
- **Notes for implementation agent** : une commande à la fois ; examiner exit code ET stdout complet avant la suivante. Premier échec inattendu = arrêter et demander arbitrage. Zéro test = NON EXÉCUTÉ, jamais PASS. Trois essais/30 min par diagnostic sont un plafond, pas une permission de retry après échec.

### T2 — séparer les fixtures de comptage et d'intégrité

- **Priority / Source finding / Rationale** : High ; R4 ; éliminer la dépendance native accidentelle des deux tests.
- **Target files/modules / Ownership bucket** : `crates/whisper-adapters/src/worker_ipc/tests/queue_inspection_characterization.rs` ; tests privés de l'orchestrateur adapters. `model_catalog/tests.rs` est une référence lue, pas un fichier à réécrire par défaut.
- **Scope budget** : 0 fichier produit ; 1 fichier de tests. Maintenir le placement existant autorisé par l'AGENTS de la crate : méthodes privées `process_queued`/`resume_interrupted`, sans élargir leur visibilité ni déplacer ces scénarios vers une façade publique uniquement pour les tests.
- **Closed invariant / Before / Target / Metric commands** : N/A structurel ; 2 erreurs PathOutsideRuntime → comptages et diagnostic terminal chacun identifiés ; C3–C5.
- **Dependencies** : T1 ; **Stop conditions** : nécessité d'ouvrir un worker, charger une DLL, inventer une qualification, utiliser un modèle réel ou retoucher le produit.
- **Implementation outline** :
  1. Conserver le catalogue et les petits modèles synthétiques au même emplacement ; conserver les deux assertions initiales : Parakeet lu 1 fois, Turbo 0.
  2. Pour les tests de **comptage**, fournir `parakeet_paths=None` intentionnellement. Simplifier le helper test-only si tous ses appelants n'ont plus besoin de worker. Supprimer la fabrication devenue inutile du worker et de son runtime ; ne pas assouplir `ParakeetWorkerPaths::validate`.
  3. Dans queue/recovery Parakeet, vérifier les compteurs et le diagnostic exact de worker non qualifié attendu dans cette configuration. Vérifier les identifiants de l'échec en reprise. Nom/commentaire : preuve de ciblage, **pas preuve d'intégrité**.
  4. Pour l'intégrité, reprendre les tests existants du catalogue : états `Missing`/`Corrupt`, SHA observé, altération entraînant une nouvelle empreinte. Conserver les cas queue Turbo absent/altéré et reprise Turbo avec alias/identité durable ; vérifier leurs diagnostics correspondant aux sources, sans accepter une liste large d'erreurs étrangères.
  5. Préserver les deux ordres de file hétérogène, l'inconnu sans lecture, le refus d'un autre modèle durable et le nettoyage RAII. Ne remplacer aucune assertion de comptage par une tolérance.
  6. Mettre en forme le seul fichier autorisé avec C2a, comme édition de source sous l'environnement isolé de T1, avant C2 ; aucun `cargo fmt` en écriture ni formatage global.
- **Tests to add or update** : les 7 tests actuels ; filtres de catalogue existants listés en C5. Si leur couverture ne suffit pas, arrêter et préciser le scénario manquant plutôt qu'ajouter un faux runtime qualifié.
- **Verification / Provenance** : C2a pour la mise en forme ciblée, lecture du diff puis C2–C5 après édition ; Source inferred.
- **Acceptance criteria** : tous les scénarios conservés ; aucun appel d'intégrité déclaré couvert par une erreur `ModelNotQualified`/`PathOutsideRuntime` ; aucune écriture `.exe` brute ; aucune mutation de code produit ni branche spéciale de test.
- **Rollback/fallback** : conserver le diff de tests pour revue ; ne pas remettre un ancien test vert ni élargir l'OR des erreurs pour obtenir du vert. Si une preuve complète de rejet Parakeet avec runtime réellement qualifié est exigée, la signaler comme besoin distinct non réalisable par ces compteurs N1.
- **Notes for implementation agent** : `verify_qualification` contrôle le worker et les cinq DLL avant de comparer le modèle ; déplacer le modèle sous runtime ne résout donc pas à lui seul le scénario complet. Le chemin alternatif retenu est la séparation des preuves, déjà recommandée par le reviewer.

### T3 — valider les tests finaux et la compilation Rust utile

- **Priority / Source finding / Rationale** : High ; R1/R4 et V3 ; ne pas transporter une preuve d'un ancien fichier.
- **Target / Ownership bucket** : fichiers de tests de T2 et `test_support.rs` en lecture ; garde N1 existante ; validation par implémenteur.
- **Scope budget** : 0 fichier produit ou de garde ; édition seulement T2 si une correction est autorisée après constat.
- **Closed invariant / Before / Target / Metric commands** : N/A structurel ; sourceguard périmé → 1 test exact exécuté et vert, 7/7 ciblés ; C2–C6, C9–C10.
- **Dependencies** : T2 ; **Stop conditions** : tout échec ou filtre vide ; aucune commande suivante lancée automatiquement.
- **Implementation outline** : format final, sourceguard exact, focus complet, tests ciblés du catalogue, build adapters et Clippy tous targets de cette crate ; conserver sorties/codes/commande/HEAD associés.
- **Tests / Verification / Provenance** : C2–C6, C9–C10 ; Source inferred, parseur et noms vérifiés en lecture.
- **Acceptance criteria** : aucune feature native, aucun téléchargement, aucune erreur ; compter tous les harnais, pas seulement la fin du wrapper. Les avertissements Clippy préexistants restent identifiés ; ne pas ajouter `-D warnings` ni corriger une dette sans rapport.
- **Rollback/fallback** : arrêt avec constat précis ; pas de formatage de tout le dépôt ni d'assouplissement de la sourceguard. Une mutation ultérieure des tests périme les vérifications concernées.
- **Notes** : la commande build est une compilation Rust de la crate ciblée ; aucun CMake/ggml/NeMo/CUDA, aucun `cargo run`.

### T4 — reprendre les contrôles de garde sans changer leurs exigences

- **Priority / Source finding / Rationale** : High ; R1–R3/P4 ; séparer la preuve des fixtures de celle du contrôle générique.
- **Target / Ownership bucket** : `ci/guard-level1.ps1`, `ci/guard-native.ps1`, `ci/test-native-build-control.ps1` en lecture ; validation CI locale existante.
- **Scope budget** : 0 fichier produit, 0 modification de script/C# par défaut. Un défaut de garde démontré exige un périmètre correctif distinct accepté avant édition.
- **Closed invariant / Before / Target / Metric commands** : N/A structurel ; burst 1057 ms et fixtures seules → commande générique complète verte ; C7–C8.
- **Dependencies** : T1, T3 avant N1 ; **Stop conditions** : première assertion en échec, dépassement d'une des deux limites ou contrôle présence qui crée une racine manquante.
- **Implementation outline** : réutiliser les SelfTest N1 de T1 tant que script/arguments/environnement pertinents sont inchangés ; sinon reprendre les contrôles affectés ; exécuter une fois le contrôle générique standard dans un PowerShell neuf avec EvidenceRoot neuf ; lire `result.json`, les trois temps et les assertions de perte/parité ; ne pas substituer `-ProtectionFixturesOnly`, `-MeasureOnly`, `-Live` ou `-ColdBurst` au résultat standard.
- **Tests / Verification / Provenance** : C7/C8 ; Source inferred, paramètres lus.
- **Acceptance criteria** : trois bursts de 55 414 événements sans perte, chacun elapsed ≤1000 ms ET elapsed+100 ms ≤1000 ms ; présence/absence en processus neuf et specs récursives passantes ; aucun watcher/build natif démarré. Si un champ n'est pas émis, il reste indéterminé.
- **Rollback/fallback** : si échec temporel, rapporter temps, charge/santé et point en cause ; ne pas répéter pour sélectionner un meilleur temps. Un diagnostic ultérieur doit être arbitré, borné à 3 essais/30 min, sans baisse du seuil ni modification du pipeline.
- **Notes** : un succès nouveau ferme la validation du candidat, pas l'historique des échecs ni les écarts de processus de l'ancien lot.

### T5 — faire confirmer la migration humaine P5

- **Priority / Source finding / Rationale** : High pour clôture PRESERVATION ; P5 ; une constante de chemin ne prouve pas une conservation physique.
- **Target / Ownership bucket** : archives `C:/dev/whisper/target/{poll-preserved,pile-preserved}` vers `C:/wb/preserved/<nom>` ; opérations humaines, agent en lecture seule.
- **Scope budget** : 0 fichier produit ; 0 déplacement/suppression par l'agent ; aucun changement d'ACL, élévation ou réparation de cibles historiques de jonctions.
- **Closed invariant / Before / Target / Metric commands** : N/A structurel ; deux destinations absentes → fichiers/octets/liens préservés et sources sans fichiers/liens restants ; C11 et recette P5 déjà documentée.
- **Dependencies** : retour humain G-P5 ; **Stop conditions** : copie partielle, destination préexistante non rapprochée, reparse inattendu, erreur Robocopy ≥8, différence d'inventaire.
- **Implementation outline** : rafraîchir comptes/octets/liens et parents canoniques avant remise ; employer la recette PowerShell 7 `/MOVE /SJ /SL` de l'annexe seulement si ses préconditions restent vraies ; humain exécute ; agent compare résultats et relit les deux destinations/sources ; vérifier ensuite la fonction réelle `Assert-NativeProtectedRootsExist` sans lancer la recette NativeBuild ni ses watchers.
- **Tests / Verification / Provenance** : contrôles nombre/taille/types/cibles de liens de l'annexe (Repo docs) ; C11 (Source inferred). Les 117586/89160572401 et 8485/11644240959, avec 181/97 liens, sont des références historiques à rafraîchir, pas des valeurs à imposer à une source changée.
- **Acceptance criteria** : métadonnées de migration cohérentes et racines réelles explicitement acceptées ; manque de racine = échec explicite, jamais création silencieuse. La fermeture R3 code reste distincte de cette opération.
- **Rollback/fallback** : conserver les deux emplacements et les sorties en cas de copie partielle ; aucun rollback/move automatique. Si l'humain reporte, annoncer « code validé, P5 restant » et ne pas conclure que toutes les réserves sont levées.
- **Notes** : P7 n'impose aucune suppression pour lever ces réserves ; build-natif reste conservé et aucun bilan C1001 n'est fusionné ici.

### T6 — exécuter les trois N1 finaux communs à P6

- **Priority / Source finding / Rationale** : High ; R1/N1/P6 ; fermer les preuves globales sur un seul candidat stable.
- **Target / Ownership bucket** : membres Cargo par défaut ; `parakeet_import/legacy_runtime_tests.rs` inchangé ; validation N1.
- **Scope budget** : 0 fichier de production/tests pendant les trois runs.
- **Closed invariant / Before / Target / Metric commands** : N/A structurel ; 0 N1 final → trois runs zéro échec, zéro nouveau résidu ; C12.
- **Dependencies** : T3/T4 verts ; T5 peut attendre sans bloquer N1 ; **Stop conditions** : un échec, compteur global incohérent, variable présente dans enfant, nouveau résidu attribuable ou mutation source.
- **Implementation outline** : dans trois shells dédiés successifs, appeler C12 avec `-WithoutCargoTargetDir` ; vérifier l'absence de la variable enfant et `target_directory=C:/wb/inspect-ciblee/cargo-target` ; avant/après chaque run inventorier TEMP effectif annoncé et TEMP utilisateur pour `parakeet-personal-no-l3b-*` ; ne rien supprimer manuellement pour produire zéro.
- **Tests / Verification / Provenance** : `cargo test --no-fail-fast` via garde, C12 ; Repo docs. Ce sont les mêmes trois exécutions pour P6 et N1, aucune série supplémentaire implicite.
- **Acceptance criteria** : sorties/codes et compte passé/échec/ignoré de chaque run listés individuellement ; résultat attendu **722/0/128** si seuls les sept tests actuels sont conservés par rapport à la référence **715/0/128**. Toute différence doit être expliquée par les noms/tests réels, sans modifier les ignorés pour ajuster le chiffre.
- **Rollback/fallback** : premier échec = arrêt et preuve conservée ; pas de N1 suivant. Une correction ensuite autorisée impose de refaire la séquence finale sur le nouveau candidat.
- **Notes** : les sélections workers de la suite de référence ne sont pas couvertes par le chiffre 722 ; si elles sont incluses dans la campagne acceptée, elles passent leur contrôle PE préalable et sont rapportées séparément, jamais mélangées à la comparaison des membres par défaut.

### T7 — revue indépendante, complément de clôture et rétention

- **Priority / Source finding / Rationale** : High ; toutes réserves ; seule une fermeture attribuée et vérifiée permet de changer le statut.
- **Target / Ownership bucket** : reviewer/code et gardes en lecture ; documents écrits seulement par la session principale. Nouveau livrable proposé : `docs/architecture/INSPECT-CIBLEE-RESERVES-COMPLEMENT.md` ; petite archive distincte sous `docs/architecture/raw/` avec nom daté fixé à la remise.
- **Scope budget** : 0 fichier produit ; 1 complément, 1 archive, et au plus un lien vers le complément ajouté au bilan clos. Aucun récit ni compteur réécrit dans le bilan/annexe de clôture précédents.
- **Closed invariant / Before / Target / Metric commands** : N/A structurel ; R1/R4 ouverts → clôture explicite par reviewer ; bilan toujours historique ; C0, inspection de l'archive et matrice de preuves.
- **Dependencies** : T3/T4/T6 ; T5 pour annoncer une levée complète ; **Stop conditions** : constat ouvert, preuve stale, deux cycles du nouveau livrable atteints, élément humain absent.
- **Implementation outline** : principale délègue la revue via `rust_review_coordinator` avec un reviewer indépendant et le même implémenteur pour corrections ; transmettre les IDs sans les renommer ; fermer R1/R4 par le reviewer seul, confirmer non-régression R2/R3 ; compléter statut P1–P7 ; conserver chaque échec et chaque run final ; archiver hors target, comparer nombres et SHA en mémoire, sans manifeste de qualification ajouté ; inventorier les créations/tailles et proposer de nouvelles commandes humaines de nettoyage.
- **Tests / Verification / Provenance** : revue + preuves C2–C12 ; `git diff --check` Observed ; validation archive nombre/SHA Repo docs.
- **Acceptance criteria** : CLEAN uniquement pour le périmètre effectivement revu avec aucun écart/gap matériel ; « toutes réserves levées » uniquement si N1/P6, P4 et P5 sont aussi démontrés. Sinon statut exact avec réserves restantes. Écarts historiques d'arrêt/budget toujours mentionnés, pas déclarés réparés rétroactivement.
- **Rollback/fallback** : conserver branche et preuves ; après deux cycles rendre LIVRÉ AVEC RÉSERVES pour décision utilisateur. Aucun merge/push, aucun nettoyage ou déplacement agent.
- **Notes** : conserver les caches persistants et les archives durables ; les anciennes tailles et commandes de suppression de l'annexe sont périmées après cette reprise, donc ne pas les exécuter aveuglément. Commits locaux seulement par la principale après revue du candidat.

## 7. Verification Matrix

Toutes les commandes ci-dessous sont des **recettes à exécuter après autorisation**, pas des commandes exécutées pendant cette planification. Lancer chaque appel séparément ; ne jamais les coller dans un batch qui continue après un exit non nul. C2–C10/C12 prennent la racine proposée `target/inspect-ciblee/levee-reserves`. La garde conserve ses autres contrôles, l'offline/locked et l'environnement N1.

| ID | Area / Command or check | Provenance | Expected result | Covers tasks |
|---|---|---|---|---|
| C0 | `git --no-optional-locks status --short` ; `git --no-optional-locks rev-parse HEAD` ; `git --no-optional-locks diff --check` | Observed | État attribué, aucun changement tiers écrasé | T1,T7 |
| C1 | Espace via `Get-PSDrive C`, mémoire via `Get-CimInstance Win32_PerfFormattedData_PerfOS_Memory`, WHEA System depuis le début du lot ; racines/cache/liaisons contrôlés | Repo docs | ≥30 Gio, ≤85 %, aucune nouvelle WHEA/crash ; sinon arrêt | T1 et chaque appel |
| C2a | `rustfmt --edition 2024 crates/whisper-adapters/src/worker_ipc/tests/queue_inspection_characterization.rs` | Source inferred | Édition limitée à ce fichier, sous les variables isolées de T1 ; aucune commande Cargo en écriture | T2 |
| C2 | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/inspect-ciblee/levee-reserves -WithoutCargoTargetDir -CargoArgsJson '["fmt","--check"]'` | Source inferred | Exit 0 | T2,T3 |
| C3 | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/inspect-ciblee/levee-reserves -WithoutCargoTargetDir -CargoArgsJson '["test","-p","whisper-adapters","--lib","worker_ipc::tests::queue_inspection_characterization::"]'` | Source inferred | Les 7 tests sélectionnés, tous verts | T2,T3 |
| C4 | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/inspect-ciblee/levee-reserves -WithoutCargoTargetDir -CargoArgsJson '["test","-p","whisper-adapters","--lib","test_support::test_sources_create_executable_fixtures_only_through_pe_constructor","--","--exact"]'` | Repo docs | 1 exécuté, 0 échec ; pas de préfixe de crate ajouté au nom libtest | T3,R1 |
| C5 | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/inspect-ciblee/levee-reserves -WithoutCargoTargetDir -CargoArgsJson '["test","-p","whisper-adapters","--lib","model_catalog::tests::"]'` | Source inferred | Tests du catalogue exécutés, états Corrupt/Missing et altération vérifiés | T2,T3 |
| C6 | Lecture des assertions et des sorties C3/C5 ; correspondance des deux compteurs initiaux avec les sources/logs rouges archivés sur `34975d43` | Repo docs | Aucun diagnostic worker/chemin présenté comme intégrité ; avant/après attribué | T2,T3,T7 |
| C7a | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/inspect-ciblee/levee-reserves` | Source inferred | Avant compilation/tests : positifs/négatifs verts, cible externe en mode variable | T1,T4 |
| C7b | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/inspect-ciblee/levee-reserves -WithoutCargoTargetDir` | Source inferred | Avant compilation/tests : positifs/négatifs verts, variable absente | T1,T4 |
| C8 | `pwsh -NoProfile -File ci/test-native-build-control.ps1 -EvidenceRoot C:/wt/inspect-ciblee/target/native-build/levee-reserves-standard-01` | Source inferred | Répertoire neuf ; commande standard complète verte, 3 temps listés avec les deux limites ; aucune option alternative | T4 |
| C9 | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/inspect-ciblee/levee-reserves -WithoutCargoTargetDir -CargoArgsJson '["build","-p","whisper-adapters"]'` | Source inferred | Compilation Rust exit 0, aucune construction native | T3 |
| C10 | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/inspect-ciblee/levee-reserves -WithoutCargoTargetDir -CargoArgsJson '["clippy","-p","whisper-adapters","--all-targets"]'` | Source inferred | Exit 0, avertissements distingués | T3 |
| C11 | Recette humaine P5 de l'annexe ; après résultats, charger par AST **uniquement** `Get-FullPath`, `Assert-NoReparseChain`, `Get-NativeProtectedRoots`, `Assert-NativeProtectedRootsExist` du script existant, appeler ce dernier dans un PowerShell neuf avec `$script:Repo` égal au worktree et le LOCALAPPDATA utilisateur réel ; lire les specs récursives sans les démarrer | Source inferred | Deux destinations cohérentes, racines réelles présentes ; aucun Invoke-NativeBuildProcess/Invoke-NativeDllCommand ni watcher | T5 |
| C12 | `pwsh -NoProfile -File ci/guard-level1.ps1 -Mode Run -LotRoot target/inspect-ciblee/levee-reserves -WithoutCargoTargetDir -CargoArgsJson '["test","--no-fail-fast"]'` | Repo docs | À lancer 3 fois, chaque fois dans un shell dédié et après examen du précédent ; zéro échec, compte global justifié, zéro nouveau résidu P6 | T6 |

C8 est un test synthétique de garde existant, pas une compilation native. Son script isole TEMP/TMP sous son EvidenceRoot. Ne pas changer de cache/profil pour accélérer le test. Les assertions/résultats restent intégralement conservés, y compris un éventuel échec. Les commandes de lecture C11 sont une recette ciblée à matérialiser dans la commande du shell de validation, sans créer un helper permanent ; elles ne doivent pas exécuter le corps principal de `guard-native.ps1`.

Recette C11, **après confirmation P5 seulement**, dans un PowerShell neuf placé au worktree, avec le LOCALAPPDATA utilisateur réel. Elle charge des définitions de fonctions par AST, sans exécuter le script principal et sans créer de racine :

```powershell
$ErrorActionPreference = 'Stop'
$script:Repo = (Resolve-Path -LiteralPath 'C:/wt/inspect-ciblee').Path
if (-not $env:LOCALAPPDATA) { throw 'LOCALAPPDATA utilisateur absent' }
$parseTokens = $null
$parseErrors = $null
$guardPath = Join-Path $script:Repo 'ci/guard-native.ps1'
$guardAst = [Management.Automation.Language.Parser]::ParseFile($guardPath,[ref]$parseTokens,[ref]$parseErrors)
if ($parseErrors.Count) { throw 'Syntaxe de garde invalide' }
foreach ($name in @('Get-FullPath','Assert-NoReparseChain','Get-NativeProtectedRoots','Assert-NativeProtectedRootsExist')) {
    $definition = $guardAst.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -ceq $name}.GetNewClosure(),$true)
    if ($null -eq $definition) { throw "Fonction absente : $name" }
    . ([scriptblock]::Create($definition.Extent.Text))
}
$actualRoots = @(Assert-NativeProtectedRootsExist)
foreach ($archiveRoot in @('C:\wb\preserved\poll-preserved','C:\wb\preserved\pile-preserved')) {
    if ($actualRoots -notcontains $archiveRoot) { throw "Archive non protégée : $archiveRoot" }
}
$actualRoots
# Confronter ensuite Get-RunWatchSpecs/Add-NativeProtectedRootWatches en lecture
# aux specs récursives déjà vérifiées en C8 ; ne démarrer aucun watcher.
```

Pour C12, relever chaque total à partir de **toutes** les lignes `test result` et vérifier les codes de chaque commande ; un simple `exit 0` ou les derniers harnais à zéro test ne constituent pas un décompte. Pour P6, sauvegarder les listes avant/après au TEMP isolé effectif et sous `%LOCALAPPDATA%/Temp` réel. Les dossiers préexistants ne sont ni supprimés ni imputés automatiquement au nouveau run.

## 8. Decision Gates

| Gate | Status | Blocks | Question | Recommended default | How to validate |
|---|---|---|---|---|---|
| G-AUTORISATION | Blocking | Toute édition/exécution de T1–T6 | Le nouveau lot, ses chemins et sa réutilisation de cible externe sont-ils acceptés ? | Rester en planification tant que l'accord précis manque | Accord écrit dans la session d'implémentation, plan commité, chemins et prérequis relevés |
| G-P5 | Blocking | Validation des racines réelles et levée complète, pas T2/T3/T6 | Migration humaine achevée et vérifiable ? | Poursuivre les travaux indépendants ; conserver P5 en réserve | Résultats avant/après et lecture réelle des destinations/sources |
| G-GÉNÉRIQUE | Validation | T6 si C8 échoue | Le contrôle complet est-il vert avec ses seuils inchangés ? | Un seul essai initial ; en cas d'échec arrêter pour arbitrage | JSON intégral et 3 durées ; pas de mode partiel substitué |
| G-FIXTURE | Validation | Fermeture R4 | La séparation des preuves couvre-t-elle les exigences sans runtime qualifié ? | Comptage sans worker + diagnostics catalogue/Turbo explicites | Reviewer croise code, assertions, sorties et limites annoncées |

Ces gates n'ajoutent aucune autorisation de service, ACL, installation, élévation, nouvelle surveillance ou build natif. Les faits nouveaux de validation ne modifient jamais le bilan clos.

## 9. Out of Scope

- **Do not do** : troisième cycle de l'ancien lot ; modification opportuniste du produit ; changement des règles de qualification/SHA/chemins admis pour satisfaire une fixture.
- Aucun faux état de qualification, aucune copie de DLL/modèle installé, aucun SHA d'un modèle réel de 1,6 Go, aucun worker natif lancé.
- Aucun cache SHA Parakeet taille/mtime/fileId, correction du mutex, refacto de polling, extension micro/démarrage, nouveau trait/hook/dépendance ou modification Cargo.lock.
- Aucun déplacement de tests/modules ni migration générale des tests inline. Conserver les déclarations/include et la visibilité actuels ; la réparation du fichier privé existant ne justifie pas une API de tests publique.
- Pas de baisse du seuil générique, suppression d'assertion, option `-MeasureOnly`/`-ProtectionFixturesOnly` pour remplacer le contrôle complet, feature native-tests/ignored, ou exécution Cargo hors garde.
- Aucune réécriture des bilans figés, suppression/move agent, fusion de build-natif/C1001, merge/push ou CI distante.

## 10. Final Self-Check

| Check | Result | Notes |
|---|---|---|
| Plan réel, sections du modèle dans l'ordre | Pass | Sept tâches ; aucun code produit modifié |
| Tous les constats tracés | Pass | R1–R4, P4/P5/P6, processus, validations et dettes différées |
| Fichiers, dépendances, outline, tests, critères et repli par tâche | Pass | T1–T7 |
| Métriques/arrêts explicites | Pass | Invariants de preuve ; aucune extraction structurelle proposée |
| Scope budget par tâche | Pass | Zéro fichier produit à corriger par défaut ; un fichier de tests |
| Placement Rust conforme au projet et exception expliquée | Pass | Réparation de tests privés séparés, pas de nouvel inline ni élargissement d'API |
| Matrice de commandes avec provenance | Pass | C0–C12 ; rien exécuté en planification sauf lectures/Git/diff |
| Gates ciblées et sans blocage inutile | Pass | P5 indépendant du correctif/N1 ; autorisation avant code |
| Chemins valides ou nouveaux emplacements proposés explicitement | Pass | Worktree observé ; nouveaux EvidenceRoot/racine/complement proposés, à accepter |
| Reprise fidèle à la clôture | Pass | Ancien lot reste clos ; historique des violations conservé |

## 11. Implementation Agent Prompt

> Après accord écrit précis sur ce nouveau lot, utiliser ce plan `docs/cadrage/plan-levee-reserves-inspect-ciblee.md`, phase A, tâches T1–T3, dans `C:/wt/inspect-ciblee` sur la branche acceptée. Mode IMPLEMENT ; propriétaire initial unique du fichier de tests `crates/whisper-adapters/src/worker_ipc/tests/queue_inspection_characterization.rs`. Tu n'es pas seul : préserver tout changement tiers et les documents de la principale ; ne pas stage/commit. Aucun changement produit ou de garde n'est autorisé par défaut. Corriger la séparation des preuves de comptage et d'intégrité selon T2, sans qualifier un runtime factice. Exécuter les commandes de la matrice uniquement via les gardes et en respectant l'ordre, une par une, avec examen de chaque sortie. Premier échec = STOP et rapport, aucun retry/correctif autonome après échec ; zéro test = NON EXÉCUTÉ. Aucun natif, déplacement, suppression manuelle, réseau ou assouplissement des seuils. Rendre diff exact, statut de chaque exigence, commandes/codes/comptes complets, santé et limites. T4–T6 s'exécutent ensuite selon les prérequis ; les trois N1 servent aussi à P6. La principale confiera le candidat à rust_review_coordinator ; reviewer seul ferme R1/R4 et juge CLEAN, au plus deux cycles pour ce nouveau livrable. La présente rédaction n'est pas cette autorisation.
