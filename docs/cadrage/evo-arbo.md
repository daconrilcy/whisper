Avancée : les trois chemins de la fenêtre sont identifiés et le parcours demandé est cadré.
Blocage : la fenêtre standard de Windows nécessite une nouvelle dépendance Rust à autoriser avant tout code.
Décision : choisir Q1 à Q4 ci-dessous avant de lancer l'implémentation.

# Évolution « Parcourir et retrouver mes chemins » — proposition du 14 octobre 2026

**Statut : cadrage proposé, aucun code produit autorisé.** Base : `main` / `v0.3.0-gpu` à `994b660a`, branche locale `evo-arbo`. Le présent lot sert l'objectif Whisper Windows et ne modifie ni moteur, ni modèle approuvé, ni choix CPU/GPU. Les décisions recommandées ci-dessous restent à accepter par l'utilisateur.

## Besoin et acceptation dans la fenêtre Windows

L'application montre exactement trois champs de chemin : **Fichier audio (WAV ou MP3)**, **Modèle local**, **Dossier des transcriptions** (`ui.rs`, lignes 249–260). La langue et le calcul ne sont pas des chemins. Les champs actuels sont saisis à la main ; l'import utilise les trois, le micro utilise modèle et sortie. Le modèle sélectionnable reste le **large-v3-turbo `.bin` approuvé** : le contrôle de son empreinte demeure, aucun changement de modèle n'est inclus.

| ID | Scénario et résultat observable (critère utilisateur) |
|---|---|
| AC1 | Depuis chacun des trois champs, ouvrir un dialogue **Windows natif** : fichier unique filtré WAV/MP3 pour l'audio, `.bin` pour le modèle, dossier unique pour la sortie. Le filtre aide à choisir ; la validation existante garde l'autorité. (1) |
| AC2 | Valider un choix : son chemin complet s'affiche dans le bon champ et l'import ou le micro utilise ce chemin. Annuler ou fermer le dialogue : champ et historique restent identiques. La saisie manuelle demeure possible si Q2 est retenue. (2) |
| AC3 | Sans réglages antérieurs : audio vide ; modèle `%LOCALAPPDATA%\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin` ; sortie `WHISPER_L04_NORMAL_RUN_ARCHIVE` si défini et non vide, sinon `%USERPROFILE%\Documents\WhisperTranscriptions`, comme aujourd'hui. (3) |
| AC4 | Après fermeture et réouverture, les trois derniers choix valides réapparaissent, selon Q1 pour l'audio, sans commande « Enregistrer ». Une saisie manuelle terminée suit la même règle que le dialogue. (4) |
| AC5 | Si un chemin enregistré a disparu, le champ concerné revient à son défaut (audio vide), avec un message nommant le champ ; l'application reste utilisable. Si le défaut lui-même manque, le montrer et laisser choisir un autre chemin. (5) |
| AC6 | Rejouer import WAV/MP3 CPU et GPU, micro CPU et GPU, arrêt/fermeture et sorties TXT/SRT/WAV de même nom ; présélection CUDA/CPU et choix manuel fixe pendant la session restent corrects. (6) |

Hors périmètre : autres préférences, glisser-déposer, choix multiple, récents, tray, MP3 en sortie, mode Auto, bascule automatique, supervision, reprise après panne et nouveau modèle.

## Architecture proposée et risques

`eframe/egui 0.35.0` ne fournit ici ni sélecteur Windows ni persistance activée. Selon `RAILGUARD.md` RG-01, l'UI appelle l'application ; la composition injecte les ports de dialogue et de réglages implémentés par les adaptateurs. Le dialogue rend `Option<PathBuf>` : aucune sélection (annulation ou échec natif) ne change le champ. **Option Q4 recommandée** : `rfd` 0.17.2, API Rust de dialogue natif (`pick_file`, `pick_folder`, filtres et dossier initial). Absent de `Cargo.lock`, son ajout direct et ses dépendances Windows demandent un accord précis, même si sa source est en cache local. L'API COM Windows `IFileOpenDialog` exigerait du code système, également soumis à accord.

