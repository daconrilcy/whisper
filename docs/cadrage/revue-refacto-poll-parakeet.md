Avancée : seconde revue indépendante CLEAN documentaire sur 65ee3264 ; sept constats fermés par le même reviewer.
Blocage : accord utilisateur en attente ; borne d'exécution des appels à arbitrer.
Décision : aucun Rust modifié ; remettre le plan relu, deux cycles documentaires consommés.

# REFACTO-POLL-PARAKEET — revue documentaire

Mandat de session : revue du plan puis du diff final via `rust_review_coordinator` ; deux cycles maximum. Le coordinateur a délégué la revue documentaire à `rust_design_reviewer`, en lecture seule, le reviewer de code habituel étant réservé aux candidats déjà implémentés. L'auteur principal seul écrit ces documents.

## Première passe

Base `4c04818c17ff0c779c441e2b99f1e87779eb7cb9`, candidat plan `ee32d9c0245aef970cbd9d2983de4819b004277b`, blob `6fa6addd64c7691db83926ac487011aa2377a4a5` (OID revérifié par Git à la remise). Verdict indépendant **FINDINGS**. Sources et journaux de préparation lus ; aucun Cargo ni mutation par les agents de revue.

| ID | Sévérité | Constat et preuve | Réponse documentaire de l'auteur, soumise à relecture |
|---|---|---|---|
| RPP-P01 | BLOQUANT | N2 non exécutable : LotRoot proposé non autorisé ; AppData/C:/wb non permis comme inputs directs. `ci/guard-native.ps1:1042–1060`, AllowedReadRoots et Get-ValuesDocument. | §7 : racine exacte target/n2-native ; copies des 21 ressources sélectionnées et deux audios sous worktree avant manifeste, sans modification de garde. |
| RPP-P02 | BLOQUANT | Le fichier verrouillé est refusé dès preflight C10 : inspect demande DELETE ; ne prouve pas échec de suppression ni progrès partiel. `windows_file_identity.rs:43–60,189–200`, cleanup/execution et RC:1321–1378. | D04 préflight zéro retrait ; D06 dossier non vide avec enfant utilisateur, fichiers owned retirés avant échec, plan/last_error persistés, cibles suivantes conservées puis reprise. Limite d'observation ordre intra-cible explicite et preuve par diff. |
| RPP-P03 | IMPORTANT | Q03 positif n'est pas assuré par faux worker synthétique : requalification/pending/modèle requis. `runtime/import/worker_start.rs::fallback_parakeet_to_cpu`, `start_parakeet_worker`. | Prérequis T1 : route CPU réellement admise, pending remplaçable, SHA/durée source, GPU Auto préaudio dans la cible ; observer un seul CPU et Ready/Transcribe. STOP si impossible sans nouveaux moyens. Routes voisines séparées. |
| RPP-P04 | IMPORTANT | Début chrono S2 incohérent, aucune terminaison bornée démontrée ; garde WaitForExit sans timeout, marge/Ctrl+C insuffisants. | Chrono dès préparation spécifique ; admission max observé+2 min et réserve clôture ; garde inchangée. Décision explicite requise pour laisser finir l'unique appel engagé après fermeture de l'admission ; sinon STOP avant appel exposé. Aucune relaxation déduite de TESTS-STABLES-2. |
| RPP-P05 | IMPORTANT | Recette import attendait un WAV alors que cette publication donne TXT/SRT. | §8 : import TXT/SRT et audio source intact ; triplet TXT/SRT/WAV réservé au live. |
| RPP-P06 | IMPORTANT | Taille brute JSON varie avec chemins/IDs OS ; comparaison non stable. | §4 : octets/tailles bruts conservés, cohérence vérifiée ; taille comparée des seuls JSON volatils recalculée après substitutions permises ; autres tailles brutes, deux replays avant gel. Pas de hash arbitrairement masqué. |
| RPP-P07 | IMPORTANT | Extraire Ready en premier crée un helper ~160 lignes, contraire au plafond nouveau ≤100 ; même risque cleanup/Result. | T3/T7/T8 et règle générale : responsabilités internes bornées d'abord, puis reliquat ; chaque fonction/commit respecte immédiatement les seuils. |

Correction factuelle supplémentaire signalée par l'auteur : le deuxième appelant verrouillé est `ArchiveStore::commit_publish` (publication.rs:135), pas `publish_import` ; nom corrigé. Cette correction ne change pas le périmètre.

## Seconde passe — verdict final documentaire

Même reviewer indépendant, candidat **65ee326442d0a9ce3d177041bd51a93a04a12d47**, blob plan **8b98fa2ec94c0cbdb98723df468235ee72978dc1** : **CLEAN documentaire**. RPP-P01, RPP-P02, RPP-P03, RPP-P04, RPP-P05, RPP-P06, RPP-P07 **fermés par le reviewer**, aucun REQUIRED ouvert. Aucun Cargo ni mutation pendant la revue ; diff check et identité des documents contrôlés. Deux cycles utilisés, pas de troisième revue.

Deux errata consultatifs du second passage appliqués par l'auteur à la remise : OID exact du premier plan ci-dessus ; cellule C12 corrigée pour associer D06 au dossier non vide et déplacer D04 fichier verrouillé vers C10. Aucun changement de solution ni extension.

Limites explicitement conservées par le reviewer : **CLEAN du plan ne signifie ni Q03/N2 exécutés, ni interruption bornée démontrée, ni accord d'implémentation**. La décision de budget §1 demeure requise pour les appels exposés. Le plan n'est pas approuvé par l'utilisateur et aucune implémentation n'est lancée.

## Preuves de préparation lues

Clippy métrique 30/32 au seuil 0, rc0 ; N1 878/0/129 sans CARGO_TARGET_DIR, rc0 ; SelfTest N1 64 positifs/99 négatifs ; WHEA 1→1, dernier RecordId 527992. Clippy qualité ultérieur rc0, 340 warnings dont 328 Clippy. [Archive de préparation](../architecture/raw/REFACTO-POLL-PARAKEET-PREUVES.zip) : **40 membres**, **949 172 octets**, SHA-256 **7e303e7729bf19a0c7438401367301a3c68dd5f99e31118fc1821524f981267e** ; nombre et chaque contenu revérifiés après réouverture contre les sources, inventaire de racines inclus. Ces résultats valident la base seulement ; aucun corpus nouveau, S2 préventif, N2 ni recette exécuté à ce stade. Aucun bilan de clôture figé prématurément ; aucun merge/push/nettoyage.
