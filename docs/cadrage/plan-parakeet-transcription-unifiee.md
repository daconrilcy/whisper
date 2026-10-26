Avancée : proposition en dix tâches et quatre lots établie à partir du design accepté.
Blocage : aucun pour la revue du plan ; prérequis d'exécution des campagnes non satisfaits.
Décision : soumettre le candidat Git à la revue indépendante ; aucun code autorisé.

# Plan d'implémentation — transcription Parakeet unifiée

## 1. Identité, autorité et base

- ID : P-PARA-UNIFIED-01 ; version : v0.1 ; statut : DRAFT.
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

PENDING REVIEW. Le rédacteur ne déclare pas son propre CLEAN/READY. Acceptation du plan, autorisation de lot, revue de code, validation produit et publication resteront distinctes.
