Avancée : cadrage et plan proposés pour une édition publique Whisper et Parakeet.
Blocage : identité et portabilité des ressources natives Parakeet et droits du modèle à établir.
Décision : le 19 novembre 2026, l’utilisateur demande deux modèles intégrés, import et micro, modes CPU/GPU/Auto et trois témoins Dell sans NVIDIA, GTX 1080 Ti et Snapdragon ARM64 sous émulation.

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
