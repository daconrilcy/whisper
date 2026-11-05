Avancée : cpu7c4 PASS sous garde ; cinq surfaces PE conformes et ggml_commit épinglé confirmé.
Blocage : cpu8 échoue dans Protobuf sur un crash interne MSVC C1001 ; plafond de quatre compilations CPU atteint.
Décision : arrêt sans relance ; répétabilité et équivalence fonctionnelle non démontrées.

# Reprise CPU — 5 novembre 2026

Reprise autorisée par le point d'entrée public absolu, budget **13:43:57–14:43:57 Paris**, candidat `4bd72071`. Aucun code, pin, version, ISA ou règle de garde modifié. Le [correctif de garde](build-natif-control-qualification-bilan.md) reste qualifié ; les DLL ne le sont pas.

| Essai | Résultat réellement observé |
| --- | --- |
| cpu7c4 | Garde PASS, 13:44:00–13:53:09 Paris (9 min 08,9 s hors préparation). Cinq DLL, flags PASS, zéro C4530 ; commit llama épinglé décodé. |
| cpu8 | FAIL, 13:56:10–14:00:38 Paris (4 min 28,5 s hors préparation). Protobuf hôte Release échoué ; aucune DLL finale. |

| Critère de clôture | CPU | GPU |
| --- | --- | --- |
| a — exports nom+ordinal, imports DLL+symboles, CPU sans CUDA | **PASS pour cpu7c4 : 5/5** | NON EXÉCUTÉ |
| b — sorties TXT/SRT/WAV identiques | NON EXÉCUTÉ | NON EXÉCUTÉ |
| c — deux builds neufs donnent les mêmes SHA-256 | **FAIL : deuxième build échoué**, aucune différence binaire mesurée | NON EXÉCUTÉ |

**Statique cpu7c4.** Quatre tailles identiques ; NeMo : **−1 536 octets (−0,0366 %)**. Cinq SHA installés inchangés. Imports directs/différés et exports nom+ordinal lus sans chargement ; cibles de forwarding non comparées. `ggml_commit = bd4f514db14d87fded667787a7a963bfbaa98e89` confirmé.

**Cause immédiate cpu8.** C1001 sur `message_lite.h:1435` en compilant `any_lite.cc`, sortie **0xC0000005**. Windows confirme le crash dans **c2.dll 19.44.35229.0** à 14:00:33 Paris. Commande et SHA des deux fichiers identiques au build réussi ; autres entrées/conditions non intégralement comparées. **Cause profonde inconnue**, aucun lien causal établi avec télémétrie/garde ; zéro violation de garde enregistrée. Ninja vcpkg interne observé : 1.13.2 ; Ninja principal : 1.12.1.

**Verdict : NON ÉQUIVALENT — qualification incomplète**, sans écart fonctionnel observé. **4/4 CPU commencées** : cpu7, cpu7c2, cpu7c4, cpu8 ; cpu7c3 refusé avant compilateur. GPU 0/2, N2 0/10 ; N1 non exécuté, Rust/Cargo identiques à main en lecture. Après échec : lecture/diagnostic statique seulement.

**Décision recommandée.** Pas de fusion au titre d'ÉQUIVALENT ET REPRODUCTIBLE. Fusion de protection des scripts/contrôles/garde envisageable avec statut DLL non qualifié ; aucune fusion automatique. Aucun remplacement installé ni suppression du spike recommandé. Suite : diagnostic MSVC ciblé et nouveau quota avant reprise. GGML_NATIVE/portabilité reste hors périmètre.

**Archives vérifiées** : [cpu7c4.zip](../architecture/raw/build-natif-cpu7c4-cpu8/cpu7c4.zip), 105 fichiers + manifeste, SHA `7da28720f405ff53fa32670c88f5e0400bc2d04c09a71b5f7be72958052a4f5c` ; [cpu8-stop.zip](../architecture/raw/build-natif-cpu7c4-cpu8/cpu8-stop.zip), 66 + manifeste, SHA `78653f3f126463e322bf33874ec94826966fc61b7229f91d9672cda693cbcf15`. ZIP rouverts : noms, nombres, tailles et SHA contrôlés. DLL/journaux/caches utiles/crash conservés, objets/PDB exclus. **Revue indépendante finale : CLEAN documentaire et archivage.** Aucun nettoyage ; espaces conservés pour diagnostic.
