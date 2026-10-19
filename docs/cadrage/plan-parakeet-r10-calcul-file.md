Avancée : correctif R10 commité, revue indépendante du code CLEAN et recette Windows déclarée OK par l'utilisateur.
Blocage : aucun pour le lot R10 ; publication et remplacement de l'installation personnelle restent distincts.
Décision : clore R10 localement sur la validation utilisateur du 19/10/2026, sans push ni installation.

# R10 — Calcul choisi au lancement d'un import en file (validé localement)

Base : `evo-startup-perf` à `0e610c95`, après L6-perso. Autorisation écrite R10 reçue le 19/10/2026 pour les trois fichiers de code ci-dessous ; le helper d'archive a été ouvert séparément par l'utilisateur pendant la campagne. Aucun déploiement ni push autorisé par cet accord.

État au 19/10/2026 : correctif au commit `09a574a6` ; 402 tests réussis, 0 échec, 34 ignorés sur 52 cibles du gate workspace sous garde ; revue du code CLEAN, R1 fermé au deuxième passage. Le desktop `personal-install` de recette est copié sous `target/parakeet-tests/r10/desktop/whisper-desktop.exe`, SHA-256 `96c2c420c3d4b3e41cc927bbe4946da927ba005f07c19954a86e1622938c9e93`, avec les deux workers Turbo qualifiés à côté ; les deux workers Parakeet qualifiés sont chargés depuis l'installation Perso stable. Après cette mise à disposition, l'utilisateur a déclaré « tout est ok cette fois » : recette R10-1 **PASS selon validation utilisateur dans la fenêtre Windows**. Le détail des observations TXT/SRT n'a pas été remis séparément et aucune mesure supplémentaire n'est revendiquée.

## Constat

`crates/whisper-desktop/src/ui.rs:602` enregistre le calcul actif lors de l'ajout. Le bouton « Traiter » transmet la copie de cette demande (`ui.rs:1264-1266`), tandis que `ui.rs:462-485` transmet une preuve GPU calculée à partir du choix **actuel**. Si l'entrée a été ajoutée en GPU puis que l'utilisateur choisit CPU, l'adaptateur reçoit `compute=Gpu` et `candidate=None` ; `crates/whisper-adapters/src/worker_ipc.rs:2647-2650` produit le message observé. `worker_ipc.rs:1930-1954` impose aussi l'égalité avec l'entrée de file durable : modifier seulement la copie envoyée depuis l'interface conduirait à `SourceChanged`.

## Comportement visé

Pour une entrée `Queued` ou `AwaitingChoice`, le clic sur « Traiter » applique le choix de calcul affiché à cet instant (CPU, GPU ou Auto) au modèle déjà associé à l'entrée. L'entrée durable reçoit ce choix avant la création du pending. La preuve GPU et la demande envoyée au worker concernent ce même choix. Une entrée `Interrupted`, en cours ou déjà préparée garde son calcul et sa clé de backend : aucune modification implicite en reprise. La sélection manuelle GPU sans preuve valide est refusée avant pending, sans repli silencieux ; Auto conserve ses règles actuelles. Turbo et le micro gardent leurs parcours.

## Chemins d'écriture proposés, et seulement ceux-ci

- `crates/whisper-desktop/src/ui.rs` : former la demande `ProcessQueued` à partir de l'entrée visible et du choix de calcul courant, sans altérer les autres champs ; utiliser le calcul de **la commande** pour sélectionner la preuve GPU, notamment lors d'une reprise. Tests locaux de changement GPU→CPU, CPU→GPU, Auto et reprise inchangée.
- `crates/whisper-adapters/src/worker_ipc.rs` : admettre uniquement la divergence du champ `config.compute` pour une entrée `Queued` ou `AwaitingChoice` ; préserver toutes les comparaisons d'identité des autres champs, la FIFO, l'admission du modèle et les refus de reprise. Persister le nouveau calcul avant `prepare_with_qualification`, puis employer la demande ainsi persistée pour l'archive et le worker. Tests locaux des refus et du parcours.
- `crates/whisper-adapters/src/queue_store.rs` : opération durable ciblée modifiant seulement `config.compute` sur une entrée en attente, après contrôle de l'identité complète et du statut ; conserver l'ordre, la source vérifiée et les autres champs. Aucun changement du schéma ou de la validation des reprises.
- Amendement accepté pendant l'exécution : `crates/whisper-adapters/tests/parakeet_archive.rs`, uniquement le helper `archive_root()` ; admettre `PARAKEET_R10_EXEC_ROOT` si sa forme canonique est exactement `target/parakeet-tests/r10/` et si la racine d'archive est son descendant. Les assertions des tests restent inchangées. Cet ajout est nécessaire au gate complet : dix tests historiques refusaient la nouvelle racine de campagne, sans échec produit identifié.

Exécution des builds, tests et preuves sous `target/parakeet-tests/r10/` et Cargo partagé `target/parakeet-tests/cargo-shared/`, avec garde à échec fermé existante. Aucune écriture dans `models/`, `sound example/`, le runtime CPU/GPU figé, l'installation Perso ou les sorties utilisateur pendant la recette automatisée.

## Cartes d'acceptation

| Carte | Résultat exigé |
| --- | --- |
| R10-1 | Ajout GPU puis choix CPU et « Traiter » : pending et worker CPU Parakeet, TXT/SRT produits, aucune preuve GPU exigée, aucun `BackendMismatch`. |
| R10-2 | Ajout CPU puis choix GPU : traitement GPU seulement avec la clé et la preuve courantes ; sans elles, refus explicite avant pending. |
| R10-3 | Choix Auto au traitement : préférence conservée ; règles existantes de sélection et de repli inchangées. |
| R10-4 | Entrée interrompue : reprise liée à la clé de backend d'origine ; le changement du sélecteur global ne la remplace pas. |
| R10-5 | Requête altérée dans tout autre champ, FIFO violée ou statut non admissible : refus inchangé ; journal durable rejoué après redémarrage conserve le nouveau calcul d'une entrée en attente. |
| R10-6 | Tests ciblés, `cargo test --offline --locked --workspace --no-fail-fast` sous garde, zéro FAIL ; revue indépendante au plus deux cycles. La recette dans la fenêtre Windows reste à l'utilisateur. |

Préflight : état Git, espace libre ≥30 Gio, sources et variables de garde, disponibilité des fixtures locales ; aucune acquisition. L'implémentation demande un accord écrit précis sur ce plan, ses chemins et le lot R10. Après candidat, délégation à `rust_implementer`, puis `rust_review_coordinator`. Aucun push ou réinstallation personnelle couverts par R10.
