Avancée : réserves GPU et sélecteurs existants relevés en lecture seule le 2 novembre 2026.
Blocage : aucune campagne autorisée par la seule rédaction du plan ; prérequis à vérifier.
Décision : proposer une qualification Parakeet GPU bornée à l'hôte Z790, sous garde N2.

# Compléter la qualification GPU

## Base et portée de la qualification

Base Git observée : `main`, `92f9a2f7`. Les correctifs du message de démarrage
(`1b76a7ea`) et du parent `settings` (`92f9a2f7`) sont commités, vérifiés et revus
CLEAN ; aucun nouveau paquet n'a encore été livré. Les non-suivis `models/`
et `sound example/` sont conservés.

Preuves de départ : `docs/cadrage/GARDE-NVML.md` et
`docs/architecture/raw/garde-nvml/results.json` attestent un succès de
`PersonalUnifiedGpuOnly`, import MP3 anglais Parakeet avec candidat GPU.
Le parent `settings` était alors préparé par la garde. Les 43 corps ciblés
historiques ne constituent pas 43 inférences GPU de bout en bout.

`ci/native-profiles.json:184` conserve `ParakeetGpu` en `DISCOVERY_PENDING` :
aucune ressource, valeur ou sélection exacte n'y figure. Il s'agit de l'état
du profil de campagne, pas d'une preuve que le GPU est inutilisable.
Le lot vise à rendre ce profil exécutable de façon vérifiable, puis à élargir
les essais Parakeet GPU manuels et Auto sur les corpus disponibles.
Les essais Turbo déjà consignés restent distincts ; aucun succès nouveau
Turbo, micro ou interface Windows ne sera déduit des résultats Parakeet import.

La qualification portera exclusivement sur le GPU NVIDIA réellement identifié
sur Z790, ses workers/runtimes/modèles et versions de pilote observés. Elle ne
vaut pas couverture d'autres cartes ou générations CUDA. Aucun nouveau
diagnostic NVML par minimisation d'environnement : la cause `ProgramW6432`
est déjà établie et sa validation système existe dans la garde.

## Critères d'acceptation

| ID | Critère |
| --- | --- |
| AC1 | `ParakeetGpu` ne devient READY qu'avec ressources existantes vérifiées, variables/écritures bornées, sélecteurs réellement listés et contrat de garde relu CLEAN. READY signifie exécutable ; les résultats d'exécution restent séparés. |
| AC2 | Chaque succès GPU inclut le candidat actuel et une preuve du backend CUDA effectivement utilisé, associée à l'exécution/identité ; un simple inventaire, une DLL chargée ou un code Cargo 0 ne suffit pas. |
| AC3 | Le cas court GPU réussit trois fois dans des RunRoot distincts, avec sorties publiées et sources intactes. La répétition reste une mesure sur un seul hôte. |
| AC4 | Les imports longs français et anglais réussissent en GPU manuel et en Auto avec GPU effectif. Une réussite Auto sur CPU est classée repli CPU, sans qualification du cas GPU attendu. |
| AC5 | Vérifier les assertions existantes de publication, TXT/SRT/WAV non vides, intégrité source et cohérence temporelle ; contrôler les annotations réellement disponibles. Aucun seuil ou assertion historique n'est abaissé. |
| AC6 | Relever durée audio, temps d'import et ratio temps traitement/durée lorsque mesurables. Le critère produit GPU plus rapide que l'audio exige un ratio inférieur à 1 ; une mesure absente ou plus lente laisse la réserve de vitesse ouverte. La qualité est rapportée selon les contrôles et références disponibles, sans déduire de WER ni de comparaison CPU non mesurées. |
| AC7 | Aucun changement installé, pilote, SDK, service, ACL, réseau ou modèle ; sorties exclusivement sous les racines N2 autorisées et cible Cargo partagée. Garde et ses contrôles ne sont pas affaiblis. |
| AC8 | Bilan détaillant exécuté/PASS/FAIL/BLOQUÉ, nombre d'inférences et répétitions, GPU réellement utilisé et réserves ; FR87/FR27 restent explicitement non qualifiés. |

## Matrice proposée à figer après inventaire

Tous les sélecteurs ci-dessous existent dans
`crates/whisper-adapters/tests/parakeet_unified_native.rs` ; leur présence dans
le harnais compilé doit être vérifiée sous garde avant utilisation.

