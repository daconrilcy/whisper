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
| 3 — Brief, capacités et décisions produit | rust_product_framer | PENDING |
| 4 — Exigences, cas d'usage, acceptation et risques fonctionnels | rust_requirements_analyst | PENDING |
| 5 — Domaine, états et invariants | rust_domain_architect | PENDING |
| 6 — Architecture, contrats, dépendances et preuves | rust_architect | PENDING |
| 7 — Questions et traçabilité transversale | sources utilisateur et contributions attribuées | PENDING |
| 8 — Readiness et suite | session principale, sur verdict du reviewer | PENDING REVIEW |
| Compte rendu de revue séparé | rust_design_reviewer | NOT RUN |

## 3. Brief produit

Contribution attendue.

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
