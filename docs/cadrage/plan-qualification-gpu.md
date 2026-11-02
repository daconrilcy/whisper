Avancée : corpus VoxPopuli préparé le 2 novembre 2026 ; 14 fichiers générés et contrôlés.
Blocage : textes non vérifiés humainement ; recettes Rust/N2 et qualification GPU non exécutées.
Décision : son des courts validé par l'utilisateur ; longs prêts pour essais techniques, qualité linguistique à distinguer.

# Compléter la qualification GPU

## Base et portée de la qualification

Base Git observée : `main`, `92f9a2f7`. Les correctifs du message de démarrage
(`1b76a7ea`) et du parent `settings` (`92f9a2f7`) sont commités, vérifiés et revus
CLEAN ; aucun nouveau paquet n'a encore été livré. Les non-suivis `models/`
et `sound example/` sont conservés.
Révision corpus demandée par l'utilisateur : utiliser les modules Python déjà
présents sous `sound example/voxpopuli/` pour préparer les fichiers nécessaires.
Les anciennes fixtures françaises disparues ne sont plus des prérequis,
des cas à restaurer ni des blocages de ce plan. Les bilans historiques restent
des constats de leurs campagnes respectives.

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
| AC3 | Le cas court EN GPU réussit trois fois dans des RunRoot distincts ; le cas court FR réussit aussi. Sorties publiées et sources intactes. La répétition reste une mesure sur un seul hôte. |
| AC4 | Les imports longs français et anglais réussissent en GPU manuel et en Auto avec GPU effectif. Une réussite Auto sur CPU est classée repli CPU, sans qualification du cas GPU attendu. |
| AC5 | Vérifier les assertions existantes de publication, TXT/SRT/WAV non vides, intégrité source et cohérence temporelle ; contrôler les annotations réellement disponibles. Aucun seuil ou assertion historique n'est abaissé. |
| AC6 | Relever durée audio, temps d'import et ratio temps traitement/durée lorsque mesurables. Le critère produit GPU plus rapide que l'audio exige un ratio inférieur à 1 ; une mesure absente ou plus lente laisse la réserve de vitesse ouverte. La qualité est rapportée selon les contrôles et références disponibles, sans déduire de WER ni de comparaison CPU non mesurées. |
| AC7 | Aucun changement installé, pilote, SDK, service, ACL, réseau ou modèle ; sorties exclusivement sous les racines N2 autorisées et cible Cargo partagée. Garde et ses contrôles ne sont pas affaiblis. |
| AC8 | Bilan détaillant exécuté/PASS/FAIL/BLOQUÉ, nombre d'inférences et répétitions, GPU réellement utilisé et réserves ; références et limites des nouveaux corpus clairement identifiées. |

## Fichiers de recette à créer ou finaliser

Sources locales constatées, sans exécution ni téléchargement :

- `sound example/voxpopuli/voxpopuli_60s.py` sélectionne des extraits originaux
  et des textes candidats depuis les annotations ;
- `sound example/voxpopuli/scan_voxpopuli_60s.py` utilise FFmpeg/ffprobe pour
  extraire du PCM mono 16 kHz et examiner les silences ;
- `clips/raw_audios/original_2020.tar` existe, environ 5,26 Go ; les annotations
  `clips/annotations/asr_fr.tsv.gz` et `asr_en.tsv.gz` existent ;
- `clips/long_examples/{fr,en}_continuous60_2020_CANDIDATE.wav`, leurs JSON/TXT
  et les fichiers `REFERENCE.txt` existent. Les JSON indiquent encore
  `CANDIDATE_NOT_HUMAN_VALIDATED` ; les noms REFERENCE ne prouvent pas à eux
  seuls une validation. Les fichiers `{fr,en}_long_2020.wav` sont des extraits
  d'environ une minute, pas les sessions de 35/40 minutes nécessaires.

Destination proposée des nouvelles recettes :
`target/n2-native/qualification-gpu/corpus/`. Préserver intégralement scripts,
archive, annotations et candidats d'origine. Les fichiers suivants sont des
données de test locales, pas des sources à ajouter à Git :

| Base du fichier | Audio à créer/finaliser | Référence et provenance à créer | Besoin |
| --- | --- | --- | --- |
| `fr-short-60s` | `.wav` PCM signé 16 bits, mono 16 kHz, 60 s ; `.mp3` dérivé du même extrait | `.reference.txt` et `.annotations.json` | court FR GPU manuel, WAV et MP3 |
| `en-short-60s` | `.wav` même format et durée ; `.mp3` dérivé | `.reference.txt` et `.annotations.json` | court EN GPU manuel, MP3 répété trois fois |
| `fr-long-35m` | `.wav`, même PCM, au moins 35 minutes de contenu distinct | `.reference.txt` et `.annotations.json` | long FR manuel et Auto |
| `en-long-40m` | `.wav`, même PCM, au moins 40 minutes de contenu distinct | `.reference.txt` et `.annotations.json` | long EN manuel et Auto |

