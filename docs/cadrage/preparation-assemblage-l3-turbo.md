# Préparation de l'assemblage L3 Turbo

Avancée : orientations de livraison acceptées et tâches T31–T33 précisées pour le candidat Turbo.
Blocage : gate L2 ouverte, droits et ressources finaux incomplets, primitive de publication à établir.
Décision : l'implémentation et les campagnes L3 attendent un mandat de lot après fermeture des prérequis.

Ce document prépare L3 à partir du [plan de distribution](plan-implementation-distribution-cpu-x64.md) et de l'[inventaire des ressources](inventaire-contrat-installation-turbo.md). Il ne déclare pas L2 terminé. État de travail : `C:/wt/distcpu-l2-reprise`, base L1 `26da7b6e`, changements T21/T22 non commités. Les candidats desktop et GPU compilés sont des entrées de travail ; les octets à livrer ne sont pas figés.

## Entrées et prérequis de L3

1. Fermer la gate L2 prévue : composition Turbo revue et vérifiée, worker CPU et GPU release identifiés, imports et options de compilation attribués à leurs PE, N1 et Clippy applicables réussis ; relier le desktop final aux références de workers selon le contrat retenu. La preuve de reconstruction après changement du marqueur et des références reste ouverte.
2. Établir la liste exacte des dépendances par PE, y compris transitive et dynamique, ainsi que les licences, notices et conditions de redistribution. Les DLL CUDA locales repérées et les fichiers Visual C++ restent des candidats, pas des ressources approuvées. Confirmer les droits du code propre et du distributeur. Une ressource ou un droit non établi bloque l'assemblage.
3. Le modèle local et le commit de conversion publié correspondent en taille et SHA-256 ; voir le [rattachement de la conversion](inventaire-contrat-installation-turbo.md#rattachement-de-la-conversion-du-modèle). Prévoir ses mentions de licence et le protocole de sélection locale. Le modèle est fourni séparément, hors ligne.
4. Confirmer le support des versions Windows Home/Pro à la publication et les prérequis GPU. Les essais matériels v2, ARM64 et l'inférence GPU sont différés selon L4 ; ne pas leur attribuer de PASS.

## Ordre de réalisation proposé

### T31 — Entrée et compatibilité

Dans `crates/whisper-bootstrap/`, remplacer le point d'entrée actuellement minimal par le parcours d'installation autorisé. Réutiliser la politique CPUID existante dans `whisper-core` et la lecture adaptée dans `whisper-adapters`. L'entrée d'installation est construite pour x86-64 v1, sans dépendance d'exécution au worker ou au modèle avant le préflight ; son PE et ses imports sont vérifiés. Conserver `forbid(unsafe_code)` et demander une décision précise si une primitive système ou une dépendance nouvelle devient nécessaire. Un processeur sous v2 ou un état indéterminé produit un message distinct et aucun effet durable.

### T32 — Transaction d'installation et refus

Préparer dans un répertoire temporaire identifié, sur le volume de `%LOCALAPPDATA%`, l'arborescence `app/` et `models/` avec les exécutables, DLL locales et notices retenus. Copier le modèle choisi par l'utilisateur et vérifier le SHA de la copie avant engagement. Vérifier racine finale absente à l'entrée et publier sans écraser une racine apparue concurremment. La primitive de publication et son comportement en concurrence doivent être démontrés avant de choisir une implémentation ; l'absence de primitive conforme interrompt cette tâche. Ne pas créer artificiellement les réglages ou journaux que le produit crée lui-même.

Couvrir avec des tests du vrai parcours décisionnel : CPU indéterminé, CPU refusé, modèle absent ou erroné, ressource altérée sur copie identifiée, annulation avant engagement et conflit de racine simultané. Comparer les états avant/après et vérifier les messages. Un échec après engagement est visible sans promettre un rollback hors périmètre. Le nettoyage ne vise que le temporaire dont le processus établit la propriété.

### T33 — Assemblage et première utilisation

À partir des artefacts finaux identifiés et des droits clos, préparer un candidat unique contenant installateur, desktop, deux workers, DLL autorisées et notices ; le modèle est sélectionné séparément hors ligne. Retenir les empreintes des artefacts et la configuration de build dans les preuves prévues du lot, sans inventer de manifeste de qualification supplémentaire. Vérifier les imports de chaque PE et les résolutions des DLL, y compris les DLL transitées depuis CUDA. Sur un profil Windows propre au parcours de recette, vérifier la création effective des réglages/journaux par le produit, le démarrage et l'inférence WAV CPU sans dépôt ni SDK CUDA ou outils de développement. La fenêtre et la recette finale utilisateur restent sous son contrôle quand le plan le prévoit.

## Vérifications et critères de sortie

Avant chaque campagne, relever branche, diff, espace libre ≥ 30 Gio, ressources actives et racines autorisées ; exécuter le SelfTest et les commandes Cargo applicables via `ci/guard-level1.ps1` suivant les signatures autorisées. Les catégories attendues sont format, tests ciblés T31/T32, N1 complet et Clippy structurel, builds séparés v1 et Turbo, puis contrôles PE/imports. Les commandes et cibles exactes dépendent de la conception et des profils de garde qui devront être arrêtés et relus avec le lot ; elles ne sont pas encore une recette exécutable.

Gate L3→L4 : les trois tâches sont exécutées, le candidat et ses ressources sont identifiés, les droits sont fermés, les refus avant mutation et le premier lancement sont démontrés. L'absence d'un prérequis ou d'un résultat laisse la gate ouverte. Les essais utilisateur, v2 exact et Windows ARM64 conservent leur statut et leurs conditions L4.

## Vérifications restantes avant assemblage

| Vérification | État et condition de fermeture |
|---|---|
| Dépendances Rust | [Paquets compilés](dependances-compilees-turbo.md) et [graphe des entrées Rust](entrees-rust-candidats-turbo.md) disponibles pour les candidats desktop et GPU ; ce graphe ne prouve pas la présence de chaque paquet. Les quatre polices du desktop candidat sont directement retrouvées dans son PE avec leurs notices candidates. Établir la composition effective ou retenir un ensemble conservateur de notices et respecter toutes ses obligations ; répéter l'analyse sur chaque PE final. Worker CPU et bootstrap finaux encore non identifiés. |
| Code natif et DLL | Rattacher `whisper-rs-sys 0.15.0` (licence déclarée Unlicense) à la version de `whisper.cpp`/GGML et aux licences incluses dans cette version. Contrôler les DLL candidates CUDA et Visual C++, imports transitifs et chargements dynamiques, versions, redistribution et résolution sur machine propre. |
| Modèle | Le fichier local correspond en taille et SHA au commit de conversion GGML publié sous licence MIT. Le chemin historique d'acquisition n'est pas connu ; préparer les mentions et fournir le modèle séparément hors ligne. |
| Identité des PE | Arrêter les PE CPU/GPU/desktop finaux, SHA et commandes ; établir l'attribution des références de workers au desktop et la reconstruction après changement des entrées. Une identité relue sur le seul chemin ne prouve pas l'image effective. |
| Contrôles et droits | Vérifier les notices et licences réellement applicables, y compris les licences alternatives ou cumulatives et polices ; confirmer les droits du code propre et la licence du distributeur Visual Studio. Les déclarations de `Cargo.toml` ne suffisent pas à fermer le point. |
| Assemblage | Revoir la primitive de publication sans écrasement, les effets du bootstrap et les commandes gardées avant le mandat L3. Ne pas assembler tant que les ressources et droits obligatoires ne sont pas clos. |

L'[index des notices candidates](index-notices-turbo.md) relie les sources examinées aux graphes des trois PE. C'est un ensemble prudent de travail, non un recueil final ni une preuve d'incorporation de chaque dépendance. Les textes applicables, leurs copyrights, les composants natifs et les conditions des DLL restent à collecter et vérifier.

Les notices AccessKit et Chromium et les éléments sans fichier racine disposent désormais d'un [complément préparé](notices-complementaires-turbo.md). Les archives Symphonia et Rust sont disponibles dans l'index. Avant l'assemblage, confronter cet ensemble aux composants effectivement remis, vérifier les éventuels fichiers `NOTICE` amont, les copyrights et licences des composants natifs, ainsi que les conditions des DLL. Les licences alternatives présentées dans le complément sont des choix de préparation à appliquer et vérifier dans la remise, pas une preuve que toutes les obligations sont déjà satisfaites.

### Composition des PE : écart à fermer

Le worker CPU L1 existant dans `C:/wt/distcpu/target/parakeet-tests/cpu-distribution-x64-20261115/targets/cpu-distribution-release/release/whisper-worker-cpu.exe` a pour SHA-256 `03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a`. Le desktop Turbo actuel `d052adba43459ce525d57c8da2d1eb45c4c189cf1871229e803a39baf66ce344` ne contient pas la représentation ASCII de cette empreinte : on ne peut pas présenter ce couple comme une composition dont le relai CPU est attribué au worker L1. Cette recherche ne démontre rien sur d'autres couples possibles.

La garde a été adaptée et relue pour admettre aussi la cible fraîche `turbo-edition-final` avec l'empreinte CPU ci-dessus ; elle refuse la suppression de cette empreinte par `-UnsetEnv`. SelfTest PASS après fermeture du constat de revue ; build gardé release Turbo code 0 avec `RUSTFLAGS=-C target-cpu=x86-64`. La cible existante a été conservée. Le nouveau desktop a pour SHA-256 `aca1b94b230b14d21ea73f89f10d464b241f5a5f9c28caed7849f9c701bba414` : sa table PE et celles du worker CPU L1 `03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a` et du worker GPU `757a7bd4c874fe076ed109dd176702a11ec3bd7ea0bc532be954a64ed92bcef8` passent le contrôle AMD64/imports. Le desktop contient l'empreinte CPU en ASCII ; les fichiers `.d` attestent les entrées de compilation du marqueur Turbo et de l'empreinte. Ceci constitue une sélection de travail, pas un paquet L3 figé ni une preuve d'inférence. Le build a émis trois avertissements `dead_code`. Vérifier encore l'invalidation après changement d'entrée et rattacher la composition et les notices à chacun des PE retenus. Le relai de diagnostic GPU reste désactivé tant que son identité n'est pas établie, selon la décision T22 ; il ne faut pas l'annoncer validé.

La preuve de build est actuellement dans `target/parakeet-tests/cpu-distribution-x64-20261115/scratch/20261116T205954467-61598cad46ae4f388fb72519588339df/` ; elle doit être archivée selon les règles du lot avant clôture et ne vaut pas preuve pérenne sous `target/`.

### Invalidation de l'empreinte CPU

Le 16 novembre, après SelfTest PASS, deux builds release gardés du desktop **générique**, avec le même profil `-C target-cpu=x86-64` et la même cible isolée `cpu-digest-invalidation`, ont été exécutés successivement. Le premier, avec l'empreinte CPU réelle, a produit le PE `d01785f668bbd5744dd6ea0636ebf90f65cd3a35f2facf810ad786a7e21876ac`, qui contenait une fois cette empreinte. Le second, avec l'empreinte synthétique de 64 chiffres `1`, a produit `f3373bfe2aae2fd5d82ed1e41a98f2a0fde2a84ed43c48f48fd0d7e7af07fd7e` : il contient une fois la valeur synthétique et plus la valeur initiale. Le journal Cargo signale explicitement que la variable `WHISPER_EXPECTED_CPU_WORKER_SHA256` a changé, puis la reconstruction de `whisper-adapters` et de `whisper-desktop` ; le fichier `.d` porte la nouvelle valeur. Les deux builds ont fini avec code 0 et trois avertissements `dead_code`.

Ce résultat établit l'invalidation de cette entrée dans ce parcours de build générique ; il ne constitue pas une reconstruction différentielle du profil Turbo, que la garde n'admet qu'en cible fraîche. Le PE synthétique est un témoin, pas une ressource distribuable. Journaux temporaires : premier build `scratch/20261116T210636050-ab780bde168f4086ac510e37fedd1837/`, second `scratch/20261116T210804453-e37748509ed64b3cbbcdfefa32eb041d/`, sous le lot CPU. À archiver avant clôture selon la rétention.
