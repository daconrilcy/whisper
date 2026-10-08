# Contrat domaine L04 — D26 DESIGN DRAFT

Owner AUTH-DOM-L04-D24. Contrat intégré depuis le TRANSPORT raw DOM-L04-D25-RECOVERY-v2 et soumis à revue. Remplace les contrats domaine L04 de D24.

# DOM-L04-D25-RECOVERY-v2 — correction domaine D24-003

**D26 : contrat intégré à la candidate DESIGN DRAFT, soumis à revue.** Auteur `/root/d24_domain_arch`, rôle `rust_domain_architect`. Owner de rédaction proposé : `AUTH-DOM-L04-D24`. Cette version remplace `DOM-L04-D25-RECOVERY-v1` et précise les contrats nécessaires à la correction de **D24-003**. Le reviewer reste propriétaire de sa fermeture.

Base : **D-WHISPER-24**, digest `80e8bbfef4c0c671e7eba47d73f73ec15bf876477f71d580dea931e80bb27793`, manifeste SHA256 `b12f527b7f3c9fc155b5b46321e541adabb70165fd0d61ea5bcd3cdf86f21357`, 322 fichiers vérifiés concordants lors de la lecture. Sources : D24/08/50/59/60/61, `ARCH-L04-CHANGE-v3`, réponses Q-04/Q-06/Q-09 et DETAIL-P03, décisions utilisateur D24. `ARCH-L04-BOUNDS-v1` est consulté comme contribution conversationnelle attribuée, encore non persistée.

Aucun fichier édité, aucun test exécuté, aucune bibliothèque choisie, aucun READY/CLEAN revendiqué.

## 1. Priorité des contrats et décisions acceptées

D25 remplace les formulations héritées qui assimilent génération worker et passage, ou imposent l’arrêt de capture à toute erreur worker.

- **DEC-L04-CAPTURE-AUTO-01 acceptée :** en Auto GPU→CPU, la capture existante continue sous bornes.
- **DEC-L04-STRICT-FINISH-01 acceptée :** après panne GPU forcé et choix CPU explicite, terminer le même passage depuis le PCM conservé, sans rouvrir le microphone.

La continuation Auto cesse lorsqu’une saturation ou une erreur empêche effectivement la conservation. Une attente longue ne constitue pas cet événement.

## 2. Identités et ownership

| Identité / donnée | Propriétaire et invariant |
|---|---|
| Groupe, passage/storage_generation, import | Domaine valide ; application possède l’identité durable. Le secours ne la change pas. |
| Flux de capture | Application possède identité/admission ; adaptateur possède le handle. PCM, compteurs et VAD restent attachés au passage/flux, indépendamment des tentatives moteur. |
| Tentative `(namespace, generation, instance)` | Application décide ; adaptateur lance et observe. Événements moteur corrélés à cette tentative seulement. |
| Confirmation et checkpoint | Domaine valide ; adaptateur stockage produit les données autoritatives à partir des records durables. |
| Stop/Quitter/annulation | Application possède les intentions mémorisées et leurs cibles. |
| Vue | UI présente sans état métier parallèle ni effet direct de stockage/native. |

Un changement de tentative ne rend pas obsolètes les progrès de la capture existante. Un ancien événement moteur ne peut avancer capture, confirmation, publication ou arrêt du successeur.

## 3. Formats logiques proposés

**`RecoveryCheckpoint v2` :** type live/import ; identité durable ; namespace/génération réservée ; offset de groupe ; sample rate ; configuration/modèle figés et identités ; frontières locales T/A ; état capture continue ou fermée et fin scellée éventuelle ; dernier commit/segment confirmé ; références aux preuves PCM/source et confirmation.

**`CoverageConfirmation v2` :** identité durable ; numéro de commit ; plage traitée `[start,end)` ; zéro ou plusieurs segments identifiés et leurs plages ; provenance tentative ; référence audio/source vérifiée ; contrôle d’intégrité.

**`AttemptJournal v1` :** identité durable ; séquence de record ; `Reserved`, `Started` ou `Retired` ; namespace/génération/instance si connue ; checkpoint de départ ; mode `ContinueCapture` ou `FinishPreservedPassage` ; contrôle d’intégrité.

