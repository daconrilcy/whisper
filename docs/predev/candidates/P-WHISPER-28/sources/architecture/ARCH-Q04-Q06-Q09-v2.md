# ARCH-Q04-Q06-Q09-v2

Auteur : `/root/q04_q06_q09_arch` — rôle `rust_architect`, lecture seule. Date : 2026-10-07.

Ce document est une **nouvelle contribution rédigée**, destinée à une persistance fidèle en TRANSPORT. Il n’est pas un export des octets de ma réponse précédente. Aucun fichier, registre, checkpoint ou code n’a été modifié par son auteur.

## Base, attribution et statuts

Base relue le 2026-10-07 :

| Objet | Identité / SHA256 |
|---|---|
| État actif `docs/predev/state.json` | `0c2c8fc8dbafd29f94e78875f4825314e3906ce3324b4118493974904c5b795e` |
| DESIGN | `D-WHISPER-19` |
| DESIGN manifest digest | `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529` |
| PLANS | `P-WHISPER-10`, parent `D-WHISPER-19` |
| PLANS manifest digest | `9b9edfcf09e39bd6017fb818fd0b9784923b8eb89707b18a9e3a1fff4d7b8f4a` |

Les bindings de l’état actif indiquent les revues `R-WHISPER-DESIGN-19` et `R-WHISPER-PLANS-10-v3`, verdicts `CLEAN`. Cette contribution ne constitue pas une nouvelle revue de ces corpus.

Propriétaire de registre : `AUTH-COORD`. Auteur technique : acteur indiqué ci-dessus. Entrées concernées : `Q-04`, `Q-06`, `Q-09`, réponses proposées `answer_version=1`, nature `reversible_detail`, échéance avant `L-WHISPER-04`. Q-06 et Q-09 affectent également L05. Dans l’état relu, les trois questions restent `open`.

Statut de cette contribution : réponses techniques proposées, `ANSWERED_DETAIL` à faire accepter et enregistrer par l’autorité compétente. La politique de conservation Q-09 a été **acceptée par l’utilisateur selon la transmission du coordinateur**, strictement pour rotation bornée, quatre fichiers de 2 MiB, sept jours maximum, journaux diagnostics uniquement. La source utilisateur doit être conservée par l’hôte. Cette acceptation ne couvre pas les autres valeurs techniques proposées ci-dessous.

## Sources locales et hashes vérifiés

Chemins relatifs à `C:\dev\whisper\docs\predev`. Les hashes suivants ont été recalculés :

| Source | SHA256 |
|---|---|
| `candidates/P-WHISPER-10/04_OPEN_DETAILS.md` | `111d740efd7f5b3e9f091deaf2c3b0cd9fdb2a70a8ab8d217042093853a69aaa` |
| `candidates/P-WHISPER-10/sources/design/08_STATE_AND_PORT_CONTRACTS.md` | `ffa9efb206c42941a9c846cfdf6a1783b766781472c556e1aad4187958406240` |
| `candidates/P-WHISPER-10/sources/design/50_ARCH_DECISIONS_D18.md` | `bac81d0eca399cd7cef48a0e87eb778aa60c0981daf35e03a959dd265dec6b80` |
| `candidates/P-WHISPER-10/sources/architecture/ARCH-L01-INTEGRATION-v2.md` | `1c8675f6d1e0f18e05556e8e3118d824e1c95591aca37e8d4e6823733787ea27` |
| `P10-L03-CLOSURE-WORK/transports/T-WHISPER-DETAIL-P03-RESOLUTION-01/DETAIL-P03-resolution.md` | `d80e2aa581926981f5ff138d9cec82940fcac626d46707c9b033e32aef766e98` |

Sources normatives lues : `rust-predev-design/SKILL.md`, les cinq règles centrales, `engineering-contract.md`, `handoff-contract.md`, `deliverable-quality.md`.

Traçabilité : Q-04 → REQ-11/UC-09/AC-09 ; Q-06 → REQ-15/16 et DETAIL-P03 ; Q-09 → REQ-11/20. TECH-D18-04/06/08 fixent isolation des appels bloquants, durabilité et frontières.

## Q-04 — progrès et diagnostic

Option retenue : suivre **chaque obligation pendante par étage**. Rejeter délai déclenchant arrêt et santé du processus considérée seule comme preuve de progrès.

| État / opération | Progrès attendu |
|---|---|
| Idle, Queued, AwaitingChoice, états terminaux stables | Aucun progrès de traitement exigé |
| Preparing | Préparation réellement achevée, puis Ready/attestation corrélée |
| Capturing | Axe des échantillons reçus ; persistance pendante suivie séparément |
| Import Running | Avance réelle de décodage, segment achevé, reçu durable |
| Inférence soumise | Avance réelle exposée ou résultat de fenêtre achevée ; présence du processus insuffisante |
| Draining | Réduction des buffers pendants, reçus durables, fin de flux |
| Finalizing | Encodeur terminé, artefact fermé/synchronisé/vérifié, publication confirmée |
| Cancelling / Quitting | Commande admise, confirmation d’arrêt, ressources et état durable stabilisés |

