Avancée : le design UI est décliné en neuf lots bornés et vérifiables.
Blocage : l’implémentation attend la résolution de Q-01/Q-02 et une autorisation précise dans la session d’exécution.
Décision : plan proposé ; il ne vaut ni GO code, ni campagne, ni validation de fenêtre.

# Plan — refonte de l’interface du transcripteur

## 1. Autorité, base et limite

- Auteur : session Codex principale. Version 1, 23 novembre 2026.
- Parent : [design-ui-transcripteur-maquette.md](design-ui-transcripteur-maquette.md), DESIGN proposé ; exigences REQ-01–12, critères AC-01–11, questions Q-01–04.
- Brief source : [brief-ui-transcripteur-maquette.md](brief-ui-transcripteur-maquette.md).
- Base inspectée : commit `540aebf77b1c0c0f423c08285c09816dea7176a2`, branche `codex/ui-transcriber-mockup`. Vérifier à nouveau avant exécution. Le changement préexistant à `AGENTS.md` n’est pas dans le périmètre.
- Aucune implémentation, campagne Cargo, revue indépendante, build ou validation fenêtre n’est autorisée/réalisée par ce plan.

## 2. Périmètre futur et chemins

Chemins relatifs à `crates/` sauf mention. Ce tableau borne les destinations candidates ; l’implémenteur confirme les fichiers réels au préflight et arrête si un contrat impose d’élargir le plan.

| Groupe | Chemins envisagés | Responsabilité |
|---|---|---|
| D-UI | `whisper-desktop/src/ui/model/view.rs`, `view/actions.rs`, `view/settings.rs`, `view/microphone.rs`, `view/capture.rs`, `view/queue.rs`, `view/history.rs`, `view/dialogs.rs`, `view/notices.rs` | Composition, parcours et présentation |
| D-controller | `whisper-desktop/src/ui/model/controller/actions.rs`, `controller/observability.rs`, `ui/model.rs`, `ui/model/preferences.rs` | Raccord aux commandes/états existants, seulement si nécessaire |
| D-tests | `whisper-desktop/src/ui/tests.rs`, `ui/tests/` | Tests UI/interaction et états visibles |
| Domaine conditionnel | `whisper-core/src/application/contracts.rs`, `domain.rs`, `ports.rs`, tests core correspondants | Seulement si planifié après revue de Q-01/Q-02 ; hors autorisation implicite |
| Adaptateur conditionnel | crate propriétaire du stockage/ouverture existant, déterminée par lecture source | Aucun nouveau fichier/adaptateur/dépendance sans décision explicite |

Pas de changement de `Cargo.toml`/`Cargo.lock`, dépendance, feature, API publique, protocole IPC, format durable, modèle, moteur, crate système ou technologie hors stack. Les fichiers du domaine conditionnel ne sont pas autorisés à être modifiés tant qu’une décision de portée et un plan révisé ne les incluent pas.

## 3. Lots et dépendances

Neuf tâches, sous la limite commune de dix. Une seule responsabilité d’écriture à la fois ; les chemins UI partagés ne sont pas parallélisés.

| Lot | Tâches | Dépend de | Sortie |
|---|---|---|---|
| L0 — Préflight technique | T1 | documents acceptés, état Git, autorisation d’implémenter | Q-01/Q-02 résolues sans extension, ou STOP et arbitrage |
| L1 — Contrats UI et rendu statique | T2–T3 | L0 PASS | façade/configuration/spectre sans régression de commandes |
| L2 — Import et séquencement | T4–T5 | L1 | deux modes, doublons, FIFO et annulation raccordés au cœur existant |
| L3 — Finalisation, résultats et liens | T6–T7 | L1 ; Q-02 PASS | résultat Parakeet publié, historique et chemins présentés correctement |
| L4 — Vérification et remise | T8–T9 | L1–L3 | suite locale attribuée, revue indépendante, observation Windows utilisateur |

Graphe : **T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9**. T4/T6 peuvent partager les vues de file ; aucune écriture concurrente. Chaque lot reste sous dix tâches. L’ensemble des rapports planifiés est de deux documents.

## 4. Tâches

