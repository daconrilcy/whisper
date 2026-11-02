Avancée : huit exécutions PASS, neuf imports Parakeet CUDA réels ; code, contrat et preuves techniques CLEAN.
Blocage : aucun pour cette matrice technique ; précision linguistique et comparaison CPU non qualifiées.
Décision : qualifier la matrice technique sur Z790, sans généralisation matérielle, installation ou push.

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
   La copie stagée ne contient pas `settings` précréé pour ces nouveaux profils.
   La recette vérifie le journal GPU réel de la route unifiée,
   `runtime_root/settings/logs` (branche GPU confirmée par la chaîne complète).
   Ne pas retirer le
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
   Vérifier la création et l'usage de `runtime_root/settings/logs` par la
   branche GPU de la route unifiée dans le staging neuf, sans fichier simulé.
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

Racine d'exécution acceptée après inventaire : `target/n2-native/` ; corpus,
requêtes et synthèse dédiés restent sous `target/n2-native/qualification-gpu/`.
Les RunRoot sont ceux produits par la garde existante sous la racine N2.
Chaque exécution
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

## Accord d'implémentation et inventaire de départ

Après validation du corpus, l'utilisateur a autorisé explicitement
l'implémentation dans cette session. Deux auteurs exclusifs :
`rust_implementer` pour les deux fichiers de tests Rust ; worker CI pour
`ci/guard-native.ps1` et `ci/native-profiles.json`. La session principale
conserve les documents, les requêtes et les commits. Les sources sont propres
au départ hors modèles/audio non suivis ; HEAD `b61b8b0c`.

Le préflight statique a établi que la garde accepte la racine exacte
`target/n2-native`, pas son sous-dossier `qualification-gpu`. L'utilisateur a
autorisé explicitement l'emploi de cette racine existante pour les exécutions,
avec corpus et bilan dans le sous-dossier. Aucune extension des racines ou
de la surveillance n'en découle. Aucun appel Cargo ou NVIDIA effectué lors
de cet arbitrage.

Inventaire Windows en lecture seule : NVIDIA GeForce GTX 1080 Ti, pilote
`32.0.15.8129` ; Parsec Virtual Display Adapter et Intel UHD Graphics 770
également déclarés. L'identité CUDA/UUID courante sera prouvée sous garde.
CUDA 12.9 et VsDevCmd existent ; libclang existant retrouvé sous
`target/pile-preserved/r8/target/n2-native/toolchain/libclang.dll`.

Les 21 ressources personnelles correspondent aux SHA du relevé
`raw/r8-install/installed-results.json` : 3 196 008 875 octets. Copie exacte
vérifiée dans `target/n2-native/qualification-gpu/inputs/personal-runtime`,
sans `settings` et sans modification de l'installation. Inventaire local :
`target/n2-native/qualification-gpu/runtime-inventory.json`. Cette copie
contient les workers qualifiés existants ; le code adapter courant sera
celui des nouveaux harnais, sans nouvelle release desktop.

Un arbitrage de revue a demandé de vérifier le vrai journal de la route
unifiée plutôt qu'un emplacement supposé ; l'utilisateur l'a accepté.
La première interprétation du reviewer, qui désignait `runtime_root/logs`,
a ensuite été rétractée après vérification de la chaîne complète :
`native.rs:168-171` appelle `spawn_live`, mais `parakeet_import.rs:375-380`
transmet `None` pour le GPU ; `worker_stderr_log:703-709` choisit donc
`runtime_root/settings/logs`. Le test renforcé vérifiera ces journaux GPU
réels, non vides et associés aux imports. Aucun routage produit modifié,
aucun fichier créé artificiellement ; le dossier `runtime_root/logs` peut
rester vide. La validation N1 de `92f9a2f7` reste distincte des résultats
natifs à venir ; aucun résultat natif n'est anticipé par cette correction.

Première revue CI : GCI-01 BLOQUANT sur attente des flux sans borne après
sortie du parent. Correction dans les seuls nouveaux profils : attente
processus/EOF bornée, suivi et arrêt des descendants observés, fixtures
redirigées parent vivant et parent sorti. Relecture statique CLEAN ; preuve
dynamique encore requise. Aucun contrôle d'isolation préventive n'est déduit
du suivi périodique des descendants.

SelfTest N2 exécuté une fois, sources figées et sans copie concurrente :
**exit 1**, `Deadline fixture did not capture both inherited child streams.`
Trace : `qualification-gpu/verification/n2-selftest-initial.log`. La fixture
`live-parent` a confirmé expiration et arrêt parent/enfant, puis échoué sur
au moins un marqueur de sortie ; la fixture `exited-parent` n'a pas démarré.
Aucun flux/état de collecte conservé permettant d'en déduire la cause.
Campagne arrêtée avant tout Cargo, NVIDIA, modèle ou inférence.

