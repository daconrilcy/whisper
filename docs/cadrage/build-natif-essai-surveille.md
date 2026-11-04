Avancée : correctif d'acquisition et garde CPU/GPU livrés ; les trois sources sont récupérées et vérifiées.
Blocage : premier bootstrap CPU arrêté ; correction relue, relance suspendue après violation observée.
Décision : NON REPRODUIT ; demander l'arbitrage de reprise, sans fusion ni suppression.

# BUILD-NATIF — bilan figé du premier essai surveillé, 4 novembre 2026

Accord reçu pour corriger l'acquisition et étendre les trois scripts/profils. Plan : `144dd921`, candidat initial : `05f0ce25`. Responsabilité des modifications : session principale, aucun changement Rust. La règle de production par le script et de conservation du spike est ajoutée dans AGENTS.md comme demandé initialement.

| Étape | Résultat et preuve |
| --- | --- |
| Revue du delta acquisition/garde | CLEAN, premier cycle indépendant |
| Préparation sources2 | PASS sous garde ; NeMo, llama et vcpkg aux trois pins connus ; 19 patches identifiés |
| Auto-tests initiaux et corrigés | PASS : 7 négatifs du script, profils/chemins/faux parent, arrêt sur événements interdits, témoin réel de surveillance et dossier courant borné |
| Essai cpu1 | FAIL avant CMake ; bootstrap vcpkg lancé à 19:42 UTC, `powershell.exe` introuvable dans son PATH |
| Surveillance | Created `C:/wt/build-natif/$null` à **19:42:40.0168553 UTC**, détection à 19:42:40.0173942 ; arbre interrompu |
| Vérification installation | 14/14 SHA-256 inchangés ; équivalences statique/fonctionnelle, GPU, N1 et N2 NON EXÉCUTÉS |

Le fichier `$null` est absent à la lecture après arrêt ; aucun effacement agent. L'événement n'est pas attribué au bootstrap sur sa seule proximité temporelle. Ne pas considérer le défaut comme résolu par le seul changement de dossier courant.

**Correction préparée sans relance :** appel du script vcpkg épinglé par Windows PowerShell explicitement résolu ; dossier courant du processus enfant validé sous l'exécution, avec refus des liens et d'un emplacement extérieur. Parseur, CheckOnly CPU/GPU, script SelfTest et garde SelfTest passent. Deuxième revue indépendante : **CLEAN pour les corrections**, sans validation du bootstrap corrigé ou de CMake ; deux cycles consommés sur ce delta. Aucun contournement de garde ni modification des options CMake.

**Traces archivées :** [guarded-attempt.zip](../architecture/raw/native-build/guarded-attempt.zip), SHA-256 `1ebaa99b0aedbc42e30a519781cc3ca828cd9e722a8523d9cface436a79753c0` ; 37 fichiers, tailles/SHA vérifiés, plus manifeste (38 entrées). Elle contient les deux auto-tests, sources2 et cpu1 avec leurs journaux de garde. Espace libre après archivage : 531 101 315 072 octets. Aucun nettoyage exécuté ni nécessaire pour reprendre ; caches persistants, sorties d'échec et chemins protégés conservés. L'archive spike02 de phase 0 reste impossible.

Revue indépendante des conclusions et de l'archive : **CLEAN documentaire**, sans nouvelle revue du code ni levée de l'arrêt.

**Décision demandée :** recommander la relance du bootstrap corrigé dans une nouvelle sortie `cpu2`, sous la garde, avec arrêt sur tout nouvel échec ou événement interdit ; puis les phases statiques/fonctionnelles du plan si l'essai passe. Alternative : conserver la campagne suspendue. Ce bilan n'autorise pas la reprise. Pas de fusion recommandée en l'état, aucune levée de conservation du spike. Un fait ultérieur sera consigné séparément.