Stocker **seulement ces trois chemins** dans `%LOCALAPPDATA%\Whisper\settings.json`, JSON version 1 avec `serde_json` déjà utilisé par les adaptateurs, sans droits administrateur. L'application confirme le jeu complet après sélection, Entrée, perte du focus d'une saisie valide ou fermeture normale ; l'adaptateur écrit un temporaire voisin puis remplace le fichier, jamais une frappe partielle. Absence du fichier : AC3 sans alerte ; contenu abîmé/illisible : AC3 avec explication ; écriture impossible : choix courant utilisable et avertissement de non-mémorisation. Au chargement, contrôler existence et type (fichier/dossier) par champ ; un champ absent revient à son défaut avec message, les autres restent valides. Une saisie invalide reste visible pendant la session sans remplacer le dernier choix mémorisé.

Point d'intégration : `root.rs` sonde actuellement CUDA avec le **modèle par défaut** avant de construire `DesktopApp`. Restaurer le modèle effectif avant cette sonde, sinon la présélection GPU/CPU peut refléter le mauvais fichier. Les requêtes import/micro utilisent ensuite les champs affichés ; conserver leurs contrôles et l'empreinte du modèle approuvé. Les dialogues ne s'ouvrent pas pendant un travail actif, comme les champs actuels.

## Plan proposé — 8 tâches, sans autorisation d'exécution

1. Accepter Q1–Q4 et figer les parcours premier lancement, annulation et reprise — AC1–AC5.
2. Ajouter l'adaptateur de dialogues Windows autorisé et les filtres fichier/dossier — AC1.
3. Relier les trois commandes « Parcourir… » aux champs, avec annulation sans effet — AC1–AC2.
4. Définir lecture/écriture des trois chemins dans le profil utilisateur et les défauts existants — AC3–AC4.
5. Traiter fichier de réglages absent, abîmé ou non inscriptible et chemins disparus — AC3–AC5.
6. Restaurer le modèle avant la sonde CUDA et préserver validation/empreinte — AC2, AC6.
7. Tester sélection, saisie, annulation, redémarrage et replis ; compiler hors ligne — AC1–AC5.
8. Vérifier dans la fenêtre les trois parcours et la non-régression import/micro/CPU/GPU/TXT/SRT/WAV — AC1–AC6.

## Décisions attendues

- **Q1, structurante, audio à la réouverture (AC4)** : A réafficher le dernier fichier audio (**recommandé**, respecte le critère 4) ; B champ vide et dialogue rouvert dans son dernier dossier (exception explicite à AC4).
- **Q2, détail, accès au dialogue (AC1–AC2)** : A bouton **« Parcourir… »** près du champ modifiable (**recommandé**) ; B champ cliquable, avec accès clavier à définir.
- **Q3, structurante, premier dossier affiché (AC1)** : A audio dans Documents, modèle dans le parent du défaut, sortie dans le dossier par défaut ou son parent existant (**recommandé**, repli sur emplacement accessible) ; B emplacement habituel choisi par Windows.
- **Q4, structurante, technologie (AC1)** : A autoriser précisément `rfd` 0.17.2 et ses dépendances Windows (**recommandé**, changement manifeste/verrou, disponibilité hors ligne à vérifier) ; B autoriser COM Windows dans le code ; C reporter AC1 et ce lot.

Sources : demande du 14 octobre 2026, `AGENTS.md`, `RAILGUARD.md` RG-01, `ui.rs`, `root.rs`, manifests/verrou ; [API rfd 0.17.2](https://docs.rs/rfd/0.17.2/rfd/struct.FileDialog.html), [IFileOpenDialog Microsoft](https://learn.microsoft.com/en-us/windows/win32/shell/common-file-dialog). Apports en lecture seule : PRODUCT-ARBO-01 v0.2, REQ-ARBO v0.2, ARCH-ARBO v3. `cargo build --offline --locked --workspace` et `cargo test --offline --locked --workspace` réussis sur l'état actuel ; comportements nouveaux non testés.
