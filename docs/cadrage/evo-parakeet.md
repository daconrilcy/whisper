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
| 11 | Stop, y compris avant moteur prêt, ferme la source puis draine les paquets déjà reçus ; seuls ceux écrits intégralement rejoignent l'audio admis. Le reliquat admis est traité et produit TXT/SRT/WAV de même base, dont la fin de séance ; aucun succès si sortie manque. |
| 12 | Préparation, capture, traitement, finalisation, arrêt et fermeture normale sont visibles ; un résultat ancien ne modifie pas une nouvelle séance. L'annulation d'import dépend de Q-PARA-06. |
| 13 | Distribution, modèle, moteur, capture, saturation et export en erreur donnent cause, action, fichiers conservés et statut incomplet ; aucun faux succès. |
| 14 | Le choix micro Windows, son défaut de réouverture et l'absence de substitution pendant la séance restent identiques pour les deux modèles. |
| 15 | CPU/GPU manuels restent fixes pendant une séance ; calcul effectif et échec sans repli manuel sont observables. |
| 16 | Les langues réellement compatibles sont explicitées ; SRT ordonné sur la timebase source avec silences, sans forçage de langue fictif. Décision Q-PARA-04. |
| 17 | Deux séances du même modèle et un changement de modèle entre séances ne mélangent ni contexte, ni texte, ni calcul. Capture anticipée et réemploi compatible restent fonctionnels, sans nouveau seuil chronométré. |
| 18 | Les sondes émettent des faits corrélables : modèle/révision, séance, calcul demandé/effectif, admission audio, moteur prêt, première inférence, premier bloc et sorties ; elles n'inventent pas un pourcentage de charge ni un gain. Aucun benchmark maintenant. |
| 19 | Sur un corpus gelé et une référence annotée, les sorties Turbo et Parakeet sont comparées et les écarts de texte/horodatage rapportés par langue et calcul, sans plafond de perte initial. Corpus et méthode Q-PARA-05. |

**États visibles requis :** prêt ; préparation sans capture ; initialisation micro ; audio admis et moteur en préparation ; transcription active ; capture arrêtée avec audio restant ; exports ; terminé ; erreur/incomplet ; fermeture normale ; réouverture. « Fenêtre ouverte », « worker Ready » et « audio annoncé capturé » sont trois constats différents. La dernière phrase du micro est un cas de contrôle. La reprise après panne demeure différée ; une fermeture normale suit le contrat Stop.

**Risques fonctionnels ouverts :**

| ID | Risque produit et critères qui le contrôlent |
| --- | --- |
| RISK-F-01 | Auto, migration ou découverte tardive écrase le modèle voulu : AC 04/08/17. |
| RISK-F-02 | `Ready` ou inventaire confondu avec inférence GPU, ou repli tardif : AC 03/04/15. |
| RISK-F-03 | Distribution incohérente ou modèle arbitraire proposé : AC 05–08. |
| RISK-F-04 | Audio admis perdu/doublé à l'arrêt ou fermeture : AC 10–13/17. |
| RISK-F-05 | Sorties incomplètes ou SRT désynchronisé annoncés comme succès : AC 09/11/13/16. |
| RISK-F-06 | Corpus non représentatif masque une perte de qualité : AC 16/19. |
| RISK-F-07 | Attente moteur ou silence pris pour absence de capture ou panne : AC 12/13/17/18. |
| RISK-F-08 | Contexte réutilisé pour un autre modèle ou résultats tardifs mélangés : AC 08/12/17. |
| RISK-F-09 | Nouveau backend modifie le micro choisi ou substitue un calcul manuel : AC 14/15. |
| RISK-F-10 | Sondes interprétées comme campagne ou preuve de gain : AC 18/19. |

Les résultats d'acceptation restent NOT RUN avant code.

