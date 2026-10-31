Avancée : cadrage du découpage de `parakeet_live.rs` établi sur les sources locales.
Blocage : le seuil chiffré, les chemins du plan et la campagne de vérification restent à valider avant implémentation.
Décision : proposer au plus 650 lignes physiques pour le fichier entier, tests compris ; aucun code autorisé par ce document.

# Refactoring de Parakeet Live — DESIGN-PL v1

## 1. Mandat et base

Demande utilisateur du 31 octobre 2026 : ramener `crates/whisper-adapters/src/Parakeet_live.rs` à environ 600 lignes en le refactorisant ; clarification explicite : **fichier entier, tests compris**. L'utilisateur a ensuite demandé ce cadrage et le plan d'implémentation. Le chemin réel suivi par Git est `crates/whisper-adapters/src/parakeet_live.rs`. Le cadrage et le plan servent la maintenance du parcours microphone Parakeet ; ils n'autorisent ni code, ni Cargo, ni push.

Base inspectée : branche `evo-startup-perf`, HEAD `b09a8b669834803cfd46e15c3c10eefdf744b518`, fichier sans diff suivi et **3 138 lignes physiques** (`(Get-Content -LiteralPath ...).Count`, lignes vides comprises). Les chemins non suivis `docs/architecture/`, `models/` et `sound example/` sont préservés. Sources : `AGENTS.md` racine et adapters, `parakeet_live.rs`, ses consommateurs cités au §3, `lib.rs`, carte d'architecture non suivie consultée comme relevé statique, et le cadrage de refactorisation de `direct_live.rs`. Revalider la base avant exécution.

## 2. Besoin, exigences et acceptation

Le mainteneur doit pouvoir suivre capture, session et inférence dans des fichiers de responsabilité claire. Pour l'utilisateur, la capture, les confirmations, l'arrêt et les sorties restent conformes au comportement actuel. Le refactoring est structurel : aucune promesse nouvelle de latence, de qualité, de reprise après panne ou de sortie.

| Besoin | Exigence et critère observable | Preuve future |
| --- | --- | --- |
| CA-01 taille | `parakeet_live.rs` entier, tests compris, **≤ 650 lignes physiques** après refactoring ; cible indicative ~600 ; compter aussi les extraits et signaler toute concentration > 1 500 lignes de production. Aucun compactage artificiel. | Comptage identique avant/après, revue du diff |
| CA-02 découpage | Chaque extrait a une responsabilité distincte ; aucun cycle de dépendance ni API produit créée pour les tests ; seuls les chemins acceptés sont édités. | Carte ancien→nouveau, compilation et revue |
| CA-03 compatibilité | Signatures et visibilités `pub(crate)` utilisées par les consommateurs, identité `JobId`/`Generation`/`session_id`, ordre des événements et erreurs, CPU/GPU/Auto, trace et sortie WAV/TXT/SRT restent conformes. | Comparaison des parcours et contrôles ciblés avant/après |
| CA-04 tests | Les neuf fonctions `#[test]` inline et leurs noms qualifiés sont conservés ; aucune assertion ou activation n'est perdue. | Inventaire lexical, `cargo test --list` sous campagne autorisée, résultats ciblés |
| CA-05 préservation | Les changements non liés, notamment les trois chemins non suivis, restent intacts. | État Git avant/après et diff borné |

La borne 650 est une **proposition technique**, pas une décision déjà donnée par l'utilisateur. Elle devra être acceptée avec le plan. Un dépassement documenté ou une nouvelle responsabilité demanderait un changement de plan, sans réduction artificielle des lignes.

Hors périmètre proposé : changement fonctionnel, protocole worker, modèle, UI, capture WASAPI de bas niveau, autres parcours, dépendance, feature, manifeste, `Cargo.lock`, nouveau thread ou canal, nouveau `unsafe`. La recette dans la fenêtre Windows reste à l'utilisateur si elle est demandée lors de la livraison.

## 3. Carte actuelle et invariants

