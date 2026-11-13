Avancée : deuxième lot proposé sur deux tests d’affichage de documents et d’attente de sessions.
Blocage : aucune modification Rust de ce lot avant acceptation de son plan et de ses chemins.
Décision : extraire les préparations et vérifications répétées des deux tests privés, sans changer leurs scénarios.

# Plan — structure des tests de présentation documentaire

## Header

- **Source audit :** Clippy frais à six crates sur le candidat P1-T1 dans `C:/wt/cs`, journal `C:/wt/cs/target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261113T191725331-e14066767cb24ec796c184a6dcb01d40/cargo-command.stdout.log`, SHA-256 `A62941AA8FA2A6333947AB3EDC7EAEB0E9E8945FA856DD350D476B50D126E679`.
- **Date de planification :** 13 novembre 2026.
- **Workspace cible :** crate `crates/whisper-desktop` dans le worktree court `C:/wt/cs`.
- **Mode :** plan uniquement, aucune modification de code pour ce lot.
- **Confiance :** high — les deux sites et l’inclusion du fichier ont été confirmés dans le journal et les sources actuelles.
- **Contrôle qualité du plan :** passed.

## 1. Résumé de l’audit

Après le lot P1-T1, le Clippy gardé compte 137 sites structurels uniques contre 168 dans la baseline, sans nouveau site. Les diagnostics restent `too_many_lines` et `cognitive_complexity`. Le présent lot vise les deux prochains sites `too_many_lines` les plus courts dans des tests privés du desktop : `unified_ui_renders_one_bounded_page_preview_parts_outputs_and_wait_state` (105 lignes) et `unified_wait_tracking_prunes_finished_evicted_sessions_and_keeps_selected_wait` (111 lignes), dans `src/ui/tests/document_presentation.rs`.

Le premier scénario vérifie l’affichage du texte paginé, de l’aperçu, des fichiers de sortie et de l’état d’attente. Le second vérifie l’éviction des sessions terminées et la conservation du suivi d’attente de la session sélectionnée. Le cliquet et les validations N1 du lot P1 passent. Aucun de ces dépassements de seuil ne démontre un défaut produit.

Les 135 autres sites du relevé à 137 sont différés à des lots futurs, chacun avec son plan accepté et ses chemins exacts.

## 2. Vérification de fraîcheur

- Inspectés : journal Clippy frais et SHA ci-dessus, `crates/whisper-desktop/AGENTS.md`, `src/ui/tests.rs`, `src/ui/tests/document_presentation.rs`, `ci/guard-level1.ps1` et le plan accepté P1-T1.
- `tests.rs` inclut le fichier ciblé avec la convention locale `include!("tests/document_presentation.rs")`.
- Les deux fonctions existent toujours à leurs sites Clippy. Le fichier reste séparé du code de production et conserve les tests privés de l’UI.
- L’état testé comprend P1-T1 non commité mais revu `CLEAN`; ses trois fichiers restent hors du présent lot. Le nouveau plan doit préserver cet état sans les modifier.
- Le profil exact Clippy à six membres, `fmt --check`, la N1 par défaut et les sélections worker ont passé sous la garde dans `C:/wt/cs`. Le chemin court a permis à ces commandes de réussir; les preuves de ces campagnes restent dans `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/`.
- L’ancien worktree P1 initial et ses racines d’échec sont conservés; aucune preuve n’est déplacée ou supprimée dans ce plan.

## 3. Hypothèses de planification

- **Validation needed :** au préflight, confirmer que le worktree est sur la révision du plan P2 et que seuls les trois fichiers P1 autorisés sont déjà modifiés; ne pas les écraser ni les inclure dans la tâche P2.
- **Validation needed :** les deux scénarios peuvent être simplifiés par des fonctions de support privées sans élargir la visibilité ni changer le contenu rendu, les identités, le suivi ou l’éviction.
- **Safe :** conserver l’inclusion existante des tests privés depuis `src/ui/tests.rs`; les helpers de support seront inclus depuis ce module, selon sa convention actuelle.
- **Decision gate :** obtenir l’accord utilisateur pour ce lot précis avant toute écriture Rust et commande Cargo. Les validations utilisent le `LotRoot` et la garde admis dans le plan précédent; tout refus ou différence de profil arrête le lot.

