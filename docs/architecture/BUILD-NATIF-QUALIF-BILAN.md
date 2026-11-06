Avancée : chaîne préparée ; une tentative CPU interrompue ; 14 DLL installées vérifiées inchangées.
Blocage : WatcherError de la garde pendant les dépendances, avant toute DLL finale et toute série N2.
Décision : CPU NON QUALIFIÉ ; GPU NON QUALIFIÉ (non exécuté, condition CPU non satisfaite). Aucun nouvel essai.

# BUILD-NATIF-QUALIF — bilan de clôture du 6 novembre 2026

Base `946765cf`, branche `build-natif-qualif`, worktree `C:/wt/build-natif-qualif`. [Plan](../cadrage/plan-build-natif-qualif.md) relu CLEAN. Deux cycles de correction de l’outillage consommés ; livraison avec réserves. Quatre builds prévus ; **une tentative CPU**, interrompue pendant vcpkg/Abseil, **zéro build achevé**, zéro run fonctionnel N2. Aucun merge, push ou remplacement installé.

| Variante | a — surfaces PE | c — deux builds identiques | b — équivalence N2 / déterminisme Reference×2 | Verdict |
|---|---|---|---|---|
| CPU | NON EXÉCUTÉ : aucune DLL finale | NON EXÉCUTÉ : build 1 interrompu | NON EXÉCUTÉ / NON ÉTABLI | NON QUALIFIÉ |
| GPU | NON EXÉCUTÉ | NON EXÉCUTÉ | NON EXÉCUTÉ / NON ÉTABLI | NON QUALIFIÉ |

**Arrêt à 17:11:14,832 Paris :** `Incomplete, unordered or ambiguous rename at drain boundary.` Sous `C:/wb/native/cpu/localappdata/Microsoft/VSApplicationInsights/`, quatre notifications de même identité décrivent `.tmp → .trn → autre .trn` dans deux buffers du drain 448. La garde les refuse et termine la campagne. Origine non attribuée ; aucune conclusion matérielle. Aucun ICE observé, aucune nouvelle WHEA. Le manifeste enfant reste `STARTED/CONFIGURE` après interruption ; `guard-result.json` et le fichier STOP établissent l’échec. Aucune différence binaire n’a pu être mesurée.

| Étape surveillée | Résultat | WHEA avant → après | Pic engagé observé |
|---|---|---|---|
| Auto-tests voie native | PASS | 1 → 1 | 19,23 % |
| Acquisition sources | PASS | 1 → 1 | 19,22 % |
| Auto-test N2 initial | FAIL, ancien oracle « 3 cas » | 1 → 1 | 19,11 % |
| Auto-test N2 après correction/reprise autorisée | PASS | 1 → 1 | 19,17 % |
| CPU 1 | ARRÊT garde | 1 → 1 | **19,49 %** |

Dernier événement Système : `527992`, ID19 du 05/11, APIC24. MemWatch intensif et contrôle direct WHEA actifs pendant les étapes et l’archivage. BIOS F9 vérifié ; suppression de l’undervolt et tests matériels verts : déclaration humaine. Outils et SHA vcpkg-tool épinglés vérifiés. Jonction historique retirée seule avec accord précis, cible préservée ; ancien workspace conservé séparément. Après arrêt : aucun outil natif observé, **419,44 Gio libres**.

**Preuves :** [dossier du lot](C:/wb/native-qualif), `steps/*/step.json`, `steps/Cpu1/guard/*/guard-events.jsonl`, `cpu/qualif-cpu1/manifest.json`, reçus de copies nombre/SHA-256, `installed-final-check.json`. Premier échec N2 et STOP conservés sous leurs noms `failed-1`. Référence N2 conforme aux 21 fichiers épinglés ; fixtures EN/FR existantes.

**Rétention et dettes :** [inventaire et commandes humaines](C:/wb/native-qualif/RETENTION.md), aucune suppression de clôture exécutée ; liens et workspace historique signalés. Renommage de la garde à corriger/qualifier avant décision de reprise. `GGML_NATIVE=ON` demandé, instructions effectives non enregistrées (configuration interrompue), portabilité non qualifiée. Remplacement des DLL installées : décision humaine distincte ; leur validité fonctionnelle après undervolt demeure non établie par ce lot.

Complément ultérieur : [reprise, builds achevés et verdicts CPU/GPU](BUILD-NATIF-QUALIF-REPRISE1.md).

[Clôture après correction du PTX et qualification GPU](BUILD-NATIF-QUALIF-PTX.md).