| Plage de base | Responsabilité actuelle | Contrat à préserver |
| --- | --- | --- |
| 1–346 | `UnifiedCapturePump`, conversion et écriture PCM acquittée, gain ; impl `ReadablePcmSource` | Capture avant inférence, ordre des ACK, contre-pression, `stop` et lecture du backlog |
| 347–717 | Requêtes/événements d'inférence, diagnostic D2, `TraceSink`, ledger de reprise | Canaux bornés, trace optionnelle et bornée, classification des erreurs |
| 718–1029 | État `ParakeetLive`, démarrage, capture et gain | Capture et WAV ouverts avant worker ; ressources arrêtées sur erreur ; interfaces crate |
| 1030–1409 | `poll`, résultats, confirmation, télémétrie | Une fenêtre active, séquence et plages vérifiées ; confirmé distinct du provisoire ; événements inchangés |
| 1410–1698 | Arrêt normal/erreur, drainage et sorties | Reliquat PCM, WAV durable, sorties/diagnostics complets ou incomplets, arrêt idempotent |
| 1699–2177 | Ordonnancement, traces JSON, reprise, `Drop` | Fenêtres et tentatives bornées, mêmes identités et nettoyage |
| 2178–2770 | Thread d'inférence, transcription/validation de fenêtre, WAV temporaire | Backend et marqueurs contrôlés, timeout/annulation, rejet des résultats étrangers |
| 2771–3138 | Trois modules de tests inline, puis horodatage | Noms qualifiés et scénarios conservés |

Consommateurs directs : `worker_ipc/parakeet_unified/mod.rs` utilise `UnifiedCapturePump` et `AppliedCaptureGain` ; `worker_ipc/runtime/{mod,live}.rs` utilise `ParakeetLive` et la pompe ; `worker_ipc/service/automatic_final.rs` utilise `automatic_final_import` et `identity`. Les tests `worker_ipc/tests/native_execution.rs` et `direct_live/d2_corpus.rs` appellent les points d'entrée `cfg(test)`. Les appels internes dans `parakeet_live.rs` et les filtres de test historiques doivent conserver leurs chemins qualifiés. Les sources, et non la carte statique d'architecture, font foi.

Invariants à comparer sur les mêmes stimuli :

1. **INV-01 capture** : PCM écrit/acquitté avant considération pour l'inférence ; gain lié à la source et à la révision ; backlog borné ; aucun échantillon annoncé capturé perdu silencieusement.
2. **INV-02 session** : `JobId`, `Generation`, `session_id`, séquence et plage de fenêtre vérifiés avant émission ; résultat ancien ou étranger rejeté selon les branches actuelles, sans nouvelle politique implicite.
3. **INV-03 confirmation** : blocs confirmés ordonnés, sans doublon ni chevauchement ; refus et plages non résolues restent visibles ; publication finale et import automatique conservent leur ordre.
4. **INV-04 moteur** : choix CPU/GPU/Auto et repli existant, attestation/backend GPU, annulation et timeout inchangés ; pas de nouveau repli après début du traitement.
5. **INV-05 arrêt** : vidage capture/convertisseur, fermeture WAV, sorties TXT/SRT et diagnostic de complétude conservent les mêmes cas d'erreur et ordre d'effets.
6. **INV-06 trace/tests** : trace optionnelle, taille et destination bornées ; diagnostic D2 réservé aux tests ; neuf tests inline conservés sous les mêmes noms.

## 4. Découpage proposé

Conserver `parakeet_live.rs` comme façade privée de crate contenant `ParakeetLive`, les déclarations de modules, constantes et liens explicites entre responsabilités. Les sous-modules sont sous `src/parakeet_live/` ; `parakeet_confirmation.rs` reste à son chemin actuel via sa déclaration `#[path]`. Les implémentations de `ParakeetLive` peuvent être réparties entre sous-modules enfants sans déplacer sa propriété d'état ni élargir l'API de la crate. Les accès entre frères sont limités à `pub(super)` lorsque nécessaires ; revoir ces visibilités dans le diff.

