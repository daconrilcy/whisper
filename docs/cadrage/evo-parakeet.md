Avancée : besoin, exigences, domaine et architecture proposés pour Parakeet TDT 0.6B v3.
Blocage : l'inférence du GGUF exact sur CPU, GTX 1080 Ti et fenêtres micro n'est pas démontrée ; la dépendance native reste à autoriser.
Décision : design DRAFT / DESIGN BLOCKED ; soumettre les choix produit et le SPIKE borné avant tout plan exécutable.

# Évolution Parakeet et catalogue de modèles — cadrage du 15 octobre 2026

**Base lue :** `evo-startup-perf` à `420199cba48abf4e9deeae73aeb4d672fc859b80`, arbre propre au préflight. **Statut :** DESIGN BLOCKED, proposition documentaire. **Autorité :** les décisions produit ci-dessous viennent des messages de l'utilisateur de cette session ; les options techniques restent proposées. Ce cadrage ne permet ni code produit, ni essai natif, ni téléchargement, ni copie de modèle, ni installation, ni push.

## 1. Besoin, décisions et périmètre

L'application Windows conserve **large-v3-turbo comme modèle premium et référence** et ajoute **Parakeet TDT 0.6B v3** pour l'import WAV/MP3 et le microphone, sur CPU et GPU. La liste « Modèle local » remplace le chemin et ne contient que les modèles connus, livrés avec l'application, présents et utilisables. L'utilisateur final ne remplace ni n'importe librement un fichier modèle. L'application et les modèles doivent pouvoir être livrés et mis à jour ensemble. `C:\Users\cyril\Downloads\models` est seulement la source provisoire du GGUF déjà téléchargé.

Le **modèle** et le **calcul** sont deux choix distincts. Auto décide CPU/GPU pour le modèle choisi, sans changer de modèle. Sans préférence de modèle, Turbo est le défaut ; si l'utilisateur choisit Parakeet, ce choix est conservé après fermeture, y compris lorsque le calcul reste Auto. Cette règle ne change pas à elle seule la présélection actuelle du calcul GPU/CPU au démarrage.

Le futur lot recherché comporte le backend Parakeet, son raccordement aux parcours existants, un catalogue solidaire de la livraison et des sondes. **La qualité** sera comparée à Turbo sur un corpus et une méthode à fixer, avec écarts rapportés sans seuil initial. **Les campagnes de durée et de charge CPU/GPU sont différées.** « Plus rapide que Turbo » est un objectif futur, pas un succès affirmé ni un seuil du premier lot. Les cibles historiques de réactivité et de premier texte restent documentées pour leurs évolutions d'origine ; elles ne deviennent pas, par silence, de nouveaux seuils Parakeet.

| ID | Décision utilisateur acceptée | Source dans la session |
| --- | --- | --- |
| DEC-PARA-01 | Ajouter Parakeet à Turbo, sur CPU et GPU. | Demande initiale. |
| DEC-PARA-02 | Liste limitée aux modèles présents et pris en charge, sans chemin de modèle libre. | Demande et correction. |
| DEC-PARA-03 | Modèles non remplaçables dans l'application finale ; application et modèles gérés ensemble à la mise à jour. | Correction. |
| DEC-PARA-04 | Turbo premium et point de comparaison ; mesurer la perte de qualité sans cible initiale. | Correction. |
| DEC-PARA-05 | Backend avec sondes, sans campagne immédiate de performance ou de charge. | Correction. |
| DEC-PARA-06 | En Auto, Turbo par défaut sans préférence ; choix explicite de Parakeet conservé. | Correction. |
| DEC-PARA-07 | Mission présente : cadrage avec agents dédiés. | Dernière demande. |

