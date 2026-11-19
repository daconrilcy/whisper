Avancée : P1 vérifie les identités des modèles et a retrouvé les reçus de deux générations de builds natifs Parakeet.
Blocage : les DLL du runtime personnel diffèrent des DLL des builds répétés ; la génération destinée au paquet dual et ses notices restent à arrêter et à vérifier.
Décision : le 19 novembre 2026, l’utilisateur demande deux modèles intégrés, import et micro, modes CPU/GPU/Auto et trois témoins Dell sans NVIDIA, GTX 1080 Ti et Snapdragon ARM64 sous émulation.

Statut : DESIGN À COMPLÉTER / PLAN CONDITIONNEL. P1 et P2 précisent ressources et contrats ; P3 à P6 demandent un design stabilisé et relu ainsi que leurs autorisations applicables.

## Objectif et référence

Le paquet propose Whisper large-v3-turbo et Parakeet, chacun sélectionnable pour WAV, MP3 et microphone. CPU et GPU manuels et Auto sont accessibles selon le matériel et le moteur effectivement disponibles. Les sorties TXT/SRT et le WAV micro sont vérifiés. Une incompatibilité est expliquée. Le choix manuel GPU ne bascule pas silencieusement sur CPU. L’étude doit identifier les comportements Auto existants pour Parakeet avant de fixer leur contrat.

Le candidat B7/tg7 est validé dans sa portée [L4](cloture-l4-portee-retenue.md). Le desktop Turbo filtre Parakeet et désactive ses workers dans `crates/whisper-desktop/src/root.rs`. Le bootstrap `crates/whisper-bootstrap/src/gui.rs` sélectionne un seul modèle. Le profil personnel utilise d’autres chemins et ressources ; il ne constitue pas un paquet autonome.

Le fichier local `C:/dev/whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf` mesure 713975456 octets ; son empreinte et ses droits sont à vérifier. Une empreinte historique e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e apparaît dans `fiche-parakeet-l3a-gpu.md` sans vérification actuelle. Le modèle Turbo est identifié dans la clôture L4.

## Exigences vérifiables

- E1. Installer les deux modèles et les ressources retenues depuis un paquet autonome ; contrôler leurs identités et signaler toute ressource manquante ou altérée.
- E2. Sélectionner chaque modèle dans l’interface, rendre visibles les modes disponibles et identifier le moteur effectif.
- E3. Sur les témoins compatibles, transcrire WAV et MP3 avec chaque modèle et chaque mode utilisable ; vérifier contenu, TXT et SRT.
- E4. Pour chaque modèle et mode utilisable, enregistrer le microphone, observer du texte confirmé avant arrêt puis vérifier la phrase finale et les fichiers WAV/TXT/SRT.
- E5. Sur le Dell sans NVIDIA, CPU et Auto fonctionnent et GPU manuel signale son indisponibilité ; sur la GTX 1080 Ti, vérifier GPU et Auto selon les capacités effectives de chaque modèle.
- E6. Sur Snapdragon Windows ARM64, identifier les processus x64 émulés et vérifier les parcours applicables ; proposer le pack graphique Microsoft si nécessaire. Aucun GPU NVIDIA n’est supposé présent.
- E7. Attribuer au candidat exact les identités, résultats, licences et notices ; distinguer essais manuels, automatiques et historiques.
- E8. Préserver les résultats Turbo acceptés et vérifier leur non-régression sur le nouveau candidat. AMD, anciens x64 sans AVX et pannes Auto provoquées restent différés selon les décisions L4.

Pour chacun des deux modèles, la matrice minimale comprend sur Dell et Snapdragon CPU et Auto pour WAV, MP3 et micro, ainsi que le refus explicite du GPU manuel. Sur GTX 1080 Ti, elle comprend CPU, GPU manuel et Auto pour ces trois entrées. Une impossibilité constatée sur une cellule obligatoire exige une décision utilisateur sur la portée avant clôture.

## Risques et architecture à établir

Les DLL natives Parakeet historiques ont été construites avec `GGML_NATIVE=ON` et des instructions AVX2/FMA/F16C selon `docs/architecture/BUILD-NATIF-QUALIF-PTX.md`. Leur adéquation aux trois machines, notamment Snapdragon émulé, est inconnue. Les reconstructions natives sont suspendues par AGENTS.md ; toute reprise exige une décision précise et la voie prévue. L’absence de preuve de compatibilité n’établit pas une incompatibilité.

Une composition possible réutilise la racine WhisperTurbo avec desktop et workers Turbo, workers Parakeet, runtimes CPU/GPU distincts, deux modèles et notices. Son contrat exact, l’isolation des DLL et les dépendances transitives sont à vérifier. Le GPU Snapdragon ne vaut pas GPU NVIDIA. Les chemins, identités, versions et droits des workers, DLL et modèles doivent être établis avant choix des ressources. Si une condition nécessaire manque, le lot concerné rend le résultat non exécuté et soumet une solution précise.