**Traçabilité inverse du design proposé :** chaque numéro renvoie à CAP/REQ/UC/AC de la table ci-dessus ; `PORT-*` désigne les contrats décrits en section 5. Une question ouverte n'est pas une acceptation.

| REQ/AC | Domaine | Contrat d'architecture | Risque ou question ouverte |
| --- | --- | --- | --- |
| 01 | INV-01/08/12 | PORT-MOT, PORT-QUAL | RISK-F-06 ; Q-05 |
| 02 | INV-03/08 | PORT-MOT, PORT-WIN | RISK-T-01b |
| 03 | INV-03/08 | PORT-MOT, PORT-WIN | RISK-T-01c, RISK-F-02 |
| 04 | INV-01/03 | PORT-MOT | RISK-F-01/02 |
| 05 | INV-02 | PORT-CAT | RISK-F-03 ; Q-03 |
| 06 | INV-02 | PORT-CAT, PORT-PREF, UI | RISK-F-03 ; Q-07 |
| 07 | INV-10 | PORT-CAT, distribution | RISK-T-02 ; Q-02 |
| 08 | INV-01 | PORT-PREF | RISK-F-01 ; Q-03/07 |
| 09 | INV-07 | PORT-PUB, PORT-MOT | RISK-F-05 ; Q-06 |
| 10 | INV-04/05 | PORT-CAP, PORT-WIN | RISK-F-04/07, RISK-T-01d |
| 11 | INV-04/06/07 | PORT-CAP, PORT-RUN, PORT-PUB | RISK-F-04/05, RISK-T-01d |
| 12 | INV-05/06 | PORT-RUN, PORT-PUB | RISK-F-04/07/08 ; Q-06 |
| 13 | INV-04/07 | PORT-CAP, PORT-RUN, PORT-PUB | RISK-F-04/05 |
| 14 | INV-11 | PORT-MIC, PORT-CAP | RISK-F-09 |
| 15 | INV-01/03 | PORT-MOT | RISK-F-02/09 |
| 16 | INV-09 | PORT-WIN, PORT-QUAL | RISK-F-05/06 ; Q-04/05 |
| 17 | INV-03/08 | PORT-MOT, PORT-RUN | RISK-F-01/08 |
| 18 | INV-09 | PORT-DIAG | RISK-F-07/10 |
| 19 | INV-12 | PORT-QUAL | RISK-F-06/10 ; Q-05 |

## 4. Domaine et invariants

Le domaine distingue `ModelId` (Turbo/Parakeet), révision de l'artefact, préférence de modèle (absente ou explicite), préférence de calcul (Auto/CPU/GPU), calcul effectif, configuration figée de séance, audio admis, fenêtre d'inférence, bloc confirmé et résultat publié. Le catalogue est constaté par l'application/adaptateur ; le domaine ne lit pas le disque. Une séance fige modèle, révision, source, langue, calcul demandé, destination et identité de micro applicables.

