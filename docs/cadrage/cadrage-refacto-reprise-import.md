Avancée : cadrage ciblé proposé pour la reprise d’un import interrompu.
Blocage : scénarios intégrés de reprise non exécutés ; voie N1 à établir pour le futur lot.
Décision : revue du design requise ; ce document n’autorise aucune modification Rust.

# Whisper — cadrage du découpage de `resume_interrupted`

**Nature :** cadrage technique d’une refactorisation interne ; aucune capacité utilisateur nouvelle.
**État :** candidat à la revue design.
**Base lue :** `main` au commit `6193fbdf` ; code observé identique dans le worktree Clippy `5f59d920`. Le relevé frais gardé du 12 novembre sur six crates conserve `clippy::too_many_lines` dans `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs:9` (`resume_interrupted`, 272 lignes). Les autres workers Turbo ne sont pas couverts.

## BESOIN

Rendre le parcours technique de reprise plus lisible en séparant les étapes de `ImportRuntime::resume_interrupted`, tout en conservant strictement les décisions, erreurs, effets durables et événements actuels. La reprise explicite d’un import interrompu fait partie du produit existant ; le cadrage ne change ni son comportement ni son interface.

### Périmètre et capacités

- **C1 — validation de la reprise :** conserver l’identité de la tâche et de la génération, les contrôles d’ordre FIFO, de configuration, de modèle et de capacité, ainsi que leurs refus.
- **C2 — reconfirmation :** garder le parcours de publication ambiguë, son verrou et l’attente de confirmation. Une reprise ne doit pas être déclenchée avant confirmation ; les sorties déjà publiées et les traitements concurrents gardent leur comportement.
- **C3 — préparation durable :** conserver le préfixe confirmé et le checkpoint retenu, l’identité historique du modèle dans la file et le chemin vérifié utilisé pour l’exécution. Préserver l’ordre des événements `Queue → ResumeCheckpoint → Prepared` et le passage ultérieur à l’exécution.
- **C4 — erreurs et compensation :** garder les formes de retour distinctes (`Failed/Return` et `Continue(Err(...))`), leurs identifiants et causes. La restauration de liaison source reste conditionnée au résultat du transfert et à l’état réellement déplacé ; aucune transaction globale n’est postulée.

**Parcours visible :** l’action explicite place la tâche en `Queued` avec son message de vérification du préfixe ; elle ne passe à `Preparing` qu’après réception d’un `Prepared` valide. Le cadrage n’exige pas l’absence de toute écriture disque avant validation : ouvrir la file peut créer le dossier et réparer son état, et la reconfirmation peut acquérir le verrou de publication.

### Critères d’acceptation du futur lot

- **AC-01 — validation :** mêmes préconditions, contrôles FIFO/identité/modèle/admission, refus, identifiants et causes.
- **AC-02 — reconfirmation :** mêmes demandes et réponses de confirmation ; aucune reprise prématurée ; état inchangé des sorties en cas d’attente ou d’annulation.
- **AC-03 — préparation :** même archive et file, mêmes règles de checkpoint/préfixe, identité durable et chemin d’exécution, même ordre `Queue → ResumeCheckpoint → Prepared`.
- **AC-04 — échecs :** mêmes voies précoces et tardives, mêmes effets et erreurs ; restauration de liaison uniquement dans les conditions actuelles ; aucune compensation ajoutée à une voie qui n’en a pas.
- **AC-05 — découpage :** les helpers privés rendent les étapes nommées et testables ; `resume_interrupted` conserve sa responsabilité d’orchestration et passe sous le seuil Clippy de 100 lignes, sans `allow` ni changement de seuil.
- **AC-06 — non-régression :** tests pertinents, Clippy et revue vérifient les contrats ci-dessus ; les cas non exécutés ou ignorés restent explicitement signalés.

## Domaine et architecture proposés

Les états produit demeurent `Interrupted → Queued → Preparing → Running` selon les événements et validations existants. Le découpage reste dans `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs`, avec des helpers privés, la visibilité actuelle de `resume_interrupted` et le type `RecoveryEffect` inchangés. L’orchestrateur conserve l’ordre des phases et choisit `Return` ou `Continue`.

Frontières de responsabilités proposées : (A) validation archive/file et inspection du modèle ; (B) reconfirmation de publication inchangée ; (C) identité durable et modèle d’exécution, vérification de capacité et choix de route ; (D) transfert des liaisons, contrôles de test existants et enregistrement archive puis file ; (E) calcul du préfixe/checkpoint, installation de l’état mémoire et émission ordonnée des événements. Ce sont des pistes de découpage, à ajuster aux blocs contigus constatés dans la source pendant l’implémentation.

**Dépendances :** l’adaptateur utilise déjà `whisper-core` et ses types de session ; aucune dépendance ou crate nouvelle n’est justifiée. Pas de SPIKE requis : les frontières proposées n’introduisent pas de faisabilité externe ou de nouveau contrat système.

## Preuves, risques et limites

- La source primaire est `crates/whisper-adapters/src/worker_ipc/runtime/recovery.rs`, complétée par `queue_store.rs`, `source_binding.rs`, le runtime d’import du cœur et les tests de reprise existants.
- Le relevé Clippy frais a terminé avec code 0 et le cliquet passe à 153 sites contre 168. L’alerte `too_many_lines` sur `resume_interrupted` est le constat ciblé restant. Les avertissements ciblés du polling, de l’arrêt Parakeet et de l’import contrôlé ne ressortent plus sur ce candidat.
- Des témoins N1 existent pour l’ordre FIFO, plusieurs refus de reprise, la garde du catalogue et le préfixe/checkpoint. Des scénarios intégrés de reconfirmation R7 et des parcours unifiés de transfert source sont ignorés avec `native-tests`; ils ne constituent pas une preuve N1 exécutée.
- La racine/classement N1 du futur lot n’est pas encore approuvée par la garde courante. Le plan devra arrêter son préflight avant toute commande Cargo si la voie permise ne couvre pas les vérifications exigées. Aucun assouplissement de garde n’est inclus.
- Risque principal : déplacer une écriture, une mutation de file ou une compensation au-delà d’une frontière peut modifier la reprise après erreur. L’ordre actuel fait foi ; un helper ne doit pas créer l’apparence d’une transaction atomique.

## Portée et autorisations

Inclus : refactorisation interne de `resume_interrupted` et tests strictement nécessaires dans le crate `whisper-adapters`, sous réserve d’un accord d’implémentation distinct et du préflight de garde.

Exclus : autre alerte Clippy, réduction générale des alertes structurelles, modification de baseline/garde, évolution produit de reprise, nouveaux effets disque, refonte d’archive, changement de modèle, tests GPU de niveau 2, builds natifs suspendus, validation Windows, merge, push et nettoyage.

**Aucune implémentation n’est autorisée par ce cadrage.** Le Clippy frais précédent autorise uniquement le constat rapporté ; toute future commande Cargo, tout chemin et tout lot devront respecter l’autorisation alors applicable.

## Content map et readiness

Les exigences produit nouvelles, UI et persistance hors du parcours existant sont non pertinentes : il s’agit d’un découpage interne sans changement de contrat. Les capacités C1–C4 sont reliées aux AC-01 à AC-04 ; les AC-05/06 couvrent la structure et la preuve future. La revue design indépendante doit vérifier les frontières, les erreurs, les preuves et la distinction entre conception et autorisation d’exécuter.

**Readiness proposée :** cadrage prêt pour revue design ; aucune décision produit ouverte. L’exécution future reste conditionnée à une autorisation précise, à un plan accepté, à la voie N1 conforme et aux vérifications de la crate.
