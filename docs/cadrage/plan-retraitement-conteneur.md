Avancée : la recette RTC-03 a été validée par l'utilisateur ; seul le dossier conteneur vide reste à retirer après remplacement.
Blocage : le code RTC-02 conserve volontairement `transcriptions/<job_id>/` après retrait de sa génération.
Décision : demander l'accord sur le lot RTC-04 ci-dessous avant tout code.

# RTC-04 — retirer le conteneur vide d'un traitement remplacé

Décision utilisateur du 20/10/2026 : « dossier conteneur » désigne
`<destination>/transcriptions/<job_id>/`, parent de `1/` ou d'une autre
génération. Le retirer seulement s'il est devenu vide à la suite du
remplacement. Conserver `transcriptions/`, tout parent contenant une génération
non sélectionnée, un fichier ou un sous-dossier, et tous les parents de la
nouvelle publication. Aucune purge rétroactive de dossiers préexistants.

## Lot et chemins

RTC-04 suit le candidat RTC-02 commité `e6520bc1`, la revue CLEAN R01–R06 et
la recette utilisateur RTC-03. Écritures de code et de tests **uniquement** :

- `crates/whisper-adapters/src/archive.rs` ;
- `crates/whisper-adapters/tests/reprocessing_cleanup.rs`.

Écritures d'exécution exclusivement sous
`target/parakeet-tests/retraitement-cible/rtc-04/` et le Cargo partagé
`target/parakeet-tests/cargo-shared/`. `models/`, `sound example/`, les
transcriptions utilisateur, les workers, DLL et l'installation personnelle
restent en lecture seule. Aucun réseau ni push. L'API Windows T6 existante
dans `windows_file_identity.rs` est réutilisée sans modifier ce module ni
ajouter d'autre `unsafe`.

## Contrat de suppression

1. À la confirmation, capturer dans l'intention et le plan durables l'identité
   Windows exacte du conteneur admissible. Le conteneur n'est admissible que
   si toutes ses générations publiées et présentes sont dans la sélection de
   remplacement. Ne jamais déduire un droit de suppression d'un simple nom ou
   d'un chemin reçu de « Parcourir ».
2. Publier et vérifier le nouveau traitement avant toute disposition. Retirer
   les anciens objets comme en RTC-02. Une fois toutes les générations de ce
   conteneur retirées, vérifier à nouveau son identité, son chemin canonique
   et son contenu. Supprimer **ce seul conteneur vide** par le handle vérifié.
   Un contenu nouveau ou non sélectionné le fait conserver et signaler.
3. Enregistrer durablement l'état de retrait du conteneur. Une interruption
   avant/après la disposition doit être rejouable sans seconde publication ni
   suppression d'un autre conteneur. Une intention RTC-02 existante sans
   preuve de conteneur ne reçoit jamais cette nouvelle suppression au réveil.
4. Une erreur de retrait laisse la nouvelle transcription disponible et
   signale le conteneur restant pour reprise explicite ; elle ne déclenche
   aucune suppression supplémentaire automatique.

## Acceptation

| Carte | Scénario isolé | Attendu |
| --- | --- | --- |
| K1 | Un traitement remplacé, seul dossier de génération du conteneur. | Ancien `1/` et son conteneur supprimés ; nouveau dossier et `transcriptions/` conservés. |
| K2 | Deux générations dans un conteneur : sélectionner une, puis toutes. | Conteneur conservé tant qu'une génération reste ; retiré seulement lorsque toutes sont retirées. |
| K3 | Fichier tiers, nouvelle génération, reparse point ou identité du conteneur changée. | Aucun objet non sélectionné supprimé ; conteneur conservé et cause visible. |
| K4 | Échec ou interruption avant/après disposition, puis reprise explicite. | Nouvelle paire TXT/SRT conservée ; reprise idempotente, aucune cible ajoutée. |
| K5 | Ancien plan RTC-02 sans preuve de conteneur. | Reprise historique inchangée ; pas de suppression rétroactive. |
| K6 | Modes Dernier, Tous et Parcourir, source SHA et non-régression. | Même sélection qu'en RTC-02, aucune extension de propriété ou de destination. |

Préflight : HEAD/statut Git et au moins 30 Gio libres ; garde RTC-04 à échec
fermé et auto-tests avant Cargo ; `CARGO_HOME`, TEMP/TMP sous RTC-04 et
`CARGO_TARGET_DIR` sur `cargo-shared`, hors ligne. Vérifier formatage, tests
ciblés et `cargo test --offline --locked --workspace --no-fail-fast` sous
garde. Revue indépendante par `rust_review_coordinator`, au plus deux cycles.
Livrer le diff, le statut K1–K6, les commandes, FAIL/NOT RUN et le verdict.
La recette dans la fenêtre Windows reste celle de l'utilisateur ; aucun paquet
ni commit de code avant CLEAN. Tout besoin d'un troisième fichier donne
`PLAN_CHANGE_REQUIRED` sans édition de ce fichier.
