Avancée : plan borné aux corrections prioritaires observées sur six crates.
Blocage : workers Turbo non mesurés ; toute compilation native reste suspendue.
Décision : séquencer trois groupes de production, puis inventorier les alertes résiduelles sans réduire la dette structurelle existante.

# Plan d’implémentation — corrections prioritaires Clippy du 12 novembre 2026

## 1. Référence et périmètre

- Source : [cadrage Clippy priorités](cadrage-clippy-priorites-20261112.md).
- Référence du dépôt : `main`, commit `150b1862`.
- Relevé exécuté : worktree `C:/wt/reduc-adapters-tests`, arbre `1558f55fcfd81c0584efa9d2d247003e4db38bdb`, identique à `main` lors de la mesure.
- Campagne : garde N1, `cargo clippy --offline --locked`, six packages explicites, `--all-targets --message-format=json`, avec quatre lints structurels activés en avertissement.
- Résultat : code 0 ; 176 diagnostics distincts ; cliquet structurel PASS à 157 clés/sites contre 168 dans la baseline.
- Limite : `whisper-worker-cpu` et `whisper-worker-gpu` non compilés. Aucun constat Clippy ne leur est attribué.
- Ce plan n’autorise ni code, ni nouvelle campagne Cargo, ni changement de garde. Il ne couvre pas la refonte générale des alertes structurelles.

## 2. Ordre et tâches

| Tâche | Priorité et portée | Travail prévu | Critère de sortie |
|---|---|---|---|
| P1-T0 | Préflight du futur lot | Vérifier le HEAD, l’état Git, le plan accepté, les chemins de crates applicables, les consignes locales, la garde et la disponibilité hors ligne. Refaire une baseline Clippy gardée seulement après accord d’implémentation explicite. Ne pas forcer les workers Turbo dans le workspace. | HEAD et sites frais concordent ; SelfTest PASS ; toute dépendance native absente arrête le périmètre concerné avant compilation. |
| P1-T1 | Contrôles privés de polling — deux fichiers de production | Examiner les producteurs et consommateurs de `PollCollectionControl`, `PriorityControl` et `prepare_poll_event`. Évaluer les représentations compactes possibles et leur coût. N’appliquer qu’une solution locale préservant les événements et transitions. | Les alertes `large_enum_variant` ×2 et `result_large_err` sont supprimées sans changer résultat, priorité, identité ou ordre. Toute nouvelle allocation est mesurée ou explicitement jugée acceptable avant adoption. |
| P1-T2 | Arrêt Parakeet | Examiner `Lifecycle::stop` et ses collaborateurs. Extraire au besoin drainage de capture ou finalisation audio lorsque la responsabilité et la propagation d’erreur restent claires. | Pas d’alerte ciblée de complexité cognitive sur `stop` ; mêmes blocs audio durables, traitement de fin de capture, délai, erreurs et événement final. |
| P1-T3 | Import Parakeet contrôlé | Examiner `run_controlled_unified_import`, ses états intermédiaires, fichiers et scheduler. Extraire au besoin les étapes sans déplacer ni réordonner les effets. | Pas d’alerte ciblée de complexité cognitive ; mêmes erreurs, admissions, écritures, fenêtres et résultat de transcription. |
| P1-T4 | Reprise d’import interrompu | Cartographier les étapes de `resume_interrupted`, puis découper par responsabilité uniquement si chaque frontière rend explicites ses préconditions et effets. | Pas de perte de validation, de reprise implicite ni d’écriture déplacée ; mêmes décisions de récupération et mêmes sorties. L’alerte de longueur peut rester si le découpage compromet ces invariants. |
| P1-T5 | Alertes non structurelles restantes | Reprendre le JSON frais, distinguer production/tests et vérifier chaque alerte hors des trois groupes précédents. Corriger seulement les alertes locales et justifiées, sans suppression opportuniste de code ni contournement de lint. | Liste complète des diagnostics restants attribuée ; corrections bornées ; tout résiduel documenté par raison et propriétaire. |
| P1-T6 | Tests et vérifications ciblés | Réutiliser les tests existants des chemins touchés ; adapter les tests seulement si une assertion d’invariant manque. Utiliser les commandes gardées permises par le plan futur et les contrôles de crates requis par les AGENTS locaux. | Scénarios pertinents passent ; nombre d’ignorés rapporté ; format et lints passent selon la matrice du lot. Aucun test GPU de niveau 2 présenté comme exécuté. |
| P1-T7 | Clôture et rapport | Rejouer la matrice Clippy autorisée sur les crates du lot, comparer les sites au relevé initial et rapporter le résiduel structurel, la couverture et les limites. Contrôler diff et état Git. | Aucun nouveau diagnostic inexpliqué ; critères P1-T1 à T6 satisfaits ou résiduels arbitrés ; résultats et réserves reproductibles. |

