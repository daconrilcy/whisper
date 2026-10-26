Avancée : cadrage complet intégré ; correction de l'équité d'ordonnancement soumise à re-revue.
Blocage : DESIGN-PU-R1 reste ouvert jusqu'à vérification par le reviewer indépendant.
Décision : texte unique révisable pendant la capture, puis résultat final automatique ; aucune implémentation autorisée.

# Parakeet — transcription unifiée au micro et à l'import

## 1. Identité, autorisation et sources

- ID : D-PARA-UNIFIED-01 ; version de travail : v0.2 ; statut : DRAFT — re-revue en attente.
- Date : 26 octobre 2026. Dépôt : `C:\dev\whisper` ; branche : `evo-startup-perf`.
- Base produit inspectée : `a483e176759fcd2f2340f221470fe09f20ec8545`.
- Auteur documentaire autorisé : session Codex principale ; spécialistes en lecture seule, contributions attribuées ci-dessous.
- Mission courante de l'utilisateur : « ok fais le cadrage avec les agents dediés ».
- Autorisé : inspection de l'existant, cadrage avec spécialistes, rédaction et commits locaux des documents dans `docs/cadrage/`, revue indépendante. Deux fichiers prévus : ce cadrage et `revue-parakeet-transcription-unifiee.md`.
- Non autorisé : implémentation, campagne d'essais, compilation, exécution audio/GPU/replay, installation, changement de moteur/modèle, écriture hors dépôt, push. Aucun ancien mandat FEAS ou de compilation n'est réutilisé.
- Plan de travail documentaire : brief produit → exigences → domaine → architecture → revue et corrections (deux cycles maximum). Aucun plan d'implémentation exécutable n'est commandé dans cette mission.
- Contrôles avant ouverture : 115,30 Gio libres sur C: ; fichiers suivis propres ; `models/` et `sound example/` non suivis, préservés. Aucune commande Cargo prévue.

### Sources d'autorité

**S-USER-01 — Demande originale, session du 26 octobre 2026 :**

> le process de capture avec Parakeet est compliqué et imparfait avec des mots qui ont besoin d'être confirmé ou qui reste en draft et qui donne des transcripts incomplets.
> Fais une analyse approfondie du processus pour le rendre fluide sans ces étapes intermédiaires.
> On doit pouvoir importer un audio ou utiliser le micro de maniere transparente et recuperer les transcription sans soucier de toutes ces etapes en tant qu'utlisateur.

**S-USER-02 — Choix explicite pendant la qualification :**

> Un seul texte visible qui se corrige automatiquement pendant la capture, puis laisse place au résultat final (recommandé).

Question posée : « Pendant l'enregistrement micro, quel affichage souhaitez-vous ? Dans les deux cas, la transcription se termine automatiquement après “Arrêter”, sans confirmation de mots à gérer. »

**S-USER-03 — Nouvelle mission autorisée :**

> ok fais le cadrage avec les agents dediés

**S-USER-04 — Durées longues incluses, réponse à Q-PU-01 pendant le cadrage :**

> Inclure les audios et enregistrements de plus de 30 minutes ; cadrer aussi leur découpage et leur finalisation automatique.

Question : faut-il inclure les durées supérieures à 30 minutes, ou garder la limite actuelle avec annonce et arrêt micro propre ? L'utilisateur retient l'inclusion des durées longues. Cela autorise leur conception, pas une campagne, une implémentation ni une durée illimitée garantie. Les limites techniques justifiées restent à établir par le cadrage.

**S-USER-05 — Absence de plafond de durée, précision de Q-PU-02 :**

> sans limite.

Question : « Quelle durée doit impérativement être prise en charge pour un fichier ou une session micro longue dans votre usage — par exemple 1 heure, 2 heures, 4 heures, ou davantage ? Cette cible servira à dimensionner le traitement et sa validation. »

Interprétation annoncée dans cette session : absence de plafond de durée imposé par l'application. Les limites physiques de disque, ressources et formats subsistent ; le cadrage doit définir leur détection et une fin propre préservant l'audio. Aucun essai fini ne prouve une durée infinie. Ne pas remplacer ce choix par un plafond arbitraire.

**S-USER-06 — Fichiers audio des très longues captures :**

> Plusieurs fichiers WAV classiques numérotés automatiquement, plus largement compatibles avec les lecteurs existants.

Question : un seul fichier `.wav` passant à RF64 si nécessaire, avec compatibilité externe dépendant du lecteur, ou plusieurs WAV classiques numérotés automatiquement ? Dans les deux cas, transcription unique et découpage automatique. L'utilisateur choisit plusieurs WAV classiques. Cette décision remplace, pour le nouveau parcours Parakeet long uniquement, le contrat d'un WAV physique unique ; la session logique et sa transcription TXT/SRT restent uniques, les fichiers audio forment une même famille de nom. Turbo et les captures existantes ne sont pas modifiés par cette décision de cadrage. Aucun RF64 ajouté à cette évolution.

**S-QUAL-01 — Qualification dans cette conversation :** revue indépendante `revue_qualification`, verdict PASS, aucun P0/P1. Le résultat final reste automatique et ne vaut pas vérification humaine. Aucun fichier ni code créé sous cette mission de qualification.

### Besoin et critères issus de la qualification

| ID | Résultat attendu |
| --- | --- |
| CA-01 | Micro : texte unique révisable, puis résultat final automatique ; aucune confirmation de mots ou sélection confirmé/brouillon. |
| CA-02 | Tout l'audio conservé dans le périmètre admis est pris en compte par le traitement final ; l'absence de double confirmation en direct n'exclut pas un mot du résultat final ; toute portion non traitée est signalée. |
| CA-03 | Import WAV/MP3 : progression, résultat et sorties cohérents avec le micro. |
| CA-04 | Résultat affiché et TXT/SRT associés à la même capture et transcription ; sortie absente ou temps invalide signalé. |
| CA-05 | Incidents : ressources récupérables conservées, capture/transcription incomplètes distinguées, action utile sans inspection des fichiers internes. |
| CA-06 | Une finalisation de A pendant B ne modifie que A. |
| CA-07 | Formats et durées admis explicites ; erreurs lexicales, omissions de traitement, doublons et temps invalides évalués séparément. |
| CA-08 | Préserver CPU/GPU/Auto, sélection/gain micro et Turbo ; distinguer délai du premier texte, retard en direct et attente après arrêt. |

### Baseline et limites constatées pendant la qualification

- `docs/cadrage/evo-parakeet-micro-progressif.md` et `evo-parakeet-micro-progressif-lot3.md` : ancien contrat provisoire/confirmé immuable et exports directs incomplets, puis résultat automatique distinct.
- Code : `parakeet_live.rs`, `parakeet_confirmation.rs`, `worker_ipc.rs`, `application.rs`, `ports.rs`, `ui.rs`, `native_engine.rs`, `model_registry.rs`, `parakeet_windows.rs` ; les spécialistes vérifieront les références utiles sur la base produit ci-dessus.
- Recette lot 3 existante : A 43,13 s, 15 intervalles confirmés/136 non résolus ; B 37,88 s, 12/55 ; import final `produced_unverified`. Ces comptes ne mesurent ni l'exactitude lexicale ni une continuité physique sans perte.
- Réserves non fermées : R-AUDIO, R-IMPORT, arrêt brutal/orphelins, backend effectif des métadonnées. Les fenêtres concurrentes A/B ne sont pas une preuve de fin tardive A pendant B.
- Import produit : admission d'une passe jusqu'à 30 minutes selon le code lu en qualification ; présence d'un planificateur long ne prouve pas son admission effective.
- Les 30 secondes de la boucle de finalisation directe ne bornent pas l'attente totale après arrêt.
- `docs/predev/` et FEAS-PM-01 : archives et diagnostics, sans autorisation nouvelle ; G1 reste arrêté.

## 2. Carte des contenus et contributions

