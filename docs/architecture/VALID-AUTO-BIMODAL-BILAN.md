Avancée : 12 Auto PASS après correctif, corps 8,45–8,72 s ; aucune inspection Turbo observée.
Blocage : aucune preuve de la cause des anciens essais lents ; leur disparition générale n'est pas établie.
Décision : NON OBSERVÉ SUR 12 ESSAIS ; quota clos, aucune correction proposée ni appliquée.

# VALID-AUTO-BIMODAL — 4 novembre 2026

Base `main=4f13bb37`, fusion et push vérifiés ; correctif `d7e6445a` inclus.
Worktree `C:/wt/valid-auto`, branche `valid-auto-bimodal`, aucun code modifié.
PersonalAutoFallback seul, instrumentation existante activée, staging neuf,
même MP3 **24 237 octets** et runtime épinglés. Arrêt prévu si corps ≥12 s.

| Essai : suffixe unique de PersonalAutoFallback-* | Corps Rust, s | Inspections Turbo / Parakeet | Lecture/SHA Parakeet, s |
|---|---:|---:|---:|
| b21b00a14d6d499797d1147f1316707f | 8,54 | 0 /5 | 7,2583 |
| 1340293e6fc244489e536531954b5016 | 8,59 | 0 /5 | 7,2756 |
| 9295ae32f99e457c8bec796dc35d1a0c | 8,57 | 0 /5 | 7,2734 |
| a92d4ad0aad441e59861b0869bdcaf24 | 8,72 | 0 /5 | 7,3323 |
| c2884c7232934e2495067ce1e77ad010 | 8,51 | 0 /5 | 7,1771 |
| de907bd6a2c746f4ab955c103161785b | 8,54 | 0 /5 | 7,2028 |
| e686c193e24a48eba02b0ff7a90d04ac | 8,58 | 0 /5 | 7,2771 |
| 313b0f064bdf47458116d679798a5d48 | 8,69 | 0 /5 | 7,3716 |
| 4d3b3075e97d42eaabac6a58bd2abeb2 | 8,66 | 0 /5 | 7,3174 |
| d937d7830b0342bdaa8b088b7a64823b | 8,71 | 0 /5 | 7,3779 |
| 082a31cd366b4eba80630a91ae4f21e1 | 8,45 | 0 /5 | 7,1486 |
| d1233cc74d754d849a17d996f78c5a0e | 8,59 | 0 /5 | 7,2754 |

Chaque essai : **une lecture complète de 713 975 456 octets et quatre hits cache**,
identité stable, un PID et un thread d'inspection, trace complète monotone.
Les cinq appelants, dans l'ordre, sont : **A** premier bind → catalogue ciblé ;
**B** routage CPU → qualification_key ; **C** routage CPU → verify_qualification ;
**D** préparation archive ; **E** démarrage worker → second bind. A lit, B–E
utilisent le cache. Attribution par phases/thread et appels synchrones du code,
pas par pile enregistrée ; [citations et preuves](../cadrage/valid-auto-bimodal-plan.md).
Le plus grand intervalle produit est toujours lookup → lecture/SHA Parakeet,
**7,149–7,378 s** ; aucun trou supplémentaire d'environ 15 s observé.
TXT/SRT/WAV identiques par SHA-256 sur les douze essais. Aucune moyenne globale.

**Chemins qui inspectent encore Turbo, hors du parcours direct de ce banc :**
préfixe des chemins ci-dessous : `crates/whisper-adapters/src/` sauf desktop.
`worker_ipc/backends/mod.rs:115` → `model_catalog.rs:344` inspecte tous les modèles.
Appelants : file `worker_ipc/runtime/queue.rs:200`, reprise
`worker_ipc/runtime/recovery.rs:19`, live `worker_ipc/backends/mod.rs:229,239,273,337`.
Le catalogue initial UI est inspecté dans `crates/whisper-desktop/src/root.rs:61`.
Ces chemins peuvent inspecter Turbo ; inspection ne signifie pas recalcul SHA
si son cache est valide. Le banc StartImport n'exécute ni UI, file, reprise ni
live. Un cache global verrouillé pendant la lecture pourrait faire attendre un
autre thread ; **aucune telle course ni invalidation observée ici**. L'hypothèse
« second SHA Turbo expliquant l'ancien mode lent » reste non prouvée.

Corps ≠ Cargo ≠ garde/staging : premier Cargo **40,730 s**, dont compilation
**31,20 s** ; suivants **9,310–9,575 s** ; garde complète **24,785–25,117 s**
après le premier. Auto-tests de garde, préflight et douze assertions natives PASS.
Revue indépendante des conclusions : **CLEAN**, aucun constat ouvert. Aucune validation
UI/GPU ni modification machine. Provenance DLL spike02, messages ignore décalés
et coût SHA Parakeet à chaque nouveau processus signalés, non traités.
Archive vérifiée **445 entrées** ; empreinte et nettoyage utilisateur dans le
plan. Aucune suppression ; VCTIP 103004 subsiste, usage des racines non exclu.
Ce bilan est figé à son commit de clôture ; aucune fusion ni push de ce lot.
