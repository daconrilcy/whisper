Avancée : trois tests non reproduits ; 190 appels B sans échec et validations finales réussies.
Blocage : aucun à la clôture ; les causes historiques restent indéterminées.
Décision : LIVRÉ AVEC RÉSERVES, aucun correctif Rust ; commits locaux, sans merge ni push.

# TESTS-STABLES-2 — bilan figé, 9 novembre 2026

Worktree `C:/wt/tests-stables-2`, branche `tests-stables-2`, sources identiques à `08062bd8`. [Plan, identités S1/S2/P1 et preuves](../cadrage/plan-tests-stables-2.md). Un échec historique attesté par test ; aucun retrouvé ici.

| Test / famille | Cause envisagée, non établie | Correction | Échecs avant / après correction |
|---|---|---|---|
| S1 — réglages concurrentiels | Ordre chemins/migration et backup v1 | Aucune | B : 0/50 seul, 0/20 suite / sans objet |
| S2 — réglages, backup existant | Racine nom+PID réutilisée | Aucune | B : 0/50 seul, 0/20 suite / sans objet |
| P1 — probe et PCM/WAV | Ordonnancement, chunks perdus ou snapshot expiré | Aucune | B : 0/50 seul, 0/20 suite / sans objet |

**Non reproduits : S1, S2, P1.** B : 190 appels uniques, filtres vérifiés, suites réglages 6/0/0 ×20 et bibliothèque 504/0/49 ×20. Avec N1 final, **73 exécutions par test**. Aucun test masqué ; trois faux négatifs du contrôleur rectifiés sur les logs, sans rejeu. Durées harnais B : S1/S2/P1 **0,82/0,81/6,19 s**, suites **0,76/589,32 s**. Durée exacte sous garde manquante ; enveloppe conservatrice **41 min 05 s**, donc plafond respecté. Lot clôturé en moins de 2 h.

**Validation finale, après reprise autorisée du préflight format refusé pour cible omise :**

| Sélection | Réussis / échecs / ignorés |
|---|---|
| N1 complet sans `CARGO_TARGET_DIR` | **878/0/129 ×3** |
| Workers CPU / Parakeet CPU / common | **12/0/9 — 26/0/11 — 11/0/1** |

N1 : **103,407/35,957/36,157 s** sous garde ; **1 007 identités inchangées** entre runs et référence fix-catalog. Workers également identiques. Format, check, Clippy passent ; avertissements préexistants. **10/10 PE PASS**, SelfTest 4/15 ; témoin réel NON EXÉCUTÉ. Compilation `whisper-rs-sys` via Cargo CPU autorisée ; WHEA **1→1**, RecordId 527992. Aucun corps GPU exécuté.

**Revue :** deux cycles via `rust_review_coordinator`/`rust_reviewer`. Diff provisoire (+60/−10) retiré ; TS2-R01–R05 fermés par retrait. Validation ultérieure sur sources initiales ; aucun CLEAN d'implémentation ni correction de cause revendiqué.

**Preuves :** [ZIP](raw/TESTS-STABLES-2-PREUVES.zip), **865 membres**, 1 386 051 octets ; nombre et chaque SHA-256 vérifiés. SHA-256 : `edf283407b5cb1d3476b421f2e6b0355c5b7a5929b290a8f8c639c23ac0830ff`. Libre début/fin : **472 641 638 400 / 465 881 767 936 octets**. [Rétention et commandes humaines](../cadrage/plan-tests-stables-2.md#racines-présentes-et-commandes-proposées-à-lhumain) : 4,87 Go proposés ; cache persistant et scratch conservés. **Aucune suppression exécutée.**

**Dettes :** causes des trois intermittences, sens de « v1 initial » avant correction S1, durée B exacte. Non reproduit ne signifie pas absence de course. Seuls plan, bilan et archive sont modifiés.

[Complément : vérification du nettoyage humain](../cadrage/bilan-nettoyage-tests-stables-2.md).
