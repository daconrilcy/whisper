Avancée : syntaxe et formatage corrigés ; contrôle fmt PASS sur le candidat commité, compilation tentée.
Blocage : compilation FAIL E0277 sur Generation: Hash ; aucun test exécuté.
Décision : campagne de nouveau arrêtée au premier échec ; correction de compilation à arbitrer.

# Plan d'implémentation — transcription Parakeet unifiée

## 1. Identité, autorité et base

- ID : P-PARA-UNIFIED-01 ; version : v0.1 ; statut : PLANS READY pour acceptation utilisateur.
- Design parent : `docs/cadrage/evo-parakeet-transcription-unifiee.md`, D-PARA-UNIFIED-01 v0.2, accepté pour planification au commit `d8d170f9`.
- Revue du design : `docs/cadrage/revue-parakeet-transcription-unifiee.md`, CLEAN sur `42c8c16d`, clôture documentaire `03506050` ; acceptation S-USER-07 ajoutée sans changement technique.
- Base produit : `a483e176759fcd2f2340f221470fe09f20ec8545`, branche `evo-startup-perf`.
- Accord utilisateur de cette mission : « ok tu peux maintenant rediger le plan d'implementation à partir de ces données de cadrage. »
- Autorisé : planification documentaire par l'agent dédié, revue indépendante, rédaction par la session principale et commits locaux dans `docs/cadrage/`.
- Non autorisé : code produit, compilation, tests, exécution audio/GPU/replay, nouvelle campagne, installation, élévation, écriture hors dépôt ou push. La validation du plan ne sera pas assimilée à une autorisation d'implémenter un lot.
- Livrables de cette mission : ce plan et `revue-plan-parakeet-transcription-unifiee.md`. Pas de nouveau railguard, outil de persistance des agents, manifeste d'empreintes ou campagne documentaire.
- Budget documentaire : dix tâches d'implémentation maximum au total, regroupées en lots ; deux cycles revue/correction maximum. Toute extension structurelle revient à l'architecte et aux autorités du design.
- État initial vérifié : fichiers suivis propres, `models/` et `sound example/` non suivis préservés ; 114,57 Gio libres sur C: ; aucune commande Cargo exécutée.
- Sources : AGENTS courant, design parent, revue, règles et contrats du pack central lus à cette reprise. Les anciennes dérogations de campagnes ne sont pas transférées à ce plan.

## 2. Contribution du rédacteur et carte du plan

Contribution `PLAN-PU v1`, auteur `/root/parakeet_plan_writer`, rôle `rust_plan_writer`. Base design `d8d170f993c5afb1537d58a5e22c253fde6a7489`, base produit `a483e176`. Sources lues par le rédacteur : design/revue, AGENTS, règles et références, modules et tests listés, déclarations de modules et Cargo.toml des trois crates, garde micro-level existante. Auteur des fichiers : session principale ; aucun fichier modifié ni commande Cargo/test/binaire exécuté par le spécialiste. Terminé : ordre, chemins, dépendances, carte d'exécutabilité et prérequis. Restant : revue PLANS, acceptation du plan et autorisations de réalisation/campagnes.

Le plan devra couvrir : lots et tâches, dépendances DOCUMENT/CODE/EXECUTION, chemins directs et indirects, scénarios et données, critères et mesures, préflight et commandes futures sous garde, détails à résoudre avant les lots dépendants, récupération proportionnée et limites NOT RUN.

## 3. Lots, tâches et vérification

### 3.1 Résultat et contrats conservés

Réaliser le parcours Parakeet accepté : texte unique révisable, puis résultat final automatique ; import et micro sans plafond applicatif de durée ; WAV classiques numérotés automatiquement pour les captures longues ; TXT/SRT uniques à temps globaux.

Les décisions DEC-T-PU-01 à 06 sont les références techniques. Le plan ne change ni moteur/modèle, ni stack, ni règles CPU/GPU/Auto, ni source/gain microphone. Il ne crée aucune reprise après panne, bibliothèque durable ou fonction d'installation nouvelle.

L'aperçu n'est jamais le filtre d'admission du résultat final. Une partie finale défectueuse ou une sortie invalide empêche l'annonce d'un paquet complet. La politique d'équité reste exactement DEC-T-PU-05 v2 : quota P=1 et tours finaux alternés, borne conditionnelle de quatre appels pour deux sessions éligibles. Aucun délai utilisateur n'en est déduit.

### 3.2 Index, lots et dépendances

Dix tâches **au total**, toutes séquentielles : les fichiers application, ports, archive et worker_ipc partagés interdisent une parallélisation implicite.

| Lot | Tâches | Démonstration de sortie | Dépendances |
| --- | --- | --- | --- |
| PU-L1 — Import unifié minimal | T1–T4 | WAV/MP3 → source bornée → fenêtres → résultat publié, par le vrai parcours applicatif avec worker contrôlé | DOCUMENT : design accepté, plan accepté ; EXECUTION : E-GUARD et autorisation avant tests |
| PU-L2 — Micro et partage moteur | T5–T6 | Capture indépendante, aperçu révisable et finalisation commune ; équité A/B contrôlée | CODE : sorties T1–T4 ; EXECUTION : validations L1 |
| PU-L3 — Cycle de vie et fenêtre | T7–T8 | Parcours utilisateur unique, A/B, annulation et fermeture cohérentes | CODE : sorties T5–T6 ; EXECUTION : validations L2 ; détails Q-RQ-01 avant T8 |
| PU-L4 — Validation et remise | T9–T10 | Régressions, preuves natives et séance longue, puis recette Windows utilisateur | CODE : sorties T1–T8 ; EXECUTION : validations L3, corpus/runtimes identifiés, Q-PU-03 pour verdicts de délais |

Un prérequis **DOCUMENT** ne remplace jamais une API **CODE**, et la compilation d'un lot ne remplace pas sa preuve **EXECUTION**. Les sorties des lots précédents sont vérifiées dans Git courant ; leurs changements attendus ne sont pas une divergence de baseline.

### 3.3 Chemins de réalisation

Les chemins sont relatifs au dépôt ; `+` désigne un nouveau fichier proposé. Les fichiers nouveaux doivent être déclarés par les `lib.rs` explicitement listés. Aucun Cargo.toml/Cargo.lock ou worker natif n'est prévu en écriture.

| Groupe | Chemins autorisés en écriture lorsqu'une tâche les référence |
| --- | --- |
| G1 — Domaine et application | `crates/whisper-core/src/{domain.rs,ports.rs,application.rs,parakeet_windows.rs,model_registry.rs,engine_capabilities.rs,lib.rs}` ; `+crates/whisper-core/src/parakeet_session.rs` ; `+crates/whisper-core/src/parakeet_scheduler.rs` |
| G2 — Audio et décodage | `+crates/whisper-adapters/src/parakeet_audio.rs` ; `crates/whisper-adapters/src/{decoder.rs,direct_live.rs,lib.rs}` |
| G3 — Document et publication | `+crates/whisper-adapters/src/parakeet_document.rs` ; `crates/whisper-adapters/src/archive.rs` |
| G4 — Pipeline et moteur | `+crates/whisper-adapters/src/parakeet_pipeline.rs` ; `crates/whisper-adapters/src/{worker_ipc.rs,parakeet_import.rs,parakeet_live.rs,lib.rs}` |
| G5 — Présentation et composition | `crates/whisper-desktop/src/{ui.rs,root.rs,lib.rs}` |
| GT — Tests | `+crates/whisper-core/tests/parakeet_unified.rs` ; `crates/whisper-core/tests/parakeet_windows.rs` ; `+crates/whisper-adapters/tests/parakeet_unified.rs` ; `+crates/whisper-adapters/tests/parakeet_unified_native.rs` ; `crates/whisper-adapters/tests/{parakeet_import.rs,parakeet_archive.rs,parakeet_control.rs,reprocessing_baseline.rs,live_capture.rs,live_archive.rs,microphone_level.rs}` ; `+crates/whisper-desktop/tests/parakeet_unified_ui.rs` |
| GD — Livrables | Les deux documents plan/revue-plan annoncés ; mises à jour d'avancement et résultats dans le plan, sans rapports supplémentaires obligatoires |

`direct_live.rs` n'est ouvert que pour réutilisation/extraction minimale des primitives WAV et conversion nécessaires à G2 ; le contrat Turbo doit rester inchangé. Les tests privés peuvent rester dans les modules sources déjà autorisés ; les tests d'intégration exercent les ports/API, sans rendre publique une infrastructure uniquement pour les tester.

Lectures indirectes communes : AGENTS, design/revue/plan, Cargo.toml/Cargo.lock, `source_binding.rs`, `capture.rs`, `vad.rs`, `parakeet_ipc.rs`, paramètres existants. Sources, modèles et runtimes sont identifiés avant chaque campagne et restent sans écriture volontaire. Un chemin produit supplémentaire nécessaire entraîne PLAN_CHANGE et revue ciblée avant modification.

### 3.4 Dix tâches

**T1 — Installer les contrats de session et le curseur de fenêtres.** Owner futur : `rust_implementer`. REQ/AC-01/02/04/05/07/10 ; DEC-T-01/03. Écriture : G1 et tests core GT. Implémenter identité/génération, bornes reçu/admis/écrit/traité, révision/final et publication distinctes, indices globaux `u64` vérifiés. Le curseur produit une fenêtre à la fois, y compris coupe à 120 s sans silence, contexte et attribution par milieu. Séparer limite de requête native et absence de plafond de session, sans retirer la limite du worker. **Sortie :** contrats compilables et tests purs des transitions, frontières et fin de source.

