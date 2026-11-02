Avancée : audit produit sans défaut ; candidat corrigé une fois, deux constats fermés.
Blocage : GN-R2 reste ouvert sur le diagnostic historique ; auto-tests et GPU non exécutés.
Décision : LIVRÉ AVEC RÉSERVES au budget fixé ; cycle supplémentaire ciblé proposé.

# GARDE-NVML

Branche `codex/garde-nvml`, worktree `C:/dev/whisper/.wt/nv`, base `53eb3bce`. [Plan autorisé](plan-garde-nvml.md). Un cycle de correction de revue maximum.

| Vérification produit en lecture seule | Fichier:ligne et environnement transmis |
| --- | --- |
| Turbo CPU/GPU et décodeur | `worker_ipc/ipc/mod.rs:57`, `runtime/import/worker_start.rs:392` (sous `whisper-adapters/src/`) : héritage complet, aucun filtrage. |
| Parakeet CPU/GPU | `whisper-adapters/src/parakeet_import.rs:411,434,451,454` : héritage, ajout `PARAKEET_L2_RUNTIME_ROOT` et log GPU `PARAKEET_L3_ASR_LOG` ; seuls sept noms historiques PARAKEET retirés. |
| Inventaires NVIDIA | `parakeet_import.rs:800`, `whisper-worker-parakeet-cpu/src/gpu_identity.rs:133` : héritage complet. |

Recherche exhaustive `crates/` (production et tests) : aucun `env_clear`/bloc Windows personnalisé ni retrait de `ProgramW6432`. Les `.env()` sont additives ; retraits dynamiques limités aux noms historiques. Aucun critère STOP produit. Les lancements Rust conservent la variable si leur parent la possède.

**Correctif :** valeur hôte `C:/Program Files`, contrôlée contre HKLM 64 bits `CurrentVersion/ProgramW6432Dir`, absolue/existante/canonique/sans jonction. Une seule nouvelle variable, réservée aux profils GPU/NVIDIA ; aucun remplacement par JSON ou VS. Isolations inchangées. Les deux CI portent le changement ; aucun Rust, manifeste, SDK, pilote, registre ou installation modifié.

| Validation | Résultat |
| --- | --- |
| Auto-tests : valeur conforme ; absent/inexistant/jonction/différent du registre refusés | NON EXÉCUTÉS |
| Témoin positif : même commande NVIDIA, code 0 et une ligne UUID | NON EXÉCUTÉ |
| Témoin négatif : seule ProgramW6432 retirée, code 255 attendu | NON EXÉCUTÉ |
| Reprise `PersonalUnifiedGpuOnly` : seul sélecteur GPU existant | NON EXÉCUTÉ |
| Revue indépendante, un cycle de correction | GN-R1/R3 fermés ; GN-R2 ouvert |

**Réserve :** `NvidiaSmiInventoryDiagnostic` utilise encore le builder exigeant PROGRAMDATA/ALLUSERSPROFILE/PUBLIC absents de son environnement. Proposition : restaurer aussi son chemin historique (builder, résolution toolchain et préflight), sans ajouter ces variables ni supprimer les contrôles. Aucun deuxième correctif appliqué après le budget. PS5 syntaxe/JSON et diff-check PASS ; aucun Cargo/NVIDIA/worker/modèle lancé.

**Non qualifié :** FR87/FR27 perdus ; recettes sans prérequis/sélecteurs prêts, dont `ParakeetGpu` DISCOVERY_PENDING. Aucun profil déjà PASS relancé, aucune fusion/push. Candidat `cad79544`, sources : les deux CI ; plan et présent bilan suivis. [Preuves](../architecture/raw/garde-nvml/results.json) : six entrées ZIP, tailles/SHA vérifiés. Espace libre relevé : **105 087 500 288 octets**.
