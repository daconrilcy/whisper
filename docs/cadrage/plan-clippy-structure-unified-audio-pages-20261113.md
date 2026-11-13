Avancée : troisième lot proposé sur deux tests privés de pagination audio unifiée.
Blocage : aucune modification Rust P3 avant acceptation du plan et de sa N1.
Décision : extraire les phases de construction et d’observation des deux scénarios, sans changer leurs oracles.

# Plan — structure des tests de pages audio unifiées

## Header

- **Source audit :** Clippy frais après P1/P2 sur le candidat `C:/wt/cs`, journal `C:/wt/cs/target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261113T194536866-650343eea09848938fff3ba5a148a72f/cargo-command.stdout.log`, SHA-256 `A62941AA8FA2A6333947AB3EDC7EAEB0E9E8945FA856DD350D476B50D126E679`.
- **Date de planification :** 13 novembre 2026.
- **Workspace cible :** crate `crates/whisper-desktop` dans `C:/wt/cs`.
- **Mode :** plan uniquement; aucun code P3 modifié.
- **Confiance :** high — les diagnostics 126/100 et 106/100, leurs fonctions et la convention d’inclusion ont été confirmés dans la source actuelle.
- **Contrôle qualité du plan :** passed.

## 1. Résumé de l’audit

Après P1 et P2, Clippy gardé compte 135 sites structurels uniques contre 168 dans la baseline; aucun nouveau site n’est apparu. Deux alertes proches concernent la gestion des pages audio d’une session unifiée, dans `src/ui/tests/unified_sessions.rs` : `stale_audio_page_retries_are_delayed_and_corruption_does_not_retry` (126 lignes) et `unified_audio_page_stays_visible_at_same_position_while_progress_refreshes` (106 lignes). Le même fichier comporte aussi un autre site à 141 lignes; celui-ci n’est pas inclus dans ce lot.

Le premier scénario vérifie l’attente avant reprise d’une réponse périmée, la coalescence d’une lecture inchangée, le second retry après délai, l’arrêt sur page fraîche et la visibilité d’une corruption réelle. Le second vérifie que la page précédente reste visible à la même position pendant les rafraîchissements de progression. Ces assertions protègent l’identité de page, les délais, les retries et la stabilité d’affichage; aucun dépassement Clippy ne démontre un défaut produit.

## 2. Vérification de fraîcheur

- Inspectés : journal Clippy frais et son SHA, `crates/whisper-desktop/AGENTS.md`, `src/ui/tests.rs`, `src/ui/tests/unified_sessions.rs`, le plan P2 et la garde N1.
- `ui/tests.rs` inclut le fichier de scénario via `include!("tests/unified_sessions.rs")`; aucun changement à cette frontière n’est requis sauf inclusion du support privé.
- Les deux tests visés existent aux lignes Clippy 561 et 691, mesurés 126/100 et 106/100; l’alerte voisine de ligne 368 (141/100) est laissée hors P3.
- P1 et P2 demeurent non commités mais revus `CLEAN` dans le même worktree. Ils constituent des changements préexistants à préserver; aucun de leurs fichiers n’est dans le scope P3.
- Le Clippy retenu a réussi avec 135 sites, baseline 168. Les SelfTests et toute la N1 ont passé au lot P2 dans le même worktree court; P3 doit refaire ses préflights et validations.

## 3. Hypothèses de planification

- **Validation needed :** préflight P3 confirme la branche, le plan commité, le worktree court, le cache Cargo, le modèle, l’espace et l’absence de collision avec les changements P1/P2.
- **Validation needed :** la construction de session/page et les observations de frame peuvent être factorisées en helpers privés; les commandes, temps de frame, révisions, identités et assertions restent inchangés.
- **Safe :** utiliser le répertoire de support privé inclus par `ui/tests.rs`, selon le modèle déjà adopté en P2, sans introduire API ou crate.
- **Decision gate :** l’accord utilisateur sur P3 et sa validation N1 complète est requis avant les modifications et les commandes Cargo.

## 4. Traçabilité des constats

| Constat Clippy frais | Sévérité | Disposition | Tâche / gate | Notes |
|---|---|---|---|---|
| `whisper-desktop`, `too_many_lines`, `src/ui/tests/unified_sessions.rs:561`, 126/100 | Structurel | Planned | P3-T1 | Retry temporisé des pages audio périmées et distinction de corruption réelle. |
| `whisper-desktop`, `too_many_lines`, `src/ui/tests/unified_sessions.rs:691`, 106/100 | Structurel | Planned | P3-T1 | Visibilité et position d’une page précédente lors du rafraîchissement. |
| Autres 133 sites du relevé courant, dont `unified_sessions.rs:368` à 141/100 | Structurel | Deferred | Lots ultérieurs | Conservés dans la mesure; chaque groupe aura son propre plan et scope précis. |

