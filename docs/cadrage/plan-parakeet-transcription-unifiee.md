Avancée : plan accepté ; candidat PU-L1 écrit partiellement et relu, trois constats corrigés et clos.
Blocage : T4 non raccordée au vrai import (R1) ; E-GUARD non satisfait, compilation et tests non exécutés.
Décision : ajustement du plan revu CLEAN ; fin de T4 autorisée dans le périmètre CHANGE-PU-L1-01.

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