L'utilisateur a ensuite autorisé un complément limité aux deux fichiers N2 :
conserver flux/EOF/erreurs de capture, handshake enfant après ses émissions,
une exécution diagnostique attendue, puis correction seulement si la cause
est démontrée et reprise auto-tests/revue/préparation si tout passe. Cette
autorisation ne lève aucun arrêt pour fuite de racine, surveillance ou
lancement natif inattendu. Les deux cycles de correction restent suivis.

Diagnostic unique exécuté comme donnée attendue :
`EXPECTED_DIAGNOSTIC_FIXTURE_FAILURE`. Parent redirigé, mais enfant avec
`IsOutputRedirected=false` et `IsErrorRedirected=false` malgré ses deux
émissions signalées ; stdout capturé vide, stderr `#< CLIXML`, aucune erreur
de job et nettoyage sans erreur. Le handshake a échoué avant armement du
délai. Cause de fixture/héritage établie, pas de défaut de capture production
démontré. Trace : `target/n2-native/selftest-voxpopuli-live-parent-7674fa64e49b43f89e7b55f5359eb7ef/fixture-trace.json` ;
commande/statut dans `qualification-gpu/verification/deadline-diagnostic-20261102T144210584/`.
Correction de la seule fixture en préparation : parent CMD caché, outil déjà
utilisé par la garde, et héritage réel des flux avec handshake avant délai.
FullSelfTest encore non repris ; aucune compilation ni inférence exécutée.

Après correction de la fixture, le FullSelfTest a passé et ses preuves ont été
relues : redirections effectives, deux marqueurs, délais armés puis expirés,
parent vivant et parent sorti avec flux ouverts, nettoyage sans erreur. Trace
`qualification-gpu/verification/n2-selftest-after-fixture-fix.log` ; les deux
`fixture-trace.json` sont conservés sous les répertoires selftest du lot N2.
Contrat Préparation/List revu CLEAN. Côté Rust, R1 (annotation MP3) et R2
(journaux GPU réels) sont fermés statiquement ; seule la préparation/list
nécessaire à la preuve de compilation reste en attente.

Préflight `ParakeetGpuPreparation` PASS, puis compilation sans exécution
démarrée. Run `ParakeetGpuPreparation-72132271b1d3483bbd21d272e882f37e` :
**FAIL**, code Cargo non acquis, 47,88 s après lancement Cargo, limite du
profil 1 800 s non atteinte. La requête globale `Win32_Process` de suivi des
descendants a expiré (borne CIM deux secondes) ; l'observation finale du
nettoyage a échoué sur la même requête. Compilation interrompue alors que les
dépendances se construisaient ; aucun corps de test, modèle ou inférence.
Traces complètes dans `target/n2-native/logs/ParakeetGpuPreparation-72132271b1d3483bbd21d272e882f37e/`.
La cause interne de lenteur du fournisseur CIM n'est pas établie. Après arrêt,
aucun Cargo/rustc actif n'a été observé ; cela ne transforme pas le run en PASS.

Deux cycles CI étant consommés, l'utilisateur a autorisé explicitement un
cycle supplémentaire limité aux deux fichiers N2 : requêtes CIM ciblées sur
les identités suivies, arrêt des processus déjà retenus même si CIM échoue,
fixture de panne CIM et arguments Préparation/List limités au seul harnais
`parakeet_unified_native`. Puis revue, auto-tests et reprise si tout passe.
Les recettes Rust, le code produit et les racines autorisées restent inchangés.

Le candidat du cycle supplémentaire cible les requêtes CIM et la préparation
du seul harnais comme demandé. Sa revue relève **GCI-02 BLOQUANT** : un seul
passage de découverte dans le nettoyage ne parcourt pas les enfants d'un
descendant nouvellement trouvé. Un helper peut donc rester actif sans erreur
signalée. Correction proposée : fermeture du parcours des nouveaux parents,
bornée globalement avec échec explicite si la borne est atteinte, et fixture
de trois générations dont deux niveaux sont inconnus au début du nettoyage.
Le cycle supplémentaire est consommé ; nouvel arbitrage demandé avant
correction/reprise. Aucun SelfTest/Cargo relancé sur ce candidat.
Accord explicite reçu ensuite : un cycle supplémentaire limité à cette boucle
de nettoyage et à la fixture de trois générations, puis relecture, auto-tests
et reprise si tout passe. Aucun autre périmètre n'est étendu.