| ID | Invariant proposé, relié aux REQ/AC |
| --- | --- |
| INV-PARA-01 | Auto ne change pas `ModelId` ; sans préférence Turbo est le défaut, et une sélection explicite valide persiste. 04/08/15. |
| INV-PARA-02 | Catalogue : connus, livrés, présents et validés seulement ; aucune entrée absente ou chemin arbitraire. 05–07. |
| INV-PARA-03 | Configuration figée après admission de la séance ; au plus un repli CPU du **même modèle** avant toute inférence ; aucun repli manuel ou après première inférence. 02–04/15. Le début de capture micro peut précéder ce jalon. |
| INV-PARA-04 | Audio reçu de Windows ≠ audio admis ; capture active n'est annoncée qu'après conservation réussie. Plages admises, traitées et confirmées sont ordonnées sans trou ni doublon silencieux. 10–11/17. |
| INV-PARA-05 | Une fenêtre n'est confirmée qu'après validation de son identité, de sa plage et de sa fin ; un résultat périmé ou contradictoire ne modifie pas la séance. 10–13/16. |
| INV-PARA-06 | Stop ferme la source de nouveaux paquets, draine ceux déjà reçus vers le WAV si leur écriture réussit, puis fige la frontière admise. Avant Ready, le WAV admis reste à traiter. Fermer normalement suit Stop. 11–12/17. |
| INV-PARA-07 | Succès implique toutes les sorties requises acquittées ; l'import conserve l'original ; un export partiel est annoncé incomplet. 09/11–13. |
| INV-PARA-08 | Contexte réutilisé seulement si modèle/révision/runtime/calcul/périphérique/options sont compatibles ; aucun résultat de séance antérieure n'est repris. 17. |
| INV-PARA-09 | Horodatages SRT valides et ordonnés sur la référence audio ; une sonde nomme uniquement le fait observé. 16/18. |
| INV-PARA-10 | Une livraison active ne mélange pas versions app, worker, runtime et modèles ; aucun remplacement multi-fichiers n'est présumé atomique. 07. |
| INV-PARA-11 | Source micro explicite absente à la réouverture : défaut Windows avec message et préférence conservée ; perte en séance : aucune autre source choisie automatiquement. 14. |
| INV-PARA-12 | Turbo est la référence premium ; les écarts de qualité sont rapportés sans seuil initial, et aucun gain de durée/charge n'est déclaré sans mesure ultérieure. 01/19. |

**Transitions de séance :** configurer → figer → préparer le moteur tout en pouvant admettre le micro → moteur prêt/attesté → inférence → arrêt de capture et drainage → publication → complet ou incomplet. Auto peut sélectionner CPU avant la première inférence même si la capture a déjà commencé. En cas d'échec GPU manuel ou après première inférence, la séance devient erreur/incomplet et préserve l'audio admis et le texte déjà confirmé selon le contrat existant. Stop répété est idempotent ; un événement d'une ancienne génération ne rouvre pas la capture. La réussite **complète** d'écriture du WAV constitue une admission pour l'arrêt normal, sans promettre de durabilité après coupure de courant.

**Admission et erreur :** les échantillons reçus, ceux en file de capture, ceux écrits intégralement dans le WAV et ceux inférés sont quatre frontières différentes. Seule la troisième avance l'audio admis. À saturation de la file, limite du format WAV ou refus du disque, la capture cesse sans trou silencieux : le paquet refusé n'est pas annoncé comme admis ; l'application signale cause et dernière plage acquittée. Si une écriture est partielle, elle revient à cette frontière avant finalisation ; si cette réparation échoue, le WAV est conservé mais déclaré potentiellement illisible. La finalisation est tentée seulement si possible. Aucun audio non acquitté n'est envoyé à l'inférence ni présenté comme préservé. La garantie concerne l'arrêt normal, pas une coupure de courant ni une reprise après panne.

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

**Contrats sensibles :** un snapshot de catalogue immuable est produit hors thread UI depuis un ensemble versionné, sans balayage libre de Downloads. Les préférences migrent de `model_path` v1 à `ModelId` v2 en préservant destination, audio, micro et GPU ; un acquittement d'écriture ancien n'écrase pas le choix récent. L'IPC nouveau doit relier release, modèle/révision, séance/génération, tentative, calcul et périphérique, et refuser un worker d'une ancienne version incompatible. Les fenêtres Parakeet reçoivent PCM mono 16 kHz avec plage absolue ; texte et positions sont copiés hors de l'ABI, convertis dans la timebase source, validés puis confirmés. Les **fenêtres RAM** sont bornées (candidat existant : 128 000 échantillons mono 16 kHz et une fenêtre en vol) ; le **WAV sur disque** constitue un backlog dont la durée n'est pas bornée par cette limite RAM. Une ressource native n'est réutilisée que pour une clé compatible, après réinitialisation des états de séance. Les écritures TXT/SRT/WAV restent distinctes ; aucun succès avant leurs acquittements.