Les fichiers modifiés ou non suivis du worktree sont à préserver. Une nouvelle composition et un nouveau bootstrap changent les identités et ne sont pas qualifiés par les seuls essais historiques. Aucun seuil de performance Parakeet n’est ajouté sans décision.

## Plan proposé, sous réserve des preuves de faisabilité

1. **P1 — Inventaire et faisabilité (E1,E2,E5–E7)** : identifier modèles, workers, DLL, empreintes, licences et chemins ; comparer leurs prérequis aux témoins. Soumettre une solution si une reconstruction suspendue est nécessaire.
2. **P2 — Contrats et préparation (E1,E2,E5,E7)** : définir le paquet, l’installation, le catalogue, les erreurs, Auto, les identités et les commandes gardées. Soumettre les changements éventuels de manifeste.
3. **P3 — Composition et installation (E1,E2,E5,E8)** : adapter desktop, bootstrap et ressources selon le contrat accepté ; couvrir modèles absents/altérés, sélection, disponibilité et premier lancement en profil vide. Prévoir revue et vérifications gardées.
4. **P4 — Assemblage et droits (E1,E7)** : intégrer les deux modèles et dépendances admises, vérifier contenus, empreintes, licences et notices, puis figer le candidat.
5. **P5 — Qualification matérielle (E2–E8)** : recette utilisateur sur Dell, GTX et Snapdragon ; vérifier WAV, MP3 et micro, CPU/GPU/Auto applicables, exports et moteur effectif. Les cas matériellement impossibles sont explicités.
6. **P6 — Bilan et remise (E7,E8)** : consolider résultats et limites par empreinte, traiter les écarts et préparer la décision de publication.

Ce plan est proposé, sans implémentation ni campagne autorisées par sa seule rédaction. Les inconnues de P1 peuvent imposer une modification du design avant P2. La distribution publique fait l’objet d’une décision distincte.

P1 produit l’inventaire attribué des ressources et licences et une matrice de compatibilité accompagnée des preuves ou inconnues. P2 fournit les contrats d’installation, de sélection, d’Auto, de résolution des DLL et d’identité, ainsi que les prérequis, chemins et contrôles précis des lots suivants. Une revue indépendante du design stabilisé précède la mise en exécution des lots P3 à P6.

## Résultat P1/P2 — contrat proposé pour les lots suivants

P1 est partiellement fermé : les deux modèles locaux correspondent à leurs tailles et SHA de référence, et les builds natifs conservés ont des sources et options épinglées. Les licences/attributions des deux modèles sont identifiées. Les DLL moteur des builds reproduits diffèrent de celles du runtime personnel qui a servi de référence ; leur compatibilité sur les trois témoins et leur adoption au candidat public ne sont pas établies. L’inventaire des notices montre les composants nécessaires, mais pas leur présence dans un paquet dual final. Aucun de ces fichiers ne devient une entrée approuvée par sa seule présence dans le runtime personnel ou dans les reçus de build. Détails : [inventaire P1](inventaire-parakeet-distribution.md), [constat CRT](constat-runtime-parakeet-distribution.md) et [provenance des builds conservés](constat-provenance-builds-parakeet.md).

### Paquet et installation