**T2 — Fournir la source audio segmentée et le décodage continu.** CODE : T1. REQ/AC-03/04/06/08 ; DEC-T-02. Écriture : G2, `ports.rs`, tests adapters GT. Rotation à 1 Gio PCM sans perte/duplication ; correspondance locale/globale, création exclusive, index parcouru par pages ; conserver le nom court et définir le suffixe numéroté avant la première écriture. Décodage WAV/MP3 séquentiel, conversion avec état et drainage final, blocs ≤80 000 échantillons. Hash/inspection streaming, source originale intacte. **Sortie :** lecture de plages traversant une rotation et incidents d'écriture vérifiés ; aucun chargement audio intégral.

**T3 — Fournir le document paginé et la publication cohérente.** CODE : T1/T2. REQ/AC-04/05/06/09 ; DEC-T-06. Écriture : G3, `ports.rs`, tests adapters GT. Persister révisions, couverture et résultats sans vecteurs cumulatifs ; lire une page bornée ; préparer TXT/SRT streaming puis engager la référence réelle. Temps invalides : texte utilisable conservé, paquet incomplet. Inclure la stratégie dans l'identité/cache ; préserver anciennes publications et remplacements refusés. **Sortie :** cohérence écran-document-fichiers démontrable via ports, erreurs d'export et d'engagement contrôlées.

**T4 — Raccorder l'import unifié de bout en bout.** CODE : T1–T3. REQ/AC-02/03/04/05/09 ; DEC-T-01/03. Écriture : G4 sauf `parakeet_live.rs`, `application.rs`, `root.rs`, tests adapters GT. Supprimer les refus de durée de session uniquement pour la nouvelle stratégie ; conserver identité/validation du candidat et bornes natives. Traiter les fenêtres séquentiellement, faire avancer la couverture contiguë et publier. Ne pas accepter une ancienne qualification single-pass comme qualification de la nouvelle stratégie. **Sortie :** import court/long contrôlé dans le vrai parcours applicatif, dernière fenêtre et phrase terminale incluses. L'activation native finale dépend de T9.

**T5 — Installer l'ordonnanceur équitable borné.** CODE : T4. REQ/AC-02/04/07/10 ; DEC-T-05 v2. Écriture : `parakeet_scheduler.rs`, `application.rs`, `ports.rs`, G4, tests core/adapters GT. Une inférence par backend, deux sessions non terminales, aperçu remplaçable capacité un, P=1, tour de rôle final incluant préparation du clip ; tables de tentatives bornées. **Sortie :** V-PU-EQUITY-01 et son témoin négatif détectent l'ancienne famine ; pas de deuxième appel pendant l'appel engagé.

**T6 — Raccorder capture et finalisation micro au pipeline commun.** CODE : T2/T4/T5. REQ/AC-01/02/04/06/08 ; DEC-T-02/04. Écriture : `parakeet_live.rs`, `worker_ipc.rs`, `parakeet_pipeline.rs`, `application.rs`, tests capture/adapters GT. Callback non bloquant, writer indépendant, frontière réellement écrite et pertes explicites. L'aperçu de 4 s remplace la zone courante ; le traitement final peut avancer pendant capture et termine le reliquat après arrêt. Retirer le budget tronquant de 30 s de cette voie. **Sortie :** stop idempotent, phrase terminale, rotation et continuité des blocs contrôlés ; gain/source/Turbo préservés.

**T7 — Fermer les cycles A/B, annulation et erreurs normales.** CODE : T3/T5/T6. REQ/AC-05/06/07/09/10. Écriture : `application.rs`, `ports.rs`, G3/G4, tests control/adapters GT. Annulation demandée distincte de son issue ; génération périmée rejetée ; résultat après engagement réel ; fermeture bloque les admissions puis attend toutes les obligations A/B. Refus d'une troisième opération avant ouverture micro. **Sortie :** scénarios de fermeture, saturation, incident moteur/disque, remplacement et résultat tardif sans attribution croisée.

**T8 — Présenter une transcription unique et paginée.** CODE : T3/T6/T7. REQ/AC-01/03/05/07/08/10 ; DEC-PU-01. Écriture : G5, API de vue G1 si nécessaire, test UI GT. Retirer le choix confirmé/brouillon du nouveau parcours ; afficher pages/révisions puis référence finale, état réel, parties audio et sorties. Supprimer les lectures intégrales de texte sur cette voie. Conserver huit fiches maximum et paramètres existants. Q-RQ-01 doit être résolue avant le comportement d'alerte concerné : aucun arrêt automatique ni délai inventé. **Sortie :** tests du vrai composant et composition ; inspection interactive réservée à T10.

**T9 — Valider les scénarios complets, natifs et longs.** CODE : T1–T8 ; EXECUTION : E-GUARD/corpus/runtimes. REQ/AC-01 à 10. Écriture : GT ; corrections uniquement dans les chemins T1–T8 approuvés et via boucle de revue. Tester CPU/GPU/Auto et refus/replis, WAV/MP3, plusieurs durées >30 min, raccords FR/EN, mémoire/UI/index, erreurs, rotation et A/B réellement concurrentes. Un seuil de rotation réduit uniquement pour test ne qualifie pas la limite réelle : ajouter un passage streaming dépassant 1 Gio PCM. **Sortie :** résultats par sous-scénario et omissions explicites, aucune qualité déduite d'un simple succès Cargo.

**T10 — Construire la remise et réaliser la recette utilisateur.** EXECUTION : T9 et détails de remise arrêtés. REQ/AC-01 à 10 ; AGENTS remise Windows. Écriture produit : aucune nouvelle ; sorties dans la racine PU-L4. Construire le profil identifié, relever chemins effectifs et vérifier workers/runtimes/modèles lus ; remettre le lanceur requis pour un profil Test sans ouvrir la fenêtre. Appliquer exactement les règles AGENTS si profil personnel retenu. L'utilisateur teste import, micro, texte unique, fin, parties WAV, fichiers et A/B. **Sortie :** bilan réel de recette et réserves ; aucune validation Windows déduite des tests.

### 3.5 Carte d'exécutabilité

Toutes les lignes sont **NOT RUN**. Racine de campagne de chaque lot : `target/parakeet-tests/parakeet-unified/lot-01` à `lot-04`, seulement proposée. Elle contient fixtures, temporaires, cache local, sorties et petites preuves. Cargo écrit exclusivement dans le target partagé canonique.

| Vérification / tâches / AC | Données et observation attendue | Chemins lus / écrits et méthode |
| --- | --- | --- |
| V1 — T1/T5 ; AC-02/04/07/10 | Plages limites, vide/fin, génération périmée, parole sans silence ; V-PU-EQUITY-01 avec worker contrôlé et témoin négatif. | G1 + tests core ; fixtures/sorties lot ; commande C1. |
| V2 — T2/T6 ; AC-04/06/08 | PCM repéré traversant rotation, blocs drainés, gain non nul, création/écriture refusée ; comparer échantillons et bornes globales. | G2/G4 + tests adapters ; fixtures/WAV/index lot ; C2. |
| V3 — T3/T4/T7 ; AC-02/03/05/06/09 | WAV/MP3, dernier intervalle, temps invalide, TXT/SRT impossible, remplacement refusé/accepté ; vérifier référence et ancienne publication. | G1/G3/G4, `source_binding.rs` en lecture ; source/copies et publications lot ; C2. |
| V4 — T6/T7 ; AC-06/07/10 | A finalise pendant B, troisième admission refusée avant micro, fermeture/annulation et résultats obsolètes. | G1/G4 + control ; événements et sorties lot ; C2. |
| V5 — T8 ; AC-01/03/05/07/08/10 | Pages et révisions du même document, résultat final et incident ; absence de lecture complète ; états CPU/GPU/micro/gain/Turbo. | G5 + test UI, API application ; données lot ; C3. |
| V6 — T9 ; AC-02/03/04/05/08 | Corpus FR/EN court puis >30 min, pauses/continu, raccords et fin annotés humainement ; CPU/GPU/Auto ; lecture des vrais résultats et mesure des ressources. | GT, sources/modèles/runtimes identifiés en lecture ; travail/clips/publications/métriques lot-04 ; C4. |
| V7 — T9 ; AC-04/06 | Passage réel >1 Gio, index/pages nombreux, erreurs physiques simulées ; mesurer mémoire de travail séparément du stockage proportionnel. | G2/G3/G4/G5, fixtures et sorties lot-04 ; C2/C4 ; pas de preuve d'infini. |
| V8 — T10 ; AC-01 à 10 | Même corpus et captures utilisateur, chevauchement A/B observé, fichiers ouverts et contenu confronté. | EXE/worker/runtimes lus ; réglages/temp/sorties exclusivement routés dans lot-04 ; C5 puis recette manuelle utilisateur. |

Les références automatiques FEAS ne sont pas un gold lexical humain. Avant V6/V8, inventorier les annotations humaines réellement disponibles et les passages encore non validés. Un extrait manquant bloque uniquement son verdict de qualité ; aucun téléchargement ni nouvel enregistrement n'est autorisé par la présente rédaction.

### 3.6 Commandes futures et prérequis de campagne

**Ces commandes sont des contenus à soumettre à une enveloppe validée, jamais des commandes Cargo directes autorisées.** Cwd `C:\dev\whisper`, shell PowerShell, Windows existant. Le nom et les modes exacts de l'enveloppe restent à désigner avant exécution.

- **C0 :** `cargo fmt --all -- --check`.
- **C1 :** `cargo test --offline --locked -p whisper-core --test parakeet_unified --test parakeet_windows`.
- **C2 :** `cargo test --offline --locked -p whisper-adapters --test parakeet_unified`.
- **C3 :** `cargo test --offline --locked -p whisper-desktop --test parakeet_unified_ui`.
- **C4 :** `cargo test --offline --locked -p whisper-adapters --test parakeet_unified_native -- --nocapture`, avec entrées natives explicitement configurées et cas exécutés comptés.
- **C5 :** `cargo build --offline --locked -p whisper-desktop --release`, profil Test initial ; `personal-install` seulement si la remise retenue et son routage sont précisément autorisés.
- **Régression de clôture T9 :** tests workspace offline/locked via la même enveloppe ; distinguer les branches natives ignorées, absentes ou non exécutées.