| Section | Propriétaire du contenu | État |
| --- | --- | --- |
| 3 — Brief, capacités et décisions produit | rust_product_framer | PRODUCT-PU v1 intégré |
| 4 — Exigences, cas d'usage, acceptation et risques fonctionnels | rust_requirements_analyst | REQUIREMENTS-PU v2 intégré |
| 5 — Domaine, états et invariants | rust_domain_architect | DOMAIN-PU v2 intégré |
| 6 — Architecture, contrats, dépendances et preuves | rust_architect | ARCH-PU v2 intégré |
| 7 — Questions et traçabilité transversale | sources utilisateur et contributions attribuées | Décisions versionnées, couverture sections 4–6 |
| 8 — Readiness et suite | session principale, sur verdict du reviewer | PENDING REVIEW |
| Compte rendu de revue séparé | rust_design_reviewer | FINDINGS initial ; re-revue en attente |

## 3. Brief produit

Contribution `PRODUCT-PU v1`, auteur `/root/parakeet_framer` (`rust_product_framer`). Base déclarée : document `c6fc20c6`, produit `a483e176`, complétée par S-USER-04 transmise par le coordinateur puis versionnée en `25e9be57`. Sources : document courant, AGENTS, règles métier centrales, cadrages micro/Parakeet et annulation applicative. Proposition DRAFT ; aucun spécialiste n'a écrit de fichier. Terminé : objectif, périmètre, parcours et questions. Restant : exigences, domaine, architecture, revue et acceptation du design.

### 3.1 Objectif et utilisateurs

Dans l'application Windows Whisper existante, l'utilisateur transcrit avec Parakeet un fichier WAV/MP3 ou une prise microphone, puis récupère le résultat sans gérer les étapes internes de reconnaissance.

La version cadrée remplace le parcours Parakeet qui expose séparément provisoire, fragments confirmés, lacunes et brouillon final. Elle présente **une transcription principale unique**. Pendant la capture, ce texte peut être automatiquement corrigé ; après l'arrêt, le résultat final prend sa place sans sélection de catégorie ni confirmation de mots. L'import suit les mêmes notions de traitement en cours, résultat disponible et incident.

L'utilisateur a explicitement inclus les **fichiers et enregistrements dépassant 30 minutes**, avec découpage et finalisation automatiques. Ce choix élargit le cadrage ; il ne qualifie pas le traitement long, ne fixe pas de durée maximale et n'autorise aucun essai.

Le résultat reste automatique. « Terminé » signifie que le traitement et les sorties prévus ont abouti ; ce libellé ne certifie ni exactitude lexicale ni vérification humaine.

### 3.2 Inventaire des capacités

Les statuts distinguent **objectif accepté**, **non-régression à conserver** et **proposition à instruire**.

| ID | Capacité / parcours | Source et statut | Périmètre / critères |
| --- | --- | --- | --- |
| CAP-PU-01 | Démarrer le micro, lire un texte unique révisable, arrêter | S-USER-01/02 ; objectif accepté | Inclus ; CA-01/08 |
| CAP-PU-02 | Finaliser automatiquement toute la prise admise et présenter le résultat principal | S-USER-01/02 ; objectif accepté | Inclus ; CA-01/02/04 |
| CAP-PU-03 | Importer WAV/MP3 et récupérer transcription et sorties sans confirmation de mots | S-USER-01 ; objectif accepté | Inclus ; CA-03/04 |
| CAP-PU-04 | Traiter les fichiers et captures >30 min, découpage et raccords automatiques | S-USER-04 ; périmètre accepté, faisabilité ouverte | Inclus ; CA-02/03/07/08 |
| CAP-PU-05 | Associer texte affiché, TXT/SRT et audio à la même capture et au même résultat | Qualification ; résultat attendu à spécifier | Inclus ; CA-04/06 |
| CAP-PU-06 | Préserver l'audio récupérable et distinguer défaut de capture, échec de transcription et export partiel | Qualification et réserves existantes ; garanties à spécifier | Inclus ; CA-02/05 |
| CAP-PU-07 | Suivre A pendant l'enregistrement de B sans attribution croisée | Qualification, lot 3 ; non-régression à vérifier | Inclus ; CA-06 |
| CAP-PU-08 | Préserver CPU/GPU/Auto, micro sélectionné, gain et parcours Turbo | AGENTS et qualification ; non-régression | Inclus ; CA-08 |
| CAP-PU-09 | Conserver les protections des transcriptions existantes lors d'un remplacement | Parcours actuel ; non-régression | Inclus ; CA-03/04/05 |
| CAP-PU-10 | Gérer attente, annulation et fermeture sans annoncer une réussite fictive | Contrats existants ; non-régression à vérifier | Inclus ; CA-05/08 |

Les réglages existants restent utilisables ; **de nouveaux réglages** n'entrent pas dans ce périmètre. Installation, mise à jour, démarrage avec Windows, raccourcis, tray, bibliothèque durable de sessions, nouveaux moteurs/modèles et sortie MP3 restent différés ou hors évolution selon `AGENTS.md`. Leur reprise exige une mission distincte. La reprise automatique après panne reste différée ; conserver une ressource récupérable après erreur normale ne l'autorise pas.

### 3.3 Parcours cibles

**Micro nominal.** L'utilisateur conserve ses choix de moteur, calcul et source, puis démarre. L'application annonce la capture selon son état réel et affiche une transcription unique évolutive. « Arrêter » termine la capture puis déclenche la finalisation automatique. Le résultat final et ses sorties deviennent accessibles dans le même parcours. Aucune réimportation manuelle du WAV n'est requise.

**Import nominal.** L'utilisateur choisit le fichier et lance. L'application vérifie son admission, traite l'audio, réalise automatiquement le découpage nécessaire dans les limites établies et présente le résultat. Les protections de remplacement concernent les fichiers existants, jamais la validation des mots.

**Séance longue.** L'utilisateur n'a ni extraits à préparer ni raccords à effectuer. Le cadrage doit établir les limites et comportements d'admission, de découpage, de conservation et de finalisation. Aucune durée illimitée n'est promise.

**Incident.** L'application explique ce qui a échoué, ce qui est conservé et l'action possible. Un texte partiel ne devient pas silencieusement un résultat complet. Une erreur lexicale dans de l'audio traité reste distincte d'une portion jamais traitée.

**Attente et concurrence.** L'avancement reste compréhensible et la capture suivante, lorsqu'elle est admise, garde son identité. Un résultat tardif de A met à jour A seulement. Ne pas inventer de pourcentage ou de temps restant.

**Annulation et fermeture.** Conserver comme références de non-régression les contrats existants : fermeture normale du micro équivalente à arrêt/finalisation ; annulation d'import avant engagement, résultat réel attendu après engagement ; rejet des résultats périmés. Leur application au nouveau traitement final reste à expliciter, sans promettre de fermeture instantanée. Sources : `evo-micro-direct.md:21/47`, `evo-parakeet.md:325/328` ; le comportement produit reste à vérifier.

### 3.4 Questions et décisions

- **Q-PU-01 — STRUCTURING, CLOSED :** durée >30 min incluse par S-USER-04. Propriétaire : utilisateur ; impacts CAP-04 et CA-02/03/07/08. Ne pas rouvrir ce choix.
- **Q-PU-02 — STRUCTURING, CLOSED sur le besoin :** S-USER-05 retient l'absence de plafond applicatif de durée. La faisabilité, les contraintes de format/disque et la gestion bornée des ressources restent à instruire par l'architecte ; aucune durée infinie ne peut être garantie sur une machine finie. Ce choix remplace la recherche d'une durée maximale produit.
- **Q-PU-03 — DETAIL, OPEN :** seuils acceptables de premier texte, retard et attente après arrêt. Analyste et architecte définissent les métriques et proposent la méthode ; l'utilisateur accepte les seuils avant la validation qui en dépend. Si un seuil conditionne l'architecture, reclasser STRUCTURING. Les anciennes cibles ne doivent pas être transposées silencieusement du « confirmé » au nouveau texte révisable.

**Orientation proposée :** réutiliser l'existant lorsque compatible, privilégier la conservation de l'audio et une finalisation explicite, et garder toutes les confirmations algorithmiques éventuelles hors des gestes demandés à l'utilisateur. Ce sont des objectifs de cadrage ; le choix des mécanismes appartient aux spécialistes suivants.

## 4. Exigences et acceptation