- **Format proposé conservé :** installateur Rust par utilisateur, sans élévation, raccourci ni inscription système par défaut, selon les décisions déjà consignées dans l’[inventaire Turbo](inventaire-contrat-installation-turbo.md). Le paquet accompagne toutes ses ressources hors ligne ; il ne demande ni chemin de modèle ni téléchargement de modèle. Les deux modèles pèsent ensemble 2 338 530 731 octets avant l’archive. La taille de l’archive livrée et l’espace temporaire requis seront mesurés sur le candidat final.
- **Racine :** `%LOCALAPPDATA%\WhisperTurbo`. Proposition d’arborescence : `app/` pour le bootstrap, le desktop et les workers Turbo, avec les DLL CUDA Turbo redistribuables à côté du worker GPU, conformément au contrat L3 Turbo déjà accepté ; `worker/` pour les workers Parakeet ; `models/` pour les deux modèles ; `native/cpu/` et `native/gpu/` pour les runtimes Parakeet correspondants ; `settings/` et `logs/` dans la racine utilisateur. Le worker Parakeet doit résoudre ses DLL depuis son runtime explicite sous `native/cpu` ou `native/gpu`, sans dépendre du répertoire courant ni du `%PATH%` global. La séparation par processus évite les collisions de noms entre les deux runtimes GPU. La recette doit encore démontrer que la composition respecte ces chemins et ne charge aucune DLL d’un autre moteur. Les sources audio et les exports restent à leurs emplacements choisis.
- **Préflight :** avant toute mutation durable, vérifier architecture/CPU et OS, disponibilité du volume et accès à `LOCALAPPDATA`, empreintes et tailles du paquet, puis l’initialisation effective du moteur graphique. Sur ARM64, si le Microsoft OpenGL/OpenCL/Vulkan Compatibility Pack nécessaire est absent ou si Glow ne démarre pas, afficher le dialogue graphique validé : ouvrir sa page Microsoft Store sur demande, permettre de revenir et revérifier, ou annuler sans installation. Le Store pack reste un prérequis Microsoft, pas un fichier embarqué.
- **Installation absente :** copier vers une zone temporaire créée par l’installateur sur le même volume, vérifier l’ensemble des empreintes copiées, puis publier la racine seulement après réussite. Une ressource obligatoire absente/altérée fait échouer l’installation entière avec cause ; aucune édition partielle ne doit se déclarer installée. Le comportement des temporaires et l’engagement atomique seront précisés et testés en P3.
- **Installation déjà présente :** l’utilisateur a déjà demandé que le bootstrap remplace une installation existante ; cette préférence est appliquée au contrat de distribution. `%LOCALAPPDATA%\WhisperTurbo` existe et contient aujourd’hui Turbo seulement. P3 remplacera automatiquement les ressources applicatives par la nouvelle génération duale après fermeture normale du desktop et sauvegarde complète, vérifiée avant mutation. Les réglages compatibles seront préservés/migrés ; si la sauvegarde, la migration ou le retour arrière échoue, l’ancien état sera restauré et l’installation déclarée en échec. Aucun arrêt forcé de processus. La préparation, les échecs injectés, la restauration et la conservation des données seront vérifiés avant recette utilisateur ; un succès ne sera annoncé qu’après contrôle complet du nouvel ensemble.

### Modèles, catalogue et routage

| Modèle | Fichier installé et identité | Source / notice | Calculs à exposer |
|---|---|---|---|
| Whisper large-v3-turbo | `models/ggml-large-v3-turbo.bin`, 1 624 555 275 octets, SHA-256 `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` | Modèle GGML rattaché à la révision whisper.cpp connue et MIT déclarée ; conserver attribution/licence dans les notices. | CPU, GPU manuel et Auto selon la disponibilité runtime CUDA/pilote. |
| NVIDIA Parakeet TDT 0.6B v3 q8_0 | `models/parakeet-tdt-0.6b-v3.q8_0.gguf`, 713 975 456 octets, SHA-256 `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e` | NVIDIA, révision `541d1f99…`, CC BY 4.0 ; notice avec attribution, lien/source, licence et modifications si applicables. | CPU, GPU manuel et Auto selon l’identité de son worker/runtime et la disponibilité NVIDIA. |

Les identités ci-dessus sont acceptées en entrée de conception, pas comme SHA des copies d’une archive future. L’application expose deux identifiants de modèle fermés ; elle ne lit pas un chemin modèle arbitraire. Les deux modèles doivent passer la vérification d’intégrité avant que l’installateur annonce le paquet prêt. Au runtime, chaque modèle est admis séparément par l’identité du fichier et de son moteur. Un modèle absent ou altéré est grisé avec une cause ; il ne peut entraîner ni passage silencieux à l’autre modèle ni changement de préférence. Une demande invalide est refusée avant admission audio.

- CPU manuel fixe le worker CPU du modèle sélectionné pour toute la transcription.
- GPU manuel fixe le worker GPU correspondant. Une preuve d’identité ou un pilote CUDA manquant donne un refus français expliquant l’indisponibilité et le choix CPU/Auto ; aucune transcription CPU n’est lancée à sa place.
- Auto évalue avant chaque nouvelle transcription la disponibilité et l’identité GPU du modèle choisi. Il privilégie le GPU correspondant s’il est admissible, sinon CPU. Si l’initialisation GPU échoue avant le début du traitement audio, une seule tentative CPU du même modèle est permise ; après le premier audio traité, aucun repli n’est permis. Cause et moteur effectif restent visibles. Un changement vers Turbo ou Parakeet n’est jamais un repli.
- Le routage Auto est ainsi spécifique au modèle, et non une simple détection globale d’une carte. Snapdragon Adreno et les adaptateurs virtuels ne satisfont pas l’identité NVIDIA ; le parcours Auto utilise donc le worker CPU Parakeet/Turbo selon le modèle choisi.
- Les workers se chargent dans des processus distincts. Les DLL Parakeet CPU et GPU restent séparées par runtime ; aucun choix ne dépend du répertoire courant ou de l’ordre global `%PATH%`. Avant livraison, l’implémentation doit établir un chemin de résolution privé et vérifiable pour chaque worker, puis contrôler imports statiques et retardés, modules effectivement chargés, versions et SHA. Le desktop ne charge pas les DLL moteur au nom d’un autre worker.