Writer unique pour chaque journal ; lecture/scan progressifs, sans chargement intégral requis. Les schémas physiques, framing et primitives de sync restent à l’architecte technique.

## 4. Confirmation, silence et MP3

Pour le live :

- T : fin de couverture de transcription confirmée ;
- A : fin du préfixe PCM contigu durable ;
- C : fin capturée connue ;
- E : curseur d’encodage du pending de la tentative.

**Invariant : `0 ≤ T ≤ A ≤ C`.** E ne prouve ni T ni une publication.

Les commits avancent la couverture contiguë depuis T. Une plage silencieuse produit une confirmation avec zéro segment. T n’est pas calculé depuis le dernier mot.

DurableAck suit la validation et durabilité du PCM/source, des résultats et du record de couverture. ACK perdu mais commit valide : réconciliation adopte le commit vérifié ; ACK reçu sans record valide : aucune confirmation.

Le successeur reconstruit un nouveau pending MP3 du même passage :

- `[0,T)` : encodage seul, texte confirmé inchangé ;
- `[T,A)` : encodage et traitement avec confirmations ;
- plages ultérieures Auto : même traitement, dans l’ordre.

Après interruption, E repart de zéro dans un nouveau pending ; T vient du journal vérifié et peut avoir avancé. Aucun flux encodeur défaillant n’est concaténé. Chaque tentative encode ses plages une fois ; seule une publication vérifiée devient visible comme Complete.

## 5. Barrière Auto finie avec PCM continu

La barrière porte sur les **transactions de confirmation de l’ancienne inférence**, pas sur toute l’activité stockage.

1. Accepter une panne GPU éligible, établie et corrélée.
2. Fermer l’admission des nouveaux résultats moteur anciens ; fixer un **watermark des transactions de confirmation déjà admises**. Cet ensemble est fini.
3. Demander l’arrêt coopératif de l’ancienne tentative ; capture et writer PCM peuvent continuer.
4. Régler chaque transaction du watermark par une issue vérifiable : commit durable adopté, absence définitive de commit, ou erreur. Une issue inconnue maintient l’attente ; elle n’est pas assimilée à un échec annulable.
5. Attendre arrêt corrélé ou sortie effectivement constatée de l’ancien enfant.
6. Produire T stabilisé et un snapshot A vérifié. Les nouvelles écritures PCM peuvent continuer après ce snapshot.
7. Vérifier les intentions ; Stop/Quitter/annulation admis avant le lancement empêchent le nouveau spawn automatique.
8. Réserver et synchroniser une génération CPU nouvelle ; revérifier les intentions.
9. Spawn CPU, handshake compatible, attestation, puis reconstruction/replay.

Une capture croissante n’empêche donc pas la fin de la barrière. Le CPU traite ensuite les préfixes durables supplémentaires ; fin du préfixe disponible ≠ fin du passage.

Erreur de confirmation/PCM empêchant conservation : fermer l’entrée concernée, préserver les preuves et Recoverable. Aucun délai ne force l’arrêt ou le secours.

## 6. Réservation et incertitude après crash

Une réservation valide est durable avant spawn et consomme son numéro, même sans Started. Le prochain numéro dépasse tous les Reserved valides ; débordement interdit le lancement.

**Reserved sans Started signifie « lancement possible, état inconnu ».** Un crash peut suivre le spawn et précéder Started. Aucun redémarrage automatique ; avant reprise explicite, établir que les anciens enfants ne peuvent plus modifier le staging utilisé.

Suffixe incomplet/non validé : ignorer son contenu pour les décisions, préserver sa preuve. Il ne peut autoriser un spawn. Corruption interne, séquence impossible ou identité discordante : Recoverable, sans la traiter comme une simple queue interrompue.

## 7. Compatibilité et migration

- Complete historique : lecture selon sa version, sans réécriture.
- Pending connu : valider identités, PCM/source et confirmations.
- Couverture contiguë démontrable, silence inclus : migration v2 référant les preuves historiques, originaux conservés.
- Pending sans couverture démontrable : **Recoverable, fragments/PCM accessibles, nouveau replay refusé avec motif**. Ne jamais inventer T à partir des fins de segments.
- Version inconnue ou incohérence : préservation et refus du traitement non supporté.

