# Reprise Whisper — MVP import CPU (13 octobre 2026)

Plan initial ; validation avec ajustements et exécution limitée du lot 1 consignées dans [reprise-mvp-lot1.md](reprise-mvp-lot1.md).

Avancée : les parcours WAV et MP3 vers TXT/SRT fonctionnent en essai d'intégration sur ce PC.
Blocage : l'usage dans la fenêtre Windows et la disponibilité du modèle au premier lancement restent à vérifier.
Décision : valider le plan MVP ci-dessous avant tout développement.

## Périmètre et constat

**MVP retenu par `AGENTS.md`** : dans l'application Windows existante, choisir un WAV ou MP3, le transcrire sur CPU et obtenir TXT et SRT. Critères proposés : **M1** chaque format valide atteint un résultat ; **M2** le calcul effectif est CPU ; **M3** TXT et SRT sont lisibles, non vides et cohérents avec l'audio ; **M4** l'original reste inchangé, les erreurs sont visibles et aucun échec n'est présenté comme terminé ; **M5** ce parcours est utilisable dans la fenêtre Windows avec un modèle local et un dossier de sortie connus.

`D-WHISPER-30` et `P-WHISPER-31` décrivent une V1 bien plus large. Le plan P31 garde la chaîne L00–L07 ; `state.json` indique IMPLEMENTATION, L00–L03 `completed`, L04–L07 `planned`. Ces statuts historiques ne démontrent pas à eux seuls le MVP. Le code L04 déjà présent est partiel au regard du lot historique ; il n'est pas requis pour fermer M1–M5.

**Observé sur `reprise-mvp`** : `cargo build --offline --locked --workspace` et `cargo test --offline --locked --workspace` réussissent. Les tests d'intégration optionnels, relancés avec le modèle local existant et les deux courts fichiers réels, passent : WAV → worker CPU attesté, source inchangée, TXT non vide (114 octets) et SRT non vide (211 octets) ; MP3 → import CPU terminé, source inchangée et archive TXT/SRT déclarée complète. La présence et l'identité du modèle sont vérifiées par le test WAV. Aucun téléchargement ni installation. Ces essais couvrent le moteur et les adaptateurs ; aucun parcours manuel de la fenêtre ou installation sur PC sans modèle n'a été constaté ici.

**Écart MVP** : l'interface expose encore micro, GPU et mode Auto, avec Auto présélectionné ; le parcours demandé doit être centré sur l'import CPU. Les erreurs de fichiers, modèle ou destination et le contenu/horodatage des sorties doivent être vérifiés dans l'application réelle. `whisper-bootstrap` reste un squelette : la mise à disposition du modèle n'est pas assurée par le produit. Pour ce MVP sur le PC actuel, un modèle local explicite peut suffire si le manque est signalé clairement.

**Différés** : micro en direct, GPU, Auto et repli, tray, reprise après panne, sortie MP3, supervision/campagnes L04, réglages OS et installation complète L05–L07. Conserver le code L04 existant sans le prolonger ni le qualifier pour ce MVP ; décider de son devenir seulement si un défaut concret du parcours import CPU l'exige.

## Plan proposé — aucune tâche exécutée

1. Fixer M1–M5 et un jeu court WAV/MP3 avec résultat attendu (M1–M4).
2. Mettre l'interface en parcours import CPU explicite, sans choix GPU/Auto ni commande micro dans le flux MVP (M2, M5).
3. Vérifier dans la fenêtre l'import WAV puis MP3 jusqu'à l'état terminé et aux deux fichiers visibles (M1, M3, M5).
4. Contrôler texte, chronologie SRT et préservation des originaux sur les deux formats ; corriger les écarts constatés (M3, M4).
5. Rendre le chemin du modèle et le dossier de sortie utilisables et leurs erreurs compréhensibles, sans téléchargement implicite (M4, M5).
6. Tester format invalide, modèle absent, destination inaccessible et interruption simple : aucun faux « terminé » ni altération de la source (M4, M5).
7. Préparer un dépôt reproductible : ajouter `target/` au `.gitignore` ; statuer sur `crates/whisper-adapters/archive/` (diagnostic généré, recommandé hors Git) et sur le dossier parasite racine `--help` (recommandé à supprimer après vérification) (M5).
8. Recompiler, relancer les tests utiles et refaire le parcours Windows WAV/MP3 → TXT/SRT sur CPU ; ne déclarer le MVP prêt que si M1–M5 sont observés (M1–M5).

**Arbitrage recommandé** : valider ce périmètre import CPU avec modèle local déjà disponible ; réserver l'acquisition automatique et les fonctions différées à une décision ultérieure.