## 5. Séquence recommandée

Traiter les deux scénarios de page audio dans une tâche de support commune, sans y ajouter le test voisin de sélection/démarrage unifié. Le périmètre ferme un invariant d’affichage et de retry pour les pages audio, en gardant les scénarios d’origine comme seuls oracles exécutés.

### Phase 1 : factoriser les scénarios de page audio sans changer leur chronologie

**Objectif :** supprimer les deux diagnostics `too_many_lines` ciblés.

**Invariant fermé :** mêmes identités, révisions, erreurs, temps de frame, commandes de lecture, nombre de retries, visibilité et position de page.

**Métriques avant :** deux sites dans `unified_sessions.rs`, 126/100 et 106/100; un site voisin non traité à 141/100.

**Métriques cible :** aucun diagnostic sur les deux tests P3; helpers sous 100 lignes; aucun nouveau site structurel; le site 141/100 reste attribué et reporté.

**Commandes métriques :** Clippy JSON gardé six membres, comparaison d’identité des sites et cliquet.

**Conditions d’arrêt :** arrêter si les assertions de délai, coalescence, nombre de lectures, état de corruption, identité, révision ou position changent; arrêter sur tout échec SelfTest, format, Clippy, N1 ou sélection worker.

**Dépendances :** P1 et P2 restent intacts et revus `CLEAN`.

**Résultat attendu :** deux tests plus courts qui conservent toute la séquence d’observation.

**Tâches :** P3-T1.

## 6. Tâches

#### P3-T1 : extraire les phases des tests de pages audio

- **Priorité :** High.
- **Constats source :** deux sites Planned ci-dessus.
- **Raison :** retirer deux alertes proches dans un seul fichier et une même responsabilité de page audio.
- **Fichiers/modules cibles :** `crates/whisper-desktop/src/ui/tests.rs`, `crates/whisper-desktop/src/ui/tests/unified_sessions.rs`, nouveau `crates/whisper-desktop/src/ui/tests/unified_sessions_support.rs`.
- **Carte d’architecture :** `ui/model` contient les états de l’application desktop; `ui/view` rend l’état; `ui/tests/` héberge les tests privés; le nouveau fichier porte uniquement les helpers privés de ces deux tests. Pas de dépendance UI vers un autre bucket.
- **Convention de modules :** préserver `include!("tests/<fichier>.rs")` depuis `ui/tests.rs`; inclure le support privé dans la même liste; pas de `#[path]`, module public ou crate nouvelle.
- **Budget de portée :** deux fichiers de test existants et un nouveau fichier de support, aucun fichier de production, manifeste, seuil ou garde.
- **Invariant fermé :** mêmes révisions de page et d’erreur, temps de frame, commandes de lecture et page visible à position stable.
- **Métriques avant :** 126/100 et 106/100; alerte voisine 141/100 en report explicite.
- **Métriques cible :** 0 des deux diagnostics visés, pas de site nouveau; chaque helper <100 lignes.
- **Commandes métriques :** Clippy gardé exact à six crates et inspection des diagnostics JSON.
- **Conditions d’arrêt :** arrêter si un délai/instant de frame, retry, lecture émise, message visible, révision ou position vérifiée est changé; arrêter si un fichier P1/P2 ou tout chemin non prévu est modifié.
- **Dépendances :** P1/P2 revus CLEAN.
- **Étapes d’implémentation :**
  1. Extraire du premier test la préparation session/sélection et la création d’un frame à temps explicite; garder dans le test l’ordre des frames et assertions sur le zéro retry immédiat, la lecture après délai, la coalescence et le refus de retry sur corruption.
  2. Extraire du second test la construction d’une session/page et l’observation des textes de frame; garder explicites les quatre progrès d’échantillons, le label « actualisation », et la comparaison de position entre chaque frame.
  3. Placer les helpers dans `unified_sessions_support.rs`, inclus via `ui/tests.rs`; ne pas déplacer ou modifier le test de ligne 368.
  4. Comparer les assertions, valeurs et séquences au candidat audité avant de lancer les vérifications.
- **Tests à ajouter/modifier :** aucun test nouveau; garder les deux tests existants et les exécuter dans la matrice N1.
- **Vérifications :** SelfTests garde/PE; `fmt --check`; Clippy six membres; N1 `test --no-fail-fast`; no-run/list/tests pour workers CPU, Parakeet CPU et common; toutes via `ci/guard-level1.ps1`.
- **Provenance :** commandes définies par `AGENTS.md` et observées sur P1/P2; `LotRoot` précis déjà enregistré, `FreshTarget` court sous ce lot, cible existante réutilisée pour les sélections worker.
- **Critères d’acceptation :**
  - Les deux diagnostics disparaissent et le site voisin 141/100 demeure identifié.
  - Les deux tests gardent noms, attributs, assertions, temps de frame, résultats et commandes observées.
  - P1/P2 restent intacts; seuls les trois chemins P3 déclarés changent.
  - Le cliquet passe sans nouveau site; aucune baseline, production, manifest, garde ou seuil n’est modifié.