Le nouveau test natif doit signaler l'absence de ses entrées comme prérequis manquant ; un test legacy qui retourne sans passage natif reste **NOT RUN**, pas PASS natif. Les tests historiques de profils single-pass conservent leur sens ; ne pas supprimer leurs refus pour faire passer la nouvelle stratégie.

**Préconditions obligatoires :**

- **E-AUTH — non acquis :** accord précis sur lot, chemins, commandes, racines et recette. L'acceptation du plan ne suffit pas.
- **E-GUARD — non satisfait :** garde compatible avec nouvelles racines, auto-tests de refus des chemins absents/externes/jonctions avant tout Cargo, surveillance pendant commande et arrêt selon AGENTS. La garde micro-level fixe une autre racine et comporte une exception UNATTRIBUTED ; elle n'est pas réutilisable telle quelle. Les dérogations micro-lot1/FEAS ne s'étendent pas à ce plan. La seule surveillance n'est pas un confinement préventif. Si conformité impossible : arrêt avant Cargo et arbitrage, sans chantier d'outillage ajouté implicitement.
- **E-ENV :** au moins 30 Gio libres ; `CARGO_HOME`, `TEMP`, `TMP` canoniques sous la racine du lot ; `CARGO_TARGET_DIR=target/parakeet-tests/cargo-shared/` ; cache utilisateur lu/copié vers le lot avant campagne sans écriture dans le profil ; `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. Toute variable applicative de réglages/source snapshot/archives/temp doit également rester routée dans les racines admises.
- **E-INPUTS :** ressources et profil effectifs identifiés ; modèle/runtime en lecture seule, corpus validé pour le critère revendiqué ; aucune installation/téléchargement.
- **E-REVIEW :** candidat Rust concret → `rust_review_coordinator`, reviewer indépendant et corrections par `rust_implementer`, deux cycles maximum.

Budgets **proposés, non autorisés** : L1 90 min/16 Gio ; L2 90 min/8 Gio ; L3 60 min/4 Gio ; L4 240 min/16 Gio hors temps de recette utilisateur. Ce sont des plafonds de campagne à préciser selon cache initial/corpus et à arrêter à épuisement, pas des durées promises de transcription. Le target partagé et son besoin disque sont vérifiés séparément ; aucun nettoyage automatique. Une prolongation exige arbitrage avant reprise.

### 3.7 Détails, récupération et clôture

- **Q-PU-03 :** mesures et seuils de premier texte/retard/attente après arrêt à fixer avant le verdict de délai T9/T10 ; intégrer les appels natifs non préemptibles et les tours équitables. Si le seuil impose un autre design, retour architecte puis revue, pas ajustement silencieux du plan.
- **Q-RQ-01 :** comportement et seuil éventuel d'alerte avant T8 ; pas d'arrêt automatique fondé sur l'âge.
- **Détails avant consommation :** nommage des parties et version d'index avant T2 ; tailles de pages/caches avant T3/T8 ; profil et routage réel de remise avant T10. Owners architecte/implémenteur selon le contrat accepté ; aucune modification de portée produit déduite de ces détails.

À tout échec : arrêter la commande/campagne, préserver sources, audio déjà écrit, sorties valides et référence précédente ; consigner cause et chemins réellement concernés. Pas de relance automatique, reset global, effacement de résultats ni restauration sans état antérieur vérifié. Une écriture hors racine entraîne le traitement AGENTS et arbitrage avant reprise.

À la clôture de chaque lot : bilan dans le plan, petites preuves archivées sous sa racine avec arborescence, nombre et SHA-256 vérifiés conformément à AGENTS ; suppression seulement des temporaires expressément approuvés, jamais du target partagé ou d'entrées encore utilisées. Relever l'espace libre.

**Couverture inverse :** CAP/REQ/AC-01→T6/T8/T10 ; 02→T1/T4/T5/T6/T9 ; 03→T2/T4/T8/T9 ; 04→T1/T2/T4/T6/T9 ; 05→T3/T4/T8/T9 ; 06→T2/T3/T6/T7/T9 ; 07→T5/T7/T8/T9 ; 08→T6/T8/T9/T10 ; 09→T3/T4/T7/T9 ; 10→T5/T7/T8/T10. Chaque groupe de chemins est consommé par au moins une tâche et un scénario ci-dessus.

La future revue PLANS vérifie ce document contre le design accepté. Les états de rédaction, CLEAN documentaire, acceptation du plan, autorisation de coder et validation Windows restent distincts.

## 4. Readiness

**PLANS READY pour acceptation utilisateur**, sur verdict **CLEAN** de `/root/parakeet_design_review`, indépendant de la rédaction, candidat Git `325471621ee182511cb3ff522294d73975967e4d`. Aucun constat REQUIRED, aucune correction requise. Voir `revue-plan-parakeet-transcription-unifiee.md`.

Le conseil PLAN-PU-A1 est conservé pour précision avant T5 : il ne bloque pas le plan et ne change aucun chemin ni contrat dans cette clôture. E-GUARD et E-AUTH ne sont pas satisfaits ; PLANS READY ne signifie pas campagne ou code autorisé. Les seuils et autres détails gardent leurs échéances du §3.7. Tous les tests produit, passages natifs et essais Windows restent NOT RUN.

La clôture ne modifie que les métadonnées de readiness et consigne le verdict, conformément à la permission explicite du reviewer. Aucun changement technique après revue. Acceptation du plan, autorisation de lot, revue de code, validation produit et publication restent distinctes.

## 5. Réalisation PU-L1 — 26 octobre 2026

Cette section décrit la réalisation postérieure à la clôture documentaire ci-dessus ; elle ne modifie pas rétroactivement l'autorité de la mission de planification.

**Autorisation de session.** Après présentation du périmètre PU-L1, T1–T4, chemins du plan, délégation, revue et commits locaux, l'utilisateur confirme : « oui je te confirme l'acceptation du plan et l'autorisation de coder ». E-AUTH est acquis pour ce code et ces commits locaux uniquement. Les campagnes restent suspendues à E-GUARD ; aucun lot suivant, push, téléchargement, installation ou écriture hors dépôt n'est autorisé par cette confirmation.

**Base et préflight.** Branche `evo-startup-perf`, HEAD initial `1b245596f897cbdbc03de8554dd53f7762bcb650`. Aucun changement suivi préexistant ; `models/` et `sound example/` non suivis préservés. Espace libre avant réalisation : 113,65 Gio. Aucune commande Cargo, rustc, rustfmt, test, audio ou GPU exécutée.

**Candidat.** Auteur du code : `/root/pu_l1_implementer`, rôle `rust_implementer`, mode IMPLEMENT puis CORRECT. Huit chemins sources : `crates/whisper-core/src/{parakeet_session.rs,ports.rs,lib.rs}` et `crates/whisper-adapters/src/{parakeet_audio.rs,parakeet_document.rs,parakeet_pipeline.rs,lib.rs,archive.rs}`. Aucun fichier Cargo ou worker natif modifié.

| Tâche | État réel du candidat |
| --- | --- |
| T1 | Contrats de session, frontières, révisions et curseur de fenêtres écrits ; tests unitaires écrits, NOT RUN. |
| T2 | Décodage séquentiel, conversion avec drainage final, source WAV segmentée et index paginé écrits ; tests de conversion, rotation à seuil réduit et erreurs écrits, NOT RUN. La rotation réelle >1 Gio reste T9. |
| T3 | Port/store paginé et couverture écrits ; génération TXT/SRT et copie des exports de l'archive converties en flux. Cohérence bout en bout non démontrée, faute de raccord et d'exécution. |
| T4 | Pipeline avec worker contrôlé et test écrits, NOT RUN. Aucun raccord au vrai `ImportApplication`/`worker_ipc`/composition ; aucune publication unifiée depuis ce parcours. T4 n'est pas terminée. |

**Revue indépendante.** Coordination `/root/pu_l1_review` (`rust_review_coordinator`), reviewer `/root/pu_l1_review/pu_l1_reviewer` (`rust_reviewer`), même reviewer sur deux passes et une correction. Le worker a été repris après une interruption par limite de réponse, sans ajouter de cycle de correction. Résultat final : **BLOCKED**, aucun CLEAN du code.

- **R1 — BLOQUANT, ouvert, T4 / AC-PU-03/05 :** vrai import non raccordé et identité de stratégie absente des demandes persistées. L'ancien parcours et ses refus de durée restent en place. Ne pas sélectionner la stratégie d'après la durée ni réutiliser une qualification legacy pour la stratégie unifiée. L'activation native reste dépendante de T9.
- **R2 — clos après correction statique :** drainage de conversion à EOF, longueur explicite et tests de fin/frontière de paquet.
- **R3 — clos après correction statique :** tolérance terminale bornée à 80 ms uniquement pour le dernier mot de la fenêtre finale, avec cas positifs et négatifs écrits.
- **R4 — clos après correction statique :** tests de continuité autour d'une rotation à seuil réduit et de conservation lors d'erreurs.

`git diff --check` réussi. Compilation, formatage, tests et exécution native **NOT RUN** : les tests écrits ne sont pas des preuves exécutées et la revue statique ne prouve pas la compilabilité ni la qualité fonctionnelle. Aucun essai dans la fenêtre Windows.

**Arbitrage de périmètre proposé, non accepté.** Étendre T4 à `crates/whisper-adapters/src/queue_store.rs` et `crates/whisper-adapters/src/source_binding.rs`, actuellement hors chemins d'écriture, afin de porter une identité explicite et versionnée de stratégie dans les demandes persistées. Préserver le sens des demandes anciennes et distinguer les nouvelles demandes unifiées sans emprunter les qualifications single-pass/VAD existantes. La représentation et la compatibilité doivent être précisées dans le PLAN_CHANGE et revues de façon ciblée avant ces modifications. Les chemins de tests supplémentaires éventuellement nécessaires devront être explicités dans cet ajustement ; aucune extension implicite n'est autorisée. Ce changement de périmètre n'est pas adopté par le présent bilan.

E-GUARD demeure une décision d'exécution distincte : aucune garde n'a été adaptée, aucune ancienne dérogation transférée et aucune campagne lancée. Aucun temporaire de campagne créé ou nettoyé par cette réalisation. Espace libre relevé à la clôture de ce candidat partiel : 113,65 Gio.

## 6. CHANGE-PU-L1-01 — identité persistée et raccord T4

**Autorité et état.** Le 26 octobre 2026, après présentation de R1 et des deux chemins de persistance manquants, l'utilisateur répond : « je t'autorise l'ajustement du plan puis terminer T4 ». Cette autorisation lève l'arbitrage de périmètre proposé au §5 ; elle couvre l'ajustement puis le code T4 et ses adaptations nécessaires, sans campagne, activation native unifiée, lot suivant ni push. Version du plan avec cet ajustement : v0.2 ; le §1 et le §4 conservent l'historique de la version v0.1.

Contribution technique `ARCH-PU-L1-01 v1`, auteur `/root/pu_l1_plan_change`, rôle `rust_architect`, lecture seule sur la base `29b924303afb89608d8841e676187bcb8505f709`. Sources : plan §3/§5, design DEC-T-PU-01/06 et PORT-PU-01 à 05 ; `ImportRequest`, dispatch `ImportApplication`, `QueueStore`, `SourceBinding`, `ArchiveStore`, `WorkerSupervisor` et composition desktop. Aucun fichier écrit ni commande Rust/campagne exécutée par l'architecte. Auteur du présent ajustement : session principale. Statut : revue ciblée CLEAN, ajustement prêt pour le code T4 autorisé.

**Nature et impact.** PLAN_CHANGE à la suite de la découverte technique R1 ; aucun changement de besoin ou de critère d'acceptation. Étend T4 et précise l'identité/cache déjà exigée par DEC-T-PU-06 ; ne crée aucune onzième tâche ni nouveau rapport. Objectif inchangé : vraie application → import unifié à worker contrôlé → publication cohérente, avec identité distincte des stratégies historiques. L'activation native unifiée reste T9. Les contrats ci-dessous complètent T4 et priment sur l'ancien inventaire de chemins pour les usages expressément listés.

### 6.1 Contrats de stratégie et compatibilité

- Ajouter à `ImportRequest` un champ typé `strategy`, en étendant `TranscriptionStrategy` avec `LegacyV1` et en conservant `BoundedWindowsV1`. La version est portée par la variante ; `stable_id()` distingue `legacy-v1` et `bounded-windows-v1`. Un champ absent dans un ancien enregistrement signifie exclusivement `LegacyV1`. Toute valeur inconnue est une erreur explicite, sans conversion silencieuse. Le même identifiant traverse préparation, file, binding, exécution et publication ; aucune sélection par durée.
- Lire les anciennes entrées sans modifier leur sens ou leurs fichiers lors de la lecture. Les nouvelles entrées unifiées sont écrites avec un schéma identifiable et refusé par l'ancien lecteur : queue v2 pour les entrées unifiées, lecture des v1 legacy conservée ; snapshot v2 pour les bindings unifiés, lecture v1 legacy conservée ; pending d'archive v4 pour la stratégie unifiée, versions 1–3 historiques inchangées. Vérifier les couples schéma/stratégie : un ancien schéma ne peut porter une stratégie unifiée ; un nouveau schéma unifié ne peut omettre la stratégie. Pas de migration destructive ni réécriture globale de la file. Une demande en attente ou reprise conserve sa stratégie ; toute substitution est refusée.
- `SourceBinding::apply_to_request` conserve la stratégie ; l'ouverture d'un binding existant vérifie sa concordance avec la demande. L'identité persistée d'archive et toute comparaison de réutilisation/cache incluent la stratégie en plus de la source, du modèle et des identités de calcul/moteur déjà exigées. Une archive legacy ne satisfait jamais une demande unifiée. La stratégie applicative ne remplace pas `ArchiveQualificationKey` : les qualificatifs single-pass/VAD existants gardent leur sens et ne qualifient pas la session unifiée. Les anciens résultats restent consultables selon leurs règles.

### 6.2 Raccord applicatif et limite d'activation

`ImportApplication` transmet la stratégie sans la recalculer lors des événements ou des reprises. La composition assemble le vrai adaptateur d'import ; celui-ci choisit le pipeline borné uniquement sur `BoundedWindowsV1`, décode séquentiellement la source liée, traite les fenêtres avec un worker contrôlé injecté, persiste document/couverture et publie via le mécanisme réel d'archive. La branche unifiée ne conserve aucun plafond applicatif de durée ; chaque clip garde les bornes natives. La publication n'est annoncée qu'après couverture finale et engagement réel des sorties.

La composition native et les nouvelles demandes de l'UI restent explicitement legacy jusqu'à T9. Les requêtes unifiées servent ici la composition contrôlée du vrai parcours applicatif. Un appel unifié dans une composition native non qualifiée est refusé explicitement avant inférence ; aucune ancienne qualification ne l'autorise. Les points d'injection de worker contrôlé sont privés aux tests ou des ports effectivement consommés, sans commutateur d'environnement permettant de contourner la qualification native.

Aucun changement de parcours UI, micro, ordonnancement, annulation A/B ou qualification native prévu dans T5–T10. Les adaptations des constructions de demandes dans `ui.rs` et `parakeet_live.rs` sont uniquement mécaniques et utilisent `LegacyV1`, sans changement de comportement.

### 6.3 Chemins additionnels de T4

| Usage autorisé pour terminer T4 | Chemins additionnels exacts |
| --- | --- |
| Persistance et propagation | `crates/whisper-adapters/src/queue_store.rs` ; `crates/whisper-adapters/src/source_binding.rs` |
| Constructions mécaniques seulement, sans comportement T6/T8 | `crates/whisper-adapters/src/parakeet_live.rs` ; `crates/whisper-desktop/src/ui.rs` |
| Constructions et tests core existants | `crates/whisper-core/tests/import_contract.rs` ; `crates/whisper-core/tests/model_selection_app.rs` ; `crates/whisper-core/tests/parakeet_import_contract.rs` ; `crates/whisper-core/tests/scheduler_contract.rs` |
| Constructions et tests adapters existants | `crates/whisper-adapters/tests/durability.rs` ; `crates/whisper-adapters/tests/legacy_model_requests.rs` ; `crates/whisper-adapters/tests/import_mp3.rs` ; `crates/whisper-adapters/tests/import_cpu.rs` ; `crates/whisper-adapters/tests/model_catalog.rs` ; `crates/whisper-adapters/tests/reprocessing_cleanup.rs` ; `crates/whisper-adapters/tests/parakeet_long.rs` ; `crates/whisper-adapters/tests/source_binding.rs` ; `crates/whisper-adapters/tests/reprocessing_create.rs` |

Les autres constructions relevées sont déjà dans les chemins PU-L1/GT : application, ports, archive, worker_ipc, root, parakeet_import/control/archive et reprocessing_baseline. Les nouveaux tests `parakeet_unified.rs` restent autorisés. Aucun Cargo manifest, worker natif ou nouveau fichier de production supplémentaire nécessaire. L'inventaire ne donne pas d'autorité de modifier des comportements étrangers à T4 dans ces fichiers.

### 6.4 Responsabilités et vérifications ciblées

| Propriétaire | Dépendance/contrat |
| --- | --- |
| Core ports/session | Possède stratégie typée et demande sérialisable ; aucun IO |
| Application | Préserve stratégie et génération ; orchestre effets/états existants |
| Queue/source-binding/archive | Implémente persistance compatible et contrôle de concordance |
| Adaptateur pipeline/worker_ipc | Sélection explicite, fenêtres bornées, effets réels et publication |
| Root | Injecte implémentations et statut d'activation ; aucune politique métier parallèle |
| UI/micro | Adaptation de construction legacy seulement |

Vérifications à écrire dans les chemins de tests ci-dessus ou GT, liées à T4 / AC-PU-03/05 et DEC-T-PU-06, toutes **NOT RUN** jusqu'à E-GUARD :

- Ancien JSON sans stratégie → legacy ; round-trip unifié ; valeur inconnue et couples schéma/stratégie incohérents refusés.
- File legacy inchangée à la lecture ; queue unifiée conserve sa stratégie après réouverture et changement CPU/Auto/GPU permis ; substitution lors d'une reprise refusée.
- Binding conserve la stratégie et refuse une discordance, sans toucher la source.
- Même source/modèle : résultat legacy non réutilisé pour stratégie unifiée.
- Import court et long via **`ImportApplication` et le vrai adaptateur raccordé**, worker contrôlé seul substitué : dernière fenêtre/phrase terminale, couverture contiguë, TXT/SRT publiés, référence réelle constatée.
- Échec worker, temps invalides ou export impossible : aucune annonce de paquet complet ; ancienne publication préservée.
- Composition native : requête unifiée explicitement refusée avant appel moteur ; imports legacy/Turbo inchangés.

Les futures commandes C1/C2 restent soumises à la même enveloppe ; les tests ciblés de persistance/binding/import seront inclus dans C2 ou dans la régression workspace prévue. Aucune commande nouvelle autorisée par cet ajustement. Récupération : préserver anciennes entrées et publications ; aucun nettoyage, migration destructive ou relance de campagne.

**Restant.** Revue ciblée de cet ajustement versionné ; implémentation T4 par le worker original ; fermeture de R1 par le reviewer indépendant. La revue du plan ne ferme pas R1 et ne vaut pas compilation. La limite de deux cycles revue/correction du code PU-L1 est conservée, avec une correction déjà accomplie. E-GUARD et validations natives/utilisateur restent distincts.

**Résultat de la revue ciblée.** `/root/pu_l1_change_review`, rôle `rust_design_reviewer`, indépendant de la rédaction et de la contribution architecte, a examiné le §6 au Git `c0b9ebc5556d268599c0ebf4f613225dd54d039c` et les sources applicables. Verdict **CLEAN**, aucun REQUIRED ni ADVISORY, aucune correction. Le reviewer confirme les constructions couvertes, les schémas existants queue v1/snapshot v1/pending archive v1–3, le refus des versions inconnues, les liens design/T4/tests et l'absence de comportement T5–T10 ajouté. Aucun fichier modifié ni commande Rust exécutée par le reviewer ; plan inchangé avant/après. Il autorise la présente consignation et l'actualisation des métadonnées sans changement technique. La revue documentaire est terminée ; restent implémentation et relecture du code R1, puis preuves d'exécution sous E-GUARD.

## 7. Résultat après reprise de T4 — 26 octobre 2026

**Base et réalisation.** Après revue de CHANGE-PU-L1-01, métadonnées versionnées au Git `694fae08e609ebda7b33b885df2cfb51e3af8767`, le même `/root/pu_l1_implementer` a repris T4 en mode IMPLEMENT. Cette reprise traite le constat R1 et compte comme la seconde correction du même livrable PU-L1, sans remise à zéro du budget. Espace libre au préflight : environ 113,4 Gio ; à la clôture : 113,38 Gio. Les fichiers non suivis `models/` et `sound example/` sont préservés.

**Changements écrits, non qualifiés.** La stratégie `LegacyV1`/`BoundedWindowsV1` traverse les demandes, l'application, la file, le binding et l'archive. Les formats queue v2, snapshot v2 et pending archive v4 sont introduits pour les demandes unifiées, avec compatibilité des formats historiques et contrôles de concordance. Une branche contrôlée raccorde `ImportApplication` à `ImportRuntime`, au pipeline et à `ArchiveStore`. La composition native et les constructions UI/micro restent legacy, avec un refus unifié natif avant préparation lorsqu'aucun worker contrôlé n'est injecté. Ces descriptions du candidat ne constituent pas une fermeture de R1 ni une validation fonctionnelle.

Tests écrits : absence/inconnue de stratégie, versions persistées et incohérences, reprise/binding/archive, parcours court et 121 s via vraie application et archive avec worker contrôlé, exports TXT/SRT et erreurs worker/temps/export. Les scénarios d'archive contrôlés requièrent `models/parakeet-tdt-0.6b-v3.q8_0.gguf` conforme à la taille/SHA du registre ; le prérequis manquant doit provoquer un échec explicite. Aucune exécution de ces tests. Les 28 chemins sources/tests modifiés appartiennent à l'union du périmètre initial et du §6.3 ; les adaptations UI/micro et les constructions historiques restent mécaniques. Aucun Cargo manifest, dépendance, worker natif ou fichier hors dépôt modifié.

**Résultat de la relecture du code.** Même coordination `/root/pu_l1_review` et même reviewer indépendant `/root/pu_l1_review/pu_l1_reviewer`. Candidat : arbre de travail après CHANGE-PU-L1-01, baseline logique `1b245596f897cbdbc03de8554dd53f7762bcb650`, epoch 2. Bilan cumulé PU-L1 : trois passes de revue, deux corrections, zéro passe de vérification exécutable. Une précision de protocole sur le packet final n'ajoute aucun cycle technique. Verdict final du coordinateur : **LIVRÉ AVEC RÉSERVES**. Le code n'est pas CLEAN et T4 n'est pas déclarée terminée.

| Constat ouvert | Preuve du reviewer sur le candidat | Correction et condition de fermeture |
| --- | --- | --- |
| R1 — T4 / §6.2 | `worker_ipc.rs:4383` : l'import unifié GPU dépend encore du profil historique `import-single-pass` avant le pipeline contrôlé. | Séparer réellement la stratégie de session de la qualification historique, préserver les bornes natives et la fermeture d'activation T9 ; vérifier le parcours contrôlé CPU/GPU par l'application sans emprunter une qualification de session legacy. |
| R5 — compilabilité du module concerné | `worker_ipc.rs:13854` : ligne `strategy:` errante dans `impl NextGeneration for ImportRequest`. | Corriger l'emplacement du champ dans la construction de demande ; faire confirmer le diff par le reviewer. La compilation reste à exécuter sous enveloppe conforme. |
| R6 — T3 / REQ-PU-04 | `worker_ipc.rs:2378` : tous les segments paginés sont recopiés dans un `VecDeque` cumulatif non borné. | Conserver une consommation bornée des pages jusqu'à l'archivage/publication, sans reconstituer l'ensemble des segments ; test ciblé de la borne et de l'intégrité des exports. |

R2, R3 et R4 restent clos par le reviewer. Aucune correction R1/R5/R6 n'a été lancée après ce résultat, la limite de deux cycles étant atteinte. Aucun constat n'est fermé par l'implémenteur ou par la session principale.

**Vérifications et décision restante.** `git diff --check` réussi sur le candidat. Cargo, rustc, rustfmt, compilation, tests, audio/GPU et recette Windows **NOT RUN**, E-GUARD toujours non satisfait. Une erreur de syntaxe observée suffit à exclure toute affirmation de compilabilité, même sans commande exécutée. Aucun binaire de remise construit, aucune activation native unifiée ni push. Le candidat reste un travail en cours à conserver dans Git ; l'utilisateur doit arbitrer une correction supplémentaire ciblée des trois constats ou le maintien de ces réserves. Une autorisation de correction supplémentaire ne résoudrait pas E-GUARD et n'autoriserait pas une campagne implicitement.

## 8. Reprise ciblée autorisée — R1, R5 et R6

**Source d'autorité, 26 octobre 2026.** Après le résultat du §7, l'utilisateur répond : « je t'autorise une reprise ciblée pour corriger R1, R5 et R6, suivie de 2 revues indépendantes ». Cette décision autorise une correction supplémentaire bornée de ces trois constats après les deux cycles historiques ; elle ne réinitialise pas leur historique et ne donne aucune autorisation générale de poursuivre les autres lots. Base de reprise : `1d8309d9984b2c284b982db196f21954aa901900`, branche `evo-startup-perf`, suivi propre, `models/` et `sound example/` préservés. Espace libre avant reprise : 113,21 Gio.

La correction revient au même `rust_implementer`, sur les chemins déjà autorisés par T1–T4 et CHANGE-PU-L1-01. Les conditions de fermeture sont celles du tableau §7 : découpler le GPU unifié contrôlé du profil de session legacy tout en maintenant l'interdiction native avant T9 ; corriger la construction de demande mal placée ; consommer les pages de segments avec une borne de mémoire jusqu'à la publication, sans copie cumulative. Les tests utiles à ces corrections peuvent être écrits dans les chemins déjà autorisés, sans ajouter de tâche ni rapport.

Le candidat sera figé puis lu par deux `rust_reviewer` indépendants de l'implémentation : le reviewer historique et un second reviewer qui ne participe ni au code ni à sa correction. Chacun rendra son propre packet avant toute synthèse ; les deux examineront R1/R5/R6 et les régressions du delta corrigé. La coordination reste sans écriture du code. Cette double revue explicitement demandée remplace, pour cette reprise uniquement, la revue unique du parcours habituel. Aucune correction postérieure à ces deux avis n'est présumée autorisée si elle exige de dépasser le mandat ciblé ou un nouvel arbitrage après épuisement de cette reprise.

E-GUARD est inchangé : aucun Cargo, rustc, rustfmt, compilation, test, binaire, campagne, adaptation de garde ou qualification native n'est autorisé par cette reprise. Les contrôles statiques et `git diff --check` sont possibles. Les résultats des revues ne seront pas présentés comme une preuve d'exécution. Aucune écriture hors dépôt, installation, dépendance, modification de worker natif ou push.

## 9. Résultat de la reprise ciblée et des deux avis

**Réalisation.** Même `/root/pu_l1_implementer`, mode CORRECT, après la consignation de l'autorisation au Git `8af5a34c` ; baseline code `1d8309d9984b2c284b982db196f21954aa901900`. Seul `crates/whisper-adapters/src/worker_ipc.rs` est modifié par le worker. La correction retire la ligne errante `strategy:`, distingue le préflight GPU contrôlé de la qualification de session legacy et remplace le `VecDeque` cumulatif par un drainage du document à curseur, avec une page d'une entrée par interrogation. Tests écrits pour le parcours CPU/GPU contrôlé court/long et pour la borne, l'ordre et l'offset final des segments. Les tests d'import requièrent le vrai modèle Parakeet conforme au registre, avec échec explicite si le prérequis manque. Aucun test exécuté. Candidat figé avant les deux lectures ; aucune édition entre les avis.

**Indépendance et comptage.** La coordination `/root/pu_l1_review` a réutilisé le reviewer historique et confié une seconde lecture à un autre `rust_reviewer` indépendant. Les deux reçoivent le même périmètre, les critères R1/R5/R6 et le même candidat figé ; leurs conclusions ne sont pas partagées avant réception de leurs packets séparés. Aucun reviewer n'a participé à l'implémentation ou à sa correction. Cette reprise constitue la troisième correction cumulée du livrable, exceptionnellement autorisée au §8 après les deux corrections historiques ; les deux lectures indépendantes s'ajoutent aux trois passes antérieures. Aucun compteur historique n'est réinitialisé et aucune correction supplémentaire n'est lancée après ces avis.

| Avis | Résultat propre, sans fusion des conclusions |
| --- | --- |
| A — reviewer historique `/root/pu_l1_review/pu_l1_reviewer` | **FINDINGS**. Ferme statiquement R5 et R6 ; R2–R4 restent clos. La dépendance du chemin contrôlé au profil de session `import-single-pass` a été retirée, mais R1 reste ouvert : le test GPU utilise `fake_gpu_candidate()` avec une identité PNP abrégée alors que `expected_gpu_for_candidate` exige l'identité complète du registre. Le scénario aboutirait à `GpuIdentityMismatch` avant le worker et ne démontre donc pas l'import GPU contrôlé. |
| B — `/root/pu_l1_review/pu_l1_second_review`, second reviewer indépendant | **FINDINGS**. Confirme statiquement les corrections du préflight GPU contrôlé, le refus de l'activation native avant T9, la suppression de la ligne errante et le drainage d'un segment par interrogation. Ne relève pas l'incohérence PNP de l'avis A. Émet S2-1 sur le traitement d'une erreur de lecture de page. |

**Constats ouverts après les deux avis :**

- **R1 — T4 / AC-PU-08, encore ouvert selon A, `worker_ipc.rs:9219` et `:12033` :** aligner la fixture GPU sur l'identité effectivement validée par le registre, sans affaiblir les contrôles de candidat/génération. Les scénarios court/long doivent atteindre le worker contrôlé par le vrai parcours GPU, puis la publication réelle. La correction de la qualification en production et la preuve contrôlée de ce parcours restent deux sous-conditions distinctes.
- **S2-1 — IMPORTANT/MEDIUM selon B, `worker_ipc.rs:2427` :** en cas d'échec de `read_page()`, `poll_controlled_document()` renvoie une erreur sans retirer `controlled_document_drain`. Les interrogations suivantes peuvent donc réémettre `Failed`, y compris après l'échec terminal de la demande. Correction attendue : terminaliser/libérer le drainage lors de cette erreur et émettre l'échec une seule fois. Preuve minimale : une erreur de page produit un seul `Failed`, puis aucun nouvel événement de ce drainage.

**État de livraison.** Verdict de coordination : **LIVRÉ AVEC RÉSERVES** ; aucun des deux avis n'est CLEAN. R5 et la borne mémoire R6 ont une fermeture statique du reviewer historique, mais cela ne ferme ni R1 ni S2-1 et ne prouve pas la compilation. Les descriptions du §8 sont le mandat ; le présent résultat est l'état réellement obtenu. T4 n'est pas déclarée terminée.

`git diff --check` réussi par le worker et confirmé par la session principale. Compilation, formatage, tests, exécution audio/native et recette Windows **NOT RUN**, E-GUARD inchangé. Espace libre à la clôture : 113,18 Gio. Aucun fichier personnel touché, campagne, nettoyage, installation, nouvelle dépendance, worker natif ou push. La décision restante est une correction ciblée des deux constats suivie de leur relecture, ou le maintien du candidat avec ces réserves ; elle n'est pas déduite implicitement de la reprise déjà réalisée.

## 10. Fermeture statique de R1 et S2-1

**Autorisation de la session, 26 octobre 2026.** Après le §9, l'utilisateur confirme : « je t'autorise la correction ciblée de ces deux points, puis leur vérification par les mêmes reviewers ». Base de reprise : `2554fe97993ca3a4f018a8aa4ad3db948e62a03d`, branche `evo-startup-perf`, suivi propre. Espace libre avant reprise : 112,85 Gio. Cette autorisation précise couvre la quatrième correction cumulée, bornée à R1/S2-1, puis deux relectures par les reviewers déjà intervenus ; elle ne réinitialise aucun compteur historique ni n'autorise une campagne.

**Candidat corrigé.** Même `/root/pu_l1_implementer`, mode CORRECT, un seul chemin source modifié : `crates/whisper-adapters/src/worker_ipc.rs`. La fixture GPU utilise désormais l'identité complète `PARAKEET_TEST_GPU_PNP_INSTANCE_ID`, sans retirer les contrôles PCI/PnP, de génération et de runtime de `expected_gpu_for_candidate`. Le modèle est résolu par le registre `ParakeetTdt06BV3` puis vérifié par `assert_real_unified_model` ; son absence reste un échec explicite. Les scénarios GPU écrits dépendent aussi du GPU correspondant et de `nvidia-smi`, ce qui ne constitue pas une exécution ni une qualification native dans cette reprise.

`poll_controlled_document()` prend le drainage avant lecture ; il ne le réinstalle qu'après un état Ready ou une page lue avec succès. Une erreur ou la fin libère ce drainage. Le test ajouté via l'adaptateur vérifie une seule `PortError::Failed`, puis `Ok(None)`. Aucun test lancé. Le candidat a été figé avant les relectures et n'a pas changé entre les avis.

**Deux avis séparés sur le même candidat.** Coordination `/root/pu_l1_review`, sans participation au code. Le reviewer historique `/root/pu_l1_review/pu_l1_reviewer` et le second `/root/pu_l1_review/pu_l1_second_review` ont chacun reçu les mêmes critères R1/S2-1 et le delta figé ; aucune conclusion de l'un n'a été transmise à l'autre avant réception de leurs packets.

| Reviewer | Résultat du packet |
| --- | --- |
| Historique | **BLOCKED** pour les preuves d'exécution sous E-GUARD ; aucun finding ouvert. Ferme statiquement R1 et S2-1 et confirme les fermetures R2–R6. |
| Second | **BLOCKED** pour les preuves d'exécution sous E-GUARD ; aucun finding ouvert. Ferme statiquement R1 et S2-1, sans nouveau défaut relevé sur le delta. |

La correction et les deux vérifications demandées sont réalisées. Les deux points de code sont clos par les reviewers, et non par le worker ou la session principale. Il n'y a aucun constat de code ouvert à l'issue de ces packets. Leur verdict **BLOCKED** ne devient pas CLEAN : compilation, formatage, tests, comportement réel et GPU restent non vérifiés. Les deux nouvelles passes s'ajoutent aux cinq lectures antérieures du code ; le cumul est de quatre corrections et sept passes de revue, avec les autorisations supplémentaires distinctes des §§8 et 10. Aucun cycle de correction supplémentaire n'est lancé.

**Contrôles et clôture.** `git diff --check` réussi par le worker et confirmé par la session principale. Cargo, rustc, rustfmt, compilation, tests, audio/GPU et recette Windows **NOT RUN** ; E-GUARD toujours non satisfait. Espace libre final : 112,81 Gio. `models/` et `sound example/` restent non suivis et préservés. Aucun fichier hors dépôt, dépendance, worker natif, installation, campagne, nettoyage ou push. Le raccord T4 dispose maintenant d'un candidat sans constat de code ouvert, mais sa démonstration exécutée et sa validation fonctionnelle ne sont pas acquises ; les dépendances EXECUTION des lots suivants restent non satisfaites.

## 11. Préparation de validation PU-L1 et arbitrage E-GUARD

**Autorité et observations du 26 octobre 2026.** À la proposition de résoudre E-GUARD puis valider PU-L1, l'utilisateur répond : « ok mets en oeuvre la prochaine etape ». Cette instruction autorise la préparation et la validation sous les contraintes applicables ; elle n'accepte pas implicitement un risque technique ni une dérogation nouvelle. Base produit `7a75e30ef14cfca8ac705e8633eae5db68a53e5d`, branche `evo-startup-perf`, suivi propre ; `models/` et `sound example/` non suivis préservés. Préflight en lecture seule : 112,24 Gio libres, aucun Cargo, garde, auto-test ou binaire exécuté.

**E-ENV/E-INPUTS préparés, non qualifiés par exécution.** `rust-toolchain.toml` demande 1.98.1 ; les exécutables `cargo.exe`, `rustc.exe`, `rustfmt.exe` et `cargo-fmt.exe` existent dans `C:\Users\cyril\.rustup\toolchains\1.98.1-x86_64-pc-windows-msvc\bin`. Une future campagne devra appeler cette chaîne directement, sans relais rustup susceptible d'écrire dans le profil. Le cache existant `target/parakeet-tests/micro-level/cargo-home` contient 29 837 fichiers, environ 0,731 Gio ; il peut être lu pour une copie indépendante, sans écriture dans ce cache et sans liens matériels. Le modèle `models/parakeet-tdt-0.6b-v3.q8_0.gguf` existe, taille 713 975 456 octets, SHA-256 relevé `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`, conforme au registre. Aucun modèle copié ni modifié. La racine proposée `target/parakeet-tests/parakeet-unified/lot-01` n'existe pas encore.

**Blocage établi.** Lecture de `target/parakeet-tests/micro-level/guard-micro-level.ps1` et `target/parakeet-tests/parakeet-micro-lot1/validate-parakeet-micro-lot1.ps1`. Ces gardes contrôlent les variables/chemins et utilisent `FileSystemWatcher` pour détecter des événements après écriture ; aucune n'est un confinement préventif. Elles sont liées à d'autres racines, commandes et exceptions `UNATTRIBUTED`, non transférables. La garde micro-level ignore aussi certains événements `.git` ; ces exclusions ne sont pas une autorisation pour PU-L1. Une simple copie ou un changement de racine ne résout donc pas E-GUARD. Les auto-tests de garde ne sont pas lancés pour contourner cet arbitrage ; aucun essai d'écriture hors racine n'est réalisé.

### 11.1 Proposition bornée à accepter avant réalisation

**Statut : proposée, non acceptée.** Autoriser pour cette seule validation PU-L1 l'adaptation minimale du lanceur PowerShell existant vers `target/parakeet-tests/parakeet-unified/lot-01/validate-pu-l1.ps1`, sans installation, élévation, ACL, service ou tâche planifiée. Aucun nouvel outil de persistance des agents ni nouveau rapport obligatoire. Ce lanceur ne serait pas présenté comme un confinement du compte Windows.

Les seules destinations applicatives volontaires resteraient :

- `C:\dev\whisper\target\parakeet-tests\parakeet-unified\lot-01\` : lanceur, copie du cache Cargo, temporaires, fixtures, paramètres de test, archives/snapshots de test, sorties et petits journaux utiles ;
- `C:\dev\whisper\target\parakeet-tests\cargo-shared\` : compilation uniquement, sans nettoyage du contenu existant.

**Risque à accepter expressément.** Comme les dérogations historiques, mais avec une nouvelle portée PU-L1 explicite : le lanceur ne peut empêcher ni détecter exhaustivement les écritures techniques de Windows, du pilote NVIDIA ou de services extérieurs. Un événement observé uniquement sous `%LOCALAPPDATA%\Temp` et sans attribution démontrée serait consigné `UNATTRIBUTED`, avec chemin et heure, sans arrêt pour cette seule observation. Cela n'autoriserait aucune destination volontaire dans le Temp utilisateur et ne qualifierait pas l'origine de l'événement. Toute écriture hors racine attribuée à la campagne, toute modification inexpliquée d'un emplacement sensible, tout événement de surveillance perdu ou échec de préflight imposerait l'arrêt et la conservation des éléments ; aucun effacement ou restauration automatique. Les événements sensibles du dépôt, y compris `.git`, ne seraient pas ignorés.

**Contrôles maintenus.** Avant tout Cargo : vérifier les ancêtres et jonctions des deux racines, l'identité exacte du target partagé, les variables présentes et canoniques ; fixer `CARGO_HOME`, `TEMP`, `TMP`, `TMPDIR`, `RUSTC_TMPDIR` et les destinations applicatives sous lot-01, avec `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, hors ligne/locked. Vérifier les entrées et la copie de cache, lire sources/modèles/runtimes sans écriture volontaire. Faire refuser par auto-tests les variables absentes/externes/jonctions avant Cargo ; les tests de surveillance éventuels doivent rester sous la racine et employer une observation simulée d'événement externe, jamais créer réellement un fichier extérieur. Surveiller pendant chaque commande ; ne pas assimiler cette surveillance à une prévention. Arrêter au premier échec de commande, conserver le diagnostic et ne pas relancer automatiquement.