Au début de ce nouveau tour, C: ne dispose plus que de 19,30 Gio, puis 20,21 Gio
au second relevé. Le seuil permanent de 30 Gio interdit tout SelfTest,
compilation ou campagne de tests ; seules correction et revue statique
continuent. Origine de cette baisse d'espace non attribuée à la campagne.
Inventaire lecture seule : lot N2 4,68 Gio, cible Cargo partagée 33,29 Gio,
archives `pile-preserved` 49,09 Gio, remise CPU 4,55 Gio. Aucune suppression.
La matrice complète conserverait environ 25 Gio supplémentaires de copies
isolées runtime/corpus ; objectif recommandé 60 Gio libres avant la campagne.
L'utilisateur a choisi de libérer l'espace manuellement. Cette réponse n'est
pas une preuve que l'espace est déjà libéré ; il sera remesuré avant reprise.

Cycle ciblé GCI-02 terminé côté source : boucle arrêt/découverte jusqu'à
fermeture des parents observés, borne globale de dix secondes, erreur
explicite si la fermeture n'est pas acquise, et fixture de trois générations
dont deux niveaux sont inconnus initialement. Seul `ci/guard-native.ps1`
modifié dans ce cycle. AST sans erreur, `git diff --check` PASS.
Même reviewer : **CLEAN statique du contrat Préparation/List**, GCI-02 fermé.
La borne est contrôlée avant/après les appels CIM synchrones et ne préempte
pas un fournisseur bloqué à l'intérieur d'un appel ; aucune isolation
préventive n'est annoncée.

Gaps dynamiques à fermer après rétablissement de l'espace : fixture des
trois générations, panne CIM simulée, deux scénarios de flux et FullSelfTest,
puis préparation du seul harnais `parakeet_unified_native` et inventaire.
Aucun SelfTest, Cargo, processus de fixture ou GPU exécuté après ce dernier
patch. Dernier relevé : 20,20 Gio ; six profils Run toujours en attente.

L'utilisateur a ensuite confirmé la libération d'espace et demandé la reprise.
Relevé réel : **58,96 Gio libres**. Les 21 fichiers du runtime et les 14 fichiers
du corpus correspondent toujours à leurs SHA ; cible Cargo partagée et
libclang présents, `settings` absent de l'entrée runtime neuve. Les suppressions
ont été effectuées manuellement par l'utilisateur ; aucune suppression par
l'agent. Reprise du FullSelfTest sur le dernier correctif GCI-02 autorisée,
puis préparation/list et campagne selon les gates déjà acceptés.

Reprise réelle après libération : FullSelfTest du dernier correctif **PASS**,
trace `qualification-gpu/verification/n2-selftest-after-gci02.log`. Même
reviewer : preuve dynamique confirmée pour trois générations (quatre passes,
fermeture complète, aucun survivant observé), panne CIM et scénarios de flux ;
arguments Préparation/List ciblés confirmés. GCI-01/GCI-02 fermés, contrat
Préparation/List **CLEAN**.

Préflight conforme, puis `ParakeetGpuPreparation` : **NO_RUN_PASS**, Cargo 0,
10,756 s, commande ciblant `--test parakeet_unified_native --no-run`.
Run `ParakeetGpuPreparation-4e8c954050ed46bdb649708a20b5aa87`.
`ParakeetGpuInventory` : **LIST_PASS**, Cargo 0, 3,413 s ; run
`ParakeetGpuInventory-f025ffb4fb104791aa6ca4f87ef12830`. Les six identités
`voxpopuli::` prévues sont présentes dans l'inventaire des 17 tests.
Ces deux opérations ne lancent aucun corps de recette native et ne constituent
pas une inférence GPU. Journaux sous `target/n2-native/logs/<run>/`.
Activation des six profils Run après cet inventaire, puis revue ciblée du
contrat final avant toute inférence.
Même reviewer Rust : **CLEAN**, R1/R2 et gap V1 fermés ; aucune qualification
native ou linguistique déduite de ce verdict de code.

Six profils Run activés READY dans le seul registre, chacun avec son sélecteur
exact : `ParakeetGpu`, `ParakeetGpuFrenchShort`, `ParakeetGpuFrenchLong`,
`ParakeetGpuEnglishLong`, `ParakeetGpuFrenchAutoLong`,
`ParakeetGpuEnglishAutoLong`. Même reviewer CI : contrat d'activation **CLEAN**.
Les six requêtes finales sous `qualification-gpu/requests/<profil>.json`
épinglent chacune les 21 fichiers runtime, 14 fichiers corpus et cinq OGG
originaux, soit 40 fichiers. Nouveau FullSelfTest sur le registre activé et
préflight de chaque requête requis avant inférence. Les longs attendent
les quatre exécutions courtes réussies et leurs preuves CUDA.