Les parcours historiques effectivement supportés restent préservés. Cette restriction du nouveau replay ne supprime pas les fragments confirmés.

Sans journal de tentatives historique, migration admissible crée un namespace neuf, frontière zéro, première réservation un ; les générations de stockage historiques restent distinctes. Événements d’ancien protocole/namespace refusés.

Import : identité queue/archive et FIFO conservés ; hash/source, sample rate et axe revalidés avant replay. Source modifiée, version incompatible ou conversion d’offset ambiguë : refus sans modification de source.

## 8. GPU forcé, Stop, annulation et CPU échoué

GPU forcé : panne → fermer micro → drainer capture admise → régler confirmations/arrêt → sceller PCM → attendre choix CPU. Choix courant explicite → réservation → CPU `FinishPreservedPassage` → finalisation du même passage. Aucun micro rouvert.

| Événement | Issue |
|---|---|
| Stop/Quitter avant spawn | pas de nouvel essai automatique ; fermer capture et stabiliser ; Complete seulement si vérifiable, sinon Recoverable |
| Stop/Quitter live après CPU lancé | fermer capture, drainer avec tentative courante ; aucune nouvelle tentative si elle échoue |
| Quitter GPU forcé sans choix | ne vaut pas choix CPU ; préserver Recoverable, attendre arrêt/stabilisation puis sortir |
| Annulation import | interrompre, invalider tentative, préserver source/confirmations ; aucun retry |
| CPU échoué | fermer capture concernée ; préserver PCM/commits/publication précédente ; Recoverable ou Interrupted, sans boucle |
| Reconstruction interrompue | pending non publiable ; prochaine reprise explicite reconstruit avec T courant vérifié |
| Arrêt non confirmé | UI interactive ouverte et diagnostic ; aucune force |
| Masquer fenêtre | aucune transition métier |

Après crash, import commencé devient Interrupted ; aucun moteur ni micro ne démarre spontanément.

## 9. Impacts REQ/UC/AC proposés

Owner produit : AUTH-USER ; rédaction domaine/requirements ; statut **proposé DRAFT**.

- **REQ-07 v3, UC-07 révision D25, AC-07 v3 :** Auto, watermark fini, capture continue, identité stable, replay sans doublon.
- **REQ-08 v3, UC-07 D25, AC-07 v3 :** CPU explicite après GPU forcé, PCM scellé, micro fermé.
- **REQ-18 v3, UC-13 D25, AC-13 v3 :** couverture incluant silence, PCM/texte distincts, migration et pertes honnêtes.
- **REQ-11 v3, UC-09 D25, AC-09 v3 :** obligations indépendantes ; aucune transition destructive par timer.
- **REQ-12/15/16/17/25 :** garanties conservées ; préciser leurs scénarios dans UC-10/11/12/20 et AC-10/11/12/20, avec provenance/version enregistrées par le coordinateur.

AC supplémentaires, **NOT RUN** : silence sans texte ; PCM continu pendant ACK retardé ; commit valide avec ACK perdu ; réservation complète sans Started ; suffixe interrompu/corruption interne ; legacy migrable/refusé ; crash pendant encodage seul ou inférence ; Stop/Quit avant/après spawn ; CPU échoué ; source import modifiée. Oracles : identité stable, T non régressif, absence de faux Complete/double audio, micro fermé quand requis, préservation et cause visible.

## 10. Travail restant

Architecte technique : erreurs GPU éligibles, schémas physiques, writer exclusif/isolation pending, preuves d’arrêt, sync/publication et lecteurs compatibles. Intégrer l’inventaire des bornes sans promettre RSS global, quota spool ou délai native.

Coordinateur/hôte : persister cet apport exact, ses sources et versions ; qualifier les formulations remplacées ; enregistrer tâches/owners et reçu/hash au checkpoint. Reviewer : vérifier le successor et D24-003. Validation : campagnes produit/native/UI/coupures **NOT RUN**.

