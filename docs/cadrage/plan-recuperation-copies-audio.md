Avancée : diagnostic du blocage SourceSnapshotCapacity et plan de récupération explicite préparés.
Blocage : deux réservations restent dans le registre global, dont une sans copie ; aucun nettoyage personnel n'est autorisé à ce stade.
Décision : plan RTC-05 soumis à l'utilisateur avant toute modification de code ou de données personnelles.

# RTC-05 — Récupérer les copies audio abandonnées

## Diagnostic observé le 20 octobre 2026, en lecture seule

Le dossier `C:\Users\cyril\Documents\WhisperTranscriptions\transcriptions\` ne contient aucun traitement. Le registre `%LOCALAPPDATA%\Whisper\settings\reprocessing-snapshots\reprocessing-snapshots.json` contient pourtant deux entrées `Bound` pour cette destination : la première ne possède plus ni copie ni fiche ; la seconde possède une copie et sa fiche. La copie restante pouvait être ouverte en lecture exclusive au moment du contrôle. L'application de recette RTC-04 était ouverte. Ces constats expliquent le refus à deux places, mais ne prouvent pas comment les entrées sont devenues orphelines.

`SourceBinding::retire` enlève actuellement la copie et la fiche avant de retirer l'entrée du registre. Une interruption entre ces actions peut laisser une entrée `Bound` sans fichiers. La fonction `reopen_for_identity` retourne `None` quand les deux fichiers manquent et `release_source_binding` considère alors l'opération réussie sans libérer cette entrée. Le registre compte toutes ses entrées, indépendamment du contenu de `transcriptions/`. Une copie encore présente peut correspondre à un choix d'import abandonné après interruption ; elle ne doit jamais être effacée sur le seul constat que le dossier des transcriptions est vide.

## Comportement retenu

1. Quand la limite de deux copies bloque un import, présenter une action **« Vérifier les copies audio retenues »**. L'inspection montre chaque réservation et indique si elle est récupérable ou protégée. Aucune réservation n'est libérée par l'inspection ou au démarrage.
2. La récupération demande une confirmation explicite. Juste avant toute disposition, revérifier sous verrou du registre l'identité de l'entrée et de la destination, l'absence de cet identifiant dans les bindings actifs du processus, dans la file durable et dans tout pending ou archive récupérable, et l'absence de verrou d'un autre processus. Une lecture impossible, un état ambigu, un journal corrompu ou une identité divergente entraîne un refus visible, sans libération.
3. Pour une entrée `Bound` dont la copie **et** la fiche sont toutes deux absentes, retirer seulement l'entrée après ces vérifications. Pour une copie et une fiche présentes et cohérentes, vérifier leur identité et leur contenu, puis les retirer avec la façade Windows par handle déjà existante ; ne jamais supprimer l'audio original, une transcription, une autre copie, une autre destination ni un objet ajouté entre inspection et confirmation. Si un seul des deux fichiers existe, conserver l'entrée et signaler une réparation manuelle nécessaire.
4. Rendre le retrait rejouable : enregistrer durablement l'intention et les identités vérifiées avant la première disposition, puis terminer le nettoyage et la libération de la place. Une interruption à chaque frontière peut être reprise par une nouvelle confirmation sans élargir les cibles. Aucun nettoyage rétrospectif automatique des entrées RTC-01 à RTC-04.
5. Après récupération, actualiser l'état de capacité et permettre à l'utilisateur de relancer l'import. Le refus normal à deux copies **actives** reste inchangé. Une annulation normale du dialogue de doublon et une publication normale libèrent leurs copies comme avant.

## Chemins d'écriture proposés pour le code

- `crates/whisper-core/src/ports.rs` : effets et résultats typés d'inspection/récupération.
- `crates/whisper-core/src/application.rs` : commande, confirmation et état visible.
- `crates/whisper-adapters/src/source_binding.rs` : inspection du registre, retrait durable et vérifications d'identité.
- `crates/whisper-adapters/src/worker_ipc.rs` : routage, connaissance des bindings actifs et résultats.
- `crates/whisper-adapters/src/queue_store.rs` : lecture de file sans création ni modification.
- `crates/whisper-desktop/src/ui.rs` : bouton, liste des places retenues, confirmation et causes de refus.
- `crates/whisper-adapters/tests/source_binding.rs` : deux places, fichiers manquants, copie présente, interruption, concurrence et refus.
- `crates/whisper-desktop/tests/reprocessing_recovery_ui.rs` : visibilité et confirmation explicite, sans effacer une place active.

La façade `windows_file_identity.rs` est **réutilisée sans édition ni nouvel `unsafe`**. Si une capacité de cette façade manque, arrêt `PLAN_CHANGE_REQUIRED` avant d'ouvrir un autre chemin. Aucun autre fichier de code n'est autorisé par ce plan.

## Cartes d'acceptation

| Carte | Scénario | Résultat exigé |
| --- | --- | --- |
| R1 | Deux entrées `Bound`, aucun traitement ; une sans fichiers, une avec copie et fiche cohérentes non verrouillées. | Inspection décrit les deux ; confirmation explicite libère les deux ; un nouvel import est admis. |
| R2 | Copie utilisée par le processus courant ou un autre processus, entrée présente dans la file ou dans un pending. | Place conservée, refus attribué ; aucun audio ni résultat supprimé. |
| R3 | Copie ou fiche seule, chemin redirigé, identité modifiée, journal corrompu, destination inconnue. | Refus sans suppression ni libération de place. |
| R4 | Arrêt après l'intention, après la disposition de la copie, après celle de la fiche, avant la libération du registre. | Reprise explicite idempotente sur les seules identités admises ; aucun autre objet touché. |
| R5 | Deux copies actives, annulation du dialogue, publication réussie. | Limite de deux conservée ; les parcours normaux libèrent la place ; aucune récupération automatique. |
| R6 | Application Test dans la fenêtre Windows après code CLEAN. | L'utilisateur récupère les places abandonnées puis importe ; les causes des places protégées sont visibles. |

## Exécution et remise

Tests uniquement sur des destinations fictives sous `target/parakeet-tests/retraitement-cible/rtc-05/`, compilation dans `target/parakeet-tests/cargo-shared/`, garde à échec fermé et auto-tests avant Cargo, cache et TEMP/TMP sous RTC-05, au moins 30 Gio libres, hors ligne. La garde protège notamment les réglages personnels et `WhisperTranscriptions`. Formatage, tests ciblés puis suite workspace `--no-fail-fast`. Implémentation confiée à `rust_implementer`, revue indépendante par `rust_review_coordinator`, au plus deux cycles. Aucun réseau ni push.

La réparation des **deux entrées personnelles actuellement constatées** ne fait pas partie des tests ni de l'implémentation. Elle sera déclenchée par l'utilisateur dans la fenêtre après livraison et confirmation explicite ; Codex ne modifie pas directement `%LOCALAPPDATA%\Whisper` ou `WhisperTranscriptions` dans ce lot. Le paquet de recette ne sera préparé qu'après revue CLEAN. Tout besoin hors des huit chemins ci-dessus ou de la racine RTC-05 demande un nouvel arbitrage avant édition.