**Budget proposé :** celui de PU-L1 déjà proposé, 90 minutes et 16 Gio de croissance totale du lot et de la compilation partagée, à contrôler pendant la campagne ; arrêt si moins de 30 Gio libres, budget atteint ou état non attribuable. Pas de nettoyage, téléchargement, nouvelle capture micro, inférence native Parakeet réelle, recette Windows ou activation unifiée. Le contrôle d'identité GPU par `nvidia-smi` reste une lecture nécessaire aux scénarios contrôlés GPU et ne vaut pas qualification T9. À la clôture, bilan dans ce plan et conservation des petites sorties sous la racine du lot selon AGENTS, sans suppression de temporaires non expressément approuvée.

### 11.2 Commandes de validation à ajuster aux tests réels

Les cibles d'intégration `parakeet_unified` proposées en C1/C2 n'ont pas été créées ; les tests ont été écrits dans les modules sources et les tests existants autorisés. Les commandes ci-dessous sont des contenus pour le futur lanceur validé, pas des autorisations de Cargo direct. Cwd `C:\dev\whisper`, PowerShell, chaîne 1.98.1 installée, une commande à la fois :

1. `cargo fmt --all -- --check` ; contrôle sans correction de fichiers.
2. `cargo test --offline --locked -p whisper-core -p whisper-adapters -p whisper-desktop --no-run` ; compilabilité du candidat et des constructions de demandes dans les tests, sans les exécuter.
3. `cargo test --offline --locked -p whisper-core --lib` et les cibles core de contrats modifiées nécessaires à PU-L1 ; vérifier sessions, frontières et propagation de stratégie.
4. `cargo test --offline --locked -p whisper-adapters --lib <filtre> -- --test-threads=1`, avec filtres explicites limités à `parakeet_audio::tests::`, `parakeet_document::tests::`, `parakeet_pipeline::tests::`, aux nouveaux cas queue/source-binding/archive de compatibilité, et aux cas contrôlés `worker_ipc::tests::` de court/long CPU/GPU, refus natif, drainage normal/erreur, temps invalides et collision d'export. Le lanceur doit énumérer les noms réellement présents et les passages exécutés ; aucun succès silencieux sans cas exécuté.
5. Exécuter les régressions ciblées de persistance et contrats déjà prévues par le plan seulement si les étapes précédentes passent et que leurs ressources/destinations sont couvertes. Le workspace complet, les campagnes natives longues et le build de remise restent T9/T10.

