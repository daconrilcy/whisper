Avancée : plan REFACTO-POLL-UNIFIED relu CLEAN après une correction documentaire.
Blocage : décisions utilisateur D1–D4 en attente ; aucune implémentation ni campagne Cargo commencée.
Décision : soumettre le plan corrigé, sans assimiler sa revue à une validation produit.

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
