Avancée : R5-1 est CLEAN et commité ; la clé du worker successeur est connue et les chemins d'admission R5-2 sont identifiés.
Blocage : la garde d'exécution demandée place toute racine PARAKEET_* sous `r5-2`, alors que le worker exact `e767…` exige la racine canonique `l2b` ; A4/A5 ne peuvent démarrer avec les deux conditions simultanées.
Décision : PLAN_CHANGE_REQUIRED sur cette seule incompatibilité de racine ; aucune implémentation ni commande Cargo R5-2 avant arbitrage écrit et revue du plan corrigé.

# PLAN-PARA-L2b-R5-2 v1 — admission en recette du successeur Test

**Statut : DRAFT — PLAN_CHANGE_REQUIRED, revue indépendante demandée.** Base `wip/l2b-candidate` au commit R5-1 `76e00f446c48647f6aaa5e1e1d42dd89695ae699`; plan parent R5 v2 `0b41d984`, R5-1 CLEAN. Décision utilisateur du 16/10/2026 : clé successeur worker `e7672a520db87926b3da9b8e5b585a1e7e57ce53ff8b1d0fe95251046aa01f4d`, modèle `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`, DLL CPU `ggml-base` `398cf89332451b547b44256d5e84bb5895a7c0fe97727e597be8989230544e0a`, `ggml-cpu` `f7546413e1e6e248df0bc8a0745a414de4c36532b201b3ba219d86864b5f1ab0`, `ggml` `2eff3c476a9796580ec788429c87e479dc33e1066c74a77cbe3a7a909734b794`, `nemo_speech_asr_c` `9d67b716fe53accf05c8a1bb43fcac0a83f22c8425641f432ce1ed9dc871e764`, `nemo_speech_asr` `923a3a5258b196de6122e6ccc9b5cdd3e48c379dc53461e38921826b75f43019`. Toutes les clés de ce plan sont limitées au profil **Test CPU** ; R4 continue de refuser le worker court `aa8df…`.

## 1. Contradiction de préflight à arbitrer

Le binaire exact R5-1 `e767…` appelle `required_runtime_root` (`crates/whisper-worker-parakeet-cpu/src/main.rs:436–454`), qui canonicalise `PARAKEET_L2_RUNTIME_ROOT` puis l'accepte uniquement s'il égale une des racines `l2a-1`, `l2a-2`, `l2b` (`main.rs:519–535`). Le profil VAD exige même exactement `l2b` (`main.rs:507–517`). Une valeur sous `target/parakeet-tests/l2/l2b/r5-2/` échoue avec `RuntimeRootMismatch` **avant Ready et avant toute inférence**. Une valeur égale à `target/parakeet-tests/l2/l2b/` fait fonctionner le binaire, mais échoue à la garde utilisateur « une racine `PARAKEET_*` sous R5-2 avant Cargo ». Modifier `main.rs` demanderait un chemin hors des six chemins autorisés et changerait le SHA du worker, donc la clé successeur fixée.

**Décision produit/périmètre recommandée :** autoriser explicitement `PARAKEET_L2_RUNTIME_ROOT` comme **ancre canonique en lecture** égale à `target/parakeet-tests/l2/l2b/`, tout en exigeant que chaque destination d'écriture (`CARGO_TARGET_DIR`, `CARGO_HOME`, `TEMP`, `TMP`, `PARAKEET_L2_ARCHIVE_ROOT`, `PARAKEET_L2_OUTPUT_DIR`, binaires/DLL copiés, logs et fixtures générées) soit résolue sous `target/parakeet-tests/l2/l2b/r5-2/`. Les chemins de modèle et d'audio sont des entrées en lecture. Aucun test existant qui écrit directement sous l'ancre `l2b` ne sera lancé. **Cette exception n'est pas encore autorisée** ; la condition actuelle reste bloquante. L'autre option serait d'autoriser `main.rs` et de qualifier un nouveau hash worker dans un plan révisé, ce qui abandonnerait la clé `e767…` décidée ici.

## 2. Chemins et mécanisme proposés après arbitrage

**Chemins d'écriture maximum accordés, sans extension implicite :**

1. `crates/whisper-adapters/src/archive.rs` — clés exactes, admission temporaire, retrait et validation des pending.
2. `crates/whisper-adapters/tests/parakeet_archive.rs` — cas positif/négatif, conservation des pending et recette complète A4 si le test existant est réutilisé.
3. `crates/whisper-adapters/src/worker_ipc.rs` — uniquement la garde de clé VAD demandée ; le reste du parcours et Turbo demeurent inchangés.
4. `crates/whisper-adapters/tests/parakeet_import.rs` — tests d'admission et de route nécessaires.
5. `crates/whisper-core/src/model_registry.rs` — fiche Test et clé successeur uniquement ; aucune capacité Turbo nouvelle.
6. `docs/cadrage/fiche-parakeet-r5-2.md` — statut de la recette, SHAs, A4/A5/B9 et réserves, créé après preuves.