Contribution `REQUIREMENTS-PU v2`, auteur `/root/parakeet_requirements` (`rust_requirements_analyst`), remplace v1 avant handoff. Base : `db220d8b`, produit `a483e176` ; sources S-USER-01 à 05, CA-01 à 08 et PRODUCT-PU v1. Sources complémentaires : `evo-micro-direct.md:17–22`, `evo-parakeet.md:325–328`, `evo-parakeet-micro-progressif.md:49–54`, bilan lot 3 `:95–99`, `ports.rs:246–300`, `application.rs:475–496`, `ui.rs:1676–1723`. Statut DRAFT ; aucun fichier ni test exécuté par le spécialiste. Terminé : exigences, scénarios, états visibles, risques et protocole. Restant : domaine, architecture, faisabilité des ressources, Q-PU-03, revue et acceptation.

### 4.1 Exigences et couverture

Exigences **proposées** à partir des objectifs acceptés et non-régressions. Chaque ligne renvoie aux UC/AC de même numéro en 4.2.

| ID | Exigence | CAP / source / CA initial |
| --- | --- | --- |
| REQ-PU-01 | Pendant la capture Parakeet, présenter un texte unique automatiquement révisable, sans catégories confirmé/brouillon ni validation de mots. | CAP-01 ; USER-01/02 ; CA-01 |
| REQ-PU-02 | Après arrêt normal, finaliser automatiquement tout l'audio admis ; remplacer le texte en cours par le résultat final. L'absence de confirmation directe n'est pas un motif d'omission finale. | CAP-02 ; USER-01/02 ; CA-01/02 |
| REQ-PU-03 | Importer WAV/MP3 sans préparation d'extraits ou confirmation de mots ; préserver la source et harmoniser progression, résultat et erreurs avec le micro. | CAP-03 ; USER-01 ; CA-03 |
| REQ-PU-04 | Ne fixer aucun plafond applicatif de durée pour fichiers et micro ; effectuer découpage, raccords et temps globaux automatiquement. La durée ne doit pas imposer le chargement intégral de l'audio ou une croissance non bornée des ressources de travail. Limites physiques de disque, ressources et formats : détecter, terminer proprement et signaler les ressources préservées. Les mécanismes relèvent de l'architecture. | CAP-04 ; USER-04/05 ; CA-02/03/07/08 |
| REQ-PU-05 | Associer résultat visible, TXT/SRT et audio à la même opération ; signaler une sortie absente ou invalide. Un résultat complet exige les sorties prévues disponibles et la couverture complète du traitement. | CAP-05 ; qualification ; CA-04 |
| REQ-PU-06 | Distinguer incident de capture, portion non traitée et échec d'export ; préserver les ressources récupérables et proposer une action utile. Aucune lacune masquée par un faux mot ou succès complet. | CAP-06 ; qualification ; CA-02/05 |
| REQ-PU-07 | Corréler résultats et états à leur capture : une fin tardive de A ne modifie ni texte, ni audio, ni chemins de B. | CAP-07 ; qualification ; CA-06 |
| REQ-PU-08 | Préserver CPU/GPU/Auto et les règles de repli existantes, source/gain micro et Turbo ; aucune substitution de modèle ou micro introduite. | CAP-08 ; AGENTS ; CA-08 |
| REQ-PU-09 | Préserver les fichiers existants tant qu'un remplacement n'est pas explicitement autorisé ; cette protection reste distincte des confirmations de mots. | CAP-09 ; parcours existant ; CA-03/04/05 |
| REQ-PU-10 | Rendre attente, arrêt, annulation et fermeture observables ; la fermeture normale attend l'issue réelle des finalisations acceptées. Aucun faux « annulé » après engagement ni terminaison fondée seulement sur l'âge. | CAP-10 ; contrats existants ; CA-05/08 |

### 4.2 Cas d'usage et acceptation

**Validation future NOT RUN.** Fixtures et injections à préparer dans un lot autorisé ; aucune expérience finie ne prouve une durée infinie.

| UC / REQ | Scénario, données et action | AC et méthode de constat |
| --- | --- | --- |
| UC-PU-01 / REQ-01 | Enregistrer des extraits FR/EN avec pauses, hésitation et parole continue. | **AC-PU-01 :** un texte principal unique évolue avant arrêt, sans validation de mots ni sélection de version. Observer dans la fenêtre et corréler les mises à jour à la capture. |
| UC-PU-02 / REQ-02 | Prononcer une phrase terminale puis arrêter ; inclure un passage non stabilisé en direct. | **AC-PU-02 :** résultat final automatique ; tous les intervalles admis ont été soumis au traitement prévu ou un défaut explicite est présent. Vérifier couverture, dernière phrase et sorties ; couverture n'équivaut pas à exactitude lexicale. |
| UC-PU-03 / REQ-03 | Importer WAV puis MP3 ; essayer aussi une source illisible/non supportée. | **AC-PU-03 :** résultat et sorties attendues au nominal ; refus motivé sinon ; source conservée, aucune préparation manuelle. Recette Windows et comparaison de l'état source. |
| UC-PU-04 / REQ-04 | Fichiers puis captures de durées croissantes, dont plusieurs >30 min, avec parole et silences repérés aux raccords et en fin ; exercer séparément les limites de ressources/format. | **AC-PU-04 :** aucun refus/arrêt fondé sur un plafond en minutes/heures ; aucune portion sautée ou duplication introduite par l'assemblage ; temps globaux cohérents. Inspection structurelle du traitement et mesures de mémoire/chargements, plus contrôle des intervalles et écoute des raccords. Aux limites physiques : issue explicite et ressources récupérables préservées. Ces essais finis ne garantissent pas l'infini. |
| UC-PU-05 / REQ-05 | Produire un résultat puis rendre TXT ou SRT impossible/invalide. | **AC-PU-05 :** écran et fichiers valides correspondent à la même opération ; sortie défectueuse identifiée ; aucun succès complet pour un paquet incomplet. Comparer identités, contenu, chemins et temps SRT ; aucun temps inventé. |
| UC-PU-06 / REQ-06 | Exercices distincts : panne moteur, saturation capture, écriture refusée, erreur de finalisation. | **AC-PU-06 :** cause, frontière audio connue, ressources et action possible visibles ; aucun trou silencieux. Vérifier événements/fichiers, puis relance explicite depuis source conservée lorsque possible ; aucune reprise après panne générale déduite. |
| UC-PU-07 / REQ-07 | Faire terminer A pendant B ; livrer aussi un résultat périmé. | **AC-PU-07 :** seule A se met à jour ; B garde texte/configuration/chemins ; périmé ignoré. Contrôle déterministe des identités puis recette avec chevauchement réellement observé. |
| UC-PU-08 / REQ-08 | CPU, GPU manuel, Auto ; micro explicite/défaut, gain non nul ; Turbo témoin. | **AC-PU-08 :** choix/règles existants conservés, moteur effectif identifiable lorsqu'il est prouvé, aucun repli ou changement de source interdit. Vérifier configuration, événements et sorties ; branches natives non exercées restent NOT RUN. |
| UC-PU-09 / REQ-09 | Réimporter une source déjà transcrite ; refuser puis autoriser le remplacement. | **AC-PU-09 :** refus préserve les sorties ; accord ne remplace que le résultat désigné ; conflit/échec explicite. Comparer fichiers et identité de publication. |
| UC-PU-10 / REQ-10 | Annuler avant/après engagement ; fermer pendant capture ou finalisation A/B ; répéter Arrêter. | **AC-PU-10 :** demande et issue distinguées, arrêt idempotent, fermeture attendant l'issue réelle des finalisations acceptées ; aucun faux succès/annulation. Contrôler transitions et fenêtre ; aucune fermeture instantanée promise. |

### 4.3 États visibles

Présentation proposée : **prêt**, **préparation**, **capture**, **transcription**, **finalisation**, **résultat disponible**, **annulation demandée**, **fermeture en attente**, **incident avec ressources conservées**. Le domaine définit ses états séparément.

L'attente montre activité et dernier progrès observés, sans pourcentage ni temps restant inventé. Un silence du moteur ne prouve pas son blocage. La stagnation n'impose pas d'arrêt par l'âge seul.