**PORT-CAT / PORT-PREF — catalogue et préférences.** Owner application, réalisation adaptateurs fichiers. Entrées : racine absolue de release résolue depuis l'exécutable et version du catalogue ; snapshot de préférence avec révision croissante. Sorties : identités des seuls modèles connus/validés et acquittement de la révision effectivement sauvegardée. Absence, corruption ou mélange de versions produit une erreur de distribution, sans chemin libre ni substitution. Un modèle disparu après inventaire est recontrôlé avant ouverture. La migration v1 reconnaît seulement Turbo connu ; un ancien chemin arbitraire n'est pas importé. Une erreur d'écriture garde le choix de la séance mais interdit d'annoncer qu'il sera retrouvé au redémarrage. Les choix visibles sont construits par l'application, non par l'UI depuis le disque.

**PORT-MOT / PORT-WIN — tentative moteur et fenêtre.** Owner application, réalisation routeur IPC et worker. Entrées : release, modèle/révision, job/génération/tentative, politique de calcul, identité GPU attendue, PCM mono 16 kHz et plage absolue. Sorties : moteur prêt, attestation de calcul effectif, résultat possédé et couverture confirmée, ou erreur typée. La première admission dans l'appel d'inférence, **distincte de la capture**, ferme la possibilité de repli Auto. Un worker ou résultat d'une autre génération/version est refusé. Un GPU affiché sans inférence native corrélée n'est pas attesté. Une fenêtre invalide, hors plage ou aux temps inversés n'est pas confirmée. Les appels natifs et IPC restent hors UI, sous un propriétaire unique du contexte.

**PORT-CAP — capture et admission.** Owner application ; adaptateurs capture et WAV possèdent respectivement la file et les écritures. Entrée : blocs micro et configuration de séance figée. Sortie : plages écrites intégralement et acquittées, arrêt capture ou erreur typée. Le callback ne bloque pas. Les files RAM sont finies ; le WAV croît jusqu'aux limites disque/format. À saturation ou écriture partielle/échouée, stopper capture/admission, conserver la dernière frontière valide et les blocs confirmés, rapporter précisément la perte non admise ; si le WAV ne peut être réparé/finalisé, signaler son état potentiellement illisible avec son chemin. Stop normal ferme la source puis draine les paquets déjà reçus et écrits avec succès ; après ce drainage, figer la dernière plage admise à inférer.

**PORT-RUN — supervision et arrêt.** Owner application ; routeur IPC et superviseur possèdent le processus, worker possède l'appel natif. Les appels natifs, IPC, `kill`, `try_wait` et toute attente sont hors thread UI. Un canal de contrôle distinct des files audio permet Stop/annulation même en saturation. Une stagnation est visible, mais le temps seul ne prouve pas une panne et ne déclenche pas un nouveau calcul. Stop normal traite toutes les plages admises puis publie ; sa durée maximale n'est **pas** promise, car le backlog disque peut être grand. Un appel natif qui ne retourne pas garde l'état « traitement restant » et interdit le succès et la libération du contexte. Une annulation ou erreur qui abandonne le reliquat ferme l'admission, invalide la tentative, préserve WAV et blocs confirmés et marque les sorties incomplètes. La politique technique de grâce/terminaison/surveillance est à fixer avant le lot moteur (Q-PARA-09), puis vérifier : si la fin du processus n'est pas constatée, état `ErreurArrêtNonConfirmé`, handle conservé et surveillé hors UI, aucune annonce d'arrêt physique ni nouvelle inférence avant ce constat. Le `wait()` bloquant actuel n'est pas une preuve de ce contrat.

**PORT-PUB — publication.** Owner application, réalisation adaptateur fichiers. Entrées : séance, plages admises/confirmées, texte, timebase et motif de fin. Sortie structurée : complet/incomplet, plage conservée et traitée, état et chemin de chaque TXT/SRT/WAV. Les écritures sont séparées : l'échec de SRT après TXT ne supprime pas fictivement TXT ou WAV. Succès uniquement après chaque sortie requise ; échec de finalisation du WAV garde son chemin avec avertissement d'illisibilité possible.

