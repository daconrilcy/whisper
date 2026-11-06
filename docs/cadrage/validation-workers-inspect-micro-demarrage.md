Avancée : les trois sélections workers N1 disposent de résultats réussis.
Blocage : aucun pour cette validation ; tests natifs ignorés et corps GPU non exécutés.
Décision : clôturer les sélections workers, conserver séparées les limites de qualification native.

# Validation des workers — clôture du 6 novembre 2026

Demande : terminer les tests workers restants. Mandat initial `c0b45b60`, exception CPU acceptée dans la session (« oui je t'autorise »), consignée dans `9e617601`. Candidat source `a2ef198d` inchangé ; aucun code, manifeste, configuration ou dossier de remise modifié par cette validation. Aucun push.

| Sélection N1 | Réussis | Échecs | Ignorés |
|---|---:|---:|---:|
| Membres par défaut | 774 | 0 | 128 |
| Worker-common | 11 | 0 | 1 |
| Worker-parakeet-cpu | 26 | 0 | 11 |
| Worker-cpu | 12 | 0 | 9 |

Les preuves des membres par défaut et de common sont antérieures et encore applicables (entrées inchangées) ; les deux autres sélections ont été exécutées ici. Les ignorés restent annoncés comme tels : aucune qualification native complète ni exécution du corps des tests GPU niveau 2 déduite.

**Parakeet CPU :** compilation Rust `--no-run`, PE ParakeetCpu des cinq harnais et du binaire avant exécution, puis sélection complète code 0. Worker_contract exécute zéro test et n'est pas compté comme couverture. Aucun moteur natif produit.

**Turbo CPU :** aucune compilation avant l'accord natif précis. Puis `--no-run` code 0 (22,21 s), PE Cpu des quatre harnais et du binaire avant exécution, sélection complète code 0.

Commandes via `ci/guard-level1.ps1 -LotRoot target/parakeet-tests/inspect-micro-demarrage/validation-workers` : `test --offline --locked -p <package> --no-run`, contrôles PE des chemins exacts, puis `test --offline --locked -p <package>`. Cwd `C:/dev/whisper`, cache/cible partagés, temporaires isolés, sans native-tests/ignored/CUDA ni téléchargement. Exécutables et SHA détaillés dans `pe-parakeet.log` et `pe-cpu.log` archivés.

**Exception native exécutée :** whisper-rs-sys 0.15.0/CMake CPU réellement compilé ; GGML_CUDA/HIP/SYCL/VULKAN OFF, BUILD_SHARED_LIBS OFF et WHISPER_CURL OFF. MSVC 14.44/SDK/CMake/libclang 22.1.8 présents. `cpu-native-proof.log` conserve sortie du 06/11 à 14:56:13 et cache CPU ; sorties CUDA historiques inchangées. Cet accord ponctuel ne vaut pas reprise générale des builds natifs suspendus. Workers installés et remises précédentes inchangés, aucune installation ou qualification Windows.

**Clôture :** [archive durable](raw/validation-workers-inspect-micro-demarrage.zip), 32 journaux, arborescence et chaque SHA vérifiés. ZIP SHA-256 `00ccbc5c61eb50dccca63c853a4ea2f7ef92ffc66442d3aeff6393b2c038cace`. Aucun cache/cible isolé à supprimer, caches partagés conservés, aucun nettoyage manuel ; environ 430 Gio libres. Seuls les compléments et preuves sont commités localement, sans fusion ni push.