## 3. Invariants de correction

- Les événements de polling gardent le même ordre de priorité et les mêmes valeurs.
- Job, génération, instance, ACK et filtrage des événements périmés gardent leur identité et leur résultat.
- L’arrêt micro conserve toutes les données annoncées comme capturées, les octets écrits, le délai borné et les causes d’échec.
- L’import et la reprise conservent préconditions, admissions, ordre des écritures, publication et résultats.
- Les assertions des tests ne sont pas supprimées ou affaiblies pour faire disparaître une alerte.
- Aucun `allow`, aucune modification de baseline ou de garde ne masque une alerte.
- Les alertes structurelles déjà référencées sont rapportées, sans objectif général de réduction dans ce lot.

## 4. Vérifications prévues

Toutes les commandes Cargo d’implémentation dépendront d’un accord précis pour le lot, du préflight prévu par `AGENTS.md` et d’une disponibilité de cache confirmée. Aucune commande Cargo n’est autorisée par le présent document.

| Vérification future | Résultat attendu | Limites |
|---|---|---|
| SelfTest N1 via `ci/guard-level1.ps1` | PASS avant Cargo | Environnement, racines et espace disque contrôlés selon la règle permanente. |
| Clippy JSON gardé sur les crates autorisées, toutes cibles | Code 0 ; diagnostics des sites corrigés fermés ; aucun nouveau site structurel | N’inclure CPU/GPU Turbo que si les prérequis natifs sont qualifiés et qu’une autorisation distincte permet la campagne. |
| Tests N1 concernés, via garde | Résultats et ignorés rapportés, aucune régression | Tests GPU niveau 2 et corps ignorés restent séparés. |
| `cargo fmt --check` gardé et `git diff --check` | PASS | Aucun appel Cargo direct ; le diff est relu pour les invariants. |
| Revue indépendante Rust selon `AGENTS.md` | Constats fermés ou réserves explicites, au plus deux cycles | Elle suit la production du candidat ; elle ne remplace pas la recette Windows si celle-ci est requise. |

## 5. Gates et arrêts

- L’implémentation ne commence qu’après accord écrit précis dans sa session et délégation au profil Rust prévu dans `AGENTS.md`.
- Une alerte située dans un fichier ou une crate hors du plan impose un nouvel arbitrage de portée.
- Si une correction de polling modifie l’ordre d’événement ou introduit un coût mémoire significatif, isoler la décision et arrêter ce site.
- Si les tests ne peuvent pas établir les invariants d’arrêt ou reprise, ne pas simplifier le test pour obtenir un PASS.
- Une compilation qui nécessite les builds natifs suspendus reste arrêtée. La disponibilité d’un ancien binaire ou cache ne qualifie pas une reprise native.
- Toute alerte structurelle nouvelle, écriture hors racines, échec de test ou écart inexpliqué arrête la campagne selon les règles du dépôt.
- Aucun commit direct sur `main`, aucun push, nettoyage ou validation Windows n’est inclus dans ce plan.

## 6. Hors périmètre

Les alertes structurelles préexistantes autres que les alertes ciblées de complexité sur `stop` et `run_controlled_unified_import), l’ensemble des alertes non mesurées des workers CPU/GPU Turbo, toute modification de la garde, toute hausse de baseline, les builds natifs suspendus, tests GPU niveau 2, changements de modèle, campagne de performance, recette Windows, commit, merge, push et nettoyage.

## 7. Provenance

Le relevé détaillé est conservé dans le journal JSON généré par la campagne du 12 novembre sous la racine de lot `C:/wt/reduc-adapters-tests/target/parakeet-tests/clippy-frais-20261112/`. Le rapport de clôture d’un éventuel lot d’implémentation devra conserver les chemins de ses sorties réelles et distinguer diagnostics observés, corrections, validations et limites.