**PORT-MIC — microphone Windows.** Owner application, réalisation adaptateur de capture existant. Entrée : défaut Windows ou identifiant explicite persisté. Sortie : source réellement ouverte ou cause d'échec. À la réouverture, micro explicite absent utilise le défaut avec message tout en conservant l'identifiant préféré ; perte pendant séance ne choisit aucune autre source. Parakeet reçoit le PCM de PORT-CAP et n'ouvre pas son propre microphone. Vérification future avec deux sources distinguables pour les deux modèles.

**PORT-QUAL — comparaison de qualité.** Owner application pour l'identité des résultats, analyste pour le corpus et la méthode acceptés. Entrées : corpus gelé, références annotées, règles de normalisation et sorties identifiées par extrait/langue/modèle/révision/calcul. Sortie : écarts de texte et d'horodatage traçables, avec échec `NonComparable` explicite si une transcription manque. Aucun échec n'est compté comme zéro erreur et aucune mesure de durée/charge n'est déclenchée. Cette validation produit reste NOT RUN.

**PORT-DIAG — sondes.** Owner application, réalisation diagnostic existant à examiner. Entrée : événements d'admission, moteur, inférence, confirmation, Stop et publication réellement observés. Sortie : faits corrélés à séance/release/modèle/calcul avec pertes de journal signalées. La collecte ne bloque pas la capture ; aucun pourcentage CPU/GPU supposé observé.

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

| ID / classe / portée | Autorité, options, conséquence et échéance ; REQ/AC affectés |
| --- | --- |
| Q-PARA-01 / **BLOCKING DESIGN READY** | **Utilisateur**, architecte auteur : autoriser un futur SPIKE borné sur NeMo-Speech.cpp C++17/C ABI, CUDA et `unsafe` Rust encapsulé, ou cadrer une autre piste. Sans accord précis, aucune preuve du moteur ni design prêt. Avant SPIKE et REQ/AC 02/03/10/11/16. Recommandation technique : SPIKE décrit en §6. |
| Q-PARA-02 / **STRUCTURING distribution** | **Utilisateur** pour le résultat de livraison, architecte pour le mécanisme : dossier de release autonome avec modèles voisins, ou emplacement séparé lié par une version. L'option séparée exige une cohérence entre versions plus complexe. Avant figer PORT-CAT/distribution, REQ/AC 05–07. Recommandation : release autonome sans installateur implicite. |
| Q-PARA-03 / **STRUCTURING catalogue** | **Utilisateur** : si une livraison anormale omet le modèle préféré, bloquer la transcription et demander réparation, ou demander un nouveau choix parmi les seuls modèles valides. Les deux évitent ligne indisponible et substitution silencieuse ; la seconde permet un usage partiel. Avant PORT-CAT/PREF, REQ/AC 05/08/13. Recommandation : bloquer si la release attendue est incohérente. |
| Q-PARA-04 / **STRUCTURING langues** | **Utilisateur** : désactiver « Langue manuelle » pour Parakeet avec explication de sa détection automatique, ou exiger un forçage à prouver/une autre voie. Sans décision, le contrôle pourrait promettre un effet absent. Avant contrat langues, REQ/AC 16/19 et RISK-T-01e. Recommandation : détection automatique affichée tant que le forçage n'est pas prouvé. |
| Q-PARA-05 / **STRUCTURING qualité** | **Utilisateur** valide représentativité et échéance ; analyste propose corpus, langues/extraits, référence humaine, normalisation et restitution. Petit corpus représentatif avec français, silences, noms et nombres recommandé ; alternative : corpus plus large, coût d'annotation supérieur. Avant exécution de l'AC 19, et avant conclusion sur qualité, REQ/AC 01/16/19. Pas de mesure de durée/charge. |
| Q-PARA-06 / **DETAIL avant lot import** | **Analyste**, utilisateur seulement si comportement visible change : constater l'annulation d'import actuelle, puis conserver les résultats partiels signalés ou ne rien publier comme final. Original toujours préservé, aucun faux succès. Avant lot import, REQ/AC 09/12/13. |
| Q-PARA-07 / **DETAIL avant migration** | **Domaine/application**, utilisateur si ancienne préférence inconnue est abandonnée : reconnaître Turbo historique et migrer son identité, ou revenir au défaut avec message. La seconde perd l'intention d'un chemin non reconnu ; ne jamais importer un autre fichier arbitraire. Avant PORT-PREF, REQ/AC 06/08. Recommandation : reconnaître Turbo connu. |
| Q-PARA-08 / **STRUCTURING admission/arrêt** | **Architecte**, avant DESIGN READY pour les garanties micro : préciser taille/pression de la file capture, limites du format et du stockage, point de refus du paquet et traitement d'une écriture WAV partielle. Sans ces règles, AC 11–13 risque de promettre un audio non conservé. PORT-CAP, REQ/AC 10–13/17. Les chiffres qui exigent une preuve restent à vérifier au lot. |
| Q-PARA-09 / **STRUCTURING supervision** | **Architecte**, avant DESIGN READY pour l'arrêt : fixer politique de constat de stagnation, annulation/erreur, escalade et surveillance d'un worker bloqué, sans confondre attente normale et panne. Grâce coopérative 2 s puis surveillance 5 s sont des **valeurs techniques candidates non acceptées**, non une borne de drainage ; aucune durée de terminaison physique n'est promise. PORT-RUN, REQ/AC 11–13/17. Les contrôles effectifs restent NOT RUN. |