Silence VAD avec capture et persistance actives est attendu. L’absence de texte n’est pas seule une panne. Un événement répété ou obsolète ne réinitialise aucun compteur. L’activité capture ne masque pas un journal ou une inférence sans progrès.

Valeurs **techniques proposées par l’architecte** :

- Polling superviseur au plus une fois par seconde.
- Avertissement à partir de 60 s sans progrès de l’obligation pendante ; prochaine observation nominale au plus tard à 61 s, sans garantie de scheduling OS.
- Diagnostic inchangé renouvelé au plus toutes les 30 s ; nouvelle erreur ou transition publiée immédiatement.
- Un avertissement par épisode et obligation ; résolution uniquement par progrès réel ou fin de l’obligation.
- Horloge relative monotone avec arithmétique contrôlée ; anomalie d’horloge ou reprise OS ne prouve jamais une panne.

Sorties : attente attendue, lenteur/absence de réponse suspectée, panne établie. Diagnostic : identité technique, phase, backend attesté, âge du progrès, profondeur/capacité des files, âge du reçu durable, codes d’erreur contrôlés, état connu du processus. Information indisponible reste inconnue.

Panne établie exige événement explicite : erreur capture/disque, sortie inattendue de l’enfant, protocole invalide ou saturation constatée. Elle autorise cessation de l’entrée concernée, préservation Recoverable et alerte. **Aucun timer ne déclenche arrêt, kill, fallback ou publication.** Inférence longue ou enfant vivant sans réponse restent en attente/diagnostic. La politique Auto conserve ses conditions d’erreur et son rejet de génération obsolète existants.

## Q-06 — Quitter sans confirmation

Retenir l’option attente/diagnostic de DETAIL-P03. Aucun geste de terminaison forcée supplémentaire.

Quitter en live coupe l’entrée puis draine/finalise ; Quitter en import annule, invalide la génération et préserve la source. La demande de sortie reste mémorisée ; aucun nouveau travail n’est admis pendant cette phase. Les clics répétés ne créent pas de commandes concurrentes.

Message immédiat proposé : « Arrêt en cours. L’application reste ouverte pendant la finalisation. »

Après l’avertissement Q-04 : « L’arrêt n’est pas confirmé. L’application reste ouverte ; les données confirmées sont conservées. »

Une erreur établie expose étage, code et état récupérable ; la plage non confirmée est indiquée lorsqu’elle est connue. Aucune garantie de durabilité n’est donnée aux buffers non confirmés.

La fenêtre reste interactive et ouverte tant que l’arrêt nécessaire n’est pas confirmé. Confirmation tardive de l’enfant courant et stabilisation durable permettent la sortie normale. Une finalisation terminale échouée avec enfant terminé préserve Recoverable et permet la sortie sans faux Complete selon REQ-15. Un événement ancien ne confirme jamais l’arrêt courant.

Aucun IO, FFI, `join` ou attente arbitrairement bloquante sur le thread UI. Aucun délai maximal de terminaison native promis. Aucun bouton de force ou export de diagnostic ajouté par cette réponse.

## Q-09 — rotation des diagnostics

**Choix utilisateur transmis par le coordinateur :** rotation bornée, quatre fichiers de 2 MiB chacun, actif inclus ; budget total 8 MiB ; sept jours maximum ; journaux diagnostics uniquement.

Le plafond de taille peut réduire la conservation sous sept jours. Aucun minimum de conservation n’est garanti. L’application applique expiration au démarrage et avant rotation/écriture ; lorsqu’elle ne s’exécute pas, aucune tâche OS n’efface les fichiers. Le fichier actif est renouvelé avant dépassement de taille ou à la première écriture après expiration. Supprimer les fichiers clos les plus anciens d’abord.

Portée exclusive : fichiers diagnostics connus créés par l’application dans son répertoire technique local dédié. Exclure journal produit, fragments, pending de récupération, MP3/TXT/SRT, manifests/pointeurs, sources import et dossier d’archives choisi par l’utilisateur.

Schéma fermé : version logiciel/protocole/politique, UTC, IDs opaques, état/backend, code enum/numéro OS, durées, compteurs et capacités. Interdire audio/PCM, transcription, contenu segment, chemins/noms utilisateur, commande, environnement, dump et message natif libre. Aucun réseau. Ne pas sérialiser `Debug`/`Display` natif libre.

Détails **techniques proposés par l’architecte, sans acceptation utilisateur revendiquée** :

