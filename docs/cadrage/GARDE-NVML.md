Avancée : GARDE-NVML livré CLEAN ; témoins ProgramW6432 et reprise Parakeet GPU réussis.
Blocage : aucun pour ce lot ; corpus FR87/FR27 et autres recettes restent non qualifiés.
Décision : correctif conservé sur branche dédiée ; aucune fusion de la pile ni push.

# GARDE-NVML

Branche `codex/garde-nvml`, worktree `C:/dev/whisper/.wt/nv`, base `53eb3bce`. [Plan autorisé](plan-garde-nvml.md). Cycle initial et fermeture GN-R2 supplémentaire explicitement autorisée. Environ 62 minutes depuis création du worktree : cible d'une heure légèrement dépassée.

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
| `PersonalUnifiedGpuOnly` : seul sélecteur GPU existant | PASS : 1 test, 0 échec ; Cargo 0, 42,695 s |
| Revue indépendante, auto-test staging et préflights | CLEAN, GN-R1/R2/R3 fermés ; PASS |

Les deux appels NVIDIA partagent EXE/argv/CWD/racines et toutes les autres variables, vérifié sur leurs PSI. UUID positif : `GPU-99e636f6…645e`. Premier essai GPU : 1 échec (Cargo 101), parent `Whisper/settings` absent ; trace conservée. Après accord explicite, la garde prépare ce seul parent vide sous le RunRoot, sans variable ni Rust modifié ; le produit crée logs. Reprise unique : **1 PASS**, corps 41,21 s ; publication TXT **49 octets**, SRT **82**, WAV **129 068**. Aucun CPU/Auto ou autre profil GPU déjà PASS relancé.

**Non qualifié :** FR87/FR27 perdus et recettes sans prérequis/sélecteurs prêts (`ParakeetGpu` DISCOVERY_PENDING). La campagne complète n'est pas verte. Candidat `61447382` ; seules les deux CI, plan/bilan et [preuves](../architecture/raw/garde-nvml/results.json) changent. ZIP : **63 entrées**, tailles/SHA vérifiés ; 23 fichiers sources et 21 fichiers stagés inchangés par SHA. Espace libre : **98 018 824 192 octets**. Aucun nettoyage, installation, fusion ni push.