## 4. Traçabilité des constats

| Constat du relevé frais | Sévérité | Disposition | Tâche / gate | Notes |
|---|---|---|---|---|
| `whisper-desktop`, `too_many_lines`, `src/ui/tests/document_presentation.rs:57`, 105/100 | Structurel | Planned | P2-T1 | Test de rendu du texte paginé, de l’aperçu, des sorties et de l’attente. |
| `whisper-desktop`, `too_many_lines`, `src/ui/tests/document_presentation.rs:245`, 111/100 | Structurel | Planned | P2-T1 | Test du pruning et de la sélection du suivi d’attente. |
| Les 135 autres sites uniques du relevé courant | Structurel | Deferred | Lots ultérieurs | Attribution détaillée dans le journal source; aucun n’est abandonné par ce plan. |

## 5. Séquence recommandée

Traiter les deux sites d’un même fichier de tests privés dans une tâche, en conservant les assertions dans les scénarios et en déplaçant uniquement la construction répétitive et les vérifications de phase dans un support privé. La tâche ferme l’invariant « mêmes données présentées et mêmes entrées conservées dans le suivi d’attente » avant tout autre fichier UI.

### Phase 1 : séparer les phases des deux scénarios de présentation

**Objectif :** supprimer les deux diagnostics `too_many_lines` sans modifier la vue ou le comportement de sélection.

**Invariant fermé :** mêmes sessions, pages, segments, messages rendus, entrées de suivi et ordre d’éviction observés avant et après extraction.

**Métriques avant :** 2 sites `too_many_lines` dans `document_presentation.rs`, fonctions à 105/100 et 111/100.

**Métriques cible :** 0 des deux diagnostics; aucun nouveau site structurel; chaque helper créé reste sous 100 lignes.

**Commandes de métrique :** Clippy JSON gardé à six crates, comparaison par lint/crate/chemin/fonction et cliquet.

**Conditions d’arrêt :** arrêter si une extraction retire ou affaiblit une assertion, change les valeurs rendues, l’identité sélectionnée, le moment de démarrage d’attente, l’ordre d’éviction, ou requiert une API publique; arrêter à tout échec de préflight, format, Clippy ou test.

**Dépendances :** P1-T1 doit rester intact et revu `CLEAN`.

**Résultat attendu :** deux tests plus lisibles et aucun avertissement `too_many_lines` sur ces fonctions.

**Tâches :** P2-T1.

## 6. Tâches

#### P2-T1 : extraire les phases des tests de présentation documentaire

