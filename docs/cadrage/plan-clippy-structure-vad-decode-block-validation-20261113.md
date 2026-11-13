# Plan P17 — Isoler la validation d’un bloc VAD décodé

Avancée : plan ciblé sur `advance_parakeet_vad_decode` (102 lignes).
Blocage : aucun ; aucun saut ni écart de samples ne doit être accepté.
Décision : extraire le prédicat d’égalité de la plage décodée dans un helper privé.

## En-tête

- Source audit : Clippy frais P16, 120 sites, `.../scratch/20261113T233617714-4b625b566b6b42848a1e2385fe5de302/cargo-command.stdout.log`.
- Date : 13 novembre 2026.
- Espace : `whisper-adapters/worker_ipc/runtime/poll/parakeet.rs`, worktree `C:/wt/cs`.
- Mode : plan d’implémentation borné.
- Confiance : high — contrôle de plage et type `DecodedPcmBlock` inspectés.
- Qualité du plan : passée.

## 1. Résumé de l’audit

Clippy signale 102 lignes dans `advance_parakeet_vad_decode`. Après extraction d’un bloc de décodage, le runtime vérifie que le bloc retourné commence/termine exactement aux bornes demandées et qu’il contient le nombre exact d’échantillons avant d’ajouter le PCM et d’avancer l’offset.

## 2. Fraîcheur

Inspection du contrôle lines 254–263, du type `DecodedPcmBlock`, des tests poll Parakeet et AGENTS adapters. P1–P16 à préserver. La preuve d’absence de gap est nécessaire avant toute accumulation et toute fenêtre.

## 3. Hypothèses

- Safe : le triplet d’égalité de plage peut être exprimé dans un helper pur retournant bool.
- Validation needed : le helper vérifie exactement start, end et `samples.len() == end-start`.
- Validation needed : le même message `DecoderFailed: VAD decode block has a gap or wrong length` et le point de contrôle restent identiques.

## 4. Traçabilité

| Constat | Priorité | Disposition | Tâche | Note |
|---|---|---|---|---|
| `advance_parakeet_vad_decode` 102 lignes | P17 | Planned | P17-T1 | Extraire la validité d’un bloc |
| 119 autres sites | Hors lot | Deferred | Aucun | Tranches suivantes |

## 5. Séquence recommandée

Le helper pur porte seulement les trois comparaisons. Le runtime conserve l’erreur, l’ordre, le `try_reserve`, l’append PCM et le déplacement de l’offset.

### Phase P17 : nommer l’invariant de continuité du decode

Objective : supprimer l’alerte de longueur ciblée.
Closed invariant : seul un bloc exact pour la plage demandée est accepté.
Before metrics : 102 lignes; 120 sites.
Target metrics : ≤100 lignes; au plus 119 sites, aucune alerte ajoutée.
Metric commands : Clippy six crates et N1.
Stop conditions : arrêter si un des trois contrôles manque/change, si le message ou l’ordre accumulation/offset change, ou gate échoue.
Dependencies : aucune.
Expected outcome : helper privé `decoded_block_matches_vad_range`.
Tasks : P17-T1.

## 6. Tâches

#### P17-T1 — Extraire le prédicat de plage décodée exacte

- Priorité : Medium
- Source : Clippy 102/100 sur `advance_parakeet_vad_decode`.
- Justification : nommer le contrôle de continuité d’un bloc PCM avant accumulation.
- Fichiers : `crates/whisper-adapters/src/worker_ipc/runtime/poll/parakeet.rs` seulement.
- Bucket : adapters/infra — worker decode.
- Scope budget : un fichier production.
- Closed invariant : plage et nombre d’échantillons exactement égaux à la requête.
- Before metrics : 102; target ≤100.
- Metric commands : Clippy via garde.
- Stop conditions : Phase P17.
- Dépendances : aucune.
- Outline : créer un helper privé acceptant `&whisper_core::ports::DecodedPcmBlock`, `start`, `end`; déplacer les comparaisons exactes; l’appeler à la condition actuelle et garder l’erreur tuple job/generation/message dans la méthode.
- Tests : conserver les tests de décodeur et VAD/poll existants; aucun nouveau test inline.
- Vérifications : fmt check, Clippy structurel six crates, N1 via garde.
- Provenance : Observed pour le site; Repo docs pour les gates.
- Acceptation : les trois critères restent nécessaires, erreur identique, contrôle avant `try_reserve`/`extend`/offset, Clippy ciblé supprimé, N1 PASS, seul fichier ciblé changé.
- Retour arrière : rétablir l’expression si un seuil ou une erreur diffère.
- Notes : calculer la longueur attendue en `u64` comme avant, sans cast inversé.

## 7. Matrice de vérification

| Domaine | Vérification | Provenance | Attendu | Tâche | Limites |
|---|---|---|---|---|---|
| Préflight | SelfTests garde/PE et disque | Repo docs | PASS | P17-T1 | témoin PE réel non exécuté |
| Format | fmt check via garde | Repo docs | PASS | P17-T1 | aucune écriture rustfmt |
| Clippy | six crates, quatre lints | Observed/Repo docs | ≤119 sites | P17-T1 | GPU N2 hors lot |
| N1 | défaut + workers | Repo docs | PASS | P17-T1 | native ignored inchangé |
| Decode | tests VAD/poll existants | Source inferred | PASS | P17-T1 | aucun moteur natif lancé |

## 8. Portes de décision

| Gate | État | Bloque | Question | Défaut | Validation |
|---|---|---|---|---|---|
| P17-G1 | Validation | clôture | Le bloc est-il exact avant accumulation ? | Oui | ordre du code et tests |

## 9. Hors périmètre

Ne pas changer la taille des chunks (80 000), SourceRange, decoder, source hash, nombre total PCM, stratégie windows ou protocole. Aucun test native/GPU.

## 10. Auto-contrôle final

| Contrôle | Résultat | Note |
|---|---|---|
| Plan concret | Pass | prédicat/helper nommé |
| Sections présentes | Pass | modèle suivi |
| Traçabilité | Pass | 1 prévu, reste différé |
| Tâche complète | Pass | cibles/gates/critères/rollback |
| Mesures/arrêts | Pass | 102→≤100 |
| Budget | Pass | un fichier |
| Tests dédiés | Pass | tests existants réutilisés |
| Provenance | Pass | source/docs |
| Porte | Pass | avant accumulation |
| Chemins | Pass | vérifiés |
| Scope | Pass | validation pure seulement |

## 11. Prompt d’implémentation

Exécute P17-T1 du plan `docs/cadrage/plan-clippy-structure-vad-decode-block-validation-20261113.md` dans `C:/wt/cs`. Préserve P1–P16. Modifie uniquement `poll/parakeet.rs`; extrais les trois comparaisons exactes en helper privé, garde l’erreur/ordre. Passe SelfTests, fmt, Clippy et N1 par garde; arrête à tout échec.