| Cas | Sélecteur exact | Moteur attendu | Limite du corps existante |
| --- | --- | --- | --- |
| G1, trois RunRoot | `native_personal_gpu_imports_english_mp3_with_attested_candidate` | Parakeet CUDA, GPU manuel | 15 min par exécution |
| G2 | `native_personal_gpu_imports_annotated_french_long_audio` | Parakeet CUDA, GPU manuel | 90 min |
| G3 | `native_personal_gpu_imports_annotated_english_long_audio` | Parakeet CUDA, GPU manuel | 90 min |
| G4 | `native_personal_auto_imports_annotated_french_long_audio` | Auto, Parakeet CUDA réellement attesté | 90 min |
| G5 | `native_personal_auto_imports_annotated_english_long_audio` | Auto, Parakeet CUDA réellement attesté | 90 min |

Sept exécutions prévues, dont quatre longues. La borne théorique des seuls
corps atteint 6 h 45 ; préparation/compilation s'ajoutent. La garde doit
accepter explicitement ces délais sans laisser un enfant survivre au parent.
Les expirations de la garde restent appliquées ; s'il faut en modifier le
contrat, le préciser et faire approuver cette modification avant campagne.

Attention : `run_native_import` accepte actuellement en Auto soit CUDA, soit
le repli CPU avant audio. Il faut donc collecter l'indication du backend et
son attestation dans les preuves de chaque cas. Si les sorties existantes ne
permettent pas cette attribution, arrêter les cas concernés pour arbitrage
d'un ajout de preuve ciblé ; aucun test n'est assoupli ni code Rust modifié
dans le présent périmètre.

Les cas historiques `gpu_app_fr_en_single_pass`, `gpu_app_en5` et les recettes
L3/L6 ne remplacent pas cette matrice : certains exigent FR87/FR27 ou des
racines/features historiques. Ils restent hors de ce premier lot. Les
ressources longues annotées déjà utilisées par `PersonalLongCpu` doivent être
revérifiées ; leur existence passée n'est pas une preuve de disponibilité actuelle.

## Huit tâches après acceptation du plan et GO d'exécution

1. **Inventaire, session principale et contributions en lecture seule.**
   Relever état Git, espace libre (minimum 30 Gio, plus place du staging),
   GPU/pilote/outils, chemins des workers et 21 ressources personnelles,
   modèles, MP3 anglais court, WAV longs FR/EN, annotations et clips référencés.
   Retrouver les copies préservées sous le dépôt après retrait des worktrees ;
   ne pas réutiliser aveuglément les anciens chemins `.wt/r8` ou `.wt/nv`.
   Référencer les identités/hashes de livraison déjà exigés, constater toute
   dérive. Préparer les requêtes exactes sans écrire hors dépôt. Aucune capture
   micro, installation, téléchargement ni activation système.
2. **Contrat de campagne.** Définir dans `ci/native-profiles.json` le profil
   `ParakeetGpu` sur les seuls cas de la matrice dont les prérequis sont prêts.
   Réutiliser le staging personnel et les descripteurs des corpus longs.
   Si nécessaire, séparer en deux profils bornés court/long pour éviter les
   reprises globales. Adapter `ci/guard-native.ps1` uniquement pour reconnaître
   ces profils dans les listes existantes : `ProgramW6432` réel validé registre,
   `CUDA_PATH` déclaré, variables de corpus, limites et staging déjà autorisés.
   Ne pas étendre globalement les variables, les racines ou l'accès aux outils.
   La copie stagée ne contient pas `settings` précréé pour ces nouveaux profils :
   le produit corrigé doit en assurer la création. Ne pas retirer le
   contournement historique de `PersonalUnifiedGpuOnly` dans ce lot.
3. **Auto-tests et revue de la garde.** Déléguer l'implémentation de ces deux
   fichiers à un worker, avec propriété exclusive ; revue indépendante en
   lecture seule avant tout appel natif. Valider arguments exacts, variables,
   `ProgramW6432`, racines, surveillance, refus, délais, staging et absence de
   préparation implicite de `settings`. Deux cycles de correction maximum.
   Les auto-tests existants sont utilisés, sans création d'environnement Windows.