L’existant décide que Parakeet dispose de transcription multilingue automatique après qualification de son artefact/runtime exact, tandis qu’un choix manuel de langue n’est pas admis sans preuve de son application réelle. Le présent lot conserve ce contrat historique ; il ne promet pas un menu de langue Parakeet tant qu’une capacité manuelle n’est pas démontrée. L’utilisateur a déjà validé la qualité générale par les recettes successives ; aucune nouvelle campagne comparative ni seuil de performance n’est ajouté ici.

### Identités, notices et contrôles prévus

1. L’assemblage P4 part d’un manifeste explicite avec chemin relatif, rôle consommateur, version/source, octets attendus, SHA-256, licence/notices, résolution et cible. Le bootstrap, desktop, quatre workers, les deux modèles et chaque DLL du candidat final sont épinglés ; le modèle n’est pas déduit du nom seul.
2. Chaque worker vérifie son modèle et son runtime avant `Ready`, puis associe sa clé de runtime au hash de son exécutable, modèle exact, profil CPU/GPU, DLL natives et backend/device requis. Une variation d’un composant invalide la preuve et redéclenche le préflight ; toute divergence ferme l’opération avant admission audio.
3. Les contrôles statiques couvrent le manifeste contre les fichiers du ZIP et de l’installation, imports PE normaux/delay-load, fichiers chargés à l’exécution, répertoires de résolution, VC Runtime, CUDA Runtime et dépendances Windows externes. La recette installée vérifie qu’un poste CPU sans SDK de développement peut démarrer et transcrire ; l’absence de pilote NVIDIA ne bloque ni l’installation ni CPU/Auto.
4. Le lot Rust suivant utilisera exclusivement les commandes admises par les gardes existantes : `ci/guard-level1.ps1` pour SelfTest, N1 et Clippy ; profils GPU/N2 via `ci/guard-native.ps1` uniquement après préflight et autorisation propre ; contrôle PE via `ci/check-worker-imports.ps1` ; aucune commande Cargo directe, aucune reconstruction native tant que la suspension WHEA n’est pas levée par décision spécifique. Les profils et racines exacts devront être établis avant le mandat du lot. Aucun de ces contrôles n’est exécuté par le présent P1/P2.
5. La recette Windows T33 observe le vrai premier lancement de ce paquet exact sur profil vierge. P5 reprend ensuite la matrice E3–E6 pour les deux modèles et les entrées WAV, MP3 et micro, sur Dell, GTX 1080 Ti et Snapdragon. Les validations manuelles historiques ne sont pas transférées à l’identité du nouveau candidat.

### Changements de manifeste et décisions nécessaires

- **Aucun changement de `Cargo.toml` ou `Cargo.lock` n’est justifié à ce stade** : les crates et workers Parakeet existent déjà. P3 vérifiera les feature sets réellement nécessaires ; si aucune dépendance nouvelle n’est requise, ils resteront intacts.
- **La garde N1 requiert probablement un changement versionné** : `ci/guard-level1.ps1` possède une liste fermée de cibles fraîches Turbo ; le lot dual aura besoin d’une cible dédiée non réutilisée, avec son SelfTest positif/négatif et les signatures exactes admises. Si un nouvel essai natif est requis plus tard, `ci/native-profiles.json` et `ci/guard-native.ps1` devront aussi être modifiés et relus avant leur exécution. Ce sont des changements de garde, non des manifestes Cargo, soumis au mandat du lot concerné.
- P3 ne peut pas commencer tant que les DLL dont la redistribution reste inconnue ne sont pas reliées à leur build et à leurs conditions. Les sources binaires historiques sous `target/` sont absentes dans l’environnement vérifié ; il faut établir si des copies exactes avec preuves existent dans une archive conservée, ou proposer la voie de restauration. Une reconstruction de DLL demeure suspendue après WHEA et ne sera pas substituée sans mandat précis. Les droits et notices du candidat final sont une gate d’assemblage, pas un avertissement reportable après publication.

Les décisions de portée acceptées sont déjà intégrées : deux modèles fournis dans le paquet, import/micro, CPU/GPU/Auto, Dell/GTX/Snapdragon, AMD différé, aucune CI Snapdragon requise, tests de panne Auto mis hors scope, qualité considérée validée sans essai comparatif additionnel, remplacement d’une installation existante avec sauvegarde/retour arrière vérifiés. Avant P3, il reste à fermer les preuves de redistribution native et à retrouver les sources binaires exactes ou faire valider une solution de restauration compatible avec la suspension de build.