Les numéros ci-dessus regroupent des commandes de vérification des tâches T1–T4 ; ils n'ajoutent pas de tâches au plan. Aucun de ces contenus n'a été exécuté. Une erreur de formatage, compilation ou test sera un résultat réel à rapporter, sans correction du code ni nouveau cycle déduit implicitement de cette proposition. E-GUARD reste non satisfait tant que cette dérogation, sa portée PowerShell/racines/budget et la campagne n'ont pas été précisément acceptées et le lanceur vérifié avant Cargo.

## 12. Campagne PU-L1 autorisée et arrêt au premier contrôle

**Autorité de session.** En réponse au §11, l'utilisateur écrit : « j'autorise dérogation et cette campagne ». Cette réponse accepte la portée PowerShell, les deux racines, les plafonds et le risque résiduel décrits au §11 pour cette campagne uniquement. Les interdictions générales et les autres lots ne sont pas modifiés. Base effective : `1781c0c0ae404a46142c87555bee27a2b8112252` (base produit `7a75e30e`), suivi propre ; fichiers personnels non suivis préservés. Espace libre avant réalisation : 111,87 Gio.

**Lanceur et garde effectivement réalisés.** `target/parakeet-tests/parakeet-unified/lot-01/validate-pu-l1.ps1`, adapté du lanceur micro-lot1, utilise directement les exécutables Rust 1.98.1 et ne passe pas par rustup. `CARGO_HOME`, `TEMP`, `TMP`, `TMPDIR`, `RUSTC_TMPDIR`, le répertoire rustup inutilisé et les réglages de test sont sous lot-01 ; `CARGO_TARGET_DIR` est exactement la racine partagée. Contrôles d'ancêtres/jonctions, présence des variables et répertoires, lockfile, espace libre et budgets ; surveillance partitionnée autour des racines de fort débit, sans ignorer `.git`. Un marqueur `STOP.json` empêche une relance automatique après échec. La garde reste une surveillance, pas un confinement.

