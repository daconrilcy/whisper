Avancée : quatre builds achevés (deux CPU, deux GPU) ; CPU qualifié sur EN/FR.
Blocage : ggml-cuda.dll diffère entre builds ; reproductibilité GPU refusée, N2 GPU non exécuté.
Décision : CPU QUALIFIÉ ; GPU NON QUALIFIÉ. STOP conservé, aucun nouvel essai ni installation.

# BUILD-NATIF-QUALIF — complément de clôture du 6 novembre 2026

Reprise demandée après le [bilan initial figé](BUILD-NATIF-QUALIF-BILAN.md), base `946765cf`, branche `build-natif-qualif`, worktree dédié. [Plan et corrections](../cadrage/plan-build-natif-qualif-reprise1.md). Revues indépendantes des correctifs CLEAN : renommages complets, mesures/erreurs MemWatch bornées, validation CUDA Windows/x64/8. Options natives et outils épinglés conservés.

| Critère | CPU r2 | GPU r4 |
|---|---|---|
| a — surfaces PE / référence installée | PASS 5/5 | PASS 9/9 |
| c — deux builds au même chemin, SHA-256 | PASS 5/5 identiques | **FAIL 8/9 identiques** |
| Reference ×2 déterministe, TXT/SRT/WAV EN/FR | PASS 6/6 | NON EXÉCUTÉ |
| b — Reference / Rebuilt, mêmes six sorties | PASS 6/6 | NON EXÉCUTÉ après c |
| Verdict | **QUALIFIÉ** | **NON QUALIFIÉ** |

**Écart GPU caractérisé :** seul `ggml-cuda.dll` varie : **33 210 368 → 33 210 880 octets**, soit +512. SHA-256 `A7BB922B…EBD3` / `6BEC3FDE…15DE` (valeurs complètes dans la preuve). Comparaison positionnelle : **11 496 801 différences**, dont `.nv_fatb` 11 480 984, `.text` 11 158, `.rdata` 86, `.pdata` 102 et en-têtes 11 ; autres écarts associés aux sections décalées/relocations. Ce comptage inclut les effets de décalage, pas seulement des mutations sémantiques. Surfaces PE toujours identiques. **Cause non établie ; signal matériel possible conservé comme tel**, aucune conclusion logicielle ou matérielle et aucun nouvel essai. STOP avant Reference GPU/Rebuilt.

| Étapes (ordre de chaque groupe) | WHEA avant → après | Pic engagé observé |
|---|---|---:|
| r1 CPU1 interrompu (délai, origine inconnue) | 1→1 | 19,97 % |
| r2 CPU1 / CPU2 | 1→1 / 1→1 | 20,10 / 20,20 % |
| r2 Reference1 / Reference2 / Rebuilt CPU | 1→1 / 1→1 / 1→1 | 20,20 / 19,30 / 19,30 % |
| r2 GPU1 / r3 GPU1 interrompus (contrôles) | 1→1 / 1→1 | 19,80 / 20,20 % |
| r4 GPU1 / GPU2 / comparaison | 1→1 / 1→1 / 1→1 | **21,00** / 20,90 / 20,70 % |

Aucune nouvelle WHEA ni ICE observé ; dernier événement Système 527992 du 05/11. MemWatch et contrôle WHEA directs actifs pendant les étapes et l’archivage. Quatre builds finis, trois tentatives de reprise interrompues. Seules jonctions CPU/GPU autorisées retirées, cibles et anciens workspaces préservés.

**Preuves :** [dossier et rétention](C:/wb/native-qualif/reprise-4/RETENTION.md), `reprise-2/cpu/{surface,repeat,reference-determinism,equivalence}.json`, `reprise-4/gpu/{surface,repeat}.json`, reçus de copies et `all-stage-summary.json`. **14 DLL installées inchangées**, aucun outil natif restant. Aucun merge/push/nettoyage de clôture ; **346,57 Gio libres** au relevé. Un test de l’ancien sérialiseur a produit un intermédiaire CLIXML de **4,75 Gio** : contenu non lu/haché/copié, suppression humaine proposée ; capture brute remplacée par une projection bornée.

**Dettes :** origine de l’écart GPU à établir avant reprise ; validation fonctionnelle des DLL GPU installées non établie par ce lot. `GGML_NATIVE=ON` conservé, configuration consignée (backend CPU AVX2/FMA/F16C, CUDA 61) ; portabilité hors lot. Tout remplacement installé exige une décision humaine distincte.