| Chemin proposé sous `src/` | Contenu issu de la base | Ordre de grandeur indicatif |
| --- | --- | --- |
| `parakeet_live.rs` | Façade, état, constantes, imports/réexports nécessaires, accès crate minces | ~350–600, plafond proposé 650 |
| `parakeet_live/capture.rs` | `UnifiedCapturePump`, `AppliedCaptureGain`, capture/gain/télémétrie de session | ~450–550 |
| `parakeet_live/lifecycle.rs` | Démarrage, arrêt normal/erreur, drainage, `Drop` | ~500–650 |
| `parakeet_live/events.rs` | `poll`, confirmation, événements, progression, gestion des plages non résolues | ~500–650 |
| `parakeet_live/windows.rs` | Planification/soumission, reprise, validation et identité de fenêtre | ~350–500 |
| `parakeet_live/inference.rs` | Thread, requêtes/événements, transcription, WAV temporaire | ~550–700 |
| `parakeet_live/trace.rs` | `TraceSink`, formatage et diagnostic D2 sous `cfg(test)` | ~350–500 |
| `parakeet_live/{recovery_tests,window_result_contract_tests,microphone_spectrum_tests}.rs` | Trois modules de tests déplacés sans renommer les modules ni scénarios | ~80 / ~60 / ~220 |

Ces tailles ne sont pas des mesures du futur code. Si les tests nécessitent un support partagé, le placer sous cette famille avec `cfg(test)` et le soumettre au périmètre avant création ; ne pas créer de branche ou champ de production pour eux. Une extraction seulement des tests laisserait environ 2 770 lignes dans le fichier ; elle ne satisferait pas CA-01.

Matrice de frontière : `adapters` exécute la capture, le disque et l'IPC ; `core` conserve les politiques existantes ; `worker_ipc` consomme l'API crate actuelle. Aucune dépendance `archive → worker_ipc` ni nouveau lien vers l'UI. Les constantes de canal `sync_channel(1)` et `sync_channel(2)`, les bornes de fenêtre/trace et les signatures des points d'entrée restent inchangées.

## 5. Risques, preuves et statut

| Risque concret | Vérification prévue | État |
| --- | --- | --- |
| Perte audio ou changement de drainage/ACK après extraction | Tests de pompe, WAV et arrêt, comparaison du diff des branches d'erreur | NOT RUN |
| Résultat tardif admis ou confirmation modifiée | Tests d'identité/résolution, revue des transitions et des mêmes stimuli | NOT RUN |
| Régression CPU/GPU/Auto ou diagnostic natif | Tests disponibles et scénarios natifs explicitement autorisés ; limites signalées | NOT RUN |
| Rupture de chemin Rust, visibilité ou filtre `--exact` | `cargo test --list`, compilation des consommateurs, noms qualifiés comparés | NOT RUN |
| Campagne hors racines autorisées | Préflight, garde applicable et auto-tests avant tout Cargo | PRÉREQUIS OUVERT |

Le dépôt possède des gardes propres à d'autres lots ; leur compatibilité avec ce nouveau lot n'est pas présumée. La future campagne exige une racine de lot et une garde existante applicable ou une adaptation **explicitement autorisée** ; aucun Cargo, même `metadata`, avant ce préflight. `CARGO_TARGET_DIR` doit rester `target/parakeet-tests/cargo-shared/`, `CARGO_HOME`/`TEMP`/`TMP` dans la racine du lot, avec les autres règles permanentes d'`AGENTS.md`. Aucun outil de surveillance nouveau n'est déduit de ce plan.

**Statut : DESIGN PROPOSÉ.** La cohérence du découpage est fondée sur une lecture statique ; aucune compilation, test ou recette n'a été réalisée. L'acceptation du seuil et des chemins avec le plan est requise avant l'implémentation. Aucune revue indépendante de ce document n'est annoncée.
