Avancée : L3 vise le GPU Parakeet pour l'import WAV/MP3 en passe unique, après la qualification CPU Test de L2.
Blocage : seul le banc CUDA0 est qualifié ; le worker et l'admission produit Parakeet restent CPU.
Décision : proposer L3a natif/attestation puis L3b application, avec qualification exacte avant activation GPU.

# Note de design — L3 GPU Parakeet pour l'import (DRAFT v1, 17/10/2026)

## Source et frontière

Source produit : [cadrage Parakeet existant](evo-parakeet.md), décisions 03/04/15/16 et correction utilisateur du 17/10/2026 : **L3 est le prochain lot**. Le banc [SPIKE-PARA-02](bilan-parakeet-faisabilite-cloture.md) a obtenu du texte FR/EN sur GTX 1080 Ti CUDA0 et CPU, avec `language_code=NULL`, sans crash hôte. Cette preuve porte sur le banc mono-GPU, pas sur le worker L2 ni sur un paquet installé. Le produit L2 actuel refuse Parakeet si `compute != Cpu` dans `whisper-adapters/src/worker_ipc.rs`, le worker `whisper-worker-parakeet-cpu` crée NeMo avec `gpu=-1`, annonce `backend=CPU` et le registre n'admet que CPU. L3 ne modifie pas Turbo et n'ouvre ni microphone Parakeet, ni import >30 min, ni VAD adaptative/B8.

## Comportement visé

Pour Parakeet/import ≤30 min, **CPU manuel** reste le parcours qualifié L2 ; **GPU manuel** utilise un worker GPU distinct et la clé exacte modèle + worker + DLL CUDA + backend + profil + périphérique. Une panne de GPU manuel est expliquée, sans bascule CPU. **Auto calcul** privilégie ce GPU seulement si une preuve d'identité et la préparation native sont obtenues avant traitement audio ; sinon une seule tentative CPU du **même** modèle est autorisée, avec cause et calcul effectif visibles. Après la première inférence, aucun repli. Le choix Auto et le modèle Parakeet explicitement préféré restent mémorisés. La **langue automatique** Parakeet garde son sens actuel (FR/EN qualifiés, sans identifiant de langue retourné) ; la langue manuelle reste indisponible.

La sélection GPU est conservatrice : qualification produit locale de **la seule GTX 1080 Ti** reconnue sans ambiguïté comme CUDA0. Le chemin d'identité rapproche la sonde Windows actuelle (PnP → DXGI/LUID), le périphérique CUDA (ordinal, LUID/PCI) et le backend NeMo/GGML réellement ouvert. Un nom, `Ready`, une allocation ou `CUDA0` seul ne constituent pas cette preuve. Si plusieurs candidats physiques subsistent ou si la correspondance n'est pas démontrée, GPU manuel refuse avant audio et Auto choisit CPU avant audio. Aucune promesse multicarte n'est faite par L3. La machine et le pilote ne sont pas figés dans le code : ils déterminent si la fiche locale s'applique.

## Architecture et états

| Frontière | Contrat L3 |
| --- | --- |
| `whisper-core` | Décider CPU/GPU/Auto à partir d'une configuration figée, d'une capacité qualifiée pour **cette clé** et d'une preuve GPU fraîche. Échec GPU manuel sans repli ; Auto au plus une tentative CPU avant inférence. CPU et GPU ont des clés séparées. |
| `whisper-adapters` | Conserver deux chemins de workers Parakeet et deux jeux natifs séparés. Vérifier modèle, worker et DLL avant pending, refuser une clé GPU différente, router la commande, associer le résultat à la génération/tentative/périphérique, préserver les pending d'une ancienne clé sans reprise GPU silencieuse. |
| Worker GPU | Variante Rust hors processus du worker Parakeet, utilisant l'ABI NeMo v0.2.0 et ses DLL CUDA privées. Sélectionner le seul CUDA0 qualifié, contrôler sa correspondance physique avant l'ouverture audio, émettre Ready avec backend CUDA0 et preuve de périphérique ; après inférence, attester le backend réellement utilisé. Garder DLL chargées pendant la vie du processus et quitter sans `FreeLibrary`. |
| Composition/UI | Montrer GPU seulement si la clé GPU, les DLL et la preuve de périphérique sont disponibles ; afficher le calcul demandé et effectif, la cause d'un repli Auto ou d'un refus manuel. Conserver la matrice Turbo reconnue. |

