Avancée : familles 7 à 9 réalisées ; 20 tests ajoutés, extractions validées et N1 finale verte.
Blocage : aucun ; avertissements et limites de qualification conservés ci-dessous.
Décision : sous-lot C clos ; bilan figé le 4 novembre 2026, aucun push.

# REFACTO-POLL — bilan C, 4 novembre 2026

Objectif : terminer le découpage de `ImportRuntime::poll()` en conservant
priorités, sorties, contrôles et ordres de nettoyage. Réalisation dans
`C:/wt/poll`, branche `refacto/poll`, selon le [plan C](REFACTO-POLL-C-PLAN.md).

| Étape | Commit local | Résultat |
| --- | --- | --- |
| Plan C | f34a212b | Référence N1 et périmètre versionnés |
| 7a | 52db4b95 | 15 tests collecte/priorités ; réserves fermées |
| 7b | 985f40e9 | Collecte extraite, 251 lignes déplacées ; CLEAN |
| 8a | 123056c2 | 5 tests routage/final ; réserves fermées |
| 8b | 02fe88df | Routage/final extraits, 244 lignes retirées ; CLEAN |

Au début C, Clippy relevait 489 lignes pour `poll()` ; à la clôture,
son corps compte **10 lignes physiques**.
Les **34 handlers** des familles 1 à 8 font au plus 96 lignes physiques.
Lints Clippy actifs sur tous : **aucun dépassement de 100 lignes ou de
complexité 25** ; scores exacts sous seuil non publiés, aucune référence
Clippy créée. Chaque extraction C reste sous 800 lignes déplacées. Sorties
immédiates et traitement final commun restent distincts, sans fusion ni
réordonnancement ; branches inatteignables conservées et documentées.

**Vérification finale `c-final-*` :** SelfTest 35 positifs/79 négatifs,
Structure 951/951, format et Clippy réussis. N1 : défaut 705 réussites/128
ignorés, common 11/1, CPU 12/9, Parakeet CPU 26/11 ; total **754 réussites,
0 échec, 149 ignorés**. Les 166 tests native-gated sont recensés, pas exécutés.
PE supplémentaire : **non concerné**. Aucun N2 C ; la campagne comparative B
reste sa preuve historique. Aucune validation Windows supplémentaire.

Les revues 7a/8a ont nécessité les compléments exceptionnels précisément
autorisés, sans remise à zéro des deux cycles ; toutes leurs réserves sont
fermées. 7b et 8b sont CLEAN. Les échecs, arbitrages et limites figurent dans
le suivi C ; aucune mutation supplémentaire n'était exigée pour ces familles.

**Limites conservées :** capture synthétique et workers contrôlés dans les
nouveaux tests ; warnings `large_enum_variant`, `result_large_err` et
`needless_return` visibles, sans impact fonctionnel démontré. Dette héritée :
helper d'archive `cfg(test)` à remplacer ultérieurement par trait injecté ;
écart nom/contenu du test GPU existant consigné en A/B, test inchangé.

**Rétention :** `target/parakeet-tests/refacto-poll-c/archive/REFACTO-POLL-C-logs-20261104.zip` :
**6 441 entrées**, 12 278 673 octets source, 4 951 701 octets archivés ;
nombre, tailles et SHA-256 de chaque entrée vérifiés. Archive SHA-256 :
`2602D8619C294FFE078BC65C7DA6E3BF4B73BF05E19FA528BD485D9ABF71C86E`.
48 jonctions SelfTest et leurs descendants exclus avec accord explicite,
sans suppression ; exécutables/intermédiaires compilés exclus de l'archive.
Aucune cible Cargo ni cache Cargo isolé propre C créé. Caches partagés et
persistants, preuves brutes, modèles et audios préservés ; aucune suppression.
Espace libre final relevé : 575,05 Gio.
