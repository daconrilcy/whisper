Avancée : plan revu CLEAN puis validé explicitement par l'utilisateur dans cette session.
Blocage : aucun au lancement ; cause temporelle encore à établir.
Décision : implémenter le diagnostic interne, puis la correction conditionnelle fondée sur ses traces.

# DIAG-CORR-PREPARE — plan accepté le 4 novembre 2026

## Objectif et autorisations

La demande courante autorise la préparation du diagnostic puis d'une correction
et de ses commits locaux. Elle ne transforme pas les six essais consommés du
lot DIAG-BANC-15S en quota illimité. Ce plan renouvelle explicitement le cadre
nécessaire selon AGENTS.md ; aucun Rust produit n'est modifié avant son acceptation.
Acceptation : l'utilisateur a répondu « je valide » au plan ffed275d, y compris
les chemins, six nouveaux Auto, deux cycles et la reprise encadrée après erreurs
de compilation/test dues au delta. Début du lot : 17:53:47 locale ; espace libre
initial 513 278 816 256 octets, worktree propre. L'état de processus relevé ne
montre aucune commande Cargo/rustc/Git ni VCTIP encore active à cette lecture.
Reprise sur `C:/wt/diag-banc`, branche `diag-banc-15s`, base `ff50f7cb`, propre.
Le bilan précédent reste figé ; ce document porte la suite, pas une réécriture.
La racine target/n2-native est réutilisée et devient une entrée du lot actif :
les anciennes commandes de nettoyage du lot clos ne doivent pas être exécutées
pendant cette suite. Les chemins encore actifs seront exclus de toute proposition
de rétention jusqu'à la nouvelle clôture.

But produit : éliminer un travail de préparation inutile ou une attente prouvée
sur l'import local, sans affaiblir l'intégrité des modèles, le routage Auto/CPU/GPU,
la publication, la transcription ni la disponibilité du premier lancement.
Les mesures de l'application normale sont un témoignage utilisateur distinct
du banc debug ; aucun seuil de latence arbitraire n'en est déduit.

## Enveloppe proposée

- 90 minutes de travail actif pour ce lot après GO ; au plus **six nouveaux
  corps Auto N2 au total**, diagnostic et après-correction compris. Aucun autre
  scénario natif, aucune modification de garde, profil, machine, manifeste,
  Cargo.lock, dépendance, worker ou DLL.
- Deux cycles revue/correction maximum par livrable. Après cette limite,
  LIVRÉ AVEC RÉSERVES et décision utilisateur ; « jusqu'à PASS » ne supprime
  pas cette règle. Ne pas multiplier les livrables pour remettre le compteur à zéro.
- N0/N1 autorisés dans le lot ; N2 via le seul profil PersonalAutoFallback,
  ressources et en-short.mp3 épinglés identiques, Auto sans candidat -> CPU.
  Garder staging neuf, aucun changement de priorité ni exclusion Defender.
- Commandes strictement séquentielles. Tous les nouveaux lots N1 sont sous
  `target/n2-native/diag-corr-prepare/n1/` afin que leurs éventuels enfants
  retardés restent dans la racine N2 déjà autorisée par la garde. Cache N1
  persistant et cible Cargo partagée inchangés ; pas de nouvelle surveillance.
- L'arrêt de VCTIP 82512 autorisé auparavant n'autorise aucun autre arrêt de
  processus. Les processus résiduels sont relevés sans attribution par coïncidence.
- Une erreur de compilation ou de test causée par le delta pourra être corrigée
  et vérifiée dans les deux cycles si l'utilisateur accepte aussi cette règle de
  reprise. Une violation de racine, d'intégrité ou de garde reste un arrêt avec
  arbitrage spécifique, sans contournement ni suppression de preuves.

## Chemins autorisés proposés

Diagnostic uniquement, par `rust_implementer` :

1. `crates/whisper-adapters/src/lib.rs` : déclarer le module de diagnostic natif.
2. `crates/whisper-adapters/src/native_timing.rs` (nouveau) : traces structurées
   opt-in, appels produit sous `native-tests` seulement ; absent du produit par
   défaut, module déclaré sous `cfg(any(feature = "native-tests", test))` et
   appels produit sous `cfg(feature = "native-tests")`.
3. `crates/whisper-adapters/src/native_timing/tests.rs` (nouveau si nécessaire) :
   tests unitaires séparés des fonctions pures de sérialisation/activation/racines,
   sans changer l'environnement global des tests ni modifier une politique produit.
   Ces tests helpers sont effectivement exécutés en N1, sans activation native.
4. `crates/whisper-adapters/src/worker_ipc/runtime/import.rs` : marqueurs autour
   de prepare_effect, bind_import_request, prepare et émission de Prepared.
5. `crates/whisper-adapters/src/model_catalog.rs` : chronométrer attente du cache,
   lecture/calcul effectifs et réutilisation, avec identité du modèle et nombre
   d'octets, sans changer le comportement à ce stade.