- Un writer parent/adaptateur ; workers transmettent les seuls événements techniques corrélés.
- Admission non bloquante ; file de 128 enregistrements maximum, chacun limité à 4 KiB avant admission.
- Budget de payload en mémoire 512 KiB ; overhead séparé à mesurer.
- Snapshot diagnostic remplaçable par sa dernière valeur ; répétitions regroupées et compteur de pertes exposé.
- Enregistrement hors schéma/plafond refusé, sans troncature ambiguë.
- Réserver la place avant écriture ; purge impossible ou erreur open/write suspend l’écriture diagnostique, expose dégradation et ne dépasse pas le budget.
- Aucune synchronisation durable par enregistrement ; conservation best effort, suffixe incomplet ignoré après coupure.

Une saturation diagnostique peut perdre des événements techniques ; elle ne perd ni commande Stop, ni fragment produit confirmé. La récupération des transcriptions ne dépend jamais de ces logs. La rotation ne promet aucune transaction entre fichiers.

## Frontières, risques et impact

Application possède politique, identité, obligations et transitions ; adaptateur superviseur possède observation processus et writer ; enfant possède contexte FFI ; UI consomme une vue pure ; root assemble. Aucune nouvelle bibliothèque, dépendance native, abstraction spéculative ou permission réseau requise.

Contrôles futurs : dépendances Cargo/features, imports par module, absence UI→adapters et natif parent, CPU sans CUDA, confinement du nettoyage. Railguard actif et RG-08 demeurent applicables ; aucun railguard activé/modifié ici.

Risques proposés, owner architecte/AUTH-COORD :

- `RISK-T-Q04-01` : faux avertissement par obligation mal modélisée.
- `RISK-T-Q04-02` : activité d’un étage masquant un autre étage bloqué.
- `RISK-T-Q06-01` : sortie ou Complete confirmé par événement obsolète.
- `RISK-T-Q09-01` : nettoyage hors diagnostic.
- `RISK-T-Q09-02` : fuite via erreur native libre.
- `RISK-T-Q09-03` : diagnostic incomplet après saturation/expiration.

`design_change_required=false` pour les trois réponses dans ce périmètre. Revenir au design si modification des garanties, conservation des données produit, contenu collecté, réseau, terminaison forcée ou arrêt au seul temps.

P10 `01_LOTS.md` mentionne IPC/ports/root/UI pour L04, alors que l’allowlist L04 de l’état relu contient cinq chemins seulement. Tout fichier supplémentaire réellement nécessaire exige résolution de portée par le coordinateur avant code. Cette contribution n’accorde aucune autorisation de fichiers.

## Preuves et travail restant

**PRODUCT_VALIDATION — tous scénarios NOT RUN :**

1. Horloge contrôlée à 59/60/61 s : avertissement seul, sans arrêt/fallback/publication.
2. Silence VAD long, capture et reçus durables actifs : aucun faux diagnostic de panne.
3. Inférence longue/processus vivant : warning possible, retour courant accepté, aucune terminaison.
4. Capture avançant et DurableAck figé : diagnostic indépendant du journal.
5. Saturation data réelle : Stop admis via contrôle indépendant.
6. Erreurs disque/capture/protocole ou sortie enfant injectées : cessation concernée, Recoverable, cause visible.
7. Quitter sur enfant bloqué : vraie fenêtre interactive ouverte, aucune force ni faux Complete ; déblocage courant puis sortie normale ; événement obsolète refusé.
8. Sentinelles audio/texte/chemins/messages natifs : aucune fuite diagnostique.
9. Dépassement taille/âge, fichier externe et purge refusée : budget tenu, portée respectée, dégradation visible.
10. Coupure pendant rotation : suffixe incomplet toléré, récupération produit indépendante.

Chaque résultat conservera stimulus, branche exercée, effet, témoin, fingerprint, logs/hashes et limites. Fixture pure ne qualifie pas enfant natif ou UI.

**DESIGN_FEASIBILITY :** complément de contrat proposé compatible avec la séparation déjà décidée ; aucun nouveau succès expérimental revendiqué, aucun SPIKE exécuté. **DELIVERY_QUALIFICATION :** packaging final, panne physique et plateforme cible restent NOT RUN.

Sources primaires consultées le 2026-10-07 : [Instant](https://doc.rust-lang.org/std/time/struct.Instant.html), [canal borné](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html), [File::sync_all](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all). Documentation actuelle, distincte de la toolchain projet et de toute preuve runtime Whisper.

Restant à AUTH-COORD/hôte : persister ce texte attribué en TRANSPORT ; vérifier reçu/hash et fidélité ; conserver la source exacte d’acceptation Q-09 ; accepter les détails techniques par autorité applicable ; fusionner par ID dans un pending contrôlé ; résoudre la portée L04 ; contrôler puis promouvoir l’état et relancer son préflight. D19/P10 restent immuables. Aucune question n’est fermée par l’auteur de cette contribution.

<oai-mem-citation>
<citation_entries>
MEMORY.md:201-203|note=[transport attribution and documentary host boundary]
</citation_entries>
<rollout_ids>
01a10b44-3030-7da3-89f3-0d664e3bbf8d
</rollout_ids>
</oai-mem-citation>
