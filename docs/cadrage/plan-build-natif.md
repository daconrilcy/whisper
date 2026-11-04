Avancée : plan BUILD-NATIF établi le 4 novembre 2026 avant archivage, inventaire et compilation.
Blocage : préflight de phase 0, chemin historique absent ; aucune phase suivante engagée.
Décision : retrouver la chaîne historique avant reprise ; architecture proposée ci-dessous à valider avant écriture native.

# Plan BUILD-NATIF

Objectif : reconstruire les DLL ggml/NeMo effectivement livrées, avec provenance figée et équivalence démontrée. Base : main `b9935b2422d47350cdd5b368265afa84e12d42ab`, branche `build-natif`, worktree `C:/wt/build-natif`. Budget global : 4 heures de travail, deux cycles revue/correction maximum (plus restrictif que les trois proposés). Aucun Rust produit modifié, fusion ni push automatique.

## Périmètre et protections

Lecture seule absolue de `C:/dev/whisper/target/parakeet-feasibility/`. Aucun remplacement dans `%LOCALAPPDATA%/Whisper`, aucune suppression, notamment de `target/poll-preserved/` et de l'archive. L'absence du chemin historique ne permet pas de conclure à sa suppression. Contrôler espace libre (au moins 30 Gio avant compilation), liens, racines et activités concurrentes pertinentes. Utiliser les gardes existantes pour Cargo/N2 ; ne pas en construire de nouvelles pour contourner un blocage. Pas de compilation avant phase 3.

## Phases et points d'arrêt (cinq tâches)

1. **Sauvegarde avant inventaire.** Vérifier la disponibilité de spike02, puis archiver tout son contenu de compilation, y compris fichiers cachés et `.git`, sous `C:/dev/whisper-native-archive/spike02-20261104.zip`, sans écraser un existant. Manifeste par chemin relatif, taille et SHA-256 ; vérifier chaque entrée décompressée, son empreinte, sa taille et le nombre total. Si archive supérieure à 5 Go, exclure seulement `.obj` et `.pdb`, avec liste exhaustive et tailles ; ne pas exclure les autres éléments. Refuser les liens non résolus sans les suivre à l'aveugle. Arrêt et compte rendu court si source absente, fichier illisible, élément manquant ou archive non vérifiée.
2. **Inventaire en lecture seule après sauvegarde vérifiée.** Relever fichier:ligne ou commande:sortie pour origine/commit complet llama.cpp, sources NeMo et modifications locales, tous paramètres CMake CPU/GPU, runtime MSVC, outils/SDK/CUDA, provenance des redistribuables CUDA et licences. Relever SHA-256, tailles, imports/exports de chaque DLL installée. Signaler immédiatement GGML_NATIVE et instructions propres au CPU hôte, sans les corriger. Arrêt si commit llama.cpp ou sources NeMo introuvables ; ne pas inventer leur provenance.
3. **Script sans exécution native.** Après validation de l'architecture concrète issue de l'inventaire, versionner `native/CMakeLists.txt`, les sources NeMo si locales et les correctifs nécessaires sous `native/patches/`, puis `ci/build-native.ps1`. Vérifications strictes des versions, échec clair, une commande par variante, sorties `target/native-build/<cpu|gpu>/`, manifestes options/versions/empreintes et déterminisme MSVC explicite. Télécharger uniquement dans un cache persistant déclaré `native/cache/` ignoré par Git dans le worktree ; aucune source de référence sous target. Revue indépendante en lecture seule du script avant lancement ; arrêt sur constat bloquant concret. Ne pas créer de script factice avant provenance établie.
4. **Équivalence statique (phase 3 utilisateur).** Compiler deux fois chaque variante depuis des répertoires de construction distincts sous la racine autorisée, sans réutiliser les DLL de la première compilation. Comparer SHA-256, puis exports/imports avec l'installation, absence de CUDA CPU (contrôle PE existant), tailles (expliquer tout écart supérieur à 2 %) et options effectives. Conserver les options historiques ; ajouter /Brepro seulement si compatible, documenter toute différence et son effet. Arrêter sur échec au lieu de modifier les options pour faire passer le contrôle.
5. **Équivalence fonctionnelle et clôture (phase 4 utilisateur).** Runtime N2 alternatif sous le worktree, copie du runtime épinglé avec seules DLL ggml/NeMo remplacées ; sélection par WHISPER_N2_PERSONAL_RUNTIME_ROOT. Maximum huit essais N2 au total, références comprises : deux PersonalAutoFallback/en-short, deux imports CPU français, deux imports GPU anglais si candidat attesté, deux réserves. Comparer TXT/SRT/WAV par SHA-256, assertions et durées par mode. N1 via garde existante : nombres et résultats comparés à main ; aucun succès déduit de l'absence de changement Rust. Revue indépendante des conclusions, documentation et bilan figé ; proposer seulement le nettoyage. Arrêt si runtime, candidat, garde ou entrées nécessaires indisponibles.

## Choix de source llama.cpp (trois options maximum)

| Option | Intérêt | Limite |
| --- | --- | --- |
| FetchContent, SHA complet | Proche d'une chaîne CMake historique si confirmée | Nécessite accès dépôt et contrôle des modifications locales |
| Archive téléchargée, SHA-256 vérifié | Contenu figé, vérification simple et cache durable | Nécessite prouver sa correspondance au commit et aux correctifs |
| Sous-module Git | Révision visible dans Git | Complexifie les checkouts et les correctifs locaux |

Recommandation provisoire : archive vérifiée conservée dans le cache persistant, commit complet et correctifs versionnés séparément ; décision finale après inventaire pour reproduire la chaîne réelle. Aucun téléchargement avant cette identification.

## Livrables et acceptation

Rapports : ce plan, `docs/architecture/NATIVE-BUILD.md`, `docs/architecture/BUILD-NATIF-BILAN.md` (trois fichiers, sous le plafond de dix). La session principale écrit et commite les documents. NATIVE-BUILD décrit provenance, commandes, licences et contrainte GGML_ASSERT au rechargement dans un même processus ; ne pas présenter cette contrainte comme reproduite sans essai. AGENTS.md reçoit la règle de production par script uniquement lorsque ce script est livré, avec maintien de conservation jusqu'à décision explicite après fusion et preuve d'équivalence.

Verdict final : REPRODUIT, ÉQUIVALENT AVEC ÉCARTS EXPLIQUÉS ou NON REPRODUIT, justifié par les preuves réellement acquises. Aucun accord de suppression déduit du verdict. Si arrêt anticipé, bilan court avec tableaux marqués NON EXÉCUTÉ ; les faits ultérieurs vont dans un nouveau document, sans réécriture du bilan figé.
