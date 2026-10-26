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
| 4 — Exigences, cas d'usage, acceptation et risques fonctionnels | rust_requirements_analyst | PENDING |
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

Contribution attendue.

## 5. Domaine

Contribution attendue.

## 6. Architecture et preuves

Contribution attendue.

## 7. Questions, décisions et couverture

Les choix produit encore structurants sont instruits puis soumis à l'utilisateur ; aucune hypothèse de durée, délai ou reprise après panne n'est acceptée par silence.

## 8. Readiness

DESIGN DRAFT — contributions et revue en attente. Acceptation utilisateur du design, planification et implémentation restent distinctes.
