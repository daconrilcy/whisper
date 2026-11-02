Avancée : garde CLEAN, auto-tests PASS ; ProgramW6432 confirme la cause NVML (0 avec, 255 sans).
Blocage : import GPU exécuté mais échoué sur le parent de journal settings absent.
Décision : arrêt après cet échec ; préparation du seul dossier manquant proposée, aucune fusion.

# GARDE-NVML

Branche `codex/garde-nvml`, worktree `C:/dev/whisper/.wt/nv`, base `53eb3bce`. [Plan autorisé](plan-garde-nvml.md). Cycle initial et fermeture GN-R2 supplémentaire explicitement autorisée.

| Vérification produit en lecture seule | Fichier:ligne et environnement transmis |
| --- | --- |
| Turbo CPU/GPU et décodeur | `worker_ipc/ipc/mod.rs:57`, `runtime/import/worker_start.rs:392` (sous `whisper-adapters/src/`) : héritage complet, aucun filtrage. |
| Parakeet CPU/GPU | `whisper-adapters/src/parakeet_import.rs:411,434,451,454` : héritage, ajout `PARAKEET_L2_RUNTIME_ROOT` et log GPU `PARAKEET_L3_ASR_LOG` ; seuls sept noms historiques PARAKEET retirés. |
| Inventaires NVIDIA | `parakeet_import.rs:800`, `whisper-worker-parakeet-cpu/src/gpu_identity.rs:133` : héritage complet. |

Recherche exhaustive `crates/` (production et tests) : aucun `env_clear`/bloc Windows personnalisé ni retrait de `ProgramW6432`. Les `.env()` sont additives ; retraits dynamiques limités aux noms historiques. Aucun critère STOP produit. Les lancements Rust conservent la variable si leur parent la possède.

**Correctif :** valeur hôte `C:/Program Files`, contrôlée contre HKLM 64 bits `CurrentVersion/ProgramW6432Dir`, absolue/existante/canonique/sans jonction. Une seule nouvelle variable, réservée aux profils GPU/NVIDIA ; aucun remplacement par JSON ou VS. Isolations inchangées. Les deux CI portent le changement ; aucun Rust, manifeste, SDK, pilote, registre ou installation modifié.

| Validation | Résultat |
| --- | --- |
| Auto-tests : conformité, absent/inexistant/jonction/différent du registre | PASS, refus attendus ; registre inchangé |
| Témoin positif : commande NVIDIA existante | PASS : code 0, une ligne UUID |
| Témoin négatif : seule ProgramW6432 retirée | PASS : code 255, erreur NVML attendue |
| `PersonalUnifiedGpuOnly` : seul sélecteur GPU existant | 1 test, 0 PASS, 1 échec ; Cargo 101, 62,168 s |
| Revue indépendante et trois préflights | CLEAN, GN-R1/R2/R3 fermés ; PASS |

Les deux appels NVIDIA partagent EXE/argv/CWD/racines et toutes les autres variables, vérifié sur leurs PSI. UUID positif : `GPU-99e636f6…645e`. La reprise GPU échoue avant transcription : `WorkerLogPath: create runtime logs`, erreur Windows 3. Le code crée `Whisper/settings/logs` non récursivement ; le parent settings manque dans le staging isolé (chemin 134 caractères). Sources `parakeet_import.rs:375,703–712`, `worker_ipc/parakeet_unified/native.rs:168`. Aucun Rust corrigé ; créer ce parent sous le RunRoot puis reprendre ce seul test nécessite l'arbitrage après échec.

**Non qualifié :** import GPU ci-dessus, FR87/FR27 perdus et recettes sans prérequis/sélecteurs prêts (`ParakeetGpu` DISCOVERY_PENDING). Aucun profil déjà PASS relancé, aucune fusion/push. Candidat `34f7498f` ; seules les deux CI, plan/bilan et [preuves](../architecture/raw/garde-nvml/results.json) changent. ZIP : 29 entrées tailles/SHA vérifiés ; 23 entrées sources et 21 fichiers stagés inchangés par SHA. Espace libre : **101 618 073 600 octets**.