Le design, sa revue indépendante et les réponses produit peuvent progresser ; **aucun plan exécutable** ne peut être déclaré READY avant la fermeture des preuves de faisabilité structurantes. Après DESIGN READY et autorisation de planifier, le `rust_plan_writer` proposera au plus dix tâches reliées aux AC, puis une revue de plan séparée. Un plan accepté n'autoriserait toujours pas le code Rust : le lot et ses chemins nécessitent un accord écrit précis dans la session concernée.

## 8. Attributions et état des contributions

Les profils `rust_product_framer` (PRODUCT-PARA v0.2), `rust_requirements_analyst` (REQ-PARA v0.2), `rust_domain_architect` (DOM-PARA v0.2) et `rust_architect` (ARCH-PARA v0.4) ont contribué **en lecture seule**. La session principale a rapproché leurs IDs et rédigé ce document. Les spécialistes ont reçu des résumés de relais plutôt qu'un fichier Git complet ; leurs paquets bruts indiquent cette limite et aucune comparaison exacte caractère à caractère n'est revendiquée. La racine active des règles `C:\Users\cyril\.codex\agent-rules\` a été résolue après le premier relais ; les copies historiques `whisper-doc-host` n'ont pas été adoptées.

La première revue indépendante du commit `f948e42c` a rendu `BLOCKED` pour DESIGN READY, avec les constats `PARA-REV-01` à `04`. Cette révision documentaire propose la fermeture de `02` à `04` ; seul le reviewer peut la confirmer sur le nouveau commit. `01` reste ouvert jusqu'aux preuves techniques autorisées. Aucun build, test, essai UI, benchmark, SPIKE, téléchargement ou déplacement de modèle n'a été exécuté dans ce cadrage. Résultats produit et faisabilité pratique : **NOT RUN**.

`RAILGUARD.md` doit être lu pour ses frontières actives ; ses passages historiques relatifs au live MP3 ou à la reprise après panne ne réintroduisent pas ces fonctions dans cette évolution. Aucun nouveau railguard, manifeste d'empreintes ou mécanisme documentaire n'est proposé.
