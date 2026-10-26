Avancée : cadrage de la transcription Parakeet unifiée ouvert ; qualification du besoin validée dans cette session.
Blocage : aucun pour instruire le cadrage ; décisions structurantes et revue DESIGN en attente.
Décision : texte unique révisable pendant la capture, puis résultat final automatique ; aucune implémentation autorisée.

# Parakeet — transcription unifiée au micro et à l'import

## 1. Identité, autorisation et sources

- ID : D-PARA-UNIFIED-01 ; version de travail : v0.1 ; statut : DRAFT.
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
| 5 — Domaine, états et invariants | rust_domain_architect | PENDING |
| 6 — Architecture, contrats, dépendances et preuves | rust_architect | PENDING |
| 7 — Questions et traçabilité transversale | sources utilisateur et contributions attribuées | PENDING |
| 8 — Readiness et suite | session principale, sur verdict du reviewer | PENDING REVIEW |
| Compte rendu de revue séparé | rust_design_reviewer | NOT RUN |

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

Contribution attendue.

## 6. Architecture et preuves

Contribution attendue.

## 7. Questions, décisions et couverture

Les choix produit encore structurants sont instruits puis soumis à l'utilisateur ; aucune hypothèse de durée, délai ou reprise après panne n'est acceptée par silence.

## 8. Readiness

DESIGN DRAFT — contributions et revue en attente. Acceptation utilisateur du design, planification et implémentation restent distinctes.
