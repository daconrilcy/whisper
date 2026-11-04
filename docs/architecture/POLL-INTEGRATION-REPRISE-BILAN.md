Avancée : classification CLEAN ; six corps Auto alternés exécutés, tous PASS.
Blocage : moyenne candidat 40,94 s > plafond 39,237 s (+14,77 %) ; cache isolé conservé après refus outil.
Décision : arrêt sans diagnostic, correction, autres scénarios ni fusion ; bilan figé le 4 novembre 2026, aucun push.

# POLL-INTEGRATION — reprise du 4 novembre 2026

Dans C:/wt/poll : candidat c2a2d961 ; base production 6316a54b via 00c2d87e
avec banc 1be46656. Profils et banc identiques, bascules hors processus actif.
Durées : **corps Rust hors compilation**, comme B.

## Classification hors poll, chaque hunk du diff main...candidat

| Fichier | Hunks côté candidat | Élément englobant / compilation |
| --- | --- | --- |
| src/archive.rs | 38–40 | Déclaration pub(crate) mod test_support sous cfg(test) |
| src/archive/test_support.rs | 1–7 | Module test_support de la ligne précédente |
| src/staging.rs | 14–16 | Déclaration mod test_support sous cfg(test) |
| src/staging/test_support.rs | 1–13 | Module test_support de la ligne précédente |
| src/worker_ipc/tests/microphone_and_finalization.rs | 334 ; 337 ; 344–371 ; 373 ; 452 | Test microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end (202), inclus par tests.rs:5 ; module worker_ipc::tests déclaré sous cfg(test), worker_ipc.rs:140–141 |
| tests/live_archive.rs | 1–4 ; 11–12 ; 15 ; 20–21 | Imports, static NEXT_TEST_ROOT, helper test_root dans la crate de test d'intégration Cargo auto-détectée |

Chemins relatifs à crates/whisper-adapters/. **Aucune ligne produit hors poll.**
Garde N1, profils/banc, registre (86 ajouts, zéro retrait), tests et documents
autorisés ; manifeste/lock/worker inchangés. Non-suivis préservés.

## Auto — six PASS, codes 0, ordre base/candidat respecté

| Auto sans candidat GPU | Mesure 1 | Mesure 2 | Mesure 3 | Moyenne |
| --- | --- | --- | --- | --- |
| Base | 25,33 s | 40,94 s | 40,74 s | 35,67 s |
| Candidat | 40,93 s | 40,71 s | 41,18 s | 40,94 s |

Plafond 35,67 ×1,10 = 39,237 s ; écart +14,7743 % : **NON CONFORME**,
calcul confirmé indépendamment. Historique informatif : 26,14 / 41,55 s,
sans décision ni attribution causale.

| Autre corps candidat | B | Résultat |
| --- | ---: | --- |
| Reprise interrompue | 13,35 s | NON EXÉCUTÉ |
| Direct Turbo CPU | 42,54 s | NON EXÉCUTÉ |
| Import long FR | 168,57 s | NON EXÉCUTÉ |
| Import long EN | 187,17 s | NON EXÉCUTÉ |

Fetch, tag avant-fusion-poll, fusion et N1 main **NON EXÉCUTÉS** ; main
inchangé. Aucun code/test corrigé, aucun diagnostic. Preuves : six reçus
target/n2-native/logs/PersonalAutoFallback-*/.
Archive target/n2-native/archive/POLL-INTEGRATION-REPRISE-20261104.zip :
121 entrées, 101 130 octets source / 69 903 archivés, nombre/tailles/SHA-256
vérifiés. SHA-256 archive :
7F9FFC982DA497DEBD7026BACECD47BD5C5AE44667E73BE9F7DCC1386DE3B46E.
Cache target/n2-native/cargo-home conservé : 651 827 048 octets, sans jonction
ni campagne active avant tentative ; refus outil (« blocked by policy »).
Commande manuelle après revérification du chemin et absence de campagne :
`Remove-Item -LiteralPath 'C:\wt\poll\target\n2-native\cargo-home' -Recurse -Force`.
Caches persistants/partagés, ressources et bilans préservés ; 596 681 900 032
octets libres après tentative de nettoyage.

[Seconde reprise : médianes, modes et observations préalables](POLL-INTEGRATION-SECONDE-REPRISE-BILAN.md).