Correction conditionnelle, **seulement si les traces la justifient** :

6. `crates/whisper-adapters/src/worker_ipc/backends/mod.rs` et
   `crates/whisper-adapters/src/model_catalog.rs` : borner l'inspection au modèle
   demandé et conserver les contrôles de ce modèle. Le code actuel appelle
   snapshot/inspect sur tous les modèles avant de traiter une requête Parakeet ;
   c'est une hypothèse de travail, pas une cause temporelle déjà prouvée.
7. `crates/whisper-adapters/src/worker_ipc/backends/catalog_tests.rs` (nouveau,
   et déclaration locale dans backends/mod.rs) et/ou
   `crates/whisper-adapters/src/model_catalog/tests.rs` (nouveau, déclaration
   dans model_catalog.rs) pour les témoins de non-régression ciblés.

La session principale écrit ce plan et le bilan de suite
`docs/architecture/DIAG-CORR-PREPARE-BILAN.md`, puis committe localement.
Aucune correction d'une autre responsabilité, invalidation générale de cache,
réduction de SHA-256 ou réécriture de synchronisation n'est autorisée par cette
branche conditionnelle : présenter alors la correction concrète nécessaire.

## Huit tâches et preuves attendues

1. **Préflight N0 et GO.** Accepter le présent plan, ses chemins et le nouveau
   quota ; relever Git, espace libre >=30 Gio, processus et racines. Aucun effacement
   des preuves ou ressources du lot précédent. Vérifier les autorisations des profils.
2. **Diagnostic interne opt-in.** Déléguer l'implémentation à rust_implementer.
   Réutiliser WHISPER_DIAG_BANC_TRACE=1 et WHISPER_L04_CAMPAIGN_RUN, déjà transmis
   par le profil. Traces JSONL séparées du banc, temps monotones/durées, processus,
   requête/génération lorsqu'ils sont disponibles. Pas de lien causal inventé entre
   deux origines d'horloge : corrélation par identités et durées mesurées.
   Sans repère commun, l'attente de file reste non déterminée ; les durées
   internes ne prouvent pas à elles seules le temps passé avant leur entrée.
   Aucun effet sur les assertions, l'ordonnancement ou les données ; aucune trace
   ni accès au disque supplémentaire dans le produit livré par défaut.
3. **Vérification et revue.** Tests unitaires utiles, format/build/Clippy sous
   garde N1 ; compilation native-tests sans exécution par l'entrée existante.
   Confier le candidat à rust_review_coordinator et son reviewer indépendant.
   Les corrections restent à l'implémenteur. Commit local du candidat vérifié.
4. **Mesures avant correction.** Un premier Auto N2 instrumenté, puis un second
   uniquement s'il faut départager les hypothèses ; conserver au moins quatre
   essais du quota pour l'après-correction. Comparer l'attente de file/service,
   bind/catalogue/cache, préparation d'archive, réception de Prepared, moteur et
   fin. Ne pas attribuer l'ancien mode lent si aucun lent n'est observé.
5. **Décision fondée sur les traces.** Si l'inspection de modèles non demandés
   constitue le coût dominant, produire le correctif borné des chemins 6/7.
   Sinon, exposer l'autre cause prouvée et son correctif précis avant tout élargissement.
   Si aucune cause n'est établie, arrêter sans correction spéculative.
6. **Correction et non-régression.** Implémenteur, tests séparés, revue indépendante
   et commits locaux. Pour l'hypothèse catalogue : un import Parakeet ne lit plus
   intégralement Turbo, tandis que le modèle demandé conserve taille, SHA-256,
   identité de fichier et revalidation lors d'un changement. Le catalogue complet
   utilisé pour afficher les modèles garde sa sémantique. CPU/GPU/Auto et les
   requêtes inconnues, invalides ou corrompues conservent leurs refus attendus.
7. **Après correction.** N1 complète utile avec inventaire explicite des nouveaux
   tests, compilation native et jusqu'à quatre Auto N2 dans le quota restant.
   Garder les durées du corps, de Cargo et de la garde séparées. Tests de comportement
   et sorties TXT/SRT/WAV inchangés pour le même en-short ; diagnostic avant/après
   prouvant la disparition du travail inutile identifié. Pas de moyenne globale
   des modes ni d'affirmation de correction des 15 s sans preuve correspondante.
8. **Clôture.** Revue indépendante des conclusions, archive des petites traces
   vérifiée (nombre, tailles, SHA-256), bilan de suite figé, commits locaux.
   Nettoyage proposé à l'utilisateur, pas exécuté par l'agent ; caches partagés,
   persistants, modèles, anciennes preuves et target/parakeet-feasibility préservés.
   Aucun push ni fusion dans main sans accord précis.

## Définition de PASS proposée