- **Priorité :** High.
- **Constats source :** les deux lignes Planned de la traçabilité.
- **Raison :** retirer deux dépassements structurels proches du seuil, dans les tests d’affichage et de suivi UI.
- **Fichiers/modules cibles :** `crates/whisper-desktop/src/ui/tests.rs`, `crates/whisper-desktop/src/ui/tests/document_presentation.rs`, nouveau `crates/whisper-desktop/src/ui/tests/document_presentation_support.rs`.
- **Carte d’architecture :** `ui/model` conserve l’état de l’interface; `ui/view` conserve le rendu; `ui/tests/` contient les tests privés UI; le nouveau fichier contient seulement les helpers privés de ces scénarios. Aucune politique produit, dépendance ou frontière de crate ne change.
- **Convention de modules :** préserver les `include!("tests/<fichier>.rs")` du fichier `ui/tests.rs`; ajouter le support selon la même convention. Pas de `#[path]`, module public ou nouvelle crate.
- **Budget de portée :** deux fichiers existants et un nouveau support de test; aucun fichier de production, manifeste, baseline ou garde.
- **Invariant fermé :** mêmes entrées et contenu rendu dans le test de page; mêmes clés et timestamps de suivi, événements de session et évictions dans le test d’attente.
- **Métriques avant :** deux sites `too_many_lines` à 105/100 et 111/100.
- **Métriques cible :** aucun diagnostic sur les deux tests; helpers sous 100 lignes; aucun nouveau site dans le crate ou le cliquet.
- **Commandes de métrique :** Clippy gardé à six membres et inspection JSON par fonction/chemin.
- **Conditions d’arrêt :** arrêt sur toute modification du comportement, des assertions, des identités, du rendu ou de l’ordre; arrêt si des changements concurrents P1 sont touchés, si un préflight échoue ou si la garde refuse le profil.
- **Dépendances :** P1-T1 revu `CLEAN`.
- **Étapes d’implémentation :**
  1. Extraire de `unified_ui_renders_one_bounded_page_preview_parts_outputs_and_wait_state` les blocs cohérents de construction de la session/page/audio et la collecte du texte rendu, en gardant dans le test les assertions qui forment l’oracle visible.
  2. Extraire de `unified_wait_tracking_prunes_finished_evicted_sessions_and_keeps_selected_wait` la construction de session, les transitions de phase et la préparation des sessions terminées; garder explicites les assertions d’identité sélectionnée, de borne à huit entrées et de timestamp stable.
  3. Placer les helpers dans le fichier de support privé inclus par `ui/tests.rs`; n’ajouter aucun scénario de test.
  4. Comparer le diff des deux tests à leur état de départ et vérifier que leurs noms/attributs restent identiques.
- **Tests à ajouter/modifier :** aucun test nouveau; conserver et exécuter les deux tests existants par la N1.
- **Vérifications :** format `fmt --check` gardé; Clippy exact à six crates; N1 `test --no-fail-fast` gardée; sélections `test --no-run`, `-- --list`, puis tests pour chacun des workers CPU/Parakeet CPU/common, conformément à la matrice racine.
- **Provenance :** commandes observées et définies dans `AGENTS.md` et le plan P1-T1; une cible courte neuve sous le `LotRoot` existant est utilisée afin de réduire les chemins temporaires dans `C:/wt/cs`.
- **Critères d’acceptation :**
  - Les deux diagnostics ciblés disparaissent, le cliquet passe, et la baseline reste inchangée.
  - Les deux tests d’origine passent avec les mêmes noms, attributs, valeurs vérifiées et ordre de mise à jour.
  - Les trois fichiers P1 déjà revus restent intacts; seules les trois voies P2 déclarées changent.
  - Aucun nouveau code produit, `allow`/`expect`, seuil, test ignoré activé, `native-tests`, DLL ou validation Windows n’est introduit/annoncé.
- **Retour arrière :** annuler uniquement les changements P2 des trois fichiers déclarés; préserver P1, les preuves et tout changement concurrent.
- **Notes à l’implémenteur :** pour le premier scénario, conserver texte final, aperçu, noms de sorties et message d’attente dans le texte collecté. Pour le second, ne pas réduire la boucle d’éviction à une assertion finale qui perdrait la borne ou la stabilité du timestamp à chaque tour.

## 7. Matrice de vérification

