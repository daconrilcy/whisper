Avancée : alerte `type_complexity` rattachée à une refactorisation locale.
Blocage : l’implémentation et ses validations ne sont pas autorisées par ce cadrage.
Décision : recommander un résultat nommé interne, sous réserve de conserver le contrat observé.

# Whisper — résultat nommé de transcription Parakeet

**Nature :** cadrage d’une refactorisation locale issue du Clippy frais du 13 novembre 2026.
**État :** cadrage et plan prêts pour examen ; aucune implémentation autorisée.

## BESOIN

Rendre explicites les données de succès retournées par `transcribe_one` afin que leur rôle soit visible à la lecture et à l’usage, tout en préservant le traitement actuel des transcriptions Parakeet.

**Observations vérifiées :** au commit `6cfc4106f6c89569767f069d67859d1b0410031e`, une campagne Clippy gardée depuis le worktree `C:\wt\refacto-reprise-import` s’est terminée avec le code 0 sur toutes les cibles de six crates : `whisper-core`, `whisper-adapters`, `whisper-desktop`, `whisper-bootstrap`, `whisper-worker-common` et `whisper-worker-parakeet-cpu`. Le cliquet a relevé 152 sites structurels contre 168 dans la baseline. Une alerte `clippy::type_complexity` couvre la signature de `transcribe_one` dans `crates/whisper-worker-parakeet-cpu/src/transcription.rs:29-38`. Journal brut : `C:\wt\refacto-reprise-import\target\parakeet-tests\garde-n1-paquets-20261112\scratch\20261113T044433373-dab8190115cd42e28afd4669ae6dc5ae\cargo-command.stdout.log`.

La fonction retourne un tuple de succès à cinq champs : nombre d’échantillons, texte, mots, SHA-256 de la source et durée d’inférence native. Le seul appelant observé est `crates/whisper-worker-parakeet-cpu/src/session.rs:180` ; il transmet ces valeurs à `Event::Result`. L’erreur reste un couple code/message classé par `classify_native_error`. `transcription.rs` et `session.rs` sont des modules frères : le type et ses champs devront avoir la visibilité minimale qui permet cet usage, à confirmer au préflight (par exemple `pub(super)` selon l’arborescence), sans API publique.

**Comportement attendu :** remplacer le tuple de succès par une structure nommée dont les champs décrivent ces cinq valeurs, avec la visibilité minimale permettant leur lecture depuis `session`. Le type reste interne au worker et ne devient pas une nouvelle forme sérialisée.

**Critères d’acceptation :**

- **CA-01 — champs explicites :** les cinq champs nommés correspondent chacun à la valeur actuelle ; aucune valeur n’est perdue, réordonnée, recalculée ou substituée.
- **CA-02 — résultat équivalent :** à entrée identique, le worker produit les mêmes valeurs de résultat et la même `Event::Result`.
- **CA-03 — erreurs et callback :** codes/messages d’erreur, classification, moment d’appel du callback `on_native_call_started` et ordre des effets restent identiques.
- **CA-04 — protocole :** le contenu et la sérialisation des événements IPC restent inchangés ; aucun type de protocole n’est modifié.
- **CA-05 — visibilité :** `session` peut lire le type et les champs au moyen d’une visibilité limitée à l’usage interne requis ; aucune API publique n’est ajoutée.
- **CA-06 — Clippy :** l’alerte ciblée est fermée sur le même périmètre mesuré, sans nouvelle alerte structurelle.
- **CA-07 — validations traçables :** chaque commande autorisée et exécutée est rapportée avec son résultat ; compilation CPU n’est pas présentée comme validation native, GPU ou matérielle.

**Périmètre proposé :** `crates/whisper-worker-parakeet-cpu/src/transcription.rs` et `src/session.rs` dans cette crate ; tests existants concernés, avec ajout seulement si un invariant n’est pas déjà couvert.

**Hors périmètre :** les autres alertes Clippy, les workers Turbo CPU/GPU non mesurés, `resume_interrupted` (absent du relevé frais), tout changement d’algorithme, de routage moteur, de feature, de protocole, de FFI, de manifeste ou de garde, ainsi que les builds natifs et la recette matérielle.

## ACCORD PRÉCIS POUR CETTE SESSION

**Objectif autorisé :** rédiger ce cadrage et le plan associé dans `docs/cadrage/`.

**Accord explicite :** l’utilisateur a demandé le cadrage et le plan. Selon `AGENTS.md`, la session principale écrit ces livrables et les commite localement sur la branche du lot ; aucun commit sur `main` ni push.

**Base observée :** `C:\dev\whisper`, `main` à `6cfc4106f6c89569767f069d67859d1b0410031e`, propre lors du contrôle. Les sources concernées, `AGENTS.md` de la crate, la carte d’architecture et le journal Clippy ci-dessus ont été consultés.

**Non autorisés par cette demande :** modification du code produit, exécution de Cargo ou de tests, build natif, changement de garde, validation dans la fenêtre Windows ou push. La rédaction et le commit local des seuls documents ne valent pas autorisation d’implémentation ni de campagne.

**Règle d’arrêt :** une divergence qui change les valeurs, l’erreur, le callback, les effets, les événements ou leur ordre, ou qui nécessite d’élargir la visibilité au-delà de l’usage interne requis, impose l’arrêt du futur lot et un arbitrage avant élargissement.

## À FAIRE

1. **[CA-01, CA-02]** Confirmer l’origine et la consommation des cinq champs dans les deux sources autorisées.
2. **[CA-03]** Établir les scénarios d’erreur et le placement temporel du callback à préserver.
3. **[CA-04]** Confirmer que le type nommé reste interne au worker et ne touche pas le contrat IPC.
4. **[CA-05]** Confirmer l’arborescence et la visibilité minimale nécessaire depuis `session`.
5. **[CA-06, CA-07]** Définir les vérifications futures permises par la garde et leurs limites de couverture.
6. **[CA-01 à CA-07]** Remettre un plan futur limité à six tâches maximum, puis arrêter le cadrage.

## DÉCISIONS À VALIDER

- **D-01 — Forme et visibilité du résultat :** recommandation : une structure nommée dans le module de transcription, avec champs sémantiques et visibilité minimale permettant l’accès depuis `session` (par exemple `pub(super)`, à confirmer). Un alias du tuple pourrait faire disparaître le diagnostic sans rendre l’association des valeurs aussi lisible. Ne pas élargir l’API publique.
- **D-02 — Autorisation future :** l’implémentation ne commence qu’après examen/acceptation du cadrage et du plan, puis accord écrit précis dans une nouvelle session. Le profil `rust_implementer` et les revues Rust prévus par `AGENTS.md` s’appliqueront alors.

## RAPPORT

Cette proposition cible une alerte mesurée et un point d’appel unique, sans modifier la forme IPC. Le relevé porte sur six crates ; `whisper-worker-cpu` et `whisper-worker-gpu` n’ont pas été mesurés. Le changement proposé ne justifie aucune conclusion sur l’exécution du moteur natif.