En fermeture normale, capture arrêtée et finalisation achevée restent distinctes. L'UI observée fonde principalement `busy` sur le job actif (`ui.rs:1676–1723`) : attendre toutes les finalisations n'est **pas une garantie actuellement démontrée**.

### 4.4 Mesures et questions

Sur les ressources Windows CPU/GPU retenues, utiliser extraits FR/EN courts, raccords annotés et durées longues croissantes :

- **Premier texte utile :** premier échantillon admis → premier texte non vide présenté ; préparation moteur et début de capture relevés séparément.
- **Retard :** frontière audio admise moins dernière frontière source effectivement traitée ; ne pas l'inférer d'un mot mal horodaté.
- **Après arrêt :** clic → capture drainée, puis → résultat et sorties disponibles.
- **Absence de progrès :** dernier événement, durée silencieuse et activité réelle du worker ; aucun seuil accepté ici.
- **Interruption normale :** comparer plages reçues/admises, WAV final et traitement ; sous contention, examiner les pertes périphérique. Un WAV cohérent ne prouve pas seul la continuité physique.
- **Longue durée :** inspecter allocations/lectures et suivre mémoire, temporaires et stockage ; vérifier que le volume total n'impose pas de chargement intégral. Architecture responsable des bornes techniques.
- **Qualité :** référence humaine distinguant erreurs ASR, portions jamais traitées, doublons de raccord et temps invalides.

**Q-PU-02 — CLOSED sur le besoin :** S-USER-05 répond « sans limite. » Aucun plafond applicatif de durée. Faisabilité et gestion des limites physiques : architecte, avant readiness de l'architecture ; aucune nouvelle question de durée à l'utilisateur.

**Q-PU-03 — DETAIL, STRUCTURING si elle conditionne l'architecture :** seuils de premier texte, retard et attente ; analyste définit les métriques, architecte instruit les compromis, utilisateur tranche avant validation dépendante. Aucun ancien seuil du confirmé n'est transposé au texte révisable.

**Q-RQ-01 — DETAIL :** seuil d'alerte d'absence de progrès ; recommandation informer sans arrêt automatique. Owner analyste/UI ; autorité utilisateur si effet sur le parcours ; avant lot UI, REQ-10/AC-10.

### 4.5 Risques et exclusions

| ID | Risque / critères | Traitement |
| --- | --- | --- |
| RISK-F-PU-01 | Faux résultat complet avec audio non traité — AC-02/04/05. | Conditions de complétude et diagnostic. |
| RISK-F-PU-02 | Raccord perdu/doublé, temps dérivés ou mémoire croissante — AC-04/05. | Contrats bornés, inspection et contrôles aux raccords. |
| RISK-F-PU-03 | Contention/limite disque-format causant interruption silencieuse — AC-04/06/07/08. | Frontières d'admission, fin propre et incident. |
| RISK-F-PU-04 | Fermeture/annulation abandonnant une finalisation — AC-10. | Issue réelle et propriété des opérations. |
| RISK-F-PU-05 | Résultat tardif/remplacement altérant une autre capture — AC-07/09. | Identités et protection des publications. |

Aucun GAP CAP-PU-01 à 10. Différés de §3.2 inchangés : nouvelles fonctions d'installation, tray, bibliothèque durable et reprise après panne exigent une mission distincte ; aucune REQ ajoutée ici. Seuils ouverts et preuves futures ne sont pas présentés comme acceptés ou réussis.

## 5. Domaine

Contribution `DOMAIN-PU v2`, auteur `/root/parakeet_domain` (`rust_domain_architect`). v1 base `3848e128`, amendement v2 base `7108019b` ; produit `a483e176` ; PRODUCT-PU v1, REQUIREMENTS-PU v2, S-USER-01 à 06. Sources code : `domain.rs:1–113`, `ports.rs:229–300`, `application.rs:475–501` et `:2280–2310`. Proposition DRAFT ; aucune implémentation ni exécution. Terminé : concepts, états, transitions, invariants et couverture. Restant : choix techniques, contraintes de formats/ressources, contrats d'effets, revue et acceptation. v2 conserve v1 sauf DOM-PU-02, ajout de portée audio, INV-PU-05/08 et Q-DOM-01 remplacés par leur auteur après S-USER-06.

### 5.1 Concepts et responsabilités

| ID | Concept / propriétaire métier | Entrées, sorties et limites |
| --- | --- | --- |
| DOM-PU-01 | **Session audio** : identité stable, origine micro/import, configuration retenue et état de capture. | Relie l'opération à sa source ; reçoit arrêt/incidents ; produit une source close ou un diagnostic de capture. Aucun changement implicite de micro, modèle ou calcul. |
| DOM-PU-02 | **Source audio logique** : chronologie normalisée et références ordonnées aux données conservées. | Plages demi-ouvertes `[début, fin)` en échantillons, fréquence explicite et arithmétique contrôlée. Pour les longues captures Parakeet, plusieurs WAV classiques numérotés appartiennent à une même famille de nom ; chaque partie conserve son intervalle global et la correspondance avec ses positions locales. Réception, admission et écriture restent distinctes. La rotation ne crée ni nouvelle session ni nouvelle transcription. La taille des parties et leur stockage relèvent de l'architecture. L'import conserve sa source originale. |
| DOM-PU-03 | **Traitement** : tentative identifiée par session et génération. | Consomme des plages de la source ; produit couverture, résultats et erreurs. Une nouvelle tentative ne réutilise pas l'identité d'une tentative périmée. |
| DOM-PU-04 | **Couverture** : sort de chaque plage de traitement. | Distingue résultat accepté, absence de texte issue d'une analyse réussie, échec et non-traitement. Une absence de texte n'est pas une preuve de silence acoustique. |
| DOM-PU-05 | **Document de transcription** : révisions puis version finale. | Chaque révision désigne la session, la tentative et les portions remplacées. La vue principale présente une seule version courante. Le document final peut différer du texte vu pendant la capture. |
| DOM-PU-06 | **Publication** : version de résultat et sorties associées. | Relie document, audio et TXT/SRT attendus. Les sorties ont chacune un état valide, absent ou défectueux ; les temps invalides ne sont jamais fabriqués. |
| DOM-PU-07 | **Autorisation de remplacement** : résultat existant précisément désigné. | Le refus préserve la publication existante ; une autorisation ne s'étend pas à une autre session/version. |
| DOM-PU-08 | **Obligation de clôture** : opérations acceptées restant à terminer. | Inclut la finalisation découlant d'une capture, même lorsque cette capture n'est plus au premier plan. Sert à fermer normalement sans abandon silencieux. |

S-USER-06 remplace le contrat d'un WAV physique unique pour le nouveau parcours Parakeet long uniquement. Le document de transcription et ses exports TXT/SRT restent uniques à l'échelle de la session ; leurs temps sont globaux, sans remise à zéro aux changements de WAV. Aucun raccord manuel n'est demandé. Turbo et les captures existantes conservent leurs contrats.

Le domaine porte règles et transitions pures ; l'application possède leur orchestration et demande les effets. Les adaptateurs réalisent capture, stockage et reconnaissance ; l'interface projette les états. Le domaine ne dépend ni des chemins Windows ni d'un worker concret.

Les références aux plages, révisions et résultats cumulés doivent rester consultables sans imposer leur chargement intégral en mémoire. Le stockage peut croître avec l'audio et le texte ; les ressources de travail doivent être bornées par l'architecture.

### 5.2 Trois axes d'état distincts

**Métier :**

- Source : `Ouverte → Drainage → Close`, ou `Interrompue`.
- Traitement : `Attente → EnCours → Finalisation → Terminé`, ou `AnnulationDemandée → Annulé`, ou `Échec`.
- Publication : `NonEngagée → Préparation → Engagement → Disponible`, ou `Défectueuse`.

Ces axes appartiennent à la session/tentative, jamais au seul panneau actuellement sélectionné. Une source close peut encore avoir un traitement actif.