| Domaine | Commande ou contrôle | Provenance | Résultat attendu | Tâches | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTest N1 et SelfTest PE avant Cargo; vérifier espace, CARGO_HOME, modèle, racines | Repo docs | PASS | P2-T1 | Témoin PE réel `NON EXÉCUTÉ` sans sélection explicite et SHA. |
| Format | `fmt --check` via garde, cible fraîche sous le `LotRoot` exact | Repo docs | PASS | P2-T1 | Pas de formatage automatique. |
| Clippy | Profil JSON à six membres, lints structurels du plan, FreshTarget court | Observed / Repo docs | Sortie 0; 0 des deux alertes; ratchet PASS sans nouveau site | P2-T1 | Les workers Turbo ne sont pas mesurés par ce profil. |
| N1 défaut | `test --no-fail-fast` via garde, `-WithoutCargoTargetDir`, FreshTarget court, modèle personnel avec SHA prévu | Repo docs | PASS; compte des tests et ignorés attribué | P2-T1 | Arrêt immédiat sur tout échec; aucune reprise ou nettoyage sans arbitrage. |
| N1 workers | Pour chaque CPU/Parakeet CPU/common : no-run, list puis test sous garde | Repo docs | Les 9 commandes passent dans cet ordre | P2-T1 | GPU niveau 2, corps ignorés et `native-tests` restent non exécutés. |
| Préservation produit | Revue du diff et assertions des deux tests | Source inferred | Oracles et transitions identiques | P2-T1 | Ne prouve pas le rendu dans la fenêtre Windows. |

## 8. Décisions et gates

| Gate | Statut | Bloque | Question | Défaut recommandé | Validation |
|---|---|---|---|---|---|
| G1 — acceptation P2 | Blocking | P2-T1 | Acceptes-tu les trois chemins P2 et le groupe de deux alertes ? | Aucun code avant accord explicite | Plan versionné dans Git et accord écrit dans la session. |
| G2 — préflight campagne | Validation | Toutes les commandes Cargo | Le cache, le modèle, les variables isolées, les racines et la garde courante sont-ils conformes ? | Même `LotRoot`, commandes exactes, cible courte neuve | SelfTests puis préflight par garde; arrêt fermé en cas d’écart. |

## 9. Hors périmètre

- Les 135 autres sites du relevé courant; ils seront couverts par des lots ultérieurs avec chemins versionnés.
- Tous les changements de production, routage UI, politique métier, modèle, dépendances, manifestes, lockfiles, baseline, seuils et garde.
- Tout téléchargement, DLL, `native-tests`, GPU niveau 2, fenêtre Windows ou activation des corps `ignore`.
- Migration générale de l’arborescence des tests et extraction de helpers non reliés aux deux constats.

## 10. Auto-contrôle final

| Contrôle | Résultat | Notes |
|---|---|---|
| Plan concret et non placeholder | Pass | Deux sites chiffrés, deux tests nommés. |
| Sections template dans l’ordre | Pass | Sections 1–11. |
| Constatations attribuées | Pass | 2 planifiées; 135 différées explicitement. |
| Tâche complète et bornée | Pass | Chemins, outline, tests, critères, rollback et notes présents. |
| Invariant/métriques/conditions d’arrêt | Pass | Définis pour phase et tâche. |
| Budget de portée | Pass | Deux fichiers existants et un support; aucun dépassement. |
| Placement des tests privés | Pass | Emplacement existant requis par la visibilité UI; aucun test neuf. |
| Provenance des commandes | Pass | `AGENTS.md`, plan P1 et profil observé. |
| Gates explicites | Pass | Acceptation et préflight distingués. |
| Chemins valides | Pass | Sources existantes; support nouveau sous le répertoire de tests existant. |
| Lot exécutable | Pass | Une tâche, deux sites, un fichier source de scénarios. |

## 11. Prompt pour l’implémenteur

Après acceptation de P2-T1, déléguer à `rust_implementer` en mode `IMPLEMENT`. Garder tous les changements P1 du worktree `C:/wt/cs` et n’écrire que dans `crates/whisper-desktop/src/ui/tests.rs`, `tests/document_presentation.rs` et `tests/document_presentation_support.rs`. Préserver les assertions et identités; ne modifier ni la production ni la garde. Refaire les SelfTests puis format, Clippy et toute la N1 via `ci/guard-level1.ps1`, avec une cible courte neuve. Arrêter sur tout échec et conserver les racines et journaux pour arbitrage.