4. **Préparation sous garde.** Utiliser la garde N2 pour les compilations
   nécessaires `--offline --locked`, l'inventaire réel des tests et les
   préflights de chaque requête. Garder `native-tests` limité aux paquets
   existants nécessaires ; ne pas ajouter de feature de recette historique.
   Les harnais de test utilisent la cible partagée existante. Aucun binaire
   de harnais compilé en workspace ne remplace un worker de livraison : les
   workers qualifiés restent identifiés et contrôlés PE/SHA séparément.
5. **Campagne courte.** Exécuter G1 trois fois, isolément et séquentiellement,
   sans parallélisme de copies ni mutation des entrées. Capturer statut,
   journaux worker, candidat/attestation, moteur effectif, sorties et durées.
   Vérifier la création de `settings/logs` par le produit dans le staging neuf.
   Si G1 ne prouve pas le GPU effectif ou échoue, ne pas lancer les longs.
6. **Campagne longue.** Exécuter G2 à G5 séquentiellement, chacun avec son
   RunRoot et sa requête figée, sources/annotations vérifiées. Réutiliser
   les assertions existantes. Distinguer succès CUDA, succès par repli CPU,
   erreur, expiration et non exécuté ; ne pas requalifier une trace ancienne
   comme résultat du candidat courant.
7. **Analyse et revue indépendante.** Réconcilier les tests réellement
   exécutés et les sorties avec la matrice ; faire vérifier l'attribution
   CUDA, la publication, les temps et les limites de qualité. Un reviewer
   vérifie les preuves et distingue CLEAN du code de garde, PASS des recettes
   et qualification produit. Les échecs ne sont pas effacés du bilan.
8. **Clôture, session principale.** Compléter ce document, le bilan de fusion
   et une petite synthèse de résultats sous `docs/architecture/raw/qualification-gpu/`.
   Archiver seulement les petites preuves demandées par la qualification,
   avec nombre/tailles/SHA vérifiés ; garder modèles, corpus et sorties lourdes
   sous le lot. Relever l'espace libre, commiter localement. Aucun push.

## Racines, fichiers et responsables

Racine proposée : `target/n2-native/qualification-gpu/` ; chaque exécution
dispose de ses TEMP/TMP/LOCALAPPDATA/CARGO_HOME propres, vérifiés par la garde.
Cible Cargo : `target/parakeet-tests/cargo-shared/`, conservée. Si le contrat
actuel de la garde ne permet pas ces racines ou les entrées retrouvées,
arrêter avant exécution et soumettre les chemins canoniques nécessaires.

Fichiers suivis modifiables après accord : `ci/native-profiles.json`,
`ci/guard-native.ps1`, ce plan, `docs/architecture/BILAN-FUSION-PILE.md`,
`docs/architecture/raw/qualification-gpu/results.json` et une archive des
petites preuves. Six fichiers suivis prévus, huit tâches. Requêtes et journaux
d'exécution restent sous la racine ignorée du lot.

La session principale écrit les documents et effectue les commits. Le worker
est seul auteur des corrections de scripts ; le reviewer reste indépendant.
Aucun Rust, test, manifeste, lockfile, `.cargo/`, garde N1/historique, modèle,
audio original ou ressource installée modifié dans ce lot. Un besoin de
nouveau test, sélecteur ou trace Rust provoque un arbitrage de périmètre.

## Arrêts et limites de fin

Appliquer AGENTS : violation du préflight, surveillance, compilation ou test
échoué entraîne arrêt, conservation des traces, diagnostic en lecture seule
et arbitrage avant reprise. Les exceptions des lots précédents ne sont pas
reconduites. Aucun pilote/SDK installé, élévation, service, isolation Windows,
compte, ACL, effacement ni réparation de l'installation autorisés.

Le lot est qualifié pour sa matrice uniquement si chaque cas GPU prévu a des
preuves attribuables et les critères AC1-8 sont satisfaits. Sinon livrer le
bilan partiel avec les cases non qualifiées et la décision nécessaire.
FR87/FR27 perdus, autres GPU, Turbo hors mesures historiques, micro GPU,
premier texte affiché en moins de 15 secondes, interface Windows et paquet
distribué restent explicitement hors qualification de cette campagne import.
Une validation visuelle ultérieure par l'utilisateur et la remise d'un nouveau
paquet incluant les correctifs récents nécessitent un lot distinct.