PASS du correctif = cause identifiée par traces internes ; changement minimal
retirant le travail/attente prouvé ; tests fonctionnels et d'intégrité utiles,
format/build/lints applicables réussis ; revue indépendante CLEAN ; bénéfice
avant/après observé sur la phase visée sans changement de transcription.

Un PASS N1 seul ne vaut pas résolution du délai. Si le coût fixe est corrigé
mais que le mode lent n'a pas été reproduit et expliqué, la conclusion globale
sur les 15 s reste NON DÉTERMINÉE / LIVRÉ AVEC RÉSERVES, même avec tous les tests verts.

## Exécution — diagnostic interne

Instrumentation dans les cinq chemins prévus, aucune optimisation à ce stade.
Première N1 arrêtée à la compilation E0382 (réemprunt du writer JSONL manquant).
Le reviewer a demandé DC1 (compilation) et DC2 (exclure le coût du marqueur de
l'attente du mutex). Correction cycle 1 par l'implémenteur, sans élargissement.
Reprise couverte par le GO du présent plan ; aucune violation de garde.

N1 corrigée : 708 PASS /128 ignorés /0 échec, soit les 705 précédents et trois
tests native_timing::tests (activation exacte, chemin raciné, JSONL monotone).
Format, build adapters et Clippy/all-targets PASS ; avertissements conservés,
ceux du module privé de qualification en cfg(test) jugés non actionnables par
le reviewer. PreparationNoRun-0e0a074b11a94028ab9fb3a862d8e924 : NO_RUN_PASS,
aucun corps natif. Revue diagnostic interne CLEAN : DC1/DC2 fermés, deux revues
et un cycle de correction, vérifications VDC1/VDC2 validées. Toute trace absente,
incomplète ou d'origine d'horloge incompatible sera rejetée pour l'analyse.
Native SelfTest précédent réutilisé : garde et profil inchangés ; SelfTest N1
du nouveau lot réussi (35 positifs/79 négatifs).

### Mesures avant correction et décision conditionnelle

Instrumentation commise dans 6b8c4000. Deux Auto PASS, cache froid par processus :
74ddac11dba640a593ff449f53daef63, corps 24,98 s ;
611963c27ca644c8a7a6d5a5aa22c360, corps 25,01 s. Quota consommé : 2/6.
Les deux traces produit sont complètes, une seule origine d'horloge chacune,
avec requête Parakeet et génération corrélées au banc.

| Durées internes, secondes | 74ddac11 | 611963c2 |
|---|---:|---:|
| Premier bind_import_request | 23,6935 | 23,6519 |
| Lecture/hachage Turbo non demandé, 1 624 555 275 octets | 16,4424 | 16,5009 |
| Lecture/hachage Parakeet demandé, 713 975 456 octets | 7,2500 | 7,1499 |
| prepare après bind | 0,0927 | environ 0,093 |

Attente du cache maximale : 0,0008 /0,0009 ms ; identité stable pour les deux
lectures. Le second bind après Prepared est un hit cache, environ 0,49 /0,44 ms.
Ce sont des durées internes : aucune soustraction entre l'horloge du banc et
celle du produit. Lecture/hachage ne sépare pas disque et calcul SHA-256.

La preuve du coût dominant sur ces deux essais est relue indépendamment CLEAN :
snapshot inspecte tous les modèles avant de résoudre la requête Parakeet.
La correction conditionnelle du plan est donc déclenchée : limiter l'inspection
au modèle demandé, conserver ses contrôles et le catalogue complet d'affichage.
Aucune cause des 15 s bimodales n'est prouvée ; aucun mode lent reproduit.
Quatre essais après correction restent autorisés ; pas d'autres essais avant.

### Correctif conditionnel vérifié avant essais

Delta limité à model_catalog.rs, worker_ipc/backends/mod.rs et nouveau
model_catalog/tests.rs : résolution de l'identité demandée puis inspection d'une
seule entrée pour bind_import. Catalogue complet inspect()/snapshot et bind_live
inchangés ; mêmes contrôles de taille/SHA-256/identité/revalidation de l'entrée.

N1 : 712 PASS /128 ignorés /0 échec (708 + quatre tests ciblés). Témoins :
Parakeet seul inspecté et catalogue complet inspectant les deux modèles ; alias
Turbo historique et chemins inconnus ; modèle demandé manquant ; changement de
taille forçant un rehash et restant Corrupt. Format/build/Clippy PASS, sans
correction automatique des avertissements existants. PreparationNoRun-
e8ba4af9544e44b1a924b4937c9f5d34 : NO_RUN_PASS. Revue performance CLEAN,
aucun constat ouvert ; compteur cumulé trois lectures de revue et un cycle de
correction technique (DC1/DC2). Le gain reste à vérifier sur les quatre essais
Auto restants ; aucun essai natif d'un autre scénario n'est autorisé.