**Opérationnel durable :** audio réellement écrit, résultats intermédiaires conservés, version des sorties et issue d'engagement constatée. Ces faits proviennent des adaptateurs ; une demande d'écriture ne prouve pas son accomplissement. Leur conservation ne crée aucune garantie de reprise après panne.

**Présentation :** capture, transcription, finalisation, résultat disponible, incident ou fermeture en attente. Le texte révisable et final utilise la même référence de document. Les anciens statuts confirmé/brouillon ne constituent plus des choix utilisateur.

### 5.3 Transitions et effets

| ID / événement | Précondition et transition | Effets demandés / erreur |
| --- | --- | --- |
| TR-PU-01 — Démarrer | Admission et configuration établies ; création d'une session et d'une tentative. | Ouvrir source/traitement. Échec d'ouverture : incident motivé, sans prétendre capturer. |
| TR-PU-02 — Bloc reçu | Source ouverte, identité valide. | Distinguer reçu, admis, puis écrit sur accusé réel ; saturation ou écriture impossible interdit toute continuité fictive. |
| TR-PU-03 — Résultat reçu | Session, génération, plage et révision courantes. | Mettre à jour couverture et document ; résultat périmé ignoré. Duplicata identique sans nouvel effet ; même identité avec contenu contradictoire : incohérence explicite. |
| TR-PU-04 — Arrêter | Source ouverte. | Passer au drainage une seule fois ; fermer la frontière finale après réception/écriture des blocs encore admis. Répéter Arrêter est sans effet supplémentaire. |
| TR-PU-05 — Source close | Frontière finale connue et données écrites identifiées. | Finaliser automatiquement le traitement des plages restantes ; aucun passage oublié parce qu'il n'a pas été stabilisé en direct. |
| TR-PU-06 — Fin de traitement | Couverture finale vérifiée, résultats persistés. | Produire la version finale ; préparer les exports. Lacune, temps invalide ou export défectueux conserve les éléments utilisables et produit un diagnostic. |
| TR-PU-07 — Publier | Sorties attendues valides, identité cohérente, remplacement autorisé si requis. | Demander l'engagement puis attendre son issue réelle. La référence principale devient finale après confirmation de disponibilité ; aucune atomicité multi-fichiers présumée. |
| TR-PU-08 — Annuler | Avant engagement de publication. | Demander l'arrêt du traitement ; rester `AnnulationDemandée` jusqu'à confirmation réelle. Après engagement, conserver l'issue réelle plutôt que déclarer fictivement « annulé ». |
| TR-PU-09 — Fermer | Application accepte la demande de fermeture normale. | Refuser de nouvelles opérations, arrêter une capture ouverte, attendre les obligations de finalisation acceptées et l'issue des engagements. A puis B restent suivies séparément. |
| TR-PU-10 — Réessayer après erreur normale | Source conservée accessible et action explicite admise. | Nouvelle génération liée à la même source ; ancien résultat isolé. Aucun redémarrage automatique après panne générale. |

Sont interdits : `capture ouverte → succès final`, publication complète avec couverture lacunaire, résultat de A appliqué à B, écrasement sans autorisation et annonce d'annulation avant son issue. Une limite physique, un dépassement de représentation ou un calcul d'indice impossible devient un incident ; ce n'est pas un plafond de durée arbitraire.

### 5.4 Invariants

| ID | Invariant et traçabilité |
| --- | --- |
| INV-PU-01 | Toute commande, plage, révision et sortie porte session et génération ; aucune attribution fondée seulement sur le job visible. REQ/AC-01, 05, 07, 10. |
| INV-PU-02 | Les frontières reçu/admis/écrit/traité ne sont pas interchangeables ; la fin maximale d'une plage ne prouve pas la couverture de ce qui la précède. REQ/AC-02, 04, 06. |
| INV-PU-03 | Le succès de traitement exige que chaque portion de la source admise ait une issue finale acceptée ; aucun échec/non-traitement résiduel. Cela ne garantit pas l'exactitude lexicale. REQ/AC-02 à 06. |
| INV-PU-04 | Une perte de capture connue reste un incident même si tout l'audio effectivement conservé a été transcrit. L'absence de perte signalée ne prouve pas, seule, une continuité physique parfaite. REQ/AC-06, 08. |
| INV-PU-05 | Les parties WAV d'une capture continue forment une partition ordonnée de l'audio écrit : fin de la partie précédente égale début de la suivante, sans perte ni duplication de rotation. Toute discontinuité constatée reste un incident explicite. Le numéro de partie ne remplace pas l'identité de session ni les bornes globales. Les recouvrements de reconnaissance ne créent pas de doublons dans le document final ; chaque résultat conserve son origine globale. REQ/AC-02, 04, 05 ; S-USER-06. |
| INV-PU-06 | Un texte final utilisable peut subsister lors d'un défaut SRT ; le paquet demeure incomplet et l'interface l'indique. Aucun temps ou mot n'est inventé. REQ/AC-05, 06. |
| INV-PU-07 | Une publication existante reste la référence jusqu'à l'issue réelle d'un remplacement autorisé ; les fichiers partiels ne deviennent pas une nouvelle publication complète. REQ/AC-05, 09. |
| INV-PU-08 | Ni durée écoulée ni capacité atteinte d'un WAV ne justifient l'arrêt normal d'une longue capture : celle-ci poursuit sa source logique dans la partie suivante. Si les ressources ou une erreur empêchent cette poursuite, terminer proprement, préserver les parties déjà conservées et annoncer la frontière connue et l'incident. Aucune suppression silencieuse d'audio. REQ/AC-04, 06, 10 ; S-USER-05/06. |

### 5.5 Couverture et points à transmettre

| REQ/UC/AC | Concepts / invariants principaux |
| --- | --- |
| 01–02 | DOM-01 à 05 ; INV-01 à 05 |
| 03–04 | DOM-02 à 06 ; INV-02, 03, 05, 08 |
| 05–06 | DOM-02, 04, 06 ; INV-02 à 04, 06–08 |
| 07–08 | DOM-01, 03, 08 ; INV-01, 04 |
| 09–10 | DOM-06 à 08 ; INV-01, 07–08 |

**Q-DOM-01 — CLOSED sur le choix produit :** S-USER-06 retient plusieurs WAV classiques numérotés pour les longues captures Parakeet, associés à une transcription logique TXT/SRT unique. RF64 est écarté de cette évolution. Restent à l'architecte, avant readiness : taille et rotation des parties, correspondance des positions locales/globales, stockage consultable sans chargement intégral et comportement en échec de rotation. Aucun nouveau plafond de durée ni choix de taille n'est introduit par le domaine.

Les seuils de délai restent Q-PU-03 ; aucun seuil n'est ajouté. Après arrêt brutal, seules les ressources effectivement conservées peuvent subsister, avec cohérence non garantie ; aucune récupération automatique ni bibliothèque durable n'est introduite.

## 6. Architecture et preuves

Contribution `ARCH-PU v2`, auteur `/root/parakeet_architect` (`rust_architect`). v1 : domaine DOMAIN-PU v2 relu au Git `133f45f6`, registre `89467aa3`, produit `a483e176` ; S-USER-01 à 06 et REQUIREMENTS-PU v2. v2 : amendement ciblé sur base `bb308b58`, traite DESIGN-PU-R1 et A1 sans revendiquer leur fermeture. Sources : fichiers cités, Cargo.toml/Cargo.lock, source locale Symphonia 0.5.5, règles et contrats centraux. Terminé : inventaire, décisions techniques proposées, contrats, ressources, couverture et classification des preuves. Restant : re-revue et acceptation du design ; plan/code non commandés. Aucun fichier modifié ni commande Cargo/exécution produit par le spécialiste.

### 6.1 Contraintes constatées et portée du changement

Lever seulement le refus des durées longues serait insuffisant :