Soit six fichiers audio et huit fichiers de référence/provenance. Les deux
couples courts peuvent être finalisés à partir des candidats de 60 s présents,
après écoute/correction du texte et vérification des limites des mots. Le MP3
est un dérivé pour tester le décodage, pas un second contenu indépendant.
Conserver sa durée décodée réelle : l'encodage peut introduire du délai ou du
padding ; l'égalité binaire ou exacte du nombre d'échantillons avec le WAV
n'est pas un critère adapté au MP3.

Pour les longs, sélectionner dans l'archive locale des passages français et
anglais associés aux annotations. Utiliser une prise originale continue si
elle fournit les annotations nécessaires ; sinon assembler des passages
distincts en documentant chaque raccord et chaque intervalle source. Aucun
bouclage d'un extrait court pour atteindre la durée, aucune transcription
générée par le moteur testé utilisée comme vérité terrain. Le module actuel
de recherche de 60 s n'est pas annoncé comme un générateur de sessions de
35/40 minutes : préparer les sélections et commandes FFmpeg de façon explicite
avec les fonctions existantes, sans nouveau programme persistant hors plan.
Si l'archive locale ne couvre pas ce besoin, consigner le manque et demander
un accord précis avant tout téléchargement ; ne pas changer silencieusement
la durée ou la qualité exigée.

Chaque `.annotations.json` utilise un schéma VoxPopuli dédié, à fixer avant
les tests : langue, échantillonnage/durée/nombre d'échantillons, SHA du WAV et
des dérivés, archive/session/locuteur, identifiants de lignes, intervalles
originaux, intervalles dans le WAV, références textuelles et état de validation.
Les intervalles couvrent la totalité du WAV, y compris silences/raccords
explicitement déclarés ; chaque chemin de provenance est relatif au dépôt.
Les textes des métadonnées ne sont pas déclarés vérifiés à l'écoute tant que
cette vérification n'a pas été faite. Pour les longs non revus intégralement,
le bilan peut qualifier exécution/endurance/exports tout en laissant la
qualité linguistique non qualifiée. WER/CER restent indicatifs tant que la
référence et la normalisation n'ont pas été validées.

## Matrice proposée à figer après inventaire

Les sélecteurs suivants sont proposés dans un nouveau sous-module de tests
VoxPopuli de `parakeet_unified_native` ; ils n'existent pas encore. Leur présence
et leurs noms exacts devront être listés sous garde avant passage du profil à READY.

| Cas | Sélecteur proposé | Moteur attendu | Borne proposée |
| --- | --- | --- | --- |
| G1, trois RunRoot | `voxpopuli::english_short_mp3_gpu` | Parakeet CUDA, GPU manuel | 15 min par exécution |
| G1-FR | `voxpopuli::french_short_wav_and_mp3_gpu` | Parakeet CUDA, GPU manuel, deux imports | 15 min par import |
| G2 | `voxpopuli::french_long_gpu` | Parakeet CUDA, GPU manuel | 90 min |
| G3 | `voxpopuli::english_long_gpu` | Parakeet CUDA, GPU manuel | 90 min |
| G4 | `voxpopuli::french_long_auto_gpu` | Auto, Parakeet CUDA réellement attesté | 90 min |
| G5 | `voxpopuli::english_long_auto_gpu` | Auto, Parakeet CUDA réellement attesté | 90 min |

Huit exécutions de test prévues, neuf imports au total, dont quatre longs.
La borne théorique des imports atteint 7 h 15 ; préparation, écoute et
compilation s'ajoutent. La garde doit
accepter explicitement ces délais sans laisser un enfant survivre au parent.
Les expirations de la garde restent appliquées ; s'il faut en modifier le
contrat, le préciser et faire approuver cette modification avant campagne.

Attention : `run_native_import` accepte actuellement en Auto soit CUDA, soit
le repli CPU avant audio. Il faut donc collecter l'indication du backend et
son attestation dans les preuves de chaque cas. Si les sorties existantes ne
permettent pas cette attribution, arrêter les cas concernés pour arbitrage
d'un ajout de preuve ciblé. Les nouveaux tests VoxPopuli exigent explicitement
CUDA pour les cas Auto de cette matrice ; les assertions des tests existants
ne sont pas assouplies et le code Rust de production reste inchangé.

