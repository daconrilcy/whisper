Avancée : plan historique CLEAN ; revue indépendante du code de phase 2 CLEAN sur 63195ef5.
Blocage : clôture T9 suspendue après interruption N2 ; release et remise non exécutées.
Décision : distinguer la revue source réussie de la validation finale encore incomplète.

# Revue du plan REFACTO-POLL-UNIFIED — 7 novembre 2026

Plan : [plan-refacto-poll-unified.md](plan-refacto-poll-unified.md). Baseline produit `08ddfde8a2f2734defaaacbebb58473d72e0380f`, worktree `C:/wt/refacto-poll`, branche `refacto-poll-unified`.

La session principale a rédigé et corrigé le document. `rust_review_coordinator` a confié sa revue indépendante et en lecture seule au même `rust_design_reviewer` pour les deux passages. Un routage initial à `rust_reviewer` a été refusé par ce profil réservé aux deltas implémentés : aucune revue de fond ni cycle de correction à cette occasion. Aucun spécialiste n’a modifié de fichier ou lancé Cargo/test.

| Passage | Candidat documentaire | Verdict et suite |
|---|---|---|
| 1 | `fde4794d8ac7aab35c36e97825b2c0597376d8bc` | FINDINGS : RPU-P01 et RPU-P02 requis ; correction groupée par la session principale |
| 2 | `1c049c81dfb8282c4cd8a87bde688de3c33daa13` | CLEAN documentaire ; les deux constats fermés par le même reviewer, aucun constat requis ouvert |

**RPU-P01 — couverture réalisable.** Le premier plan assimilait branches défensives et branches injectables. Correction : quatre issues inaccessibles U1–U4 avec preuves statiques ; échec OS de création de thread O1 explicitement non injectable et soumis à réserve acceptée ; mécanismes d’injection nommés pour les autres groupes ; toute issue encore non démontrée reste PENDING et interdit son extraction. La décision D3 ne vaut pas couverture exécutée. Constat fermé sur le plan corrigé.

**RPU-P02 — interdiction de build natif.** Une cible externe neuve et un cache de sources ne garantissent pas la réutilisation des artefacts whisper-rs-sys ; le script de dépendance peut lancer CMake. Correction : D4 rend obligatoire une preuve préalable de cache binaire compatible ou un arbitrage précis avant Cargo worker ; aucun essai exploratoire, DOCS_RS ou contournement de fingerprint. L’alternative de réutiliser le résultat antérieur du worker CPU reste une proposition, avec NON EXÉCUTÉ dans le lot si elle est acceptée. Constat fermé sur le plan corrigé, compatibilité du cache toujours non établie.

Conseils A01/A02 traités : l’ordre HashMap n’est pas corrigé par tri de trace ; cas canonique avec une seule session éligible et référence conditionnée pour les ordres observés. Profil de remise `whisper-desktop/personal-install`, commande release et workers installés explicités. Référence historique actualisée à 774/0/129 ; fixture anglaise localisée et empreinte consignée.

Vérifications de préparation exécutées par la session principale : `git diff --check` réussi ; identité main/origin vérifiée à distance ; `ci/test-guard-worktree.ps1` : **37 contrôles positifs/négatifs PASS**, sans Cargo/build natif ; présence et SHA-256 de la fixture anglaise ; présence des ressources personnelles (empreintes des workers/DLL/modèles non encore vérifiées). Aucun nouveau test de caractérisation, N1, Clippy, N2 ou essai dans la fenêtre n’a été exécuté.

CLEAN qualifie le plan à soumettre. Il n’accepte pas D1–D4, n’autorise pas les exceptions et ne constitue ni une preuve d’équivalence, ni une validation du refactor. Le bilan de réalisation n’est pas créé : le lot reste ouvert avant implémentation. Aucun merge, push ou nettoyage.

## Revue du candidat de phase 2

Après fusion locale de la phase 1, le coordinateur `rust_review_coordinator` a confié au même `rust_reviewer` indépendant le delta `e22eed93..63195ef5` : `runtime/poll_unified.rs` et `runtime/poll_unified/{recovery,document,calls}.rs`. Le plan modifié par la session principale est distingué des quatre sources modifiées par l’implémenteur. Aucun test, oracle ou référence gelée n’a changé.

Un passage de fond, zéro correction : **CLEAN pour le code T6–T8**, aucun constat ouvert. Le reviewer a vérifié l’ordre des phases, les sorties immédiates None/Err, la réinsertion en erreur, la libération avant redémarrage et l’ordre Ready/repli/dispatch. Preuves examinées : corpus figé et N1 final ×3 à 864/0/129, mêmes identités que la phase 1, mesures Clippy (poll 6/6, nouveaux helpers ≤10), format et diff-check. L’échec isolé du test desktop de catalogue reste sans cause établie ; ciblé et N1 suivants réussis, aucune assertion modifiée.

Le packet global est **BLOCKED** : N2 interrompu avant résultat et comparaison TXT/SRT, build release/remise et recette humaine non exécutés. Une clarification de présentation du packet a séparé cette limite du verdict source ; elle ne constitue ni une seconde revue ni une correction. Aucun nouvel essai n’a été lancé par le coordinateur ou le reviewer. D4(b) conserve le retest Turbo CPU NON EXÉCUTÉ. Le [bilan intermédiaire](../architecture/REFACTO-POLL-UNIFIED-BILAN.md) et son archive contiennent les preuves disponibles ; la clôture de T9 reste à réaliser.
