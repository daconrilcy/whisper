Avancée : inspections queue/recovery ciblées ; livrables PRESERVATION et preuves archivés.
Blocage : deux tests Parakeet en échec ; N1 final et P5 non réalisés.
Décision : LIVRÉ AVEC RÉSERVES après deux cycles ; aucune fusion ni publication.

# INSPECT-CIBLEE — bilan figé du 6 novembre 2026

[Complément de levée des réserves du 6 novembre 2026](INSPECT-CIBLEE-RESERVES-COMPLEMENT.md).

Base `34975d43`, branche `inspect-ciblee`, worktree `C:/wt/inspect-ciblee`. Plan relu avant code, implémentation déléguée. **R2/R3 code fermés ; R1 OPEN_VERIFICATION, R4 OPEN ; aucun CLEAN.**

**Cartographie/correctif.** Recherche inspections/SHA dans toutes les crates : démarrage desktop et quatre bindings micro inspectent tous les modèles ; import, archive, qualifications et workers ciblent leurs modèles. Queue/recovery réutilisent **`inspect_requested_model` de d7e6445a**. Appelants, coûts et contrôle de complétude : [annexe](INSPECT-CIBLEE-PRESERVATION.md).

| Preuve | Résultat |
|---|---|
| Avant correctif, code produit `34975d43` | 2/2 tests rouges : Turbo lu 1 fois alors que seul Parakeet est requis |
| N1 intermédiaire sans `CARGO_TARGET_DIR` | 718/1/128 contre référence 715/0/128 ; +4 tests alors, contrôle fixture PE échoué |
| Dernier focus, 7 nouveaux tests au total | **5 réussites / 2 échecs** ; fixtures Parakeet `PathOutsideRuntime`, intégrité attendue non atteinte |
| Validation finale | **N1 ×3, sourceguard, format/lints/build et contrôle générique finaux NON EXÉCUTÉS** après arrêt |

Compteurs synthétiques, sans modèle réel ni worker natif. Six runs release non exécutés : aucune phase queue/recovery exploitable ; marqueurs SHA sous `native-tests`, refusée en N1. Aucun gain chronométré revendiqué.

**Verrou, diagnostic seul.** Mutex tenu pendant ouverture/SHA/revalidation ; Turbo historique 16,44–16,50 s, attente alors ≤0,0029 ms. Catalogue sur thread dédié ; résultat/admissions peuvent attendre. Proposition non appliquée : calcul hors mutex, puis revalidation/insertion coordonnée par identité.

| Annexe PRESERVATION | État de clôture |
|---|---|
| P1 | Note figée 13 lignes ; dates/ressources vérifiées, enquête disque/VSS attribuée à l'utilisateur |
| P2 | Trois renvois vivants ajoutés ; bilans figés intacts et liens morts listés ci-dessous |
| P3 | Protections du banc absent retirées ; règle présence/conservation hors target dans AGENTS |
| P4 | Présence/absence et specs récursives vertes ; générique échoué à 1057 ms / 1000 ms, final non exécuté |
| P5 | `/MOVE /SJ /SL` proposé ; déplacements non confirmés, destinations encore absentes |
| P6 | Nettoyage RAII ajouté ; zéro résidu ciblé observé, **preuve des trois N1 manquante** |
| P7 | Cinq worktrees inventoriés ; build-natif conservé, bilan C1001 seul déconseillé sans preuves/références |

Liens historiques conservés : `bilan-parakeet-faisabilite-cloture`, `bilan-parakeet-fr-en-criteres-revises`, `bilan-parakeet-fr-en-go`, `bilan-parakeet-limite-decoupe-longue`, `bilan-parakeet-limite-vad-revisee` (`docs/cadrage/*.md`) ; anciens chemins d'archives dans `BILAN-FUSION-PILE.md` et `POLL-INTEGRATION-NETTOYAGE.md`.

**Limites/rétention.** Budget trois essais et arrêt sur échec enfreints au premier cycle ; dernier arrêt respecté. C libre 466971537408→460517015552 octets, mémoire 18→19 %, aucune nouvelle WHEA/crash signalée. Aucun build natif/recette Windows. [Archive](raw/INSPECT-CIBLEE-20261106T092341.zip) : 109 fichiers, ZIP 1307501 octets, chaque SHA-256 vérifié. Tailles/commandes humaines dans l'annexe ; **aucun déplacement/suppression par l'agent**, cache persistant conservé. Dettes : R1/R4, N1/P6, P4/P5, mutex, inspections micro/démarrage. Toute suite : nouveau complément, bilan inchangé.