## Résultats de la campagne native — 2 novembre 2026

FullSelfTest sur le registre activé PASS ; six préflights finaux PASS, chacun
avec 40 fichiers épinglés. Les huit exécutions ci-dessous sont ensuite PASS :
un corps de test réellement exécuté par invocation, zéro ignoré/échec,
**neuf imports** sur quatre contenus audio distincts (les répétitions et les
deux encodages du court FR ne sont pas des contenus supplémentaires).

| Cas | Audio traité | Temps commande Cargo | Ratio commande/audio | Verdict |
| --- | ---: | ---: | ---: | --- |
| EN court GPU, répétition 1 | 60 s | 40,283 s | 0,671 | PASS |
| EN court GPU, répétition 2 | 60 s | 38,719 s | 0,645 | PASS |
| EN court GPU, répétition 3 | 60 s | 38,905 s | 0,648 | PASS |
| FR court GPU, WAV puis MP3 | 2 × 60 s | 41,913 s pour les deux | 0,349 | PASS |
| FR long GPU manuel | 2 105,814 s | 85,716 s | 0,0407 | PASS |
| EN long GPU manuel | 2 433,459 s | 82,807 s | 0,0340 | PASS |
| FR long Auto avec GPU effectif | 2 105,814 s | 84,305 s | 0,0400 | PASS |
| EN long Auto avec GPU effectif | 2 433,459 s | 83,350 s | 0,0343 | PASS |

Ces temps incluent le démarrage/validation de recette et le traitement, mais
excluent le staging des entrées ; ils ne sont pas une mesure isolée du noyau
d'inférence, ni un délai de premier texte affiché. Tous sont inférieurs à la
durée de l'audio. Les quatre longs sont traités dans 3,4 à 4,1 % de leur durée
selon cette borne conservative. Le cas FR court mesure deux imports ensemble.

Neuf nouveaux journaux réels contiennent `[asr] ... backend=CUDA0` ; les
recettes Auto exigent aussi le backend CUDA effectif et ne passent pas sur
un repli CPU. Inventaires : NVIDIA GeForce GTX 1080 Ti, PCI `0000:01:00.0`,
PnP du périphérique hôte, CUDA/GGML ordinal 0 ; pilote relevé `32.0.15.8129`.
L'UUID n'est pas archivé séparément ; les contrôles d'identité de la route
native sont exercés, sans inventer de valeur absente des reçus.

TXT/SRT publiés et WAV d'archive contrôlés par les recettes : fichiers non
vides, plages/échantillons cohérents et sources intactes. Le staging neuf ne
contenait pas `settings` ; le produit a créé `settings/logs` dans ces parcours
GPU et chaque import a produit son propre journal. Les traces complètes sont
conservées sous `target/n2-native/logs/<run>/` et `runs/<run>/`.

Synthèse et références : [results.json](../architecture/raw/qualification-gpu/results.json).
Archive des petites preuves : [campaign-evidence.zip](../architecture/raw/qualification-gpu/campaign-evidence.zip),
**303 entrées**, tailles et SHA-256 comparés à chaque source ; 654 462 octets,
SHA-256 `8b9ef34f5f280407382705be7280cedc977db6383dcd56e18e3367761b18a228`.
Les audios et modèles ne sont pas embarqués dans cette archive. Les erreurs
intermédiaires et le diagnostic attendu restent présents et séparés des succès.

Limites : références textuelles VoxPopuli non vérifiées humainement, pas de
WER/CER qualifié ni de comparaison linguistique CPU/GPU ; longs non écoutés
intégralement. Aucun autre GPU/architecture, nouveau scénario Turbo, micro,
premier texte affiché, fenêtre Windows ou paquet installé qualifié par ce lot.
Le paquet portable existant reste inchangé ; remise et recette visuelle
ultérieures distinctes. Aucun nouveau nettoyage par l'agent, aucune
installation, dépendance, modification de Rust produit ou push.

Revue finale indépendante du dossier : **CLEAN**, aucun constat actionnable.
Huit stdout/invocations, neuf ASR CUDA0, identités et neuf WAV avec nombres
d'échantillons exacts recoupés ; 317 références taille/SHA et les 303 entrées
de l'archive vérifiées sans écart. Les quatre SHA des fichiers de code
correspondent au candidat testé. Ce verdict porte sur le dossier technique,
pas sur une qualification globale du produit. Espace libre de clôture :
environ 59,7 Gio ; aucun nettoyage automatique et aucun push.
