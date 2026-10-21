Avancée : le candidat autonome Q2 corrige statiquement R3, R4 et R5 ; les SHA-256 de ses sources et exécutables ont été relevés.
Blocage : la revue indépendante laisse R2 et R6 ouverts, ce qui interdit de lancer l'unique tentative Q2.
Décision : LIVRÉ AVEC RÉSERVES et NO-GO Q2 après deux cycles de revue du lot correctif.

# FEAS-PM-01 — bilan du lot correctif Q2 R2–R6

## Périmètre et contrôles

Le lot autorisé après `feas-pm-01-confinement-q2-preparation.md` concerne exclusivement les outils autonomes sous `target/parakeet-tests/micro-feasibility/confinement-q2/`. Le code produit, les ACL et la configuration permanente de Windows n'ont pas été modifiés. Aucune session ETW, sonde, tentative Q2, inférence Parakeet ou FEAS-PM-01 n'a été exécutée. La branche reste `evo-startup-perf` ; les seules entrées non suivies visibles par Git restent `models/` et `sound example/`.

L'implémenteur rapporte `cargo fmt --check`, `cargo check --offline --locked`, `cargo build --offline --locked` et `rustc` du lanceur réussis, avec `CARGO_HOME`, `TEMP` et `TMP` sous la racine Q2 et `CARGO_TARGET_DIR` sous `target/parakeet-tests/cargo-shared/`. La revue indépendante a examiné le candidat pendant deux cycles, sans essai d'exécution.

## Verdict de la revue indépendante

| Constat | État statique | Limite décisive |
| --- | --- | --- |
| R2 — attribution des processus | **OUVERT — bloquant** | Un événement fichier hors racines peut être classé étranger après le délai de corrélation alors que le processus racine a été attesté dans le Job mais que son `ProcessStart` ETW arrive tard. Une écriture du lot peut ainsi manquer à l'attribution. |
| R3 — création sans contenu | Fermé statiquement | Le candidat exploite `CreateNewFile` ID 30 avec le masque candidat `0x12D0`, sans déduire une création effective d'une simple ouverture ID 12. Livraison de l'événement et prévention restent non testées. |
| R4 — disponibilité ETW | Fermé statiquement | `READY` attend un témoin fichier ID 30 et un témoin `ProcessStart` décodés ; la clôture vérifie le drain, le résultat du consommateur et le refus attendu. La chaîne réelle reste non testée. |
| R5 — canal privé et arrêt | Fermé statiquement | Le pipe vérifie le PID du pair et la vie du consommateur ; pertes, retards et sortie déclenchent le chemin d'arrêt du Job. Ce chemin n'a pas été exécuté. |
| R6 — Job exact | **OUVERT — réserve** | Le superviseur utilise le handle du Job Q2 pour la racine, mais la preuve pour chaque enfant dépend encore de l'ordre d'arrivée des événements. Le texte de la sonde enfant ne constitue pas une attestation du Job exact. |

R2 concerne directement le risque d'écriture hors racine non attribuée ; c'est un préalable indispensable à Q2. R6 empêche de conclure que la restriction est démontrée pour l'enfant. Le candidat est donc **PARTIAL en préparation statique**, **NO-GO pour l'essai Q2**. La prévention NTFS, le démarrage sous `DETACHED_PROCESS`, la réception ETW en direct et l'arrêt automatique restent **NON EXÉCUTÉS**, sans verdict PASS/FAIL de fonctionnement.

## Identité du candidat

Empreintes SHA-256 relues sur disque et concordantes avec `target/parakeet-tests/micro-feasibility/confinement-q2/candidate-hashes.txt` :

| Fichier | SHA-256 |
| --- | --- |
| `confinement-q2/src/main.rs` | `CC5D83297856082C20FBCFD7DADEC2C49DB11AD65AD6B32E3EF889CAC25664DC` |
| `confinement-q2/launcher.rs` | `9C662215CA15610C115E9C3958734AC963EA2A2DA5E0E01673E1C5D1B09048B9` |
| `confinement-q2/etw-config.txt` | `9A62C44FC8A17169E9A3C6A5DCC90FDDB8D4D0285EF45748D4F55F06EC004439` |
| `cargo-shared/debug/feas-pm01-confinement-q2-tools.exe` | `862F6C0427D6F282A4621044FE4BDC848154CBBD4CA22CD0490B0770C487A561` |
| `cargo-shared/debug/q2-confinement-launcher.exe` | `E6D73ED596FF31291EBF2BD3BB20AAD1668CA4C84A505C9EC37CF956B9F040F6` |

Le fichier local d'empreintes donne aussi celles de `Cargo.toml` et `Cargo.lock`, ainsi que la configuration de compilation. Le répertoire `target/` est ignoré par Git : ce commit conserve l'identité du candidat mais ne versionne pas ses sources ni ses exécutables. Ne pas nettoyer ce répertoire avant une décision sur leur conservation.

## Couverture et prochaine décision

Le candidat surveille statiquement les créations de fichiers ID 30, les ouvertures/créations ID 12 corrélées à `OperationEnd` ID 24 et les écritures de contenu ID 16. Renommages, liens physiques, suppressions, modifications de sécurité ou d'attributs étendus, flux alternatifs et écritures par mappage mémoire restent non démontrés ; `etw-config.txt` les énumère. ETW observe les opérations mais ne remplace pas le contrôle préventif du jeton restreint et des ACL.

Une nouvelle autorisation précise est nécessaire avant toute correction supplémentaire, car la limite de deux cycles du présent lot est atteinte. Le périmètre minimal proposé est : (1) ne jamais classer étranger un événement d'un PID déjà attesté dans le Job sans résolution de sa génération, même hors racines ; (2) obtenir pour chaque enfant une preuve contre le handle du Job exact, indépendante de l'ordre d'arrivée ETW ; (3) soumettre ce delta à une revue indépendante. Une autorisation distincte restera nécessaire pour exécuter Q2, avec préflight des ACL, surveillance et restauration contrôlée. FEAS-PM-01 demeure interdit.
