Avancée : sources et premières dépendances natives inventoriées ; protections préparées pour le profil portable.
Blocage : exécutable vcpkg épinglé, fermeture des entrées et contrôle réseau/ISA complet non établis.
Décision : maintenir le refus du build portable ; aucun téléchargement ni build pendant cette préparation.

## Entrées identifiées

Le cache source `C:/dev/whisper/native/cache/` contient les dépôts NeMo `6a3ca369370782acee0dd155e34394ba60b920c4`, llama `bd4f514db14d87fded667787a7a963bfbaa98e89` et vcpkg `9e593bb18ea69cc5095e012465dcd675a822ed0d`, avec fichiers suivis sans modification constatée. `C:/wt/distcpu-l2-reprise/native/cache` reste absent. Les patches locaux prévus sont `native/patches/ggml-pinned-build-commit.patch` et, pour GPU uniquement, `ggml-cuda-stable-seeds.patch`.

Le manifeste NeMo ASR demande SentencePiece 0.2.1. Les manifests de ports inspectés indiquent également Abseil 20260107.1, Protobuf 6.33.4, utf8-range 6.33.4 et les ports hôte vcpkg-cmake et vcpkg-cmake-config. Il s’agit de dépendances déclarées, pas d’une preuve que les objets correspondants sont liés à une DLL finale.

| Archive dans `C:/dev/whisper/native/cache/downloads/` | SHA-512 vérifié, conforme au port |
|---|---|
| `google-sentencepiece-v0.2.1.tar.gz` | `012850b63b2323e16acc5dacc0a494ad3f6375425ee86274f0946032e47c088a3b307758b99d752fcf54acf76c82d7d13d0c14bbf07aa9b612c4f1fbd30cf1cf` |
| `abseil-abseil-cpp-20260107.1.tar.gz` | `f5012885d6b6844a9cf5ed92ad5468b8757db33dfe1364bfb232fff928e06c550c7eb4557f45186a8ac4d18b178df9be267681abab4a6de40823b574afbe9960` |
| `protocolbuffers-protobuf-v33.4.tar.gz` | `540059a93721447cf4723bcca06e91c43a4399cb366c05bf84e9d8e2c439f3107ba17803f9d912549b54c471f2dcc4c9fc834145ec441dff31ca24f9a3543aa9` |

Le port utf8-range pointe sur la même archive Protobuf. Le cache comprend aussi 7z, Ninja et deux archives MSYS2 ; leur nécessité, identité attendue et usage restent à rattacher à la chaîne exacte. La liste ci-dessus ne prouve pas la fermeture de tous les outils, ports et téléchargements possibles.

`vcpkg.exe` attendu SHA-256 `13b8175e99a884c5ad34249218754b45541a1a63f216e92603aee57a285ac741` reste introuvable dans les racines examinées. Le `bootstrap.ps1` de vcpkg épinglé désigne `https://github.com/microsoft/vcpkg-tool/releases/download/2026-07-27/vcpkg.exe` pour x64. Le fichier n’a pas été téléchargé et l’URL et le digest devront être confirmés à l’acquisition autorisée. L’exécutable Build Tools local a une autre empreinte.

## Préparation technique et limites

Le profil portable impose `GGML_OPENMP_FETCH=OFF`, `VCPKG_INSTALL_OPTIONS=--no-downloads` et prévoit `X_VCPKG_ASSET_SOURCES=clear;x-block-origin`. Les tests synthétiques couvrent le refus d’OpenMP Fetch activé, de l’option no-downloads absente et d’une commande de dépendance comportant `/arch:AVX2`. Le build portable réel reste refusé avant la configuration. Ces contrôles ne bloquent pas tout accès réseau arbitraire ni ne prouvent l’ISA des archives liées.

Une piste est de qualifier séparément un arbre de dépendances vcpkg construit au profil portable, avec sources, triplet, commandes, macros, binaires et empreintes attribués, puis d’étudier `VCPKG_MANIFEST_INSTALL=OFF` pour le configure principal. Cela implique des compilations supplémentaires et requiert un mandat distinct. La méthode complète pour empêcher les téléchargements transitifs reste à définir et à faire relire ; tout changement d’isolation système ou acquisition réseau nécessite l’autorisation applicable.

La garde est adaptée pour les profils Whisper réellement présents et ne crée pas d’installation produit. Sa revue et les tests de création tardive des racines sont nécessaires avant de considérer cette protection fermée. Les racines et caches existants sont conservés. Le dossier non suivi `userprofile` du worktree, contenant des fichiers PowerShell, a été observé ; son origine n’est pas établie et il reste en place.

## Copie et reprise

Avant toute copie vers un cache neuf du worktree, faire relire un inventaire de la fermeture réellement nécessaire, avec chemins canoniques, absence de destination, taille et empreinte pour chaque fichier, y compris les éventuels liens. Copier uniquement les entrées approuvées sans supprimer ni déplacer les originaux, puis recomparer chaque copie. La liste complète et l’acquisition ciblée de `vcpkg.exe` manquant restent à soumettre avant toute copie ou téléchargement.

Les modifications de la garde, du pilote et de leurs tests restent non commitées. Le SelfTest de la garde native et celui du pilote natif passent sans build ni téléchargement. Une première invocation de la garde avec `-LotRoot target/native-build` a été refusée avant tests car ce mode SelfTest exige sa racine N2 ; la commande sans cet argument a ensuite réussi. La revue indépendante a confirmé la correction des sentinelles des profils absents, avec un témoin réel de remplacement du parent. Aucun build, téléchargement, configuration CMake ou exécution de vcpkg n’a été effectué. Aucun résultat de portabilité n’est revendiqué.
