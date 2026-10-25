Avancée : le lot 2 est accepté sur le parcours nominal avec réserves (commit local `5c8d6d9f`) ; les états du direct et de l'import final existent dans le cœur.
Blocage : l'interface n'expose pas encore clairement ces états et la vue actuelle ne conserve qu'un statut d'import final à la fois.
Décision : lot 3 — cadrage proposé, en attente de validation ; aucune implémentation autorisée par ce document.

# Lot 3 — Présentation de la transcription progressive Parakeet

## Objectif et périmètre

Rendre compréhensible, dans la fenêtre Windows existante, ce qui est capturé, ce que l'algorithme a confirmé pendant la parole, ce qui reste non résolu et ce que l'import du WAV fournit ensuite comme **brouillon non vérifié**. Garder le parcours micro Turbo, l'import de fichiers, les choix CPU/GPU/Auto et la sélection du microphone utilisables. Le lot 3 présente les états existants ; il ne change ni le moteur Parakeet, ni la confirmation R1/R2 et le contrat B, ni les exports, ni la concurrence des captures.

Le mot « confirmé » signifie **stable selon les preuves de l'algorithme en direct**, pas « vérifié par une personne » ni « lexicalement exact ». Le brouillon final automatique peut être plus complet mais n'acquiert jamais ce statut de confirmation. Le WAV, les TXT/SRT du direct, les TXT/SRT `-auto-non-verifie` et le JSON compagnon gardent leurs identités et rôles actuels.

## Fondement dans le candidat du lot 2

- `ports.rs` expose `ParakeetHypothesis`, `ParakeetConfirmed`, `ParakeetUnresolved`, `ParakeetResolved`, `ParakeetCaptureStopped` et les quatre états `Pending`, `Running`, `ProducedUnverified`, `FailedOrUnavailable` de la transcription finale. Aucune progression chiffrée fiable de cet import n'est exposée à la vue.
- `application.rs` tient le provisoire, le texte confirmé, les plages non résolues et une `AutomaticFinalTranscriptionView` avec chemins TXT/SRT, calcul demandé, backend éventuellement connu et diagnostic. L'événement d'arrêt associe le WAV et les sorties directes à la capture. La vue d'import final est unique : le résultat tardif de A ne doit pas être présenté comme celui de B.
- `ui.rs` affiche aujourd'hui un provisoire et un confirmé dans la zone du job actif, ainsi que des messages génériques. Il n'affiche pas encore les quatre états de l'import final ni une liste de captures récentes par identité. Le JSON compagnon version 1 conserve l'identifiant, le nom du WAV, les bornes confirmées et non résolues en échantillons, et le statut de l'import final.

## Parcours, états et transitions visibles

| Situation | Déclencheur observable | Présentation demandée |
| --- | --- | --- |
| Prêt | Aucun micro actif | Modèle, calcul demandé et microphone sélectionnés ; action « Démarrer le micro ». Aucun texte de capture précédente présenté comme courant. |
| Démarrage | Demande acceptée, avant capture active | « Démarrage du microphone… ». Ne pas annoncer de WAV conservé avant sa finalisation effective. |
| Capture active | Premiers échantillons et événements Parakeet | Capture A au premier plan ; texte provisoire remplaçable, segments confirmés en lecture stable, indicateur discret des portions non résolues. Le provisoire peut évoluer même si le confirmé stagne. |
| Arrêt et finalisation | Clic sur « Arrêter le micro », avant `ParakeetCaptureStopped` | « Arrêt en cours — finalisation du WAV et des résultats directs ». Ne pas annoncer la transcription finale comme terminée. |
| WAV finalisé, import en attente | `ParakeetCaptureStopped`, puis `Pending` | Afficher le chemin du WAV, les sorties directes disponibles et « Brouillon final en attente ». Une capture incomplète reste signalée comme telle. |
| Import final en cours | `Running` | « Brouillon final en cours — non vérifié ». Pas de pourcentage, de temps restant ou de promesse de complétude. La capture suivante reste pilotable. |
| Import final réussi | `ProducedUnverified` | « Brouillon final disponible — à relire », avec ses TXT/SRT distincts. Les confirmations et lacunes du direct restent accessibles et inchangées. |
| Import final échoué ou indisponible | `FailedOrUnavailable` et diagnostic | « Brouillon final indisponible » avec cause intelligible ; afficher séparément « WAV conservé » et les sorties directes effectivement disponibles. Ne pas présenter de TXT/SRT automatique partiel comme terminé. |
| Capture B pendant l'import A | B démarre alors que A est `Pending` ou `Running` | B occupe la zone de capture principale ; A reste dans une ligne de suivi identifiée par son nom de WAV et son état. L'achèvement tardif de A met à jour A seulement, sans remplacer le provisoire, les confirmations ou les chemins de B. |

Une erreur de capture avant finalisation du WAV doit avoir son propre message : elle ne doit pas être confondue avec un échec du seul import final. Un statut de calcul effectif n'est affiché que si l'événement du worker l'établit ; sinon « calcul demandé : GPU/CPU/Auto ; calcul effectif non renseigné ».

## Organisation proposée de la fenêtre

Conserver la fenêtre et ses commandes actuelles, avec une seule zone de lecture principale et un suivi compact des captures récentes :