- `worker_ipc.rs:4626–4640` et `archive.rs:2620–2646` refusent le profil long en production.
- L'ancien parcours VAD accumule tout le PCM (`worker_ipc.rs:5983`), puis toute la liste des fenêtres (`:5994`). Il ne fournit donc pas une architecture de mémoire bornée.
- `WavOutput` refuse la limite RIFF32 (`direct_live.rs:1341`), lit à offset fixe 44 (`:1366`) et finalise un en-tête 32 bits. S-USER-06 permet de résoudre cette contrainte par rotation de WAV classiques.
- `application.rs:873` accumule le texte ; les vues contiennent des vecteurs de segments/lacunes (`:1536–1546`) ; `ui.rs:922/984` lit des TXT entiers.
- Le worker Parakeet charge une fois le moteur puis accepte des commandes successives (`main.rs:70–97`). Sa normalisation exige des mots et temps valides (`native_engine.rs:76–111`). La limite d'une requête demeure 30 minutes ; elle ne doit plus constituer une limite de session.
- Le planificateur actuel utilise des centres 60/90/120 secondes et des marges de 200 ms, mais peut refuser une longue parole sans silence (`parakeet_windows.rs:7–12,159–218`).

**DEC-T-PU-01 — proposé :** une stratégie applicative `bounded-windows-v1`, commune à l'import et à la finalisation micro, soumet des clips RIFF courts au worker existant. Elle ne se présente pas comme une qualification acquise du traitement long. Les identités d'archives/caches incluent cette stratégie ; aucune reprise d'un ancien résultat `import-single-pass` comme s'il avait été produit par la nouvelle stratégie.

### 6.2 Composants et dépendances

| ID / responsabilité | Implantation cible | Dépendances autorisées et contrôles futurs |
| --- | --- | --- |
| ARCH-PU-01 — modèles et règles | `whisper-core/domain.rs`, types purs spécifiques si séparation utile | `std`, types purs ; aucune IO/UI/adaptateur. Contrôler imports et API publiques. |
| ARCH-PU-02 — orchestration de session | `whisper-core/application.rs`, contrats `ports.rs` | Domaine et ports possédés par l'application ; aucune implémentation concrète. |
| ARCH-PU-03 — source audio logique | adaptateurs de capture, stockage et décodage Parakeet | Core, `std::fs/io`, Symphonia et convertisseur existants ; hors thread UI. |
| ARCH-PU-04 — ordonnanceur/reconnaissance | `worker_ipc.rs`, `parakeet_live.rs`, `parakeet_import.rs`, règles pures de fenêtres | Core et worker isolé ; aucun appel natif dans le core ou l'UI. |
| ARCH-PU-05 — document et publication | stockage Parakeet et `archive.rs` | Core, IO de fichiers ; réutiliser réservations, identités et protections de remplacement. |
| ARCH-PU-06 — présentation paginée | `whisper-desktop/src/ui.rs` | API de l'application et types purs ; lecture du document via port, sans `read_to_string` sur le nouveau parcours. |
| ARCH-PU-07 — assemblage | `whisper-desktop/src/root.rs` et composition existante | Instancie et injecte ; aucune seconde règle métier. |

Les dépendances physiques de `whisper-desktop` incluent les adaptateurs parce que le crate contient aussi la composition. Cela n'autorise pas de nouveaux appels d'infrastructure depuis l'UI. Les imports et lectures directs déjà observés constituent une dette locale ; le nouveau parcours utilise les ports. Pas de refonte globale des autres écrans.

**Stack conservée**, selon Cargo.toml/Cargo.lock : Rust déclaré 1.98.1, édition 2024 ; Symphonia 0.5.5 WAV/PCM/MP3, CPAL 0.18.2, Rubato 0.16.2, rtrb 0.4.0, eframe 0.35.0, serde 1.0.228, serde_json 1.0.145, sha2 0.10.9 et libloading 0.8.9. Les versions de rustc installé et des DLL déployées ne sont pas vérifiées par ce cadrage.

Le worker et ses runtimes CPU/GPU existants restent isolés. Aucun nouveau FFI, `unsafe`, pilote, service ou dépendance native n'est proposé. Les ressources personnelles/Test et leur qualification de livraison restent régies par AGENTS ; aucun paquet n'est construit ici.

### 6.3 Source longue, fenêtres et document

**DEC-T-PU-02 — proposé, réalise S-USER-06 :** source logique en échantillons 16 kHz `u64`, arithmétique contrôlée, et parties WAV classiques de **1 Gio maximum de données PCM**. Cette taille technique laisse une marge sous RIFF32 ; elle ne limite pas la durée totale. Scinder le bloc à la frontière exacte, finaliser la partie, ouvrir la suivante avec création exclusive, puis poursuivre la même chronologie.

Le nom de famille reste celui de la session ; les parties portent un numéro et leurs bornes globales dans l'index produit. Le nommage physique exact doit préserver le cas court et sera fixé avant le lot de stockage. L'index sert à retrouver l'audio, pas à qualifier les agents. Ses enregistrements sont ajoutés et consultés par pages, sans charger toutes les parties.

Un échec de création/écriture/rotation arrête proprement la capture, conserve les parties réellement écrites et expose frontière et incident. Il ne devient ni une rotation réussie ni un arrêt normal causé par la durée.

L'import conserve sa source et utilise la liaison/snapshot existante. Décodage **séquentiel une seule fois**, en blocs bornés, dans un thread d'adaptateur via les dépendances Rust existantes ; ne pas rescanner le fichier depuis le début pour chaque bloc. Le PCM nécessaire à la suite est stocké dans la source logique. Hash et inspection de source restent des parcours streaming ; pas de chargement intégral ni de nouveau hash intégral par fenêtre.

**DEC-T-PU-03 — proposé :** planification incrémentale des fenêtres finales, avec curseur contigu : cible 90 s, recherche d'un silence entre 60 et 120 s, puis **coupe à 120 s si aucun silence admissible**, contexte de 200 ms de chaque côté. Ces paramètres reprennent les valeurs existantes ; le repli sans silence supprime le refus structurel de parole continue. Leur qualité reste à valider.

Un mot doté de temps valides appartient à l'intervalle central contenant son milieu temporel, règle déjà définie par `word_belongs_to_central_range` (`parakeet_windows.rs:262–271`). Temps locaux → globaux par addition contrôlée. Aucun mot n'exige deux reconnaissances concordantes. La qualité aux raccords et les variations des temps restent des risques ASR testables ; une couverture audio complète ne les résout pas.

**DEC-T-PU-04 — proposé :** l'aperçu courant reste court et révisable, fenêtre maximale de 4 s issue du direct existant. Les résultats de fenêtres finales remplacent les portions correspondantes dans le même document. Le traitement final peut avancer pendant la capture ; après arrêt, il traite le reliquat jusqu'à la frontière close. Le budget historique de **30 s de finalisation directe est retiré de cette voie** : il ne doit pas tronquer le résultat.

Le document, ses révisions, les résultats et la couverture sont persistés par enregistrements/pages. Le succès séquentiel avance un préfixe contigu ; à la première plage finale défectueuse, l'opération conserve son résultat partiel et s'arrête avec incident. Pas de liste RAM de tous les mots, fenêtres, échecs ou révisions.

### 6.4 Contrats concrets et ressources

Les ports sont possédés par l'application ; leurs adaptateurs exécutent les effets.

| Port | Entrées / résultat / préconditions | Erreurs, thread, arrêt et durabilité |
| --- | --- | --- |
| PORT-PU-01 — Source audio | Identité, bloc/plage globale ; écrit ou lit une plage, retourne frontière réellement écrite et référence de partie. Blocs consécutifs, indices vérifiés. | Callback non bloquant ; writer dédié et tampon capture existant de 100 slots. Saturation = incident de capture. L'arrêt ferme admission puis draine ; l'ack distingue écriture, flush et synchronisation réellement réalisées. |
| PORT-PU-02 — Décodage import | Source liée, curseur ; produit prochain bloc PCM, au plus 80 000 échantillons, ou EOF réel. | IO/décodage/changement de source explicites ; état de conversion conservé entre blocs. Annulation entre blocs ; source d'origine inchangée. |
| PORT-PU-03 — Reconnaissance | Session/génération/requête, clip borné, identité moteur/calcul ; retourne texte/mots/temps et intervalle source. | Un appel par worker, hors UI ; aucune interruption native instantanée promise. Temps invalides : résultat final défectueux, audio et texte déjà utilisable conservés, jamais temps inventés. |
| PORT-PU-04 — Document | Révision ou lecture page avec version attendue ; retourne nouvelle référence ou page bornée. | Révision périmée ignorée ; conflit réel signalé. Disque proportionnel au contenu ; cache UI de pages fixe, événements limités, pas de chaîne complète. |
| PORT-PU-05 — Publication | Document final, couverture, sorties attendues, autorisation précise de remplacement. | Écriture streaming TXT/SRT vers génération préparée ; contrôles puis engagement de référence. Chaque fichier a son résultat réel. Échec conserve ancienne référence et fichiers récupérables. Pas de transaction atomique présumée entre WAV/TXT/SRT. |

