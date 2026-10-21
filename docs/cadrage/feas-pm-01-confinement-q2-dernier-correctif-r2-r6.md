Avancée : le dernier lot correctif Q2 ferme statiquement R6 et préserve R3 à R5 ; huit simulations et les compilations hors ligne réussissent.
Blocage : la seconde revue indépendante laisse R2 ouvert sur une écriture étrangère sans chemin connu ; la tentative Q2 reste NO-GO.
Décision : LIVRÉ AVEC RÉSERVES après deux cycles ; arrêter les corrections et demander un arbitrage architectural distinct.

# FEAS-PM-01 — bilan du dernier correctif Q2 R2 et R6

## Périmètre et vérifications

Le lot a modifié seulement les outils autonomes dans `target/parakeet-tests/micro-feasibility/confinement-q2/` : `src/main.rs`, `launcher.rs`, `etw-config.txt`, le relevé d'empreintes et les différences conservées face au candidat précédent. Le code produit, les ACL, Windows, `models/` et `sound example/` n'ont pas été modifiés. Aucun contrôleur ou session ETW, lanceur Q2, sonde, essai de confinement, Parakeet, CUDA, stratégie A/B ou FEAS-PM-01 n'a été exécuté.

L'implémenteur rapporte `cargo test --offline --locked` (8 simulations réussies), `cargo build --offline --locked`, `cargo fmt -- --check` et la compilation `rustc` du lanceur réussis. `CARGO_HOME`, `TEMP` et `TMP` étaient sous la racine Q2, `CARGO_TARGET_DIR` dans `target/parakeet-tests/cargo-shared/`, avec `CARGO_INCREMENTAL=0` et `CARGO_PROFILE_DEV_DEBUG=line-tables-only`. L'espace libre final du volume C est d'environ 166 Gio, au-dessus du minimum de 30 Gio. Ces contrôles ne prouvent aucun comportement de confinement en exécution.

## Revue indépendante : deux cycles, arrêt du lot

| Constat | Verdict final | Preuve ou limite |
| --- | --- | --- |
| R2 — attribution des processus | **OUVERT — bloquant** | Un `File Write` ETW ID 16 peut arriver sans chemin si le fichier était déjà ouvert avant la surveillance. Si le processus étranger s'est terminé avant le rappel ETW, l'événement est différé. `flush_deferred` le fait ensuite échouer à cause du chemin vide, même si le snapshot PID, l'heure de création et l'horodatage établissent une génération étrangère antérieure à `READY`. Le test simulé des écritures étrangères fournit un chemin et ne couvre pas ce cas. |
| R3 — création de fichier vide | Fermé statiquement | La logique précédente de création réussie est conservée ; aucune preuve d'événements réels. |
| R4 — disponibilité ETW avant `READY` | Fermé statiquement | La preuve témoin précédente est conservée ; réception réelle non exécutée. |
| R5 — canal privé et arrêt automatique | Fermé statiquement | Canal et chemins d'arrêt précédents conservés ; arrêt réel non exécuté. |
| R6 — Job Q2 exact | **Fermé statiquement** | Le handle `JOB_OBJECT_QUERY` du Job détenu par le superviseur est transmis aux modes racine `run` et `child`. La sonde vérifie `IsProcessInJob` sur ce handle avant son premier rapport ; l'enfant reste suspendu jusqu'au contrôle du superviseur sur le même Job. |

La première revue avait relevé un faux échec possible sur des écritures étrangères ordinaires et l'absence d'attestation locale du Job par la sonde racine. La correction a ajouté une preuve de génération et l'attestation du Job, puis la seconde revue a relevé le cas R2 sans chemin. La limite de deux cycles du présent lot est atteinte. Le candidat est **PARTIAL en conformité statique, NO-GO pour Q2**. Aucun PASS de prévention NTFS, d'attribution ETW, d'arrêt automatique ou de restauration ACL n'est revendiqué.

## Identité et conservation du candidat

Empreintes SHA-256 du candidat figé, relues sur disque et consignées dans `target/parakeet-tests/micro-feasibility/confinement-q2/candidate-hashes.txt` :

| Fichier | SHA-256 |
| --- | --- |
| `src/main.rs` | `3783688B3750578F481678815AD84F8508AFD6302E66E18EBA19AE99A1A143B5` |
| `launcher.rs` | `25FA7FCB5152FA9324321154CC4CE7EBF2E83627AC09101EDD3EDF20DFE37B8E` |
| `etw-config.txt` | `EBC834D58C100F04DE94633271E20E690AA48371D9918890CA576E6748F551E9` |
| `cargo-shared/debug/feas-pm01-confinement-q2-tools.exe` | `20D630CF9D6F0B1CACC205FEA8EF4EA987E7104AB20D6492C3E997D199B1710D` |
| `confinement-q2/launcher-q2.exe` | `6A60703CA22347674F555BAA4865B550B6127EEA0C750886BFB4F91F43F38229` |

Avant ce correctif, `src/main.rs` avait l'empreinte `9F244214C115B8C8BFF0E3BC12CD3F52F8855620ABD6F2E2753AB32A9256ACA6` et `launcher.rs` `D8113E9BEFB858CD11BA0AC5F9B8EA57D6204277F09B02D836B966DF4B3F0AA4`. Les snapshots `cycle3-baseline/` et les différences `cycle3-main.diff.txt` et `cycle3-launcher.diff.txt` restent sous la racine Q2. `target/` est ignoré par Git : le présent document versionne le bilan et l'identité, mais pas les sources ni les exécutables du candidat. Ne pas nettoyer ces fichiers avant une décision de conservation.

## Arbitrage nécessaire avant tout nouvel essai

Le défaut observé est une règle de classement du moniteur, pas une preuve que le jeton restrictif ne préviendrait pas une écriture. Il suffit néanmoins à rendre l'attribution non fiable pour le protocole actuel : une écriture indépendante peut provoquer l'arrêt alors que sa génération étrangère est démontrée. La couverture des fichiers déjà ouverts ajoute une dépendance au chemin qui n'est pas nécessaire pour classer une **génération étrangère prouvée**.

**Simplification proposée, sous nouvel accord explicite :** décider l'appartenance au Job à partir de la génération du processus et de sa période de vie avant d'exiger le chemin. Un événement d'une génération étrangère prouvée serait classé étranger même sans chemin ; un événement de génération Q2 exigerait le chemin pour le contrôle des racines ; toute génération ambiguë resterait en attente puis ferait échouer la campagne. Un futur lot devrait démontrer par simulations les ID 16 sans chemin, les PID réutilisés et les `ProcessStart` retardés, puis repasser en revue indépendante. Cette proposition ne vaut ni autorisation de corriger ni GO pour Q2.

Si cette simplification ne permet pas une attribution statiquement sûre, il faudra choisir une autre architecture d'isolation et requalifier séparément son accès au GPU et ses effets sur les mesures. Une surveillance ETW seule ne devient jamais une preuve de prévention. L'unique tentative Q2, CUDA et FEAS-PM-01 restent interdits jusqu'à leurs autorisations distinctes et leurs prérequis satisfaits.