Les tests longs existants `run_native_long_case` valident spécifiquement le
corpus SPS et ses TSV à 19 colonnes. Un WAV VoxPopuli renommé ne satisfait pas
ce contrat. Les nouvelles recettes valident leur provenance VoxPopuli dans
un module séparé et réutilisent les contrôles communs d'import/publication
sans supprimer ceux des recettes SPS ni mélanger leurs schémas.

## Huit tâches après acceptation du plan et GO d'exécution

1. **Inventaire, session principale et contributions en lecture seule.**
   Relever état Git, espace libre (minimum 30 Gio, plus place du staging),
   GPU/pilote/outils, chemins des workers et 21 ressources personnelles,
   modèles, modules Python/environnement existant, FFmpeg/ffprobe et sources
   VoxPopuli locales. Préparer les six audios et huit références/provenances
   listés ci-dessus, puis valider leur statut et leurs chemins avant exécution.
   Usage des modules Python et FFmpeg locaux pour la recette conformément à
   la demande de l'utilisateur ; aucune installation de paquet ou outil.
   Retrouver les copies préservées sous le dépôt après retrait des worktrees ;
   ne pas réutiliser aveuglément les anciens chemins `.wt/r8` ou `.wt/nv`.
   Référencer les identités/hashes de livraison déjà exigés, constater toute
   dérive. Préparer les requêtes exactes sans écrire hors dépôt. Aucune capture
   micro, installation, téléchargement ni activation système.
2. **Contrat de campagne.** Définir dans `ci/native-profiles.json` le profil
   `ParakeetGpu` sur les seuls cas de la matrice dont les prérequis sont prêts.
   Réutiliser le staging personnel ; déclarer les entrées VoxPopuli et leurs
   annotations dédiées, sans leur attribuer la provenance SPS existante.
   Si nécessaire, séparer en deux profils bornés court/long pour éviter les
   reprises globales. Adapter `ci/guard-native.ps1` uniquement pour reconnaître
   ces profils dans les listes existantes : `ProgramW6432` réel validé registre,
   `CUDA_PATH` déclaré, variables de corpus, limites et staging déjà autorisés.
   Ne pas étendre globalement les variables, les racines ou l'accès aux outils.
   La copie stagée ne contient pas `settings` précréé pour ces nouveaux profils :
   le produit corrigé doit en assurer la création. Ne pas retirer le
   contournement historique de `PersonalUnifiedGpuOnly` dans ce lot.
3. **Recettes, auto-tests et revue.** Déléguer les nouveaux tests Rust à
   `rust_implementer` : déclaration du sous-module et tests VoxPopuli séparés,
   prévalidation des annotations/échantillons/références, preuve du moteur
   effectif et contrôles des exports. Aucun moteur en production modifié.
   Attribuer les deux fichiers CI à un auteur exclusif, sans écriture concurrente
   sur les mêmes fichiers. Revue indépendante via `rust_review_coordinator`
   pour le delta Rust et revue du contrat N2 avant tout appel natif.
   Valider arguments exacts, variables,
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
5. **Campagne courte.** Exécuter G1 trois fois et G1-FR une fois, séquentiellement,
   sans parallélisme de copies ni mutation des entrées. Capturer statut,
   journaux worker, candidat/attestation, moteur effectif, sorties et durées.
   Vérifier la création de `settings/logs` par le produit dans le staging neuf.
   Si les courts ne prouvent pas le GPU effectif ou échouent, ne pas lancer les longs.
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
dispose de ses TEMP/TMP/LOCALAPPDATA propres ; CARGO_HOME reste isolé sous
le lot selon le contrat existant de la garde, sans imposer une copie par exécution.
Cible Cargo : `target/parakeet-tests/cargo-shared/`, conservée. Si le contrat
actuel de la garde ne permet pas ces racines ou les entrées retrouvées,
arrêter avant exécution et soumettre les chemins canoniques nécessaires.

Fichiers suivis modifiables après accord : `ci/native-profiles.json`,
`ci/guard-native.ps1`, `crates/whisper-adapters/tests/parakeet_unified_native.rs`
(seule déclaration du sous-module et adaptation minimale du support partagé
si nécessaire), `crates/whisper-adapters/tests/parakeet_unified_native/voxpopuli.rs`
(nouveau module de tests), ce plan, `docs/architecture/BILAN-FUSION-PILE.md`,
`docs/architecture/raw/qualification-gpu/results.json` et une archive des
petites preuves. Huit fichiers suivis prévus, huit tâches. Requêtes et journaux
d'exécution restent sous la racine ignorée du lot.

La session principale écrit les documents et effectue les commits. Les
implémenteurs ont chacun leur périmètre exclusif ; le reviewer reste indépendant.
Aucun Rust de production, manifeste, lockfile, `.cargo/`, garde N1/historique,
modèle, script VoxPopuli original, audio original ou ressource installée modifié.
Les deux chemins Rust de tests ci-dessus constituent l'extension explicite
proposée pour ce nouveau corpus, à accepter avant implémentation. Toute autre
trace produit ou nouveau chemin de code nécessite un arbitrage.