| ID | Travail | Critères de sortie |
|---|---|---|
| **T1** | Relire l’état Git, règles racine/desktop, contrats d’archive, publication et fermeture. Résoudre Q-01/Q-02 à partir des sources, sans Cargo. Vérifier les chemins candidats, branche et modifications concurrentes. | Note d’entrée attribuée dans le bilan futur. Si une nouvelle persistance, migration, dépendance ou modification hors D-UI/D-tests est nécessaire : STOP avant code et demander arbitrage/design révisé. |
| **T2** | Composer la façade d’accueil et l’en-tête : cartes, état effectif, raccourci de sortie, accès à configuration avancée, responsive. Réutiliser le rendu et commandes déjà existants. | REQ-01/12 ; AC-07/10 partiels. Aucune politique produit implémentée dans la vue. |
| **T3** | Réorganiser les réglages avancés et adapter le spectre à 64 bandes selon la mesure existante ; état attente/silence/périmé/saturation visible. | REQ-08/09 ; AC-07/08. Valeurs par défaut et règles de verrouillage inchangées ; agrégation linéaire en puissance puis dB. |
| **T4** | Implémenter le choix entre Importer et transcrire / Importer seulement, dialogue de doublon, préservation du choix et usage de l’identité issue de l’admission. | REQ-02/03 ; AC-01/02. Annulation du dialogue sans effet ; aucun calcul sur identité supposée. |
| **T5** | Raccorder file FIFO, positions/attente micro, annulation et fermeture/reprise confirmée à la politique cœur disponible. Garder erreurs, interruption et annulation distinctes. | REQ-04/05/06 ; AC-03/04/05. Si le cœur ne porte pas les transitions requises, ne pas dupliquer la politique dans UI : STOP, findings et plan révisé. |
| **T6** | Raccorder l’affichage de capture et résultat Parakeet au statut final après calcul existant et publication réussie ; préserver le texte confirmé. | REQ-10/11 ; AC-09/11 partiels. Aucun statut final si publication incomplète ; aucun bouton de vérification humaine ajouté. |
| **T7** | Présenter liste/historique, liens source/résultat, destination effective et messages d’échec de chemin. Conserver portée d’archive et récupération existantes. | REQ-06/07/11 ; AC-05/06/07. Liens nommés sans ambiguïté ; ne pas créer/substituer un dossier ; pas d’extension au schéma d’archive. |
| **T8** | Vérification consolidée des tests pertinents, format, build et Clippy conformément aux règles de campagne de `AGENTS.md` ; revue des régressions dans CPU/GPU/Auto et chemins. | REQ-01–12 ; AC-01–09/11 automatisables. Vérifications exécutées via garde autorisée, résultats et limites consignés ; aucun test ignoré présenté comme réussi. |
| **T9** | Revue par `rust_review_coordinator`, corrections via `rust_implementer`, maximum deux cycles ; observation finale de fenêtre par l’utilisateur. Clôturer avec bilan séparé, commits locaux seulement si autorisés ; aucun push. | Tous AC ; verdict CLEAN ou LIVRÉ AVEC RÉSERVES. Fenêtre Windows identifie résolutions observées à 100/150/200 %, sans assimiler tests au rendu manuel. |

L’ordre L1→L2/L3 pourra être ajusté uniquement après confirmation que les contrats métier nécessaires existent. Pas de refactorisation générale de l’UI. Le plan de tests, fichiers et chemins exacts sont finalisés dans un handoff après design/plan acceptés ; aucun railguard ou campagne documentaire additionnelle.

## 5. Matrice tâches / critères / chemins

| Critères | Tâches | Chemins principaux |
|---|---|---|
| AC-01/02, REQ-02/03 | T4, T8 | `view/actions.rs`, `view/dialogs.rs`, contrôleur, tests UI |
| AC-03/04/05, REQ-04/05/06 | T1, T5, T8 | `view/queue.rs`, contrats core existants, tests propriétaires |
| AC-06/07, REQ-07/08 | T2, T7, T8 | `view/history.rs`, `view/settings.rs`, service/adaptateur existant |
| AC-08, REQ-09 | T3, T8 | `view/microphone.rs`, présentation, tests UI/core existants |
| AC-09, REQ-10/11 | T6, T8 | `view/capture.rs`, `view/notices.rs`, contrats publication actuels |
| AC-10, REQ-01/12 | T2/T3, T9 | `view.rs` et vues concernées, observation Windows |
| AC-11, REQ-01–12 | T1, T8/T9 | tests et intégrations existants des crates concernées |

## 6. Vérifications futures et limites

- Avant toute commande Cargo : appliquer intégralement le préflight et la garde permanente du dépôt ; espace libre ≥30 Gio, racines/variables autorisées, activité Git concurrente évaluée. Pas d’exécution Cargo directe.
- V1 : `cargo fmt --check` par voie autorisée ; V2 : tests desktop ciblés ; V3 : tests des crates qui portent FIFO/publication seulement si inclus et autorisés ; V4 : builds desktop configurations par défaut et `personal-install` si applicables ; V5 : Clippy desktop et lints structurels selon matrice. Toutes ces commandes sont **NOT RUN** et le détail exact relève du handoff une fois chemins confirmés.
- Comparer les commandes/effets CPU/GPU/Auto et les parcours micro/import avant/après. Ne pas exécuter les recettes natives suspendues.
- Fenêtre Windows par l’utilisateur : accueil, import immédiat/différé, doublon/annulation, file, liens, configuration, capture micro, publication, tailles 1536×864 et 1024×720, minimum 960×640, échelles 100/150/200 %. Résultat NOT RUN jusqu’à observation.
- Les tests d’interface ne prouvent ni la qualité transcription ni la disponibilité atomique des trois fichiers ; le caractère atomique est la question Q-02.

## 7. Prérequis d’autorisation et readiness du plan

Le cadrage/plan ne donne pas GO d’implémentation. Avant un futur lot, il faut : revue DESIGN et PLANS acceptables, décisions Q-01/Q-02 consignées, chemins exacts et état Git inspectés, puis accord écrit explicite de l’utilisateur dans la session d’exécution couvrant le lot, les crates/fichiers et vérifications. Toute extension aux contrats core, format durable ou dépendance requiert arbitrage et révision du plan avant modification.

**Readiness PLANS : PROPOSÉ, conditionnel à Q-01/Q-02.** Aucun code, test ou validation visuelle n’a été exécuté.