Le script a d'abord passé le contrôle de syntaxe PowerShell sans exécution. Un préflight seul, exécution `c4b299b892614629bd6038e344365c1f`, a réussi **26 essais de refus** (variables absentes, externes, répertoires inexistants, jonction réelle existante lue sans écriture, chemin extérieur simulé), ainsi que le témoin réel de surveillance dans la racine. Aucun fichier extérieur créé par les auto-tests, aucune jonction créée ni retirée. Les événements simulés `.git`, Temp et perte de surveillance sont classés conformément au mandat. Aucun Cargo pendant ce premier passage.

**Campagne réelle :** exécution `6d673978c52b4b81a5df5a5ea646a50e`, mêmes contrôles préflight et 26 refus réussis, puis copie indépendante du cache existant micro-level : **29 837 fichiers, environ 0,731 Gio**, sans lien matériel ni écriture volontaire dans le cache source. Commande lancée sous garde : `cargo fmt --all -- --check`, avec le cargo installé 1.98.1, cwd `C:\dev\whisper`, stdout/stderr sous la racine du run. Résultat : **FAIL, code 1**. Le lanceur a immédiatement arrêté la séquence ; aucune compilation ni aucun test exécuté, aucune relance.

**Diagnostic exécuté, distinct des revues statiques :**