## Arrêts et limites de fin

Appliquer AGENTS : violation du préflight, surveillance, compilation ou test
échoué entraîne arrêt, conservation des traces, diagnostic en lecture seule
et arbitrage avant reprise. Les exceptions des lots précédents ne sont pas
reconduites. Aucun pilote/SDK installé, élévation, service, isolation Windows,
compte, ACL, effacement ni réparation de l'installation autorisés.

Le lot est qualifié pour sa matrice uniquement si chaque cas GPU prévu a des
preuves attribuables et les critères AC1-8 sont satisfaits. Sinon livrer le
bilan partiel avec les cases non qualifiées et la décision nécessaire.
Autres GPU, Turbo hors mesures historiques, micro GPU,
premier texte affiché en moins de 15 secondes, interface Windows et paquet
distribué restent explicitement hors qualification de cette campagne import.
Une validation visuelle ultérieure par l'utilisateur et la remise d'un nouveau
paquet incluant les correctifs récents nécessitent un lot distinct.

## Préparation réalisée — 2 novembre 2026

L'utilisateur a demandé l'utilisation des outils locaux pour générer les
ressources et confirmé que les deux candidats de 60 secondes sont bons au
niveau du son. Cet accord porte sur leur audio, pas sur l'exactitude de leurs
textes. Aucun test Rust, worker, modèle, inférence ni campagne GPU exécuté
pendant cette préparation ; aucun téléchargement ou outil installé.

Les fonctions de `voxpopuli_60s.py` ont été utilisées via Python existant et
FFmpeg/ffprobe installés, sans modifier les scripts d'origine ni créer de
nouveau programme persistant. TEMP/TMP/LOCALAPPDATA de préparation sont
placés sous le lot. Les fichiers générés sont locaux et ignorés par Git :

| Base sous `target/n2-native/qualification-gpu/corpus/` | Durée WAV | Fichiers disponibles |
| --- | --- | --- |
| `fr-short-60s` | 60,000 s | WAV, MP3, `.reference.txt`, `.annotations.json` |
| `en-short-60s` | 60,000 s | WAV, MP3, `.reference.txt`, `.annotations.json` |
| `fr-long-35m` | 2 105,814 s, soit 35 min 05,814 s | WAV, `.reference.txt`, `.annotations.json` |
| `en-long-40m` | 2 433,459 s, soit 40 min 33,459 s | WAV, `.reference.txt`, `.annotations.json` |

Six audios et huit textes/annotations, **151 356 410 octets** au total.
Les WAV courts normalisent seulement le conteneur : leurs 960 000
échantillons PCM sont identiques à ceux des candidats approuvés. Les MP3
se décodent chacun en 60 secondes d'audio mono 16 kHz.

Les longs concatènent respectivement **77 et 102 passages annotés distincts**
issus de l'archive locale. Les intervalles d'une même source ne se chevauchent
pas ; aucun bouclage ni silence ajouté. Les 179 morceaux de PCM ont été
comparés aux plages correspondantes des WAV finaux : identiques. Les
annotations couvrent chaque WAV sans trou, référencent les originaux extraits
conservés sous `corpus-preparation/raw/`, leurs intervalles, métadonnées,
textes et empreintes. Les décodages complets des quatre WAV réussissent et
leurs niveaux mesurés sont consignés. Aucun de ces contrôles ne vaut écoute
humaine des longs ni validation linguistique.

Les métadonnées utilisées ne signalent aucune ligne gold pour les passages
disponibles de l'archive. Les quatre références restent donc
`METADATA_TEXT_NOT_HUMAN_VERIFIED`. Audio court : `USER_CONFIRMED_SOUND` ;
audio long : `NOT_HUMAN_LISTENED`. Ne pas qualifier la précision ASR à partir
de ces textes sans expliquer et lever cette limite.

Contrôles : 14/14 fichiers conformes à leurs SHA après création, 4/4 WAV
décodés, 2/2 MP3 décodés, 179/179 morceaux PCM vérifiés ; **13 fichiers sources
protégés inchangés**, dont archive, annotations, modules Python et candidats.
Résultats, chemins, SHA, provenance et logs FFmpeg sous
`target/n2-native/qualification-gpu/corpus-preparation/preparation-results.json`
et le même dossier. Extraits temporaires conservés, aucune suppression.
La prochaine phase reste l'implémentation et la revue des recettes dédiées,
puis la campagne sous garde après accord ; le corpus généré n'est pas une
preuve de qualification GPU.
