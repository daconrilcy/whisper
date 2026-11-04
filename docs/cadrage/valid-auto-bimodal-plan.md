Avancée : main fusionné et poussé à 4f13bb37 ; préalable satisfait.
Blocage : aucun au préflight initial ; espace libre 492 395 466 752 octets.
Décision : exécuter le lot autorisé, sans changement de code ni de configuration.

# VALID-AUTO-BIMODAL — plan du 4 novembre 2026

Objectif : vérifier si le mode lent historique subsiste après d7e6445a et,
s'il apparaît, localiser son coût avec les traces déjà livrées. Le correctif
et l'instrumentation sont ancêtres de main 4f13bb37 ; push origin/main réussi.
Worktree neuf `C:/wt/valid-auto`, branche `valid-auto-bimodal`, depuis ce main.
Demande du lot et « ok push puis continue » autorisent ce périmètre précis.
Début : 18:42 locale ; budget une heure, échéance 19:42 locale ; au plus douze
corps Auto N2, aucune autre recette native. Le plafond spécifique remplace le
budget générique de diagnostic pour cette campagne ; aucun essai supplémentaire.

1. Préparer seulement `target/n2-native/` dans ce worktree : copier les entrées
   épinglées et le cache Cargo isolé du lot précédent, sans écrire dans sa racine.
   Paramètres PersonalAutoFallback identiques, nouveaux chemins locaux uniquement,
   `harnessTrace=1`. Source MP3 24 237 octets, SHA-256
   39048CD1482D7BCCE4052EE8027CF629FCC2F7C6714DCF6419238EE202836D8A.
   Aucun modèle, DLL, worker, profil, garde, banc ou code produit modifié.
2. Préflight par garde native existante et auto-tests de la garde dans cette
   racine ; CARGO_TARGET_DIR partagé canonique inchangé. Pas de Cargo direct,
   réseau, changement de priorité ou configuration machine. Relever les processus
   pertinents ; aucun arrêt autorisé. Les reçus gardent toutes les racines et
   variables canoniques ; les environnements sont isolés par la garde.
3. Lancer PersonalAutoFallback séquentiellement, au plus douze fois. Staging
   neuf à chaque essai, même état et ressources. Arrêter dès un corps >=12 s
   avec traces complètes, ou au budget/quota. Une violation de garde, un test
   en échec ou des traces inexploitables interrompent la suite, sans correction
   ni nouvelle exécution improvisée. Ne pas modifier les fichiers suivis pendant
   les essais. stdout/stderr/reçus et JSONL existants sont conservés.
4. Pour chaque essai, lire corps libtest, Cargo et garde séparément ; compter
   lectures complètes ET inspections/cache par modèle, avec durées lecture/SHA,
   identité, thread et phases. Vérifier unicité de trace/origine monotone,
   paires début/fin et résultats. Les traces existantes n'enregistrent pas de
   pile d'appels : n'attribuer un appelant que si phase/thread/code le prouvent,
   sinon NON DÉTERMINÉ. Aucun nouveau marqueur ni mélange d'horloges.
5. Lire les chemins de catalogue encore susceptibles d'inspecter Turbo ;
   distinguer atteignabilité dans ce banc, catalogue d'affichage et live.
   Si lent localisé, proposer au plus deux corrections sans les appliquer.
   Sinon conclure seulement NON OBSERVÉ SUR N ESSAIS. Pas de moyenne ou médiane
   globale, pas de preuve d'absence générale du mode lent.
6. Confier les conclusions à un reviewer indépendant en lecture seule, deux
   cycles revue/correction documentaire maximum. La session principale rédige
   `docs/architecture/VALID-AUTO-BIMODAL-BILAN.md`, une page figée à la clôture,
   avec tableau des essais et citations fichier:ligne. Signaler seulement la
   provenance DLL spike02, les ignore décalés et le coût SHA Parakeet restant.
7. Archiver les petites preuves hors de la racine supprimable, sous
   `target/valid-auto-bimodal/archive/`, arborescence préservée et comparaison
   nombre/tailles/SHA-256 de chaque entrée aux sources. Exclure runtimes/modèles
   recopiés et cache. Proposer les commandes de nettoyage, ne rien supprimer.
   Préserver caches partagés/persistants et les ressources/proofs historiques,
   notamment target/parakeet-feasibility et target/poll-preserved.
8. Committer localement le bilan et ce plan ; aucun nouveau push ou fusion de
   cette branche de validation implicite dans l'accord de push de main.

Le reviewer indépendant est également chargé de la lecture des chemins Turbo.
Pas de Rust à implémenter, pas de nouveau test à écrire ni campagne N1 à répéter
pour une modification documentaire seule. Toute limite de la trace sera rendue
explicitement ; le but est une observation attribuable, pas un verdict forcé.

## Exécution et clôture

À renseigner après les observations ; aucune exécution native avant ce plan.
