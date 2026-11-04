Avancée : cause dominante identifiée et corrigée ; correctif PASS, quatre Auto à 8,55–8,82 s contre 24,98–25,01 s avant.
Blocage : ancienne bimodalité de 15 s non reproduite, donc NON DÉTERMINÉE.
Décision : LIVRÉ AVEC RÉSERVES sur le diagnostic global ; quota 6/6 clos, commits locaux seulement.

# DIAG-CORR-PREPARE — 4 novembre 2026

Suite du [bilan figé précédent](DIAG-BANC-15S-BILAN.md), selon le
[plan accepté et preuves détaillées](../cadrage/diag-corr-prepare-plan.md).
Branche `diag-banc-15s` ; diagnostic `6b8c4000`, correctif `d7e6445a`.
Six Auto sans candidat → CPU, même MP3 **24 237 octets**, staging neuf et
ressources épinglées ; aucune modification de garde, profil ou machine dans ce lot.

| Phase, secondes | Avant `74ddac11` | Avant `611963c2` | Après `6d813e86` | Après `1b9928e1` | Après `6eca22a1` | Après `3ceb8159` |
|---|---:|---:|---:|---:|---:|---:|
| Corps Rust, libtest | 24,98 | 25,01 | 8,82 | 8,55 | 8,67 | 8,64 |
| Premier bind/catalogue | 23,6935 | 23,6519 | 7,5259 | 7,2626 | 7,3557 | 7,2794 |
| Dont lecture + SHA Turbo non demandé | 16,4424 | 16,5009 | **0** | **0** | **0** | **0** |
| Dont lecture + SHA Parakeet demandé | 7,2500 | 7,1499 | 7,5252 | 7,2620 | 7,3551 | 7,2788 |
| Préparation après bind : routage/source/archive | 0,0928 | 0,0932 | 0,1016 | 0,0914 | 0,0928 | 0,1026 |
| prepare_effect complet, interne produit | 23,7864 | 23,7453 | 7,6277 | 7,3541 | 7,4487 | 7,3821 |

Identifiants abrégés des dossiers `PersonalAutoFallback-*`, identifiants complets
dans le plan. Durées internes calculées dans une seule horloge monotone par
trace ; aucune soustraction entre horloges banc/produit. Corps ≠ Cargo ≠ garde/staging.
Premier Cargo après correction : 46,416 s, dont 36,01 s de compilation ; les
trois suivants : 9,427 /9,565 /9,485 s, garde complète environ 25 s.

**Verdict : PRODUIT, chemin d'inspection à cache froid, mesuré dans le banc debug.**
Avant, `bind_import` inspectait tous les modèles avant de résoudre la demande
Parakeet : **1 624 555 275 octets Turbo inutiles**, puis **713 975 456 octets Parakeet**.
Après, seul Parakeet est inspecté ; les quatre traces ont zéro événement Turbo,
une lecture complète Parakeet, identité stable et contrôles conservés.
L'attente du mutex est ≤0,0029 ms ; ce verrou n'explique pas le coût observé.
Le gain de corps observé est d'environ **16 s**, cohérent avec le travail retiré.
Ce gain **ne démontre pas la cause** des anciens écarts rapide/lent de même amplitude.
Les ~19 s annoncées face à l'application ne constituent pas une phase commune :
profil, compilation et état de cache diffèrent. Lecture/SHA ne sépare pas disque
et calcul ; aucun verdict Defender, CPU hybride ou garde, ni mesure de premier
lancement release en installation neuve. Environ 7,3 s de vérification Parakeet
restent dans ce banc ; aucune extrapolation de ce coût à l'application normale.

**Correction retenue :** inspection du seul modèle demandé pour l'import ;
coût faible, risque de sélection couvert par tests ; catalogue complet d'affichage
et live inchangés. Aucun affaiblissement taille/SHA/identité/revalidation.
TXT, SRT et WAV identiques par SHA-256 avant/après, assertions Auto inchangées.
N1 **712 PASS /128 ignorés /0 échec**, dont sept nouveaux tests depuis le lot
précédent ; format/build/Clippy PASS avec avertissements, compilation native
sans exécution PASS. Revue du code **CLEAN**, un cycle de correction utilisé ;
conclusions relues indépendamment **CLEAN**. Aucun autre scénario natif ni
validation dans la fenêtre Windows ; provenance DLL et messages ignore non traités.

Instrumentation conservée, opt-in et désactivée par défaut ; marqueurs internes
absents du produit compilé sans `native-tests`. Aucun autre correctif spéculatif
proposé. Nouvelle campagne nécessaire pour expliquer le mode lent historique,
sur décision distincte. Aucun push/fusion. Archive vérifiée : **1 147 entrées**,
nombre/tailles/SHA-256 comparés aux sources ; chemin et empreinte ZIP dans le plan.
Nettoyage proposé à l'utilisateur, **non exécuté** ; VCTIP 9264 subsiste et son
usage des racines n'est pas exclu. Ce bilan est figé à son commit de clôture.