Les tests longs existants de `tests/parakeet_long.rs` sont **en lecture seule** : ils créent `runtime.join("archive/product-vad-…")`, donc hors R5-2 si l'ancre vaut `l2b`. Ne pas les exécuter tels quels. Une recette B5/B6 confinée doit être construite à partir des interfaces existantes dans les seuls fichiers de test ci-dessus, ou s'arrêter `PLAN_CHANGE_REQUIRED` si la couverture demanderait un septième chemin. B9 peut utiliser les fichiers bruités en lecture et rapporter ses résultats sous R5-2. B8 reste INACTIVE côté produit, même si la clé VAD Test permet la recette.

## 3. Lots de tâches et cartes

| Tâche | Modification / vérification | Carte |
| --- | --- | --- |
| T1 | Faire échouer la garde PowerShell en mode fermé sur un chemin volontairement hors R5-2 **avant le premier Cargo** ; vérifier ensuite toutes les racines absolues, les SHAs, la branche et l'absence d'écriture source inattendue. Si la valeur de `PARAKEET_L2_RUNTIME_ROOT` ne respecte pas la décision écrite à venir, **aucun Cargo**. | Gate P0 |
| T2 | Retirer l'admission nouvelle L2a historique ; les pending L2a existants restent physiquement intacts mais le scan/reprise indique `ModelNotQualified` ou cause explicite, sans suppression ni nouveau pending. Maintenir le refus R4 du worker `aa8df…`. | A6, historique L2a |
| T3 | Autoriser en **recette Test seulement** le tuple exact modèle/runtime/worker `e767…` en single-pass, sans le déclarer Qualifié Test. Isoler la voie de recette de la publication finale et tester chaque divergence de hash/profil/backend. | A4 préliminaire |
| T4 | Exécuter fr87, fr27 et en15 par l'application, avec TXT/SRT et zéro `InvalidTimestamp`, contrôle `faire?`/`Je`, UTF-8 final `à...`, sorties et SHAs. Si A4 FAIL, retirer l'admission de recette **dans le même lot**, vérifier le refus avant pending et conserver les données. Si PASS, inscrire la nouvelle fiche explicite et le statut Qualifié Test de la clé. | A4 |
| T5 | Pour `import-vad`, admettre le **même hash worker** en Test uniquement dans `archive.rs` et la garde VAD de `worker_ipc.rs`, sans activer B8. Ne pas confondre les pending VAD et single-pass. | A5 préliminaire |
| T6 | Rejouer B5 et B6 sur ce binaire et ces DLL, avec état PASS/FAIL attribué ; B9 séparément à −40, −30 et −20 dBFS : compter silences admissibles, coupures ou erreur pour chaque niveau et présenter le rapport avant toute décision B8. | A5 et B9 |
| T7 | Régressions Turbo, L2a ancien pending, R4, identités de clés, scan/reprise et format/build offline/locked ; revue indépendante, deux cycles revue/correction maximum. | A6, verdict |

**Ordre impératif :** P0 → T2/T3 → A4 réel → promotion ou retrait de la clé courte → T5/T6 sur le même binaire → A6/revue → commit local seulement si CLEAN. Une réussite worker A3 ne vaut pas A4 applicatif. Si A4 FAIL, A5 ne bénéficie d'aucune qualification implicite. Si B5/B6 restent FAIL ou INCONCLUSIVE, B8 reste OFF ; B9 est informative et ne bloque pas B8 à elle seule, mais son rapport précède toute décision de l'utilisateur.

## 4. Exécution, garde et livrable

Après l'arbitrage de la section 1, la garde PowerShell doit d'abord résoudre la racine R5-2 et comparer les chemins canoniques avec séparateur de répertoire (pas un simple préfixe texte). Elle vérifie `CARGO_TARGET_DIR`, `TEMP`, `TMP`, `CARGO_HOME` et toute variable `PARAKEET_*` désignant une **destination ou racine d'écriture** ; elle vérifie à part l'ancre `PARAKEET_L2_RUNTIME_ROOT` selon la nouvelle décision, si elle est accordée. Un essai négatif avec `CARGO_TARGET_DIR` extérieur doit terminer avant la ligne Cargo et laisser le témoin de non-exécution. Tout échec de résolution ou d'identité termine le script avec code non nul. Ne jamais poursuivre après une erreur PowerShell ; aucune commande Cargo ne précède ce test.

Sous l'accord actuel strict, cette garde doit donc **refuser** l'environnement indispensable au worker : P0 = FAIL, A4/A5/B9 = NOT RUN et aucune écriture produit permise. Les données de R5-1 hors racine sont seulement inventoriées : 279 fichiers de build modifiés dans `target/debug`, 57 fichiers temporellement associés sous `%TEMP%` dont les sorties `whisper-live-retry`, `whisper-import-retry`, `whisper-stop-no-retry`, et `.cargo/.global-cache` actualisé ; aucun nettoyage. `models/`, `sound example/`, code C++, Cargo.toml/Cargo.lock, réseau, push et activation B8 restent interdits.

Le rapport final donne le diff borné, les tests réellement exécutés, le SHA du worker/modèle/cinq DLL **chargés**, les chemins TXT/SRT A4, A4 PASS/FAIL, A5 B5/B6/B9 séparés, A6 et le verdict du reviewer. La fiche Test et le commit sont produits seulement selon le résultat réel A4 et après CLEAN. **Ce DRAFT ne déclenche pas l'autorisation conditionnelle d'implémentation : sa condition de préflight n'est pas satisfaisable sans une nouvelle décision.**