**DEC-T-PU-05 — proposé, version 2 :** un ordonnanceur applicatif possède les demandes, avec une seule inférence en vol par backend et aucune concurrence GPU libre entre captures. L'arbitrage combine deux catégories : aperçu de la capture active et finalisation des sessions admises. Il applique le quota technique **P = 1 aperçu au maximum entre deux appels finaux lorsque du travail final est éligible**.

À chaque frontière d'appel :

1. Si des finalisations sont éligibles et qu'un aperçu a été exécuté depuis le dernier appel final, sélectionner obligatoirement une finalisation.
2. Sinon, sélectionner le dernier aperçu disponible ; s'il n'y en a pas, sélectionner immédiatement une finalisation.
3. Les tours finaux sont distribués **à tour de rôle entre les sessions éligibles du même backend**, initialement par ordre d'admission. Après un appel final, une session qui a encore du travail revient derrière les autres sessions éligibles.
4. Une catégorie vide n'impose aucune attente : sans aperçu, les appels finaux s'enchaînent ; sans finalisation éligible, les aperçus peuvent s'enchaîner. L'arrivée de nouveaux aperçus ne remet jamais à zéro le quota accumulé ni l'ordre des sessions finales.

Une plage finale devient éligible lorsque sa source écrite, sa configuration et son backend permettent de préparer son prochain clip borné. La préparation du clip fait partie du tour réservé : elle ne peut être indéfiniment repoussée au profit d'aperçus. Une erreur de préparation produit son issue explicite ; elle ne transforme pas la plage en travail oublié.

Avec au plus **deux sessions non terminales**, chaque session disposant de travail final obtient donc un tour après **au plus quatre sélections donnant lieu à un appel natif**, à compter d'une frontière où elle est éligible : au plus trois autres appels peuvent la précéder. Entre deux de ses tours finaux, au plus un autre tour final et deux aperçus peuvent intervenir. Cette propriété suppose que les appels engagés terminent et que les effets nécessaires de préparation aboutissent ou signalent leur échec. Elle borne un **nombre d'appels**, pas leur durée.

Le quota P = 1 est un choix technique de partage : conserver des mises à jour d'aperçu tout en réservant au moins un appel sur deux au travail final lorsqu'il existe. Le tour de rôle empêche également qu'une très longue finalisation A monopolise tous les tours finaux au détriment de B. Ce quota n'est ni un seuil de performance accepté par l'utilisateur ni une limite de durée de session.

Le callback et l'écriture audio restent indépendants des réponses d'inférence. Un aperçu peut être remplacé dans sa file de capacité un ; les plages finales restent référencées sur disque. La limite de deux sessions, le refus avant ouverture du micro en cas de saturation et les huit fiches récentes restent inchangés.

**Limite de préemption — DESIGN-PU-A1 :** un aperçu nouvellement disponible peut attendre la fin d'un appel final déjà engagé. La borne de 120 secondes concerne l'audio du clip central, avec son contexte ; elle ne borne pas le temps d'exécution natif. L'appel engagé n'est pas interrompu pour satisfaire le quota. Une fermeture normale supprime les futurs aperçus de la capture arrêtée, conserve les obligations finales et attend leurs issues réelles.

Limiter les opérations acceptées à **deux sessions non terminales** : A peut se finaliser pendant la capture B ; après arrêt B, ses plages attendent sans nouvelle liste de tâches RAM. Une nouvelle opération qui dépasserait cette capacité est refusée **avant ouverture du micro**, avec attente expliquée. C'est une limite de concurrence, pas de durée. Garder au plus huit fiches récentes, conformément à la borne actuelle. Résultats/commandes IPC conservent la limite existante de 1 Mio. Les tables worker de tentatives doivent rester limitées aux sessions actives, plutôt que croître à chaque session.

Une annulation avant engagement invalide la génération, attend la fin réelle de l'appel engagé ou son issue de processus, et rejette les retours obsolètes. Après engagement, rendre l'issue réelle. La fermeture normale bloque de nouvelles admissions, arrête la capture, puis attend toutes les obligations A/B. Aucun arrêt sur l'âge seul ni repli CPU après début audio n'est introduit. Les délais de cette attente restent Q-PU-03.

**V-PU-EQUITY-01 — PRODUCT_VALIDATION, NOT RUN ; REQ/AC-PU-02, 07 et 10.** Scénario déterministe futur sur un même backend :

- A est arrêtée et possède plusieurs plages finales ; B capture et régénère systématiquement un aperçu avant chaque décision de l'ordonnanceur.
- Ajouter aussi des plages finales de B pour exercer le partage entre sessions.
- Un worker contrôlé termine chaque appel uniquement sur commande du test. Sources et préparation répondent de manière contrôlée ; aucune mesure de performance native n'est nécessaire.
- Vérifier qu'après chaque frontière pertinente, au plus un aperçu précède un tour final ; que A et B alternent leurs tours finaux lorsqu'elles restent toutes deux éligibles ; et que chacune obtient son appel au plus tard à la quatrième sélection.
- Trace attendue possible : `aperçu B → final A1 → aperçu B → final B1 → aperçu B → final A2`. Exercer aussi l'arrivée de A lorsque le prochain tour final revient d'abord à B.
- Vérifier identités session/génération, avancement réel de couverture, puis libération du créneau lorsque A devient terminale.
- Demander la fermeture : arrêter B, ne plus produire d'aperçu, terminer les plages finales restantes et constater l'attente de leur issue.
- **Témoin négatif :** une variante donnant toujours priorité à l'aperçu doit violer l'assertion de tour final borné. Ne pas vérifier seulement une trace nominale favorable.
- **Attente non préemptible :** retenir un appel final engagé, produire un aperçu B et constater qu'aucun deuxième appel n'est lancé. Après libération, vérifier la reprise conforme. Cela démontre l'attente possible, pas un délai utilisateur.

**Précision Q-PU-03, architecte :** la question reste ouverte. Les mesures de premier texte, retard et attente après arrêt intègrent la durée résiduelle d'un appel final non préemptible et les tours imposés par DEC-T-PU-05 v2. L'équité garantit une occasion de progresser sous les hypothèses indiquées ; elle ne garantit pas le délai d'apparition d'un aperçu. Analyste/architecte instruisent, utilisateur accepte les seuils avant validation dépendante. Si un seuil impose une autre granularité ou politique, la question devient STRUCTURING et les parties affectées sont revues.

### 6.5 Publication, compatibilité et preuves

**DEC-T-PU-06 — proposé :** un document final et un TXT/SRT logiques par session, temps globaux continus malgré les parties WAV. La référence principale n'est activée qu'après couverture et sorties valides. Une erreur SRT peut laisser un texte utilisable ; le paquet reste incomplet. Les anciens fichiers confirmé/brouillon ne sont ni fusionnés ni réécrits rétroactivement.

Le store de publication actuel fournit réservations et engagement de référence (`archive.rs`, notamment `CURRENT` et index). Son implémentation doit être adaptée aux lectures/écritures streaming, pas réutilisée avec ses vecteurs complets. La cohérence lors d'erreurs normales appartient au périmètre ; reprise après panne et réparation automatique restent différées.