**Continuité requise :** import source intacte et TXT/SRT ; micro avec blocs confirmés avant arrêt, conservation de l'audio admis et TXT/SRT/WAV de même nom après drainage ; choix micro Windows et modes CPU/GPU/Auto existants. Le changement de modèle ou de calcul pendant une séance, la reprise après panne, tray, sortie MP3, découverte libre, téléchargement automatique, installateur et mise à jour automatique ne sont pas ajoutés à ce périmètre. Une livraison commune ne suppose pas un programme de mise à jour automatique.

**Changement de référence :** `docs/cadrage/evo-arbo.md` a accepté un chemin modèle modifiable et mémorisé pour son lot. DEC-PARA-02/03 remplacent ce contrat **pour le champ modèle seulement** dans cette évolution ; les champs audio et destination gardent leurs parcours. L'ancien cadrage reste historique, sans réécriture rétroactive.

## 2. Inventaire vérifié et limites

| Observation sur la base lue | Portée de la preuve |
| --- | --- |
| `crates/whisper-desktop/src/ui.rs` présente chemin modèle et « Parcourir… » ; les demandes import/micro transmettent chemin et empreinte unique Turbo. | Le catalogue et l'identité Parakeet ne sont pas raccordés. |
| La même UI propose déjà Auto, CPU et GPU. `worker_ipc.rs` possède des chemins Auto. | Le début historique d'`evo-auto.md` est périmé sur la présence du contrôle ; aucun essai Auto réel avec Parakeet n'en découle. |
| Les workers CPU/GPU chargent `whisper_rs::WhisperContext`, avec `whisper-rs =0.16.0`. IPC actuel version 2. | Le GGUF Parakeet n'est pas démontré compatible avec ces workers. |
| Le GGUF `parakeet-tdt-0.6b-v3.q8_0.gguf` existe dans Downloads, 713 975 456 octets. Les métadonnées locales indiquent le SHA-256 publié par NVIDIA. | L'empreinte des octets du fichier local n'a pas été recalculée ; aucune inférence locale exécutée ici. |
| `whisper-bootstrap` ne fournit pas aujourd'hui d'installateur complet. | L'emplacement et l'opération de livraison commune sont à concevoir. |
| `direct_live.rs` conserve un backlog WAV et dispose d'une fenêtre RAM de 128 000 échantillons mono 16 kHz, avec une fenêtre d'inférence en vol. | Cette fenêtre existante n'est pas une preuve de résultat Parakeet, notamment aux bords et pour la queue courte. |
| La finalisation WAV et les écritures TXT/SRT sont séparées ; l'arrêt du processus worker existant comporte un `wait()` sans borne explicite. | Aucune transaction atomique multi-fichiers ni borne de fin de traitement prouvée. |