- **Retour arrière :** retirer seulement les changements P3 des trois chemins autorisés; préserver P1/P2, les logs et tous les chemins retenus par la campagne.
- **Notes à l’implémenteur :** distinguer le retry différé d’une réponse périmée de la corruption persistante qui doit rester visible sans retry. Dans le second test, l’ancienne page reste visible à une position stable pendant chaque progression; ne pas réduire l’observation à un instant final.

## 7. Matrice de vérification

| Domaine | Commande ou contrôle | Provenance | Résultat attendu | Tâches | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTest garde et PE; espace ≥30 Gio, cache, modèle, variables et racines | Repo docs | PASS | P3-T1 | Témoin PE réel `NON EXÉCUTÉ` sans sélection et SHA. |
| Format | `fmt --check` via garde, FreshTarget court | Repo docs | PASS | P3-T1 | Aucun formatage automatique. |
| Clippy | Profil exact six membres, JSON et lints structurels, FreshTarget court | Observed / Repo docs | Sortie 0; zéro cible P3; ratchet PASS sans nouveau site | P3-T1 | Les alertes Turbo workers ne sont pas incluses. |
| N1 défaut | `test --no-fail-fast`, garde, sans `CARGO_TARGET_DIR`, FreshTarget court, modèle personnel vérifié | Repo docs | PASS; compte des tests/ignorés rapporté | P3-T1 | Arrêt immédiat à tout échec, roots conservés. |
| N1 workers | CPU/Parakeet CPU/common : no-run, list, tests, ordre exact via garde | Repo docs | Les neuf sélections passent | P3-T1 | Tests ignorés/native-tests et GPU niveau 2 restent non exécutés. |
| Oracles UI | Revue diff et assertions/tests ciblés dans N1 | Source inferred | Temps, données, messages et position préservés | P3-T1 | Pas de validation visuelle Windows. |

## 8. Décisions et gates

| Gate | Statut | Bloque | Question | Défaut recommandé | Validation |
|---|---|---|---|---|---|
| G1 — acceptation P3 | Blocking | P3-T1 | Acceptes-tu les trois chemins P3 et les deux alertes identifiées ? | Aucun code avant accord explicite | Plan versionné dans Git et accord écrit dans la session. |
| G2 — préflight de campagne | Validation | Toute commande Cargo | Le profil, la cible courte, le modèle et les racines sont-ils encore conformes ? | Réutiliser le `LotRoot` exact enregistré | SelfTests et contrôles de garde avant Cargo. |

## 9. Hors périmètre

- Le troisième site de `unified_sessions.rs` à 141/100 et les 132 autres sites du relevé.
- Les modifications de production, d’API, de routage, de modèles, de dépendances, de manifeste, de lockfile, de baseline, de seuil ou de garde.
- Tout téléchargement, `native-tests`, `--ignored`, build DLL, GPU niveau 2 ou validation Windows.
- Migration générale des tests UI et tout nettoyage sans lien avec ces deux constats.

## 10. Auto-contrôle final

| Contrôle | Résultat | Notes |
|---|---|---|
| Plan concret, non placeholder | Pass | Deux fonctions et métriques fraîches. |
| Sections template présentes dans l’ordre | Pass | Sections 1–11. |
| Constatations attribuées | Pass | 2 planifiées, 133 différées. |
| Tâche complète et bornée | Pass | Fichiers, étapes, commandes, critères et rollback. |
| Invariant/métriques/conditions d’arrêt | Pass | Phase et tâche. |
| Budget de portée | Pass | Deux fichiers + support neuf. |
| Placement test privé | Pass | Fichier UI existant nécessaire; aucun nouveau test. |
| Provenance commandes | Pass | `AGENTS.md`, profils observés. |
| Gates explicites | Pass | Acceptation et préflight distingués. |
| Chemins valides | Pass | Chemins existants et un support planifié. |
| Lot exécutable | Pass | Une tâche cohérente, deux tests.

## 11. Prompt pour l’implémenteur

Après acceptation, déléguer P3-T1 à `rust_implementer` en mode `IMPLEMENT` dans le worktree court `C:/wt/cs`. Préserver les six chemins P1/P2 préexistants et n’écrire que dans `crates/whisper-desktop/src/ui/tests.rs`, `tests/unified_sessions.rs` et `tests/unified_sessions_support.rs`. Refaire les SelfTests, puis format, Clippy et la N1 complète via `ci/guard-level1.ps1`. Arrêter sur tout échec et ne pas nettoyer les roots/logs.