- **V-PU-L1-01 — syntaxe, PRODUCT :** `crates/whisper-adapters/tests/reprocessing_baseline.rs:339` contient `strategy: ...LegacyV1,` directement dans le corps de `wait_for_queued_request`, avant sa boucle. Diagnostic de rustfmt : `expected identifier, found ':'`. Attribution Git lue : ajout au commit `1d8309d9`. Le fichier est dans GT ; cette erreur n'avait pas été relevée par les revues statiques précédentes. Cela interdit d'annoncer une compilabilité acquise.
- **V-PU-L1-02 — formatage :** stdout contient des écarts dans **33 fichiers Rust** du workspace ; ce nombre exclut le fichier que rustfmt n'a pas pu analyser. Les diff exacts sont conservés dans `runs/6d673978c52b4b81a5df5a5ea646a50e/01-fmt-check.stdout.log`. Certains concernent des portions ou chemins hors du delta T4, notamment `crates/whisper-core/src/model_selection_app.rs` et `crates/whisper-desktop/tests/microphone_level_ui.rs`. Aucun formatage global n'a été appliqué implicitement et aucune origine de tous ces écarts n'est présumée.

**Observations de surveillance.** Pendant le run réel : **188 événements `UNATTRIBUTED`** uniquement sous `%LOCALAPPDATA%\Temp`, avec chemin et heure consignés ; origine non déterminée. Aucun événement `FAIL`, `WATCH_ERROR` ou échec de clôture relevé par la garde. Ces observations ne prouvent ni l'absence exhaustive d'écritures extérieures, ni leur attribution à Windows/Cargo. Aucun fichier Temp observé n'a été supprimé, restauré ou qualifié d'écriture autorisée. Le traitement correspond à la dérogation acceptée.

