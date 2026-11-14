# Plan P23 — Réduire la fonction d’identité des événements

Avancée : Clippy standard après P22 passe à 112 sites; un site `too_many_lines` de 108 lignes est retenu.
Blocage : aucun; toutes les variantes et paires JobId/Generation doivent rester exactement identiques.
Décision : extraire un matcher privé par familles d’événements dans le module de métadonnées.

## Source et périmètre

Le relevé Clippy frais final de P22 signale `ImportEvent::identity` à `crates/whisper-core/src/application/event_metadata.rs:5` (108/100). Le module est la responsabilité de métadonnées d’identité du cœur, sans I/O. L’énumération d’événements est un contrat partagé; ce lot ne modifie que l’organisation interne du match.

Périmètre code unique : `crates/whisper-core/src/application/event_metadata.rs`. Préserver les changements P1–P22; aucune modification de l’énumération, protocole, réexport, API publique, autre fichier, baseline ou garde.

## P23-T1 — Extraire une famille du match exhaustif

Découper la logique en fonctions privées, nommées selon une famille stable d’événements (par exemple événements micro/Parakeet ou événements live/import) afin que `identity()` passe sous 100 lignes. Garder un match exhaustif final sur `ImportEvent`; déléguer les groupes d’arms sans changer leurs motifs, leurs valeurs ou leur ordre relatif. Conserver le sentinel `(JobId(0), Generation::first())` pour les variantes sans identité et les accès directs aux segments, compteurs, mètres, spectres et requests.

Le Clippy mesure seule au seuil 0 doit montrer le score de chaque helper ≤20. Le Clippy standard au seuil projet 25 avec ratchet demeure la gate structurelle finale.

### Acceptation

- L’alerte ciblée disparaît; le ratchet standard ne compte aucune nouvelle clé/site et passe à 111 ou moins.
- Les mêmes variantes restent exhaustivement couvertes et renvoient la même paire ou le même sentinel qu’avant; aucune mutation/clone/alloc ne remplace une extraction directe par valeur.
- SelfTest, mesure threshold-0 via le profil gardé, fmt-check, Clippy standard six crates/ratchet, N1 complet et sélections workers/PE, `git diff --check` passent par les contrôles prescrits.
- Aucun nouveau test n’est ajouté; le N1 existant sert de couverture de régression.

## Gates et arrêt

Worktree `C:/wt/cs`; LotRoot gardé `target/parakeet-tests/clippy-corrections-suivantes-20261113`; cible existante `p21-fresh-20261114`. Toute mesure passe par `-MeasureCognitiveComplexity` avec les six membres exacts et seuil 0; le gate Clippy standard reste séparé et ratcheté. Arrêter au premier échec et préserver les logs. P23 est la troisième tâche de la tranche P21–P30; demander un nouvel arbitrage après P30.

## Retour arrière

Si une extraction modifie l’identité d’une variante, retirer uniquement le découpage fautif dans le fichier attribué et rapporter les variantes affectées; ne pas corriger en élargissant le domaine ou en changeant le sentinel.