| Preuve / risque | Classe, état et échéance |
| --- | --- |
| Réutilisation worker clips successifs et validation bornée | **DESIGN_FEASIBILITY — PASS documentaire**, sources `main.rs:70–97`, `native_engine.rs:12–56`, `parakeet_ipc.rs:3–6`. Ne prouve pas qualité longue. |
| Rotation RIFF, lecture par plages, agrégation et export streaming | **DESIGN_FEASIBILITY — PASS documentaire sur le mécanisme**, appuyé sur IO/Ranges déjà employés et choix S-USER-06. Nouveau code non exécuté. |
| RISK-T-PU-01 — trous/doublons/temps aux raccords | **PRODUCT_VALIDATION — NOT RUN**, AC-02/04/05 : parole continue et pauses FR/EN, fins de capture et rotations. |
| RISK-T-PU-02 — PCM, index, UI ou résultats encore cumulatifs | **PRODUCT_VALIDATION — NOT RUN**, AC-04 : inspection de tous chemins et mesures sur durées croissantes ; mémoire de travail, temporaires et données disque distingués. |
| RISK-T-PU-03 — famine de finalisation, contention, saturation ou fermeture A/B | **PRODUCT_VALIDATION — NOT RUN**, AC-02/06/07/10 : quota P = 1, tours finaux alternés entre au plus deux sessions ; V-PU-EQUITY-01 contrôle la borne de quatre appels, le témoin négatif, les identités et les issues réelles. Compléter par une recette Windows avec chevauchement réellement observé ; la borne d'appels ne prouve aucun délai natif. |
| RISK-T-PU-04 — erreurs disque, rotation ou publication | **PRODUCT_VALIDATION — NOT RUN**, AC-04/05/06/09 : injections séparées, audio et ancienne référence préservés. |
| Runtimes personnels/Test, workers, modèles, DLL et backend réel | **DELIVERY_QUALIFICATION — NOT RUN**, avant remise Windows selon AGENTS. |

Aucun SPIKE supplémentaire n'est proposé : le design n'exige ni capacité native nouvelle, ni nouveau conteneur ni nouvelle pile. Les preuves manquantes portent sur le produit à construire. Toute découverte contraire revient à l'architecte et à la revue ; ce classement n'est pas un PASS runtime.

**Couverture bidirectionnelle :** CAP/REQ/UC/AC-01→DOM-05/INV-01→ARCH-02/04/06 ; 02–04→DOM-02/03/04/INV-02/03/05/08→ARCH-03/04 ; 05–06→DOM-06/INV-04/06/07→ARCH-03/05 ; 07–08→DOM-01/03/08/INV-01/04→ARCH-02/04/07 ; 09–10→DOM-07/08/INV-01/07/08→ARCH-02/05. Les risques ci-dessus et AC de §4 couvrent ces responsabilités ; aucun composant ajouté sans capacité associée.

RF64 est écarté par S-USER-06. La recherche primaire du 26 octobre 2026 a confirmé son mécanisme de dépassement RIFF32 dans [EBU Tech 3306-2009, §3.5](https://tech.ebu.ch/docs/tech/tech3306v1_1.pdf) ; elle n'est pas une dépendance ni une preuve exigée de la solution choisie. Aucun railguard nouveau n'est proposé.

## 7. Questions, décisions et couverture

Registre établi par la session principale à partir des réponses utilisateur et des contributions attribuées. Aucun silence ne vaut acceptation de durée, délai ou reprise après panne.

### 7.1 Décisions et questions

| ID | Nature / statut / source | Owner et échéance | Conséquence |
| --- | --- | --- | --- |
| DEC-PU-01 | Produit ACCEPTED, S-USER-01/02 | Utilisateur, acquise | Un seul texte révisable pendant capture, puis résultat final automatique ; pas de confirmation de mots à gérer. |
| DEC-PU-02 | Produit ACCEPTED, S-USER-04/05 ; Q-PU-01/02 CLOSED sur le besoin | Utilisateur, acquise | Inclure >30 min sans plafond applicatif de durée ; limites physiques et erreurs explicites, ressources de travail à borner. |
| DEC-PU-03 | Produit ACCEPTED, S-USER-06 ; Q-DOM-01 CLOSED sur le choix produit | Utilisateur, acquise | Plusieurs WAV classiques numérotés pour les longues captures Parakeet ; une session, un document et un couple TXT/SRT logiques. Pas de RF64. |
| Q-PU-03 | DETAIL OPEN ; devient STRUCTURING si seuil conditionnant architecture | Analyste/architecte pour métriques et compromis, utilisateur pour seuils ; avant validation/lot dépendant | Aucun nouveau délai maximum accepté. Conserver la distinction premier texte utile, retard, capture drainée, résultat et sorties disponibles. Ne pas requalifier l'ancienne cible GPU par simple changement de nom. |
| Q-RQ-01 | DETAIL OPEN | Analyste/UI ; avant lot UI | Information sur absence de progrès sans arrêt automatique par l'âge seul. Seuil d'alerte éventuel à préciser ; aucun seuil inventé. |

### 7.2 Changement de contrats et portée

**CHANGE-PU-01 — DESIGN_CHANGE proposé, autorité S-USER-01/02/03 :** le nouveau parcours Parakeet remplace la référence visible « confirmé immuable » et le brouillon final secondaire par le document principal révisable puis final. Les règles actuelles ne deviennent pas des garanties de reconnaissance lexicale. Impacts : direct, application, interface, import final et exports Parakeet. Les fichiers/captures déjà produits ne sont pas réécrits ; les anciens cadrages progressifs restent l'historique de la version existante. Les lots d'implémentation antérieurs ne sont pas réactivés par ce cadrage.

**CHANGE-PU-02 — DESIGN_CHANGE proposé, autorité S-USER-04/05/06 :** retirer le plafond applicatif de durée du parcours Parakeet et autoriser une famille de WAV numérotés pour les longues captures. Cette autorité précise remplace l'objectif d'un WAV physique unique uniquement sur ce nouveau parcours ; TXT/SRT restent uniques par session. Turbo, modèles, calcul manuel/Auto, règles de repli et paramètres micro existants ne changent pas de portée. Compatibilité des anciennes sources et publications à traiter par l'architecte, sans migration destructive implicite.

### 7.3 Couverture transversale

CAP-PU-01 à 10 → REQ-PU/UC-PU/AC-PU-01 à 10 : couverture bijective en §4 ; domaine et invariants en §5.5 ; architecture, risques et preuves en §6.5. Les exigences et décisions techniques restent proposées jusqu'à acceptation du design ; les décisions utilisateur DEC-PU-01 à 03 sont déjà acquises et ne sont pas rouvertes.

## 8. Readiness

DESIGN DRAFT — contributions PRODUCT-PU v1, REQUIREMENTS-PU v2, DOMAIN-PU v2 et ARCH-PU v2 intégrées. Revue initiale FINDINGS sur `bb308b58` : DESIGN-PU-R1 ouvert, amendement soumis au même reviewer ; la session principale ne ferme aucun constat. Acceptation utilisateur du design, planification et implémentation restent distinctes.

| Rubrique de readiness | Applicabilité et éléments soumis à la revue |
| --- | --- |
| Scope | §§1/3/7, sources et décisions utilisateur, inclusions/exclusions explicites. |
| Use cases / errors | §4 UC/AC-01 à 10, états visibles, annulation, fermeture et incidents normaux. |
| Domain | §5 concepts, transitions, préconditions, effets et invariants. |
| Dependencies | §6.2 matrice par responsabilité, stack existante et contrôles futurs. |
| Persistence | §§5/6 source multi-WAV, document paginé, publications et compatibilité anciennes sorties ; reprise après panne différée. |
| Concurrency | §6.4 ordonnanceur, bornes, admission A/B, annulation et fermeture réelles. |
| Platform | Windows/Rust/CPU-GPU existants, pas de nouvelle capacité native ; qualification livraison NOT RUN. |
| Acceptance | §4 protocole, scénarios et questions de seuils ouvertes ; tests produit NOT RUN. |
| Risks | §6.5 faisabilité documentaire distinguée de validation produit et livraison, réserves anciennes non fermées. |
| Railguard | N/A : aucun nouveau railguard exigé ou activé ; règles AGENTS et frontières centrales analysées. |

Suite autorisable après ce cadrage : acceptation du design, puis mission distincte de planification avec un plan futur de dix tâches maximum lié aux REQ/AC. Aucun plan exécutable ni lot de code n'est produit par cette mission.
