Avancée : démarrage autorisé du sous-lot C ; cartographie détaillée de la famille 7 en cours.
Blocage : aucun établi ; toute injection de production nécessaire sera proposée séparément.
Décision : exécuter C selon le plan accepté, sans simplification ni fusion, en commits locaux seulement.

# REFACTO-POLL — sous-lot C

## Autorité et référence

Le 3 novembre 2026, après clôture B et vérification de son nettoyage,
l'utilisateur demande « ok passe à l'étape suivante ». Cette demande engage
C, familles 7 à 9 du [plan accepté](REFACTO-POLL-PLAN.md), dans `C:/wt/poll`,
branche `refacto/poll`, depuis `105b686e14a19cde2d2f637a5990dce0c51da515`.
Les bilans A/B restent figés ; leurs preuves natives ne qualifient pas par
anticipation le candidat final C. Aucun push ni nouvelle campagne N2 autorisé.

Objectif : terminer le découpage de `ImportRuntime::poll()` en conservant
événements, priorités, effets, contrôles et ordres de destruction exacts.
État initial : poll à 489 lignes Clippy, complexité 52 ; six handlers B
conformes aux seuils. État Git suivi propre ; `models/` et `sound example/`
non suivis, conservés. Espace libre initial : 586,78 Gio.

## Séquence et chemins

1. Référence N1 sur la source inchangée, puis versionnement de ce suivi.
2. **7a** : tests de caractérisation nouveaux, séparés sous
   `crates/whisper-adapters/src/worker_ipc/runtime/poll/`, verts sur la logique
   non extraite de la famille 7 ; aucune modification de production.
3. **7b** : extraction de collecte/priorités en sous-handlers distincts,
   après revue indépendante de 7a et commit des tests.
4. **8a** : caractérisation du routage restant et du traitement final commun,
   sur leur logique inchangée, puis revue et commit.
5. **8b** : extraction restante, sans fusion ; poll doit atteindre les seuils.
6. **9** : mesures finales, vérifications, archivage et bilan figé d'une page,
   puis arrêt. Famille documentaire : aucune extraction ni Na/Nb artificiel.

Chemins Rust : `runtime/poll.rs`, nouveaux fichiers sous `runtime/poll/`,
déclarations de modules nécessaires. `ci/tests-baseline.txt` reçoit seulement
les entrées des nouveaux tests, indispensables au contrôle Structure.
Documents : présent suivi et bilan sous `docs/architecture/`. Journaux et
temporaires : `target/parakeet-tests/refacto-poll-c/`. Aucun autre chemin
produit, test existant, manifeste, lockfile, dépendance ou signature publique
modifié. Les supports cfg(test) existants peuvent être utilisés sans extension.

## Risques de caractérisation

**Famille 7.** Priorités simultanées sessions unifiées/document/file ; libération
terminale ; test micro événement/absence/erreur ; expiration idle ; Direct
actif/terminé, récupération éventuelle de worker, erreur ; capture Parakeet en
erreur et final automatique ; priorités compteurs/reset/observations/télémétrie,
capture/équité ; attente de fenêtre persistée ; erreurs VAD/début de fenêtre,
Ready synthétique unique ; collecte Parakeet événement/absence/erreur ; Stop
différé bloqué par persistance ou sortie processus ; transport vide/erreur,
avec ou sans reprise. Affiner branche par branche avant extraction, en citant
les tests traversant poll et leurs oracles sur événements, état restant et
absence d'effets. Un test du seul helper ne protège pas la collecte de poll.

**Famille 8.** Progress, Stopped normal, Failed corrélé/résiduel et événement
inattendu ; erreur live différée avec Stop/drain/failure ; télémétrie après un
résultat non terminal ; distinction entre retour immédiat et traitement final.
Les familles déjà extraites restent inchangées.

Conserver et documenter les branches inatteignables sans état artificiel.
L'exception de fixtures défensives accordée à 6a ne s'étend pas à C. Si une
branche exige un nouveau point d'injection de production, arrêter cette étape
et présenter séparément le changement proposé ; aucun ajout produit dans Na.
Aucune limite artificielle de taille pour Na. Aucune mutation obligatoire
supplémentaire : celles des familles 2/3/5/6 sont conservées comme preuves
historiques, sans les présenter comme exécutées sur C.

## Contrats d'extraction et vérifications

Sorties explicites : point suivant, sortie immédiate (y compris None), passage
au traitement final commun. Collecte : distinguer événement acquis et sortie.
Ne pas créer de booléen ambigu, boucle de drainage, clone ou état produit
pour contourner les emprunts. Aucun changement dans poll_unified_sessions ou
poll_parakeet_worker. Ne pas fusionner les nettoyages ou handlers similaires.

Chaque Nb déplace au plus 800 lignes. Chaque handler et, au terme de 8b,
poll respectent 100 lignes Clippy et une complexité cognitive de 25. Si cela
exige de réordonner du code : arrêt et signalement. Aucun allow ou référence
Clippy ajouté pour contourner un seuil ; aucune référence Clippy nouvelle.

Référence avant le premier commit, puis à chaque Na/Nb et à la clôture :
garde N1 existante, SelfTest, Structure, format, Clippy workspace/all-targets,
`cargo test`, puis sélections common/CPU/Parakeet CPU. Commandes via la garde,
offline/locked ; cible partagée `C:/dev/whisper/target/parakeet-tests/cargo-shared`,
cache persistant du worktree `target/cargo-home-shared`, autres racines isolées
sous le lot, espace libre minimal 30 Gio. Arrêt sur échec selon AGENTS ; pas
d'extension de la dérogation historique accordée aux nouveaux tests A.

Le contrôle PE interne des harnais workers reste celui de la garde. Aucune
campagne PE supplémentaire pour le seul diff adapters/tests/documents ; le
bilan distingue ce périmètre des contrôles déjà intégrés aux appels N1.

La session principale conserve documents et Git. L'implémenteur Rust est seul
auteur des changements Rust/tests ; il préserve les travaux des autres. Chaque
candidat Na/Nb passe au coordinateur et au reviewer indépendant, deux cycles
revue/correction maximum par livrable. Diff de déplacement relu avant Nb.
Archive des petites preuves avec arborescence, nombre et SHA-256 par entrée
vérifiés ; caches partagés/persistants et ressources préservés. Six tâches,
deux rapports prévus ; aucun nouvel outillage ou contrôle distant.

## Exécution — référence C

Avant tout changement Rust et premier commit C : SelfTest 35 positifs / 79
négatifs ; Structure 931 tests / 931 entrées, 166 native-gated, zéro retour
silencieux ; format et Clippy workspace/all-targets réussis. N1 : défaut
685 réussites / 128 ignorés, common 11/1, CPU 12/9, Parakeet CPU 26/11,
soit **734 réussites, zéro échec, 149 ignorés**. Les cas ignorés ne sont pas
présentés comme exécutés. Journaux `target/parakeet-tests/refacto-poll-c/reference-*.log`.
Aucun changement de source suivi lors de la référence ; lecture F7 déléguée
sans édition ni autre Cargo concurrent. Le registre conserve ses 931 entrées.