Le protocole Parakeet IPC v1 conserve les champs CPU. Une extension additive optionnelle de l'événement `Ready` transporte l'attestation matérielle GPU ; elle est obligatoire pour le worker GPU et absente sur CPU. La requête au worker GPU porte l'identité attendue et une génération ; Ready et Result doivent être cohérents avec elles. Une absence, ambiguïté ou divergence échoue **avant** l'inférence, sans écrire un faux résultat. Si cette extension ne suffit pas à garantir la compatibilité IPC v1, L3a doit conclure `DESIGN_CHANGE_REQUIRED` avant tout changement de version du protocole.

La variante GPU ne réutilise pas la DLL CPU : le dossier de référence est `target/parakeet-feasibility/spike02/runtime space/cuda/bin/` (NeMo, ggml-cuda, ggml et runtimes CUDA). Les DLL de L3 sont copiées et vérifiées sous la racine d'exécution L3 ; `nvcuda.dll` provient du pilote Windows. Aucun pilote, bibliothèque ou modèle n'est téléchargé. Le code `unsafe` GPU, s'il est nécessaire pour interroger CUDA ou l'ABI, reste dans la façade native ; son périmètre et ses invariants demanderont une autorisation écrite propre à L3. La mesure de VRAM sous WDDM reste globale ; ne pas inventer un seuil de mémoire au registre sans preuve. Un manque de mémoire constaté produit une erreur explicite (manuel) ou, seulement avant audio, la tentative CPU Auto autorisée.

## Preuve exigée et risques

L3a qualifie le **worker GPU exact**, sans activer le produit : FR87, FR27, EN15, EN5 réel et EN30 synthétique, texte/TXT/SRT quand applicables, durées positives/ordonnées, zéro `InvalidTimestamp`, backend CUDA0 attesté et identité GTX 1080 Ti correspondante. Faux identifiant, DLL CPU mélangée, DLL CUDA altérée et backend CPU annoncé sont refusés avant audio ; aucune clé CPU n'hérite de cette preuve. Un worker GPU peut traiter plusieurs fichiers puis sortir sans crash hôte. Les temps et VRAM sont rapportés comme mesures, sans seuil de vitesse ou mémoire nouvellement inventé.

L3b route les mêmes recettes par **le parcours applicatif** sur la nouvelle clé GPU ; vérifie GPU manuel, Auto GPU quand prêt, Auto CPU si indisponible avant audio, refus GPU manuel, absence de repli après inférence, pending/reprise liés à la clé, UI et matrice Turbo inchangées. EN30 passe ; >30 min est refusé avant pending. A4 FR/EN et les cinq scénarios R8 sont rejoués sur la clé GPU si leur mécanisme s'applique, avec les adaptations documentées du harness ; aucun test ignoré n'est présenté PASS. La fiche GPU Test est écrite seulement après PASS complet. La recette visuelle Windows reste réservée à l'utilisateur. L6-perso attend ensuite un amendement pour installer le **worker GPU release** et ses dépendances : la fiche Test L3 ne qualifie pas ce futur binaire.

Risque majeur : NeMo/GGML v0.2.0 remonte `CUDA0` mais pas l'identité physique depuis le recognizer ; l'attestation repose sur la corrélation CUDA/Windows et l'hypothèse vérifiée d'un seul GPU éligible. Une topologie multicarte est refusée dans L3. Dépendances CUDA privées, droits de chargement, disponibilité du pilote et mémoire sont des conditions de qualification, pas des raisons de repli silencieux. Les 23 autres langues et la qualité contre vérité terrain restent hors L3.