Le modèle [NVIDIA Parakeet TDT 0.6B v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) indique 25 langues avec détection automatique, dont le français. Le [GGUF officiel de la révision `541d1f99`](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3/blob/541d1f99c6b0c3cd0b11a95167540bb8edefd82b/parakeet-tdt-0.6b-v3.q8_0.gguf) affiche le SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`. Les [documents NeMo-Speech.cpp v0.2.0](https://github.com/NVIDIA/NeMo-Speech.cpp/releases/tag/v0.2.0) décrivent une ABI C CPU/CUDA, mais leur [guide CLI](https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/v0.2.0/docs/cli.md) décrit Parakeet TDT comme **offline only** : aucune capacité micro live native ne doit être présumée. La fiche du modèle et ces guides ne prouvent pas le fonctionnement du candidat Rust sur ce PC.

## 3. Exigences, parcours et acceptation

Chaque ligne relie la capacité incluse du brief (`CAP-PARA-N`) à `REQ-PARA-N`, au parcours `UC-PARA-N` et au contrôle futur `AC-PARA-N` de même numéro. Les contrôles de produit sont **NOT RUN** ; les questions en section 7 empêchent de les déclarer définitivement acceptés. Une observation dans la fenêtre Windows sera nécessaire pour les parcours UI.

| IDs CAP/REQ/UC/AC | Exigence et scénario observable futur |
| --- | --- |
| 01 | Turbo reste sélectionnable et accomplit import/micro avec sorties et garanties existantes, sans campagne durée/charge. |
| 02 | Parakeet + CPU transcrit WAV, MP3 et micro ; modèle et calcul effectifs sont attestés, sans Turbo substitué. |
| 03 | Parakeet + GPU transcrit ces trois parcours sur le périphérique attendu ; un échec GPU manuel est expliqué, sans CPU substitué. `Ready`, inventaire ou allocation seuls ne prouvent pas l'inférence GPU. |
| 04 | Auto évalue avant chaque séance le GPU compatible **avec le modèle retenu** ; une seule tentative CPU du même modèle est permise avant la première inférence si nécessaire, jamais après. Cause, calcul effectif et préférence restent visibles. |
| 05 | Le widget ne propose que les modèles connus de la livraison, présents et validés ; pas de fichier arbitraire, de ligne « indisponible » ou de substitution silencieuse. |
| 06 | Le widget n'a plus de saisie ni de navigation de fichier modèle ; les chemins audio et sortie restent utilisables. |
| 07 | Une livraison complète résout app, workers, runtimes et modèles depuis son ensemble, sans dépendance à Downloads ou au chemin développeur ; une livraison incohérente ne s'annonce pas réussie. |
| 08 | Profil neuf : Turbo par défaut. Parakeet choisi explicitement avec Auto : Parakeet et Auto sont retrouvés après redémarrage et réellement utilisés, sans écrasement par une sonde GPU tardive. |
| 09 | Pour chaque combinaison admise, WAV/MP3 restent intacts et les TXT/SRT exportés sont lisibles ; micro et import ne se lancent pas simultanément. |
| 10 | Pendant la capture, des phrases séparées par silence donnent des blocs confirmés ordonnés, sans doublon ni texte provisoire présenté comme confirmé. Capture réelle et moteur prêt sont distingués. |
| 11 | Stop, y compris avant moteur prêt, ferme l'admission, conserve l'audio déjà admis, draine le reliquat et produit TXT/SRT/WAV de même base, dont la fin de séance ; aucun succès si sortie manque. |
| 12 | Préparation, capture, traitement, finalisation, arrêt et fermeture normale sont visibles ; un résultat ancien ne modifie pas une nouvelle séance. L'annulation d'import dépend de Q-PARA-06. |
| 13 | Distribution, modèle, moteur, capture, saturation et export en erreur donnent cause, action, fichiers conservés et statut incomplet ; aucun faux succès. |
| 14 | Le choix micro Windows, son défaut de réouverture et l'absence de substitution pendant la séance restent identiques pour les deux modèles. |
| 15 | CPU/GPU manuels restent fixes pendant une séance ; calcul effectif et échec sans repli manuel sont observables. |
| 16 | Les langues réellement compatibles sont explicitées ; SRT ordonné sur la timebase source avec silences, sans forçage de langue fictif. Décision Q-PARA-04. |
| 17 | Deux séances du même modèle et un changement de modèle entre séances ne mélangent ni contexte, ni texte, ni calcul. Capture anticipée et réemploi compatible restent fonctionnels, sans nouveau seuil chronométré. |
| 18 | Les sondes émettent des faits corrélables : modèle/révision, séance, calcul demandé/effectif, admission audio, moteur prêt, première inférence, premier bloc et sorties ; elles n'inventent pas un pourcentage de charge ni un gain. Aucun benchmark maintenant. |
| 19 | Sur un corpus gelé et une référence annotée, les sorties Turbo et Parakeet sont comparées et les écarts de texte/horodatage rapportés par langue et calcul, sans plafond de perte initial. Corpus et méthode Q-PARA-05. |

**États visibles requis :** prêt ; préparation sans capture ; initialisation micro ; audio admis et moteur en préparation ; transcription active ; capture arrêtée avec audio restant ; exports ; terminé ; erreur/incomplet ; fermeture normale ; réouverture. « Fenêtre ouverte », « worker Ready » et « audio annoncé capturé » sont trois constats différents. La dernière phrase du micro est un cas de contrôle. La reprise après panne demeure différée ; une fermeture normale suit le contrat Stop.

**Risques fonctionnels ouverts :** préférence écrasée par Auto ou migration ; faux GPU ou repli tardif ; catalogue trompeur ; audio perdu/doublé à l'arrêt ; SRT désynchronisé ; corpus de qualité biaisé ; attente présentée comme panne ; réemploi d'un contexte incompatible. Chaque risque renvoie aux AC 03–05, 08, 10–13, 16–19 selon le scénario. Les résultats d'acceptation restent NOT RUN avant code.

## 4. Domaine et invariants

Le domaine distingue `ModelId` (Turbo/Parakeet), révision de l'artefact, préférence de modèle (absente ou explicite), préférence de calcul (Auto/CPU/GPU), calcul effectif, configuration figée de séance, audio admis, fenêtre d'inférence, bloc confirmé et résultat publié. Le catalogue est constaté par l'application/adaptateur ; le domaine ne lit pas le disque. Une séance fige modèle, révision, source, langue, calcul demandé, destination et identité de micro applicables.

| ID | Invariant proposé, relié aux REQ/AC |
| --- | --- |
| INV-PARA-01 | Auto ne change pas `ModelId` ; sans préférence Turbo est le défaut, et une sélection explicite valide persiste. 04/08/15. |
| INV-PARA-02 | Catalogue : connus, livrés, présents et validés seulement ; aucune entrée absente ou chemin arbitraire. 05–07. |
| INV-PARA-03 | Configuration figée après admission de la séance ; au plus un repli CPU du **même modèle** avant toute inférence ; aucun repli manuel ou après première inférence. 02–04/15. Le début de capture micro peut précéder ce jalon. |
| INV-PARA-04 | Audio reçu de Windows ≠ audio admis ; capture active n'est annoncée qu'après conservation réussie. Plages admises, traitées et confirmées sont ordonnées sans trou ni doublon silencieux. 10–11/17. |
| INV-PARA-05 | Une fenêtre n'est confirmée qu'après validation de son identité, de sa plage et de sa fin ; un résultat périmé ou contradictoire ne modifie pas la séance. 10–13/16. |
| INV-PARA-06 | Stop ferme l'admission puis draine. Avant Ready, le WAV déjà admis reste à traiter. Fermer normalement suit Stop. 11–12/17. |
| INV-PARA-07 | Succès implique toutes les sorties requises acquittées ; l'import conserve l'original ; un export partiel est annoncé incomplet. 09/11–13. |
| INV-PARA-08 | Contexte réutilisé seulement si modèle/révision/runtime/calcul/périphérique/options sont compatibles ; aucun résultat de séance antérieure n'est repris. 17. |
| INV-PARA-09 | Horodatages SRT valides et ordonnés sur la référence audio ; une sonde nomme uniquement le fait observé. 16/18. |
| INV-PARA-10 | Une livraison active ne mélange pas versions app, worker, runtime et modèles ; aucun remplacement multi-fichiers n'est présumé atomique. 07. |

**Transitions de séance :** configurer → figer → préparer le moteur tout en pouvant admettre le micro → moteur prêt/attesté → inférence → arrêt de capture et drainage → publication → complet ou incomplet. Auto peut sélectionner CPU avant la première inférence même si la capture a déjà commencé. En cas d'échec GPU manuel ou après première inférence, la séance devient erreur/incomplet et préserve l'audio admis et le texte déjà confirmé selon le contrat existant. Stop répété est idempotent ; un événement d'une ancienne génération ne rouvre pas la capture. La réussite d'écriture du WAV constitue une admission pour l'arrêt normal, sans promettre de durabilité après coupure de courant.

## 5. Architecture proposée et frontières

**Option technique préférée, encore non acceptée :** un worker Rust Parakeet séparé, appelant l'ABI C ASR de [NeMo-Speech.cpp v0.2.0](https://raw.githubusercontent.com/NVIDIA/NeMo-Speech.cpp/v0.2.0/docs/sdk.md) par une façade privée. La pile est native C++17/CUDA et une petite zone `unsafe` Rust serait requise ; selon `AGENTS.md`, **elle demande un accord écrit précis dans cette session avant essai ou code**. Les workers Turbo `whisper-rs` restent séparés. Les alternatives examinées sont CLI externe (chargement répété par fenêtre), conversion vers une autre représentation (ne consomme plus le fichier demandé) et réimplémentation Rust (risque/coût nettement supérieurs). Le choix ne sera figé qu'après preuve de faisabilité et décision applicable.

| Frontière | Responsabilité et contrat futur | Contrôle envisagé |
| --- | --- | --- |
| Domaine / `whisper-core` | Identité modèle, sélection, configuration figée, politiques Auto et invariants purs. Sans OS/IO/FFI/UI. | Imports et API publiques. |
| Application / `whisper-core` | Ports possédés par elle : catalogue, préférences, tentative moteur, fenêtre d'inférence, capture, publication, diagnostic ; sérialisation des effets et erreurs typées. Sans adaptateur concret. | Revue des appels et tests de contrats. |
| Adaptateurs / `whisper-adapters` | Résolution de la release, `settings.json`, routeur IPC, capture Windows, fichiers et sondes ; aucun choix métier parallèle. | Cargo/imports et essais ciblés. |
| UI / `whisper-desktop/src/ui.rs` | Liste et états visibles issus de l'application ; ni résolution de modèles, ni fichier de réglage, ni appel natif. | Inspection du vrai widget et tests UI. |
| Composition / `whisper-desktop/src/root.rs` | Assemble catalogue, préférences et workers depuis la racine de release. | Revue des dépendances et du démarrage. |
| Worker Parakeet proposé | IPC, moteur sûr et façade `native_ffi.rs` privée ; CPU/CUDA chargés dans des processus/variantes distincts. Aucun `whisper-rs`. | Signatures ABI, durée de vie des handles, aucune `unsafe` hors façade. |

**Contrats sensibles :** un snapshot de catalogue immuable est produit hors thread UI depuis un ensemble versionné, sans balayage libre de Downloads. Les préférences migrent de `model_path` v1 à `ModelId` v2 en préservant destination, audio, micro et GPU ; un acquittement d'écriture ancien n'écrase pas le choix récent. L'IPC nouveau doit relier release, modèle/révision, séance/génération, tentative, calcul et périphérique, et refuser un worker d'une ancienne version incompatible. Les fenêtres Parakeet reçoivent PCM mono 16 kHz avec plage absolue ; texte et positions sont copiés hors de l'ABI, convertis dans la timebase source, validés puis confirmés. Le WAV reste backlog borné ; le nombre de fenêtres simultanées est borné. Une ressource native n'est réutilisée que pour une clé compatible, après réinitialisation des états de séance. Les écritures TXT/SRT/WAV restent distinctes ; aucun succès avant leurs acquittements.

**Distribution proposée, pas encore décidée :** une release autonome contient exécutable, workers, variantes natives privées et les deux modèles dans un dossier relatif à l'exécutable ; préférences et sorties restent séparées. Le lancement d'une nouvelle release se fait après fermeture de l'ancienne. Le catalogue vérifie identité/version et fichiers attendus ; le chargement recontrôle le fichier avant inférence. Cette proposition ne crée ni installateur, ni service, ni ACL, ni téléchargement automatique, ni manifeste d'empreintes documentaire. Une copie multi-fichiers en place n'est pas atomique ; le mécanisme exact de préparation/activation doit être spécifié avant livraison. Les licences et DLL redistribuées doivent être qualifiées sur le paquet réel : NeMo Apache-2.0, modèle CC BY 4.0, composants CUDA selon [l'EULA 12.9](https://docs.nvidia.com/cuda/archive/12.9.1/eula/index.html#attachment-a).

**Sondes :** événements de début/fin chargement, cache, premier audio admis, début/fin appel natif, premier bloc confirmé, Stop, fin drainage et état de chaque export ; session, release, modèle, calcul demandé/effectif et identité GPU corrélés. Horloge monotone pour les durées à l'intérieur d'un processus ; ne pas soustraire les horloges monotones de deux processus. Journal diagnostique non bloquant ; une perte de diagnostic n'efface pas l'audio. Aucune mesure de charge inventée, aucun benchmark dans ce lot. Toute nouvelle écriture hors dépôt demanderait l'accord applicable ; réemploi du journal existant à examiner.

## 6. Risques techniques et preuve préalable

| ID / classe | Inconnue qui porte une garantie produit | État et fermeture attendue |
| --- | --- | --- |
| RISK-T-01a / DESIGN_FEASIBILITY | Le GGUF local correspond au fichier NVIDIA et l'ABI/DLL se charge depuis une release Windows. | NOT RUN ; SHA local, versions et chargement avec chemin contenant des espaces. |
| RISK-T-01b / DESIGN_FEASIBILITY | Inférence CPU réelle sur ce fichier via un wrapper Rust. | NOT RUN ; audio français non silencieux, texte, calcul CPU natif et libération propre. |
| RISK-T-01c / DESIGN_FEASIBILITY | CUDA 12.9 sur GTX 1080 Ti / CC 6.1 effectue réellement l'inférence sur la carte attendue. | NOT RUN ; identité GPU liée au contexte, inférence réussie et identité fausse refusée **avant audio**. Un simple `Ready Cuda` ne suffit pas. [CUDA 13 retire Pascal](https://docs.nvidia.com/cuda/archive/13.0.0/cuda-toolkit-release-notes/index.html#deprecated-architectures). |
| RISK-T-01d / DESIGN_FEASIBILITY | Parakeet offline produit des fenêtres successives, queue courte et positions réconciliables pour blocs micro/SRT. | NOT RUN ; plages contiguës, parole traversant un bord, offsets et résultat final. |
| RISK-T-01e / DESIGN_FEASIBILITY | Langue automatique et éventuel forçage se comportent comme l'UI l'annonce. | PARTIAL documentaire ; décision Q-PARA-04 et essai ciblé si forçage requis. |
| RISK-T-02 / DESIGN puis DELIVERY | Variantes natives isolées et paquet redistribuable sans dépendance globale. | PARTIAL documentaire ; build isolé puis qualification du paquet final NOT RUN. |
| RISK-T-03–06 / PRODUCT_VALIDATION | Auto/persistance, arrêt et sorties, réemploi, qualité comparative. | NOT RUN ; tests et essai produit après code, avec corpus accepté. |

**DESIGN READY est bloqué** par RISK-T-01a–e et l'absence de décision sur la pile native. La comparaison de qualité complète est une validation produit ultérieure ; elle ne remplace pas l'essai ciblé de faisabilité des fenêtres micro.

**SPIKE-PARA-01 proposé, NOT RUN :** dans `target/parakeet-feasibility/spike01/`, par un exécutant distinct et après accord précis, vérifier SHA local, versions et dépendances ; construire ASR CPU/CUDA et un petit wrapper Rust ; transcrire le même court WAV français en CPU puis GPU avec timestamps ; exercer trois fenêtres contiguës et une queue courte avec parole à une frontière ; refuser une identité GPU fausse avant audio ; détruire/réutiliser les contextes et vérifier l'isolation des DLL Whisper/Parakeet. Budget **proposé** : 120 minutes, quatre jobs build, 8 Gio RAM et 4 Gio disque additionnels. Succès seulement avec stdout/stderr, identité native, texte, positions et témoin négatif correspondant aux branches réellement exercées. Si GPU, fenêtres ou horodatages échouent, retourner au design et à l'arbitrage ; ne pas remplacer le format/modèle sans décision. Aucun de ces essais n'est autorisé ni exécuté par le présent cadrage.

## 7. Questions et gates

| ID / owner / échéance | Décision nécessaire, options et recommandation |
| --- | --- |
| Q-PARA-01 / utilisateur + architecte, avant essai | **Pile native** : autoriser un futur SPIKE borné sur NeMo-Speech.cpp C++17/C ABI, CUDA et `unsafe` Rust encapsulé ; ou choisir une autre piste à cadrer. Recommandation technique : SPIKE proposé, après accord explicite pour technologie hors stack et actions exactes. |
| Q-PARA-02 / utilisateur, avant design distribution | **Livraison commune** : dossier de release autonome avec modèles voisins, ou emplacement séparé lié par une version. Recommandation : release autonome, sans supposer un installateur. |
| Q-PARA-03 / utilisateur, avant design catalogue | **Livraison endommagée sans modèle préféré** : bloquer la transcription et demander réparation, ou demander un nouveau choix explicite parmi les seuls modèles valides. Aucune entrée indisponible ni substitution silencieuse. Recommandation : bloquer si la release attendue est incohérente. |
| Q-PARA-04 / utilisateur, avant contrat langues | **Langue manuelle avec Parakeet** : désactiver ce réglage avec explication de la détection automatique, ou exiger un forçage dont la faisabilité reste à démontrer. Recommandation : désactiver le forçage pour Parakeet si l'API ne le prouve pas. |
| Q-PARA-05 / utilisateur + analyste, avant essai qualité | **Corpus et méthode** : définir langues/extraits, référence humaine, normalisation et restitution des écarts sans seuil ; recommandation : petit corpus représentatif incluant français, silences, noms et nombres, mêmes extraits pour les deux modèles. La mesure de qualité est prévue dans le futur lot, durée/charge différées. |
| Q-PARA-06 / analyste + utilisateur si changement produit, avant lot import | **Annulation d'import** : constater le comportement actuel et fixer le sort des sorties partielles sans faux succès. |
| Q-PARA-07 / domaine + application, avant migration | **Ancien chemin mémorisé** : reconnaître Turbo historique et migrer son identité, ou revenir au défaut avec message. Recommandation : reconnaître Turbo connu, ne jamais admettre un autre fichier arbitraire. |

Le design, sa revue indépendante et les réponses produit peuvent progresser ; **aucun plan exécutable** ne peut être déclaré READY avant la fermeture des preuves de faisabilité structurantes. Après DESIGN READY et autorisation de planifier, le `rust_plan_writer` proposera au plus dix tâches reliées aux AC, puis une revue de plan séparée. Un plan accepté n'autoriserait toujours pas le code Rust : le lot et ses chemins nécessitent un accord écrit précis dans la session concernée.

## 8. Attributions et état des contributions

Les profils `rust_product_framer` (PRODUCT-PARA v0.2), `rust_requirements_analyst` (REQ-PARA v0.2), `rust_domain_architect` (DOM-PARA v0.2) et `rust_architect` (ARCH-PARA v0.4) ont contribué **en lecture seule**. La session principale a rapproché leurs IDs et rédigé ce document. Les spécialistes ont reçu des résumés de relais plutôt qu'un fichier Git complet ; leurs paquets bruts indiquent cette limite et aucune comparaison exacte caractère à caractère n'est revendiquée. La racine active des règles `C:\Users\cyril\.codex\agent-rules\` a été résolue après le premier relais ; les copies historiques `whisper-doc-host` n'ont pas été adoptées.

La revue indépendante de **ce document Git** reste à faire. Aucun build, test, essai UI, benchmark, SPIKE, téléchargement ou déplacement de modèle n'a été exécuté dans ce cadrage. Résultats produit et faisabilité pratique : **NOT RUN**.
