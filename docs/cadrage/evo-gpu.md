Avancée : le worker GPU compile et transcrit l'import de référence en 0,31 fois sa durée.
Blocage : le micro GPU dans la fenêtre et les seuils sur une minute restent à démontrer.
Décision : préciser la suite d'une panne GPU en cours de micro, la comparaison qualité et le départ du délai d'affichage.

# Évolution GPU — cadrage proposé (13 octobre 2026)

## Besoin, périmètre et critères

Dans Whisper Windows, l'utilisateur choisit **CPU ou GPU** avant chaque import WAV/MP3 ou séance micro, avec le même **large-v3-turbo** local. Le CPU reste utilisable. Le micro garde ses blocs confirmés et ses sorties TXT/SRT/WAV ; micro et import restent exclusifs. Exclus : Auto, bascule automatique, supervision, reprise après panne, autre modèle, tray, sortie MP3. D-WHISPER-30 et P-WHISPER-31 ne gouvernent pas ce lot.

| ID | Critère d'acceptation proposé, à vérifier dans la fenêtre Windows en release |
|---|---|
| AC1 | Choisir CPU puis GPU ; moteur demandé effectif, CPU encore sélectionnable après erreur, sans bascule implicite. |
| AC2 | Micro GPU avec parole, silence et phrase finale : blocs confirmés ordonnés avant Arrêter ; TXT/SRT/WAV de même nom, dernière phrase et SRT chronologique après arrêt ou fermeture. |
| AC3 | Imports WAV/MP3 GPU : TXT/SRT lisibles, source intacte ; parcours CPU préservé. |
| AC4 | Même fichier large-v3-turbo, langue et décodage CPU/GPU ; textes comparés selon Q2. |
| AC5 | Sur ce PC, import et micro de **60 s** : traitement GPU / durée audio **< 1**, résultat complet. Pour le micro, sommer le calcul des fenêtres ; relever préparation et export séparément. |
| AC6 | Premier bloc micro confirmé **affiché** en **< 15 s** selon Q3 ; l'événement worker ne suffit pas. |
| AC7 | GPU absent ou défaillant : cause et action visibles, aucun faux succès ; CPU disponible manuellement, sort de l'audio capté selon Q1. |

## Faisabilité locale et réemploi

PC : **GTX 1080 Ti** (11 264 MiB), pilote 581.29 ; CUDA **12.9**, CMake 4.4.4, Rust 1.98.1, VS2022 BuildTools et modèle local présents. Le worker `whisper-rs 0.16.0`/CUDA compile en release **hors ligne**. Les traces natives confirment `using CUDA0 backend` et 1 623,92 Mo de tampon CUDA ; `Ready Cuda` seul ne prouve pas l'usage GPU.

Extrait CPU historique (`voice-sample.wav`, 26,3 s + silence) recréé à **30,000 s** dans `target/` ; même audio/modèle/langue, workers release séparés, CPU 24 fils. **GPU : 9,349 s total = 0,312× ; 7,252 s après `Ready`. CPU : 69,411 s total = 2,314× ; 67,706 s après `Ready` = 2,257×**, contre 2,25× antérieurs. GPU **9,34 fois** plus rapide après `Ready`. Sept segments chacun ; **textes concaténés identiques**. Premier texte worker GPU à 3,238 s ; affichage micro et qualité générale non démontrés.

Réemploi : GPU `StartImport` et ancien `StartLiveWorker` ; `ComputeChoice` et attente du choix utilisateur après échec GPU. `ui.rs` masque CPU/GPU/Auto et force CPU. Le **micro direct livré** (`direct_live.rs`) impose CPU ; le GPU ignore sa commande et borne les fenêtres à 5 s contre 8 s au micro. Raccorder le parcours direct sans réactiver MP3/reprise historiques ni Auto.

Architecture : conserver capture 16 kHz, WAV et export ; figer le choix par requête, vérifier le worker avant capture, préserver identité/ordre des fenêtres et calcul hors UI. En erreur, arrêter la capture, signaler l'incomplet, laisser CPU sélectionnable. Succès après trois exports. Détecter le repli natif silencieux sur CPU.

## Plan proposé — 8 tâches, sans autorisation de coder

1. Décider Q1–Q3 et fixer les scénarios de mesure/texte — AC4–AC7.
2. Rendre le choix CPU/GPU visible sans Auto, puis le porter réellement aux requêtes import et micro — AC1, AC3.
3. Vérifier au démarrage le backend réellement utilisé et traduire l'indisponibilité en message/action — AC1, AC7.
4. Raccorder le worker GPU au micro direct et aux fenêtres de 5 à 8 s — AC2, AC5–AC6.
5. Préserver ordre, confirmation, arrêt, fermeture et exports du micro avec le moteur choisi — AC2, AC7.
6. Traiter panne GPU et résultat partiel selon Q1, sans bascule automatique — AC1, AC7.
7. Tester choix, erreurs, identité des blocs, sorties et non-régression CPU/import — AC1–AC4, AC7.
8. Qualifier dans la fenêtre 60 s import/micro, délai du premier affichage, qualité CPU/GPU et indisponibilité — AC1–AC7.

## Décisions produit attendues

Q1–Q3 sont des décisions **structurantes**, proposées à l'utilisateur avant les tâches qui en dépendent.

- **Q1, panne GPU en micro (AC2, AC7)** : A **recommandée** — arrêter, conserver WAV et blocs confirmés marqués incomplets ; l'utilisateur peut réimporter le WAV en CPU. B — erreur sans résultat partiel. C — finir sur CPU dans la même séance après accord explicite.
- **Q2, qualité (AC4)** : A **recommandée** — comparer CPU/GPU sur extrait et micro ; ponctuation tolérée, sans omission ni changement de sens, validation utilisateur. B — corpus annoté et seuil numérique à définir.
- **Q3, premier texte (AC6)** : A **recommandée** — chrono depuis Démarrer, chargement inclus, parole dès l'écoute. B — depuis la première parole, chargement séparé. Préciser si la cible s'applique aussi aux imports.

Sources : demande, `AGENTS.md`, bilan/cadrage micro, crates ; apports `PRODUCT-GPU-01 v0.2`, exigences v0.2, `ARCH-GPU-v1` (base `95933e3a`). Mesure IPC du 13 octobre : sortie de session non journalisée ; WAV dans `target/`. `cargo test --workspace --locked --offline` réussi. **Proposé** : critères produit non qualifiés, aucun code autorisé.