1. **En tête :** modèle, calcul demandé, microphone, démarrage/arrêt, puis état court de la capture active. Les compteurs techniques existants peuvent rester secondaires ; ils ne remplacent pas l'état lisible.
2. **Pendant la capture :** bloc « Confirmé en direct » stable et chronologique ; juste après, zone « Provisoire — peut changer » pour l'hypothèse courante. Ne pas répéter le même texte dans les deux zones. Une mention « N portions à confirmer » ouvre, si besoin, leurs plages audio ; aucune lacune n'est insérée comme faux mot dans la phrase.
3. **Après l'arrêt :** une fiche compacte par capture, identifiée par son nom de WAV, affiche séparément l'état du WAV, du direct et du brouillon final. Un seul panneau de lecture sélectionne « Confirmé en direct » ou « Brouillon final — non vérifié » ; le changement est explicite et ne fusionne pas les textes.
4. **Fichiers :** la fiche donne les noms et chemins réellement disponibles : WAV, TXT/SRT directs, JSON compagnon et, après succès seulement, TXT/SRT automatiques. Un chemin non produit n'est pas rendu comme lien disponible. Le dossier et l'identifiant de capture restent visibles pour retrouver A et B sans confusion.

Utiliser les intitulés et le statut textuel, pas la couleur seule. Garder les détails des lacunes et les diagnostics techniques repliés par défaut pour ne pas encombrer la lecture. Aucun nouvel écran autonome ni double transcription juxtaposée en permanence n'est proposé.

## Libellés courts proposés

| État | Libellé principal | Précision si nécessaire |
| --- | --- | --- |
| Provisoire | « Texte provisoire — peut changer » | « Non exporté comme confirmé » |
| Confirmé | « Confirmé en direct » | « Stable selon l'algorithme ; non relu » |
| Lacune | « N portions restent à confirmer » | Bornes audio consultables ; aucun texte supposé. |
| WAV finalisé | « Audio enregistré » | Nom du WAV ; direct complet ou incomplet indiqué séparément. |
| Final en attente | « Brouillon final en attente » | « Vous pouvez démarrer une autre capture. » |
| Final en cours | « Brouillon final en cours » | « Texte automatique non vérifié. » |
| Final disponible | « Brouillon final disponible — à relire » | TXT/SRT `-auto-non-verifie` distincts du direct. |
| Final indisponible | « Brouillon final indisponible » | Cause courte ; « WAV et résultats directs conservés » seulement si leurs chemins sont établis. |

## Critères d'acceptation observables

1. En capture Parakeet, le provisoire change sans altérer les segments déjà affichés comme confirmés ; une lacune ne fait pas disparaître le provisoire ni inventer un mot à l'écran. Les libellés restent lisibles sans dépendre uniquement d'une couleur.
2. À l'arrêt, l'interface distingue l'arrêt demandé, la finalisation effective du WAV, puis l'attente et le traitement de l'import final. Elle ne montre ni pourcentage ni fin d'import avant un état runtime correspondant.
3. Après succès, le panneau « Brouillon final — non vérifié » et ses fichiers portent clairement cette qualification ; le panneau « Confirmé en direct », les TXT/SRT directs et le JSON des lacunes restent accessibles sans mélange ni remplacement.
4. Si l'import final échoue, l'état d'échec et sa cause sont visibles pour la bonne capture ; le WAV et les sorties directes disponibles restent repérables, sans faux fichier final déclaré terminé.
5. Quand B démarre pendant l'import A, B reste au premier plan et contrôlable ; A conserve son état et ses fichiers dans sa propre fiche. La fin tardive de A ne modifie ni texte, ni statut, ni chemins de B.
6. Les chemins affichés correspondent aux fichiers effectivement produits pour la capture sélectionnée ; les statuts « incomplet » du direct et « non vérifié » de l'automatique sont distincts. Les parcours micro Turbo et import de fichiers ne gagnent pas de nouveaux libellés Parakeet trompeurs.
7. La recette dans la fenêtre Windows vérifie ces transitions sur le parcours nominal. Elle ne ferme pas R-AUDIO, R-IMPORT, l'arrêt brutal ni la réserve de traçabilité du backend effectif sans preuve distincte.

## Impacts techniques prévisibles, sans implémentation

`crates/whisper-desktop/src/ui.rs` porterait les zones et libellés. `crates/whisper-core/src/ports.rs` et `application.rs` devraient exposer à l'interface des états **par capture**, au lieu de la seule `AutomaticFinalTranscriptionView` courante, pour suivre A pendant B et associer chaque état à son WAV et à ses chemins. Réutiliser les événements et le JSON existants ; aucune modification des règles de confirmation, du moteur ou du format des sorties n'est prévue. Une liste durable après redémarrage et un raccourci ouvrant l'Explorateur ne sont pas présumés : ils requièrent un choix de périmètre distinct.

## Arbitrages produit à valider

1. **Lecture après succès :** conserver par défaut le panneau « Confirmé en direct » et proposer un basculement explicite vers le brouillon (recommandé), ou ouvrir automatiquement le brouillon avec un bandeau « non vérifié ». Le premier évite qu'un texte non vérifié remplace visuellement des confirmations sans geste de l'utilisateur.
2. **Lacunes :** afficher un compteur et des bornes consultables à la demande (recommandé), ou des marqueurs insérés dans le flux de lecture. Le compteur garde la lecture fluide sans présenter les silences ordinaires comme des omissions.
3. **Historique visible :** suivre A et B dans la session courante, puis retrouver les anciens fichiers par leur dossier et nom de capture (recommandé pour ce lot), ou ajouter une liste durable après redémarrage. La seconde option demanderait un périmètre de lecture et de synchronisation supplémentaire.
4. **Accès aux fichiers :** afficher les chemins avec une action « Copier le chemin » (recommandé), ou ajouter « Ouvrir le dossier » via Windows. Le premier reste dans le périmètre de la fenêtre actuelle ; l'autre suppose une intégration système à autoriser.

**Statut : LOT 3 — CADRAGE PROPOSÉ, EN ATTENTE DE VALIDATION.** Aucun GO d'implémentation ni réouverture automatique des quatre réserves du lot 2.