**Clôture et conservation.** `git status` ne montre aucune modification suivie des sources après campagne ; seuls `models/` et `sound example/` restent non suivis. `git diff --check` réussi, sans portée sur le diagnostic syntaxique de rustfmt. Petites preuves archivées en préservant leurs chemins relatifs (lanceur, marqueur d'arrêt, deux runs et leurs sorties) dans `target/parakeet-tests/parakeet-unified/lot-01/archives/pu-l1-validation-6d673978c52b4b81a5df5a5ea646a50e.zip` : **8 fichiers**, nombre et SHA-256 de chaque entrée vérifiés contre les originaux ; archive de 40 962 octets, SHA-256 `506b509fba93f56a40f9d0353dd9439c57ef4ecc280e2173993cde7a362ef711`. Aucun manifeste d'empreintes supplémentaire créé. Cache local, journaux, marqueur et target partagé préservés ; aucune suppression. Espace libre final : **111,03 Gio**. Aucun push.

**Prochaine action proposée, non exécutée.** Corriger uniquement la ligne de syntaxe parasite dans le test, puis appliquer le formatage Rust sans changement logique aux fichiers signalés par le contrôle du workspace (avec la portée explicite des chemins hors delta si acceptée). La correction revient à `rust_implementer` ; elle n'est pas déduite du mandat de validation. Une reprise de la campagne doit conserver les éléments du premier arrêt, repasser le préflight sous la même dérogation précise et respecter les budgets approuvés ; aucune relance n'est effectuée sans l'arbitrage demandé après cet échec. Les étapes de compilation/tests du §11.2 restent **NOT RUN**, et les lots PU-L2 à PU-L4 ne démarrent pas.

## 13. Correction de syntaxe, formatage et reprise autorisés

Le 26 octobre 2026, à la demande précise « Autorises-tu la correction de cette erreur, le formatage Rust du workspace sans changement logique, puis la reprise de la campagne ? », l'utilisateur répond : « je t'autorise ». Cette décision autorise la suppression de la ligne parasite du §12, le formatage Rust du workspace, y compris les chemins hors delta signalés par rustfmt, et une reprise de la validation PU-L1 dans la dérogation déjà acceptée. Elle n'autorise aucun autre changement logique ni aucune correction future de compilation/test par anticipation. Base `5eebbf34f69b845895e9cf3cdef1c0a78a434c75`, suivi initial propre, `models/` et `sound example/` préservés, 110,80 Gio libres au préflight.

Le même `rust_implementer` possède les corrections sources : retrait manuel de la ligne invalide de `reprocessing_baseline.rs`, puis `cargo fmt --all` uniquement via la garde adaptée. Le mode `FormatOnly` donne pour cette seule commande une permission transitoire aux chemins exacts des fichiers Rust suivis sous `crates/`, afin de réaliser le formatage expressément autorisé ; aucune écriture de manifeste, configuration, `.git`, modèle ou fichier personnel n'est ajoutée. Le mode `Validate` retrouve exclusivement les deux racines de campagne. Le lanceur reste la responsabilité de la session principale ; aucun autre agent ne l'adapte en parallèle.

La reprise exige un paramètre explicite `ResumeAuthorized` si un marqueur d'arrêt existe et déplace ce marqueur, sans écrasement ni suppression, vers un chemin conservé sous lot-01. Le SHA du candidat est passé explicitement au lanceur au lieu de conserver l'ancien identifiant. Les essais de refus et de surveillance précèdent tout Cargo dans chaque mode. Le temps des exécutions antérieures est compté contre les 90 minutes ; le stockage déjà présent dans le lot, notamment les 0,731 Gio de cache et les petites preuves, reste compté contre les 16 Gio. Le target partagé est mesuré avant la première compilation puis sa croissance est réutilisée entre les reprises ; aucune compilation n'a encore eu lieu.

Après le formatage, le candidat sera figé et le delta syntaxe/formatage transmis à la coordination de revue Rust prévue par AGENTS, sans nouvelle campagne documentaire. La campagne reprendra ensuite sur le candidat identifié ; elle s'arrêtera au premier échec conformément au §11. Le contrôle de formatage, la compilation et les tests restent à exécuter sur ce candidat. Aucun téléchargement, installation, worker natif, validation Windows ou push n'est inclus.

**Réalisation de la correction.** Le même `/root/pu_l1_implementer` a retiré uniquement la ligne parasite de `wait_for_queued_request`, puis exécuté le lanceur autorisé en `FormatOnly`, avec reprise explicite et candidat `decb117165c1318d615b1f8455d222789ea67877`. Run `7158dcdd454a4db394d9366b0ff877f1` : 26 essais de refus PASS, commande `cargo fmt --all` PASS, code 0. La permission transitoire couvrait 115 chemins Rust suivis ; 34 fichiers ont été modifiés. L'ancien marqueur est conservé dans `retained-stops/STOP-before-7158dcdd454a4db394d9366b0ff877f1.json`. Soixante événements Temp `UNATTRIBUTED` consignés, aucun événement `FAIL` ou `WATCH_ERROR`. Durée de cette exécution : 91,67 secondes ; 263 secondes des passages antérieurs étaient déjà comptées. Baseline partagée avant compilation : 15 669 963 879 octets, journalisée pour conserver le même budget de croissance aux reprises.

**Revue du delta figé.** Coordination `/root/pu_l1_review`, reviewer historique indépendant, baseline `decb1171`, candidat de travail après formatage. Verdict **CLEAN limité à V-PU-L1-01 et au delta syntaxe/formatage des 34 fichiers Rust**. Le reviewer ferme statiquement V-PU-L1-01 : la ligne hors expression est supprimée ; autres différences limitées à whitespace, virgules finales, accolades d'expressions équivalentes et réordonnements d'import/module sans dépendance de macro constatée. Aucun autre changement logique détecté ni finding. `git diff --check` PASS. Ce verdict ciblé ne qualifie ni la compilation ni le fonctionnement de T4. Le worker et les reviewers n'ont exécuté aucune compilation ni aucun test.

## 14. Reprise sur le candidat corrigé — compilation en échec

**Candidat et commandes.** Syntaxe et formatage versionnés au Git `3fb38845843a2874e0ed923bf669e80ddd239191`. La session principale a relancé le mode `Validate` du même lanceur, sans permission transitoire sur les sources et avec ce SHA explicite, sous l'autorisation du §13. Run `2d05360496724e8195025b3e0c830469`, préflight et 26 essais de refus PASS, cache local réutilisé sans recopie ni écriture du cache source.

| Vérification réellement lancée | Résultat |
| --- | --- |
| `cargo fmt --all -- --check` | **PASS**, code 0. V-PU-L1-01 corrigé et V-PU-L1-02 résolu pour ce candidat. |
| `cargo test --offline --locked -p whisper-core -p whisper-adapters -p whisper-desktop --no-run` | **FAIL**, code 101. Compilation interrompue sur E0277 dans whisper-core. |
| Exécution des tests core/adapters, scénarios contrôlés CPU/GPU et régressions | **NOT RUN** : séquence arrêtée avant leur lancement. |

**V-PU-L1-03 — erreur de compilation, PRODUCT.** `crates/whisper-core/src/parakeet_session.rs:14–17` dérive `Hash` pour `SessionIdentity`, qui possède un champ `Generation`. `crates/whisper-core/src/domain.rs:8` ne dérive ni n'implémente `Hash` pour `Generation`. Le compilateur émet `the trait bound Generation: Hash is not satisfied` (E0277) pour la bibliothèque et sa cible de tests : il s'agit du même défaut, pas de deux erreurs indépendantes. Le diagnostic suggère d'ajouter `Hash` à `Generation` ; la correction minimale cohérente doit être décidée par le `rust_implementer` dans les chemins G1 autorisés en préservant l'identité et l'égalité des générations. L'avertissement d'import `SAMPLE_RATE` inutilisé est distinct et n'est pas la cause de l'arrêt.

**Arrêt et surveillance.** Aucune correction, relance ou suppression après le code 101. Sources suivies inchangées après cette exécution, aucun test réellement exécuté. Quarante-six événements Temp `UNATTRIBUTED`, aucun événement `FAIL`/`WATCH_ERROR` ni échec de clôture relevé ; origine des événements Temp indéterminée et absence exhaustive d'écritures non démontrée. Le nouveau `STOP.json` bloque une reprise automatique ; le premier arrêt et son archive restent conservés. Durée du run : 77,31 secondes ; cumul enregistré des exécutions de garde/commande : environ 431,98 secondes, sous le plafond de 90 minutes. Stockage mesuré des deux racines par rapport à la baseline partagée : environ 0,722 Gio de croissance nette, y compris cache local et preuves, sous 16 Gio ; cette mesure ne prétend pas mesurer des économies disque par suppression. Espace libre final : **110,55 Gio**.

**Petites preuves archivées.** Archive `target/parakeet-tests/parakeet-unified/lot-01/archives/pu-l1-resume-2d05360496724e8195025b3e0c830469.zip`, 51 972 octets, **19 fichiers** incluant lanceur courant, quatre runs, arrêts courant/conservé. Arborescence, nombre et SHA-256 de chaque entrée vérifiés contre les originaux ; SHA-256 de l'archive `e71da11b70f35ccf85e62d237b11aabfbd46f9d120f39579110d351f14777f4c`. L'archive initiale du §12 reste inchangée et conserve aussi la version précédente du lanceur. Aucun manifeste supplémentaire, cache/target nettoyé, fichier personnel modifié ou push.

**Arbitrage proposé, non acquis.** Autoriser une correction des erreurs de compilation de PU-L1 dans ses chemins déjà approuvés, à partir de V-PU-L1-03, avec au plus deux cycles de correction/revue sur ce mandat borné ; après chaque correction relue, reprise explicitement couverte de la même campagne sous les mêmes budgets et dérogation. Chaque commande en échec resterait arrêtée et ses preuves conservées : aucune relance inchangée pour obtenir un succès, aucune extension de chemins, modification de dépendance, installation, autre lot ou correction fonctionnelle déduite de ce mandat. Après deux cycles, tout constat restant ou nouvelle décision de périmètre revient à l'utilisateur. Cette proposition vise à traiter les erreurs de compilation éventuellement révélées successivement sans masquer les arrêts ; elle n'est pas une autorisation implicite et n'a pas été mise en oeuvre.
